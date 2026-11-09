//! What a graphic's frames show that a contact sheet can hide: a face partly covered, captions
//! drawn under a graphic, captions across a short's split line, a graphic that holds still.
//! Measured from the scene the export draws at the sheet's moments (layers with their pixels),
//! not judged by a model. Each finding names its moments; what can't be checked says so. `run`
//! answers `frame_render` about one graphic; `findings` is the same look across every graphic,
//! for the check before export.
use crate::check::{Finding, Kind, Severity};
use crate::render::{Layer, LayerRole, Scene};

/// Scenes are measured this wide: enough to place a face and a caption line.
const CHECK_WIDTH: u32 = 480;
/// An overlay smaller than this share of the canvas (the cursor) is not a graphic.
const MIN_OVERLAY_SHARE: f64 = 0.04;
/// Alpha from which a graphic's pixel hides what is under it.
const OPAQUE_ALPHA: u8 = 200;
/// A graphic opaque over this share of the canvas is a card that replaces the picture: covering
/// the face then is the point, not a mistake.
const CARD_SHARE: f64 = 0.85;
/// Share of the face (or the caption line) hidden from which it counts.
const FACE_HIDDEN: f64 = 0.35;
const CAPTION_HIDDEN: f64 = 0.3;
/// A graphic changing less than this share of its signature between two moments holds still.
const STILL_CHANGE: f64 = 0.004;
/// A hold longer than this share of the graphic (and of these seconds) is worth a word.
const STILL_SHARE: f64 = 0.6;
const STILL_MIN_US: u64 = 4_000_000;
/// Where Shorts, Reels and TikTok draw their own bars, buttons and text over a 9:16 video, as
/// shares of the canvas (x, y, w, h). Words a graphic puts there get covered once posted.
const PLATFORM_BANDS: [(&str, Rect); 5] = [
    ("the top bar", (0.0, 0.0, 1.0, 0.08)),
    ("the title and caption at the bottom", (0.0, 0.8, 1.0, 0.2)),
    ("the buttons on the right", (0.88, 0.45, 0.12, 0.35)),
    ("the left edge", (0.0, 0.0, 0.05, 1.0)),
    ("the right edge", (0.95, 0.0, 0.05, 1.0)),
];
/// Changes in brightness along a row from which a sample step counts as an edge.
const EDGE_LUMA: i32 = 56;
/// Edges along one row from which the row holds words: a shape's border makes two, a word many.
const WORD_EDGES: usize = 6;
const WORD_EDGES_NARROW: usize = 4;
/// Rows of words in a band from which it counts (a stray descender doesn't).
const WORD_ROWS: usize = 3;
/// In the check before export: a frame every so often across a graphic, at least and at most
/// this many, on at most this many graphics (a look at each, not a sheet of it).
const CHECK_EVERY_US: u64 = 1_500_000;
const CHECK_MIN_FRAMES: usize = 3;
const CHECK_MAX_FRAMES: usize = 6;
const CHECK_MAX_GRAPHICS: usize = 40;
/// How long the check waits for renders and face tracks before it says what it couldn't look
/// at: renders take most of a minute each, and whatever finished stays for the next check.
const CHECK_MAX_WAIT: std::time::Duration = std::time::Duration::from_secs(180);

type Rect = (f64, f64, f64, f64);

fn rect_of(layer: &Layer) -> Rect {
    (
        layer.x as f64,
        layer.y as f64,
        layer.width as f64,
        layer.height as f64,
    )
}

/// The pixel of `layer`'s picture drawn at canvas point (`px`, `py`), as BGRA.
fn pixel(layer: &Layer, px: f64, py: f64) -> Option<[u8; 4]> {
    let frame = &layer.frame;
    if frame.format != crate::media::PixelFormat::Bgra8888 || layer.width == 0 || layer.height == 0
    {
        return None;
    }
    let u = layer.uv_x as f64 + (px - layer.x as f64) / layer.width as f64 * layer.uv_w as f64;
    let v = layer.uv_y as f64 + (py - layer.y as f64) / layer.height as f64 * layer.uv_h as f64;
    if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
        return None;
    }
    let fx = (u * frame.width as f64) as usize;
    let fy = (v * frame.height as f64) as usize;
    let at = fy * frame.stride as usize + fx * 4;
    frame.data.get(at..at + 4).map(|p| [p[0], p[1], p[2], p[3]])
}

/// The share of `area` (canvas pixels) where `layer` draws an opaque pixel, sampled on a grid.
fn opaque_share(layer: &Layer, area: Rect) -> f64 {
    let (ax, ay, aw, ah) = area;
    let (lx, ly, lw, lh) = rect_of(layer);
    let steps = 24;
    let mut hits = 0;
    for j in 0..steps {
        for i in 0..steps {
            let px = ax + (i as f64 + 0.5) / steps as f64 * aw;
            let py = ay + (j as f64 + 0.5) / steps as f64 * ah;
            let inside = px >= lx && px < lx + lw && py >= ly && py < ly + lh;
            if inside && pixel(layer, px, py).is_some_and(|p| p[3] >= OPAQUE_ALPHA) {
                hits += 1;
            }
        }
    }
    hits as f64 / (steps * steps) as f64
}

/// A small picture of what `layer` draws over the canvas, to compare moments.
fn signature(layer: &Layer, scene: &Scene) -> Vec<u8> {
    let (cols, rows) = (32, 18);
    let mut out = Vec::with_capacity(cols * rows * 4);
    for j in 0..rows {
        for i in 0..cols {
            let px = (i as f64 + 0.5) / cols as f64 * scene.width as f64;
            let py = (j as f64 + 0.5) / rows as f64 * scene.height as f64;
            out.extend(pixel(layer, px, py).unwrap_or([0; 4]));
        }
    }
    out
}

fn changed(a: &[u8], b: &[u8]) -> f64 {
    let cells = a.len().min(b.len()) / 4;
    if cells == 0 {
        return 1.0;
    }
    let moved = (0..cells)
        .filter(|&c| (0..4).any(|k| (a[c * 4 + k] as i32 - b[c * 4 + k] as i32).abs() > 24))
        .count();
    moved as f64 / cells as f64
}

/// Where the face is drawn on the canvas at `at`: the camera clip shown then, its face track's
/// nearest look, through the camera layer's crop (and mirroring). `None` when no camera shows
/// or its face isn't known.
fn face_rect(
    root: &std::path::Path,
    document: &crate::project::revision::EditDocument,
    at: u64,
    scene: &Scene,
) -> Option<Rect> {
    let camera = scene.layers.iter().find(|l| l.role == LayerRole::Webcam)?;
    let clip = document
        .sequence
        .video_tracks()
        .filter(|t| !t.hidden)
        .filter_map(|t| {
            let clip = t.clip_at(at)?;
            (crate::sequence::clip_role(&document.assets, t, clip)
                == Some(crate::sequence::Role::Webcam))
            .then_some(clip)
        })
        .next_back()?;
    let local = clip.local_us(at)?;
    let track = crate::face::cached_track(root, &clip.asset, &clip.stream)?;
    let face = track
        .samples
        .iter()
        .filter(|s| s.face.is_some())
        .min_by_key(|s| s.t_us.abs_diff(local))?
        .face?;
    let (lx, ly, lw, lh) = rect_of(camera);
    let to_x = |u: f32| lx + (u - camera.uv_x) as f64 / camera.uv_w as f64 * lw;
    let to_y = |v: f32| ly + (v - camera.uv_y) as f64 / camera.uv_h as f64 * lh;
    let (x0, x1) = {
        let (a, b) = (to_x(face.x), to_x(face.x + face.w));
        (a.min(b).max(lx), a.max(b).min(lx + lw))
    };
    let (y0, y1) = (to_y(face.y).max(ly), to_y(face.y + face.h).min(ly + lh));
    let full =
        face.w as f64 / camera.uv_w.abs() as f64 * lw * face.h as f64 / camera.uv_h as f64 * lh;
    // Mostly cropped out of the slot: nothing of it to cover.
    ((x1 - x0) * (y1 - y0) > 0.4 * full && x1 > x0 && y1 > y0).then_some((x0, y0, x1 - x0, y1 - y0))
}

/// Where a short's camera and screen meet, when one sits above the other.
fn split_line(scene: &Scene) -> Option<f64> {
    let camera = scene.layers.iter().find(|l| l.role == LayerRole::Webcam)?;
    let screen = scene.layers.iter().find(|l| l.role == LayerRole::Screen)?;
    let (top, bottom) = if camera.y <= screen.y {
        (camera, screen)
    } else {
        (screen, camera)
    };
    let meet = (top.y + top.height) as f64;
    let slack = scene.height as f64 * 0.03;
    ((bottom.y as f64 - meet).abs() <= slack).then_some((meet + bottom.y as f64) / 2.0)
}

/// The rows of `area` (canvas pixels) where `layer` draws what looks like words: many sharp
/// changes along the row, counting see-through to opaque as one. Rows are two pixels apart.
fn word_rows(layer: &Layer, area: Rect) -> usize {
    let (ax, ay, aw, ah) = area;
    let (lx, ly, lw, lh) = rect_of(layer);
    let (x0, x1) = (ax.max(lx), (ax + aw).min(lx + lw));
    let (y0, y1) = (ay.max(ly), (ay + ah).min(ly + lh));
    if x1 - x0 < 4.0 || y1 <= y0 {
        return 0;
    }
    let needed = if x1 - x0 >= 100.0 {
        WORD_EDGES
    } else {
        WORD_EDGES_NARROW
    };
    let look = |px: f64, py: f64| {
        pixel(layer, px, py)
            .filter(|p| p[3] >= OPAQUE_ALPHA)
            .map(|p| (p[2] as i32 * 299 + p[1] as i32 * 587 + p[0] as i32 * 114) / 1000)
    };
    let mut rows = 0;
    let mut py = y0 + 0.5;
    while py < y1 {
        let mut edges = 0;
        let mut last = look(x0 + 0.5, py);
        let mut px = x0 + 1.5;
        while px < x1 {
            let now = look(px, py);
            let edge = match (last, now) {
                (Some(a), Some(b)) => (a - b).abs() >= EDGE_LUMA,
                (None, None) => false,
                _ => true,
            };
            edges += edge as usize;
            last = now;
            px += 1.0;
        }
        rows += (edges >= needed) as usize;
        py += 2.0;
    }
    rows
}

/// The platform bands of a 9:16 `scene` where `layer` draws words.
fn bands_with_words(layer: &Layer, scene: &Scene) -> Vec<&'static str> {
    let (w, h) = (scene.width as f64, scene.height as f64);
    PLATFORM_BANDS
        .iter()
        .filter(|(_, (x, y, bw, bh))| word_rows(layer, (x * w, y * h, bw * w, bh * h)) >= WORD_ROWS)
        .map(|(name, _)| *name)
        .collect()
}

fn times(moments: &[u64]) -> String {
    moments
        .iter()
        .map(|&at| crate::check::clock(at))
        .collect::<Vec<_>>()
        .join(", ")
}

/// What frames showed, by the moments it showed it.
#[derive(Default)]
struct Seen {
    face_hidden: Vec<u64>,
    face_seen: bool,
    camera_seen: bool,
    captions_hidden: Vec<u64>,
    captions_split: Vec<u64>,
    unsafe_words: Vec<(u64, Vec<&'static str>)>,
    signatures: Vec<(u64, Vec<u8>)>,
}

impl Seen {
    /// The platform bands words sat under, each once.
    fn bands(&self) -> Vec<&'static str> {
        let mut bands: Vec<&'static str> = Vec::new();
        for band in self.unsafe_words.iter().flat_map(|(_, b)| b) {
            if !bands.contains(band) {
                bands.push(band);
            }
        }
        bands
    }
}

/// Measures the scenes at `moments`.
fn look_at(
    evaluator: &mut crate::export::SceneEvaluator,
    root: &std::path::Path,
    document: &crate::project::revision::EditDocument,
    moments: &[u64],
) -> Result<Seen, String> {
    let portrait = crate::style::is_portrait(document);
    let mut seen = Seen::default();
    for &at in moments {
        let scene = evaluator.scene_at(at)?;
        let canvas = (scene.width as f64) * (scene.height as f64);
        let whole = (0.0, 0.0, scene.width as f64, scene.height as f64);
        let overlays: Vec<(usize, &Layer)> = scene
            .layers
            .iter()
            .enumerate()
            .filter(|(_, l)| {
                l.role == LayerRole::Overlay
                    && (l.width as f64 * l.height as f64) >= MIN_OVERLAY_SHARE * canvas
            })
            .collect();
        if let Some((_, largest)) = overlays
            .iter()
            .max_by_key(|(_, l)| l.width as u64 * l.height as u64)
        {
            seen.signatures.push((at, signature(largest, &scene)));
        }
        if portrait {
            let mut bands: Vec<&'static str> = Vec::new();
            for (_, overlay) in &overlays {
                for band in bands_with_words(overlay, &scene) {
                    if !bands.contains(&band) {
                        bands.push(band);
                    }
                }
            }
            if !bands.is_empty() {
                seen.unsafe_words.push((at, bands));
            }
        }
        seen.camera_seen |= scene.layers.iter().any(|l| l.role == LayerRole::Webcam);
        if let Some(face) = face_rect(root, document, at, &scene) {
            seen.face_seen = true;
            let covered = overlays.iter().any(|(_, o)| {
                opaque_share(o, whole) < CARD_SHARE && opaque_share(o, face) >= FACE_HIDDEN
            });
            if covered {
                seen.face_hidden.push(at);
            }
        }
        for (ci, caption) in scene
            .layers
            .iter()
            .enumerate()
            .filter(|(_, l)| l.role == LayerRole::Caption)
        {
            let line = rect_of(caption);
            // Drawn after the captions, a graphic hides them (a card that replaces the
            // whole picture hides them on purpose).
            if overlays.iter().any(|(oi, o)| {
                *oi > ci
                    && opaque_share(o, whole) < CARD_SHARE
                    && opaque_share(o, line) >= CAPTION_HIDDEN
            }) && !seen.captions_hidden.contains(&at)
            {
                seen.captions_hidden.push(at);
            }
            if portrait {
                if let Some(y) = split_line(&scene) {
                    let slack = scene.height as f64 * 0.01;
                    if line.1 < y - slack
                        && line.1 + line.3 > y + slack
                        && !seen.captions_split.contains(&at)
                    {
                        seen.captions_split.push(at);
                    }
                }
            }
        }
    }
    Ok(seen)
}

/// The moments of a graphic's clip to look at in the check before export.
fn check_moments(start_us: u64, end_us: u64) -> Vec<u64> {
    let count = ((end_us.saturating_sub(start_us) / CHECK_EVERY_US) as usize)
        .clamp(CHECK_MIN_FRAMES, CHECK_MAX_FRAMES);
    super::storyboard::spread(start_us, end_us, count)
}

/// The findings of one graphic's frames: only what gets in a viewer's way, as holding still is
/// often fine and `frame_render` says so when someone is looking at it.
fn graphic_findings(name: &str, seen: &Seen, out: &mut Vec<Finding>) {
    let mut add = |kind: Kind, title: String, detail: String, moments: &[u64]| {
        let (Some(&first), Some(&last)) = (moments.first(), moments.last()) else {
            return;
        };
        let mut finding = Finding::new(kind, Severity::Warning, title, detail);
        finding.at(first, last);
        out.push(finding);
    };
    add(
        Kind::GraphicCoversFace,
        format!("\"{name}\" covers part of the face"),
        format!(
            "At {}. Move it beside the face, make that part see-through, or make it a full card.",
            times(&seen.face_hidden)
        ),
        &seen.face_hidden,
    );
    add(
        Kind::GraphicHidesCaptions,
        format!("\"{name}\" hides the captions"),
        format!(
            "At {}. Keep its opaque parts off the caption line, or leave the line clear.",
            times(&seen.captions_hidden)
        ),
        &seen.captions_hidden,
    );
    add(
        Kind::CaptionsCrossSplit,
        format!("Captions cross the split line under \"{name}\""),
        format!(
            "At {}. Move them into one part.",
            times(&seen.captions_split)
        ),
        &seen.captions_split,
    );
    let moments: Vec<u64> = seen.unsafe_words.iter().map(|(at, _)| *at).collect();
    add(
        Kind::GraphicUnderPlatformUi,
        format!("\"{name}\" has words under the platform's bars"),
        format!(
            "Words or detail sit under {} at {}: Shorts and TikTok draw their own over it there. \
             Keep them between 8% and 80% of the height, 5% in from the sides and, below the \
             middle, off the right 12%.",
            seen.bands().join(", "),
            times(&moments)
        ),
        &moments,
    );
}

/// Runs `wait`, setting the flag it is given once `CHECK_MAX_WAIT` has passed.
fn within_the_wait<T>(wait: impl FnOnce(&std::sync::atomic::AtomicBool) -> T) -> T {
    use std::sync::atomic::{AtomicBool, Ordering};
    let (cancel, done) = (AtomicBool::new(false), AtomicBool::new(false));
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let started = std::time::Instant::now();
            while !done.load(Ordering::SeqCst) {
                if started.elapsed() >= CHECK_MAX_WAIT {
                    cancel.store(true, Ordering::SeqCst);
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        });
        let out = wait(&cancel);
        done.store(true, Ordering::SeqCst);
        out
    })
}

/// What every graphic's frames show, for the check before export of the video or of `short`.
/// Renders what isn't rendered yet (export would have to anyway) and waits for face tracks, for
/// so long; past that it says so rather than hold up whoever asked.
pub fn findings(
    state: &crate::commands::AppState,
    short: Option<&str>,
) -> Result<Vec<Finding>, String> {
    let (document, root, _) = super::frames::document(state, short)?;
    let graphics = |document: &crate::project::revision::EditDocument| -> Vec<(String, u64, u64)> {
        let mut clips: Vec<(String, u64, u64)> = document
            .sequence
            .video_tracks()
            .filter(|t| !t.hidden)
            .flat_map(|t| &t.clips)
            .filter_map(|c| {
                let asset = document.asset(&c.asset).filter(|a| a.is_graphic())?;
                Some((asset.name.clone(), c.start_us, c.end_us()))
            })
            .collect();
        clips.sort_by_key(|c| c.1);
        clips
    };
    if graphics(&document).is_empty() {
        return Ok(Vec::new());
    }
    let rendered = within_the_wait(|cancel| {
        let rendered =
            super::frames::wait_for_graphics_until(state, short, 0, document.duration_us(), cancel);
        crate::face::wait_for_tracks(&root, &document, cancel, &mut |_| {});
        rendered.map_err(|why| {
            if cancel.load(std::sync::atomic::Ordering::SeqCst) {
                format!(
                    "Graphics were still rendering after {} minutes. What finished is kept: \
                     `graphic_render` waits for the rest, then check again.",
                    CHECK_MAX_WAIT.as_secs() / 60
                )
            } else {
                why
            }
        })
    });
    rendered?;
    let mut out = Vec::new();
    super::frames::with_document_evaluator(state, short, CHECK_WIDTH, |evaluator, document| {
        let clips = graphics(document);
        let mut face_unknown = false;
        for (name, start, end) in clips.iter().take(CHECK_MAX_GRAPHICS) {
            let seen = look_at(evaluator, &root, document, &check_moments(*start, *end))?;
            graphic_findings(name, &seen, &mut out);
            face_unknown |= seen.camera_seen && !seen.face_seen;
        }
        // A placeholder isn't the graphic: measuring it would only mislead.
        if evaluator.drew_placeholder() {
            out.clear();
            out.push(Finding::new(
                Kind::GraphicFramesNotChecked,
                Severity::Info,
                "Graphics' frames not checked".into(),
                "A graphic is still rendering. Check again once it has.".into(),
            ));
            return Ok(());
        }
        if face_unknown {
            out.push(Finding::new(
                Kind::GraphicFramesNotChecked,
                Severity::Info,
                "Faces under graphics not checked".into(),
                "The camera has no face track, so whether a graphic covers the face is unknown."
                    .into(),
            ));
        }
        if clips.len() > CHECK_MAX_GRAPHICS {
            out.push(Finding::new(
                Kind::GraphicFramesNotChecked,
                Severity::Info,
                "Not every graphic's frames were checked".into(),
                format!(
                    "The first {CHECK_MAX_GRAPHICS} of {} were. `frame_render` with `graphic` \
                     looks at any other.",
                    clips.len()
                ),
            ));
        }
        Ok(())
    })?;
    Ok(crate::check::finished(out))
}

/// The checks of `graphic`'s frames at `moments` (of the edit, or of `short`), as lines for the
/// agent: findings first, then what wasn't checked.
pub fn run(
    state: &crate::commands::AppState,
    short: Option<&str>,
    graphic: &str,
    moments: &[u64],
) -> Result<Vec<String>, String> {
    let (_, root, _) = super::frames::document(state, short)?;
    super::frames::with_document_evaluator(state, short, CHECK_WIDTH, |evaluator, document| {
        let length = document
            .sequence
            .video_tracks()
            .flat_map(|t| &t.clips)
            .filter(|c| c.asset == graphic)
            .map(|c| c.duration_us)
            .max()
            .unwrap_or(0);
        let seen = look_at(evaluator, &root, document, moments)?;
        // A placeholder isn't the graphic: measuring it would only mislead.
        if evaluator.drew_placeholder() {
            return Ok(vec![
                "Checks wait for the render: a graphic here is still rendering \
                 (`graphic_render` waits for it)."
                    .to_string(),
            ]);
        }
        let mut lines = Vec::new();
        if !seen.face_hidden.is_empty() {
            lines.push(format!(
                "- The graphic covers part of the face at {}: move it beside the face, make that \
                 part see-through, or make it a full card.",
                times(&seen.face_hidden)
            ));
        }
        if !seen.captions_hidden.is_empty() {
            lines.push(format!(
                "- It hides the captions at {}: keep its opaque parts off the caption line, or \
                 leave the line clear.",
                times(&seen.captions_hidden)
            ));
        }
        if !seen.captions_split.is_empty() {
            lines.push(format!(
                "- The captions cross the split line at {}: move them into one part.",
                times(&seen.captions_split)
            ));
        }
        if !seen.unsafe_words.is_empty() {
            let moments: Vec<u64> = seen.unsafe_words.iter().map(|(at, _)| *at).collect();
            lines.push(format!(
                "- Words or detail sit under {} at {}: Shorts and TikTok draw their own over it \
                 there. Keep them between 8% and 80% of the height, 5% in from the sides and, \
                 below the middle, off the right 12%.",
                seen.bands().join(", "),
                times(&moments)
            ));
        }
        // The longest stretch of moments in a row where it doesn't change.
        let mut still_from: Option<u64> = None;
        let mut longest = (0u64, 0u64);
        for pair in seen.signatures.windows(2) {
            let ((a, sa), (b, sb)) = (&pair[0], &pair[1]);
            if changed(sa, sb) < STILL_CHANGE {
                let from = *still_from.get_or_insert(*a);
                if b - from > longest.1 - longest.0 {
                    longest = (from, *b);
                }
            } else {
                still_from = None;
            }
        }
        let held = longest.1 - longest.0;
        if held >= STILL_MIN_US && held as f64 >= STILL_SHARE * length as f64 {
            lines.push(format!(
                "- It holds still from {} to {} ({:.0} s of {:.0} s): fine for reading a card; a \
                 demonstration should change while it is spoken about.",
                crate::check::clock(longest.0),
                crate::check::clock(longest.1),
                held as f64 / 1e6,
                length as f64 / 1e6
            ));
        }
        let mut found = if lines.is_empty() {
            vec!["Checks at these frames: nothing found.".to_string()]
        } else {
            let mut all = vec!["Checks at these frames:".to_string()];
            all.extend(lines);
            all
        };
        // Reading time is `check`'s (from the words and the length, no frames needed).
        if seen.camera_seen && !seen.face_seen {
            found.push("Not checked: the face (the camera has no face track yet).".to_string());
        }
        Ok(found)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::VideoFrame;

    fn layer(role: LayerRole, rect: (u32, u32, u32, u32), alpha: u8) -> Layer {
        let mut frame = VideoFrame::solid(16, 16, 255, 255, 255, 0).unwrap();
        for pixel in frame.data.chunks_exact_mut(4) {
            pixel[3] = alpha;
        }
        Layer::placed(frame, rect.0, rect.1, rect.2, rect.3).with_role(role)
    }

    #[test]
    fn findings_name_the_graphic_and_where_and_only_what_gets_in_the_way() {
        let mut out = Vec::new();
        graphic_findings("Clean", &Seen::default(), &mut out);
        assert!(out.is_empty());
        let seen = Seen {
            face_hidden: vec![2_000_000, 3_500_000],
            unsafe_words: vec![(3_500_000, vec!["the top bar"])],
            // Holding still is `frame_render`'s to mention, not a problem before export.
            signatures: vec![(0, vec![0; 4]), (9_000_000, vec![0; 4])],
            ..Seen::default()
        };
        graphic_findings("Prize pool", &seen, &mut out);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].kind, Kind::GraphicCoversFace);
        assert_eq!(out[0].severity, Severity::Warning);
        assert!(out[0].title.contains("Prize pool"), "{}", out[0].title);
        assert_eq!(
            (out[0].start_us, out[0].end_us),
            (Some(2_000_000), Some(3_500_000))
        );
        assert_eq!(out[1].kind, Kind::GraphicUnderPlatformUi);
        assert!(out[1].detail.contains("the top bar"), "{}", out[1].detail);
        // With ids and in order, ready for the report.
        let done = crate::check::finished(out);
        assert_eq!(done[0].id, "graphic_covers_face-2000");
        assert_eq!(done[1].id, "graphic_under_platform_ui-3500");
    }

    #[test]
    fn a_graphic_gets_a_few_frames_however_long_it_plays() {
        assert_eq!(check_moments(1_000_000, 2_000_000).len(), CHECK_MIN_FRAMES);
        assert_eq!(check_moments(0, 6_000_000).len(), 4);
        assert_eq!(check_moments(0, 600_000_000).len(), CHECK_MAX_FRAMES);
    }

    #[test]
    fn opaque_share_counts_only_what_the_layer_draws_there() {
        let card = layer(LayerRole::Overlay, (0, 0, 100, 50), 255);
        assert!((opaque_share(&card, (0.0, 0.0, 100.0, 100.0)) - 0.5).abs() < 0.05);
        let glass = layer(LayerRole::Overlay, (0, 0, 100, 100), 40);
        assert_eq!(opaque_share(&glass, (0.0, 0.0, 100.0, 100.0)), 0.0);
        // A mirrored layer (negative uv width) still maps into its picture.
        let mirrored = layer(LayerRole::Webcam, (0, 0, 100, 100), 255).mirrored();
        assert!(pixel(&mirrored, 10.0, 10.0).is_some());
    }

    #[test]
    fn the_split_line_is_where_camera_and_screen_meet() {
        let scene = Scene {
            width: 90,
            height: 160,
            background: [0.0; 4],
            layers: vec![
                layer(LayerRole::Webcam, (0, 0, 90, 70), 255),
                layer(LayerRole::Screen, (0, 70, 90, 90), 255),
            ],
        };
        assert_eq!(split_line(&scene), Some(70.0));
        let side = Scene {
            layers: vec![
                layer(LayerRole::Webcam, (0, 0, 45, 160), 255),
                layer(LayerRole::Screen, (45, 0, 45, 160), 255),
            ],
            ..scene
        };
        assert_eq!(split_line(&side), None);
    }

    /// A white 90x160 card, with dark stripes (like letters) across rows `from..to`.
    fn card_with_words(from: u32, to: u32) -> Layer {
        let mut frame = VideoFrame::solid(90, 160, 255, 255, 255, 0).unwrap();
        let stride = frame.stride as usize;
        for y in from..to {
            for x in (10..80).filter(|x| x % 4 < 2) {
                let at = y as usize * stride + x * 4;
                frame.data[at..at + 3].copy_from_slice(&[20, 20, 20]);
            }
        }
        Layer::placed(frame, 0, 0, 90, 160).with_role(LayerRole::Overlay)
    }

    #[test]
    fn words_under_the_platform_bars_are_found_and_plain_shapes_are_not() {
        let scene = Scene {
            width: 90,
            height: 160,
            background: [0.0; 4],
            layers: Vec::new(),
        };
        // Words in the middle, a plain card: nothing.
        assert!(bands_with_words(&card_with_words(60, 80), &scene).is_empty());
        assert!(
            bands_with_words(&layer(LayerRole::Overlay, (0, 0, 90, 160), 255), &scene).is_empty()
        );
        // A box with only its borders in the bottom band: two edges a row, not words.
        assert!(
            bands_with_words(&layer(LayerRole::Overlay, (20, 120, 50, 40), 255), &scene).is_empty()
        );
        // Words low down sit under the title and caption.
        assert_eq!(
            bands_with_words(&card_with_words(135, 150), &scene),
            vec!["the title and caption at the bottom"]
        );
        assert_eq!(
            bands_with_words(&card_with_words(2, 10), &scene),
            vec!["the top bar"]
        );
    }

    #[test]
    fn a_still_graphic_changes_nothing_between_moments() {
        let scene = Scene {
            width: 64,
            height: 36,
            background: [0.0; 4],
            layers: Vec::new(),
        };
        let a = signature(&layer(LayerRole::Overlay, (0, 0, 64, 36), 255), &scene);
        let b = signature(&layer(LayerRole::Overlay, (0, 0, 32, 36), 255), &scene);
        assert_eq!(changed(&a, &a), 0.0);
        assert!(changed(&a, &b) > 0.4);
    }
}
