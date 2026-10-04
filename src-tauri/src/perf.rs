//! Preview speed on a copy of a real recording, run by hand:
//! `AERO_PROBE=<dir> cargo test --release --lib perf -- --ignored --nocapture --test-threads=1`,
//! where `<dir>` holds `aero` (a recording). Plays the preview as the playback worker does
//! (720p, 30 fps, decoded no larger than the canvas, decoders started ahead of cuts), straight
//! and with many jump cuts, and times seeks, rebuilding after an edit, edits and the audio
//! mixer. `AERO_PERF_SLOW=1` lists the frames over 33 ms; `AERO_PERF_SETTLE=<s>` waits that
//! long after copying the recording, while antivirus scans the copy.
#![cfg(test)]

use crate::commands::{self, AppState};
use crate::export::SceneEvaluator;
use crate::media::audio::{AudioMixer, CHUNK_FRAMES};
use crate::playback::PreviewQuality;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn probe_dir() -> Option<PathBuf> {
    std::env::var_os("AERO_PROBE").map(PathBuf::from)
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

struct Stats(Vec<f64>);

impl Stats {
    fn line(&self, what: &str) -> String {
        let mut sorted = self.0.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = sorted.len().max(1);
        let mean = sorted.iter().sum::<f64>() / n as f64;
        let p = |q: f64| {
            sorted
                .get(((n - 1) as f64 * q) as usize)
                .copied()
                .unwrap_or(0.0)
        };
        let late = sorted.iter().filter(|&&t| t > 33.3).count();
        format!(
            "{what}: n={} mean={mean:.1}ms p50={:.1} p95={:.1} max={:.1} over33ms={late}",
            sorted.len(),
            p(0.5),
            p(0.95),
            p(1.0)
        )
    }
}

fn opened(work: &Path, dir: &Path) -> (AppState, String, PathBuf) {
    let recording = work.join("rec.aero");
    copy_dir(&dir.join("aero"), &recording);
    let _ = std::fs::remove_file(recording.join("project.json"));
    let folder =
        crate::project::folder::create_project_folder(work, "Perf", Some(&recording)).unwrap();
    let state = AppState::new();
    let opened = commands::open_project_impl(&state, folder.to_string_lossy().into()).unwrap();
    (state, opened.project_handle, folder)
}

fn document(state: &AppState) -> crate::project::revision::EditDocument {
    state
        .opened_project
        .lock()
        .as_ref()
        .unwrap()
        .history()
        .current
        .clone()
}

fn revision(state: &AppState) -> u64 {
    state
        .opened_project
        .lock()
        .as_ref()
        .unwrap()
        .summary
        .revision
}

/// What the playback worker builds after every seek or edit.
fn rebuild(
    root: &Path,
    document: &crate::project::revision::EditDocument,
    reuse: Option<crate::export::EvaluatorReuse>,
) -> (SceneEvaluator, AudioMixer) {
    let quality = PreviewQuality::default_for(true);
    let mixer = AudioMixer::new(root, document).unwrap();
    let (w, h) = document.layout.preview_dimensions().unwrap();
    let (w, h) = quality.canvas(w, h);
    let evaluator = SceneEvaluator::new_reusing(root.into(), document.clone(), w, h, reuse)
        .unwrap()
        .with_decode_limit(quality.decode_limit((w, h), true));
    (evaluator, mixer)
}

fn play(root: &Path, document: &crate::project::revision::EditDocument, label: &str) {
    crate::media::ffmpeg::release_decoders();
    let (mut evaluator, _) = rebuild(root, document, None);
    let quality = PreviewQuality::default_for(true);
    let duration = document.duration_us();
    let mut times = Vec::new();
    let started = Instant::now();
    let mut t = 0;
    // As the playback worker does: a frame on the clock's time, then decoders started ahead.
    while t < duration {
        let due = std::time::Duration::from_micros(t);
        if let Some(wait) = due.checked_sub(started.elapsed()) {
            std::thread::sleep(wait);
        }
        let frame_started = Instant::now();
        let at = quality.frame_time(t);
        evaluator.preview_at(at).unwrap();
        let ms = frame_started.elapsed().as_secs_f64() * 1000.0;
        if ms > 33.3 && std::env::var_os("AERO_PERF_SLOW").is_some() {
            println!(
                "PERF slow frame {label} at {:.3}s: {ms:.1}ms",
                at as f64 / 1e6
            );
        }
        times.push(ms);
        evaluator.prefetch(at, 1_200_000);
        t += 1_000_000 / 30;
    }
    println!("PERF {}", Stats(times).line(&format!("play {label}")));
}

fn seeks(root: &Path, document: &crate::project::revision::EditDocument, label: &str) {
    let duration = document.duration_us();
    let (mut evaluator, _) = rebuild(root, document, None);
    evaluator.preview_at(0).unwrap();
    let mut times = Vec::new();
    // Scrubbing: a run of nearby seeks forward, the same backward, then jumps.
    let mut positions: Vec<u64> = (0..20).map(|i| 3_000_000 + i * 70_000).collect();
    positions.extend((0..10).map(|i| 4_400_000 - i * 70_000));
    positions.extend((0..10).map(|i| (i * 7_919_000) % duration));
    for at in positions {
        let started = Instant::now();
        evaluator.preview_at(at).unwrap();
        times.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    println!("PERF {}", Stats(times).line(&format!("seek {label}")));
    // An edit: the scene is built again and the frame drawn where the playhead is.
    let mut reuse = evaluator.into_reuse();
    let mut times = Vec::new();
    for _ in 0..10 {
        let started = Instant::now();
        let (mut evaluator, _) = rebuild(root, document, reuse.take());
        evaluator.preview_at(4_000_000).unwrap();
        times.push(started.elapsed().as_secs_f64() * 1000.0);
        reuse = evaluator.into_reuse();
    }
    println!(
        "PERF {}",
        Stats(times).line(&format!("rebuild after edit {label}"))
    );
}

fn mix(root: &Path, document: &crate::project::revision::EditDocument, label: &str) {
    let mixer = AudioMixer::new(root, document).unwrap();
    let started = Instant::now();
    let mut frame = 0;
    let mut chunks = 0;
    while frame < mixer.total_frames {
        let chunk = mixer.read_frames(frame, CHUNK_FRAMES).unwrap();
        if chunk.is_empty() {
            break;
        }
        frame += (chunk.len() / 2) as u64;
        chunks += 1;
    }
    println!(
        "PERF mix {label}: {chunks} chunks in {:.1}ms ({:.3}ms/chunk)",
        started.elapsed().as_secs_f64() * 1000.0,
        started.elapsed().as_secs_f64() * 1000.0 / chunks.max(1) as f64
    );
}

/// Jump cuts every `every_us`, each `cut_us` long, as the pauses panel makes them.
fn jump_cut(state: &AppState, handle: &str, every_us: u64, cut_us: u64) {
    let duration = document(state).duration_us();
    let cuts: Vec<_> = (1..)
        .map(|i| i * every_us)
        .take_while(|at| at + cut_us < duration)
        .map(|at| commands::EditCut {
            start_us: at,
            end_us: at + cut_us,
        })
        .collect();
    let count = cuts.len();
    let started = Instant::now();
    commands::project_ripple_cuts_impl(state, handle.into(), revision(state), cuts).unwrap();
    println!(
        "PERF ripple {count} cuts in one edit: {:.1}ms",
        started.elapsed().as_secs_f64() * 1000.0
    );
}

fn edits(state: &AppState, handle: &str, label: &str) {
    use crate::sequence::edit::SequenceEdit;
    let mut times = Vec::new();
    for _ in 0..10 {
        let magnetic = document(state).sequence.magnetic;
        let started = Instant::now();
        commands::project_sequence_edit_impl(
            state,
            handle.into(),
            revision(state),
            SequenceEdit::SetMagnetic {
                magnetic: !magnetic,
            },
            None,
        )
        .unwrap();
        times.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    println!("PERF {}", Stats(times).line(&format!("edit {label}")));
}

#[test]
#[ignore]
fn perf_preview() {
    let Some(dir) = probe_dir() else { return };
    let work = tempfile::tempdir().unwrap();
    let (state, handle, root) = opened(work.path(), &dir);
    // Fresh copies get scanned by antivirus for a few seconds, which slows reading them.
    if let Some(settle) = std::env::var_os("AERO_PERF_SETTLE") {
        let seconds: u64 = settle.to_string_lossy().parse().unwrap_or(15);
        std::thread::sleep(std::time::Duration::from_secs(seconds));
    }
    let straight = document(&state);
    println!(
        "PERF clips straight: {}",
        straight
            .sequence
            .tracks
            .iter()
            .map(|t| t.clips.len())
            .sum::<usize>()
    );
    play(&root, &straight, "straight");
    seeks(&root, &straight, "straight");
    mix(&root, &straight, "straight");
    edits(&state, &handle, "straight");

    jump_cut(&state, &handle, 1_200_000, 250_000);
    let cut = document(&state);
    println!(
        "PERF clips cut: {}",
        cut.sequence
            .tracks
            .iter()
            .map(|t| t.clips.len())
            .sum::<usize>()
    );
    play(&root, &cut, "jump cuts");
    seeks(&root, &cut, "jump cuts");
    mix(&root, &cut, "jump cuts");
    edits(&state, &handle, "jump cuts");
}

#[cfg(windows)]
#[test]
#[ignore]
fn perf_audio_device() {
    let mut times = Vec::new();
    for _ in 0..5 {
        let started = Instant::now();
        let output = crate::playback::audio::AudioOutput::new().unwrap();
        times.push(started.elapsed().as_secs_f64() * 1000.0);
        let started = Instant::now();
        drop(output);
        println!(
            "PERF audio device drop {:.1}ms",
            started.elapsed().as_secs_f64() * 1000.0
        );
    }
    println!("PERF {}", Stats(times).line("audio device open"));
}
