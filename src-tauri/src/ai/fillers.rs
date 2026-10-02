//! AI filler and retake detection. The kept words go to the model in numbered chunks, with
//! pause lengths, and it answers with word-index spans. Spans are checked against the chunk
//! and stored by word id, so they become ordinary `<kind>-<first>-<last>` suggestions that
//! the review list, "remove all" and rejections already handle.
use super::client::JsonModel;
use crate::transcript::{AiSpan, TranscriptSuggestionKind, TranscriptWord};
use serde_json::Value;

/// Words per request, and how many of them repeat the end of the previous chunk so a retake
/// that straddles the boundary is still seen whole.
const CHUNK_WORDS: usize = 300;
const CHUNK_OVERLAP: usize = 40;
/// Pauses at least this long are shown to the model; they mark abandoned sentences.
const SHOWN_PAUSE_US: u64 = 500_000;
const MAX_FILLER_WORDS: usize = 4;
const MAX_RETAKE_WORDS: usize = 60;
/// Upper bound on requests per run (about 75 000 words).
pub const MAX_CHUNKS: usize = 300;

pub const SYSTEM_PROMPT: &str = "You edit spoken-word video transcripts (tutorials, gaming, talks). \
Find words that should be cut:\n\
- \"filler\": filler words or sounds that add nothing in context, such as um, uh, \"like\", \"you know\", \"I mean\", \"so\", \"basically\", \"kind of\". Only mark a word as filler when removing it leaves the sentence correct and meaning unchanged; keep \"like\" or \"so\" when they carry meaning.\n\
- \"retake\": a false start or abandoned attempt that the speaker restarts, such as a sentence cut off and said again. Mark the abandoned attempt only, never the take that follows it, and never content that is said only once.\n\
Words are written as [index]word. A marker like (pause 1.2s) is a silence between words.\n\
Answer with a JSON object only: {\"cuts\":[{\"type\":\"filler\"|\"retake\",\"from\":<first index>,\"to\":<last index>,\"why\":\"<a few words>\"}]}. \
Indexes are inclusive. Use {\"cuts\":[]} when nothing should be cut. Be conservative: a wrong cut is worse than a missed one.";

/// The user message for words `words[range]`, numbered by their position in `words`.
pub fn chunk_prompt(words: &[&TranscriptWord], range: std::ops::Range<usize>) -> String {
    let mut out = String::with_capacity((range.end - range.start) * 10);
    for i in range.clone() {
        if i > range.start {
            let gap = words[i]
                .source_start_us
                .saturating_sub(words[i - 1].source_end_us);
            if gap >= SHOWN_PAUSE_US {
                out.push_str(&format!(" (pause {:.1}s)", gap as f64 / 1e6));
            }
            out.push(' ');
        }
        out.push_str(&format!("[{i}]{}", words[i].text));
    }
    out
}

/// Chunk ranges covering `len` words, each overlapping the previous one.
pub fn chunks(len: usize) -> Vec<std::ops::Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0;
    while start < len {
        let end = (start + CHUNK_WORDS).min(len);
        out.push(start..end);
        if end == len {
            break;
        }
        start = end - CHUNK_OVERLAP;
    }
    out
}

/// The valid spans in one reply. Anything outside the chunk, reversed, too long or of an
/// unknown type is skipped rather than trusted.
pub fn parse_spans(
    reply: &Value,
    words: &[&TranscriptWord],
    range: std::ops::Range<usize>,
) -> Vec<AiSpan> {
    let Some(cuts) = reply.get("cuts").and_then(Value::as_array) else {
        return Vec::new();
    };
    cuts.iter()
        .filter_map(|cut| {
            let kind = match cut.get("type")?.as_str()? {
                "filler" => TranscriptSuggestionKind::Filler,
                "retake" => TranscriptSuggestionKind::Retake,
                _ => return None,
            };
            let from = usize::try_from(cut.get("from")?.as_u64()?).ok()?;
            let to = usize::try_from(cut.get("to")?.as_u64()?).ok()?;
            if from > to || from < range.start || to >= range.end {
                return None;
            }
            let limit = match kind {
                TranscriptSuggestionKind::Filler => MAX_FILLER_WORDS,
                TranscriptSuggestionKind::Retake => MAX_RETAKE_WORDS,
            };
            if to - from + 1 > limit {
                return None;
            }
            let reason = cut
                .get("why")
                .and_then(Value::as_str)
                .unwrap_or("")
                .chars()
                .take(120)
                .collect();
            Some(AiSpan {
                kind,
                first_word_id: words[from].id.clone(),
                last_word_id: words[to].id.clone(),
                reason,
            })
        })
        .collect()
}

/// Runs the model over `words` (kept spoken words, in recording order). `progress` gets the
/// fraction done; returning false from `keep_going` stops early with an error.
pub fn detect(
    model: &mut dyn JsonModel,
    words: &[&TranscriptWord],
    progress: &mut dyn FnMut(f32),
    keep_going: &dyn Fn() -> bool,
) -> Result<Vec<AiSpan>, String> {
    let ranges = chunks(words.len());
    if ranges.is_empty() {
        return Err("There are no kept words to check".into());
    }
    if ranges.len() > MAX_CHUNKS {
        return Err("This transcript is too long for one AI pass".into());
    }
    let mut spans: Vec<AiSpan> = Vec::new();
    for (n, range) in ranges.iter().enumerate() {
        if !keep_going() {
            return Err("Cancelled".into());
        }
        progress(n as f32 / ranges.len() as f32);
        let reply = model.complete_json(SYSTEM_PROMPT, &chunk_prompt(words, range.clone()))?;
        for span in parse_spans(&reply, words, range.clone()) {
            // The overlap sends some words twice: keep each span once.
            if !spans.iter().any(|s| {
                s.first_word_id == span.first_word_id && s.last_word_id == span.last_word_id
            }) {
                spans.push(span);
            }
        }
    }
    progress(1.0);
    Ok(spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcript::WordKind;
    use serde_json::json;

    fn words(texts: &[&str]) -> Vec<TranscriptWord> {
        texts
            .iter()
            .enumerate()
            .map(|(i, text)| TranscriptWord {
                id: format!("w-{i}"),
                text: text.to_string(),
                kind: WordKind::Word,
                // A long pause before word 4.
                source_start_us: i as u64 * 400_000 + if i >= 4 { 1_000_000 } else { 0 },
                source_end_us: i as u64 * 400_000 + 300_000 + if i >= 4 { 1_000_000 } else { 0 },
                confidence: None,
                speaker: None,
            })
            .collect()
    }

    #[test]
    fn prompt_numbers_words_and_marks_pauses() {
        let owned = words(&[
            "So", "the", "next", "step", "So", "the", "next", "step", "is",
        ]);
        let refs: Vec<&TranscriptWord> = owned.iter().collect();
        let prompt = chunk_prompt(&refs, 0..refs.len());
        assert!(prompt.starts_with("[0]So [1]the [2]next [3]step (pause 1.1s) [4]So"));
        assert!(prompt.ends_with("[8]is"));
    }

    #[test]
    fn chunks_overlap_and_cover_everything() {
        assert!(chunks(0).is_empty());
        assert_eq!(chunks(10), vec![0..10]);
        let ranges = chunks(700);
        assert_eq!(ranges, vec![0..300, 260..560, 520..700]);
    }

    #[test]
    fn spans_are_validated_against_the_chunk() {
        let owned = words(&["um", "so", "I", "I", "think", "like", "yes", "ok", "a", "b"]);
        let refs: Vec<&TranscriptWord> = owned.iter().collect();
        let reply = json!({ "cuts": [
            { "type": "filler", "from": 0, "to": 0, "why": "um" },
            { "type": "retake", "from": 2, "to": 2 },
            { "type": "filler", "from": 5, "to": 4 },
            { "type": "filler", "from": 0, "to": 6 },
            { "type": "filler", "from": 9, "to": 12 },
            { "type": "joke", "from": 1, "to": 1 },
            { "type": "filler", "from": -1, "to": 1 },
        ]});
        let spans = parse_spans(&reply, &refs, 0..10);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].first_word_id, "w-0");
        assert_eq!(spans[0].reason, "um");
        assert_eq!(spans[1].kind, TranscriptSuggestionKind::Retake);
        assert_eq!(spans[1].first_word_id, "w-2");
        assert!(parse_spans(&json!({"nope": 1}), &refs, 0..10).is_empty());
    }

    struct Canned {
        replies: Vec<Value>,
        prompts: Vec<String>,
    }

    impl JsonModel for Canned {
        fn complete_json(&mut self, system: &str, user: &str) -> Result<Value, String> {
            assert!(system.contains("\"cuts\""));
            self.prompts.push(user.to_string());
            Ok(self.replies.remove(0))
        }
        fn describe(&self) -> String {
            "test/canned".into()
        }
    }

    #[test]
    fn detect_runs_every_chunk_and_dedupes_the_overlap() {
        let texts: Vec<String> = (0..320).map(|i| format!("w{i}")).collect();
        let text_refs: Vec<&str> = texts.iter().map(String::as_str).collect();
        let owned = words(&text_refs);
        let refs: Vec<&TranscriptWord> = owned.iter().collect();
        // Both chunks report the same span inside the overlap (260..300).
        let span = json!({ "cuts": [{ "type": "filler", "from": 270, "to": 270 }] });
        let mut model = Canned {
            replies: vec![span.clone(), span],
            prompts: Vec::new(),
        };
        let mut seen = Vec::new();
        let spans = detect(&mut model, &refs, &mut |f| seen.push(f), &|| true).unwrap();
        assert_eq!(model.prompts.len(), 2);
        assert!(model.prompts[1].starts_with("[260]"));
        assert_eq!(spans.len(), 1);
        assert_eq!(seen.last(), Some(&1.0));
        let mut model = Canned {
            replies: vec![],
            prompts: Vec::new(),
        };
        assert_eq!(
            detect(&mut model, &refs, &mut |_| {}, &|| false).unwrap_err(),
            "Cancelled"
        );
    }
}
