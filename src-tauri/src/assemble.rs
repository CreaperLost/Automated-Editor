//! Putting imported takes together (FTG-4). With a script, each line is placed from the take
//! that says it best, trimmed to those words; the other takes that say it are listed as
//! alternatives, and a line no take says is listed as missing. Without one, the takes go
//! whole in their names' natural order (take 2 before take 10), and a take a later one says
//! again is left out for the later one, as the last try is usually the keeper. Only counted
//! and planned here: the agent proposes the placements as one batch the user accepts.
use crate::source_search::normalize;
use std::cmp::Ordering;

/// Of a line's words, how many a take must say, in order, for it to say the line.
const MATCH_MIN: f64 = 0.6;
/// A score this close to the best is as good: the later take wins.
const TIE: f64 = 0.05;
/// Room kept either side of a line's words, where the words next to it allow.
const PAD_US: u64 = 150_000;
/// A take this short is never called a repeat of another.
const REPEAT_MIN_WORDS: usize = 3;

/// One imported take: its media id, name, length, and its words on its own clock.
#[derive(Clone, Debug, Default)]
pub struct Take {
    pub media: String,
    pub name: String,
    pub duration_us: u64,
    /// (normalized text, start, end). Empty when it has no transcript.
    pub words: Vec<(String, u64, u64)>,
}

/// One placement: `media` from `start_us` to `end_us` on its own clock.
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub media: String,
    pub start_us: u64,
    pub end_us: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    pub pieces: Vec<Piece>,
    /// What the user reads beside the batch: missing lines, alternatives, repeats.
    pub report: Vec<String>,
}

/// Names compared as people read them: digits as numbers, letters ignoring case.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    fn chunks(s: &str) -> Vec<(bool, String)> {
        let mut out: Vec<(bool, String)> = Vec::new();
        for c in s.chars() {
            let digit = c.is_ascii_digit();
            match out.last_mut() {
                Some((d, text)) if *d == digit => text.push(c),
                _ => out.push((digit, c.to_string())),
            }
        }
        out
    }
    let (x, y) = (chunks(a), chunks(b));
    for (p, q) in x.iter().zip(&y) {
        let order = match (p.0, q.0) {
            (true, true) => {
                let (m, n) = (p.1.trim_start_matches('0'), q.1.trim_start_matches('0'));
                m.len().cmp(&n.len()).then_with(|| m.cmp(n))
            }
            _ => p.1.to_lowercase().cmp(&q.1.to_lowercase()),
        };
        if order != Ordering::Equal {
            return order;
        }
    }
    x.len().cmp(&y.len()).then_with(|| a.cmp(b))
}

/// The run of `said` that best says `line`, in order with slips allowed: (first word, last
/// word, how many of `line`'s words it says). The latest of equal runs.
pub fn best_span(line: &[String], said: &[String]) -> Option<(usize, usize, usize)> {
    if line.is_empty() || said.is_empty() {
        return None;
    }
    let (n, m) = (line.len(), said.len());
    // Local alignment: a word said +2, a word skipped or extra -1.
    let mut score = vec![vec![0i32; m + 1]; n + 1];
    let mut best = (0i32, 0usize, 0usize);
    for i in 1..=n {
        for j in 1..=m {
            let same = if line[i - 1] == said[j - 1] { 2 } else { -1 };
            let s = (score[i - 1][j - 1] + same)
                .max(score[i - 1][j] - 1)
                .max(score[i][j - 1] - 1)
                .max(0);
            score[i][j] = s;
            if s > 0 && s >= best.0 && line[i - 1] == said[j - 1] {
                best = (s, i, j);
            }
        }
    }
    if best.0 == 0 {
        return None;
    }
    // Back to where the run starts, counting the words it says.
    let (mut i, mut j) = (best.1, best.2);
    let (last, mut first, mut matched) = (j - 1, j - 1, 0);
    while i > 0 && j > 0 && score[i][j] > 0 {
        let same = if line[i - 1] == said[j - 1] { 2 } else { -1 };
        if score[i][j] == score[i - 1][j - 1] + same {
            if same > 0 {
                matched += 1;
                first = j - 1;
            }
            i -= 1;
            j -= 1;
        } else if score[i][j] == score[i - 1][j] - 1 {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    Some((first, last, matched))
}

fn plain(take: &Take) -> Vec<String> {
    take.words.iter().map(|w| w.0.clone()).collect()
}

fn line_words(line: &str) -> Vec<String> {
    line.split_whitespace()
        .map(normalize)
        .filter(|w| !w.is_empty())
        .collect()
}

/// `take`'s words `first..=last` with room either side, short of the words next to them.
fn piece(take: &Take, first: usize, last: usize) -> Piece {
    let words = &take.words;
    let floor = first.checked_sub(1).map_or(0, |i| words[i].2);
    let ceiling = words.get(last + 1).map_or(take.duration_us, |w| w.1);
    Piece {
        media: take.media.clone(),
        start_us: words[first].1.saturating_sub(PAD_US).max(floor),
        end_us: (words[last].2 + PAD_US).min(ceiling.max(words[last].2)),
    }
}

fn quote(line: &str) -> String {
    let short: String = line.trim().chars().take(60).collect();
    format!("\"{short}\"")
}

fn percent(x: f64) -> String {
    format!("{}%", (x * 100.0).round())
}

/// Each line of `script` from the take that says it best.
pub fn by_script(takes: &[Take], script: &[String]) -> Plan {
    let mut takes: Vec<&Take> = takes.iter().collect();
    takes.sort_by(|a, b| natural_cmp(&a.name, &b.name));
    let said: Vec<Vec<String>> = takes.iter().map(|t| plain(t)).collect();
    let mut plan = Plan::default();
    let mut used = vec![false; takes.len()];
    for (n, line) in script.iter().enumerate() {
        let words = line_words(line);
        if words.is_empty() {
            continue;
        }
        // (take, first, last, score) of each take that says it.
        let mut saying: Vec<(usize, usize, usize, f64)> = said
            .iter()
            .enumerate()
            .filter_map(|(t, s)| {
                let (first, last, matched) = best_span(&words, s)?;
                let score = matched as f64 / words.len() as f64;
                (score >= MATCH_MIN).then_some((t, first, last, score))
            })
            .collect();
        let Some(top) = saying.iter().map(|s| s.3).reduce(f64::max) else {
            plan.report
                .push(format!("Missing: line {} {}", n + 1, quote(line)));
            continue;
        };
        // The latest take as good as the best.
        let chosen = saying
            .iter()
            .rposition(|s| s.3 >= top - TIE)
            .expect("one is the best");
        let (t, first, last, score) = saying.remove(chosen);
        used[t] = true;
        plan.pieces.push(piece(takes[t], first, last));
        if !saying.is_empty() {
            let others: Vec<String> = saying
                .iter()
                .map(|(o, f, l, s)| {
                    let p = piece(takes[*o], *f, *l);
                    format!(
                        "{} at {} ({})",
                        takes[*o].name,
                        crate::mcp::tools::clock(p.start_us),
                        percent(*s)
                    )
                })
                .collect();
            plan.report.push(format!(
                "Line {}: from {} ({}); also said in {}",
                n + 1,
                takes[t].name,
                percent(score),
                others.join(", ")
            ));
        }
    }
    let unused: Vec<&str> = takes
        .iter()
        .zip(&used)
        .filter(|(t, u)| !**u && !t.words.is_empty())
        .map(|(t, _)| t.name.as_str())
        .collect();
    if !unused.is_empty() {
        plan.report
            .push(format!("Not in the script: {}", unused.join(", ")));
    }
    plan
}

/// The takes whole in natural order, each one a later take says again left out.
pub fn by_order(takes: &[Take]) -> Plan {
    let mut takes: Vec<&Take> = takes.iter().collect();
    takes.sort_by(|a, b| natural_cmp(&a.name, &b.name));
    let said: Vec<Vec<String>> = takes.iter().map(|t| plain(t)).collect();
    let mut plan = Plan::default();
    for (i, take) in takes.iter().enumerate() {
        let again = (said[i].len() >= REPEAT_MIN_WORDS)
            .then(|| {
                (i + 1..takes.len()).find(|&j| {
                    best_span(&said[i], &said[j]).is_some_and(|(_, _, matched)| {
                        matched as f64 / said[i].len() as f64 >= MATCH_MIN
                    })
                })
            })
            .flatten();
        match again {
            Some(j) => plan.report.push(format!(
                "Left out {}: {} says it again",
                take.name, takes[j].name
            )),
            None => plan.pieces.push(Piece {
                media: take.media.clone(),
                start_us: 0,
                end_us: take.duration_us,
            }),
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: u64 = 1_000_000;

    /// A take saying `text`, a word every half second.
    fn take(name: &str, text: &str) -> Take {
        let words: Vec<(String, u64, u64)> = text
            .split_whitespace()
            .enumerate()
            .map(|(i, w)| (normalize(w), i as u64 * S / 2, i as u64 * S / 2 + 400_000))
            .collect();
        Take {
            media: format!("m-{name}"),
            name: name.into(),
            duration_us: words.len() as u64 * S / 2 + S,
            words,
        }
    }

    #[test]
    fn names_sort_as_people_read_them() {
        let mut names = vec!["Take 10.mp4", "take 2.mp4", "Take 1.mp4", "intro.mp4"];
        names.sort_by(|a, b| natural_cmp(a, b));
        assert_eq!(
            names,
            vec!["intro.mp4", "Take 1.mp4", "take 2.mp4", "Take 10.mp4"]
        );
    }

    #[test]
    fn the_run_that_says_a_line_is_found_with_slips() {
        let w = |s: &str| line_words(s);
        let said = w("um okay so open the settings uh panel and then close it");
        let (first, last, matched) = best_span(&w("open the settings panel"), &said).unwrap();
        assert_eq!((first, last, matched), (3, 7, 4));
        assert!(best_span(&w("nothing here"), &said).is_none());
    }

    #[test]
    fn each_line_comes_from_the_best_take_with_alternatives_and_gaps_listed() {
        let takes = [
            take("take 10", "open the settings panel now"),
            take("take 2", "hello and welcome to the course"),
            take("take 3", "open the settings panel"),
            take("take 4", "something else entirely"),
        ];
        let script = vec![
            "Hello and welcome to the course".to_string(),
            "Open the settings panel.".to_string(),
            "Then we export the video".to_string(),
        ];
        let plan = by_script(&takes, &script);
        let media: Vec<&str> = plan.pieces.iter().map(|p| p.media.as_str()).collect();
        // Takes 3 and 10 both say line 2 in full: the later one, take 10, is used.
        assert_eq!(media, vec!["m-take 2", "m-take 10"]);
        // Trimmed to its words: "now" after them is left out.
        let p = &plan.pieces[1];
        // Up to the next word's start, short of the full room after it.
        assert_eq!((p.start_us, p.end_us), (0, 2 * S));
        assert!(plan
            .report
            .iter()
            .any(|l| l.starts_with("Line 2: from take 10")
                && l.contains("also said in take 3 at 0:00.00 (100%)")));
        assert!(plan
            .report
            .contains(&"Missing: line 3 \"Then we export the video\"".to_string()));
        assert!(plan
            .report
            .contains(&"Not in the script: take 3, take 4".to_string()));
    }

    #[test]
    fn without_a_script_takes_go_in_order_and_a_repeat_waits_for_the_later_one() {
        let takes = [
            take("clip 10", "and that is how you export"),
            take("clip 2", "first we open the settings"),
            take("clip 9", "and that is how we export it"),
        ];
        let plan = by_order(&takes);
        let media: Vec<&str> = plan.pieces.iter().map(|p| p.media.as_str()).collect();
        assert_eq!(media, vec!["m-clip 2", "m-clip 10"]);
        assert_eq!(
            plan.report,
            vec!["Left out clip 9: clip 10 says it again".to_string()]
        );
        assert_eq!(plan.pieces[0].end_us, takes[1].duration_us);
    }
}
