//! Finished frames for an agent to look at, drawn the way export draws them.
use crate::commands::AppState;
use crate::export::SceneEvaluator;
use crate::media::{ffmpeg::DecodeLimit, PixelFormat, VideoFrame};
use crate::render::Scene;
use parking_lot::Mutex;

/// The renderer for the last project revision, transcripts and size asked for: an agent
/// usually looks at several moments of the same edit in a row.
static CACHE: Mutex<Option<(String, SceneEvaluator)>> = Mutex::new(None);

/// Runs `use_it` on an evaluator of the open project's current edit at `width` (the height
/// follows the canvas shape), with `at_us` clamped into the edit. Returns its result, the
/// moment used, and the full canvas size.
fn with_evaluator<T>(
    state: &AppState,
    at_us: u64,
    width: u32,
    use_it: impl FnOnce(&mut SceneEvaluator, u64) -> Result<T, String>,
) -> Result<(T, u64, (u32, u32)), String> {
    let (document, root, handle) = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or(super::tools::NO_PROJECT_MESSAGE)?;
        let document = reader.history().current.clone();
        (
            document,
            reader.root().to_path_buf(),
            reader.summary.project_handle.clone(),
        )
    };
    let canvas = document.layout.preview_dimensions()?;
    let height = ((canvas.1 as u64 * width as u64 / canvas.0.max(1) as u64) as u32 & !1).max(2);
    let width = width & !1;
    let duration = document.sequence.duration_us();
    if duration == 0 {
        return Err("The timeline is empty".into());
    }
    let at = at_us.min(duration - 1);
    // Captions come from transcripts, which change without moving the edit revision.
    let key = format!(
        "{handle}:{}:{}:{width}x{height}",
        document.revision,
        crate::transcript::store::changes()
    );
    let mut cache = CACHE.lock();
    if cache.as_ref().map(|(k, _)| k.as_str()) != Some(key.as_str()) {
        let reuse = cache.take().and_then(|(_, e)| e.into_reuse());
        let evaluator = SceneEvaluator::new_reusing(root, document, width, height, reuse)?
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
    Ok((use_it(evaluator, at)?, at, canvas))
}

/// The finished frame at `at_us`, `width` pixels wide.
pub fn render(state: &AppState, at_us: u64, width: u32) -> Result<(VideoFrame, u64), String> {
    let (frame, at, _) = with_evaluator(state, at_us, width, |e, at| e.preview_at(at))?;
    Ok((frame, at))
}

/// The layers at `at_us`, at a small size (only their places are wanted), with the full
/// canvas size to scale them to.
pub fn scene(state: &AppState, at_us: u64) -> Result<(Scene, u64, (u32, u32)), String> {
    with_evaluator(state, at_us, 480, |e, at| e.scene_at(at))
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
    fn caption_places_follow_transcript_edits_at_the_same_revision() {
        use crate::commands::{self, transcript::CaptionEdit};
        use crate::mcp::tools::{find, Ctx, Output};
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, key) = crate::mcp::tests_support::lesson(dir.path());
        let opened = host.state.opened_project.lock().as_ref().unwrap().summary.clone();
        let captions = crate::captions::CaptionSettings {
            enabled: true,
            ..Default::default()
        };
        let opened =
            commands::project_captions_update_impl(&host.state, opened.project_handle, opened.revision, captions)
                .unwrap();
        let ctx = Ctx::new(&host, "agent", false);
        let captions_at_start = || {
            let output = (find("layout_get").unwrap().run)(&ctx, serde_json::json!({ "atUs": 100_000 })).unwrap();
            let Output::Text(text) = &output[0] else { panic!("expected text") };
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
        let revision = host.state.opened_project.lock().as_ref().unwrap().document().revision;
        assert_eq!(revision, opened.revision);
        assert_eq!(captions_at_start(), 0);
    }
}
