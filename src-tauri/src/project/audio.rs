//! Audio polish settings stored in the edit document. Each effect has its own switch and
//! applies to playback and export alike, because both read the same mixer.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// YouTube and most streaming platforms normalize to about -14 LUFS.
pub const DEFAULT_TARGET_LUFS: f32 = -14.0;
pub const DEFAULT_NOISE_REDUCTION_DB: f32 = 12.0;
pub const DEFAULT_DUCK_DB: f32 = 12.0;
pub const TARGET_LUFS_RANGE: (f32, f32) = (-30.0, -8.0);
pub const NOISE_REDUCTION_DB_RANGE: (f32, f32) = (3.0, 30.0);
pub const DUCK_DB_RANGE: (f32, f32) = (3.0, 30.0);
pub const TRACK_VOLUME_DB_RANGE: (f32, f32) = (-30.0, 12.0);

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
    /// Repair short mouth clicks on speech lanes without changing their timing.
    #[serde(default)]
    pub mouth_clicks: bool,
    #[serde(default = "default_mouth_click_strength")]
    pub mouth_click_strength: u8,
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
    /// Mute and volume per audio track id. Tracks without an entry play at full volume.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tracks: BTreeMap<String, TrackMix>,
}

/// One audio track's mix, kept under the track's id. Muting is the track's own switch.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrackMix {
    /// Gain applied to the track, in dB. 0 leaves it as recorded.
    #[serde(default)]
    pub volume_db: f32,
    /// Noise reduction on this track, in dB; `None` follows the project's switch for speech.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub denoise_db: Option<f32>,
    /// Lowered by this many dB while speech plays on a speech track; `None` follows the
    /// project's switch for background sound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duck_db: Option<f32>,
}

impl TrackMix {
    /// Linear gain.
    pub fn gain(&self) -> f64 {
        10f64.powf(self.volume_db as f64 / 20.0)
    }
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            mouth_clicks: false,
            mouth_click_strength: default_mouth_click_strength(),
            normalize: false,
            target_lufs: DEFAULT_TARGET_LUFS,
            noise_reduction: false,
            noise_reduction_db: DEFAULT_NOISE_REDUCTION_DB,
            duck_system_audio: false,
            duck_db: DEFAULT_DUCK_DB,
            tracks: BTreeMap::new(),
        }
    }
}

impl AudioSettings {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    pub fn any_enabled(&self) -> bool {
        self.normalize
            || self.noise_reduction
            || self.duck_system_audio
            || self
                .tracks
                .values()
                .any(|mix| mix.denoise_db.is_some() || mix.duck_db.is_some())
    }

    /// Noise reduction on an audio track: its own setting, else (for speech) the project's.
    pub fn lane_denoise_db(&self, lane: &str, speech: bool) -> Option<f32> {
        self.tracks
            .get(lane)
            .and_then(|mix| mix.denoise_db)
            .or((speech && self.noise_reduction).then_some(self.noise_reduction_db))
    }

    /// Ducking on an audio track: its own setting, else (for background sound) the project's.
    pub fn lane_duck_db(&self, lane: &str, background: bool) -> Option<f32> {
        self.tracks
            .get(lane)
            .and_then(|mix| mix.duck_db)
            .or((background && self.duck_system_audio).then_some(self.duck_db))
    }

    /// Linear gain for the audio track `track_id`.
    pub fn track_gain(&self, track_id: &str) -> f64 {
        self.tracks.get(track_id).map_or(1.0, TrackMix::gain)
    }

    pub fn validate(&self) -> Result<(), String> {
        if !(1..=100).contains(&self.mouth_click_strength) {
            return Err("Mouth click strength must be between 1 and 100".into());
        }
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
        check("Ducking amount", self.duck_db, DUCK_DB_RANGE)?;
        for mix in self.tracks.values() {
            check("Track volume", mix.volume_db, TRACK_VOLUME_DB_RANGE)?;
            if let Some(db) = mix.denoise_db {
                check("Noise reduction", db, NOISE_REDUCTION_DB_RANGE)?;
            }
            if let Some(db) = mix.duck_db {
                check("Ducking amount", db, DUCK_DB_RANGE)?;
            }
        }
        Ok(())
    }
}

fn default_mouth_click_strength() -> u8 {
    35
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
        assert!(!settings.mouth_clicks);
        assert_eq!(settings.mouth_click_strength, 35);
        for bad in [
            AudioSettings {
                mouth_click_strength: 0,
                ..Default::default()
            },
            AudioSettings {
                mouth_click_strength: 101,
                ..Default::default()
            },
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
            AudioSettings {
                tracks: BTreeMap::from([(
                    "mic".into(),
                    TrackMix {
                        volume_db: 40.0,
                        ..Default::default()
                    },
                )]),
                ..Default::default()
            },
        ] {
            assert!(bad.validate().is_err());
        }
    }

    #[test]
    fn track_mix_round_trips_and_defaults_to_full_volume() {
        let settings: AudioSettings =
            serde_json::from_str(r#"{"tracks":{"mic":{"volumeDb":-6}}}"#).unwrap();
        assert!((settings.track_gain("mic") - 0.501).abs() < 1e-3);
        assert_eq!(settings.track_gain("other"), 1.0);
        let json = serde_json::to_string(&AudioSettings::default()).unwrap();
        assert!(!json.contains("tracks"), "{json}");
    }
}
