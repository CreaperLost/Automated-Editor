//! Where the camera crop looks for one clip. The crop is not a follower that chases the face:
//! people move as they talk, and that is fine. Instead each clip is lined up with the face
//! once, as it starts, and holds for the whole clip, like a camera operator who reframes
//! between takes. Moves land on the cuts, where nobody sees them.
//!
//! A clip's face is the typical one over the part of the camera it plays: the median of the
//! looks in it, so a stray detection counts for nothing. A clip shorter than the gap between
//! looks borrows the nearest ones.
use super::{FaceSample, FaceTrack, SAMPLE_US};

/// How far beyond a clip's ends a look may be and still stand for it, when none is inside.
const NEAR_US: u64 = 30_000_000;

/// Where the crop looks: the face's middle across, its eye line down, and its height, all as
/// shares of the camera frame. The crop does not use the height yet; it is there for sizing
/// the face later (a closer crop in a camera-only scene, say).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FocusPoint {
    pub x: f32,
    pub eye_y: f32,
    pub face_h: f32,
}

fn median(mut values: Vec<f32>) -> f32 {
    values.sort_by(f32::total_cmp);
    values[values.len() / 2]
}

fn typical(samples: &[&FaceSample]) -> Option<FocusPoint> {
    let faces: Vec<_> = samples.iter().filter_map(|s| s.face).collect();
    if faces.is_empty() {
        return None;
    }
    Some(FocusPoint {
        x: median(faces.iter().map(|f| f.cx()).collect()),
        eye_y: median(faces.iter().map(|f| f.eye_y).collect()),
        face_h: median(faces.iter().map(|f| f.h).collect()),
    })
}

/// The face to line a clip up with, for the camera's `[in_us, out_us)`; `None` when no face
/// was seen in or near it (the crop stays centred).
pub fn clip_focus(track: &FaceTrack, in_us: u64, out_us: u64) -> Option<FocusPoint> {
    let samples = &track.samples;
    // Looks inside the clip, widened by one gap so a short clip still has some.
    let lo = samples.partition_point(|s| s.t_us + SAMPLE_US < in_us);
    let hi = samples.partition_point(|s| s.t_us < out_us.saturating_add(SAMPLE_US));
    let inside: Vec<_> = samples[lo..hi.max(lo)].iter().collect();
    if let Some(point) = typical(&inside) {
        return Some(point);
    }
    // Else the nearest face, before or after, within reach.
    let before = samples[..lo]
        .iter()
        .rev()
        .find(|s| s.face.is_some() && in_us.saturating_sub(s.t_us) <= NEAR_US);
    let after = samples[hi.max(lo)..]
        .iter()
        .find(|s| s.face.is_some() && s.t_us.saturating_sub(out_us) <= NEAR_US);
    let nearest = match (before, after) {
        (Some(b), Some(a)) => {
            if in_us - b.t_us <= a.t_us - out_us {
                b
            } else {
                a
            }
        }
        (Some(s), None) | (None, Some(s)) => s,
        (None, None) => return None,
    };
    typical(&[nearest])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::face::{FaceBox, TRACK_VERSION};

    const S: u64 = 1_000_000;

    /// A look every sample step, the face where `x_at` says.
    fn track(len_us: u64, x_at: impl Fn(u64) -> Option<f32>) -> FaceTrack {
        FaceTrack {
            version: TRACK_VERSION,
            asset: "a".into(),
            stream: "webcam".into(),
            source: String::new(),
            model: String::new(),
            sample_us: SAMPLE_US,
            samples: (0..len_us / SAMPLE_US)
                .map(|i| {
                    let t_us = i * SAMPLE_US;
                    FaceSample {
                        t_us,
                        face: x_at(t_us).map(|x| FaceBox {
                            x: x - 0.05,
                            y: 0.3,
                            w: 0.1,
                            h: 0.2,
                            eye_y: 0.38,
                            score: 0.9,
                        }),
                    }
                })
                .collect(),
        }
    }

    #[test]
    fn each_clip_lines_up_with_its_own_typical_face() {
        // Left for the first minute, right after.
        let t = track(120 * S, |t| Some(if t < 60 * S { 0.3 } else { 0.7 }));
        assert!((clip_focus(&t, 10 * S, 40 * S).unwrap().x - 0.3).abs() < 1e-6);
        assert!((clip_focus(&t, 70 * S, 100 * S).unwrap().x - 0.7).abs() < 1e-6);
        // A clip across the change takes where the face spent most of it.
        assert!((clip_focus(&t, 50 * S, 100 * S).unwrap().x - 0.7).abs() < 1e-6);
        assert!((clip_focus(&t, 20 * S, 70 * S).unwrap().x - 0.3).abs() < 1e-6);
    }

    #[test]
    fn stray_detections_count_for_nothing() {
        let t = track(60 * S, |t| Some(if t == 25 * S { 0.95 } else { 0.5 }));
        assert!((clip_focus(&t, 0, 60 * S).unwrap().x - 0.5).abs() < 1e-6);
    }

    #[test]
    fn a_clip_between_looks_borrows_the_nearest_and_no_face_keeps_the_centre() {
        let t = track(60 * S, |t| Some(if t < 30 * S { 0.3 } else { 0.7 }));
        // 1 s long, between the looks at 30 s and 35 s.
        assert!((clip_focus(&t, 31 * S, 32 * S).unwrap().x - 0.7).abs() < 1e-6);
        // Faceless around the clip, a face 20 s before it.
        let gap = track(120 * S, |t| (t < 20 * S).then_some(0.25));
        assert!((clip_focus(&gap, 40 * S, 45 * S).unwrap().x - 0.25).abs() < 1e-6);
        // Too far from any face, or none at all: centred.
        assert!(clip_focus(&gap, 100 * S, 110 * S).is_none());
        assert!(clip_focus(&track(60 * S, |_| None), 0, 60 * S).is_none());
    }
}
