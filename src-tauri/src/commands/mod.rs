use crate::dsp::{SilenceConfig, SilenceDetectionResult};
use crate::media::{EncoderGate, MediaInteropStatus, MediaParityReport};
use crate::playback::{
    self, PlaybackOwner, PlaybackStatus, PreviewHitMode, PreviewOwner, PreviewStatus,
    PreviewViewport,
};
use crate::project::{
    OpenedProject, ProjectReader, SegmentPage, WaveformPage, WaveformTrackContext,
};
use crate::sequence::edit::SequenceEdit;
use crate::sequence::{Role, StreamKind};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub mod transcript;

pub struct AppState {
    pub command_lock: Mutex<()>,
    pub opened_project: Mutex<Option<crate::project::ProjectReader>>,
    pub playback: Mutex<PlaybackOwner>,
    pub playback_shutdown: std::sync::atomic::AtomicBool,
    pub preview: Mutex<PreviewOwner>,
    /// The quality picked in the stage toolbar; `None` uses the surface's default.
    pub preview_quality: Mutex<Option<playback::PreviewQuality>>,
    /// Woken whenever a new webview preview frame is stored.
    pub preview_frame_ready: tokio::sync::Notify,
    pub encoder_gate: Arc<EncoderGate>,
    pub export: Mutex<crate::export::ExportOwner>,
    /// Bumped when the project opens or closes (or media goes), which stops waveform reads.
    pub waveform_epoch: AtomicU64,
    pub native_capture_enabled: bool,
    /// The renderer behind the Shorts Studio preview, kept while the same short and layout
    /// are being scrubbed.
    pub short_preview: Mutex<Option<ShortPreviewCache>>,
}

pub struct ShortPreviewCache {
    key: String,
    evaluator: crate::export::SceneEvaluator,
}

/// Studio preview frames: 432x768, decoded at no more than twice that.
const SHORT_PREVIEW_SIZE: (u32, u32) = (432, 768);

impl AppState {
    pub fn new() -> Self {
        Self {
            command_lock: Mutex::new(()),
            opened_project: Mutex::new(None),
            playback: Mutex::new(PlaybackOwner::closed()),
            playback_shutdown: std::sync::atomic::AtomicBool::new(false),
            preview: Mutex::new(PreviewOwner::new()),
            preview_quality: Mutex::new(None),
            preview_frame_ready: tokio::sync::Notify::new(),
            encoder_gate: Arc::new(EncoderGate::new()),
            export: Mutex::new(crate::export::ExportOwner::new()),
            waveform_epoch: AtomicU64::new(0),
            // Playback has an audio output (and audio clock) on macOS and Windows.
            native_capture_enabled: cfg!(any(target_os = "macos", windows)),
            short_preview: Mutex::new(None),
        }
    }
}

/// Returns the cross-platform default folder for AeroEdits projects:
/// `Documents/AeroEdits/` on Windows, macOS and Linux.
pub fn default_projects_dir() -> PathBuf {
    let docs_dir = dirs::document_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Documents")))
        .unwrap_or_else(std::env::temp_dir);
    docs_dir.join("AeroEdits")
}

pub fn get_default_projects_dir_impl() -> String {
    default_projects_dir().to_string_lossy().into_owned()
}

impl Default for AppState {
    fn default() -> Self {
        let _ = fs::create_dir_all(default_projects_dir());
        Self::new()
    }
}

pub fn show_in_finder_impl(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Err(format!("Path does not exist: {}", path));
    }
    #[cfg(target_os = "macos")]
    {
        let status = std::process::Command::new("open")
            .arg("-R")
            .arg(&path)
            .status()
            .map_err(|e| format!("Failed to run open -R: {e}"))?;
        if !status.success() {
            return Err(format!("open -R failed with exit status: {status}"));
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        #[cfg(target_os = "windows")]
        {
            // explorer.exe exits with status 1 even when it opens the window,
            // so only a failure to launch it is an error.
            // Explorer only understands `/select,"C:\dir\file"`: the default argument quoting
            // wraps the whole switch in quotes (any path with a space), and it opens Documents
            // instead. It also needs backslashes.
            use std::os::windows::process::CommandExt;
            let native = path.replace('/', "\\");
            let mut command = std::process::Command::new("explorer");
            if p.is_dir() {
                command.raw_arg(format!("\"{native}\""));
            } else {
                command.raw_arg(format!("/select,\"{native}\""));
            }
            command
                .spawn()
                .map_err(|e| format!("Failed to run explorer: {e}"))?;
            Ok(())
        }
        #[cfg(target_os = "linux")]
        {
            let target = if p.is_dir() {
                p
            } else {
                p.parent().unwrap_or(p)
            };
            let status = std::process::Command::new("xdg-open")
                .arg(target)
                .status()
                .map_err(|e| format!("Failed to run xdg-open: {e}"))?;
            if !status.success() {
                return Err(format!("xdg-open failed with exit status: {status}"));
            }
            Ok(())
        }
        #[cfg(all(
            not(target_os = "macos"),
            not(target_os = "windows"),
            not(target_os = "linux")
        ))]
        Ok(())
    }
}

/// What a sound stream's waveform, pauses and speech are read from: `key` is
/// `<asset>.<stream>`. On the timeline, its time maps through the clips that play it;
/// otherwise the asset's own time is the timeline (a waveform drawn per clip).
pub(crate) fn sound_context(
    reader: &ProjectReader,
    key: &str,
    on_timeline: bool,
) -> Result<WaveformTrackContext, String> {
    let document = reader.document();
    let source = document.stream_ref(key)?;
    let asset = document.asset(&source.asset).ok_or("Unknown sound")?;
    let stream = asset.stream(&source.stream).ok_or("Unknown sound")?;
    if stream.kind != StreamKind::Sound {
        return Err("That stream has no sound".into());
    }
    let segments = crate::sequence::sources::sound_segments(reader.root(), asset, &source.stream)?;
    let (retained, edited_duration_us) = if on_timeline {
        (
            crate::sequence::clock::stream_entries(&document.sequence, &asset.id, &stream.id),
            document.duration_us(),
        )
    } else {
        (
            vec![crate::project::RetainedInterval::recording(
                0,
                asset.duration_us,
            )],
            asset.duration_us,
        )
    };
    Ok(WaveformTrackContext {
        root: reader.root().to_path_buf(),
        track_id: key.to_string(),
        track_type: crate::sequence::sources::track_type(stream.role),
        segments,
        retained,
        edited_duration_us,
    })
}

/// Finds pauses in a sound stream (`<asset>.<stream>`), in timeline time through its clips.
pub fn detect_silence_impl(
    state: &AppState,
    project_handle: String,
    track_id: String,
    config: SilenceConfig,
) -> Result<SilenceDetectionResult, String> {
    config.validate()?;
    let ctx = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        require_handle(reader, &project_handle)?;
        sound_context(reader, &track_id, true)?
    };
    crate::project::silence::detect_track_silence(&ctx, &config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_title_formatting() {
        assert_eq!(
            window_title_for_project(Some("Launch Demo")),
            "AeroEdits \u{2014} Launch Demo"
        );
        assert_eq!(window_title_for_project(None), "AeroEdits");
        assert_eq!(window_title_for_project(Some("")), "AeroEdits");
        assert_eq!(window_title_for_project(Some("   ")), "AeroEdits");
    }

    #[test]
    fn preview_without_a_native_window_attaches_to_the_webview() {
        let state = AppState::new();
        let status =
            preview_attach_impl(&state, "main".into(), PreviewHitMode::Consume, None).unwrap();
        assert!(status.attached);
        assert_eq!(status.surface, "webview");
    }

    #[test]
    fn test_show_in_finder_impl_missing_path() {
        let non_existent = "/tmp/does-not-exist-aeroedits-test-finder-12345";
        let err = show_in_finder_impl(non_existent.into()).unwrap_err();
        assert!(err.contains("Path does not exist"));
    }

    // Opens a real Finder window, so it only runs on macOS.
    #[cfg(target_os = "macos")]
    #[test]
    fn test_show_in_finder_impl() {
        let dir = tempfile::tempdir().unwrap();
        let result = show_in_finder_impl(dir.path().to_string_lossy().into_owned());
        assert!(result.is_ok());
    }
}

pub fn open_project_impl(state: &AppState, path: String) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let reader = ProjectReader::open(std::path::Path::new(&path))?;
    let summary = reader.summary.clone();
    let document = reader.history().current.clone();
    let mut owner = PlaybackOwner::open(summary.project_handle.clone(), &document)?;
    *state.opened_project.lock() = Some(reader);
    owner.native_enabled = state.native_capture_enabled;
    *state.playback.lock() = owner;
    state.waveform_epoch.fetch_add(1, Ordering::SeqCst);
    Ok(summary)
}

pub fn close_project_impl(state: &AppState, project_handle: String) -> Result<(), String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    if let Some(reader) = opened.as_ref() {
        if reader.summary.project_handle != project_handle {
            return Err("Stale project handle".into());
        }
    }
    *opened = None;
    state.playback.lock().close();
    state.waveform_epoch.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

pub fn project_segments_impl(
    state: &AppState,
    project_handle: String,
    track_id: String,
    offset: usize,
    limit: usize,
) -> Result<SegmentPage, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    if reader.summary.project_handle != project_handle {
        return Err("Stale project handle".into());
    }
    reader.page(&track_id, offset, limit)
}

/// A sound stream's waveform (`<asset>.<stream>`) over `[start_us, end_us)` of its own time;
/// the timeline draws each clip's part of it.
pub fn project_waveform_impl(
    state: &AppState,
    project_handle: String,
    track_id: String,
    start_us: u64,
    end_us: u64,
    bucket_count: usize,
) -> Result<WaveformPage, String> {
    // A waveform is on its sound's own time, so edits leave it valid; only closing the
    // project stops one. Two windows asking for the same sound both get it.
    let epoch = state.waveform_epoch.load(Ordering::SeqCst);
    let ctx = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        require_handle(reader, &project_handle)?;
        sound_context(reader, &track_id, false)?
    };
    crate::project::waveform::query_waveform(&ctx, start_us, end_us, bucket_count, &|| {
        state.waveform_epoch.load(Ordering::SeqCst) != epoch
    })
}

/// Zoom suggestions from the mouse data of each recording, each on its own clock and placed
/// where its screen plays.
fn all_zoom_suggestions(
    reader: &crate::project::reader::ProjectReader,
    config: &crate::zoom::ZoomConfig,
) -> Result<crate::zoom::ZoomGeneration, String> {
    let document = reader.document();
    let mut generation = crate::zoom::ZoomGeneration {
        version: config.generation_version,
        config: config.clone(),
        suggestions: Vec::new(),
        diagnostics: Vec::new(),
    };
    for asset in reader.recordings() {
        let stream =
            match crate::telemetry::reader::read_telemetry(std::path::Path::new(&asset.path)) {
                Ok(stream) => stream,
                Err(error) => {
                    generation
                        .diagnostics
                        .push(format!("{}: {error}", asset.name));
                    continue;
                }
            };
        let mut found = crate::zoom::generate_zoom_suggestions(&stream, config)?;
        crate::zoom::attach_edited_ranges(&mut found, &document.picture_clock(&asset.id));
        generation.diagnostics.extend(found.diagnostics);
        for mut suggestion in found.suggestions {
            // Ids stay unique across recordings.
            suggestion.id = format!("{}:{}", asset.id, suggestion.id);
            suggestion.media = Some(asset.id.clone());
            generation.suggestions.push(suggestion);
        }
    }
    Ok(generation)
}

pub fn project_zoom_suggestions_impl(
    state: &AppState,
    project_handle: String,
    _config: Option<crate::zoom::ZoomConfig>,
) -> Result<crate::zoom::ZoomGeneration, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    if reader.summary.project_handle != project_handle {
        return Err("Stale project handle".into());
    }
    // The project's own settings, so every window finds the same zooms.
    let config = reader.history().current.zoom_settings.config();
    let mut generation = all_zoom_suggestions(reader, &config)?;
    let taken: std::collections::BTreeSet<_> = reader
        .summary
        .zooms
        .iter()
        .map(|z| z.id.clone())
        .chain(reader.summary.dismissed_zoom_ids.iter().cloned())
        .collect();
    generation
        .suggestions
        .retain(|suggestion| !taken.contains(&suggestion.id));
    Ok(generation)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ManualZoomInput {
    pub edited_start_us: u64,
    pub edited_end_us: u64,
    pub center_x: f64,
    pub center_y: f64,
    pub scale: f64,
}

pub(crate) fn mutate_opened(
    state: &AppState,
    project_handle: String,
    mutate: impl FnOnce(&mut crate::project::ProjectReader) -> Result<OpenedProject, String>,
) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let summary = mutate(reader)?;
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    Ok(summary)
}

/// Saves the project's auto-zoom settings (automatic zooms take the new amounts).
pub fn project_zoom_settings_set_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    settings: crate::zoom::ZoomSettings,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        reader.set_zoom_settings(expected_revision, settings)
    })
}

/// Puts the recording's zooms back, found with the project's auto-zoom settings.
pub fn project_zoom_reload_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    _config: Option<crate::zoom::ZoomConfig>,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        let config = reader.history().current.zoom_settings.config();
        let generation = all_zoom_suggestions(reader, &config)?;
        if generation.suggestions.is_empty() {
            return Err(generation
                .diagnostics
                .last()
                .cloned()
                .unwrap_or_else(|| "The recording has no mouse activity to zoom on".into()));
        }
        reader.reload_zooms(expected_revision, &generation.suggestions)
    })
}

pub fn project_zoom_accept_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    ids: Vec<String>,
    _config: Option<crate::zoom::ZoomConfig>,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        // The settings the suggestions were found with, so what is accepted is what was shown.
        let config = reader.history().current.zoom_settings.config();
        let generation = all_zoom_suggestions(reader, &config)?;
        let selected: Vec<_> = if ids.is_empty() {
            generation.suggestions
        } else {
            generation
                .suggestions
                .into_iter()
                .filter(|s| ids.iter().any(|id| id == &s.id))
                .collect()
        };
        reader.accept_zooms(expected_revision, &selected)
    })
}

pub fn project_zoom_dismiss_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    ids: Vec<String>,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        reader.dismiss_zooms(expected_revision, &ids)
    })
}

pub fn project_zoom_update_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    zoom: crate::zoom::ZoomKeyframe,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        reader.update_zoom(expected_revision, zoom)
    })
}

pub fn project_zoom_add_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    input: ManualZoomInput,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        reader.add_manual_zoom(
            expected_revision,
            input.edited_start_us,
            input.edited_end_us,
            input.center_x,
            input.center_y,
            input.scale,
        )
    })
}

pub fn project_zoom_delete_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    id: String,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        reader.delete_zoom(expected_revision, &id)
    })
}

pub fn project_layout_update_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    mut layout: crate::project::EditLayout,
    wallpaper_source: Option<String>,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        if let Some(source) = wallpaper_source {
            let relative = crate::project::layout::ingest_wallpaper(
                reader.root(),
                std::path::Path::new(&source),
            )?;
            layout.wallpaper_asset = Some(relative);
            layout.background_type = "wallpaper".into();
        }
        reader.update_layout(expected_revision, layout)
    })
}

pub fn project_audio_update_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    audio: crate::project::AudioSettings,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        reader.update_audio(expected_revision, audio)
    })
}

pub fn project_captions_update_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    captions: crate::captions::CaptionSettings,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        reader.update_captions(expected_revision, captions)
    })
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WebcamFocusDetection {
    pub project: OpenedProject,
    /// Talking-while-idle segments found by this run.
    pub detected: usize,
    pub diagnostics: Vec<String>,
}

/// Finds where the speaker talks while the screen is idle and stores those stretches as
/// auto webcam focus segments. Manual segments stay. Reading the mic happens outside the
/// project lock; the commit checks the revision so a concurrent edit is not overwritten.
pub fn project_webcam_focus_detect_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    settings: crate::webcam_focus::WebcamFocusSettings,
) -> Result<WebcamFocusDetection, String> {
    settings.validate()?;
    let (ctx, root, source_duration_us, media) = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        require_handle(reader, &project_handle)?;
        let document = reader.document();
        // The recording focus is on, else the first one.
        let asset = document
            .clock_asset(document.webcam_focus.media.as_deref())
            .and_then(|id| document.asset(id))
            .filter(|a| a.is_recording())
            .ok_or("Webcam focus needs a recording with a camera")?;
        let mic = asset.streams.iter().find(|s| s.role == Role::Mic);
        let ctx = match (settings.require_speech, mic) {
            (false, _) => None,
            (true, None) => {
                return Err("This recording has no microphone track to detect speech on".into())
            }
            (true, Some(stream)) => Some(sound_context(
                reader,
                &crate::sequence::StreamRef::new(&asset.id, &stream.id).key(),
                false,
            )?),
        };
        (
            ctx,
            std::path::PathBuf::from(&asset.path),
            asset.duration_us,
            asset.id.clone(),
        )
    };
    // Without the speech requirement the whole recording is a candidate; only the mouse
    // decides.
    let (speech, mut diagnostics) = match ctx {
        None => (vec![(0, source_duration_us)], Vec::new()),
        Some(ctx) => {
            let silence = SilenceConfig {
                threshold_db: settings.speech_threshold_db,
                min_duration_ms: settings.pause_tolerance_ms,
                padding_ms: 0,
                ..SilenceConfig::default()
            };
            let scan = crate::project::silence::scan_track_silence(&ctx, &silence)?;
            (
                crate::webcam_focus::speech_ranges(&scan.covered, &scan.source_ranges),
                scan.diagnostics,
            )
        }
    };
    let telemetry = crate::telemetry::reader::read_telemetry(&root)?;
    if telemetry.events.is_empty() {
        diagnostics.push(if settings.require_speech {
            "No mouse activity was recorded, so speech alone decides the layout".into()
        } else {
            "No mouse activity was recorded, so the whole recording counts as idle".into()
        });
    }
    let detected = crate::webcam_focus::detect_focus_ranges(&speech, &telemetry, &settings);
    let project = mutate_opened(state, project_handle, |reader| {
        let mut focus = reader.history().current.webcam_focus.clone();
        focus.enabled = true;
        focus.media = Some(media);
        focus.settings = settings;
        focus.replace_auto_segments(&detected);
        reader.update_webcam_focus(expected_revision, focus)
    })?;
    Ok(WebcamFocusDetection {
        project,
        detected: detected.len(),
        diagnostics,
    })
}

pub fn project_chapters_set_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    chapters: Vec<crate::chapters::Chapter>,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        reader.set_chapters(expected_revision, chapters)
    })
}

/// Whether some speech has a transcript to caption from.
fn has_speech_transcript(reader: &crate::project::ProjectReader) -> bool {
    let has = |id: &str| {
        crate::transcript::store::load_transcript(reader.root(), id)
            .ok()
            .flatten()
            .is_some()
    };
    let document = reader.document();
    document.captions.track_id.as_deref().is_some_and(has)
        || crate::export::caption_candidates(document)
            .iter()
            .any(|key| has(key))
}

/// One preview frame of a short, `offset_us` into it, drawn with `layout` (which may not be
/// saved yet), as a JPEG.
pub fn short_preview_frame_impl(
    state: &AppState,
    project_handle: String,
    short_id: String,
    layout: crate::shorts::ShortLayout,
    offset_us: u64,
) -> Result<Vec<u8>, String> {
    layout.validate()?;
    let (document, root, key) = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        require_handle(reader, &project_handle)?;
        let base = &reader.history().current;
        let mut short = base
            .shorts
            .iter()
            .find(|s| s.id == short_id)
            .ok_or("That short no longer exists")?
            .clone();
        short.layout = layout;
        let document = crate::shorts::short_document(base, &short, has_speech_transcript(reader))?;
        let key = format!(
            "{project_handle}:{}:{short_id}:{}",
            base.revision,
            serde_json::to_string(&short.layout).map_err(|e| e.to_string())?
        );
        (document, reader.root().to_path_buf(), key)
    };
    let at = offset_us.min(document.duration_us().saturating_sub(1));
    let mut cache = state.short_preview.lock();
    if cache.as_ref().map(|c| c.key.as_str()) != Some(key.as_str()) {
        let reuse = cache.take().and_then(|c| c.evaluator.into_reuse());
        let (width, height) = SHORT_PREVIEW_SIZE;
        let evaluator =
            crate::export::SceneEvaluator::new_reusing(root, document, width, height, reuse)?
                .with_decode_limit(crate::media::ffmpeg::DecodeLimit {
                    max_width: width * 2,
                    max_height: height * 2,
                    max_rate: 0,
                    interactive: true,
                    yuv: false,
                });
        *cache = Some(ShortPreviewCache { key, evaluator });
    }
    let frame = cache
        .as_mut()
        .ok_or("The preview renderer is not ready")?
        .evaluator
        .preview_at(at)?;
    crate::playback::preview::encode_webview_frame(&frame)
}

pub fn project_shorts_set_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    shorts: Vec<crate::shorts::Short>,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        reader.set_shorts(expected_revision, shorts)
    })
}

/// Exports one short as a 9:16 video next to the project (or to `settings.destination`),
/// named after the project and the short. Captions are turned on when a speech track has a
/// transcript.
pub fn project_short_export_impl(
    state: &AppState,
    project_handle: String,
    short_id: String,
    settings: crate::export::ExportSettings,
) -> Result<crate::export::ExportStatus, String> {
    let _guard = state.command_lock.lock();
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let base = &reader.history().current;
    let short = base
        .shorts
        .iter()
        .find(|s| s.id == short_id)
        .ok_or("That short no longer exists")?;
    let document = crate::shorts::short_document(base, short, has_speech_transcript(reader))?;
    let root = reader.root().to_path_buf();
    let stem = format!("{} - {}", reader.summary.name, short.title);
    drop(opened);
    // The first free "<project> - <short>.mp4", then " (2)", " (3)" and so on.
    let name = (1..1000)
        .map(|n| {
            if n == 1 {
                stem.clone()
            } else {
                format!("{stem} ({n})")
            }
        })
        .find(|name| {
            settings.destination.is_some()
                || !crate::export::default_destination(&root, name, 0).exists()
        })
        .ok_or("Too many exports of this short already exist")?;
    let gate = Arc::clone(&state.encoder_gate);
    let mut owner = state.export.lock();
    match crate::export::prepare_job(&root, &name, document, settings, &mut owner) {
        Ok(captured) => Ok(crate::export::spawn_job(captured, &mut owner, gate)),
        Err(status) => {
            owner.install_failed(status.clone());
            Ok(status)
        }
    }
}

pub fn project_webcam_focus_update_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    focus: crate::webcam_focus::WebcamFocus,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        reader.update_webcam_focus(expected_revision, focus)
    })
}

pub fn project_webcam_focus_add_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    edited_start_us: u64,
    edited_end_us: u64,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        reader.add_webcam_focus(expected_revision, edited_start_us, edited_end_us)
    })
}

pub fn project_webcam_focus_remove_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    edited_start_us: u64,
    edited_end_us: u64,
) -> Result<OpenedProject, String> {
    mutate_opened(state, project_handle, |reader| {
        reader.remove_webcam_focus(expected_revision, edited_start_us, edited_end_us)
    })
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EditCut {
    pub start_us: u64,
    pub end_us: u64,
}

fn require_handle(reader: &ProjectReader, project_handle: &str) -> Result<(), String> {
    if reader.summary.project_handle != project_handle {
        return Err("Stale project handle".into());
    }
    Ok(())
}

pub fn project_ripple_cuts_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    cuts: Vec<EditCut>,
) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let ranges: Vec<(u64, u64)> = cuts
        .into_iter()
        .map(|cut| (cut.start_us, cut.end_us))
        .collect();
    let summary = reader.ripple_cuts(expected_revision, &ranges)?;
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    Ok(summary)
}

/// Imports files into the project's media bin. Copying and probing run without holding
/// the project, so playback keeps going during a long import.
pub fn project_media_import_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    paths: Vec<String>,
) -> Result<OpenedProject, String> {
    if paths.is_empty() || paths.len() > 64 {
        return Err("Choose between 1 and 64 files to import".into());
    }
    let root = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        require_handle(reader, &project_handle)?;
        reader.root().to_path_buf()
    };
    let chosen: Vec<std::path::PathBuf> = paths.iter().map(std::path::PathBuf::from).collect();
    let paths = crate::media_bin::expand_import_paths(&chosen)?;
    if paths.len() > 256 {
        return Err("Choose at most 256 files to import at once".into());
    }
    let mut assets = Vec::with_capacity(paths.len());
    for path in &paths {
        match crate::media_bin::import(&root, path) {
            Ok(asset) => assets.push(asset),
            Err(error) => {
                for asset in &assets {
                    crate::media_bin::remove_files(&root, asset);
                }
                return Err(error);
            }
        }
    }
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let Some(reader) = opened
        .as_mut()
        .filter(|r| r.summary.project_handle == project_handle)
    else {
        for asset in &assets {
            crate::media_bin::remove_files(&root, asset);
        }
        return Err("The project was closed during the import".into());
    };
    reader.add_assets(expected_revision, assets)
}

pub fn project_media_remove_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    asset_id: String,
) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let summary = reader.remove_asset(expected_revision, &asset_id)?;
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    state.waveform_epoch.fetch_add(1, Ordering::SeqCst);
    Ok(summary)
}

/// What one of an asset's streams stands for.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StreamRoleInput {
    pub stream: String,
    pub role: Role,
}

pub fn project_media_roles_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    asset_id: String,
    roles: Vec<StreamRoleInput>,
) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let roles: Vec<(String, Role)> = roles.into_iter().map(|r| (r.stream, r.role)).collect();
    let summary = reader.set_stream_roles(expected_revision, &asset_id, &roles)?;
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    Ok(summary)
}

/// One timeline edit in the project's timeline, or in short `short_id`'s own timeline.
pub fn project_sequence_edit_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    edit: SequenceEdit,
    short_id: Option<String>,
) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let (summary, _) = reader.edit_sequence(expected_revision, &edit, short_id.as_deref())?;
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    Ok(summary)
}

pub fn project_undo_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    short_id: Option<String>,
) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    reader.undo(expected_revision)?;
    let summary = reader.view_or_summary(short_id.as_deref());
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    Ok(summary)
}

pub fn project_redo_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    short_id: Option<String>,
) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    reader.redo(expected_revision)?;
    let summary = reader.view_or_summary(short_id.as_deref());
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    Ok(summary)
}

pub fn project_short_view_impl(
    state: &AppState,
    project_handle: String,
    short_id: String,
) -> Result<OpenedProject, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    reader.short_view(&short_id)
}

pub fn project_short_resync_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    short_id: String,
) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let view = reader.resync_short(expected_revision, &short_id)?;
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    Ok(view)
}

/// Plays short `short_id` (its own frame, timeline and sound) instead of the video, or the
/// video again with `None`.
pub fn playback_focus_short_impl(
    state: &AppState,
    project_handle: String,
    short_id: Option<String>,
    start_us: u64,
    play: bool,
) -> Result<crate::playback::PlaybackStatus, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let mut playback = state.playback.lock();
    playback.focus_short(short_id, &reader.history().current, start_us)?;
    if play {
        playback.play()
    } else {
        playback.pause()
    }
}

pub fn project_rename_impl(
    state: &AppState,
    project_handle: String,
    new_name: String,
) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    reader.rename_project(&new_name)
}

pub fn playback_status_impl(
    state: &AppState,
    project_handle: String,
) -> Result<PlaybackStatus, String> {
    let mut playback = state.playback.lock();
    let status = playback.status()?;
    if status.state == crate::playback::PlaybackState::Closed {
        return Err("Playback is closed".into());
    }
    if status.project_handle != project_handle {
        return Err("Stale project handle".into());
    }
    Ok(status)
}

pub fn playback_play_impl(
    state: &AppState,
    project_handle: String,
) -> Result<PlaybackStatus, String> {
    let mut playback = state.playback.lock();
    if playback.status()?.project_handle != project_handle {
        return Err("Stale project handle".into());
    }
    playback.play()
}

pub fn playback_pause_impl(
    state: &AppState,
    project_handle: String,
) -> Result<PlaybackStatus, String> {
    let mut playback = state.playback.lock();
    if playback.status()?.project_handle != project_handle {
        return Err("Stale project handle".into());
    }
    playback.pause()
}

pub fn playback_seek_impl(
    state: &AppState,
    project_handle: String,
    edited_us: u64,
) -> Result<PlaybackStatus, String> {
    let mut playback = state.playback.lock();
    if playback.status()?.project_handle != project_handle {
        return Err("Stale project handle".into());
    }
    playback.seek(edited_us)
}

pub fn preview_attach_impl(
    state: &AppState,
    window_label: String,
    hit_mode: PreviewHitMode,
    native_window: Option<*mut std::ffi::c_void>,
) -> Result<PreviewStatus, String> {
    // No native window means the webview preview, where frames are fetched by the page.
    state
        .preview
        .lock()
        .attach(window_label, hit_mode, native_window)
}

pub fn preview_layout_impl(
    state: &AppState,
    viewport: PreviewViewport,
) -> Result<PreviewStatus, String> {
    if viewport.generation == 0 {
        return Err("Preview generation is required".into());
    }
    state.preview.lock().layout(viewport)
}

pub fn preview_present_fixed_impl(
    state: &AppState,
    r: f32,
    g: f32,
    b: f32,
    generation: u64,
) -> Result<PreviewStatus, String> {
    state.preview.lock().present_fixed(r, g, b, generation)
}

pub fn preview_present_fixture_impl(
    state: &AppState,
    path: String,
    generation: u64,
) -> Result<PreviewStatus, String> {
    state.preview.lock().present_fixture(&path, generation)
}

pub fn preview_status_impl(state: &AppState) -> PreviewStatus {
    state.preview.lock().status()
}

/// The preview quality in use: the one picked, or the default for the current surface.
pub fn preview_quality_impl(state: &AppState) -> playback::PreviewQuality {
    let status = state.preview.lock().status();
    let webview = if status.attached {
        status.surface == "webview"
    } else {
        !cfg!(target_os = "macos")
    };
    state
        .preview_quality
        .lock()
        .unwrap_or_else(|| playback::PreviewQuality::default_for(webview))
}

/// Takes effect on the next preview frame: the playback worker rebuilds its renderer.
pub fn preview_quality_set_impl(
    state: &AppState,
    quality: playback::PreviewQuality,
) -> Result<playback::PreviewQuality, String> {
    quality.validate()?;
    *state.preview_quality.lock() = Some(quality);
    Ok(quality)
}

pub fn preview_hit_test_impl(state: &AppState, x: f64, y: f64) -> bool {
    state.preview.lock().hit_test(x, y)
}

pub fn preview_detach_impl(
    state: &AppState,
    window_label: String,
) -> Result<PreviewStatus, String> {
    let mut preview = state.preview.lock();
    if preview.status().attached
        && preview.status().window_label.as_deref() != Some(window_label.as_str())
    {
        return Err("Stale preview window label".into());
    }
    preview.detach();
    Ok(preview.status())
}

pub fn media_interop_status_impl(state: &AppState) -> MediaInteropStatus {
    let _ = state;
    crate::media::interop_status(
        None,
        vec![
            "FFmpeg is not pinned; VideoToolbox implements the decoder/encoder contract".into(),
            "WGPU uses a CPU upload/readback fallback; Metal texture interop is untested".into(),
        ],
    )
}

pub fn media_run_parity_impl(state: &AppState) -> Result<MediaParityReport, String> {
    let dir = std::env::temp_dir().join(format!("aeroedits-f2-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let report = crate::render::run_parity(&dir, &state.encoder_gate);
    let _ = std::fs::remove_dir_all(&dir);
    report
}

pub fn export_start_impl(
    state: &AppState,
    project_handle: String,
    settings: crate::export::ExportSettings,
) -> Result<crate::export::ExportStatus, String> {
    let _guard = state.command_lock.lock();
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let document = reader.history().current.clone();
    let root = reader.root().to_path_buf();
    let name = reader.summary.name.clone();
    drop(opened);
    let gate = Arc::clone(&state.encoder_gate);
    let mut owner = state.export.lock();
    match crate::export::prepare_job(&root, &name, document, settings, &mut owner) {
        Ok(captured) => Ok(crate::export::spawn_job(captured, &mut owner, gate)),
        Err(status) => {
            owner.install_failed(status.clone());
            Ok(status)
        }
    }
}

pub fn export_status_impl(
    state: &AppState,
    job_id: Option<String>,
) -> Result<crate::export::ExportStatus, String> {
    let mut owner = state.export.lock();
    let status = owner.status();
    if let Some(id) = job_id {
        if !status.job_id.is_empty() && status.job_id != id {
            return Err("Stale export job id".into());
        }
    }
    Ok(status)
}

pub fn export_cancel_impl(
    state: &AppState,
    job_id: String,
) -> Result<crate::export::ExportStatus, String> {
    state.export.lock().cancel(&job_id)
}

/// Formats the window title for project editing.
/// When a project is open, produces "AeroEdits — <Project Name>" (e.g. "AeroEdits — Launch Demo").
/// When no project is open (None or empty), produces "AeroEdits".
pub fn window_title_for_project(project_name: Option<&str>) -> String {
    match project_name.map(str::trim).filter(|s| !s.is_empty()) {
        Some(name) => format!("AeroEdits \u{2014} {}", name),
        None => "AeroEdits".to_string(),
    }
}
