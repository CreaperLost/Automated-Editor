//! Tracks beside the main sequence. The main sequence (the recording and the media inserted
//! into it) is V1 and ripples as it is cut. Video tracks V2, V3, ... hold clips of imported
//! media at fixed positions on the timeline; a higher track draws over the ones below, and
//! their sound plays along with the main sequence. Audio tracks hold one audio stream of
//! imported media per clip: the sound of a clip that was unlinked from its picture.
use crate::media_bin::MediaKind;
use crate::project::revision::EditDocument;
use crate::project::RetainedInterval;
use serde::{Deserialize, Serialize};

pub const MAX_OVERLAY_TRACKS: usize = 8;
pub const MAX_AUDIO_TRACKS: usize = 8;
pub const MAX_CLIPS_PER_TRACK: usize = 256;
/// Shortest clip a trim can leave.
pub const MIN_CLIP_US: u64 = 100_000;

/// How a clip's picture fills the canvas.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OverlayFit {
    /// The whole picture, centred; what is below shows around it.
    #[default]
    Contain,
    /// The whole canvas, cropping the picture's edges.
    Cover,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OverlayClip {
    pub id: String,
    pub asset_id: String,
    /// Where the clip starts on the timeline.
    pub start_us: u64,
    /// Where in the media the clip starts.
    #[serde(default)]
    pub in_us: u64,
    pub duration_us: u64,
    #[serde(default)]
    pub fit: OverlayFit,
    /// On an audio track: which of the media's audio streams this clip plays.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_stream: Option<usize>,
    /// On a video track: its sound was split off onto audio tracks, so it plays silent.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub audio_unlinked: bool,
    /// Audio clips split off the same picture share this, so relinking takes them all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

impl OverlayClip {
    pub fn end_us(&self) -> u64 {
        self.start_us.saturating_add(self.duration_us)
    }

    /// The media time shown at timeline time `edited_us`, if the clip covers it.
    pub fn local_us(&self, edited_us: u64) -> Option<u64> {
        (self.start_us <= edited_us && edited_us < self.end_us())
            .then(|| self.in_us + (edited_us - self.start_us))
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TrackKind {
    #[default]
    Video,
    Audio,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OverlayTrack {
    pub id: String,
    #[serde(default)]
    pub kind: TrackKind,
    #[serde(default)]
    pub clips: Vec<OverlayClip>,
    /// Not drawn.
    #[serde(default)]
    pub hidden: bool,
    /// Not heard.
    #[serde(default)]
    pub muted: bool,
    /// What a video track's clips stand for, set on the timeline; unset, each file's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<crate::media_bin::PictureRole>,
}

impl OverlayTrack {
    pub fn is_audio(&self) -> bool {
        self.kind == TrackKind::Audio
    }

    pub fn clip_at(&self, edited_us: u64) -> Option<&OverlayClip> {
        self.clips
            .iter()
            .find(|clip| clip.local_us(edited_us).is_some())
    }
}

/// One change to the tracks, applied as one undoable edit.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TrackEdit {
    /// A new empty video track above the others, or audio track below the others.
    AddTrack {
        #[serde(default)]
        audio: bool,
    },
    RemoveTrack {
        track_id: String,
    },
    SetTrack {
        track_id: String,
        hidden: bool,
        muted: bool,
    },
    /// Marks a video track's clips as the screen or a webcam (`None`: each file decides).
    SetTrackRole {
        track_id: String,
        role: Option<crate::media_bin::PictureRole>,
    },
    /// Media from the bin onto a track at `start_us`, at its default length.
    PlaceMedia {
        asset_id: String,
        track_id: String,
        start_us: u64,
    },
    /// Moves, trims or refits a clip; it may change track.
    UpdateClip {
        clip: OverlayClip,
        track_id: String,
    },
    RemoveClip {
        clip_id: String,
    },
    /// Takes the media clip `[start_us, end_us)` out of the main sequence (which closes up)
    /// and puts it on a track at `at_us`.
    LiftFromMain {
        start_us: u64,
        end_us: u64,
        track_id: String,
        at_us: u64,
    },
    /// Takes a clip off its track and inserts it into the main sequence at `target_us`.
    DropToMain {
        clip_id: String,
        target_us: u64,
    },
    /// Splits the sound of the main-sequence media clip `[start_us, end_us)` off onto audio
    /// tracks, one clip per audio stream, so it can be moved and trimmed on its own.
    UnlinkMain {
        start_us: u64,
        end_us: u64,
    },
    /// The same for a clip on a video track.
    UnlinkClip {
        clip_id: String,
    },
    /// Joins the main-sequence media clip `[start_us, end_us)` with its split-off sound
    /// again: the audio clips (and every clip split off with them) go, and the clip plays
    /// its own sound in sync.
    RelinkMain {
        start_us: u64,
        end_us: u64,
        audio_clip_ids: Vec<String>,
    },
    /// The same for a clip on a video track.
    RelinkClip {
        clip_id: String,
        audio_clip_ids: Vec<String>,
    },
    /// Cuts edited ranges out of V1 and closes the gaps. With `all_tracks`, the other tracks
    /// lose the same time too, so everything after the cut stays in step.
    RippleDelete {
        ranges: Vec<EditedRange>,
        #[serde(default)]
        all_tracks: bool,
    },
    /// Removes the selected clips in one step: V1 ranges close up (V1 only), track clips go.
    DeleteSelection {
        ranges: Vec<EditedRange>,
        clip_ids: Vec<String>,
    },
    /// Splits at `at_us`: V1 when `main`, and each listed track clip that spans it.
    Split {
        at_us: u64,
        main: bool,
        clip_ids: Vec<String>,
    },
    /// Trims a track clip from `at_us` to one of its ends; the clips after it on its track
    /// close up the gap.
    RippleTrimClip {
        clip_id: String,
        side: ClipSide,
        at_us: u64,
    },
    /// Moves track clips together by `delta_us` (and to `track_id`, when only one moves).
    MoveClips {
        clip_ids: Vec<String>,
        delta_us: i64,
    },
    /// Moves V1 clips (side by side or not) to edited position `target_us`, as one block.
    MoveMain {
        ranges: Vec<EditedRange>,
        target_us: u64,
    },
    /// Puts cut recording time (`ranges`, in recording time) back on V1; `grow` picks the clip
    /// that grows. With `shift_tracks_at`, the tracks' clips from there on move right with it.
    Restore {
        ranges: Vec<EditedRange>,
        #[serde(default)]
        grow: crate::project::revision::RestoreGrow,
        #[serde(default)]
        shift_tracks_at: Option<u64>,
    },
    /// Inserts imported media into V1 at edited position `target_us`, at its default length.
    InsertMedia {
        asset_id: String,
        target_us: u64,
    },
    /// Without magnetism: moves V1 clips to start at `start_us`, over what is there, leaving
    /// gaps where they were.
    PlaceMain {
        ranges: Vec<EditedRange>,
        start_us: u64,
    },
    /// V1's switches: magnetic, hidden, muted.
    SetMainTrack {
        magnetic: bool,
        hidden: bool,
        muted: bool,
    },
    /// Moves a track one place up or down in its stack: a video track (or V1, `track_id`
    /// "main") among the video tracks, an audio track among the audio tracks.
    MoveTrack {
        track_id: String,
        up: bool,
    },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EditedRange {
    pub start_us: u64,
    pub end_us: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ClipSide {
    /// From the clip's start up to `at_us`.
    Start,
    /// From `at_us` to the clip's end.
    End,
}

fn pairs(ranges: &[EditedRange]) -> Vec<(u64, u64)> {
    ranges.iter().map(|r| (r.start_us, r.end_us)).collect()
}

/// Moves every track clip that starts at or after `at_us` by `delta_us`.
pub fn shift_from(document: &mut EditDocument, at_us: u64, delta_us: i64) {
    for track in &mut document.overlay_tracks {
        for clip in &mut track.clips {
            if clip.start_us >= at_us {
                clip.start_us = clip.start_us.saturating_add_signed(delta_us);
            }
        }
    }
}

/// Takes the edited ranges out of every track: later clips move back, clips across a cut
/// lose the cut part (a clip spanning it becomes two), slivers under 0.1 s go.
fn ripple_cut_tracks(document: &mut EditDocument, cuts: &[(u64, u64)]) {
    let mut ordered = cuts.to_vec();
    ordered.sort_unstable();
    for &(a, b) in ordered.iter().rev() {
        let length = b - a;
        let mut extra_ids = Vec::new();
        for index in 0..document.overlay_tracks.len() {
            let still = |asset_id: &str, document: &EditDocument| {
                document
                    .media_assets
                    .iter()
                    .any(|m| m.id == asset_id && m.kind == MediaKind::Image)
            };
            let clips = std::mem::take(&mut document.overlay_tracks[index].clips);
            let mut kept = Vec::with_capacity(clips.len() + 1);
            for clip in clips {
                let (s, e) = (clip.start_us, clip.end_us());
                if e <= a {
                    kept.push(clip);
                } else if s >= b {
                    kept.push(OverlayClip {
                        start_us: s - length,
                        ..clip
                    });
                } else {
                    let image = still(&clip.asset_id, document);
                    // The part before the cut keeps its place and start in the media.
                    if s < a && a - s >= MIN_CLIP_US {
                        kept.push(OverlayClip {
                            duration_us: a - s,
                            ..clip.clone()
                        });
                    }
                    // The part after the cut moves back to `a`.
                    if e > b && e - b >= MIN_CLIP_US {
                        let skip = b - s;
                        let after = OverlayClip {
                            id: if s < a {
                                String::new()
                            } else {
                                clip.id.clone()
                            },
                            start_us: a,
                            in_us: if image { 0 } else { clip.in_us + skip },
                            duration_us: e - b,
                            ..clip
                        };
                        if after.id.is_empty() {
                            extra_ids.push((index, kept.len()));
                        }
                        kept.push(after);
                    }
                }
            }
            document.overlay_tracks[index].clips = kept;
        }
        for (track, position) in extra_ids {
            let id = new_id(document, "clip");
            document.overlay_tracks[track].clips[position].id = id;
        }
    }
}

/// Without magnetism: the ranges are emptied on every track; nothing moves. A clip across a
/// range keeps the parts either side where they were.
fn lift_tracks(document: &mut EditDocument, cuts: &[(u64, u64)]) {
    for &(a, b) in cuts {
        let mut new_ids = Vec::new();
        for index in 0..document.overlay_tracks.len() {
            let stills: Vec<String> = document
                .media_assets
                .iter()
                .filter(|m| m.kind == MediaKind::Image)
                .map(|m| m.id.clone())
                .collect();
            let clips = std::mem::take(&mut document.overlay_tracks[index].clips);
            let mut kept = Vec::with_capacity(clips.len() + 1);
            for clip in clips {
                let (s, e) = (clip.start_us, clip.end_us());
                if e <= a || s >= b {
                    kept.push(clip);
                    continue;
                }
                if s < a && a - s >= MIN_CLIP_US {
                    kept.push(OverlayClip {
                        duration_us: a - s,
                        ..clip.clone()
                    });
                }
                if e > b && e - b >= MIN_CLIP_US {
                    let image = stills.contains(&clip.asset_id);
                    let after = OverlayClip {
                        id: if s < a {
                            String::new()
                        } else {
                            clip.id.clone()
                        },
                        start_us: b,
                        in_us: if image { 0 } else { clip.in_us + (b - s) },
                        duration_us: e - b,
                        ..clip
                    };
                    if after.id.is_empty() {
                        new_ids.push((index, kept.len()));
                    }
                    kept.push(after);
                }
            }
            document.overlay_tracks[index].clips = kept;
        }
        for (track, position) in new_ids {
            let id = new_id(document, "clip");
            document.overlay_tracks[track].clips[position].id = id;
        }
    }
}

/// One step up or down the stack. Video tracks and V1 ("main") share the picture's stack;
/// audio tracks only move among themselves.
fn move_track(document: &mut EditDocument, track_id: &str, up: bool) -> Result<(), String> {
    let audio = track_id != "main"
        && document
            .overlay_tracks
            .iter()
            .find(|t| t.id == track_id)
            .ok_or("No such track")?
            .is_audio();
    if audio {
        let ids: Vec<usize> = (0..document.overlay_tracks.len())
            .filter(|&i| document.overlay_tracks[i].is_audio())
            .collect();
        let at = ids
            .iter()
            .position(|&i| document.overlay_tracks[i].id == track_id)
            .ok_or("No such track")?;
        // Audio tracks list top to bottom: "up" is earlier.
        let other = if up {
            at.checked_sub(1)
        } else {
            Some(at + 1).filter(|&o| o < ids.len())
        }
        .ok_or("It is already at the end")?;
        document.overlay_tracks.swap(ids[at], ids[other]);
        return Ok(());
    }
    // The picture's stack, bottom to top, with V1 among the video tracks.
    let videos: Vec<usize> = (0..document.overlay_tracks.len())
        .filter(|&i| !document.overlay_tracks[i].is_audio())
        .collect();
    let mut stack: Vec<Option<usize>> = videos.iter().map(|&i| Some(i)).collect();
    let main_at = document.main_track.position.min(stack.len());
    stack.insert(main_at, None);
    let at = stack
        .iter()
        .position(|item| match item {
            None => track_id == "main",
            Some(i) => document.overlay_tracks[*i].id == track_id,
        })
        .ok_or("No such track")?;
    let other = if up {
        Some(at + 1).filter(|&o| o < stack.len())
    } else {
        at.checked_sub(1)
    }
    .ok_or("It is already at the end")?;
    stack.swap(at, other);
    document.main_track.position = stack.iter().position(Option::is_none).unwrap_or(0);
    // Rewrite the video tracks in their new order, keeping the audio tracks where they are.
    let order: Vec<crate::tracks::OverlayTrack> = stack
        .into_iter()
        .flatten()
        .map(|i| document.overlay_tracks[i].clone())
        .collect();
    let mut order = order.into_iter();
    for i in videos {
        document.overlay_tracks[i] = order.next().ok_or("Track order went wrong")?;
    }
    Ok(())
}

fn split_clip(document: &mut EditDocument, clip_id: &str, at_us: u64) -> Result<bool, String> {
    let clip = find_clip(document, clip_id)?.clone();
    if at_us <= clip.start_us || at_us >= clip.end_us() {
        return Ok(false);
    }
    let image = document
        .media_assets
        .iter()
        .any(|m| m.id == clip.asset_id && m.kind == MediaKind::Image);
    let head = at_us - clip.start_us;
    let tail = OverlayClip {
        id: new_id(document, "clip"),
        start_us: at_us,
        in_us: if image { 0 } else { clip.in_us + head },
        duration_us: clip.duration_us - head,
        ..clip.clone()
    };
    find_clip_mut(document, clip_id)?.duration_us = head;
    let track = document
        .overlay_tracks
        .iter()
        .find(|t| t.clips.iter().any(|c| c.id == clip_id))
        .map(|t| t.id.clone())
        .ok_or("No such clip")?;
    put_clip(document, &track, tail)?;
    Ok(true)
}

fn track_index(document: &EditDocument, track_id: &str) -> Result<usize, String> {
    document
        .overlay_tracks
        .iter()
        .position(|track| track.id == track_id)
        .ok_or_else(|| "No such track".to_string())
}

fn take_clip(document: &mut EditDocument, clip_id: &str) -> Result<OverlayClip, String> {
    for track in &mut document.overlay_tracks {
        if let Some(index) = track.clips.iter().position(|clip| clip.id == clip_id) {
            return Ok(track.clips.remove(index));
        }
    }
    Err("No such clip".into())
}

fn new_id(document: &EditDocument, prefix: &str) -> String {
    // Links count: a reused link would tie a new unlink to an older one, and relinking
    // either would take the other's sound away.
    let taken = |id: &str| {
        document.overlay_tracks.iter().any(|track| {
            track.id == id
                || track
                    .clips
                    .iter()
                    .any(|clip| clip.id == id || clip.link.as_deref() == Some(id))
        })
    };
    (1..)
        .map(|n| format!("{prefix}-{n}"))
        .find(|id| !taken(id))
        .unwrap()
}

fn put_clip(document: &mut EditDocument, track_id: &str, clip: OverlayClip) -> Result<(), String> {
    let index = track_index(document, track_id)?;
    if document.overlay_tracks[index].is_audio() != clip.audio_stream.is_some() {
        return Err(if clip.audio_stream.is_some() {
            "Audio clips go on audio tracks".into()
        } else {
            "Video clips go on video tracks".into()
        });
    }
    let clips = &mut document.overlay_tracks[index].clips;
    clips.push(clip);
    clips.sort_by_key(|clip| clip.start_us);
    Ok(())
}

/// The document after `edit`. Validation of the result happens when it is committed.
pub fn apply(document: &EditDocument, edit: &TrackEdit) -> Result<EditDocument, String> {
    let mut next = document.clone();
    match edit {
        TrackEdit::AddTrack { audio } => {
            add_track(&mut next, *audio)?;
        }
        TrackEdit::RemoveTrack { track_id } => {
            let index = track_index(&next, track_id)?;
            next.overlay_tracks.remove(index);
        }
        TrackEdit::SetTrack {
            track_id,
            hidden,
            muted,
        } => {
            let index = track_index(&next, track_id)?;
            let track = &mut next.overlay_tracks[index];
            track.hidden = *hidden;
            track.muted = *muted;
        }
        TrackEdit::SetTrackRole { track_id, role } => {
            let index = track_index(&next, track_id)?;
            if next.overlay_tracks[index].is_audio() {
                return Err("Audio tracks are marked as speech or background in the mix".into());
            }
            next.overlay_tracks[index].role = *role;
        }
        TrackEdit::PlaceMedia {
            asset_id,
            track_id,
            start_us,
        } => {
            let asset = next
                .media_assets
                .iter()
                .find(|asset| &asset.id == asset_id)
                .ok_or("No such imported media")?;
            let audio = next.overlay_tracks[track_index(&next, track_id)?].is_audio();
            if audio && asset.audio_paths().next().is_none() {
                return Err("That media has no sound to put on an audio track".into());
            }
            let clip = OverlayClip {
                id: new_id(&next, "clip"),
                asset_id: asset_id.clone(),
                start_us: *start_us,
                in_us: 0,
                duration_us: asset.default_clip_us(),
                fit: OverlayFit::default(),
                // An audio track takes the first stream; unlinking gives each its own.
                audio_stream: audio.then_some(0),
                audio_unlinked: false,
                link: None,
            };
            put_clip(&mut next, track_id, clip)?;
        }
        TrackEdit::UpdateClip { clip, track_id } => {
            let old = take_clip(&mut next, &clip.id)?;
            if old.asset_id != clip.asset_id {
                return Err("A clip keeps its media".into());
            }
            // Moves and trims only: what the clip plays and its link stay as they were.
            let clip = OverlayClip {
                audio_stream: old.audio_stream,
                audio_unlinked: old.audio_unlinked,
                link: old.link,
                ..clip.clone()
            };
            put_clip(&mut next, track_id, clip)?;
        }
        TrackEdit::RemoveClip { clip_id } => {
            take_clip(&mut next, clip_id)?;
        }
        TrackEdit::LiftFromMain {
            start_us,
            end_us,
            track_id,
            at_us,
        } => {
            let (asset_id, in_us, audio_unlinked) =
                media_under(&next.retained_intervals, *start_us, *end_us)?;
            // A still shows the same picture throughout: a split part of it starts at 0.
            let still = next
                .media_assets
                .iter()
                .any(|m| m.id == asset_id && m.kind == MediaKind::Image);
            let in_us = if still { 0 } else { in_us };
            let retained = &mut next.retained_intervals;
            let first = crate::project::revision::split_at_edited(retained, *start_us)?;
            let last = crate::project::revision::split_at_edited(retained, *end_us)?;
            // Not magnetic: the clip leaves a gap behind.
            let left = if next.main_track.magnetic {
                Vec::new()
            } else {
                vec![crate::project::revision::gap(end_us - start_us)]
            };
            let retained = &mut next.retained_intervals;
            retained.splice(first..last, left);
            if retained.is_empty() {
                return Err("The main track cannot be left empty".into());
            }
            next.retained_intervals =
                crate::project::revision::canonical_retained(next.retained_intervals);
            let clip = OverlayClip {
                id: new_id(&next, "clip"),
                asset_id,
                start_us: *at_us,
                in_us,
                duration_us: end_us - start_us,
                fit: OverlayFit::default(),
                audio_stream: None,
                audio_unlinked,
                link: None,
            };
            put_clip(&mut next, track_id, clip)?;
        }
        TrackEdit::DropToMain { clip_id, target_us } => {
            let clip = take_clip(&mut next, clip_id)?;
            if clip.audio_stream.is_some() {
                return Err("Audio clips stay on audio tracks".into());
            }
            let entry = RetainedInterval {
                start_us: clip.in_us,
                end_us: clip.in_us + clip.duration_us,
                media: Some(clip.asset_id),
                audio_unlinked: clip.audio_unlinked,
            };
            if next.main_track.magnetic {
                let at = crate::project::revision::split_at_edited(
                    &mut next.retained_intervals,
                    *target_us,
                )?;
                next.retained_intervals.insert(at, entry);
            } else {
                crate::project::revision::place_main(&mut next, vec![entry], *target_us)?;
            }
        }
        TrackEdit::UnlinkMain { start_us, end_us } => {
            let (asset_id, in_us, unlinked) =
                media_under(&next.retained_intervals, *start_us, *end_us)?;
            if unlinked {
                return Err("That clip's sound is already unlinked".into());
            }
            let retained = &mut next.retained_intervals;
            let first = crate::project::revision::split_at_edited(retained, *start_us)?;
            let last = crate::project::revision::split_at_edited(retained, *end_us)?;
            for interval in &mut retained[first..last] {
                interval.audio_unlinked = true;
            }
            split_off_audio(&mut next, &asset_id, *start_us, in_us, end_us - start_us)?;
        }
        TrackEdit::UnlinkClip { clip_id } => {
            let clip = find_clip(&next, clip_id)?.clone();
            if clip.audio_stream.is_some() {
                return Err("Select a video clip to unlink its sound".into());
            }
            if clip.audio_unlinked {
                return Err("That clip's sound is already unlinked".into());
            }
            split_off_audio(
                &mut next,
                &clip.asset_id,
                clip.start_us,
                clip.in_us,
                clip.duration_us,
            )?;
            find_clip_mut(&mut next, clip_id)?.audio_unlinked = true;
        }
        TrackEdit::RelinkMain {
            start_us,
            end_us,
            audio_clip_ids,
        } => {
            let (asset_id, _, unlinked) =
                media_under(&next.retained_intervals, *start_us, *end_us)?;
            if !unlinked {
                return Err("That clip's sound is not unlinked".into());
            }
            remove_split_audio(&mut next, &asset_id, audio_clip_ids)?;
            let retained = &mut next.retained_intervals;
            let first = crate::project::revision::split_at_edited(retained, *start_us)?;
            let last = crate::project::revision::split_at_edited(retained, *end_us)?;
            for interval in &mut retained[first..last] {
                interval.audio_unlinked = false;
            }
        }
        TrackEdit::RippleDelete { ranges, all_tracks } => {
            let cuts = pairs(ranges);
            if cuts.is_empty() {
                return Err("Nothing to cut".into());
            }
            if next.main_track.magnetic {
                crate::project::revision::cut_main(&mut next, &cuts)?;
                if *all_tracks {
                    ripple_cut_tracks(&mut next, &cuts);
                }
            } else {
                // Not magnetic: the time is emptied where it is; nothing moves.
                crate::project::revision::lift_main(&mut next, &cuts)?;
                if *all_tracks {
                    lift_tracks(&mut next, &cuts);
                }
            }
        }
        TrackEdit::DeleteSelection { ranges, clip_ids } => {
            if ranges.is_empty() && clip_ids.is_empty() {
                return Err("Nothing is selected".into());
            }
            for clip_id in clip_ids {
                take_clip(&mut next, clip_id)?;
            }
            if !ranges.is_empty() {
                if next.main_track.magnetic {
                    crate::project::revision::cut_main(&mut next, &pairs(ranges))?;
                } else {
                    crate::project::revision::lift_main(&mut next, &pairs(ranges))?;
                }
            }
        }
        TrackEdit::Split {
            at_us,
            main,
            clip_ids,
        } => {
            let mut changed = false;
            if *main {
                // An edge already here is fine when the tracks still have something to split.
                match crate::project::revision::split_main(&mut next, *at_us) {
                    Ok(()) => changed = true,
                    Err(error) if clip_ids.is_empty() => return Err(error),
                    Err(_) => {}
                }
            }
            for clip_id in clip_ids {
                changed |= split_clip(&mut next, clip_id, *at_us)?;
            }
            if !changed {
                return Err("There is nothing to split at the playhead".into());
            }
        }
        TrackEdit::RippleTrimClip {
            clip_id,
            side,
            at_us,
        } => {
            let clip = find_clip(&next, clip_id)?.clone();
            if *at_us <= clip.start_us || *at_us >= clip.end_us() {
                return Err("Put the playhead inside the clip to trim it".into());
            }
            let image = next
                .media_assets
                .iter()
                .any(|m| m.id == clip.asset_id && m.kind == MediaKind::Image);
            let removed = match side {
                ClipSide::Start => at_us - clip.start_us,
                ClipSide::End => clip.end_us() - at_us,
            };
            if clip.duration_us - removed < MIN_CLIP_US {
                return Err("That would leave less than 0.1 s of the clip".into());
            }
            let track_id = next
                .overlay_tracks
                .iter()
                .find(|t| t.clips.iter().any(|c| c.id == *clip_id))
                .map(|t| t.id.clone())
                .ok_or("No such clip")?;
            let end = clip.end_us();
            {
                let target = find_clip_mut(&mut next, clip_id)?;
                if *side == ClipSide::Start && !image {
                    target.in_us += removed;
                }
                target.duration_us -= removed;
            }
            // Later clips on the same track close up the gap.
            let index = track_index(&next, &track_id)?;
            for other in &mut next.overlay_tracks[index].clips {
                if other.id != *clip_id && other.start_us >= end {
                    other.start_us -= removed;
                }
            }
        }
        TrackEdit::MoveClips { clip_ids, delta_us } => {
            if clip_ids.is_empty() || *delta_us == 0 {
                return Err("Nothing to move".into());
            }
            for clip_id in clip_ids {
                let clip = find_clip_mut(&mut next, clip_id)?;
                clip.start_us = u64::try_from(clip.start_us as i64 + delta_us)
                    .map_err(|_| "Clips cannot move before the start".to_string())?;
            }
            for track in &mut next.overlay_tracks {
                track.clips.sort_by_key(|clip| clip.start_us);
            }
        }
        TrackEdit::MoveMain { ranges, target_us } => {
            crate::project::revision::move_main(&mut next, &pairs(ranges), *target_us)?;
        }
        TrackEdit::Restore {
            ranges,
            grow,
            shift_tracks_at,
        } => {
            // Only time no V1 clip already shows can come back.
            let shown: Vec<(u64, u64)> = next
                .retained_intervals
                .iter()
                .filter(|i| i.is_recording())
                .map(|i| (i.start_us, i.end_us))
                .collect();
            let mut wanted = Vec::new();
            for (start, end) in pairs(ranges) {
                let mut pieces = vec![(start, end)];
                for &(a, b) in &shown {
                    pieces = pieces
                        .into_iter()
                        .flat_map(|(s, e)| {
                            if b <= s || a >= e {
                                vec![(s, e)]
                            } else {
                                [(s, a.max(s)), (b.min(e), e)]
                                    .into_iter()
                                    .filter(|(x, y)| y > x)
                                    .collect()
                            }
                        })
                        .collect();
                }
                wanted.extend(pieces);
            }
            if wanted.is_empty() {
                return Err("Nothing to restore there".into());
            }
            let before = next.edited_duration_us()?;
            next.retained_intervals = crate::project::revision::restore_in_order(
                &next.retained_intervals,
                &wanted,
                *grow,
            );
            let grown = next.edited_duration_us()?.saturating_sub(before);
            if let (false, Some(at)) = (next.main_track.magnetic, *shift_tracks_at) {
                // Not magnetic: the clip grows into the gap beside it, and nothing moves.
                // (Without a place, as in restoring every cut, the cuts simply come back.)
                let mut cursor = 0u64;
                let mut room = None;
                for (index, entry) in next.retained_intervals.iter().enumerate() {
                    let len = entry.end_us - entry.start_us;
                    let beside = match grow {
                        crate::project::revision::RestoreGrow::End => cursor == at + grown,
                        crate::project::revision::RestoreGrow::Start => cursor + len == at,
                    };
                    if entry.is_gap() && beside {
                        room = Some(index);
                        break;
                    }
                    cursor += len;
                }
                let index = room
                    .filter(|&i| {
                        let e = &next.retained_intervals[i];
                        e.end_us - e.start_us >= grown
                    })
                    .ok_or("No room there: leave a gap beside the clip, or turn magnetic on")?;
                next.retained_intervals[index].end_us -= grown;
                next.retained_intervals =
                    crate::project::revision::canonical_retained(next.retained_intervals);
            } else if let Some(at_us) = shift_tracks_at {
                shift_from(&mut next, *at_us, grown as i64);
            }
        }
        TrackEdit::InsertMedia {
            asset_id,
            target_us,
        } => {
            let asset = next
                .media_assets
                .iter()
                .find(|asset| &asset.id == asset_id)
                .ok_or("No such imported media")?;
            let length = asset.default_clip_us();
            let entry = RetainedInterval {
                start_us: 0,
                end_us: length,
                media: Some(asset_id.clone()),
                audio_unlinked: false,
            };
            if next.main_track.magnetic {
                let at = crate::project::revision::split_at_edited(
                    &mut next.retained_intervals,
                    *target_us,
                )?;
                next.retained_intervals.insert(at, entry);
            } else {
                crate::project::revision::place_main(&mut next, vec![entry], *target_us)?;
            }
        }
        TrackEdit::PlaceMain { ranges, start_us } => {
            let mut ordered = pairs(ranges);
            ordered.sort_unstable();
            if ordered.is_empty() {
                return Err("Choose clips to move".into());
            }
            // The clips, in timeline order, then emptied where they were and put down anew.
            let mut entries = Vec::new();
            for &(a, b) in &ordered {
                entries.extend(crate::shorts::slice_retained(
                    &next.retained_intervals,
                    a,
                    b,
                ));
            }
            crate::project::revision::lift_main(&mut next, &ordered)?;
            crate::project::revision::place_main(&mut next, entries, *start_us)?;
        }
        TrackEdit::SetMainTrack {
            magnetic,
            hidden,
            muted,
        } => {
            next.main_track.magnetic = *magnetic;
            next.main_track.hidden = *hidden;
            next.main_track.muted = *muted;
        }
        TrackEdit::MoveTrack { track_id, up } => move_track(&mut next, track_id, *up)?,
        TrackEdit::RelinkClip {
            clip_id,
            audio_clip_ids,
        } => {
            let clip = find_clip(&next, clip_id)?.clone();
            if clip.audio_stream.is_some() || !clip.audio_unlinked {
                return Err("That clip's sound is not unlinked".into());
            }
            remove_split_audio(&mut next, &clip.asset_id, audio_clip_ids)?;
            find_clip_mut(&mut next, clip_id)?.audio_unlinked = false;
        }
    }
    Ok(next)
}

fn add_track(document: &mut EditDocument, audio: bool) -> Result<String, String> {
    let (kind, max, name) = if audio {
        (TrackKind::Audio, MAX_AUDIO_TRACKS, "audio")
    } else {
        (TrackKind::Video, MAX_OVERLAY_TRACKS, "video")
    };
    if document
        .overlay_tracks
        .iter()
        .filter(|track| track.kind == kind)
        .count()
        >= max
    {
        return Err(format!(
            "A project has at most {max} {name} tracks besides V1"
        ));
    }
    let id = new_id(document, "track");
    document.overlay_tracks.push(OverlayTrack {
        id: id.clone(),
        kind,
        clips: Vec::new(),
        hidden: false,
        muted: false,
        role: None,
    });
    Ok(id)
}

fn find_clip<'a>(document: &'a EditDocument, clip_id: &str) -> Result<&'a OverlayClip, String> {
    document
        .overlay_tracks
        .iter()
        .flat_map(|track| &track.clips)
        .find(|clip| clip.id == clip_id)
        .ok_or_else(|| "No such clip".into())
}

fn find_clip_mut<'a>(
    document: &'a mut EditDocument,
    clip_id: &str,
) -> Result<&'a mut OverlayClip, String> {
    document
        .overlay_tracks
        .iter_mut()
        .flat_map(|track| &mut track.clips)
        .find(|clip| clip.id == clip_id)
        .ok_or_else(|| "No such clip".into())
}

/// Puts one audio clip per audio stream of `asset_id` at `start_us`, each on the first audio
/// track with room (adding tracks as needed), all sharing one link.
fn split_off_audio(
    document: &mut EditDocument,
    asset_id: &str,
    start_us: u64,
    in_us: u64,
    duration_us: u64,
) -> Result<(), String> {
    let streams = document
        .media_assets
        .iter()
        .find(|asset| asset.id == asset_id)
        .ok_or("No such imported media")?
        .audio_paths()
        .count();
    if streams == 0 {
        return Err("That clip has no sound to unlink".into());
    }
    let link = new_id(document, "link");
    let end_us = start_us + duration_us;
    for stream in 0..streams {
        let free = document.overlay_tracks.iter().position(|track| {
            track.is_audio()
                && track
                    .clips
                    .iter()
                    .all(|clip| clip.end_us() <= start_us || end_us <= clip.start_us)
        });
        let track_id = match free {
            Some(index) => document.overlay_tracks[index].id.clone(),
            None => add_track(document, true)?,
        };
        let clip = OverlayClip {
            id: new_id(document, "clip"),
            asset_id: asset_id.to_string(),
            start_us,
            in_us,
            duration_us,
            fit: OverlayFit::default(),
            audio_stream: Some(stream),
            audio_unlinked: false,
            link: Some(link.clone()),
        };
        put_clip(document, &track_id, clip)?;
    }
    Ok(())
}

/// Removes the chosen audio clips of `asset_id` and every clip split off with them.
fn remove_split_audio(
    document: &mut EditDocument,
    asset_id: &str,
    audio_clip_ids: &[String],
) -> Result<(), String> {
    if audio_clip_ids.is_empty() {
        return Err("Select the clip and its sound, then relink".into());
    }
    let mut links = std::collections::HashSet::new();
    let mut ids = std::collections::HashSet::new();
    for id in audio_clip_ids {
        let clip = find_clip(document, id)?;
        if clip.audio_stream.is_none() || clip.asset_id != asset_id {
            return Err("Only that clip's own sound can be relinked to it".into());
        }
        ids.insert(id.clone());
        if let Some(link) = &clip.link {
            links.insert(link.clone());
        }
    }
    for track in &mut document.overlay_tracks {
        track.clips.retain(|clip| {
            !(ids.contains(&clip.id) || clip.link.as_ref().is_some_and(|l| links.contains(l)))
        });
    }
    Ok(())
}

/// The imported media and the offset into it that the main-sequence range `[start, end)`
/// shows, when all of it is one clip of one file. Recording clips stay on the main track.
fn media_under(
    retained: &[RetainedInterval],
    start_us: u64,
    end_us: u64,
) -> Result<(String, u64, bool), String> {
    if start_us >= end_us {
        return Err("Choose a clip to move".into());
    }
    let mut cursor = 0u64;
    for interval in retained {
        let length = interval.end_us - interval.start_us;
        if start_us >= cursor && end_us <= cursor + length {
            return match &interval.media {
                Some(id) => Ok((
                    id.clone(),
                    interval.start_us + (start_us - cursor),
                    interval.audio_unlinked,
                )),
                None => {
                    Err("The recording stays on V1; imported media can move to other tracks".into())
                }
            };
        }
        cursor += length;
    }
    Err("Only one whole imported clip can move to another track".into())
}

pub fn validate(document: &EditDocument) -> Result<(), String> {
    let tracks = &document.overlay_tracks;
    let audio_tracks = tracks.iter().filter(|track| track.is_audio()).count();
    if tracks.len() - audio_tracks > MAX_OVERLAY_TRACKS || audio_tracks > MAX_AUDIO_TRACKS {
        return Err("Too many tracks".into());
    }
    let mut ids = std::collections::HashSet::new();
    for track in tracks {
        if track.id.is_empty() || track.id.len() > 64 || !ids.insert(track.id.as_str()) {
            return Err("Every track needs its own id".into());
        }
        if track.clips.len() > MAX_CLIPS_PER_TRACK {
            return Err(format!("A track holds at most {MAX_CLIPS_PER_TRACK} clips"));
        }
        for clip in &track.clips {
            if clip.id.is_empty() || clip.id.len() > 64 || !ids.insert(clip.id.as_str()) {
                return Err("Every clip needs its own id".into());
            }
            let asset = document
                .media_assets
                .iter()
                .find(|asset| asset.id == clip.asset_id)
                .ok_or("A track uses media that is not imported")?;
            if clip.duration_us < MIN_CLIP_US {
                return Err("A clip must be at least 0.1 s long".into());
            }
            let fits = match asset.kind {
                // A still can be shown for as long as an image clip may last.
                MediaKind::Image => clip.in_us == 0 && clip.duration_us <= asset.duration_us,
                _ => clip
                    .in_us
                    .checked_add(clip.duration_us)
                    .is_some_and(|end| end <= asset.duration_us),
            };
            if !fits {
                return Err("The clip must lie within the media's length".into());
            }
            let sound_ok = match clip.audio_stream {
                Some(stream) => track.is_audio() && stream < asset.audio_paths().count(),
                None => !track.is_audio(),
            };
            if !sound_ok || (track.is_audio() && clip.audio_unlinked) {
                return Err("A track holds a clip of the wrong kind".into());
            }
            if clip
                .link
                .as_ref()
                .is_some_and(|l| l.is_empty() || l.len() > 64)
            {
                return Err("Invalid clip link".into());
            }
            clip.start_us
                .checked_add(clip.duration_us)
                .ok_or("Clip position is out of range")?;
        }
        if track
            .clips
            .windows(2)
            .any(|pair| pair[0].start_us > pair[1].start_us || pair[0].end_us() > pair[1].start_us)
        {
            return Err("Clips on a track cannot overlap".into());
        }
    }
    Ok(())
}

/// Removes every clip of `asset_id` from the tracks.
pub fn remove_asset(document: &mut EditDocument, asset_id: &str) {
    for track in &mut document.overlay_tracks {
        track.clips.retain(|clip| clip.asset_id != asset_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media_bin::MediaAsset;

    const S: u64 = 1_000_000;

    fn document() -> EditDocument {
        let mut document = EditDocument::from_retained(vec![
            RetainedInterval {
                start_us: 0,
                end_us: 10 * S,
                media: None,
                audio_unlinked: false,
            },
            RetainedInterval {
                start_us: S,
                end_us: 5 * S,
                media: Some("v".into()),
                audio_unlinked: false,
            },
            RetainedInterval {
                start_us: 10 * S,
                end_us: 20 * S,
                media: None,
                audio_unlinked: false,
            },
        ])
        .unwrap();
        document.media_assets = vec![
            MediaAsset {
                id: "v".into(),
                name: "broll.mp4".into(),
                kind: MediaKind::Video,
                relative_path: "assets/media/v.mp4".into(),
                source_path: None,
                missing: false,
                picture_role: Default::default(),
                sound_roles: Vec::new(),
                recording_path: None,
                audio_path: None,
                extra_audio_paths: Vec::new(),
                audio_names: Vec::new(),
                duration_us: 8 * S,
                width: 1920,
                height: 1080,
            },
            MediaAsset {
                id: "img".into(),
                name: "logo.png".into(),
                kind: MediaKind::Image,
                relative_path: "assets/media/img.png".into(),
                source_path: None,
                missing: false,
                picture_role: Default::default(),
                sound_roles: Vec::new(),
                recording_path: None,
                audio_path: None,
                extra_audio_paths: Vec::new(),
                audio_names: Vec::new(),
                duration_us: 60 * S,
                width: 400,
                height: 400,
            },
        ];
        document
    }

    #[test]
    fn tracks_stack_and_clips_place_move_and_refuse_overlap() {
        let mut doc = apply(&document(), &TrackEdit::AddTrack { audio: false }).unwrap();
        doc = apply(&doc, &TrackEdit::AddTrack { audio: false }).unwrap();
        let ids: Vec<_> = doc.overlay_tracks.iter().map(|t| t.id.clone()).collect();
        assert_eq!(ids, ["track-1", "track-2"]);
        doc = apply(
            &doc,
            &TrackEdit::PlaceMedia {
                asset_id: "img".into(),
                track_id: "track-1".into(),
                start_us: 2 * S,
            },
        )
        .unwrap();
        validate(&doc).unwrap();
        let clip = doc.overlay_tracks[0].clips[0].clone();
        assert_eq!((clip.start_us, clip.duration_us), (2 * S, 5 * S));
        assert_eq!(
            doc.overlay_tracks[0].clip_at(6 * S).map(|c| c.id.as_str()),
            Some("clip-1")
        );
        assert_eq!(clip.local_us(3 * S), Some(S));

        // Up a track and later in time.
        let moved = OverlayClip {
            start_us: 4 * S,
            ..clip.clone()
        };
        doc = apply(
            &doc,
            &TrackEdit::UpdateClip {
                clip: moved,
                track_id: "track-2".into(),
            },
        )
        .unwrap();
        assert!(doc.overlay_tracks[0].clips.is_empty());
        assert_eq!(doc.overlay_tracks[1].clips[0].start_us, 4 * S);

        // A second clip on top of the first is refused when committed.
        let overlapping = apply(
            &doc,
            &TrackEdit::PlaceMedia {
                asset_id: "img".into(),
                track_id: "track-2".into(),
                start_us: 6 * S,
            },
        )
        .unwrap();
        assert!(validate(&overlapping).is_err());

        // A video clip cannot run past the end of its file.
        let mut long = doc.clone();
        long.overlay_tracks[1].clips[0] = OverlayClip {
            id: "clip-9".into(),
            asset_id: "v".into(),
            start_us: 0,
            in_us: 6 * S,
            duration_us: 3 * S,
            fit: OverlayFit::Cover,
            audio_stream: None,
            audio_unlinked: false,
            link: None,
        };
        assert!(validate(&long).is_err());

        doc = apply(
            &doc,
            &TrackEdit::RemoveTrack {
                track_id: "track-1".into(),
            },
        )
        .unwrap();
        assert_eq!(doc.overlay_tracks.len(), 1);
        remove_asset(&mut doc, "img");
        assert!(doc.overlay_tracks[0].clips.is_empty());
    }

    #[test]
    fn media_lifts_off_the_main_track_and_drops_back() {
        let mut doc = apply(&document(), &TrackEdit::AddTrack { audio: false }).unwrap();
        // The recording stays on V1.
        assert!(apply(
            &doc,
            &TrackEdit::LiftFromMain {
                start_us: 0,
                end_us: 10 * S,
                track_id: "track-1".into(),
                at_us: 0,
            },
        )
        .is_err());
        // The imported clip at 10..14 s goes up; the main sequence closes up to 20 s.
        doc = apply(
            &doc,
            &TrackEdit::LiftFromMain {
                start_us: 10 * S,
                end_us: 14 * S,
                track_id: "track-1".into(),
                at_us: 3 * S,
            },
        )
        .unwrap();
        validate(&doc).unwrap();
        assert_eq!(doc.edited_duration_us().unwrap(), 20 * S);
        assert!(doc.retained_intervals.iter().all(|i| i.media.is_none()));
        let clip = doc.overlay_tracks[0].clips[0].clone();
        assert_eq!(
            (clip.start_us, clip.in_us, clip.duration_us),
            (3 * S, S, 4 * S)
        );

        // And back down, into the middle of the first recording clip.
        doc = apply(
            &doc,
            &TrackEdit::DropToMain {
                clip_id: clip.id,
                target_us: 5 * S,
            },
        )
        .unwrap();
        assert!(doc.overlay_tracks[0].clips.is_empty());
        assert_eq!(doc.edited_duration_us().unwrap(), 24 * S);
        assert_eq!(
            doc.retained_intervals[1],
            RetainedInterval {
                start_us: S,
                end_us: 5 * S,
                media: Some("v".into()),
                audio_unlinked: false,
            }
        );
    }

    fn with_two_audio_streams(mut doc: EditDocument) -> EditDocument {
        let asset = &mut doc.media_assets[0];
        asset.audio_path = Some("assets/media/v.audio.wav".into());
        asset.extra_audio_paths = vec!["assets/media/v.audio2.wav".into()];
        doc
    }

    #[test]
    fn main_clip_sound_unlinks_onto_audio_tracks_and_relinks() {
        let doc = with_two_audio_streams(document());
        // The recording has no separate sound to unlink.
        assert!(apply(
            &doc,
            &TrackEdit::UnlinkMain {
                start_us: 0,
                end_us: 10 * S
            }
        )
        .is_err());

        let unlinked = apply(
            &doc,
            &TrackEdit::UnlinkMain {
                start_us: 10 * S,
                end_us: 14 * S,
            },
        )
        .unwrap();
        validate(&unlinked).unwrap();
        assert!(unlinked.retained_intervals[1].audio_unlinked);
        // One audio track per stream, each with a clip in sync with the picture.
        let audio: Vec<_> = unlinked
            .overlay_tracks
            .iter()
            .filter(|t| t.is_audio())
            .collect();
        assert_eq!(audio.len(), 2);
        for (stream, track) in audio.iter().enumerate() {
            let clip = &track.clips[0];
            assert_eq!(
                (
                    clip.start_us,
                    clip.in_us,
                    clip.duration_us,
                    clip.audio_stream
                ),
                (10 * S, S, 4 * S, Some(stream))
            );
        }
        assert_eq!(audio[0].clips[0].link, audio[1].clips[0].link);
        assert!(apply(
            &unlinked,
            &TrackEdit::UnlinkMain {
                start_us: 10 * S,
                end_us: 14 * S
            }
        )
        .is_err());

        // The sound moves on its own; the picture stays unlinked through a cut before it.
        let first = audio[0].clips[0].clone();
        let mut moved = apply(
            &unlinked,
            &TrackEdit::UpdateClip {
                clip: OverlayClip {
                    start_us: 12 * S,
                    ..first.clone()
                },
                track_id: audio[0].id.clone(),
            },
        )
        .unwrap();
        validate(&moved).unwrap();
        // Audio cannot go onto a video track or into V1.
        moved = apply(&moved, &TrackEdit::AddTrack { audio: false }).unwrap();
        let video_track = moved.overlay_tracks.last().unwrap().id.clone();
        assert!(apply(
            &moved,
            &TrackEdit::UpdateClip {
                clip: first.clone(),
                track_id: video_track
            }
        )
        .is_err());
        assert!(apply(
            &moved,
            &TrackEdit::DropToMain {
                clip_id: first.id.clone(),
                target_us: 0
            }
        )
        .is_err());

        // Relinking needs one of its sound clips; it takes every clip split off with it.
        assert!(apply(
            &moved,
            &TrackEdit::RelinkMain {
                start_us: 10 * S,
                end_us: 14 * S,
                audio_clip_ids: vec![]
            }
        )
        .is_err());
        let relinked = apply(
            &moved,
            &TrackEdit::RelinkMain {
                start_us: 10 * S,
                end_us: 14 * S,
                audio_clip_ids: vec![first.id.clone()],
            },
        )
        .unwrap();
        validate(&relinked).unwrap();
        assert!(!relinked.retained_intervals[1].audio_unlinked);
        assert!(relinked.overlay_tracks.iter().all(|t| t.clips.is_empty()));
    }

    #[test]
    fn track_clip_sound_unlinks_and_lifting_keeps_it_unlinked() {
        let mut doc = with_two_audio_streams(document());
        doc = apply(
            &doc,
            &TrackEdit::UnlinkMain {
                start_us: 10 * S,
                end_us: 14 * S,
            },
        )
        .unwrap();
        doc = apply(&doc, &TrackEdit::AddTrack { audio: false }).unwrap();
        let video_track = doc.overlay_tracks.last().unwrap().id.clone();
        doc = apply(
            &doc,
            &TrackEdit::LiftFromMain {
                start_us: 10 * S,
                end_us: 14 * S,
                track_id: video_track.clone(),
                at_us: 0,
            },
        )
        .unwrap();
        let clip = doc.overlay_tracks.last().unwrap().clips[0].clone();
        assert!(clip.audio_unlinked, "lifting keeps the picture unlinked");

        // A linked clip on a video track unlinks too, and relinks with its own sound only.
        let mut fresh = apply(
            &with_two_audio_streams(document()),
            &TrackEdit::AddTrack { audio: false },
        )
        .unwrap();
        fresh = apply(
            &fresh,
            &TrackEdit::PlaceMedia {
                asset_id: "v".into(),
                track_id: "track-1".into(),
                start_us: 0,
            },
        )
        .unwrap();
        let clip_id = fresh.overlay_tracks[0].clips[0].id.clone();
        fresh = apply(
            &fresh,
            &TrackEdit::UnlinkClip {
                clip_id: clip_id.clone(),
            },
        )
        .unwrap();
        validate(&fresh).unwrap();
        assert!(fresh.overlay_tracks[0].clips[0].audio_unlinked);
        let sound = fresh
            .overlay_tracks
            .iter()
            .find(|t| t.is_audio())
            .unwrap()
            .clips[0]
            .id
            .clone();
        fresh = apply(
            &fresh,
            &TrackEdit::RelinkClip {
                clip_id: clip_id.clone(),
                audio_clip_ids: vec![sound],
            },
        )
        .unwrap();
        assert!(!fresh.overlay_tracks[0].clips[0].audio_unlinked);
        assert_eq!(
            fresh
                .overlay_tracks
                .iter()
                .map(|t| t.clips.len())
                .sum::<usize>(),
            1
        );
    }

    fn place(doc: EditDocument, asset: &str, track: &str, start_us: u64) -> EditDocument {
        apply(
            &doc,
            &TrackEdit::PlaceMedia {
                asset_id: asset.into(),
                track_id: track.into(),
                start_us,
            },
        )
        .unwrap()
    }

    fn clips_of(doc: &EditDocument, track: usize) -> Vec<(u64, u64, u64)> {
        doc.overlay_tracks[track]
            .clips
            .iter()
            .map(|c| (c.start_us, c.in_us, c.duration_us))
            .collect()
    }

    fn range(start_us: u64, end_us: u64) -> EditedRange {
        EditedRange { start_us, end_us }
    }

    #[test]
    fn ripple_delete_on_all_tracks_keeps_them_in_step() {
        // V1 is 0..24 s. A1 holds the video's sound at 2..10 s.
        let mut doc = with_two_audio_streams(document());
        doc = apply(&doc, &TrackEdit::AddTrack { audio: true }).unwrap();
        doc = place(doc, "v", "track-1", 2 * S);

        // V1 only: the sound stays where it was.
        let only_main = apply(
            &doc,
            &TrackEdit::RippleDelete {
                ranges: vec![range(4 * S, 6 * S)],
                all_tracks: false,
            },
        )
        .unwrap();
        assert_eq!(only_main.edited_duration_us().unwrap(), 22 * S);
        assert_eq!(clips_of(&only_main, 0), vec![(2 * S, 0, 8 * S)]);

        // All tracks: the clip across the cut keeps the part before and the part after.
        let all = apply(
            &doc,
            &TrackEdit::RippleDelete {
                ranges: vec![range(4 * S, 6 * S)],
                all_tracks: true,
            },
        )
        .unwrap();
        validate(&all).unwrap();
        assert_eq!(
            clips_of(&all, 0),
            vec![(2 * S, 0, 2 * S), (4 * S, 4 * S, 4 * S)]
        );
        // A cut wholly before a clip moves it back.
        let before = apply(
            &doc,
            &TrackEdit::RippleDelete {
                ranges: vec![range(0, S)],
                all_tracks: true,
            },
        )
        .unwrap();
        assert_eq!(clips_of(&before, 0), vec![(S, 0, 8 * S)]);
    }

    #[test]
    fn delete_selection_split_trim_and_moves() {
        let mut doc = with_two_audio_streams(document());
        doc = apply(&doc, &TrackEdit::AddTrack { audio: false }).unwrap();
        doc = place(doc, "img", "track-1", 0);
        doc = place(doc, "img", "track-1", 6 * S);
        let ids: Vec<String> = doc.overlay_tracks[0]
            .clips
            .iter()
            .map(|c| c.id.clone())
            .collect();

        // One edit removes V1 clips and track clips together.
        let deleted = apply(
            &doc,
            &TrackEdit::DeleteSelection {
                ranges: vec![range(10 * S, 14 * S)],
                clip_ids: vec![ids[0].clone()],
            },
        )
        .unwrap();
        assert_eq!(deleted.edited_duration_us().unwrap(), 20 * S);
        assert_eq!(deleted.overlay_tracks[0].clips.len(), 1);

        // Split only the chosen track clip; V1 is left alone.
        let split = apply(
            &doc,
            &TrackEdit::Split {
                at_us: 8 * S,
                main: false,
                clip_ids: vec![ids[1].clone()],
            },
        )
        .unwrap();
        validate(&split).unwrap();
        assert_eq!(
            clips_of(&split, 0),
            vec![(0, 0, 5 * S), (6 * S, 0, 2 * S), (8 * S, 0, 3 * S)]
        );
        assert_eq!(split.retained_intervals, doc.retained_intervals);
        // Split everything: V1 and the clip under the playhead.
        let both = apply(
            &doc,
            &TrackEdit::Split {
                at_us: 3 * S,
                main: true,
                clip_ids: vec![ids[0].clone()],
            },
        )
        .unwrap();
        assert_eq!(both.split_points_us, vec![3 * S]);
        assert_eq!(both.overlay_tracks[0].clips.len(), 3);

        // Ripple trim the first clip's start to 2 s: it keeps its place, the next closes up.
        let trimmed = apply(
            &doc,
            &TrackEdit::RippleTrimClip {
                clip_id: ids[0].clone(),
                side: ClipSide::Start,
                at_us: 2 * S,
            },
        )
        .unwrap();
        assert_eq!(
            clips_of(&trimmed, 0),
            vec![(0, 0, 3 * S), (4 * S, 0, 5 * S)]
        );

        // Both track clips move together.
        let moved = apply(
            &doc,
            &TrackEdit::MoveClips {
                clip_ids: ids.clone(),
                delta_us: S as i64,
            },
        )
        .unwrap();
        validate(&moved).unwrap();
        assert_eq!(clips_of(&moved, 0), vec![(S, 0, 5 * S), (7 * S, 0, 5 * S)]);
        assert!(apply(
            &doc,
            &TrackEdit::MoveClips {
                clip_ids: ids,
                delta_us: -(S as i64)
            }
        )
        .is_err());
    }

    #[test]
    fn separate_main_clips_move_as_one_block() {
        // V1: recording 0..10, media 10..14, recording 14..24 (edited). Move the media and
        // the last clip to the start: they keep their order.
        let doc = document();
        let moved = apply(
            &doc,
            &TrackEdit::MoveMain {
                ranges: vec![range(10 * S, 14 * S), range(14 * S, 24 * S)],
                target_us: 0,
            },
        )
        .unwrap();
        let order: Vec<_> = moved
            .retained_intervals
            .iter()
            .map(|i| (i.start_us, i.media.is_some()))
            .collect();
        assert_eq!(order, vec![(S, true), (10 * S, false), (0, false)]);
        // Two clips that do not touch: half the first recording clip and the media, to the end.
        let gathered = apply(
            &doc,
            &TrackEdit::MoveMain {
                ranges: vec![range(0, 5 * S), range(10 * S, 14 * S)],
                target_us: 24 * S,
            },
        )
        .unwrap();
        let order: Vec<_> = gathered
            .retained_intervals
            .iter()
            .map(|i| (i.start_us, i.end_us))
            .collect();
        assert_eq!(
            order,
            // The recording pieces left behind touch again, so they are one clip.
            vec![(5 * S, 20 * S), (0, 5 * S), (S, 5 * S)]
        );
    }

    #[test]
    fn independent_unlinks_get_their_own_links_and_relink_alone() {
        // Two separate pictures of the video on V2, each unlinked.
        let mut doc = apply(
            &with_two_audio_streams(document()),
            &TrackEdit::AddTrack { audio: false },
        )
        .unwrap();
        doc = place(doc, "v", "track-1", 0);
        doc = place(doc, "v", "track-1", 12 * S);
        let pictures: Vec<String> = doc.overlay_tracks[0]
            .clips
            .iter()
            .map(|c| c.id.clone())
            .collect();
        for id in &pictures {
            doc = apply(
                &doc,
                &TrackEdit::UnlinkClip {
                    clip_id: id.clone(),
                },
            )
            .unwrap();
        }
        let sounds: Vec<OverlayClip> = doc
            .overlay_tracks
            .iter()
            .filter(|t| t.is_audio())
            .flat_map(|t| t.clips.clone())
            .collect();
        let links: std::collections::BTreeSet<_> = sounds.iter().map(|c| c.link.clone()).collect();
        assert_eq!(links.len(), 2, "each unlink has its own link");
        // Relinking the first leaves the second's sound in place.
        let first_sound = sounds.iter().find(|c| c.start_us == 0).unwrap().id.clone();
        let relinked = apply(
            &doc,
            &TrackEdit::RelinkClip {
                clip_id: pictures[0].clone(),
                audio_clip_ids: vec![first_sound],
            },
        )
        .unwrap();
        validate(&relinked).unwrap();
        let left: Vec<_> = relinked
            .overlay_tracks
            .iter()
            .filter(|t| t.is_audio())
            .flat_map(|t| t.clips.iter().map(|c| c.start_us))
            .collect();
        assert_eq!(
            left,
            vec![12 * S, 12 * S],
            "the other picture keeps both its streams"
        );
    }

    #[test]
    fn a_split_still_moves_from_v1_to_a_track() {
        // A 5 s still on V1 at 24 s (after the 24 s of document()), split after 2 s.
        let mut doc = document();
        doc.retained_intervals.push(RetainedInterval {
            start_us: 0,
            end_us: 5 * S,
            media: Some("img".into()),
            audio_unlinked: false,
        });
        crate::project::revision::split_main(&mut doc, 26 * S).unwrap();
        doc = apply(&doc, &TrackEdit::AddTrack { audio: false }).unwrap();
        let lifted = apply(
            &doc,
            &TrackEdit::LiftFromMain {
                start_us: 26 * S,
                end_us: 29 * S,
                track_id: "track-1".into(),
                at_us: 0,
            },
        )
        .unwrap();
        validate(&lifted).unwrap();
        assert_eq!(clips_of(&lifted, 0), vec![(0, 0, 3 * S)]);
    }

    fn secs(a: u64, b: u64) -> EditedRange {
        range(a * S, b * S)
    }

    fn not_magnetic(document: &EditDocument) -> EditDocument {
        apply(
            document,
            &TrackEdit::SetMainTrack {
                magnetic: false,
                hidden: false,
                muted: false,
            },
        )
        .unwrap()
    }

    #[test]
    fn without_magnetism_cuts_leave_gaps_and_moves_overwrite() {
        let document = not_magnetic(&document());
        // Cutting 2–4 s leaves a gap: the timeline keeps its length.
        let cut = apply(
            &document,
            &TrackEdit::RippleDelete {
                ranges: vec![secs(2, 4)],
                all_tracks: true,
            },
        )
        .unwrap();
        assert_eq!(cut.edited_duration_us().unwrap(), 24 * S);
        assert!(cut.retained_intervals[1].is_gap());
        assert_eq!(
            cut.retained_intervals[1].end_us - cut.retained_intervals[1].start_us,
            2 * S
        );
        // A cut at the end shortens it: nothing follows the gap.
        let tail = apply(
            &cut,
            &TrackEdit::RippleDelete {
                ranges: vec![secs(20, 24)],
                all_tracks: true,
            },
        )
        .unwrap();
        assert_eq!(tail.edited_duration_us().unwrap(), 20 * S);

        // The b-roll (10–14 s) placed at 2 s fills the gap and covers 4–6 s; it leaves a gap.
        let moved = apply(
            &cut,
            &TrackEdit::PlaceMain {
                ranges: vec![secs(10, 14)],
                start_us: 2 * S,
            },
        )
        .unwrap();
        assert_eq!(moved.edited_duration_us().unwrap(), 24 * S);
        let mapper = moved.mapper().unwrap();
        assert_eq!(mapper.media_at(3 * S).map(|(id, _)| id), Some("v"));
        assert_eq!(
            mapper.media_at(11 * S).map(|(id, _)| id),
            Some(crate::project::revision::GAP)
        );
        // Placing past the end extends the timeline with a gap before the clip.
        let far = apply(
            &document,
            &TrackEdit::InsertMedia {
                asset_id: "v".into(),
                target_us: 30 * S,
            },
        )
        .unwrap();
        assert!(far.edited_duration_us().unwrap() > 30 * S);
        assert_eq!(
            far.mapper().unwrap().media_at(27 * S).map(|(id, _)| id),
            Some(crate::project::revision::GAP)
        );
    }

    #[test]
    fn tracks_reorder_with_main_among_the_video_tracks() {
        let mut document = document();
        document = apply(&document, &TrackEdit::AddTrack { audio: false }).unwrap();
        document = apply(&document, &TrackEdit::AddTrack { audio: true }).unwrap();
        document = apply(&document, &TrackEdit::AddTrack { audio: true }).unwrap();
        let video = document.overlay_tracks[0].id.clone();
        assert_eq!(document.main_track.position, 0);
        // V1 up: the video track goes under it.
        let up = apply(
            &document,
            &TrackEdit::MoveTrack {
                track_id: "main".into(),
                up: true,
            },
        )
        .unwrap();
        assert_eq!(up.main_track.position, 1);
        assert!(apply(
            &up,
            &TrackEdit::MoveTrack {
                track_id: "main".into(),
                up: true
            }
        )
        .is_err());
        // Moving the video track back up puts V1 at the bottom again.
        let back = apply(
            &up,
            &TrackEdit::MoveTrack {
                track_id: video,
                up: true,
            },
        )
        .unwrap();
        assert_eq!(back.main_track.position, 0);
        // Audio tracks swap among themselves.
        let audio: Vec<String> = document
            .overlay_tracks
            .iter()
            .filter(|t| t.is_audio())
            .map(|t| t.id.clone())
            .collect();
        let swapped = apply(
            &document,
            &TrackEdit::MoveTrack {
                track_id: audio[1].clone(),
                up: true,
            },
        )
        .unwrap();
        let order: Vec<String> = swapped
            .overlay_tracks
            .iter()
            .filter(|t| t.is_audio())
            .map(|t| t.id.clone())
            .collect();
        assert_eq!(order, vec![audio[1].clone(), audio[0].clone()]);
    }
}
