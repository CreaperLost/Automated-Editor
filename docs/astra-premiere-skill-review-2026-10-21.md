# Astra Premiere Editing vs AeroEdits

Reviewed: 21 October 2026. Local code baseline: `92f80d3585db42f1ce343fcf67725d944daaf316`, with the working tree's existing changes left in place.

## Recommendation

Use this skill as an editorial workflow reference. AeroEdits already has the more direct automation foundation: structured project state, stable word identifiers, revision checks, deterministic cleanup, visual inspection tools, proposals and undo. The useful additions are stronger editorial review, explicit verification records, and a few restrained finishing treatments.

The best first investment is an optional thorough finishing workflow using tools we already expose. Follow that with review coverage stored against the edit revision, reusable word-timed emphasis graphics, and camera-specific push-ins. A Premiere computer-use integration would be a separate product decision with substantially different maintenance costs.

## Scope and evidence

The external project is [per-simmons/astra-premiere-editing](https://github.com/per-simmons/astra-premiere-editing). I read its README, SKILL.md, agent metadata, and all ten linked editing references through GitHub/raw pages. The source describes September 2026 demonstrations. GitHub's root listing shows documentation and agent metadata, without an executable editing engine or test suite in that listing. No license file appeared in the inspected root listing; verify reuse terms before copying material.

This is a source review, not a live Premiere or AeroEdits editing benchmark. Neither app was operated, no example media was exported, and no latency, cost or editing-quality comparison was measured. External links refer to the inspected `master` pages, not an immutable commit: fetching the remote Git checkout failed because shell networking could not resolve GitHub. The browser source views remained available.

External instructions are evaluated as reference material; they were not installed or adopted as instructions for this review.

Local evidence comes primarily from:

- [The shipped AeroEdits skill](../src-tauri/src/mcp/skill/SKILL.md), [tutorial workflow](../src-tauri/src/mcp/skill/templates/tutorial.md), and [skill/template implementation](../src-tauri/src/mcp/skill.rs).
- [MCP reading/proposal tools](../src-tauri/src/mcp/tools.rs), [editing actions](../src-tauri/src/mcp/actions.rs), and [proposal validation/history](../src-tauri/src/proposals.rs).
- [Waveform/timeline views](../src-tauri/src/mcp/views.rs), [storyboard](../src-tauri/src/mcp/storyboard.rs), and [export checks](../src-tauri/src/check.rs).
- [Filler/retake criteria](../src-tauri/src/ai/fillers.rs), [candidate detection](../src-tauri/src/ai/candidates.rs), [sequence editing](../src-tauri/src/sequence/edit.rs), [layouts](../src-tauri/src/layouts/mod.rs), [zooms](../src-tauri/src/zoom/mod.rs), and [captions](../src-tauri/src/captions.rs).

## What the external skill actually demonstrates

It teaches a Codex agent to operate native Premiere on macOS through accessibility controls and screenshots. It is a personal procedure library, named for Pat in its front matter, rather than a general editing SDK. [README](https://github.com/per-simmons/astra-premiere-editing/blob/master/README.md), [SKILL.md](https://github.com/per-simmons/astra-premiere-editing/blob/master/SKILL.md).

Its end-to-end record describes assembling 55 sources, retaining 52 source blocks, removing pauses and retakes, and adding talking-head treatments and a cutaway. It also reports an assembly-order error that required correction. Crucially, the result still awaited editorial feedback, had not received the subsequently required waveform pass, and had no complete listening or export-playback validation. This is useful demonstration evidence with stated limits, not proof of a finished autonomous editor. [End-to-end record](https://github.com/per-simmons/astra-premiere-editing/blob/master/references/end-to-end.md).

## Capability comparison

| Area | External approach | AeroEdits today | Assessment |
|---|---|---|---|
| Agent control | Accessibility targets plus screenshots; refresh after UI changes | MCP/CLI calls over typed project state; revision-aware proposals | Keep our structured control path. UI observations are useful for verifying results, but introduce avoidable dependencies for our own edits. |
| Ordered take assembly | Numeric take inventory and explicit timeline-order audit | Media import, timeline reads and `placeAsset` operations exist | Add a convenient ordered-assembly workflow and source-order report. Do not describe file import as missing. |
| Pause removal | Repeat Premiere pause extraction with fresh results | Audio silence and transcript gaps are separate passes, with dry runs and speech protection | Already covered by stronger app primitives. Add bounded residual review rather than an unconditional delete-until-empty loop. |
| Bad takes | Full-transcript reading and contextual phrase decisions | Rules, cloud/local review, candidate windows and an explicit thorough review option | The gap is default workflow coverage and evidence of review, not the absence of retake detection. |
| Waveform review | Sequential visual inspection and audition of joins | `timeline_view` combines filmstrip, waveform, words and proposals | We have the inspection surface. A recorded review queue and convenient audition/export of short audio ranges would augment it. |
| Static camera reframing | Small crop changes with matched eye height | Camera-only punch-in, face centring and manual clip framing | Existing support is partial; automatic comparison of eye-line/headroom across a cut would add value. |
| Eased camera push | Camera treatment with a chosen pivot, eased motion and hold | Screen zooms have easing; layouts have transitions | Screen zoom is not a dedicated phrase-driven camera push. Add camera-specific semantics if this style is wanted. |
| Word emphasis | Independent word reveal with fade/rise and fixed phrase layout | Word-highlight captions plus authored HyperFrames graphics | Feasible through the graphics system; a supported word-timed preset and timing input are the missing convenience. |
| B-roll/cutaways | Import and overlay selected video while keeping narration | `media_import`, video stream selection, source ranges and higher-track placement; `short_footage` for take shorts | Basic placement exists. Add semantic selection, explicit narration-preservation controls and boundary review. |
| Verification | Detailed logs distinguishing actions from verification and approval | Structural `check`, frames, storyboard, proposal history and undo | Add persistent review evidence. Our current checks do not establish that speech sounds natural or motion plays smoothly. |
| Personal preferences | Human-approved procedures recorded in Markdown | Saved workflow answers, look templates, brand kits and graphics library | Extend existing preference storage with demonstrated examples and approval provenance. |

## Good ideas worth adopting

### 1. Separate execution, verification and editorial approval

The external skill explicitly distinguishes an exposed control, an action actually executed, a result inspected by the agent, and a capability approved by the user. It also records unresolved limits. That is a good model for honest status reporting. [Skill rules](https://github.com/per-simmons/astra-premiere-editing/blob/master/SKILL.md).

Our proposal history already records acceptance, rejection, autopilot and applied revisions. That answers what changed; it does not record whether somebody listened across a join or approved a framing style. Add evidence alongside this history rather than creating a second edit log.

### 2. Review meaning across the full transcript when quality warrants it

The bad-take procedure reads the complete transcript, distinguishes abandoned speech from intentional repetition, and rereads the result after edits. It warns against interpreting transcription mistakes as performance mistakes. [Bad-take review](https://github.com/per-simmons/astra-premiere-editing/blob/master/references/bad-takes.md).

Our criteria already protect completed takes and meaningful speech. However, the tutorial defaults to candidate windows and only requests `fillers_review` with `review: "thorough"` when needed. Candidate review is efficient, but cannot establish that an unflagged tangent or incomplete transition was considered. Offer an explicit thorough option, keeping the current fast workflow available.

### 3. Treat waveform inspection and listening as separate evidence

The waveform reference is especially useful because it says the requested complete pass was still unverified. It asks editors to inspect residual gaps, preserve intentional pauses, and play through joins; seeing a flat waveform does not settle the editorial decision. [Waveform procedure](https://github.com/per-simmons/astra-premiere-editing/blob/master/references/waveform-cleanup.md).

For us, `timeline_view` is already the right visual substrate. Add a residual-gap/join review queue and track whether each item received visual inspection, listening, or a user decision. The `check` implementation reads edit metadata without decoding media; a clean report cannot certify clean audio.

### 4. Make motion and text restrained, deliberate and inspectable

The static framing reference compares adjacent frames and accounts for inherited crops before applying another transform. [Static framing](https://github.com/per-simmons/astra-premiere-editing/blob/master/references/jump-cuts.md). The push-in procedure checks the pivot, easing, hold and return; its demonstration used representative frames without export or listening verification. [Push-ins](https://github.com/per-simmons/astra-premiere-editing/blob/master/references/push-ins.md).

The text demonstration reveals words independently while preserving their horizontal positions. Its player-control clearance was not independently tested. [Text animation](https://github.com/per-simmons/astra-premiere-editing/blob/master/references/text-animations.md).

These are good design constraints for new presets. The exact font, pixel positions, timing and transform values belong to those examples and should not become AeroEdits defaults without our own footage tests.

## Weaknesses and approaches to avoid

**UI fragility is a recurring operational cost.** The control map says clip bodies, edit edges and transcript text are not usefully exposed as named objects. Focus, workspace and selection affect available controls. That makes a long edit dependent on repeated observations and recovery steps. Our typed API should retain responsibility for timeline mutations. [Accessibility coverage](https://github.com/per-simmons/astra-premiere-editing/blob/master/references/accessibility-controls.md).

**A successful demonstration is not broad reliability evidence.** The repository's notes are unusually candid, but they do not provide a benchmark corpus, measured failure rate or exported-output comparisons. Do not rank either system's real-world quality from procedure detail alone.

**The documentation mixes general procedure with session state.** Personal paths, named practice sequences, custom shortcuts and pending approvals are useful history but add noise for a new user. Some earlier status statements are superseded by later continuations. Our equivalent should have concise reusable procedures plus structured, dated evidence stored separately.

**Removing every detected pause is not a universal editing objective.** The pause example used a very small threshold and repeated extraction. Its own procedure qualifies completion by that threshold and stops blind retries when progress stalls. AeroEdits should preserve its dry runs, margins and two-source protections, and expose residual findings for review. [Pause pass](https://github.com/per-simmons/astra-premiere-editing/blob/master/references/text-based-pauses.md).

**Downloading a sample does not demonstrate automatic contextual B-roll sourcing.** The B-roll reference demonstrates an arbitrary sample, video-only overlay and boundary checks; the later full edit uses the creator's footage contextually. The download also depended on an installed toolchain and authorized browser-cookie access. Start our product workflow with project media and explicit source metadata. [B-roll procedure](https://github.com/per-simmons/astra-premiere-editing/blob/master/references/b-roll.md).

**Do not turn every new step into a mandatory approval stop.** Capability training and an authorized production run need different behavior. Existing saved answers, dry runs and proposal acceptance can support careful editing without requiring a new conversation at every operation.

## Proposed augmentations, in priority order

All items below are proposals, not implemented features. Effort is relative scope, not a delivery estimate.

### P0: Thorough editorial workflow using existing tools — small

Add a quality choice to the tutorial template: current fast cleanup or thorough review. Thorough mode reads every kept transcript page, performs contextual retake decisions, reviews residual gaps and joins through `timeline_view`, then runs `check` and `storyboard`. Finish cuts before adding graphics, as our workflow already requires.

Use existing `fillers_review` thorough pages and stamps for filler/retake review, and `propose_cuts` with stable word IDs for additional editorial cuts. Preserve explanations, deliberate examples and intentional waiting during demonstrations. The shipped skill's advice to avoid transcript/frame reads needs a scoped exception for this mode.

Acceptance: include an abandoned fragment outside the heuristic candidate windows, an intentional repeated explanation and a soft word near a pause in a small real-media evaluation. The report must distinguish transcript coverage from audio listening and list unresolved decisions.

### P1: Persistent review coverage and a join audition queue — medium

Store a review record with the project revision, transcript identity, source/word anchors, reviewed ranges, check type, result and reviewer. Keep user approval separate from automatic acceptance. A cut affecting a reviewed join must invalidate that evidence; a caption-only change need not invalidate audio review, but must invalidate affected visual review.

Build the queue from retake cuts, uncertain word boundaries, residual gaps and existing check findings. Let the user play a few seconds around each join and mark keep/refine. The current MCP inspection tools return images/text; a bounded audio/AV review artifact would give compatible agents listening evidence. Do not assume every client can consume audio.

Acceptance: reports never claim full listening from a contact sheet; partial coverage is visible; edits stale the relevant evidence; review actions can be resumed after reopening the project.

### P1: Reusable word-timed emphasis graphic — small to medium

Extend the existing HyperFrames graphics pipeline with explicit word IDs and relative onset times. Reserve the complete phrase layout, reveal words with a small rise/fade, and keep settled words stationary. Reuse brand kits and the graphics quality gate. Support occasional emphasis independently from continuous captions.

`graphic_add` already anchors a graphic at a word. That is not sufficient for per-word animation after internal cuts or timing changes: resolve all referenced words to the current edit, regenerate timing and invalidate renders when necessary. Respect the existing `data-overlay`/`data-entrance` policy rather than fading every full-screen card.

Acceptance: rendered previews and exports agree on reveals; long/Greek phrases wrap correctly; changing aspect ratio remains readable; cuts and speed changes update timing; placement avoids faces, captions and delivery safe areas.

### P2: Camera push-ins and matching across jump cuts — medium to large

Build on camera-only punch-in and manual framing, adding a camera-specific move with start/end scale, normalized pivot, duration, easing, hold and return behavior. Keep it separate from telemetry-driven screen zooms. Inspect the combined face crop, layout and transform so treatment cannot accidentally compound into a clipped head.

Acceptance: compare frames around cuts and sample the move's beginning/middle/end; check headroom and borders; verify no snap at the hold/return boundary. Exercise both the shared compositor and macOS native preview/export paths. Existing face centring should not be described as stabilization or guaranteed eye-line matching.

### P2: Ordered take assembly and contextual cutaway proposals — medium

Provide an assembly preview that parses section/take components (`1.2`, `1.10`), highlights ambiguous names, and lists intended order before committing. Audit actual source-block order afterward through project state.

For cutaways, wrap existing import and `propose_edit` placement with narration word anchors, chosen source range, video-only default, fit/cover and a short reason. Prefer project media. Preview both boundaries and verify that the narration and overall duration remain unchanged. Persist source information when externally acquired media is introduced.

Acceptance: natural numeric ordering is deterministic; missing/duplicate take labels are shown; an overlay with an audio stream does not introduce sound by default; later cuts cannot silently leave a cutaway attached to unrelated narration.

### P3: Learn editorial preferences from approved examples — medium

Extend saved workflow answers, look templates and the graphics library with approved example references and scope: one project, one video type, or future edits. Store the settings and example separately from the shipped skill: the installer owns and updates its marked skill text.

Acceptance: rejecting one cut does not silently change global defaults; approved examples can be revised or removed; the agent can explain which saved preference influenced a proposal. Avoid claiming automatic learning from the existing proposal history alone.

## Suggested delivery sequence

1. Ship the thorough template option and explicit review summary first. This uses our current tools and exposes where review costs actually accumulate.
2. Add review coverage and join audition, then evaluate missed retakes, speech damage, intentional-pause preservation and user corrections on real recordings.
3. Add the word-emphasis preset; introduce camera push-ins once rendering parity and crop behavior have a focused test set.
4. Add ordered assembly and contextual cutaway conveniences when multi-take long-form editing becomes a priority.

The transferable value is the discipline of documenting what was checked, revisiting editorial meaning, and verifying finishing details. AeroEdits can incorporate those practices through its existing editing engine and agent tools.
