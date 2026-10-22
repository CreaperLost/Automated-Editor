//! The tools that change things. An agent never edits directly: it proposes a batch, the app
//! checks it on a copy of the edit, and the user accepts it in the Agent panel (or autopilot
//! does). Proposing leaves the edit's revision as it was.
use super::tools::{params, pretty, text, transcribed_stream, Ctx, Output};
use crate::commands::proposals::with_proposals;
use crate::project::ProjectReader;
use crate::proposals::{self, CutInput, Proposals, ProposalsView, Status};
use crate::sequence::edit::SequenceEdit;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CutsParams {
    /// The project's revision you read (project_info, transcript_get, timeline_get).
    pub expected_revision: u64,
    /// What the batch is for, in a sentence the user reads, such as "Remove fillers and the
    /// false start at 2:10".
    pub reason: String,
    /// Stream key such as `rec-1234.mic` whose word ids the cuts use. Default: the first
    /// transcribed one, as in transcript_get.
    #[serde(default)]
    pub stream: Option<String>,
    /// The cuts: each either `fromWord` (and `toWord`, inclusive) or `startUs` and `endUs`.
    pub cuts: Vec<CutInput>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EditParams {
    /// The project's revision you read.
    pub expected_revision: u64,
    /// What the batch is for, in a sentence the user reads.
    pub reason: String,
    /// Timeline edits, applied in order. Each is an object with `kind` and its fields
    /// (camelCase), for example `{"kind": "placeAsset", "assetId": "a-3", "atUs": 12000000,
    /// "trackId": "t-4", "streams": ["screen"]}` or `{"kind": "deleteRange", "ranges":
    /// [{"startUs": 1000000, "endUs": 2000000}], "ripple": true}`. Track and clip ids come
    /// from timeline_get, assets and streams from project_info.
    // Parsed one by one (so an error names the edit), but published with every kind's schema.
    #[schemars(with = "Vec<SequenceEdit>")]
    pub edits: Vec<Value>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListParams {
    /// Only batches still waiting for the user.
    #[serde(default)]
    pub pending_only: bool,
    /// Only your own batches.
    #[serde(default)]
    pub mine: bool,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BatchParams {
    /// Such as `b-3`.
    pub batch_id: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UndoParams {
    pub batch_id: String,
    /// The project's revision now (proposals_list reports it).
    pub expected_revision: u64,
}

/// Stores a batch with `propose`, accepts it at once under autopilot, and tells the agent
/// what happened.
pub(crate) fn propose(
    ctx: &Ctx,
    propose: impl FnOnce(&ProjectReader, &mut Proposals) -> Result<String, String>,
) -> Result<Vec<Output>, String> {
    let autopilot = ctx.autopilot;
    let ((id, accepted), view) =
        with_proposals(ctx.state, ctx.project.as_deref(), |reader, proposals| {
            let id = propose(reader, proposals)?;
            if !autopilot {
                return Ok((id, None));
            }
            let revision = reader.document().revision;
            // A batch autopilot cannot take (nothing left to cut) waits for the user instead.
            let accepted = proposals::accept(reader, proposals, &id, revision, None, false, true);
            Ok((id, Some(accepted)))
        })?;
    ctx.changed_proposals();
    let batch = view.batches.iter().find(|b| b.id == id);
    let summary = batch.map(|b| b.summary.join("\n")).unwrap_or_default();
    let head = match accepted {
        None => format!(
            "Proposed {id}. It waits for the user in the Agent panel; the edit is still at \
             revision {}. Check proposals_list later to see what they decided.",
            view.revision
        ),
        Some(Ok(_)) => {
            ctx.changed_project();
            format!(
                "Proposed {id} and autopilot accepted it: the edit is now at revision {}. The \
                 user can undo it from the Agent panel.",
                view.revision
            )
        }
        Some(Err(error)) => format!(
            "Proposed {id}; autopilot could not accept it ({error}), so it waits for the user. \
             The edit is at revision {}.",
            view.revision
        ),
    };
    let warning = view.warning.map(|w| format!("\n{w}")).unwrap_or_default();
    text(format!("{head}\n{summary}{warning}"))
}

pub fn propose_cuts(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: CutsParams = params(args)?;
    let agent = ctx.agent.clone();
    propose(ctx, |reader, proposals| {
        let stream = transcribed_stream(reader, args.stream).or_else(|e| {
            match args.cuts.iter().all(|c| c.from_word.is_none()) {
                // Ranges alone need no transcript.
                true => Ok(String::new()),
                false => Err(e),
            }
        })?;
        proposals::propose_cuts(
            reader,
            proposals,
            &agent,
            args.expected_revision,
            &stream,
            args.cuts,
            &args.reason,
        )
    })
}

pub fn propose_edit(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: EditParams = params(args)?;
    let edits = args
        .edits
        .into_iter()
        .enumerate()
        .map(|(index, edit)| {
            serde_json::from_value::<SequenceEdit>(edit)
                .map_err(|e| format!("Edit {} is not a timeline edit: {e}", index + 1))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let agent = ctx.agent.clone();
    propose(ctx, |reader, proposals| {
        proposals::propose_edits(
            reader,
            proposals,
            &agent,
            args.expected_revision,
            edits,
            &args.reason,
        )
    })
}

/// The view an agent reads: the batches without the full edit lists.
fn listing(view: &ProposalsView, agent: &str, pending_only: bool, mine: bool) -> Value {
    let batches: Vec<Value> = view
        .batches
        .iter()
        .filter(|b| !pending_only || b.status == Status::Pending)
        .filter(|b| !mine || b.agent == agent)
        .map(|b| {
            let items: Vec<Value> = b
                .items
                .iter()
                .map(|i| {
                    let mut item = json!({
                        "id": i.id,
                        "state": i.state,
                        "text": i.text,
                    });
                    if i.already_cut {
                        item["alreadyCut"] = json!(true);
                    }
                    if i.stale {
                        item["stale"] = json!(true);
                    }
                    item
                })
                .collect();
            let mut batch = json!({
                "id": b.id,
                "agent": b.agent,
                "yours": b.agent == agent,
                "kind": b.kind,
                "status": b.status,
                "reason": b.reason,
                "baseRevision": b.base_revision,
                "summary": b.summary,
            });
            if b.auto {
                batch["autoAccepted"] = json!(true);
            }
            if let Some(revision) = b.applied_revision {
                batch["appliedRevision"] = json!(revision);
            }
            if b.stale {
                batch["stale"] = json!(true);
            }
            if b.can_undo {
                batch["canUndo"] = json!(true);
            }
            if !items.is_empty() {
                batch["items"] = json!(items);
            }
            batch
        })
        .collect();
    json!({ "revision": view.revision, "batches": batches })
}

pub fn proposals_list(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: ListParams = params(args)?;
    let (project, view) = super::tools::with_reader(ctx.state, |reader| {
        let view = proposals::view(reader, &proposals::load(reader.root())?);
        Ok((reader.summary.project_handle.clone(), view))
    })?;
    let mut listed = listing(&view, &ctx.agent, args.pending_only, args.mine);
    listed["project"] = json!(project);
    pretty(&listed)
}

pub fn proposal_withdraw(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: BatchParams = params(args)?;
    let ((), view) = with_proposals(ctx.state, ctx.project.as_deref(), |_, proposals| {
        proposals::withdraw(proposals, &args.batch_id, &ctx.agent)
    })?;
    ctx.changed_proposals();
    text(format!(
        "Withdrew {}. The edit is at revision {}.",
        args.batch_id, view.revision
    ))
}

pub fn undo_batch(ctx: &Ctx, args: Value) -> Result<Vec<Output>, String> {
    let args: UndoParams = params(args)?;
    let (_, view) = with_proposals(ctx.state, ctx.project.as_deref(), |reader, proposals| {
        proposals::undo(
            reader,
            proposals,
            &args.batch_id,
            args.expected_revision,
            Some(&ctx.agent),
        )
    })?;
    ctx.changed_project();
    ctx.changed_proposals();
    text(format!(
        "Undid {} and withdrew it. The edit is now at revision {}.{}",
        args.batch_id,
        view.revision,
        view.warning.map(|w| format!("\n{w}")).unwrap_or_default()
    ))
}
