//! Bounded, non-repairing project inspection. Source files are never modified.
use super::{
    display_name_from_input,
    folder::{load_project_file, resolve_mount, save_project_file, ProjectFile},
    journal::JournalRecord,
    layout::EditLayout,
    manifest::{ProjectManifest, TrackDescriptor},
    revision::{self, EditDocument, EditHistory},
};
use crate::sequence::edit::{EditOutcome, SequenceEdit};
use crate::sequence::{Asset, Role, Sequence};
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

/// One entry of a clock: a range of an asset's time playing on the timeline, or (with
/// `media` set) time where it does not play. See [`crate::sequence::clock`].
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RetainedInterval {
    pub start_us: u64,
    pub end_us: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
}

impl RetainedInterval {
    /// A range of the asset's time.
    pub fn recording(start_us: u64, end_us: u64) -> Self {
        Self {
            start_us,
            end_us,
            media: None,
        }
    }

    pub fn is_recording(&self) -> bool {
        self.media.is_none()
    }
}

/// The project as the UI sees it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OpenedProject {
    pub project_handle: String,
    pub revision: u64,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    /// Where the timeline ends.
    pub duration_us: u64,
    /// Everything that can be played, with `missing` worked out.
    pub assets: Vec<Asset>,
    /// The timeline: video tracks bottom to top, then audio tracks top to bottom.
    pub sequence: Sequence,
    /// Frames per second the timeline steps by: the first recording's screen, else 30.
    pub fps: u32,
    pub diagnostics: Vec<String>,
    pub preview_available: bool,
    pub undo_available: bool,
    pub redo_available: bool,
    #[serde(default)]
    pub zooms: Vec<ZoomKeyframe>,
    #[serde(default)]
    pub dismissed_zoom_ids: Vec<String>,
    #[serde(default)]
    pub zoom_settings: crate::zoom::ZoomSettings,
    #[serde(default)]
    pub layout: EditLayout,
    #[serde(default)]
    pub webcam_focus: crate::webcam_focus::WebcamFocus,
    pub audio: crate::project::AudioSettings,
    #[serde(default)]
    pub captions: crate::captions::CaptionSettings,
    #[serde(default)]
    pub chapters: Vec<crate::chapters::Chapter>,
    #[serde(default)]
    pub shorts: Vec<crate::shorts::Short>,
    /// Set when this is short `id`'s own timeline (the sequence is the short's).
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
    root: PathBuf,
    history: EditHistory,
    project_file: ProjectFile,
    // Directory advisory lease. It creates no lock file and holds the project against
    // cooperating writers.
    _lease: File,
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
    // A project sees its recordings at `recordings/<asset id>/`.
    let (root, relative) = match resolve_mount(root, relative) {
        Some((folder, inside)) => (folder, inside),
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
/// how long it is.
struct RecordingIndex {
    manifest: ProjectManifest,
    segments: HashMap<String, Vec<SegmentSummary>>,
    summaries: Vec<TrackSummary>,
    duration: u64,
    diagnostics: Vec<String>,
}

fn reject_writer_lock(root: &Path) -> Result<(), String> {
    // Reject old writers/stale locks too. Recovery, not open, owns repairs.
    if fs::symlink_metadata(root.join(".lock")).is_ok() {
        return Err("Project has a writer lock; close recording or recover it first".into());
    }
    Ok(())
}

/// The tracks, with their segments, and the length of the recording in `folder`.
pub fn recording_tracks(
    folder: &Path,
) -> Result<(Vec<(TrackSummary, Vec<SegmentSummary>)>, u64), String> {
    let (_, tracks, duration, _) = read_recording(folder)?;
    Ok((tracks, duration))
}

/// Everything about the recording in `folder`: its manifest, its tracks with their segments
/// (paths relative to the folder), its length, and what was wrong with it.
pub(crate) type RecordingRead = (
    ProjectManifest,
    Vec<(TrackSummary, Vec<SegmentSummary>)>,
    u64,
    Vec<String>,
);

pub(crate) fn read_recording(folder: &Path) -> Result<RecordingRead, String> {
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
    Ok((index.manifest, tracks, index.duration, index.diagnostics))
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
        let mut previous_end = 0u64;
        let mut previous_index = 0;
        for i in 0..entries.len() {
            // A file that is missing plays nothing, so it neither overlaps nor cuts anything.
            if !entries[i].available {
                continue;
            }
            // A long recording is written in rolling segments, and each one starts a moment
            // before the last ends (a frame of video, a few ms of sound). The earlier segment
            // then ends where the next begins, so nothing plays twice. A longer overlap is a
            // real conflict.
            let overlap = previous_end.saturating_sub(entries[i].start_us);
            if overlap > 0
                && overlap <= MAX_SEGMENT_HANDOVER_US
                && entries[i].end_us > previous_end
                && entries[i].start_us > entries[previous_index].start_us
                && entries[previous_index].available
            {
                entries[previous_index].end_us = entries[i].start_us;
                previous_end = entries[i].end_us;
                previous_index = i;
                continue;
            }
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
    for pause in pauses {
        if pause.start_us < cursor || pause.start_us >= pause.end_us || pause.end_us > duration {
            return Err("Invalid or overlapping pause intervals".into());
        }
        cursor = pause.end_us;
    }
    Ok(RecordingIndex {
        manifest,
        segments,
        summaries,
        duration,
        diagnostics,
    })
}

/// How far one segment of a track may start before the last one ends and still be taken
/// as the next (see the rolling segments in `read_recording`).
const MAX_SEGMENT_HANDOVER_US: u64 = 250_000;

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
        let Some(project_file) = load_project_file(&root)? else {
            return Err(if root.join("manifest.json").is_file() {
                "That folder is a recording. Make a new project from it to edit it.".into()
            } else {
                "That folder is not an AeroEdits project".into()
            });
        };
        let mut diagnostics = Vec::new();
        let document = match revision::load_edit_document(&root) {
            Ok(Some(document)) => document,
            Ok(None) => Self::starting_document(&root, &project_file)?,
            Err(error) => {
                // Kept under another name, so the first save cannot overwrite it.
                let kept = revision::set_aside_edit_document(&root).map_err(|e| {
                    format!("This project's edit could not be read ({error}) or set aside ({e})")
                })?;
                diagnostics.push(format!(
                    "The edit saved here could not be read ({error}); it was kept as {kept} and the project starts fresh"
                ));
                Self::starting_document(&root, &project_file)?
            }
        };
        let history = EditHistory::new(document);
        let mut reader = Self {
            summary: OpenedProject {
                project_handle: uuid::Uuid::new_v4().to_string(),
                revision: 0,
                name: project_file.name.clone(),
                project_path: Some(root.to_string_lossy().into_owned()),
                duration_us: 0,
                assets: Vec::new(),
                sequence: Sequence::default(),
                fps: 30,
                diagnostics,
                preview_available: false,
                undo_available: false,
                redo_available: false,
                zooms: Vec::new(),
                dismissed_zoom_ids: Vec::new(),
                zoom_settings: Default::default(),
                layout: EditLayout::default(),
                webcam_focus: Default::default(),
                audio: Default::default(),
                captions: Default::default(),
                chapters: Vec::new(),
                shorts: Vec::new(),
                short_view: None,
            },
            root,
            history,
            project_file,
            _lease: lease,
        };
        reader.index_recordings();
        reader.sync_summary();
        Ok(reader)
    }

    /// A new project's document: its recording on the timeline, saved at once so the
    /// recording keeps its id (transcripts are kept under it).
    fn starting_document(root: &Path, file: &ProjectFile) -> Result<EditDocument, String> {
        let Some(recording) = &file.recording else {
            return Ok(EditDocument::default());
        };
        let folder = dunce::canonicalize(recording).map_err(|_| {
            format!(
                "This project's recording is missing. It was at {recording}; move it back there to open the project."
            )
        })?;
        if folder.starts_with(root) || root.starts_with(&folder) {
            return Err("A project and its recording must be separate folders".into());
        }
        reject_writer_lock(&folder)?;
        let asset = crate::sequence::sources::recording_asset(
            &folder,
            crate::sequence::sources::new_asset_id(true),
        )?;
        let document = EditDocument::from_recording(asset)?;
        revision::save_edit_document(root, &document)?;
        Ok(document)
    }

    /// Mounts every recording in the project and notes what is wrong with any of them.
    fn index_recordings(&mut self) {
        for asset in self
            .history
            .current
            .assets
            .iter()
            .filter(|a| a.is_recording())
        {
            match crate::sequence::sources::recording(&self.root, asset) {
                Ok(files) => {
                    for line in files.diagnostics.iter().take(32) {
                        self.summary
                            .diagnostics
                            .push(format!("{}: {line}", asset.name));
                    }
                }
                Err(error) => self.summary.diagnostics.push(error),
            }
        }
        self.summary.diagnostics.truncate(256);
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn history(&self) -> &EditHistory {
        &self.history
    }

    pub fn document(&self) -> &EditDocument {
        &self.history.current
    }

    /// The recordings in the project, first the one it was made from.
    pub fn recordings(&self) -> impl Iterator<Item = &Asset> {
        self.history
            .current
            .assets
            .iter()
            .filter(|a| a.is_recording())
    }

    /// A segment page of one stream of a recording (`<asset>.<stream>`).
    pub fn page(&self, key: &str, offset: usize, limit: usize) -> Result<SegmentPage, String> {
        if limit == 0 || limit > 256 {
            return Err("Page limit must be 1–256".into());
        }
        let source = crate::sequence::StreamRef::parse(key).ok_or("Unknown stream")?;
        let asset = self
            .history
            .current
            .asset(&source.asset)
            .ok_or("Unknown stream")?;
        let segments =
            match crate::sequence::sources::stream_source(&self.root, asset, &source.stream)? {
                crate::sequence::sources::StreamSource::Segments(segments) => segments,
                _ => Vec::new(),
            };
        if offset > segments.len() {
            return Err("Invalid page offset".into());
        }
        let end = offset.saturating_add(limit).min(segments.len());
        Ok(SegmentPage {
            segments: segments[offset..end].to_vec(),
            next_offset: (end < segments.len()).then_some(end),
        })
    }

    fn done(&mut self) -> OpenedProject {
        self.sync_summary();
        self.summary.clone()
    }

    /// One timeline edit in the project's timeline or (with `short`) in a short's own.
    pub fn edit_sequence(
        &mut self,
        expected_revision: u64,
        edit: &SequenceEdit,
        short: Option<&str>,
    ) -> Result<(OpenedProject, EditOutcome), String> {
        match short {
            None => {
                let outcome = self
                    .history
                    .edit_sequence(expected_revision, edit, &self.root)?;
                Ok((self.done(), outcome))
            }
            Some(short_id) => {
                let outcome = self.history.edit_short_sequence(
                    expected_revision,
                    short_id,
                    edit,
                    &self.root,
                )?;
                self.sync_summary();
                Ok((self.short_view(short_id)?, outcome))
            }
        }
    }

    pub fn ripple_cuts(
        &mut self,
        expected_revision: u64,
        cuts: &[(u64, u64)],
    ) -> Result<OpenedProject, String> {
        self.history
            .ripple_cuts(expected_revision, cuts, &self.root)?;
        Ok(self.done())
    }

    pub fn undo(&mut self, expected_revision: u64) -> Result<OpenedProject, String> {
        self.history.undo(expected_revision, &self.root)?;
        self.index_recordings_quietly();
        Ok(self.done())
    }

    pub fn redo(&mut self, expected_revision: u64) -> Result<OpenedProject, String> {
        self.history.redo(expected_revision, &self.root)?;
        self.index_recordings_quietly();
        Ok(self.done())
    }

    /// Undo or redo can bring back a recording: mount it again.
    fn index_recordings_quietly(&self) {
        for asset in self.recordings() {
            let _ = crate::sequence::sources::recording(&self.root, asset);
        }
    }

    pub fn accept_zooms(
        &mut self,
        expected_revision: u64,
        suggestions: &[crate::zoom::ZoomSuggestion],
    ) -> Result<OpenedProject, String> {
        self.history
            .accept_zooms(expected_revision, suggestions, &self.root)?;
        Ok(self.done())
    }

    pub fn set_zoom_settings(
        &mut self,
        expected_revision: u64,
        settings: crate::zoom::ZoomSettings,
    ) -> Result<OpenedProject, String> {
        self.history
            .set_zoom_settings(expected_revision, settings, &self.root)?;
        Ok(self.done())
    }

    pub fn reload_zooms(
        &mut self,
        expected_revision: u64,
        suggestions: &[crate::zoom::ZoomSuggestion],
    ) -> Result<OpenedProject, String> {
        self.history
            .reload_zooms(expected_revision, suggestions, &self.root)?;
        Ok(self.done())
    }

    pub fn dismiss_zooms(
        &mut self,
        expected_revision: u64,
        ids: &[String],
    ) -> Result<OpenedProject, String> {
        self.history
            .dismiss_zooms(expected_revision, ids, &self.root)?;
        Ok(self.done())
    }

    pub fn update_zoom(
        &mut self,
        expected_revision: u64,
        patch: crate::zoom::ZoomKeyframe,
    ) -> Result<OpenedProject, String> {
        self.history
            .update_zoom(expected_revision, patch, &self.root)?;
        Ok(self.done())
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
        Ok(self.done())
    }

    pub fn delete_zoom(
        &mut self,
        expected_revision: u64,
        id: &str,
    ) -> Result<OpenedProject, String> {
        self.history
            .delete_zoom(expected_revision, id, &self.root)?;
        Ok(self.done())
    }

    pub fn update_layout(
        &mut self,
        expected_revision: u64,
        layout: EditLayout,
    ) -> Result<OpenedProject, String> {
        self.history
            .update_layout(expected_revision, layout, &self.root)?;
        Ok(self.done())
    }

    pub fn update_webcam_focus(
        &mut self,
        expected_revision: u64,
        focus: crate::webcam_focus::WebcamFocus,
    ) -> Result<OpenedProject, String> {
        self.history
            .update_webcam_focus(expected_revision, focus, &self.root)?;
        Ok(self.done())
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
        Ok(self.done())
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
        Ok(self.done())
    }

    pub fn set_chapters(
        &mut self,
        expected_revision: u64,
        chapters: Vec<crate::chapters::Chapter>,
    ) -> Result<OpenedProject, String> {
        self.history
            .set_chapters(expected_revision, chapters, &self.root)?;
        Ok(self.done())
    }

    pub fn set_shorts(
        &mut self,
        expected_revision: u64,
        shorts: Vec<crate::shorts::Short>,
    ) -> Result<OpenedProject, String> {
        self.history
            .set_shorts(expected_revision, shorts, &self.root)?;
        Ok(self.done())
    }

    pub fn update_audio(
        &mut self,
        expected_revision: u64,
        audio: crate::project::AudioSettings,
    ) -> Result<OpenedProject, String> {
        self.history
            .update_audio(expected_revision, audio, &self.root)?;
        Ok(self.done())
    }

    pub fn update_captions(
        &mut self,
        expected_revision: u64,
        captions: crate::captions::CaptionSettings,
    ) -> Result<OpenedProject, String> {
        self.history
            .update_captions(expected_revision, captions, &self.root)?;
        Ok(self.done())
    }

    /// Adds files (or recording folders) to the project, referenced where they are; their
    /// sound is extracted once. All or nothing: a file that cannot be imported fails the call.
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
        self.add_assets(expected_revision, assets)
    }

    /// Records media already described for the project. On failure its extracted files go.
    pub fn add_assets(
        &mut self,
        expected_revision: u64,
        assets: Vec<Asset>,
    ) -> Result<OpenedProject, String> {
        if let Err(error) = self
            .history
            .add_assets(expected_revision, assets.clone(), &self.root)
        {
            for asset in &assets {
                crate::media_bin::remove_files(&self.root, asset);
            }
            return Err(error);
        }
        self.index_recordings();
        Ok(self.done())
    }

    pub fn remove_asset(
        &mut self,
        expected_revision: u64,
        asset_id: &str,
    ) -> Result<OpenedProject, String> {
        self.history
            .remove_asset(expected_revision, asset_id, &self.root)?;
        Ok(self.done())
    }

    pub fn set_stream_roles(
        &mut self,
        expected_revision: u64,
        asset_id: &str,
        roles: &[(String, Role)],
    ) -> Result<OpenedProject, String> {
        self.history
            .set_stream_roles(expected_revision, asset_id, roles, &self.root)?;
        Ok(self.done())
    }

    /// The project as short `short_id` sees it: the sequence is the short's own (or its
    /// stretch of the video), in the short's time; everything else is the project's.
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
        view.sequence = timeline.sequence.clone();
        view.duration_us = timeline.duration_us();
        view.zooms = timeline.zooms_with_ranges();
        let mut focus = timeline.webcam_focus.clone();
        focus.attach_edited_ranges(&timeline.focus_clock());
        view.webcam_focus = focus;
        view.chapters = Vec::new();
        Ok(view)
    }

    /// The short's view when `short` names one that still exists, else the project.
    pub fn view_or_summary(&self, short: Option<&str>) -> OpenedProject {
        short
            .and_then(|id| self.short_view(id).ok())
            .unwrap_or_else(|| self.summary.clone())
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

    pub fn rename_project(&mut self, new_name: &str) -> Result<OpenedProject, String> {
        let trimmed = new_name.trim();
        if trimmed.is_empty() {
            return Err("Project name cannot be empty".into());
        }
        let clean_name = display_name_from_input(trimmed);
        if clean_name == self.summary.name {
            return Ok(self.summary.clone());
        }
        // A project folder keeps its own name; recordings are never written.
        let mut next = self.project_file.clone();
        next.name = clean_name.clone();
        save_project_file(&self.root, &next)?;
        self.project_file = next;
        self.summary.name = clean_name;
        Ok(self.summary.clone())
    }

    fn sync_summary(&mut self) {
        let document = &self.history.current;
        self.summary.revision = document.revision;
        self.summary.duration_us = document.duration_us();
        self.summary.sequence = document.sequence.clone();
        self.summary.assets = document.assets.clone();
        for asset in &mut self.summary.assets {
            asset.missing = !Path::new(&asset.path).exists();
        }
        self.summary.fps = document
            .assets
            .iter()
            .filter(|a| a.is_recording())
            .flat_map(|a| a.streams.iter())
            .find(|s| s.role == Role::Screen)
            .and_then(|s| s.fps)
            .filter(|&fps| fps > 0)
            .unwrap_or(30);
        self.summary.undo_available = self.history.undo_available();
        self.summary.redo_available = self.history.redo_available();
        self.summary.zooms = document.zooms_with_ranges();
        self.summary.dismissed_zoom_ids = document.dismissed_zoom_ids.clone();
        self.summary.zoom_settings = document.zoom_settings.clone();
        self.summary.layout = document.layout.clone();
        let mut focus = document.webcam_focus.clone();
        focus.attach_edited_ranges(&document.focus_clock());
        self.summary.webcam_focus = focus;
        self.summary.audio = document.audio.clone();
        self.summary.captions = document.captions.clone();
        let mut chapters = document.chapters.clone();
        crate::chapters::attach_edited(&mut chapters, document);
        self.summary.chapters = chapters;
        let mut shorts = document.shorts.clone();
        crate::shorts::attach_edited(&mut shorts, document);
        self.summary.shorts = shorts;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::TestProject;
    use crate::project::manifest::PauseInterval;

    /// Rolling segments that each start a moment before the last ends play one after another;
    /// a real overlap still makes both unavailable.
    #[test]
    fn rolling_segments_hand_over_and_real_overlaps_are_flagged() {
        use crate::fixtures::generate_pcm16_wav;
        use crate::project::manifest::{TrackDescriptor, TrackType};
        let dir = tempfile::tempdir().unwrap();
        let mut bundle = TestProject::create(dir.path(), "rolling");
        let wav = generate_pcm16_wav(48_000, 1, &vec![1_000i16; 48_000]);
        bundle.manifest_mut().tracks.push(TrackDescriptor {
            id: "mic".into(),
            track_type: TrackType::MicAudio,
            codec: "pcm".into(),
            relative_path: "media/mic/000001.wav".into(),
            width: None,
            height: None,
            fps: None,
            sample_rate: Some(48_000),
            channels: Some(1),
            gaps_total: 0,
            media_timescale: Some(48_000),
        });
        // 0-1 s, 0.998-2 s (2 ms early, as the recorder writes them), then 1.5-3 s (a
        // half-second conflict).
        // Then 3-4 s exactly adjacent, a gap, 4.5-5.5 s, a 4 us handover (as the screen's),
        // and a file that is missing on disk.
        let parts = [
            (0, 1_000_000),
            (998_000, 2_000_000),
            (1_500_000, 3_000_000),
            (3_000_000, 4_000_000),
            (4_500_000, 5_500_000),
            (5_499_996, 6_500_000),
            (6_499_000, 7_000_000),
        ];
        for (n, (start_us, end_us)) in parts.into_iter().enumerate() {
            let relative = format!("media/mic/{:06}.wav", n + 1);
            if n != 6 {
                std::fs::write(bundle.root_path().join(&relative), &wav).unwrap();
            }
            bundle.append_journal(JournalRecord::SegmentCommitted {
                seq: n as u64,
                track_id: "mic".into(),
                relative_path: relative,
                start_us,
                end_us,
                size_bytes: wav.len() as u64,
                is_keyframe_start: true,
                media_timescale: 48_000,
                media_start_value: 0,
                host_anchor_us: start_us as i64,
            });
        }
        bundle.manifest_mut().duration_us = 7_000_000;
        bundle.manifest_mut().active_duration_us = 7_000_000;
        bundle.save_manifest();
        let (_, tracks, _, diagnostics) = read_recording(bundle.root_path()).unwrap();
        let spans: Vec<_> = tracks[0]
            .1
            .iter()
            .map(|s| (s.start_us, s.end_us, s.available))
            .collect();
        assert_eq!(
            spans,
            vec![
                (0, 998_000, true),
                (998_000, 2_000_000, false),
                (1_500_000, 3_000_000, false),
                (3_000_000, 4_000_000, true),
                (4_500_000, 5_499_996, true),
                (5_499_996, 6_500_000, true),
                // Missing: not available, and nothing before it is cut short for it.
                (6_499_000, 7_000_000, false),
            ]
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|d| d.starts_with("Overlapping segment"))
                .count(),
            1
        );
    }

    #[test]
    fn rolling_audio_handover_plays_each_side_once_without_changing_the_recording() {
        use crate::fixtures::generate_pcm16_wav;
        use crate::project::manifest::TrackType;
        let dir = tempfile::tempdir().unwrap();
        let mut bundle = TestProject::create(dir.path(), "handover");
        bundle.manifest_mut().tracks.push(TrackDescriptor {
            id: "mic".into(),
            track_type: TrackType::MicAudio,
            codec: "pcm".into(),
            relative_path: "media/mic/000001.wav".into(),
            width: None,
            height: None,
            fps: None,
            sample_rate: Some(48_000),
            channels: Some(1),
            gaps_total: 0,
            media_timescale: Some(48_000),
        });
        for (i, start_us, end_us, sample, frames) in [
            (1, 0, 1_000_000, 1_000, 48_000),
            (2, 998_000, 2_000_000, 2_000, 48_096),
        ] {
            let relative = format!("media/mic/{i:06}.wav");
            let wav = generate_pcm16_wav(48_000, 1, &vec![sample; frames]);
            fs::write(bundle.root_path().join(&relative), &wav).unwrap();
            bundle.append_journal(JournalRecord::SegmentCommitted {
                seq: i,
                track_id: "mic".into(),
                relative_path: relative,
                start_us,
                end_us,
                size_bytes: wav.len() as u64,
                is_keyframe_start: true,
                media_timescale: 48_000,
                media_start_value: 0,
                host_anchor_us: start_us as i64,
            });
        }
        bundle.manifest_mut().duration_us = 2_000_000;
        bundle.manifest_mut().active_duration_us = 2_000_000;
        bundle.save_manifest();
        let journal = fs::read(bundle.root_path().join("journal.jsonl")).unwrap();
        let manifest = fs::read(bundle.root_path().join("manifest.json")).unwrap();
        let project = project_for(bundle.root_path(), &dir.path().join("Projects"));
        let reader = ProjectReader::open(&project).unwrap();
        let mixer = crate::media::audio::AudioMixer::new(reader.root(), reader.document()).unwrap();
        // 998,000 us = frame 47,904. The old file ends and the new file starts there.
        let samples = mixer.read_frames(47_894, 30).unwrap();
        for (i, stereo) in samples.chunks_exact(2).enumerate() {
            let expected = if i < 10 { 1_000 } else { 2_000 };
            assert!(
                stereo.iter().all(|&s| (s - expected).abs() <= 1),
                "frame {i}: {stereo:?}"
            );
        }
        assert_eq!(
            fs::read(bundle.root_path().join("journal.jsonl")).unwrap(),
            journal
        );
        assert_eq!(
            fs::read(bundle.root_path().join("manifest.json")).unwrap(),
            manifest
        );
    }

    /// A project folder made from a recording, as the app makes them.
    fn project_for(recording: &Path, parent: &Path) -> PathBuf {
        crate::project::folder::create_project_folder(parent, "Edit", Some(recording)).unwrap()
    }

    /// `fs::canonicalize` returns verbatim `\?\C:\...` paths on Windows, which leak into
    /// FFmpeg arguments, the UI and comparisons with user-chosen paths.
    #[cfg(windows)]
    #[test]
    fn open_root_has_no_verbatim_prefix_on_windows() {
        let dir = tempfile::tempdir().unwrap();
        let root =
            crate::project::folder::create_project_folder(dir.path(), "verbatim", None).unwrap();
        let reader = ProjectReader::open(&root).unwrap();
        let shown = reader.root().to_string_lossy().into_owned();
        assert!(!shown.starts_with(r"\?\"), "verbatim root {shown}");
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
        assert!(!shown.starts_with(r"\?\"), "verbatim destination {shown}");
        assert_eq!(resolved.file_name().unwrap(), "out.mp4");
        assert!(ProjectReader::open(&root.join("..").join("verbatim")).is_err());
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

    /// A recording with pauses opens as one linked clip set per recorded stretch, keeps its
    /// asset id across opens, and a recording folder itself is not a project.
    #[test]
    fn a_new_project_lays_out_its_recording_between_pauses() {
        let dir = tempfile::tempdir().unwrap();
        let mut bundle = TestProject::create(dir.path(), "take");
        bundle
            .manifest_mut()
            .tracks
            .push(crate::project::TrackDescriptor {
                id: "mic".into(),
                track_type: crate::project::TrackType::MicAudio,
                codec: "pcm".into(),
                relative_path: "media/mic/000001.wav".into(),
                width: None,
                height: None,
                fps: None,
                sample_rate: Some(48_000),
                channels: Some(1),
                gaps_total: 0,
                media_timescale: Some(48_000),
            });
        bundle.manifest_mut().duration_us = 10_000_000;
        bundle.manifest_mut().active_duration_us = 9_000_000;
        bundle.manifest_mut().pause_intervals = vec![PauseInterval {
            start_us: 6_000_000,
            end_us: 7_000_000,
        }];
        bundle.save_manifest();
        let recording = bundle.root_path().to_path_buf();
        drop(bundle);
        assert!(ProjectReader::open(&recording)
            .err()
            .unwrap()
            .contains("recording"));
        let folder = project_for(&recording, &dir.path().join("Projects"));
        let mut reader = ProjectReader::open(&folder).unwrap();
        assert_eq!(reader.summary.name, "Edit");
        assert_eq!(reader.summary.duration_us, 9_000_000);
        let asset = reader.summary.assets[0].clone();
        assert!(asset.is_recording() && !asset.missing);
        assert_eq!(asset.pauses.len(), 1);
        let v1 = &reader.summary.sequence.tracks[0];
        let spans: Vec<_> = v1
            .clips
            .iter()
            .map(|c| (c.start_us, c.in_us, c.duration_us))
            .collect();
        assert_eq!(
            spans,
            vec![(0, 0, 6_000_000), (6_000_000, 7_000_000, 3_000_000)]
        );
        let summary = reader.ripple_cuts(0, &[(1_000_000, 2_000_000)]).unwrap();
        assert_eq!(summary.duration_us, 8_000_000);
        drop(reader);
        let reopened = ProjectReader::open(&folder).unwrap();
        assert_eq!(reopened.summary.assets[0].id, asset.id);
        assert_eq!(reopened.summary.duration_us, 8_000_000);
    }
}
