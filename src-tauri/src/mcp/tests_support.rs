//! An app around the server for tests: its state, and nothing to tell.
use crate::commands::{transcript::TranscriptState, AppState};

pub struct BareHost {
    pub state: AppState,
    pub transcripts: TranscriptState,
    /// Stands in for the app's settings folder.
    pub config: tempfile::TempDir,
}

impl Default for BareHost {
    fn default() -> Self {
        Self {
            state: AppState::new(),
            transcripts: TranscriptState::default(),
            config: tempfile::tempdir().unwrap(),
        }
    }
}

/// Speech stand-ins (a tone) at these times, in ms, with silence between.
pub const SPOKEN_MS: [(u64, u64); 3] = [(0, 800), (1600, 2400), (3000, 4000)];

/// A project with one 4 s sound on the timeline (a tone where words are, silence between) and a
/// transcript of it with a filler: "Hello um there. So next step now." Returns the host and the
/// sound's stream key.
pub fn lesson(dir: &std::path::Path) -> (BareHost, String) {
    use crate::commands;
    let rate = 48_000u64;
    let click = rate * 1200 / 1000;
    let samples: Vec<i16> = (0..rate * 4)
        .map(|i| {
            let ms = i * 1000 / rate;
            if SPOKEN_MS.iter().any(|&(a, b)| ms >= a && ms < b) {
                ((i as f64 * 220.0 * std::f64::consts::TAU / rate as f64).sin() * 9000.0) as i16
            } else if (click..click + 24).contains(&i) {
                // A mouth click in the pause at 1.2 s.
                if i % 2 == 0 {
                    20_000
                } else {
                    -20_000
                }
            } else {
                0
            }
        })
        .collect();
    let wav = dir.join("voice.wav");
    std::fs::write(
        &wav,
        crate::fixtures::generate_pcm16_wav(rate as u32, 1, &samples),
    )
    .unwrap();
    let folder = crate::project::folder::create_project_folder(dir, "Lesson", None).unwrap();
    let host = BareHost::default();
    let state = &host.state;
    let opened = commands::open_project_impl(state, folder.to_string_lossy().into()).unwrap();
    let opened = commands::project_media_import_impl(
        state,
        opened.project_handle.clone(),
        opened.revision,
        vec![wav.to_string_lossy().into()],
    )
    .unwrap();
    let asset = opened.assets[0].clone();
    // A voice, as a recording's microphone is.
    let opened = commands::project_media_roles_impl(
        state,
        opened.project_handle.clone(),
        opened.revision,
        asset.id.clone(),
        vec![commands::StreamRoleInput {
            stream: asset.streams[0].id.clone(),
            role: crate::sequence::Role::Mic,
        }],
    )
    .unwrap();
    commands::project_sequence_edit_impl(
        state,
        opened.project_handle.clone(),
        opened.revision,
        serde_json::from_value(serde_json::json!({
            "kind": "placeAsset", "assetId": asset.id, "atUs": 0
        }))
        .unwrap(),
        None,
    )
    .unwrap();
    let key = format!("{}.{}", asset.id, asset.streams[0].id);
    let words = [
        ("Hello", 0, 300),
        ("um", 350, 600),
        ("there.", 620, 790),
        ("So", 1600, 1800),
        ("next", 1850, 2100),
        ("step", 2150, 2390),
        ("now.", 3000, 3900),
    ];
    let mut transcript = crate::transcript::Transcript::new(
        key.clone(),
        crate::transcript::ProviderKind::Parakeet,
        "test".into(),
        None,
        Vec::new(),
    );
    transcript.words = words
        .iter()
        .enumerate()
        .map(
            |(i, (text, start, end))| crate::transcript::TranscriptWord {
                id: format!("w-{i}"),
                text: (*text).into(),
                kind: crate::transcript::WordKind::Word,
                source_start_us: start * 1000,
                source_end_us: end * 1000,
                confidence: None,
                speaker: None,
            },
        )
        .collect();
    crate::transcript::store::save_transcript(&folder, &transcript).unwrap();
    (host, key)
}

impl super::Host for BareHost {
    fn state(&self) -> &AppState {
        &self.state
    }

    fn transcripts(&self) -> &TranscriptState {
        &self.transcripts
    }

    /// Tests never spend the user's AI key.
    fn ai_review_ready(&self) -> bool {
        false
    }

    fn config_dir(&self) -> std::path::PathBuf {
        self.config.path().to_path_buf()
    }
}
