# History and checkpoints: plan v1 (2026-10-23)

[SAFE-2](../roadmap.md#safe-2). Status: **phase 1 built 23 Oct 2026** (see
[Built](#built-phase-1-23-oct-2026)); phases 2 and 3 not started. George's answers to its
three questions are under [Decisions](#decisions-george-23-oct-2026). When it ships, fold the
model into [architecture.md](../architecture.md) and delete this plan.

Goal: undo that survives closing the app, one step that takes back everything an agent did in a
run, and named checkpoints ("clean cut", "approved look") the user or the agent can go back to.
Restoring anything is a new step, so nothing is ever lost by going back.

Why now: in the EVAL-1 run about $0.30 of the short's $0.55 went on redoing work after the app
restarted and its undo history was gone ([evaluation.md](evaluation.md)).

## What exists today

| Piece | Where | How it behaves |
|---|---|---|
| Undo stack | `project/revision.rs` `EditHistory` | `current` plus `undo` and `redo` vectors of whole `EditDocument`s, in memory only, capped at `MAX_UNDO = 64`. `commit_next`, `undo` and `redo` write `project.json` atomically under `.edit.lock` and bump `revision` |
| Owner | `project/reader.rs` `ProjectReader` | Holds the `EditHistory`; builds a fresh one from `project.json` on open, so reopening empties both stacks |
| Batch undo | `proposals.rs` `undo` | Undoes an accepted batch only while `applied_revision == current`; batches live in `agent/proposals.json` and survive reopening, the stack they point into does not |
| Transcripts | `transcript/store.rs`, `transcripts/<track>.json` | Word fixes, caption marks, dismissed suggestions. Not undo steps, by design (as in the UI) |
| Graphic source | `graphics/<id>/graphic.json` + files | `graphic_set` and `graphic_write_file` write source directly; no undo ([gaps.md](../gaps.md), row "Graphic source edits have no undo") |
| Labels | — | A step has no name or author; the UI can only say "Undo" |

Sizes, measured on George's projects: `project.json` reaches 764 KB (the 16 Oct clean edit;
`sequence` is 370 KB of it, `noiseMutes` 63 KB) and gzips to 70 KB. Most steps change
`sequence`, so splitting the document into fields to share unchanged parts saves little;
compressing whole snapshots saves 90 %. `flate2` is already in `Cargo.lock` (a transitive
dependency), so it needs no new crate, only the usual dependency review line.

## Design

### Storage: `history/` in the project folder

```
history/
  states/<sha256>.json.gz   one EditDocument per file, gzip, named by the hash of its JSON
                            with `revision` zeroed (so undo/redo churn writes nothing new)
  log.json                  the stacks, steps and checkpoints (below), written atomically
  graphics/<sha256>.tar     graphic source folders, only for checkpoints (phase 3)
```

`log.json`:

```jsonc
{
  "schemaVersion": 1,
  "current": "<hash>",              // must match project.json; see "Opening" below
  "undo": ["<step id>", ...],       // oldest first
  "redo": ["<step id>", ...],
  "steps": {
    "s-41": { "before": "<hash>", "after": "<hash>", "label": "Cut 12 pauses",
              "by": "user" | "agent", "agent": "claude-code", "run": "r-7",
              "batch": "b-3", "at": "2026-10-23T14:02:11Z" }
  },
  "runs": { "r-7": { "agent": "claude-code", "started": "...", "ended": "...", "steps": 9 } },
  "checkpoints": [
    { "id": "c-2", "name": "Clean cut", "state": "<hash>", "by": "user", "at": "...",
      "graphics": { "<graphic id>": "<sha256>" }, "captionMarks": { "<track id>": {...} },
      "transcriptStamps": { "<track id>": "<stamp>" }, "auto": false }
  ]
}
```

- **Write order** in `commit_next`, `undo`, `redo`: state blob (if new) → `project.json` (as
  now, atomic, under the lock) → `log.json` (atomic). A crash between the last two leaves
  `project.json` one step ahead of the log, which opening repairs.
- **Limits:** up to 2 GB of states per project, no limit on the number of steps; the oldest
  steps are dropped first, and states a checkpoint uses never are. `MAX_UNDO` goes away.
  At 70 KB a step the worst real project fits about 28,000 steps in 2 GB.
- **Pruning** removes unreferenced state files after each write that drops a step; cheap,
  since the log lists every reference.
- **No migration** ([decisions.md](../decisions.md)): a project without `history/`
  starts with empty stacks, as today.

### Opening

`ProjectReader` loads `log.json` with `project.json`. If the log's `current` hash matches the
document, the stacks come back as they were. If not (a crash between writes, or the file was
changed outside the app), the document on disk wins: it becomes a new step labelled "Change
made outside AeroEdits" on top of the old stacks, and redo is cleared. A log that cannot be
read is renamed to `log.unreadable.json` (as `project.json` is today) and history starts empty.

### Steps get a label and an author

`commit_next` takes a small `StepMeta { label, by, agent, batch }`. Rather than touching every
caller, `ProjectReader` holds the meta for the call in progress: the Tauri command layer and the
MCP dispatcher set it once per call (the MCP tool name and its one-line answer are already the
label we want), and anything that doesn't set it gets "Edit". The Edit menu, the undo button's
tooltip and `undo`'s MCP answer then say what is undone ("Undo: Cut 12 pauses").

### Agent runs: one step to take back what the agent did

- **A run** is a stretch of changing calls from one MCP client with no user step in between.
  It ends when the user makes a step, after 15 minutes without an agent call, or when the
  agent calls `template` (a new workflow). Each step records its run id.
- **"Undo what the agent did"** (Agent panel button, and `undo` with `run: true` over MCP)
  restores the state from before the run's first step as **one new step** labelled
  "Undo agent run (9 changes)". It is offered only while the run's last step is the current
  state, the same rule `undo_batch` uses today; otherwise the panel says which user step came
  after and offers the checkpoint taken before the run (below) instead.
- Batches the run accepted are marked undone, as `proposals::undo` does for one batch.

### Checkpoints

- **What a checkpoint holds:** the edit state (a hash in `states/`), each graphic's source
  folder (`graphics/<id>/` without the render cache) as a tar named by its hash, and each
  transcript's caption marks with a stamp of the words they hang on (track id + transcript
  `created_at` + hash of word ids).
- **Made by:** the user (History panel, "Save checkpoint…", with a name); the agent
  (`checkpoint` op `save` with a name, behind `more` to keep the tool list small); and
  **automatically before an agent run's first change**, named "Before agent run, 14:02", kept
  for the newest five runs. This is the agent's version of "duplicate the project first".
- **Restoring** is one new step: the edit state comes back; graphic folders come back from their
  tars (a graphic added since is left alone, since the restored edit no longer places it; it can
  be re-added); caption marks come back only where the transcript stamp still matches, and the
  answer says which tracks were skipped and why. Pending proposals are marked stale with
  "the edit was restored to <checkpoint>", which is what NOTE-1 and PLAN-1 will lean on.
- **Rename and delete** from the History panel; deleting frees its states at the next prune.

### UI

- Undo/redo labels (Edit menu, tooltips), from step meta.
- A **History** popover from the toolbar: newest steps first, each with label, who (you /
  agent name), time; runs grouped and collapsible; checkpoints pinned at the top with
  Restore / Rename / Delete; "Save checkpoint…" button. Clicking a step does nothing in v1
  (no jumping into the middle of the stack); restore is for checkpoints.
- Agent panel: "Undo what the agent did" under the run's last batch.

### MCP

- `undo` / `redo` answers name the step. `undo` gains `run: true`.
- `checkpoint` (behind `more`): `list`, `save {name}`, `restore {id}`. Restoring asks for
  `expectedRevision` like every write.
- `status` mentions the newest checkpoint so an agent resuming tomorrow knows where it stands.

## Phases

1. **History on disk (M).** `history/` storage, `log.json`, write order, opening and repair,
   limits and pruning, step meta and labels. Done when: undo works after closing and reopening
   the app, after more than 64 steps, and after a simulated crash between `project.json` and
   `log.json`; the 764 KB project's history is pruned to the 2 GB limit (tested with a small limit).
2. **Agent runs (S).** Run ids on steps, the run boundaries, "Undo what the agent did" in the
   panel and over MCP, batches marked undone. Done when: an EVAL-1 tutorial run is taken back in
   one step after reopening the app, and the button refuses with a reason after a user edit.
3. **Checkpoints (M).** Save, auto-save before a run, restore with graphic folders and caption
   marks, stale proposals, History popover, `checkpoint` op. Done when: "clean cut" restored
   after the agent changed cuts, a graphic's text and a caption break; and a restore across a
   re-transcription skips the caption marks and says so.

Each phase is its own commit with tests beside the existing ones in `project/revision.rs`
(tempdir projects), then one check in the dev app through the real bridge
([architecture: working on the code](../architecture.md#working-on-the-code-notes-that-saved-time)).

## Built: phase 1 (23 Oct 2026)

Where it differs from the design above, this wins.

- `project/history.rs`: states as planned; the log is **`history/log.jsonl`, append-only**
  (header, then `commit` / `undo` / `redo` / `drop` lines), not a `log.json` rewritten on every
  step: one short line per edit instead of rewriting a file that grows with the history. Opening
  replays it, drops steps whose states are missing, ignores a last line cut short by a crash,
  sets an unreadable log aside as `history/log.unreadable-N.jsonl`, and rewrites it compactly.
- `EditHistory` keeps steps, not documents: undo and redo load the state from disk, so memory
  no longer grows with the history.
- **Labels come from what changed** (`history::describe`: "Cut 12.3 s", "Zooms", "Captions",
  "Cut 2.0 s, Layout and 1 more"), so no caller had to change. A step's author comes from a
  thread-local that `mcp::tools::call` sets for the length of each tool call (`as_agent`), with
  the MCP client's name and the tool; everything else is the user's. `set_next_step` is there
  for phase 2's batch steps. The undo and redo buttons' tooltips and `undo`'s MCP answer name
  the step (`OpenedProject.undoLabel` / `redoLabel`).
- **Cost, measured** (`history_cost_on_a_real_project`, ignored test): on the 744 KB 16 Oct
  edit a state is 62 KB and saving it takes 17 ms, beside about 65 ms the edit already took
  (saving `project.json`). 2 GB holds about 33,000 such steps.
- **Checked in the dev app:** an agent cut pauses and added a zoom over MCP, the app was
  stopped and started again, the reopened project's undo button read "Undo Zooms", and `undo`
  took both steps back ("Undid "Cut 58.8 s"") and `redo` brought one back.

## Decisions (George, 23 Oct 2026)

1. **Graphic source edits are not undo steps yet.** Checkpoints snapshot graphic folders;
   undo for `graphic_set` text changes and `graphic_write_file` comes with GFX-4.
2. **A checkpoint is saved automatically before every agent run**, the newest five kept.
3. **History may use up to 2 GB per project**, with no limit on the number of steps; the
   oldest steps go first and checkpoints' states are never dropped.
