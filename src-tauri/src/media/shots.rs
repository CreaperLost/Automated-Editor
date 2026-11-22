//! Shot changes in footage from elsewhere (FTG-7): FFmpeg's `scdet` on a small copy of the
//! picture. Used for footage boards (one frame per shot) and to tell the agent where shots
//! start, so B-roll in and out points don't land inside a cut.
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::SystemTime;

/// `scdet`'s score (0 to 100) at which a frame starts a new shot. 10 finds hard cuts and
/// leaves camera moves and screen scrolling alone.
const THRESHOLD: f64 = 10.0;

type Key = (PathBuf, u64, Option<SystemTime>);
static CACHE: OnceLock<Mutex<HashMap<Key, Vec<u64>>>> = OnceLock::new();

/// The times (µs from the file's start) where a new shot starts, first shot not included.
pub fn detect(path: &Path) -> Result<Vec<u64>, String> {
    let metadata = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let key = (path.to_path_buf(), metadata.len(), metadata.modified().ok());
    let cache = CACHE.get_or_init(Default::default);
    if let Some(hit) = cache.lock().get(&key) {
        return Ok(hit.clone());
    }
    let mut command = Command::new(super::ffmpeg::ffmpeg_path()?);
    crate::graphics::runtime::hide_window(&mut command);
    let output = command
        .args(["-hide_banner", "-nostdin", "-i"])
        .arg(path)
        .args([
            "-map",
            "0:v:0",
            "-an",
            "-vf",
            &format!("scale=256:-2,scdet=t={THRESHOLD}"),
            "-f",
            "null",
            "-",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .output()
        .map_err(|e| format!("Shot detection could not start: {e}"))?;
    let log = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "Shot detection failed: {}",
            log.lines().last().unwrap_or("").trim()
        ));
    }
    let shots = parse(&log);
    cache.lock().insert(key, shots.clone());
    Ok(shots)
}

fn parse(log: &str) -> Vec<u64> {
    let mut shots: Vec<u64> = log
        .split("lavfi.scd.time:")
        .skip(1)
        .filter_map(|rest| {
            let number: String = rest
                .trim_start()
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            number.parse::<f64>().ok()
        })
        .filter(|t| t.is_finite() && *t > 0.0)
        .map(|t| (t * 1e6).round() as u64)
        .collect();
    shots.sort_unstable();
    shots.dedup();
    shots
}

/// Where to show one frame of each shot of `duration_us` footage, `at_most` of them: the
/// middle of each, the longest shots first when there are more.
pub fn board_times(starts: &[u64], duration_us: u64, at_most: usize) -> Vec<u64> {
    let mut edges = vec![0];
    edges.extend(starts.iter().copied().filter(|&t| t < duration_us));
    edges.push(duration_us);
    let mut shots: Vec<(u64, u64)> = edges.windows(2).map(|w| (w[0], w[1])).collect();
    if shots.len() > at_most {
        shots.sort_by_key(|(a, b)| std::cmp::Reverse(b - a));
        shots.truncate(at_most);
        shots.sort_unstable();
    }
    shots.into_iter().map(|(a, b)| a + (b - a) / 2).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_log_is_read_and_one_frame_is_shown_per_shot() {
        let log = "frame=0 [scdet @ 0x1] lavfi.scd.score: 28.2, lavfi.scd.time: 3\n\
                   [scdet @ 0x1] lavfi.scd.score: 27.4, lavfi.scd.time: 6.033\n";
        assert_eq!(parse(log), vec![3_000_000, 6_033_000]);
        assert_eq!(
            board_times(&[3_000_000, 6_000_000], 8_000_000, 12),
            vec![1_500_000, 4_500_000, 7_000_000]
        );
        // More shots than tiles: the longest are shown, in time order.
        assert_eq!(
            board_times(&[1_000_000, 1_500_000, 6_000_000], 8_000_000, 2),
            vec![3_750_000, 7_000_000]
        );
        assert_eq!(board_times(&[], 4_000_000, 12), vec![2_000_000]);
    }

    #[test]
    fn hard_cuts_are_found_in_real_footage() {
        if super::super::ffmpeg::ffmpeg_path().is_err() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("three shots.mp4");
        let made = Command::new(super::super::ffmpeg::ffmpeg_path().unwrap())
            .args(["-v", "error", "-y"])
            .args([
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=320x180:rate=30:duration=2",
            ])
            .args([
                "-f",
                "lavfi",
                "-i",
                "smptebars=size=320x180:rate=30:duration=2",
            ])
            .args(["-f", "lavfi", "-i", "mandelbrot=size=320x180:rate=30"])
            .args([
                "-filter_complex",
                "[0][1][2]concat=n=3:v=1:a=0[v];[v]trim=duration=5[o]",
                "-map",
                "[o]",
                "-c:v",
                "libx264",
            ])
            .arg(&path)
            .status()
            .unwrap();
        assert!(made.success());
        let shots = detect(&path).unwrap();
        assert_eq!(shots.len(), 2, "{shots:?}");
        assert!(shots[0].abs_diff(2_000_000) < 100_000 && shots[1].abs_diff(4_000_000) < 100_000);
    }
}
