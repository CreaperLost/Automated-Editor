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

fn export_timed(root: &Path, document: &crate::project::revision::EditDocument, label: &str) {
    crate::media::ffmpeg::release_decoders();
    let mut owner = crate::export::ExportOwner::new();
    let settings = crate::export::ExportSettings::default();
    let captured = crate::export::prepare_job(root, label, document.clone(), settings, &mut owner)
        .unwrap_or_else(|status| panic!("prepare failed: {:?}", status.failure));
    let started = Instant::now();
    let frames = std::cell::Cell::new(0);
    crate::export::run_export(
        &captured,
        &std::sync::atomic::AtomicBool::new(false),
        |done, _| frames.set(done),
        &crate::media::EncoderGate::new(),
    )
    .unwrap_or_else(|failure| panic!("export failed: {failure:?}"));
    let seconds = started.elapsed().as_secs_f64();
    println!(
        "PERF export {label}: {} frames of 1080p30 in {seconds:.2}s ({:.1} fps, {:.2}x real time)",
        frames.get(),
        frames.get() as f64 / seconds,
        document.duration_us() as f64 / 1e6 / seconds
    );
}

#[test]
#[ignore]
fn perf_export() {
    let Some(dir) = probe_dir() else { return };
    let work = tempfile::tempdir().unwrap();
    let (state, handle, root) = opened(work.path(), &dir);
    if let Some(settle) = std::env::var_os("AERO_PERF_SETTLE") {
        let seconds: u64 = settle.to_string_lossy().parse().unwrap_or(15);
        std::thread::sleep(std::time::Duration::from_secs(seconds));
    }
    println!(
        "PERF export encoder {:?}, compositor {:?}",
        crate::media::ffmpeg::encoder_name(),
        crate::render::Compositor::new().map(|c| c.adapter_name().to_string())
    );
    export_timed(&root, &document(&state), "straight");
    jump_cut(&state, &handle, 1_200_000, 250_000);
    export_timed(&root, &document(&state), "jump cuts");
}

/// How fast frames come through the decoder pipe, unpaced, at preview and export sizes.
#[test]
#[ignore]
fn perf_decode_throughput() {
    let Some(dir) = probe_dir() else { return };
    let path = dir.join("aero/media/screen/000001.mp4");
    for (label, limit) in [
        (
            "720p preview",
            crate::media::ffmpeg::DecodeLimit {
                max_width: 1280,
                max_height: 720,
                max_rate: 30,
                interactive: true,
                yuv: false,
            },
        ),
        (
            "1080p export, every source frame",
            crate::media::ffmpeg::DecodeLimit {
                max_width: 3840,
                max_height: 2160,
                max_rate: 0,
                interactive: false,
                yuv: false,
            },
        ),
        (
            "1080p export at 30 fps",
            crate::media::ffmpeg::DecodeLimit {
                max_width: 3840,
                max_height: 2160,
                max_rate: 30,
                interactive: false,
                yuv: false,
            },
        ),
    ] {
        crate::media::ffmpeg::release_decoders();
        crate::media::ffmpeg::decode_bgra_limited(&path, 0, limit).unwrap();
        let started = Instant::now();
        let n = 300u64;
        for i in 1..=n {
            crate::media::ffmpeg::decode_bgra_limited(&path, i * 1_000_000 / 30, limit).unwrap();
        }
        let ms = started.elapsed().as_secs_f64() * 1000.0;
        println!(
            "PERF decode {label}: {:.2}ms/frame ({:.0} fps)",
            ms / n as f64,
            n as f64 * 1000.0 / ms
        );
    }
}

/// Raw FFmpeg output through a pipe: how fast frames can come at all.
#[test]
#[ignore]
fn perf_pipe_throughput() {
    use std::io::Read;
    let Some(dir) = probe_dir() else { return };
    let path = dir.join("aero/media/screen/000001.mp4");
    let ffmpeg = crate::media::ffmpeg::ffmpeg_path().unwrap();
    for fmt in ["bgra", "nv12"] {
        let started = Instant::now();
        let mut child = std::process::Command::new(ffmpeg)
            .args(["-v", "error", "-nostdin", "-i"])
            .arg(&path)
            .args(["-t", "20", "-map", "0:v:0", "-an", "-vf"])
            .arg(format!("fps=30/1,format={fmt}"))
            .args(["-f", "rawvideo", "pipe:1"])
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut out = child.stdout.take().unwrap();
        let mut buf = vec![0u8; 8 << 20];
        let mut total = 0usize;
        loop {
            let n = out.read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            total += n;
        }
        child.wait().unwrap();
        let s = started.elapsed().as_secs_f64();
        println!(
            "PERF pipe {fmt}: {:.0} MB in {s:.2}s = {:.0} MB/s, {:.2}ms per 600 frames-equivalent frame",
            total as f64 / 1e6,
            total as f64 / 1e6 / s,
            s * 1000.0 / 600.0
        );
    }
}

/// The everyday operations on a long recording (`AERO_REC`, read only): opening, waveforms,
/// pauses, zooms, edits, undo, and the preview after them.
#[test]
#[ignore]
fn perf_long_recording() {
    let Some(path) = std::env::var_os("AERO_REC") else {
        return;
    };
    let work = tempfile::tempdir().unwrap();
    let time = |label: &str, started: Instant| {
        println!(
            "PERF long {label}: {:.1}ms",
            started.elapsed().as_secs_f64() * 1000.0
        )
    };
    let t = Instant::now();
    let folder =
        crate::project::folder::create_project_folder(work.path(), "Long", Some(Path::new(&path)))
            .unwrap();
    time("make project", t);
    let state = AppState::new();
    let t = Instant::now();
    let opened = commands::open_project_impl(&state, folder.to_string_lossy().into()).unwrap();
    time("open", t);
    let handle = opened.project_handle.clone();
    let asset = opened.assets[0].id.clone();
    let doc = document(&state);
    println!(
        "PERF long duration {:.0}s, clips {}",
        doc.duration_us() as f64 / 1e6,
        doc.sequence.clips().count()
    );
    for key in ["mic", "system"] {
        let key = format!("{asset}.{key}");
        for pass in ["cold", "warm"] {
            let t = Instant::now();
            let overview =
                commands::project_waveform_overview_impl(&state, handle.clone(), key.clone())
                    .unwrap();
            time(&format!("waveform {key} {pass}"), t);
            if pass == "warm" {
                let json = serde_json::to_string(&overview).unwrap();
                println!(
                    "PERF waveform {key}: {} bars, {} bytes of JSON",
                    overview.peaks.len(),
                    json.len()
                );
                // To look at real waveforms in a browser mock: AERO_WAVE_DUMP=<folder>.
                if let Some(dump) = std::env::var_os("AERO_WAVE_DUMP") {
                    let name = if key.ends_with(".mic") {
                        "mic.json"
                    } else {
                        "system.json"
                    };
                    std::fs::write(PathBuf::from(dump).join(name), json).unwrap();
                }
            }
        }
    }
    let t = Instant::now();
    let pauses = commands::detect_silence_impl(
        &state,
        handle.clone(),
        format!("{asset}.mic"),
        crate::dsp::silence::SilenceConfig {
            threshold_db: -38.0,
            min_duration_ms: 400,
            padding_ms: 50,
            ..Default::default()
        },
    )
    .unwrap();
    time(
        &format!("detect pauses ({} found)", pauses.suggestions.len()),
        t,
    );
    let t = Instant::now();
    let zooms = commands::project_zoom_suggestions_impl(&state, handle.clone(), None).unwrap();
    time(
        &format!("zoom suggestions ({} found)", zooms.suggestions.len()),
        t,
    );
    let cuts: Vec<_> = pauses
        .suggestions
        .iter()
        .map(|s| commands::EditCut {
            start_us: s.start_us,
            end_us: s.end_us,
        })
        .collect();
    let t = Instant::now();
    commands::project_ripple_cuts_impl(&state, handle.clone(), revision(&state), cuts).unwrap();
    time("cut every pause", t);
    let cut = document(&state);
    println!(
        "PERF long clips after cuts {}",
        cut.sequence.clips().count()
    );
    edits(&state, &handle, "long");
    let t = Instant::now();
    commands::project_undo_impl(&state, handle.clone(), revision(&state), None).unwrap();
    time("undo", t);
    let cut = document(&state);
    let t = Instant::now();
    let (mut evaluator, mixer) = rebuild(&folder, &cut, None);
    time("rebuild (scene + mixer)", t);
    let t = Instant::now();
    evaluator.preview_at(cut.duration_us() / 2).unwrap();
    time("first frame mid-video", t);
    let t = Instant::now();
    mixer
        .read_frames(cut.duration_us() / 2 * 48 / 1000, CHUNK_FRAMES)
        .unwrap();
    time("first audio chunk mid-video", t);
    seeks(&folder, &cut, "long");
}

/// Building the audio mixer of the long recording (`AERO_REC`) with every effect on: the
/// playback worker builds one after each edit.
#[test]
#[ignore]
fn perf_long_mixer_with_polish() {
    let Some(path) = std::env::var_os("AERO_REC") else {
        return;
    };
    let work = tempfile::tempdir().unwrap();
    let folder =
        crate::project::folder::create_project_folder(work.path(), "Long", Some(Path::new(&path)))
            .unwrap();
    let state = AppState::new();
    commands::open_project_impl(&state, folder.to_string_lossy().into()).unwrap();
    let mut doc = document(&state);
    doc.audio.normalize = true;
    doc.audio.noise_reduction = true;
    doc.audio.duck_system_audio = true;
    for pass in ["cold", "warm", "warm"] {
        let t = Instant::now();
        let mixer = AudioMixer::new(&folder, &doc).unwrap();
        println!(
            "PERF long mixer with polish {pass}: {:.1}ms",
            t.elapsed().as_secs_f64() * 1000.0
        );
        let t = Instant::now();
        mixer.read_frames(48_000 * 300, CHUNK_FRAMES).unwrap();
        println!(
            "PERF long first denoised chunk: {:.1}ms",
            t.elapsed().as_secs_f64() * 1000.0
        );
    }
}

/// Cutting every pause of a real recording and restoring every cut gives back the timeline
/// it started from (`AERO_REC`, or the probe recording under `AERO_PROBE`).
#[test]
#[ignore]
fn probe_restore_all_cuts() {
    let recording = match (std::env::var_os("AERO_REC"), probe_dir()) {
        (Some(path), _) => PathBuf::from(path),
        (None, Some(dir)) => dir.join("aero"),
        _ => return,
    };
    let work = tempfile::tempdir().unwrap();
    let folder =
        crate::project::folder::create_project_folder(work.path(), "Restore", Some(&recording))
            .unwrap();
    let state = AppState::new();
    let opened = commands::open_project_impl(&state, folder.to_string_lossy().into()).unwrap();
    let handle = opened.project_handle.clone();
    let asset = opened.assets[0].id.clone();
    // Each track as the source time it plays where, contiguous pieces joined.
    let spans = |doc: &crate::project::revision::EditDocument| -> Vec<Vec<(u64, u64, u64)>> {
        doc.sequence
            .tracks
            .iter()
            .map(|t| {
                let mut out: Vec<(u64, u64, u64)> = Vec::new();
                for c in &t.clips {
                    match out.last_mut() {
                        Some(last)
                            if last.0 + last.2 == c.start_us && last.1 + last.2 == c.in_us =>
                        {
                            last.2 += c.duration_us
                        }
                        _ => out.push((c.start_us, c.in_us, c.duration_us)),
                    }
                }
                out
            })
            .collect()
    };
    let before = spans(&document(&state));
    let pauses = commands::detect_silence_impl(
        &state,
        handle.clone(),
        format!("{asset}.mic"),
        crate::dsp::silence::SilenceConfig {
            threshold_db: -38.0,
            min_duration_ms: 400,
            padding_ms: 50,
            ..Default::default()
        },
    )
    .unwrap();
    let cuts: Vec<_> = pauses
        .suggestions
        .iter()
        .map(|s| commands::EditCut {
            start_us: s.start_us,
            end_us: s.end_us,
        })
        .collect();
    println!("PROBE {} pauses cut", cuts.len());
    commands::project_ripple_cuts_impl(&state, handle.clone(), revision(&state), cuts).unwrap();
    if std::env::var_os("AERO_RESTORE_ONE_BY_ONE").is_some() {
        // As clicking each cut's mark: the left clip of each join on the first video track,
        // in a scrambled order, until none is left.
        let mut round = 0u64;
        loop {
            let doc = document(&state);
            let track = &doc.sequence.tracks[0];
            let joins: Vec<String> = track
                .clips
                .windows(2)
                .filter(|w| {
                    w[0].asset == w[1].asset
                        && w[0].stream == w[1].stream
                        && w[0].start_us + w[0].duration_us == w[1].start_us
                        && w[0].in_us + w[0].duration_us < w[1].in_us
                })
                .map(|w| w[0].id.clone())
                .collect();
            if joins.is_empty() {
                break;
            }
            round += 1;
            let pick = joins[(round as usize * 7919) % joins.len()].clone();
            commands::project_sequence_edit_impl(
                &state,
                handle.clone(),
                revision(&state),
                crate::sequence::edit::SequenceEdit::RestoreCuts {
                    clip_ids: vec![pick],
                },
                None,
            )
            .unwrap();
        }
        println!("PROBE restored {round} cuts one by one");
    } else {
        commands::project_sequence_edit_impl(
            &state,
            handle.clone(),
            revision(&state),
            crate::sequence::edit::SequenceEdit::RestoreCuts { clip_ids: vec![] },
            None,
        )
        .unwrap();
    }
    let after = spans(&document(&state));
    println!(
        "PROBE clips per track after restoring: {:?}",
        document(&state)
            .sequence
            .tracks
            .iter()
            .map(|t| t.clips.len())
            .collect::<Vec<_>>()
    );
    for (t, (a, b)) in before.iter().zip(&after).enumerate() {
        if a != b {
            println!("PROBE track {t} differs:\n  before {a:?}\n  after  {b:?}");
        }
    }
    println!(
        "PROBE duration before {} after {}",
        document(&state).duration_us(),
        before
            .iter()
            .flatten()
            .map(|s| s.0 + s.2)
            .max()
            .unwrap_or(0)
    );
    assert_eq!(before, after);
}

/// Export frames of the probe recording as the GPU now converts them (NV12) against what the
/// export made before (BGRA through FFmpeg's default conversion): worst and mean difference in
/// levels per plane.
#[test]
#[ignore]
fn probe_export_nv12_against_bgra() {
    use std::io::{Read, Write};
    let Some(dir) = probe_dir() else { return };
    let work = tempfile::tempdir().unwrap();
    let (state, _, root) = opened(work.path(), &dir);
    let document = document(&state);
    let (w, h) = (1920usize, 1080usize);
    let evaluator = |nv12: bool| {
        SceneEvaluator::new(root.clone(), document.clone(), w as u32, h as u32)
            .unwrap()
            .with_nv12_output(nv12)
    };
    let (mut bgra, mut nv12) = (evaluator(false), evaluator(true));
    let mut worst = [0u8; 3];
    let mut total = [0u64; 3];
    let mut count = [0u64; 3];
    let duration = document.duration_us();
    for i in 0..12 {
        let at = duration * i / 12;
        let rgb = bgra.preview_at(at).unwrap();
        let yuv = nv12.preview_at(at).unwrap();
        let mut child = std::process::Command::new(crate::media::ffmpeg::ffmpeg_path().unwrap())
            .args([
                "-v",
                "error",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "bgra",
                "-video_size",
            ])
            .arg(format!("{w}x{h}"))
            .args(["-i", "pipe:0", "-vf"])
            .arg("scale=out_color_matrix=bt709:out_range=tv,format=yuv420p")
            .args(["-f", "rawvideo", "pipe:1"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let pixels = rgb.data.clone();
        let feed = std::thread::spawn(move || stdin.write_all(&pixels).unwrap());
        let mut old = Vec::new();
        child.stdout.take().unwrap().read_to_end(&mut old).unwrap();
        feed.join().unwrap();
        child.wait().unwrap();
        let quarter = w * h / 4;
        let chroma = &yuv.data[w * h..];
        let planes: [Vec<(u8, u8)>; 3] = [
            yuv.data[..w * h]
                .iter()
                .copied()
                .zip(old[..w * h].iter().copied())
                .collect(),
            chroma
                .iter()
                .step_by(2)
                .copied()
                .zip(old[w * h..][..quarter].iter().copied())
                .collect(),
            chroma[1..]
                .iter()
                .step_by(2)
                .copied()
                .zip(old[w * h + quarter..].iter().copied())
                .collect(),
        ];
        for (p, pairs) in planes.iter().enumerate() {
            for &(a, b) in pairs {
                worst[p] = worst[p].max(a.abs_diff(b));
                total[p] += a.abs_diff(b) as u64;
                count[p] += 1;
            }
        }
    }
    for (p, name) in ["Y", "Cb", "Cr"].iter().enumerate() {
        println!(
            "PROBE {name}: worst {} levels, mean {:.3}",
            worst[p],
            total[p] as f64 / count[p] as f64
        );
    }
}

/// Jump cuts on an imported video's sound.
#[test]
#[ignore]
fn probe_imported_video_jump_cuts() {
    let dir = tempfile::tempdir().unwrap();
    let video = dir.path().join("talk.mp4");
    let status = std::process::Command::new(crate::media::ffmpeg::ffmpeg_path().unwrap())
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=320x180:rate=30",
        ])
        .args([
            "-f",
            "lavfi",
            "-i",
            "aevalsrc='if(lt(mod(t,2),1),0.5*sin(440*2*PI*t),0)':s=48000",
        ])
        .args([
            "-t",
            "6",
            "-c:v",
            "libx264",
            "-c:a",
            "aac",
            "-shortest",
            "-y",
        ])
        .arg(&video)
        .status()
        .unwrap();
    assert!(status.success());
    // With the probe recording first on the timeline, when there is one.
    let recording = probe_dir().map(|d| d.join("aero"));
    let folder =
        crate::project::folder::create_project_folder(dir.path(), "Imported", recording.as_deref())
            .unwrap();
    let state = AppState::new();
    let opened = commands::open_project_impl(&state, folder.to_string_lossy().into()).unwrap();
    let handle = opened.project_handle.clone();
    let end_us = opened.sequence.duration_us();
    let opened = commands::project_media_import_impl(
        &state,
        handle.clone(),
        opened.revision,
        vec![video.to_string_lossy().into()],
    )
    .unwrap();
    let asset = opened.assets.last().unwrap().clone();
    println!(
        "PROBE streams {:?}",
        asset
            .streams
            .iter()
            .map(|s| (&s.id, s.role, &s.audio_path))
            .collect::<Vec<_>>()
    );
    let opened = commands::project_sequence_edit_impl(
        &state,
        handle.clone(),
        opened.revision,
        crate::sequence::edit::SequenceEdit::PlaceAsset {
            asset_id: asset.id.clone(),
            at_us: end_us,
            track_id: None,
            streams: vec![],
            range: None,
        },
        None,
    )
    .unwrap();
    println!(
        "PROBE video placed at {end_us}us of {}us",
        opened.sequence.duration_us()
    );
    let sound = asset
        .streams
        .iter()
        .find(|s| s.kind == crate::sequence::StreamKind::Sound)
        .unwrap();
    let key = format!("{}.{}", asset.id, sound.id);
    let _ = opened;
    for key in [key, commands::ALL_SPEECH.to_string()] {
        let result = commands::detect_silence_impl(
            &state,
            handle.clone(),
            key.clone(),
            crate::dsp::silence::SilenceConfig::default(),
        )
        .unwrap();
        let in_video: Vec<_> = result
            .suggestions
            .iter()
            .filter(|s| s.start_us >= end_us)
            .map(|s| (s.start_us - end_us, s.end_us - end_us))
            .collect();
        println!(
            "PROBE {key}: {} pauses, in the video {in_video:?}",
            result.suggestions.len()
        );
    }
}
