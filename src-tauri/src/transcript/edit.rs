//! Edits derived from a transcript. Everything here returns cuts in edited time for the
//! current revision, so they go through the same ripple-cut path as silence and manual cuts.
use super::{Transcript, TranscriptWord, WordKind};
use crate::project::revision::MAX_CUTS_PER_REVISION;
use crate::timeline::TimelineMapper;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Filler sounds cut by "Remove filler words". Real words such as "like" or "so" are left
/// alone: whether they are filler depends on the sentence.
const FILLERS: &[&str] = &[
    "um", "umm", "ummm", "uh", "uhh", "uhm", "er", "erm", "ah", "ahh", "hmm", "hm", "mm", "mmm",
];
/// Gaps at least this long end a phrase for retake detection.
const PHRASE_PAUSE_US: u64 = 400_000;
/// A restart is preceded by at least this much pause, or by punctuation.
const RESTART_PAUSE_US: u64 = 250_000;
/// A retake repeats at least this many words of the abandoned attempt.
const MIN_RETAKE_WORDS: usize = 3;
/// Only look this far back for the abandoned attempt.
const MAX_RETAKE_SPAN_WORDS: usize = 40;
const MAX_RETAKE_SPAN_US: u64 = 30_000_000;
/// Padding at the very start or end of the transcript, where there is no neighbour word.
const EDGE_PAD_US: u64 = 50_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptViewWord {
    #[serde(flatten)]
    pub word: TranscriptWord,
    /// Edited-time position, or `None` when the word has been cut.
    pub edited_start_us: Option<u64>,
    pub edited_end_us: Option<u64>,
    /// Not shown in the captions (hidden on the caption track).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub caption_hidden: bool,
    /// A caption starts at this word (split on the caption track).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub caption_break: bool,
    /// Kept in the caption before it (merged on the caption track).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub caption_join: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptView {
    pub track_id: String,
    pub provider: super::ProviderKind,
    pub model: String,
    pub language: Option<String>,
    pub created_at: String,
    pub revision: u64,
    pub words: Vec<TranscriptViewWord>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TranscriptSuggestionKind {
    Filler,
    Retake,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptCutSuggestion {
    /// Stable while the same words are suggested: `<kind>-<first word>-<last word>`.
    pub id: String,
    pub kind: TranscriptSuggestionKind,
    pub word_ids: Vec<String>,
    pub text: String,
    pub edited_start_us: u64,
    pub edited_end_us: u64,
    /// The user rejected this suggestion; it stays out of "remove all".
    #[serde(default)]
    pub dismissed: bool,
    /// Found by the built-in rules or by an AI pass.
    #[serde(default)]
    pub source: SuggestionSource,
    /// The AI's short explanation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SuggestionSource {
    #[default]
    Rules,
    Ai,
}

fn normalize(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric() || *c == '\'')
        .flat_map(char::to_lowercase)
        .collect()
}

fn ends_clause(text: &str) -> bool {
    text.ends_with(['.', '?', '!', ',', ';', ':', '-', '\u{2014}', '\u{2026}'])
}

/// A word counts as kept when its midpoint survives the current cuts.
pub(crate) fn kept(word: &TranscriptWord, mapper: &TimelineMapper) -> bool {
    let mid = word.source_start_us + (word.source_end_us - word.source_start_us) / 2;
    mapper.source_to_edited_us(mid).is_some()
}

pub fn view(transcript: &Transcript, mapper: &TimelineMapper, revision: u64) -> TranscriptView {
    let mut words: Vec<TranscriptViewWord> = transcript
        .words
        .iter()
        .map(|word| {
            let span = mapper.edited_span_of(word.source_start_us, word.source_end_us);
            let mark = transcript.caption_mark(&word.id);
            TranscriptViewWord {
                word: word.clone(),
                edited_start_us: span.map(|s| s.0),
                edited_end_us: span.map(|s| s.1),
                caption_hidden: mark.hidden,
                caption_break: mark.cue_break,
                caption_join: mark.cue_join,
            }
        })
        .collect();
    // Read in playback order: kept words by edited time, and each cut word right after the
    // kept word before it in the recording, so it still shows where it was removed.
    let mut anchor = 0u64;
    let mut keys: Vec<(u64, usize)> = Vec::with_capacity(words.len());
    for (index, word) in words.iter().enumerate() {
        if let Some(start) = word.edited_start_us {
            anchor = start;
        }
        keys.push((anchor, index));
    }
    keys.sort_by_key(|&(key, index)| (key, words[index].edited_start_us.is_none(), index));
    let order: Vec<usize> = keys.into_iter().map(|(_, index)| index).collect();
    let mut taken: Vec<Option<TranscriptViewWord>> = words.drain(..).map(Some).collect();
    let words = order
        .into_iter()
        .filter_map(|index| taken[index].take())
        .collect();
    TranscriptView {
        track_id: transcript.track_id.clone(),
        provider: transcript.provider,
        model: transcript.model.clone(),
        language: transcript.language.clone(),
        created_at: transcript.created_at.clone(),
        revision,
        words,
    }
}

/// Edited-time cuts that remove the words in `word_ids`. Each run of removed words takes
/// half of the pause on either side with it, so the speech around it keeps a natural gap.
pub fn word_cuts(
    transcript: &Transcript,
    word_ids: &[String],
    mapper: &TimelineMapper,
) -> Result<Vec<(u64, u64)>, String> {
    let wanted: HashSet<&str> = word_ids.iter().map(String::as_str).collect();
    if wanted.is_empty() {
        return Err("No words selected".into());
    }
    let live: Vec<&TranscriptWord> = transcript
        .words
        .iter()
        .filter(|w| kept(w, mapper))
        .collect();
    let mut source_ranges = Vec::new();
    let mut i = 0;
    while i < live.len() {
        if !wanted.contains(live[i].id.as_str()) {
            i += 1;
            continue;
        }
        let first = i;
        while i < live.len() && wanted.contains(live[i].id.as_str()) {
            i += 1;
        }
        let start = live[first].source_start_us;
        let end = live[i - 1]
            .source_end_us
            .max(live[i - 1].source_start_us + 1);
        let cut_start = match first.checked_sub(1).map(|p| live[p].source_end_us) {
            Some(prev) if prev < start => prev + (start - prev) / 2,
            Some(_) => start,
            None => start.saturating_sub(EDGE_PAD_US),
        };
        let cut_end = match live.get(i).map(|n| n.source_start_us) {
            Some(next) if next > end => end + (next - end) / 2,
            Some(_) => end,
            None => end + EDGE_PAD_US,
        };
        source_ranges.push((cut_start, cut_end));
    }
    if source_ranges.is_empty() {
        return Err("Those words are already cut".into());
    }
    // Reordered clips can put later source time earlier on the timeline: sort, then merge.
    let mut edited: Vec<(u64, u64)> = source_ranges
        .into_iter()
        .flat_map(|(start, end)| mapper.source_range_to_edited(start, end))
        .filter(|(a, b)| b > a)
        .collect();
    edited.sort_unstable();
    let mut cuts: Vec<(u64, u64)> = Vec::new();
    for (a, b) in edited {
        match cuts.last_mut() {
            Some(last) if a <= last.1 => last.1 = last.1.max(b),
            _ => cuts.push((a, b)),
        }
    }
    if cuts.is_empty() {
        return Err("Those words are already cut".into());
    }
    if cuts.len() > MAX_CUTS_PER_REVISION {
        return Err(format!(
            "That would make {} cuts; select fewer words (up to {MAX_CUTS_PER_REVISION} separate runs at once)",
            cuts.len()
        ));
    }
    Ok(cuts)
}

pub fn suggestions(
    transcript: &Transcript,
    mapper: &TimelineMapper,
) -> Vec<TranscriptCutSuggestion> {
    let live: Vec<&TranscriptWord> = transcript
        .words
        .iter()
        .filter(|w| w.kind == WordKind::Word && kept(w, mapper))
        .collect();
    let normalized: Vec<String> = live.iter().map(|w| normalize(&w.text)).collect();
    let mut out = Vec::new();

    // Runs of filler sounds.
    let mut i = 0;
    while i < live.len() {
        if !FILLERS.contains(&normalized[i].as_str()) {
            i += 1;
            continue;
        }
        let first = i;
        while i < live.len() && FILLERS.contains(&normalized[i].as_str()) {
            i += 1;
        }
        push_suggestion(
            &mut out,
            TranscriptSuggestionKind::Filler,
            &live[first..i],
            transcript,
            mapper,
        );
    }

    // Retakes: a phrase restarted with the same opening words. The earlier, abandoned
    // attempt is suggested for removal and the later take is kept.
    let phrase_start = |k: usize| {
        k == 0
            || ends_clause(&live[k - 1].text)
            || live[k]
                .source_start_us
                .saturating_sub(live[k - 1].source_end_us)
                >= PHRASE_PAUSE_US
    };
    let restart = |k: usize| {
        k > 0
            && (ends_clause(&live[k - 1].text)
                || live[k]
                    .source_start_us
                    .saturating_sub(live[k - 1].source_end_us)
                    >= RESTART_PAUSE_US)
    };
    let mut covered_until = 0usize;
    for b in 1..live.len() {
        if !restart(b) {
            continue;
        }
        let lowest = b.saturating_sub(MAX_RETAKE_SPAN_WORDS).max(covered_until);
        let mut found = None;
        for a in lowest..b {
            if !phrase_start(a)
                || live[b]
                    .source_start_us
                    .saturating_sub(live[a].source_start_us)
                    > MAX_RETAKE_SPAN_US
            {
                continue;
            }
            let matched = (0..)
                .take_while(|&k| {
                    a + k < b
                        && b + k < live.len()
                        && !normalized[a + k].is_empty()
                        && normalized[a + k] == normalized[b + k]
                })
                .count();
            // The first attempt must be repeated from its first word, by at least three words
            // or entirely if it was shorter (e.g. "So the. So the next step").
            if matched >= MIN_RETAKE_WORDS || (matched >= 2 && matched == b - a) {
                found = Some(a);
                break;
            }
        }
        if let Some(a) = found {
            push_suggestion(
                &mut out,
                TranscriptSuggestionKind::Retake,
                &live[a..b],
                transcript,
                mapper,
            );
            covered_until = b;
        }
    }

    // AI spans whose words are all still kept, skipping any that overlap a rule-based one.
    let position: std::collections::HashMap<&str, usize> = live
        .iter()
        .enumerate()
        .map(|(i, w)| (w.id.as_str(), i))
        .collect();
    let mut taken: Vec<(usize, usize)> = Vec::new();
    for s in &out {
        if let (Some(&a), Some(&b)) = (
            s.word_ids.first().and_then(|id| position.get(id.as_str())),
            s.word_ids.last().and_then(|id| position.get(id.as_str())),
        ) {
            taken.push((a, b));
        }
    }
    for span in &transcript.ai_suggestions {
        let (Some(&a), Some(&b)) = (
            position.get(span.first_word_id.as_str()),
            position.get(span.last_word_id.as_str()),
        ) else {
            continue;
        };
        if a > b || taken.iter().any(|&(x, y)| a <= y && x <= b) {
            continue;
        }
        let before = out.len();
        push_suggestion(&mut out, span.kind, &live[a..=b], transcript, mapper);
        if let Some(added) = out.get_mut(before) {
            added.source = SuggestionSource::Ai;
            added.reason = Some(span.reason.clone()).filter(|r| !r.is_empty());
            taken.push((a, b));
        }
    }

    out.sort_by_key(|s| s.edited_start_us);
    for s in out.iter_mut() {
        s.id = format!(
            "{}-{}-{}",
            match s.kind {
                TranscriptSuggestionKind::Filler => "filler",
                TranscriptSuggestionKind::Retake => "retake",
            },
            s.word_ids.first().map(String::as_str).unwrap_or(""),
            s.word_ids.last().map(String::as_str).unwrap_or("")
        );
        s.dismissed = transcript.dismissed_suggestions.contains(&s.id);
    }
    out
}

fn push_suggestion(
    out: &mut Vec<TranscriptCutSuggestion>,
    kind: TranscriptSuggestionKind,
    words: &[&TranscriptWord],
    transcript: &Transcript,
    mapper: &TimelineMapper,
) {
    let ids: Vec<String> = words.iter().map(|w| w.id.clone()).collect();
    let Ok(cuts) = word_cuts(transcript, &ids, mapper) else {
        return;
    };
    let (Some(first), Some(last)) = (cuts.first(), cuts.last()) else {
        return;
    };
    out.push(TranscriptCutSuggestion {
        id: String::new(),
        kind,
        text: words
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        word_ids: ids,
        edited_start_us: first.0,
        edited_end_us: last.1,
        dismissed: false,
        source: SuggestionSource::Rules,
        reason: None,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::SourceInterval;
    use crate::transcript::{test_word, ProviderKind};

    fn transcript(words: &[(&str, u64, u64)]) -> Transcript {
        Transcript::new(
            "mic".into(),
            ProviderKind::ElevenLabs,
            "scribe_v2".into(),
            None,
            words.iter().map(|(t, s, e)| test_word(t, *s, *e)).collect(),
        )
    }

    fn mapper(ranges: &[(u64, u64)]) -> TimelineMapper {
        TimelineMapper::try_new(
            ranges
                .iter()
                .enumerate()
                .map(|(i, (s, e))| SourceInterval::new(format!("r{i}"), s * 1000, e * 1000))
                .collect(),
        )
        .unwrap()
    }

    #[test]
    fn reordered_clips_list_words_in_playback_order() {
        let t = transcript(&[
            ("one", 0, 400),
            ("two", 600, 900),
            ("cut", 1100, 1300),
            ("three", 1500, 1900),
        ]);
        // Source 1500-2000 plays first, then 0-1000; "cut" (1000-1500) is removed.
        let m = mapper(&[(1500, 2000), (0, 1000)]);
        let v = view(&t, &m, 1);
        let order: Vec<_> = v.words.iter().map(|w| w.word.text.as_str()).collect();
        // The removed word stays right after the word before it in the recording.
        assert_eq!(order, vec!["three", "one", "two", "cut"]);
        assert_eq!(v.words[0].edited_start_us, Some(0));
        assert_eq!(v.words[1].edited_start_us, Some(500_000));
        assert_eq!(v.words[2].edited_start_us, Some(1_100_000));
        assert!(v.words[3].edited_start_us.is_none());
        // Deleting "three" and "one" yields two sorted, separate edited cuts.
        let cuts = word_cuts(&t, &["w-3".into(), "w-0".into()], &m).unwrap();
        assert!(cuts.windows(2).all(|w| w[0].1 <= w[1].0), "{cuts:?}");
        assert_eq!(cuts.len(), 2);
    }

    #[test]
    fn deleting_a_word_takes_half_of_each_pause() {
        let t = transcript(&[("hello", 0, 400), ("um", 600, 800), ("world", 1000, 1400)]);
        let m = mapper(&[(0, 2000)]);
        let cuts = word_cuts(&t, &["w-1".into()], &m).unwrap();
        assert_eq!(cuts, vec![(500_000, 900_000)]);
    }

    #[test]
    fn cuts_map_into_edited_time_and_skip_cut_words() {
        let t = transcript(&[
            ("a", 0, 400),
            ("b", 600, 800),
            ("c", 1000, 1400),
            ("d", 1600, 1800),
        ]);
        // "b" is already cut: source 500-900 removed.
        let m = mapper(&[(0, 500), (900, 2000)]);
        let v = view(&t, &m, 3);
        assert!(v.words[1].edited_start_us.is_none());
        assert_eq!(v.words[2].edited_start_us, Some(600_000));
        // Deleting "c" uses "a" and "d" as its neighbours.
        let cuts = word_cuts(&t, &["w-2".into()], &m).unwrap();
        // Source 700-1500 minus the removed 500-900 becomes edited 500-1100.
        assert_eq!(cuts, vec![(500_000, 1_100_000)]);
        assert!(word_cuts(&t, &["w-1".into()], &m).is_err());
    }

    #[test]
    fn adjacent_runs_merge_and_edges_get_padding() {
        let t = transcript(&[("a", 100, 400), ("b", 500, 800), ("c", 900, 1200)]);
        let m = mapper(&[(0, 2000)]);
        let cuts = word_cuts(&t, &["w-0".into(), "w-1".into()], &m).unwrap();
        assert_eq!(cuts, vec![(50_000, 850_000)]);
        let cuts = word_cuts(&t, &["w-2".into()], &m).unwrap();
        assert_eq!(cuts, vec![(850_000, 1_250_000)]);
    }

    #[test]
    fn finds_fillers_and_retakes() {
        let t = transcript(&[
            ("So", 0, 200),
            ("um,", 300, 500),
            ("today", 600, 900),
            ("we", 950, 1000),
            ("build", 1050, 1300),
            // pause, then a restart of the same phrase
            ("Today", 2000, 2300),
            ("we", 2350, 2400),
            ("build", 2450, 2700),
            ("a", 2750, 2800),
            ("farm.", 2850, 3200),
        ]);
        let m = mapper(&[(0, 4000)]);
        let s = suggestions(&t, &m);
        assert_eq!(s.len(), 2, "{s:?}");
        assert_eq!(s[0].kind, TranscriptSuggestionKind::Filler);
        assert_eq!(s[0].word_ids, vec!["w-1"]);
        assert_eq!(s[1].kind, TranscriptSuggestionKind::Retake);
        assert_eq!(s[1].word_ids, vec!["w-2", "w-3", "w-4"]);
        assert_eq!(s[1].text, "today we build");
        assert_eq!(s[0].id, "filler-w-1-w-1");
        assert_eq!(s[1].id, "retake-w-2-w-4");
        assert!(!s[0].dismissed);

        // A rejected suggestion keeps its id and comes back marked.
        let mut t = t;
        t.set_dismissed(&[s[0].id.clone()], true).unwrap();
        let again = suggestions(&t, &m);
        assert!(again[0].dismissed);
        assert!(!again[1].dismissed);
    }

    #[test]
    fn ai_spans_join_the_rules_without_duplicates() {
        use crate::transcript::AiSpan;
        let mut t = transcript(&[
            ("So", 0, 200),
            ("um,", 300, 500),
            ("like,", 600, 800),
            ("this", 900, 1100),
            ("is", 1150, 1300),
            ("the", 1350, 1500),
            ("map.", 1550, 1900),
        ]);
        let span = |kind, a: &str, b: &str, reason: &str| AiSpan {
            kind,
            first_word_id: a.into(),
            last_word_id: b.into(),
            reason: reason.into(),
        };
        t.ai_suggestions = vec![
            // Overlaps the rule-based "um," filler: skipped.
            span(TranscriptSuggestionKind::Filler, "w-1", "w-2", "um like"),
            // New: "like," on its own is not in the rules.
            span(
                TranscriptSuggestionKind::Filler,
                "w-2",
                "w-2",
                "filler like",
            ),
            // Unknown word ids are ignored.
            span(TranscriptSuggestionKind::Retake, "w-90", "w-91", ""),
        ];
        let m = mapper(&[(0, 4000)]);
        let s = suggestions(&t, &m);
        assert_eq!(s.len(), 2, "{s:?}");
        assert_eq!(s[0].id, "filler-w-1-w-1");
        assert_eq!(s[0].source, SuggestionSource::Rules);
        assert_eq!(s[1].id, "filler-w-2-w-2");
        assert_eq!(s[1].source, SuggestionSource::Ai);
        assert_eq!(s[1].reason.as_deref(), Some("filler like"));

        // Rejections work by id for AI suggestions too.
        t.set_dismissed(&[s[1].id.clone()], true).unwrap();
        assert!(suggestions(&t, &m)[1].dismissed);

        // Once "like," is cut, its AI suggestion goes away.
        let m = mapper(&[(0, 550), (850, 4000)]);
        let s = suggestions(&t, &m);
        assert!(
            s.iter().all(|s| s.source == SuggestionSource::Rules),
            "{s:?}"
        );

        // Stored spans survive a save and load.
        let json = serde_json::to_string(&t).unwrap();
        let back: Transcript = serde_json::from_str(&json).unwrap();
        assert_eq!(back.ai_suggestions, t.ai_suggestions);
        back.validate().unwrap();
    }

    #[test]
    fn repeated_phrases_without_a_restart_are_not_retakes() {
        let t = transcript(&[
            ("one", 0, 200),
            ("by", 220, 300),
            ("one", 320, 500),
            ("by", 520, 600),
            ("one", 620, 800),
        ]);
        let m = mapper(&[(0, 1000)]);
        assert!(suggestions(&t, &m).is_empty());
    }
}
