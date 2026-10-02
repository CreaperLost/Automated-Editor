//! Versioned edit document (`project.json`). Source media is never rewritten.
use super::layout::validate_layout;
use super::reader::{open_regular, safe_path, RetainedInterval};
use crate::timeline::{SourceInterval, TimelineMapper};
use crate::webcam_focus::WebcamFocus;
use crate::zoom::{
    attach_zoom_edited_ranges, validate_zooms, ZoomKeyframe, ZoomSource, ZoomSuggestion,
    MAX_DISMISSED_ZOOMS, MAX_ZOOMS,
};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

pub use super::audio::AudioSettings;
pub use super::layout::EditLayout;
pub use crate::captions::CaptionSettings;

pub const EDIT_SCHEMA_VERSION: u32 = 1;
pub const MAX_EDIT_BYTES: u64 = 1_048_576;
pub const MAX_RETAINED_INTERVALS: usize = 10_000;
pub const MAX_UNDO: usize = 64;
pub const MAX_CUTS_PER_REVISION: usize = 256;
pub const MAX_SPLIT_POINTS: usize = 10_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditDocument {
    pub schema_version: u32,
    pub revision: u64,
    pub retained_intervals: Vec<RetainedInterval>,
    #[serde(default)]
    pub layout: EditLayout,
    #[serde(default)]
    pub zooms: Vec<ZoomKeyframe>,
    #[serde(default)]
    pub dismissed_zoom_ids: Vec<String>,
    /// Source timestamps where the user split a clip. A split never removes
    /// media, so playback and export ignore it; the timeline draws clip edges
    /// at the points that fall inside retained media.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub split_points_us: Vec<u64>,
    /// Auto webcam layout: when the webcam grows to fill the canvas.
    #[serde(default, skip_serializing_if = "WebcamFocus::is_default")]
    pub webcam_focus: WebcamFocus,
    /// Loudness, noise reduction and ducking. Applies to playback and export.
    #[serde(default, skip_serializing_if = "AudioSettings::is_default")]
    pub audio: AudioSettings,
    /// Captions burned into playback and export from a track's transcript.
    #[serde(default, skip_serializing_if = "CaptionSettings::is_default")]
    pub captions: CaptionSettings,
    /// Videos, images and audio imported into the project; timeline entries refer to them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub media_assets: Vec<crate::media_bin::MediaAsset>,
}

impl Default for EditDocument {
    fn default() -> Self {
        Self {
            schema_version: EDIT_SCHEMA_VERSION,
            revision: 0,
            retained_intervals: Vec::new(),
            layout: EditLayout::default(),
            zooms: Vec::new(),
            dismissed_zoom_ids: Vec::new(),
            split_points_us: Vec::new(),
            webcam_focus: WebcamFocus::default(),
            audio: AudioSettings::default(),
            captions: CaptionSettings::default(),
            media_assets: Vec::new(),
        }
    }
}

impl EditDocument {
    pub fn from_retained(retained: Vec<RetainedInterval>) -> Result<Self, String> {
        validate_retained(&retained)?;
        Ok(Self {
            schema_version: EDIT_SCHEMA_VERSION,
            revision: 0,
            retained_intervals: retained,
            layout: EditLayout::default(),
            zooms: Vec::new(),
            dismissed_zoom_ids: Vec::new(),
            split_points_us: Vec::new(),
            webcam_focus: WebcamFocus::default(),
            audio: AudioSettings::default(),
            captions: CaptionSettings::default(),
            media_assets: Vec::new(),
        })
    }

    pub fn mapper(&self) -> Result<TimelineMapper, String> {
        mapper_for(&self.retained_intervals)
    }

    pub fn zoom_suggestions(&self) -> Vec<ZoomSuggestion> {
        self.zooms.iter().map(ZoomKeyframe::as_suggestion).collect()
    }

    pub fn attach_zoom_ranges(&mut self) -> Result<(), String> {
        let mapper = self.mapper()?;
        attach_zoom_edited_ranges(&mut self.zooms, &mapper);
        Ok(())
    }

    pub fn edited_duration_us(&self) -> Result<u64, String> {
        Ok(self.mapper()?.total_edited_duration_us())
    }

    /// Edit points on the edited timeline: the start, every cut, every split
    /// inside retained media, and the end. Sorted and unique.
    pub fn clip_edges_edited(&self) -> Vec<u64> {
        let mut edges = vec![0u64];
        let mut cursor = 0u64;
        for interval in &self.retained_intervals {
            edges.push(cursor);
            let first = if interval.is_recording() {
                self.split_points_us
                    .partition_point(|&p| p <= interval.start_us)
            } else {
                self.split_points_us.len()
            };
            for &point in &self.split_points_us[first..] {
                if point >= interval.end_us {
                    break;
                }
                edges.push(cursor + (point - interval.start_us));
            }
            cursor += interval.end_us - interval.start_us;
        }
        edges.push(cursor);
        edges.dedup();
        edges
    }
}

/// Which side of the playhead a ripple trim removes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TrimSide {
    /// Premiere's Q: from the previous edit point up to the playhead.
    Previous,
    /// Premiere's E: from the playhead up to the next edit point.
    Next,
}

/// The edited range a ripple trim removes, given sorted `edges`. `None` when
/// there is no edit point on that side of the playhead.
pub fn ripple_trim_range(edges: &[u64], playhead_us: u64, side: TrimSide) -> Option<(u64, u64)> {
    match side {
        TrimSide::Previous => edges
            .iter()
            .rev()
            .find(|&&edge| edge < playhead_us)
            .map(|&edge| (edge, playhead_us)),
        TrimSide::Next => edges
            .iter()
            .find(|&&edge| edge > playhead_us)
            .map(|&edge| (playhead_us, edge)),
    }
}

/// The timeline mapper for a stored list of entries, in playback order.
pub fn mapper_for(retained: &[RetainedInterval]) -> Result<TimelineMapper, String> {
    TimelineMapper::try_new(
        retained
            .iter()
            .enumerate()
            .map(|(i, interval)| {
                SourceInterval::new(format!("ret-{i}"), interval.start_us, interval.end_us)
                    .with_media(interval.media.clone())
            })
            .collect(),
    )
}

pub fn validate_retained(retained: &[RetainedInterval]) -> Result<(), String> {
    if retained.len() > MAX_RETAINED_INTERVALS {
        return Err("Too many retained intervals".into());
    }
    // The list order is the timeline order; source ranges must not overlap.
    for interval in retained {
        if interval.end_us > 9_007_199_254_740_991 {
            return Err("Retained timestamp exceeds supported precision".into());
        }
        if interval.end_us <= interval.start_us {
            return Err("Retained interval must be a half-open range".into());
        }
    }
    let mut by_source: Vec<(u64, u64)> = retained
        .iter()
        .filter(|i| i.is_recording())
        .map(|i| (i.start_us, i.end_us))
        .collect();
    by_source.sort_unstable();
    if by_source.windows(2).any(|pair| pair[1].0 < pair[0].1) {
        return Err("Retained intervals must not overlap".into());
    }
    if retained.iter().any(|i| {
        i.media
            .as_deref()
            .is_some_and(|id| id.is_empty() || id.len() > 128)
    }) {
        return Err("Invalid media asset id on the timeline".into());
    }
    Ok(())
}

pub fn validate_split_points(points: &[u64]) -> Result<(), String> {
    if points.len() > MAX_SPLIT_POINTS {
        return Err("Too many split points".into());
    }
    if points.iter().any(|&p| p > 9_007_199_254_740_991) {
        return Err("Split point exceeds supported precision".into());
    }
    if points.windows(2).any(|w| w[0] >= w[1]) {
        return Err("Split points must be sorted and unique".into());
    }
    Ok(())
}

/// Source ranges that the edit removed: time in `[0, source_duration_us)` that
/// no retained interval covers, minus the recorder's pause intervals, which
/// hold no media and so can never be restored.
pub fn removed_intervals(
    retained: &[RetainedInterval],
    pauses: &[RetainedInterval],
    source_duration_us: u64,
) -> Vec<RetainedInterval> {
    let mut covered: Vec<(u64, u64)> = retained
        .iter()
        .filter(|i| i.is_recording())
        .chain(pauses.iter())
        .map(|i| (i.start_us, i.end_us.min(source_duration_us)))
        .filter(|(a, b)| b > a)
        .collect();
    covered.sort_unstable();
    let mut removed = Vec::new();
    let mut cursor = 0u64;
    for (start, end) in covered {
        if start > cursor {
            removed.push(RetainedInterval {
                start_us: cursor,
                end_us: start,
                media: None,
            });
        }
        cursor = cursor.max(end);
    }
    if cursor < source_duration_us {
        removed.push(RetainedInterval {
            start_us: cursor,
            end_us: source_duration_us,
            media: None,
        });
    }
    removed
}

/// Sorts and merges overlapping or touching ranges.
fn merge_ranges(mut ranges: Vec<RetainedInterval>) -> Vec<RetainedInterval> {
    ranges.sort_by_key(|r| (r.start_us, r.end_us));
    let mut merged: Vec<RetainedInterval> = Vec::with_capacity(ranges.len());
    for range in ranges {
        match merged.last_mut() {
            Some(last) if range.start_us <= last.end_us => {
                last.end_us = last.end_us.max(range.end_us);
            }
            _ => merged.push(range),
        }
    }
    merged
}

/// Joins neighbours in the list that are also contiguous in source time, so an edit that
/// does not reorder anything leaves the same intervals it always did.
pub fn canonical_retained(retained: Vec<RetainedInterval>) -> Vec<RetainedInterval> {
    let mut out: Vec<RetainedInterval> = Vec::with_capacity(retained.len());
    for interval in retained {
        if interval.end_us <= interval.start_us {
            continue;
        }
        match out.last_mut() {
            // Media entries are never joined: their boundaries are the user's splits.
            Some(last)
                if last.end_us == interval.start_us
                    && last.is_recording()
                    && interval.is_recording() =>
            {
                last.end_us = interval.end_us
            }
            _ => out.push(interval),
        }
    }
    out
}

/// Makes `edited_us` an interval boundary in `retained` and returns the index of the
/// interval that starts there (`retained.len()` at the end of the timeline).
fn split_at_edited(retained: &mut Vec<RetainedInterval>, edited_us: u64) -> Result<usize, String> {
    let mut cursor = 0u64;
    for index in 0..retained.len() {
        let interval = retained[index].clone();
        let length = interval.end_us - interval.start_us;
        if edited_us == cursor {
            return Ok(index);
        }
        if edited_us < cursor + length {
            let source = interval.start_us + (edited_us - cursor);
            retained[index].end_us = source;
            retained.insert(
                index + 1,
                RetainedInterval {
                    start_us: source,
                    end_us: interval.end_us,
                    media: interval.media.clone(),
                },
            );
            return Ok(index + 1);
        }
        cursor += length;
    }
    if edited_us == cursor {
        Ok(retained.len())
    } else {
        Err("Position is outside the timeline".into())
    }
}

/// Moves the edited range `[start, end)` so it starts at edited position `target` of the
/// timeline as it is before the move. `target` must not be strictly inside the range.
pub fn move_edited_range(
    retained: &[RetainedInterval],
    start_us: u64,
    end_us: u64,
    target_us: u64,
) -> Result<Vec<RetainedInterval>, String> {
    if start_us >= end_us {
        return Err("Move range must be a half-open interval".into());
    }
    if target_us > start_us && target_us < end_us {
        return Err("A clip cannot move inside itself".into());
    }
    let mut list = retained.to_vec();
    let duration: u64 = list.iter().map(|i| i.end_us - i.start_us).sum();
    if end_us > duration || target_us > duration {
        return Err("Move is outside the timeline".into());
    }
    // Split from the latest position back, so earlier positions stay valid.
    let mut points = [start_us, end_us, target_us];
    points.sort_unstable();
    for &point in points.iter().rev() {
        split_at_edited(&mut list, point)?;
    }
    let first = split_at_edited(&mut list, start_us)?;
    let last = split_at_edited(&mut list, end_us)?;
    let mut at = split_at_edited(&mut list, target_us)?;
    let block: Vec<RetainedInterval> = list.drain(first..last).collect();
    if at >= last {
        at -= block.len();
    }
    list.splice(at..at, block);
    Ok(canonical_retained(list))
}

/// Which clip grows when restored media touches two clips that are no longer neighbours
/// on the timeline (after a reorder).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RestoreGrow {
    /// The clip that ends where the restored media starts, else the one that starts where
    /// it ends.
    #[default]
    End,
    /// The clip that starts where the restored media ends, else the one that ends where it
    /// starts.
    Start,
}

/// Puts `ranges` (removed source time) back in `retained` without disturbing the order:
/// a range touching a clip extends it (`grow` picks which when it touches two);
/// anything else goes before the first clip that starts later in the recording.
pub fn restore_in_order(
    retained: &[RetainedInterval],
    ranges: &[(u64, u64)],
    grow: RestoreGrow,
) -> Vec<RetainedInterval> {
    let mut list = retained.to_vec();
    let mut pieces: Vec<RetainedInterval> = ranges
        .iter()
        .map(|&(start_us, end_us)| RetainedInterval {
            start_us,
            end_us,
            media: None,
        })
        .collect();
    pieces = merge_ranges(pieces);
    for piece in pieces {
        // Only restore time no clip already holds.
        let mut free = vec![(piece.start_us, piece.end_us)];
        for interval in list.iter().filter(|i| i.is_recording()) {
            free = free
                .into_iter()
                .flat_map(|(a, b)| {
                    let mut parts = Vec::new();
                    if interval.end_us <= a || interval.start_us >= b {
                        parts.push((a, b));
                    } else {
                        if interval.start_us > a {
                            parts.push((a, interval.start_us));
                        }
                        if interval.end_us < b {
                            parts.push((interval.end_us, b));
                        }
                    }
                    parts
                })
                .collect();
        }
        for (start_us, end_us) in free {
            let before = list
                .iter()
                .position(|i| i.is_recording() && i.end_us == start_us);
            let after = list
                .iter()
                .position(|i| i.is_recording() && i.start_us == end_us);
            // List neighbours that touch on both sides simply join.
            let neighbours = matches!((before, after), (Some(b), Some(a)) if a == b + 1);
            let pick = match grow {
                RestoreGrow::End => before.map(|b| (b, true)).or(after.map(|a| (a, false))),
                RestoreGrow::Start if neighbours => before.map(|b| (b, true)),
                RestoreGrow::Start => after.map(|a| (a, false)).or(before.map(|b| (b, true))),
            };
            if let Some((index, extend_end)) = pick {
                if extend_end {
                    list[index].end_us = end_us;
                } else {
                    list[index].start_us = start_us;
                }
            } else {
                let at = list
                    .iter()
                    .position(|i| i.is_recording() && i.start_us > start_us)
                    .unwrap_or(list.len());
                list.insert(
                    at,
                    RetainedInterval {
                        start_us,
                        end_us,
                        media: None,
                    },
                );
            }
        }
    }
    canonical_retained(list)
}

pub fn load_edit_document(root: &Path) -> Result<Option<EditDocument>, String> {
    let path = safe_path(root, "project.json")?;
    if !path.is_file() {
        return Ok(None);
    }
    let file = open_regular(&path)?;
    let mut bytes = Vec::new();
    file.take(MAX_EDIT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_EDIT_BYTES {
        return Err("Edit document exceeds size limit".into());
    }
    let document: EditDocument =
        serde_json::from_slice(&bytes).map_err(|e| format!("Invalid edit document: {e}"))?;
    if document.schema_version != EDIT_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported edit schema version: {}",
            document.schema_version
        ));
    }
    validate_retained(&document.retained_intervals)?;
    validate_layout(&document.layout)?;
    validate_zooms(&document.zooms)?;
    validate_dismissed(&document.dismissed_zoom_ids)?;
    validate_split_points(&document.split_points_us)?;
    document.webcam_focus.validate()?;
    document.mapper()?;
    Ok(Some(document))
}

pub fn save_edit_document(root: &Path, document: &EditDocument) -> Result<(), String> {
    validate_retained(&document.retained_intervals)?;
    validate_layout(&document.layout)?;
    validate_zooms(&document.zooms)?;
    validate_dismissed(&document.dismissed_zoom_ids)?;
    document.webcam_focus.validate()?;
    if document.schema_version != EDIT_SCHEMA_VERSION {
        return Err("Unsupported edit schema version".into());
    }
    let path = safe_path(root, "project.json")?;
    if let Ok(meta) = fs::symlink_metadata(&path) {
        if meta.file_type().is_symlink() {
            return Err("Edit document cannot be a symlink".into());
        }
    }
    let serialized = serde_json::to_vec_pretty(document).map_err(|e| e.to_string())?;
    if serialized.len() as u64 > MAX_EDIT_BYTES {
        return Err("Edit document exceeds size limit".into());
    }
    let mut temp = tempfile::NamedTempFile::new_in(root).map_err(|e| e.to_string())?;
    temp.write_all(&serialized).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(&path).map_err(|e| e.to_string())?;
    // The document is committed after the atomic rename. Directory sync is best-effort:
    // reporting a failed edit after publication would leave memory and disk divergent.
    if let Ok(directory) = fs::File::open(root) {
        let _ = directory.sync_all();
    }
    Ok(())
}

fn validate_dismissed(ids: &[String]) -> Result<(), String> {
    if ids.len() > MAX_DISMISSED_ZOOMS {
        return Err("Too many dismissed zoom ids".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    for id in ids {
        if id.is_empty() || id.len() > 128 || !seen.insert(id) {
            return Err("Invalid dismissed zoom id".into());
        }
    }
    Ok(())
}

fn persist_revision(root: &Path, expected: u64, next: &EditDocument) -> Result<(), String> {
    let lock_path = safe_path(root, ".edit.lock")?;
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        // No sharing: a second editor saving the same project gets a sharing
        // violation, which stands in for the Unix flock below.
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0);
    }
    let lock = options.open(lock_path).map_err(|e| e.to_string())?;
    if !lock.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Invalid edit lock".into());
    }
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err("Another editor is saving this project".into());
        }
    }
    let result = (|| {
        if load_edit_document(root)?.map(|d| d.revision).unwrap_or(0) != expected {
            return Err("Stale edit revision on disk; reopen the project".into());
        }
        save_edit_document(root, next)
    })();
    // Explicitly unlock: another thread may have forked a child that briefly
    // inherits this open-file description before exec closes CLOEXEC handles.
    // Merely closing our descriptor can then leave the lock held by the child.
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        unsafe {
            libc::flock(lock.as_raw_fd(), libc::LOCK_UN);
        }
    }
    result
}

#[derive(Clone, Debug)]
pub struct EditHistory {
    pub current: EditDocument,
    undo: Vec<EditDocument>,
    redo: Vec<EditDocument>,
}

impl EditHistory {
    pub fn new(current: EditDocument) -> Self {
        Self {
            current,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn undo_available(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn redo_available(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn commit(
        &mut self,
        expected_revision: u64,
        next_retained: Vec<RetainedInterval>,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        let mut next = self.current.clone();
        next.retained_intervals = next_retained;
        self.commit_next(expected_revision, persist_root, next)
    }

    pub fn commit_next(
        &mut self,
        expected_revision: u64,
        persist_root: &Path,
        mut next: EditDocument,
    ) -> Result<&EditDocument, String> {
        if expected_revision != self.current.revision {
            return Err("Stale edit revision".into());
        }
        validate_retained(&next.retained_intervals)?;
        mapper_for(&next.retained_intervals)?;
        validate_layout(&next.layout)?;
        validate_zooms(&next.zooms)?;
        validate_dismissed(&next.dismissed_zoom_ids)?;
        validate_split_points(&next.split_points_us)?;
        next.webcam_focus.validate()?;
        next.audio.validate()?;
        crate::media_bin::validate_assets(&next.media_assets)?;
        if let Some(missing) = next.retained_intervals.iter().find_map(|entry| {
            entry
                .media
                .as_ref()
                .filter(|id| !next.media_assets.iter().any(|asset| &asset.id == *id))
        }) {
            return Err(format!(
                "The timeline uses media {missing} that is not imported"
            ));
        }
        next.schema_version = EDIT_SCHEMA_VERSION;
        next.revision = self
            .current
            .revision
            .checked_add(1)
            .ok_or("Revision overflow")?;
        next.attach_zoom_ranges()?;
        persist_revision(persist_root, expected_revision, &next)?;
        self.undo.push(self.current.clone());
        if self.undo.len() > MAX_UNDO {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.current = next;
        Ok(&self.current)
    }

    pub fn update_layout(
        &mut self,
        expected_revision: u64,
        layout: EditLayout,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if expected_revision != self.current.revision {
            return Err("Stale edit revision".into());
        }
        validate_layout(&layout)?;
        if layout == self.current.layout {
            return Ok(&self.current);
        }
        let mut next = self.current.clone();
        next.layout = layout;
        self.commit_next(expected_revision, persist_root, next)
    }

    pub fn update_webcam_focus(
        &mut self,
        expected_revision: u64,
        focus: WebcamFocus,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if expected_revision != self.current.revision {
            return Err("Stale edit revision".into());
        }
        let focus = focus.normalized();
        focus.validate()?;
        if focus == self.current.webcam_focus {
            return Ok(&self.current);
        }
        let mut next = self.current.clone();
        next.webcam_focus = focus;
        self.commit_next(expected_revision, persist_root, next)
    }

    /// Adds a webcam focus segment over an edited range, mapped to source time.
    pub fn add_webcam_focus(
        &mut self,
        expected_revision: u64,
        edited_start_us: u64,
        edited_end_us: u64,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if edited_end_us <= edited_start_us {
            return Err("Webcam focus must be a half-open edited range".into());
        }
        let mapper = self.current.mapper()?;
        let source_start = mapper
            .edited_to_source_us(edited_start_us)
            .ok_or("Webcam focus start is not on retained media")?;
        let source_end = mapper
            .edited_to_source_us(edited_end_us.saturating_sub(1))
            .ok_or("Webcam focus end is not on retained media")?
            .saturating_add(1);
        if source_end <= source_start {
            return Err("Webcam focus range does not map onto source time".into());
        }
        let mut focus = self.current.webcam_focus.clone();
        focus.enabled = true;
        let mut n = focus.segments.len();
        let id = loop {
            let id = format!("manual-{source_start}-{n}");
            if !focus.segments.iter().any(|s| s.id == id) {
                break id;
            }
            n += 1;
        };
        focus
            .segments
            .push(crate::webcam_focus::WebcamFocusSegment {
                id,
                source_start_us: source_start,
                source_end_us: source_end,
                source: crate::webcam_focus::FocusSegmentSource::Manual,
                enabled: true,
                edited_ranges: Vec::new(),
            });
        self.update_webcam_focus(expected_revision, focus, persist_root)
    }

    pub fn update_audio(
        &mut self,
        expected_revision: u64,
        audio: AudioSettings,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if expected_revision != self.current.revision {
            return Err("Stale edit revision".into());
        }
        audio.validate()?;
        if audio == self.current.audio {
            return Ok(&self.current);
        }
        let mut next = self.current.clone();
        next.audio = audio;
        self.commit_next(expected_revision, persist_root, next)
    }

    pub fn update_captions(
        &mut self,
        expected_revision: u64,
        captions: CaptionSettings,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if expected_revision != self.current.revision {
            return Err("Stale edit revision".into());
        }
        captions.validate()?;
        if captions == self.current.captions {
            return Ok(&self.current);
        }
        let mut next = self.current.clone();
        next.captions = captions;
        self.commit_next(expected_revision, persist_root, next)
    }

    pub fn accept_zooms(
        &mut self,
        expected_revision: u64,
        suggestions: &[ZoomSuggestion],
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if suggestions.is_empty() {
            return Err("No zoom suggestions selected".into());
        }
        let mut next = self.current.clone();
        let existing: std::collections::BTreeSet<_> =
            next.zooms.iter().map(|z| z.id.clone()).collect();
        let dismissed: std::collections::BTreeSet<_> =
            next.dismissed_zoom_ids.iter().cloned().collect();
        for suggestion in suggestions {
            if existing.contains(&suggestion.id) || dismissed.contains(&suggestion.id) {
                continue;
            }
            next.zooms.push(ZoomKeyframe::from_suggestion(
                suggestion.clone(),
                ZoomSource::Generated,
            ));
        }
        if next.zooms.len() == self.current.zooms.len() {
            return Err("Those zoom suggestions are already applied or dismissed".into());
        }
        if next.zooms.len() > MAX_ZOOMS {
            return Err("Too many zoom keyframes".into());
        }
        next.zooms.sort_by(|a, b| {
            a.source_start_us
                .cmp(&b.source_start_us)
                .then(a.id.cmp(&b.id))
        });
        self.commit_next(expected_revision, persist_root, next)
    }

    pub fn dismiss_zooms(
        &mut self,
        expected_revision: u64,
        ids: &[String],
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if ids.is_empty() {
            return Err("No zoom ids selected".into());
        }
        let mut next = self.current.clone();
        let remove: std::collections::BTreeSet<_> = ids.iter().cloned().collect();
        next.zooms.retain(|z| !remove.contains(&z.id));
        for id in ids {
            if !next.dismissed_zoom_ids.iter().any(|d| d == id) {
                next.dismissed_zoom_ids.push(id.clone());
            }
        }
        if next.zooms == self.current.zooms
            && next.dismissed_zoom_ids == self.current.dismissed_zoom_ids
        {
            return Err("Those zoom ids are already dismissed".into());
        }
        self.commit_next(expected_revision, persist_root, next)
    }

    pub fn update_zoom(
        &mut self,
        expected_revision: u64,
        patch: ZoomKeyframe,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        let mut next = self.current.clone();
        let Some(existing) = next.zooms.iter_mut().find(|z| z.id == patch.id) else {
            return Err("Unknown zoom keyframe".into());
        };
        existing.source_start_us = patch.source_start_us;
        existing.source_end_us = patch.source_end_us;
        existing.center_x = patch.center_x;
        existing.center_y = patch.center_y;
        existing.scale = patch.scale;
        existing.transition_us = patch.transition_us;
        // Moving/resizing a generated zoom keeps its id so regeneration cannot
        // replace it, and marks it manual so a later accept cannot reset it.
        existing.source = ZoomSource::Manual;
        self.commit_next(expected_revision, persist_root, next)
    }

    pub fn add_manual_zoom(
        &mut self,
        expected_revision: u64,
        edited_start_us: u64,
        edited_end_us: u64,
        center_x: f64,
        center_y: f64,
        scale: f64,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if edited_end_us <= edited_start_us {
            return Err("Zoom must be a half-open edited range".into());
        }
        let mapper = self.current.mapper()?;
        let source_start = mapper
            .edited_to_source_us(edited_start_us)
            .ok_or("Zoom start is not on retained media")?;
        let source_end_sample = mapper
            .edited_to_source_us(edited_end_us.saturating_sub(1))
            .ok_or("Zoom end is not on retained media")?;
        let source_end = source_end_sample.saturating_add(1);
        if source_end <= source_start {
            return Err("Zoom range does not map onto source time".into());
        }
        let duration = source_end - source_start;
        if duration < 3 {
            return Err("Zoom range is too short".into());
        }
        let transition_us = (duration / 5)
            .clamp(1, 400_000)
            .min(duration.saturating_sub(1));
        let mut next = self.current.clone();
        let id = format!("m-{}-{}", source_start, next.zooms.len());
        next.zooms.push(ZoomKeyframe {
            id,
            source_start_us: source_start,
            source_end_us: source_end,
            center_x,
            center_y,
            scale,
            transition_us,
            origin: crate::zoom::ZoomOrigin::Click,
            contributing_event_seqs: Vec::new(),
            source: ZoomSource::Manual,
            edited_ranges: Vec::new(),
        });
        next.zooms.sort_by(|a, b| {
            a.source_start_us
                .cmp(&b.source_start_us)
                .then(a.id.cmp(&b.id))
        });
        self.commit_next(expected_revision, persist_root, next)
    }

    pub fn delete_zoom(
        &mut self,
        expected_revision: u64,
        id: &str,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        let mut next = self.current.clone();
        let Some(index) = next.zooms.iter().position(|z| z.id == id) else {
            return Err("Unknown zoom keyframe".into());
        };
        let removed = next.zooms.remove(index);
        if removed.source == ZoomSource::Generated
            && !next.dismissed_zoom_ids.iter().any(|d| d == id)
        {
            next.dismissed_zoom_ids.push(id.to_string());
        }
        self.commit_next(expected_revision, persist_root, next)
    }

    pub fn ripple_cuts(
        &mut self,
        expected_revision: u64,
        cuts: &[(u64, u64)],
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if cuts.is_empty() {
            return Err("No cuts selected".into());
        }
        if cuts.len() > MAX_CUTS_PER_REVISION {
            return Err("Too many cuts in one revision".into());
        }
        if expected_revision != self.current.revision {
            return Err("Stale edit revision".into());
        }
        let mut mapper = self.current.mapper()?;
        let mut ordered = cuts.to_vec();
        ordered.sort_unstable();
        if ordered.windows(2).any(|w| w[0].1 > w[1].0) {
            return Err("Cut ranges overlap".into());
        }
        ordered.reverse();
        for (start, end) in ordered {
            mapper.ripple_cut_edited(start, end)?;
        }
        let retained = canonical_retained(
            mapper
                .intervals()
                .iter()
                .map(|interval| RetainedInterval {
                    start_us: interval.start_us,
                    end_us: interval.end_us,
                    media: interval.media.clone(),
                })
                .collect(),
        );
        self.commit(expected_revision, retained, persist_root)
    }

    /// Premiere-style Q/E: ripple-deletes from the playhead back to the
    /// previous edit point, or forward to the next one, and closes the gap.
    /// Returns the removed edited range.
    pub fn ripple_trim(
        &mut self,
        expected_revision: u64,
        playhead_us: u64,
        side: TrimSide,
        persist_root: &Path,
    ) -> Result<(u64, u64), String> {
        if expected_revision != self.current.revision {
            return Err("Stale edit revision".into());
        }
        let duration = self.current.edited_duration_us()?;
        let playhead_us = playhead_us.min(duration);
        let edges = self.current.clip_edges_edited();
        let (start, end) = ripple_trim_range(&edges, playhead_us, side).ok_or(match side {
            TrimSide::Previous => "No edit point before the playhead",
            TrimSide::Next => "No edit point after the playhead",
        })?;
        if start == 0 && end >= duration {
            return Err("That would remove the whole timeline".into());
        }
        self.ripple_cuts(expected_revision, &[(start, end)], persist_root)?;
        Ok((start, end))
    }

    /// Splits the clip under the edited position. No media is removed.
    pub fn split(
        &mut self,
        expected_revision: u64,
        edited_us: u64,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if expected_revision != self.current.revision {
            return Err("Stale edit revision".into());
        }
        let mapper = self.current.mapper()?;
        if mapper.media_at(edited_us).is_some() {
            // Imported media splits by becoming two timeline entries.
            let mut next = self.current.clone();
            let before = next.retained_intervals.len();
            split_at_edited(&mut next.retained_intervals, edited_us)?;
            if next.retained_intervals.len() == before {
                return Err("There is already a clip edge here".into());
            }
            return self.commit_next(expected_revision, persist_root, next);
        }
        let source_us = mapper
            .edited_to_source_us(edited_us)
            .ok_or("Split point is outside the timeline")?;
        if self
            .current
            .retained_intervals
            .iter()
            .any(|interval| interval.is_recording() && interval.start_us == source_us)
        {
            return Err("There is already a clip edge here".into());
        }
        let mut next = self.current.clone();
        match next.split_points_us.binary_search(&source_us) {
            Ok(_) => return Err("There is already a clip edge here".into()),
            Err(index) => next.split_points_us.insert(index, source_us),
        }
        self.commit_next(expected_revision, persist_root, next)
    }

    /// Puts removed source ranges back on the timeline. The caller clips the
    /// ranges to media that was actually removed.
    pub fn restore(
        &mut self,
        expected_revision: u64,
        ranges: &[(u64, u64)],
        grow: RestoreGrow,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if ranges.is_empty() {
            return Err("Nothing to restore".into());
        }
        if ranges.len() > MAX_RETAINED_INTERVALS {
            return Err("Too many ranges to restore".into());
        }
        if ranges.iter().any(|(start, end)| end <= start) {
            return Err("Restore range must be a half-open interval".into());
        }
        if expected_revision != self.current.revision {
            return Err("Stale edit revision".into());
        }
        let retained = restore_in_order(&self.current.retained_intervals, ranges, grow);
        self.commit(expected_revision, retained, persist_root)
    }

    /// Adds imported media to the project's media bin.
    pub fn add_media(
        &mut self,
        expected_revision: u64,
        assets: Vec<crate::media_bin::MediaAsset>,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if expected_revision != self.current.revision {
            return Err("Stale edit revision".into());
        }
        if assets.is_empty() {
            return Err("Nothing to import".into());
        }
        let mut next = self.current.clone();
        next.media_assets.extend(assets);
        self.commit_next(expected_revision, persist_root, next)
    }

    /// Removes media from the bin and every clip of it from the timeline. The file stays
    /// in the project so undo can bring it back.
    pub fn remove_media(
        &mut self,
        expected_revision: u64,
        asset_id: &str,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if expected_revision != self.current.revision {
            return Err("Stale edit revision".into());
        }
        let mut next = self.current.clone();
        let before = next.media_assets.len();
        next.media_assets.retain(|asset| asset.id != asset_id);
        if next.media_assets.len() == before {
            return Err("No such imported media".into());
        }
        next.retained_intervals
            .retain(|entry| entry.media.as_deref() != Some(asset_id));
        if next.retained_intervals.is_empty() {
            return Err("Removing it would leave the timeline empty".into());
        }
        next.retained_intervals = canonical_retained(next.retained_intervals);
        self.commit_next(expected_revision, persist_root, next)
    }

    /// Places a clip of imported media at edited position `target`, `source_start..end`
    /// within the file (the whole default length when `None`).
    pub fn insert_media(
        &mut self,
        expected_revision: u64,
        asset_id: &str,
        target_us: u64,
        range: Option<(u64, u64)>,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if expected_revision != self.current.revision {
            return Err("Stale edit revision".into());
        }
        let asset = self
            .current
            .media_assets
            .iter()
            .find(|asset| asset.id == asset_id)
            .ok_or("No such imported media")?;
        let (start_us, end_us) = range.unwrap_or((0, asset.default_clip_us()));
        if start_us >= end_us || end_us > asset.duration_us {
            return Err("The clip must lie within the media's length".into());
        }
        let mut next = self.current.clone();
        let at = split_at_edited(&mut next.retained_intervals, target_us)?;
        next.retained_intervals.insert(
            at,
            RetainedInterval {
                start_us,
                end_us,
                media: Some(asset_id.to_string()),
            },
        );
        self.commit_next(expected_revision, persist_root, next)
    }

    /// Moves the clip (or any edited range) `[start, end)` to edited position `target`.
    pub fn move_range(
        &mut self,
        expected_revision: u64,
        start_us: u64,
        end_us: u64,
        target_us: u64,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if expected_revision != self.current.revision {
            return Err("Stale edit revision".into());
        }
        let retained = move_edited_range(
            &self.current.retained_intervals,
            start_us,
            end_us,
            target_us,
        )?;
        if retained == self.current.retained_intervals {
            return Err("The clip is already there".into());
        }
        self.commit(expected_revision, retained, persist_root)
    }

    pub fn undo(
        &mut self,
        expected_revision: u64,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if expected_revision != self.current.revision {
            return Err("Stale edit revision".into());
        }
        let mut previous = self.undo.last().cloned().ok_or("Nothing to undo")?;
        previous.revision = self
            .current
            .revision
            .checked_add(1)
            .ok_or("Revision overflow")?;
        persist_revision(persist_root, expected_revision, &previous)?;
        self.undo.pop();
        self.redo.push(self.current.clone());
        self.current = previous;
        Ok(&self.current)
    }

    pub fn redo(
        &mut self,
        expected_revision: u64,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if expected_revision != self.current.revision {
            return Err("Stale edit revision".into());
        }
        let mut next = self.redo.last().cloned().ok_or("Nothing to redo")?;
        next.revision = self
            .current
            .revision
            .checked_add(1)
            .ok_or("Revision overflow")?;
        persist_revision(persist_root, expected_revision, &next)?;
        self.redo.pop();
        self.undo.push(self.current.clone());
        self.current = next;
        Ok(&self.current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn failed_save_preserves_history_and_revision_ids_never_repeat() {
        let dir = tempdir().unwrap();
        let initial = EditDocument::from_retained(vec![RetainedInterval {
            start_us: 0,
            end_us: 1_000_000,
            media: None,
        }])
        .unwrap();
        let mut history = EditHistory::new(initial.clone());
        fs::create_dir(dir.path().join("project.json")).unwrap();
        assert!(history
            .ripple_cuts(0, &[(100_000, 200_000)], dir.path())
            .is_err());
        assert_eq!(history.current, initial);
        assert!(!history.undo_available());
        fs::remove_dir(dir.path().join("project.json")).unwrap();
        history
            .ripple_cuts(0, &[(100_000, 200_000)], dir.path())
            .unwrap();
        history.undo(1, dir.path()).unwrap();
        history
            .ripple_cuts(2, &[(300_000, 400_000)], dir.path())
            .unwrap();
        assert_eq!(history.current.revision, 3);
        assert!(history.ripple_cuts(1, &[(0, 1)], dir.path()).is_err());
        let snapshot = history.current.clone();
        fs::remove_file(dir.path().join("project.json")).unwrap();
        fs::create_dir(dir.path().join("project.json")).unwrap();
        assert!(history.undo(3, dir.path()).is_err());
        assert_eq!(history.current, snapshot);
        assert!(history.undo_available());
        assert!(!history.redo_available());
    }

    #[test]
    fn webcam_focus_round_trips_through_revisions_and_undo() {
        let dir = tempdir().unwrap();
        let initial = EditDocument::from_retained(vec![
            RetainedInterval {
                start_us: 0,
                end_us: 4_000_000,
                media: None,
            },
            RetainedInterval {
                start_us: 6_000_000,
                end_us: 10_000_000,
                media: None,
            },
        ])
        .unwrap();
        let mut history = EditHistory::new(initial);
        // Edited 3s..5s spans the cut, so it maps to source 3s..7s.
        history
            .add_webcam_focus(0, 3_000_000, 5_000_000, dir.path())
            .unwrap();
        let focus = &history.current.webcam_focus;
        assert!(focus.enabled);
        assert_eq!(focus.segments.len(), 1);
        assert_eq!(
            (
                focus.segments[0].source_start_us,
                focus.segments[0].source_end_us
            ),
            (3_000_000, 7_000_000)
        );
        let on_disk = load_edit_document(dir.path()).unwrap().unwrap();
        assert_eq!(on_disk.webcam_focus, history.current.webcam_focus);

        // UI-only edited ranges are dropped, and an unchanged update is not a revision.
        let mut echoed = history.current.webcam_focus.clone();
        echoed.attach_edited_ranges(&history.current.mapper().unwrap());
        history.update_webcam_focus(1, echoed, dir.path()).unwrap();
        assert_eq!(history.current.revision, 1);

        let mut off = history.current.webcam_focus.clone();
        off.enabled = false;
        history.update_webcam_focus(1, off, dir.path()).unwrap();
        assert!(!history.current.webcam_focus.enabled);
        history.undo(2, dir.path()).unwrap();
        assert!(history.current.webcam_focus.enabled);

        let mut bad = history.current.webcam_focus.clone();
        bad.settings.focus_size_pct = 10.0;
        assert!(history.update_webcam_focus(3, bad, dir.path()).is_err());
    }

    #[test]
    fn second_editor_cannot_overwrite_newer_disk_revision() {
        let dir = tempdir().unwrap();
        let initial = EditDocument::from_retained(vec![RetainedInterval {
            start_us: 0,
            end_us: 1_000_000,
            media: None,
        }])
        .unwrap();
        let mut first = EditHistory::new(initial.clone());
        let mut second = EditHistory::new(initial.clone());
        first.ripple_cuts(0, &[(0, 100_000)], dir.path()).unwrap();
        assert!(second
            .ripple_cuts(0, &[(0, 200_000)], dir.path())
            .unwrap_err()
            .contains("Stale"));
        assert_eq!(second.current, initial);
        assert_eq!(
            load_edit_document(dir.path()).unwrap().unwrap(),
            first.current
        );
    }

    #[test]
    fn persist_roundtrip_and_stale_revision() {
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(
            EditDocument::from_retained(vec![RetainedInterval {
                start_us: 0,
                end_us: 10_000_000,
                media: None,
            }])
            .unwrap(),
        );
        history
            .ripple_cuts(0, &[(2_000_000, 5_000_000)], dir.path())
            .unwrap();
        assert_eq!(history.current.revision, 1);
        assert_eq!(history.current.retained_intervals.len(), 2);
        assert!(history
            .ripple_cuts(0, &[(0, 1_000_000)], dir.path())
            .unwrap_err()
            .contains("Stale"));
        history.undo(1, dir.path()).unwrap();
        assert_eq!(history.current.revision, 2);
        assert_eq!(history.current.retained_intervals.len(), 1);
        history.redo(2, dir.path()).unwrap();
        let loaded = load_edit_document(dir.path()).unwrap().unwrap();
        assert_eq!(loaded.revision, 3);
        assert_eq!(loaded.retained_intervals[0].end_us, 2_000_000);
    }

    #[test]
    fn zoom_edits_undo_and_do_not_revive_dismissed_or_overwrite_manual() {
        use crate::zoom::{ZoomOrigin, ZoomSuggestion};
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(
            EditDocument::from_retained(vec![RetainedInterval {
                start_us: 0,
                end_us: 10_000_000,
                media: None,
            }])
            .unwrap(),
        );
        let suggestion = ZoomSuggestion {
            id: "z-1-n1".into(),
            source_start_us: 1_000_000,
            source_end_us: 3_000_000,
            center_x: 0.4,
            center_y: 0.4,
            scale: 2.0,
            transition_us: 400_000,
            origin: ZoomOrigin::Click,
            contributing_event_seqs: vec![1],
            edited_ranges: Vec::new(),
        };
        history
            .accept_zooms(0, &[suggestion.clone()], dir.path())
            .unwrap();
        assert_eq!(history.current.zooms.len(), 1);
        assert_eq!(history.current.zooms[0].source, ZoomSource::Generated);
        let mut moved = history.current.zooms[0].clone();
        moved.source_start_us = 1_200_000;
        moved.source_end_us = 3_200_000;
        history.update_zoom(1, moved, dir.path()).unwrap();
        assert_eq!(history.current.zooms[0].source, ZoomSource::Manual);
        history
            .accept_zooms(2, &[suggestion.clone()], dir.path())
            .unwrap_err();
        history
            .dismiss_zooms(2, &["z-1-n1".into()], dir.path())
            .unwrap();
        assert!(history.current.zooms.is_empty());
        history
            .accept_zooms(3, &[suggestion], dir.path())
            .unwrap_err();
        history.undo(3, dir.path()).unwrap();
        assert_eq!(history.current.zooms.len(), 1);
        assert_eq!(history.current.zooms[0].source, ZoomSource::Manual);
        let loaded = load_edit_document(dir.path()).unwrap().unwrap();
        assert_eq!(loaded.zooms[0].source_start_us, 1_200_000);
        assert_eq!(loaded.revision, 4);
    }

    #[test]
    fn layout_edits_undo_and_reopen() {
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(
            EditDocument::from_retained(vec![RetainedInterval {
                start_us: 0,
                end_us: 1_000_000,
                media: None,
            }])
            .unwrap(),
        );
        let mut layout = EditLayout::default();
        layout.aspect_ratio = "9:16".into();
        layout.padding_px = 24;
        layout.background_type = "solid".into();
        layout.color_start = "#ff0000".into();
        layout.webcam_mirror = false;
        layout.webcam_position = "top-left".into();
        history
            .update_layout(0, layout.clone(), dir.path())
            .unwrap();
        assert_eq!(history.current.revision, 1);
        assert_eq!(history.current.layout.aspect_ratio, "9:16");
        history.undo(1, dir.path()).unwrap();
        assert_eq!(history.current.layout.aspect_ratio, "16:9");
        history.redo(2, dir.path()).unwrap();
        let loaded = load_edit_document(dir.path()).unwrap().unwrap();
        assert_eq!(loaded.layout.padding_px, 24);
        assert_eq!(loaded.layout.color_start, "#ff0000");
        assert!(!loaded.layout.webcam_mirror);
        assert_eq!(loaded.revision, 3);
        let mut bad = layout;
        bad.padding_px = 999;
        assert!(history.update_layout(3, bad, dir.path()).is_err());
        assert_eq!(history.current.revision, 3);
    }

    #[test]
    fn audio_settings_persist_undo_and_validate() {
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(
            EditDocument::from_retained(vec![RetainedInterval {
                start_us: 0,
                end_us: 1_000_000,
                media: None,
            }])
            .unwrap(),
        );
        let audio = AudioSettings {
            normalize: true,
            duck_system_audio: true,
            duck_db: 18.0,
            ..Default::default()
        };
        history.update_audio(0, audio.clone(), dir.path()).unwrap();
        assert_eq!(history.current.revision, 1);
        assert_eq!(
            load_edit_document(dir.path()).unwrap().unwrap().audio,
            audio
        );
        history.undo(1, dir.path()).unwrap();
        assert!(history.current.audio.is_default());
        let bad = AudioSettings {
            target_lufs: 0.0,
            ..audio
        };
        assert!(history.update_audio(2, bad, dir.path()).is_err());
        assert_eq!(history.current.revision, 2);
        // Documents without audio settings still load, and default settings stay out of the file.
        let text = std::fs::read_to_string(dir.path().join("project.json")).unwrap();
        assert!(!text.contains("\"audio\""));
    }

    fn ri(start_us: u64, end_us: u64) -> RetainedInterval {
        RetainedInterval {
            start_us,
            end_us,
            media: None,
        }
    }

    #[test]
    fn removed_intervals_skip_pauses_and_cover_tail() {
        let retained = [ri(0, 1_000), ri(3_000, 5_000)];
        let pauses = [ri(1_000, 1_500), ri(5_000, 6_000)];
        assert_eq!(
            removed_intervals(&retained, &pauses, 8_000),
            vec![ri(1_500, 3_000), ri(6_000, 8_000)]
        );
        assert!(removed_intervals(&[ri(0, 8_000)], &[], 8_000).is_empty());
    }

    #[test]
    fn imported_media_inserts_between_clips_and_is_removed_with_its_asset() {
        use crate::media_bin::{MediaAsset, MediaKind};
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(
            EditDocument::from_retained(vec![ri(0, 4_000_000), ri(6_000_000, 10_000_000)]).unwrap(),
        );
        let asset = MediaAsset {
            id: "m1".into(),
            name: "intro.png".into(),
            kind: MediaKind::Image,
            relative_path: "assets/media/m1.png".into(),
            audio_path: None,
            duration_us: crate::media_bin::IMAGE_MAX_US,
            width: 640,
            height: 360,
        };
        assert!(history.insert_media(0, "m1", 0, None, dir.path()).is_err());
        history.add_media(0, vec![asset], dir.path()).unwrap();

        // Insert at the edge between the two recording clips: a 5 s still.
        history
            .insert_media(1, "m1", 4_000_000, None, dir.path())
            .unwrap();
        let media = |start_us, end_us| RetainedInterval {
            start_us,
            end_us,
            media: Some("m1".into()),
        };
        assert_eq!(
            history.current.retained_intervals,
            vec![
                ri(0, 4_000_000),
                media(0, 5_000_000),
                ri(6_000_000, 10_000_000)
            ]
        );
        let mapper = history.current.mapper().unwrap();
        assert_eq!(history.current.edited_duration_us().unwrap(), 13_000_000);
        assert_eq!(mapper.media_at(5_000_000), Some(("m1", 1_000_000)));
        assert_eq!(mapper.edited_to_source_us(5_000_000), None);
        assert_eq!(mapper.edited_to_source_us(9_500_000), Some(6_500_000));

        // Inserting in the middle of a clip splits it; a clip past the media's length is refused.
        assert!(history
            .insert_media(
                2,
                "m1",
                2_000_000,
                Some((0, crate::media_bin::IMAGE_MAX_US + 1)),
                dir.path()
            )
            .is_err());
        history
            .insert_media(2, "m1", 2_000_000, Some((0, 1_000_000)), dir.path())
            .unwrap();
        assert_eq!(
            history.current.retained_intervals,
            vec![
                ri(0, 2_000_000),
                media(0, 1_000_000),
                ri(2_000_000, 4_000_000),
                media(0, 5_000_000),
                ri(6_000_000, 10_000_000)
            ]
        );
        assert_eq!(
            load_edit_document(dir.path()).unwrap().unwrap(),
            history.current
        );

        // Removing the asset drops every clip of it and merges the recording back together.
        history.remove_media(3, "m1", dir.path()).unwrap();
        assert!(history.current.media_assets.is_empty());
        assert_eq!(
            history.current.retained_intervals,
            vec![ri(0, 4_000_000), ri(6_000_000, 10_000_000)]
        );
        history.undo(4, dir.path()).unwrap();
        assert_eq!(history.current.media_assets.len(), 1);
        assert_eq!(history.current.retained_intervals.len(), 5);
    }

    #[test]
    fn split_keeps_media_and_survives_cut_restore_and_undo() {
        let dir = tempdir().unwrap();
        let mut history =
            EditHistory::new(EditDocument::from_retained(vec![ri(0, 10_000_000)]).unwrap());
        history.split(0, 4_000_000, dir.path()).unwrap();
        assert_eq!(history.current.split_points_us, vec![4_000_000]);
        assert_eq!(history.current.retained_intervals, vec![ri(0, 10_000_000)]);
        assert!(history
            .split(1, 4_000_000, dir.path())
            .unwrap_err()
            .contains("clip edge"));
        assert!(history.split(1, 10_000_000, dir.path()).is_err());

        // Cut the second half of the first clip, then restore it.
        history
            .ripple_cuts(1, &[(2_000_000, 4_000_000)], dir.path())
            .unwrap();
        assert_eq!(
            history.current.retained_intervals,
            vec![ri(0, 2_000_000), ri(4_000_000, 10_000_000)]
        );
        // The split now sits on an interval start, so no second split there.
        assert!(history.split(2, 2_000_000, dir.path()).is_err());
        history
            .restore(2, &[(2_000_000, 4_000_000)], RestoreGrow::End, dir.path())
            .unwrap();
        assert_eq!(history.current.retained_intervals, vec![ri(0, 10_000_000)]);
        assert_eq!(history.current.split_points_us, vec![4_000_000]);

        history.undo(3, dir.path()).unwrap();
        history.undo(4, dir.path()).unwrap();
        history.undo(5, dir.path()).unwrap();
        assert!(history.current.split_points_us.is_empty());
        let loaded = load_edit_document(dir.path()).unwrap().unwrap();
        assert!(loaded.split_points_us.is_empty());
    }

    #[test]
    fn restore_merges_partial_ranges() {
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(
            EditDocument::from_retained(vec![ri(0, 1_000), ri(5_000, 6_000)]).unwrap(),
        );
        history
            .restore(0, &[(2_000, 3_000)], RestoreGrow::End, dir.path())
            .unwrap();
        assert_eq!(
            history.current.retained_intervals,
            vec![ri(0, 1_000), ri(2_000, 3_000), ri(5_000, 6_000)]
        );
        history
            .restore(
                1,
                &[(1_000, 2_000), (3_000, 5_000)],
                RestoreGrow::End,
                dir.path(),
            )
            .unwrap();
        assert_eq!(history.current.retained_intervals, vec![ri(0, 6_000)]);
        assert!(history
            .restore(2, &[], RestoreGrow::End, dir.path())
            .is_err());
        assert!(history
            .restore(2, &[(10, 10)], RestoreGrow::End, dir.path())
            .is_err());
    }

    #[test]
    fn clip_edges_include_cuts_and_splits_inside_media() {
        let mut doc = EditDocument::from_retained(vec![ri(0, 4_000), ri(6_000, 10_000)]).unwrap();
        // 5_000 sits in removed media and 6_000 on an interval start: neither adds an edge.
        doc.split_points_us = vec![2_000, 5_000, 6_000, 8_000];
        assert_eq!(doc.clip_edges_edited(), vec![0, 2_000, 4_000, 6_000, 8_000]);
        assert_eq!(
            EditDocument::from_retained(vec![])
                .unwrap()
                .clip_edges_edited(),
            vec![0]
        );
    }

    #[test]
    fn ripple_trim_range_picks_the_neighbouring_edit() {
        let edges = [0, 2_000, 5_000, 8_000];
        assert_eq!(
            ripple_trim_range(&edges, 3_000, TrimSide::Previous),
            Some((2_000, 3_000))
        );
        assert_eq!(
            ripple_trim_range(&edges, 3_000, TrimSide::Next),
            Some((3_000, 5_000))
        );
        // On an edit point, Q reaches the one before and E the one after.
        assert_eq!(
            ripple_trim_range(&edges, 5_000, TrimSide::Previous),
            Some((2_000, 5_000))
        );
        assert_eq!(
            ripple_trim_range(&edges, 5_000, TrimSide::Next),
            Some((5_000, 8_000))
        );
        assert_eq!(ripple_trim_range(&edges, 0, TrimSide::Previous), None);
        assert_eq!(ripple_trim_range(&edges, 8_000, TrimSide::Next), None);
    }

    #[test]
    fn ripple_trim_cuts_to_the_edit_point_and_undoes() {
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(
            EditDocument::from_retained(vec![ri(0, 4_000), ri(6_000, 10_000)]).unwrap(),
        );
        // Q at edited 3_000: removes [1_000, 3_000) of source back to the start.
        assert_eq!(
            history
                .ripple_trim(0, 3_000, TrimSide::Previous, dir.path())
                .unwrap(),
            (0, 3_000)
        );
        assert_eq!(
            history.current.retained_intervals,
            vec![ri(3_000, 4_000), ri(6_000, 10_000)]
        );
        // E at edited 2_000 (source 7_000): removes up to the end of that clip.
        assert_eq!(
            history
                .ripple_trim(1, 2_000, TrimSide::Next, dir.path())
                .unwrap(),
            (2_000, 5_000)
        );
        assert_eq!(
            history.current.retained_intervals,
            vec![ri(3_000, 4_000), ri(6_000, 7_000)]
        );
        history.undo(2, dir.path()).unwrap();
        assert_eq!(
            history.current.retained_intervals,
            vec![ri(3_000, 4_000), ri(6_000, 10_000)]
        );
    }

    #[test]
    fn ripple_trim_refuses_to_empty_the_timeline() {
        let dir = tempdir().unwrap();
        let mut history =
            EditHistory::new(EditDocument::from_retained(vec![ri(0, 4_000)]).unwrap());
        assert!(history
            .ripple_trim(0, 4_000, TrimSide::Previous, dir.path())
            .is_err());
        assert!(history
            .ripple_trim(0, 0, TrimSide::Next, dir.path())
            .is_err());
        assert!(history
            .ripple_trim(0, 0, TrimSide::Previous, dir.path())
            .is_err());
        assert_eq!(history.current.revision, 0);
    }

    fn spans(list: &[RetainedInterval]) -> Vec<(u64, u64)> {
        list.iter().map(|i| (i.start_us, i.end_us)).collect()
    }

    #[test]
    fn moving_a_clip_reorders_the_timeline_and_restoring_keeps_the_order() {
        const S: u64 = 1_000_000;
        let base = vec![ri(0, 10 * S)];
        // Move edited [6s, 10s) to the front.
        let moved = move_edited_range(&base, 6 * S, 10 * S, 0).unwrap();
        assert_eq!(spans(&moved), vec![(6 * S, 10 * S), (0, 6 * S)]);
        // Moving it back to the end restores the single interval.
        let back = move_edited_range(&moved, 0, 4 * S, 10 * S).unwrap();
        assert_eq!(spans(&back), vec![(0, 10 * S)]);
        // A middle clip moves to the end; edges at the target split correctly.
        let three = vec![ri(0, 2 * S), ri(4 * S, 6 * S), ri(8 * S, 10 * S)];
        let moved = move_edited_range(&three, 2 * S, 4 * S, 6 * S).unwrap();
        assert_eq!(
            spans(&moved),
            vec![(0, 2 * S), (8 * S, 10 * S), (4 * S, 6 * S)]
        );
        // Part of a clip can move into the middle of another one.
        let moved = move_edited_range(&base, 0, 2 * S, 5 * S).unwrap();
        assert_eq!(
            spans(&moved),
            vec![(2 * S, 5 * S), (0, 2 * S), (5 * S, 10 * S)]
        );
        assert!(move_edited_range(&base, 2 * S, 6 * S, 4 * S).is_err());
        assert!(move_edited_range(&base, 0, 2 * S, 11 * S).is_err());

        // Restoring the cut between reordered clips extends the clip it touches in place.
        let reordered = vec![ri(8 * S, 10 * S), ri(0, 2 * S), ri(4 * S, 6 * S)];
        let restored = restore_in_order(&reordered, &[(2 * S, 3 * S)], RestoreGrow::End);
        assert_eq!(
            spans(&restored),
            vec![(8 * S, 10 * S), (0, 3 * S), (4 * S, 6 * S)]
        );
        // Restoring a gap between list neighbours that are contiguous in source joins them.
        let restored = restore_in_order(&restored, &[(3 * S, 4 * S)], RestoreGrow::End);
        assert_eq!(spans(&restored), vec![(8 * S, 10 * S), (0, 6 * S)]);
        // Time that is already on the timeline is never added twice; the free part (6-8s)
        // extends the clip that ends where it starts.
        let restored = restore_in_order(&restored, &[(5 * S, 9 * S)], RestoreGrow::End);
        assert_eq!(spans(&restored), vec![(8 * S, 10 * S), (0, 8 * S)]);
        validate_retained(&restored).unwrap();

        // B (3-5s) plays before A (0-2s); the removed 2-3s touches both.
        let swapped = vec![ri(3 * S, 5 * S), ri(0, 2 * S)];
        assert_eq!(
            spans(&restore_in_order(
                &swapped,
                &[(2 * S, 3 * S)],
                RestoreGrow::Start
            )),
            vec![(2 * S, 5 * S), (0, 2 * S)],
            "dragging B's start back grows B"
        );
        assert_eq!(
            spans(&restore_in_order(
                &swapped,
                &[(2 * S, 3 * S)],
                RestoreGrow::End
            )),
            vec![(3 * S, 5 * S), (0, 3 * S)],
            "dragging A's end out grows A"
        );
    }

    #[test]
    fn move_range_is_undoable_and_cuts_keep_the_order() {
        const S: u64 = 1_000_000;
        let dir = tempdir().unwrap();
        let mut history =
            EditHistory::new(EditDocument::from_retained(vec![ri(0, 10 * S)]).unwrap());
        history.move_range(0, 6 * S, 10 * S, 0, dir.path()).unwrap();
        assert_eq!(
            spans(&history.current.retained_intervals),
            vec![(6 * S, 10 * S), (0, 6 * S)]
        );
        // Cut the first second of the second clip (source 0..1s at edited 4..5s).
        history
            .ripple_cuts(1, &[(4 * S, 5 * S)], dir.path())
            .unwrap();
        assert_eq!(
            spans(&history.current.retained_intervals),
            vec![(6 * S, 10 * S), (S, 6 * S)]
        );
        let loaded = load_edit_document(dir.path()).unwrap().unwrap();
        assert_eq!(
            loaded.retained_intervals,
            history.current.retained_intervals
        );
        history.undo(2, dir.path()).unwrap();
        history.undo(3, dir.path()).unwrap();
        assert_eq!(
            spans(&history.current.retained_intervals),
            vec![(0, 10 * S)]
        );
    }

    #[test]
    fn documents_without_split_points_still_load() {
        let json =
            r#"{"schemaVersion":1,"revision":3,"retainedIntervals":[{"startUs":0,"endUs":10}]}"#;
        let doc: EditDocument = serde_json::from_str(json).unwrap();
        assert!(doc.split_points_us.is_empty());
        assert!(!serde_json::to_string(&doc).unwrap().contains("splitPoints"));
    }
}
