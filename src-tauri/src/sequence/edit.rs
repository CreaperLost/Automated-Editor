//! Edits to a sequence, each one undoable step.
//!
//! With **magnetic** on (the default), editing ripples like Premiere with every track
//! sync-locked: time cut out closes up on every unlocked track, a deleted or shortened clip
//! closes its gap when nothing else plays there, and a clip that is moved, placed or lengthened
//! where something already plays makes room by pushing everything after it along. Off, clips
//! leave gaps where they were and overwrite what they land on.
//!
//! Linked clips (one moment of an asset: a recording's screen, camera and sound) move, trim
//! and get deleted together; clips on locked tracks never change.
use super::{
    validate_sequence, Asset, Clip, Fit, Role, Sequence, SourceRange, StreamKind, Track, TrackKind,
    IMAGE_CLIP_US, MAX_TRACKS, MIN_PIECE_US,
};
use crate::zoom::EditedRange;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Edge {
    Start,
    End,
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SequenceEdit {
    /// A new empty video track above the others, or audio track below the others.
    AddTrack {
        track_kind: TrackKind,
    },
    /// Removes a track with its clips.
    RemoveTrack {
        track_id: String,
    },
    SetTrack {
        track_id: String,
        #[serde(default)]
        name: String,
        hidden: bool,
        muted: bool,
        #[serde(default)]
        locked: bool,
        #[serde(default)]
        role: Option<Role>,
    },
    /// One place up or down among the tracks of its kind.
    MoveTrack {
        track_id: String,
        up: bool,
    },
    /// Puts an asset on the timeline at `at_us`: linked clips, one per stream (or per chosen
    /// stream), the first on `track_id` and the rest on free tracks of their kind.
    PlaceAsset {
        asset_id: String,
        at_us: u64,
        #[serde(default)]
        track_id: Option<String>,
        /// Only these streams; empty places them all.
        #[serde(default)]
        streams: Vec<String>,
        /// Only this part of the asset; `None` its default length.
        #[serde(default)]
        range: Option<SourceRange>,
    },
    /// Moves clips (with their linked partners) by `delta_us`; with `track_id`, the clip
    /// `anchor_id` lands on that track and the others of its kind shift tracks with it.
    MoveClips {
        clip_ids: Vec<String>,
        delta_us: i64,
        #[serde(default)]
        track_id: Option<String>,
        #[serde(default)]
        anchor_id: Option<String>,
    },
    /// Moves one edge of a clip (and its linked partners) to timeline position `to_us`.
    /// `ripple` overrides magnetic for this edit.
    TrimClip {
        clip_id: String,
        edge: Edge,
        to_us: u64,
        #[serde(default)]
        ripple: Option<bool>,
    },
    /// Splits the listed clips (and partners) at `at_us`; none listed: every unlocked track.
    Split {
        at_us: u64,
        #[serde(default)]
        clip_ids: Vec<String>,
    },
    /// Removes clips with their partners.
    Delete {
        clip_ids: Vec<String>,
        #[serde(default)]
        ripple: Option<bool>,
    },
    /// Cuts time out of every unlocked track: closed up (ripple) or left empty.
    DeleteRange {
        ranges: Vec<EditedRange>,
        #[serde(default)]
        ripple: Option<bool>,
    },
    /// Premiere's Q and E: ripple-deletes from the playhead to the previous or next edit point.
    RippleTrim {
        at_us: u64,
        side: TrimSide,
    },
    Link {
        clip_ids: Vec<String>,
    },
    Unlink {
        clip_ids: Vec<String>,
    },
    SetMagnetic {
        magnetic: bool,
    },
    SetClip {
        clip_id: String,
        fit: Fit,
    },
    /// Puts back the time cut between clips that were one stretch of their source: each listed
    /// clip grows to meet the next clip of its sound or picture, pushing the rest along. None
    /// listed: every cut on the timeline.
    RestoreCuts {
        #[serde(default)]
        clip_ids: Vec<String>,
    },
}

/// What an edit changed, besides the sequence itself.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EditOutcome {
    /// Time removed by a ripple, `(start, end)` before the edit, for the caller's playhead.
    pub removed: Option<(u64, u64)>,
}

/// Applies one edit, returning the new sequence. Nothing changes on error.
pub fn apply(
    sequence: &Sequence,
    assets: &[Asset],
    edit: &SequenceEdit,
) -> Result<(Sequence, EditOutcome), String> {
    apply_reserving(sequence, assets, edit, std::iter::empty())
}

/// [`apply`], numbering new tracks past every id in `taken` as well. A track's id names its
/// mix settings, so a new track must not take the id of one deleted (still in the settings)
/// or of a track in another sequence of the document (a short's).
pub fn apply_reserving<'t>(
    sequence: &Sequence,
    assets: &[Asset],
    edit: &SequenceEdit,
    taken: impl IntoIterator<Item = &'t str>,
) -> Result<(Sequence, EditOutcome), String> {
    let mut editor = Editor::new(sequence.clone(), assets);
    for id in taken {
        editor.next_track = editor.next_track.max(number_after('t', id) + 1);
    }
    let outcome = editor.apply(edit)?;
    let mut next = editor.seq;
    tidy(&mut next);
    validate_sequence(&next, assets)?;
    if next == *sequence {
        return Err(unchanged(edit).into());
    }
    Ok((next, outcome))
}

/// The opening sequence for a project made from a recording: its streams on V1, V2, A1, A2.
pub fn starting_sequence(assets: &[Asset], asset_id: &str) -> Result<Sequence, String> {
    let (sequence, _) = apply(
        &Sequence::default(),
        assets,
        &SequenceEdit::PlaceAsset {
            asset_id: asset_id.into(),
            at_us: 0,
            track_id: None,
            streams: Vec::new(),
            range: None,
        },
    )?;
    Ok(sequence)
}

fn unchanged(edit: &SequenceEdit) -> &'static str {
    match edit {
        SequenceEdit::MoveClips { .. } => "The clips are already there",
        SequenceEdit::DeleteRange { .. } => "There is nothing to cut there",
        SequenceEdit::Split { .. } => "There is already a clip edge here",
        _ => "Nothing changed",
    }
}

/// Sorts clips and drops links that no longer join two clips.
fn tidy(sequence: &mut Sequence) {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for track in &mut sequence.tracks {
        track.clips.sort_by_key(|c| c.start_us);
        for clip in &track.clips {
            if let Some(link) = &clip.link {
                *counts.entry(link.clone()).or_default() += 1;
            }
        }
    }
    for track in &mut sequence.tracks {
        for clip in &mut track.clips {
            if clip.link.as_ref().is_some_and(|l| counts[l] < 2) {
                clip.link = None;
            }
        }
    }
}

/// The links given to right pieces while splitting: by old link and where it was split.
type SplitLinks = HashMap<(String, u64), String>;

struct Editor<'a> {
    seq: Sequence,
    assets: &'a [Asset],
    next_clip: u64,
    next_link: u64,
    next_track: u64,
}

fn number_after(prefix: char, id: &str) -> u64 {
    id.strip_prefix(prefix)
        .and_then(|n| n.parse::<u64>().ok())
        .unwrap_or(0)
}

impl<'a> Editor<'a> {
    fn new(seq: Sequence, assets: &'a [Asset]) -> Self {
        let next_clip = seq
            .clips()
            .map(|(_, c)| number_after('c', &c.id))
            .max()
            .unwrap_or(0)
            + 1;
        let next_link = seq
            .clips()
            .filter_map(|(_, c)| c.link.as_deref())
            .map(|l| number_after('l', l))
            .max()
            .unwrap_or(0)
            + 1;
        let next_track = seq
            .tracks
            .iter()
            .map(|t| number_after('t', &t.id))
            .max()
            .unwrap_or(0)
            + 1;
        Self {
            seq,
            assets,
            next_clip,
            next_link,
            next_track,
        }
    }

    fn clip_id(&mut self) -> String {
        self.next_clip += 1;
        format!("c{}", self.next_clip - 1)
    }

    fn link_id(&mut self) -> String {
        self.next_link += 1;
        format!("l{}", self.next_link - 1)
    }

    fn asset(&self, id: &str) -> Result<&'a Asset, String> {
        self.assets
            .iter()
            .find(|a| a.id == id)
            .ok_or_else(|| "That media is not in the project".to_string())
    }

    fn is_still(&self, asset: &str) -> bool {
        self.assets.iter().any(|a| a.id == asset && a.is_still())
    }

    fn track_index(&self, id: &str) -> Result<usize, String> {
        self.seq
            .tracks
            .iter()
            .position(|t| t.id == id)
            .ok_or_else(|| "That track no longer exists".to_string())
    }

    /// Where clip `id` is: (track index, clip index).
    fn find(&self, id: &str) -> Result<(usize, usize), String> {
        self.seq
            .tracks
            .iter()
            .enumerate()
            .find_map(|(t, track)| track.clips.iter().position(|c| c.id == id).map(|c| (t, c)))
            .ok_or_else(|| "That clip no longer exists".to_string())
    }

    fn unlocked(&self) -> Vec<usize> {
        (0..self.seq.tracks.len())
            .filter(|&i| !self.seq.tracks[i].locked)
            .collect()
    }

    /// `ids` with every clip linked to one of them, leaving out locked tracks' clips. Asking for
    /// a clip on a locked track is an error.
    fn with_partners(&self, ids: &[String]) -> Result<BTreeSet<String>, String> {
        let mut links = BTreeSet::new();
        let mut out = BTreeSet::new();
        for id in ids {
            let (t, c) = self.find(id)?;
            if self.seq.tracks[t].locked {
                return Err("That track is locked".into());
            }
            let clip = &self.seq.tracks[t].clips[c];
            out.insert(clip.id.clone());
            if let Some(link) = &clip.link {
                links.insert(link.clone());
            }
        }
        for (track, clip) in self.seq.clips() {
            if !track.locked && clip.link.as_ref().is_some_and(|l| links.contains(l)) {
                out.insert(clip.id.clone());
            }
        }
        Ok(out)
    }

    /// The part of `clip` from timeline time `at` on.
    fn right_piece(&self, clip: &Clip, at: u64) -> Clip {
        let skip = at - clip.start_us;
        Clip {
            start_us: at,
            in_us: if self.is_still(&clip.asset) {
                0
            } else {
                clip.in_us + skip
            },
            duration_us: clip.end_us() - at,
            ..clip.clone()
        }
    }

    /// Splits the clip on track `t` that spans `at`. The right piece gets a new id, and a link
    /// of its own shared with the right pieces of its partners (through `links`).
    fn split_track(&mut self, t: usize, at: u64, links: &mut SplitLinks) -> bool {
        let Some(index) = self.seq.tracks[t]
            .clips
            .iter()
            .position(|c| c.start_us < at && at < c.end_us())
        else {
            return false;
        };
        let clip = self.seq.tracks[t].clips[index].clone();
        let mut right = self.right_piece(&clip, at);
        right.id = self.clip_id();
        // Partners split at the same point share the right pieces' new link; a split elsewhere
        // (another cut of the same edit) gets one of its own.
        right.link = match &clip.link {
            Some(link) => Some(match links.get(&(link.clone(), at)) {
                Some(new) => new.clone(),
                None => {
                    let new = self.link_id();
                    links.insert((link.clone(), at), new.clone());
                    new
                }
            }),
            None => None,
        };
        self.seq.tracks[t].clips[index].duration_us = at - clip.start_us;
        self.seq.tracks[t].clips.insert(index + 1, right);
        true
    }

    /// Empties `[a, b)` on track `t`: clips across it lose that part, slivers go.
    fn clear(&mut self, t: usize, a: u64, b: u64, links: &mut SplitLinks) {
        self.split_track(t, a, links);
        self.split_track(t, b, links);
        self.seq.tracks[t].clips.retain(|c| {
            let inside = c.start_us >= a && c.end_us() <= b;
            let sliver = c.duration_us < MIN_PIECE_US && (c.end_us() == a || c.start_us == b);
            !inside && !sliver
        });
    }

    /// Moves the clips of track `t` that start at or after `from` by `delta`.
    fn shift(&mut self, t: usize, from: u64, delta: i64) {
        for clip in &mut self.seq.tracks[t].clips {
            if clip.start_us >= from {
                clip.start_us = clip.start_us.saturating_add_signed(delta);
            }
        }
    }

    /// Opens `len` of empty time at `at` on every unlocked track.
    fn insert_time(&mut self, at: u64, len: u64, links: &mut SplitLinks) {
        for t in self.unlocked() {
            self.split_track(t, at, links);
            self.shift(t, at, len as i64);
        }
    }

    /// Cuts `[a, b)` out of every unlocked track and closes it up.
    fn ripple_delete(&mut self, a: u64, b: u64, links: &mut SplitLinks) {
        for t in self.unlocked() {
            self.clear(t, a, b, links);
            self.shift(t, b, -((b - a) as i64));
        }
    }

    /// Closes the parts of `ranges` where nothing plays on any unlocked track. Returns the
    /// closed ranges, in the coordinates from before.
    fn close_empty(&mut self, ranges: &[(u64, u64)]) -> Vec<(u64, u64)> {
        let mut free = merge(ranges.to_vec());
        for t in self.unlocked() {
            for clip in &self.seq.tracks[t].clips {
                free = subtract(&free, clip.start_us, clip.end_us());
            }
        }
        // Nothing after the last clip needs closing.
        let end = self.seq.duration_us();
        free.retain(|&(a, _)| a < end);
        let mut links = HashMap::new();
        for &(a, b) in free.iter().rev() {
            self.ripple_delete(a, b, &mut links);
        }
        free
    }

    fn apply(&mut self, edit: &SequenceEdit) -> Result<EditOutcome, String> {
        let magnetic = self.seq.magnetic;
        match edit {
            SequenceEdit::AddTrack { track_kind } => self.add_track(*track_kind).map(|_| ()),
            SequenceEdit::RemoveTrack { track_id } => {
                let t = self.track_index(track_id)?;
                if self.seq.tracks[t].locked {
                    return Err("Unlock the track to remove it".into());
                }
                self.seq.tracks.remove(t);
                Ok(())
            }
            SequenceEdit::SetTrack {
                track_id,
                name,
                hidden,
                muted,
                locked,
                role,
            } => {
                let t = self.track_index(track_id)?;
                let track = &mut self.seq.tracks[t];
                track.name = name.trim().to_string();
                track.hidden = *hidden;
                track.muted = *muted;
                track.locked = *locked;
                track.role = *role;
                Ok(())
            }
            SequenceEdit::MoveTrack { track_id, up } => self.move_track(track_id, *up),
            SequenceEdit::PlaceAsset {
                asset_id,
                at_us,
                track_id,
                streams,
                range,
            } => self.place(asset_id, *at_us, track_id.as_deref(), streams, *range),
            SequenceEdit::MoveClips {
                clip_ids,
                delta_us,
                track_id,
                anchor_id,
            } => self.move_clips(
                clip_ids,
                *delta_us,
                track_id.as_deref(),
                anchor_id.as_deref(),
            ),
            SequenceEdit::TrimClip {
                clip_id,
                edge,
                to_us,
                ripple,
            } => self.trim(clip_id, *edge, *to_us, ripple.unwrap_or(magnetic)),
            SequenceEdit::Split { at_us, clip_ids } => self.split(*at_us, clip_ids),
            SequenceEdit::Delete { clip_ids, ripple } => {
                let ids = self.with_partners(clip_ids)?;
                let mut gone = Vec::new();
                for track in &mut self.seq.tracks {
                    track.clips.retain(|c| {
                        let delete = ids.contains(&c.id);
                        if delete {
                            gone.push((c.start_us, c.end_us()));
                        }
                        !delete
                    });
                }
                if ripple.unwrap_or(magnetic) {
                    let closed = self.close_empty(&gone);
                    return Ok(EditOutcome {
                        removed: closed.first().copied(),
                    });
                }
                Ok(())
            }
            SequenceEdit::DeleteRange { ranges, ripple } => {
                let mut ordered: Vec<(u64, u64)> =
                    ranges.iter().map(|r| (r.start_us, r.end_us)).collect();
                if ordered.is_empty() {
                    return Err("Choose what to cut".into());
                }
                if ordered.iter().any(|(a, b)| a >= b) {
                    return Err("A cut must have a length".into());
                }
                ordered = merge(ordered);
                let mut links = HashMap::new();
                let ripple = ripple.unwrap_or(magnetic);
                for &(a, b) in ordered.iter().rev() {
                    if ripple {
                        self.ripple_delete(a, b, &mut links);
                    } else {
                        for t in self.unlocked() {
                            self.clear(t, a, b, &mut links);
                        }
                    }
                }
                return Ok(EditOutcome {
                    removed: ripple.then(|| ordered[0]),
                });
            }
            SequenceEdit::RippleTrim { at_us, side } => {
                let edges = self.edges();
                let end = self.seq.duration_us();
                let at = (*at_us).min(end);
                let (a, b) = match side {
                    TrimSide::Previous => edges
                        .iter()
                        .rev()
                        .find(|&&e| e < at)
                        .map(|&e| (e, at))
                        .ok_or("No edit point before the playhead")?,
                    TrimSide::Next => edges
                        .iter()
                        .find(|&&e| e > at)
                        .map(|&e| (at, e))
                        .ok_or("No edit point after the playhead")?,
                };
                if a == 0 && b >= end {
                    return Err("That would remove the whole timeline".into());
                }
                let mut links = HashMap::new();
                self.ripple_delete(a, b, &mut links);
                return Ok(EditOutcome {
                    removed: Some((a, b)),
                });
            }
            SequenceEdit::Link { clip_ids } => {
                let ids: BTreeSet<&String> = clip_ids.iter().collect();
                if ids.len() < 2 {
                    return Err("Choose two or more clips to link".into());
                }
                for id in &ids {
                    let (t, _) = self.find(id)?;
                    if self.seq.tracks[t].locked {
                        return Err("That track is locked".into());
                    }
                }
                let link = self.link_id();
                for track in &mut self.seq.tracks {
                    for clip in &mut track.clips {
                        if ids.contains(&clip.id) {
                            clip.link = Some(link.clone());
                        }
                    }
                }
                Ok(())
            }
            SequenceEdit::Unlink { clip_ids } => {
                let mut links = BTreeSet::new();
                for id in clip_ids {
                    let (t, c) = self.find(id)?;
                    if self.seq.tracks[t].locked {
                        return Err("That track is locked".into());
                    }
                    if let Some(link) = &self.seq.tracks[t].clips[c].link {
                        links.insert(link.clone());
                    }
                }
                if links.is_empty() {
                    return Err("Those clips are not linked".into());
                }
                // Partners on locked tracks keep theirs (it goes once nothing shares it).
                for track in self.seq.tracks.iter_mut().filter(|t| !t.locked) {
                    for clip in &mut track.clips {
                        if clip.link.as_ref().is_some_and(|l| links.contains(l)) {
                            clip.link = None;
                        }
                    }
                }
                Ok(())
            }
            SequenceEdit::SetMagnetic { magnetic } => {
                self.seq.magnetic = *magnetic;
                Ok(())
            }
            SequenceEdit::SetClip { clip_id, fit } => {
                let (t, c) = self.find(clip_id)?;
                if self.seq.tracks[t].locked {
                    return Err("That track is locked".into());
                }
                self.seq.tracks[t].clips[c].fit = *fit;
                Ok(())
            }
            SequenceEdit::RestoreCuts { clip_ids } => self.restore_cuts(clip_ids),
        }
        .map(|()| EditOutcome::default())
    }

    /// The recorded stretches cut from between `clip` and the next clip on its track, when they
    /// are one stream side by side. A recorder pause between them is no cut: nothing was
    /// recorded there.
    fn cut_after(&self, t: usize, c: usize) -> Option<Vec<(u64, u64)>> {
        let clips = &self.seq.tracks[t].clips;
        let (left, right) = (clips.get(c)?, clips.get(c + 1)?);
        if left.asset != right.asset
            || left.stream != right.stream
            || left.end_us() != right.start_us
            || left.out_us() >= right.in_us
            || self.is_still(&left.asset)
        {
            return None;
        }
        let cut: Vec<(u64, u64)> = self
            .asset(&left.asset)
            .ok()?
            .spans()
            .iter()
            .map(|s| (s.start_us.max(left.out_us()), s.end_us.min(right.in_us)))
            .filter(|(a, b)| b > a)
            .collect();
        (!cut.is_empty()).then_some(cut)
    }

    fn restore_cuts(&mut self, clip_ids: &[String]) -> Result<(), String> {
        // Left clips of every cut, latest first: restoring one moves only what comes after.
        let mut lefts: Vec<(u64, String)> = Vec::new();
        for (t, track) in self.seq.tracks.iter().enumerate() {
            if track.locked {
                continue;
            }
            for (c, clip) in track.clips.iter().enumerate() {
                if (clip_ids.is_empty() || clip_ids.contains(&clip.id))
                    && self.cut_after(t, c).is_some()
                {
                    lefts.push((clip.end_us(), clip.id.clone()));
                }
            }
        }
        lefts.sort_by(|a, b| b.cmp(a));
        let mut restored: BTreeSet<String> = BTreeSet::new();
        for (_, id) in lefts {
            let Ok((t, c)) = self.find(&id) else {
                continue;
            };
            // A partner's restore may have put this one back already.
            let Some(cut) = self.cut_after(t, c) else {
                continue;
            };
            let at = self.seq.tracks[t].clips[c].end_us();
            // The clip and the partners ending with it.
            let Ok(partners) = self.with_partners(std::slice::from_ref(&id)) else {
                continue;
            };
            let group: Vec<(usize, Clip)> = partners
                .iter()
                .filter_map(|p| {
                    let (t, c) = self.find(p).ok()?;
                    let clip = &self.seq.tracks[t].clips[c];
                    (clip.end_us() == at).then(|| (t, clip.clone()))
                })
                .collect();
            // Everything after moves along, and each recorded stretch comes back as a piece:
            // across a recorder pause, the pieces stay apart as they were recorded.
            let total: u64 = cut.iter().map(|(a, b)| b - a).sum();
            let mut links = HashMap::new();
            self.insert_time(at, total, &mut links);
            let mut start = at;
            for (a, b) in cut {
                let link = (group.len() > 1).then(|| self.link_id());
                for (t, member) in &group {
                    let piece = Clip {
                        id: self.clip_id(),
                        start_us: start,
                        in_us: a,
                        duration_us: b - a,
                        link: link.clone(),
                        ..member.clone()
                    };
                    restored.insert(piece.id.clone());
                    let clips = &mut self.seq.tracks[*t].clips;
                    let index = clips.partition_point(|c| c.start_us < start);
                    clips.insert(index, piece);
                }
                restored.extend(link);
                start += b - a;
            }
            for (_, member) in group {
                restored.insert(member.id);
                restored.extend(member.link);
            }
        }
        if restored.is_empty() {
            return Err("There is no cut to restore".into());
        }
        // What was one stretch is one clip again.
        for track in &mut self.seq.tracks {
            let mut i = 0;
            while i + 1 < track.clips.len() {
                let (left, right) = (&track.clips[i], &track.clips[i + 1]);
                let mine = restored.contains(&left.id)
                    || left.link.as_ref().is_some_and(|l| restored.contains(l));
                if mine
                    && left.asset == right.asset
                    && left.stream == right.stream
                    && left.end_us() == right.start_us
                    && left.out_us() == right.in_us
                {
                    let right = track.clips.remove(i + 1);
                    track.clips[i].duration_us += right.duration_us;
                } else {
                    i += 1;
                }
            }
        }
        Ok(())
    }

    /// Every unlocked track's clip edges, with 0.
    fn edges(&self) -> Vec<u64> {
        let mut edges: Vec<u64> = std::iter::once(0)
            .chain(
                self.seq
                    .tracks
                    .iter()
                    .filter(|t| !t.locked)
                    .flat_map(|t| t.clips.iter().flat_map(|c| [c.start_us, c.end_us()])),
            )
            .collect();
        edges.sort_unstable();
        edges.dedup();
        edges
    }

    /// Adds a track at the top of the video tracks or the bottom of the audio tracks.
    fn add_track(&mut self, kind: TrackKind) -> Result<usize, String> {
        if self.seq.tracks.iter().filter(|t| t.kind == kind).count() >= MAX_TRACKS {
            return Err(format!("Use at most {MAX_TRACKS} tracks of a kind"));
        }
        let id = format!("t{}", self.next_track);
        self.next_track += 1;
        let at = match kind {
            TrackKind::Video => self.seq.video_tracks().count(),
            TrackKind::Audio => self.seq.tracks.len(),
        };
        self.seq.tracks.insert(at, Track::new(id, kind));
        Ok(at)
    }

    fn move_track(&mut self, track_id: &str, up: bool) -> Result<(), String> {
        let t = self.track_index(track_id)?;
        let kind = self.seq.tracks[t].kind;
        // Video tracks list bottom to top (up is later); audio tracks top to bottom.
        let toward_later = match kind {
            TrackKind::Video => up,
            TrackKind::Audio => !up,
        };
        let other = if toward_later {
            Some(t + 1)
        } else {
            t.checked_sub(1)
        }
        .filter(|&o| o < self.seq.tracks.len() && self.seq.tracks[o].kind == kind)
        .ok_or("It is already at the end")?;
        self.seq.tracks.swap(t, other);
        Ok(())
    }

    /// The tracks of `kind` in stack order starting after track `from` (upwards for video,
    /// downwards for audio), or from the first when `from` is `None`.
    fn tracks_from(&self, kind: TrackKind, from: Option<usize>) -> Vec<usize> {
        let all: Vec<usize> = (0..self.seq.tracks.len())
            .filter(|&i| self.seq.tracks[i].kind == kind)
            .collect();
        match from {
            Some(f) => all.into_iter().filter(|&i| i > f).collect(),
            None => all,
        }
    }

    /// A free, unlocked track of `kind` for `[a, b)`, trying `preferred` first, then the ones
    /// after `after`; a new one when none is free.
    fn free_track(
        &mut self,
        kind: TrackKind,
        after: Option<usize>,
        a: u64,
        b: u64,
        taken: &[usize],
    ) -> Result<usize, String> {
        for t in self.tracks_from(kind, after) {
            let track = &self.seq.tracks[t];
            if !track.locked && !taken.contains(&t) && track.is_free(a, b) {
                return Ok(t);
            }
        }
        self.add_track(kind)
    }

    fn place(
        &mut self,
        asset_id: &str,
        at: u64,
        track_id: Option<&str>,
        streams: &[String],
        range: Option<SourceRange>,
    ) -> Result<(), String> {
        let asset = self.asset(asset_id)?;
        let chosen: Vec<&super::Stream> = asset
            .streams
            .iter()
            .filter(|s| streams.is_empty() || streams.contains(&s.id))
            .collect();
        if chosen.is_empty() {
            return Err("Nothing of that media to place".into());
        }
        // The pieces of the asset to place, back to back: a recording between its pauses.
        let pieces: Vec<(u64, u64)> = if asset.is_still() {
            let len = range.map_or(IMAGE_CLIP_US, |r| r.end_us.saturating_sub(r.start_us));
            vec![(0, len.min(asset.duration_us))]
        } else {
            let want = range.unwrap_or(SourceRange {
                start_us: 0,
                end_us: asset.default_clip_us(),
            });
            asset
                .spans()
                .iter()
                .map(|s| (s.start_us.max(want.start_us), s.end_us.min(want.end_us)))
                .filter(|(a, b)| b > a)
                .collect()
        };
        let total: u64 = pieces.iter().map(|(a, b)| b - a).sum();
        if total == 0 {
            return Err("That part of the media is empty".into());
        }
        let first_kind = match chosen[0].kind {
            StreamKind::Picture => TrackKind::Video,
            StreamKind::Sound => TrackKind::Audio,
        };
        // The track it is dropped on takes the first stream of its kind.
        let drop = match track_id {
            Some(id) => {
                let t = self.track_index(id)?;
                if self.seq.tracks[t].locked {
                    return Err("That track is locked".into());
                }
                Some(t)
            }
            // Not dropped on a track: the first one with room, else a new one.
            None => None,
        };
        let drop = match drop {
            Some(t) => t,
            None => self.free_track(first_kind, None, at, at + total, &[])?,
        };
        let drop_kind = self.seq.tracks[drop].kind;
        let mut links = HashMap::new();
        if !self.seq.tracks[drop].is_free(at, at + total) {
            if self.seq.magnetic {
                self.insert_time(at, total, &mut links);
            } else {
                self.clear(drop, at, at + total, &mut links);
            }
        }
        // Each stream's track: the drop track for the first of its kind, then free ones after
        // the last one used (by id: adding a track moves the others' indices).
        let drop_id = self.seq.tracks[drop].id.clone();
        let mut placed: Vec<(String, String)> = Vec::new(); // (stream, track id)
        for stream in &chosen {
            let kind = match stream.kind {
                StreamKind::Picture => TrackKind::Video,
                StreamKind::Sound => TrackKind::Audio,
            };
            let track = if kind == drop_kind && !placed.iter().any(|(_, id)| *id == drop_id) {
                drop_id.clone()
            } else {
                let taken: Vec<usize> = placed
                    .iter()
                    .map(|(_, id)| self.track_index(id))
                    .collect::<Result<_, _>>()?;
                let after = taken
                    .iter()
                    .copied()
                    .filter(|&i| self.seq.tracks[i].kind == kind)
                    .max();
                let t = self.free_track(kind, after, at, at + total, &taken)?;
                self.seq.tracks[t].id.clone()
            };
            placed.push((stream.id.clone(), track));
        }
        let mut start = at;
        for (in_us, out_us) in pieces {
            let link = (placed.len() > 1).then(|| self.link_id());
            for (stream, track) in &placed {
                let t = self.track_index(track)?;
                let clip = Clip {
                    id: self.clip_id(),
                    asset: asset_id.into(),
                    stream: stream.clone(),
                    start_us: start,
                    in_us,
                    duration_us: out_us - in_us,
                    link: link.clone(),
                    fit: Fit::Contain,
                };
                self.seq.tracks[t].clips.push(clip);
                self.seq.tracks[t].clips.sort_by_key(|c| c.start_us);
            }
            start += out_us - in_us;
        }
        Ok(())
    }

    fn move_clips(
        &mut self,
        clip_ids: &[String],
        delta: i64,
        track_id: Option<&str>,
        anchor_id: Option<&str>,
    ) -> Result<(), String> {
        let ids = self.with_partners(clip_ids)?;
        if ids.is_empty() {
            return Err("Choose clips to move".into());
        }
        // Which track each moved clip goes to: the anchor to `track_id`, the others of its kind
        // by as many tracks, the rest staying.
        let kind_index = |seq: &Sequence, t: usize| {
            seq.tracks[..t]
                .iter()
                .filter(|x| x.kind == seq.tracks[t].kind)
                .count()
        };
        let mut offset = 0i64;
        let mut offset_kind = None;
        if let (Some(track), Some(anchor)) = (track_id, anchor_id) {
            let (at, _) = self.find(anchor)?;
            let to = self.track_index(track)?;
            if self.seq.tracks[to].kind != self.seq.tracks[at].kind {
                return Err(match self.seq.tracks[at].kind {
                    TrackKind::Video => "Pictures go on video tracks".into(),
                    TrackKind::Audio => "Sound goes on audio tracks".into(),
                });
            }
            if self.seq.tracks[to].locked {
                return Err("That track is locked".into());
            }
            offset = kind_index(&self.seq, to) as i64 - kind_index(&self.seq, at) as i64;
            offset_kind = Some(self.seq.tracks[at].kind);
        }
        let mut moved: Vec<(TrackKind, i64, Clip)> = Vec::new(); // (kind, kind index, clip)
        let mut vacated = Vec::new();
        for t in 0..self.seq.tracks.len() {
            let kind = self.seq.tracks[t].kind;
            let index = kind_index(&self.seq, t) as i64;
            let target = if offset_kind == Some(kind) {
                index + offset
            } else {
                index
            };
            let clips = std::mem::take(&mut self.seq.tracks[t].clips);
            for clip in clips {
                if ids.contains(&clip.id) {
                    vacated.push((clip.start_us, clip.end_us()));
                    moved.push((kind, target, clip));
                } else {
                    self.seq.tracks[t].clips.push(clip);
                }
            }
        }
        if moved.iter().any(|(_, index, _)| *index < 0) {
            return Err("There is no track there".into());
        }
        let block_start = moved.iter().map(|(_, _, c)| c.start_us).min().unwrap_or(0);
        let block_end = moved.iter().map(|(_, _, c)| c.end_us()).max().unwrap_or(0);
        let delta = delta.max(-(block_start as i64));
        let target = block_start.saturating_add_signed(delta);
        let mut links = HashMap::new();
        let dest = if self.seq.magnetic {
            let closed = self.close_empty(&vacated);
            after_closing(target, &closed)
        } else {
            target
        };
        // Tracks for the clips, made when missing (moving above the top track).
        let mut placed: Vec<(usize, Clip)> = Vec::new();
        for (kind, index, mut clip) in moved {
            while self.seq.tracks.iter().filter(|t| t.kind == kind).count() as i64 <= index {
                self.add_track(kind)?;
            }
            let t = (0..self.seq.tracks.len())
                .filter(|&i| self.seq.tracks[i].kind == kind)
                .nth(index as usize)
                .unwrap();
            if self.seq.tracks[t].locked {
                return Err("That track is locked".into());
            }
            clip.start_us = dest + (clip.start_us - block_start);
            placed.push((t, clip));
        }
        let occupied = placed
            .iter()
            .any(|(t, c)| !self.seq.tracks[*t].is_free(c.start_us, c.end_us()));
        if occupied {
            if self.seq.magnetic {
                self.insert_time(dest, block_end - block_start, &mut links);
            } else {
                for (t, c) in &placed {
                    self.clear(*t, c.start_us, c.end_us(), &mut links);
                }
            }
        }
        for (t, clip) in placed {
            self.seq.tracks[t].clips.push(clip);
            self.seq.tracks[t].clips.sort_by_key(|c| c.start_us);
        }
        Ok(())
    }

    fn trim(&mut self, clip_id: &str, edge: Edge, to: u64, ripple: bool) -> Result<(), String> {
        let (t, c) = self.find(clip_id)?;
        if self.seq.tracks[t].locked {
            return Err("That track is locked".into());
        }
        let clip = self.seq.tracks[t].clips[c].clone();
        let old = match edge {
            Edge::Start => clip.start_us,
            Edge::End => clip.end_us(),
        };
        // The clip and the partners whose same edge is at the same place.
        let group: Vec<String> = self
            .with_partners(&[clip.id.clone()])?
            .into_iter()
            .filter(|id| {
                self.seq.clip(id).is_some_and(|(_, p)| match edge {
                    Edge::Start => p.start_us == old,
                    Edge::End => p.end_us() == old,
                })
            })
            .collect();
        let members: Vec<(usize, usize)> = group
            .iter()
            .map(|id| self.find(id))
            .collect::<Result<_, _>>()?;
        let mut links = HashMap::new();
        let shrinking = match edge {
            Edge::Start => to > old,
            Edge::End => to < old,
        };
        if to == old {
            return Err("Nothing changed".into());
        }
        if shrinking {
            let d = to.abs_diff(old);
            if members
                .iter()
                .any(|&(t, c)| self.seq.tracks[t].clips[c].duration_us <= d)
            {
                return Err("A clip can't be trimmed away; delete it instead".into());
            }
            for &(t, c) in &members {
                let still = self.is_still(&self.seq.tracks[t].clips[c].asset);
                let member = &mut self.seq.tracks[t].clips[c];
                member.duration_us -= d;
                if edge == Edge::Start {
                    member.start_us += d;
                    if !still {
                        member.in_us += d;
                    }
                }
            }
            if ripple {
                let freed = match edge {
                    Edge::Start => (old, to),
                    Edge::End => (to, old),
                };
                self.close_empty(&[freed]);
            }
            return Ok(());
        }
        // Lengthening: as far as every member's media goes.
        let mut d = to.abs_diff(old);
        for &(t, c) in &members {
            let member = &self.seq.tracks[t].clips[c];
            let asset = self.asset(&member.asset)?;
            let room = if asset.is_still() {
                asset.duration_us.saturating_sub(member.duration_us)
            } else {
                let span = asset
                    .span_at(member.in_us)
                    .ok_or("That clip's media is missing")?;
                match edge {
                    Edge::Start => member.in_us - span.start_us,
                    Edge::End => span.end_us.saturating_sub(member.out_us()),
                }
            };
            d = d.min(room);
        }
        if edge == Edge::Start && !ripple {
            d = d.min(old);
        }
        let range = |d: u64| match edge {
            Edge::Start => (old - d.min(old), old),
            Edge::End => (old, old + d),
        };
        let free = |editor: &Self, d: u64| {
            let (a, b) = range(d);
            members
                .iter()
                .all(|&(t, _)| editor.seq.tracks[t].is_free(a, b))
        };
        if d > 0 && !free(self, d) {
            if ripple {
                // Make room: everything from the edge on moves along.
                self.insert_time(old, d, &mut links);
            } else {
                // Only as far as the nearest clip in the way.
                d = members
                    .iter()
                    .map(|&(t, c)| {
                        let track = &self.seq.tracks[t];
                        match edge {
                            Edge::Start => {
                                let before = track.clips[..c].last().map_or(0, Clip::end_us);
                                old - before.min(old)
                            }
                            Edge::End => {
                                let after = track.clips.get(c + 1).map_or(u64::MAX, |n| n.start_us);
                                after - old
                            }
                        }
                    })
                    .min()
                    .unwrap_or(0)
                    .min(d);
            }
        }
        if d == 0 {
            return Err(match edge {
                Edge::End => "There's no more media after this clip, or no room for it".into(),
                Edge::Start => "There's no more media before this clip, or no room for it".into(),
            });
        }
        // Positions may have moved with the room made: find the members again.
        let members: Vec<(usize, usize)> = group
            .iter()
            .map(|id| self.find(id))
            .collect::<Result<_, _>>()?;
        for (t, c) in members {
            let still = self.is_still(&self.seq.tracks[t].clips[c].asset);
            let member = &mut self.seq.tracks[t].clips[c];
            member.duration_us += d;
            if edge == Edge::Start {
                // After making room the clip moved right by `d`; either way it now starts at
                // `old - d` or (rippled) stays at `old`.
                member.start_us = member.start_us.saturating_sub(d);
                if !still {
                    member.in_us -= d;
                }
            }
        }
        Ok(())
    }

    fn split(&mut self, at: u64, clip_ids: &[String]) -> Result<(), String> {
        let tracks: Vec<usize> = if clip_ids.is_empty() {
            self.unlocked()
        } else {
            let ids = self.with_partners(clip_ids)?;
            let mut tracks: Vec<usize> = ids
                .iter()
                .map(|id| self.find(id).map(|(t, _)| t))
                .collect::<Result<_, _>>()?;
            tracks.sort_unstable();
            tracks.dedup();
            // Only the chosen clips: a track's other clip at `at` is one of them or none.
            tracks.retain(|&t| {
                self.seq.tracks[t]
                    .clip_at(at)
                    .is_some_and(|c| ids.contains(&c.id))
            });
            tracks
        };
        let mut links = HashMap::new();
        let mut any = false;
        for t in tracks {
            any |= self.split_track(t, at, &mut links);
        }
        if !any {
            return Err("There's no clip to split here".into());
        }
        Ok(())
    }
}

/// Where position `t` (before) is after the `closed` ranges (before, disjoint) closed up.
fn after_closing(t: u64, closed: &[(u64, u64)]) -> u64 {
    let mut shift = 0;
    for &(a, b) in closed {
        if t >= b {
            shift += b - a;
        } else if t > a {
            return a - shift;
        }
    }
    t - shift
}

/// Sorts and merges overlapping or touching ranges.
fn merge(mut ranges: Vec<(u64, u64)>) -> Vec<(u64, u64)> {
    ranges.retain(|(a, b)| b > a);
    ranges.sort_unstable();
    let mut out: Vec<(u64, u64)> = Vec::new();
    for (a, b) in ranges {
        match out.last_mut() {
            Some(last) if a <= last.1 => last.1 = last.1.max(b),
            _ => out.push((a, b)),
        }
    }
    out
}

fn subtract(ranges: &[(u64, u64)], a: u64, b: u64) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    for &(x, y) in ranges {
        if b <= x || a >= y {
            out.push((x, y));
            continue;
        }
        if x < a {
            out.push((x, a));
        }
        if b < y {
            out.push((b, y));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence::tests::{image, recording, video};

    const S: u64 = 1_000_000;

    fn run(seq: &Sequence, assets: &[Asset], edit: SequenceEdit) -> Sequence {
        apply(seq, assets, &edit)
            .unwrap_or_else(|e| panic!("{edit:?}: {e}"))
            .0
    }

    /// (track number, asset, stream, start, in, length) for every clip, in track order.
    fn layout(seq: &Sequence) -> Vec<(String, String, u64, u64, u64)> {
        seq.tracks
            .iter()
            .flat_map(|t| {
                let n = seq.number(&t.id).unwrap();
                t.clips.iter().map(move |c| {
                    (
                        n.clone(),
                        c.stream.clone(),
                        c.start_us / 1000,
                        c.in_us / 1000,
                        c.duration_us / 1000,
                    )
                })
            })
            .collect()
    }

    fn rec_project() -> (Vec<Asset>, Sequence) {
        let assets = vec![recording("rec", 10 * S, &[])];
        let seq = starting_sequence(&assets, "rec").unwrap();
        (assets, seq)
    }

    fn clip_on(seq: &Sequence, number: &str, at: u64) -> String {
        let track = seq
            .tracks
            .iter()
            .find(|t| seq.number(&t.id).unwrap() == number)
            .unwrap();
        track.clip_at(at).unwrap().id.clone()
    }

    fn cut(ranges: &[(u64, u64)]) -> SequenceEdit {
        SequenceEdit::DeleteRange {
            ranges: ranges
                .iter()
                .map(|&(start_us, end_us)| EditedRange { start_us, end_us })
                .collect(),
            ripple: None,
        }
    }

    #[test]
    fn a_recording_starts_as_linked_clips_on_v1_v2_a1_a2() {
        let (_, seq) = rec_project();
        assert_eq!(
            layout(&seq),
            vec![
                ("V1".into(), "screen".into(), 0, 0, 10_000),
                ("V2".into(), "webcam".into(), 0, 0, 10_000),
                ("A1".into(), "mic".into(), 0, 0, 10_000),
                ("A2".into(), "system".into(), 0, 0, 10_000),
            ]
        );
        let links: BTreeSet<_> = seq.clips().map(|(_, c)| c.link.clone()).collect();
        assert_eq!(links.len(), 1, "all four linked");
        assert!(links.iter().all(Option::is_some));
    }

    #[test]
    fn a_paused_recording_starts_as_one_linked_set_per_span() {
        let assets = vec![recording("rec", 10 * S, &[(4 * S, 5 * S)])];
        let seq = starting_sequence(&assets, "rec").unwrap();
        let v1: Vec<_> = layout(&seq).into_iter().filter(|c| c.0 == "V1").collect();
        assert_eq!(
            v1,
            vec![
                ("V1".into(), "screen".into(), 0, 0, 4_000),
                ("V1".into(), "screen".into(), 4_000, 5_000, 5_000),
            ]
        );
        let first = seq.tracks[0].clips[0].link.clone();
        let second = seq.tracks[0].clips[1].link.clone();
        assert_ne!(first, second);
    }

    #[test]
    fn cutting_time_ripples_every_track_and_splits_linked_sets() {
        let (assets, seq) = rec_project();
        let seq = run(&seq, &assets, cut(&[(2 * S, 3 * S)]));
        assert_eq!(seq.duration_us(), 9 * S);
        for track in &seq.tracks {
            let parts: Vec<_> = track
                .clips
                .iter()
                .map(|c| (c.start_us, c.in_us, c.duration_us))
                .collect();
            assert_eq!(parts, vec![(0, 0, 2 * S), (2 * S, 3 * S, 7 * S)]);
        }
        // Left pieces stay linked together, and right pieces together, apart from the left.
        let left: BTreeSet<_> = seq.tracks.iter().map(|t| t.clips[0].link.clone()).collect();
        let right: BTreeSet<_> = seq.tracks.iter().map(|t| t.clips[1].link.clone()).collect();
        assert_eq!((left.len(), right.len()), (1, 1));
        assert_ne!(left, right);
    }

    #[test]
    fn without_magnetic_a_cut_leaves_a_gap() {
        let (assets, seq) = rec_project();
        let seq = run(&seq, &assets, SequenceEdit::SetMagnetic { magnetic: false });
        let seq = run(&seq, &assets, cut(&[(2 * S, 3 * S)]));
        assert_eq!(seq.duration_us(), 10 * S);
        assert_eq!(seq.tracks[0].clips[1].start_us, 3 * S);
        // A cut for jump cuts or words always closes up.
        let seq = run(
            &seq,
            &assets,
            SequenceEdit::DeleteRange {
                ranges: vec![EditedRange {
                    start_us: 5 * S,
                    end_us: 6 * S,
                }],
                ripple: Some(true),
            },
        );
        assert_eq!(seq.duration_us(), 9 * S);
    }

    #[test]
    fn deleting_a_linked_set_closes_the_gap_but_b_roll_over_it_keeps_it() {
        let (mut assets, seq) = rec_project();
        assets.push(image("img"));
        let seq = run(&seq, &assets, cut(&[(4 * S, 4 * S + 1)]));
        let seq = run(
            &seq,
            &assets,
            SequenceEdit::Split {
                at_us: 6 * S,
                clip_ids: vec![],
            },
        );
        // Delete the middle set (4 s to 6 s): everything after moves back.
        let middle = clip_on(&seq, "V1", 5 * S);
        let deleted = run(
            &seq,
            &assets,
            SequenceEdit::Delete {
                clip_ids: vec![middle.clone()],
                ripple: None,
            },
        );
        assert_eq!(deleted.duration_us(), seq.duration_us() - 2 * S);
        assert!(deleted.tracks.iter().all(|t| t.clips.len() == 2));
        // With an image over it on V3, the screen goes but the time stays.
        let with_image = run(
            &seq,
            &assets,
            SequenceEdit::PlaceAsset {
                asset_id: "img".into(),
                at_us: 4 * S,
                track_id: None,
                streams: vec![],
                range: Some(SourceRange {
                    start_us: 0,
                    end_us: 2 * S,
                }),
            },
        );
        let image_track = with_image
            .tracks
            .iter()
            .find(|t| t.clips.iter().any(|c| c.asset == "img"))
            .unwrap();
        assert_eq!(
            with_image.number(&image_track.id).unwrap(),
            "V3",
            "a free track above"
        );
        let middle = clip_on(&with_image, "V1", 5 * S);
        let kept = run(
            &with_image,
            &assets,
            SequenceEdit::Delete {
                clip_ids: vec![middle],
                ripple: None,
            },
        );
        assert_eq!(kept.duration_us(), with_image.duration_us());
    }

    #[test]
    fn moving_a_linked_set_reorders_like_the_old_main_track() {
        let (assets, seq) = rec_project();
        // Three sets: 0-3, 3-6, 6-10.
        let seq = run(
            &seq,
            &assets,
            SequenceEdit::Split {
                at_us: 3 * S,
                clip_ids: vec![],
            },
        );
        let seq = run(
            &seq,
            &assets,
            SequenceEdit::Split {
                at_us: 6 * S,
                clip_ids: vec![],
            },
        );
        // Drag the last set to the start.
        let last = clip_on(&seq, "V1", 7 * S);
        let moved = run(
            &seq,
            &assets,
            SequenceEdit::MoveClips {
                clip_ids: vec![last],
                delta_us: -6 * S as i64,
                track_id: None,
                anchor_id: None,
            },
        );
        for track in &moved.tracks {
            let ins: Vec<_> = track
                .clips
                .iter()
                .map(|c| (c.start_us / S, c.in_us / S))
                .collect();
            assert_eq!(ins, vec![(0, 6), (4, 0), (7, 3)], "{}", track.id);
        }
        // Dropping it back on itself changes nothing.
        let first = clip_on(&moved, "V1", 0);
        let again = apply(
            &moved,
            &assets,
            &SequenceEdit::MoveClips {
                clip_ids: vec![first],
                delta_us: S as i64,
                track_id: None,
                anchor_id: None,
            },
        );
        assert!(again.is_err());
    }

    #[test]
    fn without_magnetic_a_move_overwrites_and_leaves_a_gap() {
        let (assets, seq) = rec_project();
        let seq = run(&seq, &assets, SequenceEdit::SetMagnetic { magnetic: false });
        let seq = run(
            &seq,
            &assets,
            SequenceEdit::Split {
                at_us: 8 * S,
                clip_ids: vec![],
            },
        );
        let last = clip_on(&seq, "V1", 9 * S);
        let moved = run(
            &seq,
            &assets,
            SequenceEdit::MoveClips {
                clip_ids: vec![last],
                delta_us: -8 * S as i64,
                track_id: None,
                anchor_id: None,
            },
        );
        let v1: Vec<_> = moved.tracks[0]
            .clips
            .iter()
            .map(|c| (c.start_us / S, c.in_us / S, c.duration_us / S))
            .collect();
        assert_eq!(
            v1,
            vec![(0, 8, 2), (2, 2, 6)],
            "over the start, a gap where it was"
        );
    }

    #[test]
    fn unlinked_camera_moves_alone_and_can_change_track() {
        let (assets, seq) = rec_project();
        let camera = clip_on(&seq, "V2", 0);
        let seq = run(
            &seq,
            &assets,
            SequenceEdit::Unlink {
                clip_ids: vec![camera.clone()],
            },
        );
        assert!(
            seq.clips().all(|(_, c)| c.link.is_none()),
            "the whole set unlinks"
        );
        let seq = run(&seq, &assets, SequenceEdit::SetMagnetic { magnetic: false });
        let v3 = {
            let mut s = seq.clone();
            s = run(
                &s,
                &assets,
                SequenceEdit::AddTrack {
                    track_kind: TrackKind::Video,
                },
            );
            s
        };
        let target = v3
            .tracks
            .iter()
            .find(|t| v3.number(&t.id).as_deref() == Some("V3"))
            .unwrap()
            .id
            .clone();
        let moved = run(
            &v3,
            &assets,
            SequenceEdit::MoveClips {
                clip_ids: vec![camera.clone()],
                delta_us: S as i64,
                track_id: Some(target.clone()),
                anchor_id: Some(camera.clone()),
            },
        );
        let (track, clip) = moved.clip(&camera).unwrap();
        assert_eq!((track.id.as_str(), clip.start_us), (target.as_str(), S));
        assert_eq!(moved.tracks[0].clips[0].start_us, 0, "the screen stays");
        // Pictures never go on audio tracks.
        let a1 = moved
            .tracks
            .iter()
            .find(|t| t.kind == TrackKind::Audio)
            .unwrap()
            .id
            .clone();
        assert!(apply(
            &moved,
            &assets,
            &SequenceEdit::MoveClips {
                clip_ids: vec![camera.clone()],
                delta_us: 0,
                track_id: Some(a1),
                anchor_id: Some(camera)
            },
        )
        .is_err());
    }

    #[test]
    fn trimming_moves_partners_and_ripples() {
        let (assets, seq) = rec_project();
        let seq = run(
            &seq,
            &assets,
            SequenceEdit::Split {
                at_us: 5 * S,
                clip_ids: vec![],
            },
        );
        let first = clip_on(&seq, "V1", 0);
        // Shorten the first set to 4 s: everything after closes up.
        let short = run(
            &seq,
            &assets,
            SequenceEdit::TrimClip {
                clip_id: first.clone(),
                edge: Edge::End,
                to_us: 4 * S,
                ripple: None,
            },
        );
        assert_eq!(short.duration_us(), 9 * S);
        for track in &short.tracks {
            assert_eq!(track.clips[0].duration_us, 4 * S);
            assert_eq!(track.clips[1].start_us, 4 * S);
            assert_eq!(track.clips[1].in_us, 5 * S);
        }
        // Restoring the cut second pushes the rest along again.
        let restored = run(
            &short,
            &assets,
            SequenceEdit::TrimClip {
                clip_id: first.clone(),
                edge: Edge::End,
                to_us: 5 * S,
                ripple: Some(true),
            },
        );
        assert_eq!(layout(&restored), layout(&seq));
        // Lengthening past the media stops at its end.
        let second = clip_on(&seq, "V1", 6 * S);
        assert!(apply(
            &seq,
            &assets,
            &SequenceEdit::TrimClip {
                clip_id: second,
                edge: Edge::End,
                to_us: 12 * S,
                ripple: None
            },
        )
        .is_err());
        // Trimming the start of the first set closes the gap at the start.
        let later = run(
            &seq,
            &assets,
            SequenceEdit::TrimClip {
                clip_id: first,
                edge: Edge::Start,
                to_us: S,
                ripple: None,
            },
        );
        assert_eq!(later.tracks[0].clips[0].start_us, 0);
        assert_eq!(later.tracks[0].clips[0].in_us, S);
        assert_eq!(later.duration_us(), 9 * S);
    }

    #[test]
    fn placing_media_inserts_when_magnetic_and_finds_free_tracks() {
        let (mut assets, seq) = rec_project();
        assets.push(video("m1", 3 * S));
        // Dropped on V1 in the middle of the recording: everything after 4 s moves 3 s along.
        let v1 = seq.tracks[0].id.clone();
        let placed = run(
            &seq,
            &assets,
            SequenceEdit::PlaceAsset {
                asset_id: "m1".into(),
                at_us: 4 * S,
                track_id: Some(v1),
                streams: vec![],
                range: None,
            },
        );
        assert_eq!(placed.duration_us(), 13 * S);
        let (track, picture) = placed
            .clips()
            .find(|(_, c)| c.asset == "m1" && c.stream == "picture")
            .unwrap();
        assert_eq!(
            (placed.number(&track.id).unwrap(), picture.start_us),
            ("V1".into(), 4 * S)
        );
        let (track, sound) = placed
            .clips()
            .find(|(_, c)| c.asset == "m1" && c.stream == "sound0")
            .unwrap();
        assert_eq!(
            (placed.number(&track.id).unwrap(), sound.start_us),
            ("A1".into(), 4 * S)
        );
        assert_eq!(picture.link, sound.link);
        assert!(picture.link.is_some());
        // Dropped on a new V3 over the recording: nothing moves, its sound gets its own track.
        let with_v3 = run(
            &seq,
            &assets,
            SequenceEdit::AddTrack {
                track_kind: TrackKind::Video,
            },
        );
        let v3 = with_v3.tracks[2].id.clone();
        let over = run(
            &with_v3,
            &assets,
            SequenceEdit::PlaceAsset {
                asset_id: "m1".into(),
                at_us: 2 * S,
                track_id: Some(v3),
                streams: vec![],
                range: None,
            },
        );
        assert_eq!(over.duration_us(), 10 * S);
        let (track, _) = over
            .clips()
            .find(|(_, c)| c.asset == "m1" && c.stream == "sound0")
            .unwrap();
        assert_eq!(over.number(&track.id).unwrap(), "A3");
    }

    #[test]
    fn ripple_trim_q_and_e_use_every_tracks_edges() {
        let (assets, seq) = rec_project();
        let seq = run(
            &seq,
            &assets,
            SequenceEdit::Split {
                at_us: 4 * S,
                clip_ids: vec![],
            },
        );
        let (q, outcome) = apply(
            &seq,
            &assets,
            &SequenceEdit::RippleTrim {
                at_us: 6 * S,
                side: TrimSide::Previous,
            },
        )
        .unwrap();
        assert_eq!(outcome.removed, Some((4 * S, 6 * S)));
        assert_eq!(q.duration_us(), 8 * S);
        let (e, outcome) = apply(
            &seq,
            &assets,
            &SequenceEdit::RippleTrim {
                at_us: S,
                side: TrimSide::Next,
            },
        )
        .unwrap();
        assert_eq!(outcome.removed, Some((S, 4 * S)));
        assert_eq!(e.tracks[2].clips[0].in_us, 0);
        assert_eq!(e.tracks[2].clips[1].in_us, 4 * S);
    }

    #[test]
    fn locked_tracks_never_change() {
        let (assets, seq) = rec_project();
        let a2 = seq.tracks[3].id.clone();
        let seq = run(
            &seq,
            &assets,
            SequenceEdit::SetTrack {
                track_id: a2,
                name: "Music".into(),
                hidden: false,
                muted: false,
                locked: true,
                role: None,
            },
        );
        let cut = run(&seq, &assets, cut(&[(2 * S, 3 * S)]));
        assert_eq!(cut.tracks[3].clips, seq.tracks[3].clips);
        assert_eq!(cut.tracks[0].clips.len(), 2);
        assert_eq!(cut.number(&cut.tracks[3].id).unwrap(), "A2");
        assert_eq!(cut.tracks[3].name, "Music");
    }

    #[test]
    fn several_cuts_link_each_piece_only_with_its_partners() {
        let (assets, seq) = rec_project();
        let seq = run(&seq, &assets, cut(&[(2 * S, 3 * S), (6 * S, 7 * S)]));
        // Every track: 0-2, 3-6 and 7-10 of the source, three linked sets across the tracks.
        for track in &seq.tracks {
            let ins: Vec<_> = track.clips.iter().map(|c| c.in_us).collect();
            assert_eq!(ins, vec![0, 3 * S, 7 * S]);
        }
        for piece in 0..3 {
            let links: BTreeSet<_> = seq
                .tracks
                .iter()
                .map(|t| t.clips[piece].link.clone())
                .collect();
            assert_eq!(links.len(), 1, "piece {piece} is one set");
        }
        let sets: BTreeSet<_> = seq.tracks[0].clips.iter().map(|c| c.link.clone()).collect();
        assert_eq!(sets.len(), 3, "the pieces are separate sets");
    }

    #[test]
    fn locked_clips_keep_their_fit_links_and_track() {
        let (assets, seq) = rec_project();
        let mut locked = seq.clone();
        locked.tracks[1].locked = true;
        let camera = locked.tracks[1].clips[0].id.clone();
        let screen = locked.tracks[0].clips[0].id.clone();
        let refused = |edit: SequenceEdit| apply(&locked, &assets, &edit).unwrap_err();
        assert_eq!(
            refused(SequenceEdit::SetClip {
                clip_id: camera.clone(),
                fit: Fit::Cover
            }),
            "That track is locked"
        );
        assert_eq!(
            refused(SequenceEdit::Unlink {
                clip_ids: vec![camera.clone()]
            }),
            "That track is locked"
        );
        assert_eq!(
            refused(SequenceEdit::Link {
                clip_ids: vec![camera, screen.clone()]
            }),
            "That track is locked"
        );
        assert_eq!(
            refused(SequenceEdit::RemoveTrack {
                track_id: locked.tracks[1].id.clone()
            }),
            "Unlock the track to remove it"
        );
        // Unlinking the others leaves the locked camera's link alone until nothing shares it.
        let unlinked = run(
            &locked,
            &assets,
            SequenceEdit::Unlink {
                clip_ids: vec![screen],
            },
        );
        assert_eq!(unlinked.tracks[0].clips[0].link, None);
        assert_eq!(unlinked.tracks[1].clips[0].link, None, "alone, so it goes");
    }

    #[test]
    fn new_tracks_take_ids_no_other_track_had() {
        let (assets, seq) = rec_project();
        let last = seq.tracks.last().unwrap().id.clone();
        let removed = run(
            &seq,
            &assets,
            SequenceEdit::RemoveTrack {
                track_id: last.clone(),
            },
        );
        let add = SequenceEdit::AddTrack {
            track_kind: TrackKind::Audio,
        };
        // Without anything to avoid, the number comes back...
        let (again, _) = apply(&removed, &assets, &add).unwrap();
        assert_eq!(again.tracks.last().unwrap().id, last);
        // ...so callers pass the ids the document still knows (mix settings, shorts).
        let (fresh, _) = apply_reserving(&removed, &assets, &add, [last.as_str(), "t9"]).unwrap();
        assert_eq!(fresh.tracks.last().unwrap().id, "t10");
    }

    #[test]
    fn tracks_move_within_their_kind() {
        let (assets, seq) = rec_project();
        let v1 = seq.tracks[0].id.clone();
        let moved = run(
            &seq,
            &assets,
            SequenceEdit::MoveTrack {
                track_id: v1.clone(),
                up: true,
            },
        );
        assert_eq!(moved.tracks[1].id, v1);
        assert!(
            apply(
                &moved,
                &assets,
                &SequenceEdit::MoveTrack {
                    track_id: v1,
                    up: true
                }
            )
            .is_err(),
            "V2 is the top video track"
        );
        let a1 = seq.tracks[2].id.clone();
        assert!(
            apply(
                &seq,
                &assets,
                &SequenceEdit::MoveTrack {
                    track_id: a1,
                    up: true
                }
            )
            .is_err(),
            "A1 is the top audio track"
        );
    }

    #[test]
    fn images_stretch_and_keep_their_start() {
        let assets = vec![image("img")];
        let seq = run(
            &Sequence::default(),
            &assets,
            SequenceEdit::PlaceAsset {
                asset_id: "img".into(),
                at_us: 0,
                track_id: None,
                streams: vec![],
                range: None,
            },
        );
        assert_eq!(seq.duration_us(), IMAGE_CLIP_US);
        let id = seq.tracks[0].clips[0].id.clone();
        let long = run(
            &seq,
            &assets,
            SequenceEdit::TrimClip {
                clip_id: id,
                edge: Edge::End,
                to_us: 60 * S,
                ripple: None,
            },
        );
        assert_eq!(long.tracks[0].clips[0].duration_us, 60 * S);
        let split = run(
            &long,
            &assets,
            SequenceEdit::Split {
                at_us: 30 * S,
                clip_ids: vec![],
            },
        );
        assert_eq!(split.tracks[0].clips[1].in_us, 0);
    }

    #[test]
    fn restoring_cuts_puts_the_time_back_in_one_step() {
        let (assets, seq) = rec_project();
        let cut = run(
            &seq,
            &assets,
            super::tests::cut(&[(2 * S, 3 * S), (5 * S, 6 * S)]),
        );
        assert_eq!(cut.duration_us(), 8 * S);
        let all = run(
            &cut,
            &assets,
            SequenceEdit::RestoreCuts { clip_ids: vec![] },
        );
        assert_eq!(layout(&all), layout(&seq));
        // Just one: the first cut comes back, the second stays.
        let first = cut.tracks[0].clips[0].id.clone();
        let one = run(
            &cut,
            &assets,
            SequenceEdit::RestoreCuts {
                clip_ids: vec![first],
            },
        );
        assert_eq!(one.duration_us(), 9 * S);
        assert_eq!(one.tracks[2].clips[0].duration_us, 5 * S);
        assert!(apply(
            &seq,
            &assets,
            &SequenceEdit::RestoreCuts { clip_ids: vec![] }
        )
        .is_err());
    }

    /// Restoring puts back exactly the layout from before the cuts, recorder pauses and all:
    /// a pause is no cut, and a cut across one comes back as the pieces either side of it.
    #[test]
    fn restoring_cuts_across_recorder_pauses_leaves_no_gap() {
        let assets = vec![recording("rec", 10 * S, &[(4 * S, 5 * S), (6 * S, 7 * S)])];
        let seq = starting_sequence(&assets, "rec").unwrap();
        assert_eq!(seq.duration_us(), 8 * S);
        assert!(
            apply(
                &seq,
                &assets,
                &SequenceEdit::RestoreCuts { clip_ids: vec![] }
            )
            .is_err(),
            "pauses alone are no cut"
        );
        // Timeline 3s..6s is source 3..4, 5..6 and 7..8: across both pauses.
        for ranges in [
            vec![(3 * S, 6 * S)],
            vec![(3 * S, 4 * S)],
            vec![(S, 2 * S), (3 * S + S / 2, 7 * S)],
        ] {
            let cut = run(&seq, &assets, super::tests::cut(&ranges));
            assert!(cut.duration_us() < seq.duration_us());
            let all = run(
                &cut,
                &assets,
                SequenceEdit::RestoreCuts { clip_ids: vec![] },
            );
            assert_eq!(layout(&all), layout(&seq), "cut {ranges:?}");
            // One by one, latest first, comes to the same.
            let mut one = cut.clone();
            loop {
                let last = one.tracks[0]
                    .clips
                    .iter()
                    .rev()
                    .find(|c| {
                        let edit = SequenceEdit::RestoreCuts {
                            clip_ids: vec![c.id.clone()],
                        };
                        apply(&one, &assets, &edit).is_ok()
                    })
                    .map(|c| c.id.clone());
                let Some(id) = last else { break };
                one = run(
                    &one,
                    &assets,
                    SequenceEdit::RestoreCuts { clip_ids: vec![id] },
                );
            }
            assert_eq!(layout(&one), layout(&seq), "cut {ranges:?}, one by one");
        }
    }

    /// The edits exactly as the timeline sends them.
    #[test]
    fn edits_read_the_ui_messages() {
        let messages = [
            r#"{"kind":"split","atUs":25000000,"clipIds":[]}"#,
            r#"{"kind":"rippleTrim","atUs":25000000,"side":"previous"}"#,
            r#"{"kind":"unlink","clipIds":["c2","c5"]}"#,
            r#"{"kind":"delete","clipIds":["c2"],"ripple":null}"#,
            r#"{"kind":"delete","clipIds":["c2"],"ripple":true}"#,
            r#"{"kind":"moveClips","clipIds":["c7"],"deltaUs":9491876,"trackId":null,"anchorId":"c7"}"#,
            r#"{"kind":"moveClips","clipIds":["c7"],"deltaUs":-5,"trackId":"t3","anchorId":"c7"}"#,
            r#"{"kind":"trimClip","clipId":"c1","edge":"end","toUs":17152437}"#,
            r#"{"kind":"trimClip","clipId":"c1","edge":"start","toUs":5,"ripple":true}"#,
            r#"{"kind":"placeAsset","assetId":"m-1","atUs":0,"trackId":null}"#,
            r#"{"kind":"placeAsset","assetId":"m-1","atUs":0,"trackId":"t1","range":{"startUs":0,"endUs":5}}"#,
            r#"{"kind":"addTrack","trackKind":"audio"}"#,
            r#"{"kind":"setTrack","trackId":"t1","name":"","hidden":false,"muted":true,"locked":false,"role":"mic"}"#,
            r#"{"kind":"setTrack","trackId":"t1","name":"B","hidden":true,"muted":false,"locked":true,"role":null}"#,
            r#"{"kind":"moveTrack","trackId":"t1","up":true}"#,
            r#"{"kind":"deleteRange","ranges":[{"startUs":1,"endUs":2}],"ripple":true}"#,
            r#"{"kind":"setMagnetic","magnetic":false}"#,
            r#"{"kind":"setClip","clipId":"c1","fit":"cover"}"#,
            r#"{"kind":"restoreCuts","clipIds":[]}"#,
            r#"{"kind":"link","clipIds":["a","b"]}"#,
            r#"{"kind":"removeTrack","trackId":"t1"}"#,
        ];
        for message in messages {
            serde_json::from_str::<SequenceEdit>(message)
                .unwrap_or_else(|e| panic!("{message}: {e}"));
        }
    }

    #[test]
    fn after_closing_maps_positions() {
        assert_eq!(after_closing(10, &[(2, 4), (6, 7)]), 7);
        assert_eq!(after_closing(3, &[(2, 4)]), 2);
        assert_eq!(after_closing(1, &[(2, 4)]), 1);
    }
}
