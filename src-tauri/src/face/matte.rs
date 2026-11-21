//! Camera mattes (FX-1): where the person is in each moment of a camera stream, so the
//! background behind them can be blurred, removed or replaced. MediaPipe's selfie
//! segmentation (Apache-2.0, 450 KB) runs through ONNX Runtime on the CPU, ten times a second
//! of the camera, once, in the background; the mattes are kept next to `project.json` as
//! `mattes/<asset>.<stream>.bin`, in the asset's own time like face tracks, so they hold through
//! any edit. Each matte is 256 × 256 over the whole picture (stretched, as the model sees it),
//! smoothed over time against flicker and stored deflated: about 30 MB for ten minutes.
use crate::sequence::Asset;
use parking_lot::Mutex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub const MODEL_NAME: &str = "mediapipe_selfie_segmentation";
const MODEL_URL: &str = "https://huggingface.co/onnx-community/mediapipe_selfie_segmentation/resolve/main/onnx/model.onnx";
const MODEL_FILE: &str = "selfie_segmentation.onnx";
const MODEL_BYTES: u64 = 462_352;
const MODEL_SHA256: &str = "3241ac4ad8aa35bdaf33946776db29f7c283a413aa0b0dacb9483594b4531aad";
/// The model's square input and output.
pub const SIZE: usize = 256;
/// Mattes a second of camera.
pub const FPS: u32 = 10;
pub const MATTES_DIR: &str = "mattes";
const MAGIC: &[u8; 8] = b"AEMATTE1";
/// How much of the previous matte each one keeps, against flicker.
const SMOOTHING: f32 = 0.35;
const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;

pub fn default_model_dir() -> PathBuf {
    crate::transcript::parakeet::sibling_model_dir(MODEL_NAME)
}

pub fn model_path(dir: &Path) -> PathBuf {
    dir.join(MODEL_FILE)
}

pub fn model_present(dir: &Path) -> bool {
    std::fs::metadata(model_path(dir)).is_ok_and(|m| m.len() == MODEL_BYTES)
}

/// Segmentation runs with ONNX Runtime, which comes with the `parakeet` feature.
pub fn compiled_in() -> bool {
    cfg!(feature = "parakeet")
}

/// Downloads the model into `dir`, checked against its known hash.
pub fn download_model(dir: &Path, cancel: &AtomicBool) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let mut response = client
        .get(MODEL_URL)
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("Could not download the person matte model: {e}"))?;
    let mut bytes = Vec::with_capacity(MODEL_BYTES as usize);
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("Download cancelled".into());
        }
        let n = response
            .read(&mut buf)
            .map_err(|e| format!("The matte model download was interrupted: {e}"))?;
        if n == 0 || bytes.len() as u64 > MODEL_BYTES {
            break;
        }
        bytes.extend_from_slice(&buf[..n]);
    }
    let hash: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if bytes.len() as u64 != MODEL_BYTES || hash != MODEL_SHA256 {
        return Err("The matte model download is not the expected file".into());
    }
    crate::files::write_durable(&model_path(dir), &bytes)
}

/// A packed RGB picture of [`SIZE`] × [`SIZE`] as the model's NCHW input, 0 to 1.
pub fn input_tensor(rgb: &[u8]) -> Vec<f32> {
    let plane = SIZE * SIZE;
    let mut data = vec![0f32; 3 * plane];
    for (i, px) in rgb.chunks_exact(3).take(plane).enumerate() {
        for c in 0..3 {
            data[c * plane + i] = px[c] as f32 / 255.0;
        }
    }
    data
}

/// The model's 0-to-1 output as bytes.
pub fn to_bytes(alphas: &[f32]) -> Vec<u8> {
    alphas
        .iter()
        .map(|a| (a.clamp(0.0, 1.0) * 255.0).round() as u8)
        .collect()
}

/// `next` with [`SMOOTHING`] of `previous` kept.
pub fn smooth(previous: &[u8], next: &mut [u8]) {
    for (n, p) in next.iter_mut().zip(previous) {
        *n = (*n as f32 * (1.0 - SMOOTHING) + *p as f32 * SMOOTHING).round() as u8;
    }
}

/// Something that finds the person in a picture; tests use a stand-in.
pub trait MatteModel: Send {
    /// A [`SIZE`]² packed RGB picture → [`SIZE`]² bytes, 255 the person.
    fn matte(&mut self, rgb: &[u8]) -> Result<Vec<u8>, String>;
}

#[cfg(feature = "parakeet")]
mod engine {
    use super::{input_tensor, to_bytes, MatteModel, SIZE};
    use std::path::Path;

    pub struct Selfie {
        session: ort::session::Session,
    }

    impl Selfie {
        pub fn new(dir: &Path) -> Result<Self, String> {
            if !super::model_present(dir) {
                return Err("The person matte model is not downloaded yet".into());
            }
            // The CPU only, on two threads: it runs in the background beside playback.
            let session = parakeet_rs::ExecutionConfig::new()
                .with_intra_threads(2)
                .with_inter_threads(1)
                .build_session(&super::model_path(dir))
                .map_err(|e| format!("Could not load the person matte model: {e}"))?;
            Ok(Self { session })
        }
    }

    impl MatteModel for Selfie {
        fn matte(&mut self, rgb: &[u8]) -> Result<Vec<u8>, String> {
            let tensor =
                ort::value::Tensor::from_array(([1usize, 3, SIZE, SIZE], input_tensor(rgb)))
                    .map_err(|e| e.to_string())?;
            let outputs = self
                .session
                .run(ort::inputs!["pixel_values" => tensor])
                .map_err(|e| format!("Finding the person failed: {e}"))?;
            let (_, alphas) = outputs["alphas"]
                .try_extract_tensor::<f32>()
                .map_err(|e| e.to_string())?;
            Ok(to_bytes(alphas))
        }
    }
}

/// The model in `dir`, ready to run.
pub fn load(dir: &Path) -> Result<Box<dyn MatteModel>, String> {
    #[cfg(feature = "parakeet")]
    {
        Ok(Box::new(engine::Selfie::new(dir)?))
    }
    #[cfg(not(feature = "parakeet"))]
    {
        let _ = dir;
        Err("This build of AeroEdits does not include person mattes".into())
    }
}

/// A camera stream's mattes, deflated, by time on the asset's clock.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MatteTrack {
    pub asset: String,
    pub stream: String,
    /// What it was made from (the asset's path and length).
    pub source: String,
    /// (time, deflated [`SIZE`]² bytes), by time.
    pub frames: Vec<(u64, Vec<u8>)>,
}

fn deflate(bytes: &[u8]) -> Vec<u8> {
    let mut e = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::fast());
    e.write_all(bytes).expect("writing to memory");
    e.finish().expect("writing to memory")
}

fn inflate(bytes: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(SIZE * SIZE);
    flate2::read::DeflateDecoder::new(bytes)
        .take((SIZE * SIZE) as u64 + 1)
        .read_to_end(&mut out)
        .ok()?;
    (out.len() == SIZE * SIZE).then_some(out)
}

impl MatteTrack {
    pub fn push(&mut self, t_us: u64, matte: &[u8]) {
        self.frames.push((t_us, deflate(matte)));
    }

    /// The matte for `t_us`: the nearest one within a frame's time, else none (time the
    /// camera did not record).
    pub fn at(&self, t_us: u64) -> Option<Vec<u8>> {
        let i = self.frames.partition_point(|(t, _)| *t <= t_us);
        let near = [i.checked_sub(1), (i < self.frames.len()).then_some(i)]
            .into_iter()
            .flatten()
            .min_by_key(|&j| self.frames[j].0.abs_diff(t_us))?;
        let (t, bytes) = &self.frames[near];
        (t.abs_diff(t_us) <= 1_000_000 / FPS as u64).then(|| inflate(bytes))?
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = MAGIC.to_vec();
        let text = |out: &mut Vec<u8>, s: &str| {
            out.extend((s.len() as u32).to_le_bytes());
            out.extend(s.as_bytes());
        };
        text(&mut out, &self.asset);
        text(&mut out, &self.stream);
        text(&mut out, &self.source);
        out.extend((self.frames.len() as u32).to_le_bytes());
        for (t, bytes) in &self.frames {
            out.extend(t.to_le_bytes());
            out.extend((bytes.len() as u32).to_le_bytes());
            out.extend(bytes);
        }
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.get(..8)? != MAGIC {
            return None;
        }
        let mut pos = 8usize;
        let read = |pos: &mut usize, n: usize| -> Option<&[u8]> {
            let part = bytes.get(*pos..pos.checked_add(n)?)?;
            *pos += n;
            Some(part)
        };
        let read_u32 = |pos: &mut usize| -> Option<u32> {
            Some(u32::from_le_bytes(read(pos, 4)?.try_into().ok()?))
        };
        let read_text = |pos: &mut usize| -> Option<String> {
            let n = read_u32(pos)? as usize;
            String::from_utf8(read(pos, n)?.to_vec()).ok()
        };
        let asset = read_text(&mut pos)?;
        let stream = read_text(&mut pos)?;
        let source = read_text(&mut pos)?;
        let count = read_u32(&mut pos)? as usize;
        let mut frames = Vec::with_capacity(count.min(1 << 20));
        for _ in 0..count {
            let t = u64::from_le_bytes(read(&mut pos, 8)?.try_into().ok()?);
            let n = read_u32(&mut pos)? as usize;
            frames.push((t, read(&mut pos, n)?.to_vec()));
        }
        (pos == bytes.len()).then_some(Self {
            asset,
            stream,
            source,
            frames,
        })
    }
}

pub fn track_path(root: &Path, asset: &str, stream: &str) -> Result<PathBuf, String> {
    if !crate::ids::is_plain_id(asset) || !crate::ids::is_plain_id(stream) {
        return Err("Invalid camera stream for a matte".into());
    }
    crate::project::reader::safe_path(root, &format!("{MATTES_DIR}/{asset}.{stream}.bin"))
}

pub fn load_track(root: &Path, asset: &str, stream: &str) -> Result<Option<MatteTrack>, String> {
    let path = track_path(root, asset, stream)?;
    if !path.is_file() {
        return Ok(None);
    }
    let file = crate::project::reader::open_regular(&path)?;
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err("The camera matte file is too large".into());
    }
    Ok(MatteTrack::from_bytes(&bytes))
}

pub fn save_track(root: &Path, track: &MatteTrack) -> Result<(), String> {
    let path = track_path(root, &track.asset, &track.stream)?;
    let dir = path.parent().ok_or("No folder for mattes")?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    crate::files::write_atomic(&path, &track.to_bytes())
}

type Stamp = (u64, Option<std::time::SystemTime>);
static TRACKS: Mutex<Option<HashMap<PathBuf, (Stamp, Option<Arc<MatteTrack>>)>>> = Mutex::new(None);

/// A camera stream's mattes, read once while the file stays the same.
pub fn cached_track(root: &Path, asset: &str, stream: &str) -> Option<Arc<MatteTrack>> {
    let path = track_path(root, asset, stream).ok()?;
    let meta = std::fs::metadata(&path).ok()?;
    let stamp = (meta.len(), meta.modified().ok());
    let mut cache = TRACKS.lock();
    let cache = cache.get_or_insert_with(Default::default);
    if let Some((seen, built)) = cache.get(&path) {
        if *seen == stamp {
            return built.clone();
        }
    }
    let built = load_track(root, asset, stream).ok().flatten().map(Arc::new);
    // Only the open project's few cameras are kept.
    cache.retain(|other, _| other.parent() == path.parent());
    cache.insert(path, (stamp, built.clone()));
    built
}

fn source_of(asset: &Asset) -> String {
    format!("{}|{}", asset.path, asset.duration_us)
}

/// Whether `track` was made from `asset` as it is.
pub fn is_current(track: &MatteTrack, asset: &Asset) -> bool {
    track.source == source_of(asset)
}

/// Finds the person [`FPS`] times a second through a camera stream.
pub fn compute(
    root: &Path,
    asset: &Asset,
    stream: &str,
    model: &mut dyn MatteModel,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(f32),
) -> Result<MatteTrack, String> {
    let pieces = super::pieces(root, asset, stream)?;
    let total: u64 = pieces.iter().map(|(_, a, b)| b - a).sum::<u64>().max(1);
    let step = 1_000_000 / FPS as u64;
    let mut track = MatteTrack {
        asset: asset.id.clone(),
        stream: stream.into(),
        source: source_of(asset),
        frames: Vec::new(),
    };
    let mut done = 0u64;
    for (path, start_us, end_us) in &pieces {
        let mut previous: Option<Vec<u8>> = None;
        let mut failure = None;
        crate::media::ffmpeg::sample_rgb_fps(
            path,
            FPS,
            SIZE as u32,
            SIZE as u32,
            &mut |index, rgb| {
                let t_us = start_us + index as u64 * step;
                if t_us >= *end_us || cancel.load(Ordering::Relaxed) {
                    return false;
                }
                match model.matte(rgb) {
                    Ok(mut matte) => {
                        if let Some(p) = &previous {
                            smooth(p, &mut matte);
                        }
                        track.push(t_us, &matte);
                        previous = Some(matte);
                    }
                    Err(e) => {
                        failure = Some(e);
                        return false;
                    }
                }
                progress(((done + (t_us - start_us)) as f32 / total as f32).min(1.0));
                true
            },
        )?;
        if let Some(e) = failure {
            return Err(e);
        }
        if cancel.load(Ordering::Relaxed) {
            return Err("Finding the person was cancelled".into());
        }
        done += end_us - start_us;
    }
    track.frames.sort_by_key(|(t, _)| *t);
    Ok(track)
}

/// Where a camera's mattes stand: `ready`, `working` (with progress), `missing`, `failed` or
/// `unavailable` (a build without ONNX Runtime).
#[derive(Clone, Debug, PartialEq)]
pub struct MatteStatus {
    pub state: &'static str,
    pub progress: f32,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default)]
struct Job {
    progress: f32,
    running: bool,
    error: Option<String>,
}

static JOBS: Mutex<Option<HashMap<PathBuf, Job>>> = Mutex::new(None);

fn with_jobs<T>(f: impl FnOnce(&mut HashMap<PathBuf, Job>) -> T) -> T {
    f(JOBS.lock().get_or_insert_with(Default::default))
}

/// Where `asset`'s `stream` mattes stand.
pub fn status(root: &Path, asset: &Asset, stream: &str) -> MatteStatus {
    let Ok(path) = track_path(root, &asset.id, stream) else {
        return MatteStatus {
            state: "failed",
            progress: 0.0,
            error: Some("Invalid camera stream".into()),
        };
    };
    let job = with_jobs(|jobs| jobs.get(&path).cloned());
    let ready = cached_track(root, &asset.id, stream).is_some_and(|t| is_current(&t, asset));
    let (state, error) = match &job {
        Some(j) if j.running => ("working", None),
        _ if ready => ("ready", None),
        Some(Job { error: Some(e), .. }) => ("failed", Some(e.clone())),
        _ if !compiled_in() => ("unavailable", None),
        _ => ("missing", None),
    };
    MatteStatus {
        state,
        progress: job.filter(|j| j.running).map_or(0.0, |j| j.progress),
        error,
    }
}

/// Starts finding the person in `asset`'s `stream` on a background thread, unless its mattes
/// are ready or being found (or failed this session, unless `retry`).
pub fn ensure(root: &Path, asset: &Asset, stream: &str, retry: bool) {
    let now = status(root, asset, stream);
    if !(now.state == "missing" || (retry && now.state == "failed")) {
        return;
    }
    let Ok(path) = track_path(root, &asset.id, stream) else {
        return;
    };
    let claimed = with_jobs(|jobs| {
        if jobs.get(&path).is_some_and(|j| j.running) {
            return false;
        }
        jobs.insert(
            path.clone(),
            Job {
                running: true,
                ..Job::default()
            },
        );
        true
    });
    if !claimed {
        return;
    }
    let (root, asset, stream) = (root.to_path_buf(), asset.clone(), stream.to_string());
    std::thread::Builder::new()
        .name("camera-matte".into())
        .spawn(move || {
            let cancel = AtomicBool::new(false);
            let result = (|| {
                let dir = default_model_dir();
                if !model_present(&dir) {
                    download_model(&dir, &cancel)?;
                }
                let mut model = load(&dir)?;
                let track = compute(&root, &asset, &stream, model.as_mut(), &cancel, &mut |p| {
                    with_jobs(|jobs| {
                        if let Some(job) = jobs.get_mut(&path) {
                            job.progress = p;
                        }
                    })
                })?;
                save_track(&root, &track)
            })();
            with_jobs(|jobs| {
                jobs.insert(
                    path,
                    Job {
                        progress: 1.0,
                        running: false,
                        error: result.err(),
                    },
                )
            });
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Says the left half of every picture is the person.
    struct LeftHalf;

    impl MatteModel for LeftHalf {
        fn matte(&mut self, rgb: &[u8]) -> Result<Vec<u8>, String> {
            assert_eq!(rgb.len(), SIZE * SIZE * 3);
            Ok((0..SIZE * SIZE)
                .map(|i| if i % SIZE < SIZE / 2 { 255 } else { 0 })
                .collect())
        }
    }

    #[test]
    fn mattes_round_trip_and_are_found_by_the_nearest_time() {
        let mut track = MatteTrack {
            asset: "rec".into(),
            stream: "webcam".into(),
            source: "a|1".into(),
            frames: Vec::new(),
        };
        let full = vec![255u8; SIZE * SIZE];
        let empty = vec![0u8; SIZE * SIZE];
        track.push(0, &full);
        track.push(100_000, &empty);
        let back = MatteTrack::from_bytes(&track.to_bytes()).unwrap();
        assert_eq!(back, track);
        assert_eq!(back.at(40_000).unwrap(), full);
        assert_eq!(back.at(60_000).unwrap(), empty);
        assert!(back.at(5_000_000).is_none(), "far from any matte");
        assert!(MatteTrack::from_bytes(b"AEMATTE1\x01").is_none());
        let mut cut = track.to_bytes();
        cut.pop();
        assert!(MatteTrack::from_bytes(&cut).is_none());
    }

    #[test]
    fn inputs_scale_to_one_and_mattes_smooth_over_time() {
        let rgb: Vec<u8> = (0..SIZE * SIZE).flat_map(|_| [255u8, 0, 51]).collect();
        let input = input_tensor(&rgb);
        let plane = SIZE * SIZE;
        assert_eq!((input[0], input[plane], input[2 * plane]), (1.0, 0.0, 0.2));
        assert_eq!(to_bytes(&[0.0, 0.5, 1.2]), vec![0, 128, 255]);
        let mut next = vec![255u8; 4];
        smooth(&[0u8; 4], &mut next);
        assert_eq!(next[0], 166);
    }

    #[test]
    fn a_camera_gets_ten_mattes_a_second() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let clip = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../test/fixtures/clip.mp4")
            .canonicalize()
            .unwrap();
        let asset = crate::media_bin::import(dir.path(), &clip).unwrap();
        let stream = asset
            .streams
            .iter()
            .find(|s| s.kind == crate::sequence::StreamKind::Picture)
            .unwrap()
            .id
            .clone();
        let cancel = AtomicBool::new(false);
        let track = compute(
            dir.path(),
            &asset,
            &stream,
            &mut LeftHalf,
            &cancel,
            &mut |_| {},
        )
        .unwrap();
        let seconds = asset.duration_us as f64 / 1e6;
        let expected = (seconds * FPS as f64).round() as i64;
        assert!(
            (track.frames.len() as i64 - expected).abs() <= 2,
            "{} mattes for {seconds} s",
            track.frames.len()
        );
        save_track(dir.path(), &track).unwrap();
        let cached = cached_track(dir.path(), &asset.id, &stream).unwrap();
        assert!(is_current(&cached, &asset));
        let matte = cached.at(1_000_000).unwrap();
        assert_eq!((matte[0], matte[SIZE - 1]), (255, 0));
        assert_eq!(status(dir.path(), &asset, &stream).state, "ready");
    }

    /// The real model on a picture of a person, run by hand:
    /// `AEROEDITS_MATTE_PROBE=<video> cargo test --lib --features parakeet matte_probe -- --ignored --nocapture`
    /// (downloads the model on first use). Writes the middle matte beside the video as a PGM.
    #[cfg(feature = "parakeet")]
    #[test]
    #[ignore]
    fn matte_probe() {
        let video = PathBuf::from(std::env::var("AEROEDITS_MATTE_PROBE").unwrap());
        let dir = tempfile::tempdir().unwrap();
        let asset = crate::media_bin::import(dir.path(), &video).unwrap();
        let stream = asset.streams[0].id.clone();
        let cancel = AtomicBool::new(false);
        let models = default_model_dir();
        if !model_present(&models) {
            download_model(&models, &cancel).unwrap();
        }
        let mut model = load(&models).unwrap();
        let started = std::time::Instant::now();
        let track = compute(
            dir.path(),
            &asset,
            &stream,
            model.as_mut(),
            &cancel,
            &mut |_| {},
        )
        .unwrap();
        let bytes = track.to_bytes().len();
        println!(
            "{} mattes in {:?}, {bytes} bytes",
            track.frames.len(),
            started.elapsed()
        );
        let middle = track.at(asset.duration_us / 2).unwrap();
        let share = middle.iter().filter(|&&a| a > 127).count() as f32 / middle.len() as f32;
        println!("person covers {:.0}% of the middle frame", share * 100.0);
        let mut pgm = format!("P5 {SIZE} {SIZE} 255\n").into_bytes();
        pgm.extend(&middle);
        std::fs::write(video.with_extension("matte.pgm"), pgm).unwrap();
    }
}
