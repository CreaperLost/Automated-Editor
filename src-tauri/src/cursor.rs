//! The recorded mouse pointer, drawn by the editor.
//!
//! A recorder in `replace` cursor mode leaves the pointer out of the video and writes where it
//! was (`telemetry/events.jsonl`), what it looked like (`cursor_changed` records) and its
//! pictures (`telemetry/cursors/<id>.png`). The pointer is drawn back from those, so it moves
//! smoothly, follows the zooms and can be resized or hidden. Recordings with the pointer baked
//! into the video have nothing to draw.
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// Without a move for longer than this, the pointer was standing still.
const STILL_AFTER_US: u64 = 150_000;
/// Half-width of the smoothing window: removes the jitter of 100 ms sampling.
const SMOOTH_US: f64 = 45_000.0;
/// A click dips the pointer this much and springs back over [`CLICK_US`].
const CLICK_DIP: f64 = 0.18;
const CLICK_US: u64 = 220_000;
/// Most events read from one recording.
const MAX_EVENTS: usize = 2_000_000;

#[derive(Clone, Debug, PartialEq)]
pub struct CursorShape {
    pub id: String,
    /// Where the tip is in the picture, in source pixels.
    pub hotspot_x: f64,
    pub hotspot_y: f64,
    /// The picture's size in source pixels.
    pub width: f64,
    pub height: f64,
}

/// The pointer at one moment.
#[derive(Clone, Debug, PartialEq)]
pub struct CursorPose {
    /// Position in the recorded screen, 0..1.
    pub x: f64,
    pub y: f64,
    pub shape: CursorShape,
    /// Size factor for the click dip: 1 at rest.
    pub scale: f64,
}

#[derive(Clone, Debug, Default)]
pub struct CursorTrack {
    folder: PathBuf,
    /// (time, x, y, inside the recorded screen), in time order.
    moves: Vec<(u64, f64, f64, bool)>,
    shapes: Vec<(u64, CursorShape)>,
    downs: Vec<u64>,
    /// The recorded screen's size in pixels.
    pub source_width: f64,
    pub source_height: f64,
}

impl CursorTrack {
    /// The pointer of the recording in `folder`, or `None` when it is baked into the video or
    /// was not recorded.
    pub fn load(folder: &Path) -> Result<Option<Self>, String> {
        let telemetry = folder.join("telemetry");
        let Ok(geometry) = std::fs::File::open(telemetry.join("geometry.jsonl")) else {
            return Ok(None);
        };
        let mut replaced = false;
        let (mut source_width, mut source_height) = (0.0, 0.0);
        for line in BufReader::new(geometry).lines().map_while(Result::ok) {
            let Ok(record) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if record["cursor_mode"].as_str() == Some("replace") {
                replaced = true;
            }
            if source_width == 0.0 {
                source_width = record["output_width"].as_f64().unwrap_or(0.0);
                source_height = record["output_height"].as_f64().unwrap_or(0.0);
            }
        }
        if !replaced || source_width <= 0.0 || source_height <= 0.0 {
            return Ok(None);
        }
        let Ok(events) = std::fs::File::open(telemetry.join("events.jsonl")) else {
            return Ok(None);
        };
        let mut track = Self {
            folder: telemetry.join("cursors"),
            source_width,
            source_height,
            ..Self::default()
        };
        for line in BufReader::new(events)
            .lines()
            .map_while(Result::ok)
            .take(MAX_EVENTS)
        {
            let Ok(record) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            let Some(t) = record["t_us"].as_u64() else {
                continue;
            };
            let payload = &record["payload"];
            let position = || {
                Some((
                    record["norm_x"].as_f64()?,
                    record["norm_y"].as_f64()?,
                    record["inside_source"].as_bool().unwrap_or(true),
                ))
            };
            match payload["kind"].as_str() {
                Some("move") | Some("button_up") | Some("scroll") => {
                    if let Some((x, y, inside)) = position() {
                        track.moves.push((t, x, y, inside));
                    }
                }
                Some("button_down") => {
                    if let Some((x, y, inside)) = position() {
                        track.moves.push((t, x, y, inside));
                    }
                    track.downs.push(t);
                }
                Some("cursor_changed") => {
                    let Some(id) = payload["cursor_id"].as_str() else {
                        continue;
                    };
                    // Ids name a file in this folder: nothing that could leave it.
                    if id.is_empty()
                        || !id
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                    {
                        continue;
                    }
                    track.shapes.push((
                        t,
                        CursorShape {
                            id: id.to_string(),
                            hotspot_x: payload["hotspot_x"].as_f64().unwrap_or(0.0),
                            hotspot_y: payload["hotspot_y"].as_f64().unwrap_or(0.0),
                            width: payload["width"].as_f64().unwrap_or(32.0).clamp(4.0, 256.0),
                            height: payload["height"].as_f64().unwrap_or(32.0).clamp(4.0, 256.0),
                        },
                    ));
                }
                _ => {}
            }
        }
        track.moves.sort_by_key(|m| m.0);
        track.shapes.sort_by_key(|s| s.0);
        track.downs.sort_unstable();
        if track.moves.is_empty() || track.shapes.is_empty() {
            return Ok(None);
        }
        Ok(Some(track))
    }

    /// The picture file of a pointer shape.
    pub fn image_path(&self, shape: &CursorShape) -> PathBuf {
        self.folder.join(format!("{}.png", shape.id))
    }

    /// Where the pointer was at `t_us`, unsmoothed: it stands still between samples far
    /// apart, and moves straight between close ones.
    fn raw_at(&self, t_us: f64) -> (f64, f64, bool) {
        let index = self.moves.partition_point(|m| (m.0 as f64) <= t_us);
        if index == 0 {
            let first = self.moves[0];
            return (first.1, first.2, first.3);
        }
        let a = self.moves[index - 1];
        let Some(&b) = self.moves.get(index) else {
            return (a.1, a.2, a.3);
        };
        // A long wait before `b`: still until just before it, then the move.
        let start = (a.0 as f64).max(b.0 as f64 - STILL_AFTER_US as f64);
        if t_us <= start {
            return (a.1, a.2, a.3);
        }
        let k = ((t_us - start) / (b.0 as f64 - start).max(1.0)).clamp(0.0, 1.0);
        (a.1 + (b.1 - a.1) * k, a.2 + (b.2 - a.2) * k, a.3 || b.3)
    }

    /// The pointer at `t_us`, or `None` while it is outside the recorded screen.
    pub fn at(&self, t_us: u64) -> Option<CursorPose> {
        let t = t_us as f64;
        let (_, _, inside) = self.raw_at(t);
        if !inside {
            return None;
        }
        // A small Gaussian average along the path: smooth, and the same on every play.
        const TAPS: [f64; 7] = [-1.0, -2.0 / 3.0, -1.0 / 3.0, 0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0];
        let (mut x, mut y, mut total) = (0.0, 0.0, 0.0);
        for tap in TAPS {
            let weight = (-2.0 * tap * tap).exp();
            let (px, py, _) = self.raw_at(t + tap * SMOOTH_US * 2.0);
            x += px * weight;
            y += py * weight;
            total += weight;
        }
        let shape_index = self.shapes.partition_point(|s| s.0 <= t_us);
        let shape = self.shapes[shape_index.saturating_sub(1)].1.clone();
        let since_down = self
            .downs
            .get(self.downs.partition_point(|&d| d <= t_us).wrapping_sub(1))
            .map(|&d| t_us - d);
        let scale = match since_down {
            Some(age) if age < CLICK_US => {
                let k = age as f64 / CLICK_US as f64;
                1.0 - CLICK_DIP * (std::f64::consts::PI * k).sin()
            }
            _ => 1.0,
        };
        Some(CursorPose {
            x: x / total,
            y: y / total,
            shape,
            scale,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_recording(dir: &Path, mode: &str) {
        let telemetry = dir.join("telemetry");
        std::fs::create_dir_all(&telemetry).unwrap();
        std::fs::write(
            telemetry.join("geometry.jsonl"),
            format!(r#"{{"version":2,"geometry_id":"g","t_us":0,"output_width":1920,"output_height":1080,"cursor_mode":"{mode}"}}"#),
        )
        .unwrap();
        let lines = [
            r#"{"version":2,"seq":0,"t_us":0,"payload":{"kind":"cursor_changed","cursor_id":"c1","name":"arrow","hotspot_x":2.0,"hotspot_y":3.0,"width":32.0,"height":32.0}}"#,
            r#"{"version":2,"seq":1,"t_us":1000000,"norm_x":0.2,"norm_y":0.2,"inside_source":true,"payload":{"kind":"move"}}"#,
            r#"{"version":2,"seq":2,"t_us":3000000,"norm_x":0.6,"norm_y":0.4,"inside_source":true,"payload":{"kind":"move"}}"#,
            r#"{"version":2,"seq":3,"t_us":3100000,"norm_x":0.6,"norm_y":0.4,"inside_source":true,"payload":{"kind":"button_down"}}"#,
            r#"{"version":2,"seq":4,"t_us":5000000,"norm_x":1.4,"norm_y":0.4,"inside_source":false,"payload":{"kind":"move"}}"#,
        ];
        std::fs::write(telemetry.join("events.jsonl"), lines.join("\n")).unwrap();
    }

    #[test]
    fn the_pointer_waits_moves_smoothly_and_dips_on_a_click() {
        let dir = tempfile::tempdir().unwrap();
        write_recording(dir.path(), "replace");
        let track = CursorTrack::load(dir.path()).unwrap().unwrap();
        assert_eq!((track.source_width, track.source_height), (1920.0, 1080.0));
        // Still between the two moves until just before the second one.
        let waiting = track.at(2_500_000).unwrap();
        assert!((waiting.x - 0.2).abs() < 1e-9 && (waiting.y - 0.2).abs() < 1e-9);
        // On the way: between the two points, and continuous.
        let moving = track.at(2_930_000).unwrap();
        assert!(moving.x > 0.2 && moving.x < 0.6);
        let next = track.at(2_940_000).unwrap();
        assert!((next.x - moving.x).abs() < 0.05);
        assert_eq!(track.at(3_000_000).unwrap().shape.hotspot_y, 3.0);
        assert!(track.at(3_210_000).unwrap().scale < 0.9);
        assert_eq!(track.at(3_400_000).unwrap().scale, 1.0);
        // Off the recorded screen: nothing to draw.
        assert!(track.at(6_000_000).is_none());
        assert!(track.image_path(&waiting.shape).ends_with("c1.png"));
    }

    #[test]
    fn a_baked_pointer_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        write_recording(dir.path(), "baked");
        assert!(CursorTrack::load(dir.path()).unwrap().is_none());
        assert!(CursorTrack::load(&dir.path().join("missing"))
            .unwrap()
            .is_none());
    }
}
