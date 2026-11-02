//! Room tone under muted mouth noises (SND-4). Muting a noise leaves digital silence, which
//! sounds gated in a pause: the room's own sound goes there instead. Each sound's room tone is
//! its quietest stretch that isn't silence, found once per file (cached like the click repair)
//! and played forwards and backwards in turn, so it never clicks where it repeats.
use crate::project::pcm::PcmReader;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::SystemTime;

/// How the quietness of a sound is measured: windows this long...
const WINDOW_MS: u64 = 50;
/// ...this many in a row make the room tone (0.4 s).
const RUN: usize = 8;
/// Quieter than this is digital silence (a muted or gated stretch), not the room.
const SILENT_RMS: f32 = 1e-5;
/// Louder than this (-40 dBFS) is not a room: a sound with no quieter stretch gets none.
const MAX_ROOM_RMS: f32 = 0.01;
/// The most of a file looked through for its quietest stretch (10 minutes).
const MAX_SCAN_MS: u64 = 600_000;

/// A short stretch of a sound's room, mono, at the sound's own rate.
pub struct RoomTone {
    samples: Vec<f32>,
    rate: u32,
}

impl RoomTone {
    /// The room at `source_us` into the sound (any time: it repeats, back and forth).
    pub fn at(&self, source_us: f64) -> f64 {
        let n = self.samples.len();
        if n < 2 {
            return 0.0;
        }
        let index = (source_us.max(0.0) * self.rate as f64 / 1e6) as usize;
        let period = 2 * n - 2;
        let k = index % period;
        let k = if k < n { k } else { period - k };
        self.samples[k] as f64
    }
}

type Key = (PathBuf, u64, Option<SystemTime>);
static CACHE: OnceLock<Mutex<HashMap<Key, Option<Arc<RoomTone>>>>> = OnceLock::new();

/// The room tone of the sound in `path` (a WAV), or `None` when it has no stretch quiet
/// enough and long enough. Found once per file and kept while the app runs.
pub fn of_file(path: &Path) -> Option<Arc<RoomTone>> {
    let meta = std::fs::metadata(path).ok()?;
    let key = (path.to_path_buf(), meta.len(), meta.modified().ok());
    let cache = CACHE.get_or_init(Default::default);
    if let Some(found) = cache.lock().get(&key) {
        return found.clone();
    }
    let found = find(path).ok().flatten().map(Arc::new);
    cache.lock().insert(key, found.clone());
    found
}

fn find(path: &Path) -> Result<Option<RoomTone>, String> {
    let mut reader = PcmReader::open(path)?;
    let (rate, channels) = (reader.info().sample_rate, reader.info().channels as usize);
    let window = (rate as u64 * WINDOW_MS / 1000).max(1) as usize;
    if channels == 0 || rate == 0 {
        return Ok(None);
    }
    let windows = (MAX_SCAN_MS / WINDOW_MS) as usize;
    let mut buf = vec![0f32; window * channels];
    let mut rms = Vec::new();
    while rms.len() < windows {
        let read = reader.read_frames(&mut buf, window)?;
        if read < window {
            break;
        }
        let sum: f64 = buf
            .chunks_exact(channels)
            .map(|frame| {
                let mono = frame.iter().sum::<f32>() / channels as f32;
                (mono * mono) as f64
            })
            .sum();
        rms.push((sum / window as f64).sqrt() as f32);
    }
    // The run whose loudest window is the quietest, never touching digital silence.
    let best = (0..rms.len().saturating_sub(RUN - 1))
        .filter(|&i| rms[i..i + RUN].iter().all(|&r| r >= SILENT_RMS))
        .min_by(|&a, &b| {
            let loudest = |i: usize| rms[i..i + RUN].iter().copied().fold(0f32, f32::max);
            loudest(a).total_cmp(&loudest(b))
        });
    let Some(start) = best.filter(|&i| rms[i..i + RUN].iter().all(|&r| r <= MAX_ROOM_RMS)) else {
        return Ok(None);
    };
    reader.seek_to_frame((start * window) as u64)?;
    let frames = window * RUN;
    let mut raw = vec![0f32; frames * channels];
    let read = reader.read_frames(&mut raw, frames)?;
    let samples = raw[..read * channels]
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect();
    Ok(Some(RoomTone { samples, rate }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_room_is_the_quietest_stretch_that_is_not_silence_and_repeats_smoothly() {
        let dir = tempfile::tempdir().unwrap();
        let rate = 48_000usize;
        // 1 s of speech (loud), 0.5 s of digital silence, 1 s of a faint hum, 1 s of speech.
        let mut samples = Vec::new();
        let tone = |i: usize, level: f64| {
            ((i as f64 * 220.0 * std::f64::consts::TAU / rate as f64).sin() * level) as i16
        };
        samples.extend((0..rate).map(|i| tone(i, 12_000.0)));
        samples.extend(std::iter::repeat_n(0i16, rate / 2));
        samples.extend((0..rate).map(|i| tone(i, 60.0)));
        samples.extend((0..rate).map(|i| tone(i, 12_000.0)));
        let path = dir.path().join("voice.wav");
        std::fs::write(
            &path,
            crate::fixtures::generate_pcm16_wav(rate as u32, 1, &samples),
        )
        .unwrap();
        let room = of_file(&path).unwrap();
        // It is the hum: faint, not silent.
        let loudest = room.samples.iter().fold(0f32, |m, s| m.max(s.abs()));
        assert!(loudest > 0.0005 && loudest < 0.01, "{loudest}");
        assert_eq!(room.samples.len(), rate * 400 / 1000);
        // Back and forth: the sample after the last is the one before it.
        let n = room.samples.len();
        let us = |i: usize| i as f64 * 1e6 / rate as f64;
        assert_eq!(room.at(us(n)), room.samples[n - 2] as f64);
        assert!(std::ptr::eq(
            Arc::as_ptr(&room),
            Arc::as_ptr(&of_file(&path).unwrap())
        ));
    }
}
