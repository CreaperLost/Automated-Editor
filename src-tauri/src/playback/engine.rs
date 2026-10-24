//! One media worker per desktop app. Decode/mix/GPU work never runs on the UI thread.
use super::{audio::AudioOutput, PlaybackState, PreviewQuality};
use crate::{
    commands::AppState,
    export::{DecoderWarmer, SceneEvaluator},
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

/// What a drawn frame showed: playback generation, time, preview generation, graphics.
type FrameKey = (u64, u64, u64, u64);

/// Audio queued ahead of the play position, in mixer chunks. The cpal queue (Windows) can
/// take a deep lead, so a slow preview frame does not starve the audio clock.
const AUDIO_LEAD_CHUNKS: u64 = if cfg!(windows) { 10 } else { 3 };
/// While playing, decoders start this far ahead of the clip edges coming up.
const PREFETCH_US: u64 = 1_200_000;
/// The most frames per second the window preview draws at "Source fps" (recordings are 60).
const WINDOW_SOURCE_FPS: u32 = 60;

/// The scene and mixer for what plays. Kept across seeks; rebuilt when an edit changes it.
struct Runtime {
    content: u64,
    /// Built for the webview preview, which copies frames through JPEG.
    webview: bool,
    quality: PreviewQuality,
    evaluator: SceneEvaluator,
    mixer: AudioMixer,
    _lease: std::fs::File,
    /// The shorts either side of the one in focus have been sent to warm up.
    warmed: bool,
}

/// Times the decoders of the neighbouring shorts are asked for, a little apart (the decoder
/// cache turns warm-ups away while it is full of ones not used yet).
const WARM_TRIES: u8 = 8;
const WARM_RETRY: Duration = Duration::from_millis(250);

/// On a thread of its own, once the short in focus has its first frame out: starts decoders
/// for the first frames of the shorts either side of it in the list, so moving to one in the
/// Shorts Studio shows it without waiting for FFmpeg to start. Gives up once something else
/// plays.
fn warm_neighbour_shorts(app: &tauri::AppHandle, warmer: DecoderWarmer, short_id: String) {
    let app = app.clone();
    let content = app.state::<AppState>().playback.lock().content_generation();
    let _ = thread::Builder::new()
        .name("aeroedits-warm-shorts".into())
        .spawn(move || {
            let state = app.state::<AppState>();
            // Starting decoders takes the CPU: not while the frame is still being encoded.
            let deadline = std::time::Instant::now() + Duration::from_millis(300);
            thread::sleep(Duration::from_millis(20));
            while ENCODING.load(Ordering::Acquire) && std::time::Instant::now() < deadline {
                thread::sleep(Duration::from_millis(5));
            }
            for _ in 0..WARM_TRIES {
                if state.playback.lock().content_generation() != content {
                    return;
                }
                let started = std::time::Instant::now();
                let done = warm_neighbours_once(&state, &warmer, &short_id);
                crate::media::profile("warm neighbouring shorts", started);
                if done {
                    return;
                }
                thread::sleep(WARM_RETRY);
            }
        });
}

/// False when a decoder was turned away, to be asked again.
fn warm_neighbours_once(state: &AppState, warmer: &DecoderWarmer, short_id: &str) -> bool {
    let neighbours: Vec<_> = {
        let opened = state.opened_project.lock();
        let Some(reader) = opened.as_ref() else {
            return true;
        };
        let document = &reader.history().current;
        let Some(index) = document.shorts.iter().position(|s| s.id == short_id) else {
            return true;
        };
        [index.checked_sub(1), Some(index + 1)]
            .into_iter()
            .flatten()
            .filter_map(|i| document.shorts.get(i))
            .filter_map(|short| crate::shorts::short_document(document, short, true).ok())
            .collect()
    };
    neighbours.into_iter().fold(true, |all, document| {
        warmer.prefetch_start_of(document) && all
    })
}

/// Drawing the preview straight into the window (Windows): the swapchain, kept across edits
/// and seeks, and the layout it last drew with.
#[derive(Default)]
struct Underlay {
    presenter: Option<crate::render::present::WindowPresenter>,
    /// The preview attachment the presenter belongs to.
    generation: u64,
    layout_revision: u64,
    /// With AEROEDITS_PROFILE set, the presented rate is reported every two seconds.
    presented: u32,
    window: Option<std::time::Instant>,
}

impl Underlay {
    /// Counts a frame drawn; once a second, returns the rate drawn since the last time.
    fn count_presented(&mut self) -> Option<f64> {
        let started = *self.window.get_or_insert_with(std::time::Instant::now);
        self.presented += 1;
        if started.elapsed() < Duration::from_secs(1) {
            return None;
        }
        let fps = self.presented as f64 / started.elapsed().as_secs_f64();
        if crate::media::profiling() {
            eprintln!("[profile] preview presented {fps:.1} fps (window)");
        }
        self.window = Some(std::time::Instant::now());
        self.presented = 0;
        Some(fps)
    }
}

/// The window could not show the preview: frames go to the webview as JPEGs from now on, and
/// the page is told so it can draw them.
fn fall_back_to_webview(
    app: &tauri::AppHandle,
    state: &AppState,
    underlay: &mut Underlay,
    error: String,
) {
    use tauri::Emitter;
    eprintln!("[preview] drawing into the window failed, using the webview: {error}");
    underlay.presenter = None;
    let status = {
        let mut preview = state.preview.lock();
        preview.fall_back_to_webview();
        preview.status()
    };
    let _ = app.emit("preview-status", status);
}

/// A composited frame waiting for JPEG encoding and presentation on the webview surface.
struct WebviewJob {
    frame: crate::media::VideoFrame,
    generation: u64,
    surface_generation: u64,
    /// A short is playing: the Shorts Studio shows these frames even while the editor's
    /// preview is hidden (its window minimized or covered).
    for_short: bool,
}

/// Set while the encoder works on a frame. The media worker then skips drawing a frame it
/// would have to wait to hand over: it goes on feeding audio and draws a fresh frame once
/// the encoder is free, rather than blocking on a frame that is stale by the time it is sent.
static ENCODING: AtomicBool = AtomicBool::new(false);

/// Clears [`ENCODING`] however a job ends.
struct Encoding;

impl Drop for Encoding {
    fn drop(&mut self) {
        ENCODING.store(false, Ordering::Release);
    }
}

/// Encodes and presents webview frames on their own thread, so the next frame decodes and
/// composites while this one is compressed. One frame waits at most: the media worker draws
/// one only while the encoder is free (see [`ENCODING`]), and never waits to hand it over.
fn start_webview_encoder(app: tauri::AppHandle) -> mpsc::SyncSender<WebviewJob> {
    let (sender, jobs) = mpsc::sync_channel::<WebviewJob>(1);
    thread::spawn(move || {
        // With AEROEDITS_PROFILE set, reports the presented preview rate every two seconds.
        let mut window = std::time::Instant::now();
        let mut presented = 0u32;
        for job in jobs {
            ENCODING.store(true, Ordering::Release);
            let _encoding = Encoding;
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
            if current.attached
                && current.generation == job.surface_generation
                && (current.visible || job.for_short)
            {
                match surface.present_encoded(jpeg, job.surface_generation, job.generation) {
                    Ok(()) => {
                        owner.mark_presented(job.generation);
                        if job.for_short {
                            crate::media::switch_stage("frame ready for the studio", true);
                        }
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
        let mut underlay = Underlay::default();
        let pending = Arc::new(AtomicBool::new(false));
        let mut last_frame: Option<FrameKey> = None;
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
                &mut underlay,
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
            // enough to catch every frame at 60 fps; when paused, poll gently (a seek or another
            // short wakes it at once).
            let rendered = last_frame.is_some() && last_frame != before;
            super::wait_for_wake(Duration::from_millis(if rendered {
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
    underlay: &mut Underlay,
    pending: &Arc<AtomicBool>,
    last_frame: &mut Option<FrameKey>,
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
    let content = state.playback.lock().content_generation();
    if runtime.as_ref().map(|r| (r.content, r.webview, r.quality))
        != Some((content, webview, quality))
    {
        let rebuild_started = std::time::Instant::now();
        let (root, document) = {
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
            )
        };
        // The short in focus plays as its own vertical video.
        let document = state
            .playback
            .lock()
            .playable_document(&document)
            .map_err(error)?;
        let lease = crate::project::reader::acquire_read_lease(&root).map_err(error)?;
        crate::media::profile("rebuild: document", rebuild_started);
        let mixer = AudioMixer::new(&root, &document).map_err(error)?;
        crate::media::profile("rebuild: mixer", rebuild_started);
        // The edit changed: the graphics' export renders are made in the background now, so an
        // export finds them ready.
        crate::graphics::render::request_export(&root, &document);
        let (width, height) = document.layout.preview_dimensions().map_err(error)?;
        let (width, height) = quality.canvas(width, height);
        // Keep the GPU device and background across seeks; recreating them dominated seek time.
        let reuse = runtime.take().and_then(|old| old.evaluator.into_reuse());
        let evaluator = SceneEvaluator::new_reusing(root, document, width, height, reuse)
            .map_err(error)?
            .with_decode_limit(quality.decode_limit((width, height), webview));
        *runtime = Some(Runtime {
            content,
            webview,
            quality,
            evaluator,
            mixer,
            _lease: lease,
            warmed: false,
        });
        *last_frame = None;
        crate::media::profile("rebuild after edit", rebuild_started);
        crate::media::switch_stage("engine rebuilt", false);
    }
    let runtime = runtime.as_mut().unwrap();
    if status.state == PlaybackState::Playing && !runtime.mixer.has_audio() {
        state.playback.lock().run_without_audio(generation);
    }
    // Faster playback plays through the sound faster: queue that much more ahead.
    let lead_chunks = AUDIO_LEAD_CHUNKS * status.speed.ceil().max(1.0) as u64;
    if status.state == PlaybackState::Playing && runtime.mixer.has_audio() {
        let initialize = {
            let owner = state.playback.lock();
            owner.audio.is_none() && owner.needs_audio()
        };
        if initialize {
            let base = (status.position_us as u128 * SAMPLE_RATE as u128 / 1_000_000) as u64;
            let mut output = AudioOutput::new().map_err(error)?;
            output.set_speed(status.speed);
            let mut queued = base;
            while queued
                < (base + CHUNK_FRAMES as u64 * lead_chunks).min(runtime.mixer.total_frames)
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
                || queued >= position + lead_chunks * CHUNK_FRAMES as u64
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
    let in_window = preview.surface == "underlay";
    if !in_window || underlay.generation != preview.generation {
        // Detached or gone back to the webview: let the window's swapchain go, so a new one
        // can be made on the same window.
        underlay.presenter = None;
        underlay.generation = preview.generation;
    }
    // A short in focus is watched in the Shorts Studio, whatever the editor's window does. It
    // takes its frames as JPEGs, so they go through the webview path.
    let for_short = status.short_id.is_some() && (preview.surface == "webview" || in_window);
    if !preview.attached || !(preview.visible || for_short) || pending.load(Ordering::Acquire) {
        return Ok(playing);
    }
    // Parked at the end, the preview shows the last frame; an empty timeline shows the empty
    // canvas. Both must be drawn: in the window, nothing else paints under the page.
    let position_us = status.position_us.min(status.duration_us.saturating_sub(1));
    // While playing, the preview shows whole frames at its rate. The clock moves every tick,
    // so without this the same source frame was composited and encoded again and again,
    // holding up the next real frame.
    let render_us = if status.state == PlaybackState::Playing {
        if in_window && runtime.quality.fps == 0 {
            // Drawing into the window is cheap enough to redraw on every tick, which would show
            // the same source frame several times over: follow the source at most at 60 fps.
            super::preview::frame_start_us(position_us, WINDOW_SOURCE_FPS)
        } else {
            runtime.quality.frame_time(position_us)
        }
    } else {
        position_us
    };
    // A graphic that finished rendering changes the frame without an edit.
    let key = (
        generation,
        render_us,
        preview.generation,
        crate::graphics::render::generation(),
    );
    // Decoders start the same real time ahead at any speed.
    let horizon = (PREFETCH_US as f64 * status.speed.max(1.0)) as u64;
    if in_window && !for_short {
        let Some((window, placement)) = state.preview.lock().underlay() else {
            return Ok(playing); // Not laid out yet.
        };
        if *last_frame == Some(key) {
            // The same frame: draw it again only if the preview moved.
            if underlay.layout_revision != preview.layout_revision {
                match runtime
                    .evaluator
                    .present_again(&mut underlay.presenter, &window, &placement)
                {
                    Ok(_) => underlay.layout_revision = preview.layout_revision,
                    Err(e) => fall_back_to_webview(app, state, underlay, e),
                }
            }
            return Ok(playing);
        }
        match runtime
            .evaluator
            .present_at(render_us, &mut underlay.presenter, &window, &placement)
        {
            Ok(_) => {
                *last_frame = Some(key);
                underlay.layout_revision = preview.layout_revision;
                let fps = underlay.count_presented();
                state.preview.lock().mark_underlay_presented(fps);
                state.playback.lock().mark_presented(generation);
                if playing {
                    runtime.evaluator.prefetch(render_us, horizon);
                }
            }
            Err(crate::export::PresentError::Scene(e)) => return Err(error(e)),
            Err(crate::export::PresentError::Window(e)) => {
                fall_back_to_webview(app, state, underlay, e)
            }
        }
        return Ok(playing);
    }
    if *last_frame == Some(key) {
        // A short's frame in the window: draw it again where the preview moved to.
        if in_window && preview.visible && underlay.layout_revision != preview.layout_revision {
            // Taken apart from the `if let`: its lock would last the whole draw, and drawing
            // can wait for the window's thread while that thread waits for this lock.
            let target = state.preview.lock().underlay();
            if let Some((window, placement)) = target {
                match runtime
                    .evaluator
                    .present_again(&mut underlay.presenter, &window, &placement)
                {
                    Ok(_) => underlay.layout_revision = preview.layout_revision,
                    Err(e) => fall_back_to_webview(app, state, underlay, e),
                }
            }
        }
        return Ok(playing);
    }
    if preview.surface == "webview" && ENCODING.load(Ordering::Acquire) {
        return Ok(playing);
    }
    let frame = runtime.evaluator.preview_at(render_us).map_err(error)?;
    crate::media::switch_stage("frame composited", false);
    if status.state == PlaybackState::Playing {
        runtime.evaluator.prefetch(render_us, horizon);
    }
    if in_window && preview.visible {
        // The short's frame (sent to the Shorts Studio below) shows in the editor too. The lock
        // is let go before drawing (see above).
        let target = state.preview.lock().underlay();
        if let Some((window, placement)) = target {
            match runtime
                .evaluator
                .present_again(&mut underlay.presenter, &window, &placement)
            {
                Ok(_) => underlay.layout_revision = preview.layout_revision,
                Err(e) => fall_back_to_webview(app, state, underlay, e),
            }
        }
    }
    if preview.surface == "webview" || in_window {
        // The webview fetches frames itself, so nothing here needs the UI thread.
        match encoder.try_send(WebviewJob {
            frame,
            generation,
            surface_generation: preview.generation,
            for_short,
        }) {
            Ok(()) => {
                *last_frame = Some(key);
                // Paused on a short with its frame out: get its neighbours' first frames ready.
                if let (Some(short_id), false) = (&status.short_id, playing) {
                    if !runtime.warmed {
                        runtime.warmed = true;
                        let warmer = runtime.evaluator.decoder_warmer();
                        warm_neighbour_shorts(app, warmer, short_id.clone());
                    }
                }
            }
            // Taken up a moment ago: this frame is dropped and drawn again next tick.
            Err(mpsc::TrySendError::Full(_)) => {}
            Err(mpsc::TrySendError::Disconnected(_)) => {
                return Err(error("The preview encoder stopped".to_string()))
            }
        }
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
