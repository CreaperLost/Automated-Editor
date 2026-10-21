# AI production workflow for AeroEdits

Reviewed 21 October 2026.

Reference: [Dan Kieft — Master AI Video Editing in 25 Minutes](https://www.youtube.com/watch?v=AgUATWAOaKY).
Read the full English auto-generated transcript, expanded description and chapter list,
plus the creator's [companion prompts](https://master-ai-video-editing.vercel.app/).
Playback inspection was limited by advertisements; this is a workflow and source-code
review, not a continuous viewing, visual-quality assessment, or reproduction of the results.
Transcription errors in product/model names are not treated as specifications. Provider
availability, pricing, integration contracts and quality have not been independently tested.

Recommendation: add a persistent production plan between cleanup and finishing. Start
with timed editorial notes, asset requests and a manual export/import loop. Extend the
existing timeline, graphics and external-agent interface before adding generation providers.
This document proposes work; it changes no application behavior.

## Reference workflow

| Chapter | Workflow lesson |
| --- | --- |
| [2:04 — Basic editing](https://www.youtube.com/watch?v=AgUATWAOaKY&t=124s) | Transcribe and tighten delivery, retain spoken editing directions as labeled notes, identify candidate alternate shots, and deliver editable timelines alongside renders. |
| [5:24 — Multiple angles](https://www.youtube.com/watch?v=AgUATWAOaKY&t=324s) | Export marked edited segments, supply room references, generate alternate viewpoints, then place picture above the original performance. The companion prompt adds reusable approved angle references and duration checks. |
| [9:44 — B-roll](https://www.youtube.com/watch?v=AgUATWAOaKY&t=584s) | Build consistent character references and request shots for particular narration beats. |
| [12:57 — Motion graphics](https://www.youtube.com/watch?v=AgUATWAOaKY&t=777s) | Combine assets, style references, iteration and editable deliverables. Code-authored animation and generated animation are distinct routes. |
| [17:34 — VFX](https://www.youtube.com/watch?v=AgUATWAOaKY&t=1054s) | Plan effects around recorded gestures and precise frame cues. |
| [21:48 — Color](https://www.youtube.com/watch?v=AgUATWAOaKY&t=1308s) | Separate camera conversion from the creative look. |
| [23:32 — Sound](https://www.youtube.com/watch?v=AgUATWAOaKY&t=1412s) | Design sound against finished motion; extract generated audio and retain original picture. |

The narration describes substantial iteration and manual intervention. Treat the workflow
as assisted production, not evidence of reliable unattended editing.

## Fit with the current app

These are findings from the current local source, rather than assumptions based on feature names.

| Area | Existing foundation | Addition proposed |
| --- | --- | --- |
| Rough cut | Transcript words, silence/noise/filler/retake passes, source-time mapping, undo and cut proposals | Identify spoken editor directions before cleanup; let users retain, resolve or remove them separately. |
| Planning | Chapters, storyboard and agent proposals | Persistent typed range notes: editor instruction, alternate angle, B-roll, graphic, effect and sound. Chapters are point markers intended for viewer navigation; they are not production tasks. |
| Shot placement | Imported media, separate picture/sound streams, overlay role, multiple tracks, ordered `SequenceEdit` proposals | Associate returned media with a particular request and place only the requested streams at the resolved range. Verify placement behavior under magnetic editing. |
| Motion graphics | HyperFrames sources, templates, variables, brand kits, library and background renders | Use the brief/revise/compare workflow already researched; add timing cues and preserved source variants. |
| Generation | Existing external agents can orchestrate app actions | No production-job/provider lifecycle was established in this review. Add durable requests and manual handoff before provider adapters. |
| Sound | Audio tracks, mix controls, synthesized telemetry effects, speech cleanup | Narration-aware cue plans and imported sound-only results. Existing clicks/whooshes are not generated sound design. |
| Review | Finished-frame storyboard, structural `check`, proposals and undo | Add A/B playback, original/generated solo, synchronization review and unresolved production tasks. Still images cannot verify lip-sync or motion. |
| Delivery | MP4 export for the video or a short | Export an arbitrary resolved production range; later evaluate editable NLE handoff and an explicit fidelity report. |
| Color | Style/layout settings and compositing | A camera-input transform plus creative grading path requires new work and preview/export parity. |

Evidence:
[sequence model](../src-tauri/src/sequence/mod.rs),
[sequence edits](../src-tauri/src/sequence/edit.rs),
[MCP tools](../src-tauri/src/mcp/tools.rs),
[MCP actions](../src-tauri/src/mcp/actions.rs),
[proposals](../src-tauri/src/proposals.rs),
[chapters](../src-tauri/src/chapters.rs),
[storyboard](../src-tauri/src/mcp/storyboard.rs),
[graphics](../src-tauri/src/graphics/), and
[shared UI types](../front-end/src/lib/types.ts).

Two distinctions matter for implementation:

- The sequence already separates picture and sound. We can build alternate-shot and
  sound-only placement on that foundation. We still need an operation that proves an
  overlay did not ripple the underlying performance or import duplicate dialogue.
- The agent panel reviews proposals and history; it is not an embedded creative chat.
  An initial pilot can use the external-agent MCP interface. A new model host is not a
  prerequisite for the production plan.

## Proposed experience

Keep the editor as the main workspace. Add a **Production** panel containing timed tasks;
show their ranges on a lightweight timeline lane and in the transcript context menu.

1. **Prepare the cut.** Inspect the transcript for editor directions, review their ranges,
   then run the existing cleanup passes with those ranges protected. Detecting a direction
   creates a suggestion with its exact spoken evidence, never an automatic instruction to
   execute arbitrary speech from footage.
2. **Plan visuals.** Propose a small number of changes, each with an editorial purpose and
   a target: screen, camera slot, whole canvas, or sound track. Prefer existing screen
   footage, layouts and graphics where they explain the tutorial well. Offer synthetic
   alternate camera views only for suitable presenter passages.
3. **Prepare an asset request.** Show the selected words, resolved interval, reference
   frames/clip, desired result, duration and preservation constraints. Allow a user-written
   brief or an agent draft. Pin the references and brief for reproducibility.
4. **Create or supply the asset.** First release: export a reference package and attach a
   local result. A later release may submit through a provider, exposing actual capabilities,
   uploads, cost and job state. The same task can also use existing footage or a local graphic.
5. **Review in context.** Compare at matching relative times, listen with the original audio,
   and accept placement as one undoable action. Keep rejected results available as drafts.
6. **Finish and deliver.** Run the existing storyboard and checks, then review unresolved
   notes and play the changed intervals before exporting.

Example: while the presenter says “there are three steps,” propose a three-step graphic
whose reveals follow the relevant words. When demonstrating a click, keep the actual UI
and telemetry zoom visible. During a camera-only introduction, optionally request a short
alternate viewpoint. The planning agent should explain each choice using the recording.

## Persistent task contract

Store the task's intent separately from any generated file or timeline placement. A minimal
record needs:

- Stable task ID, kind, title, evidence text, project/session and sequence identity.
- An anchor to specific clip occurrences and source ranges, optionally transcript word IDs
  plus the transcript dependency stamp. Source time alone is ambiguous when footage repeats.
- The resolved edit range, revision and reference-export fingerprint used for the request.
- Target slot, picture/sound stream choice, duration, frame rate and aspect ratio.
- Brief, immutable reference IDs/hashes, variant lineage and required preservation rules.
- Exported reference, returned media, validation findings and eventual placement IDs.
- Lifecycle: draft, ready, submitted, running, returned, needs review, placed, failed,
  cancelled or stale; provider/job IDs and spend records when applicable.

Use half-open ranges and rational frame rates. A frame number is meaningless without the
reference clip's time base. Variable-frame-rate footage needs explicit presentation-time
mapping or a declared constant-frame-rate reference export. Avoid accumulating rounded
seconds when converting event cues and clip boundaries.

If the edit changes after reference export, preserve the returned asset but re-resolve its
anchor. Moving an otherwise identical passage can permit a new placement proposal; changed
content, speed or internal cuts require review and possibly a new reference. Never silently
reuse an old absolute timeline timestamp. Cross-cut tasks must either resolve to explicit
pieces or request a new continuous exported reference.

## Implementation order

**Phase 1 — Production notes and a manual round trip.** Add typed range notes, evidence,
anchor resolution and undoable note operations. Extend cleanup protection and export review
for unresolved editor directions. Add reference-range export with a manifest, then attach a
local result to its task and propose video-only or audio-only placement. This proves the
workflow without relying on a vendor. The existing `ExportArgs` selects a whole video or
short; arbitrary reference-range export is an addition, not an existing action.

**Phase 2 — Contextual planning and graphics.** Let the external agent propose tasks using
transcript, telemetry and storyboard evidence. Connect graphic tasks to the existing
[prompt-to-motion proposal](prompt-to-motion-workflow-review-2026-10-21.md), including
source snapshots, independent variants and internal beat timing. Add an editable cue list
for sound; use existing or imported audio before requiring generation.

**Phase 3 — Provider jobs.** Verify a provider's documented API, authentication, supported
reference combinations, duration limits, output formats and costs before choosing an
adapter. Persist job state across app restarts, prevent duplicate submissions after timeouts,
and distinguish local cancellation from confirmed provider cancellation. Expose a concrete
request preview and bounded spend controls. Keep manual result attachment available.

**Phase 4 — Broader finishing and handoff.** Pilot LUT/grade parity and a constrained NLE
interchange format. Report which layouts, zooms, captions and graphics remain editable and
which are baked. A rendered animation clip does not become an editable composition merely
by placing it in an XML timeline. Revisit sophisticated synthetic-camera and VFX workflows
after the reference/result/placement loop is reliable.

Dependencies already in the backlog:
[task 4, B-roll](future-work.md#4-automatic-b-roll-from-free-libraries),
[task 14, source search](future-work.md#14-search-and-summarize-source-footage),
[task 15, handoff](future-work.md#15-export-a-handoff-for-another-editor), and
[task 17, project binding](future-work.md#17-bind-agent-edits-to-the-intended-project).
Implement the project/session binding guard before expanding agent mutations. Existing
revision checks alone do not establish which project an asynchronous result belongs to.

## First acceptance exercise

Use one real screen-and-camera tutorial, including one spoken editorial note. Prepare a
clean cut and three tasks: a local graphic, an externally supplied alternate shot and a
sound-only result. Change the timeline while one task is waiting, reopen the project, then
attach and review the results.

Verify that:

- The spoken note survives cleanup until explicitly resolved; ordinary tutorial directions
  addressed to the viewer are not automatically classified as editor notes.
- Notes remain attached to the intended occurrence after ripple cuts, repeated source
  passages and undo. Deleted/changed targets become visibly unresolved or stale.
- Each reference export matches the edited picture, audio, duration and frame count recorded
  in its manifest; source trim timestamps are not substituted for edited playback.
- Accepting an alternate picture preserves original dialogue and underlying clips, adds no
  duplicate audio, and does not ripple later content. Placement is reversible.
- A returned sound track changes only sound; truncated or mistimed results are flagged.
- Results from another project/session or outdated content cannot be silently applied.
- Reopening, retrying and cancelling do not lose drafts or create duplicate provider jobs.
- Playback confirms continuity and sync; structural checks and storyboards supplement it.

Measure preparation time, manual corrections, time to an acceptable asset, rejected results,
sync/placement failures and actual spend. Compare against doing the same edit with the app's
current tools. No performance or generation-quality benchmark was run for this review.
