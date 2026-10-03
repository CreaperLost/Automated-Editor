//! Versioned edit document (`project.json`). Source media is never rewritten.
use super::layout::validate_layout;
use super::reader::{open_regular, safe_path, RetainedInterval};
use crate::timeline::{SourceInterval, TimelineMapper};
use crate::webcam_focus::WebcamFocus;
use crate::zoom::{
    validate_zooms, ZoomKeyframe, ZoomSource, ZoomSuggestion, MAX_DISMISSED_ZOOMS, MAX_ZOOMS,
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
    /// Chapter markers, anchored in source time. Exported as MP4 chapters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chapters: Vec<crate::chapters::Chapter>,
    /// Vertical clips picked from this video, anchored in source time.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shorts: Vec<crate::shorts::Short>,
    /// Set only on the document a short renders from: draw a split-screen vertical frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_layout: Option<crate::shorts::ShortLayout>,
    /// Video tracks V2, V3, ... above the main sequence, bottom to top.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overlay_tracks: Vec<crate::tracks::OverlayTrack>,
    /// V1 as a track: magnetic or not, hidden, muted, and where it sits among the video tracks.
    #[serde(default, skip_serializing_if = "MainTrack::is_default")]
    pub main_track: MainTrack,
}

/// A V1 entry that is empty time: black where nothing else is drawn, silent.
pub const GAP: &str = "@gap";

/// V1's settings as a track.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct MainTrack {
    /// Cuts close up and moves insert (on by default); off, they leave gaps and overwrite.
    pub magnetic: bool,
    pub hidden: bool,
    pub muted: bool,
    /// How many video tracks are below V1 (0: V1 is at the bottom).
    pub position: usize,
}

impl Default for MainTrack {
    fn default() -> Self {
        Self {
            magnetic: true,
            hidden: false,
            muted: false,
            position: 0,
        }
    }
}

impl MainTrack {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
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
            chapters: Vec::new(),
            shorts: Vec::new(),
            short_layout: None,
            overlay_tracks: Vec::new(),
            main_track: MainTrack::default(),
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
            chapters: Vec::new(),
            shorts: Vec::new(),
            short_layout: None,
            overlay_tracks: Vec::new(),
            main_track: MainTrack::default(),
        })
    }

    pub fn mapper(&self) -> Result<TimelineMapper, String> {
        mapper_for(&self.retained_intervals)
    }

    /// What stream `stream` of `asset` is when it plays on V1: the V1 sound lane's mark,
    /// else the file's own role.
    pub fn main_stream_role(
        &self,
        asset: &crate::media_bin::MediaAsset,
        stream: usize,
    ) -> crate::media_bin::SoundRole {
        self.audio
            .lane_role(&crate::media::audio::main_sound_lane(stream))
            .unwrap_or_else(|| asset.sound_role(stream))
    }

    /// The project recording's zooms (imported recordings keep their own).
    pub fn zoom_suggestions(&self) -> Vec<ZoomSuggestion> {
        self.media_zoom_suggestions(None)
    }

    /// Maps the time of the file behind transcript `track_id` onto the edited timeline. A
    /// recording track's transcript is in recording time: the usual mapper. Imported sound
    /// (`msound-<stream>-<asset>`) is in that file's own time: wherever its clips play it, on
    /// any track.
    pub fn mapper_for_transcript(
        &self,
        track_id: &str,
    ) -> Result<crate::timeline::TimelineMapper, String> {
        match media_sound(track_id) {
            None => self.mapper(),
            Some((stream, asset_id)) => Ok(self.mapper_for_sound(asset_id, stream)),
        }
    }

    /// The zooms on the clock of `media` (an imported recording), or of the project's own
    /// recording when `None`.
    pub fn media_zoom_suggestions(&self, media: Option<&str>) -> Vec<ZoomSuggestion> {
        self.zooms
            .iter()
            .filter(|zoom| zoom.media.as_deref() == media)
            .map(ZoomKeyframe::as_suggestion)
            .collect()
    }

    /// Maps the clock of imported media `asset_id` onto the edited timeline through its
    /// clips on V1 (everything else on V1 maps nothing).
    pub fn mapper_for_media(&self, asset_id: &str) -> crate::timeline::TimelineMapper {
        crate::timeline::TimelineMapper::new(
            self.retained_intervals
                .iter()
                .enumerate()
                .map(|(i, interval)| {
                    let own = interval.media.as_deref() == Some(asset_id);
                    SourceInterval::new(format!("ret-{i}"), interval.start_us, interval.end_us)
                        .with_media((!own).then(|| "other".to_string()))
                })
                .collect(),
        )
    }

    /// Maps one sound stream of imported file `asset_id` (its own time) onto the edited
    /// timeline through every clip that plays it: on V1, on video tracks and on audio tracks.
    /// Where clips of it overlap (in the timeline or in the file), the earlier one counts.
    pub fn mapper_for_sound(
        &self,
        asset_id: &str,
        stream: usize,
    ) -> crate::timeline::TimelineMapper {
        crate::timeline::TimelineMapper::new(
            self.sound_retained(asset_id, stream)
                .iter()
                .enumerate()
                .map(|(i, entry)| {
                    SourceInterval::new(format!("snd-{i}"), entry.start_us, entry.end_us)
                        .with_media(entry.media.clone())
                })
                .collect(),
        )
    }

    /// Where one sound stream of an imported file plays, as timeline entries in its own time:
    /// each clip of it (on any track) as a "recording" range, and the time between as gaps
    /// that map nowhere. Overlapping uses keep the earlier one.
    pub fn sound_retained(&self, asset_id: &str, stream: usize) -> Vec<RetainedInterval> {
        let mut placements: Vec<_> = crate::media::audio::media_placements(self)
            .into_iter()
            .filter(|p| p.asset_id == asset_id && p.stream == stream && p.len > 0)
            .collect();
        placements.sort_by_key(|p| (p.edited_start, p.in_us));
        let mut intervals = Vec::new();
        let mut used: Vec<(u64, u64)> = Vec::new();
        let mut cursor = 0u64;
        for placed in placements {
            let file = (placed.in_us, placed.in_us + placed.len);
            if placed.edited_start < cursor || used.iter().any(|&(a, b)| file.0 < b && a < file.1) {
                continue;
            }
            if placed.edited_start > cursor {
                // Time where this sound does not play: maps to nothing.
                intervals.push(RetainedInterval {
                    start_us: 0,
                    end_us: placed.edited_start - cursor,
                    media: Some("gap".into()),
                    audio_unlinked: false,
                });
            }
            intervals.push(RetainedInterval::recording(file.0, file.1));
            used.push(file);
            cursor = placed.edited_start + placed.len;
        }
        intervals
    }

    /// The zooms with where each lands on the edited timeline, each on its own clock.
    pub fn zooms_with_ranges(&self) -> Vec<ZoomKeyframe> {
        let mut zooms = self.zooms.clone();
        let main = self.mapper().ok();
        crate::zoom::attach_zoom_edited_ranges_with(&mut zooms, &|media| match media {
            None => main.clone(),
            Some(asset) => Some(self.mapper_for_media(asset)),
        });
        zooms
    }

    pub fn attach_zoom_ranges(&mut self) -> Result<(), String> {
        let mapper = self.mapper()?;
        let document = self.clone();
        crate::zoom::attach_zoom_edited_ranges_with(&mut self.zooms, &|media| match media {
            None => Some(mapper.clone()),
            Some(asset) => Some(document.mapper_for_media(asset)),
        });
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
                audio_unlinked: false,
            });
        }
        cursor = cursor.max(end);
    }
    if cursor < source_duration_us {
        removed.push(RetainedInterval {
            start_us: cursor,
            end_us: source_duration_us,
            media: None,
            audio_unlinked: false,
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
            // Gaps side by side are one gap.
            Some(last) if last.is_gap() && interval.is_gap() => {
                last.end_us += interval.end_us - interval.start_us;
            }
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
    // Nothing after the last clip: the timeline ends there.
    while out.last().is_some_and(RetainedInterval::is_gap) {
        out.pop();
    }
    out
}

/// Empty V1 time `len_us` long.
pub fn gap(len_us: u64) -> RetainedInterval {
    RetainedInterval {
        start_us: 0,
        end_us: len_us,
        media: Some(GAP.into()),
        audio_unlinked: false,
    }
}

/// Without magnetism: the V1 ranges become gaps; nothing moves.
pub(crate) fn lift_main(document: &mut EditDocument, cuts: &[(u64, u64)]) -> Result<(), String> {
    let mut ordered = cuts.to_vec();
    ordered.sort_unstable();
    let retained = &mut document.retained_intervals;
    let duration: u64 = retained.iter().map(|i| i.end_us - i.start_us).sum();
    for &(start, end) in ordered.iter().rev() {
        if start >= end || end > duration {
            return Err("That range is not on the timeline".into());
        }
        let first = split_at_edited(retained, start)?;
        let last = split_at_edited(retained, end)?;
        retained.splice(first..last, [gap(end - start)]);
    }
    document.retained_intervals =
        canonical_retained(std::mem::take(&mut document.retained_intervals));
    Ok(())
}

/// Without magnetism: puts `entries` on V1 at `start_us`, over whatever was there (past the
/// end, the time before it becomes a gap).
pub(crate) fn place_main(
    document: &mut EditDocument,
    entries: Vec<RetainedInterval>,
    start_us: u64,
) -> Result<(), String> {
    let length: u64 = entries.iter().map(|e| e.end_us - e.start_us).sum();
    if length == 0 {
        return Err("Nothing to place".into());
    }
    let retained = &mut document.retained_intervals;
    let duration: u64 = retained.iter().map(|i| i.end_us - i.start_us).sum();
    if start_us + length > duration {
        retained.push(gap(start_us + length - duration));
    }
    let first = split_at_edited(retained, start_us)?;
    let last = split_at_edited(retained, start_us + length)?;
    retained.splice(first..last, entries);
    document.retained_intervals =
        canonical_retained(std::mem::take(&mut document.retained_intervals));
    Ok(())
}

/// The imported file behind a sound id `msound-<stream>-<asset>`.
pub fn media_sound_asset(track_id: &str) -> Option<&str> {
    track_id
        .strip_prefix("msound-")?
        .split_once('-')
        .map(|(_, asset)| asset)
}

/// The stream and imported file behind a sound id `msound-<stream>-<asset>`.
pub fn media_sound(track_id: &str) -> Option<(usize, &str)> {
    let (stream, asset) = track_id.strip_prefix("msound-")?.split_once('-')?;
    Some((stream.parse().ok()?, asset))
}

/// The sound id of stream `stream` of imported file `asset_id`.
pub fn media_sound_id(stream: usize, asset_id: &str) -> String {
    format!("msound-{stream}-{asset_id}")
}

/// Splits V1 at `edited_us`: imported media becomes two entries, the recording gets a split
/// point. Errors when there is already an edge there.
pub(crate) fn split_main(document: &mut EditDocument, edited_us: u64) -> Result<(), String> {
    let mapper = document.mapper()?;
    if mapper.media_at(edited_us).is_some() {
        let before = document.retained_intervals.len();
        split_at_edited(&mut document.retained_intervals, edited_us)?;
        if document.retained_intervals.len() == before {
            return Err("There is already a clip edge here".into());
        }
        return Ok(());
    }
    let source_us = mapper
        .edited_to_source_us(edited_us)
        .ok_or("Split point is outside the timeline")?;
    if document
        .retained_intervals
        .iter()
        .any(|interval| interval.is_recording() && interval.start_us == source_us)
    {
        return Err("There is already a clip edge here".into());
    }
    match document.split_points_us.binary_search(&source_us) {
        Ok(_) => Err("There is already a clip edge here".into()),
        Err(index) => {
            document.split_points_us.insert(index, source_us);
            Ok(())
        }
    }
}

/// Removes the edited ranges from V1 and closes the gaps, keeping each entry's own fields.
pub(crate) fn cut_main(document: &mut EditDocument, cuts: &[(u64, u64)]) -> Result<(), String> {
    let mut ordered = cuts.to_vec();
    ordered.sort_unstable();
    if ordered.windows(2).any(|w| w[0].1 > w[1].0) {
        return Err("Cut ranges overlap".into());
    }
    let retained = &mut document.retained_intervals;
    let duration: u64 = retained.iter().map(|i| i.end_us - i.start_us).sum();
    for &(start, end) in ordered.iter().rev() {
        if start >= end {
            return Err("Cut must be a half-open interval".into());
        }
        if end > duration {
            return Err("Cut exceeds edited duration".into());
        }
        let first = split_at_edited(retained, start)?;
        let last = split_at_edited(retained, end)?;
        retained.drain(first..last);
    }
    document.retained_intervals =
        canonical_retained(std::mem::take(&mut document.retained_intervals));
    Ok(())
}

/// Moves the V1 ranges (which need not touch) to edited position `target`, in timeline
/// order, as one block.
pub(crate) fn move_main(
    document: &mut EditDocument,
    ranges: &[(u64, u64)],
    target_us: u64,
) -> Result<(), String> {
    let mut ordered = ranges.to_vec();
    ordered.sort_unstable();
    if ordered.is_empty() || ordered.iter().any(|(a, b)| a >= b) {
        return Err("Choose clips to move".into());
    }
    if ordered.windows(2).any(|w| w[0].1 > w[1].0) {
        return Err("Move ranges overlap".into());
    }
    if ordered.iter().any(|&(a, b)| target_us > a && target_us < b) {
        return Err("Clips cannot move inside themselves".into());
    }
    let list = &mut document.retained_intervals;
    let duration: u64 = list.iter().map(|i| i.end_us - i.start_us).sum();
    if ordered.last().is_some_and(|r| r.1 > duration) || target_us > duration {
        return Err("Move is outside the timeline".into());
    }
    let mut points: Vec<u64> = ordered.iter().flat_map(|&(a, b)| [a, b]).collect();
    points.push(target_us);
    points.sort_unstable();
    points.dedup();
    for &point in points.iter().rev() {
        split_at_edited(list, point)?;
    }
    let original = list.clone();
    // Every entry now lies wholly inside or outside each range, and before or after the target.
    let (mut before, mut moved, mut after) = (Vec::new(), Vec::new(), Vec::new());
    let mut cursor = 0u64;
    for entry in list.drain(..) {
        let (a, b) = (cursor, cursor + entry.end_us - entry.start_us);
        cursor = b;
        if ordered.iter().any(|&(s, e)| a >= s && b <= e) {
            moved.push(entry);
        } else if b <= target_us {
            before.push(entry);
        } else {
            after.push(entry);
        }
    }
    before.extend(moved);
    before.extend(after);
    if before == original {
        document.retained_intervals = canonical_retained(before);
        return Err("The clips are already there".into());
    }
    document.retained_intervals = canonical_retained(before);
    Ok(())
}

/// Makes `edited_us` an interval boundary in `retained` and returns the index of the
/// interval that starts there (`retained.len()` at the end of the timeline).
pub(crate) fn split_at_edited(
    retained: &mut Vec<RetainedInterval>,
    edited_us: u64,
) -> Result<usize, String> {
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
                    audio_unlinked: interval.audio_unlinked,
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
            audio_unlinked: false,
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
                        audio_unlinked: false,
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
        crate::chapters::validate(&next.chapters)?;
        crate::shorts::validate(&next.shorts)?;
        crate::shorts::validate_edits(&next)?;
        crate::tracks::validate(&next)?;
        if next.short_layout.is_some() {
            return Err("A project's own edit cannot use a short's split layout".into());
        }
        if let Some(missing) = next.retained_intervals.iter().find_map(|entry| {
            entry
                .media
                .as_ref()
                .filter(|id| *id != GAP)
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

    /// Replaces the chapter markers.
    pub fn set_chapters(
        &mut self,
        expected_revision: u64,
        chapters: Vec<crate::chapters::Chapter>,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if expected_revision != self.current.revision {
            return Err("Stale edit revision".into());
        }
        crate::chapters::validate(&chapters)?;
        let chapters = crate::chapters::normalized(chapters);
        if chapters == self.current.chapters {
            return Ok(&self.current);
        }
        let mut next = self.current.clone();
        next.chapters = chapters;
        self.commit_next(expected_revision, persist_root, next)
    }

    /// Replaces the shorts list.
    pub fn set_shorts(
        &mut self,
        expected_revision: u64,
        shorts: Vec<crate::shorts::Short>,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if expected_revision != self.current.revision {
            return Err("Stale edit revision".into());
        }
        crate::shorts::validate(&shorts)?;
        let shorts = crate::shorts::normalized(shorts);
        if shorts == self.current.shorts {
            return Ok(&self.current);
        }
        let mut next = self.current.clone();
        next.shorts = shorts;
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
        let pieces = self
            .current
            .mapper()?
            .edited_range_to_source(edited_start_us, edited_end_us);
        if pieces.is_empty() {
            return Err("Webcam focus needs part of the recording, not only imported media".into());
        }
        let mut focus = self.current.webcam_focus.clone();
        focus.enabled = true;
        focus.add_focus(&pieces);
        self.update_webcam_focus(expected_revision, focus, persist_root)
    }

    /// Switches webcam focus off over an edited range, whichever segments cover it.
    pub fn remove_webcam_focus(
        &mut self,
        expected_revision: u64,
        edited_start_us: u64,
        edited_end_us: u64,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if edited_end_us <= edited_start_us {
            return Err("Webcam focus must be a half-open edited range".into());
        }
        let pieces = self
            .current
            .mapper()?
            .edited_range_to_source(edited_start_us, edited_end_us);
        let mut focus = self.current.webcam_focus.clone();
        focus.remove_focus(&pieces);
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
        // A zoom stays on its own clock.
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
        let main = self.current.mapper()?;
        // Over an imported clip the zoom is on that file's clock.
        let media = main
            .media_at(edited_start_us)
            .map(|(asset, _)| asset.to_string());
        let mapper = match &media {
            Some(asset) => self.current.mapper_for_media(asset),
            None => main,
        };
        let source_start = mapper
            .edited_to_source_us(edited_start_us)
            .ok_or("Zoom start is not on retained media")?;
        let source_end_sample = mapper
            .edited_to_source_us(edited_end_us.saturating_sub(1))
            .ok_or("Zoom end must be on the same clip as its start")?;
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
            media,
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
        let mut next = self.current.clone();
        cut_main(&mut next, cuts)?;
        self.commit_next(expected_revision, persist_root, next)
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
        let mut next = self.current.clone();
        split_main(&mut next, edited_us)?;
        self.commit_next(expected_revision, persist_root, next)
    }

    /// Puts removed source ranges back on the timeline. The caller clips the
    /// ranges to media that was actually removed.
    ///
    /// With `shift_tracks_at`, clips on the other tracks that start there or later move
    /// right by the restored length, so they stay in step with V1.
    pub fn restore(
        &mut self,
        expected_revision: u64,
        ranges: &[(u64, u64)],
        grow: RestoreGrow,
        shift_tracks_at: Option<u64>,
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
        let mut next = self.current.clone();
        next.retained_intervals = retained;
        if let Some(at_us) = shift_tracks_at {
            let grown = next
                .edited_duration_us()?
                .saturating_sub(self.current.edited_duration_us()?);
            crate::tracks::shift_from(&mut next, at_us, grown as i64);
        }
        self.commit_next(expected_revision, persist_root, next)
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
        crate::tracks::remove_asset(&mut next, asset_id);
        // Shorts edited on their own lose it too.
        for short in &mut next.shorts {
            if let Some(own) = &mut short.edit {
                own.retained_intervals
                    .retain(|entry| entry.media.as_deref() != Some(asset_id));
                own.retained_intervals =
                    canonical_retained(std::mem::take(&mut own.retained_intervals));
                for track in &mut own.overlay_tracks {
                    track.clips.retain(|clip| clip.asset_id != asset_id);
                }
            }
        }
        // An empty timeline is valid: a project can be built from nothing but imports.
        next.retained_intervals = canonical_retained(next.retained_intervals);
        self.commit_next(expected_revision, persist_root, next)
    }

    /// Sets what an imported file's picture and sound streams are.
    pub fn set_media_roles(
        &mut self,
        expected_revision: u64,
        asset_id: &str,
        picture_role: crate::media_bin::PictureRole,
        sound_roles: Vec<crate::media_bin::SoundRole>,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if expected_revision != self.current.revision {
            return Err("Stale edit revision".into());
        }
        let mut next = self.current.clone();
        let asset = next
            .media_assets
            .iter_mut()
            .find(|asset| asset.id == asset_id)
            .ok_or("No such imported media")?;
        if asset.picture_role == picture_role && asset.sound_roles == sound_roles {
            return Err("Nothing changed".into());
        }
        asset.picture_role = picture_role;
        asset.sound_roles = sound_roles;
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
                audio_unlinked: false,
            },
        );
        self.commit_next(expected_revision, persist_root, next)
    }

    /// One timeline change made in short `short_id`'s own timeline. The short's first edit
    /// copies its stretch of the video into it; from then on it is edited on its own.
    pub fn edit_short_tracks(
        &mut self,
        expected_revision: u64,
        short_id: &str,
        edit: &crate::tracks::TrackEdit,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if expected_revision != self.current.revision {
            return Err("Stale edit revision".into());
        }
        let short = self
            .current
            .shorts
            .iter()
            .find(|s| s.id == short_id)
            .ok_or("That short no longer exists")?;
        let timeline = crate::shorts::short_timeline(&self.current, short)?;
        let changed = crate::tracks::apply(&timeline, edit)?;
        let next = crate::shorts::with_short_timeline(&self.current, short_id, &changed)?;
        self.commit_next(expected_revision, persist_root, next)
    }

    /// Lets a short follow the video again: its own edit is dropped.
    pub fn resync_short(
        &mut self,
        expected_revision: u64,
        short_id: &str,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if expected_revision != self.current.revision {
            return Err("Stale edit revision".into());
        }
        let mut next = self.current.clone();
        let short = next
            .shorts
            .iter_mut()
            .find(|s| s.id == short_id)
            .ok_or("That short no longer exists")?;
        if short.edit.take().is_none() {
            return Err("This short already follows the video".into());
        }
        self.commit_next(expected_revision, persist_root, next)
    }

    /// One change to the video tracks above the main sequence.
    pub fn edit_tracks(
        &mut self,
        expected_revision: u64,
        edit: &crate::tracks::TrackEdit,
        persist_root: &Path,
    ) -> Result<&EditDocument, String> {
        if expected_revision != self.current.revision {
            return Err("Stale edit revision".into());
        }
        let next = crate::tracks::apply(&self.current, edit)?;
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
            audio_unlinked: false,
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
                audio_unlinked: false,
            },
            RetainedInterval {
                start_us: 6_000_000,
                end_us: 10_000_000,
                media: None,
                audio_unlinked: false,
            },
        ])
        .unwrap();
        let mut history = EditHistory::new(initial);
        // Edited 3s..5s spans the cut, so it maps to source 3s..4s and 6s..7s.
        history
            .add_webcam_focus(0, 3_000_000, 5_000_000, dir.path())
            .unwrap();
        let focus = &history.current.webcam_focus;
        assert!(focus.enabled);
        let sources = |focus: &WebcamFocus| {
            focus
                .segments
                .iter()
                .map(|s| (s.source_start_us, s.source_end_us))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            sources(focus),
            vec![(3_000_000, 4_000_000), (6_000_000, 7_000_000)]
        );
        assert_eq!(
            focus.edited_ranges(&history.current.mapper().unwrap()),
            vec![(3_000_000, 5_000_000)]
        );
        // Adding it again does not stack; removing part of it is one undoable edit.
        history
            .add_webcam_focus(1, 3_000_000, 5_000_000, dir.path())
            .unwrap();
        assert_eq!(history.current.revision, 1);
        history
            .remove_webcam_focus(1, 4_500_000, 5_000_000, dir.path())
            .unwrap();
        assert_eq!(
            sources(&history.current.webcam_focus),
            vec![(3_000_000, 4_000_000), (6_000_000, 6_500_000)]
        );
        history.undo(2, dir.path()).unwrap();
        // Undo and redo are revisions of their own; continue from the restored focus.
        let revision = history.current.revision;
        assert_eq!(
            sources(&history.current.webcam_focus),
            vec![(3_000_000, 4_000_000), (6_000_000, 7_000_000)]
        );
        let on_disk = load_edit_document(dir.path()).unwrap().unwrap();
        assert_eq!(on_disk.webcam_focus, history.current.webcam_focus);

        // UI-only edited ranges are dropped, and an unchanged update is not a revision.
        let mut echoed = history.current.webcam_focus.clone();
        echoed.attach_edited_ranges(&history.current.mapper().unwrap());
        history
            .update_webcam_focus(revision, echoed, dir.path())
            .unwrap();
        assert_eq!(history.current.revision, revision);

        let mut off = history.current.webcam_focus.clone();
        off.enabled = false;
        history
            .update_webcam_focus(revision, off, dir.path())
            .unwrap();
        assert!(!history.current.webcam_focus.enabled);
        history.undo(revision + 1, dir.path()).unwrap();
        assert!(history.current.webcam_focus.enabled);

        let mut bad = history.current.webcam_focus.clone();
        bad.settings.focus_size_pct = 10.0;
        assert!(history
            .update_webcam_focus(history.current.revision, bad, dir.path())
            .is_err());
    }

    #[test]
    fn chapters_are_undoable_and_saved() {
        use crate::chapters::Chapter;
        let dir = tempdir().unwrap();
        let mut history =
            EditHistory::new(EditDocument::from_retained(vec![ri(0, 10_000_000)]).unwrap());
        let chapter = |id: &str, source_us: u64, title: &str| Chapter {
            id: id.into(),
            source_us,
            title: title.into(),
            edited_us: Some(123),
        };
        history
            .set_chapters(
                0,
                vec![chapter("b", 5_000_000, " Main "), chapter("a", 0, "Intro")],
                dir.path(),
            )
            .unwrap();
        let saved = &history.current.chapters;
        assert_eq!(saved[0].id, "a");
        assert_eq!(saved[1].title, "Main");
        assert!(saved.iter().all(|c| c.edited_us.is_none()));
        let json = fs::read_to_string(dir.path().join("project.json")).unwrap();
        assert!(json.contains("\"chapters\"") && !json.contains("editedUs"));
        assert_eq!(
            load_edit_document(dir.path()).unwrap().unwrap().chapters,
            history.current.chapters
        );
        // The same chapters again are not a new revision; a bad title is refused.
        let same = history.current.chapters.clone();
        history.set_chapters(1, same, dir.path()).unwrap();
        assert_eq!(history.current.revision, 1);
        assert!(history
            .set_chapters(1, vec![chapter("x", 0, "")], dir.path())
            .is_err());
        history.undo(1, dir.path()).unwrap();
        assert!(history.current.chapters.is_empty());
    }

    #[test]
    fn second_editor_cannot_overwrite_newer_disk_revision() {
        let dir = tempdir().unwrap();
        let initial = EditDocument::from_retained(vec![RetainedInterval {
            start_us: 0,
            end_us: 1_000_000,
            media: None,
            audio_unlinked: false,
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
                audio_unlinked: false,
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
                audio_unlinked: false,
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
            media: None,
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
                audio_unlinked: false,
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
                audio_unlinked: false,
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
            audio_unlinked: false,
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
            source_path: None,
            missing: false,
            picture_role: Default::default(),
            sound_roles: Vec::new(),
            recording_path: None,
            audio_path: None,
            extra_audio_paths: Vec::new(),
            audio_names: Vec::new(),
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
            audio_unlinked: false,
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
    fn unlinked_media_stays_unlinked_through_cuts_and_splits() {
        let dir = tempdir().unwrap();
        let clip = RetainedInterval {
            start_us: 0,
            end_us: 6_000_000,
            media: Some("m1".into()),
            audio_unlinked: true,
        };
        let mut history =
            EditHistory::new(EditDocument::from_retained(vec![ri(0, 4_000_000), clip]).unwrap());
        history.current.media_assets = vec![crate::media_bin::MediaAsset {
            id: "m1".into(),
            name: "clip.mp4".into(),
            kind: crate::media_bin::MediaKind::Video,
            relative_path: "assets/media/m1.mp4".into(),
            source_path: None,
            missing: false,
            picture_role: Default::default(),
            sound_roles: Vec::new(),
            recording_path: None,
            audio_path: None,
            extra_audio_paths: Vec::new(),
            audio_names: Vec::new(),
            duration_us: 6_000_000,
            width: 1920,
            height: 1080,
        }];
        history
            .ripple_cuts(
                0,
                &[(1_000_000, 2_000_000), (5_000_000, 6_000_000)],
                dir.path(),
            )
            .unwrap();
        history.split(1, 7_000_000, dir.path()).unwrap();
        let media: Vec<_> = history
            .current
            .retained_intervals
            .iter()
            .filter(|i| i.media.is_some())
            .collect();
        // The second cut and the split each divide the clip: three pieces, all unlinked.
        assert_eq!(media.len(), 3);
        assert!(media.iter().all(|i| i.audio_unlinked));
    }

    #[test]
    fn imported_sound_transcripts_map_through_their_own_clips() {
        // Recording 0..4 s, then 2 s of m1 (from 1 s into the file), then 1 s of m2.
        let media = |id: &str, start_us, end_us| RetainedInterval {
            start_us,
            end_us,
            media: Some(id.into()),
            audio_unlinked: false,
        };
        let document = EditDocument::from_retained(vec![
            ri(0, 4_000_000),
            media("m1", 1_000_000, 3_000_000),
            media("m2", 0, 1_000_000),
        ])
        .unwrap();
        let mut document = document;
        document.media_assets = ["m1", "m2"]
            .map(|id| crate::media_bin::MediaAsset {
                id: id.into(),
                name: id.into(),
                kind: crate::media_bin::MediaKind::Video,
                relative_path: format!("assets/media/{id}.mp4"),
                source_path: None,
                missing: false,
                picture_role: Default::default(),
                sound_roles: Vec::new(),
                recording_path: None,
                audio_path: Some(format!("assets/media/{id}.audio.wav")),
                extra_audio_paths: Vec::new(),
                audio_names: Vec::new(),
                duration_us: 10_000_000,
                width: 0,
                height: 0,
            })
            .to_vec();
        let id = media_sound_id(0, "m1");
        assert_eq!(media_sound(&id), Some((0, "m1")));
        let mapper = document.mapper_for_transcript(&id).unwrap();
        // A word 1.5 s into m1 plays 0.5 s into its clip, which starts at 4 s.
        assert_eq!(
            mapper.edited_span_of(1_500_000, 1_700_000),
            Some((4_500_000, 4_700_000))
        );
        // Words outside the used part, and the recording's own time, map nowhere.
        assert_eq!(mapper.edited_span_of(200_000, 300_000), None);
        // A recording track keeps the usual mapping.
        let usual = document.mapper_for_transcript("mic-1").unwrap();
        assert_eq!(
            usual.edited_span_of(1_000_000, 2_000_000),
            Some((1_000_000, 2_000_000))
        );
    }

    #[test]
    fn a_zoom_whose_footage_is_all_cut_has_no_place_on_the_timeline() {
        let dir = tempdir().unwrap();
        let mut history =
            EditHistory::new(EditDocument::from_retained(vec![ri(0, 10_000_000)]).unwrap());
        history
            .add_manual_zoom(0, 2_000_000, 4_000_000, 0.5, 0.5, 2.0, dir.path())
            .unwrap();
        assert!(!history.current.zooms_with_ranges()[0]
            .edited_ranges
            .is_empty());
        history
            .ripple_cuts(1, &[(1_000_000, 5_000_000)], dir.path())
            .unwrap();
        assert!(
            history.current.zooms_with_ranges()[0]
                .edited_ranges
                .is_empty(),
            "nothing of it is left to show"
        );
    }

    #[test]
    fn transcripts_follow_sound_on_any_track_and_files_used_twice() {
        use crate::tracks::{OverlayClip, OverlayFit, OverlayTrack, TrackKind};
        // A song only on an audio track at 5 s; a talk on V1 twice (overlapping in the file).
        let talk = |start_us, end_us| RetainedInterval {
            start_us,
            end_us,
            media: Some("talk".into()),
            audio_unlinked: false,
        };
        let mut document =
            EditDocument::from_retained(vec![talk(0, 3_000_000), talk(1_000_000, 4_000_000)])
                .unwrap();
        let asset = |id: &str, kind| crate::media_bin::MediaAsset {
            id: id.into(),
            name: id.into(),
            kind,
            relative_path: format!("assets/media/{id}.wav"),
            source_path: None,
            missing: false,
            picture_role: Default::default(),
            sound_roles: Vec::new(),
            recording_path: None,
            audio_path: Some(format!("assets/media/{id}.audio.wav")),
            extra_audio_paths: Vec::new(),
            audio_names: Vec::new(),
            duration_us: 10_000_000,
            width: 0,
            height: 0,
        };
        document.media_assets = vec![
            asset("talk", crate::media_bin::MediaKind::Video),
            asset("song", crate::media_bin::MediaKind::Audio),
        ];
        document.overlay_tracks = vec![OverlayTrack {
            id: "track-1".into(),
            kind: TrackKind::Audio,
            clips: vec![OverlayClip {
                id: "clip-1".into(),
                asset_id: "song".into(),
                start_us: 5_000_000,
                in_us: 2_000_000,
                duration_us: 3_000_000,
                fit: OverlayFit::default(),
                audio_stream: Some(0),
                audio_unlinked: false,
                link: None,
            }],
            hidden: false,
            muted: false,
            role: None,
        }];
        // A word 3 s into the song plays 1 s into its clip: at 6 s.
        let song = document
            .mapper_for_transcript(&media_sound_id(0, "song"))
            .unwrap();
        assert_eq!(
            song.edited_span_of(3_000_000, 3_200_000),
            Some((6_000_000, 6_200_000))
        );
        // The talk's first use maps; the overlapping second use is skipped, not fatal.
        let talk = document
            .mapper_for_transcript(&media_sound_id(0, "talk"))
            .unwrap();
        assert_eq!(
            talk.edited_span_of(500_000, 700_000),
            Some((500_000, 700_000))
        );
        assert_eq!(talk.edited_span_of(3_500_000, 3_700_000), None);
    }

    #[test]
    fn a_short_edited_on_its_own_leaves_the_video_alone() {
        use crate::tracks::{EditedRange, TrackEdit};
        let dir = tempdir().unwrap();
        let mut history =
            EditHistory::new(EditDocument::from_retained(vec![ri(0, 60_000_000)]).unwrap());
        let short = crate::shorts::Short {
            id: "s1".into(),
            title: "Best bit".into(),
            source_start_us: 10_000_000,
            source_end_us: 30_000_000,
            reason: String::new(),
            layout: Default::default(),
            media: None,
            edit: None,
            length_us: None,
            edited_start_us: None,
            edited_end_us: None,
        };
        history.set_shorts(0, vec![short], dir.path()).unwrap();

        // Cutting 2 s out of the short's own timeline: the short is 18 s, the video still 60 s.
        history
            .edit_short_tracks(
                1,
                "s1",
                &TrackEdit::RippleDelete {
                    ranges: vec![EditedRange {
                        start_us: 0,
                        end_us: 2_000_000,
                    }],
                    all_tracks: true,
                },
                dir.path(),
            )
            .unwrap();
        assert_eq!(history.current.edited_duration_us().unwrap(), 60_000_000);
        let own = history.current.shorts[0].edit.clone().unwrap();
        assert_eq!(own.retained_intervals, vec![ri(12_000_000, 30_000_000)]);
        let exported =
            crate::shorts::short_document(&history.current, &history.current.shorts[0], false)
                .unwrap();
        assert_eq!(exported.edited_duration_us().unwrap(), 18_000_000);

        // A cut in the video no longer reaches it.
        history
            .ripple_cuts(2, &[(0, 20_000_000)], dir.path())
            .unwrap();
        let after =
            crate::shorts::short_timeline(&history.current, &history.current.shorts[0]).unwrap();
        assert_eq!(after.edited_duration_us().unwrap(), 18_000_000);

        // The 2 s come back on the short's timeline; and a re-sync follows the video again.
        history
            .edit_short_tracks(
                3,
                "s1",
                &TrackEdit::Restore {
                    ranges: vec![EditedRange {
                        start_us: 10_000_000,
                        end_us: 12_000_000,
                    }],
                    grow: RestoreGrow::Start,
                    shift_tracks_at: Some(0),
                },
                dir.path(),
            )
            .unwrap();
        let own = history.current.shorts[0].edit.clone().unwrap();
        assert_eq!(own.retained_intervals, vec![ri(10_000_000, 30_000_000)]);
        history.resync_short(4, "s1", dir.path()).unwrap();
        assert!(history.current.shorts[0].edit.is_none());
        // Following the video now, whose first 20 s were cut: the short's start is gone.
        assert!(
            crate::shorts::short_timeline(&history.current, &history.current.shorts[0]).is_err()
        );
    }

    #[test]
    fn removing_the_only_clip_leaves_an_empty_timeline() {
        let dir = tempdir().unwrap();
        let mut history = EditHistory::new(EditDocument::from_retained(vec![]).unwrap());
        let asset = crate::media_bin::MediaAsset {
            id: "m1".into(),
            name: "clip.mp4".into(),
            kind: crate::media_bin::MediaKind::Video,
            relative_path: "assets/media/m1.mp4".into(),
            source_path: None,
            missing: false,
            picture_role: Default::default(),
            sound_roles: Vec::new(),
            recording_path: None,
            audio_path: None,
            extra_audio_paths: Vec::new(),
            audio_names: Vec::new(),
            duration_us: 5_000_000,
            width: 1920,
            height: 1080,
        };
        history.add_media(0, vec![asset], dir.path()).unwrap();
        history.insert_media(1, "m1", 0, None, dir.path()).unwrap();
        history.remove_media(2, "m1", dir.path()).unwrap();
        assert!(history.current.media_assets.is_empty());
        assert!(history.current.retained_intervals.is_empty());
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
            .restore(
                2,
                &[(2_000_000, 4_000_000)],
                RestoreGrow::End,
                None,
                dir.path(),
            )
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
            .restore(0, &[(2_000, 3_000)], RestoreGrow::End, None, dir.path())
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
                None,
                dir.path(),
            )
            .unwrap();
        assert_eq!(history.current.retained_intervals, vec![ri(0, 6_000)]);
        assert!(history
            .restore(2, &[], RestoreGrow::End, None, dir.path())
            .is_err());
        assert!(history
            .restore(2, &[(10, 10)], RestoreGrow::End, None, dir.path())
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
