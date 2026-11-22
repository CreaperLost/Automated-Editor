//! Non-destructive speech cleanup: mouth-click repair, the voice chain (SND-7: high-pass,
//! compressor, de-esser) and neural noise removal (SND-5: RNNoise). Original PCM remains the
//! transcript source. Whole source files are cleaned once, so seeks, cut order and read sizes
//! cannot change the result. Temporary WAVs are owned by the cache and any mixers still
//! reading them; loudness and the other analyses read the cleaned copy.
use super::audio::Lane;
use crate::project::{audio::AudioSettings, pcm::PcmReader, reader::safe_path};
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, OnceLock},
    time::SystemTime,
};

/// What is done to a speech file, in this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Recipe {
    /// Mouth-click repair strength (1 to 100).
    pub clicks: Option<u8>,
    /// High-pass, compressor and de-esser.
    pub chain: bool,
    /// RNNoise, pushing noise down by this many tenths of a dB at most.
    pub neural_tenth_db: Option<u32>,
}

impl Recipe {
    /// What `settings` does to `lane`, or `None` for nothing.
    pub fn for_lane(settings: &AudioSettings, lane: &Lane) -> Option<Self> {
        let neural = (settings.noise_model == crate::project::audio::NoiseModel::Neural)
            .then(|| settings.lane_denoise_db(&lane.id, lane.speech))
            .flatten()
            .map(|db| (db * 10.0).round() as u32);
        let recipe = Self {
            clicks: (lane.speech && settings.mouth_clicks).then_some(settings.mouth_click_strength),
            chain: lane.speech && settings.voice_chain,
            neural_tenth_db: neural,
        };
        (recipe.clicks.is_some() || recipe.chain || recipe.neural_tenth_db.is_some())
            .then_some(recipe)
    }

    fn filters(&self) -> Vec<String> {
        let mut filters = Vec::new();
        if let Some(strength) = self.clicks {
            // Higher threshold is more conservative. Overlap-save leaves unrepaired samples
            // alone.
            let threshold = 12.0 - f64::from(strength) * 0.10;
            filters.push(format!("adeclick=t={threshold:.2}:m=s"));
        }
        if self.chain {
            filters.extend(VOICE_CHAIN.iter().map(|f| f.to_string()));
        }
        filters
    }
}

/// Rumble under 80 Hz goes; peaks over -20 dBFS are pressed 3:1 (the leveller and
/// loudness normalization still set the level afterwards); sibilance is softened.
const VOICE_CHAIN: [&str; 3] = [
    "highpass=f=80:poles=2",
    "acompressor=threshold=0.1:ratio=3:attack=8:release=120:knee=2.8",
    "deesser=i=0.4:m=0.5:f=0.5",
];
/// RNNoise works on 48 kHz audio in 10 ms frames and answers one frame late.
const NEURAL_RATE: u32 = 48_000;

type Key = (PathBuf, u64, Option<SystemTime>, Recipe);
pub type CleanFiles = HashMap<(String, String), Arc<CleanFile>>;
pub struct CleanFile {
    path: tempfile::TempPath,
    bytes: u64,
}
impl CleanFile {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

// Serializes preparation to avoid duplicate expensive jobs on playback/export threads.
// Eviction drops cache ownership only; active mixers retain their temporary files.
#[derive(Default)]
struct Cache {
    entries: Vec<(Key, Arc<CleanFile>)>,
}
const CACHE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();

/// Called only on application exit. Static caches are not dropped automatically by Rust.
pub fn shutdown() {
    if let Some(cache) = CACHE.get() {
        for (_, clean) in cache.lock().entries.drain(..) {
            let _ = std::fs::remove_file(clean.path());
        }
    }
}

fn cached(path: &Path, recipe: Recipe) -> Result<Arc<CleanFile>, String> {
    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    let key = (
        path.to_path_buf(),
        metadata.len(),
        metadata.modified().ok(),
        recipe,
    );
    let mut cache = CACHE.get_or_init(Default::default).lock();
    if let Some(index) = cache.entries.iter().position(|(k, _)| *k == key) {
        let entry = cache.entries.remove(index);
        let result = entry.1.clone();
        cache.entries.push(entry);
        return Ok(result);
    }
    let clean = Arc::new(clean(path, recipe)?);
    while !cache.entries.is_empty()
        && cache.entries.iter().map(|(_, f)| f.bytes).sum::<u64>() + clean.bytes > CACHE_BYTES
    {
        cache.entries.remove(0);
    }
    cache.entries.push((key, clean.clone()));
    Ok(clean)
}

fn temp_wav(prefix: &str) -> Result<tempfile::TempPath, String> {
    Ok(tempfile::Builder::new()
        .prefix(prefix)
        .suffix(".wav")
        .tempfile()
        .map_err(|e| e.to_string())?
        .into_temp_path())
}

/// FFmpeg writes `input`'s first sound stream to `output` as 32-bit float WAV, through
/// `filters`, at `rate` when given.
fn ffmpeg_wav(
    input: &Path,
    output: &Path,
    filters: &[String],
    rate: Option<u32>,
) -> Result<(), String> {
    let mut command = Command::new(super::ffmpeg::ffmpeg_path()?);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
        .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y", "-i"])
        .arg(input)
        .args(["-map", "0:a:0", "-filter_threads", "2"]);
    let mut chain = filters.to_vec();
    if let Some(rate) = rate {
        chain.push(format!("aresample={rate}"));
    }
    if !chain.is_empty() {
        command.args(["-af", &chain.join(",")]);
    }
    let result = command
        .args(["-c:a", "pcm_f32le", "-f", "wav"])
        .arg(output)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("Voice cleanup could not start: {e}"))?;
    if !result.status.success() {
        return Err(format!(
            "Voice cleanup failed: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        ));
    }
    Ok(())
}

fn clean(path: &Path, recipe: Recipe) -> Result<CleanFile, String> {
    let original = PcmReader::open(path)?.info().clone();
    let output = temp_wav("aeroedits-voice-")?;
    match recipe.neural_tenth_db {
        None => ffmpeg_wav(path, &output, &recipe.filters(), None)?,
        Some(tenths) => {
            let staged = temp_wav("aeroedits-voice-in-")?;
            ffmpeg_wav(path, &staged, &recipe.filters(), Some(NEURAL_RATE))?;
            let denoised = if original.sample_rate == NEURAL_RATE {
                output.to_path_buf()
            } else {
                temp_wav("aeroedits-voice-48k-")?
                    .keep()
                    .map_err(|e| e.to_string())?
            };
            let result = neural(&staged, &denoised, tenths as f32 / 10.0).and_then(|()| {
                if original.sample_rate == NEURAL_RATE {
                    return Ok(());
                }
                // Back at the recording's own rate, frame for frame.
                let frames = original.frame_count;
                ffmpeg_wav(
                    &denoised,
                    &output,
                    &[
                        format!("aresample={}", original.sample_rate),
                        format!("apad=whole_len={frames}"),
                        format!("atrim=end_sample={frames}"),
                    ],
                    None,
                )
            });
            if denoised != output.to_path_buf() {
                let _ = std::fs::remove_file(&denoised);
            }
            result?;
        }
    }
    let cleaned = PcmReader::open(&output)?.info().clone();
    if original.sample_rate != cleaned.sample_rate
        || original.channels != cleaned.channels
        || original.frame_count != cleaned.frame_count
    {
        return Err(
            "Voice cleanup changed audio timing or channels; the original was preserved".into(),
        );
    }
    let bytes = std::fs::metadata(&output).map_err(|e| e.to_string())?.len();
    Ok(CleanFile {
        path: output,
        bytes,
    })
}

/// RNNoise on each channel of 48 kHz `input`, written to `output` with as many frames. The
/// result is mixed back with the input so noise falls by `reduction_db` at most.
fn neural(input: &Path, output: &Path, reduction_db: f32) -> Result<(), String> {
    use nnnoiseless::DenoiseState;
    const FRAME: usize = DenoiseState::FRAME_SIZE;
    let mut reader = PcmReader::open(input)?;
    let info = reader.info().clone();
    let channels = info.channels as usize;
    let frames = info.frame_count as usize;
    let mut samples = vec![0f32; frames * channels];
    let got = reader.read_frames(&mut samples, frames)?;
    samples.truncate(got * channels);
    let keep = 10f32.powf(-reduction_db / 20.0);
    let mut out = vec![0f32; samples.len()];
    for c in 0..channels {
        // One frame of silence after the end, so the last real frame comes out too.
        let mut mono: Vec<f32> = samples
            .iter()
            .skip(c)
            .step_by(channels)
            .map(|s| s * 32_768.0)
            .collect();
        mono.resize((got.div_ceil(FRAME) + 1) * FRAME, 0.0);
        let mut state = DenoiseState::new();
        let mut denoised = vec![0f32; mono.len()];
        for (from, to) in mono
            .chunks_exact(FRAME)
            .zip(denoised.chunks_exact_mut(FRAME))
        {
            state.process_frame(to, from);
        }
        for i in 0..got {
            let dry = samples[i * channels + c];
            let wet = denoised[i + FRAME] / 32_768.0;
            out[i * channels + c] = wet + (dry - wet) * keep;
        }
    }
    write_f32_wav(output, info.sample_rate, info.channels, &out)
}

fn write_f32_wav(path: &Path, rate: u32, channels: u16, samples: &[f32]) -> Result<(), String> {
    use std::io::Write;
    let data = (samples.len() * 4) as u32;
    let mut bytes = Vec::with_capacity(44 + data as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&3u16.to_le_bytes()); // IEEE float
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * channels as u32 * 4).to_le_bytes());
    bytes.extend_from_slice(&(channels * 4).to_le_bytes());
    bytes.extend_from_slice(&32u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data.to_le_bytes());
    for s in samples {
        bytes.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::File::create(path)
        .and_then(|mut f| f.write_all(&bytes))
        .map_err(|e| format!("{}: {e}", path.display()))
}

pub fn prepare(
    root: &Path,
    settings: &AudioSettings,
    lanes: &[Lane],
) -> Result<CleanFiles, String> {
    settings.validate()?;
    let mut files = CleanFiles::new();
    for lane in lanes {
        let Some(recipe) = Recipe::for_lane(settings, lane) else {
            continue;
        };
        for clip in &lane.clips {
            for segment in clip.segments.iter().filter(|s| {
                s.available && s.start_us < clip.in_us + clip.len && clip.in_us < s.end_us
            }) {
                let key = (lane.id.clone(), segment.relative_path.clone());
                if files.contains_key(&key) {
                    continue;
                }
                let path = safe_path(root, &segment.relative_path)?;
                let clean = cached(&path, recipe)?;
                files.insert(key, clean);
            }
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouth_click_repair_preserves_stereo_and_partial_tail() {
        if super::super::ffmpeg::ffmpeg_path().is_err() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("stereo.wav");
        // Non-window-aligned length, an impulse on the left, clean silence on the right.
        let mut samples = vec![0i16; 48_123 * 2];
        samples[24_000 * 2] = 25_000;
        std::fs::write(
            &path,
            crate::fixtures::generate_pcm16_wav(48_000, 2, &samples),
        )
        .unwrap();
        let clicks = Recipe {
            clicks: Some(75),
            chain: false,
            neural_tenth_db: None,
        };
        let clean = cached(&path, clicks).unwrap();
        assert!(Arc::ptr_eq(&clean, &cached(&path, clicks).unwrap()));
        let mut reader = PcmReader::open(clean.path()).unwrap();
        assert_eq!(reader.info().frame_count, 48_123);
        assert_eq!(reader.info().channels, 2);
        let mut output = vec![0f32; samples.len()];
        assert_eq!(reader.read_frames(&mut output, 48_123).unwrap(), 48_123);
        assert!(output.chunks_exact(2).all(|frame| frame[1] == 0.0));
        assert!(output[48_000].abs() < 0.1);
        // Tiny files cannot fill the filter's analysis window; they still keep every frame.
        std::fs::write(
            &path,
            crate::fixtures::generate_pcm16_wav(48_000, 2, &samples[..74]),
        )
        .unwrap();
        let short = cached(&path, clicks).unwrap();
        assert!(!Arc::ptr_eq(&clean, &short));
        assert_eq!(
            PcmReader::open(short.path()).unwrap().info().frame_count,
            37
        );
    }

    /// Hiss everywhere, a tone burst in the middle, at `rate`.
    fn noisy(path: &Path, rate: u32, channels: u16, frames: usize) {
        let mut seed = 12345u32;
        let mut samples = Vec::with_capacity(frames * channels as usize);
        for i in 0..frames {
            let t = i as f32 / rate as f32;
            let tone = if (0.4..0.7).contains(&t) {
                (t * 2.0 * std::f32::consts::PI * 220.0).sin() * 8_000.0
            } else {
                0.0
            };
            for _ in 0..channels {
                seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                let hiss = ((seed >> 16) as f32 / 32_768.0 - 1.0) * 1_500.0;
                samples.push((tone + hiss) as i16);
            }
        }
        std::fs::write(
            path,
            crate::fixtures::generate_pcm16_wav(rate, channels, &samples),
        )
        .unwrap();
    }

    fn rms(path: &Path, from: f32, to: f32) -> f32 {
        let mut reader = PcmReader::open(path).unwrap();
        let info = reader.info().clone();
        let mut all = vec![0f32; info.frame_count as usize * info.channels as usize];
        let got = reader
            .read_frames(&mut all, info.frame_count as usize)
            .unwrap();
        let c = info.channels as usize;
        let (a, b) = (
            (from * info.sample_rate as f32) as usize * c,
            ((to * info.sample_rate as f32) as usize * c).min(got * c),
        );
        (all[a..b].iter().map(|s| s * s).sum::<f32>() / (b - a) as f32).sqrt()
    }

    #[test]
    fn neural_noise_removal_takes_hiss_out_and_keeps_every_frame() {
        if super::super::ffmpeg::ffmpeg_path().is_err() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        for (rate, channels, frames) in [(48_000, 2, 48_123), (44_100, 1, 44_100)] {
            let path = dir.path().join(format!("hiss-{rate}.wav"));
            noisy(&path, rate, channels, frames);
            let recipe = Recipe {
                clicks: None,
                chain: false,
                neural_tenth_db: Some(300),
            };
            let clean = cached(&path, recipe).unwrap();
            let info = PcmReader::open(clean.path()).unwrap().info().clone();
            assert_eq!(
                (info.sample_rate, info.channels, info.frame_count),
                (rate, channels, frames as u64)
            );
            let hiss_before = rms(&path, 0.15, 0.35);
            let hiss_after = rms(clean.path(), 0.15, 0.35);
            assert!(
                hiss_after < hiss_before * 0.5,
                "{rate}: hiss {hiss_before} -> {hiss_after}"
            );
            // The tone stays, in place (RNNoise's frame of delay is taken back).
            let tone_before = rms(&path, 0.45, 0.65);
            let tone_after = rms(clean.path(), 0.45, 0.65);
            assert!(
                tone_after > tone_before * 0.5,
                "{rate}: tone {tone_before} -> {tone_after}"
            );
            assert!(
                rms(clean.path(), 0.39, 0.4) < tone_after * 0.5,
                "{rate}: tone came early"
            );
        }
    }

    #[test]
    fn the_voice_chain_takes_out_rumble_and_keeps_the_voice() {
        if super::super::ffmpeg::ffmpeg_path().is_err() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rumble.wav");
        let samples: Vec<i16> = (0..48_000)
            .map(|i| {
                ((i as f32 / 48_000.0 * 2.0 * std::f32::consts::PI * 30.0).sin() * 8_000.0) as i16
            })
            .collect();
        std::fs::write(
            &path,
            crate::fixtures::generate_pcm16_wav(48_000, 1, &samples),
        )
        .unwrap();
        let chain = Recipe {
            clicks: None,
            chain: true,
            neural_tenth_db: None,
        };
        let clean = cached(&path, chain).unwrap();
        assert_eq!(
            PcmReader::open(clean.path()).unwrap().info().frame_count,
            48_000
        );
        assert!(rms(clean.path(), 0.2, 0.9) < rms(&path, 0.2, 0.9) * 0.3);
        let settings = AudioSettings {
            voice_chain: true,
            ..Default::default()
        };
        let lane = |speech| Lane {
            id: "a1".into(),
            speech,
            music: false,
            gain: 1.0,
            clips: Vec::new(),
        };
        assert_eq!(Recipe::for_lane(&settings, &lane(true)), Some(chain));
        assert_eq!(
            Recipe::for_lane(&settings, &lane(false)),
            None,
            "speech only"
        );
    }
}
