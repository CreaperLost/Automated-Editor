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

/// Where a graphic for a short comes from.
pub enum ShortGraphicFrom {
    /// A bundled template, with a name and variable values.
    Template {
        template: String,
        name: Option<String>,
        variables: Map<String, Value>,
    },
    /// A copy of a library item.
    Library(String),
    /// A copy of one of the video's graphics (its look and text), made for the short.
    Video(String),
}

/// Makes a graphic and places it on take short `short_id`'s own timeline at `at_us` (on the
/// short), one undo step. A graphic of the video is copied, so changing it in the short leaves
/// the video's alone; it renders at the short's 9:16 like any graphic there.
pub fn project_short_graphic_add_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    short_id: String,
    from: ShortGraphicFrom,
    at_us: u64,
    length_us: Option<u64>,
) -> Result<GraphicCreated, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    let root = reader.root().to_path_buf();
    let (asset, length) = match &from {
        ShortGraphicFrom::Template {
            template,
            name,
            variables,
        } => (
            graphics::create(&root, template, name.as_deref(), variables)?,
            length_us.unwrap_or(templates::find(template)?.length_us),
        ),
        ShortGraphicFrom::Library(id) => {
            let lib = library::dir();
            let item = library::view(&lib, id)?;
            let folder = library::find(&lib, id)?;
            (
                graphics::create_from_folder(&root, &folder, None)?,
                length_us.unwrap_or(item.item.length_us),
            )
        }
        ShortGraphicFrom::Video(id) => {
            let source = graphic_asset(reader, id)?;
            // As long as it plays on the video, by default.
            let played = reader
                .document()
                .sequence
                .clips()
                .find(|(_, c)| c.asset == source.id)
                .map(|(_, c)| c.duration_us);
            (
                graphics::create_from_folder(
                    &root,
                    &graphics::folder(&root, &source.id),
                    Some(&source.name),
                )?,
                length_us.or(played).unwrap_or(4_000_000),
            )
        }
    };
    let dir = graphics::folder(&root, &asset.id);
    let placed = (|| {
        let mut next = reader.document().clone();
        next.assets.push(asset.clone());
        let mut shorts = next.shorts.clone();
        let short = shorts
            .iter_mut()
            .find(|s| s.id == short_id)
            .ok_or("That short no longer exists")?;
        let clip_id = crate::take_short::place_graphic(&next, short, &asset.id, at_us, length)?;
        next.shorts = shorts;
        let project = reader.commit_document(expected_revision, next)?;
        Ok::<_, String>((project, clip_id))
    })();
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
    let (frame, _) = crate::mcp::frames::render(state, at_us, width.clamp(120, 640))?;
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
            reader.document().graphics_brand.clone()
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
    let brand = brand
        .map(|b| b.trim().to_string())
        .filter(|b| !b.is_empty());
    if let Some(name) = &brand {
        brands::load(&brands::dir(), name)?;
    }
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
    Ok(project)
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
