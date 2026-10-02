//! Cloud language models for transcript work (filler and retake detection, later chapters
//! and shorts). OpenAI and OpenRouter both speak the OpenAI chat-completions API, so one
//! client covers them; only the base URL, key and model names differ.
//!
//! Only transcript text (words and pause lengths) is sent, never audio or video, and only
//! when the user asks for an AI pass.
pub mod client;
pub mod fillers;

use crate::secrets::KeySpec;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

const SETTINGS_FILE: &str = "ai.json";

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AiProvider {
    #[default]
    OpenAi,
    OpenRouter,
}

impl AiProvider {
    pub fn base_url(self) -> &'static str {
        match self {
            AiProvider::OpenAi => "https://api.openai.com/v1",
            AiProvider::OpenRouter => "https://openrouter.ai/api/v1",
        }
    }

    pub fn default_model(self) -> &'static str {
        match self {
            AiProvider::OpenAi => "gpt-4.1-mini",
            AiProvider::OpenRouter => "openai/gpt-4.1-mini",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            AiProvider::OpenAi => "OpenAI",
            AiProvider::OpenRouter => "OpenRouter",
        }
    }

    pub fn key_spec(self) -> KeySpec {
        match self {
            AiProvider::OpenAi => KeySpec {
                env: "OPENAI_API_KEY",
                name: "openai-api-key",
            },
            AiProvider::OpenRouter => KeySpec {
                env: "OPENROUTER_API_KEY",
                name: "openrouter-api-key",
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AiSettings {
    pub provider: AiProvider,
    /// Model id for OpenAI; empty uses the default.
    pub openai_model: String,
    /// Model id for OpenRouter, e.g. `anthropic/claude-sonnet-4.5`; empty uses the default.
    pub openrouter_model: String,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            provider: AiProvider::OpenAi,
            openai_model: String::new(),
            openrouter_model: String::new(),
        }
    }
}

impl AiSettings {
    pub fn validate(&self) -> Result<(), String> {
        for model in [&self.openai_model, &self.openrouter_model] {
            if model.len() > 128 || model.chars().any(|c| c.is_whitespace() || c.is_control()) {
                return Err("A model id is one word of at most 128 characters".into());
            }
        }
        Ok(())
    }

    /// The model the current provider uses.
    pub fn model(&self) -> String {
        let chosen = match self.provider {
            AiProvider::OpenAi => &self.openai_model,
            AiProvider::OpenRouter => &self.openrouter_model,
        };
        if chosen.trim().is_empty() {
            self.provider.default_model().to_string()
        } else {
            chosen.trim().to_string()
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiSettingsView {
    pub settings: AiSettings,
    /// Where each provider's key comes from: `keychain`, `file`, `environment`, or none.
    pub openai_key_source: Option<String>,
    pub openrouter_key_source: Option<String>,
    pub openai_default_model: String,
    pub openrouter_default_model: String,
}

pub fn load_settings(dir: &Path) -> AiSettings {
    fs::read(dir.join(SETTINGS_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<AiSettings>(&bytes).ok())
        .filter(|s| s.validate().is_ok())
        .unwrap_or_default()
}

pub fn save_settings(dir: &Path, settings: &AiSettings) -> Result<(), String> {
    settings.validate()?;
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
    std::io::Write::write_all(&mut temp, &bytes).map_err(|e| e.to_string())?;
    temp.persist(dir.join(SETTINGS_FILE))
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn settings_view(dir: &Path) -> AiSettingsView {
    let source = |provider: AiProvider| {
        crate::secrets::get(dir, provider.key_spec()).map(|(_, source)| source.to_string())
    };
    AiSettingsView {
        settings: load_settings(dir),
        openai_key_source: source(AiProvider::OpenAi),
        openrouter_key_source: source(AiProvider::OpenRouter),
        openai_default_model: AiProvider::OpenAi.default_model().into(),
        openrouter_default_model: AiProvider::OpenRouter.default_model().into(),
    }
}

/// A client for the chosen provider, or a message saying which key to add.
pub fn client_from_settings(dir: &Path) -> Result<client::ChatClient, String> {
    let settings = load_settings(dir);
    let provider = settings.provider;
    let (key, _) = crate::secrets::get(dir, provider.key_spec()).ok_or_else(|| {
        format!(
            "Add your {} API key in the AI settings first",
            provider.label()
        )
    })?;
    client::ChatClient::new(provider, key, settings.model())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_roundtrip_defaults_and_models() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_settings(dir.path()), AiSettings::default());
        assert_eq!(AiSettings::default().model(), "gpt-4.1-mini");
        let settings = AiSettings {
            provider: AiProvider::OpenRouter,
            openai_model: String::new(),
            openrouter_model: "anthropic/claude-sonnet-4.5".into(),
        };
        save_settings(dir.path(), &settings).unwrap();
        assert_eq!(load_settings(dir.path()), settings);
        assert_eq!(settings.model(), "anthropic/claude-sonnet-4.5");
        let json = fs::read_to_string(dir.path().join(SETTINGS_FILE)).unwrap();
        assert!(json.contains("\"provider\": \"openRouter\""));
        assert!(!json.to_lowercase().contains("key"));
        let bad = AiSettings {
            openai_model: "gpt 4".into(),
            ..AiSettings::default()
        };
        assert!(bad.validate().is_err());
    }
}
