//! Audio polish for the shared mixer: loudness normalization, microphone noise reduction
//! and ducking of system audio under speech.
//!
//! Each audio file is analyzed once (speech activity, loudness, a noise profile) and kept
//! in a process-wide cache, because the playback mixer is rebuilt on every seek. Everything
//! derived from the analysis is a pure function of source time, so playback and export
//! produce the same samples however they chunk their reads.
use crate::dsp::denoise::{fft_size_for, Denoiser, NoiseProfile, ProfileBuilder};
use crate::dsp::loudness::{integrated_lufs, KWeighting};
use crate::project::{
    pcm::{PcmReader, READ_FRAME_CHUNK},
    reader::{safe_path, RetainedInterval, SegmentSummary},
    AudioSettings, TrackType,
};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::SystemTime;

/// Speech activity resolution.
pub const VOICE_BLOCK_US: u64 = 10_000;
/// BS.1770 sub-block length; four make one 400 ms gating block.
pub const LOUDNESS_BLOCK_US: u64 = 100_000;
/// Blocks quieter than this are digital silence (muted mic, padding), not room noise.
const SILENCE_DB: f32 = -100.0;
/// Speech must sit this far above the noise floor.
const VOICE_ABOVE_FLOOR_DB: f32 = 10.0;
const VOICE_MIN_DB: f32 = -60.0;
/// Windows this close to the noise floor are treated as noise only.
const NOISE_WINDOW_ABOVE_FLOOR_DB: f32 = 6.0;
const MIN_NOISE_WINDOWS: usize = 8;
const MAX_NOISE_WINDOWS: usize = 400;
/// Ducking starts this early so the first syllable is not masked.
const DUCK_LOOKAHEAD_BLOCKS: usize = 5;
/// Ducking holds through short pauses between words.
const DUCK_HOLD_BLOCKS: usize = 30;
const DUCK_ATTACK_BLOCKS: f32 = 8.0;
const DUCK_RELEASE_BLOCKS: f32 = 40.0;
/// Normalization never boosts or cuts by more than this.
const MAX_NORMALIZE_DB: f64 = 20.0;
/// Peaks above -1 dBFS are softly limited after normalization.
const LIMIT_THRESHOLD: f64 = 0.891;
const CACHE_LIMIT: usize = 256;

/// What one audio file sounds like, at its own sample rate.
#[derive(Debug)]
pub struct SegmentAnalysis {
    pub sample_rate: u32,
    /// One flag per 10 ms from the start of the file: speech-level audio.
    pub voice: Vec<bool>,
    /// K-weighted mean square per 100 ms, summed over the two output channels.
    pub loudness: Vec<f64>,
    /// Absent when the file has too little quiet audio to measure.
    pub noise: Option<NoiseProfile>,
}

type CacheKey = (PathBuf, u64, Option<SystemTime>);

fn cache() -> &'static Mutex<HashMap<CacheKey, Arc<SegmentAnalysis>>> {
    static CACHE: OnceLock<Mutex<HashMap<CacheKey, Arc<SegmentAnalysis>>>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// Cached analysis of the file at `path`; recomputed when its size or mtime changes.
pub fn analyze_cached(path: &Path) -> Result<Arc<SegmentAnalysis>, String> {
    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    let key = (path.to_path_buf(), metadata.len(), metadata.modified().ok());
    if let Some(hit) = cache().lock().get(&key) {
        return Ok(Arc::clone(hit));
    }
    let analysis = Arc::new(analyze(path)?);
    let mut cache = cache().lock();
    if cache.len() >= CACHE_LIMIT {
        cache.clear();
    }
    cache.insert(key, Arc::clone(&analysis));
    Ok(analysis)
}

fn to_db(rms: f32) -> f32 {
    20.0 * rms.max(1e-10).log10()
}

/// Folds interleaved `frame` to the mixer's stereo output (mono plays on both sides).
fn fold_stereo(frame: &[f32]) -> [f32; 2] {
    if frame.len() == 1 {
        return [frame[0], frame[0]];
    }
    let mut out = [0.0; 2];
    for (ch, value) in out.iter_mut().enumerate() {
        let picked: Vec<f32> = frame.iter().skip(ch).step_by(2).copied().collect();
        *value = picked.iter().sum::<f32>() / picked.len() as f32;
    }
    out
}

fn analyze(path: &Path) -> Result<SegmentAnalysis, String> {
    let mut reader = PcmReader::open(path)?;
    let info = reader.info().clone();
    let channels = info.channels as usize;
    let rate = info.sample_rate;
    let voice_frames = (rate as u64 * VOICE_BLOCK_US / 1_000_000).max(1) as usize;
    let loudness_frames = (rate as u64 * LOUDNESS_BLOCK_US / 1_000_000).max(1) as usize;
    let mut filters = [KWeighting::new(rate), KWeighting::new(rate)];
    let mut block_db = Vec::new();
    let mut loudness = Vec::new();
    let (mut voice_sum, mut voice_n) = (0f64, 0usize);
    let (mut loud_sum, mut loud_n) = (0f64, 0usize);
    let mut buf = vec![0f32; READ_FRAME_CHUNK * channels];
    loop {
        let got = reader.read_frames(&mut buf, READ_FRAME_CHUNK)?;
        if got == 0 {
            break;
        }
        for frame in buf[..got * channels].chunks_exact(channels) {
            let stereo = fold_stereo(frame);
            let mono = (stereo[0] + stereo[1]) * 0.5;
            voice_sum += (mono * mono) as f64;
            voice_n += 1;
            if voice_n == voice_frames {
                block_db.push(to_db((voice_sum / voice_n as f64).sqrt() as f32));
                (voice_sum, voice_n) = (0.0, 0);
            }
            let l = filters[0].process(stereo[0]);
            let r = filters[1].process(stereo[1]);
            loud_sum += l * l + r * r;
            loud_n += 1;
            if loud_n == loudness_frames {
                loudness.push(loud_sum / loud_n as f64);
                (loud_sum, loud_n) = (0.0, 0);
            }
        }
    }
    if voice_n > 0 {
        block_db.push(to_db((voice_sum / voice_n as f64).sqrt() as f32));
    }
    if loud_n > 0 {
        loudness.push(loud_sum / loud_n as f64);
    }

    let mut audible: Vec<f32> = block_db
        .iter()
        .copied()
        .filter(|&db| db > SILENCE_DB)
        .collect();
    audible.sort_by(f32::total_cmp);
    let floor_db = audible.get(audible.len() / 10).copied();
    let voice = match floor_db {
        Some(floor) => {
            let threshold = (floor + VOICE_ABOVE_FLOOR_DB).max(VOICE_MIN_DB);
            block_db.iter().map(|&db| db > threshold).collect()
        }
        None => vec![false; block_db.len()],
    };
    let noise = match floor_db {
        Some(floor) => noise_profile(
            &mut reader,
            &block_db,
            voice_frames,
            floor + NOISE_WINDOW_ABOVE_FLOOR_DB,
        )?,
        None => None,
    };
    Ok(SegmentAnalysis {
        sample_rate: rate,
        voice,
        loudness,
        noise,
    })
}

/// Averages the spectrum of FFT windows that lie entirely in near-floor 10 ms blocks.
fn noise_profile(
    reader: &mut PcmReader,
    block_db: &[f32],
    voice_frames: usize,
    max_db: f32,
) -> Result<Option<NoiseProfile>, String> {
    let info = reader.info().clone();
    let channels = info.channels as usize;
    let n = fft_size_for(info.sample_rate);
    let blocks_per_window = n.div_ceil(voice_frames);
    let quiet = |db: f32| db > SILENCE_DB && db <= max_db;
    let mut starts = Vec::new();
    let mut run = 0;
    for (i, &db) in block_db.iter().enumerate() {
        run = if quiet(db) { run + 1 } else { 0 };
        if run == blocks_per_window {
            starts.push((i + 1 - blocks_per_window) as u64 * voice_frames as u64);
            run = 0;
        }
    }
    if starts.len() < MIN_NOISE_WINDOWS {
        return Ok(None);
    }
    let step = starts.len().div_ceil(MAX_NOISE_WINDOWS);
    let mut builder = ProfileBuilder::new(n);
    let mut buf = vec![0f32; n * channels];
    let mut mono = vec![0f32; n];
    for &start in starts.iter().step_by(step) {
        reader.seek_to_frame(start)?;
        if reader.read_frames(&mut buf, n)? < n {
            continue;
        }
        for (m, frame) in mono.iter_mut().zip(buf.chunks_exact(channels)) {
            let stereo = fold_stereo(frame);
            *m = (stereo[0] + stereo[1]) * 0.5;
        }
        builder.add(&mono);
    }
    Ok(builder.finish())
}

/// Polish derived for one edit: everything the mixer needs per sample.
pub struct PolishPlan {
    /// Linear output gain from loudness normalization (1.0 when it is off).
    gain: f64,
    limit: bool,
    /// System-audio gain per 10 ms of source time; empty when ducking is off.
    duck: Vec<f32>,
    /// Noise reduction for microphone files, by relative path.
    denoisers: HashMap<String, Arc<Denoiser>>,
}

impl PolishPlan {
    /// `None` when every effect is off. Files that cannot be analyzed are left unpolished.
    pub fn build(
        root: &Path,
        settings: &AudioSettings,
        retained: &[RetainedInterval],
        tracks: &[(TrackType, Vec<SegmentSummary>)],
    ) -> Option<Self> {
        if !settings.any_enabled() {
            return None;
        }
        let has_system = tracks
            .iter()
            .any(|(t, s)| *t == TrackType::SystemAudio && s.iter().any(|s| s.available));
        let duck_on = settings.duck_system_audio && has_system;
        let mut analyses: Vec<(TrackType, &SegmentSummary, Arc<SegmentAnalysis>)> = Vec::new();
        for (track_type, segments) in tracks {
            let needed = settings.normalize
                || (*track_type == TrackType::MicAudio && (settings.noise_reduction || duck_on));
            if !needed {
                continue;
            }
            for segment in segments.iter().filter(|s| s.available) {
                let Ok(path) = safe_path(root, &segment.relative_path) else {
                    continue;
                };
                if let Ok(analysis) = analyze_cached(&path) {
                    analyses.push((*track_type, segment, analysis));
                }
            }
        }
        let duck = if duck_on {
            duck_envelope(
                settings.duck_db,
                analyses
                    .iter()
                    .filter(|(t, _, _)| *t == TrackType::MicAudio)
                    .map(|(_, s, a)| (s.start_us, a.voice.as_slice())),
            )
        } else {
            Vec::new()
        };
        let mut denoisers = HashMap::new();
        if settings.noise_reduction {
            for (track_type, segment, analysis) in &analyses {
                if let (TrackType::MicAudio, Some(profile)) = (track_type, &analysis.noise) {
                    denoisers.insert(
                        segment.relative_path.clone(),
                        Arc::new(Denoiser::new(profile, settings.noise_reduction_db)),
                    );
                }
            }
        }
        let mut plan = Self {
            gain: 1.0,
            limit: settings.normalize,
            duck,
            denoisers,
        };
        if settings.normalize {
            if let Some(lufs) = plan.edit_loudness(&analyses, retained) {
                let db =
                    (settings.target_lufs as f64 - lufs).clamp(-MAX_NORMALIZE_DB, MAX_NORMALIZE_DB);
                plan.gain = 10f64.powf(db / 20.0);
            }
        }
        Some(plan)
    }

    /// Integrated loudness of the edit before normalization: retained 100 ms blocks of
    /// every track, with ducking applied, in playback order.
    fn edit_loudness(
        &self,
        analyses: &[(TrackType, &SegmentSummary, Arc<SegmentAnalysis>)],
        retained: &[RetainedInterval],
    ) -> Option<f64> {
        let end = analyses
            .iter()
            .map(|(_, s, a)| s.start_us + a.loudness.len() as u64 * LOUDNESS_BLOCK_US)
            .max()?;
        let mut energy = vec![0f64; end.div_ceil(LOUDNESS_BLOCK_US) as usize];
        for (track_type, segment, analysis) in analyses {
            for (i, &e) in analysis.loudness.iter().enumerate() {
                let start = segment.start_us + i as u64 * LOUDNESS_BLOCK_US;
                let gain = if *track_type == TrackType::SystemAudio {
                    self.duck_gain(start as f64 + LOUDNESS_BLOCK_US as f64 / 2.0)
                } else {
                    1.0
                };
                if let Some(slot) = energy.get_mut((start / LOUDNESS_BLOCK_US) as usize) {
                    *slot += e * gain * gain;
                }
            }
        }
        let kept: Vec<f64> = retained
            .iter()
            .flat_map(|r| {
                let first = r.start_us / LOUDNESS_BLOCK_US;
                let last = r.end_us.div_ceil(LOUDNESS_BLOCK_US);
                (first..last).filter(move |b| {
                    let center = b * LOUDNESS_BLOCK_US + LOUDNESS_BLOCK_US / 2;
                    center >= r.start_us && center < r.end_us
                })
            })
            .filter_map(|b| energy.get(b as usize).copied())
            .collect();
        integrated_lufs(&kept)
    }

    /// Linear gain for system audio at `source_us`.
    pub fn duck_gain(&self, source_us: f64) -> f64 {
        if self.duck.is_empty() {
            return 1.0;
        }
        // Block values sit at block centers; interpolate between them.
        let pos = (source_us / VOICE_BLOCK_US as f64 - 0.5).max(0.0);
        let i = pos.floor() as usize;
        let t = pos - i as f64;
        let a = self.duck.get(i).copied().unwrap_or(1.0) as f64;
        let b = self.duck.get(i + 1).copied().unwrap_or(1.0) as f64;
        a + (b - a) * t
    }

    pub fn denoiser(&self, relative_path: &str) -> Option<&Denoiser> {
        self.denoisers.get(relative_path).map(|d| d.as_ref())
    }

    /// Applies normalization gain and, when normalizing, a soft limiter above -1 dBFS.
    pub fn finish(&self, sample: f64) -> f64 {
        let x = sample * self.gain;
        if !self.limit || x.abs() <= LIMIT_THRESHOLD {
            return x;
        }
        let headroom = 1.0 - LIMIT_THRESHOLD;
        x.signum() * (LIMIT_THRESHOLD + headroom * ((x.abs() - LIMIT_THRESHOLD) / headroom).tanh())
    }
}

/// System-audio gain per 10 ms block of source time from microphone speech activity.
fn duck_envelope<'a>(duck_db: f32, voices: impl Iterator<Item = (u64, &'a [bool])>) -> Vec<f32> {
    let voices: Vec<(usize, &[bool])> = voices
        .map(|(start_us, v)| ((start_us / VOICE_BLOCK_US) as usize, v))
        .collect();
    let len = voices
        .iter()
        .map(|(offset, v)| offset + v.len())
        .max()
        .unwrap_or(0);
    if len == 0 {
        return Vec::new();
    }
    let mut active = vec![false; len];
    for (offset, v) in voices {
        for (i, &speaking) in v.iter().enumerate() {
            if speaking {
                let lo = (offset + i).saturating_sub(DUCK_LOOKAHEAD_BLOCKS);
                let hi = (offset + i + DUCK_HOLD_BLOCKS).min(len - 1);
                active[lo..=hi].fill(true);
            }
        }
    }
    // Attenuation in dB, ramped: slow release after speech, quick attack before it.
    let mut att: Vec<f32> = active
        .iter()
        .map(|&a| if a { duck_db } else { 0.0 })
        .collect();
    for i in 1..len {
        att[i] = att[i].max(att[i - 1] - duck_db / DUCK_RELEASE_BLOCKS);
    }
    for i in (0..len - 1).rev() {
        att[i] = att[i].max(att[i + 1] - duck_db / DUCK_ATTACK_BLOCKS);
    }
    att.into_iter().map(|db| 10f32.powf(-db / 20.0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::generate_pcm16_wav;

    #[test]
    fn duck_envelope_ramps_around_speech() {
        let mut voice = vec![false; 300];
        voice[100..150].fill(true);
        let env = duck_envelope(12.0, std::iter::once((0, voice.as_slice())));
        let full = 10f32.powf(-12.0 / 20.0);
        assert!((env[0] - 1.0).abs() < 1e-6);
        // Fully ducked during speech, including the lookahead and the hold.
        assert!(env[95..=179].iter().all(|g| (g - full).abs() < 1e-6));
        // Ramps down before speech and recovers slowly afterwards.
        assert!(env[90] < 1.0 && env[90] > full);
        assert!(env[200] > full && env[200] < 1.0);
        assert!((env[260] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn analysis_finds_speech_and_a_noise_profile() {
        let rate = 48_000;
        // One second of faint hiss, one second of a loud tone over the hiss.
        let mut state = 1u64;
        let values: Vec<i16> = (0..rate * 2)
            .map(|i| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let hiss = ((state >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 300.0;
                let tone = if i >= rate {
                    8000.0 * (std::f32::consts::TAU * 220.0 * i as f32 / rate as f32).sin()
                } else {
                    0.0
                };
                (hiss + tone) as i16
            })
            .collect();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mic.wav");
        std::fs::write(&path, generate_pcm16_wav(rate as u32, 1, &values)).unwrap();
        let analysis = analyze_cached(&path).unwrap();
        assert_eq!(analysis.voice.len(), 200);
        assert!(analysis.voice[..100].iter().all(|v| !v));
        assert!(analysis.voice[100..].iter().all(|v| *v));
        assert_eq!(analysis.loudness.len(), 20);
        assert!(analysis.noise.is_some());
        assert!(Arc::ptr_eq(&analysis, &analyze_cached(&path).unwrap()));
    }

    #[test]
    fn limiter_is_transparent_below_threshold_and_bounded_above() {
        let plan = PolishPlan {
            gain: 2.0,
            limit: true,
            duck: Vec::new(),
            denoisers: HashMap::new(),
        };
        assert!((plan.finish(0.2) - 0.4).abs() < 1e-12);
        assert!(plan.finish(5.0) <= 1.0 && plan.finish(5.0) > 0.99);
        assert!(plan.finish(0.5) < 1.0 && plan.finish(0.5) > 0.891);
        assert!(plan.finish(-5.0) >= -1.0 && plan.finish(-0.5) > -1.0);
    }
}
