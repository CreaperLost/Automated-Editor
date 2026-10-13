use serde::{Deserialize, Serialize};

pub const DEFAULT_WINDOW_MS: u32 = 20;
pub const MAX_ENERGY_POLICY: &str = "max_energy";
/// Levels below this are digital silence (a muted mic, padding), not the room.
const DIGITAL_SILENCE_DB: f32 = -100.0;
/// An automatic threshold sits at least this far above the sound's noise floor.
const AUTO_ABOVE_FLOOR_DB: f32 = 6.0;
/// When a noise floor is uncertain, stay below even the quietest sounding window.
const AUTO_BELOW_SOUND_DB: f32 = 12.0;
/// Fewer sounding windows than this cannot reliably tell floor from speech.
const AUTO_MIN_WINDOWS: usize = 50;
/// Step one removes only very quiet audio. Sounding floors belong to the text gap pass.
const AUTO_MAX_QUIET_DB: f32 = -65.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelPolicy {
    MaxEnergy,
    /// Protect brief sounds on any channel without RMS averaging or phase cancellation.
    MaxPeak,
    Channel(u16),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SilenceConfig {
    pub threshold_db: f32,    // e.g. -38.0 dBFS
    pub min_duration_ms: u32, // e.g. 400 ms
    pub padding_ms: u32,      // e.g. 50 ms
    /// Sets the threshold from the sound itself instead of `threshold_db` (0 to 1).
    /// At 0.2 the conservative baseline protects uncertain quiet audio.
    /// Higher sensitivity raises that baseline; it can also cut untranscribed quiet speech.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_level: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_policy: Option<String>,
}

impl Default for SilenceConfig {
    fn default() -> Self {
        Self {
            threshold_db: -38.0,
            min_duration_ms: 400,
            padding_ms: 50,
            auto_level: None,
            window_ms: None,
            step_ms: None,
            channel_policy: None,
        }
    }
}

impl SilenceConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !self.threshold_db.is_finite() {
            return Err("Silence threshold must be finite".into());
        }
        if self
            .auto_level
            .is_some_and(|level| !(0.0..=1.0).contains(&level))
        {
            return Err("Automatic silence level must be between 0 and 1".into());
        }
        let window_ms = self.window_ms.unwrap_or(DEFAULT_WINDOW_MS);
        if window_ms == 0 {
            return Err("Silence window must be greater than zero".into());
        }
        match self.step_ms {
            Some(0) => return Err("Silence step must be greater than zero".into()),
            Some(step) if step > window_ms => {
                return Err("Silence step cannot exceed window".into());
            }
            _ => {}
        }
        parse_channel_policy(self.channel_policy.as_deref())?;
        Ok(())
    }

    pub fn resolved_window_ms(&self) -> u32 {
        self.window_ms.unwrap_or(DEFAULT_WINDOW_MS).max(1)
    }

    pub fn resolved_step_ms(&self) -> u32 {
        match self.step_ms {
            Some(step) => step.max(1),
            None => (self.resolved_window_ms() / 2).max(1),
        }
    }

    pub fn resolved_policy(&self) -> Result<ChannelPolicy, String> {
        parse_channel_policy(self.channel_policy.as_deref())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SilenceCutInterval {
    pub id: String,
    pub start_us: u64,
    pub end_us: u64,
    pub duration_ms: u64,
    pub selected: bool,
    #[serde(default)]
    pub source_start_us: u64,
    #[serde(default)]
    pub source_end_us: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SilenceDetectionResult {
    pub track_id: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub channel_policy: String,
    pub suggestions: Vec<SilenceCutInterval>,
    pub diagnostics: Vec<String>,
    /// The level each sound was cut below (set from the sound with `auto_level`).
    #[serde(default)]
    pub thresholds: Vec<SoundThreshold>,
    #[serde(default)]
    pub transcript_dependencies: Vec<crate::transcript::TranscriptDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SoundThreshold {
    pub track_id: String,
    pub threshold_db: f32,
}

pub struct SilenceDetector;

impl SilenceDetector {
    /// Computes RMS (Root Mean Square) energy of an audio sample buffer.
    pub fn compute_rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }

        let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
        (sum_sq / samples.len() as f32).sqrt()
    }

    /// Converts an RMS amplitude value (0.0 to 1.0) to decibels relative to full scale (dBFS).
    pub fn rms_to_dbfs(rms: f32) -> f32 {
        if rms <= 1e-6 {
            -120.0
        } else {
            20.0 * rms.log10()
        }
    }

    /// Scans a monophonic PCM f32 buffer. Times are counted in sample frames.
    pub fn detect_silence(
        samples: &[f32],
        sample_rate: u32,
        config: &SilenceConfig,
    ) -> Result<Vec<SilenceCutInterval>, String> {
        Self::detect_interleaved(samples, sample_rate, 1, config)
    }

    /// Scans interleaved sample frames. `channels` is the frame width, not a time scale.
    pub fn detect_interleaved(
        samples: &[f32],
        sample_rate: u32,
        channels: u16,
        config: &SilenceConfig,
    ) -> Result<Vec<SilenceCutInterval>, String> {
        let mut detector = StreamingSilenceDetector::new(sample_rate, channels, config)?;
        detector.feed(samples, 0)?;
        Ok(intervals_from_source_ranges(detector.finish()))
    }
}

/// The level of every window of a stretch of contiguous audio, and where that audio ends.
/// Kept rather than judged as it comes, so the threshold can be set from the whole sound.
#[derive(Debug, Default)]
pub(crate) struct LevelBlock {
    /// Window start (source time) and its level in dBFS.
    windows: Vec<(u64, f32)>,
    end_us: u64,
}

/// The ranges quieter than `threshold_db`: a run of quiet windows lasts until the next window
/// that is not, or the end of its block.
pub(crate) fn silent_runs(blocks: &[LevelBlock], threshold_db: f32) -> Vec<(u64, u64)> {
    let mut runs = Vec::new();
    for block in blocks {
        let mut start = None;
        for &(at, db) in &block.windows {
            match (db < threshold_db, start) {
                (true, None) => start = Some(at),
                (false, Some(from)) => {
                    if at > from {
                        runs.push((from, at));
                    }
                    start = None;
                }
                _ => {}
            }
        }
        if let Some(from) = start {
            if block.end_us > from {
                runs.push((from, block.end_us));
            }
        }
    }
    runs
}

/// The threshold `config` asks for over these levels: fixed, or set from them (see
/// [`SilenceConfig::auto_level`]).
pub(crate) fn resolve_threshold(blocks: &[LevelBlock], config: &SilenceConfig) -> f32 {
    config
        .auto_level
        .and_then(|level| auto_threshold(blocks, level))
        .unwrap_or(config.threshold_db)
}

/// `level` of the way from the noise floor (10th percentile of the sounding windows) to the
/// speech level (95th). Digital silence means the sounding floor might be quiet speech,
/// rather than noise. In that case, or with flat/short sound, stay below the quietest sound.
/// No sounding windows falls back to the fixed threshold (which detects digital silence).
fn auto_threshold(blocks: &[LevelBlock], level: f32) -> Option<f32> {
    let mut levels: Vec<f32> = blocks
        .iter()
        .flat_map(|block| block.windows.iter().map(|&(_, db)| db))
        .collect();
    let has_digital_silence = levels.iter().any(|&db| db <= DIGITAL_SILENCE_DB);
    levels.retain(|&db| db > DIGITAL_SILENCE_DB);
    if levels.is_empty() {
        return None;
    }
    levels.sort_by(f32::total_cmp);
    let at = |fraction: f32| levels[((levels.len() - 1) as f32 * fraction) as usize];
    let (floor, speech) = (at(0.10), at(0.95));
    let baseline = if has_digital_silence
        || levels.len() < AUTO_MIN_WINDOWS
        || speech - floor < 2.0 * AUTO_ABOVE_FLOOR_DB
        || floor > AUTO_MAX_QUIET_DB
    {
        // Quiet words can be the lowest sounding cluster in clean/gated audio. Raising
        // the threshold above that cluster removes whole phrases, regardless of padding.
        // Use the minimum instead of a percentile so sparse quiet speech also survives.
        levels[0] - AUTO_BELOW_SOUND_DB
    } else {
        (floor + 0.2 * (speech - floor))
            .max(floor + AUTO_ABOVE_FLOOR_DB)
            .min(speech - AUTO_BELOW_SOUND_DB)
            .min(AUTO_MAX_QUIET_DB)
    };
    // Sensitivity must work even when the floor is uncertain. Preserve the conservative
    // baseline at 20%, but let an explicit higher setting include sounding pauses.
    let upper = speech - 6.0;
    // A near-digital minimum can put the baseline below -100 dB. A fixed dB
    // increment would leave even maximum sensitivity unable to reach ordinary pauses.
    Some(if level < 0.2 {
        baseline + (level - 0.2) * 30.0
    } else {
        baseline + ((level - 0.2) / 0.8) * (upper - baseline)
    })
}

pub(crate) struct StreamingSilenceDetector {
    sample_rate: u32,
    channels: usize,
    policy: ChannelPolicy,
    config: SilenceConfig,
    window_frames: usize,
    step_frames: usize,
    leftover: Vec<f32>,
    leftover_origin_us: u64,
    next_source_us: Option<u64>,
    /// Levels since the last discontinuity.
    current: Vec<(u64, f32)>,
    blocks: Vec<LevelBlock>,
}

impl StreamingSilenceDetector {
    pub(crate) fn new(
        sample_rate: u32,
        channels: u16,
        config: &SilenceConfig,
    ) -> Result<Self, String> {
        config.validate()?;
        if sample_rate == 0 {
            return Err("Silence sample rate must be greater than zero".into());
        }
        if channels == 0 {
            return Err("Silence channel count must be greater than zero".into());
        }
        let policy = config.resolved_policy()?;
        if let ChannelPolicy::Channel(index) = policy {
            if index as usize >= channels as usize {
                return Err("Selected silence channel is out of range".into());
            }
        }
        let window_frames = frames_for_ms(sample_rate, config.resolved_window_ms());
        let step_frames = frames_for_ms(sample_rate, config.resolved_step_ms());
        if window_frames == 0 {
            return Err("Silence window is smaller than one sample frame".into());
        }
        if step_frames == 0 {
            return Err("Silence step is smaller than one sample frame".into());
        }
        Ok(Self {
            sample_rate,
            channels: channels as usize,
            policy,
            config: config.clone(),
            window_frames,
            step_frames,
            leftover: Vec::new(),
            leftover_origin_us: 0,
            next_source_us: None,
            current: Vec::new(),
            blocks: Vec::new(),
        })
    }

    pub(crate) fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub(crate) fn channels(&self) -> u16 {
        self.channels as u16
    }

    pub(crate) fn notify_discontinuity(&mut self) {
        self.close_block();
        self.reset_window_state();
    }

    pub(crate) fn feed(&mut self, interleaved: &[f32], source_start_us: u64) -> Result<(), String> {
        if interleaved.is_empty() {
            return Ok(());
        }
        if interleaved.len() % self.channels != 0 {
            return Err("PCM chunk is not an integer number of sample frames".into());
        }
        if !self.is_contiguous(source_start_us) {
            self.notify_discontinuity();
        }

        let mut buffer = std::mem::take(&mut self.leftover);
        let origin = if buffer.is_empty() {
            source_start_us
        } else {
            self.leftover_origin_us
        };
        buffer.extend_from_slice(interleaved);

        let total_frames = buffer.len() / self.channels;
        let mut frame_idx = 0usize;
        while frame_idx + self.window_frames <= total_frames {
            let start = frame_idx * self.channels;
            let end = (frame_idx + self.window_frames) * self.channels;
            let db = window_db(&buffer[start..end], self.channels, self.policy);
            let window_us = origin.saturating_add(frame_us(frame_idx as u64, self.sample_rate));
            self.current.push((window_us, db));
            frame_idx += self.step_frames;
        }

        let leftover_frames = total_frames.saturating_sub(frame_idx);
        if leftover_frames > 0 {
            let start = frame_idx * self.channels;
            self.leftover = buffer[start..].to_vec();
            self.leftover_origin_us =
                origin.saturating_add(frame_us(frame_idx as u64, self.sample_rate));
        } else {
            self.leftover.clear();
            self.leftover_origin_us = 0;
        }
        self.next_source_us =
            Some(origin.saturating_add(frame_us(total_frames as u64, self.sample_rate)));
        Ok(())
    }

    /// The levels measured so far, ending the current block.
    pub(crate) fn take_blocks(&mut self) -> Vec<LevelBlock> {
        self.close_block();
        std::mem::take(&mut self.blocks)
    }

    pub(crate) fn finish(mut self) -> Vec<(u64, u64)> {
        let blocks = self.take_blocks();
        let threshold = resolve_threshold(&blocks, &self.config);
        finalize_regions(silent_runs(&blocks, threshold), &self.config)
    }

    fn is_contiguous(&self, source_start_us: u64) -> bool {
        let Some(expected) = self.next_source_us else {
            return true;
        };
        if source_start_us == expected {
            return true;
        }
        let frame = (1_000_000u128 / self.sample_rate.max(1) as u128) as u64;
        source_start_us.abs_diff(expected) <= frame.max(1)
    }

    /// Ends the block of contiguous audio: it runs to the end of what was fed.
    fn close_block(&mut self) {
        // Short clips and final partial windows must use their own samples, including
        // quiet speech tails and clicks that follow a preceding quiet window.
        if !self.leftover.is_empty() {
            self.current.push((
                self.leftover_origin_us,
                window_db(&self.leftover, self.channels, self.policy),
            ));
            self.leftover.clear();
        }
        let Some(&(last_us, _)) = self.current.last() else {
            return;
        };
        let end_us = self.next_source_us.unwrap_or(last_us);
        self.blocks.push(LevelBlock {
            windows: std::mem::take(&mut self.current),
            end_us,
        });
    }

    fn reset_window_state(&mut self) {
        self.leftover.clear();
        self.leftover_origin_us = 0;
        self.next_source_us = None;
        self.current.clear();
    }
}

pub(crate) fn parse_channel_policy(value: Option<&str>) -> Result<ChannelPolicy, String> {
    match value.map(str::trim).filter(|s| !s.is_empty()) {
        None => Ok(ChannelPolicy::MaxEnergy),
        Some("max_energy") | Some("maxEnergy") => Ok(ChannelPolicy::MaxEnergy),
        Some("max_peak") => Ok(ChannelPolicy::MaxPeak),
        Some(rest) if rest.starts_with("channel:") => {
            let index = rest[8..]
                .parse::<u16>()
                .map_err(|_| "Invalid silence channel policy".to_string())?;
            Ok(ChannelPolicy::Channel(index))
        }
        Some(_) => Err("Unknown silence channel policy".into()),
    }
}

pub(crate) fn channel_policy_name(policy: ChannelPolicy) -> String {
    match policy {
        ChannelPolicy::MaxEnergy => MAX_ENERGY_POLICY.into(),
        ChannelPolicy::MaxPeak => "max_peak".into(),
        ChannelPolicy::Channel(index) => format!("channel:{index}"),
    }
}

pub(crate) fn finalize_regions(
    mut regions: Vec<(u64, u64)>,
    config: &SilenceConfig,
) -> Vec<(u64, u64)> {
    regions.sort_unstable();
    let mut merged: Vec<(u64, u64)> = Vec::new();
    for (start, end) in regions {
        if end <= start {
            continue;
        }
        if let Some(last) = merged.last_mut() {
            if start <= last.1 {
                last.1 = last.1.max(end);
                continue;
            }
        }
        merged.push((start, end));
    }
    let min_duration_us = config.min_duration_ms as u64 * 1_000;
    let padding_us = config.padding_ms as u64 * 1_000;
    let mut out = Vec::new();
    for (start, end) in merged {
        if end.saturating_sub(start) < min_duration_us {
            continue;
        }
        let padded_start = start.saturating_add(padding_us);
        let padded_end = end.saturating_sub(padding_us);
        if padded_end > padded_start {
            out.push((padded_start, padded_end));
        }
    }
    out
}

pub(crate) fn intervals_from_source_ranges(ranges: Vec<(u64, u64)>) -> Vec<SilenceCutInterval> {
    ranges
        .into_iter()
        .enumerate()
        .map(|(idx, (start, end))| SilenceCutInterval {
            id: format!("silence-{}", idx + 1),
            start_us: start,
            end_us: end,
            duration_ms: end.saturating_sub(start) / 1_000,
            selected: true,
            source_start_us: start,
            source_end_us: end,
        })
        .collect()
}

fn frames_for_ms(sample_rate: u32, ms: u32) -> usize {
    (ms as u128 * sample_rate as u128 / 1_000) as usize
}

fn frame_us(frame_index: u64, sample_rate: u32) -> u64 {
    if sample_rate == 0 {
        return 0;
    }
    (frame_index as u128 * 1_000_000 / sample_rate as u128) as u64
}

/// A window's level in dBFS (the loudest channel, or the chosen one).
fn window_db(interleaved: &[f32], channels: usize, policy: ChannelPolicy) -> f32 {
    if channels == 0 || interleaved.len() < channels {
        return 0.0;
    }
    let frames = interleaved.len() / channels;
    if frames == 0 {
        return 0.0;
    }
    let rms = match policy {
        ChannelPolicy::MaxPeak => {
            if interleaved.iter().any(|sample| !sample.is_finite()) {
                return 0.0;
            }
            interleaved
                .iter()
                .fold(0.0f32, |peak, sample| peak.max(sample.abs()))
        }
        ChannelPolicy::MaxEnergy => {
            let mut best = 0.0f32;
            for ch in 0..channels {
                best = best.max(channel_rms(interleaved, channels, frames, ch));
            }
            best
        }
        ChannelPolicy::Channel(index) => channel_rms(interleaved, channels, frames, index as usize),
    };
    SilenceDetector::rms_to_dbfs(rms)
}

fn channel_rms(interleaved: &[f32], channels: usize, frames: usize, channel: usize) -> f32 {
    if channel >= channels || frames == 0 {
        return 0.0;
    }
    let mut sum_sq = 0.0f32;
    for frame in 0..frames {
        let x = interleaved[frame * channels + channel];
        sum_sq += x * x;
    }
    (sum_sq / frames as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peak_guard_keeps_quiet_clicks_stereo_and_partial_tails() {
        let config = SilenceConfig {
            threshold_db: -90.0,
            min_duration_ms: 1,
            padding_ms: 80,
            window_ms: Some(10),
            step_ms: Some(5),
            channel_policy: Some("max_peak".into()),
            ..SilenceConfig::default()
        };
        let rate = 8_000;
        let mut samples = vec![0.0; rate * 2 * 2 + 14];
        // A very quiet, single-frame stereo click (opposite phase) in the first second.
        samples[rate] = 0.0001;
        samples[rate + 1] = -0.0001;
        // Another click in the last partial window, which has no full window after it.
        *samples.last_mut().unwrap() = 0.001;
        let cuts = SilenceDetector::detect_interleaved(&samples, rate as u32, 2, &config).unwrap();
        assert!(cuts
            .iter()
            .all(|cut| cut.end_us <= 420_000 || cut.start_us >= 580_125));
        assert!(cuts.iter().all(|cut| cut.end_us < 1_920_875));
        assert!(
            SilenceDetector::detect_interleaved(&[0.001; 14], rate as u32, 2, &config)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn automatic_silence_keeps_varying_speech_in_clean_recordings() {
        let rate = 16_000;
        // The review's reproduction, plus a long recording where both the quiet phrase
        // and the digital pause occupy less than a tenth of the measured windows.
        for (quiet_start, speech_end) in [(1.0, 5.0), (10.0, 20.0)] {
            let samples: Vec<f32> = (0..rate * (speech_end as u32 + 1))
                .map(|i| {
                    let t = i as f32 / rate as f32;
                    let amplitude = if t >= speech_end {
                        0.0
                    } else if (quiet_start..quiet_start + 1.0).contains(&t) {
                        0.003
                    } else {
                        0.1
                    };
                    (t * 440.0 * std::f32::consts::TAU).sin() * amplitude
                })
                .collect();
            for auto_level in [0.0, 0.15, 0.2] {
                let config = SilenceConfig {
                    auto_level: Some(auto_level),
                    min_duration_ms: 500,
                    padding_ms: 120,
                    ..SilenceConfig::default()
                };
                let cuts = SilenceDetector::detect_silence(&samples, rate, &config).unwrap();
                assert_eq!(cuts.len(), 1, "level={auto_level}, {cuts:?}");
                assert!(
                    cuts[0].start_us >= (speech_end * 1_000_000.0) as u64 + 120_000,
                    "{cuts:?}"
                );
                assert!(
                    cuts[0].end_us <= ((speech_end + 1.0) * 1_000_000.0) as u64 - 120_000,
                    "{cuts:?}"
                );
            }
        }
    }

    #[test]
    fn automatic_silence_keeps_sparse_quiet_speech_without_a_pause() {
        let rate = 16_000;
        let samples: Vec<f32> = (0..rate * 20)
            .map(|i| {
                let t = i as f32 / rate as f32;
                let amplitude = if (10.0..11.0).contains(&t) {
                    0.003
                } else {
                    0.1
                };
                (t * 440.0 * std::f32::consts::TAU).sin() * amplitude
            })
            .collect();
        let config = SilenceConfig {
            auto_level: Some(0.2),
            min_duration_ms: 500,
            padding_ms: 120,
            ..SilenceConfig::default()
        };
        let cuts = SilenceDetector::detect_silence(&samples, rate, &config).unwrap();
        assert!(cuts.is_empty(), "{cuts:?}");
    }

    #[test]
    fn sensitivity_changes_the_fallback_and_removes_more_sounding_pauses() {
        let rate = 8_000;
        let samples: Vec<f32> = (0..rate * 4)
            .map(|i| {
                let t = i as f32 / rate as f32;
                let amplitude = if (1.0..2.0).contains(&t) {
                    0.003
                } else if t >= 3.0 {
                    0.0
                } else {
                    0.1
                };
                (t * 440.0 * std::f32::consts::TAU).sin() * amplitude
            })
            .collect();
        let config = SilenceConfig {
            min_duration_ms: 100,
            padding_ms: 0,
            auto_level: Some(0.2),
            ..SilenceConfig::default()
        };
        let gentle = SilenceDetector::detect_silence(&samples, rate, &config).unwrap();
        let tight = SilenceDetector::detect_silence(
            &samples,
            rate,
            &SilenceConfig {
                auto_level: Some(0.5),
                ..config
            },
        )
        .unwrap();
        assert_eq!(gentle.len(), 1);
        assert_eq!(tight.len(), 2);
        assert!(tight[0].start_us >= 1_000_000 && tight[0].end_us <= 2_000_000);
        // The ordinary noisy-floor branch must also respond monotonically.
        let blocks = [LevelBlock {
            end_us: 2_000_000,
            windows: (0..200)
                .map(|i| (i * 10_000, if i < 80 { -80.0 } else { -25.0 }))
                .collect(),
        }];
        let levels: Vec<_> = [0.0, 0.2, 0.5, 1.0]
            .into_iter()
            .map(|level| auto_threshold(&blocks, level).unwrap())
            .collect();
        assert!(
            levels.windows(2).all(|pair| pair[0] < pair[1]),
            "{levels:?}"
        );
        let near_zero = [LevelBlock {
            end_us: 2_000_000,
            windows: (0..200)
                .map(|i| {
                    (
                        i * 10_000,
                        if i == 0 {
                            -99.9
                        } else if i < 80 {
                            -120.0
                        } else {
                            -20.0
                        },
                    )
                })
                .collect(),
        }];
        assert!(auto_threshold(&near_zero, 0.2).unwrap() < -100.0);
        assert!((auto_threshold(&near_zero, 1.0).unwrap() + 26.0).abs() < 0.01);
    }

    #[test]
    fn rms_scans_measure_a_sounding_partial_tail() {
        let mut samples = vec![0.0; 8_000];
        samples.extend([0.1; 24]);
        let config = SilenceConfig {
            threshold_db: -42.0,
            min_duration_ms: 20,
            padding_ms: 0,
            ..SilenceConfig::default()
        };
        let cuts = SilenceDetector::detect_silence(&samples, 8_000, &config).unwrap();
        assert!(cuts.iter().all(|cut| cut.end_us <= 1_000_000));
    }

    #[test]
    fn automatic_silence_leaves_uncertain_noisy_pauses_with_digital_silence() {
        let rate = 16_000;
        let samples: Vec<f32> = (0..rate * 4)
            .map(|i| {
                let t = i as f32 / rate as f32;
                let amplitude = if t < 1.0 || t >= 3.0 {
                    0.1
                } else if t < 2.0 {
                    0.0001
                } else {
                    0.0
                };
                (t * 440.0 * std::f32::consts::TAU).sin() * amplitude
            })
            .collect();
        let config = SilenceConfig {
            auto_level: Some(0.2),
            min_duration_ms: 500,
            padding_ms: 120,
            ..SilenceConfig::default()
        };
        let cuts = SilenceDetector::detect_silence(&samples, rate, &config).unwrap();
        assert_eq!(cuts.len(), 1, "{cuts:?}");
        assert!(cuts[0].start_us >= 2_120_000, "{cuts:?}");
        assert!(cuts[0].end_us <= 2_880_000, "{cuts:?}");
    }

    #[test]
    fn continuous_clean_audio_preserves_common_quiet_phrases() {
        let rate = 16_000;
        for quiet_duration in [1.0, 2.0] {
            let samples: Vec<f32> = (0..rate * 5)
                .map(|i| {
                    let t = i as f32 / rate as f32;
                    let amplitude = if (1.0..1.0 + quiet_duration).contains(&t) {
                        0.003
                    } else {
                        0.1
                    };
                    (t * 440.0 * std::f32::consts::TAU).sin() * amplitude
                })
                .collect();
            let config = SilenceConfig {
                auto_level: Some(0.2),
                min_duration_ms: 500,
                padding_ms: 120,
                ..SilenceConfig::default()
            };
            assert!(SilenceDetector::detect_silence(&samples, rate, &config)
                .unwrap()
                .is_empty());
        }
    }

    #[test]
    fn automatic_silence_preserves_quiet_words_and_their_edges() {
        let rate = 16_000;
        let config = SilenceConfig {
            auto_level: Some(0.2),
            min_duration_ms: 500,
            padding_ms: 120,
            ..SilenceConfig::default()
        };
        // Room noise, a loud word with quiet edges, quiet speech longer than the minimum
        // pause, room noise, then speech. The fixed -38 dB threshold loses the quiet words.
        let samples: Vec<f32> = (0..rate * 6)
            .map(|i| {
                let t = i as f32 / rate as f32;
                let amplitude = if (1.2..1.8).contains(&t) || t >= 5.0 {
                    0.1
                } else if (1.0..3.5).contains(&t) {
                    0.003
                } else {
                    0.0001
                };
                (t * 440.0 * std::f32::consts::TAU).sin() * amplitude
            })
            .collect();
        let cuts = SilenceDetector::detect_silence(&samples, rate, &config).unwrap();
        assert_eq!(cuts.len(), 2, "{cuts:?}");
        assert!(cuts[0].end_us <= 880_000, "{cuts:?}");
        assert!(cuts[1].start_us >= 3_620_000, "{cuts:?}");
        assert!(cuts[1].end_us <= 4_880_000, "{cuts:?}");
    }

    #[test]
    fn automatic_silence_handles_clean_quiet_and_short_recordings() {
        let rate = 16_000;
        let config = SilenceConfig {
            auto_level: Some(0.2),
            ..SilenceConfig::default()
        };
        for speech_ms in [200, 1000] {
            let speech: Vec<f32> = (0..rate * speech_ms / 1000)
                .map(|i| (i as f32 * 440.0 * std::f32::consts::TAU / rate as f32).sin() * 0.002)
                .collect();
            let mut samples = speech.clone();
            samples.extend(vec![0.0; rate as usize]);
            samples.extend(speech);
            let cuts = SilenceDetector::detect_silence(&samples, rate, &config).unwrap();
            assert_eq!(cuts.len(), 1, "{cuts:?}");
            assert!(cuts[0].start_us >= u64::from(speech_ms) * 1000);
            assert!(cuts[0].end_us <= u64::from(speech_ms + 1000) * 1000);
        }
    }

    #[test]
    fn automatic_level_rejects_non_finite_and_out_of_range_values() {
        for value in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
            assert!(SilenceConfig {
                auto_level: Some(value),
                ..SilenceConfig::default()
            }
            .validate()
            .is_err());
        }
        // Old serialized settings still select a fixed threshold.
        let old: SilenceConfig =
            serde_json::from_str(r#"{"thresholdDb":-38,"minDurationMs":400,"paddingMs":50}"#)
                .unwrap();
        assert_eq!(old.auto_level, None);
    }

    fn speech_tone(sample_rate: u32, seconds: u32) -> Vec<f32> {
        (0..sample_rate * seconds)
            .map(|i| {
                ((i as f32 * 440.0 * 2.0 * std::f32::consts::PI) / sample_rate as f32).sin() * 0.5
            })
            .collect()
    }

    #[test]
    fn test_rms_and_dbfs() {
        let full_scale = vec![1.0f32; 100];
        let rms = SilenceDetector::compute_rms(&full_scale);
        assert!((rms - 1.0).abs() < 1e-4);
        assert!((SilenceDetector::rms_to_dbfs(rms) - 0.0).abs() < 1e-3);

        let quiet = vec![0.001f32; 100];
        let quiet_rms = SilenceDetector::compute_rms(&quiet);
        let quiet_db = SilenceDetector::rms_to_dbfs(quiet_rms);
        assert!(quiet_db < -55.0);
    }

    #[test]
    fn test_silence_detection_with_synthetic_audio() {
        let sample_rate = 48_000;
        let mut samples = speech_tone(sample_rate, 1);
        samples.extend(std::iter::repeat_n(0.0, sample_rate as usize));
        samples.extend(speech_tone(sample_rate, 1));

        let config = SilenceConfig::default();
        let cuts = SilenceDetector::detect_silence(&samples, sample_rate, &config).unwrap();
        assert_eq!(cuts.len(), 1, "Should detect exactly 1 silence pocket");

        let cut = &cuts[0];
        assert!(cut.start_us >= 1_000_000);
        assert!(cut.end_us <= 2_000_000);
        assert!(cut.duration_ms >= 800, "Padded duration should be ~900ms");
    }

    #[test]
    fn rejects_zero_window_and_non_finite_threshold() {
        let mut zero_window = SilenceConfig::default();
        zero_window.window_ms = Some(0);
        let err = zero_window.validate().unwrap_err();
        assert!(err.to_lowercase().contains("window"));

        let mut zero_step = SilenceConfig::default();
        zero_step.step_ms = Some(0);
        assert!(zero_step
            .validate()
            .unwrap_err()
            .to_lowercase()
            .contains("step"));

        let mut nan = SilenceConfig::default();
        nan.threshold_db = f32::NAN;
        assert!(nan
            .validate()
            .unwrap_err()
            .to_lowercase()
            .contains("finite"));

        let mut inf = SilenceConfig::default();
        inf.threshold_db = f32::INFINITY;
        assert!(inf
            .validate()
            .unwrap_err()
            .to_lowercase()
            .contains("finite"));
    }

    #[test]
    fn opposite_polarity_stereo_is_not_silence() {
        let sample_rate = 48_000;
        let frames = sample_rate as usize * 2;
        let mut samples = Vec::with_capacity(frames * 2);
        for _ in 0..frames {
            samples.push(0.5);
            samples.push(-0.5);
        }
        let cuts = SilenceDetector::detect_interleaved(
            &samples,
            sample_rate,
            2,
            &SilenceConfig::default(),
        )
        .unwrap();
        assert!(
            cuts.is_empty(),
            "max-energy must keep opposite-polarity stereo"
        );
    }

    #[test]
    fn selected_channel_timing_uses_sample_frames() {
        let sample_rate = 48_000u32;
        let frames = sample_rate as usize;
        let mut samples = Vec::with_capacity(frames * 3 * 2);
        for i in 0..frames * 3 {
            let ch0 = if i >= frames && i < frames * 2 {
                0.0
            } else {
                0.5
            };
            samples.push(ch0);
            samples.push(0.5);
        }
        let mut config = SilenceConfig::default();
        config.channel_policy = Some("channel:0".into());
        let cuts = SilenceDetector::detect_interleaved(&samples, sample_rate, 2, &config).unwrap();
        assert_eq!(cuts.len(), 1);
        assert!(
            cuts[0].start_us >= 1_000_000 && cuts[0].end_us <= 2_000_000,
            "channel timing must use frames, got {}-{}",
            cuts[0].start_us,
            cuts[0].end_us
        );

        let max_energy = SilenceDetector::detect_interleaved(
            &samples,
            sample_rate,
            2,
            &SilenceConfig::default(),
        )
        .unwrap();
        assert!(
            max_energy.is_empty(),
            "loud second channel should suppress max-energy cuts"
        );
    }

    #[test]
    fn contiguous_chunks_preserve_one_silence_run() {
        let sample_rate = 48_000;
        let config = SilenceConfig::default();
        let mut detector = StreamingSilenceDetector::new(sample_rate, 1, &config).unwrap();
        let first = vec![0.0f32; sample_rate as usize];
        let second = vec![0.0f32; sample_rate as usize];
        detector.feed(&first, 0).unwrap();
        detector.feed(&second, 1_000_000).unwrap();
        let ranges = detector.finish();
        assert_eq!(ranges.len(), 1, "contiguous silent chunks must merge");
        assert!(ranges[0].1.saturating_sub(ranges[0].0) >= 1_800_000);
    }

    #[test]
    fn discontinuity_resets_and_does_not_invent_silence() {
        let sample_rate = 48_000;
        let config = SilenceConfig {
            min_duration_ms: 400,
            padding_ms: 0,
            ..SilenceConfig::default()
        };
        let mut detector = StreamingSilenceDetector::new(sample_rate, 1, &config).unwrap();
        detector.feed(&vec![0.0; sample_rate as usize], 0).unwrap();
        detector.notify_discontinuity();
        detector
            .feed(&vec![0.0; sample_rate as usize], 2_000_000)
            .unwrap();
        let ranges = detector.finish();
        assert_eq!(ranges.len(), 2);
        assert!(ranges[0].1 <= 1_000_000);
        assert!(ranges[1].0 >= 2_000_000);
    }
}
