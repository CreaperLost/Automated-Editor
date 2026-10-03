//! Word-timed transcripts of a project's audio tracks, the providers that produce them, and
//! the edits derived from them (deleted words, filler words and retakes).
//!
//! Word times are source microseconds, like zoom keyframes, so a transcript stays valid
//! across every cut and undo. Edited-time positions are derived from the current
//! retained intervals whenever a transcript is shown or turned into cuts.
pub mod audio;
pub mod edit;
pub mod elevenlabs;
pub mod parakeet;
pub mod provider;
pub mod settings;
pub mod store;

use serde::{Deserialize, Serialize};

pub use edit::{
    SuggestionSource, TranscriptCutSuggestion, TranscriptSuggestionKind, TranscriptView,
};
pub use provider::{ProviderKind, TranscriptionProgress};
pub use settings::{TranscriptSettings, TranscriptSettingsView};

pub const TRANSCRIPT_SCHEMA_VERSION: u32 = 1;
pub const MAX_WORDS: usize = 500_000;
pub const MAX_WORD_CHARS: usize = 256;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WordKind {
    /// A spoken word, with any trailing punctuation the provider attached.
    Word,
    /// A non-speech sound the provider tagged, such as "(laughter)".
    AudioEvent,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptWord {
    /// Stable within one transcript: `w-<index>`.
    pub id: String,
    pub text: String,
    pub kind: WordKind,
    pub source_start_us: u64,
    pub source_end_us: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speaker: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Transcript {
    pub schema_version: u32,
    pub track_id: String,
    pub provider: ProviderKind,
    /// Provider model, e.g. `scribe_v2` or `parakeet-tdt-0.6b-v3`.
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    pub created_at: String,
    pub words: Vec<TranscriptWord>,
    /// Filler and retake suggestions the user rejected, by suggestion id.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dismissed_suggestions: Vec<String>,
    /// Spans an AI pass suggested cutting, by word id. They join the rule-based suggestions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ai_suggestions: Vec<AiSpan>,
    /// `provider/model` of the last AI pass, when there was one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai_model: Option<String>,
    /// Caption edits made on the timeline's caption track, by word id.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub caption_marks: std::collections::BTreeMap<String, CaptionMark>,
}

/// How a word sits in the captions, when the user changed it.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CaptionMark {
    /// A new caption starts at this word.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cue_break: bool,
    /// This word stays in the caption before it, wherever captions would otherwise break.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cue_join: bool,
    /// Heard but not shown in the captions.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,
}

impl CaptionMark {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

/// A run of words an AI pass suggested cutting, from `first_word_id` to `last_word_id`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiSpan {
    pub kind: TranscriptSuggestionKind,
    pub first_word_id: String,
    pub last_word_id: String,
    /// The model's short explanation, shown in the review list.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
}

pub const MAX_AI_SUGGESTIONS: usize = 20_000;

pub const MAX_DISMISSED_SUGGESTIONS: usize = 10_000;

impl Transcript {
    pub fn new(
        track_id: String,
        provider: ProviderKind,
        model: String,
        language: Option<String>,
        mut words: Vec<TranscriptWord>,
    ) -> Self {
        words.sort_by_key(|w| (w.source_start_us, w.source_end_us));
        for (i, word) in words.iter_mut().enumerate() {
            word.id = format!("w-{i}");
        }
        Self {
            schema_version: TRANSCRIPT_SCHEMA_VERSION,
            track_id,
            provider,
            model,
            language,
            created_at: chrono::Utc::now().to_rfc3339(),
            words,
            dismissed_suggestions: Vec::new(),
            ai_suggestions: Vec::new(),
            ai_model: None,
            caption_marks: Default::default(),
        }
    }

    /// The caption mark of a word (all off when it has none).
    pub fn caption_mark(&self, word_id: &str) -> CaptionMark {
        self.caption_marks.get(word_id).copied().unwrap_or_default()
    }

    /// Changes one word's caption mark; an all-off mark is dropped.
    pub fn set_caption_mark(
        &mut self,
        word_id: &str,
        change: impl FnOnce(&mut CaptionMark),
    ) -> Result<(), String> {
        if !self.words.iter().any(|w| w.id == word_id) {
            return Err("Unknown word".into());
        }
        let mut mark = self.caption_mark(word_id);
        change(&mut mark);
        if mark.is_default() {
            self.caption_marks.remove(word_id);
        } else {
            self.caption_marks.insert(word_id.to_string(), mark);
        }
        Ok(())
    }

    /// Replaces the words `word_ids` (in order, side by side) with `text`. The same number
    /// of words keeps their timing; otherwise the new words share the old span by length.
    pub fn replace_words(&mut self, word_ids: &[String], text: &str) -> Result<(), String> {
        let tokens: Vec<&str> = text.split_whitespace().collect();
        if tokens.is_empty() {
            return Err("A caption cannot be empty; hide it instead".into());
        }
        if tokens.iter().any(|t| t.chars().count() > MAX_WORD_CHARS) {
            return Err("A word is too long".into());
        }
        let positions: Vec<usize> = word_ids
            .iter()
            .map(|id| {
                self.words
                    .iter()
                    .position(|w| &w.id == id)
                    .ok_or_else(|| "Unknown word".to_string())
            })
            .collect::<Result<_, _>>()?;
        if positions.is_empty() || positions.windows(2).any(|p| p[1] <= p[0]) {
            return Err("Choose the caption's words in order".into());
        }
        if tokens.len() == positions.len() {
            for (&at, token) in positions.iter().zip(&tokens) {
                self.words[at].text = token.to_string();
            }
            return Ok(());
        }
        let first = &self.words[positions[0]];
        let start = first.source_start_us;
        let end = self.words[*positions.last().unwrap()]
            .source_end_us
            .max(start + 1);
        let speaker = first.speaker.clone();
        let weights: Vec<u64> = tokens
            .iter()
            .map(|t| t.chars().count() as u64 + 1)
            .collect();
        let total: u64 = weights.iter().sum();
        let mut next_id = self
            .words
            .iter()
            .filter_map(|w| w.id.strip_prefix("w-c").and_then(|n| n.parse::<u64>().ok()))
            .max()
            .map_or(0, |n| n + 1);
        let mut cursor = start;
        let mut done = 0u64;
        let mut replacement = Vec::with_capacity(tokens.len());
        for (i, (token, weight)) in tokens.iter().zip(&weights).enumerate() {
            done += weight;
            let word_end = start + (end - start) * done / total;
            let id = match word_ids.get(i) {
                Some(id) => id.clone(),
                None => {
                    next_id += 1;
                    format!("w-c{}", next_id - 1)
                }
            };
            replacement.push(TranscriptWord {
                id,
                text: token.to_string(),
                kind: WordKind::Word,
                source_start_us: cursor,
                source_end_us: word_end.max(cursor),
                confidence: None,
                speaker: speaker.clone(),
            });
            cursor = word_end;
        }
        for id in word_ids.iter().skip(tokens.len()) {
            self.caption_marks.remove(id);
        }
        // The listed words go; the new ones take the first one's place.
        for &at in positions.iter().rev() {
            self.words.remove(at);
        }
        let at = positions[0];
        self.words.splice(at..at, replacement);
        self.validate()
    }

    /// Moves and stretches the words `word_ids` to source span `[start, end)`, keeping their
    /// spacing, and keeping every word in time order.
    pub fn retime_words(
        &mut self,
        word_ids: &[String],
        start: u64,
        end: u64,
    ) -> Result<(), String> {
        if end <= start {
            return Err("A caption needs some length".into());
        }
        let positions: Vec<usize> = word_ids
            .iter()
            .map(|id| {
                self.words
                    .iter()
                    .position(|w| &w.id == id)
                    .ok_or_else(|| "Unknown word".to_string())
            })
            .collect::<Result<_, _>>()?;
        let (Some(&first), Some(&last)) = (positions.first(), positions.last()) else {
            return Err("Choose the caption's words".into());
        };
        let old_start = self.words[first].source_start_us;
        let old_end = self.words[last].source_end_us.max(old_start + 1);
        let scale = |t: u64| {
            start
                + ((t.saturating_sub(old_start)) as u128 * (end - start) as u128
                    / (old_end - old_start) as u128) as u64
        };
        let before = first.checked_sub(1).map(|i| self.words[i].source_start_us);
        let after = self.words.get(last + 1).map(|w| w.source_start_us);
        if before.is_some_and(|b| start < b)
            || after.is_some_and(|a| scale(self.words[last].source_start_us) > a)
        {
            return Err("A caption cannot move past the words around it".into());
        }
        for &at in &positions {
            let word = &mut self.words[at];
            word.source_start_us = scale(word.source_start_us);
            word.source_end_us = scale(word.source_end_us).max(word.source_start_us);
        }
        self.validate()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != TRANSCRIPT_SCHEMA_VERSION {
            return Err(format!(
                "Unsupported transcript schema version: {}",
                self.schema_version
            ));
        }
        if self.words.len() > MAX_WORDS {
            return Err("Transcript has too many words".into());
        }
        if self.dismissed_suggestions.len() > MAX_DISMISSED_SUGGESTIONS
            || self.dismissed_suggestions.iter().any(|id| id.len() > 64)
        {
            return Err("Transcript has too many rejected suggestions".into());
        }
        if self.ai_suggestions.len() > MAX_AI_SUGGESTIONS
            || self.ai_suggestions.iter().any(|s| {
                s.first_word_id.len() > 32 || s.last_word_id.len() > 32 || s.reason.len() > 512
            })
            || self.ai_model.as_ref().is_some_and(|m| m.len() > 256)
        {
            return Err("Transcript has invalid AI suggestions".into());
        }
        let mut previous_start = 0u64;
        let mut ids = std::collections::HashSet::with_capacity(self.words.len());
        for word in &self.words {
            if word.source_end_us < word.source_start_us {
                return Err("Transcript word ends before it starts".into());
            }
            if word.source_start_us < previous_start {
                return Err("Transcript words must be in time order".into());
            }
            if word.text.chars().count() > MAX_WORD_CHARS {
                return Err("Transcript word is too long".into());
            }
            if !ids.insert(word.id.as_str()) {
                return Err("Duplicate transcript word id".into());
            }
            previous_start = word.source_start_us;
        }
        Ok(())
    }

    /// Replaces one word's text, e.g. to fix a misheard name before it shows in captions.
    pub fn set_word_text(&mut self, word_id: &str, text: &str) -> Result<(), String> {
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        if text.is_empty() {
            return Err("A word cannot be empty; cut it instead".into());
        }
        if text.chars().count() > MAX_WORD_CHARS {
            return Err("That text is too long for one word".into());
        }
        let word = self
            .words
            .iter_mut()
            .find(|w| w.id == word_id)
            .ok_or("Unknown word")?;
        word.text = text;
        Ok(())
    }

    /// Rejects (or restores) suggestions so they stop showing in the review list.
    pub fn set_dismissed(&mut self, ids: &[String], dismissed: bool) -> Result<(), String> {
        for id in ids {
            if id.is_empty() || id.len() > 64 {
                return Err("Invalid suggestion id".into());
            }
            let present = self.dismissed_suggestions.iter().position(|d| d == id);
            match (dismissed, present) {
                (true, None) => self.dismissed_suggestions.push(id.clone()),
                (false, Some(i)) => {
                    self.dismissed_suggestions.remove(i);
                }
                _ => {}
            }
        }
        if self.dismissed_suggestions.len() > MAX_DISMISSED_SUGGESTIONS {
            return Err("Too many rejected suggestions".into());
        }
        Ok(())
    }

    pub fn text(&self) -> String {
        let mut out = String::new();
        for word in &self.words {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(&word.text);
        }
        out
    }
}

/// Converts provider seconds to microseconds, clamping negative and non-finite values to 0.
pub(crate) fn seconds_to_us(seconds: f64) -> u64 {
    if seconds.is_finite() && seconds > 0.0 {
        (seconds * 1_000_000.0).round() as u64
    } else {
        0
    }
}

#[cfg(test)]
pub(crate) fn test_word(text: &str, start_ms: u64, end_ms: u64) -> TranscriptWord {
    TranscriptWord {
        id: String::new(),
        text: text.into(),
        kind: WordKind::Word,
        source_start_us: start_ms * 1_000,
        source_end_us: end_ms * 1_000,
        confidence: None,
        speaker: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sorts_words_and_assigns_ids() {
        let t = Transcript::new(
            "mic".into(),
            ProviderKind::ElevenLabs,
            "scribe_v2".into(),
            Some("en".into()),
            vec![test_word("world", 500, 900), test_word("hello", 0, 400)],
        );
        assert_eq!(t.words[0].text, "hello");
        assert_eq!(t.words[0].id, "w-0");
        assert_eq!(t.words[1].id, "w-1");
        assert_eq!(t.text(), "hello world");
        t.validate().unwrap();
    }

    #[test]
    fn validate_rejects_reversed_words() {
        let mut t = Transcript::new(
            "mic".into(),
            ProviderKind::Parakeet,
            "m".into(),
            None,
            vec![test_word("a", 0, 100)],
        );
        t.words[0].source_end_us = 0;
        t.words[0].source_start_us = 10;
        assert!(t.validate().is_err());
    }

    #[test]
    fn word_text_edits_and_dismissals() {
        let mut t = Transcript::new(
            "mic".into(),
            ProviderKind::ElevenLabs,
            "scribe_v2".into(),
            None,
            vec![test_word("Jorge", 0, 300)],
        );
        t.set_word_text("w-0", "  George,\n ").unwrap();
        assert_eq!(t.words[0].text, "George,");
        assert!(t.set_word_text("w-0", "   ").is_err());
        assert!(t.set_word_text("w-9", "x").is_err());
        t.set_dismissed(&["filler-w-1-w-1".into()], true).unwrap();
        t.set_dismissed(&["filler-w-1-w-1".into()], true).unwrap();
        assert_eq!(t.dismissed_suggestions.len(), 1);
        t.validate().unwrap();
        t.set_dismissed(&["filler-w-1-w-1".into()], false).unwrap();
        assert!(t.dismissed_suggestions.is_empty());
    }

    #[test]
    fn seconds_conversion_clamps() {
        assert_eq!(seconds_to_us(1.25), 1_250_000);
        assert_eq!(seconds_to_us(-1.0), 0);
        assert_eq!(seconds_to_us(f64::NAN), 0);
    }
}
