//! The joins to listen to (REV-1). An agent can't hear, so it hands the user the cuts most
//! likely to sound wrong; the user plays each in the Joins panel and marks it keep or fix, and
//! the agent reads the marks back. Kept beside the edit in `agent/joins.json`, outside the
//! undo history: listening changes nothing in the edit.
//!
//! A join is held by the recording times on either side of it, on the clock `check` counts
//! cuts on (the first speech on the timeline), so later edits move it with the edit. A join a
//! later edit takes away or changes (the fix) shows as changed.
use crate::check::{self, Kind, Piece};
use crate::project::reader::{open_regular, safe_path, ProjectReader};
use crate::project::revision::EditDocument;
use crate::proposals::PROPOSALS_DIR;
use crate::transcript::{store, TranscriptWord, WordKind};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

const JOINS_FILE: &str = "agent/joins.json";
const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_JOINS: usize = 200;
const MAX_REASON_CHARS: usize = 300;
const MAX_NOTE_CHARS: usize = 500;
/// A moment the agent names snaps to the cut this close to it.
pub const SNAP_US: u64 = 1_500_000;
/// What `find` adds unless told otherwise.
pub const FIND_DEFAULT: usize = 30;
/// Without a transcript, more than this taken out between two kept stretches may be a retake.
const LONG_CUT_US: u64 = 2_500_000;
/// This many words cut out at one join is likely a retake.
const RETAKE_MIN_WORDS: usize = 2;
/// A word ending (or starting) this close to the cut may lose its tail (or its start).
const TIGHT_US: u64 = 40_000;
/// A word the transcriber was less sure of than this, next to a cut.
const UNSURE: f32 = 0.6;
/// A `check` finding this close to a cut is about it.
const FINDING_NEAR_US: u64 = 500_000;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Joins {
    pub next_id: u64,
    /// In the order they were added; shown in timeline order.
    pub items: Vec<Join>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Join {
    /// `j-<n>`.
    pub id: String,
    /// The sound whose clock it is on (none: the recording's picture).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clock: Option<String>,
    /// Where the recording stops before the join, and starts again after it. The same for a
    /// moment that is not a cut.
    pub before_us: u64,
    pub after_us: u64,
    pub reason: String,
    /// The agent that added it, or `app` for `find`.
    pub by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark: Option<Mark>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Mark {
    Keep,
    Fix,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JoinView {
    pub id: String,
    /// Where it plays on the edited timeline; none once an edit took it away or changed it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at_us: Option<u64>,
    /// Recording time cut out at the join (0 for a moment that is not a cut).
    pub removed_us: u64,
    pub reason: String,
    pub by: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mark: Option<Mark>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct JoinsView {
    /// Timeline order; changed ones last.
    pub items: Vec<JoinView>,
    pub unheard: usize,
    pub keep: usize,
    pub fix: usize,
    pub changed: usize,
}

// ---------------------------------------------------------------------------------------
// The file.

pub fn load(root: &Path) -> Result<Joins, String> {
    let path = safe_path(root, JOINS_FILE)?;
    if !path.is_file() {
        return Ok(Joins::default());
    }
    let mut bytes = Vec::new();
    open_regular(&path)?
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err("The joins file is too large".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| format!("Invalid joins file: {e}"))
}

pub fn save(root: &Path, joins: &Joins) -> Result<(), String> {
    let dir = safe_path(root, PROPOSALS_DIR)?;
    match fs::symlink_metadata(&dir) {
        Ok(meta) if !meta.is_dir() => return Err("The agent folder is not a folder".into()),
        Ok(_) => (),
        Err(_) => fs::create_dir(&dir).map_err(|e| e.to_string())?,
    }
    let path = safe_path(root, JOINS_FILE)?;
    let bytes = serde_json::to_vec_pretty(joins).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(&dir).map_err(|e| e.to_string())?;
    temp.write_all(&bytes).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(&path).map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------------------------
// Where joins are.

/// The edit's cuts, as `check` counts them: the clock's sound key and its pieces.
struct Cuts {
    clock: Option<String>,
    mapper: crate::timeline::TimelineMapper,
    pieces: Vec<Piece>,
}

impl Cuts {
    fn of(document: &EditDocument) -> Option<Self> {
        let (clock, mapper) = check::edit_clock(document)?;
        let pieces = check::pieces(&mapper);
        Some(Self {
            clock,
            mapper,
            pieces,
        })
    }

    /// The joins: (index of the piece after it).
    fn joins(&self) -> impl Iterator<Item = usize> + '_ {
        (1..self.pieces.len()).filter(|&i| self.pieces[i].cut_before)
    }

    fn at(&self, join: &Join) -> Option<u64> {
        if join.clock != self.clock {
            return None;
        }
        if join.before_us == join.after_us {
            return self.mapper.source_to_edited_us(join.before_us);
        }
        self.joins()
            .find(|&i| {
                self.pieces[i - 1].source_end == join.before_us
                    && self.pieces[i].source_start == join.after_us
            })
            .map(|i| self.pieces[i].edited_start)
    }

    /// The cut nearest edited moment `at_us`, within [`SNAP_US`]; else the moment itself.
    fn anchor(&self, at_us: u64) -> Result<(u64, u64), String> {
        let nearest = self
            .joins()
            .min_by_key(|&i| self.pieces[i].edited_start.abs_diff(at_us))
            .filter(|&i| self.pieces[i].edited_start.abs_diff(at_us) <= SNAP_US);
        if let Some(i) = nearest {
            return Ok((self.pieces[i - 1].source_end, self.pieces[i].source_start));
        }
        let source = self
            .mapper
            .edited_to_source_us(at_us)
            .ok_or_else(|| format!("Nothing of the recording plays at {}.", clock(at_us)))?;
        Ok((source, source))
    }
}

fn clock(us: u64) -> String {
    crate::mcp::tools::clock(us)
}

fn removed(join: &Join) -> u64 {
    join.after_us.abs_diff(join.before_us)
}

pub fn view(document: &EditDocument, joins: &Joins) -> JoinsView {
    let cuts = Cuts::of(document);
    let mut out = JoinsView::default();
    for join in &joins.items {
        let at_us = cuts.as_ref().and_then(|c| c.at(join));
        match (at_us, join.mark) {
            (None, _) => out.changed += 1,
            (_, None) => out.unheard += 1,
            (_, Some(Mark::Keep)) => out.keep += 1,
            (_, Some(Mark::Fix)) => out.fix += 1,
        }
        out.items.push(JoinView {
            id: join.id.clone(),
            at_us,
            removed_us: removed(join),
            reason: join.reason.clone(),
            by: join.by.clone(),
            mark: join.mark,
            note: join.note.clone(),
        });
    }
    out.items
        .sort_by_key(|j| (j.at_us.is_none(), j.at_us.unwrap_or(0)));
    out
}

fn trimmed(text: &str, max: usize) -> String {
    text.trim().chars().take(max).collect()
}

/// Adds joins at edited moments (each snapped to the cut near it), or gives a known one the
/// new reason. Returns how many were new.
pub fn add(
    document: &EditDocument,
    joins: &mut Joins,
    by: &str,
    moments: &[(u64, String)],
) -> Result<usize, String> {
    let cuts = Cuts::of(document).ok_or("The edit has no recording to listen to.")?;
    let mut new = 0;
    for (at_us, reason) in moments {
        let (before_us, after_us) = cuts.anchor(*at_us)?;
        let reason = trimmed(reason, MAX_REASON_CHARS);
        if let Some(known) = joins
            .items
            .iter_mut()
            .find(|j| j.clock == cuts.clock && j.before_us == before_us && j.after_us == after_us)
        {
            if !reason.is_empty() && !known.reason.contains(&reason) {
                known.reason = trimmed(&format!("{}; {reason}", known.reason), MAX_REASON_CHARS);
            }
            continue;
        }
        if joins.items.len() >= MAX_JOINS {
            return Err(format!(
                "The list holds {MAX_JOINS} joins; clear it before adding more."
            ));
        }
        joins.next_id += 1;
        joins.items.push(Join {
            id: format!("j-{}", joins.next_id),
            clock: cuts.clock.clone(),
            before_us,
            after_us,
            reason,
            by: by.to_string(),
            mark: None,
            note: None,
        });
        new += 1;
    }
    Ok(new)
}

/// The user's mark on join `id` (none takes it back), with a note.
pub fn mark(
    joins: &mut Joins,
    id: &str,
    mark: Option<Mark>,
    note: Option<&str>,
) -> Result<(), String> {
    let join = joins
        .items
        .iter_mut()
        .find(|j| j.id == id)
        .ok_or_else(|| format!("No join {id}"))?;
    join.mark = mark;
    if let Some(note) = note {
        let note = trimmed(note, MAX_NOTE_CHARS);
        join.note = (!note.is_empty()).then_some(note);
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------
// The cuts most likely to sound wrong.

/// Up to `max` cuts most likely to sound wrong, in timeline order: (edited time, why).
/// Long cuts (retakes), words cut into or ending right at a cut, words the transcriber was
/// unsure of next to one, cuts mid-sentence, and `check`'s blips and clipped words.
pub fn risky(root: &Path, document: &EditDocument, max: usize) -> Vec<(u64, String)> {
    let Some(cuts) = Cuts::of(document) else {
        return Vec::new();
    };
    let words: Vec<TranscriptWord> = cuts
        .clock
        .as_deref()
        .and_then(|key| store::load_transcript(root, key).ok().flatten())
        .map(|t| {
            t.words
                .into_iter()
                .filter(|w| w.kind == WordKind::Word)
                .collect()
        })
        .unwrap_or_default();
    let punctuated = words.iter().any(|w| ends_sentence(&w.text));
    let findings: Vec<(u64, u64, String)> = check::run(root, document, &[])
        .findings
        .into_iter()
        .filter(|f| matches!(f.kind, Kind::ClippedWord | Kind::Fragment))
        .filter_map(|f| Some((f.start_us?, f.end_us?, f.title)))
        .collect();

    let mut scored: Vec<(u32, u64, String)> = Vec::new();
    for i in cuts.joins() {
        let (prev, next) = (&cuts.pieces[i - 1], &cuts.pieces[i]);
        let mut score = 0;
        let mut why: Vec<String> = Vec::new();
        let mut say = |points: u32, text: String| {
            score += points;
            why.push(text);
        };
        // A retake takes words out; a long pause cut takes none and is no risk.
        let removed = words
            .iter()
            .filter(|w| {
                let mid = w.source_start_us + (w.source_end_us - w.source_start_us) / 2;
                prev.source_end < mid && mid < next.source_start
            })
            .count();
        if next.source_start < prev.source_end {
            say(2, "jumps back in the recording".into());
        } else if removed >= RETAKE_MIN_WORDS {
            say(2, format!("{removed} words cut out (a retake?)"));
        } else if words.is_empty() && next.source_start >= prev.source_end + LONG_CUT_US {
            // No transcript to tell: a long cut may be a retake.
            say(
                2,
                format!(
                    "{:.1} s cut out (a retake?)",
                    (next.source_start - prev.source_end) as f64 / 1e6
                ),
            );
        }
        let quoted = |w: &TranscriptWord| format!("\"{}\"", w.text.trim());
        // The words on each side of the cut.
        let cut_into = |t: u64| {
            words
                .iter()
                .find(|w| w.source_start_us < t && t < w.source_end_us)
        };
        if let Some(w) = cut_into(prev.source_end) {
            say(3, format!("cuts into {}", quoted(w)));
        }
        if let Some(w) = cut_into(next.source_start) {
            say(3, format!("cuts into {}", quoted(w)));
        }
        let last = words
            .iter()
            .filter(|w| w.source_end_us <= prev.source_end && w.source_end_us > prev.source_start)
            .last();
        let first = words.iter().find(|w| {
            w.source_start_us >= next.source_start && w.source_start_us < next.source_end
        });
        if let Some(w) = last.filter(|w| prev.source_end - w.source_end_us < TIGHT_US) {
            say(2, format!("{} ends right at the cut", quoted(w)));
        }
        if let Some(w) = first.filter(|w| w.source_start_us - next.source_start < TIGHT_US) {
            say(2, format!("{} starts right at the cut", quoted(w)));
        }
        for w in [last, first].into_iter().flatten() {
            if w.confidence.is_some_and(|c| c < UNSURE) {
                say(1, format!("{} was heard unsure", quoted(w)));
            }
        }
        if let (true, Some(l), Some(f)) = (punctuated, last, first) {
            let lower = f
                .text
                .trim()
                .chars()
                .next()
                .is_some_and(|c| c.is_lowercase());
            if !ends_sentence(&l.text) && lower {
                say(1, "cut mid-sentence".into());
            }
        }
        let near =
            prev.edited_end.saturating_sub(FINDING_NEAR_US)..=next.edited_start + FINDING_NEAR_US;
        for (start, end, title) in &findings {
            if near.contains(start) || near.contains(end) {
                say(3, title.to_lowercase());
            }
        }
        if score >= 2 {
            scored.push((score, next.edited_start, why.join("; ")));
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    scored.truncate(max);
    let mut out: Vec<(u64, String)> = scored.into_iter().map(|(_, at, why)| (at, why)).collect();
    out.sort_by_key(|(at, _)| *at);
    out
}

fn ends_sentence(text: &str) -> bool {
    text.trim_end().ends_with(['.', '?', '!', ':', ';'])
}

// ---------------------------------------------------------------------------------------
// For the windows and agents.

pub fn view_for(reader: &ProjectReader) -> Result<JoinsView, String> {
    Ok(view(&reader.history().current, &load(reader.root())?))
}

/// Loads the list, changes it and saves it; then the list as it is.
pub fn change(
    reader: &ProjectReader,
    edit: impl FnOnce(&EditDocument, &mut Joins) -> Result<(), String>,
) -> Result<JoinsView, String> {
    let root = reader.root();
    let document = &reader.history().current;
    let mut joins = load(root)?;
    edit(document, &mut joins)?;
    save(root, &joins)?;
    Ok(view(document, &joins))
}

/// The list as lines, for agents and the CLI.
pub fn text(view: &JoinsView) -> String {
    if view.items.is_empty() {
        return "No joins to listen to. `find` adds the cuts most likely to sound wrong; `add` \
                adds yours."
            .into();
    }
    let mut out = format!(
        "{} joins: {} not heard yet, {} keep, {} fix, {} changed by an edit since.\n",
        view.items.len(),
        view.unheard,
        view.keep,
        view.fix,
        view.changed
    );
    for j in &view.items {
        let at = j.at_us.map(clock).unwrap_or_else(|| "changed".into());
        let mark = match j.mark {
            Some(Mark::Keep) => "keep",
            Some(Mark::Fix) => "FIX",
            None => "-",
        };
        out.push_str(&format!("{} {at} [{mark}] {}", j.id, j.reason));
        if let Some(note) = &j.note {
            out.push_str(&format!(" | user: {note}"));
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::{SourceInterval, TimelineMapper};

    const S: u64 = 1_000_000;

    fn cuts(pairs: &[(u64, u64)]) -> Cuts {
        let mapper = TimelineMapper::new(
            pairs
                .iter()
                .enumerate()
                .map(|(i, &(a, b))| SourceInterval::new(format!("i{i}"), a, b))
                .collect(),
        );
        Cuts {
            clock: Some("a.audio".into()),
            pieces: check::pieces(&mapper),
            mapper,
        }
    }

    fn join(before_us: u64, after_us: u64) -> Join {
        Join {
            id: "j-1".into(),
            clock: Some("a.audio".into()),
            before_us,
            after_us,
            reason: String::new(),
            by: "test".into(),
            mark: None,
            note: None,
        }
    }

    #[test]
    fn a_moment_snaps_to_the_cut_near_it() {
        // Kept 0-4 s and 10-20 s: one cut, at 4 s on the edited timeline.
        let cuts = cuts(&[(0, 4 * S), (10 * S, 20 * S)]);
        assert_eq!(cuts.anchor(4 * S + 900_000).unwrap(), (4 * S, 10 * S));
        // Too far from it: the moment itself, as recording time.
        assert_eq!(cuts.anchor(9 * S).unwrap(), (15 * S, 15 * S));
    }

    #[test]
    fn a_join_follows_later_edits_and_is_gone_when_changed() {
        let j = join(4 * S, 10 * S);
        // An earlier cut moves it 1 s sooner.
        let later = cuts(&[(0, 1 * S), (2 * S, 4 * S), (10 * S, 20 * S)]);
        assert_eq!(later.at(&j), Some(3 * S));
        // The fix restores part of what was cut: the join is a different one.
        let fixed = cuts(&[(0, 5 * S), (10 * S, 20 * S)]);
        assert_eq!(fixed.at(&j), None);
    }
}
