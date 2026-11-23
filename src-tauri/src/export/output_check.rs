//! The exported file checked by itself: one pass of FFmpeg over what was written measures
//! what only the file shows (its true peak and loudness, long stretches with no sound, black
//! picture, a picture that holds still with no sound, and fast flashing), so a problem the
//! person can't see or hear is still named. The edit's own check (`crate::check`) runs before.
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};

/// Above this a peak can distort once a player or a site re-encodes the sound.
const TRUE_PEAK_MAX_DB: f64 = -1.0;
/// How far the loudness may land from the target before it is named.
const LOUDNESS_SLACK_LU: f64 = 1.5;
/// Quieter than this with no target set and the video sounds quiet next to others.
const QUIET_LUFS: f64 = -20.0;
/// No sound for this long is named.
const SILENCE_MIN_S: f64 = 3.0;
/// Black picture for this long is named, unless it is at the very start or end (a fade).
const BLACK_MIN_S: f64 = 0.5;
const EDGE_S: f64 = 1.0;
/// A picture that holds still at least this long during a silence is named with it.
const STILL_MIN_S: f64 = 2.0;
/// A change in average brightness this big (of 0 to 255) from one frame to the frame after
/// next is a flash's edge.
const FLASH_STEP: f64 = 40.0;
/// More than three flashes (an on and an off each) in any second is a seizure risk (the
/// three-flash rule of WCAG 2.3.1).
const FLASH_EDGES_PER_S: usize = 7;
/// Of each kind, at most this many are listed; the rest are counted in the last.
const MAX_LISTED: usize = 5;

/// Something the file check found.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    pub title: String,
    pub detail: String,
    /// Where it starts in the file; none when it is about the whole file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at_us: Option<u64>,
}

/// What one pass over the file measured.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Measured {
    pub duration_s: f64,
    pub integrated_lufs: Option<f64>,
    pub true_peak_db: Option<f64>,
    pub silences: Vec<(f64, f64)>,
    pub stills: Vec<(f64, f64)>,
    pub blacks: Vec<(f64, f64)>,
    /// Each frame's time and average brightness.
    pub luma: Vec<(f64, f64)>,
    pub has_sound: bool,
    pub has_picture: bool,
}

fn clock(s: f64) -> String {
    crate::check::clock((s.max(0.0) * 1e6) as u64)
}

fn us(s: f64) -> Option<u64> {
    Some((s.max(0.0) * 1e6) as u64)
}

/// Reads FFmpeg's log lines into `m`; spans still open at the end run to `duration_s`.
fn read_log(m: &mut Measured, log: &str) {
    let mut open_silence = None;
    let mut open_still = None;
    let mut in_peak = false;
    let number = |text: &str| -> Option<f64> {
        text.split_whitespace()
            .next()?
            .trim_end_matches(',')
            .parse()
            .ok()
    };
    for line in log.lines() {
        if let Some(rest) = line.split("silence_start: ").nth(1) {
            open_silence = number(rest);
        } else if let Some(rest) = line.split("silence_end: ").nth(1) {
            if let (Some(start), Some(end)) = (open_silence.take(), number(rest)) {
                m.silences.push((start, end));
            }
        } else if let Some(rest) = line.split("freeze_start: ").nth(1) {
            open_still = number(rest);
        } else if let Some(rest) = line.split("freeze_end: ").nth(1) {
            if let (Some(start), Some(end)) = (open_still.take(), number(rest)) {
                m.stills.push((start, end));
            }
        } else if let Some(rest) = line.split("black_start:").nth(1) {
            let end = line.split("black_end:").nth(1).and_then(number);
            if let (Some(start), Some(end)) = (number(rest), end) {
                m.blacks.push((start, end));
            }
        } else if line.trim_start().starts_with("I:") {
            m.integrated_lufs = number(line.trim_start().trim_start_matches("I:"));
        } else if line.contains("True peak:") {
            in_peak = true;
        } else if in_peak && line.trim_start().starts_with("Peak:") {
            m.true_peak_db = number(line.trim_start().trim_start_matches("Peak:"));
            in_peak = false;
        }
    }
    if let Some(start) = open_silence {
        m.silences.push((start, m.duration_s));
    }
    if let Some(start) = open_still {
        m.stills.push((start, m.duration_s));
    }
}

/// Measures the file at `path` in one decoding pass, stopping early when `cancel` is set.
pub fn measure(path: &Path, cancel: &AtomicBool) -> Result<Measured, String> {
    use crate::media::ffmpeg;
    let mut m = Measured {
        duration_s: ffmpeg::duration_us(path)? as f64 / 1e6,
        has_picture: ffmpeg::probe_video(path).is_ok(),
        has_sound: ffmpeg::audio_stream_names(path).is_ok_and(|n| !n.is_empty()),
        ..Default::default()
    };
    let mut command = std::process::Command::new(ffmpeg::ffmpeg_path()?);
    command.args([
        "-nostdin",
        "-hide_banner",
        "-nostats",
        "-loglevel",
        "info",
        "-i",
    ]);
    command.arg(path);
    if m.has_picture {
        // Small frames are enough for brightness and stillness, and quick to scan.
        command.args([
            "-map",
            "0:v:0",
            "-vf",
            "scale=160:-2:flags=fast_bilinear,freezedetect=n=0.003:d=2,\
             blackdetect=d=0.5:pix_th=0.1,signalstats,\
             metadata=mode=print:key=lavfi.signalstats.YAVG:file=-",
        ]);
    }
    if m.has_sound {
        command.args([
            "-map",
            "0:a:0",
            "-af",
            "silencedetect=n=-50dB:d=3,ebur128=peak=true:framelog=quiet",
        ]);
    }
    if !m.has_picture && !m.has_sound {
        return Ok(m);
    }
    command.args(["-f", "null", "-"]);
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not run FFmpeg: {e}"))?;
    let mut stderr = child.stderr.take().expect("piped");
    let log = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = stderr.read_to_string(&mut text);
        text
    });
    let stdout = BufReader::new(child.stdout.take().expect("piped"));
    let mut at = None;
    for line in stdout.lines() {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Cancelled".into());
        }
        let Ok(line) = line else { break };
        if let Some(t) = line.split("pts_time:").nth(1) {
            at = t.trim().parse::<f64>().ok();
        } else if let Some(y) = line.strip_prefix("lavfi.signalstats.YAVG=") {
            if let (Some(t), Ok(y)) = (at.take(), y.trim().parse::<f64>()) {
                m.luma.push((t, y));
            }
        }
    }
    let status = child
        .wait()
        .map_err(|e| format!("Could not run FFmpeg: {e}"))?;
    let log = log.join().unwrap_or_default();
    if !status.success() {
        let last = log
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("");
        return Err(format!("FFmpeg could not read the file: {}", last.trim()));
    }
    read_log(&mut m, &log);
    Ok(m)
}

/// Where brightness flashes faster than the three-flash rule allows: each stretch's start
/// and end.
fn flashing(luma: &[(f64, f64)]) -> Vec<(f64, f64)> {
    // A flash's edges: big changes in brightness, each the other way from the last.
    let mut edges: Vec<f64> = Vec::new();
    let mut last_rising: Option<bool> = None;
    let mut i = 2;
    while i < luma.len() {
        let step = luma[i].1 - luma[i - 2].1;
        let rising = step > 0.0;
        if step.abs() >= FLASH_STEP && last_rising != Some(rising) {
            edges.push(luma[i].0);
            last_rising = Some(rising);
            // One edge is one change, however many frames it takes.
            i += 2;
            continue;
        }
        i += 1;
    }
    let mut spans: Vec<(f64, f64)> = Vec::new();
    for (k, &start) in edges.iter().enumerate() {
        let within = edges[k..].iter().take_while(|&&t| t - start < 1.0).count();
        if within < FLASH_EDGES_PER_S {
            continue;
        }
        let end = edges[k + within - 1];
        match spans.last_mut() {
            Some(last) if start <= last.1 + 0.5 => last.1 = last.1.max(end),
            _ => spans.push((start, end)),
        }
    }
    spans
}

fn listed(found: &mut Vec<Found>, mut items: Vec<Found>, what: &str) {
    let more = items.len().saturating_sub(MAX_LISTED);
    items.truncate(MAX_LISTED);
    if more > 0 {
        if let Some(last) = items.last_mut() {
            last.detail
                .push_str(&format!(" {more} more {what} like this come after."));
        }
    }
    found.extend(items);
}

/// What `m` shows that a viewer would notice, given the loudness target when one is set.
pub fn findings(m: &Measured, target_lufs: Option<f32>) -> Vec<Found> {
    let mut found = Vec::new();
    if let Some(peak) = m.true_peak_db.filter(|p| *p > TRUE_PEAK_MAX_DB) {
        found.push(Found {
            title: "The sound peaks too high".into(),
            detail: format!(
                "Its true peak is {peak:.1} dBTP, over {TRUE_PEAK_MAX_DB:.0}: it can distort \
                 when a player or site re-encodes it. Lower the loudest part or the target."
            ),
            at_us: None,
        });
    }
    if let Some(lufs) = m.integrated_lufs.filter(|l| l.is_finite()) {
        match target_lufs {
            Some(target) if (lufs - target as f64).abs() > LOUDNESS_SLACK_LU => found.push(Found {
                title: "The loudness missed the target".into(),
                detail: format!(
                    "The file is at {lufs:.1} LUFS; the target is {target:.0}. Export again, \
                         or check what limits the sound."
                ),
                at_us: None,
            }),
            None if lufs < QUIET_LUFS => found.push(Found {
                title: "The video is quiet".into(),
                detail: format!(
                    "The file is at {lufs:.1} LUFS. YouTube never turns a quiet video up: turn \
                     on loudness normalization at -14."
                ),
                at_us: None,
            }),
            _ => {}
        }
    }
    let mut quiet = Vec::new();
    for &(start, end) in &m.silences {
        if end - start < SILENCE_MIN_S {
            continue;
        }
        let still = m
            .stills
            .iter()
            .any(|&(a, b)| b.min(end) - a.max(start) >= STILL_MIN_S);
        quiet.push(Found {
            title: if still {
                "Nothing moves and nothing is heard".into()
            } else {
                "No sound".into()
            },
            detail: format!(
                "From {} to {} ({:.0} s){}.",
                clock(start),
                clock(end),
                end - start,
                if still {
                    ": the picture holds still too"
                } else {
                    ""
                }
            ),
            at_us: us(start),
        });
    }
    listed(&mut found, quiet, "silences");
    let blacks = m
        .blacks
        .iter()
        .filter(|&&(start, end)| end - start >= BLACK_MIN_S)
        .filter(|&&(start, end)| start > EDGE_S && end < m.duration_s - EDGE_S)
        .map(|&(start, end)| Found {
            title: "Black picture".into(),
            detail: format!(
                "From {} to {} ({:.1} s).",
                clock(start),
                clock(end),
                end - start
            ),
            at_us: us(start),
        })
        .collect();
    listed(&mut found, blacks, "black stretches");
    let flashes = flashing(&m.luma)
        .into_iter()
        .map(|(start, end)| Found {
            title: "The picture flashes fast".into(),
            detail: format!(
                "From {} to {} it flashes more than three times a second, which can set off \
                 seizures. Slow it down or make the change in brightness smaller.",
                clock(start),
                clock(end)
            ),
            at_us: us(start),
        })
        .collect();
    listed(&mut found, flashes, "flashing stretches");
    found.sort_by_key(|f| f.at_us);
    found
}

/// Checks the file at `path`, exported from `document`.
pub fn check(
    path: &Path,
    document: &crate::project::revision::EditDocument,
    cancel: &AtomicBool,
) -> Result<Vec<Found>, String> {
    let measured = measure(path, cancel)?;
    let audio = &document.audio;
    Ok(findings(
        &measured,
        audio.normalize.then_some(audio.target_lufs),
    ))
}

/// One line per finding, for a log or an agent.
pub fn lines(found: &[Found]) -> Vec<String> {
    found
        .iter()
        .map(|f| format!("{}: {}", f.title, f.detail))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_log_reads_into_spans_loudness_and_peak() {
        let mut m = Measured {
            duration_s: 10.0,
            ..Default::default()
        };
        read_log(
            &mut m,
            "[blackdetect @ 0x1] black_start:2 black_end:3 black_duration:1\n\
             [freezedetect @ 0x2] lavfi.freezedetect.freeze_start: 3\n\
             [silencedetect @ 0x3] silence_start: 3.5\n\
             [silencedetect @ 0x3] silence_end: 7.25 | silence_duration: 3.75\n\
             [silencedetect @ 0x3] silence_start: 9\n\
             \x20 Integrated loudness:\n    I:         -22.0 LUFS\n    Threshold: -32.0 LUFS\n\
             \x20 True peak:\n    Peak:      -0.4 dBFS\n",
        );
        assert_eq!(m.blacks, vec![(2.0, 3.0)]);
        assert_eq!(m.stills, vec![(3.0, 10.0)]);
        assert_eq!(m.silences, vec![(3.5, 7.25), (9.0, 10.0)]);
        assert_eq!(m.integrated_lufs, Some(-22.0));
        assert_eq!(m.true_peak_db, Some(-0.4));
        let found = findings(&m, Some(-14.0));
        let titles: Vec<&str> = found.iter().map(|f| f.title.as_str()).collect();
        assert_eq!(
            titles,
            [
                "The sound peaks too high",
                "The loudness missed the target",
                "Black picture",
                "Nothing moves and nothing is heard"
            ],
            "{found:?}"
        );
        // A short silence at the end and a fade at the start are fine.
        assert!(
            found[3].detail.starts_with("From 0:03.5 to 0:07.2"),
            "{found:?}"
        );
        // With no target only a quiet file is named.
        let found = findings(&m, None);
        assert!(found.iter().any(|f| f.title == "The video is quiet"));
    }

    #[test]
    fn fast_flashing_is_found_and_a_single_cut_is_not() {
        let frames = |f: &dyn Fn(usize) -> f64| -> Vec<(f64, f64)> {
            (0..150).map(|i| (i as f64 / 30.0, f(i))).collect()
        };
        // Black and white every 3 frames: 5 flashes a second, from 1 s to 3 s.
        let strobe = frames(&|i| {
            if (30..90).contains(&i) && (i / 3) % 2 == 0 {
                235.0
            } else {
                16.0
            }
        });
        let spans = flashing(&strobe);
        assert_eq!(spans.len(), 1, "{spans:?}");
        assert!(spans[0].0 < 1.2 && spans[0].1 > 2.5, "{spans:?}");
        // A hard cut from dark to light and back once is no flash.
        let cut = frames(&|i| if (40..100).contains(&i) { 235.0 } else { 16.0 });
        assert!(flashing(&cut).is_empty());
        // Two flashes a second stay under the rule.
        let slow = frames(&|i| if (i / 15) % 2 == 0 { 235.0 } else { 16.0 });
        assert!(flashing(&slow).is_empty());
    }

    /// A real file: a test picture, a second of black, then a still red frame while the sound
    /// stops for 4 s. The pass finds the black, and the still silence with its picture.
    #[test]
    fn a_written_file_is_measured_in_one_pass() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("out.mp4");
        let made = std::process::Command::new(crate::media::ffmpeg::ffmpeg_path().unwrap())
            .args(["-nostdin", "-loglevel", "error", "-y"])
            .args(["-f", "lavfi", "-i", "testsrc=s=320x180:r=30:d=2"])
            .args(["-f", "lavfi", "-i", "color=black:s=320x180:r=30:d=1"])
            .args(["-f", "lavfi", "-i", "color=red:s=320x180:r=30:d=5"])
            .args(["-f", "lavfi", "-i", "sine=f=440:d=4"])
            .args(["-f", "lavfi", "-i", "anullsrc=r=48000:cl=mono:d=4"])
            .args([
                "-filter_complex",
                "[0][1][2]concat=n=3:v=1:a=0[v];[3][4]concat=n=2:v=0:a=1[a]",
                "-map",
                "[v]",
                "-map",
                "[a]",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
            ])
            .arg(&file)
            .status()
            .unwrap();
        assert!(made.success());
        let m = measure(&file, &AtomicBool::new(false)).unwrap();
        assert!(m.has_picture && m.has_sound);
        assert!(m.luma.len() > 200, "{}", m.luma.len());
        assert!(
            m.integrated_lufs.is_some() && m.true_peak_db.is_some(),
            "{m:?}"
        );
        let found = findings(&m, None);
        assert!(
            found.iter().any(|f| f.title == "Black picture"),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|f| f.title == "Nothing moves and nothing is heard"),
            "{found:?}"
        );
        assert!(found.iter().all(|f| f.title != "The picture flashes fast"));
        assert!(matches!(
            measure(&file, &AtomicBool::new(true)),
            Err(e) if e == "Cancelled"
        ));
    }

    /// `OUTPUT_CHECK_FILES=a.mp4:b.mp4 cargo test --lib output_check_probe -- --ignored
    /// --nocapture` prints what the check finds in each file and how long it took.
    #[test]
    #[ignore]
    fn output_check_probe() {
        let files = std::env::var("OUTPUT_CHECK_FILES").expect("OUTPUT_CHECK_FILES");
        for file in files.split(':') {
            let started = std::time::Instant::now();
            let m = measure(Path::new(file), &AtomicBool::new(false)).unwrap();
            println!(
                "{file}: {:.1} s checked in {:.1} s; {:?} LUFS, peak {:?} dBTP",
                m.duration_s,
                started.elapsed().as_secs_f64(),
                m.integrated_lufs,
                m.true_peak_db
            );
            for line in lines(&findings(&m, Some(-14.0))) {
                println!("  {line}");
            }
        }
    }
}
