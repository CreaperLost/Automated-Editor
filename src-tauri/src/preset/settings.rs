//! Settings a preset gives a project beyond the edit's own: how cleanup cuts (the silence and
//! gap levels and mouth-noise sensitivity the Cleanup panel and the agent start from) and how
//! it exports (the Export dialog's size, frame rate and quality).
use serde::{Deserialize, Serialize};

/// What the cleanup passes start from. "Balanced" unless a preset says otherwise.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct CleanupSettings {
    pub silence: SilenceLevel,
    pub gaps: GapLevel,
    /// Mouth noises: 0 to 1 (higher finds more).
    pub mouth_noise_sensitivity: f32,
}

/// Silence from the audio: quieter than `threshold_db` for at least `min_duration_ms` is a
/// pause; `padding_ms` is kept around words.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct SilenceLevel {
    pub threshold_db: f32,
    pub min_duration_ms: u32,
    pub padding_ms: u32,
}

/// Gaps between transcribed words.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct GapLevel {
    pub min_duration_ms: u32,
    pub padding_ms: u32,
    /// Trim quiet word tails measured in the audio.
    pub refine_word_edges: bool,
    pub edge_threshold_db: f32,
}

impl Default for CleanupSettings {
    fn default() -> Self {
        Self {
            silence: SilenceLevel::default(),
            gaps: GapLevel::default(),
            mouth_noise_sensitivity: 0.5,
        }
    }
}

impl Default for SilenceLevel {
    fn default() -> Self {
        Self {
            threshold_db: -42.0,
            min_duration_ms: 250,
            padding_ms: 40,
        }
    }
}

impl Default for GapLevel {
    fn default() -> Self {
        Self {
            min_duration_ms: 150,
            padding_ms: 40,
            refine_word_edges: true,
            edge_threshold_db: -42.0,
        }
    }
}

impl CleanupSettings {
    pub fn validate(&self) -> Result<(), String> {
        let s = &self.silence;
        if !s.threshold_db.is_finite() || !(-80.0..=-10.0).contains(&s.threshold_db) {
            return Err("Silence: the level must be -80 to -10 dB".into());
        }
        if !(20..=10_000).contains(&s.min_duration_ms) || s.padding_ms > 1_000 {
            return Err("Silence: pauses of 20 ms to 10 s, room up to 1 s".into());
        }
        let g = &self.gaps;
        if !(10..=10_000).contains(&g.min_duration_ms) || g.padding_ms > 1_000 {
            return Err("Gaps: 10 ms to 10 s, room up to 1 s".into());
        }
        if !g.edge_threshold_db.is_finite() || !(-80.0..=-10.0).contains(&g.edge_threshold_db) {
            return Err("Gaps: the word-edge level must be -80 to -10 dB".into());
        }
        let n = self.mouth_noise_sensitivity;
        if !n.is_finite() || !(0.0..=1.0).contains(&n) {
            return Err("Mouth noises: sensitivity is 0 to 1".into());
        }
        Ok(())
    }
}

/// What the Export dialog starts with.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct ExportPreferences {
    /// 720p, 1080p, 1440p or 4K (the canvas shape sets the orientation).
    pub resolution: String,
    pub fps: u32,
    /// standard, high, max, or custom (`mbps`).
    pub quality: String,
    /// The average video bitrate with quality `custom`, 1 to 200.
    pub mbps: u32,
}

impl Default for ExportPreferences {
    fn default() -> Self {
        Self {
            resolution: "1080p".into(),
            fps: 30,
            quality: "high".into(),
            mbps: 16,
        }
    }
}

pub const RESOLUTIONS: [(&str, u32, u32); 4] = [
    ("720p", 1280, 720),
    ("1080p", 1920, 1080),
    ("1440p", 2560, 1440),
    ("4K", 3840, 2160),
];

impl ExportPreferences {
    pub fn validate(&self) -> Result<(), String> {
        if !RESOLUTIONS.iter().any(|(name, _, _)| *name == self.resolution) {
            return Err("Export resolution is 720p, 1080p, 1440p or 4K".into());
        }
        if ![24, 25, 30, 60].contains(&self.fps) {
            return Err("Export frame rate is 24, 25, 30 or 60".into());
        }
        if !matches!(self.quality.as_str(), "standard" | "high" | "max" | "custom") {
            return Err("Export quality is standard, high, max or custom".into());
        }
        if !(1..=200).contains(&self.mbps) {
            return Err("Export bitrate is 1 to 200 Mbps".into());
        }
        Ok(())
    }

    /// The export these make for `layout`'s canvas.
    pub fn settings(
        &self,
        layout: &crate::project::EditLayout,
    ) -> Result<crate::export::ExportSettings, String> {
        let (_, w, h) = RESOLUTIONS
            .iter()
            .find(|(name, _, _)| *name == self.resolution)
            .copied()
            .unwrap_or(RESOLUTIONS[1]);
        let (width, height) = layout.fit_export_size(w, h)?;
        let quality = match self.quality.as_str() {
            "standard" => crate::media::VideoQuality::Standard,
            "max" => crate::media::VideoQuality::Max,
            _ => crate::media::VideoQuality::High,
        };
        Ok(crate::export::ExportSettings {
            width,
            height,
            fps: self.fps,
            quality,
            bitrate_kbps: (self.quality == "custom").then_some(self.mbps * 1_000),
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_balanced_and_valid() {
        CleanupSettings::default().validate().unwrap();
        ExportPreferences::default().validate().unwrap();
        let tall = crate::project::EditLayout {
            aspect_ratio: "9:16".into(),
            ..Default::default()
        };
        let export = ExportPreferences {
            quality: "custom".into(),
            mbps: 20,
            ..Default::default()
        }
        .settings(&tall)
        .unwrap();
        assert_eq!((export.width, export.height), (1080, 1920));
        assert_eq!(export.bitrate_kbps, Some(20_000));
        assert!(ExportPreferences {
            resolution: "8K".into(),
            ..Default::default()
        }
        .validate()
        .is_err());
    }
}
