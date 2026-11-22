//! Where the speaker gets loud or excited (CLIP-2), Clipper's idea without a chat: lines of
//! the edit whose speech runs well over the speaker's usual level around them, and lines with
//! reaction phrases. A hint for picking clips; it never makes one.
use crate::transcript::edit::TranscriptViewWord;
use crate::transcript::WordKind;

/// A line this far over the median of the lines around it is loud.
pub const LOUD_DB: f64 = 4.0;
/// The lines a line is compared with: those within this much of it (edited time).
const AROUND_US: u64 = 5 * 60 * 1_000_000;
/// A pause this long, or a sentence end after a few words, starts a new line.
const PAUSE_US: u64 = 700_000;
const MAX_LINE_US: u64 = 12_000_000;
const PHRASES: &[&str] = &[
    "no way",
    "oh my god",
    "let's go",
    "lets go",
    "wow",
    "what the",
    "insane",
    "crazy",
    "unbelievable",
    "finally",
    "holy",
    "oh no",
    "look at this",
    "check this out",
];

#[derive(Clone, Debug, PartialEq)]
pub struct Moment {
    pub from_word: String,
    pub to_word: String,
    pub edited_us: u64,
    pub edited_end_us: u64,
    pub text: String,
    /// dB over the lines around it; `None` when it wasn't measured.
    pub over_db: Option<f64>,
    /// The reaction phrase it has, if any.
    pub phrase: Option<&'static str>,
}

struct Line {
    words: Vec<usize>,
    text: String,
}

fn lines(words: &[TranscriptViewWord]) -> Vec<Line> {
    let mut out: Vec<Line> = Vec::new();
    let mut last: Option<usize> = None;
    for (i, w) in words.iter().enumerate() {
        let (Some(start), Some(_)) = (w.edited_start_us, w.edited_end_us) else {
            continue;
        };
        if w.word.kind != WordKind::Word {
            continue;
        }
        let new = match (out.last(), last) {
            (Some(line), Some(prev)) => {
                let prev_end = words[prev].edited_end_us.unwrap_or(start);
                let line_start = words[line.words[0]].edited_start_us.unwrap_or(start);
                start.saturating_sub(prev_end) >= PAUSE_US
                    || start.saturating_sub(line_start) > MAX_LINE_US
                    || (line.words.len() >= 4 && words[prev].word.text.ends_with(['.', '?', '!']))
            }
            _ => true,
        };
        if new {
            out.push(Line {
                words: Vec::new(),
                text: String::new(),
            });
        }
        let line = out.last_mut().unwrap();
        if !line.text.is_empty() {
            line.text.push(' ');
        }
        line.text.push_str(&w.word.text);
        line.words.push(i);
        last = Some(i);
    }
    out
}

fn phrase_in(text: &str) -> Option<&'static str> {
    let plain: String = text
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '\'' {
                c
            } else {
                ' '
            }
        })
        .collect();
    let plain = format!(
        " {} ",
        plain.split_whitespace().collect::<Vec<_>>().join(" ")
    );
    PHRASES
        .iter()
        .find(|p| plain.contains(&format!(" {p} ")))
        .copied()
}

/// The moments of `words` (kept words, in play order), loudest first, then the phrases.
/// `level` gives the mean square of the speech over a word's source time, if known.
pub fn moments(
    words: &[TranscriptViewWord],
    level: impl Fn(u64, u64) -> Option<f64>,
    at_most: usize,
) -> Vec<Moment> {
    let lines = lines(words);
    // Each line's level: its words' mean squares, weighted by length, in dB.
    let db: Vec<Option<f64>> = lines
        .iter()
        .map(|line| {
            let (mut energy, mut time) = (0.0, 0.0);
            for &i in &line.words {
                let w = &words[i].word;
                let len = w.source_end_us.saturating_sub(w.source_start_us) as f64;
                if let Some(ms) = level(w.source_start_us, w.source_end_us) {
                    energy += ms * len;
                    time += len;
                }
            }
            (time > 0.0 && energy > 0.0).then(|| 10.0 * (energy / time).log10())
        })
        .collect();
    let start = |line: &Line| words[line.words[0]].edited_start_us.unwrap_or(0);
    let mut out = Vec::new();
    for (k, line) in lines.iter().enumerate() {
        let at = start(line);
        let mut around: Vec<f64> = lines
            .iter()
            .zip(&db)
            .filter(|(l, _)| start(l).abs_diff(at) <= AROUND_US)
            .filter_map(|(_, d)| *d)
            .collect();
        let over = match db[k] {
            Some(d) if around.len() >= 5 => {
                around.sort_by(f64::total_cmp);
                Some(d - around[around.len() / 2])
            }
            _ => None,
        };
        let phrase = phrase_in(&line.text);
        if over.is_some_and(|o| o >= LOUD_DB) || phrase.is_some() {
            let last = *line.words.last().unwrap();
            out.push(Moment {
                from_word: words[line.words[0]].word.id.clone(),
                to_word: words[last].word.id.clone(),
                edited_us: at,
                edited_end_us: words[last].edited_end_us.unwrap_or(at),
                text: line.text.clone(),
                over_db: over,
                phrase,
            });
        }
    }
    out.sort_by(|a, b| {
        let loud = |m: &Moment| m.over_db.filter(|o| *o >= LOUD_DB).unwrap_or(f64::MIN);
        loud(b)
            .total_cmp(&loud(a))
            .then(a.edited_us.cmp(&b.edited_us))
    });
    out.truncate(at_most);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcript::TranscriptWord;

    fn word(i: usize, text: &str, at_s: f64) -> TranscriptViewWord {
        let us = (at_s * 1e6) as u64;
        TranscriptViewWord {
            word: TranscriptWord {
                id: format!("w-{i}"),
                text: text.into(),
                kind: WordKind::Word,
                source_start_us: us,
                source_end_us: us + 300_000,
                confidence: None,
                speaker: None,
            },
            edited_start_us: Some(us),
            edited_end_us: Some(us + 300_000),
            caption_hidden: false,
            caption_break: false,
            caption_join: false,
        }
    }

    #[test]
    fn a_loud_line_and_a_reaction_are_found_and_the_rest_left() {
        // Ten lines of four words, one a second apart, each line 5 s apart.
        let mut words = Vec::new();
        for line in 0..10 {
            let text = match line {
                7 => ["oh", "no", "way,", "look."],
                _ => ["this", "is", "a", "line."],
            };
            for (k, t) in text.iter().enumerate() {
                words.push(word(words.len(), t, line as f64 * 5.0 + k as f64 * 0.4));
            }
        }
        // Line 3 is 10 dB louder than the rest.
        let level = |from: u64, _| {
            Some(if (15_000_000..20_000_000).contains(&from) {
                10.0
            } else {
                1.0
            })
        };
        let found = moments(&words, level, 10);
        assert_eq!(found.len(), 2, "{found:?}");
        assert_eq!(found[0].from_word, "w-12");
        assert!((found[0].over_db.unwrap() - 10.0).abs() < 0.01);
        assert_eq!(found[1].phrase, Some("no way"));
        assert_eq!(
            (found[1].from_word.as_str(), found[1].to_word.as_str()),
            ("w-28", "w-31")
        );
    }
}
