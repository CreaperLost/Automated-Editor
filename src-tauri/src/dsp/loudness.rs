//! ITU-R BS.1770 / EBU R128 loudness: K-weighting at any sample rate and gated
//! integration over 400 ms blocks.

/// Two cascaded biquads (high-shelf "head" filter, then the RLB high-pass).
#[derive(Clone, Debug)]
pub struct KWeighting {
    b: [[f64; 3]; 2],
    a: [[f64; 3]; 2],
    state: [[f64; 4]; 2],
}

impl KWeighting {
    /// Coefficients derived for `rate` the way libebur128 does, so 48 kHz matches the
    /// tabulated BS.1770 values and other rates stay accurate.
    pub fn new(rate: u32) -> Self {
        let rate = rate as f64;
        let f0 = 1681.974450955533;
        let gain_db = 3.999843853973347;
        let q = 0.7071752369554196;
        let k = (std::f64::consts::PI * f0 / rate).tan();
        let vh = 10f64.powf(gain_db / 20.0);
        let vb = vh.powf(0.4996667741545416);
        let a0 = 1.0 + k / q + k * k;
        let shelf_b = [
            (vh + vb * k / q + k * k) / a0,
            2.0 * (k * k - vh) / a0,
            (vh - vb * k / q + k * k) / a0,
        ];
        let shelf_a = [1.0, 2.0 * (k * k - 1.0) / a0, (1.0 - k / q + k * k) / a0];
        let f0 = 38.13547087602444;
        let q = 0.5003270373238773;
        let k = (std::f64::consts::PI * f0 / rate).tan();
        let a0 = 1.0 + k / q + k * k;
        let hp_b = [1.0, -2.0, 1.0];
        let hp_a = [1.0, 2.0 * (k * k - 1.0) / a0, (1.0 - k / q + k * k) / a0];
        Self {
            b: [shelf_b, hp_b],
            a: [shelf_a, hp_a],
            state: [[0.0; 4]; 2],
        }
    }

    pub fn process(&mut self, x: f32) -> f64 {
        let mut v = x as f64;
        for stage in 0..2 {
            let [x1, x2, y1, y2] = self.state[stage];
            let b = self.b[stage];
            let a = self.a[stage];
            let y = b[0] * v + b[1] * x1 + b[2] * x2 - a[1] * y1 - a[2] * y2;
            self.state[stage] = [v, x1, y, y1];
            v = y;
        }
        v
    }
}

pub fn energy_to_lufs(energy: f64) -> f64 {
    -0.691 + 10.0 * energy.max(1e-20).log10()
}

/// Integrated loudness from 100 ms sub-block energies (mean square, summed over
/// channels) in playback order. Four consecutive sub-blocks form one 400 ms gating
/// block with 75% overlap. Returns `None` when every block is below the -70 LUFS gate.
pub fn integrated_lufs(sub_blocks: &[f64]) -> Option<f64> {
    if sub_blocks.len() < 4 {
        return None;
    }
    let blocks: Vec<f64> = sub_blocks
        .windows(4)
        .map(|w| w.iter().sum::<f64>() / 4.0)
        .collect();
    let absolute: Vec<f64> = blocks
        .into_iter()
        .filter(|&e| energy_to_lufs(e) > -70.0)
        .collect();
    if absolute.is_empty() {
        return None;
    }
    let mean = absolute.iter().sum::<f64>() / absolute.len() as f64;
    let relative_gate = energy_to_lufs(mean) - 10.0;
    let gated: Vec<f64> = absolute
        .into_iter()
        .filter(|&e| energy_to_lufs(e) > relative_gate)
        .collect();
    if gated.is_empty() {
        return None;
    }
    Some(energy_to_lufs(
        gated.iter().sum::<f64>() / gated.len() as f64,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine_sub_blocks(rate: u32, amplitude: f32, seconds: usize) -> Vec<f64> {
        let mut filter = KWeighting::new(rate);
        let block = rate as usize / 10;
        let mut energy = Vec::new();
        let mut sum = 0.0;
        for i in 0..rate as usize * seconds {
            let x = amplitude * (std::f32::consts::TAU * 997.0 * i as f32 / rate as f32).sin();
            let y = filter.process(x);
            // Same signal on both channels of a stereo output.
            sum += 2.0 * y * y;
            if (i + 1) % block == 0 {
                energy.push(sum / block as f64);
                sum = 0.0;
            }
        }
        energy
    }

    #[test]
    fn stereo_tone_at_minus_23_dbfs_reads_minus_23_lufs() {
        let amplitude = 10f32.powf(-23.0 / 20.0);
        for rate in [44_100, 48_000, 96_000] {
            let lufs = integrated_lufs(&sine_sub_blocks(rate, amplitude, 3)).unwrap();
            assert!((lufs + 23.0).abs() < 0.2, "{rate} Hz: {lufs}");
        }
    }

    #[test]
    fn quiet_blocks_are_gated_out() {
        let amplitude = 10f32.powf(-23.0 / 20.0);
        let mut blocks = sine_sub_blocks(48_000, amplitude, 3);
        // Silence is below the absolute gate and must not drag the result down.
        blocks.extend(std::iter::repeat(1e-12).take(100));
        let lufs = integrated_lufs(&blocks).unwrap();
        assert!((lufs + 23.0).abs() < 0.3, "{lufs}");
        assert_eq!(integrated_lufs(&[1e-12; 50]), None);
    }
}
