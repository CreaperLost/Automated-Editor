//! Spectral noise reduction against a stationary noise profile.
//!
//! The STFT grid is anchored at frame 0 of the source file (frame k covers
//! `[k * hop, k * hop + n)`), so any requested range is processed identically no matter
//! how playback or export chunks it. Square-root Hann windows at 50% overlap
//! reconstruct the input exactly when every gain is 1.
use super::fft::{Complex, Fft};

/// Over-subtraction factor; above 1 removes noise bins that fluctuate above the mean.
const OVER_SUBTRACTION: f32 = 2.0;

/// About 20 ms windows: long enough to resolve speech harmonics, short enough to keep
/// transients crisp.
pub fn fft_size_for(rate: u32) -> usize {
    ((rate as usize) / 50).next_power_of_two().clamp(256, 4096)
}

pub fn sqrt_hann(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| {
            let hann = 0.5 - 0.5 * (std::f64::consts::TAU * i as f64 / n as f64).cos();
            hann.sqrt() as f32
        })
        .collect()
}

/// Mean noise power per FFT bin of a mono signal.
#[derive(Clone, Debug, PartialEq)]
pub struct NoiseProfile {
    pub fft_size: usize,
    pub power: Vec<f32>,
}

/// Accumulates [`NoiseProfile`] power from windows of quiet audio.
pub struct ProfileBuilder {
    fft: Fft,
    window: Vec<f32>,
    sum: Vec<f64>,
    frames: usize,
    scratch: Vec<Complex>,
}

impl ProfileBuilder {
    pub fn new(fft_size: usize) -> Self {
        Self {
            fft: Fft::new(fft_size),
            window: sqrt_hann(fft_size),
            sum: vec![0.0; fft_size / 2 + 1],
            frames: 0,
            scratch: vec![Complex::default(); fft_size],
        }
    }

    pub fn fft_size(&self) -> usize {
        self.fft.len()
    }

    /// `mono` must hold exactly one FFT window of samples.
    pub fn add(&mut self, mono: &[f32]) {
        for (dst, (&x, &w)) in self.scratch.iter_mut().zip(mono.iter().zip(&self.window)) {
            *dst = Complex { re: x * w, im: 0.0 };
        }
        self.fft.process(&mut self.scratch, false);
        for (sum, bin) in self.sum.iter_mut().zip(&self.scratch) {
            *sum += bin.norm_sqr() as f64;
        }
        self.frames += 1;
    }

    pub fn finish(self) -> Option<NoiseProfile> {
        if self.frames == 0 {
            return None;
        }
        let frames = self.frames as f64;
        Some(NoiseProfile {
            fft_size: self.fft.len(),
            power: self.sum.iter().map(|s| (s / frames) as f32).collect(),
        })
    }
}

pub struct Denoiser {
    fft: Fft,
    window: Vec<f32>,
    noise: Vec<f32>,
    floor: f32,
}

impl Denoiser {
    pub fn new(profile: &NoiseProfile, reduction_db: f32) -> Self {
        Self {
            fft: Fft::new(profile.fft_size),
            window: sqrt_hann(profile.fft_size),
            noise: profile.power.clone(),
            floor: 10f32.powf(-reduction_db.abs() / 20.0),
        }
    }

    fn hop(&self) -> i64 {
        (self.fft.len() / 2) as i64
    }

    /// First STFT frame contributing to output `start`, and the last one for `end`.
    fn frames(&self, start: u64, end: u64) -> (i64, i64) {
        let n = self.fft.len() as i64;
        let hop = self.hop();
        let first = (start as i64 - n).div_euclid(hop) + 1;
        let last = (end.max(start + 1) as i64 - 1).div_euclid(hop);
        (first, last)
    }

    /// Input frames `[from, to)` needed to denoise output frames `[start, end)`. `from`
    /// can be negative; samples outside the file are zeros.
    pub fn input_range(&self, start: u64, end: u64) -> (i64, i64) {
        let (first, last) = self.frames(start, end);
        // One extra frame before `first` smooths the power estimate over time.
        (
            (first - 1) * self.hop(),
            last * self.hop() + self.fft.len() as i64,
        )
    }

    /// Denoises output frames `[start, end)`. `input` is interleaved and covers exactly
    /// [`Self::input_range`]. Every channel gets the same gains, taken from the mono mix,
    /// so the stereo image is kept.
    pub fn process(&self, input: &[f32], channels: usize, start: u64, end: u64) -> Vec<f32> {
        let n = self.fft.len();
        let bins = n / 2 + 1;
        let hop = self.hop();
        let (from, _) = self.input_range(start, end);
        let (first, last) = self.frames(start, end);
        let out_len = (end - start) as usize;
        let mut out = vec![0f32; out_len * channels];
        let mut spectra = vec![vec![Complex::default(); n]; channels];
        let mut previous_power: Option<Vec<f32>> = None;
        let mut power = vec![0f32; bins];
        let mut gains = vec![0f32; bins];
        for k in (first - 1)..=last {
            let offset = (k * hop - from) as usize;
            for (ch, spectrum) in spectra.iter_mut().enumerate() {
                for i in 0..n {
                    let x = input
                        .get((offset + i) * channels + ch)
                        .copied()
                        .unwrap_or(0.0);
                    spectrum[i] = Complex {
                        re: x * self.window[i],
                        im: 0.0,
                    };
                }
                self.fft.process(spectrum, false);
            }
            for (bin, p) in power.iter_mut().enumerate() {
                let (re, im) = spectra
                    .iter()
                    .fold((0.0, 0.0), |(re, im), s| (re + s[bin].re, im + s[bin].im));
                let scale = 1.0 / channels as f32;
                *p = (re * scale) * (re * scale) + (im * scale) * (im * scale);
            }
            let Some(previous) = previous_power.replace(power.clone()) else {
                continue;
            };
            for bin in 0..bins {
                // Average over two frames and three bins: a steadier estimate means less
                // "musical noise" from isolated bins flickering open.
                let lo = bin.saturating_sub(1);
                let hi = (bin + 1).min(bins - 1);
                let mut sum = 0.0;
                for b in lo..=hi {
                    sum += power[b] + previous[b];
                }
                let smoothed = sum / (2 * (hi - lo + 1)) as f32;
                let ratio = if smoothed > 0.0 {
                    1.0 - OVER_SUBTRACTION * self.noise[bin] / smoothed
                } else {
                    0.0
                };
                gains[bin] = ratio.max(self.floor * self.floor).sqrt().min(1.0);
            }
            for (ch, spectrum) in spectra.iter_mut().enumerate() {
                for bin in 0..bins {
                    let g = gains[bin];
                    spectrum[bin].re *= g;
                    spectrum[bin].im *= g;
                    if bin > 0 && bin < n / 2 {
                        spectrum[n - bin].re *= g;
                        spectrum[n - bin].im *= g;
                    }
                }
                self.fft.process(spectrum, true);
                let frame_start = k * hop;
                for i in 0..n {
                    let pos = frame_start + i as i64 - start as i64;
                    if pos < 0 || pos >= out_len as i64 {
                        continue;
                    }
                    out[pos as usize * channels + ch] += spectrum[i].re * self.window[i];
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic white noise in [-1, 1).
    fn noise(len: usize, seed: u64) -> Vec<f32> {
        let mut state = seed;
        (0..len)
            .map(|_| {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                ((state >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
            })
            .collect()
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
    }

    fn denoise_all(d: &Denoiser, signal: &[f32], start: u64, end: u64) -> Vec<f32> {
        let (from, to) = d.input_range(start, end);
        let input: Vec<f32> = (from..to)
            .map(|i| {
                if i < 0 {
                    0.0
                } else {
                    signal.get(i as usize).copied().unwrap_or(0.0)
                }
            })
            .collect();
        d.process(&input, 1, start, end)
    }

    #[test]
    fn unit_gains_reconstruct_the_input() {
        let rate = 48_000;
        let n = fft_size_for(rate);
        let signal = noise(10_000, 7);
        let profile = NoiseProfile {
            fft_size: n,
            power: vec![0.0; n / 2 + 1],
        };
        let d = Denoiser::new(&profile, 12.0);
        let out = denoise_all(&d, &signal, 0, 10_000);
        for (a, b) in out.iter().zip(&signal) {
            assert!((a - b).abs() < 1e-4);
        }
    }

    #[test]
    fn hiss_drops_and_a_tone_survives_independent_of_chunking() {
        let rate = 48_000u32;
        let n = fft_size_for(rate);
        let hiss: Vec<f32> = noise(rate as usize * 2, 3)
            .iter()
            .map(|v| v * 0.02)
            .collect();
        let mut builder = ProfileBuilder::new(n);
        for window in hiss[..rate as usize].chunks_exact(n) {
            builder.add(window);
        }
        let profile = builder.finish().unwrap();
        let d = Denoiser::new(&profile, 18.0);
        // Second half: the same hiss plus a loud 440 Hz tone.
        let tone: Vec<f32> = (0..rate as usize)
            .map(|i| 0.3 * (std::f32::consts::TAU * 440.0 * i as f32 / rate as f32).sin())
            .collect();
        let mut signal = hiss.clone();
        for (s, t) in signal[rate as usize..].iter_mut().zip(&tone) {
            *s += t;
        }
        let quiet = denoise_all(&d, &signal, 4_800, 43_200);
        let drop_db = 20.0 * (rms(&quiet) / rms(&hiss[4_800..43_200])).log10();
        assert!(drop_db < -12.0, "hiss only dropped {drop_db} dB");
        let a = 52_800u64;
        let b = 91_200u64;
        let loud = denoise_all(&d, &signal, a, b);
        let tone_rms = rms(&tone[(a as usize - 48_000)..(b as usize - 48_000)]);
        let change_db = 20.0 * (rms(&loud) / tone_rms).log10();
        assert!(change_db.abs() < 1.0, "tone changed {change_db} dB");
        let mut pieces = denoise_all(&d, &signal, a, a + 1_000);
        pieces.extend(denoise_all(&d, &signal, a + 1_000, b));
        for (x, y) in pieces.iter().zip(&loud) {
            assert!((x - y).abs() < 1e-5);
        }
    }
}
