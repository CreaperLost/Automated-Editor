//! Thumbnails on timeline clips (UI-1): tiny pictures of a clip's source, kept on disk per
//! source in `cache/thumbs/<asset>/`, so a timeline that comes back draws them without
//! decoding anything.
use super::{require_handle, AppState};
use crate::project::reader::safe_path;
use crate::sequence::sources::{stream_source, StreamSource};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// The tallest thumbnail made: tracks are never taller.
const MAX_HEIGHT: u32 = 180;
/// Times one call may ask for; the timeline asks for what is in view.
const MAX_TIMES: usize = 64;
/// Bump when thumbnails are made differently, so old ones are not used.
const VERSION: u32 = 1;

/// Null is genuine absence; a frame error stays retryable without losing its batch's pictures.
#[derive(Clone, serde::Serialize)]
#[serde(untagged)]
pub enum Thumbnail {
    Picture(String),
    Absent,
    Failed { error: String },
}

impl From<Result<Option<Vec<u8>>, String>> for Thumbnail {
    fn from(result: Result<Option<Vec<u8>>, String>) -> Self {
        use base64::Engine;
        match result {
            Ok(Some(jpeg)) => Self::Picture(format!(
                "data:image/jpeg;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(jpeg)
            )),
            Ok(None) => Self::Absent,
            Err(error) => Self::Failed { error },
        }
    }
}

/// Each time's thumbnail (`times_us` on the asset's own clock), absence, or retryable failure.
pub fn project_clip_thumbnails_impl(
    state: &AppState,
    project_handle: String,
    asset_id: String,
    stream: String,
    times_us: Vec<u64>,
    height: u32,
) -> Result<Vec<Thumbnail>, String> {
    let (root, asset) = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        require_handle(reader, &project_handle)?;
        let asset = reader
            .document()
            .asset(&asset_id)
            .cloned()
            .ok_or_else(|| format!("No asset {asset_id}"))?;
        (reader.root().to_path_buf(), asset)
    };
    if asset.missing {
        return Ok(vec![Thumbnail::Absent; times_us.len().min(MAX_TIMES)]);
    }
    let source = stream_source(&root, &asset, &stream)?;
    let height = height.clamp(16, MAX_HEIGHT);
    Ok(times_us
        .iter()
        .take(MAX_TIMES)
        .map(|&at| Thumbnail::from(thumbnail(&root, &asset.id, &source, at, height)))
        .collect())
}

/// The thumbnail at `at_us` of `source`, from the cache or made now.
fn thumbnail(
    root: &Path,
    asset_id: &str,
    source: &StreamSource,
    at_us: u64,
    height: u32,
) -> Result<Option<Vec<u8>>, String> {
    let (file, local_us) = match source {
        StreamSource::Segments(segments) => {
            // In a gap (before the camera started, a pause): the next picture there is.
            match segments
                .iter()
                .filter(|s| s.available && s.end_us > at_us)
                .min_by_key(|s| s.start_us.max(at_us))
            {
                Some(segment) => (
                    safe_path(root, &segment.relative_path)?,
                    at_us.saturating_sub(segment.start_us),
                ),
                None => return Ok(None),
            }
        }
        StreamSource::Video(path) => (path.clone(), at_us),
        StreamSource::Image(path) => (path.clone(), 0),
        StreamSource::Graphic(_) => return Ok(None),
    };
    let meta = fs::metadata(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis());
    let key = format!(
        "v{VERSION}\n{}\n{}\n{modified}\n{local_us}\n{height}",
        file.display(),
        meta.len()
    );
    let name: String = Sha256::digest(key.as_bytes())[..12]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let folder: String = asset_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    let cached = safe_path(root, &format!("cache/thumbs/{folder}/{name}.jpg"))?;
    if let Ok(bytes) = fs::read(&cached) {
        if !bytes.is_empty() {
            return Ok(Some(bytes));
        }
    }
    let jpeg = match source {
        StreamSource::Image(_) => jpeg(
            &crate::media_bin::decode_image_cached(&file, (0, height))?,
            height,
        )?,
        _ if crate::media::ffmpeg::available() => {
            crate::media::ffmpeg::thumbnail_jpeg(&file, local_us, height)?
        }
        _ => jpeg(&crate::media::decode_h264_frame(&file, local_us)?, height)?,
    };
    // Not kept is only slower next time.
    if let Some(parent) = cached.parent() {
        let _ = fs::create_dir_all(parent).and_then(|_| fs::write(&cached, &jpeg));
    }
    Ok(Some(jpeg))
}

/// A BGRA frame as a JPEG no taller than `height`.
fn jpeg(frame: &crate::media::VideoFrame, height: u32) -> Result<Vec<u8>, String> {
    if frame.format != crate::media::PixelFormat::Bgra8888 {
        return Err("Only BGRA frames make thumbnails".into());
    }
    let (w, h) = (frame.width as usize, frame.height as usize);
    let mut rgb = Vec::with_capacity(w * h * 3);
    for y in 0..h {
        let start = y * frame.stride as usize;
        let row = frame
            .data
            .get(start..start + w * 4)
            .ok_or("The frame buffer is truncated")?;
        for px in row.chunks_exact(4) {
            rgb.extend_from_slice(&[px[2], px[1], px[0]]);
        }
    }
    let mut image = image::RgbImage::from_raw(frame.width, frame.height, rgb)
        .ok_or("The frame has the wrong size")?;
    if frame.height > height {
        let width = ((frame.width as u64 * height as u64) / frame.height as u64).max(1) as u32;
        image = image::imageops::thumbnail(&image, width, height);
    }
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 70)
        .encode_image(&image)
        .map_err(|e| format!("JPEG: {e}"))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_failures_are_distinct_from_absence_and_keep_successful_pictures() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("picture.png");
        image::RgbImage::new(32, 32).save(&path).unwrap();
        let sources = [
            StreamSource::Image(path),
            StreamSource::Video(root.path().join("temporarily-missing.mp4")),
            StreamSource::Segments(vec![]),
        ];
        let replies: Vec<_> = sources
            .iter()
            .map(|source| Thumbnail::from(thumbnail(root.path(), "asset", source, 0, 32)))
            .collect();
        let json = serde_json::to_value(replies).unwrap();
        assert!(json[0]
            .as_str()
            .unwrap()
            .starts_with("data:image/jpeg;base64,"));
        assert!(json[1]["error"]
            .as_str()
            .unwrap()
            .contains("temporarily-missing"));
        assert!(json[2].is_null());
    }

    #[test]
    fn a_frame_becomes_a_small_jpeg() {
        let frame = crate::media::VideoFrame::solid(640, 360, 200, 40, 10, 255).unwrap();
        let bytes = jpeg(&frame, 180).unwrap();
        let decoded = image::load_from_memory(&bytes).unwrap().to_rgb8();
        assert_eq!(decoded.dimensions(), (320, 180));
    }
}
