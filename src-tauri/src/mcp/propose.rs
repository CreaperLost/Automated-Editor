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
    /// A take short (id or title): cut its own timeline.
    #[serde(default)]
    pub short: Option<String>,
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
    #[serde(default)]
    pub edits: Vec<Value>,
    /// B-roll over the video, placed by the app after `edits`: pictures only (the narration
    /// stays), on a free overlay track.
    #[serde(default)]
    pub broll: Vec<BrollItem>,
    /// Imported takes put together, after `edits`.
    #[serde(default)]
    pub assemble: Option<AssembleParams>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssembleParams {
    /// Lines in order, each from the take that says it best (trimmed to it); alternatives and
    /// missing lines are listed. None: whole takes in name order (2 before 10), a repeated
    /// take left for the later one.
    #[serde(default)]
    pub script: Vec<String>,
    /// Media ids. Default: every transcribed imported file not on the timeline.
    #[serde(default)]
    pub media: Vec<String>,
    /// Default: the end.
    #[serde(default)]
    pub at_s: Option<f64>,
}

/// The takes `params` names (or the default ones) in `reader`'s project.
fn takes(
    reader: &ProjectReader,
    params: &AssembleParams,
) -> Result<Vec<crate::assemble::Take>, String> {
    use crate::sequence::{AssetKind, StreamKind};
    use crate::transcript::{store, WordKind};
    let document = reader.document();
    let words_of = |asset: &crate::sequence::Asset| {
        asset
            .streams
            .iter()
            .filter(|s| s.kind == StreamKind::Sound)
            .find_map(|s| {
                store::load_transcript(reader.root(), &format!("{}.{}", asset.id, s.id))
                    .ok()
                    .flatten()
            })
            .map(|t| {
                t.words
                    .into_iter()
                    .filter(|w| w.kind == WordKind::Word)
                    .map(|w| {
                        (
                            crate::source_search::normalize(&w.text),
                            w.source_start_us,
                            w.source_end_us,
                        )
                    })
                    .filter(|w| !w.0.is_empty())
                    .collect::<Vec<_>>()
            })
    };
    let take =
        |asset: &crate::sequence::Asset, words: Vec<(String, u64, u64)>| crate::assemble::Take {
            media: asset.id.clone(),
            name: asset.name.clone(),
            duration_us: asset.duration_us,
            words,
        };
    let playable =
        |a: &crate::sequence::Asset| matches!(a.kind, AssetKind::Video | AssetKind::Audio);
    if !params.media.is_empty() {
        return params
            .media
            .iter()
            .map(|id| {
                let asset = document
                    .asset(id)
                    .filter(|a| playable(a))
                    .ok_or_else(|| format!("No imported video or sound {id}"))?;
                Ok(take(asset, words_of(asset).unwrap_or_default()))
            })
            .collect();
    }
    let placed: std::collections::HashSet<&str> = document
        .sequence
        .clips()
        .map(|(_, c)| c.asset.as_str())
        .collect();
    let found: Vec<_> = document
        .assets
        .iter()
        .filter(|a| playable(a) && !placed.contains(a.id.as_str()))
        .filter_map(|a| words_of(a).filter(|w| !w.is_empty()).map(|w| take(a, w)))
        .collect();
    if found.is_empty() {
        return Err("No transcribed imported takes off the timeline: name them in `media`".into());
    }
    Ok(found)
}

/// The placements that put the takes together, back to back from `at_s` (else the end), and
/// what the user reads about them.
fn assemble_edits(
    reader: &ProjectReader,
    before: &[SequenceEdit],
    params: &AssembleParams,
) -> Result<(Vec<SequenceEdit>, Vec<String>), String> {
    use crate::sequence::edit::apply_reserving;
    use crate::sequence::SourceRange;
    let takes = takes(reader, params)?;
    let plan = if params.script.is_empty() {
        crate::assemble::by_order(&takes)
    } else {
        if takes.iter().all(|t| t.words.is_empty()) {
            return Err("None of those takes is transcribed: transcribe them first".into());
        }
        crate::assemble::by_script(&takes, &params.script)
    };
    if plan.pieces.is_empty() {
        return Err(format!("Nothing to place. {}", plan.report.join(". ")));
    }
    let document = reader.document();
    let taken: Vec<String> = document.track_ids_in_use().map(String::from).collect();
    let mut sequence = document.sequence.clone();
    for edit in before {
        sequence = apply_reserving(
            &sequence,
            &document.assets,
            edit,
            taken.iter().map(String::as_str),
        )?
        .0;
    }
    let mut at = match params.at_s {
        Some(s) if s.is_finite() && s >= 0.0 => (s * 1e6).round() as u64,
        Some(_) => return Err("`atS` is seconds from the start".into()),
        None => sequence.duration_us(),
    };
    let edits = plan
        .pieces
        .iter()
        .map(|p| {
            let edit = SequenceEdit::PlaceAsset {
                asset_id: p.media.clone(),
                at_us: at,
                track_id: None,
                streams: Vec::new(),
                range: Some(SourceRange {
                    start_us: p.start_us,
                    end_us: p.end_us,
                }),
            };
            at += p.end_us - p.start_us;
            edit
        })
        .collect();
    Ok((edits, plan.report))
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BrollItem {
    /// An imported media id.
    pub media: String,
    /// From `fromS` on its own clock, or from the notes moment whose words are `select`.
    #[serde(default)]
    pub from_s: Option<f64>,
    #[serde(default)]
    pub select: Option<String>,
    /// Default 4.
    #[serde(default)]
    pub length_s: Option<f64>,
    /// Lands on word `atWord` (an id) or at `atS` on the timeline.
    #[serde(default)]
    pub at_word: Option<String>,
    #[serde(default)]
    pub at_s: Option<f64>,
}

const BROLL_DEFAULT_US: u64 = 4_000_000;

/// The placements for `items` on `reader`'s video, after `before` (the batch's other edits):
/// each item's picture streams on a video track above the first that is free there, or a new
/// one. `at` gives each item's timeline start.
fn broll_edits(
    reader: &ProjectReader,
    notes_dir: &std::path::Path,
    before: &[SequenceEdit],
    items: &[BrollItem],
    at: &[u64],
) -> Result<Vec<SequenceEdit>, String> {
    use crate::sequence::edit::apply_reserving;
    use crate::sequence::{SourceRange, StreamKind, TrackKind};
    let document = reader.document();
    let taken: Vec<String> = document.track_ids_in_use().map(String::from).collect();
    let mut sequence = document.sequence.clone();
    for edit in before {
        sequence = apply_reserving(
            &sequence,
            &document.assets,
            edit,
            taken.iter().map(String::as_str),
        )?
        .0;
    }
    let length_before = sequence.duration_us();
    let mut out = Vec::new();
    for (item, &at_us) in items.iter().zip(at) {
        let asset = document
            .asset(&item.media)
            .ok_or_else(|| format!("No media {}", item.media))?;
        let streams: Vec<String> = asset
            .streams
            .iter()
            .filter(|s| s.kind == StreamKind::Picture)
            .map(|s| s.id.clone())
            .collect();
        if streams.is_empty() || asset.is_graphic() {
            return Err(format!("{} has no picture for B-roll", item.media));
        }
        let from_us = match (&item.select, item.from_s) {
            (Some(words), _) => {
                let print = crate::source_notes::fingerprint(reader.root(), asset)?;
                let notes = crate::source_notes::load(notes_dir, &print)
                    .ok_or_else(|| format!("{} has no notes to select from", item.media))?;
                notes
                    .matching(words)
                    .into_iter()
                    .find_map(|(at, _)| at)
                    .ok_or_else(|| format!("No moment of {} matches \"{words}\"", item.media))?
            }
            (None, Some(s)) if s.is_finite() && s >= 0.0 => (s * 1e6).round() as u64,
            _ => return Err("Give each B-roll `fromS` or `select`".into()),
        };
        let length = item
            .length_s
            .filter(|s| s.is_finite() && *s > 0.0)
            .map_or(BROLL_DEFAULT_US, |s| (s * 1e6).round() as u64);
        let end = (from_us + length).min(asset.duration_us);
        if end <= from_us {
            return Err(format!("{} ends before {}", item.media, clock_s(from_us)));
        }
        let range = Some(SourceRange {
            start_us: from_us,
            end_us: end,
        });
        let free = sequence
            .video_tracks()
            .skip(1)
            .find(|t| !t.locked && t.is_free(at_us, at_us + (end - from_us)))
            .map(|t| t.id.clone());
        let place = SequenceEdit::PlaceAsset {
            asset_id: asset.id.clone(),
            at_us,
            track_id: free.clone(),
            streams: streams.clone(),
            range,
        };
        let edit = match free {
            Some(_) => place,
            None => SequenceEdit::OnNewTrack {
                track_kind: TrackKind::Video,
                edit: Box::new(place),
            },
        };
        sequence = apply_reserving(
            &sequence,
            &document.assets,
            &edit,
            taken.iter().map(String::as_str),
        )?
        .0;
        if sequence.duration_us() > length_before.max(at_us + (end - from_us)) {
            return Err("That B-roll would have moved the video".into());
        }
        out.push(edit);
    }
    Ok(out)
}

fn clock_s(us: u64) -> String {
    super::tools::clock(us)
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
    /// Instead: what the user turned down or redid of the agent's work here.
    #[serde(default)]
    pub corrections: bool,
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
    let short = match args.short.as_deref() {
        Some(wanted) => Some(super::clips::named(ctx, wanted)?),
        None => None,
    };
    if let Some(short) = short.as_ref().filter(|s| s.take.is_none()) {
        return Err(format!(
            "\"{}\" is a clip cut from the video: propose the cut on the video (no `short`), \
             then `clips` with `op: range` brings it into the clip",
            short.title
        ));
    }
    let stream = match (args.stream, &short) {
        (None, Some(short)) => short.take.as_ref().map(|t| t.voice.clone()),
        (stream, _) => stream,
    };
    propose(ctx, |reader, proposals| {
        let stream = transcribed_stream(reader, stream).or_else(|e| {
            match args.cuts.iter().all(|c| c.from_word.is_none()) {
                // Ranges alone need no transcript.
                true => Ok(String::new()),
                false => Err(e),
            }
        })?;
        // A take only the short plays is cut there even unnamed.
        let short = short.map(|s| s.id).or_else(|| {
            crate::shorts::take_short_of(reader.document(), &stream).map(|s| s.id.clone())
        });
        proposals::propose_cuts(
            reader,
            proposals,
            &agent,
            args.expected_revision,
            &stream,
            short.as_deref(),
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
    if edits.is_empty() && args.broll.is_empty() && args.assemble.is_none() {
        return Err("Give `edits`, `broll` or `assemble`".into());
    }
    let at = args
        .broll
        .iter()
        .map(|b| match (&b.at_word, b.at_s) {
            (Some(word), _) => super::actions::word_edited_us(ctx, None, word),
            (None, Some(s)) if s.is_finite() && s >= 0.0 => Ok((s * 1e6).round() as u64),
            _ => Err("Give each B-roll `atWord` or `atS`".to_string()),
        })
        .collect::<Result<Vec<u64>, String>>()?;
    let notes_dir = crate::source_notes::dir(&ctx.host.config_dir());
    let agent = ctx.agent.clone();
    propose(ctx, |reader, proposals| {
        let mut edits = edits;
        let mut report = Vec::new();
        if let Some(assemble) = &args.assemble {
            let (placed, said) = assemble_edits(reader, &edits, assemble)?;
            edits.extend(placed);
            report = said;
        }
        let broll = broll_edits(reader, &notes_dir, &edits, &args.broll, &at)?;
        edits.extend(broll);
        proposals::propose_edits(
            reader,
            proposals,
            &agent,
            args.expected_revision,
            edits,
            &args.reason,
            report,
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
    if args.corrections {
        let report = super::tools::with_reader(ctx.state, |reader| {
            let proposals = proposals::load(reader.root())?;
            let mut steps = reader.history().steps().recent_steps(500);
            steps.reverse();
            Ok(crate::corrections::gather(&proposals, &steps).text())
        })?;
        return super::tools::text(report);
    }
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
