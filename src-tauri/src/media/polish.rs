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
use std::collections::{HashMap, HashSet};
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
/// After normalization, peaks above -3 dBFS are softly limited, and never pass -1 dBFS: a
/// ceiling at full scale let AAC overshoot it by up to 3 dB on export, which clips on playback.
const LIMIT_KNEE: f64 = 0.708;
const LIMIT_CEILING: f64 = 0.891;
/// Normalization boosts no further than this share of the edit's loud blocks can take
/// without reaching the limiter: the rest are rare transients (a clap, a plosive). Boosting
/// a quiet recording to the target regardless clipped every syllable into the limiter, so
/// the edit sounded squashed and much louder than its LUFS.
const PEAK_PERCENTILE: f64 = 0.995;
/// The leveller turns a loud 100 ms block down only as far as its peak needs to stay under
/// the limiter, so normalization can boost quiet speech to the target instead of stopping at
/// the first loud syllable (real edits fell 4-5 dB short of -14 LUFS). Normalization still
/// stops where the edit's loud blocks (`PEAK_PERCENTILE`) would need more than this.
const LEVEL_MAX_DB: f64 = 6.0;
/// How fast the leveller turns down ahead of a loud block and back up after one, per block.
const LEVEL_ATTACK_DB: f64 = 6.0;
const LEVEL_RELEASE_DB: f64 = 1.5;
/// Analyses kept for reuse, in bytes: an hour of sound analyses to well under a megabyte, so
/// this holds every segment of many long recordings. The least recently used go first.
const CACHE_BYTES: usize = 128 << 20;

/// What one audio file sounds like, at its own sample rate.
#[derive(Debug)]
pub struct SegmentAnalysis {
    pub sample_rate: u32,
    /// One flag per 10 ms from the start of the file: speech-level audio.
    pub voice: Vec<bool>,
    /// K-weighted mean square per 100 ms, summed over the two output channels.
    pub loudness: Vec<f64>,
    /// Largest sample magnitude per 100 ms on either output channel.
    pub peaks: Vec<f32>,
    /// Absent when the file has too little quiet audio to measure.
    pub noise: Option<NoiseProfile>,
}

type CacheKey = (PathBuf, u64, Option<SystemTime>);

/// Analyses with when each was last used.
#[derive(Default)]
struct AnalysisCache {
    entries: HashMap<CacheKey, (Arc<SegmentAnalysis>, u64)>,
    tick: u64,
    bytes: usize,
}

impl SegmentAnalysis {
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.voice.capacity()
            + self.loudness.capacity() * std::mem::size_of::<f64>()
            + self.peaks.capacity() * std::mem::size_of::<f32>()
            + self
                .noise
                .as_ref()
                .map_or(0, |n| n.power.capacity() * std::mem::size_of::<f32>())
    }
}

fn cache() -> &'static Mutex<AnalysisCache> {
    static CACHE: OnceLock<Mutex<AnalysisCache>> = OnceLock::new();
    CACHE.get_or_init(Default::default)
}

/// Cached analysis of the file at `path`; recomputed when its size or mtime changes.
pub fn analyze_cached(path: &Path) -> Result<Arc<SegmentAnalysis>, String> {
    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    let key = (path.to_path_buf(), metadata.len(), metadata.modified().ok());
    {
        let mut cache = cache().lock();
        cache.tick += 1;
        let tick = cache.tick;
        if let Some((hit, used)) = cache.entries.get_mut(&key) {
            *used = tick;
            return Ok(Arc::clone(hit));
        }
    }
    let analysis = Arc::new(analyze(path)?);
    let mut cache = cache().lock();
    cache.tick += 1;
    let tick = cache.tick;
    cache.bytes += analysis.bytes();
    if let Some((old, _)) = cache.entries.insert(key, (Arc::clone(&analysis), tick)) {
        cache.bytes -= old.bytes();
    }
    // Over budget: the least recently used go, never the whole cache at once.
    while cache.bytes > CACHE_BYTES && cache.entries.len() > 1 {
        let Some(oldest) = cache
            .entries
            .iter()
            .min_by_key(|(_, (_, used))| *used)
            .map(|(k, _)| k.clone())
        else {
            break;
        };
        if let Some((gone, _)) = cache.entries.remove(&oldest) {
            cache.bytes -= gone.bytes();
        }
    }
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
    let (mut peaks, mut peak) = (Vec::new(), 0f32);
    // Per channel, the louder one counting: a channel in opposite phase cancels in a mix.
    let (mut voice_sum, mut voice_n) = ([0f64; 2], 0usize);
    let (mut loud_sum, mut loud_n) = (0f64, 0usize);
    let mut buf = vec![0f32; READ_FRAME_CHUNK * channels];
    loop {
        let got = reader.read_frames(&mut buf, READ_FRAME_CHUNK)?;
        if got == 0 {
            break;
        }
        for frame in buf[..got * channels].chunks_exact(channels) {
            let stereo = fold_stereo(frame);
            voice_sum[0] += (stereo[0] * stereo[0]) as f64;
            voice_sum[1] += (stereo[1] * stereo[1]) as f64;
            voice_n += 1;
            if voice_n == voice_frames {
                let energy = voice_sum[0].max(voice_sum[1]);
                block_db.push(to_db((energy / voice_n as f64).sqrt() as f32));
                (voice_sum, voice_n) = ([0.0; 2], 0);
            }
            peak = peak.max(stereo[0].abs()).max(stereo[1].abs());
            let l = filters[0].process(stereo[0]);
            let r = filters[1].process(stereo[1]);
            loud_sum += l * l + r * r;
            loud_n += 1;
            if loud_n == loudness_frames {
                loudness.push(loud_sum / loud_n as f64);
                peaks.push(peak);
                (loud_sum, loud_n, peak) = (0.0, 0, 0.0);
            }
        }
    }
    if voice_n > 0 {
        let energy = voice_sum[0].max(voice_sum[1]);
        block_db.push(to_db((energy / voice_n as f64).sqrt() as f32));
    }
    if loud_n > 0 {
        loudness.push(loud_sum / loud_n as f64);
        peaks.push(peak);
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
        peaks,
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

/// Analyzes, side by side, the files the enabled effects will read: loudness reads every
/// lane, noise reduction the lanes that have it, ducking the speech lanes. One after another,
/// switching an effect on for a long recording held playback up for over a second. Retain
/// the results for the whole plan build: repeated clips need no more filesystem lookups,
/// and cache eviction while another file is analyzed cannot discard this plan's work.
fn prepare_analyses<'a>(
    root: &Path,
    settings: &crate::project::AudioSettings,
    lanes: &'a [Lane],
) -> HashMap<&'a str, Arc<SegmentAnalysis>> {
    let ducking = lanes
        .iter()
        .any(|lane| !lane.speech && settings.lane_duck_db(&lane.id, true).is_some());
    let mut seen = HashSet::new();
    let mut paths = Vec::new();
    for lane in lanes {
        let wanted = settings.normalize
            || settings.lane_denoise_db(&lane.id, lane.speech).is_some()
            || (ducking && lane.speech);
        if !wanted {
            continue;
        }
        for clip in &lane.clips {
            for segment in clip.segments.iter().filter(|s| {
                s.available && s.start_us < clip.in_us + clip.len && clip.in_us < s.end_us
            }) {
                // Deduplicate before safe_path: resolving a recording mount can touch the
                // filesystem, and thousands of cut clips commonly share the same file.
                let relative = segment.relative_path.as_str();
                if seen.insert(relative) {
                    paths.push(relative);
                }
            }
        }
    }
    let analyze = |relative| {
        safe_path(root, relative)
            .and_then(|path| analyze_cached(&path))
            .ok()
            .map(|analysis| (relative, analysis))
    };
    if paths.len() < 2 {
        return paths.into_iter().filter_map(analyze).collect();
    }
    let workers = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .min(paths.len())
        .min(8);
    let next = std::sync::atomic::AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let jobs: Vec<_> = (0..workers)
            .map(|_| {
                scope.spawn(|| {
                    let mut found = Vec::new();
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(&path) = paths.get(i) else { break };
                        if let Some(entry) = analyze(path) {
                            found.push(entry);
                        }
                    }
                    found
                })
            })
            .collect();
        jobs.into_iter()
            .flat_map(|job| job.join().unwrap())
            .collect()
    })
}

/// Polish derived for one edit: everything the mixer needs per sample.
pub struct PolishPlan {
    /// Linear output gain from loudness normalization (1.0 when it is off).
    gain: f64,
    limit: bool,
    /// The leveller's gain at each 100 ms block edge of the timeline, linear in between
    /// (empty when nothing needs turning down).
    level: Vec<f64>,
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
        let analyses = prepare_analyses(root, settings, lanes);
        let analyze =
            |segment: &SegmentSummary| analyses.get(segment.relative_path.as_str()).cloned();
        // Noise reduction where a lane has it on (speech by default when switched on).
        let mut denoisers = HashMap::new();
        for lane in lanes {
            let Some(db) = settings.lane_denoise_db(&lane.id, lane.speech) else {
                continue;
            };
            // Only the segments a clip plays: a short clip of a long recording does not
            // need the rest of it analyzed.
            for segment in lane.clips.iter().flat_map(|c| {
                c.segments.iter().filter(move |s| {
                    s.available && s.start_us < c.in_us + c.len && c.in_us < s.end_us
                })
            }) {
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
            level: Vec::new(),
            denoisers,
            lane_ducks,
        };
        if settings.normalize {
            if let Some(edit) = plan.edit_loudness(lanes, duration_us, &analyze) {
                let most = match edit.peak > 0.0 {
                    true => 20.0 * (LIMIT_CEILING / edit.peak).log10() + LEVEL_MAX_DB,
                    false => MAX_NORMALIZE_DB,
                };
                let limited = |db: f64| db.clamp(-MAX_NORMALIZE_DB, MAX_NORMALIZE_DB).min(most);
                let mut db = limited(settings.target_lufs as f64 - edit.lufs);
                // The leveller takes a little loudness off the loud blocks: measured on the
                // same blocks, that is made up once, within the same limits.
                let level = level_envelope(&edit.peaks, 10f64.powf(db / 20.0));
                if let Some(levelled) = levelled_lufs(&edit.energy, &level) {
                    db = limited(db + edit.lufs - levelled);
                }
                plan.gain = 10f64.powf(db / 20.0);
                plan.level = level_envelope(&edit.peaks, plan.gain);
            }
        }
        Some(plan)
    }

    /// How loud the edit is before normalization: every lane's clips, 100 ms blocks on the
    /// timeline, with ducking applied.
    fn edit_loudness(
        &self,
        lanes: &[Lane],
        duration_us: u64,
        analyze: &dyn Fn(&SegmentSummary) -> Option<Arc<SegmentAnalysis>>,
    ) -> Option<EditLoudness> {
        let blocks = duration_us.div_ceil(LOUDNESS_BLOCK_US) as usize;
        let mut energy = vec![0f64; blocks];
        // Lanes add: their peaks sum at worst.
        let mut peaks = vec![0f64; blocks];
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
                    let first = clip.edited_at(a) / LOUDNESS_BLOCK_US;
                    let last = clip.edited_at(b).div_ceil(LOUDNESS_BLOCK_US);
                    for block in first..last {
                        let center = block * LOUDNESS_BLOCK_US + LOUDNESS_BLOCK_US / 2;
                        let Some(source) = clip.source_at(center) else {
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
                        if let (Some(slot), Some(&p)) =
                            (peaks.get_mut(block as usize), analysis.peaks.get(index))
                        {
                            *slot += p as f64 * gain;
                        }
                    }
                }
            }
        }
        let lufs = integrated_lufs(&energy)?;
        // Only blocks that count towards loudness: silence between takes says nothing.
        let gate = 10f64.powf((lufs - 20.0 + 0.691) / 10.0);
        let mut loud: Vec<f64> = energy
            .iter()
            .zip(&peaks)
            .filter(|(&e, _)| e > gate)
            .map(|(_, &p)| p)
            .collect();
        loud.sort_by(f64::total_cmp);
        let index =
            ((loud.len() as f64 * PEAK_PERCENTILE) as usize).min(loud.len().saturating_sub(1));
        Some(EditLoudness {
            lufs,
            peak: loud.get(index).copied().unwrap_or(0.0),
            energy,
            peaks,
        })
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

    /// The leveller's gain at timeline time `edited_us` (1 where nothing is turned down).
    fn level_gain(&self, edited_us: f64) -> f64 {
        if self.level.is_empty() {
            return 1.0;
        }
        let pos = (edited_us / LOUDNESS_BLOCK_US as f64).max(0.0);
        let i = pos.floor() as usize;
        let t = pos - i as f64;
        let a = self.level.get(i).copied().unwrap_or(1.0);
        let b = self.level.get(i + 1).copied().unwrap_or(1.0);
        a + (b - a) * t
    }

    /// Applies normalization gain and the leveller at timeline time `edited_us` and, when
    /// normalizing, a soft limiter from -3 dBFS that never passes -1 dBFS.
    pub fn finish(&self, sample: f64, edited_us: f64) -> f64 {
        let x = sample * self.gain * self.level_gain(edited_us);
        if !self.limit || x.abs() <= LIMIT_KNEE {
            return x;
        }
        let headroom = LIMIT_CEILING - LIMIT_KNEE;
        x.signum() * (LIMIT_KNEE + headroom * ((x.abs() - LIMIT_KNEE) / headroom).tanh())
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
                let edited_a = clip.edited_at(a);
                let edited_b = clip.edited_at(b);
                for block in edited_a / VOICE_BLOCK_US..edited_b.div_ceil(VOICE_BLOCK_US) {
                    let edited = (block * VOICE_BLOCK_US).max(edited_a);
                    let Some(source) = clip.source_at(edited) else {
                        continue;
                    };
                    let file = source.clamp(a, b) - segment.start_us;
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

/// An edit's loudness before normalization, per 100 ms block of the timeline.
struct EditLoudness {
    /// Integrated, in LUFS.
    lufs: f64,
    /// The peak all but the loudest blocks stay under (`PEAK_PERCENTILE`).
    peak: f64,
    /// Each block's K-weighted energy and peak.
    energy: Vec<f64>,
    peaks: Vec<f64>,
}

/// Integrated loudness of blocks of `energy` played through the leveller `level` (its gain
/// at block edges; the mean of a block's two edges for the block).
fn levelled_lufs(energy: &[f64], level: &[f64]) -> Option<f64> {
    let levelled: Vec<f64> = energy
        .iter()
        .enumerate()
        .map(|(i, &e)| {
            let edge = |k: usize| level.get(k).copied().unwrap_or(1.0);
            let gain = (edge(i) + edge(i + 1)) / 2.0;
            e * gain * gain
        })
        .collect();
    integrated_lufs(&levelled)
}

/// The leveller's gain at each block edge for blocks peaking at `peaks` once boosted by
/// `gain`: low enough at both edges of a block that its peak stays at the limiter's ceiling,
/// turned down ahead of it by [`LEVEL_ATTACK_DB`] a block and back up by [`LEVEL_RELEASE_DB`].
/// Linear between the edges, so never above either edge inside a block. Empty when no block
/// needs it.
fn level_envelope(peaks: &[f64], gain: f64) -> Vec<f64> {
    let needed: Vec<f64> = peaks
        .iter()
        .map(|&p| (20.0 * (LIMIT_CEILING / (p * gain).max(1e-12)).log10()).min(0.0))
        .collect();
    if needed.iter().all(|&db| db >= 0.0) {
        return Vec::new();
    }
    let mut edges: Vec<f64> = (0..=needed.len())
        .map(|k| {
            let before = k.checked_sub(1).and_then(|i| needed.get(i)).copied();
            let after = needed.get(k).copied();
            before.unwrap_or(0.0).min(after.unwrap_or(0.0))
        })
        .collect();
    for k in 1..edges.len() {
        edges[k] = edges[k].min(edges[k - 1] + LEVEL_RELEASE_DB);
    }
    for k in (0..edges.len() - 1).rev() {
        edges[k] = edges[k].min(edges[k + 1] + LEVEL_ATTACK_DB);
    }
    edges.into_iter().map(|db| 10f64.powf(db / 20.0)).collect()
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
    fn preparing_effects_keeps_only_unique_audible_files() {
        use super::super::audio::LaneClip;
        let dir = tempfile::tempdir().unwrap();
        let wav = generate_pcm16_wav(48_000, 1, &vec![1_000; 48_000]);
        for name in ["first.wav", "second.wav", "unused.wav"] {
            std::fs::write(dir.path().join(name), &wav).unwrap();
        }
        let segment = |name: &str, start_us| SegmentSummary {
            track_id: "mic".into(),
            relative_path: name.into(),
            start_us,
            end_us: start_us + 1_000_000,
            size_bytes: wav.len() as u64,
            media_timescale: 48_000,
            media_start_value: 0,
            host_anchor_us: 0,
            is_keyframe_start: None,
            available: true,
        };
        let segments = Arc::new(vec![
            segment("first.wav", 0),
            segment("second.wav", 1_000_000),
            segment("unused.wav", 2_000_000),
        ]);
        let mut lanes = vec![Lane {
            id: "speech".into(),
            speech: true,
            gain: 1.0,
            clips: (0..20)
                .map(|i| LaneClip {
                    edited_start: i * 100_000,
                    in_us: i * 100_000,
                    len: 100_000,
                    edited_len: 100_000,
                    speed: 1.0,
                    source_end: 3_000_000,
                    segments: segments.clone(),
                    mutes: Default::default(),
                })
                .collect(),
        }];
        // Another lane uses the otherwise unused file, but has no enabled effect.
        lanes.push(Lane {
            id: "background".into(),
            speech: false,
            gain: 1.0,
            clips: vec![LaneClip {
                edited_start: 0,
                in_us: 2_000_000,
                len: 1_000_000,
                edited_len: 1_000_000,
                speed: 1.0,
                source_end: 3_000_000,
                segments,
                mutes: Default::default(),
            }],
        });
        let settings = crate::project::AudioSettings {
            noise_reduction: true,
            ..Default::default()
        };
        let analyses = prepare_analyses(dir.path(), &settings, &lanes);
        assert_eq!(analyses.len(), 2);
        assert!(analyses.contains_key("first.wav") && analyses.contains_key("second.wav"));
        // Results remain usable throughout the build without reopening their files.
        std::fs::remove_file(dir.path().join("first.wav")).unwrap();
        assert_eq!(analyses["first.wav"].voice.len(), 100);
        assert_eq!(analyses["second.wav"].loudness.len(), 10);
    }

    #[test]
    fn normalization_is_unchanged_when_a_file_is_split_into_many_clips() {
        use super::super::audio::LaneClip;
        let dir = tempfile::tempdir().unwrap();
        let samples: Vec<i16> = (0..96_000)
            .map(|i| (3_000.0 * (std::f32::consts::TAU * 220.0 * i as f32 / 48_000.0).sin()) as i16)
            .collect();
        let wav = generate_pcm16_wav(48_000, 1, &samples);
        std::fs::write(dir.path().join("speech.wav"), &wav).unwrap();
        let segments = Arc::new(vec![SegmentSummary {
            track_id: "mic".into(),
            relative_path: "speech.wav".into(),
            start_us: 0,
            end_us: 2_000_000,
            size_bytes: wav.len() as u64,
            media_timescale: 48_000,
            media_start_value: 0,
            host_anchor_us: 0,
            is_keyframe_start: None,
            available: true,
        }]);
        let lane = |count| Lane {
            id: "speech".into(),
            speech: true,
            gain: 1.0,
            clips: (0..count)
                .map(|i| LaneClip {
                    edited_start: i * (2_000_000 / count),
                    in_us: i * (2_000_000 / count),
                    len: 2_000_000 / count,
                    edited_len: 2_000_000 / count,
                    speed: 1.0,
                    source_end: 2_000_000,
                    segments: segments.clone(),
                    mutes: Default::default(),
                })
                .collect(),
        };
        let settings = crate::project::AudioSettings {
            normalize: true,
            ..Default::default()
        };
        let whole = PolishPlan::build(dir.path(), &settings, &[lane(1)], 2_000_000).unwrap();
        let cuts = PolishPlan::build(dir.path(), &settings, &[lane(200)], 2_000_000).unwrap();
        assert!(whole.gain > 1.0);
        assert!((whole.gain - cuts.gain).abs() < 1e-12);
    }

    #[test]
    fn the_leveller_keeps_peaks_under_the_limiter_and_normalization_stops_past_it() {
        use super::super::audio::LaneClip;
        let dir = tempfile::tempdir().unwrap();
        // Quiet talk (about -38 LUFS) with a sharp transient every 50 ms near -6 dBFS.
        let samples: Vec<i16> = (0..96_000)
            .map(|i| {
                if i % 2_400 == 0 {
                    16_000
                } else {
                    (600.0 * (std::f32::consts::TAU * 220.0 * i as f32 / 48_000.0).sin()) as i16
                }
            })
            .collect();
        let wav = generate_pcm16_wav(48_000, 1, &samples);
        std::fs::write(dir.path().join("speech.wav"), &wav).unwrap();
        let segments = Arc::new(vec![SegmentSummary {
            track_id: "mic".into(),
            relative_path: "speech.wav".into(),
            start_us: 0,
            end_us: 2_000_000,
            size_bytes: wav.len() as u64,
            media_timescale: 48_000,
            media_start_value: 0,
            host_anchor_us: 0,
            is_keyframe_start: None,
            available: true,
        }]);
        let lane = Lane {
            id: "speech".into(),
            speech: true,
            gain: 1.0,
            clips: vec![LaneClip {
                edited_start: 0,
                in_us: 0,
                len: 2_000_000,
                edited_len: 2_000_000,
                speed: 1.0,
                source_end: 2_000_000,
                segments,
                mutes: Default::default(),
            }],
        };
        let settings = crate::project::AudioSettings {
            normalize: true,
            ..Default::default()
        };
        let plan = PolishPlan::build(dir.path(), &settings, &[lane], 2_000_000).unwrap();
        let peak = 16_000.0 / 32_768.0;
        // The loudness target alone asks for the full 20 dB; the peaks allow about 5, and the
        // leveller [`LEVEL_MAX_DB`] more.
        let limit = LIMIT_CEILING * 10f64.powf(LEVEL_MAX_DB / 20.0);
        assert!(plan.gain * peak > LIMIT_CEILING * 1.5, "gain {}", plan.gain);
        assert!(plan.gain * peak <= limit + 1e-3, "gain {}", plan.gain);
        // Every block holds a transient: turned down all the way, it stays at the ceiling.
        for ms in (0..2_000).step_by(7) {
            let level = plan.level_gain(ms as f64 * 1_000.0);
            assert!(
                plan.gain * level * peak <= LIMIT_CEILING + 1e-3,
                "{ms} ms: {level}"
            );
        }
    }

    #[test]
    fn the_leveller_turns_down_one_loud_block_and_ramps_back() {
        // Blocks under the ceiling, one 5.6 dB over it at block 5.
        let mut peaks = vec![0.5; 20];
        peaks[5] = LIMIT_CEILING * 1.9;
        let level = level_envelope(&peaks, 1.0);
        assert_eq!(level.len(), 21);
        // Both edges of the loud block hold it at the ceiling.
        let held = 1.0 / 1.9;
        assert!((level[5] - held).abs() < 1e-9 && (level[6] - held).abs() < 1e-9);
        // Turned down one block ahead (6 dB a block), back up over four after.
        assert!((level[4] - 1.0).abs() < 1e-9);
        assert!(level[7] > held && level[7] < 1.0);
        assert!((level[10] - 1.0).abs() < 1e-9);
        assert!(level_envelope(&[0.5; 20], 1.0).is_empty());
    }

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
            level: Vec::new(),
            denoisers: HashMap::new(),
        };
        let finish = |s: f64| plan.finish(s, 0.0);
        assert!((finish(0.2) - 0.4).abs() < 1e-12);
        assert!((finish(0.35) - 0.7).abs() < 1e-12);
        assert!(finish(5.0) <= LIMIT_CEILING && finish(5.0) > 0.89);
        assert!(finish(0.5) < LIMIT_CEILING && finish(0.5) > LIMIT_KNEE);
        assert!(finish(-5.0) >= -LIMIT_CEILING && finish(-0.5) > -LIMIT_CEILING);
    }
}
