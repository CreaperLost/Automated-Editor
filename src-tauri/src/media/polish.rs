//! Audio polish for the shared mixer: loudness normalization, noise reduction on speech and
//! ducking of background sound under speech.
//!
//! Each audio file is analyzed once (speech activity, loudness, a noise profile) and kept
//! in a process-wide cache, because the playback mixer is rebuilt on every seek. Everything
//! derived from the analysis is a pure function of source time, so playback and export
//! produce the same samples however they chunk their reads.
use super::audio::Lane;
use crate::dsp::denoise::{fft_size_for, Denoiser, NoiseProfile, ProfileBuilder};
use crate::dsp::loudness::{integrated_lufs, KWeighting};
use crate::project::{
    pcm::{PcmReader, READ_FRAME_CHUNK},
    reader::{safe_path, SegmentSummary},
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
/// Called per sample frame, so it allocates nothing.
fn fold_stereo(frame: &[f32]) -> [f32; 2] {
    match frame {
        [mono] => [*mono, *mono],
        [left, right] => [*left, *right],
        _ => {
            // More channels: even ones average to the left, odd ones to the right.
            let mut sums = [0.0f32; 2];
            let mut counts = [0usize; 2];
            for (i, value) in frame.iter().enumerate() {
                sums[i % 2] += value;
                counts[i % 2] += 1;
            }
            [
                sums[0] / counts[0].max(1) as f32,
                sums[1] / counts[1].max(1) as f32,
            ]
        }
    }
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
    /// Noise reduction by lane and file (relative path): each audio track has its own
    /// setting, even where two play the same sound.
    denoisers: HashMap<(String, String), Arc<Denoiser>>,
    /// Gain per 10 ms of timeline time for each ducked lane, under speech on any speech lane.
    lane_ducks: HashMap<String, Arc<Vec<f32>>>,
}

impl PolishPlan {
    /// `None` when every effect is off. Files that cannot be analyzed are left unpolished.
    pub fn build(
        root: &Path,
        settings: &crate::project::AudioSettings,
        lanes: &[Lane],
        duration_us: u64,
    ) -> Option<Self> {
        if !settings.any_enabled() {
            return None;
        }
        let analyze = |segment: &SegmentSummary| {
            safe_path(root, &segment.relative_path)
                .and_then(|path| analyze_cached(&path))
                .ok()
        };
        // Noise reduction where a lane has it on (speech by default when switched on).
        let mut denoisers = HashMap::new();
        for lane in lanes {
            let Some(db) = settings.lane_denoise_db(&lane.id, lane.speech) else {
                continue;
            };
            for segment in lane
                .clips
                .iter()
                .flat_map(|c| c.segments.iter())
                .filter(|s| s.available)
            {
                let key = (lane.id.clone(), segment.relative_path.clone());
                if denoisers.contains_key(&key) {
                    continue;
                }
                if let Some(profile) = analyze(segment).and_then(|a| a.noise.clone()) {
                    denoisers.insert(key, Arc::new(Denoiser::new(&profile, db)));
                }
            }
        }
        let lane_ducks = lane_ducks(settings, lanes, duration_us, &analyze);
        let mut plan = Self {
            gain: 1.0,
            limit: settings.normalize,
            denoisers,
            lane_ducks,
        };
        if settings.normalize {
            if let Some(lufs) = plan.edit_loudness(lanes, duration_us, &analyze) {
                let db =
                    (settings.target_lufs as f64 - lufs).clamp(-MAX_NORMALIZE_DB, MAX_NORMALIZE_DB);
                plan.gain = 10f64.powf(db / 20.0);
            }
        }
        Some(plan)
    }

    /// Integrated loudness of the edit before normalization: every lane's clips, 100 ms
    /// blocks on the timeline, with ducking applied.
    fn edit_loudness(
        &self,
        lanes: &[Lane],
        duration_us: u64,
        analyze: &dyn Fn(&SegmentSummary) -> Option<Arc<SegmentAnalysis>>,
    ) -> Option<f64> {
        let mut energy = vec![0f64; duration_us.div_ceil(LOUDNESS_BLOCK_US) as usize];
        for lane in lanes {
            for clip in &lane.clips {
                for segment in clip.segments.iter().filter(|s| s.available) {
                    let (a, b) = (
                        clip.in_us.max(segment.start_us),
                        (clip.in_us + clip.len).min(segment.end_us),
                    );
                    if b <= a {
                        continue;
                    }
                    let Some(analysis) = analyze(segment) else {
                        continue;
                    };
                    // Each timeline block takes the file's block under its middle.
                    let first = (clip.edited_start + (a - clip.in_us)) / LOUDNESS_BLOCK_US;
                    let last = (clip.edited_start + (b - clip.in_us)).div_ceil(LOUDNESS_BLOCK_US);
                    for block in first..last {
                        let center = block * LOUDNESS_BLOCK_US + LOUDNESS_BLOCK_US / 2;
                        let Some(source) = (center + clip.in_us).checked_sub(clip.edited_start)
                        else {
                            continue;
                        };
                        if source < a || source >= b {
                            continue;
                        }
                        let index = ((source - segment.start_us) / LOUDNESS_BLOCK_US) as usize;
                        let Some(&e) = analysis.loudness.get(index) else {
                            continue;
                        };
                        let gain = lane.gain * self.lane_duck_gain(&lane.id, center as f64);
                        if let Some(slot) = energy.get_mut(block as usize) {
                            *slot += e * gain * gain;
                        }
                    }
                }
            }
        }
        integrated_lufs(&energy)
    }

    /// Whether lane `lane` is lowered under speech.
    pub fn ducks(&self, lane: &str) -> bool {
        self.lane_ducks.contains_key(lane)
    }

    /// The ducking gain of lane `lane` at timeline time `edited_us` (1 when it is not ducked).
    pub fn lane_duck_gain(&self, lane: &str, edited_us: f64) -> f64 {
        let Some(envelope) = self.lane_ducks.get(lane) else {
            return 1.0;
        };
        let pos = (edited_us / VOICE_BLOCK_US as f64 - 0.5).max(0.0);
        let i = pos.floor() as usize;
        let t = pos - i as f64;
        let a = envelope.get(i).copied().unwrap_or(1.0) as f64;
        let b = envelope.get(i + 1).copied().unwrap_or(1.0) as f64;
        a + (b - a) * t
    }

    /// The noise reduction for `relative_path` played on lane `lane`, if that lane has it on.
    pub fn denoiser(&self, lane: &str, relative_path: &str) -> Option<&Denoiser> {
        self.denoisers
            .get(&(lane.to_string(), relative_path.to_string()))
            .map(|d| d.as_ref())
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

/// Speech on the timeline, per 10 ms, from every speech lane, and from it the gain of every
/// ducked (background) lane.
fn lane_ducks(
    settings: &crate::project::AudioSettings,
    lanes: &[Lane],
    duration_us: u64,
    analyze: &dyn Fn(&SegmentSummary) -> Option<Arc<SegmentAnalysis>>,
) -> HashMap<String, Arc<Vec<f32>>> {
    let ducked: Vec<(String, f32)> = lanes
        .iter()
        .filter(|lane| !lane.speech)
        .filter_map(|lane| Some((lane.id.clone(), settings.lane_duck_db(&lane.id, true)?)))
        .collect();
    if ducked.is_empty() {
        return HashMap::new();
    }
    // Cover the whole timeline, so the hold and release after the last words still apply.
    let mut speech = vec![false; duration_us.div_ceil(VOICE_BLOCK_US) as usize + 1];
    for lane in lanes.iter().filter(|lane| lane.speech) {
        for clip in &lane.clips {
            for segment in clip.segments.iter().filter(|s| s.available) {
                let (a, b) = (
                    clip.in_us.max(segment.start_us),
                    (clip.in_us + clip.len).min(segment.end_us),
                );
                if b <= a {
                    continue;
                }
                let Some(analysis) = analyze(segment) else {
                    continue;
                };
                let edited_a = clip.edited_start + (a - clip.in_us);
                let edited_b = clip.edited_start + (b - clip.in_us);
                for block in edited_a / VOICE_BLOCK_US..edited_b.div_ceil(VOICE_BLOCK_US) {
                    let edited = (block * VOICE_BLOCK_US).max(edited_a);
                    let file = clip.in_us + (edited - clip.edited_start) - segment.start_us;
                    if analysis.voice.get((file / VOICE_BLOCK_US) as usize) == Some(&true) {
                        if let Some(slot) = speech.get_mut(block as usize) {
                            *slot = true;
                        }
                    }
                }
            }
        }
    }
    // One envelope per depth, shared by the lanes that duck by it.
    let mut by_depth: HashMap<u32, Arc<Vec<f32>>> = HashMap::new();
    ducked
        .into_iter()
        .map(|(lane, db)| {
            let envelope = by_depth
                .entry(db.to_bits())
                .or_insert_with(|| {
                    Arc::new(duck_envelope(db, std::iter::once((0, speech.as_slice()))))
                })
                .clone();
            (lane, envelope)
        })
        .collect()
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

    #[test]
    fn folding_matches_the_channel_averages() {
        assert_eq!(fold_stereo(&[0.5]), [0.5, 0.5]);
        assert_eq!(fold_stereo(&[0.25, -0.5]), [0.25, -0.5]);
        // Four channels: (0 + 2) / 2 left, (1 + 3) / 2 right.
        assert_eq!(fold_stereo(&[0.0, 1.0, 2.0, 3.0]), [1.0, 2.0]);
    }
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
            lane_ducks: HashMap::new(),
            gain: 2.0,
            limit: true,
            denoisers: HashMap::new(),
        };
        assert!((plan.finish(0.2) - 0.4).abs() < 1e-12);
        assert!(plan.finish(5.0) <= 1.0 && plan.finish(5.0) > 0.99);
        assert!(plan.finish(0.5) < 1.0 && plan.finish(0.5) > 0.891);
        assert!(plan.finish(-5.0) >= -1.0 && plan.finish(-0.5) > -1.0);
    }
}
