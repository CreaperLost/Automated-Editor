//! Mouth noises, knocks and breaths in a speech recording: sound that is not voice.
//!
//! Word timings cannot find them: a recognizer stretches a word over the click or smack next to
//! it, and trimming quiet word edges stops at the noise because it is loud. So they are found
//! in the sound itself.
//!
//! The recording is split into stretches of sound: where it rises above the room's quiet (in
//! any of three bands, so a faint bright click or low thump counts) with no gap of
//! [`BRIDGE_US`] inside. A stretch with voice pitch in it is speech, with the consonants joined
//! to its vowels; a stretch without is a noise, whole: its lead-in, every click of a cluster and
//! its decay, so muting it leaves nothing of it behind. Noises a short gap apart are one.
//!
//! A noise stuck to a word with no gap (a smack right before it) is part of that word's
//! stretch; such noises are found by their sudden bright onset in the word's unvoiced edge.
//! A noise close to speech may be a consonant (the release of a "t" after its closure), so it
//! is marked as such: worth a listen, not muted by default.
use serde::{Deserialize, Serialize};

/// Clicks and smacks are measured above this.
pub const CLICK_BAND_HZ: f64 = 2_500.0;
/// Knocks and thumps are measured below this.
pub const KNOCK_BAND_HZ: f64 = 150.0;
/// Rumble below this is not sound of interest.
const RUMBLE_HZ: f64 = 80.0;
/// Level frames: 2 ms.
const HOP_US: u64 = 2_000;
/// A gap in a sound shorter than this does not end it (between the clicks of a smack).
const BRIDGE_US: u64 = 16_000;
/// Noises closer than this, with no speech between, are one.
const JOIN_US: u64 = 60_000;
/// A sound without voice longer than this is not a noise to mute (steady background, music).
const MAX_NOISE_US: u64 = 700_000;
/// A noise reaches out over its edges while above the quiet by this share of the rise that
/// found it.
const EDGE_RISE: f64 = 0.4;
/// Bright sound held this long within [`STEADY_DB`] of its loudest is a hiss of speech.
const MIN_HISS_US: u64 = 40_000;
const STEADY_DB: f64 = 10.0;
/// ...if it is within this of the loudest voice within a second (a faint steady noise is not).
const HISS_BELOW_VOICE_DB: f64 = 18.0;
/// A word's unvoiced sound (a consonant) runs at most this far past its voice...
const CONSONANT_US: u64 = 150_000;
/// ...unless it falls to within this of the room's quiet first (a gap)...
const NEAR_ROOM_DB: f64 = 6.0;
/// ...or to a trough this far below the voice...
const DEEP_DIP_DB: f64 = 25.0;
/// ...that new sound rises this far out of.
const RISE_AFTER_DIP_DB: f64 = 4.0;
/// Sound past a word's edge no longer than this may be the word's consonant.
const MAX_EDGE_CONSONANT_US: u64 = 200_000;
/// Sound past a word's edge is a noise only if most of it is this far below the word's voice.
const BELOW_VOICE_DB: f64 = 15.0;
/// A word's voice level: its loudest in this much of it next to the edge.
const WORD_LEVEL_US: u64 = 300_000;
/// Sound past a word's edge shorter than this is not looked at as a noise.
const MIN_PIECE_US: u64 = 20_000;
/// Pitch shorter than this...
const STRAY_VOICE_US: u64 = 60_000;
/// ...and this much quieter than the voice within a second is not a word.
const STRAY_VOICE_DB: f64 = 15.0;
/// A noise this long without a sudden onset is a breath.
const BREATH_US: u64 = 250_000;
/// Speech within this long before a short click makes it "near speech": it may be a word-final
/// "t" released after its closure...
const NEAR_BEFORE_US: u64 = 120_000;
/// ...if it is no longer than this.
const MAX_RELEASE_US: u64 = 40_000;
/// Release-like clicks running on longer than this together are a mouth noise.
const RELEASE_RUN_US: u64 = 120_000;
/// Speech starting within this long after a noise makes it "near speech": it may be the word's
/// first sound.
const NEAR_AFTER_US: u64 = 25_000;
/// A muted noise stays this far from speech, padding or not.
const GUARD_US: u64 = 10_000;
/// How much sound before a bright onset it is compared with.
const ONSET_SIDE_US: u64 = 20_000;
/// A noise found by its onset inside a word's edge stays this far from the word's voice.
const ONSET_GUARD_US: u64 = 30_000;
/// ...and is over within this long: longer bright sound there is a consonant.
const MAX_EDGE_CLICK_US: u64 = 40_000;
/// Quieter than this is digital silence, not a room.
const SILENT_DB: f64 = -95.0;
/// Nothing quieter than this is sound to look at.
const MIN_LEVEL_DB: f64 = -75.0;
/// The most room `padding_ms` may add around a noise.
pub const MAX_PADDING_MS: u32 = 200;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NoiseKind {
    /// A click, tongue click or a cluster of them: a sudden bright onset.
    Click,
    /// A knock, thump or pop on the desk or microphone: mostly low sound.
    Knock,
    /// A lip smack, wet mouth sound or rustle: short, without a sharp click.
    Mouth,
    /// A breath: longer and smooth.
    Breath,
}

/// One noise, in microseconds from the start of the sound analyzed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Noise {
    /// What to mute: the noise with the room asked for around it.
    pub start_us: u64,
    pub end_us: u64,
    pub kind: NoiseKind,
    /// How far its loudest moment stands above the room's quiet, in dB.
    pub contrast_db: f32,
    /// Speech is right before or after it: it may be part of a word.
    pub near_speech: bool,
    /// It may be part of a word only as a released "t", "k" or "p": one short click.
    #[serde(skip)]
    pub(crate) release_only: bool,
    /// The sound itself, without the room muted around it.
    #[serde(skip)]
    pub(crate) core_us: (u64, u64),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoiseConfig {
    /// 0 finds only the most obvious noises, 1 also faint ones.
    pub sensitivity: f32,
    /// Extra room muted either side of a noise (never into speech), in ms.
    #[serde(default = "default_padding_ms")]
    pub padding_ms: u32,
}

fn default_padding_ms() -> u32 {
    20
}

impl Default for NoiseConfig {
    fn default() -> Self {
        Self {
            sensitivity: 0.5,
            padding_ms: default_padding_ms(),
        }
    }
}

impl NoiseConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !self.sensitivity.is_finite() || !(0.0..=1.0).contains(&self.sensitivity) {
            return Err("Sensitivity must be between 0 and 1".into());
        }
        if self.padding_ms > MAX_PADDING_MS {
            return Err(format!(
                "Room around a noise is at most {MAX_PADDING_MS} ms"
            ));
        }
        Ok(())
    }

    /// How far above the room's quiet a sound must rise to count, in dB.
    fn rise_db(&self) -> f64 {
        15.0 - 9.0 * self.sensitivity as f64
    }

    /// How far a bright onset must jump above what came just before, in dB.
    fn onset_db(&self) -> f64 {
        20.0 - 10.0 * self.sensitivity as f64
    }
}

/// A second-order filter (RBJ cookbook).
struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
    x: [f64; 2],
    y: [f64; 2],
}

impl Biquad {
    fn new(rate: u32, hz: f64, high: bool) -> Self {
        let w = std::f64::consts::TAU * hz / rate as f64;
        let alpha = w.sin() / (2.0 * std::f64::consts::FRAC_1_SQRT_2);
        let cos = w.cos();
        let a0 = 1.0 + alpha;
        let b = if high {
            [(1.0 + cos) / 2.0, -(1.0 + cos), (1.0 + cos) / 2.0]
        } else {
            [(1.0 - cos) / 2.0, 1.0 - cos, (1.0 - cos) / 2.0]
        };
        Self {
            b: [b[0] / a0, b[1] / a0, b[2] / a0],
            a: [-2.0 * cos / a0, (1.0 - alpha) / a0],
            x: [0.0; 2],
            y: [0.0; 2],
        }
    }

    fn run(&mut self, x: f64) -> f64 {
        let y = self.b[0] * x + self.b[1] * self.x[0] + self.b[2] * self.x[1]
            - self.a[0] * self.y[0]
            - self.a[1] * self.y[1];
        self.x = [x, self.x[0]];
        self.y = [y, self.y[0]];
        y
    }
}

fn db(energy: f64) -> f64 {
    10.0 * (energy + 1e-12).log10()
}

/// Mean energy of `levels` (dB) over `[a, b)`, in dB; `None` when empty.
fn mean_db(levels: &[f64], a: usize, b: usize) -> Option<f64> {
    let b = b.min(levels.len());
    (b > a).then(|| {
        db(levels[a..b]
            .iter()
            .map(|l| 10f64.powf(l / 10.0))
            .sum::<f64>()
            / (b - a) as f64)
    })
}

/// Voiced speech per frame of `HOP_US`: periodic (a pitch between 70 and 400 Hz) between 200
/// Hz and 1 kHz, where a voice's harmonics are and a thump's rumble is not, and well above the
/// quietest sound there.
fn voiced_frames(mono: &[f32], rate: u32, frames: usize) -> Vec<bool> {
    // At about 8 kHz: plenty for pitch.
    let step = (rate as usize / 8_000).max(1);
    let low_rate = rate as f64 / step as f64;
    let mut below = Biquad::new(rate, 1_000.0, false);
    let mut above = Biquad::new(rate, 200.0, true);
    let band: Vec<f64> = mono
        .iter()
        .map(|&s| above.run(below.run(s as f64)))
        .step_by(step)
        .collect();
    let window = (low_rate * 0.032) as usize;
    let (min_lag, max_lag) = ((low_rate / 400.0) as usize, (low_rate / 70.0) as usize);
    // Pitch every 10 ms, then spread to the 2 ms frames.
    let every = 5;
    let centres: Vec<usize> = (0..frames)
        .step_by(every)
        .map(|frame| (frame as f64 * HOP_US as f64 / 1e6 * low_rate) as usize)
        .collect();
    let level = |centre: usize| {
        let start = centre.saturating_sub(window / 2).min(band.len());
        let x = &band[start..(start + window).min(band.len())];
        db(x.iter().map(|v| v * v).sum::<f64>() / x.len().max(1) as f64)
    };
    let levels: Vec<f64> = centres.iter().map(|&c| level(c)).collect();
    let sounding: Vec<f64> = levels.iter().copied().filter(|&l| l > SILENT_DB).collect();
    let floor = percentile(&sounding, 0.1);
    let mut voiced = vec![false; frames];
    for (i, &centre) in centres.iter().enumerate() {
        let frame = i * every;
        let start = centre.saturating_sub(window / 2);
        if levels[i] > floor + 15.0 && start + window + max_lag < band.len() {
            let low = &band;
            let x = &low[start..start + window];
            let energy: f64 = x.iter().map(|v| v * v).sum();
            // A pitch is a peak of the correlation inside the range, not its rising edge (a
            // slow rumble keeps rising towards its own, longer period).
            let correlation: Vec<f64> = (min_lag - 1..=max_lag + 1)
                .map(|lag| {
                    let y = &low[start + lag..start + lag + window];
                    let (mut cross, mut lagged) = (0.0, 0.0);
                    for (a, b) in x.iter().zip(y) {
                        cross += a * b;
                        lagged += b * b;
                    }
                    cross / (energy * lagged).sqrt().max(1e-12)
                })
                .collect();
            let best = correlation
                .windows(3)
                .filter(|w| w[1] >= w[0] && w[1] >= w[2])
                .map(|w| w[1])
                .fold(0.0, f64::max);
            if best >= 0.65 {
                for v in &mut voiced[frame.saturating_sub(every / 2)..(frame + every).min(frames)] {
                    *v = true;
                }
            }
        }
    }
    voiced
}

fn percentile(values: &[f64], p: f64) -> f64 {
    if values.is_empty() {
        return -120.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    sorted[((sorted.len() - 1) as f64 * p) as usize]
}

/// Frame levels of a sound in the bands noises and speech are told apart by.
struct Levels {
    /// Above [`CLICK_BAND_HZ`].
    bright: Vec<f64>,
    /// Below [`KNOCK_BAND_HZ`].
    deep: Vec<f64>,
    /// Everything above [`RUMBLE_HZ`].
    broad: Vec<f64>,
}

impl Levels {
    fn of(mono: &[f32], rate: u32, hop: usize) -> Self {
        let frames = mono.len() / hop;
        let mut high = Biquad::new(rate, CLICK_BAND_HZ, true);
        let mut low = Biquad::new(rate, KNOCK_BAND_HZ, false);
        let mut above_rumble = Biquad::new(rate, RUMBLE_HZ, true);
        let mut levels = Self {
            bright: Vec::with_capacity(frames),
            deep: Vec::with_capacity(frames),
            broad: Vec::with_capacity(frames),
        };
        for chunk in mono.chunks_exact(hop) {
            let (mut h, mut l, mut b) = (0.0, 0.0, 0.0);
            for &s in chunk {
                let s = s as f64;
                let (hs, ls, bs) = (high.run(s), low.run(s), above_rumble.run(s));
                h += hs * hs;
                l += ls * ls;
                b += bs * bs;
            }
            let n = hop as f64;
            levels.bright.push(db(h / n));
            levels.deep.push(db(l / n));
            levels.broad.push(db(b / n));
        }
        levels
    }
}

/// The room's quiet in `levels`: its tenth percentile, digital silence left out.
fn quiet_of(levels: &[f64]) -> f64 {
    let sounding: Vec<f64> = levels.iter().copied().filter(|&l| l > SILENT_DB).collect();
    percentile(&sounding, 0.1)
}

/// The longest run of frames in `[a, b)` that stay within [`STEADY_DB`] of the loudest.
fn steady(levels: &[f64], a: usize, b: usize) -> usize {
    let top = levels[a..b].iter().copied().fold(f64::MIN, f64::max);
    let (mut best, mut run) = (0, 0);
    for &level in &levels[a..b] {
        run = if level >= top - STEADY_DB { run + 1 } else { 0 };
        best = best.max(run);
    }
    best
}

/// A stretch of sound: frames `[start, end)`, and whether there is voice in it.
#[derive(Clone, Copy, Debug)]
struct Stretch {
    start: usize,
    end: usize,
    voiced: bool,
}

/// Sound that may be a noise: frames `[start, end)`, which may grow out to `[lo, hi)`; the
/// speech (or a word's own sound) ending before it at `before` and starting after it at `after`.
#[derive(Clone, Copy, Debug)]
struct Piece {
    start: usize,
    end: usize,
    lo: usize,
    hi: usize,
    before: Option<usize>,
    after: Option<usize>,
    /// It is past a word's edge in the same stretch of sound.
    word_edge: bool,
}

/// The runs of voiced frames in `[a, b)`, as `(start, end)`.
fn voiced_runs(voiced: &[bool], a: usize, b: usize) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut i = a;
    while i < b {
        if !voiced[i] {
            i += 1;
            continue;
        }
        let start = i;
        while i < b && voiced[i] {
            i += 1;
        }
        runs.push((start, i));
    }
    runs
}

/// `voiced` without stray pitch: a short run of it far quieter than the voice around it is
/// something in a noise that happens to repeat (a wet click train, a whine), not a word.
fn clean_voice(mut voiced: Vec<bool>, broad: &[f64]) -> Vec<bool> {
    let runs = voiced_runs(&voiced, 0, voiced.len());
    let short = (STRAY_VOICE_US / HOP_US) as usize;
    let around = (1_000_000 / HOP_US) as usize;
    let loudest = |a: usize, b: usize| broad[a..b].iter().copied().fold(f64::MIN, f64::max);
    for (k, &(a, b)) in runs.iter().enumerate() {
        if b - a >= short {
            continue;
        }
        // The loudest other voice within a second either side.
        let others = runs
            .iter()
            .enumerate()
            .filter(|&(j, &(x, y))| j != k && y + around > a && x < b + around)
            .map(|(_, &(x, y))| loudest(x, y))
            .fold(f64::MIN, f64::max);
        if others > f64::MIN && loudest(a, b) < others - STRAY_VOICE_DB {
            voiced[a..b].fill(false);
        }
    }
    voiced
}

/// Walking from a word's voice edge at `from` to `limit` (forwards or backwards), where the
/// word's own sound ends, as a frame boundary: where `smooth` falls to `room`, or the bottom of
/// a trough at least [`DEEP_DIP_DB`] below `word` that sound rises [`RISE_AFTER_DIP_DB`] out of.
fn word_edge(smooth: &[f64], from: usize, limit: usize, word: f64, room: f64) -> usize {
    let forward = limit >= from;
    let mut trough = (f64::MAX, from);
    let mut i = from;
    while i != limit {
        let level = if forward { smooth[i] } else { smooth[i - 1] };
        if level <= room {
            return i;
        }
        if level < trough.0 {
            trough = (level, i);
        } else if trough.0 <= word - DEEP_DIP_DB && level >= trough.0 + RISE_AFTER_DIP_DB {
            return trough.1;
        }
        i = if forward { i + 1 } else { i - 1 };
    }
    limit
}

/// Finds the noises in `mono` (one channel at `rate`), in time from its first sample.
pub fn find_noises(mono: &[f32], rate: u32, config: &NoiseConfig) -> Vec<Noise> {
    let hop = (rate as u64 * HOP_US / 1_000_000) as usize;
    if hop == 0 || mono.len() < hop * 4 {
        return Vec::new();
    }
    let frames = mono.len() / hop;
    let levels = Levels::of(mono, rate, hop);
    let voiced = clean_voice(voiced_frames(mono, rate, frames), &levels.broad);
    let frames_of = |us: u64| (us / HOP_US) as usize;

    // Sound: any band well above its own quiet.
    let rise = config.rise_db();
    let quiet = [
        quiet_of(&levels.broad),
        quiet_of(&levels.bright),
        quiet_of(&levels.deep),
    ];
    let above = |i: usize, rise: f64| {
        [&levels.broad, &levels.bright, &levels.deep]
            .iter()
            .zip(quiet)
            .any(|(band, quiet)| band[i] > (quiet + rise).max(MIN_LEVEL_DB))
    };
    let sounding = |i: usize| above(i, rise);
    // A noise's soft lead-in and decay: still above the quiet, if only just.
    let faint = |i: usize| above(i, (rise * EDGE_RISE).max(3.0));
    // Stretches of sound. Low rumble alone starts one (a knock) but carries none on: it would
    // join a noise to the word beside it across a gap of room sound.
    let bridge = frames_of(BRIDGE_US);
    let clear = |i: usize| {
        [&levels.broad, &levels.bright]
            .iter()
            .zip(quiet)
            .any(|(band, quiet)| band[i] > (quiet + rise).max(MIN_LEVEL_DB))
    };
    let mut stretches: Vec<Stretch> = Vec::new();
    let mut i = 0;
    while i < frames {
        if !sounding(i) {
            i += 1;
            continue;
        }
        let start = i;
        let mut low_only = !clear(i);
        let mut end = i + 1;
        let mut gap = 0;
        i += 1;
        while i < frames && gap < bridge {
            if clear(i) || (low_only && sounding(i)) {
                low_only &= !clear(i);
                end = i + 1;
                gap = 0;
            } else {
                gap += 1;
            }
            i += 1;
        }
        stretches.push(Stretch {
            start,
            end,
            voiced: voiced[start..end].iter().any(|&v| v),
        });
        i = end;
    }

    let speech: Vec<&Stretch> = stretches.iter().filter(|s| s.voiced).collect();
    // The speech ending at or before frame `at`, and starting at or after it.
    let speech_before = |at: usize| {
        let k = speech.partition_point(|s| s.end <= at);
        k.checked_sub(1).map(|k| speech[k].end)
    };
    let speech_after = |at: usize| {
        let k = speech.partition_point(|s| s.start < at);
        speech.get(k).map(|s| s.start)
    };
    let pad = (config.padding_ms as u64 * 1_000 / HOP_US) as usize;
    let guard = frames_of(GUARD_US);
    let peak =
        |band: &[f64], a: usize, b: usize| band[a..b].iter().copied().fold(f64::MIN, f64::max);
    let onset = config.onset_db();
    let side = frames_of(ONSET_SIDE_US);
    // The most a bright onset in `[a, b)` jumps above the 20 ms before it, in dB.
    let sharpest = |a: usize, b: usize| {
        (a..b)
            .filter_map(|i| {
                Some(levels.bright[i] - mean_db(&levels.bright, i.checked_sub(side + 1)?, i - 1)?)
            })
            .fold(f64::MIN, f64::max)
    };

    // Where a word's own sound ends past its voice: where it falls to the room, or at the
    // bottom of a deep trough that new sound rises out of; a consonant's length on at most (an
    // "s" runs on without a drop). Levels smoothed over 10 ms, so a vowel's wobble is no trough.
    let consonant = frames_of(CONSONANT_US);
    let word = frames_of(WORD_LEVEL_US);
    let smooth: Vec<f64> = (0..frames)
        .map(|i| {
            mean_db(&levels.broad, i.saturating_sub(2), (i + 3).min(frames)).unwrap_or(f64::MIN)
        })
        .collect();
    let room = quiet[0] + rise + NEAR_ROOM_DB;
    let dip_after = |voice_end: usize, limit: usize| {
        let level = peak(&levels.broad, voice_end.saturating_sub(word), voice_end);
        word_edge(
            &smooth,
            voice_end,
            (voice_end + consonant).min(limit),
            level,
            room,
        )
    };
    let dip_before = |voice_start: usize, limit: usize| {
        let level = peak(&levels.broad, voice_start, (voice_start + word).min(frames));
        word_edge(
            &smooth,
            voice_start,
            voice_start.saturating_sub(consonant).max(limit),
            level,
            room,
        )
    };

    // What may be noise: stretches without voice, and the sound past a word's own edges
    // inside stretches with voice (a smack or breath running straight on from a word).
    let mut pieces: Vec<Piece> = Vec::new();
    for stretch in &stretches {
        let runs = voiced_runs(&voiced, stretch.start, stretch.end);
        if runs.is_empty() {
            // Low ringing for a moment (a filter's tail after a sudden stop) is no knock.
            if stretch.end - stretch.start < frames_of(MIN_PIECE_US)
                && !(stretch.start..stretch.end).any(&clear)
            {
                continue;
            }
            let before = speech_before(stretch.start);
            let after = speech_after(stretch.end);
            pieces.push(Piece {
                start: stretch.start,
                end: stretch.end,
                lo: before.map_or(0, |end| end + guard),
                hi: after.map_or(frames, |start| start.saturating_sub(guard)),
                before,
                after,
                word_edge: false,
            });
            continue;
        }
        let mut cursor = (stretch.start, None);
        let mut gaps = Vec::new();
        for &(voice_start, voice_end) in &runs {
            gaps.push((cursor.0, voice_start, cursor.1, Some(voice_start)));
            cursor = (voice_end, Some(voice_end));
        }
        gaps.push((cursor.0, stretch.end, cursor.1, None));
        for (a, b, voice_before, voice_after) in gaps {
            if b <= a {
                continue;
            }
            let start = voice_before.map_or(a, |end| dip_after(end, b));
            let end = voice_after.map_or(b, |start| dip_before(start, a));
            if end < start + frames_of(MIN_PIECE_US) {
                continue;
            }
            // Its own sound, not the fading end of the word: it rises again, or clicks.
            let rises = peak(&levels.broad, (start + 5).min(end - 1), end)
                >= levels.broad[start] + 3.0
                || sharpest(start, end) >= onset;
            #[cfg(test)]
            if std::env::var("NOISE_TRACE").is_ok() && (!rises || !(start..end).any(&sounding)) {
                eprintln!(
                    "dropped gap {:.3}-{:.3} cut {:.3}-{:.3} rises {}",
                    a as f64 * 0.002,
                    b as f64 * 0.002,
                    start as f64 * 0.002,
                    end as f64 * 0.002,
                    rises
                );
            }
            if !rises || !(start..end).any(&sounding) {
                continue;
            }
            // Next to a word, only sound well below its voice for most of it (a click in it
            // may be loud): the word's own consonants are loud throughout.
            let voice = voice_before
                .map(|end| peak(&levels.broad, end.saturating_sub(word), end))
                .into_iter()
                .chain(
                    voice_after.map(|start| peak(&levels.broad, start, (start + word).min(frames))),
                )
                .fold(f64::MIN, f64::max);
            #[cfg(test)]
            if std::env::var("NOISE_TRACE").is_ok() {
                eprintln!(
                    "gap {:.3}-{:.3} cut {:.3}-{:.3} voice {:.1} piece body {:.1} rises {}",
                    a as f64 * 0.002,
                    b as f64 * 0.002,
                    start as f64 * 0.002,
                    end as f64 * 0.002,
                    voice,
                    percentile(&levels.broad[start..end], 0.75),
                    rises
                );
            }
            if percentile(&levels.broad[start..end], 0.75) > voice - BELOW_VOICE_DB {
                continue;
            }
            let before = voice_before.map(|_| start).or_else(|| speech_before(a));
            let after = match voice_after {
                // A drop separates it from the word after.
                Some(_) => None,
                None => speech_after(b),
            };
            pieces.push(Piece {
                start,
                end,
                lo: voice_before.map_or(before.map_or(0, |end| end + guard), |_| start),
                hi: voice_after.map_or(after.map_or(frames, |s| s.saturating_sub(guard)), |_| end),
                before,
                after,
                word_edge: true,
            });
        }
    }

    let mut found = Vec::new();
    // `NOISE_TRACE=<from>,<to>` (seconds into this sound) prints what the probe sees there.
    #[cfg(test)]
    if let Some((from, to)) = std::env::var("NOISE_TRACE").ok().and_then(|v| {
        let (a, b) = v.split_once(',')?;
        Some((a.parse::<f64>().ok()?, b.parse::<f64>().ok()?))
    }) {
        let frame = |s: f64| ((s * 1e6) as u64 / HOP_US) as usize;
        eprintln!(
            "quiet broad/bright/deep {:.1}/{:.1}/{:.1} dB, rise {:.1}",
            quiet[0], quiet[1], quiet[2], rise
        );
        for s in stretches
            .iter()
            .filter(|s| s.end > frame(from) && s.start < frame(to))
        {
            eprintln!(
                "stretch {:.3}-{:.3} voiced {}",
                s.start as f64 * 0.002,
                s.end as f64 * 0.002,
                s.voiced
            );
        }
        for p in pieces
            .iter()
            .filter(|p| p.end > frame(from) && p.start < frame(to))
        {
            eprintln!(
                "piece {:.3}-{:.3}",
                p.start as f64 * 0.002,
                p.end as f64 * 0.002
            );
        }
        for i in (frame(from)..frame(to).min(frames)).step_by(5) {
            eprintln!(
                "{:.3} broad {:+5.1} bright {:+5.1} deep {:+5.1} {}{}",
                i as f64 * 0.002,
                levels.broad[i] - quiet[0],
                levels.bright[i] - quiet[1],
                levels.deep[i] - quiet[2],
                if voiced[i] { "V" } else { "." },
                if sounding(i) { "S" } else { "." }
            );
        }
    }
    for piece in &pieces {
        let (a, b) = (piece.start, piece.end);
        if (b - a) as u64 * HOP_US > MAX_NOISE_US {
            continue;
        }
        let (before, after) = (piece.before, piece.after);
        let sharp = sharpest(a, b) >= onset;
        let kind = if peak(&levels.deep, a, b) > peak(&levels.bright, a, b) + 6.0 {
            NoiseKind::Knock
        } else if sharp {
            NoiseKind::Click
        } else if (b - a) as u64 * HOP_US >= BREATH_US {
            NoiseKind::Breath
        } else {
            NoiseKind::Mouth
        };
        // A word-final "t", "k" or "p" is one short click a closure's length after the word;
        // a sound almost touching the next word may be how it starts.
        let release = kind == NoiseKind::Click
            && (b - a) as u64 * HOP_US <= MAX_RELEASE_US
            && before.is_some_and(|end| ((a - end) as u64) * HOP_US < NEAR_BEFORE_US);
        // Bright sound held steady, about as loud as the voice nearby, is a hiss of speech
        // ("s", "sh", "f"), not the separate clicks of a smack or a faint steady noise.
        let near_voice = {
            let around = frames_of(1_000_000);
            let (x, y) = (a.saturating_sub(around), (b + around).min(frames));
            (x..y)
                .filter(|&i| voiced[i])
                .map(|i| levels.broad[i])
                .fold(f64::MIN, f64::max)
        };
        let hiss = steady(&levels.bright, a, b) as u64 * HOP_US >= MIN_HISS_US
            && peak(&levels.broad, a, b) >= near_voice - HISS_BELOW_VOICE_DB;
        // A short sound just past a word's edge may be its consonant ("s" of "st...", "t", "k"):
        // short counting its faint body too, which a consonant does not trail.
        let whole = {
            let (mut x, mut y) = (a, b);
            while x > piece.lo && faint(x - 1) {
                x -= 1;
            }
            while y < piece.hi && faint(y) {
                y += 1;
            }
            y - x
        };
        let consonant_like = piece.word_edge && whole as u64 * HOP_US <= MAX_EDGE_CONSONANT_US;
        let after_close = after.is_some_and(|start| ((start - b) as u64) * HOP_US < NEAR_AFTER_US);
        let near_speech = release || hiss || consonant_like || after_close;
        #[cfg(test)]
        if std::env::var("NOISE_TRACE").is_ok() {
            eprintln!(
                "class {:.3}-{:.3} {:?} release {} hiss {} near {} steady {}ms",
                a as f64 * 0.002,
                b as f64 * 0.002,
                kind,
                release,
                hiss,
                near_speech,
                steady(&levels.bright, a, b) * 2
            );
        }
        let (lo, hi) = (piece.lo, piece.hi);
        // Out to where it fades into the room, then the room asked for.
        let (mut a, mut b) = (a, b);
        while a > lo && faint(a - 1) {
            a -= 1;
        }
        while b < hi && faint(b) {
            b += 1;
        }
        let (start, end) = (
            a.saturating_sub(pad).max(lo.min(a)),
            (b + pad).min(hi.max(b)),
        );
        found.push(Noise {
            start_us: start as u64 * HOP_US,
            end_us: end as u64 * HOP_US,
            kind,
            contrast_db: (peak(&levels.broad, a, b) - quiet[0]) as f32,
            near_speech,
            release_only: release && !hiss && !consonant_like && !after_close,
            core_us: (a as u64 * HOP_US, b as u64 * HOP_US),
        });
    }

    // Noises stuck to a word: a sudden bright onset in the unvoiced edge of a speech stretch,
    // well away from its voice.
    let onset_guard = frames_of(ONSET_GUARD_US);
    for stretch in speech {
        let first_voice = (stretch.start..stretch.end)
            .find(|&i| voiced[i])
            .unwrap_or(stretch.end);
        let last_voice = (stretch.start..stretch.end)
            .rev()
            .find(|&i| voiced[i])
            .unwrap_or(stretch.start);
        for (from, to) in [
            (stretch.start, first_voice.saturating_sub(onset_guard)),
            (last_voice + 1 + onset_guard, stretch.end),
        ] {
            let mut i = from.max(side + 1);
            while i < to {
                let Some(base) = mean_db(&levels.bright, i - side - 1, i - 1) else {
                    i += 1;
                    continue;
                };
                if levels.bright[i] - base < onset || levels.bright[i] < MIN_LEVEL_DB {
                    i += 1;
                    continue;
                }
                let mut end = i + 1;
                while end < to && levels.bright[end] > base + onset / 2.0 {
                    end += 1;
                }
                // A click dies away; bright sound that carries on is a consonant ("s").
                if (end - i) as u64 * HOP_US > MAX_EDGE_CLICK_US {
                    i = end;
                    continue;
                }
                let small = pad.min(frames_of(5_000));
                found.push(Noise {
                    start_us: i.saturating_sub(small).max(from) as u64 * HOP_US,
                    end_us: (end + small).min(to) as u64 * HOP_US,
                    kind: NoiseKind::Click,
                    contrast_db: (peak(&levels.bright, i, end) - base) as f32,
                    near_speech: true,
                    release_only: false,
                    core_us: (i as u64 * HOP_US, end as u64 * HOP_US),
                });
                i = end;
            }
        }
    }
    join(found, &voiced)
}

/// Sorts noises and makes those that overlap or are a short gap apart, with no voice between,
/// one (a click inside a knock is the knock).
fn join(mut noises: Vec<Noise>, voiced: &[bool]) -> Vec<Noise> {
    noises.sort_by_key(|n| n.start_us);
    let mut out: Vec<Noise> = Vec::new();
    for noise in noises {
        let joins = out.last().is_some_and(|last| {
            let (gap_from, gap_to) = (
                (last.end_us / HOP_US) as usize,
                (noise.start_us / HOP_US) as usize,
            );
            // What may be speech never drags a clear noise (or its mute) along with it.
            noise.near_speech == last.near_speech
                && noise.start_us <= last.end_us + JOIN_US
                && voiced
                    .get(gap_from..gap_to.max(gap_from).min(voiced.len()))
                    .is_none_or(|gap| !gap.iter().any(|&v| v))
        });
        // A clear noise and one that may be speech, overlapping: the longer one stands (a
        // sliver at the edge of the other is part of it).
        if let Some(last) = out.last_mut() {
            if noise.near_speech != last.near_speech && noise.start_us < last.end_us {
                if noise.end_us - noise.start_us > last.end_us - last.start_us {
                    *last = noise;
                }
                continue;
            }
        }
        match out.last_mut() {
            Some(last) if joins => {
                last.end_us = last.end_us.max(noise.end_us);
                // The more telling kind wins: a click or knock over a mouth sound or breath.
                let rank = |k: NoiseKind| match k {
                    NoiseKind::Knock => 3,
                    NoiseKind::Click => 2,
                    NoiseKind::Mouth => 1,
                    NoiseKind::Breath => 0,
                };
                if rank(noise.kind) > rank(last.kind) {
                    last.kind = noise.kind;
                }
                last.contrast_db = last.contrast_db.max(noise.contrast_db);
                last.release_only &= noise.release_only;
                last.core_us = (
                    last.core_us.0.min(noise.core_us.0),
                    last.core_us.1.max(noise.core_us.1),
                );
            }
            _ => out.push(noise),
        }
    }
    // A word ends in one release: a run of clicks after it is a mouth noise.
    for noise in &mut out {
        if noise.release_only && noise.core_us.1 - noise.core_us.0 > RELEASE_RUN_US {
            noise.near_speech = false;
            noise.release_only = false;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 48_000;

    /// A deterministic white noise source, -0.5..0.5.
    fn white(state: &mut u64) -> f32 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (*state >> 40) as f32 / (1u64 << 24) as f32 - 0.5
    }

    /// `seconds` of faint hiss (about -60 dB).
    fn room(seconds: f64) -> Vec<f32> {
        let mut state = 7u64;
        (0..at(seconds))
            .map(|_| white(&mut state) * 0.002)
            .collect()
    }

    fn at(seconds: f64) -> usize {
        (seconds * RATE as f64) as usize
    }

    /// A click: a few milliseconds of bright, quickly decaying sound.
    fn click(signal: &mut [f32], seconds: f64, amplitude: f32) {
        let start = at(seconds);
        for i in 0..at(0.004) {
            let decay = (-(i as f32) / 40.0).exp();
            let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
            signal[start + i] += amplitude * decay * sign;
        }
    }

    /// Soft noise from `seconds` for `length` seconds.
    fn hiss(signal: &mut [f32], seconds: f64, length: f64, amplitude: f32, seed: u64) {
        let mut state = seed;
        for sample in &mut signal[at(seconds)..at(seconds + length)] {
            *sample += amplitude * white(&mut state);
        }
    }

    /// A vowel: 120 Hz with harmonics, `length` seconds from `seconds`.
    fn vowel(signal: &mut [f32], seconds: f64, length: f64) {
        let start = at(seconds);
        for i in 0..at(length) {
            let t = i as f32 / RATE as f32;
            let mut v = 0.0;
            for h in 1..12 {
                v += (std::f32::consts::TAU * 120.0 * h as f32 * t).sin() / h as f32;
            }
            signal[start + i] += 0.15 * v;
        }
    }

    fn us(seconds: f64) -> u64 {
        (seconds * 1e6) as u64
    }

    fn covering(noises: &[Noise], from: f64, to: f64) -> Option<&Noise> {
        noises
            .iter()
            .find(|n| n.start_us <= us(from) && us(to) <= n.end_us)
    }

    #[test]
    fn finds_clicks_in_quiet_with_room_around_them() {
        let mut signal = room(4.0);
        click(&mut signal, 0.5, 0.3);
        click(&mut signal, 1.2, 0.05);
        click(&mut signal, 3.2, 0.3);
        let noises = find_noises(&signal, RATE, &NoiseConfig::default());
        assert_eq!(noises.len(), 3, "{noises:?}");
        for (noise, t) in noises.iter().zip([0.5, 1.2, 3.2]) {
            assert_eq!(noise.kind, NoiseKind::Click, "{noise:?}");
            assert!(!noise.near_speech, "{noise:?}");
            // The click and its decay, with the 20 ms asked for either side.
            assert!(
                covering(&noises, t - 0.018, t + 0.024).is_some(),
                "{t}: {noises:?}"
            );
            assert!(noise.end_us - noise.start_us < 80_000, "{noise:?}");
        }
    }

    /// A smack is many small clicks and wet sound over tens of milliseconds: it is muted whole,
    /// not just its sharpest click.
    #[test]
    fn a_smack_is_one_noise_from_its_first_sound_to_its_last() {
        let mut signal = room(2.0);
        hiss(&mut signal, 1.0, 0.09, 0.01, 11);
        for (t, a) in [
            (1.0, 0.04),
            (1.022, 0.2),
            (1.041, 0.06),
            (1.063, 0.12),
            (1.085, 0.03),
        ] {
            click(&mut signal, t, a);
        }
        let noises = find_noises(&signal, RATE, &NoiseConfig::default());
        assert_eq!(noises.len(), 1, "{noises:?}");
        assert!(covering(&noises, 0.985, 1.105).is_some(), "{noises:?}");
        assert_eq!(noises[0].kind, NoiseKind::Click);
        // More room asked for, more muted.
        let wide = find_noises(
            &signal,
            RATE,
            &NoiseConfig {
                padding_ms: 60,
                ..Default::default()
            },
        );
        assert!(covering(&wide, 0.945, 1.145).is_some(), "{wide:?}");
    }

    #[test]
    fn noises_next_to_speech_are_marked_and_padding_stays_out_of_it() {
        let mut signal = room(4.0);
        vowel(&mut signal, 1.0, 0.4);
        // 50 ms after the word ends: may be a released "t".
        click(&mut signal, 1.45, 0.3);
        // Well after it.
        click(&mut signal, 2.5, 0.3);
        let config = NoiseConfig {
            padding_ms: 150,
            ..Default::default()
        };
        let noises = find_noises(&signal, RATE, &config);
        let near = covering(&noises, 1.45, 1.452).expect("the click after the word");
        assert!(near.near_speech);
        assert!(
            near.start_us >= us(1.4),
            "padding stops short of the word: {near:?}"
        );
        let clear = covering(&noises, 2.5, 2.502).expect("the click on its own");
        assert!(!clear.near_speech);
        assert!(
            noises
                .iter()
                .all(|n| n.end_us <= us(1.4) || n.start_us >= us(1.4)),
            "nothing inside the word: {noises:?}"
        );
    }

    #[test]
    fn sensitivity_decides_faint_clicks() {
        let mut signal = room(2.0);
        click(&mut signal, 1.0, 0.004);
        let strict = find_noises(
            &signal,
            RATE,
            &NoiseConfig {
                sensitivity: 0.0,
                ..Default::default()
            },
        );
        let keen = find_noises(
            &signal,
            RATE,
            &NoiseConfig {
                sensitivity: 1.0,
                ..Default::default()
            },
        );
        assert!(strict.is_empty(), "{strict:?}");
        assert_eq!(keen.len(), 1, "{keen:?}");
    }

    /// An "s" between words, about as loud as them, may be speech: listed, never chosen for
    /// you. A faint steady noise is not.
    #[test]
    fn a_steady_hiss_is_marked_as_maybe_speech() {
        let mut signal = room(4.0);
        hiss(&mut signal, 1.0, 0.14, 0.2, 9);
        vowel(&mut signal, 1.4, 0.4);
        click(&mut signal, 2.0, 0.3);
        hiss(&mut signal, 2.6, 0.14, 0.01, 13);
        let noises = find_noises(&signal, RATE, &NoiseConfig::default());
        assert!(
            covering(&noises, 1.0, 1.14).is_some_and(|n| n.near_speech),
            "{noises:?}"
        );
        assert!(
            covering(&noises, 2.0, 2.002).is_some_and(|n| !n.near_speech),
            "{noises:?}"
        );
        assert!(
            covering(&noises, 2.6, 2.74).is_some_and(|n| !n.near_speech),
            "{noises:?}"
        );
    }

    /// George's misses (Untitled 11 Oct 2026, 0:13.6 and 0:26.0): a wet mouth sound running
    /// straight on from the end of a word, with no gap, is still found and chosen; it stops
    /// short of the word. The "s" of an "st" start, cut off from its vowel by the closure, is
    /// only offered.
    #[test]
    fn a_noise_running_on_from_a_word_is_found_and_a_word_initial_s_is_not_muted() {
        let mut signal = room(4.0);
        vowel(&mut signal, 1.0, 0.4);
        hiss(&mut signal, 1.4, 0.45, 0.006, 21);
        for (t, a) in [(1.47, 0.05), (1.58, 0.08), (1.66, 0.04), (1.78, 0.06)] {
            click(&mut signal, t, a);
        }
        // "st...": an "s", a 50 ms closure, then the vowel.
        hiss(&mut signal, 2.6, 0.1, 0.2, 23);
        vowel(&mut signal, 2.75, 0.4);
        for sensitivity in [0.0, 0.5, 1.0] {
            let config = NoiseConfig {
                sensitivity,
                ..Default::default()
            };
            let noises = find_noises(&signal, RATE, &config);
            let wet = covering(&noises, 1.47, 1.79)
                .unwrap_or_else(|| panic!("{sensitivity}: {noises:?}"));
            assert!(!wet.near_speech, "{sensitivity}: {wet:?}");
            assert!(wet.start_us >= us(1.39), "not into the word: {wet:?}");
            assert!(
                noises
                    .iter()
                    .filter(|n| n.start_us < us(2.75) && n.end_us > us(2.6))
                    .all(|n| n.near_speech),
                "{sensitivity}: {noises:?}"
            );
        }
    }

    /// An "s" running into a vowel is part of the word.
    #[test]
    fn consonants_joined_to_a_vowel_are_speech() {
        let mut signal = room(3.0);
        hiss(&mut signal, 0.85, 0.15, 0.2, 3);
        vowel(&mut signal, 1.0, 0.6);
        let noises = find_noises(
            &signal,
            RATE,
            &NoiseConfig {
                sensitivity: 1.0,
                ..Default::default()
            },
        );
        assert!(noises.is_empty(), "{noises:?}");
    }

    #[test]
    fn finds_knocks_and_breaths() {
        let mut signal = room(3.0);
        // A thump: 60 Hz, decaying over ~40 ms.
        let start = at(1.0);
        for i in 0..at(0.06) {
            let t = i as f32 / RATE as f32;
            signal[start + i] +=
                0.4 * (-t / 0.015).exp() * (std::f32::consts::TAU * 60.0 * t).sin();
        }
        // A breath: 400 ms of soft noise swelling in and out.
        let mut state = 5u64;
        let (start, len) = (at(2.0), at(0.4));
        for i in 0..len {
            let swell = (std::f32::consts::PI * i as f32 / len as f32).sin();
            signal[start + i] += 0.03 * swell * white(&mut state);
        }
        let noises = find_noises(&signal, RATE, &NoiseConfig::default());
        assert_eq!(noises.len(), 2, "{noises:?}");
        assert_eq!(noises[0].kind, NoiseKind::Knock);
        assert!(covering(&noises, 1.0, 1.04).is_some());
        assert_eq!(noises[1].kind, NoiseKind::Breath);
        assert!(covering(&noises, 1.99, 2.41).is_some(), "{noises:?}");
    }
}

/// `AERO_NOISE_WAV=<file.wav> cargo test --release --lib noise_probe -- --ignored --nocapture`:
/// what the detector finds in a real recording, and what is left near each noise once the
/// chosen ones are muted (found again within 30 ms of a muted moment). With
/// `AERO_NOISE_OUT=<file.wav>` it writes the muted sound.
#[cfg(test)]
#[test]
#[ignore]
fn noise_probe() {
    let path = std::env::var("AERO_NOISE_WAV").expect("AERO_NOISE_WAV");
    let mut reader = crate::project::pcm::PcmReader::open(std::path::Path::new(&path)).unwrap();
    let info = reader.info().clone();
    let channels = info.channels as usize;
    let mut buf = vec![0f32; info.frame_count as usize * channels];
    let got = reader
        .read_frames(&mut buf, info.frame_count as usize)
        .unwrap();
    let mut mono: Vec<f32> = buf[..got * channels]
        .chunks_exact(channels)
        .map(|f| f.iter().sum::<f32>() / channels as f32)
        .collect();
    let rate = info.sample_rate;
    let config = NoiseConfig {
        sensitivity: std::env::var("AERO_NOISE_SENS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.5),
        ..Default::default()
    };
    let started = std::time::Instant::now();
    let noises = find_noises(&mono, rate, &config);
    let took = started.elapsed();
    let chosen: Vec<&Noise> = noises.iter().filter(|n| !n.near_speech).collect();
    let verbose = std::env::var("AERO_NOISE_LIST").is_ok();
    for n in noises.iter().filter(|_| verbose) {
        println!(
            "{:8.3}-{:8.3} {:>6} {:5.1} dB{}",
            n.start_us as f64 / 1e6,
            n.end_us as f64 / 1e6,
            format!("{:?}", n.kind),
            n.contrast_db,
            if n.near_speech { "  near speech" } else { "" }
        );
    }
    let sample = |us: u64| (us as u128 * rate as u128 / 1_000_000) as usize;
    for n in &chosen {
        let (a, b) = (sample(n.start_us), sample(n.end_us).min(mono.len()));
        mono[a..b].fill(0.0);
    }
    let again = find_noises(&mono, rate, &config);
    let left: Vec<&Noise> = again
        .iter()
        .filter(|n| {
            chosen
                .iter()
                .any(|m| n.start_us < m.end_us + 30_000 && m.start_us < n.end_us + 30_000)
        })
        .collect();
    let count = |kind: NoiseKind| noises.iter().filter(|n| n.kind == kind).count();
    let muted_ms: u64 = chosen.iter().map(|n| (n.end_us - n.start_us) / 1000).sum();
    println!(
        "{} found ({} click, {} knock, {} mouth, {} breath; {} near speech) in {:.0}s, {:?}; {} chosen ({} ms muted); left next to them: {}",
        noises.len(),
        count(NoiseKind::Click),
        count(NoiseKind::Knock),
        count(NoiseKind::Mouth),
        count(NoiseKind::Breath),
        noises.iter().filter(|n| n.near_speech).count(),
        got as f64 / rate as f64,
        took,
        chosen.len(),
        muted_ms,
        left.len()
    );
    if let Ok(out) = std::env::var("AERO_NOISE_OUT") {
        let pcm: Vec<i16> = mono
            .iter()
            .map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
            .collect();
        std::fs::write(out, crate::fixtures::generate_pcm16_wav(rate, 1, &pcm)).unwrap();
    }
}
