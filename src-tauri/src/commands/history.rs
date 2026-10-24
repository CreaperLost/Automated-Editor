//! The History popover's commands: recent steps, and checkpoints to save, rename, delete and
//! go back to.
use super::{require_handle, AppState};
use crate::project::history::{Author, History};
use crate::project::{OpenedProject, ProjectReader};
use crate::proposals::ProposalsView;
use serde::Serialize;

/// Steps the popover lists.
const SHOWN_STEPS: usize = 40;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StepView {
    pub label: String,
    pub by: Author,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    pub at: String,
    /// Undone: Redo brings it back.
    pub undone: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CheckpointView {
    pub id: String,
    pub name: String,
    pub by: Author,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    pub auto: bool,
    pub at: String,
    pub graphics: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryView {
    pub project_handle: String,
    /// Newest first.
    pub checkpoints: Vec<CheckpointView>,
    /// Newest first: undone ones (waiting for Redo) on top.
    pub steps: Vec<StepView>,
}

pub fn view(reader: &ProjectReader) -> HistoryView {
    let history: &History = reader.history().steps();
    HistoryView {
        project_handle: reader.summary.project_handle.clone(),
        checkpoints: history
            .checkpoints()
            .iter()
            .rev()
            .map(|c| CheckpointView {
                id: c.id.clone(),
                name: c.name.clone(),
                by: c.by,
                agent: c.agent.clone(),
                auto: c.auto,
                at: c.at.clone(),
                graphics: c.graphics.len(),
            })
            .collect(),
        steps: history
            .recent_steps(SHOWN_STEPS)
            .into_iter()
            .map(|(s, undone)| StepView {
                label: s.label.clone(),
                by: s.by,
                agent: s.agent.clone(),
                at: s.at.clone(),
                undone,
            })
            .collect(),
    }
}

/// Runs `change` on the open project under the command lock and returns the history after.
fn with_history(
    state: &AppState,
    handle: &str,
    change: impl FnOnce(&mut ProjectReader) -> Result<(), String>,
) -> Result<HistoryView, String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened.as_mut().ok_or("No opened project")?;
    require_handle(reader, handle)?;
    change(reader)?;
    Ok(view(reader))
}

pub fn project_history_impl(
    state: &AppState,
    project_handle: String,
) -> Result<HistoryView, String> {
    with_history(state, &project_handle, |_| Ok(()))
}

pub fn checkpoint_save_impl(
    state: &AppState,
    project_handle: String,
    name: String,
) -> Result<HistoryView, String> {
    with_history(state, &project_handle, |reader| {
        reader.save_checkpoint(&name).map(|_| ())
    })
}

pub fn checkpoint_rename_impl(
    state: &AppState,
    project_handle: String,
    id: String,
    name: String,
) -> Result<HistoryView, String> {
    with_history(state, &project_handle, |reader| {
        reader.rename_checkpoint(&id, &name)
    })
}

pub fn checkpoint_delete_impl(
    state: &AppState,
    project_handle: String,
    id: String,
) -> Result<HistoryView, String> {
    with_history(state, &project_handle, |reader| {
        reader.delete_checkpoint(&id)
    })
}

/// What going back to a checkpoint changed.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RestoreOutcome {
    pub project: OpenedProject,
    pub proposals: ProposalsView,
    pub history: HistoryView,
    /// What was put back, in a sentence or two.
    pub message: String,
}

pub fn checkpoint_restore_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    id: String,
) -> Result<RestoreOutcome, String> {
    let ((project, message), proposals) =
        super::proposals::with_proposals(state, Some(&project_handle), |reader, p| {
            crate::proposals::restore_checkpoint(reader, p, expected_revision, &id)
        })?;
    let history = project_history_impl(state, project_handle)?;
    Ok(RestoreOutcome {
        project,
        proposals,
        history,
        message,
    })
}
