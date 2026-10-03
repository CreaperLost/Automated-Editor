//! Transcription commands. Transcripts are per audio track and stored in the project folder;
//! word deletions and suggestions become ordinary ripple cuts on the edit document.
use super::AppState;
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

fn audio_track(
    state: &AppState,
    project_handle: &str,
    track_id: &str,
) -> Result<TrackContext, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    super::require_handle(reader, project_handle)?;
    // Imported sound: one stream of a file, over the file's own time.
    if let Some((stream, asset_id)) = crate::project::revision::media_sound(track_id) {
        let asset = reader
            .history()
            .current
            .media_assets
            .iter()
            .find(|asset| asset.id == asset_id)
            .ok_or("Unknown imported media")?;
        let path = asset
            .audio_paths()
            .nth(stream)
            .ok_or("That media has no such audio stream")?;
        return Ok(TrackContext {
            root: reader.root().to_path_buf(),
            segments: vec![SegmentSummary {
                track_id: track_id.to_string(),
                relative_path: path.clone(),
                start_us: 0,
                end_us: asset.duration_us,
                size_bytes: crate::project::file_len(reader.root(), path),
                media_timescale: crate::media::audio::SAMPLE_RATE,
                media_start_value: 0,
                host_anchor_us: 0,
                is_keyframe_start: None,
                available: true,
            }],
        });
    }
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
        &document.mapper_for_transcript(track_id)?,
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
        &reader.history().current.mapper_for_transcript(&track_id)?,
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
        edit::word_cuts(
            &transcript,
            &word_ids,
            &document.mapper_for_transcript(&track_id)?,
        )?
    };
    // Nothing is selected in the transcript sense: every track loses the same time, so
    // pictures and sound elsewhere stay in step with the words.
    let ranges = cuts
        .into_iter()
        .map(|(start_us, end_us)| crate::tracks::EditedRange { start_us, end_us })
        .collect();
    super::project_tracks_edit_impl(
        state,
        project_handle,
        expected_revision,
        crate::tracks::TrackEdit::RippleDelete {
            ranges,
            all_tracks: true,
        },
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
    // Missing settings or key fail here, before the job slot or the project is touched.
    let mut client =
        crate::ai::client_from_settings(&config_dir())?.with_cancel(transcripts.cancel.clone());
    let _run = RunGuard::start(transcripts, format!("AI review of {track_id}"))?;
    let (created_at, words) = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        super::require_handle(reader, &project_handle)?;
        let transcript = store::load_transcript(reader.root(), &track_id)?
            .ok_or("Transcribe this track first")?;
        let mapper = reader.history().current.mapper_for_transcript(&track_id)?;
        let words: Vec<crate::transcript::TranscriptWord> = transcript
            .words
            .iter()
            .filter(|w| w.kind == crate::transcript::WordKind::Word && edit::kept(w, &mapper))
            .cloned()
            .collect();
        (transcript.created_at, words)
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
            &document.mapper_for_transcript(&track_id)?,
            document.revision,
        )
        .words
    };
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
            &document.mapper_for_transcript(&track_id)?,
            document.revision,
        )
        .words
    };
    let mut shorts = crate::ai::shorts::suggest(&mut client, &words)?;
    // Picked from imported speech: the times are that file's own.
    if let Some(asset) = crate::project::revision::media_sound_asset(&track_id) {
        for short in &mut shorts {
            short.media = Some(asset.to_string());
        }
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
    let recorded: Vec<String> = [TrackType::MicAudio, TrackType::SystemAudio]
        .iter()
        .flat_map(|kind| {
            reader
                .summary
                .tracks
                .iter()
                .filter(move |t| t.descriptor.track_type == *kind)
                .map(|t| t.descriptor.id.clone())
        })
        .collect();
    let document_for_roles = document;
    let imported: Vec<String> = document
        .media_assets
        .iter()
        .flat_map(|asset| {
            (0..asset.audio_paths().count())
                .filter(|&stream| {
                    document_for_roles.main_stream_role(asset, stream)
                        == crate::media_bin::SoundRole::Mic
                })
                .map(|stream| crate::project::revision::media_sound_id(stream, &asset.id))
                .collect::<Vec<_>>()
        })
        .collect();
    let root = reader.root();
    let Some(transcript) = crate::captions::caption_source(settings, recorded, imported, |id| {
        store::load_transcript(root, id).ok().flatten()
    }) else {
        return Ok(CaptionTrackView::default());
    };
    let mapper = document.mapper_for_transcript(&transcript.track_id)?;
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
                caption_timeline(reader, short_id.as_deref())?.mapper_for_transcript(&track_id)?;
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
