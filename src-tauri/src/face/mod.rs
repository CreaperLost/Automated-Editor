//! Face tracks: where the speaker's face is in a camera stream, found once in the background
//! and kept next to `project.json` as `faces/<asset>.<stream>.json`. Like transcripts they are
//! analysis results, not edits, so they sit outside the undo history. They are in the camera
//! asset's own time, so they hold through any edit of the video or of a short.
//!
//! Shorts frame their camera on the face: [`align`] lines each clip up with it as the clip
//! starts, and `render::focus_uv` cuts the crop around that. Only streams whose role is the
//! camera get a track: a recording without a camera (gaming footage, say) has none and keeps
//! its framing.
pub mod align;
pub mod yunet;

use crate::project::revision::EditDocument;
use crate::sequence::sources::StreamSource;
use crate::sequence::{Asset, Role, StreamKind};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub use align::{clip_focus, FocusPoint};

pub const FACES_DIR: &str = "faces";
/// Bumped when detection changes enough that old tracks should be found again.
pub const TRACK_VERSION: u32 = 2;
/// The camera is looked at once every five seconds: each clip is lined up once (see
/// [`align`]), so a few looks per clip are plenty.
pub const SAMPLE_US: u64 = 5_000_000;
const SAMPLE_EVERY_S: u32 = 5;
/// Pictures are scaled to this width for the detector: plenty for a face that fills a webcam.
const DETECT_WIDTH: u32 = 640;
const MAX_TRACK_BYTES: u64 = 32 * 1024 * 1024;

/// A face, as shares of the camera picture: its box, the height of its eye line and how sure
/// the detector was.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FaceBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub eye_y: f32,
    pub score: f32,
}

impl FaceBox {
    pub fn cx(&self) -> f32 {
        self.x + self.w / 2.0
    }

    /// Four decimals: plenty for framing, and a third of the file size.
    fn rounded(self) -> Self {
        let r = |v: f32| (v * 10_000.0).round() / 10_000.0;
        Self {
            x: r(self.x),
            y: r(self.y),
            w: r(self.w),
            h: r(self.h),
            eye_y: r(self.eye_y),
            score: r(self.score),
        }
    }
}

/// What the detector saw at one moment; `face` is empty where it saw none (a gap).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FaceSample {
    pub t_us: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face: Option<FaceBox>,
}

/// The main face through a camera stream, in the asset's time. Time the camera did not record
/// (between a recording's segments) has no samples.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FaceTrack {
    pub version: u32,
    pub asset: String,
    pub stream: String,
    /// What the track was made from (the asset's path and length): another file under the same
    /// id is looked at again.
    pub source: String,
    pub model: String,
    pub sample_us: u64,
    pub samples: Vec<FaceSample>,
}

impl FaceTrack {
    /// The share of samples that found a face.
    pub fn face_share(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        self.samples.iter().filter(|s| s.face.is_some()).count() as f32 / self.samples.len() as f32
    }
}

fn source_of(asset: &Asset) -> String {
    format!("{}|{}", asset.path, asset.duration_us)
}

fn valid_part(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn track_path(root: &Path, asset: &str, stream: &str) -> Result<PathBuf, String> {
    if !valid_part(asset) || !valid_part(stream) {
        return Err("Invalid camera stream for a face track".into());
    }
    crate::project::reader::safe_path(root, &format!("{FACES_DIR}/{asset}.{stream}.json"))
}

pub fn load_track(root: &Path, asset: &str, stream: &str) -> Result<Option<FaceTrack>, String> {
    let path = track_path(root, asset, stream)?;
    if !path.is_file() {
        return Ok(None);
    }
    let file = crate::project::reader::open_regular(&path)?;
    let mut bytes = Vec::new();
    file.take(MAX_TRACK_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_TRACK_BYTES {
        return Err("The face track is too large".into());
    }
    let track: FaceTrack = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    Ok((track.version == TRACK_VERSION).then_some(track))
}

pub fn save_track(root: &Path, track: &FaceTrack) -> Result<(), String> {
    let path = track_path(root, &track.asset, &track.stream)?;
    let dir = path.parent().ok_or("No folder for face tracks")?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec(track).map_err(|e| e.to_string())?;
    let mut temp = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
    std::io::Write::write_all(&mut temp, &bytes).map_err(|e| e.to_string())?;
    temp.persist(&path).map_err(|e| e.to_string())?;
    Ok(())
}

/// The camera streams in the project: picture streams whose role is the camera.
pub fn camera_streams(document: &EditDocument) -> Vec<(String, String)> {
    document
        .assets
        .iter()
        .filter(|a| !a.is_still() && !a.missing)
        .flat_map(|a| {
            a.streams
                .iter()
                .filter(|s| s.kind == StreamKind::Picture && s.role == Role::Webcam)
                .map(|s| (a.id.clone(), s.id.clone()))
        })
        .collect()
}

type Stamp = (u64, Option<std::time::SystemTime>);
type TrackCache = HashMap<PathBuf, (Stamp, Option<Arc<FaceTrack>>)>;
static TRACKS: Mutex<Option<TrackCache>> = Mutex::new(None);

/// A camera stream's face track, if it has one. Read once while the file stays the same, so
/// each frame costs one file check.
pub fn cached_track(root: &Path, asset: &str, stream: &str) -> Option<Arc<FaceTrack>> {
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
    cache.insert(path, (stamp, built.clone()));
    built
}

/// Where a camera stream's face track stands.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FaceTrackStatus {
    pub asset: String,
    pub stream: String,
    pub asset_name: String,
    /// `ready`, `detecting`, `missing`, `failed`, or `unavailable` (a build without ONNX
    /// Runtime).
    pub state: String,
    /// 0 to 1 while detecting.
    pub progress: f32,
    /// The share of the camera where a face was found, once ready.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub face_share: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
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

/// Each camera stream's face track: found, being found, or not yet.
pub fn status(root: &Path, document: &EditDocument) -> Vec<FaceTrackStatus> {
    camera_streams(document)
        .into_iter()
        .filter_map(|(asset_id, stream)| {
            let asset = document.asset(&asset_id)?;
            let path = track_path(root, &asset_id, &stream).ok()?;
            let job = with_jobs(|jobs| jobs.get(&path).cloned());
            let track = load_track(root, &asset_id, &stream)
                .ok()
                .flatten()
                .filter(|t| t.source == source_of(asset));
            let (state, error) = match (&track, &job) {
                (_, Some(job)) if job.running => ("detecting", None),
                (Some(_), _) => ("ready", None),
                (None, Some(Job { error: Some(e), .. })) => ("failed", Some(e.clone())),
                (None, _) if !yunet::compiled_in() => ("unavailable", None),
                (None, _) => ("missing", None),
            };
            Some(FaceTrackStatus {
                asset: asset_id,
                stream,
                asset_name: asset.name.clone(),
                state: state.into(),
                progress: job.filter(|j| j.running).map_or(0.0, |j| j.progress),
                face_share: track.map(|t| t.face_share()),
                error,
            })
        })
        .collect()
}

/// Starts finding the face in every camera stream that has no track yet, each on a background
/// thread. Streams already being looked at, or that failed this session, are left alone
/// unless `retry`.
pub fn ensure(root: &Path, document: &EditDocument, retry: bool) {
    if !yunet::compiled_in() {
        return;
    }
    for status in status(root, document) {
        let start = status.state == "missing" || (retry && status.state == "failed");
        if !start {
            continue;
        }
        let Some(asset) = document.asset(&status.asset).cloned() else {
            continue;
        };
        let Ok(path) = track_path(root, &status.asset, &status.stream) else {
            continue;
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
            continue;
        }
        let root = root.to_path_buf();
        let stream = status.stream.clone();
        std::thread::Builder::new()
            .name("face-track".into())
            .spawn(move || {
                let cancel = AtomicBool::new(false);
                let result = detect_and_save(&root, &asset, &stream, &cancel, &mut |p| {
                    with_jobs(|jobs| {
                        if let Some(job) = jobs.get_mut(&path) {
                            job.progress = p;
                        }
                    })
                });
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
}

fn detect_and_save(
    root: &Path,
    asset: &Asset,
    stream: &str,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(f32),
) -> Result<(), String> {
    let dir = yunet::default_model_dir();
    if !yunet::model_present(&dir) {
        yunet::download_model(&dir, cancel)?;
    }
    let mut model = yunet::load(&dir)?;
    let track = detect(root, asset, stream, model.as_mut(), cancel, progress)?;
    save_track(root, &track)
}

/// The pieces of a camera stream on disk: a file and where its time 0 is on the asset's clock,
/// up to where it ends.
fn pieces(root: &Path, asset: &Asset, stream: &str) -> Result<Vec<(PathBuf, u64, u64)>, String> {
    Ok(
        match crate::sequence::sources::stream_source(root, asset, stream)? {
            StreamSource::Segments(segments) => segments
                .iter()
                .filter(|s| s.available && s.end_us > s.start_us)
                .filter_map(|s| {
                    crate::project::reader::safe_path(root, &s.relative_path)
                        .ok()
                        .map(|p| (p, s.start_us, s.end_us))
                })
                .collect(),
            StreamSource::Video(path) => vec![(path, 0, asset.duration_us.max(1))],
            StreamSource::Image(_) | StreamSource::Graphic(_) => Vec::new(),
        },
    )
}

/// Finds the main face every few seconds through a camera stream.
pub fn detect(
    root: &Path,
    asset: &Asset,
    stream: &str,
    model: &mut dyn yunet::FaceModel,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(f32),
) -> Result<FaceTrack, String> {
    use std::sync::atomic::Ordering;
    let pieces = pieces(root, asset, stream)?;
    let total: u64 = pieces.iter().map(|(_, a, b)| b - a).sum::<u64>().max(1);
    let mut done = 0u64;
    let mut samples = Vec::new();
    for (path, start_us, end_us) in &pieces {
        if cancel.load(Ordering::Relaxed) {
            return Err("Face detection cancelled".into());
        }
        let info = crate::media::ffmpeg::probe_video(path)?;
        let (w, h) = yunet::fit(info.width, info.height, DETECT_WIDTH);
        let mut failure = None;
        crate::media::ffmpeg::sample_bgr(path, SAMPLE_EVERY_S, w, h, &mut |index, bgr| {
            let t_us = start_us + index as u64 * SAMPLE_US;
            if t_us >= *end_us || cancel.load(Ordering::Relaxed) {
                return false;
            }
            match model.detect(bgr, w as usize, h as usize) {
                Ok(found) => samples.push(FaceSample {
                    t_us,
                    face: yunet::main_face(&found, w as usize, h as usize).map(FaceBox::rounded),
                }),
                Err(e) => {
                    failure = Some(e);
                    return false;
                }
            }
            progress(((done + (t_us - start_us)) as f32 / total as f32).min(1.0));
            true
        })?;
        if let Some(e) = failure {
            return Err(e);
        }
        if cancel.load(Ordering::Relaxed) {
            return Err("Face detection cancelled".into());
        }
        done += end_us - start_us;
    }
    samples.sort_by_key(|s| s.t_us);
    Ok(FaceTrack {
        version: TRACK_VERSION,
        asset: asset.id.clone(),
        stream: stream.into(),
        source: source_of(asset),
        model: yunet::MODEL_NAME.into(),
        sample_us: SAMPLE_US,
        samples,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real detector on a real camera file, run by hand:
    /// `AEROEDITS_FACE_PROBE=<video> cargo test --lib --features parakeet face_probe -- --ignored --nocapture`
    /// (downloads the model on first use). Prints where it found the face and how fast.
    #[cfg(feature = "parakeet")]
    #[test]
    #[ignore]
    fn face_probe() {
        let Some(video) = std::env::var_os("AEROEDITS_FACE_PROBE").map(PathBuf::from) else {
            return;
        };
        let dir = yunet::default_model_dir();
        if !yunet::model_present(&dir) {
            yunet::download_model(&dir, &AtomicBool::new(false)).unwrap();
        }
        let mut model = yunet::load(&dir).unwrap();
        let duration_us = crate::media::ffmpeg::duration_us(&video).unwrap();
        let asset: Asset = serde_json::from_value(serde_json::json!({
            "id": "cam", "name": "Camera", "kind": "video", "path": video,
            "durationUs": duration_us, "width": 0, "height": 0,
            "streams": [{"id": "picture", "name": "Picture", "kind": "picture", "role": "webcam"}]
        }))
        .unwrap();
        let root = tempfile::tempdir().unwrap();
        let started = std::time::Instant::now();
        let track = detect(
            root.path(),
            &asset,
            "picture",
            model.as_mut(),
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap();
        let took = started.elapsed();
        let faces: Vec<FaceBox> = track.samples.iter().filter_map(|s| s.face).collect();
        let mean = |f: &dyn Fn(&FaceBox) -> f32| {
            faces.iter().map(f).sum::<f32>() / faces.len().max(1) as f32
        };
        println!(
            "{} samples in {took:?} ({:.1} ms each); a face in {:.0}%; mean centre x {:.3}, eye y {:.3}, height {:.3}, score {:.2}",
            track.samples.len(),
            took.as_secs_f64() * 1000.0 / track.samples.len().max(1) as f64,
            track.face_share() * 100.0,
            mean(&|f| f.cx()),
            mean(&|f| f.eye_y),
            mean(&|f| f.h),
            mean(&|f| f.score),
        );
        if let Some(focus) = clip_focus(&track, 0, duration_us) {
            println!("One clip of all of it lines up on {focus:?}");
            for t in (0..duration_us).step_by(10_000_000) {
                let clip = clip_focus(&track, t, t + 10_000_000);
                println!("  a 10 s clip at {:>5.1}s: {clip:?}", t as f64 / 1e6);
            }
            // With `AEROEDITS_FACE_PROBE_OUT=<dir>`: the camera alone in a 9:16 frame at 1 s,
            // cut from the centre and around the face, as PNGs to look at.
            if let Some(out) = std::env::var_os("AEROEDITS_FACE_PROBE_OUT").map(PathBuf::from) {
                let t = 1_000_000.min(duration_us / 2);
                let frame = crate::media::ffmpeg::decode_bgra(&video, t).unwrap();
                let save = |name: &str, focus: Option<FocusPoint>| {
                    let (x, y, w, h) = crate::render::focus_uv(
                        focus,
                        frame.width,
                        frame.height,
                        1080,
                        1920,
                        (0.0, 0.0),
                    );
                    let (fw, fh) = (frame.width as f32, frame.height as f32);
                    let (x0, y0) = ((x * fw) as u32, (y * fh) as u32);
                    let (cw, ch) = ((w * fw) as u32, (h * fh) as u32);
                    let mut image = image::RgbImage::new(cw, ch);
                    for (px, py, pixel) in image.enumerate_pixels_mut() {
                        let i = ((y0 + py) * frame.stride + (x0 + px) * 4) as usize;
                        *pixel = image::Rgb([frame.data[i + 2], frame.data[i + 1], frame.data[i]]);
                    }
                    image.save(out.join(name)).unwrap();
                };
                save("camera-only-centre.png", None);
                save("camera-only-face.png", Some(focus));
            }
        }
        assert!(track.face_share() > 0.0);
    }

    #[test]
    fn tracks_round_trip_and_reject_odd_names() {
        let dir = tempfile::tempdir().unwrap();
        let track = FaceTrack {
            version: TRACK_VERSION,
            asset: "rec".into(),
            stream: "webcam".into(),
            source: "x|1".into(),
            model: yunet::MODEL_NAME.into(),
            sample_us: SAMPLE_US,
            samples: vec![
                FaceSample {
                    t_us: 0,
                    face: Some(FaceBox {
                        x: 0.6,
                        y: 0.2,
                        w: 0.2,
                        h: 0.4,
                        eye_y: 0.35,
                        score: 0.9,
                    }),
                },
                FaceSample {
                    t_us: SAMPLE_US,
                    face: None,
                },
            ],
        };
        save_track(dir.path(), &track).unwrap();
        assert_eq!(
            load_track(dir.path(), "rec", "webcam").unwrap(),
            Some(track.clone())
        );
        assert_eq!(track.face_share(), 0.5);
        let cached = cached_track(dir.path(), "rec", "webcam").unwrap();
        assert!((clip_focus(&cached, 0, SAMPLE_US).unwrap().x - 0.7).abs() < 1e-6);
        assert!(load_track(dir.path(), "rec", "other").unwrap().is_none());
        assert!(track_path(dir.path(), "../x", "webcam").is_err());
        assert!(track_path(dir.path(), "rec", "a.b").is_err());
    }
}
