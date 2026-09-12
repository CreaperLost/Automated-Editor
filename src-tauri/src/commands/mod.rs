use crate::dsp::{SilenceConfig, SilenceDetectionResult};
use crate::media::{EncoderGate, MediaInteropStatus, MediaParityReport};
use crate::playback::{
    self, PlaybackOwner, PlaybackStatus, PreviewHitMode, PreviewOwner, PreviewStatus,
    PreviewViewport,
};
use crate::project::{
    OpenedProject, ProjectReader, ProjectRecoveryReport, RecoveryEngine,
    SegmentPage, WaveformPage, WaveformTrackContext,
};
use crate::session::SessionStateMachine;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub struct AppState {
    pub state_machine: SessionStateMachine,
    pub command_lock: Mutex<()>,
    pub project_base_dir: PathBuf,
    pub opened_project: Mutex<Option<crate::project::ProjectReader>>,
    pub playback: Mutex<PlaybackOwner>,
    pub playback_shutdown: std::sync::atomic::AtomicBool,
    pub preview: Mutex<PreviewOwner>,
    pub encoder_gate: Arc<EncoderGate>,
    pub export: Mutex<crate::export::ExportOwner>,
    pub waveform_epoch: AtomicU64,
    pub waveform_generations: Mutex<HashMap<String, u64>>,
    pub native_capture_enabled: bool,
}

impl AppState {
    pub fn new(project_base_dir: PathBuf) -> Self {
        Self {
            state_machine: SessionStateMachine::new(),
            command_lock: Mutex::new(()),
            project_base_dir,
            opened_project: Mutex::new(None),
            playback: Mutex::new(PlaybackOwner::closed()),
            playback_shutdown: std::sync::atomic::AtomicBool::new(false),
            preview: Mutex::new(PreviewOwner::new()),
            encoder_gate: Arc::new(EncoderGate::new()),
            export: Mutex::new(crate::export::ExportOwner::new()),
            waveform_epoch: AtomicU64::new(0),
            waveform_generations: Mutex::new(HashMap::new()),
            native_capture_enabled: cfg!(target_os = "macos"),
        }
    }

    pub fn new_test(project_base_dir: PathBuf) -> Self {
        Self {
            state_machine: SessionStateMachine::new(),
            command_lock: Mutex::new(()),
            project_base_dir,
            opened_project: Mutex::new(None),
            playback: Mutex::new(PlaybackOwner::closed()),
            playback_shutdown: std::sync::atomic::AtomicBool::new(false),
            preview: Mutex::new(PreviewOwner::new()),
            encoder_gate: Arc::new(EncoderGate::new()),
            export: Mutex::new(crate::export::ExportOwner::new()),
            waveform_epoch: AtomicU64::new(0),
            waveform_generations: Mutex::new(HashMap::new()),
            native_capture_enabled: false,
        }
    }
}

/// Returns the cross-platform default storage directory for AeroShoot recordings and projects:
/// `Documents/AeroShootRec/` on macOS, Windows, and Linux.
pub fn default_projects_dir() -> PathBuf {
    let docs_dir = dirs::document_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Documents")))
        .unwrap_or_else(std::env::temp_dir);
    docs_dir.join("AeroShootRec")
}

pub fn get_default_projects_dir_impl() -> String {
    default_projects_dir().to_string_lossy().into_owned()
}

fn looks_like_project_bundle(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "aero") || path.join("manifest.json").is_file()
}

/// Parent folder for a new `.aero` bundle. User-supplied paths must already exist.
pub fn resolve_project_parent(requested: Option<&str>, default: &Path) -> Result<PathBuf, String> {
    let supplied = requested.map(str::trim).filter(|value| !value.is_empty());
    let parent = match supplied {
        Some(value) => PathBuf::from(value),
        None => default.to_path_buf(),
    };
    if parent.as_os_str().as_encoded_bytes().contains(&0) {
        return Err("Project location is invalid".into());
    }
    if !parent.is_absolute() {
        return Err("Project location must be an absolute folder".into());
    }
    if parent
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("Project location cannot contain '..'".into());
    }
    if !parent.exists() {
        if supplied.is_none() {
            fs::create_dir_all(&parent)
                .map_err(|error| format!("Failed to create project folder: {error}"))?;
        } else {
            return Err("Project location does not exist".into());
        }
    }
    let meta = fs::symlink_metadata(&parent).map_err(|error| error.to_string())?;
    if meta.file_type().is_symlink() {
        return Err("Project location cannot be a symbolic link".into());
    }
    if !meta.is_dir() {
        return Err("Project location must be a folder".into());
    }
    if looks_like_project_bundle(&parent) {
        return Err("Choose a folder, not an existing .aero project".into());
    }
    Ok(parent)
}

impl Default for AppState {
    fn default() -> Self {
        let base_dir = default_projects_dir();
        let _ = fs::create_dir_all(&base_dir);
        Self::new(base_dir)
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
            let status = std::process::Command::new("explorer")
                .arg(format!("/select,{}", path))
                .status()
                .map_err(|e| format!("Failed to run explorer: {e}"))?;
            if !status.success() {
                return Err(format!("explorer failed with exit status: {status}"));
            }
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
        if reader.summary.project_handle != project_handle {
            return Err("Stale project handle".into());
        }
        let track = reader
            .summary
            .tracks
            .iter()
            .find(|track| track.descriptor.id == track_id)
            .ok_or("Unknown track")?;
        WaveformTrackContext {
            root: reader.root().to_path_buf(),
            track_id: track_id.clone(),
            track_type: track.descriptor.track_type,
            segments: reader
                .segments_for(&track_id)
                .ok_or("Unknown track")?
                .to_vec(),
            retained: reader.summary.retained_intervals.clone(),
            edited_duration_us: reader.summary.edited_duration_us,
        }
    };
    crate::project::silence::detect_track_silence(&ctx, &config)
}

pub fn recover_project_impl(project_dir: PathBuf) -> Result<ProjectRecoveryReport, String> {
    RecoveryEngine::scan_and_recover(project_dir).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn project_parent_rejects_bundle_and_parent_dir_components() {
        let dir = tempdir().unwrap();
        let bundle = ProjectBundle::create_new(dir.path(), "sess", "Inside").unwrap();
        let err = resolve_project_parent(Some(&bundle.root_path().to_string_lossy()), dir.path())
            .unwrap_err();
        assert!(err.contains(".aero"));
        assert!(resolve_project_parent(Some("/tmp/aeroshoot/../secret"), dir.path()).is_err());
        assert!(resolve_project_parent(Some("/tmp/does-not-exist-aeroshoot"), dir.path()).is_err());
    }

    #[test]
    fn test_window_title_formatting() {
        assert_eq!(
            window_title_for_project(Some("Launch Demo")),
            "AeroShoot \u{2014} Launch Demo"
        );
        assert_eq!(window_title_for_project(None), "AeroShoot");
        assert_eq!(window_title_for_project(Some("")), "AeroShoot");
        assert_eq!(window_title_for_project(Some("   ")), "AeroShoot");
    }

    #[test]
    fn test_show_in_finder_impl() {
        let non_existent = "/tmp/does-not-exist-aeroshoot-test-finder-12345";
        let err = show_in_finder_impl(non_existent.into()).unwrap_err();
        assert!(err.contains("Path does not exist"));

        let dir = tempdir().unwrap();
        let result = show_in_finder_impl(dir.path().to_string_lossy().into_owned());
        assert!(result.is_ok());
    }
}

pub fn open_project_impl(state: &AppState, path: String) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let reader = ProjectReader::open(std::path::Path::new(&path))?;
    let summary = reader.summary.clone();
    let tracks = playback::tracks_from_reader(&reader);
    let document = reader.history().current.clone();
    let mut owner = PlaybackOwner::open(
        summary.project_handle.clone(),
        reader.root().to_path_buf(),
        &document,
        tracks,
    )?;
    *state.opened_project.lock() = Some(reader);
    owner.native_enabled = state.native_capture_enabled;
    *state.playback.lock() = owner;
    state.waveform_epoch.fetch_add(1, Ordering::SeqCst);
    state.waveform_generations.lock().clear();
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
    state.waveform_generations.lock().clear();
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

pub fn project_waveform_impl(
    state: &AppState,
    project_handle: String,
    track_id: String,
    start_us: u64,
    end_us: u64,
    bucket_count: usize,
) -> Result<WaveformPage, String> {
    let epoch = state.waveform_epoch.load(Ordering::SeqCst);
    let generation = {
        let mut generations = state.waveform_generations.lock();
        let slot = generations.entry(track_id.clone()).or_insert(0);
        *slot += 1;
        *slot
    };
    let ctx = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        if reader.summary.project_handle != project_handle {
            return Err("Stale project handle".into());
        }
        let track = reader
            .summary
            .tracks
            .iter()
            .find(|track| track.descriptor.id == track_id)
            .ok_or("Unknown track")?;
        WaveformTrackContext {
            root: reader.root().to_path_buf(),
            track_id: track_id.clone(),
            track_type: track.descriptor.track_type,
            segments: reader
                .segments_for(&track_id)
                .ok_or("Unknown track")?
                .to_vec(),
            retained: reader.summary.retained_intervals.clone(),
            edited_duration_us: reader.summary.edited_duration_us,
        }
    };
    crate::project::waveform::query_waveform(&ctx, start_us, end_us, bucket_count, &|| {
        if state.waveform_epoch.load(Ordering::SeqCst) != epoch {
            return true;
        }
        state
            .waveform_generations
            .lock()
            .get(&track_id)
            .copied()
            .unwrap_or(0)
            != generation
    })
}

pub fn project_zoom_suggestions_impl(
    state: &AppState,
    project_handle: String,
    config: Option<crate::zoom::ZoomConfig>,
) -> Result<crate::zoom::ZoomGeneration, String> {
    let config = config.unwrap_or_default();
    config.validate()?;
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    if reader.summary.project_handle != project_handle {
        return Err("Stale project handle".into());
    }
    let stream = crate::telemetry::reader::read_telemetry(reader.root())?;
    let mut generation = crate::zoom::generate_zoom_suggestions(&stream, &config)?;
    let mapper = reader.history().current.mapper()?;
    crate::zoom::attach_edited_ranges(&mut generation, &mapper);
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

fn mutate_opened(
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

pub fn project_zoom_accept_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    ids: Vec<String>,
) -> Result<OpenedProject, String> {
    let config = crate::zoom::ZoomConfig::default();
    mutate_opened(state, project_handle, |reader| {
        let stream = crate::telemetry::reader::read_telemetry(reader.root())?;
        let generation = crate::zoom::generate_zoom_suggestions(&stream, &config)?;
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
    state.waveform_epoch.fetch_add(1, Ordering::SeqCst);
    Ok(summary)
}

pub fn project_undo_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let summary = reader.undo(expected_revision)?;
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    state.waveform_epoch.fetch_add(1, Ordering::SeqCst);
    Ok(summary)
}

pub fn project_redo_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let summary = reader.redo(expected_revision)?;
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    state.waveform_epoch.fetch_add(1, Ordering::SeqCst);
    Ok(summary)
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
    if native_window.is_none() {
        return Err("Native preview requires a desktop window".into());
    }
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
    let dir = std::env::temp_dir().join(format!("aeroshoot-f2-{}", uuid::Uuid::new_v4()));
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
    let session_state = state.state_machine.current();
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let document = reader.history().current.clone();
    let tracks = playback::tracks_from_reader(reader);
    let root = reader.root().to_path_buf();
    let name = reader.summary.manifest.project_name.clone();
    drop(opened);
    let gate = Arc::clone(&state.encoder_gate);
    let mut owner = state.export.lock();
    match crate::export::prepare_job(
        session_state,
        &root,
        &name,
        document,
        tracks,
        settings,
        &mut owner,
    ) {
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
/// When a project is open, produces "AeroShoot — <Project Name>" (e.g. "AeroShoot — Launch Demo").
/// When no project is open (None or empty), produces "AeroShoot".
pub fn window_title_for_project(project_name: Option<&str>) -> String {
    match project_name.map(str::trim).filter(|s| !s.is_empty()) {
        Some(name) => format!("AeroShoot \u{2014} {}", name),
        None => "AeroShoot".to_string(),
    }
}
