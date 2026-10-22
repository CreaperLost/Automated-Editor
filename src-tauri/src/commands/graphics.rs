//! Graphics: the renderer's install, the templates, and making and changing graphics in the
//! open project.
use super::{require_handle, AppState};
use crate::graphics::{self, brands, gate, library, render, runtime, templates};
use crate::project::OpenedProject;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::sync::atomic::{AtomicBool, Ordering};

/// One install at a time, and its cancel switch.
static INSTALLING: AtomicBool = AtomicBool::new(false);
static CANCEL: AtomicBool = AtomicBool::new(false);

pub fn graphics_runtime_status_impl() -> runtime::RuntimeStatus {
    runtime::status()
}

pub fn graphics_runtime_install_impl(
    progress: &mut dyn FnMut(runtime::InstallProgress),
) -> Result<runtime::RuntimeStatus, String> {
    if INSTALLING.swap(true, Ordering::SeqCst) {
        return Err("The graphics renderer is already being installed".into());
    }
    CANCEL.store(false, Ordering::SeqCst);
    let result = runtime::install(&CANCEL, progress);
    INSTALLING.store(false, Ordering::SeqCst);
    // Renders that failed for want of the renderer can run now.
    render::sources_changed();
    result
}

pub fn graphics_runtime_cancel_impl() {
    CANCEL.store(true, Ordering::SeqCst);
}

pub fn graphics_runtime_remove_impl() -> Result<runtime::RuntimeStatus, String> {
    if INSTALLING.load(Ordering::SeqCst) {
        return Err("Wait for the install to finish, or cancel it".into());
    }
    runtime::remove()
}

pub fn graphics_templates_impl() -> Vec<templates::TemplateView> {
    templates::views()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphicCreated {
    pub project: OpenedProject,
    pub asset_id: String,
    pub clip_id: String,
}

/// One graphic for the video's timeline: where it comes from, where and how long it plays.
pub struct VideoGraphic {
    pub from: ShortGraphicFrom,
    pub at_us: u64,
    /// Default: the template's or library item's own.
    pub length_us: Option<u64>,
    pub name: Option<String>,
    pub variables: Map<String, Value>,
}

/// A graphic `project_graphics_add_impl` placed.
pub struct VideoGraphicPlaced {
    pub asset_id: String,
    pub clip_id: String,
    pub name: String,
    pub length_us: u64,
    /// Where it came from, for the agent ("template key-point", "library hub").
    pub said: String,
}

/// Makes `graphics` (templates or library copies) and places each on the video's timeline,
/// all as one undo step; on an error nothing is placed and the folders made go again.
pub fn project_graphics_add_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    graphics: Vec<VideoGraphic>,
) -> Result<(OpenedProject, Vec<VideoGraphicPlaced>), String> {
    if graphics.is_empty() {
        return Err("Name a graphic to add".into());
    }
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let root = reader.root().to_path_buf();
    let mut folders = Vec::new();
    let made = (|| {
        let mut next = reader.document().clone();
        let mut placed = Vec::new();
        for (n, graphic) in graphics.iter().enumerate() {
            let at = |error: String| {
                if graphics.len() > 1 {
                    format!("Graphic {}: {error}", n + 1)
                } else {
                    error
                }
            };
            if matches!(graphic.from, ShortGraphicFrom::Video { .. }) {
                return Err(at(
                    "`graphic` copies one of the video's graphics into a take short \
                     (`short_footage`)"
                        .into(),
                ));
            }
            let (asset, length, said) = crate::under_take::make_graphic(
                &root,
                &root,
                &next,
                &graphic.from,
                graphic.length_us,
                &graphic.name,
                &graphic.variables,
            )
            .map_err(at)?;
            folders.push(graphics::folder(&root, &asset.id));
            let (document, clip_id) =
                graphics::place(&next, asset.clone(), graphic.at_us, length).map_err(at)?;
            next = document;
            let length_us = next
                .sequence
                .clip(&clip_id)
                .map_or(length, |(_, c)| c.duration_us);
            placed.push(VideoGraphicPlaced {
                asset_id: asset.id,
                clip_id,
                name: asset.name,
                length_us,
                said,
            });
        }
        let project = reader.commit_document(expected_revision, next)?;
        Ok::<_, String>((project, placed))
    })();
    let (project, placed) = match made {
        Ok(done) => done,
        Err(error) => {
            for dir in &folders {
                let _ = std::fs::remove_dir_all(dir);
            }
            return Err(error);
        }
    };
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    Ok((project, placed))
}

/// Makes a graphic from `template` and places it at `at_us`, one undo step. `length_us`
/// defaults to the template's.
#[allow(clippy::too_many_arguments)]
pub fn project_graphic_create_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    template: String,
    name: Option<String>,
    variables: Map<String, Value>,
    at_us: u64,
    length_us: Option<u64>,
) -> Result<GraphicCreated, String> {
    let found = templates::find(&template)?;
    add_graphic(
        state,
        &project_handle,
        expected_revision,
        |root| graphics::create(root, &template, name.as_deref(), &variables),
        at_us,
        length_us.unwrap_or(found.length_us),
    )
}

/// Makes a graphic in the project as a copy of library item `library_id` and places it at
/// `at_us` for `length_us` (default the length it was saved with), one undo step.
pub fn project_graphic_from_library_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    library_id: String,
    at_us: u64,
    length_us: Option<u64>,
) -> Result<GraphicCreated, String> {
    let lib = library::dir();
    let item = library::view(&lib, &library_id)?;
    let from = library::find(&lib, &library_id)?;
    add_graphic(
        state,
        &project_handle,
        expected_revision,
        |root| graphics::create_from_folder(root, &from, None),
        at_us,
        length_us.unwrap_or(item.item.length_us),
    )
}

pub use crate::under_take::GraphicFrom as ShortGraphicFrom;

/// Makes a graphic and places it on take short `short_id`'s own timeline at `at_us` (on the
/// short), one undo step. A graphic of the video is copied, so changing it in the short leaves
/// the video's alone; it renders at the short's 9:16 like any graphic there.
#[allow(clippy::too_many_arguments)]
pub fn project_short_graphic_add_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    short_id: String,
    from: ShortGraphicFrom,
    at_us: u64,
    length_us: Option<u64>,
    mode: Option<crate::shorts::GraphicSlot>,
) -> Result<GraphicCreated, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let root = reader.root().to_path_buf();
    let item = crate::under_take::Under::Graphic {
        from,
        at_us,
        length_us,
        name: None,
        variables: Map::new(),
        mode,
    };
    let done = crate::under_take::put(&root, &root, reader.document(), &short_id, &[item])?;
    let project = match reader.commit_document(expected_revision, done.document.clone()) {
        Ok(project) => project,
        Err(error) => {
            done.discard();
            return Err(error);
        }
    };
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    let (asset_id, clip_id) = done.graphics[0].clone();
    Ok(GraphicCreated {
        project,
        asset_id,
        clip_id,
    })
}

/// Makes a graphic in the project's folder with `make` and places it, one undo step; the
/// folder goes again if placing fails.
fn add_graphic(
    state: &AppState,
    project_handle: &str,
    expected_revision: u64,
    make: impl FnOnce(&std::path::Path) -> Result<crate::sequence::Asset, String>,
    at_us: u64,
    length_us: u64,
) -> Result<GraphicCreated, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, project_handle)?;
    let asset = make(reader.root())?;
    let dir = graphics::folder(reader.root(), &asset.id);
    let placed = graphics::place(reader.document(), asset.clone(), at_us, length_us).and_then(
        |(next, clip_id)| {
            reader
                .commit_document(expected_revision, next)
                .map(|project| (project, clip_id))
        },
    );
    let (project, clip_id) = match placed {
        Ok(done) => done,
        Err(error) => {
            let _ = std::fs::remove_dir_all(&dir);
            return Err(error);
        }
    };
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    Ok(GraphicCreated {
        project,
        asset_id: asset.id,
        clip_id,
    })
}

fn graphic_asset(
    reader: &crate::project::ProjectReader,
    asset_id: &str,
) -> Result<crate::sequence::Asset, String> {
    reader
        .document()
        .asset(asset_id)
        .filter(|a| a.is_graphic())
        .cloned()
        .ok_or_else(|| "That graphic is not in the project".to_string())
}

pub fn project_graphic_get_impl(
    state: &AppState,
    project_handle: String,
    asset_id: String,
) -> Result<graphics::GraphicView, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let asset = graphic_asset(reader, &asset_id)?;
    graphics::view(reader.root(), &asset)
}

/// Every graphic in the project, with where its clips play.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphicListed {
    #[serde(flatten)]
    pub graphic: graphics::GraphicView,
    /// (clip id, start, length) on the timeline.
    pub clips: Vec<(String, u64, u64)>,
}

pub fn project_graphics_list_impl(
    state: &AppState,
    project_handle: String,
) -> Result<Vec<GraphicListed>, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let document = reader.document();
    Ok(document
        .assets
        .iter()
        .filter(|a| a.is_graphic())
        .filter_map(|asset| {
            let graphic = graphics::view(reader.root(), asset).ok()?;
            let clips = document
                .sequence
                .clips()
                .filter(|(_, c)| c.asset == asset.id)
                .map(|(_, c)| (c.id.clone(), c.start_us, c.duration_us))
                .collect();
            Some(GraphicListed { graphic, clips })
        })
        .collect())
}

/// The timeline at `at_us` as it plays (graphics, webcam and captions included), as a small PNG
/// data URL: what the Graphics panel shows for each graphic.
pub fn project_frame_thumbnail_impl(
    state: &AppState,
    project_handle: String,
    at_us: u64,
    width: u32,
) -> Result<String, String> {
    {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        require_handle(reader, &project_handle)?;
    }
    let (frame, _) = crate::mcp::frames::render(state, None, at_us, width.clamp(120, 640))?;
    use base64::Engine;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(crate::mcp::frames::png(&frame)?)
    ))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphicUpdated {
    pub graphic: graphics::GraphicView,
    /// The project, when the name changed (that is an edit, with its own undo step).
    pub project: Option<OpenedProject>,
}

/// Changes a graphic's variables and brand kit (its source: no undo step; it renders again)
/// and, with `name`, its name (an edit). `brand` is `default` (the project's), `none` or a kit.
#[allow(clippy::too_many_arguments)]
pub fn project_graphic_set_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    asset_id: String,
    variables: Map<String, Value>,
    name: Option<String>,
    brand: Option<String>,
) -> Result<GraphicUpdated, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let asset = graphic_asset(reader, &asset_id)?;
    let dir = graphics::folder(reader.root(), &asset.id);
    let mut source = graphics::read_source(&dir)?;
    if !variables.is_empty() {
        let declared = graphics::declarations(&dir)?;
        source.variables = graphics::merge_variables(&declared, &source.variables, &variables)?;
    }
    let brand = brand
        .map(|b| graphics::brand_choice(&brands::dir(), &b))
        .transpose()?
        .filter(|b| *b != source.brand);
    if let Some(brand) = &brand {
        source.brand = brand.clone();
    }
    let name = name
        .map(|n| n.trim().chars().take(80).collect::<String>())
        .filter(|n| !n.is_empty() && *n != asset.name);
    let mut project = None;
    if let Some(name) = &name {
        let mut next = reader.document().clone();
        if let Some(found) = next.assets.iter_mut().find(|a| a.id == asset.id) {
            found.name = name.clone();
        }
        project = Some(reader.commit_document(expected_revision, next)?);
        source.name = name.clone();
    }
    if !variables.is_empty() || name.is_some() || brand.is_some() {
        graphics::write_source(&dir, &source)?;
    }
    if project.is_some() {
        state
            .playback
            .lock()
            .apply_document(&reader.history().current)?;
    }
    let asset = graphic_asset(reader, &asset_id)?;
    Ok(GraphicUpdated {
        graphic: graphics::view(reader.root(), &asset)?,
        project,
    })
}

/// Sets look tokens of a graphic's own over the video's (`None` takes one back to the video's).
/// Like its variables, its source changes in place (no undo step); it renders again.
pub fn project_graphic_look_set_impl(
    state: &AppState,
    project_handle: String,
    asset_id: String,
    tokens: std::collections::BTreeMap<String, Option<String>>,
) -> Result<graphics::GraphicView, String> {
    let _guard = state.command_lock.lock();
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let asset = graphic_asset(reader, &asset_id)?;
    let dir = graphics::folder(reader.root(), &asset.id);
    let mut source = graphics::read_source(&dir)?;
    for (name, value) in tokens {
        match value {
            Some(value) => source.look.insert(name, value.trim().to_string()),
            None => source.look.remove(&name),
        };
    }
    graphics::look::check_tokens(&source.look)?;
    graphics::write_source(&dir, &source)?;
    render::sources_changed();
    graphics::view(reader.root(), &asset)
}

/// Forgets failed renders and looks at the graphics' files again (after a hand edit).
pub fn graphics_render_again_impl() {
    render::sources_changed();
}

/// The brand kits, and the open project's default kit.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrandsView {
    pub kits: Vec<brands::BrandKitView>,
    /// The project's default kit (it may have been removed since).
    pub project_default: Option<String>,
    pub folder: String,
}

pub fn graphics_brands_impl(
    state: &AppState,
    project_handle: Option<String>,
) -> Result<BrandsView, String> {
    let dir = brands::dir();
    let project_default = match project_handle {
        Some(handle) => {
            let opened = state.opened_project.lock();
            let reader = opened.as_ref().ok_or("No opened project")?;
            require_handle(reader, &handle)?;
            // A kit renamed since shows by its name now.
            reader
                .document()
                .graphics_brand
                .as_deref()
                .map(|b| brands::current_name(&dir, b))
        }
        None => None,
    };
    Ok(BrandsView {
        kits: brands::list(&dir)
            .iter()
            .map(|kit| brands::view(&dir, kit))
            .collect(),
        project_default,
        folder: dir.display().to_string(),
    })
}

/// Saves a brand kit (new, changed or renamed); graphics that use it render again.
pub fn graphics_brand_save_impl(
    kit: brands::BrandKitInput,
) -> Result<brands::BrandKitView, String> {
    let dir = brands::dir();
    let saved = brands::save(&dir, &kit)?;
    Ok(brands::view(&dir, &saved))
}

pub fn graphics_brand_remove_impl(name: String) -> Result<(), String> {
    brands::remove(&brands::dir(), &name)
}

/// Sets the project's default brand kit (`None` for none), one undo step.
pub fn project_graphics_brand_set_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    brand: Option<String>,
) -> Result<OpenedProject, String> {
    // Stored by the kit's name now (an old name leads to its kit).
    let brand = brand
        .map(|b| b.trim().to_string())
        .filter(|b| !b.is_empty())
        .map(|name| brands::load(&brands::dir(), &name).map(|kit| kit.name))
        .transpose()?;
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    if reader.document().graphics_brand == brand {
        return Ok(reader.view_or_summary(None));
    }
    let mut next = reader.document().clone();
    next.graphics_brand = brand;
    let project = reader.commit_document(expected_revision, next)?;
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    // Every graphic restyles: render them all now, not as the playhead reaches each.
    render::request_all(reader.root(), &reader.history().current);
    Ok(project)
}

/// The video's look, the presets, every token's value, and the graphics with tokens of their
/// own.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LookView {
    pub look: graphics::look::Look,
    pub presets: Vec<graphics::look::PresetView>,
    pub tokens: std::collections::BTreeMap<&'static str, String>,
    /// Graphics of this video that override tokens: (name, the tokens).
    pub overrides: Vec<(String, Vec<String>)>,
}

pub fn graphics_look_impl(state: &AppState, project_handle: String) -> Result<LookView, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let document = reader.document();
    let overrides = document
        .assets
        .iter()
        .filter(|a| a.is_graphic())
        .filter_map(|asset| {
            let source = graphics::read_source(&graphics::folder(reader.root(), &asset.id)).ok()?;
            (!source.look.is_empty()).then(|| (source.name, source.look.into_keys().collect()))
        })
        .collect();
    Ok(LookView {
        look: document.graphics_look.clone(),
        presets: graphics::look::presets(),
        tokens: document.graphics_look.values(),
        overrides,
    })
}

/// Sets the video's look, one undo step; every graphic that uses it renders again.
pub fn project_graphics_look_set_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    look: graphics::look::Look,
) -> Result<OpenedProject, String> {
    look.validate()?;
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    if reader.document().graphics_look == look {
        return Ok(reader.view_or_summary(None));
    }
    let mut next = reader.document().clone();
    next.graphics_look = look;
    let project = reader.commit_document(expected_revision, next)?;
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    // Every graphic restyles: render them all now, not as the playhead reaches each.
    render::request_all(reader.root(), &reader.history().current);
    Ok(project)
}

/// How a graphic of the video would look with another look.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LookStill {
    pub asset_id: String,
    pub name: String,
    /// A PNG data URL, 640 px wide.
    pub image: String,
}

/// The most graphics a look's preview draws.
const LOOK_PREVIEW_GRAPHICS: usize = 3;

/// Stills of up to three of the video's graphics (the first of each, by where they play) with
/// `look` instead of the video's, without changing anything. Takes a few seconds.
pub fn graphics_look_preview_impl(
    state: &AppState,
    project_handle: String,
    look: graphics::look::Look,
) -> Result<Vec<LookStill>, String> {
    look.validate()?;
    let specs: Vec<(String, render::Spec)> = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        require_handle(reader, &project_handle)?;
        let mut document = reader.document().clone();
        document.graphics_look = look;
        let canvas = document.layout.preview_dimensions()?;
        let mut clips: Vec<&crate::sequence::Clip> = document
            .sequence
            .video_tracks()
            .filter(|t| !t.hidden)
            .flat_map(|t| t.clips.iter())
            .filter(|c| document.asset(&c.asset).is_some_and(|a| a.is_graphic()))
            .collect();
        clips.sort_by_key(|c| c.start_us);
        let mut seen = Vec::new();
        clips.retain(|c| {
            let new = !seen.contains(&c.asset);
            seen.push(c.asset.clone());
            new
        });
        clips
            .into_iter()
            .take(LOOK_PREVIEW_GRAPHICS)
            .filter_map(|clip| {
                let asset = document.asset(&clip.asset)?;
                let spec = render::Spec::for_clip(
                    reader.root(),
                    &document,
                    asset,
                    clip,
                    canvas,
                    &[],
                    Default::default(),
                );
                Some((asset.name.clone(), spec))
            })
            .collect()
    };
    if specs.is_empty() {
        return Err("This video has no graphics to show the look on".into());
    }
    let cancel = AtomicBool::new(false);
    let stills: Vec<Result<LookStill, String>> = std::thread::scope(|scope| {
        let jobs: Vec<_> = specs
            .iter()
            .map(|(name, spec)| {
                let cancel = &cancel;
                scope.spawn(move || {
                    // Past the entrance, before any exit.
                    let at = (spec.length_us as f64 / 1e6 * 0.6).min(2.5);
                    let png = render::still(spec, at, cancel)?;
                    let small = image::load_from_memory(&png)
                        .map_err(|e| e.to_string())?
                        .thumbnail(640, 640);
                    let mut out = std::io::Cursor::new(Vec::new());
                    small
                        .write_to(&mut out, image::ImageFormat::Png)
                        .map_err(|e| e.to_string())?;
                    use base64::Engine;
                    Ok(LookStill {
                        asset_id: spec.asset_id.clone(),
                        name: name.clone(),
                        image: format!(
                            "data:image/png;base64,{}",
                            base64::engine::general_purpose::STANDARD.encode(out.into_inner())
                        ),
                    })
                })
            })
            .collect();
        jobs.into_iter()
            .map(|job| {
                job.join()
                    .unwrap_or_else(|_| Err("The preview stopped".into()))
            })
            .collect()
    });
    let mut out = Vec::new();
    for still in stills {
        out.push(still?);
    }
    Ok(out)
}

pub fn graphics_library_impl() -> Vec<library::LibraryView> {
    library::list(&library::dir())
}

/// A project graphic as the gate and the library see it, read under the lock and used after.
struct GraphicAt {
    name: String,
    dir: std::path::PathBuf,
    cache: std::path::PathBuf,
    /// How long its first clip plays.
    length_us: u64,
    project: String,
    project_folder: Option<String>,
}

fn graphic_at(state: &AppState, project_handle: &str, asset_id: &str) -> Result<GraphicAt, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    require_handle(reader, project_handle)?;
    let asset = graphic_asset(reader, asset_id)?;
    let length_us = reader
        .document()
        .sequence
        .clips()
        .find(|(_, c)| c.asset == asset.id)
        .map_or(4_000_000, |(_, c)| c.duration_us);
    Ok(GraphicAt {
        name: graphics::read_source(&graphics::folder(reader.root(), &asset.id))
            .map(|s| s.name)
            .unwrap_or(asset.name.clone()),
        dir: graphics::folder(reader.root(), &asset.id),
        cache: graphics::cache_folder(reader.root(), &asset.id),
        length_us,
        project: reader.summary.name.clone(),
        project_folder: Some(reader.root().display().to_string()),
    })
}

/// The gate's answer, with its contact sheet as a data URL for the page.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GateView {
    #[serde(flatten)]
    pub report: gate::Report,
    pub sheet_url: Option<String>,
}

impl From<gate::Report> for GateView {
    fn from(report: gate::Report) -> Self {
        let sheet_url = report.sheet.as_deref().and_then(png_data_url);
        Self { report, sheet_url }
    }
}

fn png_data_url(path: &str) -> Option<String> {
    use base64::Engine;
    let bytes = std::fs::read(path).ok()?;
    Some(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

/// Runs the gate on a project graphic (or answers from its last run on the same files). Takes a
/// while: it renders staged copies of the graphic.
pub fn project_graphic_check_impl(
    state: &AppState,
    project_handle: String,
    asset_id: String,
    progress: &mut dyn FnMut(f64, &str),
) -> Result<gate::Report, String> {
    let at = graphic_at(state, &project_handle, &asset_id)?;
    gate::run(&at.dir, &at.cache, &AtomicBool::new(false), progress)
}

/// What saving a graphic to the library asks for.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySave {
    pub asset_id: String,
    /// Default the graphic's name.
    #[serde(default)]
    pub label: Option<String>,
    /// Its short name; default made from the label.
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub use_when: Option<String>,
    /// Default how long its first clip plays.
    #[serde(default)]
    pub length_us: Option<u64>,
    /// Save it although the gate fails, marked as not general.
    #[serde(default)]
    pub as_is: bool,
    /// A library item (id or name) this becomes the next version of.
    #[serde(default)]
    pub update: Option<String>,
    /// Save it as a new item although one is close to it.
    #[serde(default)]
    pub as_new: bool,
}

/// How a save went: saved, or why not (the gate failed, or a close item was found to update).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySaved {
    pub saved: Option<library::LibraryView>,
    pub check: GateView,
    /// A library item or bundled template much like it, when it was not saved for that.
    pub close: Option<library::CloseMatch>,
}

/// Runs the gate on a project graphic and, if it passes (or `as_is`), saves it to the library:
/// as a new item, or as the next version of `update`. A close library item or template stops a
/// new save until `update` or `as_new` says which.
pub fn project_graphic_library_save_impl(
    state: &AppState,
    project_handle: String,
    request: LibrarySave,
    progress: &mut dyn FnMut(f64, &str),
) -> Result<LibrarySaved, String> {
    let at = graphic_at(state, &project_handle, &request.asset_id)?;
    let lib = library::dir();
    if let Some(update) = &request.update {
        // Checked before the gate runs, which takes a while.
        library::view(&lib, update)?;
    }
    let report = match gate::run(&at.dir, &at.cache, &AtomicBool::new(false), progress) {
        Ok(report) => report,
        // Saved as is without the gate when the renderer can't run.
        Err(error) if request.as_is => gate::Report {
            passed: false,
            checks: vec![gate::Check {
                id: "gate".into(),
                label: "Gate".into(),
                status: gate::Status::Fail,
                reason: error,
            }],
            source_hash: gate::source_hash(&at.dir)?,
            checked_at: chrono::Utc::now().to_rfc3339(),
            seconds: 0.0,
            sheet: None,
        },
        Err(error) => return Err(error),
    };
    let label = request
        .label
        .clone()
        .filter(|l| !l.trim().is_empty())
        .unwrap_or(at.name.clone());
    if !report.passed && !request.as_is {
        return Ok(LibrarySaved {
            saved: None,
            check: report.into(),
            close: None,
        });
    }
    if request.update.is_none() && !request.as_new {
        let variables: Vec<String> = graphics::declarations(&at.dir)?
            .into_iter()
            .map(|d| d.id)
            .collect();
        if let Some(close) = library::closest(&lib, &label, &variables, None) {
            return Ok(LibrarySaved {
                saved: None,
                check: report.into(),
                close: Some(close),
            });
        }
    }
    let saved = library::save(
        &lib,
        &at.dir,
        &library::SaveRequest {
            name: request.name.clone(),
            label,
            use_when: request.use_when.clone().unwrap_or_default(),
            length_us: request.length_us.unwrap_or(at.length_us),
            project: Some(at.project),
            project_folder: at.project_folder,
            general: report.passed,
            failed: report.failed(),
            update: request.update.clone(),
            report: Some(report.clone()),
        },
    )?;
    Ok(LibrarySaved {
        saved: Some(saved),
        check: report.into(),
        close: None,
    })
}

/// A library item's contact sheet as a data URL, if it was checked.
pub fn graphics_library_sheet_impl(id: String) -> Result<Option<String>, String> {
    let item = library::view(&library::dir(), &id)?;
    Ok(item.sheet.as_deref().and_then(png_data_url))
}

pub fn graphics_library_remove_impl(id: String) -> Result<(), String> {
    library::remove(&library::dir(), &id)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::sequence::edit::SequenceEdit;

    #[test]
    fn the_shorts_tab_names_where_a_graphic_comes_from() {
        let from = |json: &str| serde_json::from_str::<ShortGraphicFrom>(json).unwrap();
        assert!(matches!(
            from(r#"{"kind":"video","graphic":"g-1"}"#),
            ShortGraphicFrom::Video { graphic } if graphic == "g-1"
        ));
        assert!(matches!(
            from(r#"{"kind":"library","id":"lib-2"}"#),
            ShortGraphicFrom::Library { id } if id == "lib-2"
        ));
        assert!(matches!(
            from(r#"{"kind":"template","template":"key-point"}"#),
            ShortGraphicFrom::Template { template } if template == "key-point"
        ));
    }

    /// A project with a 4 s sound on the timeline (needs FFmpeg for the import); returns its
    /// handle and revision, or `None` without FFmpeg.
    pub(crate) fn project_with_sound(
        state: &AppState,
        dir: &std::path::Path,
    ) -> Option<(String, u64)> {
        if !crate::media::ffmpeg::available() {
            return None;
        }
        let wav = dir.join("voice.wav");
        let samples = vec![0i16; 48_000 * 4];
        std::fs::write(
            &wav,
            crate::fixtures::generate_pcm16_wav(48_000, 1, &samples),
        )
        .unwrap();
        let folder = crate::project::folder::create_project_folder(dir, "Lesson", None).unwrap();
        let opened =
            super::super::open_project_impl(state, folder.to_string_lossy().into()).unwrap();
        let opened = super::super::project_media_import_impl(
            state,
            opened.project_handle.clone(),
            opened.revision,
            vec![wav.to_string_lossy().into()],
        )
        .unwrap();
        let asset = opened.assets[0].clone();
        let opened = super::super::project_sequence_edit_impl(
            state,
            opened.project_handle.clone(),
            opened.revision,
            SequenceEdit::PlaceAsset {
                asset_id: asset.id,
                at_us: 0,
                track_id: None,
                streams: Vec::new(),
                range: None,
            },
            None,
        )
        .unwrap();
        Some((opened.project_handle, opened.revision))
    }

    #[test]
    fn a_graphic_is_made_and_placed_on_its_own_track_in_one_step() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::new();
        let Some((handle, revision)) = project_with_sound(&state, dir.path()) else {
            return;
        };
        let created = project_graphic_create_impl(
            &state,
            handle.clone(),
            revision,
            "step".into(),
            Some("Step 2".into()),
            serde_json::json!({"step": 2, "title": "Install"})
                .as_object()
                .unwrap()
                .clone(),
            500_000,
            Some(3_000_000),
        )
        .unwrap();
        assert_eq!(created.project.revision, revision + 1);
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().unwrap();
        let document = reader.document();
        let (track, clip) = document.sequence.clip(&created.clip_id).unwrap();
        assert_eq!(track.name, graphics::TRACK_NAME);
        assert_eq!((clip.start_us, clip.duration_us), (500_000, 3_000_000));
        // On top of the other video tracks.
        assert_eq!(
            document.sequence.video_tracks().last().unwrap().id,
            track.id
        );
        let view =
            graphics::view(reader.root(), document.asset(&created.asset_id).unwrap()).unwrap();
        assert_eq!(view.name, "Step 2");
        assert_eq!(view.variables["step"], 2);
        assert_eq!(view.template.as_deref(), Some("step"));
    }
}
