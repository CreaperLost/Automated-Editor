//! Audio polish settings stored in the edit document. Each effect has its own switch and
//! applies to playback and export alike, because both read the same mixer.
use serde::{Deserialize, Serialize};

/// YouTube and most streaming platforms normalize to about -14 LUFS.
pub const DEFAULT_TARGET_LUFS: f32 = -14.0;
pub const DEFAULT_NOISE_REDUCTION_DB: f32 = 12.0;
pub const DEFAULT_DUCK_DB: f32 = 12.0;
pub const TARGET_LUFS_RANGE: (f32, f32) = (-30.0, -8.0);
pub const NOISE_REDUCTION_DB_RANGE: (f32, f32) = (3.0, 30.0);
pub const DUCK_DB_RANGE: (f32, f32) = (3.0, 30.0);

fn default_target_lufs() -> f32 {
    DEFAULT_TARGET_LUFS
}

fn default_noise_reduction_db() -> f32 {
    DEFAULT_NOISE_REDUCTION_DB
}

fn default_duck_db() -> f32 {
    DEFAULT_DUCK_DB
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioSettings {
    /// Scale the whole edit to `target_lufs` integrated loudness.
    #[serde(default)]
    pub normalize: bool,
    #[serde(default = "default_target_lufs")]
    pub target_lufs: f32,
    /// Spectral noise reduction on the microphone.
    #[serde(default)]
    pub noise_reduction: bool,
    /// How far steady background noise is pushed down, in dB.
    #[serde(default = "default_noise_reduction_db")]
    pub noise_reduction_db: f32,
    /// Lower system audio while the microphone has speech.
    #[serde(default)]
    pub duck_system_audio: bool,
    /// How far system audio is lowered under speech, in dB.
    #[serde(default = "default_duck_db")]
    pub duck_db: f32,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            normalize: false,
            target_lufs: DEFAULT_TARGET_LUFS,
            noise_reduction: false,
            noise_reduction_db: DEFAULT_NOISE_REDUCTION_DB,
            duck_system_audio: false,
            duck_db: DEFAULT_DUCK_DB,
        }
    }
}

impl AudioSettings {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    pub fn any_enabled(&self) -> bool {
        self.normalize || self.noise_reduction || self.duck_system_audio
    }

    pub fn validate(&self) -> Result<(), String> {
        let check = |name: &str, value: f32, (lo, hi): (f32, f32)| {
            if value.is_finite() && (lo..=hi).contains(&value) {
                Ok(())
            } else {
                Err(format!("{name} must be between {lo} and {hi}"))
            }
        };
        check("Target loudness", self.target_lufs, TARGET_LUFS_RANGE)?;
        check(
            "Noise reduction",
            self.noise_reduction_db,
            NOISE_REDUCTION_DB_RANGE,
        )?;
        check("Ducking amount", self.duck_db, DUCK_DB_RANGE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_fields_take_defaults_and_ranges_are_enforced() {
        let settings: AudioSettings = serde_json::from_str(r#"{"normalize":true}"#).unwrap();
        assert!(settings.normalize && settings.any_enabled());
        assert_eq!(settings.target_lufs, DEFAULT_TARGET_LUFS);
        assert!(settings.validate().is_ok());
        assert!(AudioSettings::default().is_default());
        for bad in [
            AudioSettings {
                target_lufs: -2.0,
                ..Default::default()
            },
            AudioSettings {
                duck_db: f32::NAN,
                ..Default::default()
            },
            AudioSettings {
                noise_reduction_db: 50.0,
                ..Default::default()
            },
        ] {
            assert!(bad.validate().is_err());
        }
    }
}
