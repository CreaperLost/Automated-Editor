//! Rendering graphics in the background, one at a time (each render already runs several
//! Chrome workers), into `<project>/assets/graphics/<asset id>/`.
//!
//! A render is named by what it was made from:
//! `<w>x<h>-<length ms>-<fps>-<entrance>-<hash>.webm`, where the hash covers every file in the
//! graphic's folder, the canvas, the length, the frame rate, how it comes on and goes off (see
//! [`super::Entrance`]) and the HyperFrames version. So a change to the source, the clip's
//! length, the canvas or where it plays needs a new render, and an unchanged graphic never renders twice. Until the new one is
//! there, the last render for that canvas plays (or a placeholder). WebM VP9 keeps the page's
//! transparency, so a template can fade its card in and out over the full-screen camera.
use super::runtime;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};

/// Graphics render at this rate; playback and export read them at their own.
pub const RENDER_FPS: u32 = 30;
/// The long side of the canvas a graphic renders at; export scales it to the output.
pub const RENDER_LONG_SIDE: u32 = 1920;

/// The fastest a graphic renders for an export.
const MAX_FINAL_FPS: u32 = 60;

/// Which render a view plays: the draft the preview and storyboard use, or the final one an
/// export of `fps` frames a second needs (at its frame rate, and twice as sharp when the
/// export is larger than the draft). An export never plays a draft.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Quality {
    #[default]
    Draft,
    Final {
        fps: u32,
    },
}

/// One graphic, as it plays on one canvas for one length.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Spec {
    pub asset_id: String,
    pub name: String,
    /// The graphic's folder (its source).
    pub dir: PathBuf,
    /// Where its renders go.
    pub cache: PathBuf,
    /// The composition's size: what the graphic lays itself out in.
    pub width: u32,
    pub height: u32,
    /// Pixels per composition pixel: 2 renders the same layout at twice the size, as Chrome
    /// draws a page on a high-density screen.
    pub density: u32,
    pub fps: u32,
    pub length_us: u64,
    /// How it comes on and goes off where it plays: a cut, or an ease over the camera.
    pub entrance: super::Entrance,
    /// The style it takes (its own, else the project's), by name and as JSON.
    pub style_name: String,
    pub style_json: String,
    /// The style's graphics look as CSS, with the graphic's own tokens over it.
    pub look: String,
    /// How it shows (`data-mode` on its root): `full`, `split` beside the face in a short, or
    /// `over` a picture that shows through.
    pub mode: &'static str,
}

/// The size a graphic renders at for a canvas of `width` x `height` (preview canvases are
/// smaller than the video's): the same shape with its long side [`RENDER_LONG_SIDE`].
pub fn render_size(width: u32, height: u32) -> (u32, u32) {
    let (w, h) = (width.max(1) as f64, height.max(1) as f64);
    let scale = RENDER_LONG_SIDE as f64 / w.max(h);
    let even = |v: f64| ((v * scale).round() as u32).max(2) & !1;
    (even(w), even(h))
}

/// The canvas a graphic renders for: the edit's own when the view has its shape. Views of the
/// edit (the preview at the window's size, storyboard tiles, the export) round its shape a
/// pixel or two differently, and each rounding would be a render of its own.
pub fn shared_canvas(edit: Option<(u32, u32)>, view: (u32, u32)) -> (u32, u32) {
    let same_shape = |(a, b): (u32, u32)| {
        let (a, b, w, h) = (
            a as f64,
            b as f64,
            view.0.max(1) as f64,
            view.1.max(1) as f64,
        );
        ((a / b) / (w / h) - 1.0).abs() < 0.02
    };
    edit.filter(|edit| same_shape(*edit)).unwrap_or(view)
}

impl Spec {
    pub fn new(
        root: &Path,
        asset: &crate::sequence::Asset,
        canvas: (u32, u32),
        length_us: u64,
    ) -> Self {
        let (width, height) = render_size(canvas.0, canvas.1);
        Self {
            asset_id: asset.id.clone(),
            name: asset.name.clone(),
            dir: super::folder(root, &asset.id),
            cache: super::cache_folder(root, &asset.id),
            width,
            height,
            density: 1,
            fps: RENDER_FPS,
            length_us: length_us.max(100_000),
            entrance: super::Entrance::Cut,
            style_name: crate::style::DEFAULT.into(),
            style_json: serde_json::to_string(&crate::style::Style::default()).unwrap_or_default(),
            look: super::look::Look::default().css(&Default::default()),
            mode: "full",
        }
    }

    /// The same graphic and length rendered for a canvas of its own: `document`'s render size
    /// for `canvas` with the slot `clip_id` plays in (a part of a short's split), its style
    /// (its own or the document's), and how it comes on and goes off there. `full_camera` is where the
    /// camera fills the frame ([`super::full_camera_ranges`] of `document`). `quality` picks
    /// the draft or an export's final render.
    pub fn for_clip(
        root: &Path,
        document: &crate::project::revision::EditDocument,
        asset: &crate::sequence::Asset,
        clip: &crate::sequence::Clip,
        canvas: (u32, u32),
        full_camera: &[(u64, u64)],
        quality: Quality,
    ) -> Self {
        let shared = shared_canvas(document.layout.preview_dimensions().ok(), canvas);
        let mut spec = Self::new(root, asset, shared, clip.duration_us);
        let mut whole_frame = true;
        if let Some(short) = &document.short_layout {
            let slot = crate::shorts::graphic_slot(short, &clip.id);
            whole_frame = slot == crate::shorts::GraphicSlot::Full;
            spec.mode = slot.data_mode();
            let (_, _, w, h) =
                crate::shorts::graphic_rect(document, &clip.id, spec.width, spec.height);
            spec.width = (w & !1).max(2);
            spec.height = (h & !1).max(2);
        }
        // A graphic in a slot of a short covers part of the frame; on the whole frame it fills
        // it unless its composition says it is an overlay.
        spec.entrance =
            super::entrance_for(full_camera, clip, || whole_frame && !overlay_of(&spec.dir));
        spec.dress(
            &crate::style::of(document),
            crate::style::is_portrait(document),
        );
        if let Quality::Final { fps } = quality {
            spec.fps = fps.clamp(1, MAX_FINAL_FPS);
            // Larger than the draft: twice as sharp, where HyperFrames can (a whole 16:9 or
            // 9:16 frame; it renders only at its own sizes).
            if canvas.0.max(canvas.1) > RENDER_LONG_SIDE && density_preset(&spec).is_some() {
                spec.density = 2;
            }
        }
        spec
    }

    /// Takes `project`'s style, or the graphic's own when it picked one, on a canvas of that
    /// shape, with the graphic's own look tokens over it.
    pub fn dress(&mut self, project: &crate::style::Style, portrait: bool) {
        let source = super::read_source(&self.dir).ok();
        let style = source
            .as_ref()
            .and_then(|s| s.style.as_deref())
            .and_then(crate::style::store::cached)
            .unwrap_or_else(|| project.clone());
        let own = source.map(|s| s.look).unwrap_or_default();
        self.look = style.graphics_for(portrait).css(&own);
        self.style_name = style.name.clone();
        self.style_json = serde_json::to_string(&style).unwrap_or_default();
    }

    /// The style this render takes.
    fn style(&self) -> crate::style::Style {
        let mut style: crate::style::Style =
            serde_json::from_str(&self.style_json).unwrap_or_default();
        style.name = self.style_name.clone();
        style
    }

    /// Renders are named by their size in pixels.
    fn prefix(&self) -> String {
        format!(
            "{}x{}-",
            self.width * self.density,
            self.height * self.density
        )
    }

    /// The file name up to the source hash: everything about the output (canvas, length, fps,
    /// entrance). Two renders of one graphic with the same stem differ only in their source.
    fn stem(&self) -> String {
        format!(
            "{}{}-{}-{}-",
            self.prefix(),
            self.length_us / 1000,
            self.fps,
            self.entrance.as_str()
        )
    }

    fn file_name(&self, hash: &str) -> String {
        format!("{}{hash}.webm", self.stem())
    }

    /// Whether a newer request for `self` makes a waiting render of `other` pointless: the same
    /// graphic rendered for the same output, from older source. The same graphic at another
    /// length (the video and a short) or canvas is a different render and must stay queued.
    /// A split graphic at another size is the split moved (dragging it asks for every size on the
    /// way): only the newest size is worth making.
    fn supersedes(&self, other: &Spec) -> bool {
        let resized = self.mode == "split"
            && other.mode == "split"
            && self.length_us == other.length_us
            && self.fps == other.fps
            && self.entrance == other.entrance;
        self.asset_id == other.asset_id && (resized || self.stem() == other.stem())
    }

    /// What names this render: the source files and everything about the output.
    fn hash(&self) -> Result<String, String> {
        let mut hasher = Sha256::new();
        hasher.update(runtime::HYPERFRAMES_VERSION.as_bytes());
        hasher.update(format!(
            "{}x{}x{}@{}:{}:{}",
            self.width,
            self.height,
            self.density,
            self.fps,
            self.length_us,
            self.entrance.as_str()
        ));
        // A part of a short's split lays out as a phone frame; over a picture, without its
        // background. (Left out for the whole frame, so those renders keep their names.)
        if self.mode != "full" {
            hasher.update(format!("mode {}", self.mode));
        }
        let mut files = Vec::new();
        list_files(&self.dir, &self.dir, &mut files)?;
        files.sort();
        for (relative, path) in files {
            hasher.update(relative.as_bytes());
            hasher.update([0]);
            let bytes = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            hasher.update((bytes.len() as u64).to_le_bytes());
            hasher.update(&bytes);
        }
        // The style, so changing it (or picking another) renders it again.
        hasher.update(b"style\0");
        hasher.update(crate::style::brand::fingerprint(&self.style()));
        // The look, so changing it renders every graphic again; and how it shows.
        hasher.update(b"look\0");
        hasher.update(self.look.as_bytes());
        hasher.update(self.mode.as_bytes());
        let digest = hasher.finalize();
        Ok(digest.iter().take(8).map(|b| format!("{b:02x}")).collect())
    }
}

/// Every file under `dir` (not dot files) as (path relative to `base` with `/`, path).
pub(crate) fn list_files(
    base: &Path,
    dir: &Path,
    out: &mut Vec<(String, PathBuf)>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            list_files(base, &path, out)?;
        } else if path.is_file() {
            let relative = path
                .strip_prefix(base)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or(name);
            out.push((relative, path));
        }
        if out.len() > 2000 {
            return Err("A graphic's folder holds too many files".into());
        }
    }
    Ok(())
}

/// What plays for a graphic right now.
#[derive(Clone, Debug, PartialEq)]
pub enum Lookup {
    /// Its render for this canvas and length.
    Ready(PathBuf),
    /// An older render for this canvas (a different length or source) while the new one is
    /// made, with that render's length.
    Stale(PathBuf, u64),
    /// Nothing yet: draw a placeholder.
    Pending,
}

/// What the editor shows about a graphic's renders.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderState {
    /// idle, queued, rendering, ready or failed.
    pub state: String,
    /// 0 to 1 while rendering.
    #[serde(default)]
    pub progress: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

struct Job {
    spec: Spec,
    hash: String,
}

#[derive(Default)]
struct Queue {
    waiting: VecDeque<Job>,
    running: Option<(String, String, f64)>,
    /// Renders that failed, by file name, with why; not tried again until a source changes.
    failed: HashMap<String, String>,
    /// The latest error by asset, for the editor.
    errors: HashMap<String, String>,
    /// Renders [`ensure`] is making on its own thread, by target; the worker leaves them alone.
    elsewhere: HashSet<PathBuf>,
    worker: bool,
}

type Hashes = HashMap<Spec, Result<String, String>>;

struct Renderer {
    queue: Mutex<Queue>,
    wake: Condvar,
    /// Bumped whenever a render finishes or a source changes; lookups made before are stale.
    generation: AtomicU64,
    listener: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
    /// Hashes worked out at a generation, by spec.
    hashes: Mutex<(u64, Hashes)>,
    /// Set while the app closes or a test wants the worker to stop.
    cancel: AtomicBool,
}

fn renderer() -> &'static Renderer {
    static RENDERER: OnceLock<Renderer> = OnceLock::new();
    RENDERER.get_or_init(|| Renderer {
        queue: Mutex::new(Queue::default()),
        wake: Condvar::new(),
        generation: AtomicU64::new(1),
        listener: Mutex::new(None),
        hashes: Mutex::new((0, HashMap::new())),
        cancel: AtomicBool::new(false),
    })
}

/// Called (from the render thread) when a render finishes or fails, or a source changes, so the
/// app can redraw the preview and tell the page.
pub fn set_listener(listener: Arc<dyn Fn() + Send + Sync>) {
    *renderer()
        .listener
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = Some(listener);
}

fn notify() {
    let listener = renderer()
        .listener
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    if let Some(listener) = listener {
        listener();
    }
}

/// Bumped by every change; a caller holding lookups from an older one looks again.
pub fn generation() -> u64 {
    renderer().generation.load(Ordering::SeqCst)
}

/// A graphic's files changed (variables, an agent's write, "render again"): look again, and
/// let renders that failed be tried again.
pub fn sources_changed() {
    let r = renderer();
    {
        let mut queue = r.queue.lock().unwrap_or_else(|e| e.into_inner());
        queue.failed.clear();
        queue.errors.clear();
    }
    r.generation.fetch_add(1, Ordering::SeqCst);
    notify();
}

fn hash_of(spec: &Spec) -> Result<String, String> {
    let r = renderer();
    let now = generation();
    let mut hashes = r.hashes.lock().unwrap_or_else(|e| e.into_inner());
    if hashes.0 != now {
        *hashes = (now, HashMap::new());
    }
    if let Some(found) = hashes.1.get(spec) {
        return found.clone();
    }
    let hash = spec.hash();
    hashes.1.insert(spec.clone(), hash.clone());
    hash
}

/// Whether the graphic in `dir` says it covers only part of the frame, read once per
/// generation (a source change bumps it): placing graphics asks for every frame.
/// Where graphics fill the whole frame on `document`'s timeline, in edited time, merged: a
/// short's graphics shown full (not split or over the face), a video's unless the composition is
/// an overlay. Captions give way there, whether or not they are on.
pub fn whole_frame_ranges(
    root: &Path,
    document: &crate::project::revision::EditDocument,
) -> Vec<(u64, u64)> {
    let mut ranges: Vec<(u64, u64)> = Vec::new();
    for track in document.sequence.video_tracks().filter(|t| !t.hidden) {
        for clip in &track.clips {
            if !document.asset(&clip.asset).is_some_and(|a| a.is_graphic()) {
                continue;
            }
            let fills = match &document.short_layout {
                Some(short) => {
                    crate::shorts::graphic_slot(short, &clip.id) == crate::shorts::GraphicSlot::Full
                }
                None => !overlay_of(&super::folder(root, &clip.asset)),
            };
            if fills && clip.end_us() > clip.start_us {
                ranges.push((clip.start_us, clip.end_us()));
            }
        }
    }
    ranges.sort();
    let mut merged: Vec<(u64, u64)> = Vec::with_capacity(ranges.len());
    for (a, b) in ranges {
        match merged.last_mut() {
            Some(last) if a <= last.1 => last.1 = last.1.max(b),
            _ => merged.push((a, b)),
        }
    }
    merged
}

/// Whether `at` falls in one of `ranges` (sorted, from [`whole_frame_ranges`]).
pub fn in_ranges(ranges: &[(u64, u64)], at: u64) -> bool {
    let i = ranges.partition_point(|r| r.0 <= at);
    i > 0 && at < ranges[i - 1].1
}

/// Whether `[start, end)` meets one of `ranges`.
pub fn meets_ranges(ranges: &[(u64, u64)], start: u64, end: u64) -> bool {
    ranges.iter().any(|&(a, b)| a < end && start < b)
}

fn overlay_of(dir: &Path) -> bool {
    static OVERLAYS: OnceLock<Mutex<(u64, HashMap<PathBuf, bool>)>> = OnceLock::new();
    let now = generation();
    let mut overlays = OVERLAYS
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if overlays.0 != now {
        *overlays = (now, HashMap::new());
    }
    *overlays.1.entry(dir.to_path_buf()).or_insert_with(|| {
        std::fs::read_to_string(dir.join(super::ENTRY)).is_ok_and(|html| super::is_overlay(&html))
    })
}

/// What plays for `spec`, asking for its render when there is none (without waiting).
pub fn lookup(spec: &Spec) -> Lookup {
    let Ok(hash) = hash_of(spec) else {
        return Lookup::Pending;
    };
    let path = spec.cache.join(spec.file_name(&hash));
    if path.is_file() {
        return Lookup::Ready(path);
    }
    request(spec, &hash);
    match latest_cached(spec) {
        Some((path, length)) => Lookup::Stale(path, length),
        None => Lookup::Pending,
    }
}

/// Asks for the render of every graphic in `document` that is missing one for the preview
/// (after a change that restyles them all: the look, the style), so they render now rather
/// than when the playhead reaches each.
pub fn request_all(root: &Path, document: &crate::project::revision::EditDocument) {
    let Ok(canvas) = document.layout.preview_dimensions() else {
        return;
    };
    for spec in specs_for(root, document, canvas, Quality::Draft) {
        lookup(&spec);
    }
}

/// [`latest`], found once per generation (a render finishing or a source changing bumps it):
/// a graphic waiting for its render asks on every frame it plays.
fn latest_cached(spec: &Spec) -> Option<(PathBuf, u64)> {
    type Found = HashMap<(PathBuf, String), Option<(PathBuf, u64)>>;
    static LATEST: OnceLock<Mutex<(u64, Found)>> = OnceLock::new();
    let now = generation();
    let key = (spec.cache.clone(), spec.prefix());
    let mut found = LATEST
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if found.0 != now {
        *found = (now, HashMap::new());
    }
    match found.1.get(&key) {
        // Cleaning the cache may have taken it since.
        Some(Some((path, length))) if path.is_file() => Some((path.clone(), *length)),
        Some(None) => None,
        _ => {
            let latest = latest(spec);
            found.1.insert(key, latest.clone());
            latest
        }
    }
}

/// The newest render for `spec`'s canvas, with its length. A split graphic whose split moved has
/// none at its new size yet: the newest at its width (the split's part spans the short's width)
/// stands in, stretched a little, rather than the placeholder.
fn latest(spec: &Spec) -> Option<(PathBuf, u64)> {
    let newest = |prefix: &str, same_height: bool| {
        std::fs::read_dir(&spec.cache)
            .ok()?
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().into_owned();
                let rest = name.strip_prefix(prefix)?.strip_suffix(".webm")?;
                // `{height}-{length}-...` when only the width was matched.
                let rest = match same_height {
                    true => rest,
                    false => rest.split_once('-')?.1,
                };
                let length_ms: u64 = rest.split('-').next()?.parse().ok()?;
                let modified = entry.metadata().ok()?.modified().ok()?;
                Some((modified, entry.path(), length_ms * 1000))
            })
            .max_by_key(|(modified, _, _)| *modified)
            .map(|(_, path, length)| (path, length))
    };
    newest(&spec.prefix(), true).or_else(|| {
        (spec.mode == "split")
            .then(|| newest(&format!("{}x", spec.width * spec.density), false))
            .flatten()
    })
}

fn request(spec: &Spec, hash: &str) {
    let r = renderer();
    let name = spec.file_name(hash);
    let mut queue = r.queue.lock().unwrap_or_else(|e| e.into_inner());
    // Two graphics from the same template and values share a name, but each has its own cache.
    if queue.failed.contains_key(&name)
        || queue.elsewhere.contains(&spec.cache.join(&name))
        || queue
            .running
            .as_ref()
            .is_some_and(|(id, n, _)| *id == spec.asset_id && *n == name)
        || queue
            .waiting
            .iter()
            .any(|j| j.spec.asset_id == spec.asset_id && j.spec.file_name(&j.hash) == name)
    {
        return;
    }
    // A newer request for the same graphic and output replaces one still waiting.
    queue.waiting.retain(|j| !spec.supersedes(&j.spec));
    queue.waiting.push_back(Job {
        spec: spec.clone(),
        hash: hash.to_string(),
    });
    if !queue.worker {
        queue.worker = true;
        std::thread::Builder::new()
            .name("graphics-render".into())
            .spawn(work)
            .ok();
    }
    r.wake.notify_all();
}

fn work() {
    let r = renderer();
    loop {
        let job = {
            let mut queue = r.queue.lock().unwrap_or_else(|e| e.into_inner());
            loop {
                if let Some(job) = queue.waiting.pop_front() {
                    // Made (or being made) by an export or graphic_render since it was asked for.
                    let target = job.spec.cache.join(job.spec.file_name(&job.hash));
                    if queue.elsewhere.contains(&target) || target.is_file() {
                        continue;
                    }
                    queue.running = Some((
                        job.spec.asset_id.clone(),
                        job.spec.file_name(&job.hash),
                        0.0,
                    ));
                    break job;
                }
                queue = r.wake.wait(queue).unwrap_or_else(|e| e.into_inner());
            }
        };
        let name = job.spec.file_name(&job.hash);
        let result = render_now(&job.spec, &name, &r.cancel, &mut |fraction| {
            let mut queue = r.queue.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(running) = queue.running.as_mut() {
                running.2 = fraction;
            }
            drop(queue);
            notify();
        });
        {
            let mut queue = r.queue.lock().unwrap_or_else(|e| e.into_inner());
            queue.running = None;
            match result {
                Ok(()) => {
                    queue.errors.remove(&job.spec.asset_id);
                }
                Err(error) => {
                    queue.failed.insert(name, error.clone());
                    queue.errors.insert(job.spec.asset_id.clone(), error);
                }
            }
        }
        r.generation.fetch_add(1, Ordering::SeqCst);
        r.wake.notify_all();
        notify();
    }
}

pub fn state_of(asset_id: &str) -> RenderState {
    let queue = renderer().queue.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((id, _, progress)) = &queue.running {
        if id == asset_id {
            return RenderState {
                state: "rendering".into(),
                progress: *progress,
                error: None,
            };
        }
    }
    if queue.waiting.iter().any(|j| j.spec.asset_id == asset_id) {
        return RenderState {
            state: "queued".into(),
            ..Default::default()
        };
    }
    if let Some(error) = queue.errors.get(asset_id) {
        return RenderState {
            state: "failed".into(),
            progress: 0.0,
            error: Some(error.clone()),
        };
    }
    RenderState {
        state: "idle".into(),
        ..Default::default()
    }
}

/// HyperFrames' name for rendering `spec`'s composition at twice its size, if it has one.
fn density_preset(spec: &Spec) -> Option<&'static str> {
    match (spec.width, spec.height) {
        (1920, 1080) => Some("landscape-4k"),
        (1080, 1920) => Some("portrait-4k"),
        _ => None,
    }
}

/// The renders `document` needs on a canvas of `canvas` (width, height): one per graphic clip
/// on a shown video track, in the slot it plays in, at `quality`.
pub fn specs_for(
    root: &Path,
    document: &crate::project::revision::EditDocument,
    canvas: (u32, u32),
    quality: Quality,
) -> Vec<Spec> {
    let mut specs: Vec<Spec> = Vec::new();
    let full_camera = super::full_camera_ranges(document);
    for track in document.sequence.video_tracks().filter(|t| !t.hidden) {
        for clip in &track.clips {
            let Some(asset) = document.asset(&clip.asset).filter(|a| a.is_graphic()) else {
                continue;
            };
            let spec = Spec::for_clip(root, document, asset, clip, canvas, &full_camera, quality);
            if !specs.contains(&spec) {
                specs.push(spec);
            }
        }
    }
    specs
}

/// Renders whatever of `specs` is missing, waiting for each (for export). Reports the overall
/// fraction done.
pub fn ensure(
    specs: &[Spec],
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(f64, &str),
) -> Result<(), String> {
    let todo = missing(specs)?;
    let count = todo.len();
    // Renders made here, so a second graphic with the same source copies the first's.
    let mut made: Vec<PathBuf> = Vec::new();
    for (index, (spec, name)) in todo.into_iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err("Export cancelled".into());
        }
        let label = format!("Rendering graphic \"{}\"", spec.name);
        progress(index as f64 / count as f64, &label);
        // Wait for the queue's render of it if it is running now, else make it here, with the
        // worker kept off it until this one is done.
        let target = spec.cache.join(&name);
        let _claim = claim(&name, &target, cancel);
        if target.is_file() {
            continue;
        }
        if let Some(twin) = made.iter().find(|p| p.ends_with(&name) && p.is_file()) {
            if copy_render(twin, &spec, &name).is_ok() {
                made.push(target);
                continue;
            }
        }
        render_now(&spec, &name, cancel, &mut |fraction| {
            progress((index as f64 + fraction) / count as f64, &label)
        })
        .map_err(|e| format!("Graphic \"{}\" did not render: {e}", spec.name))?;
        made.push(target);
        renderer().generation.fetch_add(1, Ordering::SeqCst);
    }
    Ok(())
}

/// The renders of `specs` not in their caches yet, each once, with their file names. Two
/// graphics from the same template and values share a name but not a cache: both are needed.
fn missing(specs: &[Spec]) -> Result<Vec<(Spec, String)>, String> {
    let mut todo: Vec<(Spec, String)> = Vec::new();
    for spec in specs {
        let name = spec.file_name(&spec.hash()?);
        let path = spec.cache.join(&name);
        if !path.is_file() && !todo.iter().any(|(s, n)| s.cache.join(n) == path) {
            todo.push((spec.clone(), name));
        }
    }
    Ok(todo)
}

/// Copies another graphic's identical render into `spec`'s cache as `name`.
fn copy_render(from: &Path, spec: &Spec, name: &str) -> Result<(), String> {
    std::fs::create_dir_all(&spec.cache).map_err(|e| format!("{}: {e}", spec.cache.display()))?;
    let landing = spec.cache.join(format!(".landing-copy-{name}"));
    std::fs::copy(from, &landing).map_err(|e| format!("{}: {e}", landing.display()))?;
    std::fs::rename(&landing, spec.cache.join(name)).map_err(|e| e.to_string())?;
    remove_superseded(spec, name);
    Ok(())
}

/// `ensure`'s hold on `target` while it makes it: the worker neither starts it nor is asked for
/// it, so two renders never share it.
struct Claim(PathBuf);

impl Drop for Claim {
    fn drop(&mut self) {
        let r = renderer();
        r.queue
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .elsewhere
            .remove(&self.0);
        r.wake.notify_all();
    }
}

/// Waits for the worker's render of `name` if it is running, then claims `target`, in one hold
/// of the lock so the worker can't start it in between.
fn claim(name: &str, target: &Path, cancel: &AtomicBool) -> Claim {
    let r = renderer();
    let mut queue = r.queue.lock().unwrap_or_else(|e| e.into_inner());
    while (queue.running.as_ref().is_some_and(|(_, n, _)| n == name)
        || queue.elsewhere.contains(target))
        && !cancel.load(Ordering::Relaxed)
    {
        queue = r
            .wake
            .wait_timeout(queue, std::time::Duration::from_millis(250))
            .unwrap_or_else(|e| e.into_inner())
            .0;
    }
    queue.elsewhere.insert(target.to_path_buf());
    Claim(target.to_path_buf())
}

/// Renders `spec` into its cache as `name`, now, on this thread.
fn render_now(
    spec: &Spec,
    name: &str,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(f64),
) -> Result<(), String> {
    let paths = runtime::paths()?;
    let ffmpeg = crate::media::ffmpeg::ffmpeg_path()?;
    let ffprobe = crate::media::ffmpeg::ffprobe_path()?;
    std::fs::create_dir_all(&spec.cache).map_err(|e| format!("{}: {e}", spec.cache.display()))?;
    // Staged and rendered in a short folder of the runtime's: HyperFrames and Chrome add their
    // own temp names to these paths, and inside a project they passed Windows' 260 characters.
    let id: String = name
        .trim_end_matches(".webm")
        .rsplit('-')
        .next()
        .unwrap_or(name)
        .chars()
        .take(12)
        .collect();
    // Its own folder per render, never one named by the graphic alone: a second render of the
    // same graphic (this app's or another instance's) clearing that folder took the first's
    // output from under HyperFrames as it assembled.
    static RENDERS: AtomicU64 = AtomicU64::new(0);
    let works = runtime::runtime_dir().join("work");
    sweep_stale_work(&works);
    let work = works.join(format!(
        "{id}-{}-{}",
        std::process::id(),
        RENDERS.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&work);
    let stage = work.join("s");
    let result = (|| {
        let (_, variables) = stage_render(spec, &stage, cancel)?;
        let part = work.join("out.webm");
        let resolution = (spec.density > 1).then(|| density_preset(spec)).flatten();
        run_hyperframes(
            &paths, ffmpeg, ffprobe, &stage, &part, &variables, spec.fps, resolution, cancel,
            progress,
        )?;
        // Into the project under a temporary name first, so a render is whole or absent.
        let target = spec.cache.join(name);
        let landing = spec.cache.join(format!(
            ".landing-{}.webm",
            work.file_name().unwrap_or_default().to_string_lossy()
        ));
        if std::fs::rename(&part, &landing).is_err() {
            // Another drive: copy instead.
            std::fs::copy(&part, &landing).map_err(|e| format!("{}: {e}", landing.display()))?;
        }
        std::fs::rename(&landing, &target).map_err(|e| format!("{}: {e}", target.display()))?;
        remove_superseded(spec, name);
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&work);
    result
}

/// Copies `spec`'s graphic into `stage` as it renders: sized to its canvas and length, with
/// the video's look, GSAP beside it when it loads it, and its style. Returns the values of
/// its variables (the kit's colours where the graphic left them) and the file they are in.
fn stage_render(
    spec: &Spec,
    stage: &Path,
    cancel: &AtomicBool,
) -> Result<(serde_json::Map<String, serde_json::Value>, PathBuf), String> {
    copy_dir(&spec.dir, stage)?;
    // The render's own copy takes the canvas and length; the source keeps its own.
    let entry = stage.join(super::ENTRY);
    let html = std::fs::read_to_string(&entry).map_err(|e| format!("{}: {e}", entry.display()))?;
    let sized = super::size_composition(
        &html,
        spec.width,
        spec.height,
        spec.length_us as f64 / 1e6,
        spec.entrance,
    );
    // The video's look, after the composition's own defaults for the same tokens.
    let sized = super::look::inject_css(&sized, &spec.look);
    let sized = super::with_root_attribute(&sized, "data-mode", spec.mode);
    let split = |css: &str| match spec.mode {
        "split" => super::as_split_part(css, spec.width, spec.height),
        _ => css.to_string(),
    };
    std::fs::write(&entry, split(&sized)).map_err(|e| e.to_string())?;
    // Its own style sheets beside it lay out for the split part the same way.
    if spec.mode == "split" {
        for sheet in std::fs::read_dir(stage)
            .map_err(|e| e.to_string())?
            .flatten()
        {
            let path = sheet.path();
            if path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("css"))
            {
                if let Ok(css) = std::fs::read_to_string(&path) {
                    std::fs::write(&path, split(&css)).map_err(|e| e.to_string())?;
                }
            }
        }
    }
    // GSAP, when the composition loads it, beside it (fetched once).
    super::gsap::stage(stage, cancel)?;
    let source = super::read_source(&spec.dir)?;
    // The style's brand fills the colours the graphic left to it and sets the fonts.
    let style = spec.style();
    let own_colors = ["text", "background"]
        .iter()
        .any(|id| source.variables.contains_key(*id));
    crate::style::brand::stage(&style, stage, own_colors)?;
    let declared = super::declarations(stage).unwrap_or_default();
    let values = crate::style::brand::fill_variables(&source.variables, &declared, &style);
    let variables = stage.join(".aeroedits-variables.json");
    std::fs::write(
        &variables,
        serde_json::to_string(&values).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok((values, variables))
}

/// One still of `spec`'s graphic `at_s` seconds in, as a PNG: what it would look like with its
/// look and kit, without rendering it (a look's preview). Takes a few seconds.
pub fn still(spec: &Spec, at_s: f64, cancel: &AtomicBool) -> Result<Vec<u8>, String> {
    let paths = runtime::paths()?;
    static STILLS: AtomicU64 = AtomicU64::new(0);
    let work = runtime::runtime_dir().join("work").join(format!(
        "still-{}-{}",
        std::process::id(),
        STILLS.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&work);
    let stage = work.join("s");
    let result = (|| {
        let (values, _) = stage_render(spec, &stage, cancel)?;
        // A snapshot takes no variables file: they go in as the declarations' defaults.
        let entry = stage.join(super::ENTRY);
        let html =
            std::fs::read_to_string(&entry).map_err(|e| format!("{}: {e}", entry.display()))?;
        std::fs::write(&entry, super::with_defaults(&html, &values)?).map_err(|e| e.to_string())?;
        let shots = work.join("shots");
        let at = format!("{:.2}", at_s.clamp(0.0, spec.length_us as f64 / 1e6));
        super::gate::snapshot(&paths, &stage, &at, &shots, cancel)?;
        let png = std::fs::read_dir(&shots)
            .map_err(|e| e.to_string())?
            .flatten()
            .map(|e| e.path())
            .find(|p| p.extension().is_some_and(|x| x == "png"))
            .ok_or("The graphic did not draw")?;
        std::fs::read(&png).map_err(|e| e.to_string())
    })();
    let _ = std::fs::remove_dir_all(&work);
    result
}

/// Removes render folders left by a crash or a killed app: none takes a day. The gate's own
/// folders are its to clear.
fn sweep_stale_work(works: &Path) {
    let Ok(entries) = std::fs::read_dir(works) else {
        return;
    };
    let day = std::time::Duration::from_secs(24 * 60 * 60);
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().starts_with("gate-") {
            continue;
        }
        let old = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age > day);
        if old {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
}

/// Renders of other lengths kept per canvas: a graphic used at several lengths (in the video and
/// a short, or twice) needs each, and trimming it leaves old ones behind.
const KEPT_LENGTHS: usize = 8;

/// After `name` landed: older renders of the same canvas, length and frame rate are no longer
/// needed. Other lengths stay (another clip may play them), the newest [`KEPT_LENGTHS`] of them.
fn remove_superseded(spec: &Spec, name: &str) {
    let Ok(entries) = std::fs::read_dir(&spec.cache) else {
        return;
    };
    let same = format!(
        "{}{}-{}-{}-",
        spec.prefix(),
        spec.length_us / 1000,
        spec.fps,
        spec.entrance.as_str()
    );
    let mut others = Vec::new();
    for entry in entries.flatten() {
        let other = entry.file_name().to_string_lossy().into_owned();
        if other == name || !other.starts_with(&spec.prefix()) || !other.ends_with(".webm") {
            continue;
        }
        if other.starts_with(&same) {
            let _ = std::fs::remove_file(entry.path());
        } else if let Ok(modified) = entry.metadata().and_then(|m| m.modified()) {
            others.push((modified, entry.path()));
        }
    }
    others.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, path) in others.into_iter().skip(KEPT_LENGTHS) {
        let _ = std::fs::remove_file(path);
    }
}

#[allow(clippy::too_many_arguments)]
fn run_hyperframes(
    paths: &runtime::RuntimePaths,
    ffmpeg: &Path,
    ffprobe: &Path,
    project: &Path,
    output: &Path,
    variables: &Path,
    fps: u32,
    resolution: Option<&str>,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(f64),
) -> Result<(), String> {
    let mut command = Command::new(&paths.node);
    command
        .arg(&paths.cli)
        .arg("render")
        .arg(project)
        .arg("--output")
        .arg(output)
        .args(["--format", "webm", "--fps", &fps.to_string()]);
    // The same layout at a larger size: Chrome draws at a higher device pixel ratio.
    if let Some(resolution) = resolution {
        command.args(["--resolution", resolution]);
    }
    command
        .arg("--variables-file")
        .arg(variables)
        .arg("--frames-cache-dir")
        .arg(paths.cache.join("frames"))
        .env("HYPERFRAMES_BROWSER_PATH", &paths.chrome)
        .env("HYPERFRAMES_FFMPEG_PATH", ffmpeg)
        .env("HYPERFRAMES_FFPROBE_PATH", ffprobe)
        .env("HYPERFRAMES_FONT_CACHE_DIR", paths.cache.join("fonts"))
        .env("HYPERFRAMES_NO_TELEMETRY", "1")
        .env("HYPERFRAMES_SKIP_SKILLS", "1")
        .env("PATH", runtime::render_path()?)
        .current_dir(project)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    runtime::hide_window(&mut command);
    let mut child = command
        .spawn()
        .map_err(|e| format!("Could not start the graphics renderer: {e}"))?;
    let stderr = child.stderr.take();
    let errors = std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(mut stderr) = stderr {
            let _ = stderr.read_to_string(&mut text);
        }
        text
    });
    // Progress lines come on stdout: `@hf-progress {"pct":42,...}`.
    let stdout = child.stdout.take();
    let (lines_tx, lines_rx) = std::sync::mpsc::channel::<String>();
    let reader = std::thread::spawn(move || {
        if let Some(stdout) = stdout {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if lines_tx.send(line).is_err() {
                    break;
                }
            }
        }
    });
    let mut tail: VecDeque<String> = VecDeque::new();
    let status = loop {
        while let Ok(line) = lines_rx.try_recv() {
            if let Some(json) = line.strip_prefix("@hf-progress ") {
                if let Some(pct) = serde_json::from_str::<serde_json::Value>(json)
                    .ok()
                    .and_then(|v| v["pct"].as_f64())
                {
                    progress((pct / 100.0).clamp(0.0, 1.0));
                }
            } else if !line.contains("[INFO]") && !line.trim().is_empty() {
                tail.push_back(line);
                if tail.len() > 12 {
                    tail.pop_front();
                }
            }
        }
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Cancelled".into());
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    let _ = reader.join();
    while let Ok(line) = lines_rx.try_recv() {
        if !line.starts_with("@hf-progress") && !line.contains("[INFO]") && !line.trim().is_empty()
        {
            tail.push_back(line);
        }
    }
    let errors = errors.join().unwrap_or_default();
    if !status.success() || !output.is_file() {
        let mut why: Vec<String> = errors
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.contains("[INFO]"))
            .map(String::from)
            .collect();
        why.extend(tail);
        let why: Vec<String> = why
            .into_iter()
            .rev()
            .take(8)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        return Err(strip_ansi(&why.join("\n")));
    }
    Ok(())
}

/// HyperFrames' lint of a graphic's source (for compositions an agent wrote): its findings, or
/// "0 errors, 0 warnings".
pub fn lint(dir: &Path) -> Result<String, String> {
    let paths = runtime::paths()?;
    let mut command = Command::new(&paths.node);
    command
        .arg(&paths.cli)
        .arg("lint")
        .arg(dir)
        .env("HYPERFRAMES_BROWSER_PATH", &paths.chrome)
        .env("HYPERFRAMES_NO_TELEMETRY", "1")
        .env("HYPERFRAMES_SKIP_SKILLS", "1")
        .env("NO_COLOR", "1")
        .env("PATH", runtime::render_path()?)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    runtime::hide_window(&mut command);
    let output = command
        .output()
        .map_err(|e| format!("Could not run the graphics linter: {e}"))?;
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    let lines: Vec<String> = strip_ansi(&text)
        .lines()
        .map(|l| l.trim_start_matches(['◆', '◇', ' ']).trim_end().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with("Linting "))
        .collect();
    Ok(lines.join("\n"))
}

pub(crate) fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out.trim().to_string()
}

/// Copies every file under `from` (not dot files) to `to`.
pub(crate) fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    let mut files = Vec::new();
    list_files(from, from, &mut files)?;
    for (relative, path) in files {
        let target = to.join(&relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::copy(&path, &target).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(dir: &Path, cache: &Path, length_us: u64) -> Spec {
        Spec {
            asset_id: "g1".into(),
            name: "Title".into(),
            dir: dir.to_path_buf(),
            cache: cache.to_path_buf(),
            width: 1920,
            height: 1080,
            density: 1,
            fps: 30,
            length_us,
            entrance: super::super::Entrance::Cut,
            style_name: "Light".into(),
            style_json: serde_json::to_string(&crate::style::Style::default()).unwrap(),
            look: String::new(),
            mode: "full",
        }
    }

    #[test]
    fn a_newer_request_replaces_only_the_same_output() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("cache");
        let video = spec(dir.path(), &cache, 4_000_000);
        // The same graphic and output, newer source: the waiting one is pointless.
        assert!(video.supersedes(&spec(dir.path(), &cache, 4_000_000)));
        // The same graphic and canvas at another length (the video and a short) both render.
        assert!(!video.supersedes(&spec(dir.path(), &cache, 6_000_000)));
        let mut other_canvas = spec(dir.path(), &cache, 4_000_000);
        other_canvas.width = 1080;
        other_canvas.height = 1920;
        assert!(!video.supersedes(&other_canvas));
        // A split graphic at a new size is the split moved: the older size is dropped.
        let split = |height| Spec {
            width: 1080,
            height,
            mode: "split",
            ..spec(dir.path(), &cache, 4_000_000)
        };
        assert!(split(1248).supersedes(&split(1152)));
        assert!(!split(1248).supersedes(&other_canvas));
        let mut eased = spec(dir.path(), &cache, 4_000_000);
        eased.entrance = super::super::Entrance::Ease;
        assert!(!video.supersedes(&eased));
        let mut other_graphic = spec(dir.path(), &cache, 4_000_000);
        other_graphic.asset_id = "g2".into();
        assert!(!video.supersedes(&other_graphic));
    }

    #[test]
    fn the_name_changes_with_the_style() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("index.html"),
            "<div data-composition-id=\"g\">",
        )
        .unwrap();
        std::fs::write(dir.path().join("graphic.json"), "{\"name\":\"T\"}").unwrap();
        let mut a = spec(dir.path(), &dir.path().join("cache"), 4_000_000);
        let light = a.hash().unwrap();
        let dark = crate::style::builtin().remove(1);
        a.dress(&dark, false);
        let darker = a.hash().unwrap();
        assert_ne!(darker, light);
        let mut changed = dark.clone();
        changed.brand.colors.accent = "#00FF00".into();
        a.dress(&changed, false);
        assert_ne!(a.hash().unwrap(), darker, "a changed style renders again");
        // On a tall canvas the look is scaled.
        a.dress(&dark, true);
        assert_ne!(a.hash().unwrap(), darker);
        // The graphic's own style wins over the project's.
        std::fs::write(
            dir.path().join("graphic.json"),
            "{\"name\":\"T\",\"style\":\"Light\"}",
        )
        .unwrap();
        a.dress(&dark, false);
        assert_eq!(a.style_name, "Light");
    }

    #[test]
    fn every_view_of_the_edit_asks_for_one_render() {
        // A short's storyboard tile (176 x 312) used to render at 1082 x 1920 and its export
        // at 1080 x 1920: the same graphic twice.
        let short = Some((1080, 1920));
        for view in [
            (176, 312),
            (176, 313),
            (607, 1080),
            (1080, 1920),
            (2160, 3840),
        ] {
            assert_eq!(
                render_size_of(shared_canvas(short, view)),
                (1080, 1920),
                "{view:?}"
            );
        }
        // A view of another shape keeps its own.
        assert_eq!(shared_canvas(short, (1920, 1080)), (1920, 1080));
        assert_eq!(shared_canvas(None, (176, 312)), (176, 312));
    }

    fn render_size_of(canvas: (u32, u32)) -> (u32, u32) {
        render_size(canvas.0, canvas.1)
    }

    #[test]
    fn render_sizes_keep_the_canvas_shape() {
        assert_eq!(render_size(1280, 720), (1920, 1080));
        assert_eq!(render_size(3840, 2160), (1920, 1080));
        assert_eq!(render_size(540, 960), (1080, 1920));
        assert_eq!(render_size(1080, 1080), (1920, 1920));
    }

    #[test]
    fn the_name_changes_with_the_source_length_and_canvas() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("index.html"),
            "<div data-composition-id=\"g\">",
        )
        .unwrap();
        std::fs::write(dir.path().join("graphic.json"), "{\"name\":\"T\"}").unwrap();
        let cache = dir.path().join("cache");
        let a = spec(dir.path(), &cache, 4_000_000);
        let first = a.hash().unwrap();
        assert_eq!(
            a.hash().unwrap(),
            first,
            "the same source names the same render"
        );
        let longer = spec(dir.path(), &cache, 5_000_000);
        assert_ne!(longer.hash().unwrap(), first);
        let mut portrait = a.clone();
        (portrait.width, portrait.height) = (1080, 1920);
        assert_ne!(portrait.hash().unwrap(), first);
        std::fs::write(
            dir.path().join("graphic.json"),
            "{\"name\":\"T\",\"variables\":{\"title\":\"B\"}}",
        )
        .unwrap();
        assert_ne!(a.hash().unwrap(), first);
        assert!(a.file_name(&first).starts_with("1920x1080-4000-30-cut-"));
        // An ease over the camera is a render of its own, beside the cut one.
        let mut eased = a.clone();
        eased.entrance = super::super::Entrance::Ease;
        assert_ne!(eased.hash().unwrap(), a.hash().unwrap());
        assert!(eased.file_name("h").starts_with("1920x1080-4000-30-ease-"));
    }

    #[test]
    fn an_older_render_for_the_canvas_stands_in() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("cache");
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(cache.join("1920x1080-3500-30-aaaa.webm"), b"x").unwrap();
        std::fs::write(cache.join("1080x1920-3500-30-bbbb.webm"), b"x").unwrap();
        let found = latest(&spec(dir.path(), &cache, 4_000_000)).unwrap();
        assert!(found.0.ends_with("1920x1080-3500-30-aaaa.webm"));
        assert_eq!(found.1, 3_500_000);
    }

    fn graphic_folder(root: &Path, id: &str) -> PathBuf {
        let dir = root.join(id);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("index.html"), "<div data-composition-id=\"g\">").unwrap();
        std::fs::write(dir.join("graphic.json"), "{\"name\":\"T\"}").unwrap();
        dir
    }

    #[test]
    fn the_same_template_twice_renders_into_both_caches() {
        let root = tempfile::tempdir().unwrap();
        let a = spec(
            &graphic_folder(root.path(), "g1"),
            &root.path().join("cache1"),
            4_000_000,
        );
        let mut b = spec(
            &graphic_folder(root.path(), "g2"),
            &root.path().join("cache2"),
            4_000_000,
        );
        b.asset_id = "g2".into();
        let todo = missing(&[a.clone(), b.clone(), a.clone()]).unwrap();
        assert_eq!(todo.len(), 2, "each graphic needs its own render");
        assert_eq!(
            todo[0].1, todo[1].1,
            "the same source names the same render"
        );
        // The second copies the first's render.
        let made = a.cache.join(&todo[0].1);
        std::fs::create_dir_all(&a.cache).unwrap();
        std::fs::write(&made, b"webm").unwrap();
        copy_render(&made, &b, &todo[1].1).unwrap();
        assert!(missing(&[a, b]).unwrap().is_empty());
    }

    #[test]
    fn the_worker_is_not_asked_for_a_render_graphic_render_is_making() {
        let root = tempfile::tempdir().unwrap();
        let mut a = spec(
            &graphic_folder(root.path(), "claimed"),
            &root.path().join("cache"),
            4_000_000,
        );
        a.asset_id = "claimed".into();
        let hash = a.hash().unwrap();
        let target = a.cache.join(a.file_name(&hash));
        let queued = || {
            renderer()
                .queue
                .lock()
                .unwrap()
                .waiting
                .iter()
                .any(|j| j.spec.asset_id == "claimed")
        };
        let held = claim(&a.file_name(&hash), &target, &AtomicBool::new(false));
        request(&a, &hash);
        assert!(
            !queued(),
            "the preview's ask waits for graphic_render's render"
        );
        drop(held);
        assert!(!renderer().queue.lock().unwrap().elsewhere.contains(&target));
    }

    #[test]
    fn a_render_keeps_the_other_lengths_on_its_canvas() {
        let dir = tempfile::tempdir().unwrap();
        let cache = dir.path().join("cache");
        std::fs::create_dir_all(&cache).unwrap();
        for name in [
            "1920x1080-4000-30-cut-aaaa.webm",
            "1920x1080-8000-30-cut-old0.webm",
            "1920x1080-8000-30-cut-new0.webm",
            "1920x1080-8000-30-ease-eeee.webm",
            "1080x1920-8000-30-cut-cccc.webm",
        ] {
            std::fs::write(cache.join(name), b"x").unwrap();
        }
        remove_superseded(
            &spec(dir.path(), &cache, 8_000_000),
            "1920x1080-8000-30-cut-new0.webm",
        );
        let mut left: Vec<String> = std::fs::read_dir(&cache)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(
            left,
            [
                "1080x1920-8000-30-cut-cccc.webm",
                "1920x1080-4000-30-cut-aaaa.webm",
                "1920x1080-8000-30-cut-new0.webm",
                "1920x1080-8000-30-ease-eeee.webm",
            ],
            "only the older render of the same length and entrance goes"
        );
    }

    /// A real render through the installed runtime: `AEROEDITS_GRAPHICS_RUNTIME=<installed
    /// runtime> cargo test --lib graphics_render_real -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn graphics_render_real() {
        let root = tempfile::tempdir().unwrap();
        let variables = serde_json::json!({"title": "Rendered by AeroEdits"});
        let asset =
            super::super::create(root.path(), "title", None, variables.as_object().unwrap())
                .unwrap();
        let cancel = AtomicBool::new(false);
        // The draft for two canvases, then a 4K export's final render: the 1080p layout at
        // twice the density, 60 frames a second.
        // Easing in, so the first frame is see-through (a cut shows the card at once).
        let draft = |canvas| Spec {
            entrance: super::super::Entrance::Ease,
            ..Spec::new(root.path(), &asset, canvas, 3_000_000)
        };
        let final_4k = Spec {
            density: 2,
            fps: 60,
            ..draft((1920, 1080))
        };
        for spec in [draft((1280, 720)), draft((540, 960)), final_4k] {
            let canvas = (spec.width * spec.density, spec.height * spec.density);
            let started = std::time::Instant::now();
            ensure(std::slice::from_ref(&spec), &cancel, &mut |f, label| {
                eprintln!("{label}: {:.0}%", f * 100.0)
            })
            .unwrap();
            eprintln!(
                "{canvas:?} rendered in {:.1} s",
                started.elapsed().as_secs_f64()
            );
            let Lookup::Ready(path) = lookup(&spec) else {
                panic!("no render for {canvas:?}");
            };
            let probe = crate::media::ffmpeg::probe_video(&path).unwrap();
            assert_eq!((probe.width, probe.height), canvas);
            eprintln!("{canvas:?} at {} fps: {}", spec.fps, path.display());
            if let Some(out) = std::env::var_os("AEROEDITS_GRAPHICS_OUT") {
                let out = Path::new(&out).join(path.file_name().unwrap());
                std::fs::copy(&path, out).unwrap();
            }
            let frame = crate::media::ffmpeg::decode_bgra_limited(
                &path,
                1_500_000,
                crate::media::ffmpeg::DecodeLimit::NONE,
            )
            .unwrap();
            // The card covers the frame mid-way: opaque in the middle.
            let mid = ((frame.height / 2 * frame.stride) + frame.width / 2 * 4) as usize;
            assert!(frame.data[mid + 3] > 240, "alpha {}", frame.data[mid + 3]);
            let first = crate::media::ffmpeg::decode_bgra_limited(
                &path,
                0,
                crate::media::ffmpeg::DecodeLimit::NONE,
            )
            .unwrap();
            assert!(
                first.data[mid + 3] < 40,
                "the first frame should be see-through"
            );
        }
    }

    #[test]
    fn progress_noise_is_stripped_from_errors() {
        assert_eq!(strip_ansi("\u{1b}[31mBad\u{1b}[0m thing"), "Bad thing");
    }
}
