//! Immutable-revision export job. Preview and export share one scene evaluator.
use crate::media::audio::{AudioMixer, CHANNELS, CHUNK_FRAMES, SAMPLE_RATE};
mod native;

use crate::media::ffmpeg::{DecodeLimit, FfmpegExport};
use crate::media::{
    decode_h264_frame_limited, media_backend, media_duration_us, EncoderGate, MediaBackend,
    PixelFormat, RateControl, VideoFrame, VideoQuality, MAX_FRAME_DIM,
};
use crate::project::reader::{safe_path, SegmentSummary};
use crate::project::revision::EditDocument;
use crate::render::present::{Placement, WindowPresenter, WindowTarget};
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

/// A cut that skips at most this much of what is playing reads on through it on the open
/// decoder (a few frames); a longer one gets a decoder of its own, started ahead.
const READ_ON_US: u64 = 200_000;

pub struct SceneEvaluator {
    root: PathBuf,
    document: EditDocument,
    compositor: Option<Compositor>,
    width: u32,
    height: u32,
    decode_limit: DecodeLimit,
    /// Frames come out as NV12 (for an encoder) rather than BGRA, when the GPU composites.
    nv12_output: bool,
    /// The document is fixed, so the wallpaper or gradient is built once rather than per frame.
    wallpaper: std::cell::OnceCell<Option<VideoFrame>>,
    /// The timeline's layouts as they play, worked out once.
    layouts: std::cell::OnceCell<crate::layouts::Resolved>,
    /// How each camera clip is framed (by clip id), once its face is known.
    camera_framing: std::cell::RefCell<
        std::collections::HashMap<String, (Option<crate::face::FocusPoint>, (f32, f32))>,
    >,
    /// Where the camera fills the frame, for the graphics that ease in over it.
    full_camera: std::cell::OnceCell<Vec<(u64, u64)>>,
    /// Caption cues from the captioned sound's transcript; empty when captions are off.
    caption_cues: std::cell::OnceCell<Vec<crate::captions::CaptionCue>>,
    /// The last cue drawn and its colored frame for the active word.
    caption_cache: std::cell::RefCell<CaptionCache>,
    /// Recorded pointers by recording, looked up once per evaluator.
    cursors: std::cell::RefCell<Cursors>,
    /// Clip edges whose decoders have been started ahead (see [`SceneEvaluator::prefetch`]),
    /// and where that last looked from.
    prefetched: std::cell::RefCell<(u64, std::collections::BTreeSet<u64>)>,
    /// Zooms by recording, with the camera paths that follow the mouse, worked out once.
    zooms: std::cell::RefCell<std::collections::HashMap<String, Zooms>>,
    /// What plays each stream, by asset and stream, looked up once: a lookup checks the
    /// recording's files on disk, too slow to repeat for every layer of every frame.
    sources: std::cell::RefCell<Sources>,
    /// Pointer pictures by file, decoded once.
    cursor_images: std::cell::RefCell<CursorImages>,
    /// What stands in for a graphic still rendering, by its name and size.
    graphic_placeholders:
        std::cell::RefCell<std::collections::HashMap<(String, u32, u32), VideoFrame>>,
    /// The last shortcut badge drawn, by recording and shortcut.
    shortcut_badge: std::cell::RefCell<Option<((String, usize), Option<VideoFrame>)>>,
    /// The crop to the active window by recording, worked out once (`None`: no crop).
    window_crops: std::cell::RefCell<
        std::collections::HashMap<String, Option<std::sync::Arc<crate::window_crop::CropPlan>>>,
    >,
    /// When the webcam bubble hides because the work is under it, on the timeline.
    bubble_hidden: std::cell::OnceCell<Vec<(u64, u64)>>,
}

type Cursors =
    std::collections::HashMap<String, Option<std::sync::Arc<crate::cursor::CursorTrack>>>;
type Sources = std::collections::HashMap<
    String,
    std::collections::HashMap<String, std::sync::Arc<crate::sequence::sources::StreamSource>>,
>;
type CursorImages = std::collections::HashMap<PathBuf, Option<VideoFrame>>;

/// What an evaluator looked up about the project's media alone (its folder and assets), not
/// its edit: still right for a rebuild after an edit or for another short of the same media.
struct MediaLookups {
    root: PathBuf,
    assets: Vec<Asset>,
    sources: Sources,
    cursors: Cursors,
    cursor_images: CursorImages,
}

impl MediaLookups {
    fn fits(&self, root: &Path, document: &EditDocument) -> bool {
        self.root == root && self.assets == document.assets
    }
}

#[derive(Default)]
struct CaptionCache {
    raster: Option<(usize, Option<crate::captions::CueRaster>)>,
    frame: Option<((usize, Option<usize>), VideoFrame)>,
}

/// State worth keeping when the playback worker rebuilds its evaluator after a seek or an
/// edit: the GPU device (slow to create), the background if the layout kept it, and the
/// media lookups if the media did not change.
pub struct EvaluatorReuse {
    compositor: Compositor,
    background: Option<(String, Option<VideoFrame>)>,
    media: MediaLookups,
}

/// Starts decoders for other documents (other shorts of the project) as an evaluator would
/// decode them, from any thread (see [`SceneEvaluator::decoder_warmer`]).
pub struct DecoderWarmer {
    root: PathBuf,
    width: u32,
    height: u32,
    decode_limit: DecodeLimit,
    yuv: bool,
    assets: Vec<Asset>,
    sources: Sources,
}

impl DecoderWarmer {
    /// Starts decoders for the first frame of `document`, so changing to it shows that frame
    /// without waiting for FFmpeg to start. False when one was turned away (too many starting).
    pub fn prefetch_start_of(&self, document: EditDocument) -> bool {
        let other = SceneEvaluator::with_parts(
            self.root.clone(),
            document,
            None,
            self.width,
            self.height,
            std::cell::OnceCell::new(),
        )
        .with_decode_limit(self.decode_limit);
        if other.document.assets == self.assets {
            *other.sources.borrow_mut() = self.sources.clone();
        }
        other.prefetch_shown(0, &[], self.yuv)
    }
}

/// The background a document draws: the project's, or the short's own choice.
fn background_layout(document: &EditDocument) -> crate::project::layout::EditLayout {
    match &document.short_layout {
        Some(short) => crate::shorts::background_layout(&document.layout, short),
        None => document.layout.clone(),
    }
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

/// The size of a recording's camera picture, from its manifest (16:9 when it says nothing).
fn camera_size(folder: &Path) -> (u32, u32) {
    let size = std::fs::read(folder.join("manifest.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|manifest| {
            let track = manifest["tracks"]
                .as_array()?
                .iter()
                .find(|t| t["id"] == "webcam" || t["trackType"] == "webcam")?
                .clone();
            Some((
                track["width"].as_u64()? as u32,
                track["height"].as_u64()? as u32,
            ))
        });
    size.filter(|(w, h)| *w > 0 && *h > 0)
        .unwrap_or((1280, 720))
}

/// A picture to decode, with nothing borrowed, so screen and camera decode side by side.
enum PictureJob {
    Segment(SegmentSummary, u64),
    Video(PathBuf, u64),
    Ready(VideoFrame),
}

impl PictureJob {
    /// `yuv`: recording video comes as NV12, for a GPU compositor to convert (imported files
    /// stay BGRA: they may carry transparency).
    fn run(self, root: &Path, limit: DecodeLimit, yuv: bool) -> Result<VideoFrame, String> {
        match self {
            PictureJob::Segment(segment, time) => {
                decode_layer(root, &segment, time, DecodeLimit { yuv, ..limit })
            }
            PictureJob::Video(path, time) => {
                crate::media::ffmpeg::decode_bgra_limited(&path, time, limit)
            }
            PictureJob::Ready(frame) => Ok(frame),
        }
    }
}

/// Why [`SceneEvaluator::present_at`] drew nothing.
#[derive(Debug)]
pub enum PresentError {
    /// The frame could not be made (a source that will not decode, say).
    Scene(String),
    /// The window could not show it: the preview has to go back to the webview.
    Window(String),
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
    /// Graphics, bottom track first: drawn over everything else.
    graphics: Vec<&'a Clip>,
}

impl<'a> Shown<'a> {
    /// The clips whose pictures show: the screen, the camera (when the bubble is on) and the
    /// overlays.
    fn pictures(self, webcam: bool) -> Vec<&'a Clip> {
        self.screen
            .map(|(_, clip)| clip)
            .into_iter()
            .chain(self.webcam.filter(|_| webcam))
            .chain(self.overlays.into_iter().map(|(_, clip)| clip))
            .collect()
    }
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
        let key = background_key(&root, &background_layout(&document), width, height);
        let wallpaper = std::cell::OnceCell::new();
        let mut media = None;
        let compositor = match reuse {
            Some(reuse) => {
                if let Some((_, frame)) = reuse.background.filter(|(k, _)| *k == key) {
                    let _ = wallpaper.set(frame);
                }
                media = Some(reuse.media).filter(|media| media.fits(&root, &document));
                reuse.compositor
            }
            None => Compositor::new()?,
        };
        let evaluator =
            Self::with_parts(root, document, Some(compositor), width, height, wallpaper);
        if let Some(media) = media {
            evaluator.take_lookups(media);
        }
        Ok(evaluator)
    }

    /// What this evaluator has looked up about the project's media so far.
    fn lookups(&self) -> MediaLookups {
        MediaLookups {
            root: self.root.clone(),
            assets: self.document.assets.clone(),
            sources: self.sources.borrow().clone(),
            cursors: self.cursors.borrow().clone(),
            cursor_images: self.cursor_images.borrow().clone(),
        }
    }

    fn take_lookups(&self, media: MediaLookups) {
        *self.sources.borrow_mut() = media.sources;
        *self.cursors.borrow_mut() = media.cursors;
        *self.cursor_images.borrow_mut() = media.cursor_images;
    }

    fn with_parts(
        root: PathBuf,
        document: EditDocument,
        compositor: Option<Compositor>,
        width: u32,
        height: u32,
        wallpaper: std::cell::OnceCell<Option<VideoFrame>>,
    ) -> Self {
        Self {
            root,
            document,
            compositor,
            width,
            height,
            decode_limit: DecodeLimit::NONE,
            nv12_output: false,
            wallpaper,
            layouts: std::cell::OnceCell::new(),
            camera_framing: Default::default(),
            full_camera: std::cell::OnceCell::new(),
            caption_cues: std::cell::OnceCell::new(),
            caption_cache: std::cell::RefCell::new(CaptionCache::default()),
            cursors: Default::default(),
            prefetched: Default::default(),
            zooms: Default::default(),
            sources: Default::default(),
            cursor_images: Default::default(),
            graphic_placeholders: Default::default(),
            shortcut_badge: Default::default(),
            window_crops: Default::default(),
            bubble_hidden: std::cell::OnceCell::new(),
        }
    }

    /// Gives up the parts a replacement evaluator can reuse.
    pub fn into_reuse(self) -> Option<EvaluatorReuse> {
        let key = background_key(
            &self.root,
            &background_layout(&self.document),
            self.width,
            self.height,
        );
        let media = self.lookups();
        let background = self.wallpaper.into_inner().map(|frame| (key, frame));
        self.compositor.map(|compositor| EvaluatorReuse {
            compositor,
            background,
            media,
        })
    }

    /// Whether recording video is decoded as NV12: a third of BGRA's bytes through the decoder
    /// pipe (its slowest part), converted by the GPU compositor. Without one, BGRA.
    fn yuv(&self) -> bool {
        self.compositor.is_some()
    }

    /// Decodes sources no larger or faster than `limit`, for preview.
    pub fn with_decode_limit(mut self, limit: DecodeLimit) -> Self {
        self.decode_limit = limit;
        self
    }

    /// With `nv12` and a GPU compositor, frames come out as limited-range BT.709 NV12, as an
    /// H.264 encoder takes them (see [`SceneEvaluator::output_format`]).
    pub fn with_nv12_output(mut self, nv12: bool) -> Self {
        self.nv12_output = nv12;
        self
    }

    /// The format of the frames [`SceneEvaluator::preview_at`] makes.
    pub fn output_format(&self) -> PixelFormat {
        if self.nv12_output && self.compositor.is_some() {
            PixelFormat::Nv12
        } else {
            PixelFormat::Bgra8888
        }
    }

    pub fn preview_at(&mut self, edited_us: u64) -> Result<VideoFrame, String> {
        let scene = self.composed_scene(edited_us)?;
        let started = std::time::Instant::now();
        let mut frame = match self.compositor.as_mut() {
            Some(compositor) if self.nv12_output => compositor.composite_nv12(&scene)?,
            Some(compositor) => compositor.composite(&scene)?,
            None => Compositor::composite_cpu(&scene)?,
        };
        crate::media::profile("composite", started);
        frame.pts_us = edited_us;
        Ok(frame)
    }

    /// Like [`SceneEvaluator::preview_at`], but draws the frame straight into the window of
    /// `presenter` (made here on first use, and again if the compositor changed). Returns false
    /// when the window had nothing to draw into.
    pub fn present_at(
        &mut self,
        edited_us: u64,
        presenter: &mut Option<WindowPresenter>,
        window: &WindowTarget,
        placement: &Placement,
    ) -> Result<bool, PresentError> {
        let scene = self
            .composed_scene(edited_us)
            .map_err(PresentError::Scene)?;
        let started = std::time::Instant::now();
        let presenter = self
            .presenter_for(presenter, window)
            .map_err(PresentError::Window)?;
        let compositor = self.compositor.as_ref().ok_or(PresentError::Window(
            "The window preview needs the GPU compositor".into(),
        ))?;
        let presented = compositor
            .composite_present(&scene, presenter, placement)
            .map_err(PresentError::Window)?;
        crate::media::profile("composite and present", started);
        Ok(presented)
    }

    /// Draws the last frame into the window again, for a preview that moved. Returns false
    /// when there is no frame yet.
    pub fn present_again(
        &mut self,
        presenter: &mut Option<WindowPresenter>,
        window: &WindowTarget,
        placement: &Placement,
    ) -> Result<bool, String> {
        let presenter = self.presenter_for(presenter, window)?;
        let compositor = self.compositor.as_ref().ok_or("No GPU compositor")?;
        compositor.present_last(presenter, placement)
    }

    fn presenter_for<'p>(
        &self,
        presenter: &'p mut Option<WindowPresenter>,
        window: &WindowTarget,
    ) -> Result<&'p mut WindowPresenter, String> {
        let compositor = self
            .compositor
            .as_ref()
            .ok_or("The window preview needs the GPU compositor")?;
        if !presenter.as_ref().is_some_and(|p| compositor.owns(p)) {
            *presenter = None;
            *presenter = Some(compositor.window_presenter(window.clone())?);
        }
        Ok(presenter.as_mut().unwrap())
    }

    /// The scene at `edited_us`, with the static background marked for the GPU to keep.
    fn composed_scene(&mut self, edited_us: u64) -> Result<Scene, String> {
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
                background_key(
                    &self.root,
                    &background_layout(&self.document),
                    self.width,
                    self.height,
                )
                .hash(&mut hasher);
                layer.cache_key = Some(hasher.finish());
            }
        }
        Ok(scene)
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
            if document.asset(&clip.asset).is_some_and(|a| a.is_graphic()) {
                shown.graphics.push(clip);
                continue;
            }
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

    /// The timeline's layouts as they play.
    fn layouts(&self) -> &crate::layouts::Resolved {
        self.layouts
            .get_or_init(|| crate::layouts::resolve::of_document(&self.document))
    }

    /// How camera clip `clip` is framed: its face, or the hand framing.
    fn framing_of(&self, clip: &Clip) -> (Option<crate::face::FocusPoint>, (f32, f32)) {
        if let Some(found) = self.camera_framing.borrow().get(&clip.id) {
            return *found;
        }
        let style = &self.document.layout;
        let found = crate::shorts::camera_framing(&self.root, style, clip);
        // A face not found yet may be found while this plays (its track is made in the
        // background): look again next time.
        if found.0.is_some() || !style.face_framing || style.camera_clips.contains_key(&clip.id) {
            self.camera_framing
                .borrow_mut()
                .insert(clip.id.clone(), found);
        }
        found
    }

    /// The screen's place and the part of it that shows in a slot that fills: cut to the
    /// slot's shape, following the zooms (a short's own zoom and pan on top).
    fn fill_view(
        &self,
        asset: &str,
        local: u64,
        frame: (u32, u32),
        rect: crate::layouts::arrange::Rect,
    ) -> (crate::layouts::arrange::Rect, (f32, f32, f32, f32)) {
        let short = self.document.short_layout.clone().unwrap_or_default();
        let crop = self.screen_crop_at(asset, local);
        let (center, zoom) = if short.follow_zooms {
            let zooms = self.zooms_for(asset);
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
        let (placed, uv) = crate::shorts::screen_placement(
            frame.0,
            frame.1,
            crop,
            rect.pixels(self.width, self.height),
            zoom,
            center,
            (short.screen_pan_x, short.screen_pan_y),
        );
        (crate::layouts::arrange::Rect::of(placed), uv)
    }

    pub fn scene_at(&self, edited_us: u64) -> Result<Scene, String> {
        use crate::layouts::arrange::{arrange, blend, Canvas, Sources};
        let shown = self.shown(edited_us);
        let at = |clip: &Clip| clip.local_us(edited_us).unwrap_or(clip.in_us);
        let layouts = self.layouts();
        // Screen and camera decode in parallel; each waits on its own FFmpeg process. A picture
        // that some layout shows keeps decoding while hidden, so a layout change never waits
        // for a decoder to start.
        let screen_job = shown
            .screen
            .filter(|_| layouts.shows_screen())
            .map(|(_, clip)| self.picture_job(clip, at(clip)))
            .transpose()?
            .flatten();
        let webcam_job = shown
            .webcam
            .filter(|_| layouts.shows_camera())
            .map(|clip| self.picture_job(clip, at(clip)))
            .transpose()?
            .flatten();
        let (root, limit, yuv) = (&self.root, self.decode_limit, self.yuv());
        let (screen, webcam) = std::thread::scope(|scope| {
            let webcam = webcam_job.map(|job| scope.spawn(move || job.run(root, limit, yuv)));
            let screen = screen_job
                .map(|job| job.run(root, limit, yuv))
                .transpose()?;
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
        let style = &self.document.layout;
        let unit = crate::render::layout_px_unit(self.width, self.height);
        let screen_clip = shown.screen.map(|(_, clip)| clip);
        let local = screen_clip.map(at);
        let frame = layouts.frame_at(edited_us);
        let canvas = Canvas {
            width: self.width,
            height: self.height,
            unit,
        };
        let sources = Sources {
            screen: screen.as_ref().map(|f| (f.width, f.height)),
            camera: webcam.as_ref().map(|f| (f.width, f.height)),
        };
        let from = arrange(frame.from, style, canvas, sources);
        let to = arrange(frame.to, style, canvas, sources);
        let parts = blend(&from, &to, frame.weight);
        // Nothing to frame: the canvas is black, with any overlays on it.
        let mut scene = if screen.is_some() || webcam.is_some() {
            Scene::backdrop(
                self.width,
                self.height,
                &background_layout(&self.document),
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
        if let (Some(picture), Some(slot), Some(clip), Some(local)) =
            (screen, parts.screen, screen_clip, local)
        {
            let size = (picture.width, picture.height);
            let view = |s: &crate::layouts::arrange::ScreenSlot| {
                if s.fill {
                    self.fill_view(&clip.asset, local, size, s.rect)
                } else {
                    (s.rect, self.screen_view(&clip.asset, local))
                }
            };
            // Between a fitted and a filled screen, each end's view blends into the other's.
            let (rect, uv) = match (from.screen, to.screen) {
                (Some(a), Some(b)) if a.fill != b.fill && frame.weight > 0.0 && frame.weight < 1.0 => {
                    let ((ra, ua), (rb, ub), t) = (view(&a), view(&b), frame.weight);
                    let l = |x: f32, y: f32| x + (y - x) * t;
                    (
                        ra.lerp(rb, t),
                        (l(ua.0, ub.0), l(ua.1, ub.1), l(ua.2, ub.2), l(ua.3, ub.3)),
                    )
                }
                _ => view(&slot),
            };
            let slot = crate::layouts::arrange::ScreenSlot { rect, ..slot };
            scene.push_screen(picture, &slot, uv, style, unit);
            self.push_cursor(&mut scene, &clip.asset, local)?;
        }
        if let (Some(picture), Some(slot)) = (webcam, parts.camera) {
            let (focus, offset) = shown
                .webcam
                .map_or((None, (0.0, 0.0)), |clip| self.framing_of(clip));
            let uv = crate::render::camera_uv((picture.width, picture.height), &slot, focus, offset);
            scene.push_camera(picture, &slot, uv, style, unit)?;
            // A bubble gets out of the way of work under it.
            let hidden = crate::bubble_hide::hidden_weight(self.bubble_hidden(), edited_us);
            if hidden > 0.0 {
                let bubble = frame.share(|l| matches!(l, crate::layouts::Layout::Bubble(_)));
                scene.fade_webcam(1.0 - hidden * bubble);
            }
        }
        self.push_overlays(&mut scene, &shown, edited_us)?;
        if let (Some(clip), Some(local)) = (screen_clip, local) {
            self.push_shortcut_badge(&mut scene, &clip.asset, local);
        }
        // A short's graphics over the screen go under the captions; graphics go over
        // everything else.
        self.push_graphics(&mut scene, &shown, edited_us, true)?;
        if let Some((caption, x, y)) = self.caption_at(edited_us) {
            let y = match &self.document.short_layout {
                // Where the short's layout puts them, moving as the layout changes.
                Some(short) => {
                    let spot = |parts| {
                        crate::shorts::caption_y(short.caption_spot, parts, caption.height, self.height)
                    };
                    let (a, b) = (spot(&from), spot(&to));
                    (a + (b - a) * frame.weight).round() as u32
                }
                None => y,
            };
            scene.push_caption(caption, x, y);
        }
        self.push_graphics(&mut scene, &shown, edited_us, false)?;
        Ok(scene)
    }

    /// Overlay clips over the canvas in track order; one below the screen's track goes under
    /// it. A still keeps its texture on the GPU from frame to frame.
    fn push_overlays(
        &self,
        scene: &mut Scene,
        shown: &Shown<'_>,
        edited_us: u64,
    ) -> Result<(), String> {
        let screen_track = shown.screen.map(|(index, _)| index);
        for (index, clip) in &shown.overlays {
            let local = clip.local_us(edited_us).unwrap_or(clip.in_us);
            let job = self.picture_job(clip, local)?;
            let still = matches!(job, Some(PictureJob::Ready(_)));
            if let Some(mut frame) = job
                .map(|job| job.run(&self.root, self.decode_limit, self.yuv()))
                .transpose()?
            {
                let key = still.then(|| {
                    use std::hash::{Hash, Hasher};
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    ("still", &clip.asset).hash(&mut hasher);
                    hasher.finish()
                });
                frame.pts_us = edited_us;
                let before = scene.layers.len();
                scene.push_overlay(frame, clip.fit == crate::sequence::Fit::Cover, key);
                if screen_track.is_some_and(|screen| *index < screen) {
                    scene.move_under_main(before);
                }
            }
        }
        Ok(())
    }

    /// Graphics in their slots: each one's render for its slot and length, an older render while
    /// a new one is made, or a placeholder until there is one. `over_screen` picks the pass: the
    /// graphics a short shows over its screen (pushed before the captions), or the rest (over
    /// everything: the whole canvas, or a part of a short's split).
    fn push_graphics(
        &self,
        scene: &mut Scene,
        shown: &Shown<'_>,
        edited_us: u64,
        over_screen: bool,
    ) -> Result<(), String> {
        use crate::shorts::GraphicSlot;
        for clip in &shown.graphics {
            let rect = match &self.document.short_layout {
                Some(short) => {
                    let slot = crate::shorts::graphic_slot(short, &clip.id);
                    if (slot == GraphicSlot::OverScreen) != over_screen {
                        continue;
                    }
                    crate::shorts::graphic_rect(&self.document, &clip.id, self.width, self.height)
                }
                None if over_screen => continue,
                None => (0, 0, self.width, self.height),
            };
            let local = clip.local_us(edited_us).unwrap_or(clip.in_us);
            let Some(job) = self.picture_job(clip, local)? else {
                continue;
            };
            let key = match &job {
                PictureJob::Ready(frame) => Some({
                    use std::hash::{Hash, Hasher};
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    (
                        "graphic placeholder",
                        &clip.asset,
                        frame.width,
                        frame.height,
                    )
                        .hash(&mut hasher);
                    hasher.finish()
                }),
                _ => None,
            };
            let mut frame = match job.run(&self.root, self.decode_limit, false) {
                Ok(frame) => frame,
                // A render that will not decode shows nothing rather than stopping playback.
                Err(_) => continue,
            };
            frame.pts_us = edited_us;
            scene.push_graphic(frame, rect, key);
        }
        Ok(())
    }

    /// The picture job for a graphic clip at `local_us` into it.
    fn graphic_job(&self, clip: &Clip, local_us: u64) -> Option<PictureJob> {
        use crate::graphics::render::{self, Lookup};
        let asset = self.document.asset(&clip.asset)?;
        let full_camera = self
            .full_camera
            .get_or_init(|| crate::graphics::full_camera_ranges(&self.document));
        let spec = render::Spec::for_clip(
            &self.root,
            &self.document,
            asset,
            clip,
            (self.width, self.height),
            full_camera,
        );
        // Never past a render's last frame.
        let last_frame = 1_000_000 / render::RENDER_FPS as u64;
        match render::lookup(&spec) {
            Lookup::Ready(path) => Some(PictureJob::Video(
                path,
                local_us.min(spec.length_us.saturating_sub(last_frame)),
            )),
            Lookup::Stale(path, length) => Some(PictureJob::Video(
                path,
                local_us.min(length.saturating_sub(last_frame)),
            )),
            Lookup::Pending => {
                let mut placeholders = self.graphic_placeholders.borrow_mut();
                // The slot's shape: the whole canvas, or a part of a short's split.
                let frame = placeholders
                    .entry((asset.name.clone(), spec.width, spec.height))
                    .or_insert_with(|| {
                        crate::graphics::placeholder_frame(
                            spec.width,
                            spec.height,
                            &format!("{} (rendering)", asset.name),
                        )
                    })
                    .clone();
                Some(PictureJob::Ready(frame))
            }
        }
    }

    /// While playing: starts decoders for what the video tracks show at each clip edge in the
    /// next `horizon_us` after `edited_us`, so a cut plays from a decoder that is already
    /// there rather than waiting for one to start and seek.
    pub fn prefetch(&self, edited_us: u64, horizon_us: u64) {
        let end = edited_us.saturating_add(horizon_us);
        let mut edges = std::collections::BTreeSet::new();
        for track in self.document.sequence.video_tracks().filter(|t| !t.hidden) {
            // Clips are in order: skip those that end before now, stop after the horizon.
            let first = track.clips.partition_point(|c| c.end_us() <= edited_us);
            for clip in track.clips[first..]
                .iter()
                .take_while(|c| c.start_us <= end)
            {
                for edge in [clip.start_us, clip.end_us()] {
                    if edge > edited_us && edge <= end {
                        edges.insert(edge);
                    }
                }
            }
        }
        // A long recording is in segment files one after another: where the next one starts
        // inside a clip playing now, a decoder of that file is needed too.
        let webcam = self.layouts().shows_camera();
        for clip in self.shown(edited_us).pictures(webcam) {
            let Some(local) = clip.local_us(edited_us) else {
                continue;
            };
            let Some(source) = self.segments_of(clip) else {
                continue;
            };
            let crate::sequence::sources::StreamSource::Segments(segments) = &*source else {
                continue;
            };
            let reach = local.saturating_add(horizon_us).min(clip.out_us());
            for segment in segments
                .iter()
                .filter(|s| s.available && s.start_us > local && s.start_us < reach)
            {
                edges.insert(clip.start_us + (segment.start_us - clip.in_us));
            }
        }
        let mut prefetched = self.prefetched.borrow_mut();
        let (from, done) = &mut *prefetched;
        // After a seek the edges ahead are new ones.
        if edited_us < *from || edited_us > from.saturating_add(horizon_us) {
            done.clear();
        }
        *from = edited_us;
        for edge in edges {
            if !done.insert(edge) {
                continue;
            }
            let before: Vec<(&Clip, u64)> = self
                .shown(edge - 1)
                .pictures(webcam)
                .into_iter()
                .filter_map(|clip| Some((clip, clip.local_us(edge - 1)?)))
                .collect();
            // Turned away for now (too many decoders starting): try this edge again later.
            if !self.prefetch_shown(edge, &before, self.yuv()) {
                done.remove(&edge);
            }
        }
    }

    /// Starts decoders for the pictures shown at `edge`, other than those a decoder already
    /// playing (`before`, with where it is) reads on to. False when one was turned away.
    fn prefetch_shown(&self, edge: u64, before: &[(&Clip, u64)], yuv: bool) -> bool {
        let webcam = self.layouts().shows_camera();
        let mut accepted = true;
        for clip in self.shown(edge).pictures(webcam) {
            let local = clip.local_us(edge).unwrap_or(clip.in_us);
            // A short jump on in the file already playing: its decoder reads on to it.
            let reads_on = before.iter().any(|(was, at)| {
                was.asset == clip.asset
                    && was.stream == clip.stream
                    && local >= *at
                    && local - at <= READ_ON_US
                    && self.segment_file(was, *at) == self.segment_file(clip, local)
            });
            if reads_on {
                continue;
            }
            let Some(asset) = self.document.asset(&clip.asset) else {
                continue;
            };
            use crate::sequence::sources::StreamSource;
            let Ok(source) = self.stream_source(asset, &clip.stream) else {
                continue;
            };
            match &*source {
                StreamSource::Segments(segments) => {
                    let Some(segment) = segments
                        .iter()
                        .find(|s| s.available && s.start_us <= local && local < s.end_us)
                    else {
                        continue;
                    };
                    if let Ok(path) = safe_path(&self.root, &segment.relative_path) {
                        accepted &= crate::media::prefetch_video(
                            &path,
                            local - segment.start_us,
                            DecodeLimit {
                                yuv,
                                ..self.decode_limit
                            },
                        );
                    }
                }
                StreamSource::Video(path) => {
                    accepted &= crate::media::prefetch_video(path, local, self.decode_limit)
                }
                _ => {}
            }
        }
        accepted
    }

    /// What another thread needs to start decoders the way this evaluator decodes.
    pub fn decoder_warmer(&self) -> DecoderWarmer {
        DecoderWarmer {
            root: self.root.clone(),
            width: self.width,
            height: self.height,
            decode_limit: self.decode_limit,
            yuv: self.yuv(),
            assets: self.document.assets.clone(),
            sources: self.sources.borrow().clone(),
        }
    }

    /// What plays stream `stream` of `asset`, from this evaluator's cache. A failed lookup (a
    /// recording that is gone) is not kept, so it shows again once it is back.
    fn stream_source(
        &self,
        asset: &crate::sequence::Asset,
        stream: &str,
    ) -> Result<std::sync::Arc<crate::sequence::sources::StreamSource>, String> {
        let cached = self
            .sources
            .borrow()
            .get(asset.id.as_str())
            .and_then(|streams| streams.get(stream))
            .cloned();
        if let Some(source) = cached {
            return Ok(source);
        }
        let source = std::sync::Arc::new(crate::sequence::sources::stream_source(
            &self.root, asset, stream,
        )?);
        self.sources
            .borrow_mut()
            .entry(asset.id.clone())
            .or_default()
            .insert(stream.to_string(), source.clone());
        Ok(source)
    }

    /// The segment files of a clip's stream, when it plays from a recording's segments.
    fn segments_of(
        &self,
        clip: &Clip,
    ) -> Option<std::sync::Arc<crate::sequence::sources::StreamSource>> {
        use crate::sequence::sources::StreamSource;
        let asset = self.document.asset(&clip.asset)?;
        let source = self.stream_source(asset, &clip.stream).ok()?;
        matches!(*source, StreamSource::Segments(_)).then_some(source)
    }

    /// Which segment file plays `local` of a clip's stream (none for other sources).
    fn segment_file(&self, clip: &Clip, local: u64) -> Option<String> {
        use crate::sequence::sources::StreamSource;
        let source = self.segments_of(clip)?;
        let StreamSource::Segments(segments) = &*source else {
            return None;
        };
        segments
            .iter()
            .find(|s| s.available && s.start_us <= local && local < s.end_us)
            .map(|s| s.relative_path.clone())
    }

    /// The wallpaper or gradient, built once per evaluator. A short draws its own choice.
    fn background(&self) -> Result<Option<VideoFrame>, String> {
        Ok(match self.wallpaper.get() {
            Some(cached) => cached.clone(),
            None => {
                let layout = background_layout(&self.document);
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

    /// The crop to the active window over recording `asset`, worked out once: `None` when it
    /// is off, or the recording has no window telemetry or never needs a crop.
    fn window_crop(&self, asset: &str) -> Option<std::sync::Arc<crate::window_crop::CropPlan>> {
        if let Some(plan) = self.window_crops.borrow().get(asset) {
            return plan.clone();
        }
        let layout = &self.document.layout;
        let base = layout.screen_crop_uv();
        let plan = self
            .document
            .asset(asset)
            .filter(|a| layout.window_crop && a.is_recording())
            .and_then(|recording| {
                let windows =
                    crate::telemetry::windows::recording_windows(Path::new(&recording.path));
                let windows = windows.as_ref().as_ref()?;
                let off: Vec<(u64, u64)> = layout
                    .window_crop_off
                    .iter()
                    .filter(|r| self.document.clock_asset(r.media.as_deref()) == Some(asset))
                    .map(|r| (r.source_start_us, r.source_end_us))
                    .collect();
                let aspect = match (recording.width, recording.height) {
                    (w, h) if w > 0 && h > 0 => w as f32 / h as f32,
                    _ => 16.0 / 9.0,
                };
                let plan = crate::window_crop::CropPlan::plan(
                    windows,
                    &off,
                    base,
                    aspect,
                    layout.window_crop_padding_pct,
                );
                plan.crops(base).then(|| std::sync::Arc::new(plan))
            });
        self.window_crops
            .borrow_mut()
            .insert(asset.to_string(), plan.clone());
        plan
    }

    /// The part of recording `asset`'s screen kept at `local` on its clock: the layout's crop,
    /// or the active window's.
    fn screen_crop_at(&self, asset: &str, local: u64) -> (f32, f32, f32, f32) {
        match self.window_crop(asset) {
            Some(plan) => plan.rect_at(local),
            None => self.document.layout.screen_crop_uv(),
        }
    }

    /// What of recording `asset`'s screen shows at `local` (UV of the full picture): the crop
    /// with any zoom inside it.
    fn screen_view(&self, asset: &str, local: u64) -> (f32, f32, f32, f32) {
        let crop = self.screen_crop_at(asset, local);
        let zooms = self.zooms_for(asset);
        if zooms.is_empty() {
            return crop;
        }
        let config = crate::zoom::eval_config_for(&self.document.zooms);
        let camera = crate::zoom::evaluate_at_source(&zooms, local, &config);
        crate::window_crop::zoom_inside(
            self.document.layout.screen_crop_uv(),
            crop,
            camera.uv_rect(),
        )
    }

    /// When the webcam bubble hides, worked out once per edit.
    fn bubble_hidden(&self) -> &[(u64, u64)] {
        self.bubble_hidden.get_or_init(|| self.find_bubble_hidden())
    }

    /// Timeline stretches where clicks, drags, scrolls, a resting pointer or the caret land on
    /// the camera bubble showing then, found on the style's own canvas size (not this
    /// evaluator's), so the preview and every export size hide it at the same moments. Only
    /// bubbles over a fitted screen: where the screen fills, its view moves with the zooms.
    fn find_bubble_hidden(&self) -> Vec<(u64, u64)> {
        use crate::bubble_hide::{hide_ranges, hits, on_bubble, probes, ENTER_MARGIN, STAY_MARGIN};
        use crate::layouts::Layout;
        let layout = &self.document.layout;
        if !layout.webcam_auto_hide {
            return Vec::new();
        }
        let Ok((width, height)) = layout.preview_dimensions() else {
            return Vec::new();
        };
        // Each bubble, with where it shows.
        let resolved = self.layouts();
        let mut bubbles: Vec<(crate::layouts::Bubble, Vec<(u64, u64)>)> = Vec::new();
        for shown in std::iter::once(&resolved.base).chain(resolved.pieces.iter().map(|p| &p.layout))
        {
            let Layout::Bubble(bubble) = shown else {
                continue;
            };
            if bubble.screen.fills(width, height) || bubbles.iter().any(|(b, _)| b == bubble) {
                continue;
            }
            let ranges = resolved.ranges_where(|l| l == shown);
            bubbles.push((bubble.clone(), ranges));
        }
        if bubbles.is_empty() {
            return Vec::new();
        }
        let unit = crate::render::layout_px_unit(width, height);
        let short_side = width.min(height) as f32;
        let (mut enter, mut stay) = (Vec::new(), Vec::new());
        for asset in self.document.assets.iter().filter(|a| a.is_recording()) {
            let clock = self.document.picture_clock(&asset.id);
            if clock.intervals().is_empty() {
                continue;
            }
            let (source_w, source_h) = match (asset.width, asset.height) {
                (w, h) if w > 0 && h > 0 => (w, h),
                _ => (1920, 1080),
            };
            let screen =
                crate::render::screen_layer_rect(width, height, layout, source_w, source_h, unit);
            let (camera_w, camera_h) = camera_size(Path::new(&asset.path));
            let activity = crate::telemetry::activity::recording_activity(Path::new(&asset.path));
            // Only what plays.
            let probes: Vec<_> = probes(&activity)
                .into_iter()
                .filter(|&(a, b, _, _)| !clock.source_range_to_edited(a, b + 1).is_empty())
                .collect();
            if probes.is_empty() {
                continue;
            }
            // Where a point of the source lands on the canvas, if it shows at all.
            let canvas_point = |t: u64, x: f64, y: f64| {
                let (ux, uy, uw, uh) = self.screen_view(&asset.id, t);
                let (u, v) = (
                    (x as f32 - ux) / uw.max(1e-6),
                    (y as f32 - uy) / uh.max(1e-6),
                );
                ((0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v)).then_some((
                    screen.0 as f32 + u * screen.2 as f32,
                    screen.1 as f32 + v * screen.3 as f32,
                ))
            };
            for (bubble, showing) in &bubbles {
                let rect = crate::render::webcam_bubble_rect(
                    width, height, camera_w, camera_h, layout, bubble,
                );
                for (margin, out) in [(ENTER_MARGIN, &mut enter), (STAY_MARGIN, &mut stay)] {
                    let found = hits(&probes, |t, x, y| {
                        canvas_point(t, x, y)
                            .is_some_and(|(px, py)| on_bubble(rect, margin * short_side, px, py))
                    });
                    let edited: Vec<(u64, u64)> = found
                        .into_iter()
                        .flat_map(|(a, b)| clock.source_range_to_edited(a, b + 1))
                        .collect();
                    // Only while this bubble is the one showing.
                    out.extend(crate::layouts::subtract(
                        &edited,
                        &crate::layouts::subtract(&[(0, u64::MAX)], showing),
                    ));
                }
            }
        }
        hide_ranges(&enter, &stay)
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
            .and_then(|a| crate::cursor::CursorTrack::load_cached(Path::new(&a.path)));
        self.cursors
            .borrow_mut()
            .insert(asset.to_string(), track.clone());
        track
    }

    /// Draws a recording's pointer at `source_us` on its clock over the screen.
    /// The keyboard shortcut pressed just before `source_us` of a recording, as a badge in a
    /// corner of the screen: bottom right, or top right when the captions or the camera are
    /// down there. Nothing when the recorder did not record the keyboard.
    fn push_shortcut_badge(&self, scene: &mut Scene, asset: &str, source_us: u64) {
        use crate::telemetry::keys::{badge_keys, recording_keys, BADGE_US};
        let layout = &self.document.layout;
        if !layout.shortcut_badges {
            return;
        }
        let Some(screen) = scene
            .layers
            .iter()
            .find(|layer| layer.role == crate::render::LayerRole::Screen)
        else {
            return;
        };
        let (sx, sy, sw, sh) = (screen.x, screen.y, screen.width, screen.height);
        let Some(recording) = self.document.asset(asset).filter(|a| a.is_recording()) else {
            return;
        };
        let keys = recording_keys(Path::new(&recording.path));
        let Some(keys) = keys.as_ref() else {
            return;
        };
        let Some((index, shortcut)) = keys.badge_at(source_us) else {
            return;
        };
        let key = (asset.to_string(), index);
        let frame = {
            let mut cache = self.shortcut_badge.borrow_mut();
            if cache.as_ref().map(|(k, _)| k) != Some(&key) {
                let text = badge_keys(shortcut, keys.platform.as_deref());
                let frame = crate::captions::rasterize_badge(&text, self.width, self.height);
                *cache = Some((key, frame));
            }
            cache.as_ref().and_then(|(_, frame)| frame.clone())
        };
        let Some(mut frame) = frame else {
            return;
        };
        // In quickly, out a little slower.
        let age = source_us - shortcut.t_us;
        let alpha = (age as f32 / 100_000.0)
            .min((BADGE_US - age) as f32 / 250_000.0)
            .clamp(0.0, 1.0);
        if alpha < 1.0 {
            for px in frame.data.chunks_exact_mut(4) {
                px[3] = (px[3] as f32 * alpha).round() as u8;
            }
        }
        let margin = (self.width.min(self.height) as f32 * 0.025).round() as u32;
        let x = (sx + sw).saturating_sub(frame.width + margin).max(sx);
        let captions = &self.document.captions;
        // A camera bubble in the screen's bottom right corner takes the badge's usual place.
        let bubble_there = scene.layers.iter().any(|l| {
            l.role == crate::render::LayerRole::Webcam
                && l.width < self.width / 2
                && l.x + l.width / 2 > sx + sw / 2
                && l.y + l.height / 2 > sy + sh / 2
        });
        let below_taken = (captions.enabled && captions.position == "bottom") || bubble_there;
        let y = if below_taken {
            sy + margin
        } else {
            (sy + sh).saturating_sub(frame.height + margin).max(sy)
        };
        scene.push_caption(frame, x, y);
    }

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
        use crate::sequence::sources::StreamSource;
        let Some(asset) = self.document.asset(&clip.asset) else {
            return Ok(None);
        };
        let source = match self.stream_source(asset, &clip.stream) {
            Ok(source) => source,
            // A recording that is gone shows nothing rather than stopping playback.
            Err(_) => return Ok(None),
        };
        Ok(match &*source {
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
            StreamSource::Video(path) => path
                .is_file()
                .then(|| PictureJob::Video(path.clone(), local_us)),
            StreamSource::Graphic(_) => self.graphic_job(clip, local_us),
            StreamSource::Image(path) => {
                if !path.is_file() {
                    return Ok(None);
                }
                Some(PictureJob::Ready(crate::media_bin::decode_image_cached(
                    path,
                    (self.decode_limit.max_width, self.decode_limit.max_height),
                )?))
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
/// output keeps full detail for smart zoom up to 2x. Sources are decoded no faster than the
/// export's frame rate: a 60 fps recording in a 30 fps export would otherwise pass every
/// other frame through the decoder pipe for nothing (the pipe is the slowest part).
fn export_decode_limit(width: u32, height: u32, fps: u32) -> DecodeLimit {
    DecodeLimit {
        max_width: width.saturating_mul(2),
        max_height: height.saturating_mul(2),
        max_rate: fps,
        interactive: false,
        yuv: false,
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

/// Work for the export's writer thread.
enum WriteJob {
    Video(u64, VideoFrame),
    /// Audio at a time: interleaved samples and the number of sample frames.
    Audio(u64, Vec<i16>, u32),
    EndAudio,
}

/// While exporting, decoders start this far (in video time) ahead of the cuts coming up:
/// about as much wall time as a decoder takes to start, at export speed. Further ahead,
/// they crowd out the ones playing (and each other) in the decoder cache.
const EXPORT_PREFETCH_US: u64 = 1_500_000;

impl ExportWriter {
    /// Whether it takes NV12 frames (from the GPU compositor) as well as BGRA.
    fn takes_nv12() -> bool {
        media_backend() == MediaBackend::Ffmpeg
    }

    /// `input`: the frames it will be given; NV12 only if [`ExportWriter::takes_nv12`].
    #[allow(clippy::too_many_arguments)]
    fn begin(
        path: &Path,
        width: u32,
        height: u32,
        fps: u32,
        sample_rate: u32,
        channels: u16,
        rate: RateControl,
        input: PixelFormat,
    ) -> Result<Self, String> {
        match media_backend() {
            MediaBackend::Native if input == PixelFormat::Bgra8888 => {
                NativeExport::begin(path, width, height, fps, sample_rate, channels, rate)
                    .map(Self::Native)
            }
            MediaBackend::Native => Err("This export writer takes BGRA frames only".into()),
            MediaBackend::Ffmpeg => FfmpegExport::begin_with_rate(
                path,
                width,
                height,
                fps,
                sample_rate,
                channels,
                rate,
                input,
            )
            .map(Self::Ffmpeg),
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
    // Every graphic's render must be there before the first frame.
    let graphics = crate::graphics::render::specs_for(
        &captured.root,
        &captured.document,
        (captured.settings.width, captured.settings.height),
    );
    crate::graphics::render::ensure(&graphics, cancel, &mut |_, _| on_progress(0, frames))
        .map_err(|message| {
            if cancel.load(Ordering::SeqCst) {
                ExportFailure::Cancelled { message }
            } else {
                ExportFailure::Native { message }
            }
        })?;
    let mixer = AudioMixer::new(&captured.root, &captured.document)
        .map_err(|message| ExportFailure::Io { message })?;
    let (sample_rate, channels) = if mixer.has_audio() {
        (SAMPLE_RATE, CHANNELS)
    } else {
        (0, 0)
    };
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
        captured.settings.fps,
    ))
    // The GPU converts frames to the encoder's YUV: a third of the bytes to read back and
    // pipe, and no conversion on the CPU.
    .with_nv12_output(ExportWriter::takes_nv12());
    let mut session = ExportWriter::begin(
        &captured.temp,
        captured.settings.width,
        captured.settings.height,
        captured.settings.fps,
        sample_rate,
        channels,
        captured.settings.rate_control(),
        evaluator.output_format(),
    )
    .map_err(|message| ExportFailure::Native { message })?;

    // The encoder is fed on a thread of its own, a couple of frames behind: while it takes
    // one frame, the next is decoded and composited.
    let (jobs, queue) = std::sync::mpsc::sync_channel::<WriteJob>(2);
    let writer = std::thread::Builder::new()
        .name("aeroedits-export-writer".into())
        .spawn(move || -> Result<ExportWriter, String> {
            for job in queue {
                match job {
                    WriteJob::Video(pts_us, frame) => session.write_video(pts_us, &frame)?,
                    WriteJob::Audio(pts_us, pcm, count) => {
                        session.write_audio(pts_us, &pcm, count, CHANNELS)?
                    }
                    WriteJob::EndAudio => session.end_audio()?,
                }
            }
            Ok(session)
        })
        .map_err(|e| ExportFailure::Native {
            message: format!("Could not start the export writer: {e}"),
        })?;
    // Stops the writer and gives back its session, or the error that stopped it.
    let stop_writer =
        |jobs: std::sync::mpsc::SyncSender<WriteJob>,
         writer: std::thread::JoinHandle<Result<ExportWriter, String>>| {
            drop(jobs);
            writer
                .join()
                .map_err(|_| "The export writer stopped unexpectedly".to_string())?
        };
    let cancelled = || ExportFailure::Cancelled {
        message: "Export cancelled".into(),
    };

    let mut audio_frame = 0u64;
    let mut audio_ended = channels == 0;
    let mut written = Ok(());
    for index in 0..frames {
        if cancel.load(Ordering::SeqCst) {
            drop(stop_writer(jobs, writer));
            return Err(cancelled());
        }
        let pts_us = frame_time_us(index, captured.settings.fps);
        if pts_us >= duration_us {
            break;
        }
        let frame = match evaluator.preview_at(pts_us) {
            Ok(frame) => frame,
            Err(message) => {
                drop(stop_writer(jobs, writer));
                return Err(ExportFailure::Native { message });
            }
        };
        // Decoders for the cuts coming up start while this stretch is exported.
        evaluator.prefetch(pts_us, EXPORT_PREFETCH_US);
        let started = std::time::Instant::now();
        // A writer that stopped has dropped its end: its error comes back when it is joined.
        if jobs.send(WriteJob::Video(pts_us, frame)).is_err() {
            break;
        }
        crate::media::profile("export encode (waiting)", started);
        let started = std::time::Instant::now();
        if channels > 0 {
            // Audio runs a second ahead of video: the macOS writer interleaves its
            // tracks and can hold video back until it has audio past that point.
            let end = ((index as u128 + 1) * SAMPLE_RATE as u128 / captured.settings.fps as u128)
                as u64
                + SAMPLE_RATE as u64;
            let end = end.min(mixer.total_frames);
            while audio_frame < end && written.is_ok() {
                if cancel.load(Ordering::SeqCst) {
                    drop(stop_writer(jobs, writer));
                    return Err(cancelled());
                }
                let count = CHUNK_FRAMES.min((end - audio_frame) as usize);
                let chunk = match mixer.read_frames(audio_frame, count) {
                    Ok(chunk) => chunk,
                    Err(message) => {
                        drop(stop_writer(jobs, writer));
                        return Err(ExportFailure::Io { message });
                    }
                };
                let pts = (audio_frame as u128 * 1_000_000 / SAMPLE_RATE as u128) as u64;
                if jobs
                    .send(WriteJob::Audio(pts, chunk, count as u32))
                    .is_err()
                {
                    written = Err(());
                }
                audio_frame += count as u64;
            }
            if written.is_ok() && !audio_ended && audio_frame >= mixer.total_frames {
                if jobs.send(WriteJob::EndAudio).is_err() {
                    written = Err(());
                }
                audio_ended = true;
            }
        }
        crate::media::profile("export audio", started);
        if written.is_err() {
            break;
        }
        on_progress(index + 1, frames);
    }
    let session = stop_writer(jobs, writer).map_err(|message| ExportFailure::Native { message })?;
    let started = std::time::Instant::now();
    session
        .finish(duration_us)
        .map_err(|message| ExportFailure::Native { message })?;
    if cancel.load(Ordering::SeqCst) {
        return Err(ExportFailure::Cancelled {
            message: "Export cancelled".into(),
        });
    }
    crate::media::profile("export finish", started);
    let started = std::time::Instant::now();
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
    // Two frames are decoded to check the file: as the preview decodes (in software on
    // Windows), which starts far sooner than a GPU decoder for so little.
    let check = DecodeLimit {
        max_width: captured.settings.width,
        max_height: captured.settings.height,
        max_rate: 0,
        interactive: true,
        yuv: false,
    };
    for pts in [0, frame_time_us(frames - 1, captured.settings.fps)] {
        let decoded = decode_h264_frame_limited(&captured.temp, pts, check)
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
    crate::media::profile("export verify", started);
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
            // No hard links there (exFAT, FAT32): move the file instead, never replacing one.
            match move_no_replace(temp, dest) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    Err(ExportFailure::Collision {
                        message: "Export destination already exists".into(),
                    })
                }
                Err(error) if error.kind() == std::io::ErrorKind::Unsupported => {
                    Err(ExportFailure::Io {
                        message: _err.to_string(),
                    })
                }
                Err(error) => Err(ExportFailure::Io {
                    message: error.to_string(),
                }),
            }
        }
    }
}

/// Renames `from` to `to` on the same volume, failing with `AlreadyExists` instead of
/// replacing a file at `to`.
fn move_no_replace(from: &Path, to: &Path) -> std::io::Result<()> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        use std::os::unix::ffi::OsStrExt;
        let c = |p: &Path| {
            std::ffi::CString::new(p.as_os_str().as_bytes())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
        };
        let (from, to) = (c(from)?, c(to)?);
        #[cfg(target_os = "macos")]
        let done = unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_EXCL) };
        #[cfg(target_os = "linux")]
        let done = unsafe {
            libc::renameat2(
                libc::AT_FDCWD,
                from.as_ptr(),
                libc::AT_FDCWD,
                to.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        if done == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_WRITE_THROUGH};
        let wide = |p: &Path| {
            p.as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect::<Vec<u16>>()
        };
        let (from, to) = (wide(from), wide(to));
        // Without MOVEFILE_REPLACE_EXISTING an existing `to` fails as ERROR_ALREADY_EXISTS.
        if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), MOVEFILE_WRITE_THROUGH) } != 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
    {
        let _ = (from, to);
        Err(std::io::ErrorKind::Unsupported.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::decode_h264_frame;

    #[test]
    fn the_hard_link_fallback_moves_without_replacing() {
        let dir = tempfile::tempdir().unwrap();
        let (temp, dest) = (dir.path().join("a.tmp"), dir.path().join("a.mp4"));
        fs::write(&temp, b"new").unwrap();
        move_no_replace(&temp, &dest).unwrap();
        assert!(!temp.exists());
        assert_eq!(fs::read(&dest).unwrap(), b"new");

        fs::write(&temp, b"newer").unwrap();
        let error = move_no_replace(&temp, &dest).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&dest).unwrap(), b"new");
        assert!(temp.exists());
    }

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
        let limit = export_decode_limit(1920, 1080, 30);
        assert_eq!(
            (limit.max_width, limit.max_height, limit.max_rate),
            (3840, 2160, 30)
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
        let red = || {
            (0..20)
                .map(|_| VideoFrame::solid(64, 64, 0, 0, 255, 0).unwrap())
                .collect()
        };
        project_with_camera(dir, webcam.then(red))
    }

    /// A 2 s recording of a screen, a microphone and, given its pictures (10 fps), a camera.
    fn project_with_camera(dir: &Path, webcam: Option<Vec<VideoFrame>>) -> PathBuf {
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
        if let Some(pictures) = webcam {
            let path = root.join("media/webcam/000001.mp4");
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            crate::media::ffmpeg::encode_bgra_mp4(&path, &pictures, 10).unwrap();
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
            layouts: crate::layouts::LayoutTrack::for_short(),
            media: None,
            edit: None,
            take: None,
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
        document.layouts = short.layouts.clone();

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
                    interactive: true,
                    yuv: false,
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
            nv12_output: false,
            wallpaper: std::cell::OnceCell::new(),
            layouts: std::cell::OnceCell::new(),
            camera_framing: Default::default(),
            full_camera: std::cell::OnceCell::new(),
            caption_cues: std::cell::OnceCell::new(),
            caption_cache: std::cell::RefCell::new(CaptionCache::default()),
            cursors: Default::default(),
            prefetched: Default::default(),
            zooms: Default::default(),
            sources: Default::default(),
            cursor_images: Default::default(),
            graphic_placeholders: Default::default(),
            shortcut_badge: Default::default(),
            window_crops: Default::default(),
            bubble_hidden: std::cell::OnceCell::new(),
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

    /// An evaluator without a GPU: scenes only.
    fn scenes(root: &Path, document: EditDocument, width: u32, height: u32) -> SceneEvaluator {
        SceneEvaluator {
            root: root.into(),
            document,
            compositor: None,
            width,
            height,
            decode_limit: DecodeLimit::NONE,
            nv12_output: false,
            wallpaper: std::cell::OnceCell::new(),
            layouts: std::cell::OnceCell::new(),
            camera_framing: Default::default(),
            full_camera: std::cell::OnceCell::new(),
            caption_cues: std::cell::OnceCell::new(),
            caption_cache: std::cell::RefCell::new(CaptionCache::default()),
            cursors: Default::default(),
            prefetched: Default::default(),
            zooms: Default::default(),
            sources: Default::default(),
            cursor_images: Default::default(),
            graphic_placeholders: Default::default(),
            shortcut_badge: Default::default(),
            window_crops: Default::default(),
            bubble_hidden: std::cell::OnceCell::new(),
        }
    }

    /// A 2x2 red picture on `count` video tracks, as overlays.
    fn overlays_document(dir: &Path, count: usize) -> EditDocument {
        let path = dir.join("logo.png");
        image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]))
            .save(&path)
            .unwrap();
        let mut document = EditDocument::default();
        document.assets = vec![serde_json::from_value(serde_json::json!({
            "id": "logo", "name": "Logo", "kind": "image", "path": path,
            "durationUs": 5_000_000, "width": 2, "height": 2,
            "streams": [{"id": "picture", "name": "Picture", "kind": "picture", "role": "overlay"}]
        }))
        .unwrap()];
        for i in 1..=count {
            let mut track =
                crate::sequence::Track::new(format!("t{i}"), crate::sequence::TrackKind::Video);
            track.clips.push(Clip {
                id: format!("c{i}"),
                asset: "logo".into(),
                stream: "picture".into(),
                start_us: 0,
                in_us: 0,
                duration_us: 5_000_000,
                link: None,
                fit: crate::sequence::Fit::Contain,
            });
            document.sequence.tracks.push(track);
        }
        document.validate().unwrap();
        document
    }

    #[test]
    fn every_video_track_and_the_captions_have_a_layer() {
        let dir = tempfile::tempdir().unwrap();
        let document = overlays_document(dir.path(), 16);
        let scene = scenes(dir.path(), document, 64, 64)
            .scene_at(1_000_000)
            .unwrap();
        let overlays = scene
            .layers
            .iter()
            .filter(|l| l.role == crate::render::LayerRole::Overlay)
            .count();
        assert_eq!(overlays, 16);
        // Stills keep their texture on the GPU from frame to frame.
        assert!(scene.layers.iter().all(|l| l.cache_key.is_some()));
        let mut scene = scene;
        let caption = scene.layers[0].frame.clone();
        scene.push_caption(caption, 0, 0);
        assert!(scene
            .layers
            .iter()
            .any(|l| l.role == crate::render::LayerRole::Caption));
    }

    #[test]
    fn a_shortcut_shows_as_a_badge_in_a_corner_of_the_screen() {
        let dir = tempfile::tempdir().unwrap();
        let recording = dir.path().join("take.aero");
        std::fs::create_dir_all(recording.join("telemetry")).unwrap();
        std::fs::write(
            recording.join("telemetry/keys.jsonl"),
            concat!(
                r#"{"version":1,"seq":0,"t_us":0,"kind":"capture","platform":"windows"}"#,
                "\n",
                r#"{"version":1,"seq":1,"t_us":2000000,"kind":"shortcut","modifiers":["ctrl"],"code":"KeyS","label":"S"}"#,
                "\n"
            ),
        )
        .unwrap();
        let mut document = EditDocument::default();
        document.assets = vec![serde_json::from_value(serde_json::json!({
            "id": "rec", "name": "Take", "kind": "recording", "path": recording,
            "durationUs": 10_000_000, "streams": []
        }))
        .unwrap()];
        document.layouts.base = crate::layouts::Layout::ScreenOnly(Default::default());
        let evaluator = scenes(dir.path(), document, 640, 360);
        let screen = || {
            let mut scene = Scene {
                width: 640,
                height: 360,
                background: [0.0; 4],
                layers: Vec::new(),
            };
            let frame = VideoFrame::solid(64, 36, 0, 0, 0, 0).unwrap();
            scene
                .layers
                .push(crate::render::Layer::placed(frame, 20, 10, 600, 340));
            scene
        };
        // Each badge's place, size and the opacity at its middle.
        let badges = |scene: &Scene| {
            scene
                .layers
                .iter()
                .filter(|l| l.role == crate::render::LayerRole::Caption)
                .map(|l| {
                    let (w, h) = (l.frame.width as usize, l.frame.height as usize);
                    let alpha = l.frame.data[(h / 2 * w + w / 2) * 4 + 3];
                    (l.x, l.y, l.width, l.height, alpha)
                })
                .collect::<Vec<_>>()
        };
        // Before the press, and long after it, nothing.
        let mut scene = screen();
        evaluator.push_shortcut_badge(&mut scene, "rec", 1_000_000);
        evaluator.push_shortcut_badge(&mut scene, "rec", 9_000_000);
        assert!(badges(&scene).is_empty());
        // Just after: bottom right of the screen, fully shown.
        let mut scene = screen();
        evaluator.push_shortcut_badge(&mut scene, "rec", 2_500_000);
        let shown = badges(&scene);
        assert_eq!(shown.len(), 1);
        let (x, y, w, h, alpha) = shown[0];
        assert_eq!(x + w, 620 - 9);
        assert_eq!(y + h, 350 - 9);
        assert!(alpha > 200, "{alpha}");
        // Fading in at the very start.
        let mut scene = screen();
        evaluator.push_shortcut_badge(&mut scene, "rec", 2_020_000);
        assert!(badges(&scene)[0].4 < alpha);
    }

    /// A transparent WebM over a red picture: red shows through where the clip is clear.
    #[test]
    fn a_transparent_video_overlay_shows_what_is_below() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let mut document = overlays_document(dir.path(), 1);
        let path = dir.path().join("graphic.webm");
        let status = std::process::Command::new(crate::media::ffmpeg::ffmpeg_path().unwrap())
            .args(["-nostdin", "-loglevel", "error", "-y", "-f", "lavfi", "-i"])
            .arg("color=c=black@0.0:s=64x64:r=10:d=1,format=rgba,drawbox=x=16:y=16:w=32:h=32:color=white:t=fill:replace=1")
            .args(["-c:v", "libvpx-vp9", "-pix_fmt", "yuva420p", "-b:v", "0", "-crf", "20"])
            .arg(&path)
            .status()
            .unwrap();
        assert!(status.success());
        document.assets.push(
            serde_json::from_value(serde_json::json!({
                "id": "graphic", "name": "Graphic", "kind": "video", "path": path,
                "durationUs": 1_000_000, "width": 64, "height": 64,
                "streams": [{"id": "picture", "name": "Picture", "kind": "picture", "role": "overlay"}]
            }))
            .unwrap(),
        );
        let mut track = crate::sequence::Track::new("t2".into(), crate::sequence::TrackKind::Video);
        track.clips.push(Clip {
            id: "g".into(),
            asset: "graphic".into(),
            stream: "picture".into(),
            start_us: 0,
            in_us: 0,
            duration_us: 1_000_000,
            link: None,
            fit: crate::sequence::Fit::Contain,
        });
        document.sequence.tracks.push(track);
        document.validate().unwrap();
        let scene = scenes(dir.path(), document, 64, 64)
            .scene_at(500_000)
            .unwrap();
        let frame = crate::render::Compositor::composite_cpu(&scene).unwrap();
        let bgra = |x: u32, y: u32| {
            let i = (y * frame.stride + x * 4) as usize;
            [frame.data[i], frame.data[i + 1], frame.data[i + 2]]
        };
        let (corner, centre) = (bgra(4, 4), bgra(32, 32));
        assert!(
            corner[2] > 230 && corner[0] < 25 && corner[1] < 25,
            "corner {corner:?}"
        );
        assert!(centre.iter().all(|&c| c > 230), "centre {centre:?}");
        crate::media::ffmpeg::release_decoders();
    }

    #[test]
    fn a_short_draws_its_overlays_and_its_own_background() {
        let dir = tempfile::tempdir().unwrap();
        let document = overlays_document(dir.path(), 1);
        let short: crate::shorts::Short = serde_json::from_value(serde_json::json!({
            "id": "s1", "title": "Short", "media": "logo",
            "sourceStartUs": 0, "sourceEndUs": 5_000_000
        }))
        .unwrap();
        let mut short_document = crate::shorts::short_document(&document, &short, false).unwrap();
        let scene = scenes(dir.path(), short_document.clone(), 64, 64)
            .scene_at(1_000_000)
            .unwrap();
        assert!(scene
            .layers
            .iter()
            .any(|l| l.role == crate::render::LayerRole::Overlay));
        // A preview kept across edits keeps its background only while the short's choice
        // stays the same.
        let key = |document: &EditDocument| {
            background_key(dir.path(), &background_layout(document), 64, 64)
        };
        let before = key(&short_document);
        let layout = short_document.short_layout.as_mut().unwrap();
        layout.background_type = "gradient".into();
        layout.background_color_start = "#0000FF".into();
        assert_ne!(key(&short_document), before);
    }

    #[test]
    fn a_short_draws_each_graphic_in_its_slot() {
        use crate::shorts::GraphicSlot;
        let dir = tempfile::tempdir().unwrap();
        let mut document = overlays_document(dir.path(), 1);
        let graphic =
            crate::graphics::create(dir.path(), "title", None, &serde_json::Map::new()).unwrap();
        let mut track =
            crate::sequence::Track::new("tg".to_string(), crate::sequence::TrackKind::Video);
        track.clips.push(Clip {
            id: "g1".into(),
            asset: graphic.id.clone(),
            stream: "picture".into(),
            start_us: 0,
            in_us: 0,
            duration_us: 3_000_000,
            link: None,
            fit: crate::sequence::Fit::Contain,
        });
        document.assets.push(graphic);
        document.sequence.tracks.push(track);
        document.validate().unwrap();
        let short: crate::shorts::Short = serde_json::from_value(serde_json::json!({
            "id": "s1", "title": "Short", "media": "logo",
            "sourceStartUs": 0, "sourceEndUs": 5_000_000
        }))
        .unwrap();
        let base = crate::shorts::short_document(&document, &short, false).unwrap();
        // The camera takes the top 35%: 56 of 160 rows.
        for (slot, rect) in [
            (GraphicSlot::Full, (0, 0, 90, 160)),
            (GraphicSlot::Camera, (0, 0, 90, 56)),
            (GraphicSlot::Screen, (0, 56, 90, 104)),
            (GraphicSlot::OverScreen, (0, 56, 90, 104)),
        ] {
            let mut short_document = base.clone();
            let layout = short_document.short_layout.as_mut().unwrap();
            layout.graphic_slots.insert("g1".into(), slot);
            let evaluator = scenes(dir.path(), short_document.clone(), 90, 160);
            let scene = evaluator.scene_at(1_000_000).unwrap();
            let last = scene.layers.last().unwrap();
            assert_eq!((last.x, last.y, last.width, last.height), rect, "{slot:?}");
            // It renders for that space, at the render scale (1080 wide).
            let spec = crate::graphics::render::specs_for(dir.path(), &short_document, (90, 160));
            assert_eq!(spec.len(), 1);
            let expected_h = (rect.3 as f64 * 1920.0 / 160.0).round() as u32 & !1;
            assert_eq!(
                (spec[0].width, spec[0].height),
                (1080, expected_h),
                "{slot:?}"
            );
        }
        // In the 16:9 video it is the whole canvas.
        let scene = scenes(dir.path(), document, 160, 90)
            .scene_at(1_000_000)
            .unwrap();
        let last = scene.layers.last().unwrap();
        assert_eq!((last.x, last.y, last.width, last.height), (0, 0, 160, 90));
    }

    #[test]
    fn only_a_graphic_over_the_full_screen_camera_eases() {
        use crate::graphics::Entrance;
        use crate::shorts::GraphicSlot;
        let dir = tempfile::tempdir().unwrap();
        let mut document = overlays_document(dir.path(), 1);
        let card =
            crate::graphics::create(dir.path(), "title", None, &serde_json::Map::new()).unwrap();
        let lower_third =
            crate::graphics::create(dir.path(), "blank", None, &serde_json::Map::new()).unwrap();
        let html_path = std::path::Path::new(&lower_third.path).join(crate::graphics::ENTRY);
        let html = std::fs::read_to_string(&html_path).unwrap();
        std::fs::write(
            &html_path,
            html.replacen(
                "data-composition-id=\"graphic\"",
                "data-composition-id=\"graphic\" data-overlay",
                1,
            ),
        )
        .unwrap();
        crate::graphics::render::sources_changed();
        let clip = |id: &str, asset: &str| Clip {
            id: id.into(),
            asset: asset.into(),
            stream: "picture".into(),
            start_us: 1_000_000,
            in_us: 0,
            duration_us: 3_000_000,
            link: None,
            fit: crate::sequence::Fit::Contain,
        };
        let (card_clip, lower_clip) = (clip("g1", &card.id), clip("g2", &lower_third.id));
        document.assets.push(card.clone());
        document.assets.push(lower_third.clone());
        let camera = [(0, 5_000_000)];
        let entrance = |document: &EditDocument, asset, clip, full_camera: &[(u64, u64)]| {
            crate::graphics::render::Spec::for_clip(
                dir.path(),
                document,
                asset,
                clip,
                (160, 90),
                full_camera,
            )
            .entrance
        };
        // The 16:9 video: the full-screen card cuts, over the camera or not.
        assert_eq!(
            entrance(&document, &card, &card_clip, &camera),
            Entrance::Cut
        );
        assert_eq!(entrance(&document, &card, &card_clip, &[]), Entrance::Cut);
        // The lower third eases over the full-screen camera, and cuts over the screen.
        assert_eq!(
            entrance(&document, &lower_third, &lower_clip, &camera),
            Entrance::Ease
        );
        assert_eq!(
            entrance(&document, &lower_third, &lower_clip, &[]),
            Entrance::Cut
        );
        // A short: the card on the whole frame still cuts; in the camera's part of the split it
        // covers only part of the frame, so over a camera scene it eases.
        let short: crate::shorts::Short = serde_json::from_value(serde_json::json!({
            "id": "s1", "title": "Short", "media": "logo",
            "sourceStartUs": 0, "sourceEndUs": 5_000_000
        }))
        .unwrap();
        let mut short_document = crate::shorts::short_document(&document, &short, false).unwrap();
        assert_eq!(
            entrance(&short_document, &card, &card_clip, &camera),
            Entrance::Cut
        );
        let layout = short_document.short_layout.as_mut().unwrap();
        layout
            .graphic_slots
            .insert("g1".into(), GraphicSlot::Camera);
        assert_eq!(
            entrance(&short_document, &card, &card_clip, &camera),
            Entrance::Ease
        );
        assert_eq!(
            entrance(&short_document, &card, &card_clip, &[]),
            Entrance::Cut
        );
        // Without a camera-only section nothing fills the frame with the camera.
        assert!(crate::graphics::full_camera_ranges(&document).is_empty());
    }

    /// Recording video is decoded as NV12 and converted on the GPU; the picture must match the
    /// BGRA FFmpeg makes itself.
    #[test]
    #[cfg_attr(
        not(target_os = "macos"),
        ignore = "needs a GPU adapter and FFmpeg; run with --ignored on a machine that has them"
    )]
    fn gpu_nv12_layers_match_bgra() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pattern.mp4");
        let status = std::process::Command::new(crate::media::ffmpeg::ffmpeg_path().unwrap())
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=640x360:rate=30",
                "-t",
                "1",
            ])
            .args(["-pix_fmt", "yuv420p", "-c:v", "libx264", "-y"])
            .arg(&path)
            .status()
            .unwrap();
        assert!(status.success());
        let limit = |yuv| DecodeLimit {
            max_width: 640,
            max_height: 360,
            max_rate: 30,
            interactive: true,
            yuv,
        };
        let compositor = Compositor::new().unwrap();
        let draw = |frame: VideoFrame| {
            let scene = Scene {
                width: 640,
                height: 360,
                background: [0.0, 0.0, 0.0, 1.0],
                layers: vec![crate::render::Layer::placed(frame, 0, 0, 640, 360)],
            };
            compositor.composite(&scene).unwrap()
        };
        let bgra = crate::media::ffmpeg::decode_bgra_limited(&path, 500_000, limit(false)).unwrap();
        let nv12 = crate::media::ffmpeg::decode_bgra_limited(&path, 500_000, limit(true)).unwrap();
        assert_eq!(nv12.format, crate::media::PixelFormat::Nv12);
        let (a, b) = (draw(bgra), draw(nv12.clone()));
        let total: u64 = a
            .data
            .chunks_exact(4)
            .zip(b.data.chunks_exact(4))
            .map(|(x, y)| (0..3).map(|c| x[c].abs_diff(y[c]) as u64).sum::<u64>())
            .sum();
        let mean = total as f64 / (640.0 * 360.0 * 3.0);
        assert!(
            mean < 3.0,
            "NV12 differs from BGRA by {mean:.2} levels on average"
        );
        // The CPU conversion (for the CPU compositor) agrees with the shader.
        let cpu = nv12.to_bgra();
        let total: u64 = cpu
            .data
            .chunks_exact(4)
            .zip(b.data.chunks_exact(4))
            .map(|(x, y)| (0..3).map(|c| x[c].abs_diff(y[c]) as u64).sum::<u64>())
            .sum();
        assert!((total as f64 / (640.0 * 360.0 * 3.0)) < 3.0);
        crate::media::ffmpeg::release_decoders();
    }

    /// Export frames are converted to the encoder's YUV on the GPU; they must match FFmpeg's
    /// own conversion of the same picture to a level. (Its default bicubic filter shrinks colour
    /// a little differently at sharp colour edges, up to 9 levels there and 0.3 on average; the
    /// GPU averages each 2x2 block, like FFmpeg's `area`.)
    #[test]
    #[cfg_attr(
        not(target_os = "macos"),
        ignore = "needs a GPU adapter and FFmpeg; run with --ignored on a machine that has them"
    )]
    fn gpu_nv12_output_matches_ffmpeg_conversion() {
        use std::io::{Read, Write};
        let (w, h) = (640usize, 360usize);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pattern.mp4");
        let ffmpeg = crate::media::ffmpeg::ffmpeg_path().unwrap();
        // Colour bars, gradients, moving shapes and text: sharp colour edges included.
        let status = std::process::Command::new(ffmpeg)
            .args(["-v", "error", "-f", "lavfi", "-i"])
            .arg(format!("testsrc2=size={w}x{h}:rate=30"))
            .args([
                "-t", "1", "-pix_fmt", "yuv444p", "-c:v", "libx264", "-qp", "0", "-y",
            ])
            .arg(&path)
            .status()
            .unwrap();
        assert!(status.success());
        let limit = DecodeLimit {
            max_width: w as u32,
            max_height: h as u32,
            max_rate: 30,
            interactive: true,
            yuv: false,
        };
        let picture = crate::media::ffmpeg::decode_bgra_limited(&path, 500_000, limit).unwrap();
        let compositor = Compositor::new().unwrap();
        let scene = Scene {
            width: w as u32,
            height: h as u32,
            background: [0.0, 0.0, 0.0, 1.0],
            layers: vec![crate::render::Layer::placed(
                picture, 0, 0, w as u32, h as u32,
            )],
        };
        let bgra = compositor.composite(&scene).unwrap();
        let nv12 = compositor.composite_nv12(&scene).unwrap();
        assert_eq!(nv12.format, crate::media::PixelFormat::Nv12);
        assert_eq!((nv12.stride as usize, nv12.data.len()), (w, w * h * 3 / 2));
        // FFmpeg's conversion, as the export ran it, with a box filter for colour.
        let mut child = std::process::Command::new(ffmpeg)
            .args([
                "-v",
                "error",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "bgra",
                "-video_size",
            ])
            .arg(format!("{w}x{h}"))
            .args(["-i", "pipe:0", "-vf"])
            .arg("scale=out_color_matrix=bt709:out_range=tv:flags=area,format=yuv420p")
            .args(["-f", "rawvideo", "pipe:1"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let pixels = bgra.data.clone();
        let feed = std::thread::spawn(move || stdin.write_all(&pixels).unwrap());
        let mut expected = Vec::new();
        child
            .stdout
            .take()
            .unwrap()
            .read_to_end(&mut expected)
            .unwrap();
        feed.join().unwrap();
        assert!(child.wait().unwrap().success());
        assert_eq!(expected.len(), w * h * 3 / 2);
        // (worst, mean) difference in levels.
        let differ = |a: &mut dyn Iterator<Item = (u8, u8)>| {
            let (mut worst, mut total, mut count) = (0u8, 0u64, 0u64);
            for (x, y) in a {
                worst = worst.max(x.abs_diff(y));
                total += x.abs_diff(y) as u64;
                count += 1;
            }
            (worst, total as f64 / count as f64)
        };
        let quarter = w * h / 4;
        let chroma = &nv12.data[w * h..];
        let luma = differ(
            &mut nv12.data[..w * h]
                .iter()
                .copied()
                .zip(expected[..w * h].iter().copied()),
        );
        let cb = differ(
            &mut chroma
                .iter()
                .step_by(2)
                .copied()
                .zip(expected[w * h..][..quarter].iter().copied()),
        );
        let cr = differ(
            &mut chroma[1..]
                .iter()
                .step_by(2)
                .copied()
                .zip(expected[w * h + quarter..].iter().copied()),
        );
        println!("luma {luma:?}, cb {cb:?}, cr {cr:?} (worst, mean levels)");
        for plane in [luma, cb, cr] {
            assert!(plane.0 <= 1 && plane.1 < 0.05, "differs: {plane:?}");
        }
    }

    /// A recording with screen and camera, opened as a project, with `telemetry` files
    /// (name, lines) written into it.
    fn telemetry_project(dir: &Path, telemetry: &[(&str, &[&str])]) -> (PathBuf, EditDocument) {
        let recording = project_with_tracks(dir, true);
        std::fs::create_dir_all(recording.join("telemetry")).unwrap();
        for (name, lines) in telemetry {
            let mut text = lines.join("\n");
            text.push('\n');
            std::fs::write(recording.join("telemetry").join(name), text).unwrap();
        }
        let (folder, reader) = opened(&recording);
        let mut document = reader.document().clone();
        document.zooms.clear();
        (folder, document)
    }

    fn screen_uv(scene: &Scene) -> (f32, f32, f32, f32) {
        let screen = scene
            .layers
            .iter()
            .find(|l| l.role == crate::render::LayerRole::Screen)
            .unwrap();
        (screen.uv_x, screen.uv_y, screen.uv_w, screen.uv_h)
    }

    const ONE_WINDOW: &str = r#"{"version":1,"seq":1,"t_us":0,"kind":"front","window":"0x1","role":"window","bounds":{"norm_x":0.0,"norm_y":0.0,"norm_w":0.5,"norm_h":0.5}}"#;

    #[test]
    fn the_screen_crops_to_the_active_window_and_zooms_inside_it() {
        let dir = tempfile::tempdir().unwrap();
        let (folder, mut document) =
            telemetry_project(dir.path(), &[("windows.jsonl", &[ONE_WINDOW])]);
        let base = document.layout.screen_crop_uv();
        let cropped = screen_uv(
            &scenes(&folder, document.clone(), 320, 180)
                .scene_at(500_000)
                .unwrap(),
        );
        // The window's top-left quarter, padded, in the screen's own shape.
        assert!(
            cropped.2 < 0.6 && cropped.0 == 0.0 && cropped.1 == 0.0,
            "{cropped:?}"
        );
        assert!((cropped.2 / cropped.3 - base.2 / base.3).abs() < 1e-3);
        // A 2x zoom on the window's middle works inside the crop and stays on its target.
        let recording = document.assets[0].id.clone();
        let zoom = crate::zoom::ZoomKeyframe {
            id: "z".into(),
            source_start_us: 0,
            source_end_us: 2_000_000,
            center_x: 0.25,
            center_y: 0.25,
            scale: 2.0,
            transition_us: 0,
            origin: crate::zoom::ZoomOrigin::Click,
            contributing_event_seqs: Vec::new(),
            source: crate::zoom::ZoomSource::Manual,
            edited_ranges: Vec::new(),
            media: Some(recording.clone()),
            fixed: true,
        };
        document.zooms = vec![zoom];
        let zoomed = screen_uv(
            &scenes(&folder, document.clone(), 320, 180)
                .scene_at(500_000)
                .unwrap(),
        );
        assert!((zoomed.2 - cropped.2 / 2.0).abs() < 1e-4, "{zoomed:?}");
        assert!((zoomed.0 + zoomed.2 / 2.0 - 0.25).abs() < 1e-4);
        // Off for the whole video, or for this stretch: the layout's own crop.
        document.zooms.clear();
        document.layout.window_crop = false;
        let off = screen_uv(
            &scenes(&folder, document.clone(), 320, 180)
                .scene_at(500_000)
                .unwrap(),
        );
        assert_eq!(off, base);
        document.layout.window_crop = true;
        document.layout.window_crop_off = vec![crate::window_crop::CropOffRange {
            media: Some(recording),
            source_start_us: 0,
            source_end_us: 2_000_000,
        }];
        let stretch = screen_uv(
            &scenes(&folder, document, 320, 180)
                .scene_at(500_000)
                .unwrap(),
        );
        assert_eq!(stretch, base);
    }

    /// A click at (x, y) of the source at `t_us`, as an old recorder wrote it.
    fn click_line(seq: u32, t_us: u64, x: f64, y: f64) -> String {
        format!(
            r#"{{"version":1,"seq":{seq},"t_us":{t_us},"geometry_id":"g1","kind":"click","norm_x":{x},"norm_y":{y},"inside_source":true,"visible":true}}"#
        )
    }

    const GEOMETRY: &str = r#"{"geometry_id":"g1","t_us":0,"physical_width":64,"physical_height":64,"scale_factor":1,"crop_x":0,"crop_y":0,"crop_width":64,"crop_height":64}"#;

    #[test]
    fn the_bubble_fades_out_while_clicks_land_under_it() {
        let dir = tempfile::tempdir().unwrap();
        // The default bubble sits bottom left; (0.08, 0.7) of the screen is under it.
        let under = click_line(0, 1_000_000, 0.08, 0.7);
        let (folder, mut document) = telemetry_project(
            dir.path(),
            &[
                ("geometry.jsonl", &[GEOMETRY]),
                ("events.jsonl", &[under.as_str()]),
            ],
        );
        let webcam_layers = |document: &EditDocument, t: u64| {
            scenes(&folder, document.clone(), 320, 180)
                .scene_at(t)
                .unwrap()
                .layers
                .iter()
                .filter(|l| l.role == crate::render::LayerRole::Webcam)
                .map(|l| l.opacity)
                .collect::<Vec<_>>()
        };
        assert_eq!(webcam_layers(&document, 200_000), vec![1.0]);
        assert!(
            webcam_layers(&document, 1_000_000).is_empty(),
            "hidden at the click"
        );
        // Fading in between.
        let fading = webcam_layers(&document, 700_000);
        assert!(
            fading.len() == 1 && fading[0] > 0.0 && fading[0] < 1.0,
            "{fading:?}"
        );
        // A click elsewhere leaves it alone; so does switching it off.
        document.layout.webcam_auto_hide = false;
        assert_eq!(webcam_layers(&document, 1_000_000), vec![1.0]);
        let elsewhere = click_line(0, 1_000_000, 0.9, 0.2);
        std::fs::create_dir_all(dir.path().join("b")).unwrap();
        let (folder, document) = telemetry_project(
            &dir.path().join("b"),
            &[
                ("geometry.jsonl", &[GEOMETRY]),
                ("events.jsonl", &[elsewhere.as_str()]),
            ],
        );
        let scene = scenes(&folder, document, 320, 180)
            .scene_at(1_000_000)
            .unwrap();
        assert!(scene
            .layers
            .iter()
            .any(|l| l.role == crate::render::LayerRole::Webcam));
    }

    #[test]
    fn clicks_sound_on_their_own_quiet_lane_that_mutes() {
        let dir = tempfile::tempdir().unwrap();
        let lines = [
            click_line(0, 1_000_000, 0.9, 0.2),
            click_line(1, 1_050_000, 0.5, 0.5),
        ];
        let (folder, mut document) = telemetry_project(
            dir.path(),
            &[
                ("geometry.jsonl", &[GEOMETRY]),
                ("events.jsonl", &[lines[0].as_str(), lines[1].as_str()]),
            ],
        );
        document.audio.sound_effects.clicks = true;
        let effects = crate::sound_effects::plan(&document);
        // Two presses 50 ms apart: one click sound.
        assert_eq!(effects.len(), 1, "{effects:?}");
        assert_eq!(effects[0].at_us, 1_000_000);
        let read = |document: &EditDocument| {
            let mixer = AudioMixer::new(&folder, document).unwrap();
            let start = 48_000 - 2_400;
            let mut samples = Vec::new();
            for chunk in 0..2 {
                samples.extend(mixer.read_frames(start + chunk * 4_800, 4_800).unwrap());
            }
            samples
        };
        let with = read(&document);
        document.audio.sound_effects.muted = true;
        assert!(crate::sound_effects::plan(&document).is_empty());
        let without = read(&document);
        let differences: Vec<i32> = with
            .iter()
            .zip(&without)
            .map(|(a, b)| (*a as i32 - *b as i32).abs())
            .collect();
        // Silent before the click, a click just after it, and quiet.
        assert!(differences[..4_800].iter().all(|&d| d == 0));
        let loudest = *differences.iter().max().unwrap();
        assert!(loudest > 300, "{loudest}");
        assert!(loudest < (0.13 * 32_767.0) as i32, "{loudest}");
    }

    /// A short of a recording whose 16:9 camera (160x90, red) has the speaker right of centre
    /// (blue, columns 88-120, a face track saying so), with the camera taking 70% of the frame.
    fn off_centre_face_short(dir: &Path) -> (PathBuf, EditDocument) {
        use crate::face::{FaceBox, FaceSample, FaceTrack};
        let frames: Vec<VideoFrame> = (0..20)
            .map(|_| {
                let mut frame = VideoFrame::solid(160, 90, 0, 0, 255, 0).unwrap();
                for y in 0..90u32 {
                    for x in 88..120u32 {
                        let i = (y * frame.stride + x * 4) as usize;
                        frame.data[i..i + 3].copy_from_slice(&[255, 0, 0]);
                    }
                }
                frame
            })
            .collect();
        let (root, reader) = opened(&project_with_camera(dir, Some(frames)));
        let base = reader.history().current.clone();
        let (asset, stream) = crate::face::camera_streams(&base).remove(0);
        let face = FaceBox {
            x: 0.55,
            y: 0.2,
            w: 0.2,
            h: 0.5,
            eye_y: 0.4,
            score: 0.9,
        };
        crate::face::save_track(
            &root,
            &FaceTrack {
                version: crate::face::TRACK_VERSION,
                asset,
                stream,
                source: String::new(),
                model: "test".into(),
                sample_us: crate::face::SAMPLE_US,
                samples: (0..8)
                    .map(|i| FaceSample {
                        t_us: i * crate::face::SAMPLE_US,
                        face: Some(face),
                    })
                    .collect(),
            },
        )
        .unwrap();
        let mut document = base.clone();
        document.sequence =
            crate::shorts::slice_sequence(&base.sequence, &base.assets, 300_000, 1_200_000);
        document.layout.aspect_ratio = "9:16".into();
        document.short_layout = Some(crate::shorts::ShortLayout::default());
        document.layouts.base = crate::layouts::Layout::SideBySide(crate::layouts::SideBySide {
            camera_first: true,
            camera_pct: 70.0,
            ..Default::default()
        });
        (root, document)
    }

    /// The document with its camera clips framed by hand, in the middle.
    fn framed_by_hand(document: &EditDocument) -> EditDocument {
        let mut manual = document.clone();
        let cameras: Vec<String> = document
            .sequence
            .tracks
            .iter()
            .flat_map(|t| &t.clips)
            .filter(|c| c.stream == "webcam")
            .map(|c| c.id.clone())
            .collect();
        assert!(!cameras.is_empty());
        for id in cameras {
            manual.layout.camera_clips.insert(id, Default::default());
        }
        manual
    }

    fn camera_uv(scene: &Scene) -> (f32, f32, f32, f32) {
        let layer = scene
            .layers
            .iter()
            .find(|l| l.role == crate::render::LayerRole::Webcam)
            .expect("a camera layer");
        // The camera is mirrored by default: the window is the same, read right to left.
        if layer.uv_w < 0.0 {
            (layer.uv_x + layer.uv_w, layer.uv_y, -layer.uv_w, layer.uv_h)
        } else {
            (layer.uv_x, layer.uv_y, layer.uv_w, layer.uv_h)
        }
    }

    /// The preview (smaller, decoding the camera at half size) and export cut a short's camera
    /// around the face identically; manual framing keeps the centre crop.
    #[test]
    fn a_shorts_camera_is_framed_on_an_off_centre_face_in_preview_and_export_alike() {
        let dir = tempfile::tempdir().unwrap();
        let (root, document) = off_centre_face_short(dir.path());
        let preview = scenes(&root, document.clone(), 360, 640).with_decode_limit(DecodeLimit {
            max_width: 80,
            max_height: 45,
            max_rate: 0,
            interactive: true,
            yuv: false,
        });
        let export = scenes(&root, document.clone(), 1080, 1920);
        let at = 400_000;
        let (px, py, pw, ph) = camera_uv(&preview.scene_at(at).unwrap());
        let (ex, ey, ew, eh) = camera_uv(&export.scene_at(at).unwrap());
        for (a, b) in [(px, ex), (py, ey), (pw, ew), (ph, eh)] {
            assert!((a - b).abs() < 1e-3, "preview {a} vs export {b}");
        }
        // The camera spot is 1080x1344: a window 0.45 of the camera's width, centred on the
        // face at 0.65 rather than on the middle.
        assert!((ew - 0.45).abs() < 0.01, "{ew}");
        assert!((ex + ew / 2.0 - 0.65).abs() < 1e-3, "{ex}");
        // That clip framed by hand, in the middle: the old centre crop.
        let manual = framed_by_hand(&document);
        let (mx, ..) = camera_uv(&scenes(&root, manual, 1080, 1920).scene_at(at).unwrap());
        assert!((mx - (1.0 - ew) / 2.0).abs() < 1e-3, "{mx}");
        crate::media::release_decoders();
    }

    /// Footage without a camera (gaming, say) has no face track to find and nothing changes.
    #[test]
    fn a_recording_without_a_camera_has_no_face_track() {
        let dir = tempfile::tempdir().unwrap();
        let (root, reader) = opened(&project_with_tracks(dir.path(), false));
        let document = reader.history().current.clone();
        assert!(crate::face::camera_streams(&document).is_empty());
        assert!(crate::face::status(&root, &document).is_empty());
        crate::face::ensure(&root, &document, true);
        assert!(!root.join(crate::face::FACES_DIR).exists());
    }

    /// The same, drawn: the middle of the camera band is the face (blue) in preview and
    /// export, where the centre crop shows the red beside it.
    #[test]
    #[cfg_attr(
        not(target_os = "macos"),
        ignore = "needs a GPU adapter and FFmpeg; run with --ignored on a machine that has them"
    )]
    fn gpu_a_shorts_camera_shows_the_off_centre_face_in_preview_and_export() {
        let dir = tempfile::tempdir().unwrap();
        let (root, document) = off_centre_face_short(dir.path());
        let compositor = Compositor::new().unwrap();
        let middle = |document: EditDocument, w: u32, h: u32, limit: DecodeLimit| {
            let scene = scenes(&root, document, w, h)
                .with_decode_limit(limit)
                .scene_at(400_000)
                .unwrap();
            let frame = compositor.composite(&scene).unwrap();
            // The middle of the camera band (the top 70%).
            let i = ((h * 35 / 100) * frame.stride + (w / 2) * 4) as usize;
            (frame.data[i], frame.data[i + 1], frame.data[i + 2])
        };
        let small = DecodeLimit {
            max_width: 80,
            max_height: 45,
            max_rate: 0,
            interactive: true,
            yuv: false,
        };
        for (b, g, r) in [
            middle(document.clone(), 360, 640, small),
            middle(document.clone(), 1080, 1920, DecodeLimit::NONE),
        ] {
            assert!(b > 180 && r < 80 && g < 80, "the face: b{b} g{g} r{r}");
        }
        let manual = framed_by_hand(&document);
        let (b, _, r) = middle(manual, 1080, 1920, DecodeLimit::NONE);
        assert!(r > 180 && b < 80, "the centre crop misses it: b{b} r{r}");
        crate::media::release_decoders();
    }

    #[test]
    fn status_json_omits_pixels() {
        let json = serde_json::to_value(ExportStatus::idle()).unwrap();
        assert!(json.get("pixels").is_none());
        assert!(json.get("samples").is_none());
        assert_eq!(json["state"], "idle");
    }
}
