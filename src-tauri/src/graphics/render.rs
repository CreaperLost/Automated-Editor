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

/// One graphic, as it plays on one canvas for one length.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Spec {
    pub asset_id: String,
    pub name: String,
    /// The graphic's folder (its source).
    pub dir: PathBuf,
    /// Where its renders go.
    pub cache: PathBuf,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub length_us: u64,
    /// How it comes on and goes off where it plays: a cut, or an ease over the camera.
    pub entrance: super::Entrance,
    /// The app's brand kits folder.
    pub brands: PathBuf,
    /// The project's brand kit, for a graphic that has not picked its own.
    pub default_brand: Option<String>,
}

/// The size a graphic renders at for a canvas of `width` x `height` (preview canvases are
/// smaller than the video's): the same shape with its long side [`RENDER_LONG_SIDE`].
pub fn render_size(width: u32, height: u32) -> (u32, u32) {
    let (w, h) = (width.max(1) as f64, height.max(1) as f64);
    let scale = RENDER_LONG_SIDE as f64 / w.max(h);
    let even = |v: f64| ((v * scale).round() as u32).max(2) & !1;
    (even(w), even(h))
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
            fps: RENDER_FPS,
            length_us: length_us.max(100_000),
            entrance: super::Entrance::Cut,
            brands: super::brands::dir(),
            default_brand: None,
        }
    }

    /// The same graphic and length rendered for a canvas of its own: `document`'s render size
    /// for `canvas` with the slot `clip_id` plays in (a part of a short's split), the
    /// document's brand kit, and how it comes on and goes off there. `full_camera` is where the
    /// camera fills the frame ([`super::full_camera_ranges`] of `document`).
    pub fn for_clip(
        root: &Path,
        document: &crate::project::revision::EditDocument,
        asset: &crate::sequence::Asset,
        clip: &crate::sequence::Clip,
        canvas: (u32, u32),
        full_camera: &[(u64, u64)],
    ) -> Self {
        let mut spec = Self::new(root, asset, canvas, clip.duration_us);
        let mut whole_frame = true;
        if let Some(short) = &document.short_layout {
            whole_frame =
                crate::shorts::graphic_slot(short, &clip.id) == crate::shorts::GraphicSlot::Full;
            let (_, _, w, h) =
                crate::shorts::graphic_rect(document, &clip.id, spec.width, spec.height);
            spec.width = (w & !1).max(2);
            spec.height = (h & !1).max(2);
        }
        // A graphic in a slot of a short covers part of the frame; on the whole frame it fills
        // it unless its composition says it is an overlay.
        spec.entrance =
            super::entrance_for(full_camera, clip, || whole_frame && !overlay_of(&spec.dir));
        spec.default_brand = document.graphics_brand.clone();
        spec
    }

    /// The kit this render uses, if it has one and the kit is there.
    fn brand(&self) -> Option<super::brands::BrandKit> {
        let own = super::read_source(&self.dir).ok()?.brand;
        let name = super::brands::effective(own.as_deref(), self.default_brand.as_deref())?;
        super::brands::load(&self.brands, name).ok()
    }

    fn prefix(&self) -> String {
        format!("{}x{}-", self.width, self.height)
    }

    fn file_name(&self, hash: &str) -> String {
        format!(
            "{}{}-{}-{}-{}.webm",
            self.prefix(),
            self.length_us / 1000,
            self.fps,
            self.entrance.as_str(),
            hash
        )
    }

    /// What names this render: the source files and everything about the output.
    fn hash(&self) -> Result<String, String> {
        let mut hasher = Sha256::new();
        hasher.update(runtime::HYPERFRAMES_VERSION.as_bytes());
        hasher.update(format!(
            "{}x{}@{}:{}:{}",
            self.width,
            self.height,
            self.fps,
            self.length_us,
            self.entrance.as_str()
        ));
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
        // The brand kit, so changing the kit (or picking another) renders it again.
        match self.brand() {
            Some(kit) => {
                hasher.update(b"brand\0");
                hasher.update(kit.name.as_bytes());
                hasher.update([0]);
                hasher.update(super::brands::fingerprint(&self.brands, &kit));
            }
            None => hasher.update(b"no brand\0"),
        }
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
    match latest(spec) {
        Some((path, length)) => Lookup::Stale(path, length),
        None => Lookup::Pending,
    }
}

/// The newest render for `spec`'s canvas, with its length.
fn latest(spec: &Spec) -> Option<(PathBuf, u64)> {
    let prefix = spec.prefix();
    std::fs::read_dir(&spec.cache)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let rest = name.strip_prefix(&prefix)?.strip_suffix(".webm")?;
            let length_ms: u64 = rest.split('-').next()?.parse().ok()?;
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((modified, entry.path(), length_ms * 1000))
        })
        .max_by_key(|(modified, _, _)| *modified)
        .map(|(_, path, length)| (path, length))
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
    // A newer request for the same graphic and canvas replaces one still waiting.
    queue
        .waiting
        .retain(|j| !(j.spec.asset_id == spec.asset_id && j.spec.prefix() == spec.prefix()));
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

/// The renders `document` needs on a canvas of `canvas` (width, height): one per graphic clip
/// on a shown video track, in the slot it plays in.
pub fn specs_for(
    root: &Path,
    document: &crate::project::revision::EditDocument,
    canvas: (u32, u32),
) -> Vec<Spec> {
    let mut specs: Vec<Spec> = Vec::new();
    let full_camera = super::full_camera_ranges(document);
    for track in document.sequence.video_tracks().filter(|t| !t.hidden) {
        for clip in &track.clips {
            let Some(asset) = document.asset(&clip.asset).filter(|a| a.is_graphic()) else {
                continue;
            };
            let spec = Spec::for_clip(root, document, asset, clip, canvas, &full_camera);
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
    copy_dir(&spec.dir, &stage)?;
    let result = (|| {
        // The render's own copy takes the canvas and length; the source keeps its own.
        let entry = stage.join(super::ENTRY);
        let html =
            std::fs::read_to_string(&entry).map_err(|e| format!("{}: {e}", entry.display()))?;
        let sized = super::size_composition(
            &html,
            spec.width,
            spec.height,
            spec.length_us as f64 / 1e6,
            spec.entrance,
        );
        std::fs::write(&entry, sized).map_err(|e| e.to_string())?;
        let source = super::read_source(&spec.dir)?;
        // The brand kit fills the colours the graphic left to it and sets the font.
        let values = match spec.brand() {
            Some(kit) => {
                super::brands::stage(&spec.brands, &kit, &stage)?;
                let declared = super::declarations(&stage).unwrap_or_default();
                super::brands::fill_variables(&source.variables, &declared, &kit)
            }
            None => source.variables,
        };
        let variables = stage.join(".aeroedits-variables.json");
        std::fs::write(
            &variables,
            serde_json::to_string(&values).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let part = work.join("out.webm");
        run_hyperframes(
            &paths, ffmpeg, ffprobe, &stage, &part, &variables, spec.fps, cancel, progress,
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
        .args(["--format", "webm", "--fps", &fps.to_string()])
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
            fps: 30,
            length_us,
            entrance: super::super::Entrance::Cut,
            brands: dir.join("brands"),
            default_brand: None,
        }
    }

    #[test]
    fn the_name_changes_with_the_brand_kit() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("index.html"),
            "<div data-composition-id=\"g\">",
        )
        .unwrap();
        std::fs::write(dir.path().join("graphic.json"), "{\"name\":\"T\"}").unwrap();
        let brands = tempfile::tempdir().unwrap();
        let mut a = spec(dir.path(), &dir.path().join("cache"), 4_000_000);
        a.brands = brands.path().to_path_buf();
        let plain = a.hash().unwrap();
        let kit = |accent: &str| super::super::brands::BrandKitInput {
            name: "Acme".into(),
            background: "#000000".into(),
            text: "#FFFFFF".into(),
            accent: accent.into(),
            ..Default::default()
        };
        super::super::brands::save(&a.brands, &kit("#FF0000")).unwrap();
        // A missing default kit is no kit.
        a.default_brand = Some("Gone".into());
        assert_eq!(a.hash().unwrap(), plain);
        a.default_brand = Some("Acme".into());
        let branded = a.hash().unwrap();
        assert_ne!(branded, plain);
        super::super::brands::save(
            &a.brands,
            &super::super::brands::BrandKitInput {
                previous: Some("Acme".into()),
                ..kit("#00FF00")
            },
        )
        .unwrap();
        assert_ne!(a.hash().unwrap(), branded, "a changed kit renders again");
        // The graphic's own "none" wins over the project's kit.
        std::fs::write(
            dir.path().join("graphic.json"),
            "{\"name\":\"T\",\"brand\":\"none\"}",
        )
        .unwrap();
        let none = a.hash().unwrap();
        assert_ne!(none, plain, "graphic.json changed");
        a.default_brand = None;
        assert_eq!(a.hash().unwrap(), none);
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
        for canvas in [(1280, 720), (540, 960)] {
            let spec = Spec::new(root.path(), &asset, canvas, 3_000_000);
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
            assert_eq!((probe.width, probe.height), (spec.width, spec.height));
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
