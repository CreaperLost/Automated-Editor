//! Matching speech across takes (AUTO-2): a re-recorded sentence shouldn't jump in loudness or
//! tone. Each take (sound file) of speech is measured once: its loudness while speaking and the
//! shape of its spectrum in octave bands. The edit's takes are then brought to their common
//! level, and a take whose tone differs gets a gentle EQ towards the others (a few dB at most).
//!
//! The EQ runs as biquads from a short warm-up before each read, so playback and export give
//! the same samples however they chunk their reads (to well under a thousandth of full scale).
use super::fft::{Complex, Fft};

/// The octave bands measured, and where the EQ's shelves and peaks sit for each.
pub const BAND_CENTERS: [f32; 7] = [120.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0];
pub const BANDS: usize = BAND_CENTERS.len();
/// A take within this of the level is left as it is (dB).
const LEVEL_SLACK_DB: f32 = 1.0;
/// Level matching moves a take this far at most (dB).
const MAX_LEVEL_DB: f32 = 9.0;
/// A take is EQed only when some band is this far from the others (dB)...
const TONE_SLACK_DB: f32 = 2.0;
/// ...by this share of the difference, at most this far per band: gentle.
const TONE_SHARE: f32 = 0.6;
const MAX_TONE_DB: f32 = 4.0;
/// Biquads settle within this; reads start this early.
const WARMUP_US: u64 = 100_000;
/// FFT windows measured per take, at most.
const MAX_WINDOWS: usize = 300;

/// What a take's speech sounds like.
#[derive(Clone, Debug, PartialEq)]
pub struct SpeechProfile {
    /// Integrated loudness of its speech (LUFS).
    pub lufs: f64,
    /// Power per band (dB) less their mean: the spectrum's shape only.
    pub bands: [f32; BANDS],
}

/// Collects a take's speech spectrum from mono windows of speech.
pub struct BandMeter {
    fft: Fft,
    rate: u32,
    window: Vec<f32>,
    power: [f64; BANDS],
    windows: usize,
}

impl BandMeter {
    pub fn new(rate: u32) -> Self {
        let n = if rate > 32_000 { 4096 } else { 2048 };
        let window = (0..n)
            .map(|i| {
                let x = std::f32::consts::TAU * i as f32 / n as f32;
                0.5 - 0.5 * x.cos()
            })
            .collect();
        Self {
            fft: Fft::new(n),
            rate,
            window,
            power: [0.0; BANDS],
            windows: 0,
        }
    }

    pub fn size(&self) -> usize {
        self.fft.len()
    }

    pub fn max_windows() -> usize {
        MAX_WINDOWS
    }

    /// Adds one window of speech (`size()` mono samples).
    pub fn add(&mut self, mono: &[f32]) {
        let n = self.fft.len();
        let mut data: Vec<Complex> = mono
            .iter()
            .zip(&self.window)
            .map(|(s, w)| Complex { re: s * w, im: 0.0 })
            .collect();
        data.resize(n, Complex::default());
        self.fft.process(&mut data, false);
        let hz = self.rate as f32 / n as f32;
        for (bin, value) in data.iter().enumerate().take(n / 2).skip(1) {
            if let Some(band) = band_of(bin as f32 * hz) {
                self.power[band] += value.norm_sqr() as f64;
            }
        }
        self.windows += 1;
    }

    /// The bands' shape, once enough speech was heard.
    pub fn finish(&self) -> Option<[f32; BANDS]> {
        if self.windows < 8 {
            return None;
        }
        let mut db = [0f32; BANDS];
        for (out, &p) in db.iter_mut().zip(&self.power) {
            *out = 10.0 * (p / self.windows as f64).max(1e-20).log10() as f32;
        }
        // Bands past half the sample rate hold nothing: leave them out of the shape.
        let live: Vec<usize> = (0..BANDS)
            .filter(|&b| BAND_CENTERS[b] * 1.41 < self.rate as f32 / 2.0)
            .collect();
        let mean = live.iter().map(|&b| db[b]).sum::<f32>() / live.len().max(1) as f32;
        for (b, value) in db.iter_mut().enumerate() {
            *value = if live.contains(&b) {
                *value - mean
            } else {
                0.0
            };
        }
        Some(db)
    }
}

/// The band holding `hz`: octaves around each centre, the first and last open-ended down to
/// 70 Hz and up to 12 kHz.
fn band_of(hz: f32) -> Option<usize> {
    if !(70.0..12_000.0).contains(&hz) {
        return None;
    }
    let edge = |i: usize| BAND_CENTERS[i] * std::f32::consts::SQRT_2;
    (0..BANDS).find(|&i| i == BANDS - 1 || hz < edge(i))
}

/// How one take is changed to sit with the others.
#[derive(Clone, Debug, PartialEq)]
pub struct TakeMatch {
    pub gain_db: f32,
    /// Per band; all zero when its tone is left alone.
    pub eq_db: [f32; BANDS],
}

impl TakeMatch {
    pub fn is_noop(&self) -> bool {
        self.gain_db == 0.0 && self.eq_db.iter().all(|g| *g == 0.0)
    }

    pub fn has_eq(&self) -> bool {
        self.eq_db.iter().any(|g| *g != 0.0)
    }
}

/// How each take is matched to the rest. `takes` are each take's profile and how long the edit
/// plays it; the reference is their duration-weighted average. Fewer than two takes with
/// speech: nothing to match.
pub fn match_takes(takes: &[(SpeechProfile, f64)]) -> Vec<TakeMatch> {
    let total: f64 = takes.iter().map(|(_, used)| used).sum();
    if takes.len() < 2 || total <= 0.0 {
        return takes
            .iter()
            .map(|_| TakeMatch {
                gain_db: 0.0,
                eq_db: [0.0; BANDS],
            })
            .collect();
    }
    let level = takes.iter().map(|(p, used)| p.lufs * used).sum::<f64>() / total;
    let mut shape = [0f32; BANDS];
    for (p, used) in takes {
        for (s, b) in shape.iter_mut().zip(&p.bands) {
            *s += b * (*used / total) as f32;
        }
    }
    takes
        .iter()
        .map(|(p, _)| {
            let diff = (level - p.lufs) as f32;
            let gain_db = if diff.abs() < LEVEL_SLACK_DB {
                0.0
            } else {
                diff.clamp(-MAX_LEVEL_DB, MAX_LEVEL_DB)
            };
            let mut eq_db = [0f32; BANDS];
            let off: Vec<f32> = shape.iter().zip(&p.bands).map(|(r, b)| r - b).collect();
            if off.iter().any(|d| d.abs() >= TONE_SLACK_DB) {
                for (g, d) in eq_db.iter_mut().zip(&off) {
                    let g_db = (d * TONE_SHARE).clamp(-MAX_TONE_DB, MAX_TONE_DB);
                    *g = if g_db.abs() < 0.5 { 0.0 } else { g_db };
                }
                // The EQ changes the shape, not the level: centre it.
                let mean = eq_db.iter().sum::<f32>() / BANDS as f32;
                for g in &mut eq_db {
                    *g -= mean;
                }
            }
            TakeMatch { gain_db, eq_db }
        })
        .collect()
}

#[derive(Clone, Copy, Debug)]
struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
}

impl Biquad {
    /// RBJ cookbook filters: a low shelf, a peak, or a high shelf at `hz`.
    fn new(kind: usize, hz: f32, gain_db: f32, rate: u32) -> Self {
        let a = 10f64.powf(gain_db as f64 / 40.0);
        let w = std::f64::consts::TAU * (hz as f64 / rate as f64).min(0.49);
        let (cos, sin) = (w.cos(), w.sin());
        let (b, den) = if kind == 0 || kind == BANDS - 1 {
            // Shelves with slope 1.
            let alpha = sin / 2.0 * std::f64::consts::SQRT_2;
            let k = 2.0 * a.sqrt() * alpha;
            let low = kind == 0;
            let s = if low { 1.0 } else { -1.0 };
            let b0 = a * ((a + 1.0) - s * (a - 1.0) * cos + k);
            let b1 = s * 2.0 * a * ((a - 1.0) - s * (a + 1.0) * cos);
            let b2 = a * ((a + 1.0) - s * (a - 1.0) * cos - k);
            let a0 = (a + 1.0) + s * (a - 1.0) * cos + k;
            let a1 = -s * 2.0 * ((a - 1.0) + s * (a + 1.0) * cos);
            let a2 = (a + 1.0) + s * (a - 1.0) * cos - k;
            ([b0, b1, b2], [a0, a1, a2])
        } else {
            // A peak an octave wide.
            let alpha = sin * (std::f64::consts::LN_2 / 2.0 * w / sin).sinh();
            (
                [1.0 + alpha * a, -2.0 * cos, 1.0 - alpha * a],
                [1.0 + alpha / a, -2.0 * cos, 1.0 - alpha / a],
            )
        };
        Self {
            b: [b[0] / den[0], b[1] / den[0], b[2] / den[0]],
            a: [den[1] / den[0], den[2] / den[0]],
        }
    }
}

/// A take's match as a filter over its samples.
pub struct TakeFilter {
    gain: f32,
    filters: Vec<Biquad>,
    warmup: u64,
}

impl TakeFilter {
    pub fn new(take: &TakeMatch, rate: u32) -> Self {
        let filters = take
            .eq_db
            .iter()
            .enumerate()
            .filter(|(_, g)| **g != 0.0)
            .map(|(i, g)| Biquad::new(i, BAND_CENTERS[i], *g, rate))
            .collect();
        Self {
            gain: 10f32.powf(take.gain_db / 20.0),
            filters,
            warmup: rate as u64 * WARMUP_US / 1_000_000,
        }
    }

    /// The input frames needed for output frames `[start, end)`: a warm-up before them.
    pub fn input_range(&self, start: u64, end: u64) -> (u64, u64) {
        if self.filters.is_empty() {
            return (start, end);
        }
        (start.saturating_sub(self.warmup), end)
    }

    /// Filters `input` (interleaved, starting at frame `from`) and answers the frames from
    /// `start` on.
    pub fn process(&self, input: &[f32], channels: usize, from: u64, start: u64) -> Vec<f32> {
        let skip = (start.saturating_sub(from) as usize * channels).min(input.len());
        if self.filters.is_empty() {
            return input[skip..].iter().map(|s| s * self.gain).collect();
        }
        let frames = input.len() / channels.max(1);
        let mut out = vec![0f32; input.len()];
        for ch in 0..channels {
            let mut signal: Vec<f64> = (0..frames)
                .map(|f| input[f * channels + ch] as f64)
                .collect();
            for filter in &self.filters {
                let (mut x1, mut x2, mut y1, mut y2) = (0.0, 0.0, 0.0, 0.0);
                for s in &mut signal {
                    let x = *s;
                    let y = filter.b[0] * x + filter.b[1] * x1 + filter.b[2] * x2
                        - filter.a[0] * y1
                        - filter.a[1] * y2;
                    (x2, x1, y2, y1) = (x1, x, y1, y);
                    *s = y;
                }
            }
            for (f, s) in signal.iter().enumerate() {
                out[f * channels + ch] = *s as f32 * self.gain;
            }
        }
        out.split_off(skip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(lufs: f64, bands: [f32; BANDS]) -> SpeechProfile {
        SpeechProfile { lufs, bands }
    }

    #[test]
    fn a_quieter_duller_take_is_brought_up_and_brightened_gently() {
        let flat = [0.0; BANDS];
        let dull = [3.0, 3.0, 1.5, 0.0, -1.5, -3.0, -3.0];
        let matched = match_takes(&[(profile(-20.0, flat), 9.0), (profile(-26.0, dull), 1.0)]);
        assert_eq!(matched[0].gain_db, 0.0, "within the slack of the reference");
        assert!((matched[1].gain_db - 5.4).abs() < 0.01, "{:?}", matched[1]);
        assert!(matched[1].eq_db[6] > 1.0 && matched[1].eq_db[0] < -1.0);
        assert!(matched[1]
            .eq_db
            .iter()
            .all(|g| g.abs() <= MAX_TONE_DB + 0.5));
        assert!(!matched[0].has_eq());
    }

    #[test]
    fn one_take_or_alike_takes_are_left_alone() {
        let flat = [0.0; BANDS];
        assert!(match_takes(&[(profile(-20.0, flat), 1.0)])[0].is_noop());
        let both = match_takes(&[(profile(-20.0, flat), 1.0), (profile(-20.6, flat), 1.0)]);
        assert!(both.iter().all(TakeMatch::is_noop));
    }

    #[test]
    fn filtering_in_chunks_gives_the_same_samples() {
        let take = TakeMatch {
            gain_db: 3.0,
            eq_db: [-2.0, 0.0, 1.0, 0.0, 2.0, 3.0, 2.5],
        };
        let rate = 48_000;
        let filter = TakeFilter::new(&take, rate);
        let signal: Vec<f32> = (0..96_000)
            .map(|i| {
                let t = i as f32 / rate as f32;
                0.3 * (t * 440.0 * std::f32::consts::TAU).sin()
                    + 0.2 * (t * 3100.0 * std::f32::consts::TAU).sin()
                    + 0.1 * ((i * 7919 % 1000) as f32 / 1000.0 - 0.5)
            })
            .collect();
        let whole = |a: u64, b: u64| {
            let (from, to) = filter.input_range(a, b);
            filter.process(&signal[from as usize..to as usize], 1, from, a)
        };
        let all = whole(10_000, 90_000);
        let mut pieces = whole(10_000, 50_000);
        pieces.extend(whole(50_000, 90_000));
        assert_eq!(all.len(), pieces.len());
        let worst = all
            .iter()
            .zip(&pieces)
            .map(|(a, b)| (a - b).abs())
            .fold(0f32, f32::max);
        assert!(worst < 1e-3, "{worst}");
    }

    #[test]
    fn the_meter_finds_where_a_tone_sits() {
        let rate = 48_000;
        let mut meter = BandMeter::new(rate);
        let n = meter.size();
        for w in 0..10 {
            let mono: Vec<f32> = (0..n)
                .map(|i| {
                    let t = (w * n + i) as f32 / rate as f32;
                    (t * 1000.0 * std::f32::consts::TAU).sin()
                })
                .collect();
            meter.add(&mono);
        }
        let bands = meter.finish().unwrap();
        let loudest = (0..BANDS)
            .max_by(|&a, &b| bands[a].total_cmp(&bands[b]))
            .unwrap();
        assert_eq!(loudest, 3, "{bands:?}");
    }

    #[test]
    fn a_flat_eq_with_gain_only_scales() {
        let take = TakeMatch {
            gain_db: -6.0,
            eq_db: [0.0; BANDS],
        };
        let filter = TakeFilter::new(&take, 48_000);
        let out = filter.process(&[1.0, 0.5, -1.0, 0.25], 2, 0, 1);
        assert_eq!(out.len(), 2);
        assert!((out[0] + 0.501).abs() < 0.01);
    }
}
