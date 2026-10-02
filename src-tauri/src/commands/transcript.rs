//! Transcription commands. Transcripts are per audio track and stored in the project folder;
//! word deletions and suggestions become ordinary ripple cuts on the edit document.
use super::{project_ripple_cuts_impl, AppState, EditCut};
use crate::project::reader::SegmentSummary;
use crate::project::{OpenedProject, TrackType};
use crate::transcript::edit;
use crate::transcript::elevenlabs::ScribeTranscriber;
use crate::transcript::parakeet::{self, ParakeetTranscriber};
use crate::transcript::provider::{transcribe_segments, ChunkTranscriber};
use crate::transcript::settings::{self, config_dir};
use crate::transcript::{
    store, ProviderKind, TranscriptCutSuggestion, TranscriptSettings, TranscriptSettingsView,
    TranscriptView, TranscriptionProgress,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

/// One transcription or model download at a time; `cancel` stops whichever is running.
#[derive(Default)]
pub struct TranscriptState {
    pub running: Mutex<Option<String>>,
    pub cancel: AtomicBool,
}

struct RunGuard<'a>(&'a TranscriptState);

impl<'a> RunGuard<'a> {
    fn start(state: &'a TranscriptState, what: String) -> Result<Self, String> {
        let mut running = state.running.lock();
        if let Some(current) = running.as_ref() {
            return Err(format!("Already busy: {current}"));
        }
        *running = Some(what);
        state.cancel.store(false, Ordering::SeqCst);
        Ok(Self(state))
    }
}

impl Drop for RunGuard<'_> {
    fn drop(&mut self) {
        *self.0.running.lock() = None;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptRunResult {
    pub view: TranscriptView,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelDownloadProgress {
    pub bytes_done: u64,
    pub bytes_total: Option<u64>,
}

struct TrackContext {
    root: PathBuf,
    segments: Vec<SegmentSummary>,
}

fn audio_track(
    state: &AppState,
    project_handle: &str,
    track_id: &str,
) -> Result<TrackContext, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    super::require_handle(reader, project_handle)?;
    let track = reader
        .summary
        .tracks
        .iter()
        .find(|t| t.descriptor.id == track_id)
        .ok_or("Unknown track")?;
    if !matches!(
        track.descriptor.track_type,
        TrackType::MicAudio | TrackType::SystemAudio
    ) {
        return Err("Only audio tracks can be transcribed".into());
    }
    Ok(TrackContext {
        root: reader.root().to_path_buf(),
        segments: reader
            .segments_for(track_id)
            .ok_or("Unknown track")?
            .to_vec(),
    })
}

fn current_view(
    state: &AppState,
    project_handle: &str,
    track_id: &str,
) -> Result<Option<TranscriptView>, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    super::require_handle(reader, project_handle)?;
    let Some(transcript) = store::load_transcript(reader.root(), track_id)? else {
        return Ok(None);
    };
    let document = &reader.history().current;
    Ok(Some(edit::view(
        &transcript,
        &document.mapper()?,
        document.revision,
    )))
}

/// Playback keeps its own copy of the captions; rebuild it after the transcript changes.
fn refresh_playback(state: &AppState) {
    let opened = state.opened_project.lock();
    if let Some(reader) = opened.as_ref() {
        if reader.history().current.captions.enabled {
            let _ = state
                .playback
                .lock()
                .apply_document(&reader.history().current);
        }
    }
}

/// Loads, changes and saves the transcript of `track_id` in the open project.
fn modify_transcript<T>(
    state: &AppState,
    project_handle: &str,
    track_id: &str,
    change: impl FnOnce(&mut crate::transcript::Transcript) -> Result<T, String>,
) -> Result<T, String> {
    let _guard = state.command_lock.lock();
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    super::require_handle(reader, project_handle)?;
    let mut transcript =
        store::load_transcript(reader.root(), track_id)?.ok_or("Transcribe this track first")?;
    let out = change(&mut transcript)?;
    store::save_transcript(reader.root(), &transcript)?;
    Ok(out)
}

pub fn transcript_settings_get_impl() -> TranscriptSettingsView {
    settings::settings_view(&config_dir())
}

pub fn transcript_settings_set_impl(
    new_settings: TranscriptSettings,
) -> Result<TranscriptSettingsView, String> {
    let dir = config_dir();
    settings::save_settings(&dir, &new_settings)?;
    Ok(settings::settings_view(&dir))
}

pub fn transcript_set_api_key_impl(key: String) -> Result<TranscriptSettingsView, String> {
    let dir = config_dir();
    settings::set_api_key(&dir, &key)?;
    Ok(settings::settings_view(&dir))
}

pub fn transcript_get_impl(
    state: &AppState,
    project_handle: String,
    track_id: String,
) -> Result<Option<TranscriptView>, String> {
    current_view(state, &project_handle, &track_id)
}

pub fn transcript_run_impl(
    state: &AppState,
    transcripts: &TranscriptState,
    project_handle: String,
    track_id: String,
    progress: &mut dyn FnMut(TranscriptionProgress),
) -> Result<TranscriptRunResult, String> {
    let _run = RunGuard::start(transcripts, format!("transcribing {track_id}"))?;
    let ctx = audio_track(state, &project_handle, &track_id)?;
    let dir = config_dir();
    let config = settings::load_settings(&dir);
    let language = Some(config.language.clone()).filter(|l| !l.is_empty());
    let mut transcriber: Box<dyn ChunkTranscriber> = match config.provider {
        ProviderKind::ElevenLabs => {
            let key = settings::api_key(&dir)
                .map(|(k, _)| k)
                .ok_or("Add your ElevenLabs API key in transcription settings first")?;
            Box::new(ScribeTranscriber::new(
                key,
                Some(config.scribe_model.clone()),
                language,
                config.keyterms.clone(),
            )?)
        }
        ProviderKind::Parakeet => {
            progress(TranscriptionProgress {
                track_id: track_id.clone(),
                fraction: 0.0,
                message: "Loading the Parakeet model".into(),
            });
            Box::new(ParakeetTranscriber::new(&config.parakeet_dir())?)
        }
    };
    let output = transcribe_segments(
        transcriber.as_mut(),
        &ctx.root,
        &track_id,
        &ctx.segments,
        &transcripts.cancel,
        progress,
    )?;
    store::save_transcript(&ctx.root, &output.transcript)?;
    refresh_playback(state);
    let view = current_view(state, &project_handle, &track_id)?
        .ok_or("The transcript was saved but could not be read back")?;
    Ok(TranscriptRunResult {
        view,
        diagnostics: output.diagnostics,
    })
}

pub fn transcript_cancel_impl(transcripts: &TranscriptState) {
    transcripts.cancel.store(true, Ordering::SeqCst);
}

pub fn transcript_delete_impl(
    state: &AppState,
    project_handle: String,
    track_id: String,
) -> Result<(), String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    super::require_handle(reader, &project_handle)?;
    store::delete_transcript(reader.root(), &track_id)?;
    drop(opened);
    refresh_playback(state);
    Ok(())
}

pub fn transcript_suggestions_impl(
    state: &AppState,
    project_handle: String,
    track_id: String,
) -> Result<Vec<TranscriptCutSuggestion>, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    super::require_handle(reader, &project_handle)?;
    let transcript =
        store::load_transcript(reader.root(), &track_id)?.ok_or("Transcribe this track first")?;
    Ok(edit::suggestions(
        &transcript,
        &reader.history().current.mapper()?,
    ))
}

pub fn transcript_set_word_text_impl(
    state: &AppState,
    project_handle: String,
    track_id: String,
    word_id: String,
    text: String,
) -> Result<TranscriptView, String> {
    modify_transcript(state, &project_handle, &track_id, |t| {
        t.set_word_text(&word_id, &text)
    })?;
    refresh_playback(state);
    current_view(state, &project_handle, &track_id)?.ok_or_else(|| "Transcript disappeared".into())
}

pub fn transcript_dismiss_suggestions_impl(
    state: &AppState,
    project_handle: String,
    track_id: String,
    ids: Vec<String>,
    dismissed: bool,
) -> Result<Vec<TranscriptCutSuggestion>, String> {
    modify_transcript(state, &project_handle, &track_id, |t| {
        t.set_dismissed(&ids, dismissed)
    })?;
    transcript_suggestions_impl(state, project_handle, track_id)
}

pub fn transcript_cut_words_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    track_id: String,
    word_ids: Vec<String>,
) -> Result<OpenedProject, String> {
    let cuts = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        super::require_handle(reader, &project_handle)?;
        let document = &reader.history().current;
        if document.revision != expected_revision {
            return Err("Stale edit revision".into());
        }
        let transcript = store::load_transcript(reader.root(), &track_id)?
            .ok_or("Transcribe this track first")?;
        edit::word_cuts(&transcript, &word_ids, &document.mapper()?)?
    };
    project_ripple_cuts_impl(
        state,
        project_handle,
        expected_revision,
        cuts.into_iter()
            .map(|(start_us, end_us)| EditCut { start_us, end_us })
            .collect(),
    )
}

pub fn transcript_download_model_impl(
    transcripts: &TranscriptState,
    progress: &mut dyn FnMut(ModelDownloadProgress),
) -> Result<TranscriptSettingsView, String> {
    let _run = RunGuard::start(transcripts, "downloading the Parakeet model".into())?;
    let dir = config_dir();
    let model_dir = settings::load_settings(&dir).parakeet_dir();
    parakeet::download_model(
        &model_dir,
        &transcripts.cancel,
        &mut |bytes_done, bytes_total| {
            progress(ModelDownloadProgress {
                bytes_done,
                bytes_total,
            })
        },
    )?;
    Ok(settings::settings_view(&dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_job_runs_at_a_time() {
        let state = TranscriptState::default();
        let guard = RunGuard::start(&state, "a".into()).unwrap();
        assert!(RunGuard::start(&state, "b".into()).is_err());
        drop(guard);
        assert!(RunGuard::start(&state, "b".into()).is_ok());
    }
}

pub fn ai_settings_get_impl() -> crate::ai::AiSettingsView {
    crate::ai::settings_view(&config_dir())
}

pub fn ai_settings_set_impl(
    new_settings: crate::ai::AiSettings,
) -> Result<crate::ai::AiSettingsView, String> {
    let dir = config_dir();
    crate::ai::save_settings(&dir, &new_settings)?;
    Ok(crate::ai::settings_view(&dir))
}

pub fn ai_set_api_key_impl(
    provider: crate::ai::AiProvider,
    key: String,
) -> Result<crate::ai::AiSettingsView, String> {
    let dir = config_dir();
    crate::secrets::set(&dir, provider.key_spec(), &key)?;
    Ok(crate::ai::settings_view(&dir))
}

/// Sends the kept words to the chosen AI provider and stores the filler and retake spans it
/// finds with the transcript. Returns the merged suggestion list.
pub fn transcript_ai_suggest_impl(
    state: &AppState,
    transcripts: &TranscriptState,
    project_handle: String,
    track_id: String,
    progress: &mut dyn FnMut(TranscriptionProgress),
) -> Result<Vec<TranscriptCutSuggestion>, String> {
    let _run = RunGuard::start(transcripts, format!("AI review of {track_id}"))?;
    let (created_at, words) = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        super::require_handle(reader, &project_handle)?;
        let transcript = store::load_transcript(reader.root(), &track_id)?
            .ok_or("Transcribe this track first")?;
        let mapper = reader.history().current.mapper()?;
        let words: Vec<crate::transcript::TranscriptWord> = transcript
            .words
            .iter()
            .filter(|w| w.kind == crate::transcript::WordKind::Word && edit::kept(w, &mapper))
            .cloned()
            .collect();
        (transcript.created_at, words)
    };
    // No lock is held while waiting on the network.
    let mut client = crate::ai::client_from_settings(&config_dir())?;
    let refs: Vec<&crate::transcript::TranscriptWord> = words.iter().collect();
    let spans = crate::ai::fillers::detect(
        &mut client,
        &refs,
        &mut |fraction| {
            progress(TranscriptionProgress {
                track_id: track_id.clone(),
                fraction: fraction as f64,
                message: format!("AI review {}%", (fraction * 100.0).round()),
            })
        },
        &|| !transcripts.cancel.load(Ordering::SeqCst),
    )?;
    let model = crate::ai::client::JsonModel::describe(&client);
    modify_transcript(state, &project_handle, &track_id, |t| {
        if t.created_at != created_at {
            return Err(
                "The track was transcribed again during the AI review; run it again".into(),
            );
        }
        t.ai_suggestions = spans;
        t.ai_model = Some(model);
        Ok(())
    })?;
    transcript_suggestions_impl(state, project_handle, track_id)
}

/// Asks the AI provider for chapters from a track's transcript and replaces the project's
/// chapters with them (one undoable edit).
pub fn project_chapters_generate_impl(
    state: &AppState,
    transcripts: &TranscriptState,
    project_handle: String,
    track_id: String,
) -> Result<OpenedProject, String> {
    let _run = RunGuard::start(transcripts, "Finding chapters".into())?;
    let words = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        super::require_handle(reader, &project_handle)?;
        let transcript = store::load_transcript(reader.root(), &track_id)?
            .ok_or("Transcribe this track first")?;
        let document = &reader.history().current;
        edit::view(&transcript, &document.mapper()?, document.revision).words
    };
    let mut client = crate::ai::client_from_settings(&config_dir())?;
    let chapters = crate::ai::chapters::suggest(&mut client, &words)?;
    if transcripts.cancel.load(Ordering::SeqCst) {
        return Err("Cancelled".into());
    }
    // Chapters are anchored in source time, so they apply to whatever the edit is now.
    super::mutate_opened(state, project_handle, |reader| {
        let revision = reader.history().current.revision;
        reader.set_chapters(revision, chapters)
    })
}
