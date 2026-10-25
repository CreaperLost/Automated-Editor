//! The user's vocabulary (LNG-5): product names and jargon written the way the user writes
//! them. Scribe is biased towards them while it listens (`keyterms`); after any transcriber,
//! the words that spell a term with other spacing or case ("claude code", "chat gpt") take
//! the term's spelling. Parakeet takes no hints, so for it a run of two or more words that
//! sounds like a term ("cloud code") becomes the term too. A single word that only sounds like
//! one ("codecs", "cloud") is left alone: it is as likely the real word.
use super::{TranscriptWord, WordKind};

/// Most words a heard run may span beyond the term's own.
const EXTRA_WORDS: usize = 2;

/// Gives terms their spelling in `words`, returning what changed (`"cloud code" -> "Claude
/// Code" (2)`). `sound_alike` also takes runs that sound like a term. Word ids are numbered
/// again when runs merge.
pub fn apply(words: &mut Vec<TranscriptWord>, terms: &[String], sound_alike: bool) -> Vec<String> {
    let mut terms: Vec<(&str, Vec<&str>, String)> = terms
        .iter()
        .map(|t| t.trim())
        .filter(|t| !t.is_empty())
        .map(|t| (t, t.split_whitespace().collect(), letters(t)))
        .filter(|(_, _, key)| key.chars().count() >= 3)
        .collect();
    // Longer terms first, so "Claude Code" wins over "Claude".
    terms.sort_by_key(|(_, tokens, key)| std::cmp::Reverse((tokens.len(), key.len())));
    let mut changes: Vec<(String, String, usize)> = Vec::new();
    let mut merged = false;
    let mut i = 0;
    while i < words.len() {
        let found = terms.iter().find_map(|(term, tokens, key)| {
            let most = (tokens.len() + EXTRA_WORDS).min(words.len() - i);
            (1..=most).rev().find_map(|span| {
                let run = &words[i..i + span];
                if run.iter().any(|w| w.kind != WordKind::Word) {
                    return None;
                }
                let heard: String = run.iter().map(|w| letters(&w.text)).collect();
                let same = heard == *key;
                let sounds =
                    sound_alike && (span > 1 || tokens.len() > 1) && sounds_like(&heard, key);
                (same || sounds).then_some((span, *term, tokens))
            })
        });
        let Some((span, term, tokens)) = found else {
            i += 1;
            continue;
        };
        let heard: Vec<String> = words[i..i + span].iter().map(|w| w.text.clone()).collect();
        let tail = trailing(heard.last().map_or("", String::as_str)).to_string();
        if span == tokens.len() {
            for (k, token) in tokens.iter().enumerate() {
                let own_tail = trailing(&heard[k]).to_string();
                words[i + k].text = format!("{token}{own_tail}");
            }
        } else {
            let end = words[i + span - 1].source_end_us;
            words[i].text = format!("{term}{tail}");
            words[i].source_end_us = end;
            words.drain(i + 1..i + span);
            merged = true;
        }
        let before = heard
            .iter()
            .map(|w| w.trim_end_matches(|c: char| !c.is_alphanumeric()))
            .collect::<Vec<_>>()
            .join(" ");
        if before != term {
            match changes.iter_mut().find(|c| c.0 == before && c.1 == term) {
                Some(change) => change.2 += 1,
                None => changes.push((before, term.to_string(), 1)),
            }
        }
        i += if span == tokens.len() { span } else { 1 };
    }
    if merged {
        for (n, word) in words.iter_mut().enumerate() {
            word.id = format!("w-{n}");
        }
    }
    changes
        .into_iter()
        .map(|(from, to, n)| match n {
            1 => format!("\"{from}\" -> \"{to}\""),
            n => format!("\"{from}\" -> \"{to}\" ({n})"),
        })
        .collect()
}

/// Letters and digits, lower case: what a word spells whatever its spacing and punctuation.
fn letters(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Punctuation after a word's last letter, kept when the word is respelled.
fn trailing(text: &str) -> &str {
    let end = text
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_alphanumeric())
        .map_or(0, |(i, c)| i + c.len_utf8());
    &text[end..]
}

/// Two spellings sound alike: the same consonants, in order, after spelling sounds one way,
/// and few letters apart.
fn sounds_like(heard: &str, term: &str) -> bool {
    let room = (term.chars().count() / 4).max(2);
    skeleton(heard) == skeleton(term) && distance(heard, term) <= room
}

/// The consonant outline of a spelling: "ph" as f, c, q and ck as k, x as ks, z as s; vowels
/// (and h, w, y) after the first letter dropped; doubled letters once.
fn skeleton(text: &str) -> String {
    let spelled = text
        .replace("ph", "f")
        .replace("ck", "k")
        .replace('x', "ks")
        .replace(['c', 'q'], "k")
        .replace('z', "s");
    let mut out = String::new();
    for (i, c) in spelled.chars().enumerate() {
        if i > 0 && "aeiouhwy".contains(c) {
            continue;
        }
        if out.ends_with(c) {
            continue;
        }
        out.push(c);
    }
    out
}

/// Edit distance between two spellings.
fn distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let next = (previous + usize::from(ca != cb))
                .min(row[j] + 1)
                .min(row[j + 1] + 1);
            previous = row[j + 1];
            row[j + 1] = next;
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(text: &str) -> Vec<TranscriptWord> {
        text.split_whitespace()
            .enumerate()
            .map(|(i, w)| TranscriptWord {
                id: format!("w-{i}"),
                text: w.into(),
                kind: WordKind::Word,
                source_start_us: i as u64 * 1_000,
                source_end_us: i as u64 * 1_000 + 900,
                confidence: None,
                speaker: None,
            })
            .collect()
    }

    fn text(words: &[TranscriptWord]) -> String {
        words
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn terms(list: &[&str]) -> Vec<String> {
        list.iter().map(|t| t.to_string()).collect()
    }

    #[test]
    fn spelling_and_spacing_take_the_term_for_every_transcriber() {
        let mut w = words("open claude code, then ask chat gpt about github.");
        let changes = apply(&mut w, &terms(&["Claude Code", "ChatGPT", "GitHub"]), false);
        assert_eq!(text(&w), "open Claude Code, then ask ChatGPT about GitHub.");
        assert_eq!(changes.len(), 3, "{changes:?}");
        // "chat gpt" merged into one word spanning both, and ids follow.
        let chat = w.iter().find(|w| w.text == "ChatGPT").unwrap();
        assert_eq!((chat.source_start_us, chat.source_end_us), (5_000, 6_900));
        assert!(w.iter().enumerate().all(|(i, w)| w.id == format!("w-{i}")));
    }

    #[test]
    fn parakeet_runs_that_sound_like_a_term_take_it() {
        let mut w = words("I use cloud code and code x daily");
        apply(&mut w, &terms(&["Claude Code", "Codex"]), true);
        assert_eq!(text(&w), "I use Claude Code and Codex daily");
    }

    #[test]
    fn a_single_word_that_only_sounds_alike_is_left() {
        let list = terms(&["Claude", "Codex"]);
        let mut w = words("the cloud stores codecs");
        assert!(apply(&mut w, &list, true).is_empty());
        assert_eq!(text(&w), "the cloud stores codecs");
        // Without sound-alikes, "cloud code" stays too.
        let mut w = words("cloud code");
        assert!(apply(&mut w, &terms(&["Claude Code"]), false).is_empty());
        // Words already right are not reported.
        let mut w = words("Claude Code");
        assert!(apply(&mut w, &terms(&["Claude Code"]), true).is_empty());
    }

    #[test]
    fn skeletons_hear_spellings_alike() {
        assert_eq!(skeleton("cloudcode"), skeleton("claudecode"));
        assert_eq!(skeleton("codex"), skeleton("codecs"));
        assert_ne!(skeleton("codex"), skeleton("code"));
        assert_eq!(distance("kitten", "sitting"), 3);
        assert_eq!(trailing("code,"), ",");
        assert_eq!(trailing("end?!"), "?!");
    }
}
