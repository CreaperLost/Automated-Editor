pub mod ai;
pub mod captions;
pub mod chapters;
pub mod commands;
pub mod cursor;
pub mod dsp;
pub mod export;
pub mod fixtures;
pub mod media;
pub mod media_bin;
mod parity;
pub mod playback;
pub mod project;
pub mod render;
pub mod secrets;
pub mod sequence;
pub mod shorts;
pub mod telemetry;
pub mod timeline;
pub mod transcript;
pub mod webcam_focus;
pub mod zoom;

#[cfg(feature = "tauri-app")]
use commands::*;
#[cfg(feature = "tauri-app")]
use tauri::{Manager, State};

/// The NSWindow for the native preview view, or `None` where preview frames go to the
/// webview instead.
#[cfg(feature = "tauri-app")]
fn preview_ns_window(
    app: &tauri::AppHandle,
    window_label: &str,
) -> Result<Option<*mut std::ffi::c_void>, String> {
    let window = app
        .get_webview_window(window_label)
        .ok_or_else(|| format!("Unknown window label: {window_label}"))?;
    if media::media_backend() != media::MediaBackend::Native {
        return Ok(None);
    }
    #[cfg(target_os = "macos")]
    {
        window
            .ns_window()
            .map(Some)
            .map_err(|e| format!("Failed to get NSWindow: {e}"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = window;
        Ok(None)
    }
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn open_project(
    app: tauri::AppHandle,
    path: String,
) -> Result<project::OpenedProject, String> {
    let app_handle = app.clone();
    let opened = tauri::async_runtime::spawn_blocking(move || {
        commands::open_project_impl(&app_handle.state::<AppState>(), path)
    })
    .await
    .map_err(|e| e.to_string())??;

    if let Some(window) = app.get_webview_window("main") {
        let title = commands::window_title_for_project(Some(&opened.name));
        let _ = window.set_title(&title);
    }

    Ok(opened)
}

/// Makes a project folder (in `location`, or the default projects folder) and opens it. With
/// `recording`, the project edits that recording folder without writing to it; without, it
/// starts empty.
#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn project_create(
    app: tauri::AppHandle,
    name: String,
    location: Option<String>,
    recording: Option<String>,
) -> Result<project::OpenedProject, String> {
    let folder = tauri::async_runtime::spawn_blocking(move || {
        let parent = location
            .filter(|path| !path.trim().is_empty())
            .map(std::path::PathBuf::from)
            .unwrap_or_else(commands::default_projects_dir);
        project::folder::create_project_folder(
            &parent,
            &name,
            recording.as_deref().map(std::path::Path::new),
        )
    })
    .await
    .map_err(|e| e.to_string())??;
    open_project(app, folder.to_string_lossy().into_owned()).await
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn pick_recording_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    pick_directory_dialog(app, "Choose a Recording", commands::default_projects_dir()).await
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn pick_project_location(app: tauri::AppHandle) -> Result<Option<String>, String> {
    pick_directory_dialog(
        app,
        "Choose Where to Save the Project",
        commands::default_projects_dir(),
    )
    .await
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn close_project(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    project_handle: String,
) -> Result<(), String> {
    commands::close_project_impl(&state, project_handle)?;
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_title(&commands::window_title_for_project(None));
    }
    Ok(())
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_segments(
    state: State<'_, AppState>,
    project_handle: String,
    track_id: String,
    offset: usize,
    limit: usize,
) -> Result<project::SegmentPage, String> {
    commands::project_segments_impl(&state, project_handle, track_id, offset, limit)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn project_waveform(
    app: tauri::AppHandle,
    project_handle: String,
    track_id: String,
    start_us: u64,
    end_us: u64,
    bucket_count: usize,
) -> Result<project::WaveformPage, String> {
    tauri::async_runtime::spawn_blocking(move || {
        commands::project_waveform_impl(
            &app.state::<AppState>(),
            project_handle,
            track_id,
            start_us,
            end_us,
            bucket_count,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn project_zoom_suggestions(
    app: tauri::AppHandle,
    project_handle: String,
    config: Option<zoom::ZoomConfig>,
) -> Result<zoom::ZoomGeneration, String> {
    tauri::async_runtime::spawn_blocking(move || {
        commands::project_zoom_suggestions_impl(&app.state::<AppState>(), project_handle, config)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_zoom_accept(
    state: State<'_, AppState>,
    project_handle: String,
    expected_revision: u64,
    ids: Vec<String>,
    config: Option<zoom::ZoomConfig>,
) -> Result<project::OpenedProject, String> {
    commands::project_zoom_accept_impl(&state, project_handle, expected_revision, ids, config)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_zoom_settings_set(
    state: State<'_, AppState>,
    project_handle: String,
    expected_revision: u64,
    settings: zoom::ZoomSettings,
) -> Result<project::OpenedProject, String> {
    commands::project_zoom_settings_set_impl(&state, project_handle, expected_revision, settings)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_zoom_reload(
    state: State<'_, AppState>,
    project_handle: String,
    expected_revision: u64,
    config: Option<zoom::ZoomConfig>,
) -> Result<project::OpenedProject, String> {
    commands::project_zoom_reload_impl(&state, project_handle, expected_revision, config)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_zoom_dismiss(
    state: State<'_, AppState>,
    project_handle: String,
    expected_revision: u64,
    ids: Vec<String>,
) -> Result<project::OpenedProject, String> {
    commands::project_zoom_dismiss_impl(&state, project_handle, expected_revision, ids)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_zoom_update(
    state: State<'_, AppState>,
    project_handle: String,
    expected_revision: u64,
    zoom: zoom::ZoomKeyframe,
) -> Result<project::OpenedProject, String> {
    commands::project_zoom_update_impl(&state, project_handle, expected_revision, zoom)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_zoom_add(
    state: State<'_, AppState>,
    project_handle: String,
    expected_revision: u64,
    input: commands::ManualZoomInput,
) -> Result<project::OpenedProject, String> {
    commands::project_zoom_add_impl(&state, project_handle, expected_revision, input)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_zoom_delete(
    state: State<'_, AppState>,
    project_handle: String,
    expected_revision: u64,
    id: String,
) -> Result<project::OpenedProject, String> {
    commands::project_zoom_delete_impl(&state, project_handle, expected_revision, id)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn project_webcam_focus_detect(
    app: tauri::AppHandle,
    project_handle: String,
    expected_revision: u64,
    settings: webcam_focus::WebcamFocusSettings,
) -> Result<commands::WebcamFocusDetection, String> {
    tauri::async_runtime::spawn_blocking(move || {
        commands::project_webcam_focus_detect_impl(
            &app.state::<AppState>(),
            project_handle,
            expected_revision,
            settings,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_webcam_focus_update(
    state: State<'_, AppState>,
    project_handle: String,
    expected_revision: u64,
    focus: webcam_focus::WebcamFocus,
) -> Result<project::OpenedProject, String> {
    commands::project_webcam_focus_update_impl(&state, project_handle, expected_revision, focus)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_webcam_focus_add(
    state: State<'_, AppState>,
    project_handle: String,
    expected_revision: u64,
    edited_start_us: u64,
    edited_end_us: u64,
) -> Result<project::OpenedProject, String> {
    commands::project_webcam_focus_add_impl(
        &state,
        project_handle,
        expected_revision,
        edited_start_us,
        edited_end_us,
    )
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_webcam_focus_remove(
    state: State<'_, AppState>,
    project_handle: String,
    expected_revision: u64,
    edited_start_us: u64,
    edited_end_us: u64,
) -> Result<project::OpenedProject, String> {
    commands::project_webcam_focus_remove_impl(
        &state,
        project_handle,
        expected_revision,
        edited_start_us,
        edited_end_us,
    )
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_layout_update(
    state: State<'_, AppState>,
    project_handle: String,
    expected_revision: u64,
    layout: project::EditLayout,
    wallpaper_source: Option<String>,
) -> Result<project::OpenedProject, String> {
    commands::project_layout_update_impl(
        &state,
        project_handle,
        expected_revision,
        layout,
        wallpaper_source,
    )
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_audio_update(
    state: State<'_, AppState>,
    project_handle: String,
    expected_revision: u64,
    audio: project::AudioSettings,
) -> Result<project::OpenedProject, String> {
    commands::project_audio_update_impl(&state, project_handle, expected_revision, audio)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_captions_update(
    state: State<'_, AppState>,
    project_handle: String,
    expected_revision: u64,
    captions: captions::CaptionSettings,
) -> Result<project::OpenedProject, String> {
    commands::project_captions_update_impl(&state, project_handle, expected_revision, captions)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_ripple_cuts(
    state: State<'_, AppState>,
    project_handle: String,
    expected_revision: u64,
    cuts: Vec<commands::EditCut>,
) -> Result<project::OpenedProject, String> {
    commands::project_ripple_cuts_impl(&state, project_handle, expected_revision, cuts)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn project_media_import(
    app: tauri::AppHandle,
    project_handle: String,
    expected_revision: u64,
    paths: Vec<String>,
) -> Result<project::OpenedProject, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        commands::project_media_import_impl(&state, project_handle, expected_revision, paths)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_media_remove(
    state: State<'_, AppState>,
    project_handle: String,
    expected_revision: u64,
    asset_id: String,
) -> Result<project::OpenedProject, String> {
    commands::project_media_remove_impl(&state, project_handle, expected_revision, asset_id)
}

/// Sets what an asset's streams stand for (screen or camera; speech or background).
#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_media_roles(
    state: State<'_, AppState>,
    project_handle: String,
    expected_revision: u64,
    asset_id: String,
    roles: Vec<commands::StreamRoleInput>,
) -> Result<project::OpenedProject, String> {
    commands::project_media_roles_impl(&state, project_handle, expected_revision, asset_id, roles)
}

/// One timeline edit: tracks, clips, cuts, links. In a short's own timeline with `short_id`.
#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_sequence_edit(
    state: State<'_, AppState>,
    project_handle: String,
    expected_revision: u64,
    edit: sequence::edit::SequenceEdit,
    short_id: Option<String>,
) -> Result<project::OpenedProject, String> {
    commands::project_sequence_edit_impl(&state, project_handle, expected_revision, edit, short_id)
}

/// Short `short_id`'s own timeline, as a project the timeline can show.
#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_short_view(
    state: State<'_, AppState>,
    project_handle: String,
    short_id: String,
) -> Result<project::OpenedProject, String> {
    commands::project_short_view_impl(&state, project_handle, short_id)
}

/// Drops a short's own edit, so it follows the video again.
#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_short_resync(
    state: State<'_, AppState>,
    project_handle: String,
    expected_revision: u64,
    short_id: String,
) -> Result<project::OpenedProject, String> {
    commands::project_short_resync_impl(&state, project_handle, expected_revision, short_id)
}

/// Plays a short instead of the video (or the video again).
#[cfg(feature = "tauri-app")]
#[tauri::command]
fn playback_focus_short(
    state: State<'_, AppState>,
    project_handle: String,
    short_id: Option<String>,
    start_us: Option<u64>,
    play: Option<bool>,
) -> Result<playback::PlaybackStatus, String> {
    commands::playback_focus_short_impl(
        &state,
        project_handle,
        short_id,
        start_us.unwrap_or(0),
        play.unwrap_or(false),
    )
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn pick_media_files(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        app.run_on_main_thread(move || {
            let picked = rfd::FileDialog::new()
                .set_title("Import media")
                .add_filter("Video, images and audio", &media_bin::import_extensions())
                .pick_files()
                .unwrap_or_default()
                .into_iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            let _ = tx.send(picked);
        })
        .map_err(|error| error.to_string())?;
        rx.recv().map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

/// A folder whose videos, images and audio are imported together.
#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn pick_media_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        app.run_on_main_thread(move || {
            let picked = rfd::FileDialog::new()
                .set_title("Import a folder of media")
                .pick_folder()
                .map(|path| path.to_string_lossy().into_owned());
            let _ = tx.send(picked);
        })
        .map_err(|error| error.to_string())?;
        rx.recv().map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_undo(
    state: State<'_, AppState>,
    project_handle: String,
    expected_revision: u64,
    short_id: Option<String>,
) -> Result<project::OpenedProject, String> {
    commands::project_undo_impl(&state, project_handle, expected_revision, short_id)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_redo(
    state: State<'_, AppState>,
    project_handle: String,
    expected_revision: u64,
    short_id: Option<String>,
) -> Result<project::OpenedProject, String> {
    commands::project_redo_impl(&state, project_handle, expected_revision, short_id)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_rename(
    state: State<'_, AppState>,
    project_handle: String,
    new_name: String,
) -> Result<project::OpenedProject, String> {
    commands::project_rename_impl(&state, project_handle, new_name)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn playback_status(
    state: State<'_, AppState>,
    project_handle: String,
) -> Result<playback::PlaybackStatus, String> {
    commands::playback_status_impl(&state, project_handle)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn playback_play(
    state: State<'_, AppState>,
    project_handle: String,
) -> Result<playback::PlaybackStatus, String> {
    commands::playback_play_impl(&state, project_handle)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn playback_pause(
    state: State<'_, AppState>,
    project_handle: String,
) -> Result<playback::PlaybackStatus, String> {
    commands::playback_pause_impl(&state, project_handle)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn playback_seek(
    state: State<'_, AppState>,
    project_handle: String,
    edited_us: u64,
) -> Result<playback::PlaybackStatus, String> {
    commands::playback_seek_impl(&state, project_handle, edited_us)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn preview_attach(
    app: tauri::AppHandle,
    window_label: String,
    hit_mode: playback::PreviewHitMode,
) -> Result<playback::PreviewStatus, String> {
    let ns_window = preview_ns_window(&app, &window_label)?;
    commands::preview_attach_impl(&app.state::<AppState>(), window_label, hit_mode, ns_window)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn preview_layout(
    state: State<'_, AppState>,
    viewport: playback::PreviewViewport,
) -> Result<playback::PreviewStatus, String> {
    commands::preview_layout_impl(&state, viewport)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn preview_present_fixed(
    state: State<'_, AppState>,
    r: f32,
    g: f32,
    b: f32,
    generation: Option<u64>,
) -> Result<playback::PreviewStatus, String> {
    commands::preview_present_fixed_impl(&state, r, g, b, generation.unwrap_or(0))
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn preview_present_fixture(
    state: State<'_, AppState>,
    path: String,
    generation: Option<u64>,
) -> Result<playback::PreviewStatus, String> {
    commands::preview_present_fixture_impl(&state, path, generation.unwrap_or(0))
}

/// The latest webview preview frame newer than `after`: an 8-byte little-endian sequence
/// number followed by JPEG bytes, or an empty body when nothing newer arrives within
/// `wait_ms` (at most a second). Waiting here hands each frame over the moment it is stored,
/// instead of on the webview's next poll.
#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn preview_frame(
    state: State<'_, AppState>,
    after: u64,
    wait_ms: Option<u64>,
) -> Result<tauri::ipc::Response, String> {
    let deadline = tokio::time::Instant::now()
        + std::time::Duration::from_millis(wait_ms.unwrap_or(0).min(1_000));
    loop {
        let ready = state.preview_frame_ready.notified();
        tokio::pin!(ready);
        // Registered before the check, so a frame stored in between still wakes this wait.
        ready.as_mut().enable();
        if let Some(frame) = state.preview.lock().web_frame_after(after) {
            return Ok(tauri::ipc::Response::new(frame.as_ref().clone()));
        }
        if tokio::time::timeout_at(deadline, ready).await.is_err() {
            return Ok(tauri::ipc::Response::new(Vec::new()));
        }
    }
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn preview_quality(state: State<'_, AppState>) -> playback::PreviewQuality {
    commands::preview_quality_impl(&state)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn preview_quality_set(
    state: State<'_, AppState>,
    quality: playback::PreviewQuality,
) -> Result<playback::PreviewQuality, String> {
    commands::preview_quality_set_impl(&state, quality)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn preview_status(state: State<'_, AppState>) -> playback::PreviewStatus {
    commands::preview_status_impl(&state)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn preview_hit_test(state: State<'_, AppState>, x: f64, y: f64) -> bool {
    commands::preview_hit_test_impl(&state, x, y)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn preview_detach(
    state: State<'_, AppState>,
    window_label: String,
    generation: Option<u64>,
) -> Result<playback::PreviewStatus, String> {
    if generation.is_some_and(|g| g != state.preview.lock().status().generation) {
        return Err("Stale preview generation".into());
    }
    commands::preview_detach_impl(&state, window_label)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn detect_silence(
    app: tauri::AppHandle,
    project_handle: String,
    track_id: String,
    config: dsp::SilenceConfig,
) -> Result<dsp::SilenceDetectionResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        commands::detect_silence_impl(&app.state::<AppState>(), project_handle, track_id, config)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn transcript_settings_get() -> transcript::TranscriptSettingsView {
    commands::transcript::transcript_settings_get_impl()
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn transcript_settings_set(
    settings: transcript::TranscriptSettings,
) -> Result<transcript::TranscriptSettingsView, String> {
    commands::transcript::transcript_settings_set_impl(settings)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn transcript_set_api_key(key: String) -> Result<transcript::TranscriptSettingsView, String> {
    commands::transcript::transcript_set_api_key_impl(key)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_chapters_set(
    state: State<'_, AppState>,
    project_handle: String,
    expected_revision: u64,
    chapters: Vec<chapters::Chapter>,
) -> Result<project::OpenedProject, String> {
    commands::project_chapters_set_impl(&state, project_handle, expected_revision, chapters)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn project_chapters_generate(
    app: tauri::AppHandle,
    project_handle: String,
    track_id: String,
) -> Result<project::OpenedProject, String> {
    tauri::async_runtime::spawn_blocking(move || {
        commands::transcript::project_chapters_generate_impl(
            &app.state::<AppState>(),
            &app.state::<commands::transcript::TranscriptState>(),
            project_handle,
            track_id,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_shorts_set(
    state: State<'_, AppState>,
    project_handle: String,
    expected_revision: u64,
    shorts: Vec<shorts::Short>,
) -> Result<project::OpenedProject, String> {
    commands::project_shorts_set_impl(&state, project_handle, expected_revision, shorts)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn project_shorts_generate(
    app: tauri::AppHandle,
    project_handle: String,
    track_id: String,
) -> Result<project::OpenedProject, String> {
    tauri::async_runtime::spawn_blocking(move || {
        commands::transcript::project_shorts_generate_impl(
            &app.state::<AppState>(),
            &app.state::<commands::transcript::TranscriptState>(),
            project_handle,
            track_id,
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_short_export(
    state: State<'_, AppState>,
    project_handle: String,
    short_id: String,
    settings: export::ExportSettings,
) -> Result<export::ExportStatus, String> {
    commands::project_short_export_impl(&state, project_handle, short_id, settings)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn short_preview_frame(
    app: tauri::AppHandle,
    project_handle: String,
    short_id: String,
    layout: shorts::ShortLayout,
    offset_us: u64,
) -> Result<tauri::ipc::Response, String> {
    tauri::async_runtime::spawn_blocking(move || {
        commands::short_preview_frame_impl(
            &app.state::<AppState>(),
            project_handle,
            short_id,
            layout,
            offset_us,
        )
        .map(tauri::ipc::Response::new)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// The project open in the editor, for windows that open after it (the Shorts Studio).
#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_current(state: State<'_, AppState>) -> Option<project::OpenedProject> {
    state
        .opened_project
        .lock()
        .as_ref()
        .map(|reader| reader.summary.clone())
}

/// Opens the Shorts Studio window, or brings it to the front if it is already open.
///
/// Async on purpose: Tauri runs synchronous commands on the main thread, and creating a
/// window there deadlocks WebView2 on Windows (the window stays white).
#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn open_shorts_window(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    if let Some(window) = app.get_webview_window(SHORTS_WINDOW) {
        let _ = window.unminimize();
        return window.set_focus().map_err(|e| e.to_string());
    }
    tauri::WebviewWindowBuilder::new(
        &app,
        SHORTS_WINDOW,
        tauri::WebviewUrl::App("index.html".into()),
    )
    // Tells the page which app to render before any script runs; a URL query is not
    // carried reliably by every platform's asset protocol.
    .initialization_script("window.__AEROEDITS_WINDOW__ = 'shorts';")
    .title("AeroEdits Shorts Studio")
    .inner_size(1280.0, 860.0)
    .min_inner_size(960.0, 640.0)
    // HTML5 drag and drop needs the native file-drop handler off, as in the main window.
    .disable_drag_drop_handler()
    .build()
    .map(|_| ())
    .map_err(|e| e.to_string())
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn ai_settings_get() -> ai::AiSettingsView {
    commands::transcript::ai_settings_get_impl()
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn ai_settings_set(settings: ai::AiSettings) -> Result<ai::AiSettingsView, String> {
    commands::transcript::ai_settings_set_impl(settings)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn ai_set_api_key(provider: ai::AiProvider, key: String) -> Result<ai::AiSettingsView, String> {
    commands::transcript::ai_set_api_key_impl(provider, key)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn transcript_ai_suggest(
    app: tauri::AppHandle,
    project_handle: String,
    track_id: String,
) -> Result<Vec<transcript::TranscriptCutSuggestion>, String> {
    use tauri::Emitter;
    tauri::async_runtime::spawn_blocking(move || {
        let emitter = app.clone();
        commands::transcript::transcript_ai_suggest_impl(
            &app.state::<AppState>(),
            &app.state::<commands::transcript::TranscriptState>(),
            project_handle,
            track_id,
            &mut |progress| {
                let _ = emitter.emit("transcript-progress", progress);
            },
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn transcript_get(
    state: State<'_, AppState>,
    project_handle: String,
    track_id: String,
) -> Result<Option<transcript::TranscriptView>, String> {
    commands::transcript::transcript_get_impl(&state, project_handle, track_id)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn transcript_run(
    app: tauri::AppHandle,
    project_handle: String,
    track_id: String,
) -> Result<commands::transcript::TranscriptRunResult, String> {
    use tauri::Emitter;
    tauri::async_runtime::spawn_blocking(move || {
        let emitter = app.clone();
        commands::transcript::transcript_run_impl(
            &app.state::<AppState>(),
            &app.state::<commands::transcript::TranscriptState>(),
            project_handle,
            track_id,
            &mut |progress| {
                let _ = emitter.emit("transcript-progress", progress);
            },
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn transcript_cancel(state: State<'_, commands::transcript::TranscriptState>) {
    commands::transcript::transcript_cancel_impl(&state)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn transcript_delete(
    state: State<'_, AppState>,
    project_handle: String,
    track_id: String,
) -> Result<(), String> {
    commands::transcript::transcript_delete_impl(&state, project_handle, track_id)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn transcript_suggestions(
    state: State<'_, AppState>,
    project_handle: String,
    track_id: String,
) -> Result<Vec<transcript::TranscriptCutSuggestion>, String> {
    commands::transcript::transcript_suggestions_impl(&state, project_handle, track_id)
}

/// The caption track: the captioned transcript's cues in edited time.
#[cfg(feature = "tauri-app")]
#[tauri::command]
fn project_caption_cues(
    state: State<'_, AppState>,
    project_handle: String,
    short_id: Option<String>,
) -> Result<commands::transcript::CaptionTrackView, String> {
    commands::transcript::project_caption_cues_impl(&state, project_handle, short_id)
}

/// Edits a caption from the timeline (text, timing, split, merge, hide), saved in the transcript.
#[cfg(feature = "tauri-app")]
#[tauri::command]
fn transcript_caption_edit(
    state: State<'_, AppState>,
    project_handle: String,
    track_id: String,
    edit: commands::transcript::CaptionEdit,
    short_id: Option<String>,
) -> Result<commands::transcript::CaptionTrackView, String> {
    commands::transcript::transcript_caption_edit_impl(
        &state,
        project_handle,
        track_id,
        edit,
        short_id,
    )
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn transcript_strip_punctuation(
    state: State<'_, AppState>,
    project_handle: String,
    track_id: String,
) -> Result<transcript::TranscriptView, String> {
    commands::transcript::transcript_strip_punctuation_impl(&state, project_handle, track_id)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn transcript_set_word_text(
    state: State<'_, AppState>,
    project_handle: String,
    track_id: String,
    word_id: String,
    text: String,
) -> Result<transcript::TranscriptView, String> {
    commands::transcript::transcript_set_word_text_impl(
        &state,
        project_handle,
        track_id,
        word_id,
        text,
    )
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn transcript_dismiss_suggestions(
    state: State<'_, AppState>,
    project_handle: String,
    track_id: String,
    ids: Vec<String>,
    dismissed: bool,
) -> Result<Vec<transcript::TranscriptCutSuggestion>, String> {
    commands::transcript::transcript_dismiss_suggestions_impl(
        &state,
        project_handle,
        track_id,
        ids,
        dismissed,
    )
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn transcript_cut_words(
    state: State<'_, AppState>,
    project_handle: String,
    expected_revision: u64,
    track_id: String,
    word_ids: Vec<String>,
) -> Result<project::OpenedProject, String> {
    commands::transcript::transcript_cut_words_impl(
        &state,
        project_handle,
        expected_revision,
        track_id,
        word_ids,
    )
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn transcript_download_model(
    app: tauri::AppHandle,
) -> Result<transcript::TranscriptSettingsView, String> {
    use tauri::Emitter;
    tauri::async_runtime::spawn_blocking(move || {
        let emitter = app.clone();
        commands::transcript::transcript_download_model_impl(
            &app.state::<commands::transcript::TranscriptState>(),
            &mut |progress| {
                let _ = emitter.emit("transcript-model-progress", progress);
            },
        )
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn media_interop_status(state: State<'_, AppState>) -> media::MediaInteropStatus {
    commands::media_interop_status_impl(&state)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn media_run_parity(state: State<'_, AppState>) -> Result<media::MediaParityReport, String> {
    commands::media_run_parity_impl(&state)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn export_start(
    state: State<'_, AppState>,
    project_handle: String,
    settings: export::ExportSettings,
) -> Result<export::ExportStatus, String> {
    commands::export_start_impl(&state, project_handle, settings)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn export_status(
    state: State<'_, AppState>,
    job_id: Option<String>,
) -> Result<export::ExportStatus, String> {
    commands::export_status_impl(&state, job_id)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn export_cancel(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<export::ExportStatus, String> {
    commands::export_cancel_impl(&state, job_id)
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn get_default_projects_dir() -> String {
    commands::get_default_projects_dir_impl()
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn pick_project_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    pick_directory_dialog(
        app,
        "Open AeroEdits Project",
        commands::default_projects_dir(),
    )
    .await
}

#[cfg(feature = "tauri-app")]
async fn pick_directory_dialog(
    app: tauri::AppHandle,
    title: &'static str,
    directory: std::path::PathBuf,
) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        app.run_on_main_thread(move || {
            let mut dialog = rfd::FileDialog::new().set_title(title);
            if directory.is_dir() {
                dialog = dialog.set_directory(&directory);
            }
            let picked = dialog
                .pick_folder()
                .map(|path| path.to_string_lossy().into_owned());
            let _ = tx.send(picked);
        })
        .map_err(|error| error.to_string())?;
        rx.recv().map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn pick_wallpaper_source(app: tauri::AppHandle) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        app.run_on_main_thread(move || {
            let picked = rfd::FileDialog::new()
                .set_title("Choose wallpaper")
                .add_filter("Images", &["png", "jpg", "jpeg"])
                .pick_file()
                .map(|path| path.to_string_lossy().into_owned());
            let _ = tx.send(picked);
        })
        .map_err(|error| error.to_string())?;
        rx.recv().map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
async fn pick_export_destination(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    project_handle: Option<String>,
) -> Result<Option<String>, String> {
    let (default_dir, default_filename, bundle_root) = {
        let opened = state.opened_project.lock();
        if let Some(reader) = opened.as_ref() {
            if let Some(ref handle) = project_handle {
                if reader.summary.project_handle != *handle {
                    return Err("Stale project handle".into());
                }
            }
            let root = reader.root().to_path_buf();
            let parent = root.parent().unwrap_or(&root).to_path_buf();
            let project_name = reader.summary.name.clone();
            let filename = crate::export::default_export_filename(&project_name);
            (parent, filename, Some(root))
        } else {
            (
                commands::default_projects_dir(),
                "Untitled.mp4".to_string(),
                None,
            )
        }
    };
    pick_save_file_dialog(
        app,
        "Export Video",
        default_dir,
        default_filename,
        bundle_root,
    )
    .await
}

#[cfg(feature = "tauri-app")]
async fn pick_save_file_dialog(
    app: tauri::AppHandle,
    title: &'static str,
    directory: std::path::PathBuf,
    default_filename: String,
    bundle_root: Option<std::path::PathBuf>,
) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (tx, rx) = std::sync::mpsc::channel();
        app.run_on_main_thread(move || {
            let mut dialog = rfd::FileDialog::new()
                .set_title(title)
                .set_file_name(&default_filename)
                .add_filter("MP4 Video", &["mp4"]);
            if directory.is_dir() {
                dialog = dialog.set_directory(&directory);
            }
            let picked = dialog
                .save_file()
                .map(|path| path.to_string_lossy().into_owned());
            let _ = tx.send(picked);
        })
        .map_err(|error| error.to_string())?;
        let picked = rx.recv().map_err(|error| error.to_string())?;
        if let Some(ref path_str) = picked {
            let path = std::path::PathBuf::from(path_str);
            if crate::export::is_inside_bundle(&path, bundle_root.as_deref()) {
                return Err("Export destination cannot be inside the project bundle".into());
            }
        }
        Ok(picked)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn set_window_title(window: tauri::Window, title: String) -> Result<(), String> {
    window.set_title(&title).map_err(|e| e.to_string())
}

#[cfg(feature = "tauri-app")]
#[tauri::command]
fn show_in_finder(path: String) -> Result<(), String> {
    commands::show_in_finder_impl(path)
}

#[cfg(feature = "tauri-app")]
const SHORTS_WINDOW: &str = "shorts";

#[cfg(feature = "tauri-app")]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::AppState::default())
        .manage(commands::transcript::TranscriptState::default())
        .setup(|app| {
            playback::engine::start(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                let _ = commands::preview_detach_impl(
                    &window.state::<AppState>(),
                    window.label().to_string(),
                );
                // The Shorts Studio only works alongside the editor: closing the editor
                // closes the app, rather than leaving the studio on torn-down state.
                if window.label() == "main" {
                    window.app_handle().exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            open_project,
            close_project,
            project_rename,
            project_segments,
            project_waveform,
            project_zoom_suggestions,
            project_zoom_accept,
            project_zoom_reload,
            project_zoom_settings_set,
            project_zoom_dismiss,
            project_zoom_update,
            project_zoom_add,
            project_zoom_delete,
            project_layout_update,
            project_webcam_focus_detect,
            project_webcam_focus_update,
            project_webcam_focus_add,
            project_webcam_focus_remove,
            project_audio_update,
            project_captions_update,
            project_ripple_cuts,
            project_sequence_edit,
            project_media_import,
            project_media_roles,
            project_caption_cues,
            transcript_caption_edit,
            project_short_view,
            project_short_resync,
            playback_focus_short,
            project_media_remove,
            pick_media_files,
            pick_media_folder,
            project_undo,
            project_redo,
            playback_status,
            playback_play,
            playback_pause,
            playback_seek,
            preview_attach,
            preview_layout,
            preview_present_fixed,
            preview_present_fixture,
            preview_status,
            preview_frame,
            preview_quality,
            preview_quality_set,
            preview_hit_test,
            preview_detach,
            media_interop_status,
            media_run_parity,
            export_start,
            export_status,
            export_cancel,
            detect_silence,
            transcript_settings_get,
            transcript_settings_set,
            transcript_set_api_key,
            ai_settings_get,
            ai_settings_set,
            ai_set_api_key,
            transcript_ai_suggest,
            project_chapters_set,
            project_chapters_generate,
            project_shorts_set,
            project_shorts_generate,
            project_short_export,
            short_preview_frame,
            open_shorts_window,
            project_current,
            transcript_get,
            transcript_run,
            transcript_cancel,
            transcript_delete,
            transcript_suggestions,
            transcript_cut_words,
            transcript_set_word_text,
            transcript_strip_punctuation,
            transcript_dismiss_suggestions,
            transcript_download_model,
            get_default_projects_dir,
            pick_project_folder,
            pick_recording_folder,
            pick_project_location,
            project_create,
            pick_export_destination,
            pick_wallpaper_source,
            set_window_title,
            show_in_finder
        ])
        .build(tauri::generate_context!())
        .expect("error while building the AeroEdits application")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                app.state::<AppState>()
                    .playback_shutdown
                    .store(true, std::sync::atomic::Ordering::Release);
            }
        });
}
