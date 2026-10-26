//! The local pass ahead of an in-house filler and retake review. It flags what might be a
//! filler or a retake, without a model and the same way every time, so the agent reads only
//! the words around each flag instead of the whole transcript. Indexes are the review's own:
//! positions in the kept words, as the full pages number them, so cuts are saved the same way.
use super::fillers::chunk_prompt;
use crate::transcript::edit::{
    can_end_sentence, ends_clause, is_filler_sound, is_punctuation, normalize, same_word,
};
use crate::transcript::TranscriptWord;
use std::ops::Range;

/// Words shown on either side of a flag.
pub const CONTEXT_WORDS: usize = 15;
/// Words per page of windows.
const PAGE_WORDS: usize = 400;
/// Windows at most this many words apart are read as one.
const JOIN_GAP_WORDS: usize = 5;
/// A pause at least this long between two words breaks the phrase.
const BREAK_PAUSE_US: u64 = 250_000;
/// A pause at least this long can be where a sentence is begun again.
const HALT_PAUSE_US: u64 = 400_000;
/// A phrase said again: at least this many words, the second time starting at most this many
/// words and this long after the first ended.
const MIN_REPEAT_WORDS: usize = 3;
const MAX_REPEAT_GAP_WORDS: usize = 6;
const MAX_REPEAT_GAP_US: u64 = 4_000_000;
/// A restart: a phrase begun again with at least its first two words, at most this far back.
const MIN_RESTART_WORDS: usize = 2;
const MAX_RESTART_WORDS: usize = 16;
const MAX_RESTART_US: u64 = 12_000_000;
/// A first attempt that ended its sentence counts only if it is at most this many words
/// longer than what was said again: two whole sentences that open alike are no retake.
const MAX_FINISHED_EXTRA_WORDS: usize = 4;

/// Before "like", these keep its meaning: "I like", "looks like", "just like".
const LIKE_MEANT_AFTER: &str = "i you we they he she i'd you'd we'd they'd would could might \
    do does did don't doesn't didn't really look looks looked looking feel feels felt seem \
    seems seemed sound sounds sounded something anything nothing everything things stuff more \
    less much just exactly kind sort not won't wouldn't to also still i'll you'll we'll they'll";
/// After "like", these keep its meaning: "like this".
const LIKE_MEANT_BEFORE: &str = "this that these those so";
/// After "so", these keep its meaning: "so that", "so much", "and so on".
const SO_MEANT_BEFORE: &str = "that much many far long on forth good bad easy hard fast slow \
    cool nice simple big small little important useful quick quickly often different close";
/// Before "so", these keep its meaning: "I think so", "or so".
const SO_MEANT_AFTER: &str = "think hope guess believe do did does done or not if even";
/// After "so", these mark it as a filler however it is said: "so yeah", "so basically".
const SO_FILLER_BEFORE: &str = "yeah basically like anyway okay ok right";
/// Before "you know", these keep its meaning: "do you know", "now you know".
const YOU_KNOW_MEANT_AFTER: &str =
    "do did don't didn't if now will would may might should let then want";
/// After "you know" or "I mean", these keep its meaning: "you know how", "I mean it".
const KNOW_MEANT_BEFORE: &str = "what how that where why when who which the this it about if \
    whether them him her me to";
/// Before "I mean", these keep its meaning: "what I mean".
const MEAN_MEANT_AFTER: &str = "what which that if do did don't";
/// Before "kind of" or "sort of", these make it "a type of": "what kind of file".
const KIND_MEANT_AFTER: &str = "a an the this that these those what which some any every same \
    different one other another each no my your our their his her its whatever all certain \
    specific new right wrong first last best";

// Greek, written as `normalize` gives it: lowercase, accents folded.
/// Before "βασικά" or "ουσιαστικά", these make it a word ("τα βασικά", the basics).
const GR_BASIC_MEANT_AFTER: &str = "τα τις τους πιο πολυ";
/// After "ας πούμε", these make it "let's say that".
const GR_LETS_SAY_MEANT_BEFORE: &str = "οτι πως";
/// Before "ξέρεις" or "ξέρετε", these keep its meaning: "δεν ξέρεις", "αν ξέρετε".
const GR_KNOW_MEANT_AFTER: &str = "δεν το τα τον την τη αν μας με σε τους πια ηδη";
/// After "ξέρεις" or "ξέρετε", these keep its meaning: "ξέρεις τι", "ξέρετε πώς".
const GR_KNOW_MEANT_BEFORE: &str = "τι πως οτι να αν που ποιος ποια ποιο ποτε γιατι το τα τον \
    την τη πολλα ποσο ποσα";
/// "εντάξει" or "οκ" opening a sentence is a filler; inside one ("είναι εντάξει") a word.
const GR_OKAY: &str = "ενταξει οκ οκει";

/// `word` is one of the space-separated words in `list`.
fn among(list: &str, word: &str) -> bool {
    list.split_whitespace().any(|w| w == word)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flag {
    /// um, uh and the like: the rules cut these by default.
    Sound,
    /// A word that is a filler in some sentences only ("like", "so", "you know").
    Filler,
    /// A phrase said again: the first attempt may be a false start.
    Retake,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub flag: Flag,
    /// Word indexes, inclusive.
    pub from: usize,
    pub to: usize,
    /// What to check, shown to the agent.
    pub note: String,
}

/// A spoken word: punctuation and filler sounds left out.
struct Spoken {
    /// Its index in the kept words.
    at: usize,
    norm: String,
    /// A pause, a filler sound or punctuation comes before it (or it is the first).
    after_break: bool,
    /// A pause, a filler sound, a sentence end or a dash comes before it: a sentence can be
    /// begun again here. A comma alone is not enough ("how to install it, how to use it").
    after_halt: bool,
    /// A sentence ends right before it (". ? !").
    after_stop: bool,
}

/// What the punctuation ending `text` says: (it ends a sentence, it ends or breaks one off).
fn halts(text: &str) -> (bool, bool) {
    let text = text.trim_end();
    // Greek asks with ";" (or its own U+037E). Scribe writes it on the word ("γιατί;").
    let stop = text.ends_with(['.', '?', '!', '\u{2026}', '\u{037e}'])
        || (text.ends_with(';') && crate::transcript::language::has_greek(text));
    (stop, stop || text.ends_with(['-', '\u{2013}', '\u{2014}']))
}

/// Everything the local pass flags in `words` (the kept words, in recording order).
pub fn find(words: &[&TranscriptWord]) -> Vec<Candidate> {
    let mut out = Vec::new();
    let mut spoken: Vec<Spoken> = Vec::new();
    let (mut after_break, mut after_halt, mut after_stop) = (true, true, false);
    let mut last_end: Option<u64> = None;
    for (i, word) in words.iter().enumerate() {
        if is_punctuation(&word.text) {
            let (stop, halt) = halts(&word.text);
            after_break |= ends_clause(word.text.trim());
            after_halt |= halt;
            after_stop |= stop;
            continue;
        }
        let norm = normalize(&word.text);
        if norm.is_empty() {
            continue;
        }
        if last_end.is_some_and(|end| word.source_start_us.saturating_sub(end) >= BREAK_PAUSE_US) {
            after_break = true;
        }
        // A restart needs a longer pause, and never follows a comma: a pause after a comma is
        // a list or a clause going on.
        if last_end.is_some_and(|end| word.source_start_us.saturating_sub(end) >= HALT_PAUSE_US)
            && !words[i - 1].text.trim_end().ends_with(',')
        {
            after_halt = true;
        }
        last_end = Some(word.source_end_us);
        if is_filler_sound(&norm) {
            match out.last_mut() {
                Some(Candidate {
                    flag: Flag::Sound,
                    to,
                    note,
                    ..
                }) if *to + 1 == i => {
                    *to = i;
                    note.push(' ');
                    note.push_str(&word.text);
                }
                _ => out.push(Candidate {
                    flag: Flag::Sound,
                    from: i,
                    to: i,
                    note: word.text.clone(),
                }),
            }
            (after_break, after_halt) = (true, true);
            continue;
        }
        spoken.push(Spoken {
            at: i,
            norm,
            after_break,
            after_halt,
            after_stop,
        });
        after_break = ends_clause(&word.text);
        (after_stop, after_halt) = halts(&word.text);
    }
    out.extend(fillers(words, &spoken));
    out.extend(retakes(words, &spoken));
    out.sort_by_key(|c| (c.from, c.to));
    out
}

/// Words that are fillers in some sentences only, unless the words around them keep their
/// meaning.
fn fillers(words: &[&TranscriptWord], s: &[Spoken]) -> Vec<Candidate> {
    let norm = |k: usize| s.get(k).map_or("", |w| w.norm.as_str());
    // The word said right before or after `k`, with no break between.
    let before = |k: usize| {
        if s[k].after_break {
            ""
        } else {
            norm(k - 1)
        }
    };
    let after = |k: usize| match s.get(k + 1) {
        Some(next) if !next.after_break => next.norm.as_str(),
        _ => "",
    };
    let joined = |k: usize| s.get(k).is_some_and(|w| !w.after_break);
    let mut out = Vec::new();
    let mut k = 0;
    while k < s.len() {
        let (len, meant) = match norm(k) {
            "basically" => (1, false),
            "like" => (
                1,
                among(LIKE_MEANT_AFTER, before(k)) || among(LIKE_MEANT_BEFORE, after(k)),
            ),
            "so" => {
                let lead = s[k].after_break || after(k).is_empty();
                let filler = lead || among(SO_FILLER_BEFORE, after(k));
                (
                    1,
                    !filler || among(SO_MEANT_BEFORE, after(k)) || among(SO_MEANT_AFTER, before(k)),
                )
            }
            "you" if norm(k + 1) == "know" && joined(k + 1) => (
                2,
                among(YOU_KNOW_MEANT_AFTER, before(k)) || among(KNOW_MEANT_BEFORE, after(k + 1)),
            ),
            "i" if norm(k + 1) == "mean" && joined(k + 1) => (
                2,
                among(MEAN_MEANT_AFTER, before(k)) || among(KNOW_MEANT_BEFORE, after(k + 1)),
            ),
            "kind" | "sort" if norm(k + 1) == "of" && joined(k + 1) => {
                (2, among(KIND_MEANT_AFTER, before(k)))
            }
            "kinda" | "sorta" => (1, among(KIND_MEANT_AFTER, before(k))),
            "λοιπον" | "δηλαδη" | "καπως" => (1, false),
            "βασικα" | "ουσιαστικα" => (1, among(GR_BASIC_MEANT_AFTER, before(k))),
            "ας" if norm(k + 1) == "πουμε" && joined(k + 1) => {
                (2, among(GR_LETS_SAY_MEANT_BEFORE, after(k + 1)))
            }
            "τελος" if norm(k + 1) == "παντων" && joined(k + 1) => (2, false),
            "ξερεις" | "ξερετε" => (
                1,
                among(GR_KNOW_MEANT_AFTER, before(k)) || among(GR_KNOW_MEANT_BEFORE, after(k)),
            ),
            w if among(GR_OKAY, w) => (1, !s[k].after_break),
            _ => (0, true),
        };
        if !meant {
            let (from, to) = (s[k].at, s[k + len - 1].at);
            out.push(Candidate {
                flag: Flag::Filler,
                from,
                to,
                note: format!("\"{}\" (filler?)", said(words, from, to)),
            });
        }
        k += len.max(1);
    }
    out
}

/// Phrases said again soon after: the first attempt may be a false start. Repeating one word
/// ("very very", "no no no") is emphasis, never flagged.
fn retakes(words: &[&TranscriptWord], s: &[Spoken]) -> Vec<Candidate> {
    let start = |k: usize| words[s[k].at].source_start_us;
    let end = |k: usize| words[s[k].at].source_end_us;
    // How many words from `a` are said again from `b`, the first attempt not overlapping.
    let matched = |a: usize, b: usize| {
        (0..)
            .take_while(|&k| {
                a + k < b && b + k < s.len() && same_word(&s[a + k].norm, &s[b + k].norm)
            })
            .count()
    };
    let one_word = |a: usize, m: usize| s[a..a + m].iter().all(|w| w.norm == s[a].norm);
    // A finished sentence ending on the phrase, then a new one opening with it: said again on
    // purpose ("what we call human in the loop. Human in the loop means..."). The phrase
    // doesn't open its sentence, and the sentence ends on a word it can end on.
    let defining = |a: usize, m: usize, b: usize| {
        b == a + m
            && s[b].after_stop
            && a > 0
            && !s[a].after_stop
            && can_end_sentence(&s[a + m - 1].norm)
    };
    let mut out = Vec::new();
    let mut covered = 0;
    let mut b = 1;
    while b < s.len() {
        let mut found = None;
        let mut meant = None;
        // A phrase of three or more words said again within a few seconds.
        let lowest = b
            .saturating_sub(MAX_RESTART_WORDS + MAX_REPEAT_GAP_WORDS)
            .max(covered);
        for a in lowest..b {
            let m = matched(a, b);
            if m >= MIN_REPEAT_WORDS
                && b - (a + m) <= MAX_REPEAT_GAP_WORDS
                && start(b).saturating_sub(end(a + m - 1)) <= MAX_REPEAT_GAP_US
                && !one_word(a, m)
            {
                if defining(a, m, b) {
                    meant = Some(m);
                } else {
                    found = Some((a, m));
                }
                break;
            }
        }
        // A phrase stopped and begun again with its first words.
        if found.is_none() && meant.is_none() && s[b].after_halt {
            for a in b.saturating_sub(MAX_RESTART_WORDS).max(covered)..b {
                if !(a == 0 || s[a].after_break)
                    || start(b).saturating_sub(start(a)) > MAX_RESTART_US
                {
                    continue;
                }
                let m = matched(a, b);
                let finished = s[b].after_stop;
                if m >= MIN_RESTART_WORDS
                    && !one_word(a, m)
                    && (!finished || b - a <= m + MAX_FINISHED_EXTRA_WORDS)
                {
                    if defining(a, m, b) {
                        meant = Some(m);
                    } else {
                        found = Some((a, m));
                    }
                    break;
                }
            }
        }
        // Said again on purpose: no part of it is a retake either.
        if let Some(m) = meant {
            covered = b;
            b += m;
            continue;
        }
        match found {
            Some((a, m)) => {
                let (from, to) = (s[a].at, s[b].at - 1);
                out.push(Candidate {
                    flag: Flag::Retake,
                    from,
                    to,
                    note: format!(
                        "\"{}\" said again (retake?)",
                        said(words, s[a].at, s[a + m - 1].at)
                    ),
                });
                covered = b;
                b += m;
            }
            None => b += 1,
        }
    }
    out
}

/// The words `from..=to` as said.
fn said(words: &[&TranscriptWord], from: usize, to: usize) -> String {
    words[from..=to]
        .iter()
        .map(|w| w.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Words around one or more flags, read together.
#[derive(Clone, Debug, PartialEq)]
pub struct Window {
    pub words: Range<usize>,
    pub flags: Vec<Candidate>,
}

/// What a candidate review reads: the flags, their windows and how the windows are paged.
#[derive(Clone, Debug)]
pub struct Plan {
    pub candidates: Vec<Candidate>,
    pub windows: Vec<Window>,
    /// Each page's windows, as a range of `windows`.
    pub pages: Vec<Range<usize>>,
}

/// Flags `words` (the kept words) and lays out the windows to read. Flags overlapping
/// `settled` (index ranges, inclusive, that the rules cut already) are left out, as are
/// filler sounds: the rules cut those by default.
pub fn plan(words: &[&TranscriptWord], settled: &[(usize, usize)]) -> Plan {
    let candidates: Vec<Candidate> = find(words)
        .into_iter()
        .filter(|c| {
            c.flag == Flag::Sound || !settled.iter().any(|&(a, b)| c.from <= b && a <= c.to)
        })
        .collect();
    let mut windows: Vec<Window> = Vec::new();
    for c in candidates.iter().filter(|c| c.flag != Flag::Sound) {
        let range =
            c.from.saturating_sub(CONTEXT_WORDS)..(c.to + CONTEXT_WORDS + 1).min(words.len());
        match windows.last_mut() {
            Some(last)
                if range.start <= last.words.end + JOIN_GAP_WORDS
                    && range.end.max(last.words.end) - last.words.start <= PAGE_WORDS =>
            {
                last.words.end = last.words.end.max(range.end);
                last.flags.push(c.clone());
            }
            _ => windows.push(Window {
                words: range,
                flags: vec![c.clone()],
            }),
        }
    }
    let mut pages: Vec<Range<usize>> = Vec::new();
    let mut size = 0;
    for (n, window) in windows.iter().enumerate() {
        let len = window.words.len();
        match pages.last_mut() {
            Some(page) if size + len <= PAGE_WORDS => {
                page.end = n + 1;
                size += len;
            }
            _ => {
                pages.push(n..n + 1);
                size = len;
            }
        }
    }
    Plan {
        candidates,
        windows,
        pages,
    }
}

impl Plan {
    pub fn count(&self, flag: Flag) -> usize {
        self.candidates.iter().filter(|c| c.flag == flag).count()
    }

    /// Words in all the windows (a word in two overlapping windows counts twice: it is read
    /// twice).
    pub fn shown_words(&self) -> usize {
        self.windows.iter().map(|w| w.words.len()).sum()
    }

    /// Page `page`'s windows: each one's flags, then its words as `[index]word` with pauses.
    pub fn page_text(&self, words: &[&TranscriptWord], page: usize) -> Option<String> {
        let range = self.pages.get(page)?;
        let texts: Vec<String> = range
            .clone()
            .map(|n| {
                let window = &self.windows[n];
                let flags: Vec<String> = window
                    .flags
                    .iter()
                    .map(|c| {
                        let at = if c.from == c.to {
                            format!("[{}]", c.from)
                        } else {
                            format!("[{}]-[{}]", c.from, c.to)
                        };
                        format!("{at} {}", c.note)
                    })
                    .collect();
                format!(
                    "Window {} of {}, check: {}\n{}",
                    n + 1,
                    self.windows.len(),
                    flags.join("; "),
                    chunk_prompt(words, window.words.clone())
                )
            })
            .collect();
        Some(texts.join("\n\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcript::WordKind;

    /// Words 300 ms apart, each 250 ms long; "|" before a word puts a one-second pause there.
    fn words(text: &str) -> Vec<TranscriptWord> {
        let mut at = 0u64;
        text.split_whitespace()
            .enumerate()
            .map(|(i, token)| {
                let (pause, text) = match token.strip_prefix('|') {
                    Some(rest) => (1_000_000, rest),
                    None => (0, token),
                };
                at += pause;
                let word = TranscriptWord {
                    id: format!("w-{i}"),
                    text: text.to_string(),
                    kind: WordKind::Word,
                    source_start_us: at,
                    source_end_us: at + 250_000,
                    confidence: None,
                    speaker: None,
                };
                at += 300_000;
                word
            })
            .collect()
    }

    fn flags(text: &str) -> Vec<(Flag, usize, usize)> {
        let owned = words(text);
        let refs: Vec<&TranscriptWord> = owned.iter().collect();
        find(&refs)
            .into_iter()
            .map(|c| (c.flag, c.from, c.to))
            .collect()
    }

    #[test]
    fn greek_sounds_fillers_and_their_meanings() {
        use Flag::*;
        // "Εεε" is a sound; "λοιπόν" and "ξέρεις," fillers; "ξέρεις τι" keeps its meaning.
        assert_eq!(
            flags("Εεε λοιπόν σήμερα, ξέρεις, φτιάχνουμε κάτι. |Ξέρεις τι θέλω;"),
            vec![(Sound, 0, 0), (Filler, 1, 1), (Filler, 3, 3)]
        );
        // "τα βασικά" is "the basics"; "βασικά" alone is a filler, accents or none.
        assert_eq!(flags("Μάθαμε τα βασικά."), vec![]);
        assert_eq!(flags("Βασικα είναι απλό."), vec![(Filler, 0, 0)]);
        // "εντάξει" opening a sentence, not inside one; "ας πούμε ότι" means it.
        assert_eq!(flags("Εντάξει, πάμε. Είναι εντάξει."), vec![(Filler, 0, 0)]);
        assert_eq!(
            flags("Ας πούμε ότι τρέχει. Τρέχει, ας πούμε, γρήγορα."),
            vec![(Filler, 5, 6)]
        );
    }

    #[test]
    fn a_greek_question_ends_its_sentence() {
        // "Τι κάνει;" is a whole question: said again it is no retake.
        assert!(halts("κάνει;").0);
        assert!(!halts("what;").0);
    }

    #[test]
    fn filler_sounds_in_every_spelling() {
        assert_eq!(
            flags("Ummm, so to speak. Uhh uh the erm Hmmm mm here"),
            vec![
                (Flag::Sound, 0, 0),
                (Flag::Filler, 1, 1),
                (Flag::Sound, 4, 5),
                (Flag::Sound, 7, 9),
            ]
        );
        // "so to speak" after a filler sound: "so" opens the phrase, so it is a flag.
        assert!(flags("the summer and errands").is_empty());
    }

    #[test]
    fn fillers_that_carry_meaning_are_not_flagged() {
        // Meant: a verb, a comparison, a type of, a purpose, a real question.
        for meant in [
            "I like this tool a lot",
            "it looks like the old one",
            "do it like this and save",
            "what kind of file is it",
            "it is a sort of plugin",
            "move it so that you can see",
            "it is so much faster",
            "do you know what this does",
            "you know how to save files",
            "that is what I mean by fast",
            "I think so too",
        ] {
            let found: Vec<_> = flags(meant)
                .into_iter()
                .filter(|f| f.0 == Flag::Filler)
                .collect();
            assert!(found.is_empty(), "{meant}: {found:?}");
        }
    }

    #[test]
    fn fillers_said_as_fillers_are_flagged() {
        for (filler, from, to) in [
            ("it is, like, really fast", 2, 2),
            ("and it was like really fast", 3, 3),
            ("So, let's open the editor", 0, 0),
            ("click save so yeah that works", 2, 2),
            ("it is basically done", 2, 2),
            ("you know, the editor is fast", 0, 1),
            ("it's, I mean, it works", 1, 2),
            ("it is kind of slow here", 2, 3),
            ("it is kinda slow here", 2, 2),
        ] {
            assert_eq!(flags(filler), vec![(Flag::Filler, from, to)], "{filler}");
        }
    }

    #[test]
    fn retakes_are_flagged_and_emphasis_is_not() {
        // The same three words again, right away: the first attempt and nothing after.
        assert_eq!(
            flags("we want to |we want to add a clip"),
            vec![(Flag::Retake, 0, 2)]
        );
        // A sentence stopped and begun again with its first two words.
        assert_eq!(
            flags("The next. |The next step is export"),
            vec![(Flag::Retake, 0, 1)]
        );
        // Cut off mid-sentence, a pause, and said again.
        assert_eq!(
            flags("open the settings and |open the settings panel"),
            vec![(Flag::Retake, 0, 3)]
        );
        // The same opening in a list, after a comma, is no restart.
        assert!(flags("what they do, how to install them, |how to use them").is_empty());
        assert!(flags("you might like them, you might not like them").is_empty());
        // Broken off with a dash and begun again.
        assert_eq!(
            flags("we open the\u{2014} we open the panel"),
            vec![(Flag::Retake, 0, 2)]
        );
        // One word repeated is emphasis.
        assert!(flags("it is very very very good").is_empty());
        assert!(flags("no no no no that is wrong").is_empty());
        // Two whole sentences that open alike are not a retake.
        assert!(flags(
            "The export runs in the background and saves the file. |The export \
             dialog shows its progress"
        )
        .is_empty());
        // A finished sentence ending on a phrase, then one opening with it, is meant.
        assert!(flags(
            "this is what we call |human in the loop. Human in the loop means you check it"
        )
        .is_empty());
        // Broken off on a small word and begun again is still a retake.
        assert_eq!(
            flags("now we open up the. |Open up the settings panel"),
            vec![(Flag::Retake, 2, 4)]
        );
        // A phrase said again much later is a new sentence.
        assert!(flags(
            "add a clip |here we trim it down and move on to the next part of \
             the video and |add a clip"
        )
        .is_empty());
    }

    #[test]
    fn windows_cover_the_flags_and_skip_what_the_rules_cut() {
        // 100 plain words with "like" at 50 and a filler sound at 80.
        let mut texts: Vec<String> = (0..100).map(|i| format!("w{i}")).collect();
        texts[49] = "and".into();
        texts[50] = "like".into();
        texts[80] = "um".into();
        let owned = words(&texts.join(" "));
        let refs: Vec<&TranscriptWord> = owned.iter().collect();
        let plan = plan(&refs, &[]);
        assert_eq!(plan.count(Flag::Sound), 1);
        assert_eq!(plan.count(Flag::Filler), 1);
        assert_eq!(plan.windows.len(), 1);
        assert_eq!(plan.windows[0].words, 35..66);
        assert_eq!(plan.shown_words(), 31);
        assert_eq!(plan.pages, vec![0..1]);
        let text = plan.page_text(&refs, 0).unwrap();
        assert!(
            text.starts_with("Window 1 of 1, check: [50] \"like\" (filler?)\n[35]w35 "),
            "{text}"
        );
        assert!(text.ends_with("[65]w65"), "{text}");
        assert!(plan.page_text(&refs, 1).is_none());

        // Close flags share a window; a flag the rules settled opens none.
        texts[60] = "basically".into();
        let owned = words(&texts.join(" "));
        let refs: Vec<&TranscriptWord> = owned.iter().collect();
        let joined = super::plan(&refs, &[]);
        assert_eq!(joined.windows.len(), 1);
        assert_eq!(joined.windows[0].words, 35..76);
        assert_eq!(joined.windows[0].flags.len(), 2);
        let settled = super::plan(&refs, &[(48, 52)]);
        assert_eq!(settled.windows.len(), 1);
        assert_eq!(settled.windows[0].words, 45..76);
        assert!(super::plan(&refs, &[(0, 99)]).windows.is_empty());
    }
}
