//! Where each asset's streams are on disk. A recording's streams are the recorder's segment
//! files, seen inside the project at `recordings/<asset id>/` (so every reader resolves them
//! against the project root, as it always has); a file's picture is the file itself, and its
//! sound the WAV extracted at import.
use super::{Asset, AssetKind, Role, SourceRange, Stream, StreamKind};
use crate::project::folder::{mount_path, mount_recording};
use crate::project::manifest::{ProjectManifest, TrackType};
use crate::project::reader::{SegmentSummary, TrackSummary};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// A recording's tracks with their segments, paths seen through the project.
#[derive(Debug)]
pub struct RecordingFiles {
    pub manifest: ProjectManifest,
    pub tracks: Vec<(TrackSummary, Vec<SegmentSummary>)>,
    pub duration_us: u64,
    pub diagnostics: Vec<String>,
}

impl RecordingFiles {
    pub fn track(&self, id: &str) -> Option<&(TrackSummary, Vec<SegmentSummary>)> {
        self.tracks.iter().find(|(t, _)| t.descriptor.id == id)
    }
}

type Stamp = (
    u64,
    Option<std::time::SystemTime>,
    Option<std::time::SystemTime>,
);

fn stamp(folder: &Path) -> Stamp {
    let journal = std::fs::metadata(folder.join("journal.jsonl")).ok();
    let manifest = std::fs::metadata(folder.join("manifest.json")).ok();
    (
        journal.as_ref().map_or(0, |m| m.len()),
        journal.and_then(|m| m.modified().ok()),
        manifest.and_then(|m| m.modified().ok()),
    )
}

type Cache = HashMap<(PathBuf, String), (Stamp, Arc<RecordingFiles>)>;

/// The recording behind `asset`, indexed once while its files stay the same, and mounted in
/// project `root` so its paths resolve there.
pub fn recording(root: &Path, asset: &Asset) -> Result<Arc<RecordingFiles>, String> {
    if asset.kind != AssetKind::Recording {
        return Err(format!("{} is not a recording", asset.name));
    }
    let folder = PathBuf::from(&asset.path);
    mount_recording(root, &asset.id, &folder);
    static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let key = (folder.clone(), asset.id.clone());
    let now = stamp(&folder);
    if let Some((seen, files)) = cache.lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
        if *seen == now {
            return Ok(files.clone());
        }
    }
    if !folder.is_dir() {
        return Err(format!(
            "{} is missing (it was at {})",
            asset.name, asset.path
        ));
    }
    let (manifest, mut tracks, duration_us, diagnostics) =
        crate::project::reader::read_recording(&folder)?;
    let mount = mount_path(&asset.id);
    for (summary, segments) in &mut tracks {
        summary.descriptor.relative_path = format!("{mount}/{}", summary.descriptor.relative_path);
        for segment in segments {
            segment.relative_path = format!("{mount}/{}", segment.relative_path);
        }
    }
    let files = Arc::new(RecordingFiles {
        manifest,
        tracks,
        duration_us,
        diagnostics,
    });
    cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(key, (now, files.clone()));
    Ok(files)
}

/// What plays a stream.
#[derive(Clone, Debug)]
pub enum StreamSource {
    /// Segment files on the asset's clock (with gaps where nothing was recorded), relative
    /// to the project root.
    Segments(Vec<SegmentSummary>),
    /// A video file's picture, decoded where it is.
    Video(PathBuf),
    /// A still picture.
    Image(PathBuf),
}

/// What plays stream `stream` of `asset`.
pub fn stream_source(root: &Path, asset: &Asset, stream: &str) -> Result<StreamSource, String> {
    let info = asset
        .stream(stream)
        .ok_or_else(|| format!("{} has no stream {stream}", asset.name))?;
    match asset.kind {
        AssetKind::Recording => {
            let files = recording(root, asset)?;
            Ok(StreamSource::Segments(
                files
                    .track(stream)
                    .map(|(_, s)| s.clone())
                    .unwrap_or_default(),
            ))
        }
        AssetKind::Image => Ok(StreamSource::Image(PathBuf::from(&asset.path))),
        AssetKind::Video if info.kind == StreamKind::Picture => {
            Ok(StreamSource::Video(PathBuf::from(&asset.path)))
        }
        AssetKind::Video | AssetKind::Audio => {
            let path = info
                .audio_path
                .as_ref()
                .ok_or_else(|| format!("{}'s sound was not extracted", asset.name))?;
            Ok(StreamSource::Segments(vec![wav_segment(
                root,
                stream,
                path,
                asset.duration_us,
            )]))
        }
    }
}

/// The segments of a sound stream (an extracted WAV is one segment over the whole file).
pub fn sound_segments(
    root: &Path,
    asset: &Asset,
    stream: &str,
) -> Result<Vec<SegmentSummary>, String> {
    match stream_source(root, asset, stream)? {
        StreamSource::Segments(segments) => Ok(segments),
        _ => Err(format!("{}'s {stream} is not sound", asset.name)),
    }
}

fn wav_segment(root: &Path, track_id: &str, path: &str, duration_us: u64) -> SegmentSummary {
    SegmentSummary {
        track_id: track_id.into(),
        relative_path: path.into(),
        start_us: 0,
        end_us: duration_us,
        // Its real size: readers check it to catch a file being rewritten.
        size_bytes: crate::project::file_len(root, path),
        media_timescale: crate::media::audio::SAMPLE_RATE,
        media_start_value: 0,
        host_anchor_us: 0,
        is_keyframe_start: None,
        available: true,
    }
}

/// The recorder's track kind a role is processed as (speech or background sound).
pub fn track_type(role: Role) -> TrackType {
    match role {
        Role::Screen | Role::Overlay => TrackType::Screen,
        Role::Webcam => TrackType::Webcam,
        Role::Mic => TrackType::MicAudio,
        Role::Background => TrackType::SystemAudio,
    }
}

/// A recording folder as an asset: its screen and camera, its microphones and system audio,
/// its pauses, on the recording's own clock.
pub fn recording_asset(folder: &Path, id: String) -> Result<Asset, String> {
    let folder = dunce::canonicalize(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let (manifest, tracks, duration_us, _) = crate::project::reader::read_recording(&folder)?;
    if duration_us == 0 {
        return Err("That recording is empty".into());
    }
    let count = |kind: TrackType| {
        tracks
            .iter()
            .filter(|(t, _)| t.descriptor.track_type == kind)
            .count()
    };
    let mut streams = Vec::new();
    for (track, _) in &tracks {
        let d = &track.descriptor;
        let (kind, role, name) = match d.track_type {
            TrackType::Screen => (StreamKind::Picture, Role::Screen, "Screen"),
            TrackType::Webcam => (StreamKind::Picture, Role::Webcam, "Camera"),
            TrackType::MicAudio => (StreamKind::Sound, Role::Mic, "Microphone"),
            TrackType::SystemAudio => (StreamKind::Sound, Role::Background, "System audio"),
        };
        let name = if count(d.track_type) > 1 {
            format!("{name} ({})", d.id)
        } else {
            name.to_string()
        };
        streams.push(Stream {
            id: d.id.clone(),
            kind,
            role,
            name,
            audio_path: None,
            fps: d.fps,
        });
    }
    if streams.is_empty() {
        return Err("That recording has nothing in it".into());
    }
    let screen = tracks
        .iter()
        .find(|(t, s)| t.descriptor.track_type == TrackType::Screen && !s.is_empty());
    let (mut width, mut height) = screen
        .map(|(t, _)| {
            (
                t.descriptor.width.unwrap_or(0),
                t.descriptor.height.unwrap_or(0),
            )
        })
        .unwrap_or((0, 0));
    if let Some((_, segments)) = screen.filter(|_| width == 0 || height == 0) {
        let first = crate::project::reader::safe_path(&folder, &segments[0].relative_path)?;
        if let Ok(info) = crate::media::ffmpeg::probe_video(&first) {
            (width, height) = (info.width, info.height);
        }
    }
    let name = Some(manifest.project_name.trim())
        .filter(|n| !n.is_empty())
        .map(String::from)
        .unwrap_or_else(|| {
            folder
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Recording".into())
        });
    Ok(Asset {
        id,
        name,
        kind: AssetKind::Recording,
        path: folder
            .to_string_lossy()
            .trim_start_matches(r"\\?\")
            .to_string(),
        streams,
        duration_us,
        width,
        height,
        pauses: manifest
            .pause_intervals
            .iter()
            .filter(|p| p.end_us > p.start_us && p.end_us <= duration_us)
            .map(|p| SourceRange {
                start_us: p.start_us,
                end_us: p.end_us,
            })
            .collect(),
        missing: false,
    })
}

/// A new asset id: `rec-…` for a recording, `m-…` for a file.
pub fn new_asset_id(recording: bool) -> String {
    let unique = &uuid::Uuid::new_v4().simple().to_string()[..12];
    if recording {
        format!("rec-{unique}")
    } else {
        format!("m-{unique}")
    }
}
