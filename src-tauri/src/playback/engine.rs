//! One media worker per desktop app. Decode/mix/GPU work never runs on the UI thread.
use super::{audio::AudioOutput, PlaybackState, PreviewQuality};
use crate::{
    commands::AppState,
    export::SceneEvaluator,
    media::audio::{AudioMixer, CHUNK_FRAMES, SAMPLE_RATE},
};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::Duration,
};
use tauri::Manager;

/// Audio queued ahead of the play position, in mixer chunks. The cpal queue (Windows) can
/// take a deep lead, so a slow preview frame does not starve the audio clock.
const AUDIO_LEAD_CHUNKS: u64 = if cfg!(windows) { 10 } else { 3 };

struct Runtime {
    generation: u64,
    /// Built for the webview preview, which copies frames through JPEG.
    webview: bool,
    quality: PreviewQuality,
    evaluator: SceneEvaluator,
    mixer: AudioMixer,
    _lease: std::fs::File,
}

/// A composited frame waiting for JPEG encoding and presentation on the webview surface.
struct WebviewJob {
    frame: crate::media::VideoFrame,
    generation: u64,
    surface_generation: u64,
}

/// Encodes and presents webview frames on their own thread, so the next frame decodes and
/// composites while this one is compressed. The channel holds no buffer: handing over a frame
/// waits for the previous one to finish, which keeps at most one frame in flight.
fn start_webview_encoder(app: tauri::AppHandle) -> mpsc::SyncSender<WebviewJob> {
    let (sender, jobs) = mpsc::sync_channel::<WebviewJob>(0);
    thread::spawn(move || {
        // With AEROEDITS_PROFILE set, reports the presented preview rate every two seconds.
        let mut window = std::time::Instant::now();
        let mut presented = 0u32;
        for job in jobs {
            if window.elapsed() >= Duration::from_secs(2) {
                if presented > 0 && crate::media::profiling() {
                    eprintln!(
                        "[profile] preview presented {:.1} fps",
                        presented as f64 / window.elapsed().as_secs_f64()
                    );
                }
                window = std::time::Instant::now();
                presented = 0;
            }
            let state = app.state::<AppState>();
            let started = std::time::Instant::now();
            let jpeg = super::preview::encode_webview_frame(&job.frame);
            crate::media::profile("jpeg", started);
            let mut owner = state.playback.lock();
            if !owner.status().is_ok_and(|s| {
                s.generation == job.generation
                    && !matches!(s.state, PlaybackState::Closed | PlaybackState::Error)
            }) {
                continue;
            }
            let jpeg = match jpeg {
                Ok(jpeg) => jpeg,
                Err(e) => {
                    owner.fail(job.generation, e);
                    continue;
                }
            };
            let mut surface = state.preview.lock();
            let current = surface.status();
            if current.attached && current.generation == job.surface_generation && current.visible {
                match surface.present_encoded(jpeg, job.surface_generation) {
                    Ok(()) => {
                        owner.mark_presented(job.generation);
                        presented += 1;
                        state.preview_frame_ready.notify_waiters();
                    }
                    Err(e) => owner.fail(job.generation, e),
                }
            }
        }
    });
    sender
}

pub fn start(app: tauri::AppHandle) {
    thread::spawn(move || {
        let encoder = start_webview_encoder(app.clone());
        let mut runtime: Option<Runtime> = None;
        let pending = Arc::new(AtomicBool::new(false));
        let mut last_frame: Option<(u64, u64, u64)> = None;
        while !app
            .state::<AppState>()
            .playback_shutdown
            .load(Ordering::Acquire)
        {
            let state = app.state::<AppState>();
            let before = last_frame;
            let result = tick(
                &app,
                &state,
                &encoder,
                &mut runtime,
                &pending,
                &mut last_frame,
            );
            let playing = matches!(result, Ok(true));
            if let Err((generation, error)) = result {
                state.playback.lock().fail(generation, error);
                let app_copy = app.clone();
                let _ = app.run_on_main_thread(move || {
                    let state = app_copy.state::<AppState>();
                    if state
                        .playback
                        .lock()
                        .status()
                        .is_ok_and(|s| s.generation == generation)
                    {
                        let mut preview = state.preview.lock();
                        let snapshot = preview.status();
                        if snapshot.attached {
                            let _ = preview.present_fixed(0.0, 0.0, 0.0, snapshot.generation);
                        }
                    }
                });
            }
            // Right after a frame, go straight on to the next one. While playing, check often
            // enough to catch every frame at 60 fps; when paused, poll gently.
            let rendered = last_frame.is_some() && last_frame != before;
            thread::sleep(Duration::from_millis(if rendered {
                1
            } else if playing {
                3
            } else {
                15
            }));
        }
        app.state::<AppState>().playback.lock().close();
    });
}

fn tick(
    app: &tauri::AppHandle,
    state: &AppState,
    encoder: &mpsc::SyncSender<WebviewJob>,
    runtime: &mut Option<Runtime>,
    pending: &Arc<AtomicBool>,
    last_frame: &mut Option<(u64, u64, u64)>,
) -> Result<bool, (u64, String)> {
    let status = state.playback.lock().status().map_err(|e| (0, e))?;
    if matches!(status.state, PlaybackState::Closed | PlaybackState::Error) {
        *runtime = None;
        return Ok(false);
    }
    let playing = status.state == PlaybackState::Playing;
    let generation = status.generation;
    let error = |e| (generation, e);
    let webview = state.preview.lock().status().surface == "webview";
    let quality = state
        .preview_quality
        .lock()
        .unwrap_or_else(|| PreviewQuality::default_for(webview));
    if runtime
        .as_ref()
        .map(|r| (r.generation, r.webview, r.quality))
        != Some((generation, webview, quality))
    {
        let rebuild_started = std::time::Instant::now();
        let (root, document, tracks) = {
            let opened = state.opened_project.lock();
            let Some(reader) = opened.as_ref() else {
                return Ok(false);
            };
            if reader.summary.project_handle != status.project_handle {
                return Ok(false);
            }
            (
                reader.root().to_path_buf(),
                reader.history().current.clone(),
                super::tracks_from_reader(reader),
            )
        };
        // The short in focus plays as its own vertical video.
        let document = state
            .playback
            .lock()
            .playable_document(&document)
            .map_err(error)?;
        let lease = crate::project::reader::acquire_read_lease(&root).map_err(error)?;
        let mixer = AudioMixer::new(&root, &document, &tracks).map_err(error)?;
        let (width, height) = document.layout.preview_dimensions().map_err(error)?;
        let (width, height) = quality.canvas(width, height);
        // Keep the GPU device and background across seeks; recreating them dominated seek time.
        let reuse = runtime.take().and_then(|old| old.evaluator.into_reuse());
        let evaluator = SceneEvaluator::new_reusing(root, document, tracks, width, height, reuse)
            .map_err(error)?
            .with_decode_limit(quality.decode_limit((width, height), webview));
        *runtime = Some(Runtime {
            generation,
            webview,
            quality,
            evaluator,
            mixer,
            _lease: lease,
        });
        *last_frame = None;
        crate::media::profile("rebuild after seek", rebuild_started);
    }
    let runtime = runtime.as_mut().unwrap();
    if status.state == PlaybackState::Playing && runtime.mixer.has_audio() {
        let initialize = {
            let owner = state.playback.lock();
            owner.audio.is_none()
        };
        if initialize {
            let base = (status.position_us as u128 * SAMPLE_RATE as u128 / 1_000_000) as u64;
            let mut output = AudioOutput::new().map_err(error)?;
            let mut queued = base;
            while queued
                < (base + CHUNK_FRAMES as u64 * AUDIO_LEAD_CHUNKS).min(runtime.mixer.total_frames)
            {
                let chunk = runtime
                    .mixer
                    .read_frames(queued, CHUNK_FRAMES)
                    .map_err(error)?;
                if chunk.is_empty() {
                    break;
                }
                queued += (chunk.len() / 2) as u64;
                output.queue(&chunk).map_err(error)?;
            }
            let mut owner = state.playback.lock();
            let current = owner.status().map_err(error)?;
            if current.generation != generation || current.state != PlaybackState::Playing {
                return Ok(playing);
            }
            owner.audio_start_frame = base;
            owner.audio_queued_frame = queued;
            output.play();
            owner.audio = Some(output);
        }
        loop {
            let (queued, position) = {
                let mut owner = state.playback.lock();
                let current = owner.status().map_err(error)?;
                if current.generation != generation || current.state != PlaybackState::Playing {
                    break;
                }
                (
                    owner.audio_queued_frame,
                    (current.position_us as u128 * SAMPLE_RATE as u128 / 1_000_000) as u64,
                )
            };
            if queued >= runtime.mixer.total_frames
                || queued >= position + AUDIO_LEAD_CHUNKS * CHUNK_FRAMES as u64
            {
                break;
            }
            let chunk = runtime
                .mixer
                .read_frames(queued, CHUNK_FRAMES)
                .map_err(error)?;
            if chunk.is_empty() {
                break;
            }
            let mut owner = state.playback.lock();
            let current = owner.status().map_err(error)?;
            if current.generation != generation || current.state != PlaybackState::Playing {
                return Ok(playing);
            }
            if let Some(output) = owner.audio.as_mut() {
                output.queue(&chunk).map_err(error)?;
                owner.audio_queued_frame += (chunk.len() / 2) as u64;
            } else {
                break;
            }
        }
    }
    let status = state.playback.lock().status().map_err(error)?;
    if status.generation != generation {
        return Ok(playing);
    }
    let preview = state.preview.lock().status();
    if !preview.attached
        || !preview.visible
        || status.duration_us == 0
        || status.position_us >= status.duration_us
        || pending.load(Ordering::Acquire)
    {
        return Ok(playing);
    }
    // While playing, the preview shows whole frames at its rate. The clock moves every tick,
    // so without this the same source frame was composited and encoded again and again,
    // holding up the next real frame.
    let render_us = if status.state == PlaybackState::Playing {
        runtime.quality.frame_time(status.position_us)
    } else {
        status.position_us
    };
    let key = (generation, render_us, preview.generation);
    if *last_frame == Some(key) {
        return Ok(playing);
    }
    let frame = runtime.evaluator.preview_at(render_us).map_err(error)?;
    if preview.surface == "webview" {
        // The webview fetches frames itself, so nothing here needs the UI thread.
        encoder
            .send(WebviewJob {
                frame,
                generation,
                surface_generation: preview.generation,
            })
            .map_err(|_| error("The preview encoder stopped".to_string()))?;
        *last_frame = Some(key);
        return Ok(playing);
    }
    pending.store(true, Ordering::Release);
    let pending_done = Arc::clone(pending);
    let app_copy = app.clone();
    let surface_generation = preview.generation;
    app.run_on_main_thread(move || {
        let state = app_copy.state::<AppState>();
        let mut owner = state.playback.lock();
        if owner.status().is_ok_and(|s| {
            s.generation == generation
                && !matches!(s.state, PlaybackState::Closed | PlaybackState::Error)
        }) {
            let mut surface = state.preview.lock();
            let current = surface.status();
            if current.attached && current.generation == surface_generation && current.visible {
                match surface.present_frame(&frame, surface_generation) {
                    Ok(()) => {
                        owner.mark_presented(generation);
                    }
                    Err(e) => owner.fail(generation, e),
                }
            }
        }
        pending_done.store(false, Ordering::Release);
    })
    .map_err(|e| {
        pending.store(false, Ordering::Release);
        error(e.to_string())
    })?;
    *last_frame = Some(key);
    Ok(playing)
}
