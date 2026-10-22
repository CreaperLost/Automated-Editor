//! What an agent proposes, waiting for the user: batches of cuts or of timeline edits, kept
//! beside the edit in `agent/proposals.json`. Proposing changes nothing in the edit and takes
//! no undo step (so the agent's own next proposal is not stale); accepting a batch commits it
//! as one undo step.
//!
//! Cuts are kept by word id, which stays valid through every later cut and undo, so they are
//! worked out again from the edit as it is when accepted. Timeline edits and cuts given as
//! timeline ranges hold only for the revision they were proposed against.
use crate::mcp::tools::clock;
use crate::project::reader::{open_regular, safe_path, OpenedProject, ProjectReader};
use crate::project::revision::EditDocument;
use crate::sequence::edit::SequenceEdit;
use crate::transcript::edit::{word_cuts, ProposalRef, SuggestionSource};
use crate::transcript::{store, Transcript, TranscriptCutSuggestion, TranscriptSuggestionKind};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

pub const PROPOSALS_DIR: &str = "agent";
const PROPOSALS_FILE: &str = "agent/proposals.json";
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
/// Decided batches kept for the review panel's history.
const KEEP_DECIDED: usize = 50;
const MAX_PENDING: usize = 50;
pub const MAX_ITEMS: usize = 500;
pub const MAX_EDITS: usize = 100;
const MAX_REASON_CHARS: usize = 500;
const MAX_SPAN_WORDS: usize = 400;
const MAX_SUMMARY_LINES: usize = 40;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Proposals {
    pub next_id: u64,
    /// Oldest first.
    pub batches: Vec<Batch>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Batch {
    /// `b-<n>`.
    pub id: String,
    /// The agent's name for itself, from its MCP handshake.
    pub agent: String,
    pub reason: String,
    pub created_at: String,
    /// The edit revision it was checked against.
    pub base_revision: u64,
    pub change: Change,
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided_at: Option<String>,
    /// Accepted by autopilot rather than by the user.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub auto: bool,
    /// The revision its last accept made: it can be undone while that is still current.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_revision: Option<u64>,
    /// What it does, worked out when it was proposed.
    pub summary: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Change {
    /// Time cut out of every track, as word cuts are.
    Cuts {
        stream: String,
        /// The word stamp of the transcript its word items name (`Transcript::dependency`).
        /// Retranscribing reuses word ids, so words are only looked up while it still holds.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        words: Option<String>,
        items: Vec<CutItem>,
    },
    /// Timeline edits applied in order, all or nothing.
    Edits { edits: Vec<SequenceEdit> },
    /// Footage, graphics and files put under take short `short`, all or nothing. It holds
    /// while the short and the video's timeline are as they were (`stamp`).
    Under {
        short: String,
        stamp: String,
        items: Vec<crate::under_take::Under>,
    },
    /// One of the app's own actions the agent ran (jump cuts, fillers, zooms, ...), applied
    /// at once like a click; kept so the user sees it and can undo it.
    Action { action: String },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CutItem {
    /// `c-<n>`, within its batch.
    pub id: String,
    pub kind: TranscriptSuggestionKind,
    /// The words from `first_word_id` to `last_word_id` in the recording...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_word_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_word_id: Option<String>,
    /// ...or this range of the edited timeline at the batch's base revision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_us: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_us: Option<u64>,
    pub text: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    pub state: ItemState,
    /// The revision that cut it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<u64>,
}

impl CutItem {
    fn words(&self) -> Option<(&str, &str)> {
        Some((
            self.first_word_id.as_deref()?,
            self.last_word_id.as_deref()?,
        ))
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ItemState {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    /// Waiting for the user (for cuts: some items still are).
    Pending,
    Accepted,
    Rejected,
    /// The agent took it back, or undid it.
    Withdrawn,
    /// An action that was undone.
    Undone,
}

fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

// ---------------------------------------------------------------------------------------
// The file.

pub fn load(root: &Path) -> Result<Proposals, String> {
    let path = safe_path(root, PROPOSALS_FILE)?;
    if !path.is_file() {
        return Ok(Proposals::default());
    }
    let mut bytes = Vec::new();
    open_regular(&path)?
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err("The agent proposals file is too large".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| format!("Invalid agent proposals file: {e}"))
}

pub fn save(root: &Path, proposals: &Proposals) -> Result<(), String> {
    let dir = safe_path(root, PROPOSALS_DIR)?;
    match fs::symlink_metadata(&dir) {
        Ok(meta) if !meta.is_dir() => return Err("The agent folder is not a folder".into()),
        Ok(_) => (),
        Err(_) => fs::create_dir(&dir).map_err(|e| e.to_string())?,
    }
    let path = safe_path(root, PROPOSALS_FILE)?;
    let bytes = serde_json::to_vec_pretty(proposals).map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err("The agent proposals file is too large".into());
    }
    let mut temp = tempfile::NamedTempFile::new_in(&dir).map_err(|e| e.to_string())?;
    temp.write_all(&bytes).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(&path).map_err(|e| e.to_string())?;
    Ok(())
}

impl Proposals {
    pub fn get(&self, id: &str) -> Result<&Batch, String> {
        self.batches
            .iter()
            .find(|b| b.id == id)
            .ok_or_else(|| format!("No proposal batch {id}"))
    }

    fn get_mut(&mut self, id: &str) -> Result<&mut Batch, String> {
        self.batches
            .iter_mut()
            .find(|b| b.id == id)
            .ok_or_else(|| format!("No proposal batch {id}"))
    }

    fn add(&mut self, mut batch: Batch) -> Result<String, String> {
        let pending = self
            .batches
            .iter()
            .filter(|b| b.status == Status::Pending)
            .count();
        if pending >= MAX_PENDING {
            return Err(format!(
                "{pending} proposals are already waiting. Ask the user to review them first."
            ));
        }
        self.next_id += 1;
        batch.id = format!("b-{}", self.next_id);
        let id = batch.id.clone();
        self.batches.push(batch);
        self.prune();
        Ok(id)
    }

    /// Lists an action the agent ran, which took the edit from revision `before` to `after`.
    pub fn record_action(
        &mut self,
        agent: &str,
        action: &str,
        summary: Vec<String>,
        before: u64,
        after: u64,
    ) -> String {
        self.next_id += 1;
        let id = format!("b-{}", self.next_id);
        self.batches.push(Batch {
            id: id.clone(),
            agent: agent.to_string(),
            reason: summary.first().cloned().unwrap_or_default(),
            created_at: now(),
            base_revision: before,
            change: Change::Action {
                action: action.to_string(),
            },
            status: Status::Accepted,
            decided_at: Some(now()),
            auto: false,
            applied_revision: Some(after),
            summary,
        });
        self.prune();
        id
    }

    /// Keeps every pending batch and the newest decided ones.
    fn prune(&mut self) {
        let decided = self
            .batches
            .iter()
            .filter(|b| b.status != Status::Pending)
            .count();
        let mut drop = decided.saturating_sub(KEEP_DECIDED);
        self.batches.retain(|b| {
            if drop > 0 && b.status != Status::Pending {
                drop -= 1;
                false
            } else {
                true
            }
        });
    }
}

// ---------------------------------------------------------------------------------------
// Proposing.

/// One cut an agent asks for: a run of words, or a range of the edited timeline.
#[derive(Clone, Debug, Default, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CutInput {
    /// First word to cut, such as `w-120` (from transcript_get).
    #[serde(default)]
    pub from_word: Option<String>,
    /// Last word to cut (inclusive). Default: `fromWord`.
    #[serde(default)]
    pub to_word: Option<String>,
    /// Or a range of the edited timeline, microseconds (only valid at this revision).
    #[serde(default)]
    pub start_us: Option<u64>,
    #[serde(default)]
    pub end_us: Option<u64>,
    /// `filler`, `retake` or `cut` (default).
    #[serde(default)]
    pub kind: Option<TranscriptSuggestionKind>,
    /// Why, in a few words, for the user.
    #[serde(default)]
    pub reason: Option<String>,
}

fn check_reason(reason: &str) -> Result<String, String> {
    let reason = reason.trim();
    if reason.chars().count() > MAX_REASON_CHARS {
        return Err(format!("Keep reasons under {MAX_REASON_CHARS} characters"));
    }
    Ok(reason.to_string())
}

fn check_revision(document: &EditDocument, expected: u64) -> Result<(), String> {
    if document.revision != expected {
        return Err(format!(
            "Stale edit revision: the project is at revision {}. Read it again (project_info, \
             transcript_get) and propose against that.",
            document.revision
        ));
    }
    Ok(())
}

/// The ids of the words from `first` to `last` in the recording, cut or not.
fn span_ids(transcript: &Transcript, first: &str, last: &str) -> Result<Vec<String>, String> {
    let at = |id: &str| {
        transcript
            .words
            .iter()
            .position(|w| w.id == id)
            .ok_or_else(|| format!("No word {id} in {}", transcript.track_id))
    };
    let (a, b) = (at(first)?, at(last)?);
    if a > b {
        return Err(format!("{first} comes after {last}"));
    }
    if b - a + 1 > MAX_SPAN_WORDS {
        return Err(format!(
            "{first}..{last} is more than {MAX_SPAN_WORDS} words; split it into smaller cuts"
        ));
    }
    Ok(transcript.words[a..=b]
        .iter()
        .map(|w| w.id.clone())
        .collect())
}

fn span_text(transcript: &Transcript, ids: &[String]) -> String {
    let text: Vec<&str> = transcript
        .words
        .iter()
        .filter(|w| ids.contains(&w.id))
        .map(|w| w.text.trim())
        .collect();
    let text = text.join(" ");
    if text.chars().count() > 160 {
        format!("{}…", text.chars().take(157).collect::<String>())
    } else {
        text
    }
}

/// Sorted, merged ranges.
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

/// Where the items would cut the edit as it is now; `None` for items with nothing left to cut.
/// Range items count only while the edit is still at `base_revision`.
fn item_cuts(
    document: &EditDocument,
    transcript: Option<&Transcript>,
    stream: &str,
    base_revision: u64,
    item: &CutItem,
) -> Result<Option<Vec<(u64, u64)>>, String> {
    if let Some((first, last)) = item.words() {
        let transcript = transcript.ok_or("The transcript is gone")?;
        let ids = span_ids(transcript, first, last)?;
        return Ok(word_cuts(transcript, &ids, &document.transcript_clock(stream)?).ok());
    }
    match (item.start_us, item.end_us) {
        (Some(start), Some(end)) => {
            if document.revision != base_revision {
                return Err(format!(
                    "{} is a timeline range from revision {base_revision}; the edit has changed \
                     since",
                    item.id
                ));
            }
            Ok(Some(vec![(start, end.min(document.duration_us()))]).filter(|_| start < end))
        }
        _ => Err(format!("{} has neither words nor a range", item.id)),
    }
}

fn load_stream_transcript(root: &Path, stream: &str) -> Result<Transcript, String> {
    store::load_transcript(root, stream)?.ok_or_else(|| format!("{stream} has no transcript"))
}

fn words_stamp(transcript: &Transcript) -> Option<String> {
    transcript.dependency().word_stamp
}

fn cuts_summary(document: &EditDocument, items: &[CutItem], removed: &[(u64, u64)]) -> Vec<String> {
    let total: u64 = removed.iter().map(|(a, b)| b - a).sum();
    let duration = document.duration_us();
    let mut lines = vec![format!(
        "{} cut{}, {:.1} s shorter ({} → {})",
        items.len(),
        if items.len() == 1 { "" } else { "s" },
        total as f64 / 1e6,
        clock(duration),
        clock(duration.saturating_sub(total)),
    )];
    lines.extend(items.iter().take(MAX_SUMMARY_LINES).map(|item| {
        let place = match item.words() {
            Some((a, b)) if a == b => a.to_string(),
            Some((a, b)) => format!("{a}..{b}"),
            None => format!(
                "{}-{}",
                clock(item.start_us.unwrap_or(0)),
                clock(item.end_us.unwrap_or(0))
            ),
        };
        format!("{} {place} \"{}\"", item.id, item.text)
    }));
    if items.len() > MAX_SUMMARY_LINES {
        lines.push(format!("… and {} more", items.len() - MAX_SUMMARY_LINES));
    }
    lines
}

/// Checks the cuts against the edit and stores them as a pending batch.
pub fn propose_cuts(
    reader: &ProjectReader,
    proposals: &mut Proposals,
    agent: &str,
    expected_revision: u64,
    stream: &str,
    inputs: Vec<CutInput>,
    reason: &str,
) -> Result<String, String> {
    let document = reader.document();
    check_revision(document, expected_revision)?;
    if inputs.is_empty() {
        return Err("Propose at least one cut".into());
    }
    if inputs.len() > MAX_ITEMS {
        return Err(format!("At most {MAX_ITEMS} cuts in one batch"));
    }
    let transcript = store::load_transcript(reader.root(), stream)?;
    let mut items = Vec::with_capacity(inputs.len());
    let mut ranges = Vec::new();
    for (index, input) in inputs.into_iter().enumerate() {
        let id = format!("c-{}", index + 1);
        let mut item = CutItem {
            id: id.clone(),
            kind: input.kind.unwrap_or(TranscriptSuggestionKind::Cut),
            first_word_id: None,
            last_word_id: None,
            start_us: None,
            end_us: None,
            text: String::new(),
            reason: check_reason(input.reason.as_deref().unwrap_or(""))?,
            state: ItemState::Pending,
            revision: None,
        };
        match (input.from_word, input.start_us, input.end_us) {
            (Some(first), None, None) => {
                let transcript = transcript
                    .as_ref()
                    .ok_or_else(|| format!("{stream} has no transcript, so cut by range"))?;
                let last = input.to_word.unwrap_or_else(|| first.clone());
                let ids = span_ids(transcript, &first, &last)?;
                // What would be cut: the words of the span still in the edit.
                let clock = document.transcript_clock(stream)?;
                let kept: Vec<String> = transcript
                    .words
                    .iter()
                    .filter(|w| ids.contains(&w.id) && crate::transcript::edit::kept(w, &clock))
                    .map(|w| w.id.clone())
                    .collect();
                item.text = span_text(transcript, if kept.is_empty() { &ids } else { &kept });
                item.first_word_id = Some(first);
                item.last_word_id = Some(last);
            }
            (None, Some(start), Some(end)) => {
                if start >= end || end > document.duration_us() {
                    return Err(format!(
                        "Cut {} must be a range inside the timeline (0 to {} µs)",
                        index + 1,
                        document.duration_us()
                    ));
                }
                item.start_us = Some(start);
                item.end_us = Some(end);
                item.text = format!("{} to {}", clock(start), clock(end));
            }
            _ => {
                return Err(format!(
                    "Cut {} needs either fromWord (and toWord) or startUs and endUs",
                    index + 1
                ))
            }
        }
        let cuts = item_cuts(
            document,
            transcript.as_ref(),
            stream,
            expected_revision,
            &item,
        )?
        .ok_or_else(|| format!("Cut {} ({}) is already cut", index + 1, item.text))?;
        ranges.extend(cuts);
        items.push(item);
    }
    let removed = merge(ranges);
    let summary = cuts_summary(document, &items, &removed);
    let words = transcript
        .as_ref()
        .filter(|_| items.iter().any(|i| i.words().is_some()))
        .and_then(words_stamp);
    proposals.add(Batch {
        id: String::new(),
        agent: agent.to_string(),
        reason: check_reason(reason)?,
        created_at: now(),
        base_revision: expected_revision,
        change: Change::Cuts {
            stream: stream.to_string(),
            words,
            items,
        },
        status: Status::Pending,
        decided_at: None,
        auto: false,
        applied_revision: None,
        summary,
    })
}

fn edit_kind(edit: &SequenceEdit) -> String {
    serde_json::to_value(edit)
        .ok()
        .and_then(|v| v.get("kind").and_then(|k| k.as_str()).map(str::to_string))
        .unwrap_or_else(|| "edit".into())
}

/// The edit with `edits` applied in order to a copy of it.
pub fn apply_edits(
    document: &EditDocument,
    edits: &[SequenceEdit],
) -> Result<EditDocument, String> {
    let mut next = document.clone();
    for (index, edit) in edits.iter().enumerate() {
        let (sequence, _) = crate::sequence::edit::apply_reserving(
            &next.sequence,
            &next.assets,
            edit,
            next.track_ids_in_use(),
        )
        .map_err(|e| format!("Edit {} ({}): {e}", index + 1, edit_kind(edit)))?;
        next.sequence = sequence;
    }
    next.validate()?;
    Ok(next)
}

/// The clips added, removed and changed between two edits, one line each.
pub fn sequence_diff(before: &EditDocument, after: &EditDocument) -> Vec<String> {
    use std::collections::HashMap;
    let place = |document: &EditDocument| {
        let mut clips = HashMap::new();
        for track in &document.sequence.tracks {
            let number = document
                .sequence
                .number(&track.id)
                .unwrap_or_else(|| track.id.clone());
            for clip in &track.clips {
                clips.insert(clip.id.clone(), (number.clone(), clip.clone()));
            }
        }
        clips
    };
    let media = |document: &EditDocument, asset: &str| {
        document
            .assets
            .iter()
            .find(|a| a.id == asset)
            .map(|a| a.name.clone())
            .unwrap_or_else(|| asset.to_string())
    };
    let (old, new) = (place(before), place(after));
    let mut lines = Vec::new();
    let span = |c: &crate::sequence::Clip| format!("{}-{}", clock(c.start_us), clock(c.end_us()));
    let mut added: Vec<_> = new
        .iter()
        .filter(|(id, _)| !old.contains_key(*id))
        .collect();
    added.sort_by_key(|(_, (_, c))| c.start_us);
    for (id, (track, clip)) in added {
        lines.push(format!(
            "+ {track} {id} {} {}",
            media(after, &clip.asset),
            span(clip)
        ));
    }
    let mut removed: Vec<_> = old
        .iter()
        .filter(|(id, _)| !new.contains_key(*id))
        .collect();
    removed.sort_by_key(|(_, (_, c))| c.start_us);
    for (id, (track, clip)) in removed {
        lines.push(format!(
            "- {track} {id} {} {}",
            media(before, &clip.asset),
            span(clip)
        ));
    }
    let mut changed: Vec<_> = new
        .iter()
        .filter_map(|(id, (track, clip))| {
            let (was_track, was) = old.get(id)?;
            let moved = was_track != track
                || was.start_us != clip.start_us
                || was.in_us != clip.in_us
                || was.duration_us != clip.duration_us
                || was.fit != clip.fit;
            moved.then_some((id, track, clip, was_track, was))
        })
        .collect();
    changed.sort_by_key(|(_, _, c, _, _)| c.start_us);
    for (id, track, clip, was_track, was) in changed {
        let fit = if was.fit != clip.fit {
            " (fit changed)"
        } else {
            ""
        };
        lines.push(format!(
            "~ {track} {id} {} (was {was_track} {}){fit}",
            span(clip),
            span(was)
        ));
    }
    let tracks = |d: &EditDocument| {
        d.sequence
            .tracks
            .iter()
            .map(|t| t.id.clone())
            .collect::<Vec<_>>()
    };
    let (old_tracks, new_tracks) = (tracks(before), tracks(after));
    for id in new_tracks.iter().filter(|id| !old_tracks.contains(id)) {
        lines.push(format!(
            "+ track {}",
            after.sequence.number(id).unwrap_or_else(|| id.clone())
        ));
    }
    for id in old_tracks.iter().filter(|id| !new_tracks.contains(id)) {
        lines.push(format!(
            "- track {}",
            before.sequence.number(id).unwrap_or_else(|| id.clone())
        ));
    }
    let total = lines.len();
    lines.truncate(MAX_SUMMARY_LINES);
    if total > MAX_SUMMARY_LINES {
        lines.push(format!("… and {} more", total - MAX_SUMMARY_LINES));
    }
    let (a, b) = (before.duration_us(), after.duration_us());
    if a != b {
        lines.insert(0, format!("Length {} → {}", clock(a), clock(b)));
    }
    if lines.is_empty() {
        lines.push("No change to the timeline".into());
    }
    lines
}

/// Checks the edits on a copy of the edit and stores them as a pending batch.
pub fn propose_edits(
    reader: &ProjectReader,
    proposals: &mut Proposals,
    agent: &str,
    expected_revision: u64,
    edits: Vec<SequenceEdit>,
    reason: &str,
) -> Result<String, String> {
    let document = reader.document();
    check_revision(document, expected_revision)?;
    if edits.is_empty() {
        return Err("Propose at least one edit".into());
    }
    if edits.len() > MAX_EDITS {
        return Err(format!("At most {MAX_EDITS} edits in one batch"));
    }
    let next = apply_edits(document, &edits)?;
    let summary = sequence_diff(document, &next);
    proposals.add(Batch {
        id: String::new(),
        agent: agent.to_string(),
        reason: check_reason(reason)?,
        created_at: now(),
        base_revision: expected_revision,
        change: Change::Edits { edits },
        status: Status::Pending,
        decided_at: None,
        auto: false,
        applied_revision: None,
        summary,
    })
}

/// Proposes putting `items` under take short `short`, checked on a copy of the edit.
pub fn propose_under(
    reader: &ProjectReader,
    proposals: &mut Proposals,
    agent: &str,
    short: &str,
    items: Vec<crate::under_take::Under>,
    reason: &str,
) -> Result<String, String> {
    if items.len() > MAX_EDITS {
        return Err(format!("At most {MAX_EDITS} items in one batch"));
    }
    let document = reader.document();
    let summary = crate::under_take::check(reader.root(), document, short, &items)?;
    proposals.add(Batch {
        id: String::new(),
        agent: agent.to_string(),
        reason: check_reason(reason)?,
        created_at: now(),
        base_revision: document.revision,
        change: Change::Under {
            short: short.to_string(),
            stamp: crate::under_take::stamp(document, short),
            items,
        },
        status: Status::Pending,
        decided_at: None,
        auto: false,
        applied_revision: None,
        summary,
    })
}

// ---------------------------------------------------------------------------------------
// Deciding.

impl Batch {
    /// Pending, accepted or rejected from its items once none is left waiting.
    fn settle(&mut self) {
        if self.status == Status::Withdrawn {
            return;
        }
        let Change::Cuts { items, .. } = &self.change else {
            return;
        };
        let was = self.status;
        self.status = if items.iter().any(|i| i.state == ItemState::Pending) {
            Status::Pending
        } else if items.iter().any(|i| i.state == ItemState::Accepted) {
            Status::Accepted
        } else {
            Status::Rejected
        };
        if self.status != Status::Pending && was == Status::Pending {
            self.decided_at = Some(now());
        }
        if self.status == Status::Pending {
            self.decided_at = None;
        }
    }
}

/// Accepts batch `id` (only items `only`, for cuts) as one undo step. With `reject_rest`, its
/// other waiting items are rejected. `None` when there was nothing left to change (every
/// chosen word was already cut).
pub fn accept(
    reader: &mut ProjectReader,
    proposals: &mut Proposals,
    id: &str,
    expected_revision: u64,
    only: Option<&[String]>,
    reject_rest: bool,
    auto: bool,
) -> Result<Option<OpenedProject>, String> {
    let document = reader.document();
    if document.revision != expected_revision {
        return Err("Stale edit revision".into());
    }
    let batch = proposals.get(id)?;
    if batch.status == Status::Withdrawn {
        return Err(format!("{id} was withdrawn"));
    }
    let base_revision = batch.base_revision;
    let (summary, chosen) = match &batch.change {
        Change::Action { .. } => return Err(format!("{id} was already applied")),
        Change::Edits { edits } => {
            if batch.status != Status::Pending {
                return Err(format!("{id} is already decided"));
            }
            if document.revision != base_revision {
                return Err(format!(
                    "{id} was made for revision {base_revision} and the edit has changed since. \
                     Ask the agent to propose it again."
                ));
            }
            let next = apply_edits(document, edits)?;
            (
                Some(reader.commit_document(expected_revision, next)?),
                Vec::new(),
            )
        }
        Change::Under {
            short,
            stamp,
            items,
        } => {
            if batch.status != Status::Pending {
                return Err(format!("{id} is already decided"));
            }
            if crate::under_take::stamp(document, short) != *stamp {
                return Err(format!(
                    "{id} was made before {short} or the video changed. Ask the agent to                      propose it again."
                ));
            }
            let root = reader.root().to_path_buf();
            let done = crate::under_take::put(&root, &root, document, short, items)?;
            match reader.commit_document(expected_revision, done.document.clone()) {
                Ok(project) => (Some(project), Vec::new()),
                Err(error) => {
                    done.discard();
                    return Err(error);
                }
            }
        }
        Change::Cuts {
            stream,
            words,
            items,
        } => {
            let chosen: Vec<&CutItem> = items
                .iter()
                .filter(|i| i.state == ItemState::Pending)
                .filter(|i| only.is_none_or(|ids| ids.contains(&i.id)))
                .collect();
            if let Some(ids) = only {
                if let Some(missing) = ids.iter().find(|id| !items.iter().any(|i| &i.id == *id)) {
                    return Err(format!("{id} has no item {missing}"));
                }
            }
            if chosen.is_empty() {
                return Err(format!("{id} has nothing waiting to accept"));
            }
            let transcript = if chosen.iter().any(|i| i.words().is_some()) {
                let transcript = load_stream_transcript(reader.root(), stream)?;
                if words_stamp(&transcript) != *words {
                    return Err(format!(
                        "{id} names words of an older transcript of {stream}: it was \
                         retranscribed or its words changed since. Ask the agent to propose it \
                         again."
                    ));
                }
                Some(transcript)
            } else {
                None
            };
            let mut ranges = Vec::new();
            for item in &chosen {
                if let Some(cuts) =
                    item_cuts(document, transcript.as_ref(), stream, base_revision, item)?
                {
                    ranges.extend(cuts);
                }
            }
            let ranges = merge(ranges);
            let chosen: Vec<String> = chosen.iter().map(|i| i.id.clone()).collect();
            if ranges.is_empty() {
                (None, chosen)
            } else {
                (
                    Some(reader.ripple_cuts(expected_revision, &ranges)?),
                    chosen,
                )
            }
        }
    };
    let revision = summary.as_ref().map(|s| s.revision);
    let batch = proposals.get_mut(id)?;
    batch.auto = auto;
    if revision.is_some() {
        batch.applied_revision = revision;
    }
    match &mut batch.change {
        Change::Action { .. } => {}
        Change::Edits { .. } | Change::Under { .. } => {
            batch.status = Status::Accepted;
            batch.decided_at = Some(now());
        }
        Change::Cuts { items, .. } => {
            for item in items.iter_mut() {
                if chosen.contains(&item.id) {
                    item.state = ItemState::Accepted;
                    item.revision = revision;
                } else if reject_rest && item.state == ItemState::Pending {
                    item.state = ItemState::Rejected;
                }
            }
            batch.settle();
        }
    }
    proposals.prune();
    Ok(summary)
}

/// Rejects batch `id` (only items `only`, for cuts), or with `rejected: false` puts rejected
/// items back to waiting.
pub fn reject(
    proposals: &mut Proposals,
    id: &str,
    only: Option<&[String]>,
    rejected: bool,
) -> Result<(), String> {
    let batch = proposals.get_mut(id)?;
    if batch.status == Status::Withdrawn {
        return Err(format!("{id} was withdrawn"));
    }
    match &mut batch.change {
        Change::Action { .. } => return Err(format!("{id} was already applied; undo it instead")),
        Change::Edits { .. } | Change::Under { .. } => match (batch.status, rejected) {
            (Status::Pending, true) => {
                batch.status = Status::Rejected;
                batch.decided_at = Some(now());
            }
            (Status::Rejected, false) => {
                batch.status = Status::Pending;
                batch.decided_at = None;
            }
            _ => return Err(format!("{id} is already decided")),
        },
        Change::Cuts { items, .. } => {
            let (from, to) = if rejected {
                (ItemState::Pending, ItemState::Rejected)
            } else {
                (ItemState::Rejected, ItemState::Pending)
            };
            let mut changed = 0;
            for item in items.iter_mut() {
                if item.state == from && only.is_none_or(|ids| ids.contains(&item.id)) {
                    item.state = to;
                    changed += 1;
                }
            }
            if changed == 0 {
                return Err(format!("{id} has nothing to change"));
            }
            batch.settle();
        }
    }
    proposals.prune();
    Ok(())
}

/// The agent takes back a batch it made that is still waiting.
pub fn withdraw(proposals: &mut Proposals, id: &str, agent: &str) -> Result<(), String> {
    let batch = proposals.get_mut(id)?;
    if batch.agent != agent {
        return Err(format!("{id} was proposed by {}, not by you", batch.agent));
    }
    if batch.status != Status::Pending {
        return Err(format!("{id} is no longer waiting"));
    }
    batch.status = Status::Withdrawn;
    batch.decided_at = Some(now());
    proposals.prune();
    Ok(())
}

/// Undoes batch `id`'s last accept, while nothing has changed the edit since. For the user the
/// items it cut wait again; for the agent (`by_agent`) the batch is withdrawn.
pub fn undo(
    reader: &mut ProjectReader,
    proposals: &mut Proposals,
    id: &str,
    expected_revision: u64,
    by_agent: Option<&str>,
) -> Result<OpenedProject, String> {
    let current = reader.document().revision;
    if current != expected_revision {
        return Err("Stale edit revision".into());
    }
    let batch = proposals.get(id)?;
    if let Some(agent) = by_agent {
        if batch.agent != agent {
            return Err(format!("{id} was proposed by {}, not by you", batch.agent));
        }
    }
    let Some(applied) = batch.applied_revision else {
        return Err(format!("{id} has not been accepted"));
    };
    if applied != current {
        return Err(format!(
            "{id} can no longer be undone on its own: the edit changed after it (revision \
             {applied}, now {current})"
        ));
    }
    let summary = reader.undo(expected_revision)?;
    let batch = proposals.get_mut(id)?;
    batch.applied_revision = None;
    batch.auto = false;
    match &mut batch.change {
        Change::Action { .. } => batch.status = Status::Undone,
        Change::Edits { .. } | Change::Under { .. } => batch.status = Status::Pending,
        Change::Cuts { items, .. } => {
            for item in items.iter_mut() {
                if item.state == ItemState::Accepted && item.revision == Some(applied) {
                    item.state = ItemState::Pending;
                    item.revision = None;
                }
            }
            batch.status = Status::Pending;
            batch.settle();
        }
    }
    if by_agent.is_some() && batch.status != Status::Undone {
        batch.status = Status::Withdrawn;
    }
    batch.decided_at = (batch.status != Status::Pending).then(now);
    Ok(summary)
}

// ---------------------------------------------------------------------------------------
// Views.

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ItemView {
    pub id: String,
    pub kind: TranscriptSuggestionKind,
    pub state: ItemState,
    pub text: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_word_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_word_id: Option<String>,
    /// The words it would cut now (only the ones still in the edit).
    pub word_ids: Vec<String>,
    /// Where it would cut now, on the edited timeline.
    pub edited_start_us: Option<u64>,
    pub edited_end_us: Option<u64>,
    /// Waiting, but there is nothing left to cut (the words went another way).
    pub already_cut: bool,
    /// A timeline range from an older revision, or words of an older transcript.
    pub stale: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BatchView {
    pub id: String,
    pub agent: String,
    pub reason: String,
    pub created_at: String,
    pub base_revision: u64,
    /// `cuts`, `edits`, `under` (things put under a take short), or `action` (one of the
    /// app's actions the agent ran).
    pub kind: &'static str,
    pub status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decided_at: Option<String>,
    pub auto: bool,
    pub applied_revision: Option<u64>,
    pub summary: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edits: Option<Vec<SequenceEdit>>,
    /// For an action the agent ran: which (`silence`, `fillers`, ...).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    /// For `under`: the take short it goes under.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,
    pub items: Vec<ItemView>,
    /// Made for an older revision and can no longer be accepted as it is.
    pub stale: bool,
    /// Its accept is the latest change, so it can be undone on its own.
    pub can_undo: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProposalsView {
    /// The project these are of: batch ids repeat across projects, so a window drops a view
    /// of a project it no longer shows.
    pub project_handle: String,
    pub revision: u64,
    /// Newest first.
    pub batches: Vec<BatchView>,
    /// Set when a change was made but the proposals file could not be saved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

pub fn view(reader: &ProjectReader, proposals: &Proposals) -> ProposalsView {
    let document = reader.document();
    let mut transcripts: std::collections::HashMap<String, (Option<Transcript>, Option<String>)> =
        Default::default();
    let batches = proposals
        .batches
        .iter()
        .rev()
        .map(|batch| {
            let mut out = BatchView {
                id: batch.id.clone(),
                agent: batch.agent.clone(),
                reason: batch.reason.clone(),
                created_at: batch.created_at.clone(),
                base_revision: batch.base_revision,
                kind: "edits",
                status: batch.status,
                decided_at: batch.decided_at.clone(),
                auto: batch.auto,
                applied_revision: batch.applied_revision,
                summary: batch.summary.clone(),
                stream: None,
                edits: None,
                action: None,
                short: None,
                items: Vec::new(),
                stale: false,
                can_undo: batch.applied_revision == Some(document.revision),
            };
            match &batch.change {
                Change::Action { action } => {
                    out.kind = "action";
                    out.action = Some(action.clone());
                }
                Change::Edits { edits } => {
                    out.edits = Some(edits.clone());
                    out.stale =
                        batch.status == Status::Pending && batch.base_revision != document.revision;
                }
                Change::Under { short, stamp, .. } => {
                    out.kind = "under";
                    out.short = Some(short.clone());
                    out.stale = batch.status == Status::Pending
                        && crate::under_take::stamp(document, short) != *stamp;
                }
                Change::Cuts {
                    stream,
                    words,
                    items,
                } => {
                    out.kind = "cuts";
                    out.stream = Some(stream.clone());
                    let (transcript, stamp) =
                        transcripts.entry(stream.clone()).or_insert_with(|| {
                            let transcript =
                                store::load_transcript(reader.root(), stream).ok().flatten();
                            let stamp = transcript.as_ref().and_then(words_stamp);
                            (transcript, stamp)
                        });
                    // Word items of an older transcript go stale: its ids name other words now.
                    let transcript = transcript.as_ref().filter(|_| stamp == words);
                    out.items = items
                        .iter()
                        .map(|item| {
                            item_view(document, transcript, stream, batch.base_revision, item)
                        })
                        .collect();
                    let waiting: Vec<_> = out
                        .items
                        .iter()
                        .filter(|i| i.state == ItemState::Pending)
                        .collect();
                    out.stale = !waiting.is_empty() && waiting.iter().all(|i| i.stale);
                }
            }
            out
        })
        .collect();
    ProposalsView {
        project_handle: reader.summary.project_handle.clone(),
        revision: document.revision,
        batches,
        warning: None,
    }
}

fn item_view(
    document: &EditDocument,
    transcript: Option<&Transcript>,
    stream: &str,
    base_revision: u64,
    item: &CutItem,
) -> ItemView {
    let mut view = ItemView {
        id: item.id.clone(),
        kind: item.kind,
        state: item.state,
        text: item.text.clone(),
        reason: item.reason.clone(),
        first_word_id: item.first_word_id.clone(),
        last_word_id: item.last_word_id.clone(),
        word_ids: Vec::new(),
        edited_start_us: None,
        edited_end_us: None,
        already_cut: false,
        stale: false,
    };
    if let (Some((first, last)), Some(transcript)) = (item.words(), transcript) {
        if let (Ok(ids), Ok(clock)) = (
            span_ids(transcript, first, last),
            document.transcript_clock(stream),
        ) {
            view.word_ids = ids
                .into_iter()
                .filter(|id| {
                    transcript
                        .words
                        .iter()
                        .find(|w| &w.id == id)
                        .is_some_and(|w| crate::transcript::edit::kept(w, &clock))
                })
                .collect();
        }
    }
    match item_cuts(document, transcript, stream, base_revision, item) {
        Ok(Some(cuts)) => {
            view.edited_start_us = cuts.first().map(|c| c.0);
            view.edited_end_us = cuts.last().map(|c| c.1);
        }
        Ok(None) => view.already_cut = item.state == ItemState::Pending,
        Err(_) => view.stale = item.state == ItemState::Pending,
    }
    view
}

/// The agents' waiting (and rejected) word cuts on `stream`, as transcript suggestions, so they
/// sit in the transcript's review list beside the rules' and the AI pass's.
pub fn transcript_suggestions(
    reader: &ProjectReader,
    proposals: &Proposals,
    transcript: &Transcript,
    stream: &str,
) -> Vec<TranscriptCutSuggestion> {
    let document = reader.document();
    let stamp = words_stamp(transcript);
    let mut out = Vec::new();
    for batch in &proposals.batches {
        let Change::Cuts {
            stream: s,
            words,
            items,
        } = &batch.change
        else {
            continue;
        };
        // A batch about an older transcript's words would mark other words now.
        if s != stream || batch.status == Status::Withdrawn || *words != stamp {
            continue;
        }
        for item in items {
            if item.state == ItemState::Accepted || item.words().is_none() {
                continue;
            }
            let view = item_view(
                document,
                Some(transcript),
                stream,
                batch.base_revision,
                item,
            );
            let (Some(start), Some(end)) = (view.edited_start_us, view.edited_end_us) else {
                continue;
            };
            out.push(TranscriptCutSuggestion {
                id: format!("agent-{}-{}", batch.id, item.id),
                kind: item.kind,
                word_ids: view.word_ids,
                text: item.text.clone(),
                edited_start_us: start,
                edited_end_us: end,
                dismissed: item.state == ItemState::Rejected,
                source: SuggestionSource::Agent,
                reason: Some(if item.reason.is_empty() {
                    batch.reason.clone()
                } else {
                    item.reason.clone()
                })
                .filter(|r| !r.is_empty()),
                reviewer: None,
                proposal: Some(ProposalRef {
                    batch_id: batch.id.clone(),
                    item_id: item.id.clone(),
                    agent: batch.agent.clone(),
                }),
            });
        }
    }
    out
}

/// `suggestions` with the agents' cuts on `stream` joined in: an agent's cut of exactly the
/// same words replaces the rules' one, since it carries a reason and a decision of its own.
pub fn with_agent_suggestions(
    reader: &ProjectReader,
    transcript: &Transcript,
    stream: &str,
    mut suggestions: Vec<TranscriptCutSuggestion>,
) -> Vec<TranscriptCutSuggestion> {
    let Ok(proposals) = load(reader.root()) else {
        return suggestions;
    };
    let agent = transcript_suggestions(reader, &proposals, transcript, stream);
    if agent.is_empty() {
        return suggestions;
    }
    suggestions.retain(|s| !agent.iter().any(|a| a.word_ids == s.word_ids));
    suggestions.extend(agent);
    suggestions.sort_by_key(|s| s.edited_start_us);
    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::{self, AppState};
    use crate::transcript::{ProviderKind, TranscriptWord, WordKind};
    use serde_json::json;

    /// A project with a 2 s sound on the timeline and a transcript of it; returns its stream key.
    fn project(dir: &Path) -> (AppState, String) {
        let wav = dir.join("voice.wav");
        let samples = vec![0i16; 48_000 * 2];
        fs::write(
            &wav,
            crate::fixtures::generate_pcm16_wav(48_000, 1, &samples),
        )
        .unwrap();
        let folder = crate::project::folder::create_project_folder(dir, "Lesson", None).unwrap();
        let state = AppState::new();
        let opened = commands::open_project_impl(&state, folder.to_string_lossy().into()).unwrap();
        let opened = commands::project_media_import_impl(
            &state,
            opened.project_handle.clone(),
            opened.revision,
            vec![wav.to_string_lossy().into()],
        )
        .unwrap();
        let asset = opened.assets[0].clone();
        commands::project_sequence_edit_impl(
            &state,
            opened.project_handle.clone(),
            opened.revision,
            serde_json::from_value(json!({ "kind": "placeAsset", "assetId": asset.id, "atUs": 0 }))
                .unwrap(),
            None,
        )
        .unwrap();
        let key = format!("{}.{}", asset.id, asset.streams[0].id);
        let words = [
            ("Hello", 0, 300),
            ("um", 400, 600),
            ("there.", 700, 1000),
            ("So", 1200, 1400),
            ("next", 1500, 1800),
        ];
        let mut transcript = Transcript::new(
            key.clone(),
            ProviderKind::Parakeet,
            "test".into(),
            None,
            Vec::new(),
        );
        transcript.words = words
            .iter()
            .enumerate()
            .map(|(i, (text, start, end))| TranscriptWord {
                id: format!("w-{i}"),
                text: (*text).into(),
                kind: WordKind::Word,
                source_start_us: start * 1000,
                source_end_us: end * 1000,
                confidence: None,
                speaker: None,
            })
            .collect();
        store::save_transcript(&folder, &transcript).unwrap();
        (state, key)
    }

    fn words(from: &str, to: &str, reason: &str) -> CutInput {
        CutInput {
            from_word: Some(from.into()),
            to_word: Some(to.into()),
            reason: Some(reason.into()),
            ..Default::default()
        }
    }

    fn with<T>(state: &AppState, f: impl FnOnce(&mut ProjectReader, &mut Proposals) -> T) -> T {
        let mut opened = state.opened_project.lock();
        let reader = opened.as_mut().unwrap();
        let mut proposals = load(reader.root()).unwrap();
        let out = f(reader, &mut proposals);
        save(reader.root(), &proposals).unwrap();
        out
    }

    fn revision_and_length(reader: &ProjectReader) -> (u64, u64) {
        (reader.document().revision, reader.document().duration_us())
    }

    #[test]
    fn a_cut_batch_waits_then_lands_as_one_undo_step_and_undoes_back_to_waiting() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (state, key) = project(dir.path());
        with(&state, |reader, proposals| {
            let (revision, length) = revision_and_length(reader);
            let id = propose_cuts(
                reader,
                proposals,
                "claude-code",
                revision,
                &key,
                vec![
                    words("w-1", "w-1", "filler"),
                    words("w-3", "w-4", "tangent"),
                ],
                "Tighten the intro",
            )
            .unwrap();
            assert_eq!(id, "b-1");
            // Proposing changes nothing in the edit.
            assert_eq!(revision_and_length(reader), (revision, length));
            let batch = proposals.get(&id).unwrap();
            assert_eq!(batch.status, Status::Pending);
            assert!(
                batch.summary[0].starts_with("2 cuts"),
                "{:?}",
                batch.summary
            );
            assert!(
                batch.summary[1].contains("c-1 w-1 \"um\""),
                "{:?}",
                batch.summary
            );

            // A stale revision is refused with the current one named.
            let error = propose_cuts(
                reader,
                proposals,
                "a",
                revision + 7,
                &key,
                vec![words("w-0", "w-0", "")],
                "",
            )
            .unwrap_err();
            assert!(error.contains(&format!("revision {revision}")), "{error}");

            let summary = accept(reader, proposals, &id, revision, None, false, false)
                .unwrap()
                .unwrap();
            assert_eq!(summary.revision, revision + 1);
            assert!(reader.document().duration_us() < length);
            let batch = proposals.get(&id).unwrap();
            assert_eq!(batch.status, Status::Accepted);
            assert_eq!(batch.applied_revision, Some(revision + 1));
            let view = view(reader, proposals);
            assert!(view.batches[0].can_undo);
            assert!(view.batches[0]
                .items
                .iter()
                .all(|i| i.state == ItemState::Accepted));

            // One undo step: undoing the batch brings back the whole length.
            undo(reader, proposals, &id, revision + 1, None).unwrap();
            assert_eq!(reader.document().duration_us(), length);
            let batch = proposals.get(&id).unwrap();
            assert_eq!(batch.status, Status::Pending);
            assert_eq!(batch.applied_revision, None);
        });
        // And it was saved beside the edit.
        let root = state
            .opened_project
            .lock()
            .as_ref()
            .unwrap()
            .root()
            .to_path_buf();
        assert_eq!(load(&root).unwrap().batches.len(), 1);
    }

    #[test]
    fn word_cuts_go_stale_when_the_transcript_is_replaced() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (state, key) = project(dir.path());
        with(&state, |reader, proposals| {
            let (revision, length) = revision_and_length(reader);
            let id = propose_cuts(
                reader,
                proposals,
                "a",
                revision,
                &key,
                vec![words("w-1", "w-1", "")],
                "",
            )
            .unwrap();
            // Retranscribing gives the ids to other words: `w-1` is no longer "um".
            let mut transcript = store::load_transcript(reader.root(), &key)
                .unwrap()
                .unwrap();
            transcript.words[1].text = "Important".into();
            transcript.words[1].source_start_us = 320_000;
            transcript.words[1].source_end_us = 680_000;
            store::save_transcript(reader.root(), &transcript).unwrap();

            let shown = view(reader, proposals);
            assert!(shown.batches[0].stale);
            assert!(shown.batches[0].items[0].stale);
            assert!(transcript_suggestions(reader, proposals, &transcript, &key).is_empty());
            let error = accept(reader, proposals, &id, revision, None, false, false).unwrap_err();
            assert!(error.contains("older transcript"), "{error}");
            assert_eq!(revision_and_length(reader), (revision, length));
        });
    }

    #[test]
    fn word_cuts_survive_later_edits_but_timeline_ranges_go_stale() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (state, key) = project(dir.path());
        with(&state, |reader, proposals| {
            let revision = reader.document().revision;
            let by_words = propose_cuts(
                reader,
                proposals,
                "a",
                revision,
                &key,
                vec![words("w-3", "w-3", "")],
                "",
            )
            .unwrap();
            let by_range = propose_cuts(
                reader,
                proposals,
                "a",
                revision,
                &key,
                vec![CutInput {
                    start_us: Some(1_900_000),
                    end_us: Some(2_000_000),
                    ..Default::default()
                }],
                "dead air",
            )
            .unwrap();
            // The user cuts "um" meanwhile: everything after it moves earlier.
            reader.ripple_cuts(revision, &[(350_000, 650_000)]).unwrap();
            let revision = reader.document().revision;
            // A span over the gap reads as what is left of it.
            let across = propose_cuts(
                reader,
                proposals,
                "a",
                revision,
                &key,
                vec![words("w-0", "w-2", "")],
                "",
            )
            .unwrap();
            let Change::Cuts { items, .. } = &proposals.get(&across).unwrap().change else {
                panic!()
            };
            assert_eq!(items[0].text, "Hello there.");
            let view = view(reader, proposals);
            let range = view.batches.iter().find(|b| b.id == by_range).unwrap();
            assert!(range.stale);
            let word = view.batches.iter().find(|b| b.id == by_words).unwrap();
            assert!(!word.stale);
            assert_eq!(word.items[0].word_ids, vec!["w-3".to_string()]);
            // "So" (1.2 s in the recording) now plays 0.3 s earlier.
            assert!(word.items[0].edited_start_us.unwrap() < 1_100_000);

            assert!(
                accept(reader, proposals, &by_range, revision, None, false, false)
                    .unwrap_err()
                    .contains("timeline range from revision")
            );
            let before = reader.document().duration_us();
            accept(reader, proposals, &by_words, revision, None, false, false).unwrap();
            assert!(reader.document().duration_us() < before);
        });
    }

    #[test]
    fn items_are_accepted_and_rejected_one_by_one() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (state, key) = project(dir.path());
        with(&state, |reader, proposals| {
            let revision = reader.document().revision;
            let id = propose_cuts(
                reader,
                proposals,
                "a",
                revision,
                &key,
                vec![
                    words("w-1", "w-1", ""),
                    words("w-3", "w-3", ""),
                    words("w-4", "w-4", ""),
                ],
                "",
            )
            .unwrap();
            reject(proposals, &id, Some(&["c-2".into()]), true).unwrap();
            let transcript = store::load_transcript(reader.root(), &key)
                .unwrap()
                .unwrap();
            let listed = transcript_suggestions(reader, proposals, &transcript, &key);
            assert_eq!(listed.len(), 3);
            assert_eq!(listed[0].source, SuggestionSource::Agent);
            assert_eq!(listed[0].id, format!("agent-{id}-c-1"));
            assert!(listed[1].dismissed);
            assert_eq!(listed[0].proposal.as_ref().unwrap().batch_id, id);

            // The transcript list accepts one item; the batch keeps waiting for the rest.
            accept(
                reader,
                proposals,
                &id,
                revision,
                Some(&["c-1".into()]),
                false,
                false,
            )
            .unwrap();
            assert_eq!(proposals.get(&id).unwrap().status, Status::Pending);
            // The panel accepts what is ticked and rejects the rest.
            let revision = reader.document().revision;
            reject(proposals, &id, Some(&["c-2".into()]), false).unwrap();
            accept(
                reader,
                proposals,
                &id,
                revision,
                Some(&["c-3".into()]),
                true,
                false,
            )
            .unwrap();
            let batch = proposals.get(&id).unwrap();
            assert_eq!(batch.status, Status::Accepted);
            let Change::Cuts { items, .. } = &batch.change else {
                panic!()
            };
            let states: Vec<ItemState> = items.iter().map(|i| i.state).collect();
            assert_eq!(
                states,
                [
                    ItemState::Accepted,
                    ItemState::Rejected,
                    ItemState::Accepted
                ]
            );
            // Undo only takes back the last accept.
            let revision = reader.document().revision;
            undo(reader, proposals, &id, revision, None).unwrap();
            let Change::Cuts { items, .. } = &proposals.get(&id).unwrap().change else {
                panic!()
            };
            assert_eq!(items[0].state, ItemState::Accepted);
            assert_eq!(items[2].state, ItemState::Pending);
        });
    }

    #[test]
    fn edit_batches_apply_together_and_only_at_their_revision() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (state, _) = project(dir.path());
        with(&state, |reader, proposals| {
            let revision = reader.document().revision;
            let tracks = reader.document().sequence.tracks.len();
            let edits: Vec<SequenceEdit> = serde_json::from_value(json!([
                { "kind": "addTrack", "trackKind": "video" },
                { "kind": "split", "atUs": 1_000_000 },
            ]))
            .unwrap();
            let id = propose_edits(
                reader,
                proposals,
                "codex",
                revision,
                edits.clone(),
                "Split at the topic change",
            )
            .unwrap();
            let summary = &proposals.get(&id).unwrap().summary;
            assert!(
                summary.iter().any(|l| l.starts_with("+ track V")),
                "{summary:?}"
            );
            assert!(summary.iter().any(|l| l.starts_with("~ A1")), "{summary:?}");
            assert_eq!(reader.document().revision, revision);

            // A bad edit is refused when proposed, naming which one.
            let bad: Vec<SequenceEdit> =
                serde_json::from_value(json!([{ "kind": "delete", "clipIds": ["nope"] }])).unwrap();
            let error = propose_edits(reader, proposals, "codex", revision, bad, "").unwrap_err();
            assert!(error.starts_with("Edit 1 (delete)"), "{error}");

            accept(reader, proposals, &id, revision, None, false, false).unwrap();
            assert_eq!(reader.document().revision, revision + 1);
            assert_eq!(reader.document().sequence.tracks.len(), tracks + 1);
            // Both edits are one undo step.
            reader.undo(revision + 1).unwrap();
            assert_eq!(reader.document().sequence.tracks.len(), tracks);

            // A batch made before another change goes stale rather than guessing.
            let revision = reader.document().revision;
            let late = propose_edits(reader, proposals, "codex", revision, edits, "").unwrap();
            reader.ripple_cuts(revision, &[(0, 100_000)]).unwrap();
            assert!(view(reader, proposals).batches[0].stale);
            let revision = reader.document().revision;
            assert!(
                accept(reader, proposals, &late, revision, None, false, false)
                    .unwrap_err()
                    .contains("propose it again")
            );
        });
    }

    #[test]
    fn agents_withdraw_and_undo_only_their_own_latest_batches() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (state, key) = project(dir.path());
        with(&state, |reader, proposals| {
            let revision = reader.document().revision;
            let id = propose_cuts(
                reader,
                proposals,
                "claude-code",
                revision,
                &key,
                vec![words("w-1", "w-1", "")],
                "",
            )
            .unwrap();
            assert!(withdraw(proposals, &id, "codex")
                .unwrap_err()
                .contains("claude-code"));
            accept(reader, proposals, &id, revision, None, false, true).unwrap();
            assert!(proposals.get(&id).unwrap().auto);
            assert!(withdraw(proposals, &id, "claude-code")
                .unwrap_err()
                .contains("no longer waiting"));

            // Something else changed the edit after it: it can't be undone alone.
            let after = reader.document().revision;
            reader.ripple_cuts(after, &[(0, 100_000)]).unwrap();
            let now = reader.document().revision;
            assert!(undo(reader, proposals, &id, now, Some("claude-code"))
                .unwrap_err()
                .contains("can no longer be undone"));
            reader.undo(now).unwrap();
            let now = reader.document().revision;
            // The revision number moved on, so even after the user's undo it is not the batch's.
            assert!(undo(reader, proposals, &id, now, Some("claude-code")).is_err());
        });
    }

    #[test]
    fn merge_sorts_and_joins_overlaps() {
        assert_eq!(
            merge(vec![(50, 60), (0, 10), (5, 20), (20, 30), (40, 40)]),
            vec![(0, 30), (50, 60)]
        );
    }

    #[test]
    fn prune_keeps_every_pending_batch_and_the_newest_decided() {
        let batch = |status| Batch {
            id: String::new(),
            agent: "a".into(),
            reason: String::new(),
            created_at: String::new(),
            base_revision: 0,
            change: Change::Edits { edits: Vec::new() },
            status,
            decided_at: None,
            auto: false,
            applied_revision: None,
            summary: Vec::new(),
        };
        let mut proposals = Proposals::default();
        proposals.batches.push(batch(Status::Pending));
        for _ in 0..KEEP_DECIDED + 5 {
            proposals.batches.push(batch(Status::Rejected));
        }
        proposals.prune();
        assert_eq!(proposals.batches.len(), KEEP_DECIDED + 1);
        assert_eq!(proposals.batches[0].status, Status::Pending);
    }
}
