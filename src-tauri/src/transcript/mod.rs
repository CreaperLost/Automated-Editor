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

pub use edit::{TranscriptCutSuggestion, TranscriptSuggestionKind, TranscriptView};
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
}

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
        }
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
