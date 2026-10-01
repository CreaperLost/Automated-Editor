//! Turns an audio track's PCM segments into 16 kHz mono chunks for speech recognition.
//!
//! Chunks split at the quietest point near the size limit, so a word is rarely cut in half,
//! and each chunk carries its source start time so provider timestamps map back exactly.
//! Short gaps between segments are filled with silence; long gaps start a new chunk.
use crate::project::pcm::{PcmReader, READ_FRAME_CHUNK};
use crate::project::reader::{safe_path, SegmentSummary};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

pub const ASR_SAMPLE_RATE: u32 = 16_000;
const QUIET_FRAME_SAMPLES: usize = 320; // 20 ms
const MAX_FILLED_GAP_US: u64 = 2_000_000;
const MAX_DIAGNOSTICS: usize = 32;

#[derive(Clone, Copy, Debug)]
pub struct ChunkPlan {
    /// Longest chunk handed to the provider.
    pub max_chunk_us: u64,
    /// How far back from the limit to look for a quiet split point.
    pub search_window_us: u64,
}

pub struct AudioChunk {
    pub source_start_us: u64,
    /// Mono samples at [`ASR_SAMPLE_RATE`], in -1.0..=1.0.
    pub samples: Vec<f32>,
}

impl AudioChunk {
    pub fn duration_us(&self) -> u64 {
        samples_to_us(self.samples.len() as u64)
    }

    pub fn to_wav_bytes(&self) -> Vec<u8> {
        let pcm: Vec<i16> = self
            .samples
            .iter()
            .map(|s| (s.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16)
            .collect();
        crate::fixtures::generate_pcm16_wav(ASR_SAMPLE_RATE, 1, &pcm)
    }
}

fn samples_to_us(samples: u64) -> u64 {
    (samples as u128 * 1_000_000 / ASR_SAMPLE_RATE as u128) as u64
}

fn us_to_samples(us: u64) -> usize {
    (us as u128 * ASR_SAMPLE_RATE as u128 / 1_000_000) as usize
}

/// Box-filter resampler to 16 kHz mono. Averaging every input sample that falls inside an
/// output period low-passes enough for speech recognition when downsampling.
struct Resampler {
    ratio: f64,
    next_boundary: f64,
    input_index: u64,
    sum: f32,
    count: u32,
    last: f32,
}

impl Resampler {
    fn new(input_rate: u32) -> Self {
        let ratio = input_rate as f64 / ASR_SAMPLE_RATE as f64;
        Self {
            ratio,
            next_boundary: ratio,
            input_index: 0,
            sum: 0.0,
            count: 0,
            last: 0.0,
        }
    }

    fn push(&mut self, sample: f32, out: &mut Vec<f32>) {
        self.sum += sample;
        self.count += 1;
        self.input_index += 1;
        while self.input_index as f64 >= self.next_boundary {
            let value = if self.count > 0 {
                self.sum / self.count as f32
            } else {
                self.last
            };
            out.push(value);
            self.last = value;
            self.sum = 0.0;
            self.count = 0;
            self.next_boundary += self.ratio;
        }
    }
}

struct Chunker<'a, F: FnMut(AudioChunk) -> Result<(), String>> {
    plan: ChunkPlan,
    run_start_us: u64,
    emitted_samples: u64,
    buffer: Vec<f32>,
    sink: &'a mut F,
}

impl<F: FnMut(AudioChunk) -> Result<(), String>> Chunker<'_, F> {
    fn current_end_us(&self) -> u64 {
        self.run_start_us + samples_to_us(self.emitted_samples + self.buffer.len() as u64)
    }

    fn start_run(&mut self, source_us: u64) -> Result<(), String> {
        self.flush()?;
        self.run_start_us = source_us;
        self.emitted_samples = 0;
        Ok(())
    }

    fn extend(&mut self, samples: &[f32]) -> Result<(), String> {
        let max = us_to_samples(self.plan.max_chunk_us).max(QUIET_FRAME_SAMPLES * 2);
        let window = us_to_samples(self.plan.search_window_us).min(max / 2);
        let mut rest = samples;
        while !rest.is_empty() {
            let take = (max - self.buffer.len()).min(rest.len());
            self.buffer.extend_from_slice(&rest[..take]);
            rest = &rest[take..];
            if self.buffer.len() >= max {
                let split = quietest_split(&self.buffer, max - window, max);
                self.emit(split)?;
            }
        }
        Ok(())
    }

    fn emit(&mut self, len: usize) -> Result<(), String> {
        if len == 0 {
            return Ok(());
        }
        let rest = self.buffer.split_off(len);
        let samples = std::mem::replace(&mut self.buffer, rest);
        let source_start_us = self.run_start_us + samples_to_us(self.emitted_samples);
        self.emitted_samples += samples.len() as u64;
        (self.sink)(AudioChunk {
            source_start_us,
            samples,
        })
    }

    fn flush(&mut self) -> Result<(), String> {
        let len = self.buffer.len();
        self.emit(len)
    }
}

/// Start of the quietest 20 ms frame in `buffer[from..to]`, or `to` if the range is empty.
fn quietest_split(buffer: &[f32], from: usize, to: usize) -> usize {
    let mut best = to;
    let mut best_energy = f32::INFINITY;
    let mut start = from;
    while start + QUIET_FRAME_SAMPLES <= to {
        let energy: f32 = buffer[start..start + QUIET_FRAME_SAMPLES]
            .iter()
            .map(|s| s * s)
            .sum();
        if energy < best_energy {
            best_energy = energy;
            best = start + QUIET_FRAME_SAMPLES / 2;
        }
        start += QUIET_FRAME_SAMPLES;
    }
    best
}

/// Streams `segments` as 16 kHz mono chunks into `sink`, in source-time order. `progress`
/// receives the source time reached. Returns diagnostics for segments that were skipped.
pub fn for_each_chunk<F, P>(
    root: &Path,
    segments: &[SegmentSummary],
    plan: ChunkPlan,
    cancel: &AtomicBool,
    mut progress: P,
    mut sink: F,
) -> Result<Vec<String>, String>
where
    F: FnMut(AudioChunk) -> Result<(), String>,
    P: FnMut(u64),
{
    let mut diagnostics = Vec::new();
    let mut chunker = Chunker {
        plan,
        run_start_us: 0,
        emitted_samples: 0,
        buffer: Vec::new(),
        sink: &mut sink,
    };
    let mut started = false;
    let mut ordered: Vec<&SegmentSummary> = segments.iter().collect();
    ordered.sort_by_key(|s| s.start_us);

    for segment in ordered {
        if cancel.load(Ordering::Relaxed) {
            return Err("Transcription cancelled".into());
        }
        let opened = safe_path(root, &segment.relative_path).and_then(|path| {
            if !segment.available || !path.is_file() {
                return Err("missing file".into());
            }
            PcmReader::open(&path)
        });
        let mut reader = match opened {
            Ok(reader) => reader,
            Err(error) => {
                if diagnostics.len() < MAX_DIAGNOSTICS {
                    diagnostics.push(format!(
                        "Skipped audio segment {} ({error})",
                        segment.relative_path
                    ));
                }
                continue;
            }
        };
        let info = reader.info().clone();

        let cursor = chunker.current_end_us();
        if !started || segment.start_us < cursor || segment.start_us - cursor > MAX_FILLED_GAP_US {
            chunker.start_run(segment.start_us)?;
            started = true;
        } else if segment.start_us > cursor {
            let gap = vec![0.0f32; us_to_samples(segment.start_us - cursor)];
            chunker.extend(&gap)?;
        }

        let channels = info.channels as usize;
        let segment_frames = info
            .frames_for_us(segment.end_us.saturating_sub(segment.start_us))
            .min(info.frame_count);
        let mut resampler = Resampler::new(info.sample_rate);
        let mut interleaved = vec![0.0f32; READ_FRAME_CHUNK * channels];
        let mut out = Vec::with_capacity(READ_FRAME_CHUNK);
        let mut frames_read = 0u64;
        while frames_read < segment_frames {
            if cancel.load(Ordering::Relaxed) {
                return Err("Transcription cancelled".into());
            }
            let want = (segment_frames - frames_read).min(READ_FRAME_CHUNK as u64) as usize;
            let frames = match reader.read_frames(&mut interleaved, want) {
                Ok(0) => break,
                Ok(frames) => frames,
                Err(error) => {
                    if diagnostics.len() < MAX_DIAGNOSTICS {
                        diagnostics.push(format!(
                            "Stopped reading {} early ({error})",
                            segment.relative_path
                        ));
                    }
                    break;
                }
            };
            out.clear();
            for frame in interleaved[..frames * channels].chunks_exact(channels) {
                let mono = frame.iter().sum::<f32>() / channels as f32;
                resampler.push(mono, &mut out);
            }
            chunker.extend(&out)?;
            frames_read += frames as u64;
            progress(segment.start_us + info.frame_us(frames_read));
        }
    }
    chunker.flush()?;
    Ok(diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::generate_pcm16_wav;

    fn segment(path: &str, start_us: u64, end_us: u64, size: u64) -> SegmentSummary {
        SegmentSummary {
            track_id: "mic".into(),
            relative_path: path.into(),
            start_us,
            end_us,
            size_bytes: size,
            media_timescale: 48_000,
            media_start_value: 0,
            host_anchor_us: 0,
            is_keyframe_start: None,
            available: true,
        }
    }

    fn write_wav(dir: &Path, name: &str, rate: u32, channels: u16, samples: &[i16]) -> u64 {
        let bytes = generate_pcm16_wav(rate, channels, samples);
        std::fs::write(dir.join(name), &bytes).unwrap();
        bytes.len() as u64
    }

    #[test]
    fn resamples_and_splits_at_quiet_point() {
        let dir = tempfile::tempdir().unwrap();
        // 3 s of 48 kHz stereo: loud tone, except a quiet gap at 1.8-1.9 s.
        let mut samples = Vec::new();
        for i in 0..(48_000 * 3) {
            let t = i as f32 / 48_000.0;
            let v = if (1.8..1.9).contains(&t) {
                0
            } else {
                ((t * 440.0 * std::f32::consts::TAU).sin() * 10_000.0) as i16
            };
            samples.push(v);
            samples.push(v);
        }
        let size = write_wav(dir.path(), "a.wav", 48_000, 2, &samples);
        let segs = vec![segment("a.wav", 5_000_000, 8_000_000, size)];
        let mut chunks = Vec::new();
        let plan = ChunkPlan {
            max_chunk_us: 2_000_000,
            search_window_us: 500_000,
        };
        let diags = for_each_chunk(
            dir.path(),
            &segs,
            plan,
            &AtomicBool::new(false),
            |_| {},
            |c| {
                chunks.push((c.source_start_us, c.samples.len()));
                Ok(())
            },
        )
        .unwrap();
        assert!(diags.is_empty());
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].0, 5_000_000);
        // Split lands inside the quiet gap, 1.8-1.9 s into the segment.
        let split_us = samples_to_us(chunks[0].1 as u64);
        assert!((1_800_000..=1_900_000).contains(&split_us), "{split_us}");
        assert_eq!(chunks[1].0, 5_000_000 + split_us);
        let total: usize = chunks.iter().map(|c| c.1).sum();
        assert!((47_990..=48_000).contains(&total), "{total}");
    }

    #[test]
    fn fills_short_gaps_and_restarts_after_long_ones() {
        let dir = tempfile::tempdir().unwrap();
        let one_second = vec![1000i16; 16_000];
        let size = write_wav(dir.path(), "a.wav", 16_000, 1, &one_second);
        write_wav(dir.path(), "b.wav", 16_000, 1, &one_second);
        write_wav(dir.path(), "c.wav", 16_000, 1, &one_second);
        let segs = vec![
            segment("a.wav", 0, 1_000_000, size),
            segment("b.wav", 1_500_000, 2_500_000, size),
            segment("c.wav", 10_000_000, 11_000_000, size),
        ];
        let mut chunks = Vec::new();
        for_each_chunk(
            dir.path(),
            &segs,
            ChunkPlan {
                max_chunk_us: 60_000_000,
                search_window_us: 5_000_000,
            },
            &AtomicBool::new(false),
            |_| {},
            |c| {
                chunks.push((c.source_start_us, c.duration_us()));
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(chunks, vec![(0, 2_500_000), (10_000_000, 1_000_000)]);
    }

    #[test]
    fn missing_segments_are_reported_and_cancel_stops() {
        let dir = tempfile::tempdir().unwrap();
        let segs = vec![segment("missing.wav", 0, 1_000_000, 10)];
        let plan = ChunkPlan {
            max_chunk_us: 1_000_000,
            search_window_us: 100_000,
        };
        let diags = for_each_chunk(
            dir.path(),
            &segs,
            plan,
            &AtomicBool::new(false),
            |_| {},
            |_| Ok(()),
        )
        .unwrap();
        assert_eq!(diags.len(), 1);
        assert!(for_each_chunk(
            dir.path(),
            &segs,
            plan,
            &AtomicBool::new(true),
            |_| {},
            |_| Ok(())
        )
        .is_err());
    }
}
