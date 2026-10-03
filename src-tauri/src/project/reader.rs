//! Bounded, non-repairing project inspection. Source files are never modified.
use super::{
    display_name_from_input,
    folder::{
        load_project_file, mount_recording, mounted_recording, save_project_file, under_mount,
        ProjectFile, RECORDING_MOUNT,
    },
    journal::JournalRecord,
    layout::EditLayout,
    manifest::{ProjectManifest, TrackDescriptor},
    revision::{self, EditDocument, EditHistory, TrimSide},
};
use crate::zoom::ZoomKeyframe;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{BufRead, BufReader, Read},
    path::{Component, Path, PathBuf},
};

const MANIFEST_LIMIT: u64 = 1_048_576;
const JOURNAL_LIMIT: u64 = 33_554_432;
const LINE_LIMIT: u64 = 65_536;
const RECORD_LIMIT: usize = 100_000;
const MAX_SAFE_TIME: u64 = 9_007_199_254_740_991;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SegmentSummary {
    pub track_id: String,
    pub relative_path: String,
    pub start_us: u64,
    pub end_us: u64,
    pub size_bytes: u64,
    pub media_timescale: u32,
    pub media_start_value: i64,
    pub host_anchor_us: i64,
    pub is_keyframe_start: Option<bool>,
    pub available: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrackSummary {
    pub descriptor: TrackDescriptor,
    pub segment_count: usize,
    pub available_segment_count: usize,
}

/// One entry of the edited timeline, in playback order. Without `media` it is a range of the
/// recording; with it, a range of an imported media asset (times within that file).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RetainedInterval {
    pub start_us: u64,
    pub end_us: u64,
    /// Imported media asset id; `None` is the recording.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
    /// For imported media: its sound was split off onto audio tracks, so this clip plays
    /// silent here.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub audio_unlinked: bool,
}

impl RetainedInterval {
    /// A range of the recording.
    pub fn recording(start_us: u64, end_us: u64) -> Self {
        Self {
            start_us,
            end_us,
            media: None,
            audio_unlinked: false,
        }
    }

    pub fn is_recording(&self) -> bool {
        self.media.is_none()
    }

    /// Empty V1 time.
    pub fn is_gap(&self) -> bool {
        self.media.as_deref() == Some(crate::project::revision::GAP)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OpenedProject {
    pub project_handle: String,
    pub revision: u64,
    pub manifest: ProjectManifest,
    pub source_duration_us: u64,
    pub edited_duration_us: u64,
    pub retained_intervals: Vec<RetainedInterval>,
    pub tracks: Vec<TrackSummary>,
    pub diagnostics: Vec<String>,
    pub preview_available: bool,
    pub undo_available: bool,
    pub redo_available: bool,
    #[serde(default)]
    pub zooms: Vec<ZoomKeyframe>,
    #[serde(default)]
    pub dismissed_zoom_ids: Vec<String>,
    #[serde(default)]
    pub layout: EditLayout,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    /// The recording folder being edited; `None` for a project without a recording. For an
    /// older recording folder that holds its own edits, the same as `project_path`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recording_path: Option<String>,
    /// Source ranges the edit cut out that can be put back.
    #[serde(default)]
    pub removed_intervals: Vec<RetainedInterval>,
    #[serde(default)]
    pub split_points_us: Vec<u64>,
    #[serde(default)]
    pub webcam_focus: crate::webcam_focus::WebcamFocus,
    pub audio: crate::project::AudioSettings,
    #[serde(default)]
    pub captions: crate::captions::CaptionSettings,
    #[serde(default)]
    pub media_assets: Vec<crate::media_bin::MediaAsset>,
    #[serde(default)]
    pub chapters: Vec<crate::chapters::Chapter>,
    #[serde(default)]
    pub shorts: Vec<crate::shorts::Short>,
    /// Video tracks V2, V3, ... above the main sequence, bottom to top.
    #[serde(default)]
    pub overlay_tracks: Vec<crate::tracks::OverlayTrack>,
    /// V1 as a track: magnetic, hidden, muted, stack position.
    #[serde(default)]
    pub main_track: crate::project::revision::MainTrack,
    /// Auto-zoom settings.
    #[serde(default)]
    pub zoom_settings: crate::zoom::ZoomSettings,
    /// Set when this is short `id`'s own timeline (the timeline fields are the short's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_view: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SegmentPage {
    pub segments: Vec<SegmentSummary>,
    pub next_offset: Option<usize>,
}

pub struct ProjectReader {
    pub summary: OpenedProject,
    segments: HashMap<String, Vec<SegmentSummary>>,
    root: PathBuf,
    history: EditHistory,
    /// Set for a project folder (not an older recording folder holding its own edits).
    project_file: Option<ProjectFile>,
    // Directory advisory lease is shared with the recording writer. It creates
    // no lock file and holds the source snapshot against cooperating writers.
    _lease: File,
    /// The same, on a project folder's recording.
    _recording_lease: Option<File>,
}

pub(crate) fn is_safe_track_id(id: &str) -> bool {
    !id.is_empty()
        && id != "."
        && id != ".."
        && !id.contains('/')
        && !id.contains('\\')
        && !id.contains('\0')
        && !id.contains(':')
}

fn track_media_dir(track_id: &str) -> Result<PathBuf, String> {
    if !is_safe_track_id(track_id) {
        return Err("Invalid track ID".into());
    }
    Ok(PathBuf::from("media").join(track_id))
}

fn segment_belongs_to_track(track_id: &str, relative_path: &str) -> Result<(), String> {
    let dir = track_media_dir(track_id)?;
    let path = Path::new(relative_path);
    if !path.starts_with(&dir) || path == dir.as_path() {
        return Err("Segment is outside its track directory".into());
    }
    Ok(())
}

pub(crate) fn safe_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    if relative.is_empty() || relative.contains('\\') || relative.contains(':') {
        return Err("Invalid project-relative path".into());
    }
    // A project folder sees its recording at `recording/`.
    let (root, relative) = match under_mount(relative).zip(mounted_recording(root)) {
        Some((inside, recording)) => (recording, inside),
        None => (root.to_path_buf(), relative),
    };
    let mut path = root;
    for part in Path::new(relative).components() {
        let Component::Normal(part) = part else {
            return Err("Unsafe project-relative path".into());
        };
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err("Symlink in project path".into())
            }
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(path)
}

pub(crate) fn open_regular(path: &Path) -> Result<File, String> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // Open a symlink itself rather than its target, so the regular-file
        // check below rejects it like O_NOFOLLOW does on Unix.
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = options.open(path).map_err(|e| e.to_string())?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Expected regular metadata file".into());
    }
    Ok(file)
}

fn bounded_read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let file = open_regular(path)?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("Project metadata exceeds size limit".into());
    }
    Ok(bytes)
}

#[cfg(windows)]
const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
#[cfg(windows)]
const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;

/// Holds the project directory open while it is read. On Unix this also takes
/// a shared `flock`, so a writer holding an exclusive lock is detected. Windows
/// has no directory locks; there the `.lock` file check in `ProjectReader::open`
/// is the only writer guard.
pub(crate) fn acquire_read_lease(root: &Path) -> Result<File, String> {
    #[cfg(windows)]
    let lease = {
        use std::os::windows::fs::OpenOptionsExt;
        // Directories can only be opened as a handle with backup semantics.
        fs::OpenOptions::new()
            .read(true)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(root)
            .map_err(|e| e.to_string())?
    };
    #[cfg(not(windows))]
    let lease = File::open(root).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        if unsafe { libc::flock(lease.as_raw_fd(), libc::LOCK_SH | libc::LOCK_NB) } != 0 {
            return Err("Project is in use by a writer".into());
        }
    }
    Ok(lease)
}

/// What a recording folder holds: its manifest, the committed segments of each track, and
/// the recorded time without pauses.
struct RecordingIndex {
    manifest: ProjectManifest,
    segments: HashMap<String, Vec<SegmentSummary>>,
    summaries: Vec<TrackSummary>,
    duration: u64,
    retained: Vec<RetainedInterval>,
    diagnostics: Vec<String>,
}

impl RecordingIndex {
    /// A project without a recording: no tracks and nothing on the timeline yet.
    fn empty() -> Self {
        Self {
            manifest: ProjectManifest {
                version: ProjectManifest::CURRENT_VERSION,
                session_id: "no-recording".into(),
                project_name: "Untitled".into(),
                created_at: String::new(),
                duration_us: 0,
                active_duration_us: 0,
                pause_intervals: Vec::new(),
                gaps_total: 0,
                source_geometry: None,
                cursor_mode: None,
                tracks: Vec::new(),
            },
            segments: HashMap::new(),
            summaries: Vec::new(),
            duration: 0,
            retained: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Prefixes every media path with `mount`, where the project sees its recording.
    fn mount_at(&mut self, mount: &str) {
        let prefix = |path: &mut String| *path = format!("{mount}/{path}");
        for segment in self.segments.values_mut().flatten() {
            prefix(&mut segment.relative_path);
        }
        for track in &mut self.manifest.tracks {
            prefix(&mut track.relative_path);
        }
        for summary in &mut self.summaries {
            prefix(&mut summary.descriptor.relative_path);
        }
    }
}

fn reject_writer_lock(root: &Path) -> Result<(), String> {
    // Reject old writers/stale locks too. Recovery, not open, owns repairs.
    if fs::symlink_metadata(root.join(".lock")).is_ok() {
        return Err("Project has a writer lock; close recording or recover it first".into());
    }
    Ok(())
}

/// The tracks, with their segments, and the length of the recording in `folder`: what an
/// imported recording plays from.
pub fn recording_tracks(
    folder: &Path,
) -> Result<(Vec<(TrackSummary, Vec<SegmentSummary>)>, u64), String> {
    let index = index_recording(folder)?;
    let tracks = index
        .summaries
        .iter()
        .map(|track| {
            let segments = index
                .segments
                .get(&track.descriptor.id)
                .cloned()
                .unwrap_or_default();
            (track.clone(), segments)
        })
        .collect();
    Ok((tracks, index.duration))
}

fn index_recording(root: &Path) -> Result<RecordingIndex, String> {
    let root = root.to_path_buf();
    let bytes = bounded_read(&safe_path(&root, "manifest.json")?, MANIFEST_LIMIT)?;
    let manifest: ProjectManifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    manifest.validate().map_err(|e| e.to_string())?;
    if manifest.tracks.len() > 64
        || manifest.pause_intervals.len() > 10_000
        || manifest.duration_us > MAX_SAFE_TIME
    {
        return Err("Project exceeds metadata limits".into());
    }
    let mut tracks = HashMap::new();
    let mut segments: HashMap<String, Vec<SegmentSummary>> = HashMap::new();
    for track in &manifest.tracks {
        if !is_safe_track_id(&track.id) {
            return Err("Invalid track ID".into());
        }
        safe_path(&root, &track.relative_path)?;
        if tracks.insert(track.id.clone(), track).is_some() {
            return Err("Duplicate or empty track ID".into());
        }
        segments.insert(track.id.clone(), Vec::new());
    }
    let mut diagnostics = Vec::new();
    let journal_path = safe_path(&root, "journal.jsonl")?;
    let mut duration = manifest.duration_us;
    let mut paths = HashSet::new();
    if journal_path.exists() {
        let file = open_regular(&journal_path)?;
        let meta = file.metadata().map_err(|e| e.to_string())?;
        if !meta.is_file() || meta.len() > JOURNAL_LIMIT {
            return Err("Journal exceeds size limit or is not a file".into());
        }
        let mut reader = BufReader::new(file.take(JOURNAL_LIMIT + 1));
        let mut total = 0u64;
        let mut last_seq = None;
        for line_number in 0..=RECORD_LIMIT {
            let mut line = Vec::new();
            let count = reader
                .by_ref()
                .take(LINE_LIMIT + 1)
                .read_until(b'\n', &mut line)
                .map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            total += count as u64;
            if count as u64 > LINE_LIMIT || total > JOURNAL_LIMIT || line_number == RECORD_LIMIT {
                return Err("Journal record limit exceeded".into());
            }
            if line.iter().all(u8::is_ascii_whitespace) {
                continue;
            }
            let record: JournalRecord = match serde_json::from_slice(&line) {
                Ok(record) => record,
                Err(error) if error.is_eof() && !line.ends_with(b"\n") => {
                    diagnostics.push(
                        "Incomplete final journal line ignored; source was not repaired".into(),
                    );
                    break;
                }
                Err(error) => {
                    return Err(format!("Corrupt journal line {}: {error}", line_number + 1))
                }
            };
            if last_seq.is_some_and(|seq| record.seq() <= seq) {
                return Err("Journal sequence is not increasing".into());
            }
            last_seq = Some(record.seq());
            let segment = match record {
                JournalRecord::SegmentCommitted {
                    track_id,
                    relative_path,
                    start_us,
                    end_us,
                    size_bytes,
                    media_timescale,
                    media_start_value,
                    host_anchor_us,
                    is_keyframe_start,
                    ..
                } => Some(SegmentSummary {
                    track_id,
                    relative_path,
                    start_us,
                    end_us,
                    size_bytes,
                    media_timescale,
                    media_start_value,
                    host_anchor_us,
                    is_keyframe_start: Some(is_keyframe_start),
                    available: true,
                }),
                JournalRecord::UnindexedSegmentRecovered {
                    track_id,
                    relative_path,
                    start_us,
                    end_us,
                    size_bytes,
                    media_timescale,
                    media_start_value,
                    host_anchor_us,
                    ..
                } => Some(SegmentSummary {
                    track_id,
                    relative_path,
                    start_us,
                    end_us,
                    size_bytes,
                    media_timescale,
                    media_start_value,
                    host_anchor_us,
                    is_keyframe_start: None,
                    available: true,
                }),
                _ => None,
            };
            if let Some(mut segment) = segment {
                let path = safe_path(&root, &segment.relative_path)?;
                if !tracks.contains_key(&segment.track_id) {
                    return Err("Segment references unknown track".into());
                }
                segment_belongs_to_track(&segment.track_id, &segment.relative_path)?;
                if segment.start_us >= segment.end_us || segment.end_us > MAX_SAFE_TIME {
                    return Err("Invalid segment time interval".into());
                }
                duration = duration.max(segment.end_us);
                if !paths.insert(segment.relative_path.clone()) {
                    return Err(format!(
                        "Conflicting duplicate segment: {}",
                        segment.relative_path
                    ));
                }
                segment.available = fs::metadata(path)
                    .map(|m| m.is_file() && m.len() == segment.size_bytes && m.len() > 0)
                    .unwrap_or(false);
                if !segment.available && diagnostics.len() < 256 {
                    diagnostics.push(format!(
                        "Missing or size-mismatched media: {}",
                        segment.relative_path
                    ));
                }
                segments.get_mut(&segment.track_id).unwrap().push(segment);
            }
        }
    } else {
        diagnostics.push("No journal found; no committed media indexed".into());
    }
    let mut summaries = Vec::new();
    for track in &manifest.tracks {
        let entries = segments.get_mut(&track.id).unwrap();
        entries.sort_by_key(|s| (s.start_us, s.end_us));
        let mut previous_end = 0;
        let mut previous_index = 0;
        for i in 0..entries.len() {
            if entries[i].start_us < previous_end {
                entries[i].available = false;
                entries[previous_index].available = false;
                if diagnostics.len() < 256 {
                    diagnostics.push(format!("Overlapping segment: {}", entries[i].relative_path));
                }
            }
            if entries[i].end_us > previous_end {
                previous_end = entries[i].end_us;
                previous_index = i;
            }
        }
        summaries.push(TrackSummary {
            descriptor: track.clone(),
            segment_count: entries.len(),
            available_segment_count: entries.iter().filter(|s| s.available).count(),
        });
    }
    let mut pauses = manifest.pause_intervals.clone();
    pauses.sort_by_key(|p| p.start_us);
    let mut cursor = 0;
    let mut retained = Vec::new();
    for pause in pauses {
        if pause.start_us < cursor || pause.start_us >= pause.end_us || pause.end_us > duration {
            return Err("Invalid or overlapping pause intervals".into());
        }
        if cursor < pause.start_us {
            retained.push(RetainedInterval {
                start_us: cursor,
                end_us: pause.start_us,
                media: None,
                audio_unlinked: false,
            });
        }
        cursor = pause.end_us;
    }
    if cursor < duration {
        retained.push(RetainedInterval {
            start_us: cursor,
            end_us: duration,
            media: None,
            audio_unlinked: false,
        });
    }
    Ok(RecordingIndex {
        manifest,
        segments,
        summaries,
        duration,
        retained,
        diagnostics,
    })
}

impl ProjectReader {
    pub fn open(path: &Path) -> Result<Self, String> {
        if !path.is_absolute() || path.components().any(|p| matches!(p, Component::ParentDir)) {
            return Err("Choose an absolute project directory without traversal".into());
        }
        let meta = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err("Expected a project directory, not a symlink".into());
        }
        let root = dunce::canonicalize(path).map_err(|e| e.to_string())?;
        let lease = acquire_read_lease(&root)?;
        let project_file = load_project_file(&root)?;
        let (index, recording_lease) = match &project_file {
            // A project folder: its recording, if any, lives elsewhere and is only read.
            Some(file) => match &file.recording {
                Some(recording) => {
                    let recording_root = dunce::canonicalize(recording).map_err(|_| {
                        format!(
                            "This project's recording is missing. It was at {recording}; move it back there to open the project."
                        )
                    })?;
                    if recording_root.starts_with(&root) || root.starts_with(&recording_root) {
                        return Err("A project and its recording must be separate folders".into());
                    }
                    let recording_lease = acquire_read_lease(&recording_root)?;
                    reject_writer_lock(&recording_root)?;
                    let mut index = index_recording(&recording_root)?;
                    index.mount_at(RECORDING_MOUNT);
                    mount_recording(&root, &recording_root);
                    (index, Some(recording_lease))
                }
                None => (RecordingIndex::empty(), None),
            },
            // An older recording folder that holds its own edits.
            None => {
                reject_writer_lock(&root)?;
                (index_recording(&root)?, None)
            }
        };
        let RecordingIndex {
            mut manifest,
            segments,
            summaries,
            duration,
            retained,
            mut diagnostics,
        } = index;
        if let Some(file) = &project_file {
            manifest.project_name = file.name.clone();
        }
        let mut history = EditHistory::new(EditDocument::from_retained(retained.clone())?);
        match revision::load_edit_document(&root) {
            Ok(Some(document)) => {
                // Imported media keeps times within its own file.
                if document
                    .retained_intervals
                    .iter()
                    .any(|s| s.is_recording() && s.end_us > duration)
                {
                    return Err("Edit interval exceeds source duration".into());
                }
                history = EditHistory::new(document);
            }
            Ok(None) => {}
            Err(error) => {
                if diagnostics.len() < 256 {
                    diagnostics.push(format!("Edit document ignored: {error}"));
                }
            }
        }
        let retained = history.current.retained_intervals.clone();
        let edited_duration_us = history.current.edited_duration_us()?;
        let zooms = history.current.zooms_with_ranges();
        let mut reader = Self {
            summary: OpenedProject {
                project_handle: uuid::Uuid::new_v4().to_string(),
                revision: history.current.revision,
                manifest,
                source_duration_us: duration,
                edited_duration_us,
                retained_intervals: retained,
                tracks: summaries,
                diagnostics,
                preview_available: false,
                undo_available: history.undo_available(),
                redo_available: history.redo_available(),
                zooms,
                dismissed_zoom_ids: history.current.dismissed_zoom_ids.clone(),
                layout: history.current.layout.clone(),
                project_path: Some(root.to_string_lossy().into_owned()),
                recording_path: match &project_file {
                    Some(file) => file.recording.clone(),
                    None => Some(root.to_string_lossy().into_owned()),
                },
                removed_intervals: Vec::new(),
                split_points_us: Vec::new(),
                webcam_focus: Default::default(),
                audio: history.current.audio.clone(),
                captions: history.current.captions.clone(),
                media_assets: history.current.media_assets.clone(),
                chapters: Vec::new(),
                shorts: Vec::new(),
                overlay_tracks: Vec::new(),
                main_track: history.current.main_track.clone(),
                zoom_settings: history.current.zoom_settings.clone(),
                short_view: None,
            },
            segments,
            root,
            history,
            project_file,
            _lease: lease,
            _recording_lease: recording_lease,
        };
        reader.sync_summary();
        Ok(reader)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Where the recording's own files (manifest, journal, telemetry) are: the linked
    /// recording of a project folder, or the folder itself for an older recording folder.
    pub fn source_root(&self) -> PathBuf {
        mounted_recording(&self.root)
            .filter(|_| self.project_file.is_some())
            .unwrap_or_else(|| self.root.clone())
    }

    /// Whether this project has a recording (a project can start empty).
    pub fn has_recording(&self) -> bool {
        self.project_file
            .as_ref()
            .is_none_or(|file| file.recording.is_some())
    }

    pub fn segments_for(&self, track_id: &str) -> Option<&[SegmentSummary]> {
        self.segments.get(track_id).map(Vec::as_slice)
    }

    pub fn page(&self, track_id: &str, offset: usize, limit: usize) -> Result<SegmentPage, String> {
        if limit == 0 || limit > 256 {
            return Err("Page limit must be 1–256".into());
        }
        let segments = self.segments.get(track_id).ok_or("Unknown track")?;
        if offset > segments.len() {
            return Err("Invalid page offset".into());
        }
        let end = offset.saturating_add(limit).min(segments.len());
        Ok(SegmentPage {
            segments: segments[offset..end].to_vec(),
            next_offset: (end < segments.len()).then_some(end),
        })
    }

    pub fn history(&self) -> &EditHistory {
        &self.history
    }

    pub fn ripple_cuts(
        &mut self,
        expected_revision: u64,
        cuts: &[(u64, u64)],
    ) -> Result<OpenedProject, String> {
        self.history
            .ripple_cuts(expected_revision, cuts, &self.root)?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    pub fn ripple_trim(
        &mut self,
        expected_revision: u64,
        playhead_us: u64,
        side: TrimSide,
    ) -> Result<OpenedProject, String> {
        self.history
            .ripple_trim(expected_revision, playhead_us, side, &self.root)?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    pub fn split(
        &mut self,
        expected_revision: u64,
        edited_us: u64,
    ) -> Result<OpenedProject, String> {
        self.history
            .split(expected_revision, edited_us, &self.root)?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    /// Moves the edited range `[start, end)` (usually one clip) to edited position `target`.
    pub fn move_range(
        &mut self,
        expected_revision: u64,
        start_us: u64,
        end_us: u64,
        target_us: u64,
    ) -> Result<OpenedProject, String> {
        self.history
            .move_range(expected_revision, start_us, end_us, target_us, &self.root)?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    /// Restores removed media inside the requested source ranges. Parts of a
    /// range that were never removed, or that fall in a recorder pause, are
    /// ignored.
    pub fn restore_cuts(
        &mut self,
        expected_revision: u64,
        ranges: &[(u64, u64)],
        grow: crate::project::revision::RestoreGrow,
        shift_tracks_at: Option<u64>,
    ) -> Result<OpenedProject, String> {
        let mut restorable = Vec::new();
        for &(start, end) in ranges {
            if end <= start {
                return Err("Restore range must be a half-open interval".into());
            }
            for removed in &self.summary.removed_intervals {
                let a = start.max(removed.start_us);
                let b = end.min(removed.end_us);
                if a < b {
                    restorable.push((a, b));
                }
            }
        }
        if restorable.is_empty() {
            return Err("Nothing to restore in that range".into());
        }
        self.history.restore(
            expected_revision,
            &restorable,
            grow,
            shift_tracks_at,
            &self.root,
        )?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    pub fn undo(&mut self, expected_revision: u64) -> Result<OpenedProject, String> {
        self.history.undo(expected_revision, &self.root)?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    pub fn redo(&mut self, expected_revision: u64) -> Result<OpenedProject, String> {
        self.history.redo(expected_revision, &self.root)?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    pub fn accept_zooms(
        &mut self,
        expected_revision: u64,
        suggestions: &[crate::zoom::ZoomSuggestion],
    ) -> Result<OpenedProject, String> {
        self.history
            .accept_zooms(expected_revision, suggestions, &self.root)?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    pub fn set_zoom_settings(
        &mut self,
        expected_revision: u64,
        settings: crate::zoom::ZoomSettings,
    ) -> Result<OpenedProject, String> {
        self.history
            .set_zoom_settings(expected_revision, settings, &self.root)?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    pub fn reload_zooms(
        &mut self,
        expected_revision: u64,
        suggestions: &[crate::zoom::ZoomSuggestion],
    ) -> Result<OpenedProject, String> {
        self.history
            .reload_zooms(expected_revision, suggestions, &self.root)?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    pub fn dismiss_zooms(
        &mut self,
        expected_revision: u64,
        ids: &[String],
    ) -> Result<OpenedProject, String> {
        self.history
            .dismiss_zooms(expected_revision, ids, &self.root)?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    pub fn update_zoom(
        &mut self,
        expected_revision: u64,
        patch: crate::zoom::ZoomKeyframe,
    ) -> Result<OpenedProject, String> {
        self.history
            .update_zoom(expected_revision, patch, &self.root)?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    pub fn add_manual_zoom(
        &mut self,
        expected_revision: u64,
        edited_start_us: u64,
        edited_end_us: u64,
        center_x: f64,
        center_y: f64,
        scale: f64,
    ) -> Result<OpenedProject, String> {
        self.history.add_manual_zoom(
            expected_revision,
            edited_start_us,
            edited_end_us,
            center_x,
            center_y,
            scale,
            &self.root,
        )?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    pub fn delete_zoom(
        &mut self,
        expected_revision: u64,
        id: &str,
    ) -> Result<OpenedProject, String> {
        self.history
            .delete_zoom(expected_revision, id, &self.root)?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    pub fn update_layout(
        &mut self,
        expected_revision: u64,
        layout: EditLayout,
    ) -> Result<OpenedProject, String> {
        self.history
            .update_layout(expected_revision, layout, &self.root)?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    pub fn update_webcam_focus(
        &mut self,
        expected_revision: u64,
        focus: crate::webcam_focus::WebcamFocus,
    ) -> Result<OpenedProject, String> {
        self.history
            .update_webcam_focus(expected_revision, focus, &self.root)?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    pub fn add_webcam_focus(
        &mut self,
        expected_revision: u64,
        edited_start_us: u64,
        edited_end_us: u64,
    ) -> Result<OpenedProject, String> {
        self.history.add_webcam_focus(
            expected_revision,
            edited_start_us,
            edited_end_us,
            &self.root,
        )?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    pub fn remove_webcam_focus(
        &mut self,
        expected_revision: u64,
        edited_start_us: u64,
        edited_end_us: u64,
    ) -> Result<OpenedProject, String> {
        self.history.remove_webcam_focus(
            expected_revision,
            edited_start_us,
            edited_end_us,
            &self.root,
        )?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    pub fn set_chapters(
        &mut self,
        expected_revision: u64,
        chapters: Vec<crate::chapters::Chapter>,
    ) -> Result<OpenedProject, String> {
        self.history
            .set_chapters(expected_revision, chapters, &self.root)?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    pub fn set_shorts(
        &mut self,
        expected_revision: u64,
        shorts: Vec<crate::shorts::Short>,
    ) -> Result<OpenedProject, String> {
        self.history
            .set_shorts(expected_revision, shorts, &self.root)?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    pub fn update_audio(
        &mut self,
        expected_revision: u64,
        audio: crate::project::AudioSettings,
    ) -> Result<OpenedProject, String> {
        self.history
            .update_audio(expected_revision, audio, &self.root)?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    /// Copies files into the project and adds them to the media bin. All or nothing: a file
    /// that cannot be imported fails the whole call.
    pub fn import_media(
        &mut self,
        expected_revision: u64,
        paths: &[std::path::PathBuf],
    ) -> Result<OpenedProject, String> {
        if paths.is_empty() || paths.len() > 64 {
            return Err("Choose between 1 and 64 files to import".into());
        }
        let mut assets = Vec::with_capacity(paths.len());
        for path in paths {
            match crate::media_bin::import(&self.root, path) {
                Ok(asset) => assets.push(asset),
                Err(error) => {
                    for asset in &assets {
                        crate::media_bin::remove_files(&self.root, asset);
                    }
                    return Err(error);
                }
            }
        }
        self.add_media(expected_revision, assets)
    }

    /// Records media already copied into the project. On failure the copies are deleted.
    pub fn add_media(
        &mut self,
        expected_revision: u64,
        assets: Vec<crate::media_bin::MediaAsset>,
    ) -> Result<OpenedProject, String> {
        if let Err(error) = self
            .history
            .add_media(expected_revision, assets.clone(), &self.root)
        {
            for asset in &assets {
                crate::media_bin::remove_files(&self.root, asset);
            }
            return Err(error);
        }
        self.sync_summary();
        Ok(self.summary.clone())
    }

    pub fn remove_media(
        &mut self,
        expected_revision: u64,
        asset_id: &str,
    ) -> Result<OpenedProject, String> {
        self.history
            .remove_media(expected_revision, asset_id, &self.root)?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    pub fn set_media_roles(
        &mut self,
        expected_revision: u64,
        asset_id: &str,
        picture_role: crate::media_bin::PictureRole,
        sound_roles: Vec<crate::media_bin::SoundRole>,
    ) -> Result<OpenedProject, String> {
        self.history.set_media_roles(
            expected_revision,
            asset_id,
            picture_role,
            sound_roles,
            &self.root,
        )?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    pub fn insert_media(
        &mut self,
        expected_revision: u64,
        asset_id: &str,
        target_us: u64,
        range: Option<(u64, u64)>,
    ) -> Result<OpenedProject, String> {
        self.history
            .insert_media(expected_revision, asset_id, target_us, range, &self.root)?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    /// The project as short `short_id` sees it: the timeline fields are the short's own (or
    /// its stretch of the video), in the short's time; everything else is the project's.
    pub fn short_view(&self, short_id: &str) -> Result<OpenedProject, String> {
        let base = &self.history.current;
        let short = base
            .shorts
            .iter()
            .find(|s| s.id == short_id)
            .ok_or("That short no longer exists")?;
        let timeline = crate::shorts::short_timeline(base, short)?;
        let mut view = self.summary.clone();
        view.short_view = Some(short_id.to_string());
        view.retained_intervals = timeline.retained_intervals.clone();
        view.split_points_us = timeline.split_points_us.clone();
        view.overlay_tracks = timeline.overlay_tracks.clone();
        view.edited_duration_us = timeline.edited_duration_us()?;
        view.zooms = timeline.zooms_with_ranges();
        let mut focus = base.webcam_focus.clone();
        if let Ok(mapper) = timeline.mapper() {
            focus.attach_edited_ranges(&mapper);
        }
        view.webcam_focus = focus;
        view.chapters = Vec::new();
        view.removed_intervals = revision::removed_intervals(
            &timeline.retained_intervals,
            &self.pauses(),
            self.summary.source_duration_us,
        );
        Ok(view)
    }

    /// The short's view when `short` names one that still exists, else the project.
    pub fn view_or_summary(&self, short: Option<&str>) -> OpenedProject {
        short
            .and_then(|id| self.short_view(id).ok())
            .unwrap_or_else(|| self.summary.clone())
    }

    /// A timeline edit made in the project's timeline or (with `short`) in a short's own.
    pub fn edit_tracks_in(
        &mut self,
        expected_revision: u64,
        edit: &crate::tracks::TrackEdit,
        short: Option<&str>,
    ) -> Result<OpenedProject, String> {
        let Some(short_id) = short else {
            return self.edit_tracks(expected_revision, edit);
        };
        self.history
            .edit_short_tracks(expected_revision, short_id, edit, &self.root)?;
        self.sync_summary();
        self.short_view(short_id)
    }

    /// Lets a short follow the video again.
    pub fn resync_short(
        &mut self,
        expected_revision: u64,
        short_id: &str,
    ) -> Result<OpenedProject, String> {
        self.history
            .resync_short(expected_revision, short_id, &self.root)?;
        self.sync_summary();
        self.short_view(short_id)
    }

    fn pauses(&self) -> Vec<RetainedInterval> {
        self.summary
            .manifest
            .pause_intervals
            .iter()
            .map(|p| RetainedInterval::recording(p.start_us, p.end_us))
            .collect()
    }

    pub fn edit_tracks(
        &mut self,
        expected_revision: u64,
        edit: &crate::tracks::TrackEdit,
    ) -> Result<OpenedProject, String> {
        self.history
            .edit_tracks(expected_revision, edit, &self.root)?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    pub fn update_captions(
        &mut self,
        expected_revision: u64,
        captions: crate::captions::CaptionSettings,
    ) -> Result<OpenedProject, String> {
        self.history
            .update_captions(expected_revision, captions, &self.root)?;
        self.sync_summary();
        Ok(self.summary.clone())
    }

    pub fn rename_project(&mut self, new_name: &str) -> Result<OpenedProject, String> {
        let trimmed = new_name.trim();
        if trimmed.is_empty() {
            return Err("Project name cannot be empty".into());
        }
        let clean_name = display_name_from_input(trimmed);
        if clean_name == self.summary.manifest.project_name {
            return Ok(self.summary.clone());
        }
        // A project folder keeps its own name; the recording is never written.
        if let Some(file) = &mut self.project_file {
            let mut next = file.clone();
            next.name = clean_name.clone();
            save_project_file(&self.root, &next)?;
            *file = next;
            self.summary.manifest.project_name = clean_name;
            return Ok(self.summary.clone());
        }
        let mut manifest = self.summary.manifest.clone();
        manifest.project_name = clean_name;
        manifest
            .save_with_backup(&self.root.join("manifest.json"))
            .map_err(|e| e.to_string())?;
        self.summary.manifest = manifest;
        Ok(self.summary.clone())
    }

    fn sync_summary(&mut self) {
        self.summary.revision = self.history.current.revision;
        self.summary.retained_intervals = self.history.current.retained_intervals.clone();
        self.summary.edited_duration_us = self
            .history
            .current
            .edited_duration_us()
            .unwrap_or(self.summary.edited_duration_us);
        self.summary.undo_available = self.history.undo_available();
        self.summary.redo_available = self.history.redo_available();
        self.summary.zooms = self.history.current.zooms_with_ranges();
        self.summary.dismissed_zoom_ids = self.history.current.dismissed_zoom_ids.clone();
        self.summary.layout = self.history.current.layout.clone();
        self.summary.split_points_us = self.history.current.split_points_us.clone();
        let mut focus = self.history.current.webcam_focus.clone();
        if let Ok(mapper) = self.history.current.mapper() {
            focus.attach_edited_ranges(&mapper);
        }
        self.summary.webcam_focus = focus;
        self.summary.audio = self.history.current.audio.clone();
        self.summary.captions = self.history.current.captions.clone();
        self.summary.media_assets = self.history.current.media_assets.clone();
        for asset in &mut self.summary.media_assets {
            asset.missing = asset
                .file_path(&self.root)
                .map_or(true, |path| !path.exists());
        }
        let mut chapters = self.history.current.chapters.clone();
        if let Ok(mapper) = self.history.current.mapper() {
            crate::chapters::attach_edited(&mut chapters, &mapper);
        }
        self.summary.chapters = chapters;
        let mut shorts = self.history.current.shorts.clone();
        crate::shorts::attach_edited(&mut shorts, &self.history.current);
        self.summary.shorts = shorts;
        self.summary.overlay_tracks = self.history.current.overlay_tracks.clone();
        self.summary.main_track = self.history.current.main_track.clone();
        self.summary.zoom_settings = self.history.current.zoom_settings.clone();
        let pauses: Vec<RetainedInterval> = self
            .summary
            .manifest
            .pause_intervals
            .iter()
            .map(|p| RetainedInterval {
                start_us: p.start_us,
                end_us: p.end_us,
                media: None,
                audio_unlinked: false,
            })
            .collect();
        self.summary.removed_intervals = revision::removed_intervals(
            &self.history.current.retained_intervals,
            &pauses,
            self.summary.source_duration_us,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::TestProject;
    use crate::project::manifest::PauseInterval;

    /// `fs::canonicalize` returns verbatim `\\?\C:\...` paths on Windows, which leak into
    /// FFmpeg arguments, the UI and comparisons with user-chosen paths.
    #[cfg(windows)]
    #[test]
    fn open_root_has_no_verbatim_prefix_on_windows() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = TestProject::create(dir.path(), "verbatim");
        let root = bundle.root_path().to_path_buf();
        drop(bundle);
        let reader = ProjectReader::open(&root).unwrap();
        let shown = reader.root().to_string_lossy().into_owned();
        assert!(!shown.starts_with(r"\\?\"), "verbatim root {shown}");
        assert!(reader.root().is_absolute());

        let parent = dir.path().join("exports");
        fs::create_dir_all(&parent).unwrap();
        let requested = parent.join("out.mp4");
        let resolved = crate::export::resolve_destination(
            reader.root(),
            "verbatim",
            0,
            Some(requested.to_str().unwrap()),
            &[],
        )
        .unwrap();
        let shown = resolved.to_string_lossy().into_owned();
        assert!(!shown.starts_with(r"\\?\"), "verbatim destination {shown}");
        assert_eq!(resolved.file_name().unwrap(), "out.mp4");
        // Traversal and the inside-bundle check still apply.
        assert!(ProjectReader::open(&root.join("..").join("verbatim.aero")).is_err());
        let inside = reader.root().join("out.mp4");
        assert!(crate::export::resolve_destination(
            reader.root(),
            "verbatim",
            0,
            Some(inside.to_str().unwrap()),
            &[],
        )
        .is_err());
    }

    #[test]
    fn restore_cuts_never_restores_pauses_or_uncut_media() {
        let dir = tempfile::tempdir().unwrap();
        let mut bundle = TestProject::create(dir.path(), "restore");
        bundle.manifest_mut().duration_us = 10_000_000;
        bundle.manifest_mut().active_duration_us = 9_000_000;
        bundle.manifest_mut().pause_intervals = vec![PauseInterval {
            start_us: 6_000_000,
            end_us: 7_000_000,
        }];
        bundle.save_manifest();
        let root = bundle.root_path().to_path_buf();
        drop(bundle);
        let mut reader = ProjectReader::open(&root).unwrap();
        assert!(reader.summary.removed_intervals.is_empty());
        assert!(reader
            .restore_cuts(0, &[(0, 10_000_000)], Default::default(), None)
            .is_err());

        let summary = reader.ripple_cuts(0, &[(1_000_000, 2_000_000)]).unwrap();
        assert_eq!(
            summary.removed_intervals,
            vec![RetainedInterval {
                start_us: 1_000_000,
                end_us: 2_000_000,
                media: None,
                audio_unlinked: false,
            }]
        );
        let summary = reader
            .restore_cuts(1, &[(0, 10_000_000)], Default::default(), None)
            .unwrap();
        assert!(summary.removed_intervals.is_empty());
        assert_eq!(
            summary.retained_intervals,
            vec![
                RetainedInterval {
                    start_us: 0,
                    end_us: 6_000_000,
                    media: None,
                    audio_unlinked: false,
                },
                RetainedInterval {
                    start_us: 7_000_000,
                    end_us: 10_000_000,
                    media: None,
                    audio_unlinked: false,
                },
            ]
        );

        let summary = reader.split(2, 3_000_000).unwrap();
        assert_eq!(summary.split_points_us, vec![3_000_000]);
        assert_eq!(summary.revision, 3);
    }
}
