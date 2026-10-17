//! Turns a face track into where the camera crop looks. It is not a crop that chases the face:
//! people move as they talk, and that is fine. Instead the crop is lined up with the face
//! every so often, like a camera operator who reframes between takes. The track is cut into
//! 30 s windows; each window's typical face (the median of its looks, so a stray detection
//! counts for nothing) is where the crop should point. When that is outside a dead zone
//! around where the crop already points, the crop eases over just before the window starts,
//! so it moves at most once every 30 s and the face is near the centre for each window.
//!
//! The path is worked out once for the whole track, in source time, so any frame (preview,
//! export, a seek into the middle of a move) reads the same point.
use super::{FaceSample, FaceTrack};

/// How long the crop holds before it may line up again.
pub const WINDOW_US: u64 = 30_000_000;
/// How far (as a share of the camera frame's width and height) a window's face may sit from
/// where the crop points before the crop lines up with it.
pub const DEAD_ZONE_X: f32 = 0.06;
pub const DEAD_ZONE_Y: f32 = 0.06;
/// How long one move takes; it lands as its window starts.
pub const MOVE_US: u64 = 1_000_000;

/// Where the crop looks at one moment: the face's middle across, its eye line down, and its
/// height, all as shares of the camera frame. The crop does not use the height yet; it is
/// there for sizing the face later (a closer crop in a camera-only scene, say).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FocusPoint {
    pub x: f32,
    pub eye_y: f32,
    pub face_h: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Move {
    start_us: u64,
    from: FocusPoint,
    to: FocusPoint,
}

/// The framing for a whole track: a resting point and the moves between windows.
#[derive(Clone, Debug, PartialEq)]
pub struct FocusPath {
    rest: FocusPoint,
    moves: Vec<Move>,
}

fn outside(at: FocusPoint, face: FocusPoint) -> bool {
    (face.x - at.x).abs() > DEAD_ZONE_X || (face.eye_y - at.eye_y).abs() > DEAD_ZONE_Y
}

/// Smoothstep: starts and lands gently.
fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn median(mut values: Vec<f32>) -> f32 {
    values.sort_by(f32::total_cmp);
    values[values.len() / 2]
}

impl FocusPath {
    /// The path for `track`, or `None` when it never found a face (the crop stays centred).
    pub fn build(track: &FaceTrack) -> Option<Self> {
        Self::from_samples(&track.samples)
    }

    pub fn from_samples(samples: &[FaceSample]) -> Option<Self> {
        // Each window with a face in it, and its typical face. Windows without one hold.
        let mut windows: Vec<(u64, Vec<FocusPoint>)> = Vec::new();
        for sample in samples {
            let Some(face) = sample.face else { continue };
            let window = sample.t_us / WINDOW_US;
            let point = FocusPoint {
                x: face.cx(),
                eye_y: face.eye_y,
                face_h: face.h,
            };
            match windows.last_mut() {
                Some((w, points)) if *w == window => points.push(point),
                _ => windows.push((window, vec![point])),
            }
        }
        let targets: Vec<(u64, FocusPoint)> = windows
            .into_iter()
            .map(|(window, points)| {
                let typical = FocusPoint {
                    x: median(points.iter().map(|p| p.x).collect()),
                    eye_y: median(points.iter().map(|p| p.eye_y).collect()),
                    face_h: median(points.iter().map(|p| p.face_h).collect()),
                };
                (window * WINDOW_US, typical)
            })
            .collect();
        // Before the first face is seen the crop already points at it: the whole track is
        // known, so there is no reason to open on the centre and swing over.
        let rest = targets.first()?.1;
        let mut at = rest;
        let mut moves = Vec::new();
        for &(start, target) in &targets[1..] {
            if outside(at, target) {
                moves.push(Move {
                    start_us: start.saturating_sub(MOVE_US),
                    from: at,
                    to: target,
                });
                at = target;
            }
        }
        Some(Self { rest, moves })
    }

    /// Where the crop points at source time `t_us`.
    pub fn at(&self, t_us: u64) -> FocusPoint {
        let index = self.moves.partition_point(|m| m.start_us <= t_us);
        match index.checked_sub(1).map(|i| self.moves[i]) {
            None => self.rest,
            Some(m) => {
                let k = ease((t_us - m.start_us) as f32 / MOVE_US as f32);
                let mix = |a: f32, b: f32| a + (b - a) * k;
                FocusPoint {
                    x: mix(m.from.x, m.to.x),
                    eye_y: mix(m.from.eye_y, m.to.eye_y),
                    face_h: mix(m.from.face_h, m.to.face_h),
                }
            }
        }
    }

    /// How many times the crop moves.
    pub fn move_count(&self) -> usize {
        self.moves.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::FaceBox;

    const S: u64 = 1_000_000;
    const STEP: u64 = super::super::SAMPLE_US;

    fn sample(t_us: u64, x: Option<f32>) -> FaceSample {
        FaceSample {
            t_us,
            face: x.map(|x| FaceBox {
                x: x - 0.05,
                y: 0.3,
                w: 0.1,
                h: 0.2,
                eye_y: 0.38,
                score: 0.9,
            }),
        }
    }

    /// A look every sample step, the face where `x_at` says.
    fn track(len_us: u64, x_at: impl Fn(u64) -> Option<f32>) -> Vec<FaceSample> {
        (0..len_us / STEP)
            .map(|i| sample(i * STEP, x_at(i * STEP)))
            .collect()
    }

    #[test]
    fn a_face_moving_about_inside_the_dead_zone_never_moves_the_crop() {
        let samples = track(300 * S, |t| {
            Some(0.5 + if (t / STEP) % 2 == 0 { 0.05 } else { -0.05 })
        });
        let path = FocusPath::from_samples(&samples).unwrap();
        assert_eq!(path.move_count(), 0);
    }

    #[test]
    fn the_crop_lines_up_once_per_window_and_lands_as_it_starts() {
        // At 0.3 for 40 s, then at 0.75: most of the second window is at 0.75.
        let samples = track(120 * S, |t| Some(if t < 40 * S { 0.3 } else { 0.75 }));
        let path = FocusPath::from_samples(&samples).unwrap();
        assert_eq!(path.move_count(), 1);
        assert!(
            (path.at(WINDOW_US - MOVE_US - 1).x - 0.3).abs() < 1e-6,
            "still before"
        );
        let mid = path.at(WINDOW_US - MOVE_US / 2).x;
        assert!((mid - 0.525).abs() < 1e-3, "half way at half time: {mid}");
        assert!((path.at(WINDOW_US).x - 0.75).abs() < 1e-6, "landed");
        assert!((path.at(119 * S).x - 0.75).abs() < 1e-6);
        // Eased and monotonic: no overshoot.
        let mut last = 0.0;
        for k in 0..=60 {
            let x = path.at(WINDOW_US - MOVE_US + k * MOVE_US / 60).x;
            assert!(x >= last - 1e-6 && x <= 0.75 + 1e-6);
            last = x;
        }
    }

    #[test]
    fn a_restless_face_moves_the_crop_at_most_every_window() {
        // Swapping sides every 10 s, for five minutes.
        let samples = track(300 * S, |t| {
            Some(if (t / (10 * S)) % 2 == 0 { 0.3 } else { 0.7 })
        });
        let path = FocusPath::from_samples(&samples).unwrap();
        assert!(path.move_count() <= 10);
        for pair in path.moves.windows(2) {
            assert!(pair[1].start_us - pair[0].start_us >= WINDOW_US);
        }
    }

    #[test]
    fn stray_detections_count_for_nothing() {
        let samples = track(120 * S, |t| Some(if t == 45 * S { 0.95 } else { 0.5 }));
        let path = FocusPath::from_samples(&samples).unwrap();
        assert_eq!(path.move_count(), 0);
    }

    #[test]
    fn no_face_holds_and_a_track_without_faces_has_no_path() {
        // No face in the first window, then at 0.2, then none for a while, then at 0.8.
        let samples = track(150 * S, |t| match t / WINDOW_US {
            0 => None,
            1 => Some(0.2),
            2 | 3 => None,
            _ => Some(0.8),
        });
        let path = FocusPath::from_samples(&samples).unwrap();
        // Before the first face: already on it, not on the centre.
        assert!((path.at(0).x - 0.2).abs() < 1e-6);
        // The empty windows hold where it was.
        assert!((path.at(100 * S).x - 0.2).abs() < 1e-6);
        // Then it lines up when the face is seen again.
        assert!((path.at(4 * WINDOW_US).x - 0.8).abs() < 1e-6);
        assert!(FocusPath::from_samples(&track(60 * S, |_| None)).is_none());
        assert!(FocusPath::from_samples(&[]).is_none());
    }
}
