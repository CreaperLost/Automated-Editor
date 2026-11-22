//! Speech detection by model (SND-6): RNNoise's voice probability every 10 ms, the same
//! network that removes noise (`nnnoiseless`, built in, no download). Silence detection uses
//! it to keep quiet speech that a level threshold alone would cut; it never makes a cut by
//! itself.
use nnnoiseless::DenoiseState;

const RATE: u32 = 48_000;
const FRAME: usize = DenoiseState::FRAME_SIZE;
/// A 10 ms frame at or over this probability is speech.
const SPEECH: f32 = 0.6;
/// Speech is held this long on either side: word onsets and tails are quieter than their
/// middle and score lower.
pub const HANG_US: u64 = 150_000;

pub struct SpeechDetector {
    state: Box<DenoiseState<'static>>,
    /// Input samples per 48 kHz sample.
    ratio: f64,
    /// Where the next 48 kHz sample falls, in input samples since the stream began.
    next: f64,
    /// Input samples seen.
    seen: u64,
    frame: Vec<f32>,
    frame_us: u64,
    out: Vec<f32>,
    speech: Vec<(u64, u64)>,
}

impl SpeechDetector {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            state: DenoiseState::new(),
            ratio: sample_rate.max(1) as f64 / RATE as f64,
            next: 0.0,
            seen: 0,
            frame: Vec::with_capacity(FRAME),
            frame_us: 0,
            out: vec![0.0; FRAME],
            speech: Vec::new(),
        }
    }

    /// Interleaved `samples` (-1 to 1), starting at `source_us`. Chunks follow on in time.
    pub fn feed(&mut self, samples: &[f32], channels: usize, source_us: u64, sample_rate: u32) {
        let channels = channels.max(1);
        let first = self.seen;
        for (i, frame) in samples.chunks_exact(channels).enumerate() {
            let mono = frame.iter().sum::<f32>() / channels as f32 * 32_768.0;
            let n = first + i as u64;
            // Each 48 kHz sample takes the input sample it falls in.
            while self.next < (n + 1) as f64 {
                if self.frame.is_empty() {
                    self.frame_us = source_us + i as u64 * 1_000_000 / sample_rate.max(1) as u64;
                }
                self.frame.push(mono);
                self.next += self.ratio;
                if self.frame.len() == FRAME {
                    let p = self.state.process_frame(&mut self.out, &self.frame);
                    if p >= SPEECH {
                        let (a, b) = (self.frame_us, self.frame_us + 10_000);
                        match self.speech.last_mut() {
                            Some(last) if a <= last.1 + 2 * HANG_US => last.1 = b,
                            _ => self.speech.push((a, b)),
                        }
                    }
                    self.frame.clear();
                }
            }
        }
        self.seen = first + (samples.len() / channels) as u64;
    }

    /// Source-time ranges of speech, each held [`HANG_US`] on either side.
    pub fn finish(self) -> Vec<(u64, u64)> {
        self.speech
            .into_iter()
            .map(|(a, b)| (a.saturating_sub(HANG_US), b + HANG_US))
            .collect()
    }
}

/// `runs` with every part that overlaps `speech` taken out. Both are sorted.
pub fn without(runs: Vec<(u64, u64)>, speech: &[(u64, u64)]) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    for (mut a, b) in runs {
        let first = speech.partition_point(|&(_, e)| e <= a);
        for &(s, e) in &speech[first..] {
            if s >= b {
                break;
            }
            if s > a {
                out.push((a, s));
            }
            a = a.max(e);
            if a >= b {
                break;
            }
        }
        if a < b {
            out.push((a, b));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speech_is_taken_out_of_silent_runs() {
        assert_eq!(
            without(vec![(0, 100), (200, 300)], &[(50, 60), (90, 210)]),
            vec![(0, 50), (60, 90), (210, 300)]
        );
        assert_eq!(without(vec![(0, 100)], &[]), vec![(0, 100)]);
        assert_eq!(without(vec![(10, 20)], &[(0, 100)]), vec![]);
    }

    /// The fixture's voice is found as speech, at 48 kHz and resampled from 16 kHz.
    #[test]
    fn the_fixture_voice_is_speech_and_hiss_is_not() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../test/fixtures/voice.wav");
        let mut reader = crate::project::pcm::PcmReader::open(&path).unwrap();
        let info = reader.info().clone();
        let ch = info.channels as usize;
        let mut samples = vec![0f32; info.frame_count as usize * ch];
        let got = reader
            .read_frames(&mut samples, info.frame_count as usize)
            .unwrap();
        samples.truncate(got * ch);
        let mut detector = SpeechDetector::new(info.sample_rate);
        for (k, chunk) in samples.chunks(4_096 * ch).enumerate() {
            let at = (k * 4_096) as u64 * 1_000_000 / info.sample_rate as u64;
            detector.feed(chunk, ch, at, info.sample_rate);
        }
        let speech = detector.finish();
        let total: u64 = speech.iter().map(|(a, b)| b - a).sum();
        let length = got as u64 * 1_000_000 / info.sample_rate as u64;
        assert!(total > length / 4, "speech {total} of {length}: {speech:?}");

        let mut seed = 7u32;
        let hiss: Vec<f32> = (0..48_000 * 2)
            .map(|_| {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                ((seed >> 16) as f32 / 32_768.0 - 1.0) * 0.02
            })
            .collect();
        let mut detector = SpeechDetector::new(48_000);
        detector.feed(&hiss, 1, 0, 48_000);
        assert!(detector.finish().is_empty());
    }
}
