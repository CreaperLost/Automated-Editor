//! A storyboard: one small contact sheet of an edit, a tile per scene, so an agent can check a
//! whole edit for the price of one image (and the user can see it at a glance). Each tile is a
//! finished frame, drawn as export draws it, over a caption strip: its time, what the frame
//! shows (screen, camera, split), the zoom, a graphic, a chapter, and what is said.
//!
//! Scenes start where the picture changes: the camera filling the frame or a short's scene,
//! a zoom, a graphic, a chapter, a long silence. Stretches too short to read merge into a
//! neighbour, the weakest change first, and the sheet holds at most [`MAX_TILES`].
use super::tools::{clock, params, with_reader, Ctx, Output};
use super::views::{blit_bgra, draw_text, fill, text_width};
use crate::commands::AppState;
use crate::project::revision::EditDocument;
use crate::render::{LayerRole, Scene};
use image::{Rgba, RgbaImage};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const MAX_TILES: usize = 48;
const DEFAULT_TILES: usize = 24;
/// Tiles shorter than this merge into a neighbour.
const MIN_TILE_US: u64 = 2_000_000;
/// With tiles to spare, stretches longer than this are halved, so a long take still shows.
const LONG_TILE_US: u64 = 30_000_000;
/// Silence this long between words is a scene of its own.
const LONG_GAP_US: u64 = 3_000_000;
/// Changes closer together than this are one.
const SAME_MARK_US: u64 = 250_000;
/// Words of speech a tile's JSON carries at most, in characters.
const SAY_CHARS: usize = 90;

/// How firmly a change holds a tile edge when tiles merge; the weakest goes first.
const BY_GAP: u8 = 1;
const BY_ZOOM_END: u8 = 1;
const BY_ZOOM: u8 = 2;
const BY_SCENE: u8 = 3;

const BACKGROUND: Rgba<u8> = Rgba([22, 24, 28, 255]);
const TITLE: Rgba<u8> = Rgba([230, 232, 236, 255]);
const ACCENT: Rgba<u8> = Rgba([120, 200, 255, 255]);
const CHAPTER: Rgba<u8> = Rgba([255, 200, 110, 255]);
const SAID: Rgba<u8> = Rgba([150, 156, 168, 255]);
const GAP: u32 = 4;
const STRIP_H: u32 = 46;
const TEXT_PX: f32 = 11.0;
const LINE_H: i64 = 14;

#[derive(Deserialize, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct StoryboardArgs {
    /// A short's id or title: its 9:16 frames, on its own timeline. Default: the video.
    #[serde(default)]
    pub short: Option<String>,
    /// Start of the stretch (edited microseconds). Default: the start.
    #[serde(default)]
    pub from_us: Option<u64>,
    /// End of the stretch (edited microseconds). Default: the end.
    #[serde(default)]
    pub to_us: Option<u64>,
    /// Tiles at most, 1 to 48. Default 24.
    #[serde(default)]
    pub max_tiles: Option<usize>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tile {
    pub n: usize,
    pub from: String,
    pub to: String,
    pub from_us: u64,
    pub to_us: u64,
    /// The moment the frame shows.
    pub at_us: u64,
    /// `screen`, `screen+cam` (the webcam bubble), `camera`, `split` (a short's camera and
    /// screen), `graphic` (full screen over the video) or `blank`.
    pub kind: &'static str,
    /// The zoom in the frame.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zoom: Option<f64>,
    /// Zooms starting within the tile.
    #[serde(skip_serializing_if = "is_zero")]
    pub zooms: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graphic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub say: String,
}

/// The sheet and where its tiles are, for the app to map clicks back to tiles.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    #[serde(skip)]
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub cols: u32,
    pub gap: u32,
    pub tile_width: u32,
    /// The frame and its caption strip.
    pub tile_height: u32,
    pub revision: u64,
    pub short: Option<String>,
    pub tiles: Vec<Tile>,
}

/// Splits `[from, to)` at `marks` (time, how firmly it holds), merging stretches shorter than
/// [`MIN_TILE_US`] and the weakest edges until at most `max_tiles` are left, then halving long
/// stretches while tiles are to spare.
pub(crate) fn split(from: u64, to: u64, marks: &[(u64, u8)], max_tiles: usize) -> Vec<(u64, u64)> {
    if to <= from {
        return Vec::new();
    }
    let max_tiles = max_tiles.max(1);
    let mut inner: Vec<(u64, u8)> = marks
        .iter()
        .copied()
        .filter(|&(t, _)| t > from && t < to)
        .collect();
    inner.sort_unstable();
    let mut points: Vec<(u64, u8)> = Vec::new();
    for (t, firm) in inner {
        match points.last_mut() {
            Some(last) if t - last.0 < SAME_MARK_US => last.1 = last.1.max(firm),
            _ => points.push((t, firm)),
        }
    }
    let edges = |points: &[(u64, u8)]| -> Vec<u64> {
        std::iter::once(from)
            .chain(points.iter().map(|p| p.0))
            .chain(std::iter::once(to))
            .collect()
    };
    while !points.is_empty() {
        let lens: Vec<u64> = edges(&points).windows(2).map(|w| w[1] - w[0]).collect();
        let too_short = |i: usize| lens[i] < MIN_TILE_US;
        let any_short = (0..lens.len()).any(too_short);
        if !any_short && lens.len() <= max_tiles {
            break;
        }
        // Edge `i` sits between stretches `i` and `i + 1`.
        let Some(weakest) = (0..points.len())
            .filter(|&i| !any_short || too_short(i) || too_short(i + 1))
            .min_by_key(|&i| (points[i].1, lens[i].min(lens[i + 1]), lens[i] + lens[i + 1]))
        else {
            break;
        };
        points.remove(weakest);
    }
    let mut tiles: Vec<(u64, u64)> = edges(&points).windows(2).map(|w| (w[0], w[1])).collect();
    while tiles.len() < max_tiles {
        let Some((i, &(a, b))) = tiles.iter().enumerate().max_by_key(|(_, r)| r.1 - r.0) else {
            break;
        };
        if b - a <= LONG_TILE_US {
            break;
        }
        let middle = a + (b - a) / 2;
        tiles[i] = (a, middle);
        tiles.insert(i + 1, (middle, b));
    }
    tiles
}

/// What an edit holds that a storyboard shows, on its timeline.
#[derive(Default)]
pub(crate) struct Facts {
    /// Stretches showing only the camera or only the screen, with that kind.
    pub scenes: Vec<(u64, u64, &'static str)>,
    pub zooms: Vec<(u64, u64, f64)>,
    pub graphics: Vec<(u64, u64, String)>,
    pub chapters: Vec<(u64, String)>,
    pub words: Vec<(u64, u64, String)>,
    /// A short's frame: split between camera and screen outside its scenes.
    pub vertical: bool,
}

impl Facts {
    fn gather(document: &EditDocument, root: &std::path::Path, stream: Option<&str>) -> Self {
        let mut facts = Facts {
            vertical: document.short_layout.is_some(),
            ..Default::default()
        };
        match &document.short_layout {
            Some(layout) => {
                facts.scenes = crate::short_scenes::edited_ranges(&layout.scenes, |media| {
                    document
                        .clock_asset(media)
                        .map(|asset| document.asset_clock(asset))
                })
                .into_iter()
                .map(|(a, b, mode)| {
                    let kind = match mode {
                        crate::short_scenes::SceneMode::Camera => "camera",
                        crate::short_scenes::SceneMode::Screen => "screen",
                    };
                    (a, b, kind)
                })
                .collect();
            }
            None if document.layout.webcam_enabled => {
                facts.scenes = document
                    .webcam_focus
                    .edited_ranges(&document.focus_clock())
                    .into_iter()
                    .map(|(a, b)| (a, b, "camera"))
                    .collect();
            }
            None => {}
        }
        for zoom in document.zooms_with_ranges() {
            // Cuts inside a zoom leave its pieces back to back: one zoom.
            let mut pieces: Vec<(u64, u64, f64)> = Vec::new();
            for range in &zoom.edited_ranges {
                match pieces.last_mut() {
                    Some(last) if range.start_us <= last.1 + 1_000 => {
                        last.1 = last.1.max(range.end_us)
                    }
                    _ => pieces.push((range.start_us, range.end_us, zoom.scale)),
                }
            }
            facts.zooms.extend(pieces);
        }
        facts.zooms.sort_by_key(|z| z.0);
        for track in document.sequence.video_tracks().filter(|t| !t.hidden) {
            for clip in &track.clips {
                if let Some(asset) = document.asset(&clip.asset).filter(|a| a.is_graphic()) {
                    facts
                        .graphics
                        .push((clip.start_us, clip.end_us(), asset.name.clone()));
                }
            }
        }
        facts.graphics.sort_by_key(|g| g.0);
        facts.chapters = crate::chapters::timeline(&document.chapters, document);
        if let Some(key) = stream {
            let transcript = crate::transcript::store::load_transcript(root, key)
                .ok()
                .flatten();
            if let (Some(transcript), Ok(clock)) = (transcript, document.transcript_clock(key)) {
                let view = crate::transcript::edit::view(&transcript, &clock, document.revision);
                facts.words = view
                    .words
                    .into_iter()
                    .filter_map(|w| {
                        Some((
                            w.edited_start_us?,
                            w.edited_end_us?,
                            w.word.text.trim().into(),
                        ))
                    })
                    .collect();
                facts.words.sort_by_key(|w| w.0);
            }
        }
        facts
    }

    /// Where the picture changes, and how firmly each change holds a tile edge.
    pub fn marks(&self) -> Vec<(u64, u8)> {
        let mut marks = Vec::new();
        for &(a, b, _) in &self.scenes {
            marks.extend([(a, BY_SCENE), (b, BY_SCENE)]);
        }
        for &(a, b, _) in &self.zooms {
            marks.extend([(a, BY_ZOOM), (b, BY_ZOOM_END)]);
        }
        for (a, b, _) in &self.graphics {
            marks.extend([(*a, BY_SCENE), (*b, BY_SCENE)]);
        }
        marks.extend(self.chapters.iter().map(|(at, _)| (*at, BY_SCENE)));
        for pair in self.words.windows(2) {
            let (end, next) = (pair[0].1, pair[1].0);
            if next.saturating_sub(end) >= LONG_GAP_US {
                marks.extend([(end, BY_GAP), (next, BY_GAP)]);
            }
        }
        if let Some(first) = self.words.first().filter(|w| w.0 >= LONG_GAP_US) {
            marks.push((first.0, BY_GAP));
        }
        if let Some(last) = self.words.last() {
            marks.push((last.1, BY_GAP));
        }
        marks
    }

    /// The moment a tile's frame shows: the middle of its graphic if it has one, else its
    /// middle.
    fn moment(&self, a: u64, b: u64) -> u64 {
        let graphic = self
            .graphics
            .iter()
            .map(|(c, d, _)| (a.max(*c), b.min(*d)))
            .filter(|(c, d)| c < d)
            .max_by_key(|(c, d)| d - c);
        let (c, d) = graphic.unwrap_or((a, b));
        c + (d - c) / 2
    }

    fn tile(&self, n: usize, (a, b): (u64, u64), at: u64, scene: Option<&Scene>) -> Tile {
        let inside = |c: u64, d: u64| c <= at && at < d;
        let shown = |role: LayerRole| {
            scene.map_or(0.0, |s| {
                s.layers
                    .iter()
                    .filter(|l| l.role == role)
                    .map(|l| l.width as f64 * l.height as f64)
                    .fold(0.0, f64::max)
                    / (s.width.max(1) as f64 * s.height.max(1) as f64)
            })
        };
        let graphic = self.graphics.iter().find(|g| inside(g.0, g.1));
        let kind = match self.scenes.iter().find(|s| inside(s.0, s.1)) {
            // On the video a graphic covers everything; in a short it may keep to a slot.
            _ if graphic.is_some() && !self.vertical => "graphic",
            Some(&(_, _, kind)) => kind,
            None => match (shown(LayerRole::Screen) > 0.0, shown(LayerRole::Webcam)) {
                (_, cam) if cam > 0.6 => "camera",
                (true, cam) if cam > 0.0 && self.vertical => "split",
                (true, cam) if cam > 0.0 => "screen+cam",
                (true, _) => "screen",
                (false, cam) if cam > 0.0 => "camera",
                (false, _) => "blank",
            },
        };
        let said: Vec<&str> = self
            .words
            .iter()
            .filter(|w| w.0 >= a && w.0 < b)
            .map(|w| w.2.as_str())
            .collect();
        Tile {
            n,
            from: clock(a),
            to: clock(b),
            from_us: a,
            to_us: b,
            at_us: at,
            kind,
            zoom: self
                .zooms
                .iter()
                .find(|z| inside(z.0, z.1))
                .map(|z| (z.2 * 100.0).round() / 100.0),
            zooms: self.zooms.iter().filter(|z| z.0 >= a && z.0 < b).count(),
            graphic: graphic.map(|g| g.2.clone()),
            chapter: self
                .chapters
                .iter()
                .find(|c| c.0 >= a && c.0 < b)
                .map(|c| c.1.clone()),
            say: shorten(&said.join(" "), SAY_CHARS),
        }
    }
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

fn shorten(text: &str, chars: usize) -> String {
    if text.chars().count() <= chars {
        return text.to_string();
    }
    let cut: String = text.chars().take(chars.saturating_sub(3)).collect();
    format!("{}...", cut.trim_end())
}

/// `text` cut to fit `width` pixels at the strip's size.
fn fit(text: &str, width: f32) -> String {
    if text_width(text, TEXT_PX) <= width {
        return text.to_string();
    }
    let mut chars: Vec<char> = text.chars().collect();
    while !chars.is_empty() {
        chars.pop();
        let candidate = format!("{}...", chars.iter().collect::<String>().trim_end());
        if text_width(&candidate, TEXT_PX) <= width {
            return candidate;
        }
    }
    String::new()
}

/// Tile width and the most columns for a canvas shape, keeping the sheet under 2048 pixels.
fn grid(canvas: (u32, u32)) -> (u32, u32) {
    if canvas.1 > canvas.0 {
        (176, 10)
    } else if canvas.1 == canvas.0 {
        (240, 8)
    } else {
        (320, 6)
    }
}

/// Builds the storyboard of the open project, or of one of its shorts.
pub fn build(
    state: &AppState,
    args: &StoryboardArgs,
    progress: &dyn Fn(f64, &str),
) -> Result<Board, String> {
    let (root, stream) = with_reader(state, |reader| {
        Ok((
            reader.root().to_path_buf(),
            super::tools::transcribed_stream(reader, None).ok(),
        ))
    })?;
    let max_tiles = args.max_tiles.unwrap_or(DEFAULT_TILES).clamp(1, MAX_TILES);
    let canvas = {
        let (document, _, _) = super::frames::document(state, args.short.as_deref())?;
        document.layout.preview_dimensions()?
    };
    let (tile_w, max_cols) = grid(canvas);
    super::frames::with_document_evaluator(
        state,
        args.short.as_deref(),
        tile_w,
        |evaluator, document| {
            let duration = document.duration_us();
            let from = args.from_us.unwrap_or(0).min(duration.saturating_sub(1));
            let to = args.to_us.unwrap_or(duration).min(duration);
            if to <= from {
                return Err("toUs must come after fromUs, inside the timeline".into());
            }
            let facts = Facts::gather(document, &root, stream.as_deref());
            let ranges = split(from, to, &facts.marks(), max_tiles);
            let mut frames = Vec::with_capacity(ranges.len());
            let mut tiles = Vec::with_capacity(ranges.len());
            for (i, &range) in ranges.iter().enumerate() {
                progress(
                    i as f64 / ranges.len() as f64,
                    &format!("Tile {} of {}", i + 1, ranges.len()),
                );
                let at = facts.moment(range.0, range.1).min(duration - 1);
                let frame = evaluator.preview_at(at)?;
                let scene = evaluator.scene_at(at).ok();
                tiles.push(facts.tile(i + 1, range, at, scene.as_ref()));
                frames.push(frame);
            }
            let tile_h = frames.first().map_or(2, |f| f.height);
            let cols = (tiles.len() as u32).clamp(1, max_cols);
            let rows = (tiles.len() as u32).div_ceil(cols).max(1);
            let cell_h = tile_h + STRIP_H;
            let width = GAP + cols * (tile_w + GAP);
            let height = GAP + rows * (cell_h + GAP);
            let mut image = RgbaImage::from_pixel(width, height, BACKGROUND);
            for (i, (tile, frame)) in tiles.iter().zip(&frames).enumerate() {
                let (col, row) = (i as u32 % cols, i as u32 / cols);
                let x = (GAP + col * (tile_w + GAP)) as i64;
                let y = (GAP + row * (cell_h + GAP)) as i64;
                blit_bgra(&mut image, frame, x, y);
                draw_strip(&mut image, tile, x, y + tile_h as i64, tile_w);
            }
            let mut png = std::io::Cursor::new(Vec::new());
            image
                .write_to(&mut png, image::ImageFormat::Png)
                .map_err(|e| format!("Could not encode the storyboard: {e}"))?;
            progress(1.0, "Storyboard ready");
            Ok(Board {
                png: png.into_inner(),
                width,
                height,
                cols,
                gap: GAP,
                tile_width: tile_w,
                tile_height: cell_h,
                revision: document.revision,
                short: args.short.clone(),
                tiles,
            })
        },
    )
}

/// The caption strip under a tile's frame: number, times and chapter; what the frame shows;
/// what is said.
fn draw_strip(image: &mut RgbaImage, tile: &Tile, x: i64, y: i64, width: u32) {
    let room = width as f32 - 8.0;
    let (x, top) = (x + 4, y + 3);
    let head = format!("#{} {}-{}", tile.n, tile.from, tile.to);
    draw_text(image, x, top, &fit(&head, room), TEXT_PX, TITLE);
    if let Some(chapter) = &tile.chapter {
        let used = text_width(&head, TEXT_PX) + 6.0;
        if room - used > 30.0 {
            let title = fit(chapter, room - used);
            draw_text(image, x + used as i64, top, &title, TEXT_PX, CHAPTER);
        }
    }
    let mut shows = tile.kind.to_string();
    if let Some(zoom) = tile.zoom {
        shows.push_str(&format!(" | zoom {zoom:.1}x"));
    }
    match tile.zooms {
        0 => {}
        1 if tile.zoom.is_some() => {}
        1 => shows.push_str(" | 1 zoom"),
        n => shows.push_str(&format!(" | {n} zooms")),
    }
    if let Some(graphic) = &tile.graphic {
        shows.push_str(&format!(" | gfx {graphic}"));
    }
    draw_text(image, x, top + LINE_H, &fit(&shows, room), TEXT_PX, ACCENT);
    let said = if tile.say.is_empty() {
        "(no speech)".to_string()
    } else {
        format!("\"{}\"", tile.say)
    };
    draw_text(image, x, top + 2 * LINE_H, &fit(&said, room), TEXT_PX, SAID);
    // A hairline between the frame and its strip.
    fill(
        image,
        x - 4,
        y,
        x - 4 + width as i64,
        y + 1,
        Rgba([60, 64, 72, 255]),
    );
}

pub fn storyboard(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: StoryboardArgs = params(args)?;
    let board = build(ctx.state, &args, &|fraction, message| {
        ctx.progress(fraction, message)
    })?;
    let what = match &board.short {
        Some(short) => format!("short {short} (9:16, its own timeline)"),
        None => "the video".into(),
    };
    let header = format!(
        "Storyboard of {what}, revision {}: {} tiles, {}x{}, read left to right in rows of {}. \
         Each tile's frame is at atUs; kind is what the frame shows.",
        board.revision,
        board.tiles.len(),
        board.width,
        board.height,
        board.cols
    );
    let tiles =
        serde_json::to_string(&json!({ "tiles": board.tiles })).map_err(|e| e.to_string())?;
    Ok(vec![
        Output::Png(board.png),
        Output::Text(format!("{header}\n{tiles}")),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: u64 = 1_000_000;

    #[test]
    fn scenes_split_at_changes() {
        let marks = [(10 * S, BY_SCENE), (20 * S, BY_ZOOM), (25 * S, BY_ZOOM_END)];
        assert_eq!(
            split(0, 40 * S, &marks, 24),
            vec![
                (0, 10 * S),
                (10 * S, 20 * S),
                (20 * S, 25 * S),
                (25 * S, 40 * S)
            ]
        );
    }

    #[test]
    fn short_stretches_merge_into_a_neighbour() {
        // A zoom of one second: its edges are weaker than the camera's, so they go.
        let marks = [
            (10 * S, BY_SCENE),
            (15 * S, BY_ZOOM),
            (16 * S, BY_ZOOM_END),
            (30 * S, BY_SCENE),
        ];
        let tiles = split(0, 40 * S, &marks, 24);
        assert!(tiles.iter().all(|(a, b)| b - a >= MIN_TILE_US), "{tiles:?}");
        assert!(tiles.iter().any(|&(a, _)| a == 10 * S));
        assert!(tiles.iter().any(|&(a, _)| a == 30 * S));
        // Changes a moment apart count once.
        assert_eq!(
            split(
                0,
                10 * S,
                &[(5 * S, BY_ZOOM), (5 * S + 100_000, BY_SCENE)],
                24
            ),
            vec![(0, 5 * S), (5 * S, 10 * S)]
        );
    }

    #[test]
    fn the_cap_drops_the_weakest_edges_first() {
        let mut marks: Vec<(u64, u8)> = (1..60).map(|i| (i * 5 * S, BY_ZOOM)).collect();
        marks.push((100 * S + 2 * S, BY_SCENE));
        marks.push((200 * S + 2 * S, BY_SCENE));
        let tiles = split(0, 300 * S, &marks, 8);
        assert_eq!(tiles.len(), 8);
        assert_eq!(tiles.first().unwrap().0, 0);
        assert_eq!(tiles.last().unwrap().1, 300 * S);
        assert!(tiles.windows(2).all(|w| w[0].1 == w[1].0));
        for scene in [102 * S, 202 * S] {
            assert!(tiles.iter().any(|&(a, _)| a == scene), "{tiles:?}");
        }
    }

    #[test]
    fn long_takes_are_halved_while_tiles_are_to_spare() {
        assert_eq!(split(0, 100 * S, &[], 1), vec![(0, 100 * S)]);
        let tiles = split(0, 100 * S, &[], 24);
        assert_eq!(tiles.len(), 4);
        assert!(tiles.iter().all(|(a, b)| b - a <= LONG_TILE_US));
        // A range shorter than a tile is still one tile.
        assert_eq!(split(0, S, &[(S / 2, BY_SCENE)], 24), vec![(0, S)]);
        assert!(split(5, 5, &[], 24).is_empty());
    }

    #[test]
    fn marks_come_from_scenes_zooms_graphics_chapters_and_long_silences() {
        let facts = Facts {
            scenes: vec![(10 * S, 20 * S, "camera")],
            zooms: vec![(30 * S, 34 * S, 2.0)],
            graphics: vec![(40 * S, 45 * S, "Title".into())],
            chapters: vec![(0, "Intro".into()), (50 * S, "Setup".into())],
            words: vec![
                (S, 2 * S, "Hello".into()),
                (2 * S, 3 * S, "there.".into()),
                (8 * S, 9 * S, "Now".into()),
            ],
            vertical: false,
        };
        let marks = facts.marks();
        for t in [10, 20, 30, 34, 40, 45, 50, 3, 8, 9] {
            assert!(marks.iter().any(|m| m.0 == t * S), "no mark at {t}s");
        }
        let tile = facts.tile(1, (40 * S, 50 * S), facts.moment(40 * S, 50 * S), None);
        assert_eq!(tile.at_us, 42 * S + S / 2);
        assert_eq!(tile.graphic.as_deref(), Some("Title"));
        assert_eq!(tile.chapter, None);
        assert_eq!(tile.kind, "graphic");
        assert_eq!(facts.tile(1, (45 * S, 50 * S), 47 * S, None).kind, "blank");
        assert_eq!(facts.tile(1, (20 * S, 40 * S), 25 * S, None).zooms, 1);
        let opening = facts.tile(1, (0, 10 * S), 5 * S, None);
        assert_eq!(opening.say, "Hello there. Now");
        assert_eq!(opening.chapter.as_deref(), Some("Intro"));
        let camera = facts.tile(2, (10 * S, 20 * S), 15 * S, None);
        assert_eq!(camera.kind, "camera");
        assert_eq!(
            facts.tile(3, (30 * S, 34 * S), 32 * S, None).zoom,
            Some(2.0)
        );
    }

    #[test]
    fn long_text_is_shortened() {
        assert_eq!(shorten("short", 10), "short");
        assert_eq!(shorten("a long line of speech", 10), "a long...");
        assert!(text_width(&fit(&"word ".repeat(40), 150.0), TEXT_PX) <= 150.0);
    }

    #[test]
    fn a_lesson_makes_a_sheet_with_a_tile_per_scene() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = crate::mcp::tests_support::lesson(dir.path());
        let board = build(&host.state, &StoryboardArgs::default(), &|_, _| {}).unwrap();
        assert!(!board.tiles.is_empty());
        let image = image::load_from_memory(&board.png).unwrap();
        assert_eq!((image.width(), image.height()), (board.width, board.height));
        assert!(board.width <= 2048);
        assert_eq!(board.tiles[0].from_us, 0);
        let ctx = Ctx::new(&host, "agent", false);
        let output = storyboard(&ctx, json!({ "maxTiles": 2 })).unwrap();
        let Output::Text(text) = &output[1] else {
            panic!("expected text")
        };
        assert!(text.contains("\"tiles\""), "{text}");
    }
}
