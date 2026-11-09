//! Writes the canned answers of the front end's preview build (`front-end/src/mock/`): the
//! smoke test's fixture project, opened and asked what the editor asks, by the real commands.
//! Rerun after a change to what those commands return:
//!
//! `cd src-tauri && cargo test --lib preview_mock -- --ignored`
use crate::commands::*;
use serde::Serialize;
use serde_json::{Map, Value};
use std::path::Path;

/// Where the preview build says the project is, in place of this run's temporary folder.
const SHOWN_HOME: &str = "/home/george/AeroEdits";

fn json(value: impl Serialize) -> Value {
    serde_json::to_value(value).unwrap()
}

#[test]
#[ignore = "writes front-end/src/mock/backend.json"]
fn preview_mock_writes_the_front_end_answers() {
    let dir = tempfile::tempdir().unwrap();
    let recording = crate::smoke::recording(dir.path());
    let (state, opened, folder, mic) = crate::smoke::open(dir.path(), &recording);
    let handle = opened.project_handle.clone();
    let mut out = Map::new();
    let mut put = |cmd: &str, value: Value| {
        out.insert(cmd.into(), value);
    };
    // A few words, so the transcript and captions have something to show.
    let words = [
        ("Today", 100, 450),
        ("we", 500, 650),
        ("cut", 700, 1000),
        ("the", 1050, 1150),
        ("pauses", 1200, 1750),
        ("out.", 1800, 1950),
        ("Like", 4100, 4400),
        ("this.", 4450, 4900),
    ];
    crate::transcript::store::save_transcript(
        &folder,
        &crate::transcript::Transcript::new(
            mic.clone(),
            crate::transcript::ProviderKind::Parakeet,
            "parakeet-tdt-0.6b-v3".into(),
            Some("en".into()),
            words
                .iter()
                .map(|(text, from, to)| crate::transcript::test_word(text, *from, *to))
                .collect(),
        ),
    )
    .unwrap();

    put("open_project", json(&opened));
    put(
        "presets",
        json(preset::presets_impl(&state, Some(handle.clone())).unwrap()),
    );
    put(
        "proposals_list",
        json(proposals::proposals_list_impl(&state, handle.clone()).unwrap()),
    );
    put(
        "faceless_status",
        json(faceless::faceless_status_impl(&state)),
    );
    put(
        "preview_attach",
        json(
            preview_attach_impl(
                &state,
                "main".into(),
                crate::playback::PreviewHitMode::Consume,
                None,
            )
            .unwrap(),
        ),
    );
    put("preview_quality", json(preview_quality_impl(&state)));
    put(
        "transcript_suggestions",
        json(
            transcript::transcript_suggestions_impl(&state, handle.clone(), mic.clone(), None)
                .unwrap(),
        ),
    );
    put(
        "project_effects_at",
        json(effects::project_effects_at_impl(&state, handle.clone(), 0).unwrap()),
    );
    // The timeline's film strip: a thumbnail every half second of the screen.
    let screen = opened
        .assets
        .iter()
        .find(|a| a.streams.iter().any(|s| s.id == "screen"))
        .unwrap();
    let times: Vec<u64> = (0..12).map(|i| i * 500_000).collect();
    put(
        "project_clip_thumbnails",
        serde_json::json!({
            "timesUs": times,
            "thumbnails": json(
                thumbs::project_clip_thumbnails_impl(
                    &state,
                    handle.clone(),
                    screen.id.clone(),
                    "screen".into(),
                    times.clone(),
                    96,
                )
                .unwrap()
            ),
        }),
    );
    // The libraries are big: a dozen of each is enough to see the panels.
    let mut sounds = sounds::sound_library_list_impl(None).unwrap();
    sounds.truncate(12);
    put("sound_library_list", json(sounds));
    let mut music = music::music_library_list_impl(None).unwrap();
    music.truncate(12);
    put("music_library_list", json(music));
    put(
        "joins_list",
        json(crate::joins::view_for(state.opened_project.lock().as_ref().unwrap()).unwrap()),
    );
    put(
        "noise_review",
        json(noises::noise_review_impl(&state, handle.clone(), mic.clone(), None).unwrap()),
    );
    // The Export dialog.
    put(
        "publish_package",
        json(publish::publish_package_impl(&state, &handle, "video".into()).unwrap()),
    );
    put(
        "project_check",
        json(project_check_of(&state, handle.clone(), None).unwrap()),
    );
    put(
        "project_check_frames",
        json(project_check_frames_impl(&state, handle.clone(), None).unwrap()),
    );
    put(
        "export_deliveries",
        json(export_deliveries_impl(&state, handle.clone()).unwrap()),
    );
    put("layout_favourites_list", json(Vec::<Value>::new()));
    put(
        "playback_status",
        json(playback_status_impl(&state, handle.clone()).unwrap()),
    );
    put(
        "project_zoom_suggestions",
        json(project_zoom_suggestions_impl(&state, handle.clone(), None).unwrap()),
    );
    put(
        "project_caption_cues",
        json(transcript::project_caption_cues_impl(&state, handle.clone(), None).unwrap()),
    );
    put(
        "project_sound_effects",
        json(project_sound_effects_impl(&state, handle.clone(), None).unwrap()),
    );
    put(
        "project_waveform_overview",
        json(project_waveform_overview_impl(&state, handle.clone(), mic.clone()).unwrap()),
    );
    put(
        "transcript_get",
        json(transcript::transcript_get_impl(&state, handle.clone(), mic.clone(), None).unwrap()),
    );
    put(
        "mcp_status",
        json(crate::mcp::status(&dir.path().join("config"))),
    );

    // This machine's folders never reach the file.
    let mut text = serde_json::to_string_pretty(&Value::Object(out)).unwrap();
    let mut local: Vec<(String, &str)> = vec![(dir.path().to_string_lossy().into(), SHOWN_HOME)];
    local.extend(
        std::env::current_exe()
            .ok()
            .map(|p| (p.to_string_lossy().into(), "/opt/aeroedits/aeroedits")),
    );
    local.extend(dirs::home_dir().map(|p| (p.to_string_lossy().into(), "/home/george")));
    for (path, shown) in local {
        text = text.replace(&path, shown);
    }
    let text = regex::Regex::new(r"/tmp/\.tmp\w+")
        .unwrap()
        .replace_all(&text, SHOWN_HOME);
    let mock = Path::new(env!("CARGO_MANIFEST_DIR")).join("../front-end/src/mock");
    std::fs::write(mock.join("backend.json"), format!("{text}\n")).unwrap();

    // The preview's picture: the project's frame at 1 s, as the export draws it.
    drop(state);
    let video = dir.path().join("preview.mp4");
    let request = crate::render_cli::parse(&[
        folder.to_string_lossy().into(),
        "--out".into(),
        video.to_string_lossy().into(),
        "--size".into(),
        "960x540".into(),
    ])
    .unwrap();
    crate::render_cli::render(&request, |_, _| {}).unwrap();
    crate::media::release_decoders();
    let frame = mock.join("frame.jpg");
    let status = std::process::Command::new(crate::media::ffmpeg::ffmpeg_path().unwrap())
        .args(["-v", "error", "-y", "-ss", "1", "-i"])
        .arg(&video)
        .args(["-frames:v", "1", "-q:v", "6"])
        .arg(&frame)
        .status()
        .unwrap();
    assert!(status.success(), "ffmpeg could not take the frame");
    eprintln!("wrote {}", mock.display());
}
