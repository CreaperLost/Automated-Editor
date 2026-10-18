//! Transcription commands. Transcripts are per audio track and stored in the project folder;
//! word deletions and suggestions become ordinary ripple cuts on the edit document.
use super::AppState;
use crate::project::reader::SegmentSummary;
use crate::project::OpenedProject;
use crate::transcript::align;
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
    /// Shared with AI clients so Cancel also stops a request waiting on the network.
    pub cancel: std::sync::Arc<AtomicBool>,
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

/// The sound stream (`<asset>.<stream>`) to transcribe, over its own time.
fn audio_track(
    state: &AppState,
    project_handle: &str,
    track_id: &str,
) -> Result<TrackContext, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    super::require_handle(reader, project_handle)?;
    let ctx = super::sound_context(reader, track_id, false)?;
    Ok(TrackContext {
        root: ctx.root,
        segments: ctx.segments,
    })
}

fn current_view(
    state: &AppState,
    project_handle: &str,
    track_id: &str,
    short: Option<&str>,
) -> Result<Option<TranscriptView>, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    super::require_handle(reader, project_handle)?;
    let Some(transcript) = store::load_transcript(reader.root(), track_id)? else {
        return Ok(None);
    };
    Ok(Some(edit::view(
        &transcript,
        &transcript_clock(reader, track_id, short)?,
        reader.history().current.revision,
    )))
}

/// The short whose own timeline a sound's words are placed on and cut from: `short` when it
/// names one, else the take short the sound is the voice of (it plays only there), else none.
fn target_short<'a>(
    document: &'a crate::project::revision::EditDocument,
    key: &str,
    short: Option<&str>,
) -> Option<&'a crate::shorts::Short> {
    short
        .and_then(|id| document.shorts.iter().find(|s| s.id == id))
        .or_else(|| crate::shorts::take_short_of(document, key))
}

/// Where a sound's words play: a short's own timeline (see [`target_short`]), else the
/// video's.
fn transcript_clock(
    reader: &crate::project::ProjectReader,
    key: &str,
    short: Option<&str>,
) -> Result<crate::timeline::TimelineMapper, String> {
    let document = &reader.history().current;
    match target_short(document, key, short) {
        Some(short) => crate::shorts::short_timeline(document, short)?.transcript_clock(key),
        None => document.transcript_clock(key),
    }
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

/// The words of `track_id` where they play: in `short`'s own timeline when one is named.
pub fn transcript_get_impl(
    state: &AppState,
    project_handle: String,
    track_id: String,
    short: Option<String>,
) -> Result<Option<TranscriptView>, String> {
    current_view(state, &project_handle, &track_id, short.as_deref())
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
            Box::new(
                ScribeTranscriber::new(
                    key,
                    Some(config.scribe_model.clone()),
                    language,
                    config.keyterms.clone(),
                )?
                .with_cancel(transcripts.cancel.clone()),
            )
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
    let mut output = transcribe_segments(
        transcriber.as_mut(),
        &ctx.root,
        &track_id,
        &ctx.segments,
        &transcripts.cancel,
        progress,
    )?;
    // Cancel pressed while the last chunk finished keeps the transcript there was.
    if transcripts.cancel.load(Ordering::SeqCst) {
        return Err("Transcription cancelled".into());
    }
    // A transcript that could not be aligned is still a transcript: keep it and say why.
    match align_if_wanted(&mut output.transcript, &ctx, &transcripts.cancel, progress) {
        Ok(Some(info)) => output.diagnostics.push(format!(
            "Aligned {} words to the audio ({} kept their times)",
            info.aligned_words, info.kept_words
        )),
        Ok(None) => {}
        Err(_) if transcripts.cancel.load(Ordering::SeqCst) => {
            return Err("Transcription cancelled".into());
        }
        Err(error) => output
            .diagnostics
            .push(format!("Words were not aligned to the audio: {error}")),
    }
    {
        let _guard = state.command_lock.lock();
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        super::require_handle(reader, &project_handle)?;
        if reader.root() != ctx.root {
            return Err("The project changed during transcription".into());
        }
        store::save_transcript(reader.root(), &output.transcript)?;
    }
    refresh_playback(state);
    let view = current_view(state, &project_handle, &track_id, None)?
        .ok_or("The transcript was saved but could not be read back")?;
    Ok(TranscriptRunResult {
        view,
        diagnostics: output.diagnostics,
    })
}

/// Aligns an existing transcript's words to the audio. Text and ids stay; cuts already made
/// stay where they are, and suggestions worked out from the old times go stale.
pub fn transcript_align_impl(
    state: &AppState,
    transcripts: &TranscriptState,
    project_handle: String,
    track_id: String,
    progress: &mut dyn FnMut(TranscriptionProgress),
) -> Result<TranscriptRunResult, String> {
    let _run = RunGuard::start(transcripts, format!("aligning words of {track_id}"))?;
    if !align::compiled_in() {
        return Err("This build of AeroEdits does not include word alignment".into());
    }
    if !align::model_present(&align::default_model_dir()) {
        return Err("Download the word alignment model in transcription settings first".into());
    }
    let ctx = audio_track(state, &project_handle, &track_id)?;
    let mut transcript = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        super::require_handle(reader, &project_handle)?;
        store::load_transcript(reader.root(), &track_id)?.ok_or("Transcribe this track first")?
    };
    let before = transcript.dependency();
    let info = run_alignment(&mut transcript, &ctx, &transcripts.cancel, progress)?;
    if transcripts.cancel.load(Ordering::SeqCst) {
        return Err("Alignment cancelled".into());
    }
    {
        let _guard = state.command_lock.lock();
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        super::require_handle(reader, &project_handle)?;
        // Someone edited the words while the audio was read: theirs win.
        let now = store::load_transcript(reader.root(), &track_id)?.map(|t| t.dependency());
        if now.as_ref() != Some(&before) {
            return Err("The transcript changed while aligning; align it again".into());
        }
        store::save_transcript(reader.root(), &transcript)?;
    }
    refresh_playback(state);
    let view = current_view(state, &project_handle, &track_id, None)?
        .ok_or("The transcript was saved but could not be read back")?;
    Ok(TranscriptRunResult {
        view,
        diagnostics: vec![format!(
            "Aligned {} words to the audio ({} kept their times)",
            info.aligned_words, info.kept_words
        )],
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
    let _guard = state.command_lock.lock();
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
    short: Option<String>,
) -> Result<Vec<TranscriptCutSuggestion>, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    super::require_handle(reader, &project_handle)?;
    let transcript =
        store::load_transcript(reader.root(), &track_id)?.ok_or("Transcribe this track first")?;
    let found = edit::suggestions(
        &transcript,
        &transcript_clock(reader, &track_id, short.as_deref())?,
    );
    Ok(crate::proposals::with_agent_suggestions(
        reader,
        &transcript,
        &track_id,
        found,
    ))
}

pub fn transcript_set_word_text_impl(
    state: &AppState,
    project_handle: String,
    track_id: String,
    word_id: String,
    text: String,
    short: Option<String>,
) -> Result<TranscriptView, String> {
    modify_transcript(state, &project_handle, &track_id, |t| {
        t.set_word_text(&word_id, &text)
    })?;
    refresh_playback(state);
    current_view(state, &project_handle, &track_id, short.as_deref())?
        .ok_or_else(|| "Transcript disappeared".into())
}

/// Takes the punctuation off every word of the transcript (captions follow).
pub fn transcript_strip_punctuation_impl(
    state: &AppState,
    project_handle: String,
    track_id: String,
    short: Option<String>,
) -> Result<TranscriptView, String> {
    modify_transcript(state, &project_handle, &track_id, |t| {
        if t.strip_punctuation() == 0 {
            return Err("There is no punctuation to remove".into());
        }
        Ok(())
    })?;
    refresh_playback(state);
    current_view(state, &project_handle, &track_id, short.as_deref())?
        .ok_or_else(|| "Transcript disappeared".into())
}

pub fn transcript_dismiss_suggestions_impl(
    state: &AppState,
    project_handle: String,
    track_id: String,
    ids: Vec<String>,
    dismissed: bool,
    short: Option<String>,
) -> Result<Vec<TranscriptCutSuggestion>, String> {
    modify_transcript(state, &project_handle, &track_id, |t| {
        t.set_dismissed(&ids, dismissed)
    })?;
    transcript_suggestions_impl(state, project_handle, track_id, short)
}

pub fn transcript_cut_words_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    track_id: String,
    word_ids: Vec<String>,
) -> Result<OpenedProject, String> {
    transcript_cut_words_checked_impl(
        state,
        project_handle,
        expected_revision,
        track_id,
        word_ids,
        None,
        None,
    )
}

pub fn transcript_cut_words_checked_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    track_id: String,
    word_ids: Vec<String>,
    expected_word_stamp: Option<String>,
    short: Option<String>,
) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    let cuts = {
        super::require_handle(reader, &project_handle)?;
        let document = &reader.history().current;
        if document.revision != expected_revision {
            return Err("Stale edit revision".into());
        }
        let transcript = store::load_transcript(reader.root(), &track_id)?
            .ok_or("Transcribe this track first")?;
        if expected_word_stamp
            .as_ref()
            .is_some_and(|stamp| transcript.dependency().word_stamp.as_ref() != Some(stamp))
        {
            return Err("The transcript changed. Review the words again before cutting.".into());
        }
        edit::word_cuts(
            &transcript,
            &word_ids,
            &transcript_clock(reader, &track_id, short.as_deref())?,
        )?
    };
    // A short's words (a take's too) are cut from its own timeline.
    let short = target_short(reader.document(), &track_id, short.as_deref()).map(|s| s.id.clone());
    // Nothing is selected in the transcript sense: every track loses the same time, so
    // pictures and sound elsewhere stay in step with the words.
    let ranges = cuts
        .into_iter()
        .map(|(start_us, end_us)| crate::zoom::EditedRange { start_us, end_us })
        .collect();
    super::edit_sequence_locked(
        state,
        reader,
        expected_revision,
        &crate::sequence::edit::SequenceEdit::DeleteRange {
            ranges,
            ripple: Some(true),
        },
        short.as_deref(),
    )
}

/// Downloads the Parakeet model, or with `model` `aligner` the word alignment model.
pub fn transcript_download_model_impl(
    transcripts: &TranscriptState,
    model: Option<&str>,
    progress: &mut dyn FnMut(ModelDownloadProgress),
) -> Result<TranscriptSettingsView, String> {
    let aligner = match model.unwrap_or("parakeet") {
        "parakeet" => false,
        "aligner" => true,
        other => return Err(format!("Unknown model {other}")),
    };
    let what = if aligner {
        "downloading the word alignment model"
    } else {
        "downloading the Parakeet model"
    };
    let _run = RunGuard::start(transcripts, what.into())?;
    let dir = config_dir();
    let mut report = |bytes_done, bytes_total| {
        progress(ModelDownloadProgress {
            bytes_done,
            bytes_total,
        })
    };
    if aligner {
        align::download_model(
            &align::default_model_dir(),
            &transcripts.cancel,
            &mut report,
        )?;
    } else {
        let model_dir = settings::load_settings(&dir).parakeet_dir();
        parakeet::download_model(&model_dir, &transcripts.cancel, &mut report)?;
    }
    Ok(settings::settings_view(&dir))
}

/// Aligns `transcript`'s words to its track's audio when alignment is on and the model is
/// there. `Ok(None)` when it did not run (switched off, not downloaded, not in this build).
fn align_if_wanted(
    transcript: &mut crate::transcript::Transcript,
    ctx: &TrackContext,
    cancel: &std::sync::atomic::AtomicBool,
    progress: &mut dyn FnMut(TranscriptionProgress),
) -> Result<Option<align::AlignmentInfo>, String> {
    let config = settings::load_settings(&config_dir());
    let model_dir = align::default_model_dir();
    if !config.align_words || !align::compiled_in() || !align::model_present(&model_dir) {
        return Ok(None);
    }
    run_alignment(transcript, ctx, cancel, progress).map(Some)
}

fn run_alignment(
    transcript: &mut crate::transcript::Transcript,
    ctx: &TrackContext,
    cancel: &std::sync::atomic::AtomicBool,
    progress: &mut dyn FnMut(TranscriptionProgress),
) -> Result<align::AlignmentInfo, String> {
    let track_id = transcript.track_id.clone();
    progress(TranscriptionProgress {
        track_id: track_id.clone(),
        fraction: 0.0,
        message: "Loading the word alignment model".into(),
    });
    let (mut model, vocab) = align::load(&align::default_model_dir())?;
    align::align_transcript(
        transcript,
        model.as_mut(),
        &vocab,
        &ctx.root,
        &ctx.segments,
        cancel,
        &mut |fraction| {
            progress(TranscriptionProgress {
                track_id: track_id.clone(),
                fraction,
                message: "Aligning words to the audio".into(),
            })
        },
    )
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
    // Missing settings or key fail here, before the job slot or the project is touched.
    let mut client =
        crate::ai::client_from_settings(&config_dir())?.with_cancel(transcripts.cancel.clone());
    let _run = RunGuard::start(transcripts, format!("AI review of {track_id}"))?;
    let (dependency, words) = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        super::require_handle(reader, &project_handle)?;
        let transcript = store::load_transcript(reader.root(), &track_id)?
            .ok_or("Transcribe this track first")?;
        let mapper = transcript_clock(reader, &track_id, None)?;
        let words: Vec<crate::transcript::TranscriptWord> = transcript
            .words
            .iter()
            .filter(|w| w.kind == crate::transcript::WordKind::Word && edit::kept(w, &mapper))
            .cloned()
            .collect();
        (transcript.dependency(), words)
    };
    // No lock is held while waiting on the network.
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
        if t.dependency() != dependency {
            return Err(
                "The track was transcribed again during the AI review; run it again".into(),
            );
        }
        t.ai_suggestions = spans;
        t.ai_model = Some(model);
        Ok(())
    })?;
    transcript_suggestions_impl(state, project_handle, track_id, None)
}

/// The model name an in-house review is stored under (the connected agent's own name follows).
pub const IN_HOUSE: &str = "in-house";

/// One page of the kept words for an in-house filler and retake review: the connected agent
/// reads them itself instead of the app sending them to OpenRouter or OpenAI.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewPage {
    pub page: usize,
    pub pages: usize,
    /// Kept words in all.
    pub words: usize,
    /// Words as `[index]word`, pauses as `(pause 1.2s)`; indexes count over all pages.
    pub text: String,
    /// Names these kept words: the review is saved against it.
    pub stamp: String,
}

fn kept_words(
    state: &AppState,
    project_handle: &str,
    track_id: &str,
) -> Result<
    (
        crate::transcript::TranscriptDependency,
        Vec<crate::transcript::TranscriptWord>,
    ),
    String,
> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    super::require_handle(reader, project_handle)?;
    let transcript =
        store::load_transcript(reader.root(), track_id)?.ok_or("Transcribe this track first")?;
    let mapper = transcript_clock(reader, track_id, None)?;
    let words = transcript
        .words
        .iter()
        .filter(|w| w.kind == crate::transcript::WordKind::Word && edit::kept(w, &mapper))
        .cloned()
        .collect();
    Ok((transcript.dependency(), words))
}

fn review_stamp<'a>(
    words: impl IntoIterator<Item = &'a crate::transcript::TranscriptWord>,
) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for word in words {
        word.id.hash(&mut hasher);
        word.text.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

/// Page `page` of the kept words of `track_id`, numbered as the AI review numbers them.
pub fn transcript_review_page_impl(
    state: &AppState,
    project_handle: String,
    track_id: String,
    page: usize,
) -> Result<ReviewPage, String> {
    let (_, words) = kept_words(state, &project_handle, &track_id)?;
    let ranges = crate::ai::fillers::chunks(words.len());
    if ranges.is_empty() {
        return Err("There are no kept words to check".into());
    }
    let range = ranges.get(page).cloned().ok_or_else(|| {
        format!(
            "There are {} pages: 0 to {}",
            ranges.len(),
            ranges.len() - 1
        )
    })?;
    let refs: Vec<&crate::transcript::TranscriptWord> = words.iter().collect();
    Ok(ReviewPage {
        page,
        pages: ranges.len(),
        words: words.len(),
        text: crate::ai::fillers::chunk_prompt(&refs, range),
        stamp: review_stamp(&words),
    })
}

/// One page of a candidate review: only the words around what the local pass flagged as a
/// possible filler or retake, numbered as the full pages number them, with the same stamp.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidatePage {
    pub page: usize,
    /// Pages of windows; 0 when nothing was flagged.
    pub pages: usize,
    /// Kept words in all, and how many words and pages a full read is (its pages overlap).
    pub words: usize,
    pub full_words: usize,
    pub full_pages: usize,
    /// Words in all the windows, over every page.
    pub shown_words: usize,
    pub windows: usize,
    /// Flags to check: possible fillers and possible retakes.
    pub fillers: usize,
    pub retakes: usize,
    /// Filler sounds and retakes the rules cut by default.
    pub rule_fillers: usize,
    pub rule_retakes: usize,
    /// The page's windows; empty when nothing was flagged.
    pub text: String,
    pub stamp: String,
}

/// Page `page` of the candidate windows of `track_id`. Flags the rules settle (filler sounds,
/// clear retakes) open no window: they are cut by default.
pub fn transcript_review_candidates_impl(
    state: &AppState,
    project_handle: String,
    track_id: String,
    page: usize,
) -> Result<CandidatePage, String> {
    use crate::ai::candidates::{self, Flag};
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    super::require_handle(reader, &project_handle)?;
    let transcript =
        store::load_transcript(reader.root(), &track_id)?.ok_or("Transcribe this track first")?;
    let mapper = transcript_clock(reader, &track_id, None)?;
    let words: Vec<&crate::transcript::TranscriptWord> = transcript
        .words
        .iter()
        .filter(|w| w.kind == crate::transcript::WordKind::Word && edit::kept(w, &mapper))
        .collect();
    if words.is_empty() {
        return Err("There are no kept words to check".into());
    }
    let index: std::collections::HashMap<&str, usize> = words
        .iter()
        .enumerate()
        .map(|(i, w)| (w.id.as_str(), i))
        .collect();
    let rules: Vec<_> = edit::suggestions(&transcript, &mapper)
        .into_iter()
        .filter(|s| s.source == edit::SuggestionSource::Rules && !s.dismissed)
        .collect();
    let settled: Vec<(usize, usize)> = rules
        .iter()
        .filter_map(|s| {
            Some((
                *index.get(s.word_ids.first()?.as_str())?,
                *index.get(s.word_ids.last()?.as_str())?,
            ))
        })
        .collect();
    let rule_count = |kind| rules.iter().filter(|s| s.kind == kind).count();
    let plan = candidates::plan(&words, &settled);
    let text = match plan.page_text(&words, page) {
        Some(text) => text,
        None if page == 0 && plan.pages.is_empty() => String::new(),
        None => {
            return Err(format!(
                "There are {} pages: 0 to {}",
                plan.pages.len(),
                plan.pages.len().saturating_sub(1)
            ))
        }
    };
    let full = crate::ai::fillers::chunks(words.len());
    Ok(CandidatePage {
        page,
        pages: plan.pages.len(),
        words: words.len(),
        full_words: full.iter().map(|r| r.len()).sum(),
        full_pages: full.len(),
        shown_words: plan.shown_words(),
        windows: plan.windows.len(),
        fillers: plan.count(Flag::Filler),
        retakes: plan.count(Flag::Retake),
        rule_fillers: rule_count(crate::transcript::TranscriptSuggestionKind::Filler),
        rule_retakes: rule_count(crate::transcript::TranscriptSuggestionKind::Retake),
        text,
        stamp: review_stamp(words.iter().copied()),
    })
}

/// Saves an in-house review of `track_id`: `cuts` as the AI review answers
/// (`[{"type","from","to","why"}]`, indexes from the pages read with `stamp`). It replaces the
/// last review, whoever made it; the spans become ordinary suggestions. Returns how many were
/// kept and the merged suggestion list.
pub fn transcript_review_save_impl(
    state: &AppState,
    project_handle: String,
    track_id: String,
    stamp: &str,
    cuts: serde_json::Value,
    reviewer: &str,
) -> Result<(usize, Vec<TranscriptCutSuggestion>), String> {
    let (dependency, words) = kept_words(state, &project_handle, &track_id)?;
    if review_stamp(&words) != stamp {
        return Err(
            "The kept words changed since those pages were read (a cut, an undo, a word fixed): \
             read the pages again"
                .into(),
        );
    }
    let given = cuts.as_array().map_or(0, Vec::len);
    let refs: Vec<&crate::transcript::TranscriptWord> = words.iter().collect();
    let spans =
        crate::ai::fillers::parse_spans(&serde_json::json!({ "cuts": cuts }), &refs, 0..refs.len());
    if spans.len() > crate::transcript::MAX_AI_SUGGESTIONS {
        return Err("Too many cuts for one review".into());
    }
    let kept = spans.len();
    if given > 0 && kept == 0 {
        return Err(
            "None of those cuts could be used: each needs \"type\" (filler or retake), \"from\" \
             and \"to\" (indexes from the pages, inclusive; a filler at most 4 words, a retake \
             at most 60)"
                .into(),
        );
    }
    let model: String = format!("{IN_HOUSE} ({reviewer})")
        .chars()
        .take(200)
        .collect();
    modify_transcript(state, &project_handle, &track_id, |t| {
        if t.dependency() != dependency {
            return Err(
                "The track was transcribed again during the review; review it again".into(),
            );
        }
        t.ai_suggestions = spans;
        t.ai_model = Some(model);
        Ok(())
    })?;
    Ok((
        kept,
        transcript_suggestions_impl(state, project_handle, track_id, None)?,
    ))
}

/// Asks the AI provider for chapters from a track's transcript and replaces the project's
/// chapters with them (one undoable edit).
pub fn project_chapters_generate_impl(
    state: &AppState,
    transcripts: &TranscriptState,
    project_handle: String,
    track_id: String,
) -> Result<OpenedProject, String> {
    // Missing settings or key fail here, before the job slot or the project is touched.
    let mut client =
        crate::ai::client_from_settings(&config_dir())?.with_cancel(transcripts.cancel.clone());
    let _run = RunGuard::start(transcripts, "Finding chapters".into())?;
    let words = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        super::require_handle(reader, &project_handle)?;
        let transcript = store::load_transcript(reader.root(), &track_id)?
            .ok_or("Transcribe this track first")?;
        let document = &reader.history().current;
        edit::view(
            &transcript,
            &document.transcript_clock(&track_id)?,
            document.revision,
        )
        .words
    };
    let mut chapters = crate::ai::chapters::suggest(&mut client, &words)?;
    // The words' times are their recording's or file's own.
    let asset = crate::sequence::StreamRef::parse(&track_id).map(|r| r.asset);
    for chapter in &mut chapters {
        chapter.media = asset.clone();
    }
    if transcripts.cancel.load(Ordering::SeqCst) {
        return Err("Cancelled".into());
    }
    // Chapters are anchored in source time, so they apply to whatever the edit is now.
    super::mutate_opened(state, project_handle, |reader| {
        let revision = reader.history().current.revision;
        reader.set_chapters(revision, chapters)
    })
}

/// Asks the AI provider for moments that work as shorts and replaces the shorts list with
/// them (one undoable edit).
pub fn project_shorts_generate_impl(
    state: &AppState,
    transcripts: &TranscriptState,
    project_handle: String,
    track_id: String,
) -> Result<OpenedProject, String> {
    // Missing settings or key fail here, before the job slot or the project is touched.
    let mut client =
        crate::ai::client_from_settings(&config_dir())?.with_cancel(transcripts.cancel.clone());
    let _run = RunGuard::start(transcripts, "Finding shorts".into())?;
    let words = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        super::require_handle(reader, &project_handle)?;
        let transcript = store::load_transcript(reader.root(), &track_id)?
            .ok_or("Transcribe this track first")?;
        let document = &reader.history().current;
        edit::view(
            &transcript,
            &document.transcript_clock(&track_id)?,
            document.revision,
        )
        .words
    };
    let mut shorts = crate::ai::shorts::suggest(&mut client, &words)?;
    // The words' times are their recording's or file's own.
    let asset = crate::sequence::StreamRef::parse(&track_id).map(|r| r.asset);
    for short in &mut shorts {
        short.media = asset.clone();
    }
    if transcripts.cancel.load(Ordering::SeqCst) {
        return Err("Cancelled".into());
    }
    super::mutate_opened(state, project_handle, |reader| {
        let revision = reader.history().current.revision;
        reader.set_shorts(revision, shorts)
    })
}

/// One caption on the timeline's caption track.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptionCueView {
    pub start_us: u64,
    pub end_us: u64,
    pub text: String,
    pub word_ids: Vec<String>,
    /// Where each word starts on the edited timeline.
    pub word_starts_us: Vec<u64>,
}

/// The caption track: the transcript captions read from and its cues in edited time.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaptionTrackView {
    pub track_id: Option<String>,
    pub cues: Vec<CaptionCueView>,
}

/// A change made on the caption track. Every change is saved in the transcript, so the
/// transcript panel, preview and export all show it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum CaptionEdit {
    /// New text for a caption's words.
    SetText { word_ids: Vec<String>, text: String },
    /// Moves or stretches a caption to edited `[start_us, end_us)`.
    Retime {
        word_ids: Vec<String>,
        start_us: u64,
        end_us: u64,
    },
    /// A new caption starts at this word.
    Split { word_id: String },
    /// This caption joins the one before it.
    Merge { word_id: String },
    /// Shows or hides words in the captions; the sound is untouched.
    Hide { word_ids: Vec<String>, hidden: bool },
}

fn caption_track(
    reader: &crate::project::reader::ProjectReader,
    short: Option<&str>,
) -> Result<CaptionTrackView, String> {
    let timeline = caption_timeline(reader, short)?;
    let document = &timeline;
    let settings = &document.captions;
    let root = reader.root();
    let candidates = crate::export::caption_candidates(document);
    let Some(transcript) =
        crate::captions::caption_source(settings, candidates, Vec::new(), |id| {
            store::load_transcript(root, id).ok().flatten()
        })
    else {
        return Ok(CaptionTrackView::default());
    };
    let mapper = document.transcript_clock(&transcript.track_id)?;
    let cues = crate::captions::build_cues(&transcript, &mapper, settings)
        .into_iter()
        .map(|cue| CaptionCueView {
            start_us: cue.start_us,
            end_us: cue.end_us,
            text: cue
                .words
                .iter()
                .map(|w| w.text.as_str())
                .collect::<Vec<_>>()
                .join(" "),
            word_starts_us: cue.words.iter().map(|w| w.start_us).collect(),
            word_ids: cue.words.into_iter().map(|w| w.id).collect(),
        })
        .collect();
    Ok(CaptionTrackView {
        track_id: Some(transcript.track_id),
        cues,
    })
}

/// The timeline captions are placed on: the project's, or short `short`'s own.
fn caption_timeline(
    reader: &crate::project::reader::ProjectReader,
    short: Option<&str>,
) -> Result<crate::project::revision::EditDocument, String> {
    let base = &reader.history().current;
    match short {
        Some(id) => {
            let short = base
                .shorts
                .iter()
                .find(|s| s.id == id)
                .ok_or("That short no longer exists")?;
            crate::shorts::short_timeline(base, short)
        }
        None => Ok(base.clone()),
    }
}

pub fn project_caption_cues_impl(
    state: &AppState,
    project_handle: String,
    short_id: Option<String>,
) -> Result<CaptionTrackView, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    super::require_handle(reader, &project_handle)?;
    caption_track(reader, short_id.as_deref())
}

pub fn transcript_caption_edit_impl(
    state: &AppState,
    project_handle: String,
    track_id: String,
    change: CaptionEdit,
    short_id: Option<String>,
) -> Result<CaptionTrackView, String> {
    // Edited times become the file's own time through the transcript's mapper.
    let span = match &change {
        CaptionEdit::Retime {
            start_us, end_us, ..
        } => {
            let opened = state.opened_project.lock();
            let reader = opened.as_ref().ok_or("No opened project")?;
            super::require_handle(reader, &project_handle)?;
            let mapper =
                caption_timeline(reader, short_id.as_deref())?.transcript_clock(&track_id)?;
            let start = mapper
                .edited_to_source_us(*start_us)
                .ok_or("A caption has to start over its own sound")?;
            let last = mapper
                .edited_to_source_us(end_us.saturating_sub(1).max(*start_us))
                .ok_or("A caption has to end over its own sound")?;
            if last < start {
                return Err("A caption cannot reach across a reordered clip".into());
            }
            Some((start, last + 1))
        }
        _ => None,
    };
    modify_transcript(state, &project_handle, &track_id, |t| match &change {
        CaptionEdit::SetText { word_ids, text } => t.replace_words(word_ids, text),
        CaptionEdit::Retime { word_ids, .. } => {
            let (start, end) = span.unwrap_or_default();
            t.retime_words(word_ids, start, end)
        }
        CaptionEdit::Split { word_id } => t.set_caption_mark(word_id, |m| {
            m.cue_break = true;
            m.cue_join = false;
        }),
        CaptionEdit::Merge { word_id } => t.set_caption_mark(word_id, |m| {
            m.cue_join = true;
            m.cue_break = false;
        }),
        CaptionEdit::Hide { word_ids, hidden } => {
            for id in word_ids {
                t.set_caption_mark(id, |m| m.hidden = *hidden)?;
            }
            Ok(())
        }
    })?;
    refresh_playback(state);
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    caption_track(reader, short_id.as_deref())
}
