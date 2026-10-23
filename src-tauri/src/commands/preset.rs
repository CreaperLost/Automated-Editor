//! Presets from the editor and agents: the list, the open project's preset and what it changed
//! from it, following another (one undo step), taking changes back, and saving presets
//! (app-wide), from the project's settings or as written.
use super::require_handle;
use crate::commands::AppState;
use crate::preset::{self, store, PresetView};
use crate::project::OpenedProject;
use serde::Serialize;
use serde_json::{Map, Value};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetsView {
    pub presets: Vec<PresetView>,
    /// The open project's preset and its shorts' preset, if a project is open.
    pub project_preset: Option<String>,
    pub shorts_preset: Option<String>,
    /// What the open project changed from its preset, by part, as a merge patch (what is always
    /// the project's own left out).
    pub changes: Map<String, Value>,
    /// How many values that is.
    pub changed: usize,
    /// The preset's value of every setting, by part, to put one back.
    pub inherited: Map<String, Value>,
    pub folder: String,
}

pub fn presets_impl(
    state: &AppState,
    project_handle: Option<String>,
) -> Result<PresetsView, String> {
    let (project_preset, shorts_preset, changes, changed, inherited) = match project_handle {
        Some(handle) => {
            let opened = state.opened_project.lock();
            let reader = opened.as_ref().ok_or("No opened project")?;
            require_handle(reader, &handle)?;
            let document = reader.document();
            let mut changes = preset::changes(document);
            preset::strip_own(&mut changes);
            (
                Some(preset::of(document).name),
                Some(preset::store::resolve(&document.shorts_preset).name),
                changes,
                preset::changed_count(document),
                preset::of(document).inherited(),
            )
        }
        None => (None, None, Map::new(), 0, Map::new()),
    };
    Ok(PresetsView {
        presets: store::list().iter().map(preset::view).collect(),
        project_preset,
        shorts_preset,
        changes,
        changed,
        inherited,
        folder: store::dir().display().to_string(),
    })
}

/// Commits `next` as one undo step and redraws.
fn commit(
    state: &AppState,
    project_handle: &str,
    expected_revision: u64,
    edit: impl FnOnce(
        &crate::project::revision::EditDocument,
    ) -> Result<crate::project::revision::EditDocument, String>,
) -> Result<OpenedProject, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, project_handle)?;
    let next = edit(reader.document())?;
    if &next == reader.document() {
        return Ok(reader.view_or_summary(None));
    }
    next.validate()?;
    let project = reader.commit_document(expected_revision, next)?;
    state
        .playback
        .lock()
        .apply_document(&reader.history().current)?;
    Ok(project)
}

/// The project follows preset `name` (or its shorts do, with `shorts`), one undo step: what it
/// changed stays changed. Following another preset also dresses it in that preset's style when
/// it still wears its old preset's.
pub fn project_preset_set_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    name: String,
    shorts: bool,
) -> Result<OpenedProject, String> {
    let preset = store::load(&name)?;
    commit(state, &project_handle, expected_revision, |document| {
        if shorts {
            let mut next = document.clone();
            next.shorts_preset = preset.name.clone();
            return Ok(next);
        }
        let old = preset::of(document);
        let mut next = preset::switch(document, &preset.name)?;
        next.preset = preset.name.clone();
        if document.style == old.style && !preset.style.is_empty() {
            next.style = crate::style::store::load(&preset.style)
                .map(|s| s.name)
                .unwrap_or_else(|_| document.style.clone());
        }
        Ok(next)
    })
}

/// Sets what the project's cleanup passes start from, one undo step.
pub fn project_cleanup_set_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    cleanup: preset::settings::CleanupSettings,
) -> Result<OpenedProject, String> {
    cleanup.validate()?;
    commit(state, &project_handle, expected_revision, |document| {
        let mut next = document.clone();
        next.cleanup = cleanup;
        Ok(next)
    })
}

/// Sets what the project's Export dialog starts with, one undo step.
pub fn project_export_set_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    export: preset::settings::ExportPreferences,
) -> Result<OpenedProject, String> {
    export.validate()?;
    commit(state, &project_handle, expected_revision, |document| {
        let mut next = document.clone();
        next.export = export;
        Ok(next)
    })
}

/// Renames the user's preset `name` to `to`; projects that follow it keep following it (their
/// old name for it leads to it, see `preset::store`).
pub fn preset_rename_impl(_state: &AppState, name: String, to: String) -> Result<PresetView, String> {
    let current = store::load(&name)?;
    if preset::is_builtin(&current.name) {
        return Err(format!("{} is built in and keeps its name", current.name));
    }
    let saved = store::save(&store::PresetInput {
        name: to.trim().to_string(),
        previous: Some(current.name.clone()),
        method: current.method.clone(),
        rules: current.rules.clone(),
        preset: current.clone(),
    })?;
    Ok(preset::view(&saved))
}

/// Takes the project's changes back to its preset's values (what is always its own stays):
/// all of them, a `part` (`captions`, `layout`...), or one `field` of it. One undo step.
pub fn project_preset_reset_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    part: Option<String>,
    field: Option<String>,
) -> Result<OpenedProject, String> {
    commit(
        state,
        &project_handle,
        expected_revision,
        |document| match (part.as_deref(), field.as_deref()) {
            (Some(part), Some(field)) => {
                let inherited = preset::of(document).inherited();
                let mut value = serde_json::to_value(document).map_err(|e| e.to_string())?;
                let was = inherited
                    .get(part)
                    .and_then(|p| p.get(field))
                    .cloned()
                    .unwrap_or(Value::Null);
                let slot = value
                    .get_mut(part)
                    .and_then(Value::as_object_mut)
                    .ok_or_else(|| format!("No part {part}"))?;
                if was.is_null() {
                    slot.remove(field);
                } else {
                    slot.insert(field.to_string(), was);
                }
                serde_json::from_value(value).map_err(|e| e.to_string())
            }
            (part, _) => preset::reset(document, part),
        },
    )
}

/// Saves the open project's settings into a preset: its own (`name` absent; a built-in can't
/// change) or a new one called `name`, which the project then follows. The project's changes
/// become the preset's values, so it has none left.
pub fn preset_save_from_project_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    name: Option<String>,
) -> Result<OpenedProject, String> {
    let (current, values, style) = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        require_handle(reader, &project_handle)?;
        let document = reader.document();
        (
            preset::of(document),
            preset::captured(document),
            document.style.clone(),
        )
    };
    let target = match name.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
        Some(name) => name.to_string(),
        None if preset::is_builtin(&current.name) => {
            return Err(format!(
                "{} is built in and never changes: save as a new preset",
                current.name
            ))
        }
        None => current.name.clone(),
    };
    let fresh = target != current.name;
    let mut next = current.clone();
    next.values = values;
    if fresh {
        next.style = style;
    }
    store::save(&store::PresetInput {
        name: target.clone(),
        previous: (!fresh).then(|| current.name.clone()),
        method: current.method.clone(),
        rules: current.rules.clone(),
        preset: next,
    })?;
    commit(state, &project_handle, expected_revision, |document| {
        let mut next = preset::switch(document, &target)?;
        next.preset = target.clone();
        // The values are the preset's now: nothing is changed here any more.
        preset::reset(&next, None)
    })
}

/// Saves a preset as written (the editor's method and rules editor, agents' choices). Projects
/// following it follow the new values; the open one redraws.
pub fn preset_save_impl(state: &AppState, input: store::PresetInput) -> Result<PresetView, String> {
    let before = changes_before(state, input.previous.as_deref().unwrap_or(&input.name));
    let saved = store::save(&input)?;
    follow(state, before);
    Ok(preset::view(&saved))
}

pub fn preset_remove_impl(state: &AppState, name: String) -> Result<(), String> {
    let before = changes_before(state, &name);
    store::remove(&name)?;
    follow(state, before);
    Ok(())
}

/// The open project as stored (its changes from its preset), when it follows preset `name`:
/// taken before that preset changes, so the project then follows the new values.
fn changes_before(state: &AppState, name: &str) -> Option<Value> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref()?;
    preset::of(reader.document())
        .name
        .eq_ignore_ascii_case(name.trim())
        .then(|| preset::stored(reader.document()).ok())?
}

/// The open project read again from `stored` against its preset as it is now: values it never
/// changed follow the preset (one undo step when anything changed).
fn follow(state: &AppState, stored: Option<Value>) {
    let Some(stored) = stored else { return };
    let mut opened = state.opened_project.lock();
    let Some(reader) = opened.as_mut() else {
        return;
    };
    let Ok(next) = preset::read(stored) else {
        return;
    };
    if &next != reader.document() && next.validate().is_ok() {
        let revision = reader.document().revision;
        if reader.commit_document(revision, next).is_ok() {
            let _ = state
                .playback
                .lock()
                .apply_document(&reader.history().current);
        }
    }
}

/// Changes preset `name` by a JSON merge patch over what `library` shows of it (`values`,
/// `short`, `choices`, `description`, `style`, `method`, `rules`). A built-in is never changed:
/// the change goes into a copy, `save_as` or "My <name>", which the open project follows if it
/// followed the built-in.
pub fn preset_patch_impl(
    state: &AppState,
    name: &str,
    patch: &Value,
    save_as: Option<&str>,
) -> Result<(PresetView, Option<String>), String> {
    let current = store::load(name)?;
    let mut value = serde_json::to_value(&current).map_err(|e| e.to_string())?;
    value["method"] = Value::String(current.method.clone());
    value["rules"] = Value::String(current.rules.clone());
    preset::merge(&mut value, patch);
    let method = value["method"].as_str().unwrap_or_default().to_string();
    let rules = value["rules"].as_str().unwrap_or_default().to_string();
    if let Some(choices) = value.get_mut("choices").and_then(Value::as_object_mut) {
        crate::mcp::skill::forget_review(choices);
    }
    let next: preset::Preset =
        serde_json::from_value(value).map_err(|e| format!("Not a preset: {e}"))?;
    let copy = match save_as.map(str::trim).filter(|t| !t.is_empty()) {
        Some(target) if target != current.name => Some(target.to_string()),
        _ if preset::is_builtin(&current.name) => {
            let taken: Vec<String> = store::list()
                .into_iter()
                .map(|p| p.name.to_lowercase())
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
        _ => None,
    };
    let target = copy.clone().unwrap_or_else(|| current.name.clone());
    let before = changes_before(state, &current.name).filter(|_| copy.is_none());
    let saved = store::save(&store::PresetInput {
        name: target.clone(),
        previous: copy.is_none().then(|| current.name.clone()),
        preset: next,
        method,
        rules,
    })?;
    follow(state, before);
    // The open project follows the copy of the preset it followed.
    if copy.is_some() {
        let open = {
            let opened = state.opened_project.lock();
            opened.as_ref().map(|r| {
                (
                    r.summary.project_handle.clone(),
                    r.document().revision,
                    preset::of(r.document()).name.eq_ignore_ascii_case(&current.name),
                )
            })
        };
        if let Some((handle, revision, true)) = open {
            project_preset_set_impl(state, handle, revision, target.clone(), false)?;
        }
    }
    Ok((preset::view(&saved), copy))
}
