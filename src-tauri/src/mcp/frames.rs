//! Finished frames for an agent to look at, drawn the way export draws them.
use crate::commands::AppState;
use crate::export::SceneEvaluator;
use crate::media::{ffmpeg::DecodeLimit, PixelFormat, VideoFrame};
use crate::project::revision::EditDocument;
use crate::render::Scene;
use parking_lot::Mutex;

/// The renderer for the last project revision, transcripts and size asked for: an agent
/// usually looks at several moments of the same edit in a row.
static CACHE: Mutex<Option<(String, SceneEvaluator)>> = Mutex::new(None);

thread_local! {
    /// A graphic version drawn in place of the one of its family that plays (GFX-4), while a
    /// frame tool looks at it before it is used.
    static TRYING: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// Runs `look` with frames drawn as if graphic `version` played where its family plays.
pub fn trying<T>(version: &str, look: impl FnOnce() -> T) -> T {
    let before = TRYING.with(|t| t.replace(Some(version.to_string())));
    let out = look();
    TRYING.with(|t| *t.borrow_mut() = before);
    out
}

/// The document frames are drawn from: the open project's current edit, or with `short` (an
/// id or title) that short's 9:16 one, on its own timeline. With the project root and a key
/// that changes whenever the picture may.
pub fn document(
    state: &AppState,
    short: Option<&str>,
) -> Result<(EditDocument, std::path::PathBuf, String), String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or(super::tools::NO_PROJECT_MESSAGE)?;
    let base = &reader.history().current;
    // Captions come from transcripts, which change without moving the edit revision.
    let mut key = format!(
        "{}:{}:{}",
        reader.summary.project_handle,
        base.revision,
        crate::transcript::store::changes()
    );
    let swapped;
    let base = match TRYING.with(|t| t.borrow().clone()) {
        Some(version) => {
            key.push_str(&format!(":trying {version}"));
            swapped = crate::graphics::versions::swap(base, reader.root(), &version);
            &swapped
        }
        None => base,
    };
    let document = match short {
        None => base.clone(),
        Some(wanted) => {
            let short = base
                .shorts
                .iter()
                .find(|s| s.id == wanted)
                .or_else(|| {
                    base.shorts
                        .iter()
                        .find(|s| s.title.eq_ignore_ascii_case(wanted.trim()))
                })
                .ok_or_else(|| {
                    let known: Vec<String> = base
                        .shorts
                        .iter()
                        .map(|s| format!("{} \"{}\"", s.id, s.title))
                        .collect();
                    format!("No short {wanted}. Shorts: {}", known.join(", "))
                })?;
            key.push_str(&format!(":{}", short.id));
            crate::shorts::short_document(
                base,
                short,
                crate::commands::has_speech_transcript(reader),
            )?
        }
    };
    Ok((document, reader.root().to_path_buf(), key))
}

/// Renders, and waits for, the graphics that play between `from_us` and `to_us` whose render is
/// missing or older than their source (a fix just made): frames drawn after it show them as
/// they are now, not the last render nor a placeholder.
pub fn wait_for_graphics(
    state: &AppState,
    short: Option<&str>,
    from_us: u64,
    to_us: u64,
) -> Result<(), String> {
    let (document, root, _) = document(state, short)?;
    let playing: std::collections::HashSet<&str> = document
        .sequence
        .video_tracks()
        .flat_map(|t| &t.clips)
        .filter(|c| c.start_us <= to_us && c.end_us() > from_us)
        .map(|c| c.asset.as_str())
        .collect();
    let canvas = document.layout.preview_dimensions()?;
    let specs: Vec<_> =
        crate::graphics::render::specs_for(&root, &document, canvas, Default::default())
            .into_iter()
            .filter(|s| playing.contains(s.asset_id.as_str()))
            .collect();
    if specs.is_empty() {
        return Ok(());
    }
    crate::graphics::render::ensure(
        &specs,
        &std::sync::atomic::AtomicBool::new(false),
        &mut |_, _, _| {},
    )
    .map(|_| ())
}

/// Runs `use_it` on an evaluator of [`document`] at `width` (the height follows the canvas
/// shape), with the document it draws.
pub fn with_document_evaluator<T>(
    state: &AppState,
    short: Option<&str>,
    width: u32,
    use_it: impl FnOnce(&mut SceneEvaluator, &EditDocument) -> Result<T, String>,
) -> Result<T, String> {
    let (document, root, key) = document(state, short)?;
    let canvas = document.layout.preview_dimensions()?;
    let height = ((canvas.1 as u64 * width as u64 / canvas.0.max(1) as u64) as u32 & !1).max(2);
    let width = width & !1;
    if document.sequence.duration_us() == 0 {
        return Err("The timeline is empty".into());
    }
    let key = format!("{key}:{width}x{height}");
    let mut cache = CACHE.lock();
    if cache.as_ref().map(|(k, _)| k.as_str()) != Some(key.as_str()) {
        let reuse = cache.take().and_then(|(_, e)| e.into_reuse());
        let evaluator = SceneEvaluator::new_reusing(root, document.clone(), width, height, reuse)?
            .with_decode_limit(DecodeLimit {
                max_width: width * 2,
                max_height: height * 2,
                max_rate: 0,
                interactive: true,
                yuv: false,
            });
        *cache = Some((key, evaluator));
    }
    let evaluator = &mut cache.as_mut().ok_or("The renderer is not ready")?.1;
    use_it(evaluator, &document)
}

/// Runs `use_it` on an evaluator of the open project's current edit (or `short`'s) at `width`
/// (the height follows the canvas shape), with `at_us` clamped into the edit. Returns its
/// result, the moment used, and the full canvas size.
fn with_evaluator<T>(
    state: &AppState,
    short: Option<&str>,
    at_us: u64,
    width: u32,
    use_it: impl FnOnce(&mut SceneEvaluator, u64) -> Result<T, String>,
) -> Result<(T, u64, (u32, u32)), String> {
    with_document_evaluator(state, short, width, |evaluator, document| {
        let at = at_us.min(document.sequence.duration_us() - 1);
        let canvas = document.layout.preview_dimensions()?;
        Ok((use_it(evaluator, at)?, at, canvas))
    })
}

/// The finished frame at `at_us`, `width` pixels wide: of the edit, or of `short` (an id or
/// title) on its own timeline.
pub fn render(
    state: &AppState,
    short: Option<&str>,
    at_us: u64,
    width: u32,
) -> Result<(VideoFrame, u64), String> {
    let (frame, at, _) = with_evaluator(state, short, at_us, width, |e, at| e.preview_at(at))?;
    Ok((frame, at))
}

/// How much of the edit a sample clip shows either side of what it is about.
pub const SAMPLE_MARGIN_US: u64 = 1_500_000;

/// A playable clip of the edit (or of `short`) from `from_us` to `to_us`, drawn as the export
/// draws it, at 720p with its sound: for the user to watch a graphic in place (the agent looks
/// at frames). It goes in a "AeroEdits samples" folder in the system's temporary folder, as
/// an export can't write inside the project. Returns its path.
pub fn sample_clip(
    state: &AppState,
    short: Option<&str>,
    from_us: u64,
    to_us: u64,
    label: &str,
) -> Result<std::path::PathBuf, String> {
    let (mut document, root, _) = document(state, short)?;
    let length = document.duration_us();
    let (from, to) = (from_us.min(length), to_us.min(length));
    if to <= from {
        return Err("Nothing to show there".into());
    }
    let ranges: Vec<crate::zoom::EditedRange> = [(0, from), (to, length)]
        .into_iter()
        .filter(|(a, b)| b > a)
        .map(|(start_us, end_us)| crate::zoom::EditedRange { start_us, end_us })
        .collect();
    if !ranges.is_empty() {
        let (sequence, _) = crate::sequence::edit::apply(
            &document.sequence,
            &document.assets,
            &crate::sequence::edit::SequenceEdit::DeleteRange {
                ranges,
                ripple: Some(true),
            },
        )?;
        document.sequence = sequence;
    }
    let folder = std::env::temp_dir().join("AeroEdits samples");
    std::fs::create_dir_all(&folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let stem: String = label
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .take(60)
        .collect();
    let dest = folder.join(format!(
        "{} {}.mp4",
        stem.trim(),
        &uuid::Uuid::new_v4().simple().to_string()[..6]
    ));
    let settings = crate::export::ExportSettings {
        width: 1280,
        height: 720,
        destination: Some(dest.display().to_string()),
        ..Default::default()
    };
    let mut owner = crate::export::ExportOwner::new();
    let captured = crate::export::prepare_job(&root, label, document, settings, &mut owner)
        .map_err(|status| {
            status
                .failure
                .map(|f| f.message().to_string())
                .unwrap_or_else(|| "The clip could not start".into())
        })?;
    crate::export::run_export(
        &captured,
        &std::sync::atomic::AtomicBool::new(false),
        |_, _| {},
        &state.encoder_gate,
    )
    .map_err(|failure| format!("The clip failed: {}", failure.message()))
}

/// The layers at `at_us`, at a small size (only their places are wanted), with the full
/// canvas size to scale them to.
pub fn scene(state: &AppState, at_us: u64) -> Result<(Scene, u64, (u32, u32)), String> {
    with_evaluator(state, None, at_us, 480, |e, at| e.scene_at(at))
}

pub fn png(frame: &VideoFrame) -> Result<Vec<u8>, String> {
    if frame.format != PixelFormat::Bgra8888 {
        return Err("Only BGRA frames can be saved as PNG".into());
    }
    let (w, h) = (frame.width as usize, frame.height as usize);
    let mut rgba = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        let row = frame
            .data
            .get(y * frame.stride as usize..y * frame.stride as usize + w * 4)
            .ok_or("The frame buffer is truncated")?;
        for px in row.chunks_exact(4) {
            rgba.extend_from_slice(&[px[2], px[1], px[0], 255]);
        }
    }
    let image = image::RgbaImage::from_raw(frame.width, frame.height, rgba)
        .ok_or("The frame has the wrong size")?;
    let mut out = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| format!("Could not encode the frame: {e}"))?;
    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_swaps_bgra_to_rgba_and_honours_the_stride() {
        let mut frame = VideoFrame::solid(3, 2, 10, 20, 30, 255).unwrap();
        frame.stride = 3 * 4;
        let png = png(&frame).unwrap();
        let decoded = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(decoded.dimensions(), (3, 2));
        assert_eq!(decoded.get_pixel(2, 1).0, [30, 20, 10, 255]);
    }

    #[test]
    fn a_sample_clip_is_the_stretch_asked_for_with_its_sound() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = crate::mcp::tests_support::lesson(dir.path());
        let path = sample_clip(&host.state, None, 1_000_000, 2_500_000, "Step: one/two").unwrap();
        assert!(path.starts_with(std::env::temp_dir().join("AeroEdits samples")));
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        assert!(
            name.starts_with("Step_ one_two ") && name.ends_with(".mp4"),
            "{name}"
        );
        let length = crate::media::ffmpeg::duration_us(&path).unwrap();
        assert!((1_400_000..=1_650_000).contains(&length), "{length}");
        assert!(!crate::media::ffmpeg::audio_stream_names(&path)
            .unwrap()
            .is_empty());
        let _ = std::fs::remove_file(path);
        assert!(sample_clip(&host.state, None, 9_000_000, 9_500_000, "x").is_err());
    }

    #[test]
    fn a_style_trial_is_a_clip_of_the_moments_first_to_last() {
        use crate::mcp::tools::{find, Ctx, Output};
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, _) = crate::mcp::tests_support::lesson(dir.path());
        let ctx = Ctx::new(&host, "agent", false);
        let run = |args| (find("frame_render").unwrap().run)(&ctx, args);
        let out =
            run(serde_json::json!({ "moments": [500_000, 3_600_000], "sample": true })).unwrap();
        let Some(Output::Text(text)) = out.get(1) else {
            panic!("no text");
        };
        assert!(text.contains("(0:00.50 to 0:03.60, with sound)"), "{text}");
        let path = text
            .split(": ")
            .last()
            .unwrap()
            .split(". Look")
            .next()
            .unwrap();
        let length = crate::media::ffmpeg::duration_us(std::path::Path::new(path)).unwrap();
        assert!((2_900_000..=3_300_000).contains(&length), "{length}");
        let _ = std::fs::remove_file(path);
        // A trial is a few beats, not the edit.
        let long = run(serde_json::json!({ "moments": [0, 25_000_000], "sample": true }));
        assert!(long.unwrap_err().contains("at most 20 s"));
        assert!(run(serde_json::json!({ "atUs": 0, "sample": true })).is_err());
    }

    #[test]
    fn caption_places_follow_transcript_edits_at_the_same_revision() {
        use crate::commands::{self, transcript::CaptionEdit};
        use crate::mcp::tools::{find, Ctx, Output};
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, key) = crate::mcp::tests_support::lesson(dir.path());
        let opened = host
            .state
            .opened_project
            .lock()
            .as_ref()
            .unwrap()
            .summary
            .clone();
        let captions = crate::captions::CaptionSettings {
            enabled: true,
            ..Default::default()
        };
        let opened = commands::project_captions_update_impl(
            &host.state,
            opened.project_handle,
            opened.revision,
            captions,
        )
        .unwrap();
        let ctx = Ctx::new(&host, "agent", false);
        let captions_at_start = || {
            let output =
                (find("layout_get").unwrap().run)(&ctx, serde_json::json!({ "atUs": 100_000 }))
                    .unwrap();
            let Output::Text(text) = &output[0] else {
                panic!("expected text")
            };
            let layout: serde_json::Value = serde_json::from_str(text).unwrap();
            layout["captions"].as_array().unwrap().len()
        };
        assert_eq!(captions_at_start(), 1);
        let hidden = commands::transcript::transcript_caption_edit_impl(
            &host.state,
            opened.project_handle.clone(),
            key,
            CaptionEdit::Hide {
                word_ids: (0..7).map(|i| format!("w-{i}")).collect(),
                hidden: true,
            },
            None,
        )
        .unwrap();
        assert!(hidden.cues.is_empty());
        // Hiding words is not an edit step, yet the agent sees the captions gone.
        let revision = host
            .state
            .opened_project
            .lock()
            .as_ref()
            .unwrap()
            .document()
            .revision;
        assert_eq!(revision, opened.revision);
        assert_eq!(captions_at_start(), 0);
    }
}
