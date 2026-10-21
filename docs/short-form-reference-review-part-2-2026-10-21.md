# Short-form reference review, part 2 — 21 October 2026

This continues the [first three-video review](short-form-reference-review-2026-10-21.md). The new references strengthen the case for a focused short-form skill and add concrete opportunities in procedural audio, production requirements, render quality, and finishing controls.

The current application already supplies much of the execution machinery. Its value should be measured through dependable operations, editable decisions, reuse, and correction time. MCP connectivity by itself does not establish better editorial judgment.

## Evidence and scope

Read the complete English transcripts and descriptions of the three requested videos. Inspected sampled output frames and portions of the prompts shown on screen, including the Opus brief around 1:27–1:42. The original output projects and complete community-hosted skills were not audited. Creator judgments and timing reports are distinguished from code findings and our own recommendations below.

This is a workflow review with sampled visual inspection. It is not a complete audiovisual quality assessment, a measured model comparison, or a hands-on reproduction of the examples. No application or skill code was changed and no render benchmark was run.

Source inspection used repository commit `fbc06da`, with a clean working tree before these research edits. In particular, the current code has unified layout sections and a `layouts` MCP action; recommendations should build on those rather than introduce another layout system.

## The three references

| Video | What the demonstration contributes | What it does not establish |
| --- | --- | --- |
| [Opus creates and edits entire videos](https://www.youtube.com/watch?v=MXh8tNsEdvU) | A raw talking head becomes a reported 41-second short. Separate examples create a launch trailer and a Three.js animation, with code-generated audio. The visible editing brief is detailed, including dialogue joins, layout selection, stems, and verification. | Success frequency, reproducible cost, or general superiority over another model. One user prompt can still involve substantial internal work. |
| [Astra editing results](https://www.youtube.com/watch?v=wZtuanHjumw) | The same recording and three prompts are tried as standalone renders and in Borumi. The creator finds limited visual improvement from the editor alone, identifies a weak result, and prefers the later custom-skill treatment. | A controlled measurement of the skill's effect: the later treatment changes the instructions, and the examples are a small, subjective sample. |
| [Claude and HyperFrames workflow](https://www.youtube.com/watch?v=ciCRH4Q8EkQ) | A manually trimmed 16-second input becomes a branded short using a dedicated skill, with a reported five-minute run. Style settings are separated into a brand file. | A five-minute complete raw-footage cleanup workflow. Captions and sound are discussed as possible additional work at the end. |

Useful source locations: [Opus prompt and first output](https://www.youtube.com/watch?v=MXh8tNsEdvU&t=78s), [Astra editor comparison](https://www.youtube.com/watch?v=wZtuanHjumw&t=479s), [Astra custom skill](https://www.youtube.com/watch?v=wZtuanHjumw&t=694s), [HyperFrames input preparation](https://www.youtube.com/watch?v=ciCRH4Q8EkQ&t=330s), [HyperFrames brand configuration](https://www.youtube.com/watch?v=ciCRH4Q8EkQ&t=592s).

## What this changes in the assessment

### 1. Capture a reusable production brief

The Opus example makes an important operational point: a successful request can encode a complete production process. Its visible brief covers synchronization, speech editing, camera visibility, composition, audio delivery and final verification. Treat that as a source of requirements to distill, rather than a long prompt to resend unchanged on every job.

Separate three types of instruction:

- **Requirements:** target aspect ratio, chosen output quality, speech preservation, source synchronization, deliverables, and editable output.
- **Style preferences:** camera versus graphic balance, caption treatment, hard cuts versus moves, sound coverage, and musical character.
- **Analysis results:** transcript, source ranges, uncertain cuts, visible actions and measured audio properties.

The app stores and validates requirements and computes reusable analysis. A versioned style profile holds preferences. The skill makes the editorial choices using that information. MCP conveys the selected profile and plan and returns concise execution results.

Numeric audio thresholds and join timings shown in a successful example should remain configurable treatments. Their effectiveness depends on the recording, room noise, word alignment and performance; a copied number is not a general quality guarantee.

### 2. Distinguish brief compliance from personal taste

The Astra creator requests a restrained treatment, then dislikes its lack of visual excitement. This is useful evidence about his preferred style, but it is not proof that the system failed merely because it followed a minimal brief. Sources: [minimal treatment](https://www.youtube.com/watch?v=wZtuanHjumw&t=146s), [creator assessment](https://www.youtube.com/watch?v=wZtuanHjumw&t=198s).

Evaluate our outputs separately for:

- Whether they preserve meaning, meet the requested treatment and render correctly.
- Whether the selected treatment suits the user, topic and audience.
- Whether the execution of that treatment is visually and sonically strong.

An agent can satisfy the first and still need direction on the other two. Explicit style examples, including why a result was accepted or rejected, are more useful than asking for an undefined premium or viral result.

### 3. The editor should preserve decisions and reduce repeated work

The Borumi comparison makes editable finishing a concrete benefit. That supports our architecture. It also shows why adding an MCP connection cannot substitute for style guidance.

Be precise about editability: code-authored projects can retain editable source, even when the demonstrated handoff is an MP4. AeroEdits adds native controls over cuts, layouts, captions and audio, while a rendered graphic retains its source and variables. The internal layers of a HyperFrames or Three.js graphic do not automatically become separate native timeline objects.

Our useful target is efficient revision: change a caption treatment, move one visual, replace a sound, or adjust a layout while retaining the other approved decisions. Preserve stable IDs, user overrides and dependency-aware caches. Avoid turning every small revision into a new whole-video creative task.

## Current application findings and ownership

| Area | Confirmed current foundation | Remaining work | Owner |
| --- | --- | --- | --- |
| Layouts | [`layouts`](../src-tauri/src/mcp/layouts.rs) supports video/short scope, base layouts, batched sections, favourites, retiming, transitions and punch-ins. [`LayoutSection`](../src-tauri/src/layouts/mod.rs) stores asset-time anchors. | Select treatments by meaning and coordinate graphics/captions with them. Extend this system. | Skill selects; app executes; existing MCP action exposes it. |
| Graphics | Editable HyperFrames source, variables, brand kits, transparent renders and caching already exist. | A focused short-form recipe collection and a predictable connection to the editorial plan. | App/library plus skill selection. |
| Render quality | [`render.rs`](../src-tauri/src/graphics/render.rs) sets graphics to 30 fps and a 1920-pixel canvas long side. Export requests graphics through that specification. | An explicit draft/final quality policy that can match the intended output. | App, with quality options exposed through MCP. |
| Sound generation | [`sound_effects.rs`](../src-tauri/src/sound_effects.rs) already synthesizes deterministic clicks and whooshes and suppresses dense repetition. | Additional reusable sounds, semantic event selection, editable cues and style-specific coverage. | App implements sounds/timing; skill directs; MCP controls. |
| Audio finishing | Track gain, speech ducking and loudness processing exist. Imported audio and timeline edits exist. | Per-cue treatment, a clear review surface and measured final-output reporting; stems if needed. | App and MCP. |
| Typography | [`captions.rs`](../src-tauri/src/captions.rs) has timed cues and word highlighting, but embeds one bold font. [`BrandKit`](../src-tauri/src/graphics/brands.rs) supplies graphics colors, one font and a logo. | Shared typography roles, configurable caption fonts/weights, emphasis runs and style overrides. | App/profile data; skill chooses emphasis. |
| Presenter analysis | [`face/mod.rs`](../src-tauri/src/face/mod.rs) caches face boxes and eye-line height, sampled every five seconds for framing. | Eye-contact assessment is a separate capability; subject masks are also absent from that analysis. | Optional app analysis with explicit confidence and review. |
| Quality checks | [`check.rs`](../src-tauri/src/check.rs), graphics checks, storyboards and frame rendering already support inspection. | Brief/style checks and targeted audiovisual review for semantic coverage, joins, movement and mix quality. | App measures; skill/human judges. |

These are source findings, not certification of runtime performance or released-build behavior.

## A concrete motion-quality gap

The render constants deserve attention before attributing every visual difference to the model:

- `RENDER_FPS = 30`.
- `RENDER_LONG_SIDE = 1920`.
- `Spec::new` uses those values, and the export path calls `specs_for` without passing an export frame-rate override.

Consequently, selecting a higher export rate does not create additional motion samples in these graphic assets. Larger exports also scale graphics from the fixed render size. This is independent of the original footage's frame rate or the animation code's ability to describe smooth movement.

Add separate preview and final policies. Preview can use a bounded budget; final graphics can render at the chosen export cadence and sufficient slot dimensions when requested. Keep frame rate and dimensions in cache identity, as the current design already does. Check that export lookup uses the same policy as export preparation and never silently consumes a draft asset.

Validate with fast text, thin lines, a moving UI card, a small graphic slot, and a high-resolution export. Compare motion cadence, sharpness and render cost. Higher settings should be a deliberate production choice; no performance or perceptual improvement has been measured here.

## Procedural audio should join the earlier library proposal

The earlier assessment proposed a curated sound library. These references make a hybrid approach more attractive: extend the procedural palette for common synthetic sounds and retain imported assets for sounds whose character depends on recordings or detailed sound design.

Useful candidate presets are a soft UI tap, key tick, pop, confirmation chime, airy transition, low impact and short riser. Implement and audition them once. Each preset should have a deterministic seed, bounded parameters, known duration and an alignment marker. Cache by preset version and parameters. The agent supplies a sound choice and intensity, rather than writing a synthesis script for each edit.

Connect the sounds to the event contract proposed in part 1. A template can expose a button activation, card arrival, counter completion, text reveal or transition. The app resolves event and cue clocks, applies gain/fades, and preserves user changes. Native cues remain independently editable because a [graphic currently exposes a picture stream only](../src-tauri/src/graphics/mod.rs).

Sound coverage must belong to the selected style. The previous creator's guide favors extensive motion coverage; the visible Opus brief is more selective and excludes sounds on certain camera/layout changes. This difference supports selectable policies such as sparse UI punctuation, energetic explanatory motion, and clean dialogue. Neither coverage policy should become a universal engine rule.

Treat procedural music as an optional asset workflow with listening evaluation. A code-created bed can be useful, but these demonstrations do not establish that it consistently replaces a well-chosen music track or costs less after iteration.

## New optional capabilities: eye contact, cutouts and 3D

The visible Opus brief requests using graphics when the presenter looks away and describes a head/shoulder cutout extending above a camera card. These are distinct capabilities, not ordinary face centering.

For an eye-contact workflow, cache confidence-rated source-time observations and inspect uncertain stretches selectively. Face visibility and head position do not by themselves establish gaze. Cover an unsuitable visual take with a relevant graphic while preserving speech; do not delete meaningful content solely because of gaze.

For a cutout treatment, compute and cache a source-aligned subject matte, then composite it natively over the graphic and camera card. Check edge quality and positional alignment during movement in both preview and export. Background removal is already recorded as future work in the [backlog](future-work.md). Implement it when a chosen style requires it.

The Three.js example broadens the possible visual vocabulary. It does not establish that AeroEdits needs a native 3D editor. Retain custom source-authored graphics as an extension path, verify a known-good Three.js composition inside the app's pinned runtime, and promote useful results into reusable assets. The app currently pins [HyperFrames 0.8.162](../src-tauri/src/graphics/runtime.rs); broader external skill descriptions do not certify compatibility with that exact runtime.

## What should change in the skill and MCP

Add a focused standalone talking-head workflow. The current [shorts](../src-tauri/src/mcp/skill/templates/shorts.md) and [take-shorts](../src-tauri/src/mcp/skill/templates/take-shorts.md) routes remain centered on material associated with a longer recording.

The new skill should load one style profile, inspect the cleaned script, create the beat plan, choose relevant recipes, execute app operations, and inspect the result. Its references should contain annotated successful examples and specific failure examples, with operational details loaded only when needed.

Keep one source for style data. Extend the existing brand/template infrastructure with typography, layout preferences, motion character and audio policy. Generate a compact style summary for the agent from that source; avoid maintaining a second brand file whose values can drift from the app's settings.

Use the existing `layouts`, graphics, proposal, undo and inspection tools. Add or extend typed interfaces for profile selection, beat plans, render policy, audio events/cues and targeted previews. Return changed IDs, timing, validation findings and the new revision. The MCP implementation should call the same tested operations as the UI.

Caption ownership should be explicit per beat: native captions, integrated graphic text, or intentionally hidden. Avoid accidental duplicated text and competing animation. Emphasis text should refer to the transcript rather than requiring the agent to rewrite every word and timestamp.

## Quality and cost: what to measure next

The examples do not provide comparable financial costs. A short user prompt also says little about token usage, tool calls, internal retries, transcription, rendering or preparation. The five-minute demonstration begins after manual trimming; the Astra video reports much longer runs under different conditions. No speed ratio or model ranking should be inferred from those numbers.

Instrument time and cost by stage: analysis, cleanup, planning, asset authoring, graphics render, native composition, and review. Record model usage when the client provides it, actual paid asset calls, cache hits, retries and human corrections. Prefer native reuse for routine operations, reserving model work for creative and ambiguous decisions.

Add three kinds of review to the existing technical checks:

1. **Brief checks:** correct format and chosen quality, expected assets, caption treatment, sound policy and deliverables.
2. **Structural checks:** source/beat coverage, bounded events and cues, unintended empty visual intervals, and retained user overrides. Explicit holds and presenter-rest beats must remain valid.
3. **Perceptual review:** inspect short moving sequences with sound around joins, reveals, transitions and important speech. A still image cannot verify motion or mix quality.

For the next small experiment, keep one fixed recording, style brief and model configuration. Compare generic versus focused-skill instructions in the standalone and app workflows. Measure brief compliance, taste, finishing time and actual usage separately. Repeat promising results on different material, including Greek speech, before generalizing. If comparing models later, hold the task, tools, assets, quality and revision allowance constant.

## Updated priorities

1. **Define one style and its acceptance criteria.** Separate requirements from preferences; use a dedicated short-form skill and a small reference set.
2. **Connect a durable beat plan to existing layouts and graphics.** Keep native decisions editable and changes scoped to the affected beats.
3. **Add an explicit graphics quality policy.** Support intentional preview/final settings and verify actual motion cadence and sharpness.
4. **Build event-driven sound with procedural presets and imported assets.** Extend the existing synthesis and mixer; preserve individual cue edits.
5. **Improve caption typography and targeted review.** Evaluate the finished treatment and correction cost, not just successful export.
6. **Add matting, gaze-aware presentation and more elaborate 3D only for selected styles.** They expand the vocabulary but are not prerequisites for a strong first short-form workflow.

The most useful product target remains a repeatable, editable short with known quality and production cost. These references provide additional implementation details for reaching that target; they do not establish that switching models or adding more generic tools will achieve it by itself.
