//! Non-destructive mouth-click repair. Original PCM remains the analysis/transcript source.
//! Whole source files are repaired once, so seeks, cut order and read sizes cannot change
//! the result. Temporary WAVs are owned by the cache and any mixers still reading them.
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

type Key = (PathBuf, u64, Option<SystemTime>, u8);
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

fn cached(path: &Path, strength: u8) -> Result<Arc<CleanFile>, String> {
    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
    let key = (
        path.to_path_buf(),
        metadata.len(),
        metadata.modified().ok(),
        strength,
    );
    let mut cache = CACHE.get_or_init(Default::default).lock();
    if let Some(index) = cache.entries.iter().position(|(k, _)| *k == key) {
        let entry = cache.entries.remove(index);
        let result = entry.1.clone();
        cache.entries.push(entry);
        return Ok(result);
    }
    let clean = Arc::new(repair(path, strength)?);
    while !cache.entries.is_empty()
        && cache.entries.iter().map(|(_, f)| f.bytes).sum::<u64>() + clean.bytes > CACHE_BYTES
    {
        cache.entries.remove(0);
    }
    cache.entries.push((key, clean.clone()));
    Ok(clean)
}

fn repair(path: &Path, strength: u8) -> Result<CleanFile, String> {
    let original = PcmReader::open(path)?.info().clone();
    let output = tempfile::Builder::new()
        .prefix("aeroedits-mouth-clicks-")
        .suffix(".wav")
        .tempfile()
        .map_err(|e| e.to_string())?
        .into_temp_path();
    // Higher threshold is more conservative. Overlap-save leaves unrepaired samples alone.
    let threshold = 12.0 - f64::from(strength) * 0.10;
    let filter = format!("adeclick=t={threshold:.2}:m=s");
    let mut command = Command::new(super::ffmpeg::ffmpeg_path()?);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let result = command
        .args(["-hide_banner", "-loglevel", "error", "-nostdin", "-y", "-i"])
        .arg(path)
        .args([
            "-map",
            "0:a:0",
            "-filter_threads",
            "2",
            "-af",
            &filter,
            "-c:a",
            "pcm_f32le",
            "-f",
            "wav",
        ])
        .arg(&output)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("Mouth click cleanup could not start: {e}"))?;
    if !result.status.success() {
        return Err(format!(
            "Mouth click cleanup failed: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        ));
    }
    let repaired = PcmReader::open(&output)?.info().clone();
    if original.sample_rate != repaired.sample_rate
        || original.channels != repaired.channels
        || original.frame_count != repaired.frame_count
    {
        return Err(
            "Mouth click cleanup changed audio timing or channels; the original was preserved"
                .into(),
        );
    }
    let bytes = std::fs::metadata(&output).map_err(|e| e.to_string())?.len();
    Ok(CleanFile {
        path: output,
        bytes,
    })
}

pub fn prepare(
    root: &Path,
    settings: &AudioSettings,
    lanes: &[Lane],
) -> Result<CleanFiles, String> {
    settings.validate()?;
    let mut files = CleanFiles::new();
    if !settings.mouth_clicks {
        return Ok(files);
    }
    for lane in lanes.iter().filter(|lane| lane.speech) {
        for clip in &lane.clips {
            for segment in clip.segments.iter().filter(|s| {
                s.available && s.start_us < clip.in_us + clip.len && clip.in_us < s.end_us
            }) {
                let key = (lane.id.clone(), segment.relative_path.clone());
                if files.contains_key(&key) {
                    continue;
                }
                let path = safe_path(root, &segment.relative_path)?;
                let clean = cached(&path, settings.mouth_click_strength)?;
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
        let clean = cached(&path, 75).unwrap();
        assert!(Arc::ptr_eq(&clean, &cached(&path, 75).unwrap()));
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
        let short = cached(&path, 75).unwrap();
        assert!(!Arc::ptr_eq(&clean, &short));
        assert_eq!(
            PcmReader::open(short.path()).unwrap().info().frame_count,
            37
        );
    }
}
