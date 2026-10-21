# ChatCut compared with AeroEdits

Reviewed: 21 October 2026.

## Recommendation

ChatCut's most useful advantage for us is the workflow around creative editing: reference a
look, try a short sample, revise it precisely, then apply it across an editable timeline.
AeroEdits already has much of the infrastructure needed to support that experience. The
first investment should be a better art-direction and revision workflow using HyperFrames,
followed by precise editor-to-agent references. Replacing the renderer or adding every
generative model would have a weaker connection to our immediate tutorial use case.

The second tutorial adds a concrete near-term opportunity: a guided B-roll planning and
placement workflow built on our existing timeline operations. It also strengthens the case
for direct graphic controls and persistent editing preferences. See the
[second-video review](#second-video-review-brendan-jowetts-full-editing-workflow) and
[B-roll implementation](#7-guided-b-roll-placement-before-a-generation-suite).

This is a source-informed product assessment, not a measured claim that either editor
produces better cuts or renders faster.

The [deeper product audit](#deeper-product-audit-homepage-and-operational-documentation)
adds two priorities: saving a proven editing workflow for future episodes, and native
tutorial effects such as spotlighting and redaction. Local media/rendering are shared
capabilities, so they should not be presented as exclusive differentiation.

## Evidence and limits

- Read the supplied video's complete English auto-generated transcript and visually
  inspected its introduction, assembly workflow, several graphics revisions, cost screen,
  and final-result section. This was a transcript-led review with sampled playback, not
  uninterrupted audiovisual viewing.
- For the second supplied video, read the complete 21:08 English auto-generated transcript
  and visually inspected the prompting/setup state, finished graphics, direct color controls,
  both B-roll examples and export settings. This was also a transcript-led review with
  sampled playback; no independent listening, full exported-film review or timing benchmark
  was performed.
- Read ChatCut's plugin page and documentation for selection, design styles, graphics,
  libraries, versions, generation credits, voiceover, transparent export and XML handoff.
  Documented features below remain vendor claims; I did not install ChatCut, upload our
  footage, or conduct an export or quality benchmark.
- Inspected AeroEdits' current source, including agent tools, graphic variables, brand kits,
  library persistence, render cache, project revisions and relevant UI. Existing local
  research was context; implementation findings below come from source as well as README.
- No application code was changed. Earlier research and roadmap files were left untouched.
- A subsequent product audit read the canonical homepage, public graphics-style gallery,
  timeline, Desktop/local-agent, Skills, generation approvals, model capability, credits,
  collaboration, CapCut handoff, effects, recording, translation, export recovery and selected
  changelog documentation. The public homepage was also inspected in the browser, including
  both billing-period views. The supplied campaign URL did not load through web fetching;
  the canonical `https://chatcut.io/` did. No authenticated editor, installation, paid job or
  private project was tested; interactive homepage demonstrations are marketing examples.

## What the supplied video actually establishes

[Marcin AI's demonstration](https://www.youtube.com/watch?v=F8dxyyqkw9M) is about iteratively
producing an intro, rather than proving unattended editing of a long production:

| Video position | Observation |
| --- | --- |
| 1:08–2:37 | Supplies a motion-design reference and raw footage; requests cleanup before graphics. |
| 4:37–6:18 | Tests five seconds, then revises shadows, glow and translucent cards before extending the treatment. |
| 6:18–7:55 | Requests overlays without a full-screen background and expands to twenty seconds with synchronized graphics and sound. |
| 8:03–8:23 | On-screen text estimates **$6.40** of host API-equivalent usage so far and explicitly excludes ChatCut charges. Auto-captions incorrectly render the amount as “$640.” |
| 9:16–10:58 | Continues the approved direction and presents the finished intro with website imagery and animated elements. |

Interpretation: the transferable technique is approving a small representative sample before
scaling production. The demonstration does not establish total cost, general reliability,
or that every showcased result is a dedicated ChatCut feature rather than agent-authored work.

## Second-video review: Brendan Jowett's full editing workflow

[Opus 5.5 is INSANE At Video Editing (Full Tutorial)](https://www.youtube.com/watch?v=Gw198jpieAE),
published 8 October 2026, explicitly discloses ChatCut sponsorship.

### Timestamped observations

| Position in tutorial | Evidence |
| --- | --- |
| [5:05](https://www.youtube.com/watch?v=Gw198jpieAE&t=305s) | Shows a browser timeline alongside an external agent workflow. |
| [6:31–8:44](https://www.youtube.com/watch?v=Gw198jpieAE&t=391s) | Requests a whole-video edit informed by earlier channel examples. |
| [9:47–11:35](https://www.youtube.com/watch?v=Gw198jpieAE&t=587s) | Shows camera-with-graphic layouts, full-screen cards and overlays. |
| [11:41–13:06](https://www.youtube.com/watch?v=Gw198jpieAE&t=701s) | Creator praises retake removal; the complete source/output comparison is not shown. |
| [13:10–14:21](https://www.youtube.com/watch?v=Gw198jpieAE&t=790s) | Selects a graphic on canvas and exposes color, text and scoped AI controls. |
| [14:26–17:12](https://www.youtube.com/watch?v=Gw198jpieAE&t=866s) | Requests unwanted captions removed and background music lowered, then reviews changes. |
| [17:15–19:53](https://www.youtube.com/watch?v=Gw198jpieAE&t=1035s) | Agent proposes B-roll ideas/passages; creator sources two clips and chooses positions; results use trimmed footage over narration. |
| [20:20–20:52](https://www.youtube.com/watch?v=Gw198jpieAE&t=1220s) | Shows 4K/60 export settings and claims cloud rendering is faster. |

This is a sponsored case study, not controlled evidence of model superiority, perfect
editing, complete production cost or comparative render speed.

### What changes in our product interpretation

The following are implementation recommendations informed by the review and our source,
not additional claims of tested ChatCut performance.

**1. Make revision scope a first-class part of the workflow.**
Extend the proposed selection references with an explicit scope: element, graphic, scene,
track or whole timeline. Simple text/color adjustments should use local controls. Structural
design requests should carry the selected graphic's ID, current variables and revision to
the agent. Before applying, show which properties and clips will change. A color correction
must not regenerate a whole sequence or discard a manual text correction. Re-read current
state after a user edit so an agent cannot overwrite it using an earlier cached view.

This strengthens implementation items 2 and 4. It does not require us to build an entire
in-app chat service first: a targeted external-agent request is a useful initial version.
ChatCut's [AI panel documentation](https://chatcut.io/docs/ai-panel-overview) independently
describes project references, task progress and contextual requests; our Agent panel currently
handles proposals/history and relies on conversation in the external agent's app.

**2. Persist editorial preferences separately from visual branding.**
Extend saved workflow answers with caption intent, graphic density, punch-in strength,
music behavior and permitted use of B-roll. Keep these configurable for long videos and
shorts separately. A tutorial can need explanatory titles without continuous burned-in
subtitles. Model those separately so “hide captions” preserves diagrams and step headings.
Provide a reversible suppression operation rather than removing the transcript needed for
editing and timing.

Local evidence: `captions.rs` already has an enable switch and honors per-word hidden marks;
`transcript/store.rs` persists caption marks. This is an orchestration and preference-design
task, not a reason to replace the caption engine. ChatCut also documents caption visibility
and hiding individual cues without deleting media
([caption documentation](https://chatcut.io/docs/transcript-and-captions)).

**3. Treat narration continuity as an invariant of illustrative B-roll.**
A B-roll plan should identify the concept being explained, its word-ID anchors, candidate
asset, selected source range, destination and reason. For a software tutorial, search the
creator's own screen captures and result footage first. Show candidate frames before
placement. Use generic illustrative footage only when it helps understanding; avoid
substituting it for the actual UI action the viewer needs to follow.

The initial version can work entirely with supplied files. Asset search, downloading and
generation are separate extensions; none is required to prove useful placement. ChatCut's
[media documentation](https://chatcut.io/docs/my-assets) likewise distinguishes importing
reusable media from attaching prompt context and from placing it on a timeline.

**4. Audio adjustment needs a clear target and preserved dialogue.**
Expose music and narration tracks by role and provide an audition range around a mix change.
Keep integrated loudness normalization separate from music gain and speech ducking. A global
gain change is not equivalent to “keep the intro energetic, then lower the music.” For the
first version, support separate music clips/tracks for those sections; a later version can
add a gain envelope and transition ramps if creator tests require it.

Local evidence: `project/audio.rs` already provides per-track `volume_db`, `duck_db`, gain
conversion and global normalization. The inspected `TrackMix` has no time-varying gain
envelope. Improve the agent's targeting and preview workflow before adding another audio
processor. Check the speech-to-music balance with playback; a storyboard cannot verify it.

**5. Scale the graphic plan across a whole lesson.**
Add a chapter-level coverage map to the style trial: which sections benefit from a visual
explanation, where a screen demonstration should remain unobstructed, and where plain camera
footage provides a useful break. Record graphics as explanatory beats with a purpose, not a
fixed number of animations per minute. Evaluate semantic relevance, reading time and
consistency across the middle and ending as well as the opening.

Our existing storyboard and `useWhen` library metadata support this. Add an intermediate
review of the beat plan before rendering many new graphics, followed by a full-edit
storyboard and targeted motion previews.

### Export clarification from current documentation

ChatCut's current [web/desktop export documentation](https://chatcut.io/docs/web-export-limits)
describes compatible manual web exports rendering in the browser, with cloud fallback for
unsupported cases. Agent Plugin exports default to browser rendering and require an explicit
cloud request. Desktop video export runs locally. Browser export can offer 4K on compatible
devices; actual cloud duration includes upload, network and queue time. Accordingly, browser
access does not establish that a job ran in the cloud, and the route alone cannot establish
faster delivery. The [desktop export guide](https://chatcut.io/docs/desktop-local-export)
also documents local 4K delivery.

For AeroEdits, 4K and configurable export frame rate already exist. Track preparation,
rendering and file completion separately when benchmarking, and verify the resulting file
rather than counting an export-settings screenshot as a completed output. No cloud renderer
is recommended on speed grounds without measuring equivalent projects end to end.

## Deeper product audit: homepage and operational documentation

### Product structure and reusable workflows

The [homepage](https://chatcut.io/) presents outcome-oriented entry points for cleanup,
shorts, graphics, translation and promotional videos, with editable-result demonstrations.
The public [graphics-style gallery](https://chatcut.io/skills/motion-graphics?template=mg-packaging-editorial-explainer)
lets users choose a named treatment and supply talking-head footage. The lesson for our
onboarding is to demonstrate an achievable result before exposing a long settings form.
For tutorials, show three examples: cleaned screen tutorial, visual explanation, and a short
made from a new take. Avoid measuring our gap by their advertised quantity of skills.

ChatCut's [Skills documentation](https://chatcut.io/docs/skills-overview) defines a Skill as
reusable agent workflow guidance. Users can attach it to a request and save a successful
process through a guided summary/confirmation flow for use in other projects. This is
separate from reusable graphics and visual branding.

Our source has three different reuse mechanisms already:

| Mechanism | Current implementation | Remaining product work |
| --- | --- | --- |
| Editing method | `mcp/skill.rs` ships three fixed workflows and stores their saved answers. | User-authored, named workflows capturing decisions and review steps from an approved edit. |
| Saved look | `templates.rs` and `TemplatesMenu.tsx` save/apply layout, captions, zoom settings and graphic-brand selection. | Clear naming and discovery; avoid implying saved settings contain the full editing method. |
| Graphic asset | `graphics/library.rs` stores source, variables, usage guidance and item versions. | Previewable collections associated with a series look and editing method. |

**Proposed addition:** “Save this editing workflow” should capture reusable instructions,
defaults, exceptions and quality checks, with references to a look and graphics collection.
Exclude episode-specific paths, word IDs and timestamps. Show a readable summary before
saving; version it and allow an episode to keep its earlier version. First prove reuse across
two tutorials with different footage. A public marketplace is unnecessary for that test.

### Operational gaps beyond generated graphics

These are documented competitor capabilities, not independently tested results. Local
absence findings mean no equivalent was found in the inspected source paths, not that a
whole-repository runtime audit proved impossibility.

| Area | What ChatCut documents | AeroEdits comparison and proposed response |
| --- | --- | --- |
| Timeline variants | Create, duplicate and switch timelines sharing one media pool. [Timeline](https://chatcut.io/docs/timeline) | We have a main sequence and independently editable shorts, not a generic sequence-tab model. Consider full-length alternate edits and duplicate-for-review after checkpoints; keep them distinct from 3-minute-limited shorts. |
| Cut polish | Audio crossfades/video dissolves at eligible cuts; separate track controls for ducking. [Timeline](https://chatcut.io/docs/timeline) | Gain/ducking exist. `SequenceEdit` has no crossfade/transition operation. Evaluate short audio seam fades with adequate source handles before decorative video transitions; speech timing must remain intact. |
| Tutorial effects | Desktop exposes masks, magnifier/local-mosaic geometry and blending, with direct manipulation. [Effects](https://chatcut.io/docs/ai-effects) | Existing zoom/crop/shadow controls are not a general effect stack. Add a small native set: spotlight, redaction and magnifier, anchored to the demonstrated screen area and recording time. |
| Localization | Translated captions remain separately editable and follow retained source timing; dubbing/lip-sync is a separate workflow. [Captions](https://chatcut.io/docs/transcript-and-captions), [translation](https://chatcut.io/docs/video-translation) | We transcribe English/Greek and have word-aware captions, but no translation pipeline was found. For Greek tutorials, test English subtitle translation/export before dubbing or lip-sync. Preserve authored translations through cuts and undo. |
| Recording corrections | Records voiceover, camera or screen at the playhead and recovers interrupted takes. [Recording](https://chatcut.io/docs/recording) | Imported takes and voice/camera alignment already exist; no equivalent record-at-playhead flow was found. Integrate a correction take from AeroShoot or a small narration recorder before rebuilding screen capture inside this editor. |
| Media organization | Hierarchical bins, folder structure, relinking and processing/retry states. [Media](https://chatcut.io/docs/my-assets) | We already import folders and filter/search media. Consider bins and relink repair as libraries grow; do not list basic folder import or search as missing. |
| Delivery recovery | Queue entries expose stages, cancellation, retry and result retrieval, but are not a durable account archive. [Export queue](https://chatcut.io/docs/export-history) | `ExportOwner` holds one optional live job and supports status/cancel. A durable delivery list and batch-short queue would add value; preserve completed file paths and the revision exported. |
| Team review | Separate watch links and editing invitations; watch media lasts one day and there is no comment-only collaborator role. [Sharing](https://chatcut.io/docs/collaboration-and-sharing) | No comparable collaboration service was found. Start with an exported video/storyboard review package and timestamped notes. Defer real-time coediting unless teamwork becomes a core need. |
| CapCut delivery | Desktop-only editable CapCut/Jianying drafts, with optional rendered graphics and explicit unsupported items. [Draft export](https://chatcut.io/docs/export-capcut-jianying) | This broadens the earlier interchange finding. Determine creators' destination editor before choosing an interchange; scope and validate one handoff first. |

**Native tutorial effects need real compositor support.** A graphic overlaid on a recording
cannot safely be assumed to alter the pixels beneath it. Redaction and magnification need
consistent source-to-canvas mapping through crop, zoom, layout changes and cuts, in both
preview and export. Start with explicitly positioned, time-bounded rectangles. Test a region
near the edge during a zoom and active-window change. Do not infer automatic object tracking
from the presence of a movable mask control.

**Localization needs more than a second text field.** Give translated cues stable source
anchors and an independent revision history. Manual wording fixes must survive repagination;
source cuts must update coverage without squeezing a whole translated sentence into the
remaining fragment. Keep subtitle export independent of whether captions are burned into
the picture. For software terms and product names, include a creator-reviewed glossary.

### Agent architecture and cost need to be compared by route

The [Desktop guide](https://chatcut.io/docs/desktop-app) documents both an agent running
inside the editor and an external desktop agent controlling the native project through local
MCP. Its [Agent Plugin guide](https://chatcut.io/docs/agent-plugin) separately describes a
hosted connection and an editor in the agent's Browser panel. Desktop supports local project
packages, local media and rendering; some services still require the network. This makes
“our editor is local and agent-driven” an insufficient standalone competitive claim.

Our initial advantage to test is the quality and predictability of tutorial-specific behavior:
telemetry placement, correct screen/camera timing, retained speech and efficient review.
An embedded agent composer could reduce context switching, but is a larger integration than
selection references. Preserve our external MCP/CLI path while testing whether an embedded
conversation improves correction speed enough to justify its maintenance and billing work.

The [credits policy](https://chatcut.io/docs/credits-policy) distinguishes built-in agent
messages, which use variable model billing, from external host reasoning, which uses the
host's allowance. Hosted generation is another charge category; manual editing,
transcription and export do not consume credits, though plan gates remain. Therefore an
ordinary edit does not necessarily need paid media generation. Compare the same route and
requested assets before making a cost claim; generic raw-footage length is insufficient.

The [generation-confirmation guide](https://chatcut.io/docs/generation-confirmations)
documents estimates and category/scope-specific approval choices. Its scope descriptions
also differ in detail from the general credits page. If we add paid providers, make scope
explicit in the UI and persisted job record. Editing autopilot should not silently become
permission to spend on generated assets. Record the request, references, estimate, job ID,
outcome and actual cost; resuming a job should not submit or bill a duplicate.

### Marketing breadth does not establish feature parity across surfaces

The homepage names many models, but the [capability matrix](https://chatcut.io/docs/model-capability-matrix)
says Kling 3.0 is not currently in the manual Video Gen picker. Models also differ in
reference inputs and edit/extend support. Treat this as a concrete reason to verify the
active tool/surface, not as proof that every named model is available everywhere.

Recent release notes are useful evidence of product maturity and active maintenance:
[0.6.7](https://chatcut.io/docs/changelog/2026-10-10-smart-cutout-shared-luts-caption-fixes)
describes shared LUTs, translated-caption preservation and fixes to grading/media selection;
[0.6.9](https://chatcut.io/docs/changelog/2026-10-10-smart-cutout-update-0-6-9)
repairs cutout insertion and its first-frame preview. These do not prove unreliability today.
They identify practical regression cases worth using in our evaluation: first-use model
setup, reference selection, saved-look recovery and preview/export agreement.

### Resulting priority adjustments

1. **Keep the immediate focus:** style trials, precise references, a coherent tutorial
   graphics collection, guided B-roll and fast local revisions.
2. **Add a reusable editing-method capture:** build on our existing saved answers/looks and
   validate it on the next episode, rather than creating another independent preset system.
3. **Bring native tutorial effects and audio seam polish forward:** evaluate screen
   spotlight/redaction/magnification and speech-preserving seam fades before a broad effects
   library or more generative-video providers.
4. **Next expansion for this creator:** translated subtitle workflow, useful alternate
   timelines, and a durable batch-delivery list. Prioritize among these using actual repeated
   production work.
5. **Defer scope-heavy breadth:** public marketplace, live collaboration, avatars, lip-sync
   dubbing and a large model catalog until the tutorial workflow is proven.

Additional local evidence: `templates.rs`, `mcp/skill.rs`, `shorts.rs`,
`components/navigation/TemplatesMenu.tsx`, `components/media/MediaPanel.tsx`,
`sequence/edit.rs`, `render/mod.rs`, `render/composite.wgsl`, and `export/mod.rs`.
No competitor source code or authenticated editor behavior was inspected.

## What they offer, what we already have, and the remaining gap

| Area | ChatCut evidence | AeroEdits today | Implication |
| --- | --- | --- | --- |
| Agent-driven editing | Plugin connects conversational requests to an editor and live timeline. | MCP and CLI actions, agent setup, proposals, autopilot and undo already exist. | Improve onboarding and contextual revisions; “add AI editing” is not a useful backlog item. |
| Editing ordinary media | Product accepts recordings and other footage. | Empty projects and import of video, audio, images and `.aero` recordings already exist. | We are not restricted to `.aero`; assess import reliability separately. |
| Transcript cleanup and shorts | Advertises filler/pause removal and long-to-short editing. | Silence, gaps, fillers, retakes, word alignment, shorts and take-based shorts already exist. | Compare speech retained and correction effort on identical footage before claiming an accuracy gap. |
| Reusable graphics | Templates plus generated editable graphics. | HyperFrames source, declared variables, Inspector controls, reusable library, versions of library items and validation already exist. | Improve the available designs, discovery and iteration experience. |
| Reusable design direction | Styles hold visual references and written design rules as well as branding. | Kits hold background/text/accent colors, a font/font file and logo. | Extend kits or add an associated style document with composition and motion guidance. |
| Precise AI references | Selection Mode adds references from clips, media, viewer regions, timeline times and transcript selections. | Manual selections exist; the inspected MCP interface does not expose a comparable editor-selection reference workflow. | Add a structured context bridge; avoid asking users to describe timestamps and objects repeatedly. |
| Graphic revision | Documentation describes direct element selection and positioning on the desktop canvas. | Inspector edits declared variables; changes trigger a background render. | Element-level manipulation and quicker revision previews are meaningful gaps. |
| Audio and visual asset breadth | Libraries include effects, transitions, LUTs and sound effects; product also offers generated media and narration. | General media import, noise cleanup, mixing/ducking and synthesized telemetry clicks/whooshes exist. No equivalent integrated generation suite was found in the inspected action catalog. | Build the assets needed for tutorials first; add generation through a provider interface when justified. |
| Restore points | Named project versions alongside undo/redo. | Revision-checked edits and bounded undo history; reusable graphic items have their own versions. No named whole-project restore-point feature was found. | Add durable creative checkpoints before wide changes. |
| Delivery | Per-graphic alpha export and NLE interchange. | H.264/AAC final export; internal graphics cache already supports alpha WebM. | Expose standalone graphic delivery, then consider an explicitly limited timeline interchange. |
| Guided B-roll | Media import and contextual editing are documented; the second tutorial adds a placement example above. | Imported video, overlay roles, stream selection and proposed timeline edits already exist. | Add semantic planning and safe placement around retained narration; a new generation engine is not a prerequisite. |

External evidence: [plugin](https://chatcut.io/chatgpt-plugin),
[styles](https://chatcut.io/docs/design-styles),
[selection](https://chatcut.io/docs/selection-mode),
[graphics](https://chatcut.io/docs/motion-graphics),
[library](https://chatcut.io/docs/library),
[versions](https://chatcut.io/docs/versions-undo-redo).

### Important distinctions

**We already support transparent graphics internally.**
`src-tauri/src/graphics/render.rs` explicitly renders WebM VP9 with transparency, and shorts
have graphic placement slots. The opportunity is broader, easier composition and delivery,
not adding transparency from scratch. Ordinary graphics are layered over captions, which
also makes placement and occlusion controls relevant to tutorial readability.

**An editable source is different from an instant revision.**
Our source is editable and reusable, but changing a variable invalidates a render. ChatCut
advertises live property updates and documents direct canvas editing. Faster iteration is a
valuable target, but their actual latency and preview/export consistency remain untested.

**A visual style is more than a brand kit.**
We need guidance such as “show one idea per card, leave the demonstrated control visible,
use a short slide rather than a bounce, keep the heading hierarchy consistent.” ChatCut's
style documentation includes reference images and written guidance. It also says choosing a
style guides subsequent generations rather than automatically restyling existing graphics.
Our current brand-kit changes invalidate affected renders, so the semantics already differ.

**Agent work should not be confused with a built-in feature.**
Reference research, fetching assets, authoring custom graphics and repairing an import can
involve the host agent and external tools. We should copy useful workflows while separately
identifying which behaviors belong inside AeroEdits for predictable results.

## Implementation order

Effort below is relative engineering scope, not a calendar estimate. Acceptance criteria are
proposed checks, not results already measured.

Recommended delivery sequence: style trial, selection references, curated graphics and
supplied-file B-roll, faster revisions and checkpoints, then NLE interchange. The B-roll
scope is detailed in item 7; generation providers remain a later expansion.

### 1. A short style trial before producing the whole video — first

**Experience:** after cleanup, choose a representative 5–10 second range and a visual
reference. Produce two or three useful graphic beats, revise them, and approve a look before
applying it elsewhere. For an AI tutorial, test a step callout, a comparison or diagram,
and a result card. Each must explain something or direct attention.

**Implementation:** extend the existing editing template with an optional style trial;
store its approved reference, chosen assets and rules in project metadata. Extend brand
direction with spacing, type hierarchy, motion timing, reference images and exclusions.
Keep editorial decisions in a beat plan anchored to word IDs and source ranges, then use
the existing `graphic_add`, `graphic_write_file`, `graphic_render`, `graphic_check`,
`frame_render` and storyboard tools.

The beat plan should record the purpose, anchor, duration, placement and asset for each
graphic. Approval should persist these choices so a later revision does not silently invent
a different look. A deterministic check can catch timing and safe-area problems; visual
relevance still needs review.

**Scope:** small-to-medium for the workflow; medium for a reusable style model and UI.
**Acceptance:** an approved sample extends to another minute without changing its visual
language, covering the demonstrated control, or requiring repeated design instructions.

### 2. “Ask AI about this” references — next

**Experience:** select transcript words, a graphic, a clip, a timeline range or a rectangle
in the preview and use “Ask AI about this.” The request carries the exact selection.

**Implementation:** add a serializable reference containing project identity, revision,
timeline/short ID, clip/asset IDs, word IDs, source and edited ranges, and optional normalized
canvas rectangle. Include the transcript dependency stamp: transcript saves do not advance
the edit revision. Expose it through a read-only MCP action. Start with a copyable prompt or
context token for external agents; a built-in chat is a separate product decision. Reject
stale or mismatched references with an actionable response and reuse revision checks.

**Scope:** medium. **Acceptance:** “move this away from the button” resolves to the selected
graphic and region; switching shorts or editing the timeline cannot target the wrong item.

### 3. A curated tutorial graphics collection and better discovery

**Experience:** preview and search useful designs by purpose: step, shortcut, warning,
before/after, code highlight, model comparison, price comparison, workflow diagram and
result reveal. Include camera overlays, screen overlays and full-screen explainers.

**Implementation:** build on our existing library, `useWhen`, variables, contact sheets,
validation and brand bindings. Add purpose tags and searchable descriptions before building
a marketplace. Add representative motion previews and safe placement variants. Package
the same visual system across related templates rather than collecting unrelated effects.

**Scope:** small-to-medium platform work plus substantial design work.
**Acceptance:** an agent finds and adapts a suitable existing design for common tutorial
beats without writing a fresh composition each time.

### 4. Fast graphic revisions, then direct canvas controls

**Experience:** changing a headline, size or position should give useful feedback promptly.
Later, clicking a graphic element should identify exactly what can be edited.

**Implementation:** first add a lightweight still/low-resolution preview path with debounced
updates and a visible final-render state. Then expose stable element IDs and editable
bounds from compositions; map handles to declared properties. Reuse the same source and
time evaluation for preview and export. Treat blur of underlying footage as a compositor
feature: a translucent card rendered in isolation cannot automatically blur the video below.

**Scope:** medium for preview improvements; large for arbitrary element manipulation.
**Acceptance:** rapid edits cannot display a stale result as current; full-quality export
agrees with the approved preview, including alpha edges and placement.

### 5. Named creative checkpoints

**Experience:** save “clean cut,” “approved look” and “full treatment,” then restore any
one after reopening the project.

**Implementation:** snapshot the edit document plus referenced graphic source, asset
metadata and authored caption marks with their transcript dependencies. Transcript files
sit outside the edit undo history, so a document-only snapshot cannot recover every caption
revision. Do not rely only on revision numbers or the undo stack. Reference immutable media
where possible; preserve recoverable source versions when graphics are rewritten. Restoration
must create a new current revision and invalidate pending proposals and stale references.

**Scope:** medium. **Acceptance:** restoring a checkpoint recovers the intended graphics
and edit together, including after more changes than the undo history retains.

### 6. Portable graphics, then a constrained NLE handoff

**Experience:** export a selected graphic with alpha for another editor; optionally send
a rough cut to Premiere/Resolve for finishing.

**Implementation:** first expose supported standalone graphics delivery using the existing
alpha cache, adding ProRes 4444 only with platform-specific capability checks. Then evaluate
FCP7 XML or another chosen interchange for source ranges, track placement and audio. Export
unsupported compositing as rendered assets and show a precise limitations report.

ChatCut documents per-graphic ProRes 4444 export, rather than transparent export of its whole
timeline. Its XML handoff can omit captions, effects, transitions and several other constructs;
it is not a lossless project transfer. Sources:
[alpha export](https://chatcut.io/docs/export-transparent-motion-graphics),
[XML limits](https://chatcut.io/docs/export-xml).

**Scope:** medium for graphic delivery; large for reliable interchange.
**Acceptance:** test the exported graphic and a representative timeline in the destination
editor; account explicitly for every unsupported construct.

### 7. Guided B-roll placement before a generation suite

**Experience:** choose a few passages that need visual support, review candidate footage and
source trims, then place the selected clips while the original narration continues.

**Implementation:** compose the existing `transcript_get`, `media_import`, `project_info`,
`timeline_get` and `propose_edit` operations into a guided workflow. Use word IDs for
selection, resolve them against the current edited timeline just before placement, and
return the resulting clip IDs for later revision. Select picture streams explicitly: an
empty `PlaceAsset.streams` currently places all streams, including unwanted source audio.
Use a dedicated free video track with the appropriate overlay role. Avoid a placement onto
an occupied magnetic track, which can insert time and shift the edit. Preserve duration,
spoken content, existing graphics and intentional screen demonstrations.

Propose each placement with its semantic reason and a before/after preview; keep it one
reviewable undo unit. If the narration is cut afterward, mark its planned coverage stale and
re-resolve the anchors. A future retrieval system can index footage by source, descriptive
tags, time ranges and representative frames, without coupling that catalog to one model.

**Scope:** small-to-medium for supplied-file planning and placement; larger for semantic
retrieval across a media library. **Acceptance:** adding, replacing or removing B-roll does
not shift dialogue, import unwanted audio, break a short's context, or change runtime; a
later transcript edit cannot silently leave a clip attached to the wrong passage.

## Broader features worth considering later

Generated narration, images, video and music can reduce trips to other products. ChatCut
documents script-to-narration as an audio asset, with destination controlled by the request
([voiceover](https://chatcut.io/docs/ai-voiceover)); its product overview also presents the
other generation categories ([overview](https://chatcut.io/chatgpt-plugin)).

For us, start with a practical replacement-narration or explainer-asset workflow if creator
testing establishes demand. Use an adapter that returns ordinary project assets with
provider, prompt/reference, generation settings, job status and cost metadata. Support
retry/cancel without duplicating timeline clips or losing the previously approved asset.
Keep generated illustrative footage visibly distinct from captured product evidence in the
authoring workflow, especially when explaining real software behavior.

Color correction, background removal, LUTs and richer audio transitions are also legitimate
expansions. They are less directly tied to our present tutorial strengths than the six items
above. A full effects marketplace, avatars and a large video-model catalog would broaden
scope considerably before proving better tutorial outcomes.

## Cost and positioning

**Pricing pages do not agree.** The earlier
[plugin page](https://chatcut.io/chatgpt-plugin) displayed $25/month for 100 credits. The
canonical [homepage](https://chatcut.io/), inspected with Monthly selected on 21 October 2026,
instead starts its paid cards at 200 credits:

| Monthly credits | Displayed monthly billing | Displayed annual equivalent | Annual charge |
| --- | --- | --- | --- |
| 200 | $45/month | $42/month | $504 |
| 400 | $88/month | $49/month | $588 |
| 800 | $160/month | $98/month | $1,176 |

The [dedicated pricing page](https://chatcut.io/pricing) also displayed those annual figures.
These are promotional page observations, not a verified checkout quote. Do not assume the
$25 plan remains generally purchasable or compare an annual equivalent with monthly billing.
The homepage also lists a free starter option; feature entitlements and cloud export limits
must be evaluated separately from credits.

The [credits policy](https://chatcut.io/docs/credits-policy) distinguishes built-in agent
model usage, host-agent allowance and hosted-generation charges. Manual editing, uploads,
transcription and export do not consume credits; export limits or plan gates can still apply.
Estimates are not completed charges: use transaction history for actual usage. See also
[generation accounting](https://chatcut.io/docs/generation-credits).

For AeroEdits, report per-project local operations, provider charges and unknown/subscription
usage separately. Show estimated generation expense before a paid job and actual expense
afterward where the provider supplies it. Avoid presenting an API-equivalent estimate as a
complete bill. Local deterministic editing and cached reuse are useful cost advantages in
principle; comparative cost still requires an identical-input test.

Our strongest differentiation is tutorial-specific understanding: mouse/keyboard/window
telemetry, screen and camera layouts, avoiding important UI, word-aware cleanup, and
inspection of the finished composition. Those are implemented capabilities; this research
does not establish that ChatCut lacks all equivalents. Build around them and improve the
creative workflow surrounding them.

## Proposed comparison before a major integration

Use the same three inputs in both products: a short talking head, a 3–5 minute screen
tutorial with camera and telemetry available to AeroEdits, and an English/Greek tutorial
sample. Give both the same content goal, visual reference, asset set and revision requests.

Record time to an acceptable first cut, lost or clipped speech, graphics that obscure UI,
number of manual corrections, revision latency, render/export failures and complete external
spend. Ask the same reviewers whether graphics clarify the spoken concept. Include the
questions “change one phrase,” “move this callout” and “restore the approved look.” This
tests production usefulness and controllability alongside visual polish.

Extend that comparison with a whole-lesson revision exercise: retain manually corrected
graphic text, remove subtitle cues while preserving explanatory titles, lower only the
background track, insert two supplied video-only B-roll ranges, make a subsequent speech cut,
and undo the placements. Check the middle and ending of the film as well as its intro.
Measure how many extra instructions and manual fixes each revision requires. Our tutorial
template already calls for reviewing the first/last ten seconds and each retake cut; retain
that check rather than treating successful sample playback as proof of complete accuracy.

## Local implementation evidence

- `src-tauri/src/mcp/actions.rs`: editing actions, imports, shorts, graphics, branding,
  checks and export. `src-tauri/src/mcp/tools.rs` and `views.rs`: project reading and visual
  inspection interfaces.
- `front-end/src/components/settings/AgentsDialog.tsx`: existing agent connection/setup.
- `front-end/src/components/agent/AgentPanel.tsx:225`: proposal/history UI; explicitly an
  external-agent workflow rather than an in-app chat composer.
- `src-tauri/src/graphics/brands.rs:28`: current brand model.
- `src-tauri/src/graphics/library.rs:24`: reusable items, purpose guidance and item versions.
- `src-tauri/src/graphics/mod.rs:34`: editable source and declared variable schema.
- `front-end/src/components/inspector/GraphicSection.tsx:19`: variable editing and render
  refresh behavior.
- `src-tauri/src/graphics/render.rs`: alpha WebM cache, content hashing and render sizing.
- `src-tauri/src/project/revision.rs:24`: bounded undo; edit document/revision architecture.
- `front-end/src/components/project/NewProjectDialog.tsx:133`: empty projects with ordinary
  imported media.
- Additional second-video checks: `src-tauri/src/sequence/mod.rs` (`Role::Overlay` and audio
  roles), `sequence/edit.rs` (`PlaceAsset`, stream filtering and magnetic insertion),
  `project/audio.rs` (`TrackMix`), `captions.rs` (caption suppression), `transcript/store.rs`
  (caption marks), and `mcp/skill/templates/tutorial.md` (existing post-edit review guidance).

No runtime benchmark or application test suite was run for this documentation-only review.
