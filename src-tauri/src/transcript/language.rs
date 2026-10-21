//! The spoken language of a recording. English runs locally (Parakeet) or online; Greek runs
//! online only, through ElevenLabs Scribe or Scribe via OpenRouter: no local model transcribes
//! Greek well enough (Parakeet v3 gets about a third of its words wrong).
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema,
)]
pub enum Language {
    #[default]
    #[serde(rename = "en")]
    English,
    #[serde(rename = "el")]
    Greek,
}

impl Language {
    /// The ISO 639-1 code stored in settings and transcripts, and sent to Scribe.
    pub fn code(self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Greek => "el",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Greek => "Greek",
        }
    }

    /// A code as settings, transcripts or providers write it: ISO 639-1 or 639-3, any case.
    pub fn from_code(code: &str) -> Option<Self> {
        match code.trim().to_ascii_lowercase().as_str() {
            "en" | "eng" | "english" => Some(Language::English),
            "el" | "ell" | "gre" | "greek" => Some(Language::Greek),
            _ => None,
        }
    }

    /// The language a transcript is in; older ones without a code are English.
    pub fn of(transcript: &super::Transcript) -> Self {
        transcript
            .language
            .as_deref()
            .and_then(Self::from_code)
            .unwrap_or_default()
    }

    /// Parakeet is an English model.
    pub fn runs_locally(self) -> bool {
        self == Language::English
    }
}

/// The language setting: detected by the online transcriber (Scribe hears it), or set. Only
/// English can be transcribed on this computer.
#[derive(
    Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, schemars::JsonSchema,
)]
pub enum SpokenLanguage {
    #[default]
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "en")]
    English,
    #[serde(rename = "el")]
    Greek,
}

impl SpokenLanguage {
    /// The language when it is set, `None` when the transcriber detects it.
    pub fn fixed(self) -> Option<Language> {
        match self {
            SpokenLanguage::Auto => None,
            SpokenLanguage::English => Some(Language::English),
            SpokenLanguage::Greek => Some(Language::Greek),
        }
    }

    pub fn label(self) -> &'static str {
        self.fixed().map_or("Auto (detected)", Language::label)
    }

    /// Parakeet transcribes English only, so only a language set to English runs locally.
    pub fn runs_locally(self) -> bool {
        self.fixed().is_some_and(Language::runs_locally)
    }
}

/// `c` without its Greek accent or diaeresis (ά → α, ϊ → ι, ΐ → ι), so the same word matches
/// however a transcriber stressed it. Other characters come back as they are.
pub fn fold_greek_accent(c: char) -> char {
    match c {
        'ά' | 'ὰ' | 'ᾶ' => 'α',
        'έ' | 'ὲ' => 'ε',
        'ή' | 'ὴ' | 'ῆ' => 'η',
        'ί' | 'ὶ' | 'ῖ' | 'ϊ' | 'ΐ' => 'ι',
        'ό' | 'ὸ' => 'ο',
        'ύ' | 'ὺ' | 'ῦ' | 'ϋ' | 'ΰ' => 'υ',
        'ώ' | 'ὼ' | 'ῶ' => 'ω',
        'Ά' => 'Α',
        'Έ' => 'Ε',
        'Ή' => 'Η',
        'Ί' | 'Ϊ' => 'Ι',
        'Ό' => 'Ο',
        'Ύ' | 'Ϋ' => 'Υ',
        'Ώ' => 'Ω',
        _ => c,
    }
}

/// `text` in capitals as Greek writes them: without the stress accent (καλημέρα → ΚΑΛΗΜΕΡΑ).
/// The diaeresis stays (ΠΡΩΪΝΟ). Other scripts are uppercased as usual.
pub fn to_caps(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    // ΐ and ΰ have no capital of their own: uppercasing gives Ι or Υ plus a combining diaeresis
    // and accent. The accent goes and the diaeresis joins its letter again.
    for c in text.to_uppercase().chars() {
        match c {
            '\u{0301}' => {}
            '\u{0308}' if out.ends_with('Ι') => {
                out.pop();
                out.push('Ϊ');
            }
            '\u{0308}' if out.ends_with('Υ') => {
                out.pop();
                out.push('Ϋ');
            }
            'Ά' => out.push('Α'),
            'Έ' => out.push('Ε'),
            'Ή' => out.push('Η'),
            'Ί' => out.push('Ι'),
            'Ό' => out.push('Ο'),
            'Ύ' => out.push('Υ'),
            'Ώ' => out.push('Ω'),
            c => out.push(c),
        }
    }
    out
}

/// True when `text` has a Greek letter: its `;` is then a question mark.
pub fn has_greek(text: &str) -> bool {
    text.chars()
        .any(|c| matches!(c, '\u{0370}'..='\u{03ff}' | '\u{1f00}'..='\u{1fff}'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_from_settings_and_providers() {
        assert_eq!(Language::from_code("ell"), Some(Language::Greek));
        assert_eq!(Language::from_code(" EN "), Some(Language::English));
        assert_eq!(Language::from_code("fr"), None);
        assert_eq!(Language::Greek.code(), "el");
        assert!(!Language::Greek.runs_locally());
    }

    #[test]
    fn accents_fold_and_greek_is_seen() {
        let folded: String = "Λοιπόν, ΐσως".chars().map(fold_greek_accent).collect();
        assert_eq!(folded, "Λοιπον, ισως");
        assert!(has_greek("τι;"));
        assert!(!has_greek("what;"));
    }

    #[test]
    fn greek_capitals_drop_the_accent_and_keep_the_diaeresis() {
        assert_eq!(to_caps("καλημέρα"), "ΚΑΛΗΜΕΡΑ");
        assert_eq!(to_caps("πρωϊνό, ΐσως"), "ΠΡΩΪΝΟ, ΪΣΩΣ");
        assert_eq!(to_caps("don't"), "DON'T");
    }
}
