//! Imported media: videos, images and audio placed on the timeline between (or instead of)
//! parts of the recording.
//!
//! Import leaves the file where it is and refers to it by its path (copying a long video
//! took as long as the import itself). It probes the file and extracts any audio once to a
//! 48 kHz stereo WAV in `assets/media/`, so playback and export mix it like recorded audio. A
//! file that is later moved or deleted shows as missing until it is removed from the project. Video
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
/// Audio streams kept beyond the first; more are rare and would only slow the mix.
const MAX_AUDIO_STREAMS: usize = 15;

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

/// What a file's picture stands for, so it is edited and drawn like that part of a recording.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PictureRole {
    /// The screen: the canvas layout's size, corners, shadow and crop apply, and zooms.
    #[default]
    Screen,
    /// A camera: on a track above V1 it is drawn in the webcam bubble.
    Webcam,
}

/// What a sound stream is, so it is treated like that part of a recording.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoundRole {
    /// Speech: it can be transcribed and captioned.
    Mic,
    /// Music, game or desktop sound.
    Background,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaAsset {
    pub id: String,
    /// The original file name, for display.
    pub name: String,
    pub kind: MediaKind,
    /// A copy inside the project, relative to its root; imports from before media was
    /// referenced in place. Empty when `source_path` is set.
    #[serde(default)]
    pub relative_path: String,
    /// The original file, used where it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    /// The file is no longer there. Worked out when the project is read, never saved.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub missing: bool,
    #[serde(default)]
    pub picture_role: PictureRole,
    /// A role per audio stream; streams past the end take [`MediaAsset::sound_role`]'s default.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sound_roles: Vec<SoundRole>,
    /// An imported recording: its folder. The picture is its screen, the webcam bubble shows
    /// its camera, the sound streams are its microphone and system audio, and its mouse
    /// data gives its own zooms. Times in the asset are the recording's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recording_path: Option<String>,
    /// Extracted 48 kHz stereo WAV of the first audio stream, relative to the root, when
    /// the file has audio.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_path: Option<String>,
    /// The second and later audio streams (a recording with mic and desktop on separate
    /// tracks), extracted the same way. They all play together with the first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra_audio_paths: Vec<String>,
    /// A display name per audio stream, in order; empty for imports that predate it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub audio_names: Vec<String>,
    /// Playable length; for images, how far a clip can be stretched.
    pub duration_us: u64,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
}

impl MediaAsset {
    /// What stream `stream` is: as set, else a video's first stream is speech and every
    /// other stream (and an audio file, usually music) is background.
    pub fn sound_role(&self, stream: usize) -> SoundRole {
        self.sound_roles.get(stream).copied().unwrap_or(
            if stream == 0 && self.kind == MediaKind::Video {
                SoundRole::Mic
            } else {
                SoundRole::Background
            },
        )
    }

    /// Where the media's own file is.
    pub fn file_path(&self, root: &Path) -> Result<std::path::PathBuf, String> {
        match &self.source_path {
            Some(path) => Ok(std::path::PathBuf::from(path)),
            None => safe_path(root, &self.relative_path),
        }
    }

    /// Every extracted audio stream, first stream first.
    pub fn audio_paths(&self) -> impl Iterator<Item = &String> {
        self.audio_path.iter().chain(&self.extra_audio_paths)
    }

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
        let file_ok = match &asset.source_path {
            Some(path) => {
                asset.relative_path.is_empty()
                    && !path.is_empty()
                    && path.len() <= 4096
                    && !path.contains('\0')
                    && Path::new(path).is_absolute()
            }
            None => in_media_dir(&asset.relative_path),
        };
        if !file_ok || asset.audio_paths().any(|p| !in_media_dir(p)) {
            return Err("Imported media must live in assets/media or at an absolute path".into());
        }
        if asset.extra_audio_paths.len() > MAX_AUDIO_STREAMS
            || (asset.audio_path.is_none() && !asset.extra_audio_paths.is_empty())
            || asset.audio_names.len() > MAX_AUDIO_STREAMS + 1
        {
            return Err("Invalid imported media audio".into());
        }
        if asset
            .recording_path
            .as_ref()
            .is_some_and(|p| p.is_empty() || p.len() > 4096 || !Path::new(p).is_absolute())
        {
            return Err("An imported recording needs its absolute folder".into());
        }
        if asset.sound_roles.len() > asset.audio_paths().count() {
            return Err("More sound roles than audio streams".into());
        }
        if asset.duration_us == 0 {
            return Err("Imported media has no duration".into());
        }
    }
    Ok(())
}

/// The importable files a chosen path stands for: the file itself, or the supported files
/// directly inside a folder, by name. A recording folder is refused: it is not plain media.
pub fn expand_import_paths(
    paths: &[std::path::PathBuf],
) -> Result<Vec<std::path::PathBuf>, String> {
    let mut out = Vec::new();
    for path in paths {
        if !path.is_dir() {
            out.push(path.clone());
            continue;
        }
        if is_recording(path) {
            // A recording comes in whole: screen, camera, sound and mouse data.
            out.push(path.clone());
            continue;
        }
        let mut files: Vec<_> = fs::read_dir(path)
            .map_err(|e| format!("{}: {e}", path.display()))?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|p| p.is_file() && kind_for(p).is_some())
            .collect();
        files.sort();
        if files.is_empty() {
            return Err(format!("{} has no videos, images or audio", path.display()));
        }
        out.extend(files);
    }
    Ok(out)
}

/// Whether `path` is a recorder folder.
pub fn is_recording(path: &Path) -> bool {
    path.is_dir() && path.join("manifest.json").is_file()
}

/// Describes `source` for the project, which refers to it where it is, and extracts its
/// sound. The caller records the asset in the edit document.
pub fn import(root: &Path, source: &Path) -> Result<MediaAsset, String> {
    if is_recording(source) {
        return import_recording(root, source);
    }
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
    let source = fs::canonicalize(source).map_err(|e| format!("{}: {e}", source.display()))?;
    // Windows canonical paths carry a \\?\ prefix other tools do not expect.
    let source_text = source
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .to_string();
    let id = format!("m-{}", &uuid::Uuid::new_v4().simple().to_string()[..12]);
    fs::create_dir_all(safe_path(root, MEDIA_DIR)?)
        .map_err(|e| format!("Could not create {MEDIA_DIR}: {e}"))?;
    let (duration_us, width, height, audio) = describe(root, &id, &source, kind)?;
    let mut audio_paths = audio.paths.into_iter();
    Ok(MediaAsset {
        id,
        name,
        kind,
        relative_path: String::new(),
        source_path: Some(source_text),
        missing: false,
        picture_role: Default::default(),
        sound_roles: Vec::new(),
        recording_path: None,
        audio_path: audio_paths.next(),
        extra_audio_paths: audio_paths.collect(),
        audio_names: audio.names,
        duration_us,
        width,
        height,
    })
}

#[derive(Default)]
struct ExtractedAudio {
    paths: Vec<String>,
    names: Vec<String>,
}

type Described = (u64, u32, u32, ExtractedAudio);

fn describe(root: &Path, id: &str, file: &Path, kind: MediaKind) -> Result<Described, String> {
    use crate::media::ffmpeg;
    match kind {
        MediaKind::Image => {
            let (width, height) =
                image::image_dimensions(file).map_err(|e| format!("Unreadable image: {e}"))?;
            Ok((IMAGE_MAX_US, width, height, ExtractedAudio::default()))
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
            let mut names = ffmpeg::audio_stream_names(file)?;
            names.truncate(MAX_AUDIO_STREAMS + 1);
            if names.is_empty() && kind == MediaKind::Audio {
                return Err("The file has no audio stream".into());
            }
            let mut audio = ExtractedAudio::default();
            for stream in 0..names.len() {
                // The first stream keeps the name older projects use.
                let relative = if stream == 0 {
                    format!("{MEDIA_DIR}/{id}.audio.wav")
                } else {
                    format!("{MEDIA_DIR}/{id}.audio{}.wav", stream + 1)
                };
                let extracted = safe_path(root, &relative)
                    .and_then(|target| ffmpeg::extract_audio_wav(file, stream, &target));
                if let Err(error) = extracted {
                    remove_paths(root, audio.paths.iter().chain([&relative]));
                    return Err(error);
                }
                audio.paths.push(relative);
            }
            audio.names = names;
            Ok((duration_us, width, height, audio))
        }
    }
}

fn remove_paths<'a>(root: &Path, paths: impl IntoIterator<Item = &'a String>) {
    for path in paths {
        if let Ok(path) = safe_path(root, path) {
            let _ = fs::remove_file(path);
        }
    }
}

fn plain_path(path: &Path) -> String {
    // Windows canonical paths carry a \\?\ prefix other tools do not expect.
    path.to_string_lossy()
        .trim_start_matches(r"\\?\")
        .to_string()
}

/// A recording as one piece of media: its screen with its camera, microphone and system
/// audio (each joined into one WAV in the project), on the recording's own clock.
fn import_recording(root: &Path, folder: &Path) -> Result<MediaAsset, String> {
    use crate::project::TrackType;
    let folder = fs::canonicalize(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let (tracks, duration_us) = crate::project::reader::recording_tracks(&folder)?;
    if duration_us == 0 {
        return Err("That recording is empty".into());
    }
    let screen = tracks
        .iter()
        .find(|(t, s)| t.descriptor.track_type == TrackType::Screen && !s.is_empty())
        .ok_or("That recording has no screen video")?;
    let (mut width, mut height) = (
        screen.0.descriptor.width.unwrap_or(0),
        screen.0.descriptor.height.unwrap_or(0),
    );
    if width == 0 || height == 0 {
        let first = safe_path(&folder, &screen.1[0].relative_path)?;
        let info = crate::media::ffmpeg::probe_video(&first)?;
        (width, height) = (info.width, info.height);
    }
    let id = format!("m-{}", &uuid::Uuid::new_v4().simple().to_string()[..12]);
    fs::create_dir_all(safe_path(root, MEDIA_DIR)?)
        .map_err(|e| format!("Could not create {MEDIA_DIR}: {e}"))?;
    let mut audio = ExtractedAudio::default();
    let mut roles = Vec::new();
    for (kind, name, role) in [
        (TrackType::MicAudio, "Microphone", SoundRole::Mic),
        (
            TrackType::SystemAudio,
            "System audio",
            SoundRole::Background,
        ),
    ] {
        for (track, segments) in tracks
            .iter()
            .filter(|(t, _)| t.descriptor.track_type == kind)
        {
            let parts: Vec<_> = segments
                .iter()
                .filter(|s| s.available)
                .map(|s| Ok((safe_path(&folder, &s.relative_path)?, s.start_us)))
                .collect::<Result<_, String>>()?;
            if parts.is_empty() {
                continue;
            }
            let stream = audio.paths.len();
            let relative = if stream == 0 {
                format!("{MEDIA_DIR}/{id}.audio.wav")
            } else {
                format!("{MEDIA_DIR}/{id}.audio{}.wav", stream + 1)
            };
            let joined = safe_path(root, &relative).and_then(|target| {
                crate::media::ffmpeg::assemble_audio_wav(&parts, duration_us, &target)
            });
            if let Err(error) = joined {
                remove_paths(root, audio.paths.iter().chain([&relative]));
                return Err(error);
            }
            audio.paths.push(relative);
            let several = tracks
                .iter()
                .filter(|(t, _)| t.descriptor.track_type == kind)
                .count()
                > 1;
            audio.names.push(if several {
                format!("{name} ({})", track.descriptor.id)
            } else {
                name.to_string()
            });
            roles.push(role);
        }
    }
    let name = fs::read(folder.join("manifest.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|m| {
            m.get("projectName")
                .and_then(|n| n.as_str())
                .map(String::from)
        })
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| {
            folder
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Recording".into())
        });
    let mut paths = audio.paths.into_iter();
    Ok(MediaAsset {
        id,
        name,
        kind: MediaKind::Video,
        relative_path: String::new(),
        source_path: Some(plain_path(&folder)),
        missing: false,
        picture_role: PictureRole::Screen,
        sound_roles: roles,
        recording_path: Some(plain_path(&folder)),
        audio_path: paths.next(),
        extra_audio_paths: paths.collect(),
        audio_names: audio.names,
        duration_us,
        width,
        height,
    })
}

/// Deletes the files an import made in the project; used when an import is abandoned before
/// it is recorded. A file referenced in place belongs to the user and is never touched.
pub fn remove_files(root: &Path, asset: &MediaAsset) {
    let copy = Some(&asset.relative_path).filter(|p| !p.is_empty() && asset.source_path.is_none());
    remove_paths(root, copy.into_iter().chain(asset.audio_paths()));
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
            source_path: None,
            missing: false,
            picture_role: Default::default(),
            sound_roles: Vec::new(),
            recording_path: None,
            audio_path: None,
            extra_audio_paths: Vec::new(),
            audio_names: Vec::new(),
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
        // Referenced where it is, not copied.
        assert!(asset.relative_path.is_empty());
        let referenced = asset.file_path(&root).unwrap();
        assert!(
            referenced.is_absolute() && referenced.ends_with("logo.png") && referenced.is_file()
        );
        let frame = decode_image(&asset.file_path(&root).unwrap()).unwrap();
        assert_eq!((frame.width, frame.height), (4, 2));
        // Red in RGBA is [0, 0, 255] in BGRA.
        assert_eq!(&frame.data[0..4], &[0, 0, 255, 255]);
        assert!(import(&root, &dir.path().join("missing.png")).is_err());
    }

    /// A recording with mic and desktop on separate audio tracks keeps both.
    #[test]
    fn every_audio_stream_is_extracted_with_its_title() {
        let Ok(ffmpeg) = crate::media::ffmpeg::ffmpeg_path() else {
            eprintln!("skipping: FFmpeg is not available");
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("two-tracks.mkv");
        let status = std::process::Command::new(ffmpeg)
            .args(["-v", "error", "-y"])
            .args(["-f", "lavfi", "-i", "color=c=red:s=64x36:r=30:d=0.5"])
            .args([
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:sample_rate=48000:duration=0.5",
            ])
            .args([
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=880:sample_rate=48000:duration=0.5",
            ])
            .args(["-map", "0:v", "-map", "1:a", "-map", "2:a"])
            .args(["-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac"])
            .args(["-metadata:s:a:0", "title=Mic", "-shortest"])
            .arg(&source)
            .status()
            .unwrap();
        assert!(status.success());
        let root = dir.path().join("p.aero");
        fs::create_dir_all(&root).unwrap();
        let asset = import(&root, &source).unwrap();
        assert_eq!(asset.kind, MediaKind::Video);
        assert_eq!(asset.audio_paths().count(), 2);
        assert_eq!(
            asset.audio_names,
            vec!["Mic".to_string(), "Audio 2".to_string()]
        );
        for path in asset.audio_paths() {
            assert!(root.join(path).is_file(), "{path} was extracted");
        }
        validate_assets(std::slice::from_ref(&asset)).unwrap();
        remove_files(&root, &asset);
        assert!(asset.audio_paths().all(|path| !root.join(path).exists()));
        assert!(source.is_file(), "the user's own file is never removed");
    }

    #[test]
    fn folders_expand_to_their_media_and_recordings_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("broll");
        fs::create_dir_all(&folder).unwrap();
        for name in ["b.png", "a.mp4", "notes.txt"] {
            fs::write(folder.join(name), b"x").unwrap();
        }
        let single = dir.path().join("c.wav");
        let found = expand_import_paths(&[folder.clone(), single.clone()]).unwrap();
        assert_eq!(
            found,
            vec![folder.join("a.mp4"), folder.join("b.png"), single]
        );

        let recording = dir.path().join("rec");
        fs::create_dir_all(&recording).unwrap();
        fs::write(recording.join("manifest.json"), b"{}").unwrap();
        // A recording comes in whole, as one import.
        assert_eq!(
            expand_import_paths(&[recording.clone()]).unwrap(),
            vec![recording]
        );
        let empty = dir.path().join("empty");
        fs::create_dir_all(&empty).unwrap();
        assert!(expand_import_paths(&[empty]).is_err());
    }
}
