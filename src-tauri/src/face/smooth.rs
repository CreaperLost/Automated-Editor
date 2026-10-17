//! Turns a face track into where the camera crop looks, the way a camera operator frames a
//! speaker: hold still while the face stays near where the frame points, and when it leaves
//! that dead zone, ease over to the new spot in one smooth move. The face's size is low-passed
//! so a box that flickers from one detection to the next does not shake the frame.
//!
//! The path is worked out once for the whole track, in source time, so any frame (preview,
//! export, a seek into the middle of a move) reads the same point.
use super::{FaceSample, FaceTrack};

/// How far (as a share of the camera frame's width and height) the face may wander from where
/// the frame points before the frame follows it.
pub const DEAD_ZONE_X: f32 = 0.06;
pub const DEAD_ZONE_Y: f32 = 0.06;
/// How long one move takes.
pub const MOVE_US: u64 = 600_000;
/// The face has to be outside the dead zone for this many samples in a row before the frame
/// moves: one stray detection does not swing it.
const CONFIRM_SAMPLES: usize = 2;
/// Time constant of the low-pass on the face's size.
const SIZE_TAU_US: f64 = 1_000_000.0;

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
    from: (f32, f32),
    to: (f32, f32),
}

/// The smoothed framing for a whole track.
#[derive(Clone, Debug, PartialEq)]
pub struct FocusPath {
    /// Where the frame rests before the first move.
    rest: (f32, f32),
    moves: Vec<Move>,
    /// The low-passed face height at each sample with a face.
    sizes: Vec<(u64, f32)>,
}

fn outside(rest: (f32, f32), point: (f32, f32)) -> bool {
    (point.0 - rest.0).abs() > DEAD_ZONE_X || (point.1 - rest.1).abs() > DEAD_ZONE_Y
}

/// Smoothstep: starts and lands gently.
fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl FocusPath {
    /// The path for `track`, or `None` when it never found a face (the crop stays centred).
    pub fn build(track: &FaceTrack) -> Option<Self> {
        Self::from_samples(&track.samples)
    }

    pub fn from_samples(samples: &[FaceSample]) -> Option<Self> {
        let faces: Vec<(u64, (f32, f32), f32)> = samples
            .iter()
            .filter_map(|s| s.face.map(|f| (s.t_us, (f.cx(), f.eye_y), f.h)))
            .collect();
        let first = faces.first()?;
        // Before the first face is seen the frame already points at it: the whole track is
        // known, so there is no reason to open on the centre and swing over.
        let rest = first.1;
        let mut moves = Vec::new();
        let mut at = rest;
        let mut busy_until = 0u64;
        let mut i = 0;
        while i < faces.len() {
            let (t, point, _) = faces[i];
            if t < busy_until || !outside(at, point) {
                i += 1;
                continue;
            }
            // Out of the dead zone: it has to stay out for a few samples in a row (gaps with
            // no face do not break the run), then the frame moves to where it settled.
            let run: Vec<_> = faces[i..]
                .iter()
                .take(CONFIRM_SAMPLES)
                .take_while(|(_, p, _)| outside(at, *p))
                .collect();
            if run.len() < CONFIRM_SAMPLES {
                i += 1;
                continue;
            }
            let n = run.len() as f32;
            let to = (
                run.iter().map(|(_, p, _)| p.0).sum::<f32>() / n,
                run.iter().map(|(_, p, _)| p.1).sum::<f32>() / n,
            );
            moves.push(Move {
                start_us: t,
                from: at,
                to,
            });
            at = to;
            busy_until = t + MOVE_US;
            i += 1;
        }
        let mut sizes = Vec::with_capacity(faces.len());
        let mut size = first.2;
        let mut last_t = first.0;
        for &(t, _, h) in &faces {
            let dt = t.saturating_sub(last_t) as f64;
            let k = 1.0 - (-dt / SIZE_TAU_US).exp();
            size += (h - size) * k as f32;
            sizes.push((t, size));
            last_t = t;
        }
        Some(Self { rest, moves, sizes })
    }

    /// Where the frame points at source time `t_us`.
    pub fn at(&self, t_us: u64) -> FocusPoint {
        let index = self.moves.partition_point(|m| m.start_us <= t_us);
        let (x, eye_y) = match index.checked_sub(1).map(|i| self.moves[i]) {
            None => self.rest,
            Some(m) => {
                let k = ease((t_us - m.start_us) as f32 / MOVE_US as f32);
                (
                    m.from.0 + (m.to.0 - m.from.0) * k,
                    m.from.1 + (m.to.1 - m.from.1) * k,
                )
            }
        };
        let face_h = {
            let i = self.sizes.partition_point(|(t, _)| *t <= t_us);
            match (i.checked_sub(1).map(|i| self.sizes[i]), self.sizes.get(i)) {
                (Some((t0, a)), Some(&(t1, b))) if t1 > t0 => {
                    a + (b - a) * ((t_us - t0) as f32 / (t1 - t0) as f32)
                }
                (Some((_, a)), _) => a,
                (None, Some(&(_, b))) => b,
                (None, None) => 0.0,
            }
        };
        FocusPoint { x, eye_y, face_h }
    }

    /// How many times the frame moves.
    pub fn move_count(&self) -> usize {
        self.moves.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::FaceBox;

    const STEP: u64 = super::super::SAMPLE_US;

    fn sample(i: u64, x: Option<f32>) -> FaceSample {
        FaceSample {
            t_us: i * STEP,
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

    fn track(xs: &[Option<f32>]) -> Vec<FaceSample> {
        xs.iter()
            .enumerate()
            .map(|(i, x)| sample(i as u64, *x))
            .collect()
    }

    #[test]
    fn a_face_inside_the_dead_zone_does_not_move_the_frame() {
        // Jitter of a few percent around 0.7: the frame sits still on where it first was.
        let xs: Vec<_> = (0..40)
            .map(|i| Some(0.7 + if i % 2 == 0 { 0.025 } else { -0.025 }))
            .collect();
        let path = FocusPath::from_samples(&track(&xs)).unwrap();
        assert_eq!(path.move_count(), 0);
        for t in [0, 3 * STEP, 39 * STEP, 100 * STEP] {
            assert!((path.at(t).x - 0.725).abs() < 1e-6);
        }
    }

    #[test]
    fn leaving_the_dead_zone_eases_over_in_one_move() {
        // At 0.3 for 2 s, then at 0.75.
        let xs: Vec<_> = (0..20)
            .map(|i| Some(if i < 8 { 0.3 } else { 0.75 }))
            .collect();
        let path = FocusPath::from_samples(&track(&xs)).unwrap();
        assert_eq!(path.move_count(), 1);
        let start = 8 * STEP;
        assert!((path.at(start - 1).x - 0.3).abs() < 1e-6, "still before");
        let mid = path.at(start + MOVE_US / 2).x;
        assert!((mid - 0.525).abs() < 1e-3, "half way at half time: {mid}");
        // Eased: slow at the start.
        let early = path.at(start + MOVE_US / 10).x;
        assert!(early - 0.3 < 0.45 * 0.1, "{early}");
        assert!((path.at(start + MOVE_US).x - 0.75).abs() < 1e-6, "landed");
        assert!((path.at(start + 10 * MOVE_US).x - 0.75).abs() < 1e-6);
        // The move is monotonic: no overshoot.
        let mut last = 0.0;
        for k in 0..=60 {
            let x = path.at(start + k * MOVE_US / 60).x;
            assert!(x >= last - 1e-6 && x <= 0.75 + 1e-6);
            last = x;
        }
    }

    #[test]
    fn a_single_stray_detection_is_ignored() {
        let mut xs: Vec<_> = (0..20).map(|_| Some(0.5)).collect();
        xs[10] = Some(0.9);
        let path = FocusPath::from_samples(&track(&xs)).unwrap();
        assert_eq!(path.move_count(), 0);
        assert!((path.at(10 * STEP).x - 0.5).abs() < 1e-6);
    }

    #[test]
    fn no_face_holds_the_last_spot_and_a_track_without_faces_has_no_path() {
        let mut xs: Vec<Option<f32>> = vec![None; 4];
        xs.extend([Some(0.2); 4]);
        xs.extend([None; 10]);
        xs.extend([Some(0.8); 4]);
        let path = FocusPath::from_samples(&track(&xs)).unwrap();
        // Before the first face: already on it, not on the centre.
        assert!((path.at(0).x - 0.2).abs() < 1e-6);
        // The gap holds where it was.
        assert!((path.at(12 * STEP).x - 0.2).abs() < 1e-6);
        // Then it moves to the face's new spot when the face comes back.
        assert!((path.at(18 * STEP + MOVE_US).x - 0.8).abs() < 1e-6);
        assert!(FocusPath::from_samples(&track(&[None; 10])).is_none());
        assert!(FocusPath::from_samples(&[]).is_none());
    }

    #[test]
    fn the_face_size_is_low_passed() {
        let mut samples = track(&[Some(0.5); 20]);
        for s in &mut samples[10..] {
            s.face.as_mut().unwrap().h = 0.4;
        }
        let path = FocusPath::from_samples(&samples).unwrap();
        assert!((path.at(9 * STEP).face_h - 0.2).abs() < 1e-6);
        // One sample after the jump it has moved only part of the way.
        let next = path.at(10 * STEP).face_h;
        assert!(next > 0.2 && next < 0.3, "{next}");
        assert!((path.at(19 * STEP).face_h - 0.4).abs() < 0.03);
        // Between samples it is interpolated, not stepped.
        let between = path.at(10 * STEP + STEP / 2).face_h;
        assert!(between > next && between < path.at(11 * STEP).face_h);
    }
}
