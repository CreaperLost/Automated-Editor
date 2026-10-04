//! AI chapter suggestions. The kept transcript goes to the model as numbered, time-stamped
//! lines; it answers with the lines where new topics start and a title for each. Chapters are
//! anchored at the first word of their line, in source time.
use super::client::JsonModel;
use crate::chapters::{Chapter, MAX_TITLE_CHARS};
use crate::transcript::edit::TranscriptViewWord;
use crate::transcript::WordKind;
use serde_json::Value;

const LINE_PAUSE_US: u64 = 1_000_000;
const LINE_MAX_US: u64 = 20_000_000;
const LINE_SENTENCE_MIN_WORDS: usize = 8;
/// Longer transcripts are merged into fewer, longer lines to keep the request a sane size.
const MAX_LINES: usize = 1_200;
/// YouTube needs chapters at least 10 seconds long.
pub const MIN_CHAPTER_US: u64 = 10_000_000;
const MAX_AI_CHAPTERS: usize = 50;

pub const SYSTEM_PROMPT: &str = "You write YouTube chapters for videos from their transcript. \
The transcript is numbered lines in the form [index] m:ss text, in playback order. \
Split it where the topic or activity changes. Use about one chapter per 2 to 5 minutes, at least 3 when the video is long enough, and never two chapters less than 30 seconds apart. \
The first chapter starts at line 0. Titles are short (2 to 6 words), specific, in the transcript's language, without timestamps, numbering or emoji. \
Answer with a JSON object only: {\"chapters\":[{\"line\":<index>,\"title\":\"<title>\"}]}.";

/// A run of kept words shown to the model as one line.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub edited_us: u64,
    pub source_us: u64,
    /// End of the line's last word.
    pub edited_end_us: u64,
    pub source_end_us: u64,
    pub text: String,
}

fn format_time(us: u64) -> String {
    let total = us / 1_000_000;
    let (h, m, s) = (total / 3600, (total / 60) % 60, total % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// Lines from the kept words of a transcript view (already in playback order).
pub fn lines(words: &[TranscriptViewWord]) -> Vec<Line> {
    let mut out: Vec<Line> = Vec::new();
    let mut count = 0usize;
    let mut last_end: Option<u64> = None;
    let mut last_text = String::new();
    for w in words {
        let (Some(start), Some(end)) = (w.edited_start_us, w.edited_end_us) else {
            continue;
        };
        if w.word.kind != WordKind::Word {
            continue;
        }
        let new_line = match (out.last(), last_end) {
            (Some(line), Some(prev_end)) => {
                start.saturating_sub(prev_end) >= LINE_PAUSE_US
                    || start < prev_end.saturating_sub(1)
                    || start.saturating_sub(line.edited_us) > LINE_MAX_US
                    || (count >= LINE_SENTENCE_MIN_WORDS && last_text.ends_with(['.', '?', '!']))
            }
            _ => true,
        };
        if new_line {
            out.push(Line {
                edited_us: start,
                source_us: w.word.source_start_us,
                edited_end_us: end,
                source_end_us: w.word.source_end_us,
                text: String::new(),
            });
            count = 0;
        }
        let line = out.last_mut().unwrap();
        if !line.text.is_empty() {
            line.text.push(' ');
        }
        line.text.push_str(&w.word.text);
        line.edited_end_us = end;
        line.source_end_us = w.word.source_end_us;
        count += 1;
        last_end = Some(end);
        last_text = w.word.text.clone();
    }
    while out.len() > MAX_LINES {
        out = out
            .chunks(2)
            .map(|pair| Line {
                edited_us: pair[0].edited_us,
                source_us: pair[0].source_us,
                edited_end_us: pair[pair.len() - 1].edited_end_us,
                source_end_us: pair[pair.len() - 1].source_end_us,
                text: pair
                    .iter()
                    .map(|l| l.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
            })
            .collect();
    }
    out
}

pub fn prompt(lines: &[Line]) -> String {
    lines
        .iter()
        .enumerate()
        .map(|(i, line)| format!("[{i}] {} {}", format_time(line.edited_us), line.text))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Turns a reply into chapters: known lines only, in order, at least 10 s apart, the first
/// at line 0, with clean one-line titles.
pub fn parse_chapters(reply: &Value, lines: &[Line]) -> Vec<Chapter> {
    let mut picked: Vec<(usize, String)> = reply
        .get("chapters")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let line = usize::try_from(item.get("line")?.as_u64()?).ok()?;
                    let title: String = item
                        .get("title")?
                        .as_str()?
                        .chars()
                        .filter(|c| !c.is_control())
                        .collect::<String>()
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .chars()
                        .take(MAX_TITLE_CHARS)
                        .collect();
                    (line < lines.len() && !title.is_empty()).then_some((line, title))
                })
                .collect()
        })
        .unwrap_or_default();
    picked.sort_by_key(|(line, _)| *line);
    picked.dedup_by_key(|(line, _)| *line);
    if let Some(first) = picked.first_mut() {
        first.0 = 0;
    }
    let mut chapters: Vec<Chapter> = Vec::new();
    let mut last_at: Option<u64> = None;
    for (line, title) in picked {
        let at = lines[line].edited_us;
        if last_at.is_some_and(|prev| at < prev + MIN_CHAPTER_US) {
            continue;
        }
        last_at = Some(if chapters.is_empty() { 0 } else { at });
        chapters.push(Chapter {
            id: format!("ch-{}", lines[line].source_us),
            source_us: lines[line].source_us,
            title,
            // The caller knows whose words these are.
            media: None,
            edited_us: None,
        });
        if chapters.len() == MAX_AI_CHAPTERS {
            break;
        }
    }
    chapters
}

pub fn suggest(
    model: &mut dyn JsonModel,
    words: &[TranscriptViewWord],
) -> Result<Vec<Chapter>, String> {
    let lines = lines(words);
    if lines.len() < 2 {
        return Err("The transcript is too short for chapters".into());
    }
    let reply = model.complete_json(SYSTEM_PROMPT, &prompt(&lines))?;
    let chapters = parse_chapters(&reply, &lines);
    if chapters.is_empty() {
        return Err("The AI did not suggest any usable chapters".into());
    }
    Ok(chapters)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcript::TranscriptWord;
    use serde_json::json;

    const S: u64 = 1_000_000;

    /// Words every 0.5 s, with a 2 s pause before each `breaks` index.
    fn view(texts: &[&str], breaks: &[usize]) -> Vec<TranscriptViewWord> {
        let mut t = 0;
        texts
            .iter()
            .enumerate()
            .map(|(i, text)| {
                if breaks.contains(&i) {
                    t += 2 * S;
                }
                let start = t;
                t += S / 2;
                TranscriptViewWord {
                    word: TranscriptWord {
                        id: format!("w-{i}"),
                        text: text.to_string(),
                        kind: WordKind::Word,
                        source_start_us: start + 100 * S,
                        source_end_us: start + 100 * S + 400_000,
                        confidence: None,
                        speaker: None,
                    },
                    edited_start_us: Some(start),
                    edited_end_us: Some(start + 400_000),
                    caption_hidden: false,
                    caption_break: false,
                    caption_join: false,
                }
            })
            .collect()
    }

    #[test]
    fn lines_break_at_pauses_and_show_times() {
        let words = view(&["Hello", "there.", "Now", "we", "build"], &[2]);
        let lines = lines(&words);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "Hello there.");
        assert_eq!(lines[1].edited_us, 3 * S);
        assert_eq!(lines[1].source_us, 103 * S);
        assert_eq!(
            prompt(&lines),
            "[0] 0:00 Hello there.\n[1] 0:03 Now we build"
        );
        assert_eq!(format_time(3_725 * S), "1:02:05");
    }

    #[test]
    fn chapters_are_validated_ordered_and_spaced() {
        let lines: Vec<Line> = (0..6)
            .map(|i| Line {
                edited_us: i * 8 * S,
                source_us: 1_000 + i,
                edited_end_us: i * 8 * S + 7 * S,
                source_end_us: 2_000 + i,
                text: format!("line {i}"),
            })
            .collect();
        let reply = json!({ "chapters": [
            { "line": 3, "title": "  Building\\n the  base " },
            { "line": 1, "title": "Too close to the start" },
            { "line": 2, "title": "Setup" },
            { "line": 9, "title": "Out of range" },
            { "line": 5, "title": "" },
            { "line": 4, "title": "Too close to Building" },
        ]});
        let chapters = parse_chapters(&reply, &lines);
        let summary: Vec<(u64, &str)> = chapters
            .iter()
            .map(|c| (c.source_us, c.title.as_str()))
            .collect();
        // The first pick moves to line 0; picks under 10 s after the previous are dropped.
        assert_eq!(
            summary,
            vec![
                (1_000, "Too close to the start"),
                (1_002, "Setup"),
                (1_004, "Too close to Building")
            ]
        );
        crate::chapters::validate(&chapters).unwrap();
    }
}
