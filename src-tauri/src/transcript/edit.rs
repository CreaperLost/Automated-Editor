//! Edits derived from a transcript. Everything here returns cuts in edited time for the
//! current revision, so they go through the same ripple-cut path as silence and manual cuts.
use super::{Transcript, TranscriptWord, WordKind};
use crate::timeline::TimelineMapper;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Filler sounds cut by "Remove filler words", with repeated letters folded so every spelling
/// a transcriber writes counts ("ummm", "uhh", "errm", "hmmm"). Real words such as "like" or
/// "so" are left alone: whether they are filler depends on the sentence. Greek ones (εμ, χμ)
/// sit in the same list: no English word is spelled in Greek letters.
const FILLERS: &[&str] = &[
    "um", "uh", "uhm", "er", "erm", "ehm", "ah", "hm", "εμ", "χμ", "αχμ",
];
/// Sounds that fold to one letter, so count only when drawn out ("mm", "εεε", "μμ"): "ε" or
/// "α" alone can be a word.
const DRAWN_OUT: &[&str] = &["m", "ε", "α", "μ"];

/// A filler sound such as um or uh; `normalized` as `normalize` gives it.
pub(crate) fn is_filler_sound(normalized: &str) -> bool {
    let mut folded = String::with_capacity(normalized.len());
    for c in normalized.chars() {
        if !folded.ends_with(c) {
            folded.push(c);
        }
    }
    FILLERS.contains(&folded.as_str())
        || (DRAWN_OUT.contains(&folded.as_str()) && normalized.chars().count() >= 2)
}

/// Gaps at least this long end a phrase for retake detection.
const PHRASE_PAUSE_US: u64 = 400_000;
/// A restart is preceded by at least this much pause, or by punctuation.
const RESTART_PAUSE_US: u64 = 250_000;
/// A retake repeats at least this many words of the abandoned attempt.
const MIN_RETAKE_WORDS: usize = 3;
/// Only look this far back for the abandoned attempt: a retake follows its false start
/// closely, and a long first attempt is a sentence of its own, not a false start.
const MAX_RETAKE_SPAN_WORDS: usize = 16;
const MAX_RETAKE_SPAN_US: u64 = 12_000_000;
/// A finished first attempt (it ends a sentence) counts only if it is little more than the
/// repeated words: at most this many words beyond them.
const MAX_FINISHED_EXTRA_WORDS: usize = 2;
/// Words a sentence broken off often ends on ("open the."), as `normalize` gives them: a
/// sentence ending on one is not taken as finished on purpose.
const CUT_OFF_ENDINGS: &[&str] = &[
    "the", "a", "an", "to", "of", "and", "or", "but", "so", "in", "on", "at", "for", "with",
    "from", "by", "into", "is", "are", "was", "were", "be", "this", "that", "these", "those", "my",
    "your", "our", "their", "its", "his", "her", "we", "you", "they", "i", "it",
];

/// A sentence ending on `normalized` can be a finished one (not a word it breaks off on).
pub fn can_end_sentence(normalized: &str) -> bool {
    !CUT_OFF_ENDINGS.contains(&normalized)
}

/// Two normalized words are the same said again, one drawn out or not: a letter held three
/// times or more counts once ("γιααα" is "για", "sooo" is "so").
pub fn same_word(a: &str, b: &str) -> bool {
    fn held(word: &str) -> String {
        let chars: Vec<char> = word.chars().collect();
        let mut out = String::new();
        let mut i = 0;
        while i < chars.len() {
            let run = chars[i..].iter().take_while(|&&c| c == chars[i]).count();
            for _ in 0..if run >= 3 { 1 } else { run } {
                out.push(chars[i]);
            }
            i += run;
        }
        out
    }
    a == b || held(a) == held(b)
}

/// Padding at the very start or end of the transcript, where there is no neighbour word.
const EDGE_PAD_US: u64 = 50_000;
/// Words that can be meant said twice alone ("that that", "had had") but, with a small word,
/// say a pair again only as a restart ("that the, that the", "what is, what is").
const PAIR_WORDS: &[&str] = &["that", "had", "what", "how", "where", "when", "who", "why"];

/// Small words that said twice in a row are a stutter ("we we", "and and", "it's a it's a").
/// Words said twice for weight ("very very", "pretty pretty", "really really") are meant, so
/// only these count; "that that" and "had had" are left out, as both can be meant. Written as
/// `normalize` gives them (Greek without accents).
const STUTTER_WORDS: &[&str] = &[
    "i",
    "i'm",
    "i've",
    "i'll",
    "i'd",
    "we",
    "we're",
    "we've",
    "we'll",
    "you",
    "you're",
    "you've",
    "you'll",
    "they",
    "they're",
    "he",
    "she",
    "it",
    "it's",
    "its",
    "that's",
    "there's",
    "what's",
    "let's",
    "this",
    "these",
    "those",
    "the",
    "a",
    "an",
    "and",
    "but",
    "or",
    "to",
    "of",
    "in",
    "on",
    "at",
    "for",
    "with",
    "from",
    "by",
    "as",
    "if",
    "is",
    "are",
    "was",
    "were",
    "be",
    "can",
    "will",
    "would",
    "could",
    "should",
    "do",
    "does",
    "did",
    "don't",
    "have",
    "has",
    "not",
    "my",
    "your",
    "our",
    "their",
    "his",
    "her",
    "και",
    "να",
    "θα",
    "το",
    "τα",
    "τον",
    "την",
    "της",
    "του",
    "των",
    "ο",
    "η",
    "οι",
    "σε",
    "με",
    "για",
    "απο",
    "που",
    "δεν",
    "ειναι",
];

/// The sentence ends after `text` (". ? !"), not just a clause.
fn ends_sentence(text: &str) -> bool {
    text.trim_end()
        .ends_with(['.', '?', '!', '\u{2026}', '\u{037e}'])
}

/// A word broken off and said again whole: written with one trailing hyphen ("j-", "t-",
/// "inst-"), and either a letter or two or the start of the next word. A double dash ("like--")
/// breaks off a sentence, not a word.
fn broken_off(text: &str, normalized: &str, next: &str) -> bool {
    let text = text.trim_end();
    text.ends_with('-')
        && !text.ends_with("--")
        && !normalized.is_empty()
        && (normalized.chars().count() <= 2 || next.starts_with(normalized))
}

/// Punctuation on its own, as some providers give it ("." or ",").
pub(crate) fn is_punctuation(text: &str) -> bool {
    let text = text.trim();
    !text.is_empty() && text.chars().all(|c| !c.is_alphanumeric())
}

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
    pub word_stamp: String,
    pub words: Vec<TranscriptViewWord>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TranscriptSuggestionKind {
    Filler,
    Retake,
    /// Anything else an agent suggests cutting (a tangent, a mistake, dead air).
    Cut,
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
    /// Who made the AI review that found it: `in-house (<agent>)` for the connected agent,
    /// else the OpenRouter or OpenAI model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewer: Option<String>,
    /// An agent's proposal item this is, decided through its batch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposal: Option<ProposalRef>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProposalRef {
    pub batch_id: String,
    pub item_id: String,
    pub agent: String,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SuggestionSource {
    #[default]
    Rules,
    Ai,
    /// An agent connected over MCP proposed it.
    Agent,
}

/// Lowercase letters and digits, with Greek accents folded (λοιπόν and λοιπον match).
pub(crate) fn normalize(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric() || *c == '\'')
        .flat_map(char::to_lowercase)
        .map(super::language::fold_greek_accent)
        .collect()
}

pub(crate) fn ends_clause(text: &str) -> bool {
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
        word_stamp: transcript.dependency().word_stamp.unwrap_or_default(),
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
    Ok(cuts)
}

pub fn suggestions(
    transcript: &Transcript,
    mapper: &TimelineMapper,
) -> Vec<TranscriptCutSuggestion> {
    // Some providers give punctuation as words of its own ("video" "."): fold it into the
    // word before, so phrases and sentence ends read as they were said.
    let mut live: Vec<&TranscriptWord> = Vec::new();
    let mut ends: Vec<bool> = Vec::new();
    let mut stops: Vec<bool> = Vec::new();
    for word in transcript
        .words
        .iter()
        .filter(|w| w.kind == WordKind::Word && kept(w, mapper))
    {
        if is_punctuation(&word.text) {
            if let (Some(end), Some(stop)) = (ends.last_mut(), stops.last_mut()) {
                *end |= ends_clause(&word.text);
                *stop |= ends_sentence(&word.text);
            }
            continue;
        }
        ends.push(ends_clause(&word.text));
        stops.push(ends_sentence(&word.text));
        live.push(word);
    }
    let normalized: Vec<String> = live.iter().map(|w| normalize(&w.text)).collect();
    let mut out = Vec::new();

    // Runs of filler sounds.
    let mut i = 0;
    while i < live.len() {
        if !is_filler_sound(&normalized[i]) {
            i += 1;
            continue;
        }
        let first = i;
        while i < live.len() && is_filler_sound(&normalized[i]) {
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
            || ends[k - 1]
            || live[k]
                .source_start_us
                .saturating_sub(live[k - 1].source_end_us)
                >= PHRASE_PAUSE_US
    };
    let restart = |k: usize| {
        k > 0
            && (ends[k - 1]
                || live[k]
                    .source_start_us
                    .saturating_sub(live[k - 1].source_end_us)
                    >= RESTART_PAUSE_US)
    };
    // The phrase ends after word `k`: punctuation, a pause, or the end of the words.
    let breaks_after = |k: usize| k + 1 >= live.len() || restart(k + 1);
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
                        && same_word(&normalized[a + k], &normalized[b + k])
                })
                .count();
            // The first attempt must be repeated from its first word, by at least three words
            // or entirely if it was shorter (e.g. "So the. So the next step").
            let length = b - a;
            if !(matched >= MIN_RETAKE_WORDS || (matched >= 2 && matched == length)) {
                continue;
            }
            // And it must have been abandoned: cut off before its sentence ended, or (when it
            // did end) barely more than the words said again. Two whole sentences that open
            // alike ("I'm going to ...") are not a retake.
            let finished = ends[b - 1];
            let extra = length - matched;
            let abandoned = if finished {
                extra <= MAX_FINISHED_EXTRA_WORDS
            } else {
                // Cut off, it may go past the repeated words by no more words than were
                // repeated: a long, different continuation is content, not a false start ("you
                // will be able to see exactly how much money you are going to take and / you
                // will be able to withdraw ..." lists what you can do).
                extra <= matched
            };
            // Parallel phrases are not a retake: when the later one swaps the attempt's last
            // words for as many others and stops there too, both were meant ("It will not have
            // a token, it will not have an airdrop."). A false start is either repeated whole
            // or corrected into something longer.
            let parallel = extra > 0 && breaks_after(b + matched + extra - 1);
            // Nor is a finished sentence that ends on the phrase, then a new one opening with
            // it: "this is what we call human in the loop. Human in the loop means...". The
            // phrase doesn't open its own sentence, which ends on a word it can end on.
            let defining = stops[b - 1]
                && extra == 0
                && a > 0
                && !stops[a - 1]
                && can_end_sentence(&normalized[b - 1]);
            if abandoned && !parallel && !defining {
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

    // Stutters: a small word said twice in a row ("we we used", "it's a it's a"), and a word
    // broken off and said whole ("j- just"). The first go is cut.
    let mut stutters: Vec<(usize, usize)> = Vec::new();
    let mut k = 0;
    while k + 1 < live.len() {
        let same = |a: usize, b: usize| normalized.get(b).is_some_and(|n| *n == normalized[a]);
        let small = |k: usize| STUTTER_WORDS.contains(&normalized[k].as_str());
        let in_pair = |k: usize| small(k) || PAIR_WORDS.contains(&normalized[k].as_str());
        // A small word, then itself with a contraction: "they, they've", "we we're".
        let contracted = |k: usize| {
            normalized
                .get(k + 1)
                .and_then(|next| next.strip_prefix(normalized[k].as_str()))
                .is_some_and(|rest| rest.starts_with('\'') && rest.len() > 1)
        };
        // Two words count only if both are small: "one by one by one" is meant.
        let span = if small(k) && !stops[k] && (same(k, k + 1) || contracted(k)) {
            Some((k, k))
        } else if in_pair(k)
            && in_pair(k + 1)
            && !stops[k]
            && !stops[k + 1]
            && same(k, k + 2)
            && same(k + 1, k + 3)
        {
            Some((k, k + 1))
        } else if broken_off(&live[k].text, &normalized[k], &normalized[k + 1]) {
            Some((k, k))
        } else {
            None
        };
        match span {
            Some((a, b)) => {
                stutters.push((a, b));
                k = b + 1;
            }
            None => k += 1,
        }
    }
    let position_of = |id: &str| live.iter().position(|w| w.id == id);
    let retaken: Vec<(usize, usize)> = out
        .iter()
        .filter_map(|s| {
            Some((
                position_of(s.word_ids.first()?)?,
                position_of(s.word_ids.last()?)?,
            ))
        })
        .collect();
    for (a, b) in stutters {
        if retaken.iter().any(|&(x, y)| a <= y && x <= b) {
            continue;
        }
        push_suggestion(
            &mut out,
            TranscriptSuggestionKind::Retake,
            &live[a..=b],
            transcript,
            mapper,
        );
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
            added.reviewer = transcript.ai_model.clone();
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
                TranscriptSuggestionKind::Cut => "cut",
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
        reviewer: None,
        proposal: None,
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
    fn filler_cuts_cover_the_entire_tutorial_beyond_256_ranges() {
        let words: Vec<_> = (0..600)
            .map(|i| {
                (
                    if i % 2 == 0 { "um" } else { "keep" },
                    i * 500,
                    i * 500 + 200,
                )
            })
            .collect();
        let t = transcript(&words);
        let m = mapper(&[(0, 300_000)]);
        let ids = t
            .words
            .iter()
            .filter(|w| w.text == "um")
            .map(|w| w.id.clone())
            .collect::<Vec<_>>();
        let cuts = word_cuts(&t, &ids, &m).unwrap();
        assert_eq!(cuts.len(), 300);
        assert_eq!(*cuts.last().unwrap(), (298_850_000, 299_350_000));
        for word in t.words.iter().filter(|w| w.text == "keep") {
            assert!(cuts
                .iter()
                .all(|&(a, b)| b <= word.source_start_us || a >= word.source_end_us));
        }
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
    fn greek_filler_sounds_count_when_drawn_out() {
        for sound in ["Εεε", "εμ", "Μμμ", "χμμ", "έεε"] {
            assert!(is_filler_sound(&normalize(sound)), "{sound}");
        }
        for word in ["ε", "α", "και", "μα"] {
            assert!(!is_filler_sound(&normalize(word)), "{word}");
        }
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

    #[test]
    fn retakes_read_punctuation_given_as_words_and_skip_whole_sentences_that_open_alike() {
        // "I'm going to show you the app ." ... "I'm going to explain the price ."
        let t = transcript(&[
            ("I'm", 0, 200),
            ("going", 220, 400),
            ("to", 420, 500),
            ("show", 520, 800),
            ("you", 820, 900),
            ("the", 920, 1000),
            ("app", 1020, 1300),
            (".", 1300, 1300),
            ("I'm", 2000, 2200),
            ("going", 2220, 2400),
            ("to", 2420, 2500),
            ("explain", 2520, 2900),
            ("the", 2920, 3000),
            ("price", 3020, 3300),
            (".", 3300, 3300),
        ]);
        let m = mapper(&[(0, 4000)]);
        assert!(
            suggestions(&t, &m).is_empty(),
            "two whole sentences, not a retake"
        );

        // "So today we , so today we build a farm ." with a separate comma token.
        let t = transcript(&[
            ("So", 0, 200),
            ("today", 220, 500),
            ("we", 520, 600),
            (",", 600, 600),
            ("so", 1200, 1400),
            ("today", 1420, 1700),
            ("we", 1720, 1800),
            ("build", 1820, 2100),
            ("a", 2120, 2200),
            ("farm", 2220, 2500),
            (".", 2500, 2500),
        ]);
        let s = suggestions(&t, &m);
        assert_eq!(s.len(), 1, "{s:?}");
        assert_eq!(s[0].kind, TranscriptSuggestionKind::Retake);
        assert_eq!(s[0].text, "So today we");
    }

    /// Phrases said one after another, half a second apart; each word 200 ms, 20 ms apart.
    fn spoken(phrases: &[&str]) -> Transcript {
        let mut words: Vec<(String, u64, u64)> = Vec::new();
        let mut at = 0u64;
        for phrase in phrases {
            for word in phrase.split_whitespace() {
                words.push((word.to_string(), at, at + 200));
                at += 220;
            }
            at += 500;
        }
        let refs: Vec<(&str, u64, u64)> =
            words.iter().map(|(w, a, b)| (w.as_str(), *a, *b)).collect();
        transcript(&refs)
    }

    fn retakes(phrases: &[&str]) -> Vec<String> {
        let t = spoken(phrases);
        let m = mapper(&[(0, 600_000)]);
        suggestions(&t, &m)
            .into_iter()
            .filter(|s| s.kind == TranscriptSuggestionKind::Retake)
            .map(|s| s.text)
            .collect()
    }

    #[test]
    fn parallel_statements_are_not_retakes() {
        // From a real edit (21 Oct 2026): both halves are meant, one word apart.
        let found = retakes(&[
            "is officially closing down.",
            "It will not have a token,",
            "it will not have an airdrop.",
            "What they will do is distribute stable coins",
        ]);
        assert!(found.is_empty(), "{found:?}");
        // The same without punctuation, as Parakeet often writes it.
        let found = retakes(&[
            "It will not have a token",
            "it will not have an airdrop",
            "what they will do is distribute stable coins",
        ]);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_list_that_opens_alike_is_not_a_retake() {
        // From the same edit: three things you will be able to do, each said in full.
        let found = retakes(&[
            "so in a few days",
            "you will be able to see exactly how much money you are going to take and",
            "you will be able to withdraw on Solana or on BNB chain",
            "you will be able to redeem and sell all of that",
        ]);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_phrase_ending_one_sentence_and_opening_the_next_is_meant() {
        // From the "Agent test" edit (EN-A): a pause before the term, then it is defined.
        let found = retakes(&[
            "and this is what we call",
            "human in the loop.",
            "Human in the loop essentially means that",
        ]);
        assert!(found.is_empty(), "{found:?}");
        // Broken off on a small word, it is still a retake.
        assert_eq!(
            retakes(&["now we", "open up the.", "Open up the settings panel"]),
            vec!["open up the."]
        );
        // From George's Greek test (EL-B): a drawn-out first word, then the phrase again.
        assert_eq!(
            retakes(&[
                "Λοιπόν, αυτό εδώ είναι ένα τεστ",
                "γιααα",
                "το ελληνικό βίντεο,",
                "για το ελληνικό βίντεο το οποίο θα κάνουμε σήμερα.",
            ]),
            vec!["γιααα το ελληνικό βίντεο,"]
        );
        // And a sentence begun again whole after the last one ended.
        assert_eq!(
            retakes(&["That works.", "Let's begin.", "Let's begin with the export"]),
            vec!["Let's begin."]
        );
    }

    #[test]
    fn false_starts_are_still_retakes() {
        // Repeated whole, then carried on.
        assert_eq!(
            retakes(&[
                "you will be able to",
                "you will be able to withdraw on Solana"
            ]),
            vec!["you will be able to"]
        );
        // Corrected into something longer.
        assert_eq!(
            retakes(&["now click the button,", "now click the blue button."]),
            vec!["now click the button,"]
        );
        // Cut off a word or two after the repeated words.
        assert_eq!(
            retakes(&[
                "if you go to the set",
                "if you go to the settings page and click"
            ]),
            vec!["if you go to the set"]
        );
    }

    #[test]
    fn small_words_said_twice_are_stutters() {
        // From real edits (16-22 Oct 2026).
        assert_eq!(retakes(&["We we used 17% of it"]), vec!["We"]);
        assert_eq!(retakes(&["It's a it's a clear win"]), vec!["It's a"]);
        assert_eq!(retakes(&["I'm not I'm not going"]), vec!["I'm not"]);
        assert_eq!(
            retakes(&["That's that's completely completely obvious."]),
            vec!["That's"]
        );
        assert_eq!(
            retakes(&["a problem with with previous models"]),
            vec!["with"]
        );
        // From the 24 Oct recording (EN-D).
        assert_eq!(
            retakes(&["what is, what is going to happen?"]),
            vec!["what is,"]
        );
        assert_eq!(
            retakes(&["we can see that the, that the robot's hand"]),
            vec!["that the,"]
        );
        assert_eq!(
            retakes(&["I gotta say, they, they've got some demos"]),
            vec!["they,"]
        );
        // Alone, these may be meant; two different pair words are not a stutter.
        assert!(retakes(&["I know that that works"]).is_empty());
        assert!(retakes(&["what that is"]).is_empty());
    }

    #[test]
    fn words_said_twice_for_weight_are_meant() {
        for phrase in [
            "this is pretty, pretty amazing",
            "very very important",
            "really, really low",
            "blah blah blah",
            "I know that that works",
        ] {
            assert!(retakes(&[phrase]).is_empty(), "{phrase}");
        }
        // Across a sentence end both are meant.
        assert!(retakes(&["be honest with you. You should take a look"]).is_empty());
    }

    #[test]
    fn a_broken_off_word_is_cut() {
        assert_eq!(retakes(&["So we j- just like that"]), vec!["j-"]);
        assert_eq!(retakes(&["very important t- for speed"]), vec!["t-"]);
        assert_eq!(retakes(&["now inst- install it"]), vec!["inst-"]);
        // A sentence broken off with a double dash keeps its words.
        assert!(retakes(&["this is amazing. Like-- And if you"]).is_empty());
        // A hyphen ending a longer word that does not start the next is left alone.
        assert!(retakes(&["self- and peer review"]).is_empty());
    }
}
