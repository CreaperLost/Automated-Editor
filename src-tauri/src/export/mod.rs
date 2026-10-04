//! Immutable-revision export job. Preview and export share one scene evaluator.
use crate::media::audio::{AudioMixer, CHANNELS, CHUNK_FRAMES, SAMPLE_RATE};
mod native;

use crate::media::ffmpeg::{DecodeLimit, FfmpegExport};
use crate::media::{
    decode_h264_frame, decode_h264_frame_limited, media_backend, media_duration_us, EncoderGate,
    MediaBackend, RateControl, VideoFrame, VideoQuality, MAX_FRAME_DIM,
};
use crate::project::reader::{safe_path, SegmentSummary};
use crate::project::revision::EditDocument;
use crate::render::{Compositor, Scene};
use crate::sequence::{Asset, Clip, Role};
use native::NativeExport;

pub(crate) use native::media_duration_us as native_media_duration_us;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use uuid::Uuid;

pub const ALLOWED_FPS: [u32; 6] = [10, 15, 24, 25, 30, 60];
pub const MIN_BITRATE_KBPS: u32 = 1_000;
pub const MAX_BITRATE_KBPS: u32 = 200_000;
/// One AAC frame at 48 kHz plus a small encoder-delay allowance.
pub const AUDIO_DURATION_SLACK_US: u64 = 80_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExportSettings {
    pub video_codec: String,
    pub audio_codec: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    #[serde(default)]
    pub destination: Option<String>,
    /// Constant-quality preset; ignored when `bitrate_kbps` is set.
    #[serde(default)]
    pub quality: VideoQuality,
    /// Average video bitrate. `None` uses `quality` instead.
    #[serde(default)]
    pub bitrate_kbps: Option<u32>,
}

impl ExportSettings {
    pub fn rate_control(&self) -> RateControl {
        match self.bitrate_kbps {
            Some(kbps) => RateControl::Bitrate(kbps as u64 * 1_000),
            None => RateControl::Quality(self.quality),
        }
    }
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            video_codec: "h264".into(),
            audio_codec: "aac".into(),
            width: 1920,
            height: 1080,
            fps: 30,
            destination: None,
            quality: VideoQuality::default(),
            bitrate_kbps: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportState {
    Idle,
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExportFailure {
    Collision { message: String },
    Cancelled { message: String },
    InvalidSettings { message: String },
    SourcePath { message: String },
    EncoderBusy { message: String },
    Native { message: String },
    Io { message: String },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExportStatus {
    pub job_id: String,
    pub state: ExportState,
    pub captured_revision: u64,
    pub video_codec: String,
    pub audio_codec: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub progress_numerator: u32,
    pub progress_denominator: u32,
    pub output_path: Option<String>,
    pub failure: Option<ExportFailure>,
    pub diagnostics: Vec<String>,
}

impl ExportStatus {
    pub fn idle() -> Self {
        Self {
            job_id: String::new(),
            state: ExportState::Idle,
            captured_revision: 0,
            video_codec: "h264".into(),
            audio_codec: "aac".into(),
            width: 1920,
            height: 1080,
            fps: 30,
            progress_numerator: 0,
            progress_denominator: 0,
            output_path: None,
            failure: None,
            diagnostics: Vec::new(),
        }
    }
}

struct LiveJob {
    id: String,
    cancel: Arc<AtomicBool>,
    status: Arc<Mutex<ExportStatus>>,
    join: Option<JoinHandle<()>>,
}

pub struct ExportOwner {
    job: Option<LiveJob>,
}

impl ExportOwner {
    pub fn new() -> Self {
        Self { job: None }
    }

    pub fn status(&mut self) -> ExportStatus {
        self.reap();
        self.job
            .as_ref()
            .map(|job| job.status.lock().clone())
            .unwrap_or_else(ExportStatus::idle)
    }

    pub fn cancel(&mut self, job_id: &str) -> Result<ExportStatus, String> {
        self.reap();
        let job = self.job.as_mut().ok_or("No export job")?;
        if job.id != job_id {
            return Err("Stale export job id".into());
        }
        job.cancel.store(true, Ordering::SeqCst);
        let mut status = job.status.lock();
        if matches!(status.state, ExportState::Queued | ExportState::Running) {
            status.diagnostics = vec!["Cancellation requested; waiting for cleanup".into()];
        }
        Ok(status.clone())
    }

    pub fn busy(&mut self) -> bool {
        self.reap();
        self.job
            .as_ref()
            .map(|job| {
                job.join.as_ref().is_some_and(|join| !join.is_finished())
                    || matches!(
                        job.status.lock().state,
                        ExportState::Queued | ExportState::Running
                    )
            })
            .unwrap_or(false)
    }

    pub fn install_failed(&mut self, status: ExportStatus) {
        self.reap();
        if self.busy() {
            return;
        }
        self.job = Some(LiveJob {
            id: status.job_id.clone(),
            cancel: Arc::new(AtomicBool::new(false)),
            status: Arc::new(Mutex::new(status)),
            join: None,
        });
    }

    fn reap(&mut self) {
        if let Some(job) = self.job.as_mut() {
            if job
                .join
                .as_ref()
                .map(|handle| handle.is_finished())
                .unwrap_or(true)
            {
                if let Some(handle) = job.join.take() {
                    if handle.join().is_err() {
                        let mut status = job.status.lock();
                        status.state = ExportState::Failed;
                        status.output_path = None;
                        status.failure = Some(ExportFailure::Native {
                            message: "Export worker terminated unexpectedly".into(),
                        });
                    }
                }
            }
        }
    }
}

impl Default for ExportOwner {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ExportOwner {
    fn drop(&mut self) {
        if let Some(job) = self.job.as_mut() {
            job.cancel.store(true, Ordering::SeqCst);
            if let Some(handle) = job.join.take() {
                let _ = handle.join();
            }
        }
    }
}

#[derive(Clone)]
pub struct CapturedExport {
    pub job_id: String,
    pub root: PathBuf,
    pub document: EditDocument,
    pub dest: PathBuf,
    pub temp: PathBuf,
    pub settings: ExportSettings,
    pub lease: Arc<File>,
}

type Zooms = std::sync::Arc<Vec<crate::zoom::ZoomSuggestion>>;

pub struct SceneEvaluator {
    root: PathBuf,
    document: EditDocument,
    compositor: Option<Compositor>,
    width: u32,
    height: u32,
    decode_limit: DecodeLimit,
    /// The document is fixed, so the wallpaper or gradient is built once rather than per frame.
    wallpaper: std::cell::OnceCell<Option<VideoFrame>>,
    /// Enabled webcam focus segments on the timeline, merged.
    webcam_focus: std::cell::OnceCell<Vec<(u64, u64)>>,
    /// Caption cues from the captioned sound's transcript; empty when captions are off.
    caption_cues: std::cell::OnceCell<Vec<crate::captions::CaptionCue>>,
    /// The last cue drawn and its colored frame for the active word.
    caption_cache: std::cell::RefCell<CaptionCache>,
    /// Decoded stills, most recently used last.
    image_cache: std::cell::RefCell<Vec<(String, VideoFrame)>>,
    /// Recorded pointers by recording, read once per evaluator.
    cursors: std::cell::RefCell<
        std::collections::HashMap<String, Option<std::sync::Arc<crate::cursor::CursorTrack>>>,
    >,
    /// Zooms by recording, with the camera paths that follow the mouse, worked out once.
    zooms: std::cell::RefCell<std::collections::HashMap<String, Zooms>>,
    /// Pointer pictures by file, decoded once.
    cursor_images: std::cell::RefCell<std::collections::HashMap<PathBuf, Option<VideoFrame>>>,
}

#[derive(Default)]
struct CaptionCache {
    raster: Option<(usize, Option<crate::captions::CueRaster>)>,
    frame: Option<((usize, Option<usize>), VideoFrame)>,
}

/// State worth keeping when the playback worker rebuilds its evaluator after a seek or an
/// edit: the GPU device (slow to create) and the background, if the layout kept it.
pub struct EvaluatorReuse {
    compositor: Compositor,
    background: Option<(String, Option<VideoFrame>)>,
}

fn background_key(
    root: &Path,
    layout: &crate::project::layout::EditLayout,
    w: u32,
    h: u32,
) -> String {
    format!(
        "{}|{}|{:?}|{}|{}|{}|{w}x{h}",
        root.display(),
        layout.background_type,
        layout.wallpaper_asset,
        layout.background_preset,
        layout.color_start,
        layout.color_end
    )
}

/// A picture to decode, with nothing borrowed, so screen and camera decode side by side.
enum PictureJob {
    Segment(SegmentSummary, u64),
    Video(PathBuf, u64),
    Ready(VideoFrame),
}

impl PictureJob {
    fn run(self, root: &Path, limit: DecodeLimit) -> Result<VideoFrame, String> {
        match self {
            PictureJob::Segment(segment, time) => decode_layer(root, &segment, time, limit),
            PictureJob::Video(path, time) => {
                crate::media::ffmpeg::decode_bgra_limited(&path, time, limit)
            }
            PictureJob::Ready(frame) => Ok(frame),
        }
    }
}

/// The video clips showing at one moment: the screen, the camera and the overlays.
#[derive(Default)]
struct Shown<'a> {
    /// The topmost screen clip, with the index of its video track.
    screen: Option<(usize, &'a Clip)>,
    /// The topmost camera clip.
    webcam: Option<&'a Clip>,
    /// Overlay clips, bottom track first, with their track index and fit.
    overlays: Vec<(usize, &'a Clip)>,
}

impl SceneEvaluator {
    pub fn new(
        root: PathBuf,
        document: EditDocument,
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        Self::new_reusing(root, document, width, height, None)
    }

    pub fn new_reusing(
        root: PathBuf,
        document: EditDocument,
        width: u32,
        height: u32,
        reuse: Option<EvaluatorReuse>,
    ) -> Result<Self, String> {
        let key = background_key(&root, &document.layout, width, height);
        let wallpaper = std::cell::OnceCell::new();
        let compositor = match reuse {
            Some(reuse) => {
                if let Some((_, frame)) = reuse.background.filter(|(k, _)| *k == key) {
                    let _ = wallpaper.set(frame);
                }
                reuse.compositor
            }
            None => Compositor::new()?,
        };
        Ok(Self {
            root,
            document,
            compositor: Some(compositor),
            width,
            height,
            decode_limit: DecodeLimit::NONE,
            wallpaper,
            webcam_focus: std::cell::OnceCell::new(),
            caption_cues: std::cell::OnceCell::new(),
            caption_cache: std::cell::RefCell::new(CaptionCache::default()),
            image_cache: std::cell::RefCell::new(Vec::new()),
            cursors: Default::default(),
            zooms: Default::default(),
            cursor_images: Default::default(),
        })
    }

    /// Gives up the parts a replacement evaluator can reuse.
    pub fn into_reuse(self) -> Option<EvaluatorReuse> {
        let key = background_key(&self.root, &self.document.layout, self.width, self.height);
        let background = self.wallpaper.into_inner().map(|frame| (key, frame));
        self.compositor.map(|compositor| EvaluatorReuse {
            compositor,
            background,
        })
    }

    /// Decodes sources no larger or faster than `limit`, for preview.
    pub fn with_decode_limit(mut self, limit: DecodeLimit) -> Self {
        self.decode_limit = limit;
        self
    }

    pub fn preview_at(&mut self, edited_us: u64) -> Result<VideoFrame, String> {
        let started = std::time::Instant::now();
        let mut scene = self.scene_at(edited_us)?;
        crate::media::profile("scene (decode)", started);
        // The background is the same picture on every frame: let the GPU keep it.
        if self.wallpaper.get().is_some_and(Option::is_some) {
            if let Some(layer) = scene
                .layers
                .iter_mut()
                .find(|layer| layer.role == crate::render::LayerRole::Background)
            {
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                background_key(&self.root, &self.document.layout, self.width, self.height)
                    .hash(&mut hasher);
                layer.cache_key = Some(hasher.finish());
            }
        }
        let started = std::time::Instant::now();
        let mut frame = match self.compositor.as_mut() {
            Some(compositor) => compositor.composite(&scene)?,
            None => Compositor::composite_cpu(&scene)?,
        };
        crate::media::profile("composite", started);
        frame.pts_us = edited_us;
        Ok(frame)
    }

    /// What the video tracks show at `edited_us`, by role.
    fn shown(&self, edited_us: u64) -> Shown<'_> {
        let document = &self.document;
        let mut shown = Shown::default();
        for (index, track) in document.sequence.video_tracks().enumerate() {
            if track.hidden {
                continue;
            }
            let Some(clip) = track.clip_at(edited_us) else {
                continue;
            };
            match crate::sequence::clip_role(&document.assets, track, clip) {
                // A higher track's screen or camera takes over from a lower one's.
                Some(Role::Screen) => shown.screen = Some((index, clip)),
                Some(Role::Webcam) => shown.webcam = Some(clip),
                Some(_) => shown.overlays.push((index, clip)),
                None => {}
            }
        }
        shown
    }

    pub fn scene_at(&self, edited_us: u64) -> Result<Scene, String> {
        let shown = self.shown(edited_us);
        let at = |clip: &Clip| clip.local_us(edited_us).unwrap_or(clip.in_us);
        // Screen and camera decode in parallel; each waits on its own FFmpeg process.
        let screen_job = shown
            .screen
            .map(|(_, clip)| self.picture_job(clip, at(clip)))
            .transpose()?
            .flatten();
        let webcam_job = shown
            .webcam
            .filter(|_| self.document.layout.webcam_enabled)
            .map(|clip| self.picture_job(clip, at(clip)))
            .transpose()?
            .flatten();
        let (root, limit) = (&self.root, self.decode_limit);
        let (screen, webcam) = std::thread::scope(|scope| {
            let webcam = webcam_job.map(|job| scope.spawn(move || job.run(root, limit)));
            let screen = screen_job.map(|job| job.run(root, limit)).transpose()?;
            let webcam = match webcam {
                Some(handle) => Some(
                    handle
                        .join()
                        .map_err(|_| "Webcam decode panicked".to_string())??,
                ),
                None => None,
            };
            Ok::<_, String>((screen, webcam))
        })?;
        if let Some(short) = self.document.short_layout.clone() {
            return self.split_scene(&short, edited_us, &shown, screen, webcam);
        }
        let unit = crate::render::layout_px_unit(self.width, self.height);
        let has_screen = screen.is_some();
        let has_webcam = webcam.is_some();
        // Nothing to frame: the canvas is black, with any overlays on it.
        let mut scene = if has_screen || has_webcam {
            let mut layout = self.document.layout.clone();
            layout.webcam_enabled = has_webcam;
            Scene::from_layout_scaled(
                self.width,
                self.height,
                &layout,
                screen,
                webcam,
                self.background()?,
                unit,
            )?
        } else {
            Scene {
                width: self.width,
                height: self.height,
                background: [0.0, 0.0, 0.0, 1.0],
                layers: Vec::new(),
            }
        };
        if let (Some((_, clip)), true) = (shown.screen, has_screen) {
            let local = at(clip);
            let zooms = self.zooms_for(&clip.asset);
            if !zooms.is_empty() {
                let config = crate::zoom::eval_config_for(&self.document.zooms);
                let camera = crate::zoom::evaluate_at_source(&zooms, local, &config);
                let crop = self.document.layout.screen_crop_uv();
                let (x, y, w, h) = crate::render::zoom_within_crop(crop, camera.uv_rect());
                scene.apply_screen_uv(x, y, w, h);
            }
            self.push_cursor(&mut scene, &clip.asset, local)?;
        }
        if has_webcam {
            let focus = &self.document.webcam_focus;
            let ranges = self
                .webcam_focus
                .get_or_init(|| focus.edited_ranges(&self.document.focus_clock()));
            let weight = crate::webcam_focus::focus_weight(
                ranges,
                edited_us,
                focus.settings.transition_us(),
            );
            scene.apply_webcam_focus(
                &self.document.layout,
                focus.settings.focus_size_pct,
                weight,
                unit,
            );
        }
        // Overlays draw in track order; one below the screen's track goes under it.
        let screen_track = shown.screen.map(|(index, _)| index);
        for (index, clip) in &shown.overlays {
            let job = self.picture_job(clip, at(clip))?;
            if let Some(frame) = job
                .map(|job| job.run(&self.root, self.decode_limit))
                .transpose()?
            {
                let before = scene.layers.len();
                scene.push_overlay(frame, clip.fit == crate::sequence::Fit::Cover);
                if screen_track.is_some_and(|screen| *index < screen) {
                    scene.move_under_main(before);
                }
            }
        }
        // Captions go over everything.
        if let Some((frame, x, y)) = self.caption_at(edited_us) {
            scene.push_caption(frame, x, y);
        }
        Ok(scene)
    }

    /// A short's vertical frame: the camera across the top or bottom and the screen in the
    /// rest, cut to that shape and following the screen's zooms, with captions placed to suit.
    fn split_scene(
        &self,
        short: &crate::shorts::ShortLayout,
        edited_us: u64,
        shown: &Shown<'_>,
        screen: Option<VideoFrame>,
        webcam: Option<VideoFrame>,
    ) -> Result<Scene, String> {
        use crate::render::{Layer, LayerRole};
        let layout = &self.document.layout;
        let webcam = webcam.filter(|_| layout.webcam_enabled);
        let rects = crate::shorts::split_rects(self.width, self.height, short, webcam.is_some());
        let mut layers = Vec::new();
        let screen_clip = shown.screen.map(|(_, clip)| clip);
        let local = screen_clip.map(|clip| clip.local_us(edited_us).unwrap_or(clip.in_us));
        if let (Some(screen), Some(clip), Some(local)) = (screen, screen_clip, local) {
            let crop = layout.screen_crop_uv();
            let (center, zoom) = if short.follow_zooms {
                let zooms = self.zooms_for(&clip.asset);
                let config = crate::zoom::eval_config_for(&self.document.zooms);
                let camera = crate::zoom::evaluate_at_source(&zooms, local, &config);
                (
                    (camera.center_x as f32, camera.center_y as f32),
                    short.screen_zoom * camera.scale.max(1.0) as f32,
                )
            } else {
                (
                    (crop.0 + crop.2 / 2.0, crop.1 + crop.3 / 2.0),
                    short.screen_zoom,
                )
            };
            let ((x, y, w, h), (uv_x, uv_y, uv_w, uv_h)) = crate::shorts::screen_placement(
                screen.width,
                screen.height,
                crop,
                rects.screen,
                zoom,
                center,
                (short.screen_pan_x, short.screen_pan_y),
            );
            let mut layer = Layer::placed(screen, x, y, w, h).with_role(LayerRole::Screen);
            layer.uv_x = uv_x;
            layer.uv_y = uv_y;
            layer.uv_w = uv_w;
            layer.uv_h = uv_h;
            layers.push(layer);
        }
        if let (Some((x, y, w, h)), Some(webcam)) = (rects.camera, webcam) {
            let mut layer = Layer::placed(webcam, x, y, w, h)
                .with_role(LayerRole::Webcam)
                .cover_uv(w, h);
            if layout.webcam_mirror {
                layer = layer.mirrored();
            }
            layers.push(layer);
        }
        // The short's background (the video's own unless it chose another), behind everything.
        let background_layout = crate::shorts::background_layout(layout, short);
        if let Some(paper) = self.background()? {
            layers.insert(
                0,
                Layer::placed(paper, 0, 0, self.width, self.height)
                    .with_role(LayerRole::Background)
                    .cover_uv(self.width, self.height),
            );
        }
        let mut scene = Scene {
            width: self.width,
            height: self.height,
            background: background_layout.background_rgba()?.0,
            layers,
        };
        if let (Some(clip), Some(local)) = (screen_clip, local) {
            self.push_cursor(&mut scene, &clip.asset, local)?;
        }
        if let Some((frame, x, _)) = self.caption_at(edited_us) {
            let y = crate::shorts::caption_y(short.caption_spot, &rects, frame.height, self.height);
            scene.push_caption(frame, x, y);
        }
        Ok(scene)
    }

    /// The wallpaper or gradient, built once per evaluator. A short draws its own choice.
    fn background(&self) -> Result<Option<VideoFrame>, String> {
        Ok(match self.wallpaper.get() {
            Some(cached) => cached.clone(),
            None => {
                let layout = match &self.document.short_layout {
                    Some(short) => crate::shorts::background_layout(&self.document.layout, short),
                    None => self.document.layout.clone(),
                };
                let loaded =
                    crate::render::background_frame(&self.root, &layout, self.width, self.height)?;
                self.wallpaper.get_or_init(|| loaded).clone()
            }
        })
    }

    /// The zooms on a recording's clock, each following the mouse along it (unless fixed or
    /// following is off).
    fn zooms_for(&self, asset: &str) -> Zooms {
        if let Some(zooms) = self.zooms.borrow().get(asset) {
            return zooms.clone();
        }
        let settings = &self.document.zoom_settings;
        let samples = self
            .document
            .asset(asset)
            .filter(|a| a.is_recording() && settings.follow)
            .map(|a| crate::zoom::recording_cursor_samples(Path::new(&a.path)));
        let zooms: Vec<crate::zoom::ZoomSuggestion> = self
            .document
            .zooms
            .iter()
            .filter(|zoom| self.document.clock_asset(zoom.media.as_deref()) == Some(asset))
            .map(|zoom| {
                let mut suggestion = zoom.as_suggestion();
                if let Some(samples) = samples.as_ref().filter(|_| !zoom.fixed) {
                    suggestion.path = crate::zoom::follow_path(
                        samples,
                        &suggestion,
                        settings.follow_ms as u64 * 1_000,
                    );
                }
                suggestion
            })
            .collect();
        let zooms = std::sync::Arc::new(zooms);
        self.zooms
            .borrow_mut()
            .insert(asset.to_string(), zooms.clone());
        zooms
    }

    /// A recording's recorded pointer, read once.
    fn cursor_track(&self, asset: &str) -> Option<std::sync::Arc<crate::cursor::CursorTrack>> {
        if let Some(track) = self.cursors.borrow().get(asset) {
            return track.clone();
        }
        // A pointer that cannot be read is simply not drawn.
        let track = self
            .document
            .asset(asset)
            .filter(|a| a.is_recording())
            .and_then(|a| {
                crate::cursor::CursorTrack::load(Path::new(&a.path))
                    .ok()
                    .flatten()
            })
            .map(std::sync::Arc::new);
        self.cursors
            .borrow_mut()
            .insert(asset.to_string(), track.clone());
        track
    }

    /// Draws a recording's pointer at `source_us` on its clock over the screen.
    fn push_cursor(&self, scene: &mut Scene, asset: &str, source_us: u64) -> Result<(), String> {
        let layout = &self.document.layout;
        if !layout.cursor_visible {
            return Ok(());
        }
        let Some(track) = self.cursor_track(asset) else {
            return Ok(());
        };
        let Some(pose) = track.at(source_us) else {
            return Ok(());
        };
        let path = track.image_path(&pose.shape);
        let frame = {
            let mut images = self.cursor_images.borrow_mut();
            images
                .entry(path.clone())
                .or_insert_with(|| crate::media_bin::decode_image(&path).ok())
                .clone()
        };
        let Some(frame) = frame else {
            return Ok(());
        };
        let size = (layout.cursor_size_pct as f64 / 100.0) * pose.scale;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(&path, &mut hasher);
        scene.push_cursor(
            frame,
            (pose.x, pose.y),
            (pose.shape.hotspot_x * size, pose.shape.hotspot_y * size),
            (pose.shape.width * size, pose.shape.height * size),
            (track.source_width, track.source_height),
            std::hash::Hasher::finish(&hasher),
        );
        Ok(())
    }

    /// What to decode for a clip's picture at `local_us` on its asset's clock. In a
    /// recording's gap between segments the screen holds its last picture and the camera
    /// shows nothing; a missing file shows nothing.
    fn picture_job(&self, clip: &Clip, local_us: u64) -> Result<Option<PictureJob>, String> {
        use crate::sequence::sources::{stream_source, StreamSource};
        const MAX_CACHED_IMAGES: usize = 4;
        let Some(asset) = self.document.asset(&clip.asset) else {
            return Ok(None);
        };
        let source = match stream_source(&self.root, asset, &clip.stream) {
            Ok(source) => source,
            // A recording that is gone shows nothing rather than stopping playback.
            Err(_) => return Ok(None),
        };
        Ok(match source {
            StreamSource::Segments(segments) => {
                let containing = segments
                    .iter()
                    .find(|s| s.available && s.start_us <= local_us && local_us < s.end_us)
                    .map(|s| (s, local_us));
                let screen = asset
                    .stream(&clip.stream)
                    .is_some_and(|s| s.role == Role::Screen);
                let job = containing.or_else(|| {
                    screen
                        .then(|| {
                            segments
                                .iter()
                                .rev()
                                .find(|s| s.available && s.end_us <= local_us)
                                .map(|s| (s, s.end_us.saturating_sub(1)))
                        })
                        .flatten()
                });
                job.map(|(segment, time)| PictureJob::Segment(segment.clone(), time))
            }
            // A file moved or deleted since import shows nothing.
            StreamSource::Video(path) => {
                path.is_file().then_some(PictureJob::Video(path, local_us))
            }
            StreamSource::Image(path) => {
                if !path.is_file() {
                    return Ok(None);
                }
                let mut cache = self.image_cache.borrow_mut();
                let frame = match cache.iter().position(|(id, _)| *id == clip.asset) {
                    Some(index) => cache.remove(index).1,
                    None => crate::media_bin::decode_image(&path)?,
                };
                cache.push((clip.asset.clone(), frame.clone()));
                if cache.len() > MAX_CACHED_IMAGES {
                    cache.remove(0);
                }
                Some(PictureJob::Ready(frame))
            }
        })
    }

    /// The transcript captions read from: the chosen sound, else the first speech on the
    /// timeline, else any speech in the project.
    fn caption_transcript(&self) -> Option<crate::transcript::Transcript> {
        let load = |key: &str| {
            crate::transcript::store::load_transcript(&self.root, key)
                .ok()
                .flatten()
        };
        let candidates = caption_candidates(&self.document);
        crate::captions::caption_source(&self.document.captions, candidates, Vec::new(), load)
    }

    fn caption_at(&self, edited_us: u64) -> Option<(VideoFrame, u32, u32)> {
        let settings = &self.document.captions;
        if !settings.enabled {
            return None;
        }
        let cues = self.caption_cues.get_or_init(|| {
            self.caption_transcript()
                .and_then(|t| {
                    let clock = self.document.transcript_clock(&t.track_id).ok()?;
                    Some(crate::captions::build_cues(&t, &clock, settings))
                })
                .unwrap_or_default()
        });
        let index = crate::captions::cue_at(cues, edited_us)?;
        let cue = &cues[index];
        let active = crate::captions::active_word(cue, edited_us);
        let mut cache = self.caption_cache.borrow_mut();
        let frame = match &cache.frame {
            Some((key, frame)) if *key == (index, active) => frame.clone(),
            _ => {
                if cache.raster.as_ref().map(|(i, _)| *i) != Some(index) {
                    let raster =
                        crate::captions::rasterize_cue(cue, settings, self.width, self.height);
                    cache.raster = Some((index, raster));
                }
                let raster = cache.raster.as_ref()?.1.as_ref()?;
                let frame = crate::captions::colorize(raster, settings, active);
                cache.frame = Some(((index, active), frame.clone()));
                frame
            }
        };
        let (x, y) = crate::captions::placement(
            settings,
            self.width,
            self.height,
            frame.width,
            frame.height,
        );
        Some((frame, x, y))
    }
}

/// The sounds captions may read, best first: speech on the timeline (top audio track first,
/// earliest clip first), then the rest of the project's speech.
pub fn caption_candidates(document: &EditDocument) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |key: String| {
        if !out.contains(&key) {
            out.push(key);
        }
    };
    for track in document.sequence.audio_tracks() {
        for clip in &track.clips {
            if crate::sequence::clip_role(&document.assets, track, clip) == Some(Role::Mic) {
                push(clip.source().key());
            }
        }
    }
    for asset in &document.assets {
        for stream in asset.streams.iter().filter(|s| s.role == Role::Mic) {
            push(crate::sequence::StreamRef::new(&asset.id, &stream.id).key());
        }
    }
    out
}

/// Sources larger than twice the output are shrunk by FFmpeg's filtered scaler before
/// compositing; the compositor's bilinear sampling aliases at bigger reductions. Twice the
/// output keeps full detail for smart zoom up to 2x.
fn export_decode_limit(width: u32, height: u32) -> DecodeLimit {
    DecodeLimit {
        max_width: width.saturating_mul(2),
        max_height: height.saturating_mul(2),
        max_rate: 0,
    }
}

fn decode_layer(
    root: &Path,
    segment: &SegmentSummary,
    source_us: u64,
    limit: DecodeLimit,
) -> Result<VideoFrame, String> {
    let path = safe_path(root, &segment.relative_path)?;
    // Each independently written segment has an AVAsset timeline beginning at zero.
    // Journal source/host/media anchors describe recording time, not the asset time.
    let local_us = source_us
        .checked_sub(segment.start_us)
        .ok_or("Invalid segment timestamp")?;
    decode_h264_frame_limited(&path, local_us, limit).map_err(|error| {
        format!(
            "{} at local {}us: {}",
            segment.relative_path, local_us, error
        )
    })
}

/// The H.264/AAC writer for whichever media backend is active.
enum ExportWriter {
    Native(NativeExport),
    Ffmpeg(FfmpegExport),
}

impl ExportWriter {
    fn begin(
        path: &Path,
        width: u32,
        height: u32,
        fps: u32,
        sample_rate: u32,
        channels: u16,
        rate: RateControl,
    ) -> Result<Self, String> {
        match media_backend() {
            MediaBackend::Native => {
                NativeExport::begin(path, width, height, fps, sample_rate, channels, rate)
                    .map(Self::Native)
            }
            MediaBackend::Ffmpeg => {
                FfmpegExport::begin_with_rate(path, width, height, fps, sample_rate, channels, rate)
                    .map(Self::Ffmpeg)
            }
        }
    }

    fn write_video(&mut self, pts_us: u64, frame: &VideoFrame) -> Result<(), String> {
        match self {
            Self::Native(session) => session.write_video(pts_us, frame),
            Self::Ffmpeg(session) => session.write_video(frame),
        }
    }

    fn write_audio(
        &mut self,
        pts_us: u64,
        pcm: &[i16],
        frames: u32,
        channels: u16,
    ) -> Result<(), String> {
        match self {
            Self::Native(session) => session.write_audio(pts_us, pcm, frames, channels),
            Self::Ffmpeg(session) => {
                let expected = frames as usize * channels.max(1) as usize;
                let samples = pcm
                    .get(..expected)
                    .ok_or("Export audio buffer is truncated")?;
                session.write_audio(samples)
            }
        }
    }

    /// Called once the last audio has been written.
    fn end_audio(&mut self) -> Result<(), String> {
        match self {
            Self::Native(session) => session.end_audio(),
            Self::Ffmpeg(_) => Ok(()),
        }
    }

    fn finish(self, duration_us: u64) -> Result<(), String> {
        match self {
            Self::Native(session) => session.finish(duration_us),
            Self::Ffmpeg(session) => session.finish(),
        }
    }
}

pub fn validate_settings(settings: &ExportSettings) -> Result<(), ExportFailure> {
    let codec = settings.video_codec.to_ascii_lowercase();
    if codec != "h264" {
        return Err(ExportFailure::InvalidSettings {
            message: format!("Unsupported video codec: {}", settings.video_codec),
        });
    }
    if settings.audio_codec.to_ascii_lowercase() != "aac" {
        return Err(ExportFailure::InvalidSettings {
            message: format!("Unsupported audio codec: {}", settings.audio_codec),
        });
    }
    if !ALLOWED_FPS.contains(&settings.fps) {
        return Err(ExportFailure::InvalidSettings {
            message: format!("Unsupported export frame rate: {}", settings.fps),
        });
    }
    if let Some(kbps) = settings.bitrate_kbps {
        if !(MIN_BITRATE_KBPS..=MAX_BITRATE_KBPS).contains(&kbps) {
            return Err(ExportFailure::InvalidSettings {
                message: format!(
                    "Export bitrate must be {}..={} Mbps",
                    MIN_BITRATE_KBPS / 1_000,
                    MAX_BITRATE_KBPS / 1_000
                ),
            });
        }
    }
    if settings.width < 16
        || settings.height < 16
        || settings.width > MAX_FRAME_DIM
        || settings.height > MAX_FRAME_DIM
        || settings.width % 2 != 0
        || settings.height % 2 != 0
    {
        return Err(ExportFailure::InvalidSettings {
            message: "Export canvas must be even, 16..=4096".into(),
        });
    }
    Ok(())
}

pub fn default_export_filename(project_name: &str) -> String {
    let mut out = String::new();
    for ch in project_name.chars() {
        if ch.is_control() || matches!(ch, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
            continue;
        }
        out.push(ch);
    }
    let out = out.trim().trim_matches('.').to_string();
    let stem = if out.is_empty() || out == "." || out == ".." {
        "Untitled"
    } else if let Some(stripped) = out
        .strip_suffix(".mp4")
        .or_else(|| out.strip_suffix(".MP4"))
    {
        stripped
    } else {
        &out
    };
    format!("{stem}.mp4")
}

pub fn is_inside_bundle(path: &Path, bundle_root: Option<&Path>) -> bool {
    if let Some(bundle) = bundle_root {
        if let Ok(canonical_bundle) = dunce::canonicalize(bundle) {
            if let Some(parent) = path.parent() {
                if let Ok(canonical_parent) = dunce::canonicalize(parent) {
                    if canonical_parent.starts_with(&canonical_bundle) {
                        return true;
                    }
                }
            }
            if let Ok(canonical_path) = dunce::canonicalize(path) {
                if canonical_path.starts_with(&canonical_bundle) {
                    return true;
                }
            }
        }
        if path.starts_with(bundle) {
            return true;
        }
        if let Some(parent) = path.parent() {
            if parent.starts_with(bundle) {
                return true;
            }
        }
    }

    if let Some(parent) = path.parent() {
        for component in parent.components() {
            let name = component.as_os_str().to_string_lossy();
            if name.to_ascii_lowercase().ends_with(".aero") {
                return true;
            }
        }
    }

    if path
        .to_string_lossy()
        .to_ascii_lowercase()
        .ends_with(".aero")
    {
        return true;
    }

    false
}

pub fn default_destination(project_root: &Path, project_name: &str, _revision: u64) -> PathBuf {
    let parent = project_root.parent().unwrap_or(project_root);
    parent.join(default_export_filename(project_name))
}

pub fn resolve_destination(
    project_root: &Path,
    project_name: &str,
    revision: u64,
    requested: Option<&str>,
    sources: &[Asset],
) -> Result<PathBuf, ExportFailure> {
    let dest = match requested {
        Some(path) if !path.trim().is_empty() => PathBuf::from(path),
        _ => default_destination(project_root, project_name, revision),
    };
    let project_root = dunce::canonicalize(project_root).map_err(|e| ExportFailure::Io {
        message: e.to_string(),
    })?;
    if dest.as_os_str().is_empty() {
        return Err(ExportFailure::InvalidSettings {
            message: "Export destination is empty".into(),
        });
    }
    let parent = dest.parent().ok_or_else(|| ExportFailure::Io {
        message: "Export destination has no parent directory".into(),
    })?;
    if !parent.exists() {
        return Err(ExportFailure::Io {
            message: "Export destination directory does not exist".into(),
        });
    }
    if parent.is_file() {
        return Err(ExportFailure::Io {
            message: "Export destination parent is not a directory".into(),
        });
    }
    let canonical_parent = dunce::canonicalize(parent).map_err(|e| ExportFailure::Io {
        message: e.to_string(),
    })?;
    let file_name = dest.file_name().ok_or_else(|| ExportFailure::Io {
        message: "Export destination is missing a file name".into(),
    })?;
    let resolved = canonical_parent.join(file_name);
    if resolved.starts_with(&project_root) || is_inside_bundle(&resolved, Some(&project_root)) {
        return Err(ExportFailure::SourcePath {
            message: "Export destination cannot be inside the project bundle".into(),
        });
    }
    // Never over the media it is made from, nor inside a recording.
    for asset in sources {
        let source = PathBuf::from(&asset.path);
        let canonical = dunce::canonicalize(&source).unwrap_or(source);
        let clash = if asset.is_recording() {
            resolved.starts_with(&canonical)
        } else {
            resolved == canonical
        };
        if clash {
            return Err(ExportFailure::SourcePath {
                message: "Export destination cannot overwrite the media it is made from".into(),
            });
        }
    }
    Ok(resolved)
}

pub fn frame_count(duration_us: u64, fps: u32) -> Result<u32, ExportFailure> {
    if duration_us == 0 {
        return Err(ExportFailure::InvalidSettings {
            message: "Edited timeline is empty".into(),
        });
    }
    if !ALLOWED_FPS.contains(&fps) {
        return Err(ExportFailure::InvalidSettings {
            message: "Unsupported frame rate".into(),
        });
    }
    let count =
        u32::try_from((duration_us as u128 * fps as u128).div_ceil(1_000_000)).map_err(|_| {
            ExportFailure::InvalidSettings {
                message: "Timeline exceeds output timestamp bounds".into(),
            }
        })?;
    Ok(count)
}

pub fn frame_time_us(index: u32, fps: u32) -> u64 {
    (index as u128 * 1_000_000 / fps as u128) as u64
}

fn status_from(
    job_id: &str,
    document: &EditDocument,
    settings: &ExportSettings,
    state: ExportState,
    failure: Option<ExportFailure>,
) -> ExportStatus {
    ExportStatus {
        job_id: job_id.into(),
        state,
        captured_revision: document.revision,
        video_codec: settings.video_codec.clone(),
        audio_codec: settings.audio_codec.clone(),
        width: settings.width,
        height: settings.height,
        fps: settings.fps,
        progress_numerator: 0,
        progress_denominator: 0,
        output_path: None,
        failure,
        diagnostics: Vec::new(),
    }
}

pub fn prepare_job(
    root: &Path,
    project_name: &str,
    document: EditDocument,
    settings: ExportSettings,
    owner: &mut ExportOwner,
) -> Result<CapturedExport, ExportStatus> {
    let job_id = Uuid::new_v4().to_string();
    if owner.busy() {
        return Err(status_from(
            &job_id,
            &document,
            &settings,
            ExportState::Failed,
            Some(ExportFailure::EncoderBusy {
                message: "An export job is already running".into(),
            }),
        ));
    }
    if let Err(failure) = validate_settings(&settings) {
        return Err(status_from(
            &job_id,
            &document,
            &settings,
            ExportState::Failed,
            Some(failure),
        ));
    }
    let mut settings = settings;
    match document
        .layout
        .fit_export_size(settings.width, settings.height)
    {
        Ok((width, height)) => {
            settings.width = width;
            settings.height = height;
        }
        Err(message) => {
            return Err(status_from(
                &job_id,
                &document,
                &settings,
                ExportState::Failed,
                Some(ExportFailure::InvalidSettings { message }),
            ));
        }
    }
    let dest = match resolve_destination(
        root,
        project_name,
        document.revision,
        settings.destination.as_deref(),
        &document.assets,
    ) {
        Ok(path) => path,
        Err(failure) => {
            return Err(status_from(
                &job_id,
                &document,
                &settings,
                ExportState::Failed,
                Some(failure),
            ));
        }
    };
    if dest.exists() {
        return Err(status_from(
            &job_id,
            &document,
            &settings,
            ExportState::Failed,
            Some(ExportFailure::Collision {
                message: "Export destination already exists".into(),
            }),
        ));
    }
    let duration = document.duration_us();
    if let Err(failure) = frame_count(duration, settings.fps) {
        return Err(status_from(
            &job_id,
            &document,
            &settings,
            ExportState::Failed,
            Some(failure),
        ));
    }
    let temp = dest.with_file_name(format!(
        ".{}-aeroedits-partial-{}.mp4",
        dest.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("export"),
        &job_id[..8.min(job_id.len())]
    ));
    let lease = crate::project::reader::acquire_read_lease(root).map_err(|message| {
        status_from(
            &job_id,
            &document,
            &settings,
            ExportState::Failed,
            Some(ExportFailure::Io { message }),
        )
    })?;
    Ok(CapturedExport {
        lease: Arc::new(lease),
        job_id,
        root: root.to_path_buf(),
        document,
        dest,
        temp,
        settings,
    })
}

pub fn spawn_job(
    captured: CapturedExport,
    owner: &mut ExportOwner,
    gate: Arc<EncoderGate>,
) -> ExportStatus {
    let cancel = Arc::new(AtomicBool::new(false));
    let status = Arc::new(Mutex::new(status_from(
        &captured.job_id,
        &captured.document,
        &captured.settings,
        ExportState::Queued,
        None,
    )));
    let status_clone = Arc::clone(&status);
    let cancel_clone = Arc::clone(&cancel);
    let id = captured.job_id.clone();
    let join = std::thread::spawn(move || {
        run_job(captured, cancel_clone, status_clone, &gate);
    });
    owner.job = Some(LiveJob {
        id,
        cancel,
        status: Arc::clone(&status),
        join: Some(join),
    });
    let snapshot = status.lock().clone();
    snapshot
}

pub fn run_job(
    captured: CapturedExport,
    cancel: Arc<AtomicBool>,
    status: Arc<Mutex<ExportStatus>>,
    gate: &EncoderGate,
) {
    {
        let mut slot = status.lock();
        slot.state = ExportState::Running;
    }
    match run_export(
        &captured,
        &cancel,
        |done, total| {
            let mut slot = status.lock();
            slot.progress_numerator = done;
            slot.progress_denominator = total;
            slot.state = ExportState::Running;
        },
        gate,
    ) {
        Ok(path) => {
            let mut slot = status.lock();
            slot.state = ExportState::Completed;
            slot.output_path = Some(path.to_string_lossy().into());
            slot.progress_numerator = slot.progress_denominator.max(1);
            slot.failure = None;
        }
        Err(failure) => {
            let cancelled = matches!(failure, ExportFailure::Cancelled { .. });
            let mut slot = status.lock();
            slot.state = if cancelled {
                ExportState::Cancelled
            } else {
                ExportState::Failed
            };
            slot.failure = Some(failure);
            slot.output_path = None;
        }
    }
}

struct TempGuard {
    path: PathBuf,
    keep: bool,
}

impl Drop for TempGuard {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_file(&self.path);
        }
    }
}

pub fn run_export(
    captured: &CapturedExport,
    cancel: &AtomicBool,
    on_progress: impl FnMut(u32, u32),
    gate: &EncoderGate,
) -> Result<PathBuf, ExportFailure> {
    let result = export_to_temp(captured, cancel, on_progress, gate);
    // Decoder processes opened for the export are not needed once it ends.
    crate::media::release_decoders();
    result
}

fn export_to_temp(
    captured: &CapturedExport,
    cancel: &AtomicBool,
    mut on_progress: impl FnMut(u32, u32),
    gate: &EncoderGate,
) -> Result<PathBuf, ExportFailure> {
    let _slot = gate
        .try_acquire()
        .map_err(|message| ExportFailure::EncoderBusy { message })?;
    if cancel.load(Ordering::SeqCst) {
        return Err(ExportFailure::Cancelled {
            message: "Export cancelled".into(),
        });
    }
    if captured.dest.exists() {
        return Err(ExportFailure::Collision {
            message: "Export destination already exists".into(),
        });
    }
    if fs::symlink_metadata(&captured.temp).is_ok() {
        return Err(ExportFailure::Collision {
            message: "Temporary export path exists".into(),
        });
    }
    let mut temp = TempGuard {
        path: captured.temp.clone(),
        keep: false,
    };
    let duration_us = captured.document.duration_us();
    let frames = frame_count(duration_us, captured.settings.fps)?;
    let mixer = AudioMixer::new(&captured.root, &captured.document)
        .map_err(|message| ExportFailure::Io { message })?;
    let (sample_rate, channels) = if mixer.has_audio() {
        (SAMPLE_RATE, CHANNELS)
    } else {
        (0, 0)
    };
    let mut session = ExportWriter::begin(
        &captured.temp,
        captured.settings.width,
        captured.settings.height,
        captured.settings.fps,
        sample_rate,
        channels,
        captured.settings.rate_control(),
    )
    .map_err(|message| ExportFailure::Native { message })?;
    let mut evaluator = SceneEvaluator::new(
        captured.root.clone(),
        captured.document.clone(),
        captured.settings.width,
        captured.settings.height,
    )
    .map_err(|message| ExportFailure::Native { message })?
    .with_decode_limit(export_decode_limit(
        captured.settings.width,
        captured.settings.height,
    ));

    let mut audio_frame = 0u64;
    let mut audio_ended = channels == 0;
    for index in 0..frames {
        if cancel.load(Ordering::SeqCst) {
            drop(session);
            return Err(ExportFailure::Cancelled {
                message: "Export cancelled".into(),
            });
        }
        let pts_us = frame_time_us(index, captured.settings.fps);
        if pts_us >= duration_us {
            break;
        }
        let frame = evaluator
            .preview_at(pts_us)
            .map_err(|message| ExportFailure::Native { message })?;
        session
            .write_video(pts_us, &frame)
            .map_err(|message| ExportFailure::Native { message })?;
        if channels > 0 {
            // Audio runs a second ahead of video: the macOS writer interleaves its
            // tracks and can hold video back until it has audio past that point.
            let end = ((index as u128 + 1) * SAMPLE_RATE as u128 / captured.settings.fps as u128)
                as u64
                + SAMPLE_RATE as u64;
            let end = end.min(mixer.total_frames);
            while audio_frame < end {
                if cancel.load(Ordering::SeqCst) {
                    return Err(ExportFailure::Cancelled {
                        message: "Export cancelled".into(),
                    });
                }
                let count = CHUNK_FRAMES.min((end - audio_frame) as usize);
                let chunk = mixer
                    .read_frames(audio_frame, count)
                    .map_err(|message| ExportFailure::Io { message })?;
                let pts = (audio_frame as u128 * 1_000_000 / SAMPLE_RATE as u128) as u64;
                session
                    .write_audio(pts, &chunk, count as u32, CHANNELS)
                    .map_err(|message| ExportFailure::Native { message })?;
                audio_frame += count as u64;
            }
            if !audio_ended && audio_frame >= mixer.total_frames {
                session
                    .end_audio()
                    .map_err(|message| ExportFailure::Native { message })?;
                audio_ended = true;
            }
        }
        on_progress(index + 1, frames);
    }
    session
        .finish(duration_us)
        .map_err(|message| ExportFailure::Native { message })?;
    if cancel.load(Ordering::SeqCst) {
        return Err(ExportFailure::Cancelled {
            message: "Export cancelled".into(),
        });
    }
    let actual_duration =
        media_duration_us(&captured.temp).map_err(|message| ExportFailure::Native { message })?;
    if actual_duration.abs_diff(duration_us) > AUDIO_DURATION_SLACK_US {
        return Err(ExportFailure::Native {
            message: format!(
                "Output duration {} differs from {}",
                actual_duration, duration_us
            ),
        });
    }
    for pts in [0, frame_time_us(frames - 1, captured.settings.fps)] {
        let decoded = decode_h264_frame(&captured.temp, pts)
            .map_err(|message| ExportFailure::Native { message })?;
        if (decoded.width, decoded.height) != (captured.settings.width, captured.settings.height) {
            return Err(ExportFailure::Native {
                message: "Output dimensions do not match settings".into(),
            });
        }
    }
    if cancel.load(Ordering::SeqCst) {
        return Err(ExportFailure::Cancelled {
            message: "Export cancelled".into(),
        });
    }
    // Windows cannot rename or delete the temp file while a decoder still has it open.
    crate::media::release_decoders();
    write_chapters(&captured.temp, &captured.document, duration_us)
        .map_err(|message| ExportFailure::Native { message })?;
    publish_output(&captured.temp, &captured.dest)?;
    temp.keep = true;
    Ok(captured.dest.clone())
}

/// Adds the document's chapters to the finished file, in place. Nothing to do without any.
fn write_chapters(temp: &Path, document: &EditDocument, duration_us: u64) -> Result<(), String> {
    let timeline = crate::chapters::timeline(&document.chapters, document);
    if timeline.is_empty() {
        return Ok(());
    }
    let side = |suffix: &str| {
        let mut name = temp.as_os_str().to_owned();
        name.push(suffix);
        PathBuf::from(name)
    };
    let metadata = side(".chapters.txt");
    let with_chapters = side(".chapters.mp4");
    let result = (|| {
        fs::write(
            &metadata,
            crate::chapters::ffmetadata(&timeline, duration_us),
        )
        .map_err(|e| e.to_string())?;
        crate::media::ffmpeg::add_chapters(temp, &metadata, &with_chapters)?;
        fs::rename(&with_chapters, temp).map_err(|e| e.to_string())
    })();
    let _ = fs::remove_file(&metadata);
    let _ = fs::remove_file(&with_chapters);
    result
}

fn publish_output(temp: &Path, dest: &Path) -> Result<(), ExportFailure> {
    if dest.exists() {
        let _ = fs::remove_file(temp);
        return Err(ExportFailure::Collision {
            message: "Export destination already exists".into(),
        });
    }
    {
        // Windows only flushes a handle opened for writing; a read-only one is "Access is denied".
        let opened = fs::OpenOptions::new().write(true).open(temp);
        let file = opened.map_err(|e| ExportFailure::Io {
            message: e.to_string(),
        })?;
        file.sync_all().map_err(|e| ExportFailure::Io {
            message: e.to_string(),
        })?;
    }
    match fs::hard_link(temp, dest) {
        Ok(()) => {
            let _ = fs::remove_file(temp);
            Ok(())
        }
        Err(_) if dest.exists() => {
            let _ = fs::remove_file(temp);
            Err(ExportFailure::Collision {
                message: "Export destination already exists".into(),
            })
        }
        Err(_err) => {
            if dest.exists() {
                let _ = fs::remove_file(temp);
                return Err(ExportFailure::Collision {
                    message: "Export destination already exists".into(),
                });
            }
            #[cfg(target_os = "macos")]
            {
                use std::os::unix::ffi::OsStrExt;
                let from = std::ffi::CString::new(temp.as_os_str().as_bytes()).map_err(|e| {
                    ExportFailure::Io {
                        message: e.to_string(),
                    }
                })?;
                let to = std::ffi::CString::new(dest.as_os_str().as_bytes()).map_err(|e| {
                    ExportFailure::Io {
                        message: e.to_string(),
                    }
                })?;
                if unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_EXCL) } == 0 {
                    return Ok(());
                }
                let error = std::io::Error::last_os_error();
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    return Err(ExportFailure::Collision {
                        message: "Export destination already exists".into(),
                    });
                }
                Err(ExportFailure::Io {
                    message: error.to_string(),
                })
            }
            #[cfg(not(target_os = "macos"))]
            Err(ExportFailure::Io {
                message: _err.to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_prores_and_odd_dimensions() {
        let mut settings = ExportSettings::default();
        settings.video_codec = "prores".into();
        assert!(matches!(
            validate_settings(&settings),
            Err(ExportFailure::InvalidSettings { .. })
        ));
        settings.video_codec = "h264".into();
        settings.width = 63;
        assert!(matches!(
            validate_settings(&settings),
            Err(ExportFailure::InvalidSettings { .. })
        ));
    }

    #[test]
    fn quality_defaults_to_high_and_bitrate_is_bounded() {
        let settings: ExportSettings = serde_json::from_value(serde_json::json!({
            "videoCodec": "h264", "audioCodec": "aac", "width": 1920, "height": 1080, "fps": 30
        }))
        .unwrap();
        assert_eq!(
            settings.rate_control(),
            RateControl::Quality(VideoQuality::High)
        );
        let custom: ExportSettings = serde_json::from_value(serde_json::json!({
            "videoCodec": "h264", "audioCodec": "aac", "width": 1920, "height": 1080, "fps": 60,
            "quality": "max", "bitrateKbps": 24000
        }))
        .unwrap();
        assert_eq!(custom.rate_control(), RateControl::Bitrate(24_000_000));
        assert!(validate_settings(&custom).is_ok());
        for kbps in [MIN_BITRATE_KBPS - 1, MAX_BITRATE_KBPS + 1] {
            let settings = ExportSettings {
                bitrate_kbps: Some(kbps),
                ..ExportSettings::default()
            };
            assert!(matches!(
                validate_settings(&settings),
                Err(ExportFailure::InvalidSettings { .. })
            ));
        }
    }

    #[test]
    fn export_decode_limit_is_twice_the_output() {
        let limit = export_decode_limit(1920, 1080);
        assert_eq!(
            (limit.max_width, limit.max_height, limit.max_rate),
            (3840, 2160, 0)
        );
    }

    #[test]
    fn exclusive_end_is_not_an_output_sample() {
        assert_eq!(frame_count(200_000, 10).unwrap(), 2);
        assert_eq!(frame_count(250_000, 10).unwrap(), 3);
        assert_eq!(frame_count(1, 30).unwrap(), 1);
        assert!(frame_count(u64::MAX, 60).is_err());
        assert_eq!(frame_time_us(0, 10), 0);
        assert_eq!(frame_time_us(1, 10), 100_000);
        assert!(frame_time_us(2, 10) >= 200_000);
    }

    /// A `.aero` recording with two seconds of 64x64 screen video, whose grey level steps up
    /// every 100ms, and a mic track. Returns the recording folder.
    fn screen_and_mic_project(dir: &Path) -> PathBuf {
        project_with_tracks(dir, false)
    }

    /// The screen (grey ramp) and mic recording, plus a solid red webcam when `webcam` is set.
    fn project_with_tracks(dir: &Path, webcam: bool) -> PathBuf {
        use crate::fixtures::{generate_pcm16_wav, TestProject};
        use crate::project::manifest::{TrackDescriptor, TrackType};
        use crate::project::JournalRecord;
        let mut bundle = TestProject::create(dir, "export");
        let root = bundle.root_path().to_path_buf();
        let frames: Vec<VideoFrame> = (0..20u8)
            .map(|i| VideoFrame::solid(64, 64, 10 + i * 12, 10 + i * 12, 10 + i * 12, 0).unwrap())
            .collect();
        let screen_path = root.join("media/screen/000001.mp4");
        crate::media::ffmpeg::encode_bgra_mp4(&screen_path, &frames, 10).unwrap();
        let wav = generate_pcm16_wav(48_000, 1, &vec![8_000i16; 96_000]);
        fs::write(root.join("media/mic/000001.wav"), &wav).unwrap();
        let mut tracks = vec![(
            "screen",
            TrackType::Screen,
            "h264",
            "media/screen/000001.mp4",
            10,
        )];
        if webcam {
            let red: Vec<VideoFrame> = (0..20)
                .map(|_| VideoFrame::solid(64, 64, 0, 0, 255, 0).unwrap())
                .collect();
            let path = root.join("media/webcam/000001.mp4");
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            crate::media::ffmpeg::encode_bgra_mp4(&path, &red, 10).unwrap();
            tracks.push((
                "webcam",
                TrackType::Webcam,
                "h264",
                "media/webcam/000001.mp4",
                10,
            ));
        }
        tracks.push((
            "mic",
            TrackType::MicAudio,
            "pcm",
            "media/mic/000001.wav",
            48_000,
        ));
        for (id, track_type, codec, path, timescale) in tracks {
            let audio = track_type == TrackType::MicAudio;
            bundle.manifest_mut().tracks.push(TrackDescriptor {
                id: id.into(),
                track_type,
                codec: codec.into(),
                relative_path: path.into(),
                width: (!audio).then_some(64),
                height: (!audio).then_some(64),
                fps: (!audio).then_some(10),
                sample_rate: audio.then_some(48_000),
                channels: audio.then_some(1),
                gaps_total: 0,
                media_timescale: Some(timescale),
            });
            bundle.append_journal(JournalRecord::SegmentCommitted {
                seq: 0,
                track_id: id.into(),
                relative_path: path.into(),
                start_us: 0,
                end_us: 2_000_000,
                size_bytes: fs::metadata(root.join(path)).unwrap().len(),
                is_keyframe_start: true,
                media_timescale: timescale,
                media_start_value: 0,
                host_anchor_us: 0,
            });
        }
        bundle.manifest_mut().duration_us = 2_000_000;
        bundle.manifest_mut().active_duration_us = 2_000_000;
        bundle.save_manifest();
        drop(bundle);

        root
    }

    /// A recording opened as a new project, the way the app makes one.
    fn opened(recording: &Path) -> (PathBuf, crate::project::ProjectReader) {
        let parent = recording.parent().unwrap().join("projects");
        let folder =
            crate::project::folder::create_project_folder(&parent, "Edit", Some(recording))
                .unwrap();
        let reader = crate::project::ProjectReader::open(&folder).unwrap();
        (folder, reader)
    }

    fn edit(reader: &mut crate::project::ProjectReader, edit: crate::sequence::edit::SequenceEdit) {
        let revision = reader.summary.revision;
        reader
            .edit_sequence(revision, &edit, None)
            .unwrap_or_else(|e| panic!("{edit:?}: {e}"));
    }

    /// Source 0.5 s to 1.2 s cut out of every track.
    fn cut_project(dir: &Path) -> (PathBuf, crate::project::ProjectReader) {
        let (folder, mut reader) = opened(&screen_and_mic_project(dir));
        reader.ripple_cuts(0, &[(500_000, 1_200_000)]).unwrap();
        (folder, reader)
    }

    /// The source 1.2 s to 2.0 s set first, then 0 to 0.5 s.
    fn reordered_project(dir: &Path) -> (PathBuf, crate::project::ProjectReader) {
        let (folder, mut reader) = cut_project(dir);
        let second = reader.summary.sequence.tracks[0].clips[1].id.clone();
        edit(
            &mut reader,
            crate::sequence::edit::SequenceEdit::MoveClips {
                clip_ids: vec![second],
                delta_us: -500_000,
                track_id: None,
                anchor_id: None,
            },
        );
        (folder, reader)
    }

    fn track_id(reader: &crate::project::ProjectReader, number: &str) -> String {
        let sequence = &reader.summary.sequence;
        sequence
            .tracks
            .iter()
            .find(|t| sequence.number(&t.id).as_deref() == Some(number))
            .unwrap()
            .id
            .clone()
    }

    fn red_video_with_tone(path: &Path, seconds: f64) {
        let status = std::process::Command::new(crate::media::ffmpeg::ffmpeg_path().unwrap())
            .args(["-v", "error", "-y", "-f", "lavfi", "-i"])
            .arg(format!("color=c=red:s=64x36:r=30:d={seconds}"))
            .args(["-f", "lavfi", "-i"])
            .arg(format!(
                "sine=frequency=440:sample_rate=48000:duration={seconds}"
            ))
            .args([
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
                "-shortest",
            ])
            .arg(path)
            .status()
            .unwrap();
        assert!(status.success());
    }

    fn blue_png(path: &Path) {
        image::RgbaImage::from_pixel(32, 18, image::Rgba([0, 0, 255, 255]))
            .save(path)
            .unwrap();
    }

    fn small() -> ExportSettings {
        ExportSettings {
            width: 320,
            height: 180,
            fps: 30,
            ..ExportSettings::default()
        }
    }

    fn export(root: &Path, document: EditDocument, settings: ExportSettings) -> PathBuf {
        let mut owner = ExportOwner::new();
        let captured = prepare_job(root, "export", document, settings, &mut owner)
            .unwrap_or_else(|status| panic!("prepare failed: {:?}", status.failure));
        run_export(
            &captured,
            &AtomicBool::new(false),
            |_, _| {},
            &EncoderGate::new(),
        )
        .unwrap_or_else(|failure| panic!("export failed: {failure:?}"))
    }

    /// Video tracks over the recording: a higher track draws over a lower one, a hidden track
    /// draws nothing, and a clip's sound plays unless its track is muted.
    #[test]
    #[cfg_attr(
        not(target_os = "macos"),
        ignore = "needs a GPU adapter and FFmpeg; run with --ignored on a machine that has them"
    )]
    fn gpu_overlay_tracks_draw_in_order_and_play_their_sound() {
        use crate::sequence::edit::SequenceEdit;
        use crate::sequence::{SourceRange, TrackKind};
        let dir = tempfile::tempdir().unwrap();
        let (root, mut reader) = opened(&screen_and_mic_project(dir.path()));
        let png = dir.path().join("blue.png");
        blue_png(&png);
        let clip = dir.path().join("red.mp4");
        red_video_with_tone(&clip, 1.0);
        let summary = reader.import_media(0, &[png, clip]).unwrap();
        let (image_id, video_id) = (summary.assets[1].id.clone(), summary.assets[2].id.clone());
        edit(
            &mut reader,
            SequenceEdit::AddTrack {
                track_kind: TrackKind::Video,
            },
        );
        edit(
            &mut reader,
            SequenceEdit::AddTrack {
                track_kind: TrackKind::Video,
            },
        );
        // The red video on V2 from 0.2 s, the blue image on V3 from 0.6 s to 1.4 s.
        let (v2, v3) = (track_id(&reader, "V2"), track_id(&reader, "V3"));
        edit(
            &mut reader,
            SequenceEdit::PlaceAsset {
                asset_id: video_id.clone(),
                at_us: 200_000,
                track_id: Some(v2),
                streams: vec![],
                range: None,
            },
        );
        edit(
            &mut reader,
            SequenceEdit::PlaceAsset {
                asset_id: image_id,
                at_us: 600_000,
                track_id: Some(v3.clone()),
                streams: vec![],
                range: Some(SourceRange {
                    start_us: 0,
                    end_us: 800_000,
                }),
            },
        );
        // Nothing was pushed along: 2 s of recording.
        assert_eq!(reader.summary.duration_us, 2_000_000);
        let mut document = reader.history().current.clone();
        if std::env::var("NOCURSOR").is_ok() {
            document.layout.cursor_visible = false;
        }
        let colour = |document: &EditDocument, t: u64| {
            let mut evaluator =
                SceneEvaluator::new(root.clone(), document.clone(), 320, 180).unwrap();
            let frame = evaluator.preview_at(t).unwrap();
            let i = ((90 * frame.stride) + 160 * 4) as usize;
            (frame.data[i], frame.data[i + 2])
        };
        let (b, r) = colour(&document, 100_000);
        assert!(b < 120 && r < 120, "recording first, got b{b} r{r}");
        let (b, r) = colour(&document, 400_000);
        assert!(r > 180 && b < 80, "the V2 video covers it, got b{b} r{r}");
        let (b, r) = colour(&document, 700_000);
        assert!(b > 180 && r < 80, "the V3 image is on top, got b{b} r{r}");
        let mut hidden = document.clone();
        let index = hidden
            .sequence
            .tracks
            .iter()
            .position(|t| t.id == v3)
            .unwrap();
        hidden.sequence.tracks[index].hidden = true;
        let (b, r) = colour(&hidden, 700_000);
        assert!(
            r > 180 && b < 80,
            "with V3 hidden the video shows, got b{b} r{r}"
        );

        let frame = 400_000 * SAMPLE_RATE as u64 / 1_000_000;
        let with_sound = AudioMixer::new(&root, &document)
            .unwrap()
            .read_frames(frame, 480)
            .unwrap();
        let mut muted = document.clone();
        let (sound_track, _) = muted
            .sequence
            .clips()
            .find(|(_, c)| c.asset == video_id && c.stream == "sound0")
            .unwrap();
        let sound_track = sound_track.id.clone();
        for track in &mut muted.sequence.tracks {
            if track.id == sound_track {
                track.muted = true;
            }
        }
        let without = AudioMixer::new(&root, &muted)
            .unwrap()
            .read_frames(frame, 480)
            .unwrap();
        let spread = |pcm: &[i16]| {
            let (lo, hi) = pcm
                .iter()
                .fold((i16::MAX, i16::MIN), |(lo, hi), &s| (lo.min(s), hi.max(s)));
            hi as i32 - lo as i32
        };
        assert!(
            spread(&with_sound) > 2_000,
            "the clip's tone plays over the recording"
        );
        assert!(spread(&without) < 200, "a muted track is silent");

        // Exported too: the frame at 0.4 s is the video's red.
        let output = export(&root, document, small());
        let frame = decode_h264_frame(&output, 450_000).unwrap();
        let i = ((90 * frame.stride) + 160 * 4) as usize;
        assert!(
            frame.data[i + 2] > 180 && frame.data[i] < 80,
            "export shows the video over the recording"
        );
        crate::media::release_decoders();
    }

    /// A project with no recording: made from imported media only, it previews and exports.
    #[test]
    #[cfg_attr(
        not(target_os = "macos"),
        ignore = "needs a GPU adapter and FFmpeg; run with --ignored on a machine that has them"
    )]
    fn gpu_project_without_a_recording_previews_and_exports() {
        use crate::project::reader::ProjectReader;
        use crate::sequence::edit::SequenceEdit;
        let dir = tempfile::tempdir().unwrap();
        let folder =
            crate::project::folder::create_project_folder(dir.path(), "Slides", None).unwrap();
        let png = dir.path().join("card.png");
        blue_png(&png);
        let mut reader = ProjectReader::open(&folder).unwrap();
        let id = reader.import_media(0, &[png]).unwrap().assets[0].id.clone();
        edit(
            &mut reader,
            SequenceEdit::PlaceAsset {
                asset_id: id,
                at_us: 0,
                track_id: None,
                streams: vec![],
                range: Some(crate::sequence::SourceRange {
                    start_us: 0,
                    end_us: 1_000_000,
                }),
            },
        );
        let document = reader.history().current.clone();
        let mut evaluator =
            SceneEvaluator::new(folder.clone(), document.clone(), 320, 180).unwrap();
        let frame = evaluator.preview_at(500_000).unwrap();
        let i = ((90 * frame.stride) + 160 * 4) as usize;
        assert!(
            frame.data[i] > 180 && frame.data[i + 2] < 80,
            "the card is blue"
        );
        let output = export(&folder, document, small());
        let duration = media_duration_us(&output).unwrap();
        assert!(
            duration.abs_diff(1_000_000) <= AUDIO_DURATION_SLACK_US + 40_000,
            "duration {duration}"
        );
        let frame = decode_h264_frame(&output, 500_000).unwrap();
        let i = ((90 * frame.stride) + 160 * 4) as usize;
        assert!(
            frame.data[i] > 180 && frame.data[i + 2] < 80,
            "export shows the card"
        );
        crate::media::release_decoders();
    }

    /// A recording imported into an empty project is an asset like the project's own: its
    /// screen on V1, its camera in the bubble, its microphone as sound, its zoom on its clock.
    #[test]
    #[cfg_attr(
        not(target_os = "macos"),
        ignore = "needs a GPU adapter and FFmpeg; run with --ignored on a machine that has them"
    )]
    fn gpu_imported_recording_plays_with_its_camera_sound_and_zoom() {
        use crate::project::reader::ProjectReader;
        use crate::sequence::edit::SequenceEdit;
        use crate::sequence::{AssetKind, Role};
        let dir = tempfile::tempdir().unwrap();
        let recording = project_with_tracks(&dir.path().join("rec"), true);
        let folder =
            crate::project::folder::create_project_folder(dir.path(), "Joined", None).unwrap();
        let mut reader = ProjectReader::open(&folder).unwrap();
        let summary = reader.import_media(0, &[recording.clone()]).unwrap();
        let asset = summary.assets[0].clone();
        assert_eq!(asset.kind, AssetKind::Recording);
        let roles: Vec<_> = asset
            .streams
            .iter()
            .map(|s| (s.id.as_str(), s.role))
            .collect();
        assert_eq!(
            roles,
            vec![
                ("screen", Role::Screen),
                ("webcam", Role::Webcam),
                ("mic", Role::Mic)
            ]
        );
        edit(
            &mut reader,
            SequenceEdit::PlaceAsset {
                asset_id: asset.id.clone(),
                at_us: 0,
                track_id: None,
                streams: vec![],
                range: Some(crate::sequence::SourceRange {
                    start_us: 0,
                    end_us: 1_000_000,
                }),
            },
        );
        // A zoom drawn over the clip is on the recording's clock.
        let revision = reader.summary.revision;
        reader
            .add_manual_zoom(revision, 100_000, 900_000, 0.25, 0.25, 2.0)
            .unwrap();
        let document = reader.history().current.clone();
        assert_eq!(document.zooms[0].media.as_deref(), Some(asset.id.as_str()));
        assert!(!reader.summary.zooms[0].edited_ranges.is_empty());
        let mixer = AudioMixer::new(&folder, &document).unwrap();
        assert!(
            mixer.has_audio(),
            "its microphone plays from its own folder"
        );

        let mut layout_only = document.clone();
        layout_only.zooms.clear();
        let evaluator = SceneEvaluator::new(folder.clone(), layout_only, 320, 180).unwrap();
        let scene = evaluator.scene_at(500_000).unwrap();
        assert!(
            scene
                .layers
                .iter()
                .any(|l| l.role == crate::render::LayerRole::Screen),
            "the recording's screen is on V1"
        );
        assert!(
            scene
                .layers
                .iter()
                .any(|l| l.role == crate::render::LayerRole::Webcam),
            "its camera is in the bubble"
        );
        let zoomed = SceneEvaluator::new(folder.clone(), document, 320, 180)
            .unwrap()
            .scene_at(500_000)
            .unwrap();
        let screen = zoomed
            .layers
            .iter()
            .find(|l| l.role == crate::render::LayerRole::Screen)
            .unwrap();
        assert!(
            screen.uv_w < 0.9,
            "its zoom crops into the screen: {}",
            screen.uv_w
        );
        crate::media::release_decoders();
    }

    /// Imported media on V1: an image and a video with sound inserted between parts of the
    /// recording (magnetic pushes the rest along), in preview and export.
    #[test]
    #[cfg_attr(
        not(target_os = "macos"),
        ignore = "needs a GPU adapter and FFmpeg; run with --ignored on a machine that has them"
    )]
    fn gpu_export_plays_imported_media_between_recording_clips() {
        use crate::sequence::edit::SequenceEdit;
        let dir = tempfile::tempdir().unwrap();
        let (root, mut reader) = opened(&screen_and_mic_project(dir.path()));
        let png = dir.path().join("blue.png");
        blue_png(&png);
        let clip = dir.path().join("red.mp4");
        red_video_with_tone(&clip, 0.6);
        let summary = reader.import_media(0, &[png, clip]).unwrap();
        assert_eq!(summary.assets.len(), 3);
        let image_id = summary.assets[1].id.clone();
        let video = summary.assets[2].clone();
        assert!(
            video.streams.iter().any(|s| s.audio_path.is_some()),
            "the video's sound was extracted"
        );
        let v1 = track_id(&reader, "V1");
        edit(
            &mut reader,
            SequenceEdit::PlaceAsset {
                asset_id: video.id.clone(),
                at_us: 500_000,
                track_id: Some(v1.clone()),
                streams: vec![],
                range: None,
            },
        );
        edit(
            &mut reader,
            SequenceEdit::PlaceAsset {
                asset_id: image_id,
                at_us: 500_000 + video.duration_us,
                track_id: Some(v1),
                streams: vec![],
                range: Some(crate::sequence::SourceRange {
                    start_us: 0,
                    end_us: 1_000_000,
                }),
            },
        );
        let total = 2_000_000 + video.duration_us + 1_000_000;
        assert_eq!(reader.summary.duration_us, total);

        let document = reader.history().current.clone();
        let mut evaluator = SceneEvaluator::new(root.clone(), document.clone(), 320, 180).unwrap();
        let preview = evaluator
            .preview_at(500_000 + video.duration_us + 200_000)
            .unwrap();
        let centre = ((90 * preview.stride) + 160 * 4) as usize;
        assert!(
            preview.data[centre] > 200 && preview.data[centre + 2] < 60,
            "image clip is blue"
        );
        let mixer = AudioMixer::new(&root, &document).unwrap();
        assert!(mixer.has_audio());
        let frame = (500_000 + 300_000) as u64 * SAMPLE_RATE as u64 / 1_000_000;
        let tone = mixer.read_frames(frame, 480).unwrap();
        assert!(
            tone.iter().any(|&s| s.unsigned_abs() > 1_000),
            "the imported clip's tone plays"
        );

        let output = export(&root, document, small());
        let duration = media_duration_us(&output).unwrap();
        assert!(
            duration.abs_diff(total) <= AUDIO_DURATION_SLACK_US + 40_000,
            "duration {duration}"
        );
        let at = |t: u64| {
            let frame = decode_h264_frame(&output, t).unwrap();
            let i = ((90 * frame.stride) + 160 * 4) as usize;
            (frame.data[i], frame.data[i + 1], frame.data[i + 2])
        };
        let (b, _, r) = at(800_000);
        assert!(r > 180 && b < 80, "video clip is red, got b{b} r{r}");
        let (b, _, r) = at(500_000 + video.duration_us + 500_000);
        assert!(b > 180 && r < 80, "image clip is blue, got b{b} r{r}");
        crate::media::release_decoders();
    }

    /// Chapters land in the MP4 in playback order, from 0, with cut ones left out.
    #[test]
    #[cfg_attr(
        not(target_os = "macos"),
        ignore = "needs a GPU adapter and FFmpeg; run with --ignored on a machine that has them"
    )]
    fn gpu_export_writes_chapters_in_playback_order() {
        use crate::chapters::Chapter;
        use std::process::Command;

        let dir = tempfile::tempdir().unwrap();
        // Source 1.0 s to 2.0 s first, then 0 to 0.6 s.
        let (root, mut reader) = opened(&screen_and_mic_project(dir.path()));
        reader.ripple_cuts(0, &[(600_000, 1_000_000)]).unwrap();
        let second = reader.summary.sequence.tracks[0].clips[1].id.clone();
        edit(
            &mut reader,
            crate::sequence::edit::SequenceEdit::MoveClips {
                clip_ids: vec![second],
                delta_us: -600_000,
                track_id: None,
                anchor_id: None,
            },
        );
        let mut document = reader.history().current.clone();
        let chapter = |id: &str, source_us: u64, title: &str| Chapter {
            id: id.into(),
            source_us,
            title: title.into(),
            media: None,
            edited_us: None,
        };
        document.chapters = vec![
            chapter("a", 100_000, "Intro; part=1"),
            chapter("b", 800_000, "Cut away"),
            chapter("c", 1_200_000, "Main"),
        ];
        let output = export(&root, document, small());
        let read = Command::new(crate::media::ffmpeg::ffmpeg_path().unwrap())
            .args(["-v", "error", "-i"])
            .arg(&output)
            .args(["-f", "ffmetadata", "-"])
            .output()
            .unwrap();
        assert!(read.status.success());
        let text = String::from_utf8_lossy(&read.stdout);
        let titles: Vec<&str> = text
            .lines()
            .filter_map(|l| l.strip_prefix("title="))
            .collect();
        // "Main" plays first (from 0); "Intro" starts after the 1 s clip; "Cut away" is gone.
        assert_eq!(titles, vec!["Main", "Intro\\; part\\=1"], "{text}");
        let starts: Vec<&str> = text
            .lines()
            .filter_map(|l| l.strip_prefix("START="))
            .collect();
        assert_eq!(starts.len(), 2, "{text}");
        assert_eq!(starts[0], "0");
        // The second chapter starts at 1.1 s (1.0 s clip + 0.1 s into the next one).
        // FFmpeg may rewrite the timebase (macOS writes 1/48000, the audio track's), so read
        // the one it reports.
        let timebase: f64 = text
            .lines()
            .find_map(|l| l.strip_prefix("TIMEBASE=1/"))
            .unwrap_or_else(|| panic!("no timebase in {text}"))
            .parse()
            .unwrap();
        let second_s = starts[1].parse::<f64>().unwrap() / timebase;
        assert!((second_s - 1.1).abs() < 0.01, "{text}");
        assert!(media_duration_us(&output).unwrap() > 1_500_000);
        assert!(decode_h264_frame(&output, 0).is_ok(), "video still decodes");
        crate::media::release_decoders();
    }

    /// A short exports as a 9:16 split-screen video of just its stretch of the edit: the
    /// camera across the top, the screen below.
    #[test]
    #[cfg_attr(
        not(target_os = "macos"),
        ignore = "needs a GPU adapter and FFmpeg; run with --ignored on a machine that has them"
    )]
    fn gpu_export_renders_a_short_as_a_split_vertical_clip() {
        use crate::shorts::{short_document, Short, ShortLayout};

        let dir = tempfile::tempdir().unwrap();
        let (root, reader) = opened(&project_with_tracks(dir.path(), true));
        let base = reader.history().current.clone();
        let short = Short {
            id: "s".into(),
            title: "Short".into(),
            source_start_us: 300_000,
            source_end_us: 1_200_000,
            reason: String::new(),
            layout: ShortLayout::default(),
            media: None,
            edit: None,
            length_us: None,
            edited_start_us: None,
            edited_end_us: None,
        };
        // Real shorts are at least 3 s; this fixture is 2 s, so build the document by hand.
        assert!(short_document(&base, &short, false).is_err());
        let mut document = base.clone();
        document.sequence =
            crate::shorts::slice_sequence(&base.sequence, &base.assets, 300_000, 1_200_000);
        document.layout.aspect_ratio = "9:16".into();
        document.short_layout = Some(short.layout.clone());

        // Standard sizes are reshaped to the canvas: 720p becomes 720x1280.
        let output = export(
            &root,
            document,
            ExportSettings {
                width: 1280,
                height: 720,
                fps: 30,
                ..ExportSettings::default()
            },
        );
        let frame = decode_h264_frame(&output, 100_000).unwrap();
        assert_eq!((frame.width, frame.height), (720, 1280));
        let pixel = |y: u32| {
            let i = (y * frame.stride + 360 * 4) as usize;
            (frame.data[i], frame.data[i + 1], frame.data[i + 2])
        };
        // The camera takes the top 35% (448 px): red.
        let (b, g, r) = pixel(200);
        assert!(
            r > 180 && g < 70 && b < 70,
            "camera band is red, got b{b} g{g} r{r}"
        );
        // The screen fills the rest: grey, not red, not the black background.
        let (b, g, r) = pixel(900);
        assert!(
            r.abs_diff(b) < 30 && r.abs_diff(g) < 30 && r > 20,
            "screen band is grey, got b{b} g{g} r{r}"
        );
        let duration = media_duration_us(&output).unwrap();
        assert!(
            duration.abs_diff(900_000) <= AUDIO_DURATION_SLACK_US + 40_000,
            "duration {duration}"
        );
        crate::media::release_decoders();
    }

    /// Export with the clips reordered: the later recording plays first.
    #[test]
    #[cfg_attr(
        not(target_os = "macos"),
        ignore = "needs a GPU adapter and FFmpeg; run with --ignored on a machine that has them"
    )]
    fn gpu_export_plays_reordered_clips_in_timeline_order() {
        let dir = tempfile::tempdir().unwrap();
        let (root, reader) = reordered_project(dir.path());
        let output = export(&root, reader.history().current.clone(), small());
        let duration = media_duration_us(&output).unwrap();
        assert!(
            duration.abs_diff(1_300_000) <= AUDIO_DURATION_SLACK_US,
            "duration {duration}"
        );
        // The screen fixture is 10 fps with level 10 + 12 * frame.
        let level_at = |t: u64| {
            let frame = decode_h264_frame(&output, t).unwrap();
            frame.data[((90 * frame.stride) + 160 * 4) as usize + 1]
        };
        let first = level_at(150_000);
        assert!(
            first.abs_diff(10 + 12 * 13) <= 14,
            "opens on source 1.35s, got {first}"
        );
        let later = level_at(1_050_000);
        assert!(
            later.abs_diff(10 + 12 * 2) <= 14,
            "then source 0.25s, got {later}"
        );
        crate::media::release_decoders();
    }

    /// Full export of a recording project with a cut: decode, composite, encode, mux.
    #[test]
    #[cfg_attr(
        not(target_os = "macos"),
        ignore = "needs a GPU adapter and FFmpeg; run with --ignored on a machine that has them"
    )]
    fn gpu_export_writes_a_playable_mp4_across_a_cut() {
        let dir = tempfile::tempdir().unwrap();
        let (root, reader) = cut_project(dir.path());
        let output = export(&root, reader.history().current.clone(), small());
        assert!(output.is_file());
        let duration = media_duration_us(&output).unwrap();
        assert!(
            duration.abs_diff(1_300_000) <= AUDIO_DURATION_SLACK_US,
            "duration {duration}"
        );
        let leftovers: Vec<_> = fs::read_dir(output.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains("partial"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "temp files left behind: {leftovers:?}"
        );
        // Just after the cut the screen shows source frame 12, not frame 5.
        let after_cut = decode_h264_frame(&output, 550_000).unwrap();
        assert_eq!((after_cut.width, after_cut.height), (320, 180));
        let centre = ((90 * after_cut.stride) + 160 * 4) as usize;
        let level = after_cut.data[centre + 1];
        let expected = 10 + 12 * 12;
        assert!(
            level.abs_diff(expected) <= 14,
            "level {level} after the cut, expected about {expected}"
        );
        crate::media::release_decoders();
    }

    /// The webview preview path: small canvas, capped decode, JPEG out.
    #[test]
    #[cfg_attr(
        not(target_os = "macos"),
        ignore = "needs a GPU adapter and FFmpeg; run with --ignored on a machine that has them"
    )]
    fn gpu_webview_preview_frame_across_a_cut() {
        use crate::playback::preview::{encode_webview_frame, PreviewQuality};
        let dir = tempfile::tempdir().unwrap();
        let (root, reader) = cut_project(dir.path());
        let (width, height) = PreviewQuality::default_for(true).canvas(1920, 1080);
        let mut evaluator =
            SceneEvaluator::new(root, reader.history().current.clone(), width, height)
                .unwrap()
                .with_decode_limit(DecodeLimit {
                    max_width: 32,
                    max_height: 32,
                    max_rate: 30,
                });
        // Play forward across the cut the way the playback worker does.
        let mut last = None;
        for edited_us in (0..1_300_000).step_by(33_333) {
            last = Some(evaluator.preview_at(edited_us).unwrap());
        }
        let frame = last.unwrap();
        assert_eq!((frame.width, frame.height), (1280, 720));
        let jpeg = encode_webview_frame(&frame).unwrap();
        let decoded = image::load_from_memory(&jpeg).unwrap().to_rgb8();
        assert_eq!(decoded.dimensions(), (1280, 720));
        // The last frame shows source frame 19, the brightest step.
        let level = decoded.get_pixel(640, 360).0[1];
        let expected = 10 + 19 * 12;
        assert!(
            level.abs_diff(expected) <= 16,
            "level {level}, expected about {expected}"
        );
        crate::media::release_decoders();
    }

    /// Captions come from the mic transcript, skip cut words and land in the frame.
    #[test]
    fn captions_render_from_the_transcript_in_edited_time() {
        use crate::fixtures::{generate_pcm16_wav, TestProject};
        use crate::project::manifest::{TrackDescriptor, TrackType};
        use crate::project::JournalRecord;
        use crate::transcript::{store, test_word, ProviderKind, Transcript};

        let dir = tempfile::tempdir().unwrap();
        let mut bundle = TestProject::create(dir.path(), "captions");
        let recording = bundle.root_path().to_path_buf();
        let path = "media/mic/000001.wav";
        fs::write(
            recording.join(path),
            generate_pcm16_wav(48_000, 1, &vec![0i16; 96_000]),
        )
        .unwrap();
        bundle.manifest_mut().tracks.push(TrackDescriptor {
            id: "mic".into(),
            track_type: TrackType::MicAudio,
            codec: "pcm".into(),
            relative_path: path.into(),
            width: None,
            height: None,
            fps: None,
            sample_rate: Some(48_000),
            channels: Some(1),
            gaps_total: 0,
            media_timescale: Some(48_000),
        });
        bundle.append_journal(JournalRecord::SegmentCommitted {
            seq: 0,
            track_id: "mic".into(),
            relative_path: path.into(),
            start_us: 0,
            end_us: 2_000_000,
            size_bytes: fs::metadata(recording.join(path)).unwrap().len(),
            is_keyframe_start: true,
            media_timescale: 48_000,
            media_start_value: 0,
            host_anchor_us: 0,
        });
        bundle.manifest_mut().duration_us = 2_000_000;
        bundle.manifest_mut().active_duration_us = 2_000_000;
        bundle.save_manifest();
        drop(bundle);
        let (root, mut reader) = opened(&recording);
        let key = format!("{}.mic", reader.summary.assets[0].id);
        store::save_transcript(
            &root,
            &Transcript::new(
                key.clone(),
                ProviderKind::ElevenLabs,
                "scribe_v2".into(),
                None,
                vec![
                    test_word("Hello", 0, 400),
                    test_word("um", 600, 900),
                    test_word("world", 1300, 1700),
                ],
            ),
        )
        .unwrap();
        // "um" is cut out: source 500-1200 removed.
        reader.ripple_cuts(0, &[(500_000, 1_200_000)]).unwrap();
        let mut document = reader.history().current.clone();
        document.layout.background_type = "solid".into();
        document.layout.color_start = "#000000".into();
        document.layout.color_end = "#000000".into();
        document.captions.enabled = true;
        document.captions.text_color = "#FFFFFF".into();
        document.captions.highlight_words = false;
        let evaluator = SceneEvaluator {
            root: root.clone(),
            document,
            compositor: None,
            width: 320,
            height: 180,
            decode_limit: DecodeLimit::NONE,
            wallpaper: std::cell::OnceCell::new(),
            webcam_focus: std::cell::OnceCell::new(),
            caption_cues: std::cell::OnceCell::new(),
            caption_cache: std::cell::RefCell::new(CaptionCache::default()),
            image_cache: std::cell::RefCell::new(Vec::new()),
            cursors: Default::default(),
            zooms: Default::default(),
            cursor_images: Default::default(),
        };
        let (_, _, y) = evaluator.caption_at(100_000).unwrap();
        assert!(y > 90, "caption sits in the lower half, y {y}");
        let cues = evaluator.caption_cues.get().unwrap();
        let words: Vec<_> = cues
            .iter()
            .flat_map(|c| c.words.iter().map(|w| w.text.as_str()))
            .collect();
        assert_eq!(words, vec!["Hello", "world"]);
        // "world" starts at edited 600 ms once the cut closes up.
        assert_eq!(cues.last().unwrap().words.last().unwrap().start_us, 600_000);

        let scene = evaluator.scene_at(100_000).unwrap();
        let frame = Compositor::composite_cpu(&scene).unwrap();
        let bright = frame
            .data
            .chunks_exact(4)
            .filter(|p| p[0] > 200 && p[1] > 200 && p[2] > 200)
            .count();
        assert!(bright > 50, "caption text drawn, {bright} white pixels");
        // The GPU blends the caption the same way, where a GPU adapter exists.
        if let Ok(gpu) = Compositor::new() {
            let frame = gpu.composite(&scene).unwrap();
            let gpu_bright = frame
                .data
                .chunks_exact(4)
                .filter(|p| p[0] > 200 && p[1] > 200 && p[2] > 200)
                .count();
            assert!(
                gpu_bright.abs_diff(bright) <= bright / 10,
                "gpu {gpu_bright} vs cpu {bright}"
            );
        }
    }

    #[test]
    fn status_json_omits_pixels() {
        let json = serde_json::to_value(ExportStatus::idle()).unwrap();
        assert!(json.get("pixels").is_none());
        assert!(json.get("samples").is_none());
        assert_eq!(json["state"], "idle");
    }
}
