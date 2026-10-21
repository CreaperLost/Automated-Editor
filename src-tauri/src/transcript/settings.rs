//! App-wide transcription settings. The ElevenLabs key never goes in a project or in the
//! settings file: it lives in the OS credential store (Windows Credential Manager, macOS
//! Keychain), or in an owner-only file on other platforms. `ELEVENLABS_API_KEY` overrides it.
use super::{align, parakeet, Language, ProviderKind};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const SETTINGS_FILE: &str = "transcription.json";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct TranscriptSettings {
    pub provider: ProviderKind,
    /// What is spoken: sent to the provider, and the filler, retake and alignment passes
    /// follow it. Greek is transcribed online only.
    pub language: Language,
    /// ElevenLabs model id; empty uses the default.
    pub scribe_model: String,
    /// Words to bias recognition towards, such as game or product names.
    pub keyterms: Vec<String>,
    /// Which Parakeet transcribes English: Unified (the most accurate words) or Verbatim
    /// (TDT v3: keeps stutters, repeats and filler sounds).
    pub parakeet_model: parakeet::ParakeetModel,
    /// Folder holding the Parakeet ONNX files; empty uses the app data folder.
    pub parakeet_model_dir: String,
    /// Align word times to the audio after transcribing, when the model is downloaded.
    pub align_words: bool,
}

impl Default for TranscriptSettings {
    fn default() -> Self {
        Self {
            provider: ProviderKind::Parakeet,
            language: Language::English,
            scribe_model: String::new(),
            keyterms: Vec::new(),
            parakeet_model: parakeet::ParakeetModel::Unified,
            parakeet_model_dir: String::new(),
            align_words: false,
        }
    }
}

impl TranscriptSettings {
    pub fn validate(&self) -> Result<(), String> {
        if self.provider == ProviderKind::Parakeet && !self.language.runs_locally() {
            return Err(format!(
                "{} is transcribed online: choose ElevenLabs Scribe or Scribe via OpenRouter",
                self.language.label()
            ));
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
            self.parakeet_model.default_dir()
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
    /// Where the OpenRouter key comes from (shared with AI review), or none.
    pub openrouter_key_source: Option<String>,
    pub parakeet_available: bool,
    pub parakeet_accelerator: String,
    pub parakeet_model_dir: String,
    pub parakeet_model_present: bool,
    /// Download size of the Parakeet model this build uses, in bytes.
    pub parakeet_model_bytes: u64,
    /// Both Parakeet models, so the choice shows what each needs before it is saved.
    pub parakeet_models: Vec<ParakeetModelStatus>,
    /// Word alignment runs in this build (it shares Parakeet's ONNX Runtime).
    pub aligner_available: bool,
    pub aligner_model_present: bool,
    pub aligner_model_dir: String,
    /// Download size of the alignment model, in bytes.
    pub aligner_model_bytes: u64,
    /// This build runs ONNX Runtime on CUDA, so the NVIDIA GPU pack is of use.
    pub gpu_pack_wanted: bool,
    /// An NVIDIA driver is installed.
    pub nvidia_driver: bool,
    pub gpu_pack_present: bool,
    /// Download size of the GPU pack, in bytes.
    pub gpu_pack_bytes: u64,
    /// Why the GPU is not in use, once a model tried it; `None` when it is or none tried.
    pub gpu_problem: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParakeetModelStatus {
    pub model: parakeet::ParakeetModel,
    pub present: bool,
    /// Download size in this build, in bytes.
    pub bytes: u64,
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
        openrouter_key_source: openrouter_key(dir).map(|(_, source)| source.to_string()),
        parakeet_available: parakeet::compiled_in(),
        parakeet_accelerator: parakeet::accelerator().into(),
        parakeet_model_present: settings.parakeet_model.present(&model_dir),
        parakeet_model_bytes: settings.parakeet_model.bytes(),
        parakeet_models: [
            parakeet::ParakeetModel::Unified,
            parakeet::ParakeetModel::Verbatim,
        ]
        .into_iter()
        .map(|model| {
            let dir = TranscriptSettings {
                parakeet_model: model,
                ..settings.clone()
            }
            .parakeet_dir();
            ParakeetModelStatus {
                model,
                present: model.present(&dir),
                bytes: model.bytes(),
            }
        })
        .collect(),
        parakeet_model_dir: model_dir.to_string_lossy().into_owned(),
        aligner_available: align::compiled_in(),
        aligner_model_present: align::model_present(&align::default_model_dir()),
        aligner_model_dir: align::default_model_dir().to_string_lossy().into_owned(),
        aligner_model_bytes: align::MODEL_BYTES,
        gpu_pack_wanted: crate::cuda::wanted(),
        nvidia_driver: crate::cuda::nvidia_driver(),
        gpu_pack_present: crate::cuda::present(&crate::cuda::default_dir()),
        gpu_pack_bytes: crate::cuda::download_bytes(),
        gpu_problem: crate::cuda::loaded().and_then(Result::err),
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

/// The OpenRouter key Scribe-via-OpenRouter runs on: the same one AI review uses.
pub fn openrouter_key(dir: &Path) -> Option<(String, &'static str)> {
    crate::secrets::get(dir, crate::ai::AiProvider::OpenRouter.key_spec())
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
        s.language = Language::Greek;
        assert!(s.validate().is_err());
        s.provider = ProviderKind::OpenRouter;
        assert!(s.validate().is_ok());
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"language\":\"el\""));
    }
}
