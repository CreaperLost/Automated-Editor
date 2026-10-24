//! Styles from the editor and agents: the list, saving and removing one (app-wide), and the
//! open project's pick (one undo step).
use super::require_handle;
use crate::commands::AppState;
use crate::project::OpenedProject;
use crate::style::{self, store, StyleView};
use serde::Serialize;

/// Every style, the open project's, and what the editor offers for the graphics look.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StylesView {
    pub styles: Vec<StyleView>,
    /// The open project's style (by its name now), if a project is open.
    pub project_style: Option<String>,
    /// The graphics look's presets (app, calm, bold).
    pub look_presets: Vec<crate::graphics::look::PresetView>,
    /// Every graphics look token's name.
    pub token_names: Vec<&'static str>,
    /// The background presets a frame can use.
    pub backgrounds: Vec<&'static str>,
    /// Graphics of the open project with a style of their own or tokens of their own:
    /// (name, its style or none, the tokens it sets).
    pub own: Vec<(String, Option<String>, Vec<String>)>,
    pub folder: String,
}

pub fn styles_impl(state: &AppState, project_handle: Option<String>) -> Result<StylesView, String> {
    let dir = store::dir();
    // Made before a style of the user's own is saved, so "Open the styles folder" has one.
    let _ = std::fs::create_dir_all(&dir);
    let (project_style, own) = match project_handle {
        Some(handle) => {
            let opened = state.opened_project.lock();
            let reader = opened.as_ref().ok_or("No opened project")?;
            require_handle(reader, &handle)?;
            let document = reader.document();
            let own = document
                .assets
                .iter()
                .filter(|a| a.is_graphic())
                .filter_map(|asset| {
                    let source = crate::graphics::read_source(&crate::graphics::folder(
                        reader.root(),
                        &asset.id,
                    ))
                    .ok()?;
                    (source.style.is_some() || !source.look.is_empty())
                        .then(|| (source.name, source.style, source.look.into_keys().collect()))
                })
                .collect();
            (Some(store::current_name(&dir, &document.style)), own)
        }
        None => (None, Vec::new()),
    };
    Ok(StylesView {
        styles: store::list_in(&dir).iter().map(style::view).collect(),
        project_style,
        look_presets: crate::graphics::look::presets(),
        token_names: crate::graphics::look::token_names(),
        backgrounds: crate::project::layout::BACKGROUND_PRESETS.to_vec(),
        own,
        folder: dir.display().to_string(),
    })
}

/// Redraws the open project after a style it may use changed: the preview and every graphic.
fn restyled(state: &AppState) {
    let opened = state.opened_project.lock();
    if let Some(reader) = opened.as_ref() {
        let _ = state
            .playback
            .lock()
            .apply_document(&reader.history().current);
        crate::graphics::render::request_all(reader.root(), &reader.history().current);
    }
}

/// Saves a style (new, changed, renamed, or a copy of a built-in), app-wide. Every project
/// using it follows; the open one redraws.
pub fn style_save_impl(state: &AppState, input: store::StyleInput) -> Result<StyleView, String> {
    let saved = store::save(&input)?;
    // A renamed style the open project uses: the project follows it by its old name.
    restyled(state);
    Ok(style::view(&saved))
}

/// Changes style `name` by a JSON merge patch (see [`style::Style::patched`]), app-wide;
/// `save_as` saves the result under another name instead. A built-in never changes: the change
/// goes into a copy ("My <name>"), which the open project wears if it wore the built-in.
/// Returns the style saved and whether it is a copy.
pub fn style_patch_impl(
    state: &AppState,
    name: &str,
    patch: &serde_json::Value,
    save_as: Option<&str>,
) -> Result<(StyleView, bool), String> {
    let current = store::load(name)?;
    let mut next = current.patched(patch)?;
    let copy_name = match save_as
        .map(str::trim)
        .filter(|t| !t.is_empty() && *t != current.name)
    {
        Some(target) => Some(target.to_string()),
        None if style::is_builtin(&current.name) => {
            let taken: Vec<String> = store::list()
                .into_iter()
                .map(|s| s.name.to_lowercase())
                .collect();
            let base = format!("My {}", current.name.to_lowercase());
            let mut name = base.clone();
            let mut n = 2;
            while taken.contains(&name.to_lowercase()) {
                name = format!("{base} {n}");
                n += 1;
            }
            Some(name)
        }
        None => None,
    };
    let copied = copy_name.is_some();
    let input = match copy_name.as_deref() {
        // A copy: its files are taken from the style it copies.
        Some(target) => {
            if let Some(folder) = store::folder_of(&current) {
                let fonts = &mut next.brand.fonts;
                for file in [
                    &mut fonts.title.file,
                    &mut fonts.body.file,
                    &mut next.brand.logo,
                ]
                .into_iter()
                .flatten()
                {
                    if store::plain_file(file) {
                        *file = folder.join(&*file).display().to_string();
                    }
                }
            }
            store::StyleInput {
                name: target.to_string(),
                previous: None,
                style: next,
            }
        }
        None => store::StyleInput {
            name: current.name.clone(),
            previous: Some(current.name.clone()),
            style: next,
        },
    };
    let saved = style_save_impl(state, input)?;
    if copied && save_as.is_none() {
        let wore = {
            let opened = state.opened_project.lock();
            opened.as_ref().map(|r| {
                (
                    r.summary.project_handle.clone(),
                    r.document().revision,
                    r.document().style.eq_ignore_ascii_case(&current.name),
                )
            })
        };
        if let Some((handle, revision, true)) = wore {
            project_style_set_impl(state, handle, revision, saved.name.clone())?;
        }
    }
    Ok((saved, copied))
}

pub fn style_remove_impl(state: &AppState, name: String) -> Result<(), String> {
    store::remove(&name)?;
    restyled(state);
    Ok(())
}

/// Dresses the open project in style `name`, one undo step; every graphic renders again.
pub fn project_style_set_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    name: String,
) -> Result<OpenedProject, String> {
    let name = store::load(&name)?.name;
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    if reader.document().style == name {
        return Ok(reader.view_or_summary(None));
    }
    let mut next = reader.document().clone();
    next.style = name;
    let project = reader.commit_document(expected_revision, next)?;
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    crate::graphics::render::request_all(reader.root(), &reader.history().current);
    Ok(project)
}
