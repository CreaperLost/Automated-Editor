//! Imported media: videos, images and audio copied into the project and placed on the
//! timeline between (or instead of) parts of the recording.
//!
//! Import copies the file into `assets/media/`, probes it, and extracts any audio once to a
//! 48 kHz stereo WAV next to it, so playback and export mix it like recorded audio. Video
//! frames always decode through FFmpeg (whatever the platform's recording backend), so any
//! format FFmpeg reads works.
use crate::media::{ColorInfo, PixelFormat, VideoFrame, MAX_FRAME_DIM};
use crate::project::reader::safe_path;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const MAX_MEDIA_ASSETS: usize = 512;
/// How long an image plays when first placed on the timeline.
pub const IMAGE_CLIP_US: u64 = 5_000_000;
/// How long an image can be stretched to.
pub const IMAGE_MAX_US: u64 = 3_600_000_000;
const MEDIA_DIR: &str = "assets/media";

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mov", "m4v", "mkv", "webm", "avi"];
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg"];
const AUDIO_EXTENSIONS: &[&str] = &["wav", "mp3", "m4a", "aac", "flac", "ogg"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Video,
    Image,
    Audio,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaAsset {
    pub id: String,
    /// The original file name, for display.
    pub name: String,
    pub kind: MediaKind,
    /// The copy inside the project, relative to its root.
    pub relative_path: String,
    /// Extracted 48 kHz stereo WAV, relative to the root, when the file has audio.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_path: Option<String>,
    /// Playable length; for images, how far a clip can be stretched.
    pub duration_us: u64,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
}

impl MediaAsset {
    /// The length a clip of this asset starts with on the timeline.
    pub fn default_clip_us(&self) -> u64 {
        match self.kind {
            MediaKind::Image => IMAGE_CLIP_US,
            _ => self.duration_us,
        }
    }
}

pub fn kind_for(path: &Path) -> Option<MediaKind> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let ext = ext.as_str();
    if VIDEO_EXTENSIONS.contains(&ext) {
        Some(MediaKind::Video)
    } else if IMAGE_EXTENSIONS.contains(&ext) {
        Some(MediaKind::Image)
    } else if AUDIO_EXTENSIONS.contains(&ext) {
        Some(MediaKind::Audio)
    } else {
        None
    }
}

/// File-dialog filter: every extension import accepts.
pub fn import_extensions() -> Vec<&'static str> {
    VIDEO_EXTENSIONS
        .iter()
        .chain(IMAGE_EXTENSIONS)
        .chain(AUDIO_EXTENSIONS)
        .copied()
        .collect()
}

pub fn validate_assets(assets: &[MediaAsset]) -> Result<(), String> {
    if assets.len() > MAX_MEDIA_ASSETS {
        return Err("Too many imported media files".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    for asset in assets {
        if asset.id.is_empty() || asset.id.len() > 128 || !seen.insert(&asset.id) {
            return Err("Invalid imported media id".into());
        }
        let in_media_dir = |path: &str| {
            path.starts_with(&format!("{MEDIA_DIR}/"))
                && !path.contains("..")
                && !path.contains('\\')
        };
        if !in_media_dir(&asset.relative_path)
            || asset
                .audio_path
                .as_deref()
                .is_some_and(|p| !in_media_dir(p))
        {
            return Err("Imported media must live in assets/media".into());
        }
        if asset.duration_us == 0 {
            return Err("Imported media has no duration".into());
        }
    }
    Ok(())
}

/// Copies `source` into the project and describes it. The caller records the asset in the
/// edit document.
pub fn import(root: &Path, source: &Path) -> Result<MediaAsset, String> {
    let kind = kind_for(source).ok_or_else(|| {
        format!(
            "{} is not a supported video, image or audio file",
            source.display()
        )
    })?;
    let meta = fs::metadata(source).map_err(|e| format!("{}: {e}", source.display()))?;
    if !meta.is_file() {
        return Err(format!("{} is not a file", source.display()));
    }
    let name = source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "media".into());
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin")
        .to_ascii_lowercase();
    let id = format!("m-{}", &uuid::Uuid::new_v4().simple().to_string()[..12]);
    let relative_path = format!("{MEDIA_DIR}/{id}.{ext}");
    let target = safe_path(root, &relative_path)?;
    fs::create_dir_all(target.parent().ok_or("Bad media path")?)
        .map_err(|e| format!("Could not create {MEDIA_DIR}: {e}"))?;
    fs::copy(source, &target).map_err(|e| format!("Could not copy {name}: {e}"))?;
    let described = describe(root, &id, &target, kind);
    if described.is_err() {
        let _ = fs::remove_file(&target);
    }
    let (duration_us, width, height, audio_path) = described?;
    Ok(MediaAsset {
        id,
        name,
        kind,
        relative_path,
        audio_path,
        duration_us,
        width,
        height,
    })
}

type Described = (u64, u32, u32, Option<String>);

fn describe(root: &Path, id: &str, file: &Path, kind: MediaKind) -> Result<Described, String> {
    use crate::media::ffmpeg;
    match kind {
        MediaKind::Image => {
            let (width, height) =
                image::image_dimensions(file).map_err(|e| format!("Unreadable image: {e}"))?;
            Ok((IMAGE_MAX_US, width, height, None))
        }
        MediaKind::Video | MediaKind::Audio => {
            let duration_us = ffmpeg::duration_us(file)?;
            if duration_us == 0 {
                return Err("The file has no playable length".into());
            }
            let (width, height) = if kind == MediaKind::Video {
                let info = ffmpeg::probe_video(file)?;
                (info.width, info.height)
            } else {
                (0, 0)
            };
            let audio_path = if ffmpeg::has_audio_stream(file)? {
                let relative = format!("{MEDIA_DIR}/{id}.audio.wav");
                ffmpeg::extract_audio_wav(file, &safe_path(root, &relative)?)?;
                Some(relative)
            } else if kind == MediaKind::Audio {
                return Err("The file has no audio stream".into());
            } else {
                None
            };
            Ok((duration_us, width, height, audio_path))
        }
    }
}

/// Deletes an asset's files; used when an import is abandoned before it is recorded.
pub fn remove_files(root: &Path, asset: &MediaAsset) {
    for path in std::iter::once(&asset.relative_path).chain(asset.audio_path.iter()) {
        if let Ok(path) = safe_path(root, path) {
            let _ = fs::remove_file(path);
        }
    }
}

/// Decodes an image asset to a BGRA frame no larger than the working-set limit.
pub fn decode_image(path: &Path) -> Result<VideoFrame, String> {
    let mut image = image::open(path)
        .map_err(|e| format!("Unreadable image: {e}"))?
        .to_rgba8();
    let (w, h) = image.dimensions();
    if w > MAX_FRAME_DIM || h > MAX_FRAME_DIM {
        let scale = MAX_FRAME_DIM as f64 / w.max(h) as f64;
        let nw = ((w as f64 * scale) as u32).max(1);
        let nh = ((h as f64 * scale) as u32).max(1);
        image = image::imageops::resize(&image, nw, nh, image::imageops::FilterType::Triangle);
    }
    let (width, height) = image.dimensions();
    let mut data = image.into_raw();
    for pixel in data.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    Ok(VideoFrame {
        pts_us: 0,
        width,
        height,
        stride: width * 4,
        format: PixelFormat::Bgra8888,
        color: ColorInfo::rec709_full(),
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_follow_extensions_and_paths_stay_in_the_media_folder() {
        assert_eq!(kind_for(Path::new("a/Clip.MP4")), Some(MediaKind::Video));
        assert_eq!(kind_for(Path::new("x.jpeg")), Some(MediaKind::Image));
        assert_eq!(kind_for(Path::new("x.mp3")), Some(MediaKind::Audio));
        assert_eq!(kind_for(Path::new("x.exe")), None);
        let ok = MediaAsset {
            id: "m-1".into(),
            name: "a.png".into(),
            kind: MediaKind::Image,
            relative_path: "assets/media/m-1.png".into(),
            audio_path: None,
            duration_us: IMAGE_MAX_US,
            width: 2,
            height: 2,
        };
        validate_assets(std::slice::from_ref(&ok)).unwrap();
        let mut bad = ok.clone();
        bad.relative_path = "assets/media/../../etc/passwd".into();
        assert!(validate_assets(&[bad]).is_err());
        assert!(validate_assets(&[ok.clone(), ok]).is_err(), "duplicate ids");
    }

    #[test]
    fn images_import_and_decode_to_bgra() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("logo.png");
        let mut img = image::RgbaImage::new(4, 2);
        img.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        img.save(&source).unwrap();
        let root = dir.path().join("p.aero");
        fs::create_dir_all(&root).unwrap();
        let asset = import(&root, &source).unwrap();
        assert_eq!(asset.kind, MediaKind::Image);
        assert_eq!((asset.width, asset.height), (4, 2));
        assert_eq!(asset.name, "logo.png");
        assert_eq!(asset.default_clip_us(), IMAGE_CLIP_US);
        validate_assets(std::slice::from_ref(&asset)).unwrap();
        let frame = decode_image(&root.join(&asset.relative_path)).unwrap();
        assert_eq!((frame.width, frame.height), (4, 2));
        // Red in RGBA is [0, 0, 255] in BGRA.
        assert_eq!(&frame.data[0..4], &[0, 0, 255, 255]);
        assert!(import(&root, &dir.path().join("missing.png")).is_err());
    }
}
