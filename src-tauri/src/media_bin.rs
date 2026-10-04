//! Importing media: videos, images and audio, and recordings made with AeroEdits.
//!
//! Import leaves the file where it is and refers to it by its path (copying a long video
//! took as long as the import itself). It probes the file and extracts each audio stream once
//! to a 48 kHz stereo WAV in `assets/media/`, so playback and export mix it like recorded
//! sound. A file that is later moved or deleted shows as missing until it is removed from the
//! project. Video frames always decode through FFmpeg, so any format FFmpeg reads works. A
//! recording comes in whole, as an asset with its own streams, played from its folder.
use crate::media::{ColorInfo, PixelFormat, VideoFrame, MAX_FRAME_DIM};
use crate::project::reader::safe_path;
use crate::sequence::{Asset, AssetKind, Role, Stream, StreamKind, IMAGE_MAX_US};
use std::fs;
use std::path::Path;

const MEDIA_DIR: &str = "assets/media";
/// Audio streams kept beyond the first; more are rare and would only slow the mix.
const MAX_AUDIO_STREAMS: usize = 15;

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mov", "m4v", "mkv", "webm", "avi"];
const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg"];
const AUDIO_EXTENSIONS: &[&str] = &["wav", "mp3", "m4a", "aac", "flac", "ogg"];

pub fn kind_for(path: &Path) -> Option<AssetKind> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let ext = ext.as_str();
    if VIDEO_EXTENSIONS.contains(&ext) {
        Some(AssetKind::Video)
    } else if IMAGE_EXTENSIONS.contains(&ext) {
        Some(AssetKind::Image)
    } else if AUDIO_EXTENSIONS.contains(&ext) {
        Some(AssetKind::Audio)
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
pub fn import(root: &Path, source: &Path) -> Result<Asset, String> {
    if is_recording(source) {
        return crate::sequence::sources::recording_asset(
            source,
            crate::sequence::sources::new_asset_id(true),
        );
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
    let id = crate::sequence::sources::new_asset_id(false);
    fs::create_dir_all(safe_path(root, MEDIA_DIR)?)
        .map_err(|e| format!("Could not create {MEDIA_DIR}: {e}"))?;
    let (duration_us, width, height, mut streams) = describe(root, &id, &source, kind)?;
    let picture = |role| Stream {
        id: "picture".into(),
        kind: StreamKind::Picture,
        role,
        name: "Picture".into(),
        audio_path: None,
        fps: None,
    };
    match kind {
        AssetKind::Video => streams.insert(0, picture(Role::Screen)),
        AssetKind::Image => streams.push(picture(Role::Overlay)),
        _ => {}
    }
    Ok(Asset {
        id,
        name,
        kind,
        path: plain_path(&source),
        streams,
        duration_us,
        width,
        height,
        pauses: Vec::new(),
        missing: false,
    })
}

type Described = (u64, u32, u32, Vec<Stream>);

/// Length, size and the sound streams (extracted) of a file.
fn describe(root: &Path, id: &str, file: &Path, kind: AssetKind) -> Result<Described, String> {
    use crate::media::ffmpeg;
    match kind {
        AssetKind::Image => {
            let (width, height) =
                image::image_dimensions(file).map_err(|e| format!("Unreadable image: {e}"))?;
            Ok((IMAGE_MAX_US, width, height, Vec::new()))
        }
        AssetKind::Video | AssetKind::Audio => {
            let duration_us = ffmpeg::duration_us(file)?;
            if duration_us == 0 {
                return Err("The file has no playable length".into());
            }
            let (width, height) = if kind == AssetKind::Video {
                let info = ffmpeg::probe_video(file)?;
                (info.width, info.height)
            } else {
                (0, 0)
            };
            let mut names = ffmpeg::audio_stream_names(file)?;
            names.truncate(MAX_AUDIO_STREAMS + 1);
            if names.is_empty() && kind == AssetKind::Audio {
                return Err("The file has no audio stream".into());
            }
            let mut streams: Vec<Stream> = Vec::new();
            for (stream, name) in names.into_iter().enumerate() {
                let relative = format!("{MEDIA_DIR}/{id}.sound{stream}.wav");
                let extracted = safe_path(root, &relative)
                    .and_then(|target| ffmpeg::extract_audio_wav(file, stream, &target));
                if let Err(error) = extracted {
                    let done: Vec<String> = streams
                        .iter()
                        .filter_map(|s| s.audio_path.clone())
                        .collect();
                    remove_paths(root, done.iter().chain([&relative]));
                    return Err(error);
                }
                streams.push(Stream {
                    id: format!("sound{stream}"),
                    kind: StreamKind::Sound,
                    // A video's first sound is usually its speech; the rest, and an audio
                    // file, background.
                    role: if stream == 0 && kind == AssetKind::Video {
                        Role::Mic
                    } else {
                        Role::Background
                    },
                    name,
                    audio_path: Some(relative),
                    fps: None,
                });
            }
            Ok((duration_us, width, height, streams))
        }
        AssetKind::Recording => Err("A recording is imported from its folder".into()),
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
    // Windows canonical paths carry a verbatim prefix other tools do not expect.
    path.to_string_lossy()
        .trim_start_matches(r"\\?\")
        .to_string()
}

/// Deletes the files an import made in the project; used when an import is abandoned before
/// it is recorded. A file referenced in place belongs to the user and is never touched.
pub fn remove_files(root: &Path, asset: &Asset) {
    remove_paths(
        root,
        asset.streams.iter().filter_map(|s| s.audio_path.as_ref()),
    );
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
        assert_eq!(kind_for(Path::new("a/Clip.MP4")), Some(AssetKind::Video));
        assert_eq!(kind_for(Path::new("x.jpeg")), Some(AssetKind::Image));
        assert_eq!(kind_for(Path::new("x.mp3")), Some(AssetKind::Audio));
        assert_eq!(kind_for(Path::new("x.exe")), None);
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
        assert_eq!(asset.kind, AssetKind::Image);
        assert_eq!((asset.width, asset.height), (4, 2));
        assert_eq!(asset.name, "logo.png");
        assert_eq!(asset.default_clip_us(), crate::sequence::IMAGE_CLIP_US);
        assert_eq!(asset.streams[0].role, Role::Overlay);
        crate::sequence::validate_assets(std::slice::from_ref(&asset)).unwrap();
        // Referenced where it is, not copied.
        let referenced = Path::new(&asset.path);
        assert!(
            referenced.is_absolute() && referenced.ends_with("logo.png") && referenced.is_file()
        );
        let frame = decode_image(referenced).unwrap();
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
        assert_eq!(asset.kind, AssetKind::Video);
        let streams: Vec<_> = asset
            .streams
            .iter()
            .map(|s| (s.id.as_str(), s.kind, s.role, s.name.as_str()))
            .collect();
        assert_eq!(
            streams,
            vec![
                ("picture", StreamKind::Picture, Role::Screen, "Picture"),
                ("sound0", StreamKind::Sound, Role::Mic, "Mic"),
                ("sound1", StreamKind::Sound, Role::Background, "Audio 2"),
            ]
        );
        let paths: Vec<&String> = asset
            .streams
            .iter()
            .filter_map(|s| s.audio_path.as_ref())
            .collect();
        for path in &paths {
            assert!(root.join(path).is_file(), "{path} was extracted");
        }
        crate::sequence::validate_assets(std::slice::from_ref(&asset)).unwrap();
        remove_files(&root, &asset);
        assert!(paths.iter().all(|path| !root.join(path).exists()));
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
