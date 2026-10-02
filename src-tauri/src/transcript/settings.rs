//! App-wide transcription settings. The ElevenLabs key never goes in a project or in the
//! settings file: it lives in the OS credential store (Windows Credential Manager, macOS
//! Keychain), or in an owner-only file on other platforms. `ELEVENLABS_API_KEY` overrides it.
use super::{parakeet, ProviderKind};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const SETTINGS_FILE: &str = "transcription.json";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct TranscriptSettings {
    pub provider: ProviderKind,
    /// ISO language code passed to the provider; empty lets it detect the language.
    pub language: String,
    /// ElevenLabs model id; empty uses the default.
    pub scribe_model: String,
    /// Words to bias recognition towards, such as game or product names.
    pub keyterms: Vec<String>,
    /// Folder holding the Parakeet ONNX files; empty uses the app data folder.
    pub parakeet_model_dir: String,
}

impl Default for TranscriptSettings {
    fn default() -> Self {
        Self {
            provider: ProviderKind::Parakeet,
            language: "en".into(),
            scribe_model: String::new(),
            keyterms: Vec::new(),
            parakeet_model_dir: String::new(),
        }
    }
}

impl TranscriptSettings {
    pub fn validate(&self) -> Result<(), String> {
        if self.language.len() > 16
            || !self
                .language
                .chars()
                .all(|c| c.is_ascii_alphabetic() || c == '-')
        {
            return Err("Language must be a short code such as en".into());
        }
        if self.scribe_model.len() > 64 {
            return Err("Model id is too long".into());
        }
        if self.keyterms.len() > 100 || self.keyterms.iter().any(|k| k.chars().count() > 50) {
            return Err("Use up to 100 key terms of 50 characters or fewer".into());
        }
        if self.parakeet_model_dir.len() > 4096 {
            return Err("Model folder path is too long".into());
        }
        Ok(())
    }

    pub fn parakeet_dir(&self) -> PathBuf {
        if self.parakeet_model_dir.trim().is_empty() {
            parakeet::default_model_dir()
        } else {
            PathBuf::from(self.parakeet_model_dir.trim())
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSettingsView {
    pub settings: TranscriptSettings,
    /// Where the ElevenLabs key comes from: `keychain`, `file`, `environment`, or none.
    pub elevenlabs_key_source: Option<String>,
    pub parakeet_available: bool,
    pub parakeet_accelerator: String,
    pub parakeet_model_dir: String,
    pub parakeet_model_present: bool,
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("AeroEdits")
}

pub fn load_settings(dir: &Path) -> TranscriptSettings {
    fs::read(dir.join(SETTINGS_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<TranscriptSettings>(&bytes).ok())
        .filter(|s| s.validate().is_ok())
        .unwrap_or_default()
}

pub fn save_settings(dir: &Path, settings: &TranscriptSettings) -> Result<(), String> {
    settings.validate()?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
    std::io::Write::write_all(&mut temp, &bytes).map_err(|e| e.to_string())?;
    temp.persist(dir.join(SETTINGS_FILE))
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn settings_view(dir: &Path) -> TranscriptSettingsView {
    let settings = load_settings(dir);
    let model_dir = settings.parakeet_dir();
    TranscriptSettingsView {
        elevenlabs_key_source: api_key(dir).map(|(_, source)| source.to_string()),
        parakeet_available: parakeet::compiled_in(),
        parakeet_accelerator: parakeet::accelerator().into(),
        parakeet_model_present: parakeet::model_present(&model_dir),
        parakeet_model_dir: model_dir.to_string_lossy().into_owned(),
        settings,
    }
}

pub const ELEVENLABS_KEY: crate::secrets::KeySpec = crate::secrets::KeySpec {
    env: "ELEVENLABS_API_KEY",
    name: "elevenlabs-api-key",
};

/// The ElevenLabs key and where it came from.
pub fn api_key(dir: &Path) -> Option<(String, &'static str)> {
    crate::secrets::get(dir, ELEVENLABS_KEY)
}

/// Stores `key`, or removes the stored key when `key` is empty.
pub fn set_api_key(dir: &Path, key: &str) -> Result<(), String> {
    crate::secrets::set(dir, ELEVENLABS_KEY, key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_roundtrip_and_fall_back_to_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_settings(dir.path()), TranscriptSettings::default());
        let mut s = TranscriptSettings::default();
        s.provider = ProviderKind::ElevenLabs;
        s.keyterms = vec!["Factorio".into()];
        save_settings(dir.path(), &s).unwrap();
        assert_eq!(load_settings(dir.path()), s);
        fs::write(dir.path().join(SETTINGS_FILE), b"{not json").unwrap();
        assert_eq!(load_settings(dir.path()), TranscriptSettings::default());
    }

    #[test]
    fn invalid_settings_are_rejected() {
        let mut s = TranscriptSettings::default();
        s.language = "en; rm".into();
        assert!(s.validate().is_err());
    }
}
