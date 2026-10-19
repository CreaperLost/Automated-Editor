//! What the hands did in a recording, read once from its telemetry: mouse presses, drags,
//! scrolls, where the pointer was, and typing with its caret. Bubble hiding and click sounds
//! both start from it.
use super::keys::TypingRun;
use super::reader::{CanonicalKind, TelemetryStream};
use std::path::Path;
use std::sync::Arc;

/// A mouse button going down on the recorded source.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Press {
    pub t_us: u64,
    pub x: f64,
    pub y: f64,
    /// `None` for an old recording's derived click (button unknown).
    pub button: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RecordingActivity {
    /// Button downs inside the source, in order. Old recordings' clicks stand in for downs
    /// when the stream has none.
    pub presses: Vec<Press>,
    /// A button held (down to up), in order.
    pub drags: Vec<(u64, u64)>,
    /// Scroll ticks with where the pointer was, when known.
    pub scrolls: Vec<(u64, Option<(f64, f64)>)>,
    /// The pointer on the source, (time, x, y), in order.
    pub pointer: Vec<(u64, f64, f64)>,
    /// Typing runs, from the keyboard stream.
    pub typing: Vec<TypingRun>,
}

fn on_source(x: f64, y: f64, inside: bool) -> bool {
    inside && (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y)
}

impl RecordingActivity {
    pub fn from_stream(stream: &TelemetryStream) -> Self {
        let mut out = Self::default();
        let mut clicks = Vec::new();
        let mut held: Option<u64> = None;
        for event in &stream.events {
            let t = event.t_us;
            match event.kind {
                CanonicalKind::Move {
                    norm_x,
                    norm_y,
                    inside_source,
                } => {
                    if on_source(norm_x, norm_y, inside_source) {
                        out.pointer.push((t, norm_x, norm_y));
                    }
                }
                CanonicalKind::ButtonDown {
                    button,
                    norm_x,
                    norm_y,
                    inside_source,
                } => {
                    if on_source(norm_x, norm_y, inside_source) {
                        out.pointer.push((t, norm_x, norm_y));
                        out.presses.push(Press {
                            t_us: t,
                            x: norm_x,
                            y: norm_y,
                            button,
                        });
                    }
                    held.get_or_insert(t);
                }
                CanonicalKind::ButtonUp {
                    norm_x,
                    norm_y,
                    inside_source,
                    ..
                } => {
                    if on_source(norm_x, norm_y, inside_source) {
                        out.pointer.push((t, norm_x, norm_y));
                    }
                    if let Some(start) = held.take() {
                        out.drags.push((start, t));
                    }
                }
                CanonicalKind::Click {
                    norm_x,
                    norm_y,
                    inside_source,
                } => {
                    if on_source(norm_x, norm_y, inside_source) {
                        out.pointer.push((t, norm_x, norm_y));
                        clicks.push(Press {
                            t_us: t,
                            x: norm_x,
                            y: norm_y,
                            button: None,
                        });
                    }
                }
                CanonicalKind::Scroll {
                    norm_x,
                    norm_y,
                    inside_source,
                } => {
                    let at = match (norm_x, norm_y) {
                        (Some(x), Some(y)) if on_source(x, y, inside_source.unwrap_or(true)) => {
                            Some((x, y))
                        }
                        _ => None,
                    };
                    out.scrolls.push((t, at));
                }
                // The state of the buttons is unknown across a gap.
                CanonicalKind::Gap { .. } => held = None,
                CanonicalKind::Note => {}
            }
        }
        if out.presses.is_empty() {
            out.presses = clicks;
        }
        out.presses.sort_by_key(|p| p.t_us);
        out.pointer.sort_by_key(|p| p.0);
        out.scrolls.sort_by_key(|s| s.0);
        out.drags.sort_unstable();
        if let Some(keys) = &stream.keys {
            out.typing = keys.typing.clone();
        }
        out
    }

    /// Stretches where the pointer stayed within `radius` (of the source) for at least
    /// `min_us`: (start, end, x, y), with the place it rested.
    pub fn rests(&self, radius: f64, min_us: u64) -> Vec<(u64, u64, f64, f64)> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < self.pointer.len() {
            let (start, x, y) = self.pointer[i];
            let mut j = i + 1;
            while j < self.pointer.len() {
                let (_, px, py) = self.pointer[j];
                if (px - x).hypot(py - y) > radius {
                    break;
                }
                j += 1;
            }
            // It rested until the move that left (or the last sample seen).
            let end = match self.pointer.get(j) {
                Some(&(t, _, _)) => t,
                None => self.pointer[j - 1].0,
            };
            if end.saturating_sub(start) >= min_us {
                out.push((start, end, x, y));
            }
            i = j;
        }
        out
    }
}

/// The activity of the recording in `folder`, read once while its telemetry is unchanged.
pub fn recording_activity(folder: &Path) -> Arc<RecordingActivity> {
    use std::sync::{Mutex, OnceLock};
    type Cache = Mutex<
        std::collections::HashMap<
            std::path::PathBuf,
            ([Option<std::time::SystemTime>; 2], Arc<RecordingActivity>),
        >,
    >;
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let modified = ["events.jsonl", "keys.jsonl"].map(|name| {
        std::fs::metadata(folder.join("telemetry").join(name))
            .and_then(|m| m.modified())
            .ok()
    });
    let cache = CACHE.get_or_init(Default::default);
    if let Some((stamp, activity)) = cache.lock().unwrap_or_else(|e| e.into_inner()).get(folder) {
        if *stamp == modified {
            return activity.clone();
        }
    }
    let activity = Arc::new(
        super::reader::read_telemetry(folder)
            .map(|stream| RecordingActivity::from_stream(&stream))
            .unwrap_or_default(),
    );
    cache
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(folder.to_path_buf(), (modified, activity.clone()));
    activity
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pointer(samples: &[(u64, f64, f64)]) -> RecordingActivity {
        RecordingActivity {
            pointer: samples.to_vec(),
            ..Default::default()
        }
    }

    #[test]
    fn a_rest_lasts_until_the_pointer_leaves() {
        let activity = pointer(&[
            (0, 0.5, 0.5),
            (400_000, 0.501, 0.5),
            (900_000, 0.5, 0.502),
            (1_000_000, 0.6, 0.5),
            (1_100_000, 0.7, 0.5),
        ]);
        assert_eq!(
            activity.rests(0.01, 600_000),
            vec![(0, 1_000_000, 0.5, 0.5)]
        );
        // Too short a rest is not one.
        assert!(activity.rests(0.01, 1_200_000).is_empty());
    }
}
