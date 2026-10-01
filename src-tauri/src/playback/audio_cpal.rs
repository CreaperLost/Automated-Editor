//! Playback audio through cpal (WASAPI on Windows). The mixer's 48 kHz stereo PCM is queued
//! here and resampled to the device rate in the output callback. The clock is the number of
//! source frames the callback has consumed.
use crate::media::audio::SAMPLE_RATE;
use std::collections::VecDeque;

/// Queued 48 kHz stereo PCM plus the linear resampler state, shared with the audio callback.
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Default)]
pub(crate) struct PcmQueue {
    samples: VecDeque<i16>,
    /// Source frames fully played out, which is the playback clock.
    consumed: u64,
    /// Position between the first two queued frames, in source frames.
    phase: f64,
    pub(crate) playing: bool,
    pub(crate) failed: Option<String>,
}

#[cfg_attr(not(windows), allow(dead_code))]
impl PcmQueue {
    pub(crate) fn push(&mut self, pcm: &[i16]) {
        self.samples.extend(pcm.iter().copied());
    }

    pub(crate) fn consumed(&self) -> u64 {
        self.consumed
    }

    fn frame(&self, index: usize) -> Option<[f32; 2]> {
        let left = *self.samples.get(index * 2)?;
        let right = *self.samples.get(index * 2 + 1)?;
        Some([
            f32::from(left) / f32::from(i16::MAX),
            f32::from(right) / f32::from(i16::MAX),
        ])
    }

    /// Fills `out` (interleaved, `channels` per frame) at `device_rate`. Plays silence while
    /// paused or starved, without advancing the clock.
    pub(crate) fn render(&mut self, out: &mut [f32], channels: usize, device_rate: u32) {
        let channels = channels.max(1);
        let step = f64::from(SAMPLE_RATE) / f64::from(device_rate.max(1));
        for frame in out.chunks_mut(channels) {
            let sample = if self.playing {
                self.next_frame(step)
            } else {
                None
            };
            let [left, right] = sample.unwrap_or([0.0, 0.0]);
            match frame {
                [mono] => *mono = (left + right) * 0.5,
                [l, r, rest @ ..] => {
                    *l = left;
                    *r = right;
                    rest.fill(0.0);
                }
                [] => {}
            }
        }
    }

    fn next_frame(&mut self, step: f64) -> Option<[f32; 2]> {
        let a = self.frame(0)?;
        // With one frame left there is nothing to interpolate towards yet.
        let b = self.frame(1).unwrap_or(a);
        let t = self.phase as f32;
        let out = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
        self.phase += step;
        while self.phase >= 1.0 && !self.samples.is_empty() {
            self.samples.drain(..2);
            self.consumed += 1;
            self.phase -= 1.0;
        }
        if self.samples.is_empty() {
            self.phase = 0.0;
        }
        Some(out)
    }
}

#[cfg(windows)]
mod device {
    use super::PcmQueue;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use parking_lot::Mutex;
    use std::sync::{mpsc, Arc};

    /// cpal streams are not `Send`, so each one lives on its own thread until dropped.
    pub struct AudioOutput {
        queue: Arc<Mutex<PcmQueue>>,
        stop: Option<mpsc::Sender<()>>,
        thread: Option<std::thread::JoinHandle<()>>,
    }

    fn build<T>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        queue: Arc<Mutex<PcmQueue>>,
    ) -> Result<cpal::Stream, cpal::BuildStreamError>
    where
        T: cpal::SizedSample + cpal::FromSample<f32>,
    {
        let channels = config.channels as usize;
        let rate = config.sample_rate.0;
        let errors = Arc::clone(&queue);
        let mut scratch = Vec::new();
        device.build_output_stream(
            config,
            move |out: &mut [T], _| {
                scratch.resize(out.len(), 0.0f32);
                queue.lock().render(&mut scratch, channels, rate);
                for (dst, src) in out.iter_mut().zip(&scratch) {
                    *dst = T::from_sample(*src);
                }
            },
            move |error| {
                errors.lock().failed = Some(format!("Audio device error: {error}"));
            },
            None,
        )
    }

    fn open(queue: Arc<Mutex<PcmQueue>>) -> Result<cpal::Stream, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No audio output device is available")?;
        let supported = device
            .default_output_config()
            .map_err(|e| format!("Could not read the audio device format: {e}"))?;
        let config = supported.config();
        let shared = queue;
        let stream = match supported.sample_format() {
            cpal::SampleFormat::F32 => build::<f32>(&device, &config, shared),
            cpal::SampleFormat::I16 => build::<i16>(&device, &config, shared),
            cpal::SampleFormat::U16 => build::<u16>(&device, &config, shared),
            cpal::SampleFormat::I32 => build::<i32>(&device, &config, shared),
            other => return Err(format!("Unsupported audio device format: {other}")),
        }
        .map_err(|e| format!("Could not start the audio output device: {e}"))?;
        stream
            .play()
            .map_err(|e| format!("Could not start the audio output device: {e}"))?;
        Ok(stream)
    }

    impl AudioOutput {
        pub fn new() -> Result<Self, String> {
            let queue = Arc::new(Mutex::new(PcmQueue::default()));
            let shared = Arc::clone(&queue);
            let (ready_tx, ready_rx) = mpsc::channel();
            let (stop_tx, stop_rx) = mpsc::channel::<()>();
            let thread = std::thread::spawn(move || match open(shared) {
                Ok(stream) => {
                    let _ = ready_tx.send(Ok(()));
                    // Blocks until the owner drops its sender.
                    let _ = stop_rx.recv();
                    let _ = stream.pause();
                }
                Err(error) => {
                    let _ = ready_tx.send(Err(error));
                }
            });
            ready_rx
                .recv()
                .map_err(|_| "The audio output thread stopped".to_string())??;
            Ok(Self {
                queue,
                stop: Some(stop_tx),
                thread: Some(thread),
            })
        }

        pub fn queue(&mut self, pcm: &[i16]) -> Result<(), String> {
            self.queue.lock().push(pcm);
            Ok(())
        }

        pub fn play(&mut self) {
            self.queue.lock().playing = true;
        }

        pub fn position_frames(&self) -> Result<u64, String> {
            let queue = self.queue.lock();
            match &queue.failed {
                Some(error) => Err(error.clone()),
                None => Ok(queue.consumed()),
            }
        }
    }

    impl Drop for AudioOutput {
        fn drop(&mut self) {
            drop(self.stop.take());
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

#[cfg(windows)]
pub use device::AudioOutput;

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp(frames: i16) -> Vec<i16> {
        (0..frames).flat_map(|i| [i * 100, -i * 100]).collect()
    }

    #[test]
    fn same_rate_plays_frames_in_order_and_counts_them() {
        let mut queue = PcmQueue::default();
        queue.push(&ramp(10));
        queue.playing = true;
        let mut out = vec![0.0f32; 4 * 2];
        queue.render(&mut out, 2, 48_000);
        let scale = f32::from(i16::MAX);
        for frame in 0..4 {
            assert!((out[frame * 2] - frame as f32 * 100.0 / scale).abs() < 1e-6);
            assert!((out[frame * 2 + 1] + frame as f32 * 100.0 / scale).abs() < 1e-6);
        }
        assert_eq!(queue.consumed(), 4);
    }

    #[test]
    fn resamples_to_the_device_rate() {
        let mut queue = PcmQueue::default();
        queue.push(&ramp(100));
        queue.playing = true;
        // 24 kHz device: each output frame advances two source frames.
        let mut out = vec![0.0f32; 10];
        queue.render(&mut out, 1, 24_000);
        assert_eq!(queue.consumed(), 20);
        // 96 kHz device: halfway frames are interpolated.
        let mut queue = PcmQueue::default();
        queue.push(&ramp(100));
        queue.playing = true;
        let mut out = vec![0.0f32; 4];
        queue.render(&mut out, 2, 96_000);
        let half = 50.0 / f32::from(i16::MAX);
        assert!((out[2] - half).abs() < 1e-6, "interpolated {}", out[2]);
        assert_eq!(queue.consumed(), 1);
    }

    #[test]
    fn paused_or_starved_output_is_silent_and_holds_the_clock() {
        let mut queue = PcmQueue::default();
        queue.push(&ramp(4));
        let mut out = vec![1.0f32; 8];
        queue.render(&mut out, 2, 48_000);
        assert!(out.iter().all(|s| *s == 0.0));
        assert_eq!(queue.consumed(), 0);
        queue.playing = true;
        let mut out = vec![1.0f32; 32];
        queue.render(&mut out, 2, 48_000);
        assert_eq!(queue.consumed(), 4, "every queued frame is played out");
        assert!(out[8..].iter().all(|s| *s == 0.0), "then silence");
    }

    #[test]
    fn extra_device_channels_are_silent() {
        let mut queue = PcmQueue::default();
        queue.push(&[i16::MAX, i16::MAX, i16::MAX, i16::MAX]);
        queue.playing = true;
        let mut out = vec![9.0f32; 6];
        queue.render(&mut out, 6, 48_000);
        assert_eq!(&out[..2], &[1.0, 1.0]);
        assert!(out[2..].iter().all(|s| *s == 0.0));
    }
}
