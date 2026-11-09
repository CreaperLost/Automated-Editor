//! End-to-end smoke test of the core pipeline on the committed fixtures in `test/fixtures/`:
//! a recording (screen + mic) opens as a project, the silence pass finds the pause, the cut
//! is applied as the app applies it, and `aeroedits render` exports the saved edit to an MP4
//! (with stills) that ffprobe checks.
//!
//! Runs headless on Linux: it needs FFmpeg/ffprobe and a wgpu adapter (a software Vulkan
//! adapter such as Mesa's lavapipe is enough). Without them it skips with a message, unless
//! `AEROEDITS_REQUIRE_FFMPEG=1` / `AEROEDITS_REQUIRE_GPU=1` make a missing tool a failure.
//! `AEROEDITS_SMOKE_OUT=<file.mp4>` keeps the export (for `scripts/snapshot-frames.sh`).
//!
//! Run alone: `cargo test --lib smoke -- --nocapture`.

use crate::commands::{
    detect_silence_impl, open_project_impl, project_sequence_edit_impl, AppState,
};
use crate::dsp::SilenceConfig;
use crate::fixtures::TestProject;
use crate::project::{JournalRecord, TrackDescriptor, TrackType};
use crate::sequence::edit::SequenceEdit;
use std::path::{Path, PathBuf};

const FIXTURE_US: u64 = 6_000_000;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("test")
        .join("fixtures")
}

fn required(var: &str) -> bool {
    std::env::var(var).is_ok_and(|v| !v.is_empty() && v != "0")
}

/// Why the test can't run here, or None when it can.
fn missing_tools() -> Option<String> {
    if crate::media::ffmpeg::ffmpeg_path().is_err() || crate::media::ffmpeg::ffprobe_path().is_err()
    {
        assert!(
            !required("AEROEDITS_REQUIRE_FFMPEG"),
            "AEROEDITS_REQUIRE_FFMPEG is set but FFmpeg/ffprobe were not found"
        );
        return Some("FFmpeg/ffprobe not found".into());
    }
    if let Err(error) = crate::render::Compositor::new() {
        assert!(
            !required("AEROEDITS_REQUIRE_GPU"),
            "AEROEDITS_REQUIRE_GPU is set but no wgpu adapter: {error}"
        );
        return Some(format!(
            "no wgpu adapter ({error}); install mesa-vulkan-drivers for a software one"
        ));
    }
    None
}

/// A 6 s `.aero` recording made of the fixtures: screen.mp4 (30 fps) and voice.wav (mic).
fn recording(dir: &Path) -> PathBuf {
    let mut bundle = TestProject::create(dir, "smoke");
    let root = bundle.root_path().to_path_buf();
    let tracks = [
        (
            "screen",
            TrackType::Screen,
            "h264",
            "screen.mp4",
            "media/screen/000001.mp4",
            30,
        ),
        (
            "mic",
            TrackType::MicAudio,
            "pcm",
            "voice.wav",
            "media/mic/000001.wav",
            48_000,
        ),
    ];
    for (id, track_type, codec, fixture, path, timescale) in tracks {
        let size = std::fs::copy(fixtures().join(fixture), root.join(path)).unwrap_or_else(|e| {
            panic!("copy fixture {fixture}: {e} (run scripts/make-fixtures.sh)")
        });
        let audio = track_type == TrackType::MicAudio;
        bundle.manifest_mut().tracks.push(TrackDescriptor {
            id: id.into(),
            track_type,
            codec: codec.into(),
            relative_path: path.into(),
            width: (!audio).then_some(320),
            height: (!audio).then_some(180),
            fps: (!audio).then_some(30),
            sample_rate: audio.then_some(48_000),
            channels: audio.then_some(1),
            gaps_total: 0,
            media_timescale: Some(timescale),
        });
        bundle.append_journal(JournalRecord::SegmentCommitted {
            seq: 0,
            track_id: id.into(),
            relative_path: path.into(),
            start_us: 0,
            end_us: FIXTURE_US,
            size_bytes: size,
            is_keyframe_start: true,
            media_timescale: timescale,
            media_start_value: 0,
            host_anchor_us: 0,
        });
    }
    bundle.manifest_mut().duration_us = FIXTURE_US;
    bundle.manifest_mut().active_duration_us = FIXTURE_US;
    bundle.save_manifest();
    root
}

/// `key=value` lines from ffprobe for one stream (`v:0`, `a:0`) or the format.
fn probe(file: &Path, select: Option<&str>, entries: &str) -> Vec<(String, String)> {
    let mut command = std::process::Command::new(crate::media::ffmpeg::ffprobe_path().unwrap());
    command.args(["-v", "error", "-count_packets"]);
    if let Some(stream) = select {
        command.args(["-select_streams", stream]);
    }
    let output = command
        .args(["-show_entries", entries, "-of", "default=nw=1"])
        .arg(file)
        .output()
        .unwrap();
    assert!(output.status.success(), "ffprobe failed on {file:?}");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

fn value<'a>(entries: &'a [(String, String)], key: &str) -> &'a str {
    entries
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
        .unwrap_or_else(|| panic!("ffprobe gave no {key}: {entries:?}"))
}

#[test]
fn smoke_recording_silence_cut_exports_a_shorter_mp4() {
    if let Some(reason) = missing_tools() {
        eprintln!("SKIPPED smoke test: {reason}");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let recording = recording(dir.path());
    let folder =
        crate::project::folder::create_project_folder(dir.path(), "Smoke", Some(&recording))
            .unwrap();

    // Open, as the app does.
    let state = AppState::new();
    let opened = open_project_impl(&state, folder.to_string_lossy().into()).unwrap();
    assert_eq!(
        opened.duration_us, FIXTURE_US,
        "the recording opens at its length"
    );
    let mic = opened
        .assets
        .iter()
        .flat_map(|asset| {
            asset
                .streams
                .iter()
                .map(move |stream| crate::sequence::StreamRef::new(&asset.id, &stream.id).key())
        })
        .find(|key| key.ends_with(".mic"))
        .unwrap_or_else(|| panic!("no mic stream in {:?}", opened.assets));

    // The silence pass: the 2 s of digital silence between the tones, less its margins.
    let found = detect_silence_impl(
        &state,
        opened.project_handle.clone(),
        mic,
        SilenceConfig::default(),
    )
    .unwrap();
    assert_eq!(
        found.suggestions.len(),
        1,
        "one pause: {:?}",
        found.suggestions
    );
    let pause = &found.suggestions[0];
    let removed = pause.end_us - pause.start_us;
    assert!(
        (1_600_000..=2_000_000).contains(&removed),
        "the pause is most of 2.0-4.0 s: {pause:?}"
    );
    assert!(
        pause.start_us >= 1_900_000 && pause.end_us <= 4_100_000,
        "{pause:?}"
    );

    // Cut it out with ripple, as accepting the suggestion does.
    let cut = project_sequence_edit_impl(
        &state,
        opened.project_handle.clone(),
        opened.revision,
        SequenceEdit::DeleteRange {
            ranges: vec![crate::zoom::EditedRange {
                start_us: pause.start_us,
                end_us: pause.end_us,
            }],
            ripple: Some(true),
        },
        None,
    )
    .unwrap();
    let edited_us = FIXTURE_US - removed;
    assert_eq!(cut.duration_us, edited_us);

    // Export the saved edit.
    let output = match std::env::var("AEROEDITS_SMOKE_OUT") {
        Ok(path) if !path.is_empty() => PathBuf::from(path),
        _ => dir.path().join("smoke.mp4"),
    };
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    // Export the saved edit as `aeroedits render <project> --out x.mp4 --frames 3` does.
    let request = crate::render_cli::parse(&[
        folder.to_string_lossy().into(),
        "--out".into(),
        output.to_string_lossy().into(),
        "--size".into(),
        "320x180".into(),
        "--frames".into(),
        "3".into(),
    ])
    .unwrap();
    let rendered = crate::render_cli::render(&request, |_, _| {})
        .unwrap_or_else(|message| panic!("render failed: {message}"));
    let written = rendered.video.clone();
    crate::media::release_decoders();
    assert_eq!(rendered.stills.len(), 3, "{rendered:?}");
    for still in rendered.stills.iter().chain(&rendered.contact_sheet) {
        assert!(still.is_file(), "no still {still:?}");
    }
    assert!(written.is_file(), "export wrote {written:?}");

    // Check the file the way a person would with ffprobe.
    let format = probe(&written, None, "format=duration");
    let duration: f64 = value(&format, "duration").parse().unwrap();
    let expected = edited_us as f64 / 1e6;
    assert!(
        (duration - expected).abs() < 0.15,
        "duration {duration:.3} s, expected {expected:.3} s"
    );
    let video = probe(
        &written,
        Some("v:0"),
        "stream=codec_name,width,height,nb_read_packets",
    );
    assert_eq!(value(&video, "codec_name"), "h264");
    assert_eq!(value(&video, "width"), "320");
    assert_eq!(value(&video, "height"), "180");
    let frames: f64 = value(&video, "nb_read_packets").parse().unwrap();
    assert!(
        (frames - expected * 30.0).abs() <= 3.0,
        "{frames} frames for {expected:.3} s at 30 fps"
    );
    let audio = probe(&written, Some("a:0"), "stream=codec_name");
    assert!(
        !value(&audio, "codec_name").is_empty(),
        "the export has a sound stream"
    );
    eprintln!(
        "smoke: {:.3} s recording -> cut {:.3} s of silence -> {} ({duration:.3} s, {frames} frames)",
        FIXTURE_US as f64 / 1e6,
        removed as f64 / 1e6,
        written.display()
    );
}
