//! AI short suggestions: the model reads the time-stamped transcript lines and picks
//! self-contained moments that work on their own as vertical shorts.
use super::chapters::{lines, prompt, Line};
use super::client::JsonModel;
use crate::shorts::Short;
use crate::transcript::edit::TranscriptViewWord;
use serde_json::Value;

const MIN_US: u64 = 15_000_000;
const MAX_US: u64 = 90_000_000;
const MAX_PICKS: usize = 10;

pub const SYSTEM_PROMPT: &str = "You pick moments from a long video to post as vertical shorts (TikTok, YouTube Shorts, Reels). \
The transcript is numbered lines in the form [index] m:ss text, in playback order. \
Pick up to 5 moments that make sense on their own without the rest of the video: a clear tip, a funny or surprising moment, a strong result. \
Each should be 20 to 60 seconds long, start with a hook, end at the end of a sentence, and not overlap the others. \
Answer with a JSON object only: {\"shorts\":[{\"from\":<first line>,\"to\":<last line>,\"title\":\"<catchy title, under 60 characters>\",\"why\":\"<one short sentence>\"}]}. \
Use {\"shorts\":[]} when nothing stands on its own.";

/// The valid picks: known lines in order, 15 to 90 s long, not overlapping an earlier pick.
pub fn parse_shorts(reply: &Value, lines: &[Line]) -> Vec<Short> {
    let Some(items) = reply.get("shorts").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut taken: Vec<(usize, usize)> = Vec::new();
    let mut out = Vec::new();
    for item in items {
        let pick = (|| {
            let from = usize::try_from(item.get("from")?.as_u64()?).ok()?;
            let to = usize::try_from(item.get("to")?.as_u64()?).ok()?;
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
            if from > to || to >= lines.len() || title.is_empty() {
                return None;
            }
            let length = lines[to]
                .edited_end_us
                .saturating_sub(lines[from].edited_us);
            if !(MIN_US..=MAX_US).contains(&length)
                || lines[to].source_end_us <= lines[from].source_us
                || taken.iter().any(|&(a, b)| from <= b && a <= to)
            {
                return None;
            }
            Some((from, to, title, clean("why", 300)))
        })();
        let Some((from, to, title, reason)) = pick else {
            continue;
        };
        taken.push((from, to));
        out.push(Short {
            id: format!("short-{}", lines[from].source_us),
            title,
            source_start_us: lines[from].source_us,
            source_end_us: lines[to].source_end_us,
            reason,
            layout: Default::default(),
            edited_start_us: None,
            edited_end_us: None,
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
) -> Result<Vec<Short>, String> {
    let lines = lines(words);
    if lines.len() < 2 {
        return Err("The transcript is too short to pick shorts from".into());
    }
    let reply = model.complete_json(SYSTEM_PROMPT, &prompt(&lines))?;
    let shorts = parse_shorts(&reply, &lines);
    if shorts.is_empty() {
        return Err("The AI found no moment that works as a short on its own".into());
    }
    Ok(shorts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const S: u64 = 1_000_000;

    #[test]
    fn picks_are_checked_for_length_order_and_overlap() {
        // Ten lines of 10 s each.
        let lines: Vec<Line> = (0..10)
            .map(|i| Line {
                edited_us: i * 10 * S,
                source_us: 500 * S + i * 10 * S,
                edited_end_us: i * 10 * S + 9 * S,
                source_end_us: 500 * S + i * 10 * S + 9 * S,
                text: format!("line {i}"),
            })
            .collect();
        let reply = json!({ "shorts": [
            { "from": 1, "to": 3, "title": "  The   trick ", "why": "clear tip" },
            { "from": 2, "to": 4, "title": "Overlaps" },
            { "from": 5, "to": 5, "title": "Too short" },
            { "from": 0, "to": 9, "title": "Too long" },
            { "from": 6, "to": 8, "title": "" },
            { "from": 7, "to": 12, "title": "Out of range" },
            { "from": 6, "to": 7, "title": "Ending" },
        ]});
        let shorts = parse_shorts(&reply, &lines);
        let summary: Vec<(&str, u64, u64)> = shorts
            .iter()
            .map(|s| (s.title.as_str(), s.source_start_us, s.source_end_us))
            .collect();
        assert_eq!(
            summary,
            vec![
                ("The trick", 510 * S, 539 * S),
                ("Ending", 560 * S, 579 * S)
            ]
        );
        assert_eq!(shorts[0].reason, "clear tip");
        crate::shorts::validate(&shorts).unwrap();
    }
}
