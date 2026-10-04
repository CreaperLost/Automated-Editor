//! Probes on copies of real projects, run by hand:
//! `AERO_PROBE=<dir> cargo test --lib parity -- --ignored --nocapture`, where `<dir>` holds
//! `probe` (a project with an imported video and its transcript) and `aero` (a recording with
//! a microphone transcript). They print what the editor finds (pauses, transcript positions,
//! zooms) so a change to the timeline model can be checked against the numbers from before.
#![cfg(test)]

use crate::commands::{self, AppState};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

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

fn balanced() -> crate::dsp::silence::SilenceConfig {
    crate::dsp::silence::SilenceConfig {
        threshold_db: -38.0,
        min_duration_ms: 400,
        padding_ms: 50,
        ..Default::default()
    }
}

fn pauses(result: &crate::dsp::silence::SilenceDetectionResult) -> Value {
    json!(result
        .suggestions
        .iter()
        .map(|s| [s.start_us, s.end_us, s.source_start_us, s.source_end_us])
        .collect::<Vec<_>>())
}

fn words(view: &Option<crate::transcript::edit::TranscriptView>) -> Value {
    let Some(view) = view else {
        return Value::Null;
    };
    let placed: Vec<(u64, u64)> = view
        .words
        .iter()
        .filter_map(|w| w.edited_start_us.zip(w.edited_end_us))
        .collect();
    let edges: Vec<_> = placed
        .iter()
        .take(3)
        .chain(placed.iter().rev().take(3))
        .collect();
    json!({ "total": view.words.len(), "placed": placed.len(), "edges": edges })
}

/// The project with an imported video: its streams, the pauses found on its sound, where its
/// transcript's words land, and the same after cutting 5 s to 7 s.
#[test]
#[ignore]
fn parity_imported_media() {
    let Some(dir) = probe_dir() else { return };
    let work = tempfile::tempdir().unwrap();
    let root = work.path().join("probe");
    copy_dir(&dir.join("probe"), &root);
    // The project as the editor makes it now: the same file imported and put on the timeline,
    // with its transcript.
    let old: Value =
        serde_json::from_slice(&std::fs::read(root.join("project.json")).unwrap()).unwrap();
    let source = old["mediaAssets"][0]["sourcePath"]
        .as_str()
        .unwrap()
        .to_string();
    let old_id = old["mediaAssets"][0]["id"].as_str().unwrap().to_string();
    std::fs::remove_file(root.join("project.json")).unwrap();
    let state = AppState::new();
    let opened = commands::open_project_impl(&state, root.to_string_lossy().into()).unwrap();
    let handle = opened.project_handle.clone();
    let imported =
        commands::project_media_import_impl(&state, handle.clone(), 0, vec![source]).unwrap();
    let asset = imported.assets[0].id.clone();
    commands::project_sequence_edit_impl(
        &state,
        handle.clone(),
        1,
        crate::sequence::edit::SequenceEdit::PlaceAsset {
            asset_id: asset,
            at_us: 0,
            track_id: None,
            streams: vec![],
            range: None,
        },
        None,
    )
    .unwrap();
    let key = probe_media_sound_key(&state);
    let mut transcript: Value = serde_json::from_slice(
        &std::fs::read(root.join(format!("transcripts/msound-0-{old_id}.json"))).unwrap(),
    )
    .unwrap();
    transcript["trackId"] = json!(key);
    std::fs::write(
        root.join(format!("transcripts/{key}.json")),
        serde_json::to_vec(&transcript).unwrap(),
    )
    .unwrap();
    let before =
        commands::detect_silence_impl(&state, handle.clone(), key.clone(), balanced()).unwrap();
    let words_before =
        commands::transcript::transcript_get_impl(&state, handle.clone(), key.clone()).unwrap();
    probe_cut(&state, &handle, (5_000_000, 7_000_000));
    let words_after =
        commands::transcript::transcript_get_impl(&state, handle.clone(), key.clone()).unwrap();
    let after = commands::detect_silence_impl(&state, handle, key, balanced()).unwrap();
    println!(
        "PARITY imported {}",
        json!({
            "streams": probe_stream_names(&state),
            "pauses": pauses(&before),
            "words": words(&words_before),
            "wordsAfterCut": words(&words_after),
            "pausesAfterCut": pauses(&after),
        })
    );
}

/// A fresh project made from the recording: pauses on the microphone, transcript positions,
/// the zooms found, and the same after cutting 5 s to 7 s.
#[test]
#[ignore]
fn parity_recording() {
    let Some(dir) = probe_dir() else { return };
    let work = tempfile::tempdir().unwrap();
    let recording = work.path().join("rec.aero");
    copy_dir(&dir.join("aero"), &recording);
    let _ = std::fs::remove_file(recording.join("project.json"));
    let transcripts = recording.join("transcripts");
    let folder =
        crate::project::folder::create_project_folder(work.path(), "Probe", Some(&recording))
            .unwrap();
    let state = AppState::new();
    let opened = commands::open_project_impl(&state, folder.to_string_lossy().into()).unwrap();
    let handle = opened.project_handle.clone();
    let key = probe_mic_key(&state);
    std::fs::create_dir_all(folder.join("transcripts")).unwrap();
    let mut transcript: Value =
        serde_json::from_slice(&std::fs::read(transcripts.join("mic.json")).unwrap()).unwrap();
    transcript["trackId"] = json!(key);
    std::fs::write(
        folder.join("transcripts").join(format!("{key}.json")),
        serde_json::to_vec(&transcript).unwrap(),
    )
    .unwrap();
    let before =
        commands::detect_silence_impl(&state, handle.clone(), key.clone(), balanced()).unwrap();
    let words_before =
        commands::transcript::transcript_get_impl(&state, handle.clone(), key.clone()).unwrap();
    let zooms = commands::project_zoom_suggestions_impl(&state, handle.clone(), None).unwrap();
    probe_cut(&state, &handle, (5_000_000, 7_000_000));
    let words_after =
        commands::transcript::transcript_get_impl(&state, handle.clone(), key.clone()).unwrap();
    let after = commands::detect_silence_impl(&state, handle, key, balanced()).unwrap();
    println!(
        "PARITY recording {}",
        json!({
            "pauses": pauses(&before),
            "words": words(&words_before),
            "zooms": zooms.suggestions.iter().map(|z| json!([z.source_start_us, z.source_end_us, z.scale])).collect::<Vec<_>>(),
            "wordsAfterCut": words(&words_after),
            "pausesAfterCut": pauses(&after),
        })
    );
}

// What the probes need from the model: these change with it, the output above does not.

fn probe_media_sound_key(state: &AppState) -> String {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().unwrap();
    let asset = &reader.history().current.assets[0];
    crate::sequence::StreamRef::new(&asset.id, &asset.speech_stream().unwrap().id).key()
}

fn probe_stream_names(state: &AppState) -> Value {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().unwrap();
    let asset = &reader.history().current.assets[0];
    let sound: Vec<&str> = asset
        .streams
        .iter()
        .filter(|s| s.kind == crate::sequence::StreamKind::Sound)
        .map(|s| s.name.as_str())
        .collect();
    let kind = match asset.kind {
        crate::sequence::AssetKind::Video => "video",
        _ => "other",
    };
    json!({ "kind": kind, "sound": sound, "duration": asset.duration_us })
}

fn probe_mic_key(state: &AppState) -> String {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().unwrap();
    format!("{}.mic", reader.history().current.assets[0].id)
}

fn probe_cut(state: &AppState, handle: &str, cut: (u64, u64)) {
    let revision = state
        .opened_project
        .lock()
        .as_ref()
        .unwrap()
        .summary
        .revision;
    commands::project_ripple_cuts_impl(
        state,
        handle.to_string(),
        revision,
        vec![commands::EditCut {
            start_us: cut.0,
            end_us: cut.1,
        }],
    )
    .unwrap();
}
