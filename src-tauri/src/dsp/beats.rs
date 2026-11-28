//! Beats and bars of a song (SND-8), so music beds loop and end on a bar instead of mid-phrase.
//! No model: an onset envelope from spectral flux in pitch-spaced bands, the tempo from its
//! autocorrelation (leaning to common tempos), beats by dynamic programming, and the bar's
//! first beat where the bass hits hardest. Four beats to a bar. The idea is Editroom's (MIT);
//! checked against librosa on six library songs (see the PR for SND-8).
use super::fft::{Complex, Fft};

/// The rate songs are read at for this.
pub const RATE: u32 = 22_050;
const WINDOW: usize = 1024;
const HOP: usize = 512;
const BEATS_PER_BAR: usize = 4;
/// Tempos looked for, in beats per minute.
const MIN_BPM: f64 = 60.0;
const MAX_BPM: f64 = 190.0;
/// Tempos near this win a close call (a song at 70 could be counted at 140).
const LIKELY_BPM: f64 = 120.0;
/// Frames in each stretch the tempo is measured over (about nine seconds).
const TEMPO_WINDOW: usize = 384;
/// Below this share of the envelope's energy at the beat's lag, the song has no steady beat.
const MIN_PERIODICITY: f64 = 0.15;
/// How firmly the beats keep the tempo against stray onsets (librosa's default).
const TIGHTNESS: f64 = 100.0;
/// Bass: up to this frequency.
const BASS_HZ: f64 = 150.0;
/// Band energy counted as silence (about 80 dB under a full-scale tone), so the noise of
/// near-empty bands doesn't read as onsets.
const BAND_FLOOR: f32 = 1e-3;
/// Below this average rise (dB per band per frame) nothing is struck: a held sound, not a beat.
/// The library's calmest songs average about 0.3.
const MIN_ONSETS_DB: f32 = 0.1;
/// Bands the onset envelope is measured in.
const BANDS: usize = 40;
/// Frames quieter than this under the song's loudest are its silent ends.
const QUIET_DB: f32 = -35.0;
/// At least this many bars before a grid is trusted.
const MIN_BARS: usize = 4;

/// Where a song's beats and bars are, in microseconds of the file.
#[derive(Clone, Debug, PartialEq)]
pub struct Beats {
    pub bpm: f64,
    pub beats_us: Vec<u64>,
    /// The first beat of each bar.
    pub bars_us: Vec<u64>,
    /// How much of the file was looked at: no bars are known past it.
    pub until_us: u64,
}

impl Beats {
    /// The last bar start at or before `us`.
    pub fn bar_at_or_before(&self, us: u64) -> Option<u64> {
        self.bars_us.iter().rev().copied().find(|&b| b <= us)
    }

    /// The first bar start at or after `us`.
    pub fn bar_at_or_after(&self, us: u64) -> Option<u64> {
        self.bars_us.iter().copied().find(|&b| b >= us)
    }
}

/// The beats of mono `samples` at [`RATE`]; none when it has no steady beat.
pub fn find(samples: &[f32]) -> Option<Beats> {
    if samples.len() < WINDOW * 4 {
        return None;
    }
    let (flux, bass, level) = spectra(samples);
    // Only the part with sound: a quiet intro or tail has no beat to count.
    let loudest = level.iter().cloned().fold(f32::MIN, f32::max);
    let first = level.iter().position(|&l| l > loudest + QUIET_DB)?;
    let last = level.iter().rposition(|&l| l > loudest + QUIET_DB)?;
    let envelope = onsets(&flux);
    let rising = envelope[first..=last].iter().sum::<f32>() / (last - first + 1) as f32;
    if rising < MIN_ONSETS_DB {
        return None;
    }
    let frame_s = HOP as f64 / RATE as f64;
    let period = tempo(&envelope[first..=last], frame_s)?;
    let beats = track(&envelope, first, last, period);
    if beats.len() < BEATS_PER_BAR * MIN_BARS {
        return None;
    }
    // The bar starts on the beat whose every fourth hit carries the most bass.
    let downbeat = (0..BEATS_PER_BAR)
        .max_by(|&a, &b| {
            let weight = |phase: usize| -> f32 {
                let hits: Vec<f32> = beats
                    .iter()
                    .skip(phase)
                    .step_by(BEATS_PER_BAR)
                    .map(|&f| peak(&bass, f, 2))
                    .collect();
                hits.iter().sum::<f32>() / hits.len().max(1) as f32
            };
            weight(a).total_cmp(&weight(b))
        })
        .unwrap_or(0);
    // A frame's rise is in the part of its window the one before didn't cover: its last half.
    let to_us = |f: usize| ((f * HOP + WINDOW * 3 / 4) as f64 / RATE as f64 * 1e6).round() as u64;
    let beats_us: Vec<u64> = beats.iter().map(|&f| to_us(f)).collect();
    let bars_us = beats_us
        .iter()
        .skip(downbeat)
        .step_by(BEATS_PER_BAR)
        .copied()
        .collect();
    Some(Beats {
        bpm: 60.0 / (period * frame_s),
        beats_us,
        bars_us,
        until_us: (samples.len() as f64 / RATE as f64 * 1e6) as u64,
    })
}

/// Per frame: spectral flux (log band energies rising, over bands spaced like pitch so hi-hats
/// don't outvote the rest), bass energy and level in dB.
fn spectra(samples: &[f32]) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let fft = Fft::new(WINDOW);
    let hann: Vec<f32> = (0..WINDOW)
        .map(|i| (0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / WINDOW as f64).cos()) as f32)
        .collect();
    let bin_hz = RATE as f64 / WINDOW as f64;
    let bass_bins = ((BASS_HZ / bin_hz).ceil() as usize).max(2);
    // Band edges from 40 Hz to 8 kHz, evenly spaced in pitch, each at least one bin wide.
    let mut edges: Vec<usize> = (0..=BANDS)
        .map(|i| {
            (40.0 * (8000.0f64 / 40.0).powf(i as f64 / BANDS as f64) / bin_hz).round() as usize
        })
        .collect();
    for i in 1..edges.len() {
        edges[i] = edges[i].max(edges[i - 1] + 1);
    }
    let mut previous = [0f32; BANDS];
    let mut buffer = vec![Complex::default(); WINDOW];
    let (mut flux, mut bass, mut level) = (Vec::new(), Vec::new(), Vec::new());
    let mut at = 0;
    while at + WINDOW <= samples.len() {
        let mut power = 0f32;
        for (i, slot) in buffer.iter_mut().enumerate() {
            let s = samples[at + i];
            power += s * s;
            *slot = Complex {
                re: s * hann[i],
                im: 0.0,
            };
        }
        fft.process(&mut buffer, false);
        let mut rise = 0f32;
        for (band, slot) in previous.iter_mut().enumerate() {
            let energy: f32 = buffer[edges[band]..edges[band + 1]]
                .iter()
                .map(|c| c.norm_sqr())
                .sum::<f32>()
                / (edges[band + 1] - edges[band]) as f32;
            let db = 10.0 * (energy + BAND_FLOOR).log10();
            rise += (db - *slot).max(0.0);
            *slot = db;
        }
        let bass_energy: f32 = buffer[1..bass_bins]
            .iter()
            .map(|c| c.norm_sqr().sqrt())
            .sum();
        flux.push(rise / BANDS as f32);
        bass.push(bass_energy);
        level.push(10.0 * (power / WINDOW as f32 + 1e-12).log10());
        at += HOP;
    }
    (flux, bass, level)
}

/// The flux above its local average (about half a second each side), never below zero.
fn onsets(flux: &[f32]) -> Vec<f32> {
    let radius = (0.5 * RATE as f64 / HOP as f64) as usize;
    let mut sums = vec![0f64; flux.len() + 1];
    for (i, &f) in flux.iter().enumerate() {
        sums[i + 1] = sums[i] + f as f64;
    }
    (0..flux.len())
        .map(|i| {
            let (a, b) = (i.saturating_sub(radius), (i + radius + 1).min(flux.len()));
            let mean = (sums[b] - sums[a]) / (b - a) as f64;
            (flux[i] as f64 - mean).max(0.0) as f32
        })
        .collect()
}

/// The beat's length in frames (fractional): the envelope's autocorrelation in windows of
/// about nine seconds, each scaled by its own energy and averaged (so every part of the song
/// counts alike), leaning toward common tempos.
fn tempo(envelope: &[f32], frame_s: f64) -> Option<f64> {
    let shortest = (60.0 / MAX_BPM / frame_s).floor() as usize;
    let longest = (60.0 / MIN_BPM / frame_s).ceil() as usize;
    if envelope.len() < longest * 8 {
        return None;
    }
    let window = TEMPO_WINDOW.min(envelope.len());
    let mut r = vec![0f64; longest + 2];
    let mut windows = 0;
    let mut start = 0;
    while start + window <= envelope.len() {
        let part = &envelope[start..start + window];
        let mean = part.iter().map(|&e| e as f64).sum::<f64>() / window as f64;
        let part: Vec<f64> = part.iter().map(|&e| e as f64 - mean).collect();
        let energy: f64 = part.iter().map(|e| e * e).sum();
        if energy > 1e-6 * window as f64 {
            for (lag, slot) in r.iter_mut().enumerate() {
                let sum: f64 = (0..window - lag).map(|i| part[i] * part[i + lag]).sum();
                *slot += sum / energy;
            }
            windows += 1;
        }
        start += window / 4;
    }
    if windows == 0 {
        return None;
    }
    r.iter_mut().for_each(|v| *v /= windows as f64);
    let weighted = |lag: usize| -> f64 {
        let bpm = 60.0 / (lag as f64 * frame_s);
        let octaves = (bpm / LIKELY_BPM).log2();
        r[lag] * (-0.5 * octaves.powi(2)).exp()
    };
    let best = (shortest..=longest).max_by(|&a, &b| weighted(a).total_cmp(&weighted(b)))?;
    if r[best] < MIN_PERIODICITY {
        return None;
    }
    // Between frames: the peak of a parabola through the best lag and its neighbours.
    let (a, b, c) = (r[best - 1], r[best], r[best + 1]);
    let bend = a - 2.0 * b + c;
    let shift = if bend < 0.0 {
        (0.5 * (a - c) / bend).clamp(-0.5, 0.5)
    } else {
        0.0
    };
    Some(best as f64 + shift)
}

/// Beat frames from `first` to `last` by dynamic programming (Ellis, 2007): each frame's best
/// score is its onset strength plus the best earlier beat's score, less a penalty for a gap
/// unlike `period`; the beats are traced back from the best frame in the last beat.
fn track(envelope: &[f32], first: usize, last: usize, period: f64) -> Vec<usize> {
    let part = &envelope[first..=last];
    let mean = part.iter().map(|&e| e as f64).sum::<f64>() / part.len() as f64;
    let spread = (part.iter().map(|&e| (e as f64 - mean).powi(2)).sum::<f64>() / part.len() as f64)
        .sqrt()
        .max(1e-9);
    let strength: Vec<f64> = part.iter().map(|&e| e as f64 / spread).collect();
    let (shortest, longest) = (
        (period / 2.0).round().max(1.0) as usize,
        (period * 2.0).round() as usize,
    );
    let mut score = vec![0f64; part.len()];
    let mut from: Vec<Option<usize>> = vec![None; part.len()];
    for t in 0..part.len() {
        let mut best: Option<(f64, usize)> = None;
        for gap in shortest..=longest.min(t) {
            let earlier = t - gap;
            let penalty = (gap as f64 / period).ln().powi(2);
            let s = score[earlier] - TIGHTNESS * penalty;
            if best.is_none_or(|(b, _)| s > b) {
                best = Some((s, earlier));
            }
        }
        match best {
            Some((s, earlier)) if s > 0.0 => {
                score[t] = strength[t] + s;
                from[t] = Some(earlier);
            }
            _ => score[t] = strength[t],
        }
    }
    // The end: the best-scoring frame within the last beat.
    let tail = part.len().saturating_sub(period.round() as usize);
    let Some(mut at) = (tail..part.len()).max_by(|&a, &b| score[a].total_cmp(&score[b])) else {
        return Vec::new();
    };
    let mut beats = vec![at + first];
    while let Some(earlier) = from[at] {
        beats.push(earlier + first);
        at = earlier;
    }
    beats.reverse();
    beats
}

/// The largest value within `radius` frames of `at`.
fn peak(values: &[f32], at: usize, radius: usize) -> f32 {
    let (lo, hi) = (
        at.saturating_sub(radius),
        (at + radius + 1).min(values.len()),
    );
    values[lo.min(hi)..hi].iter().cloned().fold(0.0, f32::max)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `seconds` of a 120 BPM groove: a kick on each bar's first beat (from `kick_beat`), a
    /// snare-like noise burst on the other beats, and a quiet pad under it all.
    fn groove(bpm: f64, seconds: f64, kick_beat: usize) -> Vec<f32> {
        let n = (seconds * RATE as f64) as usize;
        let beat = 60.0 / bpm;
        let mut out: Vec<f32> = (0..n)
            .map(|i| 0.02 * (std::f64::consts::TAU * 220.0 * i as f64 / RATE as f64).sin() as f32)
            .collect();
        let mut seed = 7u32;
        let mut noise = || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (seed >> 8) as f32 / (1u32 << 24) as f32 * 2.0 - 1.0
        };
        let mut k = 0;
        while (k as f64 * beat) < seconds {
            let start = (k as f64 * beat * RATE as f64) as usize;
            let kick = k % BEATS_PER_BAR == kick_beat;
            for j in 0..(0.12 * RATE as f64) as usize {
                let Some(slot) = out.get_mut(start + j) else {
                    break;
                };
                let t = j as f64 / RATE as f64;
                let decay = (-t * 30.0).exp() as f32;
                *slot += if kick {
                    0.9 * decay * (std::f64::consts::TAU * 55.0 * t).sin() as f32
                } else {
                    0.4 * decay * noise()
                };
            }
            k += 1;
        }
        out
    }

    #[test]
    fn a_steady_groove_gives_its_tempo_and_its_bars() {
        for (bpm, kick_beat) in [(120.0, 0), (96.0, 2)] {
            let beats = find(&groove(bpm, 30.0, kick_beat)).expect("a beat");
            assert!((beats.bpm - bpm).abs() < 1.5, "{} for {bpm}", beats.bpm);
            let beat_us = 60e6 / bpm;
            // Every bar starts on a kick: a multiple of four beats from the first kick.
            for &bar in &beats.bars_us {
                let beats_in = (bar as f64 / beat_us - kick_beat as f64) / 4.0;
                assert!(
                    (beats_in - beats_in.round()).abs() * 4.0 * beat_us < 40_000.0,
                    "bar at {bar} for {bpm}"
                );
            }
            assert!(beats.bars_us.len() >= 10);
            assert_eq!(beats.bar_at_or_after(1), beats.bars_us.first().copied());
        }
    }

    /// A real song: `BEATS_PROBE=<file> cargo test --lib beats_probe -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn beats_probe() {
        let path = std::env::var("BEATS_PROBE").expect("BEATS_PROBE");
        let samples =
            crate::media::ffmpeg::decode_mono(std::path::Path::new(&path), RATE, 600).unwrap();
        let started = std::time::Instant::now();
        match find(&samples) {
            Some(b) => println!(
                "{path}: {:.1} BPM, {} beats, {} bars, first bar {:.2} s, last {:.2} s ({} ms)",
                b.bpm,
                b.beats_us.len(),
                b.bars_us.len(),
                b.bars_us[0] as f64 / 1e6,
                *b.bars_us.last().unwrap() as f64 / 1e6,
                started.elapsed().as_millis()
            ),
            None => println!("{path}: no steady beat"),
        }
        if let (Ok(out), Some(b)) = (std::env::var("BEATS_OUT"), find(&samples)) {
            let line = |v: &[u64]| {
                v.iter()
                    .map(|us| format!("{:.3}", *us as f64 / 1e6))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            std::fs::write(
                out,
                format!("{}\n{}\n", line(&b.beats_us), line(&b.bars_us)),
            )
            .unwrap();
        }
    }

    #[test]
    fn a_held_tone_and_silence_have_no_beat() {
        let tone: Vec<f32> = (0..RATE as usize * 20)
            .map(|i| 0.3 * (std::f64::consts::TAU * 330.0 * i as f64 / RATE as f64).sin() as f32)
            .collect();
        assert_eq!(find(&tone), None);
        assert_eq!(find(&vec![0.0; RATE as usize * 20]), None);
    }
}
