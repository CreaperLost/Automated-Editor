//! Reviewing what agents propose: the Agent panel's commands, and the shared locking the
//! agents' own tools use too.
use super::{require_handle, AppState};
use crate::project::{OpenedProject, ProjectReader};
use crate::proposals::{self, Proposals, ProposalsView};
use serde::Serialize;

/// What deciding a batch changed.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProposalOutcome {
    /// The project, when the edit changed.
    pub project: Option<OpenedProject>,
    pub proposals: ProposalsView,
}

/// Runs `change` on the open project and its proposals under the command lock, saves the
/// proposals, and brings playback up to date when the edit changed. Without `handle`, the
/// project open now (for an agent).
pub(crate) fn with_proposals<T>(
    state: &AppState,
    handle: Option<&str>,
    change: impl FnOnce(&mut ProjectReader, &mut Proposals) -> Result<T, String>,
) -> Result<(T, ProposalsView), String> {
    let _guard = state.command_lock.lock();
    let mut opened = state.opened_project.lock();
    let reader = opened
        .as_mut()
        .ok_or(crate::mcp::tools::NO_PROJECT_MESSAGE)?;
    if let Some(handle) = handle {
        require_handle(reader, handle)?;
    }
    let mut proposals = proposals::load(reader.root())?;
    let revision = reader.document().revision;
    let out = change(reader, &mut proposals)?;
    let saved = proposals::save(reader.root(), &proposals);
    let edited = reader.document().revision != revision;
    if edited {
        state
            .playback
            .lock()
            .apply_document(&reader.history().current)?;
    }
    let mut view = proposals::view(reader, &proposals);
    match saved {
        Ok(()) => {}
        // The edit is saved and stays: failing to record it must not report it as not made.
        Err(error) if edited => {
            view.warning = Some(format!(
                "The edit was made, but the Agent panel's history could not be saved: {error}"
            ))
        }
        Err(error) => return Err(error),
    }
    Ok((out, view))
}

pub fn proposals_list_impl(
    state: &AppState,
    project_handle: String,
) -> Result<ProposalsView, String> {
    let opened = state.opened_project.lock();
    let reader = opened.as_ref().ok_or("No opened project")?;
    require_handle(reader, &project_handle)?;
    Ok(proposals::view(reader, &proposals::load(reader.root())?))
}

pub fn proposal_accept_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    batch_id: String,
    item_ids: Option<Vec<String>>,
    reject_rest: bool,
) -> Result<ProposalOutcome, String> {
    // A rule goes into its preset (app-wide, like the Library's own changes), then is marked.
    let rule = {
        let opened = state.opened_project.lock();
        let reader = opened.as_ref().ok_or("No opened project")?;
        require_handle(reader, &project_handle)?;
        proposals::pending_rule(&proposals::load(reader.root())?, &batch_id)?
    };
    if let Some((preset, rule, example)) = rule {
        let current = crate::preset::store::load(&preset)?;
        let rules = proposals::rules_with(&current.rules, &rule, &example);
        let (saved, _) = super::preset::preset_patch_impl(
            state,
            &preset,
            &serde_json::json!({ "rules": rules }),
            None,
        )?;
        let ((), proposals) = with_proposals(state, Some(&project_handle), |_, p| {
            proposals::rule_accepted(p, &batch_id, &saved.name)
        })?;
        let project = {
            let opened = state.opened_project.lock();
            opened.as_ref().map(|r| r.summary.clone())
        };
        return Ok(ProposalOutcome { project, proposals });
    }
    let (project, proposals) = with_proposals(state, Some(&project_handle), |reader, p| {
        proposals::accept(
            reader,
            p,
            &batch_id,
            expected_revision,
            item_ids.as_deref(),
            reject_rest,
            false,
        )
    })?;
    Ok(ProposalOutcome { project, proposals })
}

pub fn proposal_reject_impl(
    state: &AppState,
    project_handle: String,
    batch_id: String,
    item_ids: Option<Vec<String>>,
    rejected: bool,
) -> Result<ProposalsView, String> {
    let ((), proposals) = with_proposals(state, Some(&project_handle), |_, p| {
        proposals::reject(p, &batch_id, item_ids.as_deref(), rejected)
    })?;
    Ok(proposals)
}

pub fn proposal_undo_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
    batch_id: String,
) -> Result<ProposalOutcome, String> {
    let (project, proposals) = with_proposals(state, Some(&project_handle), |reader, p| {
        proposals::undo(reader, p, &batch_id, expected_revision, None)
    })?;
    Ok(ProposalOutcome {
        project: Some(project),
        proposals,
    })
}

/// Takes back everything the newest agent run did, as one step.
pub fn proposal_undo_run_impl(
    state: &AppState,
    project_handle: String,
    expected_revision: u64,
) -> Result<ProposalOutcome, String> {
    let (project, proposals) = with_proposals(state, Some(&project_handle), |reader, p| {
        proposals::undo_run(reader, p, expected_revision, None).map(|(project, _)| project)
    })?;
    Ok(ProposalOutcome {
        project: Some(project),
        proposals,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Windows refuses to replace a read-only file, which stands in for a locked or unwritable
    /// proposals file.
    #[cfg(windows)]
    #[test]
    fn an_accept_that_cannot_be_recorded_still_reports_the_edit_it_made() {
        if !crate::media::ffmpeg::available() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let (host, key) = crate::mcp::tests_support::lesson(dir.path());
        let state = &host.state;
        let (handle, revision, length, root, id) = {
            let opened = state.opened_project.lock();
            let reader = opened.as_ref().unwrap();
            let revision = reader.document().revision;
            let mut p = Proposals::default();
            let cut = proposals::CutInput {
                from_word: Some("w-1".into()),
                ..Default::default()
            };
            let id = proposals::propose_cuts(
                reader,
                &mut p,
                "test",
                revision,
                &key,
                None,
                vec![cut],
                "",
            )
            .unwrap();
            proposals::save(reader.root(), &p).unwrap();
            let root = reader.root().to_path_buf();
            (
                reader.summary.project_handle.clone(),
                revision,
                reader.document().duration_us(),
                root,
                id,
            )
        };
        let file = root.join("agent").join("proposals.json");
        let writable = std::fs::metadata(&file).unwrap().permissions();
        let mut read_only = writable.clone();
        read_only.set_readonly(true);
        std::fs::set_permissions(&file, read_only).unwrap();
        let outcome = proposal_accept_impl(state, handle, revision, id, None, false);
        std::fs::set_permissions(&file, writable).unwrap();

        let outcome = outcome.unwrap();
        let project = outcome.project.unwrap();
        assert_eq!(project.revision, revision + 1);
        assert!(outcome
            .proposals
            .warning
            .unwrap()
            .contains("could not be saved"));
        let opened = state.opened_project.lock();
        assert!(opened.as_ref().unwrap().document().duration_us() < length);
    }
}
