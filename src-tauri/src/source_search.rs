//! Search what was said in every saved transcript (FTG-1): the recording, imported takes and
//! videos, including what the edit cut out. Each hit says where it plays in the edit, or that
//! it was cut, or that its media isn't on the timeline, so an agent (or the user) can find a
//! moment by its words and bring it back.
use crate::project::revision::EditDocument;
use crate::sequence::StreamKind;
use crate::transcript::{store, TranscriptWord, WordKind};
use parking_lot::Mutex;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

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

/// A transcript's words ready to search, kept between searches while its file is unchanged.
struct Indexed {
    /// The file's size and time when read: a write from another process shows as a change.
    stamp: (u64, Option<SystemTime>),
    /// Its words (not punctuation or sounds).
    words: Vec<TranscriptWord>,
    /// Each of `words`, normalized.
    plain: Vec<String>,
}

/// Searched transcripts by file, of the last project searched only. Saving or deleting a
/// transcript drops its entry ([`forget`]).
static INDEXED: Mutex<BTreeMap<PathBuf, Arc<Indexed>>> = Mutex::new(BTreeMap::new());

fn stamp(path: &Path) -> Option<(u64, Option<SystemTime>)> {
    let meta = std::fs::metadata(path).ok()?;
    meta.is_file().then(|| (meta.len(), meta.modified().ok()))
}

/// `key`'s transcript in `root`, ready to search: read and normalized once per change.
fn indexed(root: &Path, key: &str) -> Option<Arc<Indexed>> {
    let path = store::transcript_path(root, key).ok()?;
    // Taken before reading: a write in between makes the next search read it again.
    let stamp = stamp(&path)?;
    if let Some(entry) = INDEXED.lock().get(&path).filter(|e| e.stamp == stamp) {
        return Some(entry.clone());
    }
    let transcript = store::load_transcript(root, key).ok().flatten()?;
    let words: Vec<TranscriptWord> = transcript
        .words
        .into_iter()
        .filter(|w| w.kind == WordKind::Word)
        .collect();
    let plain = words.iter().map(|w| normalize(&w.text)).collect();
    let entry = Arc::new(Indexed {
        stamp,
        words,
        plain,
    });
    let mut cache = INDEXED.lock();
    cache.retain(|other, _| other.parent() == path.parent());
    cache.insert(path, entry.clone());
    Some(entry)
}

/// Drops `key`'s transcript in `root` from the search cache (it was saved or deleted).
pub fn forget(root: &Path, key: &str) {
    if let Ok(path) = store::transcript_path(root, key) {
        INDEXED.lock().remove(&path);
    }
}

/// What a search replaced by a newer one from the same window answers.
pub const REPLACED: &str = "A newer search replaced this one.";

/// The latest search each window asked for (its Search panel), by window.
static LATEST: Mutex<BTreeMap<String, u64>> = Mutex::new(BTreeMap::new());
static ASKED: AtomicU64 = AtomicU64::new(0);

/// A search asked for by a window: one still running when the same window asks again stops,
/// its answer no longer wanted.
pub struct Latest {
    window: String,
    id: u64,
}

impl Latest {
    pub fn new(window: &str) -> Self {
        let id = ASKED.fetch_add(1, Ordering::Relaxed) + 1;
        LATEST.lock().insert(window.to_string(), id);
        Self {
            window: window.to_string(),
            id,
        }
    }

    /// Whether the window has asked for another search since.
    pub fn replaced(&self) -> bool {
        LATEST.lock().get(&self.window) != Some(&self.id)
    }
}

/// The first `cap` hits in order, exact ones first, and how many there were in all: hits past
/// the cap are counted, never made.
struct Kept<T> {
    cap: usize,
    exact: Vec<T>,
    loose: Vec<T>,
    total: usize,
}

impl<T> Kept<T> {
    fn new(limit: usize) -> Self {
        Self {
            cap: limit.clamp(1, LIMIT_MAX),
            exact: Vec::new(),
            loose: Vec::new(),
            total: 0,
        }
    }

    fn push(&mut self, exact: bool, hit: impl FnOnce() -> T) {
        self.total += 1;
        let list = if exact {
            &mut self.exact
        } else {
            &mut self.loose
        };
        if list.len() < self.cap {
            list.push(hit());
        }
    }

    fn finish(self) -> (Vec<T>, usize) {
        let mut hits = self.exact;
        hits.extend(self.loose);
        hits.truncate(self.cap);
        (hits, self.total)
    }
}

fn excerpt(words: &[TranscriptWord], first: usize, last: usize) -> String {
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
    search_until(root, document, query, stream, limit, &|| false)
}

/// [`search`], given up (with [`REPLACED`]) as soon as `replaced` says a newer one is wanted.
pub fn search_until(
    root: &Path,
    document: &EditDocument,
    query: &str,
    stream: Option<&str>,
    limit: usize,
    replaced: &dyn Fn() -> bool,
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
    let mut kept = Kept::new(limit);
    for asset in &document.assets {
        for s in asset.streams.iter().filter(|s| s.kind == StreamKind::Sound) {
            let key = format!("{}.{}", asset.id, s.id);
            if stream.is_some_and(|wanted| wanted != key) {
                continue;
            }
            if replaced() {
                return Err(REPLACED.into());
            }
            let Some(index) = indexed(root, &key) else {
                continue;
            };
            results.streams += 1;
            let words = &index.words;
            let on_timeline = placed.contains(asset.id.as_str());
            let clock = on_timeline
                .then(|| document.transcript_clock(&key).ok())
                .flatten();
            let short = document
                .shorts
                .iter()
                .find(|sh| sh.take.as_ref().is_some_and(|t| t.voice == key))
                .map(|sh| sh.title.clone());
            for (first, last, exact) in find_in(&index.plain, &wanted) {
                kept.push(exact, || {
                    let (a, b) = (&words[first], &words[last]);
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
                    Hit {
                        stream: key.clone(),
                        media: asset.name.clone(),
                        source_start_us: a.source_start_us,
                        source_end_us: b.source_end_us,
                        first_word: a.id.clone(),
                        last_word: b.id.clone(),
                        excerpt: excerpt(words, first, last),
                        edited_us,
                        status,
                        short: short.clone(),
                        exact,
                    }
                });
            }
        }
    }
    (results.hits, results.total) = kept.finish();
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

    #[test]
    fn kept_hits_put_exact_first_and_count_every_hit() {
        let mut kept = Kept::new(3);
        let mut made = 0;
        for (i, exact) in [false, true, false, true, false, true, true, false]
            .into_iter()
            .enumerate()
        {
            kept.push(exact, || {
                made += 1;
                (i, exact)
            });
        }
        let (hits, total) = kept.finish();
        assert_eq!(total, 8);
        assert_eq!(hits, vec![(1, true), (3, true), (5, true)]);
        // Three of each kind at most: the rest are only counted.
        assert_eq!(made, 6);

        let mut kept = Kept::new(3);
        for (i, exact) in [false, true, false, false].into_iter().enumerate() {
            kept.push(exact, || i);
        }
        assert_eq!(kept.finish(), (vec![1, 0, 2], 4));
    }

    #[test]
    fn transcripts_are_read_once_until_saved_again() {
        use crate::transcript::{test_word, ProviderKind, Transcript};
        let dir = tempfile::tempdir().unwrap();
        let save = |text: &str| {
            let t = Transcript::new(
                "mic".into(),
                ProviderKind::Parakeet,
                "m".into(),
                None,
                vec![test_word(text, 0, 300)],
            );
            store::save_transcript(dir.path(), &t).unwrap();
        };
        save("Hello");
        let first = indexed(dir.path(), "mic").unwrap();
        assert_eq!(first.plain, vec!["hello".to_string()]);
        assert!(Arc::ptr_eq(&first, &indexed(dir.path(), "mic").unwrap()));
        // Same length, so only the save's own invalidation can tell.
        save("Jello");
        assert_eq!(
            indexed(dir.path(), "mic").unwrap().plain,
            vec!["jello".to_string()]
        );
        store::delete_transcript(dir.path(), "mic").unwrap();
        assert!(indexed(dir.path(), "mic").is_none());
    }

    /// `cargo test --release --lib search_cost -- --ignored --nocapture`: reading a long
    /// transcript against searching it once it is cached.
    #[test]
    #[ignore]
    fn search_cost() {
        use crate::transcript::{test_word, ProviderKind, Transcript};
        use std::time::Instant;
        let dir = tempfile::tempdir().unwrap();
        let said = [
            "open", "the", "settings", "panel", "then", "wallet", "σου", "εδώ",
        ];
        let long = (0..100_000u64)
            .map(|i| {
                test_word(
                    said[i as usize % said.len()],
                    i * 400_000,
                    i * 400_000 + 300_000,
                )
            })
            .collect();
        let t = Transcript::new("mic".into(), ProviderKind::Parakeet, "m".into(), None, long);
        store::save_transcript(dir.path(), &t).unwrap();
        let query = words("the settings");
        let time = |label: &str| {
            let start = Instant::now();
            let index = indexed(dir.path(), "mic").unwrap();
            let mut kept = Kept::new(LIMIT_MAX);
            for (first, _, exact) in find_in(&index.plain, &query) {
                kept.push(exact, || first);
            }
            let (_, total) = kept.finish();
            println!("{label}: {:?} for {total} hits", start.elapsed());
        };
        time("cold");
        time("warm");
        time("warm");
    }

    #[test]
    fn a_newer_search_from_the_same_window_replaces_one() {
        let first = Latest::new("test-window");
        let other = Latest::new("test-other-window");
        assert!(!first.replaced());
        let second = Latest::new("test-window");
        assert!(first.replaced());
        assert!(!second.replaced());
        assert!(!other.replaced());
    }
}
