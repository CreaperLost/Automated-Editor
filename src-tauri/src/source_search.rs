//! Search what was said in every saved transcript (FTG-1): the recording, imported takes and
//! videos, including what the edit cut out. Each hit says where it plays in the edit, or that
//! it was cut, or that its media isn't on the timeline, so an agent (or the user) can find a
//! moment by its words and bring it back.
use crate::project::revision::EditDocument;
use crate::sequence::StreamKind;
use crate::transcript::{store, TranscriptWord, WordKind};
use serde::Serialize;
use std::path::Path;

/// Hits returned unless told otherwise, and at most.
pub const LIMIT_DEFAULT: usize = 20;
pub const LIMIT_MAX: usize = 100;
/// Words of context either side of a hit.
const CONTEXT_WORDS: usize = 6;
/// Query words this long or longer match a word one letter off (a typo, a mishearing).
const FUZZY_MIN_CHARS: usize = 5;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Hit {
    /// `<asset>.<stream>`.
    pub stream: String,
    pub media: String,
    pub source_start_us: u64,
    pub source_end_us: u64,
    pub first_word: String,
    pub last_word: String,
    /// The words around it, the hit in `[ ]`.
    pub excerpt: String,
    /// Where it starts in the edit; for a cut line, where the edit joins over it; none when
    /// its media isn't on the timeline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_us: Option<u64>,
    pub status: Status,
    /// The take short whose voice it is, by title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,
    /// Matched every query word exactly (else one was a prefix or a letter off).
    pub exact: bool,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    /// Plays in the edit (at `editedUs`).
    Plays,
    /// Its media is on the timeline but these words were cut out.
    Cut,
    /// Its media isn't on the timeline (an imported take or video).
    NotPlaced,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Results {
    /// Exact hits first, each group in recording order per stream.
    pub hits: Vec<Hit>,
    /// Hits found, before `limit`.
    pub total: usize,
    /// Streams searched.
    pub streams: usize,
}

/// Lower case, letters and digits only, Greek accents dropped.
pub fn normalize(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(unaccent)
        .filter(|c| c.is_alphanumeric())
        .collect()
}

fn unaccent(c: char) -> char {
    match c {
        'ά' => 'α',
        'έ' => 'ε',
        'ή' => 'η',
        'ί' | 'ϊ' | 'ΐ' => 'ι',
        'ό' => 'ο',
        'ύ' | 'ϋ' | 'ΰ' => 'υ',
        'ώ' => 'ω',
        'ς' => 'σ',
        'à' | 'á' | 'â' | 'ä' | 'ã' => 'a',
        'è' | 'é' | 'ê' | 'ë' => 'e',
        'ì' | 'í' | 'î' | 'ï' => 'i',
        'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
        'ù' | 'ú' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        other => other,
    }
}

/// At most one letter added, dropped or changed.
fn one_off(a: &str, b: &str) -> bool {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a.len().abs_diff(b.len()) > 1 {
        return false;
    }
    let (short, long) = if a.len() <= b.len() {
        (&a, &b)
    } else {
        (&b, &a)
    };
    let start = short
        .iter()
        .zip(long.iter())
        .take_while(|(x, y)| x == y)
        .count();
    if start == short.len() {
        return true;
    }
    if short.len() == long.len() {
        short[start + 1..] == long[start + 1..]
    } else {
        short[start..] == long[start + 1..]
    }
}

/// How query word `q` matches word `w`: `Some(true)` exactly, `Some(false)` loosely.
fn matches(q: &str, w: &str, last: bool) -> Option<bool> {
    if q == w {
        Some(true)
    } else if (last && w.starts_with(q) && q.chars().count() >= 2)
        || (q.chars().count() >= FUZZY_MIN_CHARS && one_off(q, w))
    {
        Some(false)
    } else {
        None
    }
}

/// Where the words of `query` come in `words`, in order: (first index, last index, exact).
pub fn find_in(words: &[String], query: &[String]) -> Vec<(usize, usize, bool)> {
    let mut out = Vec::new();
    if query.is_empty() {
        return out;
    }
    let mut i = 0;
    while i + query.len() <= words.len() {
        let mut exact = true;
        let found = query.iter().enumerate().all(|(k, q)| {
            match matches(q, &words[i + k], k + 1 == query.len()) {
                Some(e) => {
                    exact &= e;
                    true
                }
                None => false,
            }
        });
        if found {
            out.push((i, i + query.len() - 1, exact));
            i += query.len();
        } else {
            i += 1;
        }
    }
    out
}

fn excerpt(words: &[&TranscriptWord], first: usize, last: usize) -> String {
    let from = first.saturating_sub(CONTEXT_WORDS);
    let to = (last + CONTEXT_WORDS + 1).min(words.len());
    let mut out = String::new();
    if from > 0 {
        out.push('…');
    }
    for (i, w) in words.iter().enumerate().take(to).skip(from) {
        if !out.is_empty() && !out.ends_with('…') {
            out.push(' ');
        }
        if i == first {
            out.push('[');
        }
        out.push_str(w.text.trim());
        if i == last {
            out.push(']');
        }
    }
    if to < words.len() {
        out.push('…');
    }
    out
}

/// Searches every transcribed sound of `document` (or only `stream`) for `query`.
pub fn search(
    root: &Path,
    document: &EditDocument,
    query: &str,
    stream: Option<&str>,
    limit: usize,
) -> Result<Results, String> {
    let wanted: Vec<String> = query
        .split_whitespace()
        .map(normalize)
        .filter(|w| !w.is_empty())
        .collect();
    if wanted.is_empty() {
        return Err("Give `query`: a word or phrase.".into());
    }
    let placed: std::collections::HashSet<&str> = document
        .sequence
        .clips()
        .map(|(_, c)| c.asset.as_str())
        .collect();
    let mut results = Results::default();
    let mut loose = Vec::new();
    for asset in &document.assets {
        for s in asset.streams.iter().filter(|s| s.kind == StreamKind::Sound) {
            let key = format!("{}.{}", asset.id, s.id);
            if stream.is_some_and(|wanted| wanted != key) {
                continue;
            }
            let Some(transcript) = store::load_transcript(root, &key).ok().flatten() else {
                continue;
            };
            results.streams += 1;
            let words: Vec<&TranscriptWord> = transcript
                .words
                .iter()
                .filter(|w| w.kind == WordKind::Word)
                .collect();
            let plain: Vec<String> = words.iter().map(|w| normalize(&w.text)).collect();
            let on_timeline = placed.contains(asset.id.as_str());
            let clock = on_timeline
                .then(|| document.transcript_clock(&key).ok())
                .flatten();
            let short = document
                .shorts
                .iter()
                .find(|sh| sh.take.as_ref().is_some_and(|t| t.voice == key))
                .map(|sh| sh.title.clone());
            for (first, last, exact) in find_in(&plain, &wanted) {
                let (a, b) = (words[first], words[last]);
                let plays = clock
                    .as_ref()
                    .and_then(|c| c.source_to_edited_us(a.source_start_us));
                let status = match (plays, on_timeline) {
                    (Some(_), _) => Status::Plays,
                    (None, true) => Status::Cut,
                    (None, false) => Status::NotPlaced,
                };
                // A cut line: where the edit joins over it (what plays next).
                let edited_us = plays.or_else(|| {
                    let c = clock.as_ref()?;
                    let after = c.source_range_to_edited(b.source_end_us, u64::MAX);
                    after.first().map(|r| r.0).or(Some(document.duration_us()))
                });
                let hit = Hit {
                    stream: key.clone(),
                    media: asset.name.clone(),
                    source_start_us: a.source_start_us,
                    source_end_us: b.source_end_us,
                    first_word: a.id.clone(),
                    last_word: b.id.clone(),
                    excerpt: excerpt(&words, first, last),
                    edited_us,
                    status,
                    short: short.clone(),
                    exact,
                };
                if exact {
                    results.hits.push(hit);
                } else {
                    loose.push(hit);
                }
            }
        }
    }
    results.hits.extend(loose);
    results.total = results.hits.len();
    results.hits.truncate(limit.clamp(1, LIMIT_MAX));
    Ok(results)
}

/// The results as lines, for agents and the CLI.
pub fn text(results: &Results, query: &str) -> String {
    let clock = crate::mcp::tools::clock;
    if results.streams == 0 {
        return "Nothing is transcribed yet, so there is nothing to search.".into();
    }
    if results.hits.is_empty() {
        return format!(
            "\"{query}\" isn't in the {} transcript{}.",
            results.streams,
            if results.streams == 1 { "" } else { "s" }
        );
    }
    let mut out = format!(
        "{} hit{} for \"{query}\"{}. Lines: stream, recording time, where it is in the edit, \
         word ids, words.\n",
        results.total,
        if results.total == 1 { "" } else { "s" },
        if results.total > results.hits.len() {
            format!(" (first {} shown)", results.hits.len())
        } else {
            String::new()
        }
    );
    for h in &results.hits {
        let place = match (h.status, h.edited_us) {
            (Status::Plays, Some(at)) => format!("plays at {}", clock(at)),
            (Status::Cut, Some(at)) => format!("cut (the join at {})", clock(at)),
            (Status::Cut, None) => "cut".into(),
            _ => match &h.short {
                Some(title) => format!("voice of short \"{title}\""),
                None => "not on the timeline".into(),
            },
        };
        out.push_str(&format!(
            "{} {}-{} {place} {}..{} {}{}\n",
            h.stream,
            clock(h.source_start_us),
            clock(h.source_end_us),
            h.first_word,
            h.last_word,
            h.excerpt,
            if h.exact { "" } else { " (close match)" }
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<String> {
        text.split_whitespace().map(normalize).collect()
    }

    #[test]
    fn phrases_match_whole_words_in_order() {
        let said = words("So, open the Settings panel. Then settle it.");
        assert_eq!(find_in(&said, &words("the settings")), vec![(2, 3, true)]);
        // The last word may be the start of one; earlier words must be whole.
        assert_eq!(find_in(&said, &words("open the sett")), vec![(1, 3, false)]);
        assert!(find_in(&said, &words("pen the")).is_empty());
    }

    #[test]
    fn long_words_match_one_letter_off_and_accents_fold() {
        let said = words("Ανοίγουμε το Wallett σου");
        assert_eq!(find_in(&said, &words("wallet")), vec![(2, 2, false)]);
        assert_eq!(find_in(&said, &words("ανοιγουμε")), vec![(0, 0, true)]);
        assert!(one_off("wallet", "walled"));
        assert!(!one_off("wallet", "pallets"));
    }
}
