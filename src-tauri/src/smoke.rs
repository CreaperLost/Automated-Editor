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
    let voice = std::fs::read(fixtures().join("voice.wav"))
        .unwrap_or_else(|e| panic!("read voice.wav: {e} (run scripts/make-fixtures.sh)"));
    recording_with_voice(dir, &voice)
}

/// A 6 s `.aero` recording of screen.mp4 and `voice` (a 48 kHz mono WAV) as its mic.
fn recording_with_voice(dir: &Path, voice: &[u8]) -> PathBuf {
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
        if fixture == "voice.wav" {
            std::fs::write(root.join(path), voice).unwrap();
        } else {
            std::fs::copy(fixtures().join(fixture), root.join(path)).unwrap_or_else(|e| {
                panic!("copy fixture {fixture}: {e} (run scripts/make-fixtures.sh)")
            });
        }
        let size = std::fs::metadata(root.join(path)).unwrap().len();
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

/// The project made from `recording`, opened as the app opens it, with its mic's sound key.
fn open(
    dir: &Path,
    recording: &Path,
) -> (AppState, crate::project::OpenedProject, PathBuf, String) {
    let folder =
        crate::project::folder::create_project_folder(dir, "Smoke", Some(recording)).unwrap();
    let state = AppState::new();
    let opened = open_project_impl(&state, folder.to_string_lossy().into()).unwrap();
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
    (state, opened, folder, mic)
}

/// Exports `folder` with `aeroedits render <folder> --out <out> --size <size>`.
fn render(folder: &Path, out: &Path, size: &str) -> PathBuf {
    let request = crate::render_cli::parse(&[
        folder.to_string_lossy().into(),
        "--out".into(),
        out.to_string_lossy().into(),
        "--size".into(),
        size.into(),
    ])
    .unwrap();
    let rendered = crate::render_cli::render(&request, |_, _| {})
        .unwrap_or_else(|message| panic!("render failed: {message}"));
    crate::media::release_decoders();
    rendered.video
}

/// Where a test's export goes: `AEROEDITS_SMOKE_DIR/<name>.mp4` when set (to look at), else `dir`.
fn output(dir: &Path, name: &str) -> PathBuf {
    let folder = match std::env::var("AEROEDITS_SMOKE_DIR") {
        Ok(path) if !path.is_empty() => PathBuf::from(path),
        _ => dir.to_path_buf(),
    };
    std::fs::create_dir_all(&folder).unwrap();
    let file = folder.join(format!("{name}.mp4"));
    let _ = std::fs::remove_file(&file);
    file
}

/// ffmpeg's output (stdout, stderr) for these arguments; panics when it fails.
fn ffmpeg(args: &[&str]) -> (Vec<u8>, String) {
    let output = std::process::Command::new(crate::media::ffmpeg::ffmpeg_path().unwrap())
        .args(["-hide_banner", "-nostats"])
        .args(args)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "ffmpeg {args:?}: {stderr}");
    (output.stdout, stderr)
}

/// The loudest sample of `file` from `from` to `to` seconds, mono, 0..1.
fn peak(file: &Path, from: f64, to: f64) -> f32 {
    let (raw, _) = ffmpeg(&[
        "-v",
        "error",
        "-i",
        &file.to_string_lossy(),
        "-ss",
        &format!("{from:.3}"),
        "-t",
        &format!("{:.3}", to - from),
        "-vn",
        "-ac",
        "1",
        "-ar",
        "48000",
        "-f",
        "f32le",
        "-",
    ]);
    raw.chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]).abs())
        .fold(0.0, f32::max)
}

/// Integrated loudness of `file`'s sound (EBU R128), in LUFS.
fn loudness(file: &Path) -> f64 {
    let (_, log) = ffmpeg(&[
        "-i",
        &file.to_string_lossy(),
        "-vn",
        "-af",
        "ebur128",
        "-f",
        "null",
        "-",
    ]);
    let summary = log.rsplit("Summary:").next().unwrap();
    summary
        .lines()
        .find_map(|line| line.trim().strip_prefix("I:"))
        .and_then(|rest| rest.trim().trim_end_matches("LUFS").trim().parse().ok())
        .unwrap_or_else(|| panic!("no integrated loudness in {summary}"))
}

/// How many pixels of the frame at `at` seconds are near white (where caption text is drawn;
/// the test pattern has none).
fn white_pixels(file: &Path, at: f64) -> usize {
    let (raw, _) = ffmpeg(&[
        "-v",
        "error",
        "-ss",
        &format!("{at:.3}"),
        "-i",
        &file.to_string_lossy(),
        "-frames:v",
        "1",
        "-f",
        "rawvideo",
        "-pix_fmt",
        "rgb24",
        "-",
    ]);
    raw.chunks_exact(3)
        .filter(|p| p.iter().all(|&c| c > 220))
        .count()
}

/// 6 s of 48 kHz mono: a quiet tone (about -29 LUFS), room sound with one sharp click at
/// 3.0 s, then the tone again.
fn quiet_voice_with_click() -> Vec<u8> {
    let rate = 48_000usize;
    let mut seed = 7u64;
    let samples: Vec<i16> = (0..rate * 6)
        .map(|i| {
            let t = i as f64 / rate as f64;
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let room = ((seed >> 40) as f64 / (1u64 << 24) as f64 - 0.5) * 0.004;
            let tone = if !(2.0..4.0).contains(&t) {
                0.05 * (std::f64::consts::TAU * 440.0 * t).sin()
            } else {
                0.0
            };
            let since = t - 3.0;
            let click = if (0.0..0.004).contains(&since) {
                let n = i - 3 * rate;
                0.3 * (-(n as f64) / 40.0).exp() * if n % 2 == 0 { 1.0 } else { -1.0 }
            } else {
                0.0
            };
            ((room + tone + click) * i16::MAX as f64) as i16
        })
        .collect();
    crate::fixtures::generate_pcm16_wav(48_000, 1, &samples)
}

/// Voice polish end to end: the mouth-noise pass finds the click, muting it silences it in the
/// export, and loudness normalizing brings the quiet voice to -14 LUFS.
#[test]
fn smoke_mouth_noise_is_muted_and_loudness_is_normalized() {
    if let Some(reason) = missing_tools() {
        eprintln!("SKIPPED smoke test: {reason}");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let recording = recording_with_voice(dir.path(), &quiet_voice_with_click());
    let source = recording.join("media/mic/000001.wav");
    assert!(
        peak(&source, 2.98, 3.03) > 0.25,
        "the click is in the source"
    );
    let (state, opened, folder, mic) = open(dir.path(), &recording);
    let handle = opened.project_handle.clone();

    let scan = crate::commands::noises::detect_mouth_noises_impl(
        &state,
        handle.clone(),
        mic.clone(),
        crate::dsp::noises::NoiseConfig::default(),
        None,
    )
    .unwrap();
    let click = scan
        .noises
        .iter()
        .find(|n| n.source_start_us <= 3_000_000 && 3_003_000 <= n.source_end_us)
        .unwrap_or_else(|| panic!("no noise over the click at 3.0 s: {:?}", scan.noises));
    let muted = crate::commands::noises::project_noise_mutes_impl(
        &state,
        handle.clone(),
        opened.revision,
        mic,
        click.parts.clone(),
        false,
        None,
    )
    .unwrap();
    let mut audio = state
        .opened_project
        .lock()
        .as_ref()
        .unwrap()
        .document()
        .audio
        .clone();
    audio.normalize = true;
    audio.target_lufs = -14.0;
    crate::commands::project_audio_update_impl(&state, handle, muted.revision, audio).unwrap();
    drop(state);

    let written = render(&folder, &output(dir.path(), "voice-polish"), "320x180");
    let lufs = loudness(&written);
    assert!(
        (lufs + 14.0).abs() <= 1.0,
        "integrated loudness {lufs:.1} LUFS, wanted -14 ±1"
    );
    let at_click = peak(&written, 2.98, 3.03);
    let room = peak(&written, 2.3, 2.8);
    assert!(
        at_click <= room * 1.5 + 0.005,
        "the click is muted: peak {at_click:.3} there, room peaks at {room:.3}"
    );
    eprintln!(
        "smoke: click at 3.0 s found ({} noise(s)) and muted (peak {at_click:.3}, room {room:.3}); {lufs:.1} LUFS",
        scan.noises.len()
    );
}

/// A 9:16 project exports portrait (a landscape size asked for is turned) with its captions
/// drawn from the transcript.
#[test]
fn smoke_portrait_export_draws_captions() {
    use crate::transcript::{store, test_word, ProviderKind, Transcript};
    if let Some(reason) = missing_tools() {
        eprintln!("SKIPPED smoke test: {reason}");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let recording = recording(dir.path());
    let (state, opened, folder, mic) = open(dir.path(), &recording);
    let handle = opened.project_handle.clone();
    store::save_transcript(
        &folder,
        &Transcript::new(
            mic,
            ProviderKind::ElevenLabs,
            "scribe_v2".into(),
            None,
            vec![test_word("HELLO", 0, 900), test_word("WORLD", 1000, 1900)],
        ),
    )
    .unwrap();
    let (mut layout, mut captions) = {
        let opened = state.opened_project.lock();
        let document = opened.as_ref().unwrap().document();
        (document.layout.clone(), document.captions.clone())
    };
    layout.aspect_ratio = "9:16".into();
    let laid = crate::commands::project_layout_update_impl(
        &state,
        handle.clone(),
        opened.revision,
        layout,
        None,
    )
    .unwrap();
    captions.enabled = true;
    crate::commands::project_captions_update_impl(&state, handle, laid.revision, captions).unwrap();
    drop(state);

    let written = render(
        &folder,
        &output(dir.path(), "portrait-captions"),
        "1280x720",
    );
    let video = probe(&written, Some("v:0"), "stream=width,height");
    assert_eq!(
        (value(&video, "width"), value(&video, "height")),
        ("720", "1280"),
        "a 9:16 project exports portrait"
    );
    // "HELLO" shows from 0 to 0.9 s, nothing from 2 s on.
    let (with_word, without) = (white_pixels(&written, 0.5), white_pixels(&written, 3.0));
    assert!(
        with_word > 1_000 && without * 10 < with_word,
        "caption text: {with_word} white pixels with a word, {without} without"
    );
    eprintln!(
        "smoke: 9:16 project -> 720x1280 export; caption \"HELLO\" is {with_word} white pixels \
         at 0.5 s, {without} at 3.0 s -> {}",
        written.display()
    );
}
