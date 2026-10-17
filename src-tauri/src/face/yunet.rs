//! YuNet (OpenCV Zoo, 2023mar, MIT), a 230 KB face detector, run through ONNX Runtime on the
//! CPU. It is downloaded on first use next to the other models, the way the transcription and
//! alignment models are. Decoding its outputs is plain Rust, so it is tested without the model.
use super::FaceBox;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub const MODEL_NAME: &str = "face_detection_yunet_2023mar";
const MODEL_URL: &str = "https://huggingface.co/opencv/face_detection_yunet/resolve/main/face_detection_yunet_2023mar.onnx";
const MODEL_FILE: &str = "face_detection_yunet_2023mar.onnx";
/// The file is this size exactly; anything else is a broken download.
const MODEL_BYTES: u64 = 232_589;
/// The strides of YuNet's three heads.
const STRIDES: [usize; 3] = [8, 16, 32];
/// The model takes a fixed 640x640 input: pictures go in its top-left corner, black around.
pub const INPUT: usize = 640;
/// Detections below this confidence are dropped.
const MIN_SCORE: f32 = 0.6;
/// Boxes overlapping more than this are the same face.
const NMS_IOU: f32 = 0.3;

pub fn default_model_dir() -> PathBuf {
    crate::transcript::parakeet::default_model_dir()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(std::env::temp_dir)
        .join(MODEL_NAME)
}

pub fn model_path(dir: &Path) -> PathBuf {
    dir.join(MODEL_FILE)
}

pub fn model_present(dir: &Path) -> bool {
    std::fs::metadata(model_path(dir)).is_ok_and(|m| m.len() == MODEL_BYTES)
}

/// Face detection runs with ONNX Runtime, which comes with the `parakeet` feature.
pub fn compiled_in() -> bool {
    cfg!(feature = "parakeet")
}

/// Downloads the model into `dir` (a quarter of a megabyte, so no progress is reported).
pub fn download_model(dir: &Path, cancel: &AtomicBool) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;
    let mut response = client
        .get(MODEL_URL)
        .send()
        .map_err(|e| format!("Could not download the face detector: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Downloading the face detector failed: {}",
            response.status()
        ));
    }
    let mut bytes = Vec::with_capacity(MODEL_BYTES as usize);
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("Download cancelled".into());
        }
        let n = response
            .read(&mut buf)
            .map_err(|e| format!("Download of the face detector was interrupted: {e}"))?;
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&buf[..n]);
        if bytes.len() as u64 > MODEL_BYTES {
            break;
        }
    }
    if bytes.len() as u64 != MODEL_BYTES {
        return Err(format!(
            "The face detector download is {} bytes, not {MODEL_BYTES}",
            bytes.len()
        ));
    }
    let mut temp = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
    temp.write_all(&bytes).map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    temp.persist(model_path(dir)).map_err(|e| e.to_string())?;
    Ok(())
}

/// The size a `w` x `h` picture is scaled to for the model: it fits in the input, at most
/// `max_w` wide, and both sides are even.
pub fn fit(w: u32, h: u32, max_w: u32) -> (u32, u32) {
    let scale = (max_w.min(INPUT as u32) as f64 / w.max(1) as f64)
        .min(INPUT as f64 / h.max(1) as f64)
        .min(1.0);
    let even = |v: f64| ((v.round() as u32) & !1).max(2);
    (even(w as f64 * scale), even(h as f64 * scale))
}

/// A BGR picture (`w` x `h`, packed, at most the input's size) as YuNet's NCHW float input,
/// padded with black on the right and bottom. YuNet takes 0-255 values as they are.
pub fn input_tensor(bgr: &[u8], w: usize, h: usize) -> (usize, usize, Vec<f32>) {
    let (pw, ph) = (INPUT, INPUT);
    let plane = pw * ph;
    let mut data = vec![0f32; 3 * plane];
    for y in 0..h.min(INPUT) {
        for x in 0..w.min(INPUT) {
            let i = (y * w + x) * 3;
            let o = y * pw + x;
            data[o] = bgr[i] as f32;
            data[plane + o] = bgr[i + 1] as f32;
            data[2 * plane + o] = bgr[i + 2] as f32;
        }
    }
    (pw, ph, data)
}

/// One head's outputs, flat: `cls` and `obj` one per cell, `bbox` four, `kps` ten.
pub struct Head<'a> {
    pub cls: &'a [f32],
    pub obj: &'a [f32],
    pub bbox: &'a [f32],
    pub kps: &'a [f32],
}

/// A detection in input pixels: box, the two eyes and confidence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Detection {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub eyes: [(f32, f32); 2],
    pub score: f32,
}

impl Detection {
    fn iou(&self, other: &Detection) -> f32 {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = (self.x + self.w).min(other.x + other.w);
        let y1 = (self.y + self.h).min(other.y + other.h);
        let inter = (x1 - x0).max(0.0) * (y1 - y0).max(0.0);
        let union = self.w * self.h + other.w * other.h - inter;
        if union <= 0.0 {
            0.0
        } else {
            inter / union
        }
    }
}

/// The faces in YuNet's three heads (strides 8, 16, 32) for a `pw` x `ph` input, after
/// non-maximum suppression, best first. Follows OpenCV's `FaceDetectorYN` decoding.
pub fn decode(heads: &[Head<'_>; 3], pw: usize, ph: usize) -> Vec<Detection> {
    let mut found = Vec::new();
    for (head, stride) in heads.iter().zip(STRIDES) {
        let cols = pw / stride;
        let rows = ph / stride;
        let s = stride as f32;
        for r in 0..rows {
            for c in 0..cols {
                let idx = r * cols + c;
                let (Some(&cls), Some(&obj)) = (head.cls.get(idx), head.obj.get(idx)) else {
                    continue;
                };
                let score = (cls.clamp(0.0, 1.0) * obj.clamp(0.0, 1.0)).sqrt();
                if score < MIN_SCORE {
                    continue;
                }
                let (Some(b), Some(k)) = (
                    head.bbox.get(idx * 4..idx * 4 + 4),
                    head.kps.get(idx * 10..idx * 10 + 10),
                ) else {
                    continue;
                };
                let cx = (c as f32 + b[0]) * s;
                let cy = (r as f32 + b[1]) * s;
                let w = b[2].exp() * s;
                let h = b[3].exp() * s;
                let point = |n: usize| ((k[2 * n] + c as f32) * s, (k[2 * n + 1] + r as f32) * s);
                found.push(Detection {
                    x: cx - w / 2.0,
                    y: cy - h / 2.0,
                    w,
                    h,
                    eyes: [point(0), point(1)],
                    score,
                });
            }
        }
    }
    found.sort_by(|a, b| b.score.total_cmp(&a.score));
    let mut kept: Vec<Detection> = Vec::new();
    for d in found {
        if kept.iter().all(|k| k.iou(&d) <= NMS_IOU) {
            kept.push(d);
        }
    }
    kept
}

/// The face to frame on: the biggest confident one (the speaker sits closest to the camera),
/// in shares of the `w` x `h` picture.
pub fn main_face(detections: &[Detection], w: usize, h: usize) -> Option<FaceBox> {
    let (w, h) = (w.max(1) as f32, h.max(1) as f32);
    detections
        .iter()
        .max_by(|a, b| (a.w * a.h * a.score).total_cmp(&(b.w * b.h * b.score)))
        .map(|d| {
            let x0 = d.x.clamp(0.0, w);
            let y0 = d.y.clamp(0.0, h);
            let x1 = (d.x + d.w).clamp(0.0, w);
            let y1 = (d.y + d.h).clamp(0.0, h);
            let eye_y = ((d.eyes[0].1 + d.eyes[1].1) / 2.0).clamp(y0, y1);
            FaceBox {
                x: x0 / w,
                y: y0 / h,
                w: (x1 - x0) / w,
                h: (y1 - y0) / h,
                eye_y: eye_y / h,
                score: d.score,
            }
        })
}

/// Finds faces in BGR pictures.
pub trait FaceModel {
    fn detect(&mut self, bgr: &[u8], w: usize, h: usize) -> Result<Vec<Detection>, String>;
}

#[cfg(feature = "parakeet")]
mod engine {
    use super::{decode, input_tensor, Detection, FaceModel, Head};
    use std::path::Path;

    pub struct YuNet {
        session: ort::session::Session,
    }

    impl YuNet {
        pub fn new(dir: &Path) -> Result<Self, String> {
            if !super::model_present(dir) {
                return Err(format!(
                    "The face detector is not downloaded yet (looked in {})",
                    dir.display()
                ));
            }
            // The CPU only, on two threads: it runs in the background beside playback.
            let session = parakeet_rs::ExecutionConfig::new()
                .with_intra_threads(2)
                .with_inter_threads(1)
                .build_session(&super::model_path(dir))
                .map_err(|e| format!("Could not load the face detector: {e}"))?;
            Ok(Self { session })
        }
    }

    impl FaceModel for YuNet {
        fn detect(&mut self, bgr: &[u8], w: usize, h: usize) -> Result<Vec<Detection>, String> {
            let (pw, ph, data) = input_tensor(bgr, w, h);
            let tensor = ort::value::Tensor::from_array(([1usize, 3, ph, pw], data))
                .map_err(|e| e.to_string())?;
            let outputs = self
                .session
                .run(ort::inputs!["input" => tensor])
                .map_err(|e| format!("Face detection failed: {e}"))?;
            let mut flat: Vec<Vec<f32>> = Vec::with_capacity(12);
            for kind in ["cls", "obj", "bbox", "kps"] {
                for stride in super::STRIDES {
                    let (_, values) = outputs[format!("{kind}_{stride}").as_str()]
                        .try_extract_tensor::<f32>()
                        .map_err(|e| e.to_string())?;
                    flat.push(values.to_vec());
                }
            }
            let head = |i: usize| Head {
                cls: &flat[i],
                obj: &flat[3 + i],
                bbox: &flat[6 + i],
                kps: &flat[9 + i],
            };
            Ok(decode(&[head(0), head(1), head(2)], pw, ph))
        }
    }
}

/// The detector in `dir`, ready to run.
pub fn load(dir: &Path) -> Result<Box<dyn FaceModel>, String> {
    #[cfg(feature = "parakeet")]
    {
        Ok(Box::new(engine::YuNet::new(dir)?))
    }
    #[cfg(not(feature = "parakeet"))]
    {
        let _ = dir;
        Err("This build of AeroEdits does not include face detection".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One head's outputs: cls, obj, bbox, kps.
    type RawHead = (Vec<f32>, Vec<f32>, Vec<f32>, Vec<f32>);

    /// Heads for a 64x32 input with nothing in them, and one confident cell.
    fn heads_with(cell: Option<(usize, usize, usize)>) -> Vec<RawHead> {
        STRIDES
            .iter()
            .enumerate()
            .map(|(i, &stride)| {
                let n = (64 / stride) * (32 / stride);
                let mut cls = vec![0.0; n];
                let obj = vec![1.0; n];
                let mut bbox = vec![0.0; n * 4];
                let kps = vec![0.0; n * 10];
                if let Some((head, r, c)) = cell {
                    if head == i {
                        let idx = r * (64 / stride) + c;
                        cls[idx] = 0.81;
                        // Box 2 cells wide and 3 high, centred half a cell right of the cell.
                        bbox[idx * 4] = 0.5;
                        bbox[idx * 4 + 2] = 2f32.ln();
                        bbox[idx * 4 + 3] = 3f32.ln();
                    }
                }
                (cls, obj, bbox, kps)
            })
            .collect()
    }

    fn run(raw: &[RawHead]) -> Vec<Detection> {
        let head = |i: usize| Head {
            cls: &raw[i].0,
            obj: &raw[i].1,
            bbox: &raw[i].2,
            kps: &raw[i].3,
        };
        decode(&[head(0), head(1), head(2)], 64, 32)
    }

    #[test]
    fn decodes_a_box_the_way_opencv_does() {
        assert!(run(&heads_with(None)).is_empty());
        // Stride 16, row 1, column 2.
        let found = run(&heads_with(Some((1, 1, 2))));
        assert_eq!(found.len(), 1);
        let d = found[0];
        assert!((d.score - 0.9).abs() < 1e-5, "sqrt(cls * obj)");
        assert!((d.w - 32.0).abs() < 1e-3 && (d.h - 48.0).abs() < 1e-3);
        // Centre (2.5, 1) cells = (40, 16) px.
        assert!((d.x + d.w / 2.0 - 40.0).abs() < 1e-3);
        assert!((d.y + d.h / 2.0 - 16.0).abs() < 1e-3);
        // Landmarks at offset 0 sit on the cell's corner.
        assert_eq!(d.eyes[0], (32.0, 16.0));
    }

    #[test]
    fn overlapping_boxes_are_one_face_and_the_biggest_is_the_speaker() {
        let a = Detection {
            x: 10.0,
            y: 10.0,
            w: 20.0,
            h: 20.0,
            eyes: [(15.0, 16.0), (25.0, 18.0)],
            score: 0.9,
        };
        let b = Detection { x: 12.0, ..a };
        assert!(a.iou(&b) > NMS_IOU);
        let far = Detection {
            x: 60.0,
            w: 30.0,
            h: 30.0,
            score: 0.7,
            ..a
        };
        let face = main_face(&[a, far], 100, 50).unwrap();
        assert!((face.x - 0.6).abs() < 1e-6 && (face.w - 0.3).abs() < 1e-6);
        let small = main_face(&[a], 100, 50).unwrap();
        assert!((small.eye_y - 17.0 / 50.0).abs() < 1e-6);
        assert!(main_face(&[], 100, 50).is_none());
    }

    #[test]
    fn input_is_planar_bgr_padded_to_the_model_size() {
        assert_eq!(fit(1280, 720, 640), (640, 360));
        assert_eq!(
            fit(720, 1280, 640),
            (360, 640),
            "a tall camera fits by height"
        );
        assert_eq!(fit(320, 240, 640), (320, 240), "never scaled up");
        let bgr = [1u8, 2, 3, 4, 5, 6];
        let (pw, ph, data) = input_tensor(&bgr, 2, 1);
        assert_eq!((pw, ph), (INPUT, INPUT));
        let plane = INPUT * INPUT;
        assert_eq!(
            (data[0], data[1], data[plane], data[2 * plane + 1]),
            (1.0, 4.0, 2.0, 6.0)
        );
        assert_eq!(data[INPUT], 0.0, "padding is black");
    }
}
