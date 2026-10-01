//! Small in-place radix-2 complex FFT. Audio polish only needs a few hundred
//! transforms of 512-2048 points per second of audio, so this is plenty.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Complex {
    pub re: f32,
    pub im: f32,
}

impl Complex {
    pub fn norm_sqr(self) -> f32 {
        self.re * self.re + self.im * self.im
    }
}

/// Precomputed twiddles and bit reversal for one power-of-two size.
pub struct Fft {
    n: usize,
    twiddles: Vec<Complex>,
    reversed: Vec<u32>,
}

impl Fft {
    pub fn new(n: usize) -> Self {
        assert!(
            n.is_power_of_two() && n >= 2,
            "FFT size must be a power of two"
        );
        let twiddles = (0..n / 2)
            .map(|k| {
                let angle = -std::f64::consts::TAU * k as f64 / n as f64;
                Complex {
                    re: angle.cos() as f32,
                    im: angle.sin() as f32,
                }
            })
            .collect();
        let bits = n.trailing_zeros();
        let reversed = (0..n as u32)
            .map(|i| i.reverse_bits() >> (32 - bits))
            .collect();
        Self {
            n,
            twiddles,
            reversed,
        }
    }

    pub fn len(&self) -> usize {
        self.n
    }

    /// Forward transform. `inverse` conjugates the twiddles and scales by 1/n.
    pub fn process(&self, data: &mut [Complex], inverse: bool) {
        assert_eq!(data.len(), self.n);
        for (i, &j) in self.reversed.iter().enumerate() {
            let j = j as usize;
            if i < j {
                data.swap(i, j);
            }
        }
        let mut size = 2;
        while size <= self.n {
            let half = size / 2;
            let stride = self.n / size;
            for start in (0..self.n).step_by(size) {
                for k in 0..half {
                    let mut w = self.twiddles[k * stride];
                    if inverse {
                        w.im = -w.im;
                    }
                    let a = data[start + k];
                    let b = data[start + k + half];
                    let t = Complex {
                        re: b.re * w.re - b.im * w.im,
                        im: b.re * w.im + b.im * w.re,
                    };
                    data[start + k] = Complex {
                        re: a.re + t.re,
                        im: a.im + t.im,
                    };
                    data[start + k + half] = Complex {
                        re: a.re - t.re,
                        im: a.im - t.im,
                    };
                }
            }
            size *= 2;
        }
        if inverse {
            let scale = 1.0 / self.n as f32;
            for v in data.iter_mut() {
                v.re *= scale;
                v.im *= scale;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_tone_and_round_trips() {
        let n = 64;
        let fft = Fft::new(n);
        let input: Vec<Complex> = (0..n)
            .map(|i| Complex {
                re: (std::f32::consts::TAU * 5.0 * i as f32 / n as f32).cos(),
                im: 0.0,
            })
            .collect();
        let mut data = input.clone();
        fft.process(&mut data, false);
        let peak = (0..n / 2)
            .max_by(|&a, &b| data[a].norm_sqr().total_cmp(&data[b].norm_sqr()))
            .unwrap();
        assert_eq!(peak, 5);
        assert!((data[5].re - n as f32 / 2.0).abs() < 1e-3);
        fft.process(&mut data, true);
        for (a, b) in data.iter().zip(&input) {
            assert!((a.re - b.re).abs() < 1e-5 && a.im.abs() < 1e-5);
        }
    }
}
