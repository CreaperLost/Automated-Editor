//! AI clip suggestions: the model reads the time-stamped transcript lines and picks ideas
//! that work on their own as vertical clips, each from one stretch or a few gathered pieces.
use super::chapters::{lines, prompt, Line};
use super::client::JsonModel;
use crate::transcript::edit::TranscriptViewWord;
use serde_json::Value;

const MIN_US: u64 = 15_000_000;
const MAX_US: u64 = 90_000_000;
const MAX_PICKS: usize = 10;
const MAX_PIECES: usize = 4;

pub const SYSTEM_PROMPT: &str = "You pick ideas from a long video to post as vertical clips (TikTok, YouTube Shorts, Reels). \
The transcript is numbered lines in the form [index] m:ss text, in playback order. \
Pick up to 5 ideas that make sense on their own without the rest of the video: a clear tip, a funny or surprising moment, a strong result. \
Start from the idea, a premise a viewer gets in one sentence, then take the lines that serve it: usually one stretch, or up to 4 pieces from anywhere in the video when the idea is spread out, in the order that tells it best, the hook first. \
Each clip should be 20 to 60 seconds long in all, every piece start and end on whole sentences, the last on the payoff, and no line be used twice. \
Answer with a JSON object only: {\"shorts\":[{\"idea\":\"<the premise, one sentence>\",\"pieces\":[{\"from\":<first line>,\"to\":<last line>}],\"title\":\"<catchy title, under 60 characters>\",\"why\":\"<one short sentence>\"}]}. \
Use {\"shorts\":[]} when nothing stands on its own.";

/// One idea the model picked: its pieces as source stretches on the transcript's clock, in
/// play order.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub title: String,
    pub idea: String,
    pub reason: String,
    pub pieces: Vec<(u64, u64)>,
}

/// The valid picks: known lines in order within each piece, 15 to 90 s in all, no line used
/// twice in a pick or by an earlier pick. A pick may also give one `from`/`to`.
pub fn parse_shorts(reply: &Value, lines: &[Line]) -> Vec<Found> {
    let Some(items) = reply.get("shorts").and_then(Value::as_array) else {
        return Vec::new();
    };
    let line = |v: Option<&Value>| v?.as_u64().and_then(|n| usize::try_from(n).ok());
    let mut taken: Vec<(usize, usize)> = Vec::new();
    let mut out = Vec::new();
    for item in items {
        let pick = (|| {
            let clean = |key: &str, limit: usize| -> String {
                item.get(key)
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .chars()
                    .filter(|c| !c.is_control())
                    .take(limit)
                    .collect()
            };
            let title = clean("title", 100);
            let ranges: Vec<(usize, usize)> = match item.get("pieces").and_then(Value::as_array) {
                Some(pieces) => pieces
                    .iter()
                    .map(|p| Some((line(p.get("from"))?, line(p.get("to"))?)))
                    .collect::<Option<_>>()?,
                None => vec![(line(item.get("from"))?, line(item.get("to"))?)],
            };
            if title.is_empty() || ranges.is_empty() || ranges.len() > MAX_PIECES {
                return None;
            }
            let mut length = 0;
            for (n, &(from, to)) in ranges.iter().enumerate() {
                let overlaps = |&(a, b): &(usize, usize)| from <= b && a <= to;
                if from > to
                    || to >= lines.len()
                    || lines[to].source_end_us <= lines[from].source_us
                    || ranges[..n].iter().any(overlaps)
                    || taken.iter().any(overlaps)
                {
                    return None;
                }
                length += lines[to]
                    .edited_end_us
                    .saturating_sub(lines[from].edited_us);
            }
            if !(MIN_US..=MAX_US).contains(&length) {
                return None;
            }
            Some((ranges, title, clean("idea", 300), clean("why", 300)))
        })();
        let Some((ranges, title, idea, reason)) = pick else {
            continue;
        };
        taken.extend(&ranges);
        out.push(Found {
            title,
            idea,
            reason,
            pieces: ranges
                .iter()
                .map(|&(from, to)| (lines[from].source_us, lines[to].source_end_us))
                .collect(),
        });
        if out.len() == MAX_PICKS {
            break;
        }
    }
    out
}

pub fn suggest(
    model: &mut dyn JsonModel,
    words: &[TranscriptViewWord],
) -> Result<Vec<Found>, String> {
    let lines = lines(words);
    if lines.len() < 2 {
        return Err("The transcript is too short to pick clips from".into());
    }
    let reply = model.complete_json(SYSTEM_PROMPT, &prompt(&lines))?;
    let found = parse_shorts(&reply, &lines);
    if found.is_empty() {
        return Err("The AI found no moment that works as a clip on its own".into());
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const S: u64 = 1_000_000;

    fn ten_lines() -> Vec<Line> {
        // Ten lines of 10 s each.
        (0..10)
            .map(|i| Line {
                edited_us: i * 10 * S,
                source_us: 500 * S + i * 10 * S,
                edited_end_us: i * 10 * S + 9 * S,
                source_end_us: 500 * S + i * 10 * S + 9 * S,
                text: format!("line {i}"),
            })
            .collect()
    }

    #[test]
    fn picks_are_checked_for_length_order_and_overlap() {
        let lines = ten_lines();
        let reply = json!({ "shorts": [
            { "from": 1, "to": 3, "title": "  The   trick ", "why": "clear tip" },
            { "from": 2, "to": 4, "title": "Overlaps" },
            { "from": 5, "to": 5, "title": "Too short" },
            { "from": 0, "to": 9, "title": "Too long" },
            { "from": 6, "to": 8, "title": "" },
            { "from": 7, "to": 12, "title": "Out of range" },
            { "from": 6, "to": 7, "title": "Ending" },
        ]});
        let found = parse_shorts(&reply, &lines);
        let summary: Vec<(&str, Vec<(u64, u64)>)> = found
            .iter()
            .map(|f| (f.title.as_str(), f.pieces.clone()))
            .collect();
        assert_eq!(
            summary,
            vec![
                ("The trick", vec![(510 * S, 539 * S)]),
                ("Ending", vec![(560 * S, 579 * S)])
            ]
        );
        assert_eq!(found[0].reason, "clear tip");
    }

    #[test]
    fn a_pick_gathers_pieces_in_its_own_order() {
        let lines = ten_lines();
        let reply = json!({ "shorts": [
            { "idea": "It pays off", "title": "Payoff first",
              "pieces": [{ "from": 8, "to": 8 }, { "from": 1, "to": 2 }] },
            { "title": "Reuses a line", "pieces": [{ "from": 4, "to": 5 }, { "from": 5, "to": 6 }] },
            { "title": "Takes another's", "pieces": [{ "from": 2, "to": 3 }, { "from": 5, "to": 5 }] },
            { "title": "Bad piece", "pieces": [{ "from": 4, "to": 5 }, { "from": 7 }] },
            { "title": "Too many", "pieces": [{ "from": 3, "to": 3 }, { "from": 4, "to": 4 },
              { "from": 5, "to": 5 }, { "from": 6, "to": 6 }, { "from": 7, "to": 7 }] },
            { "title": "Spread out", "pieces": [{ "from": 6, "to": 6 }, { "from": 4, "to": 4 }] },
        ]});
        let found = parse_shorts(&reply, &lines);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].idea, "It pays off");
        assert_eq!(
            found[0].pieces,
            vec![(580 * S, 589 * S), (510 * S, 529 * S)]
        );
        assert_eq!(found[1].title, "Spread out");
        assert_eq!(
            found[1].pieces,
            vec![(560 * S, 569 * S), (540 * S, 549 * S)]
        );
    }
}
