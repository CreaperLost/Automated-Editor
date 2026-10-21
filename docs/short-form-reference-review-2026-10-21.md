# Short-form reference review — 21 October 2026

The strongest opportunity is a reusable short-form directing workflow connected to AeroEdits' deterministic scene, caption, and audio systems. The existing rendering and editing foundations support much of this. The main gaps are a durable editorial plan, a richer short-form template collection, coordinated sound cues, and consistent access to those capabilities through MCP.

This is a research and source-code assessment. No application or skill implementation was changed, and no output-quality or cost benchmark was run.

Follow-up: [three additional video references and updated findings](short-form-reference-review-part-2-2026-10-21.md). The inventory below was refreshed for the unified `layouts` implementation at `fbc06da`; older scene/focus action names have been superseded. Part 2 also records the current graphics render-quality limits and procedural-audio opportunities.

## Evidence and limits

- Read the complete English transcripts of all three requested YouTube videos, their descriptions and chapters, and sampled their finished reels in the browser.
- With permission, inspected the first video's linked Google Drive materials: `Prompts List.docx`, `UE-Sound-Design-Claude-Training-COMPLETE.md`, and the inventory of its inspiration folder.
- Audited the current working tree, the shipped AeroEdits skill and shorts templates, and relevant installed HyperFrames skill entry points. The working tree includes uncommitted development; source availability does not certify a released build or runtime performance.
- Did not access the creator's paid visual guide or proprietary clipping/planning skills. Their effectiveness cannot be independently assessed from the tutorials.
- This was a workflow review with sampled visual inspection, not a full audiovisual quality evaluation. Sound-quality comparisons require listening to actual rendered results.

## What each reference contributes

| Reference | Distinct contribution | Implication for AeroEdits |
| --- | --- | --- |
| [Opus 5.5 reel tutorial](https://www.youtube.com/watch?v=5UnK3c-nlIs) | Code-authored animation, a script-linked planning page, reusable visual guidance, and a separate sound-design pass. The creator explicitly supplies scene direction. | Closest architectural match: capture that direction in structured plans and reusable templates. |
| [Ultimate AI reels guide](https://www.youtube.com/watch?v=pwGeWRlrU10) | Separates clip choice, planning, still design, animation, and assembly. Alternates explanatory cutaways with presenter footage. | Use inexpensive composition previews before expensive motion work, and make a deliberate choice about which lines need a visual. |
| [AI talking-head tutorial](https://www.youtube.com/watch?v=WcMOCtas1pg) | Builds a visual vocabulary from examples and corrections. Uses recurring branding and visual metaphors to explain abstract speech. | Store approved examples with reasons and retrieve the relevant ones when planning each beat. |

The two older workflows use external captioning, image/video generation in Higgsfield, and manual assembly in Premiere. Those steps are explicit in the demonstrations: [second video, assembly](https://www.youtube.com/watch?v=pwGeWRlrU10&t=634s), [third video, assembly](https://www.youtube.com/watch?v=WcMOCtas1pg&t=829s). The newest workflow uses Remotion and retains substantial human planning: [planning](https://www.youtube.com/watch?v=5UnK3c-nlIs&t=351s).

The first creator reports approximately 110 minutes elapsed and 20 minutes of his own work, with an estimated $1–$5 per edit allocated from subscription usage. The second reports approximately $25 in image/video credits and additional manual finishing. These are creator reports, use different accounting and content, and exclude a reproducible comparison of preparation, subscriptions, retries, and assets. They are not AeroEdits cost forecasts. Sources: [first cost discussion](https://www.youtube.com/watch?v=5UnK3c-nlIs&t=872s), [second cost discussion](https://www.youtube.com/watch?v=pwGeWRlrU10&t=667s).

Their use of “training” describes studying examples and saving instructions as skills. The demonstrations do not establish model fine-tuning or measured improvements in audience retention. Treat “viral” as their marketing description, not a validated property of an editing treatment.

## What the current code already provides

| Capability | Source evidence | Assessment |
| --- | --- | --- |
| Transcription, alignment, silence/filler/retake cleanup | [tutorial workflow](../src-tauri/src/mcp/skill/templates/tutorial.md), [transcript implementation](../src-tauri/src/transcript/) | Existing pipeline. Reuse it instead of asking the agent to invent cutting scripts. Provider and language limitations still apply. |
| Short selection and word-range creation | [AI short suggestions](../src-tauri/src/ai/shorts.rs), [short creation](../src-tauri/src/mcp/actions.rs) | Already present. The built-in suggestion prompt is brief; editorial judgment can improve through examples and a focused skill. |
| Camera/screen/split layouts, face framing, graphic slots | [shorts](../src-tauri/src/shorts.rs), [layout sections](../src-tauri/src/layouts/mod.rs), [layout MCP action](../src-tauri/src/mcp/layouts.rs) | Useful for presenter-only, full-screen, and split graphic treatments. The unified action supports per-short sections, favourites, transitions and punch-ins. Automatic selection remains strongly oriented toward recorded screen activity. |
| Take-based shorts and footage from an existing recording | [take workflow](../src-tauri/src/mcp/skill/templates/take-shorts.md), [take implementation](../src-tauri/src/take_short.rs) | Existing useful foundation. The skill assumes a long-video/take relationship and lacks a dedicated standalone talking-head workflow. |
| Editable HyperFrames source, cached renders, graphics library, brand kits | [graphics](../src-tauri/src/graphics/mod.rs), [rendering](../src-tauri/src/graphics/render.rs), [library](../src-tauri/src/graphics/library.rs), [brands](../src-tauri/src/graphics/brands.rs) | Strong reuse foundation. The bundled designs are six tutorial templates plus a blank composition; short-form scene coverage is limited. |
| Caption timing, word highlighting, wrapping and positioning | [captions](../src-tauri/src/captions.rs) | Present. The renderer embeds Liberation Sans Bold and does not expose a font family, paired fonts, or per-word style/animation runs. |
| Synthesized clicks/whooshes and speech ducking | [sound effects](../src-tauri/src/sound_effects.rs), [audio settings](../src-tauri/src/project/audio.rs), [mixing](../src-tauri/src/media/polish.rs) | Present. Automatic sounds follow telemetry/layout/zooms, use two synthesized sound types, and deliberately avoid graphics. They are off by default. |
| Imported audio/video and batched timeline edits | [MCP actions](../src-tauri/src/mcp/actions.rs), [proposal tools](../src-tauri/src/mcp/propose.rs), [sequence edits](../src-tauri/src/sequence/edit.rs) | Present. Arbitrary imported SFX placement is possible on general project sequences. A complete short-scoped cue API, per-cue envelopes, and semantic audio library are missing. |
| Storyboards, frame inspection, technical checks, revision checks and undo | [storyboard](../src-tauri/src/mcp/storyboard.rs), [checks](../src-tauri/src/check.rs), [proposals](../src-tauri/src/mcp/propose.rs) | Existing low-cost inspection and mutation foundations. They do not establish animation taste, sound quality, or retention. |

Two details should guide the design:

1. **Graphics are picture assets.** `graphics::asset_for` creates a picture stream only. HyperFrames' broader audio capabilities do not automatically become editable audio in AeroEdits. Keep SFX as native timeline cues rather than assuming a graphic's embedded audio will play.
2. **Word placement is not a persistent semantic anchor.** `graphic_add(atWord)` resolves a word to edited time at creation. The generic `Clip` stores time, duration, asset and link fields, not a retained word/event dependency. Existing source-time mapping and ripple behavior are valuable, but should not be described as a complete semantic dependency system.

## Where each responsibility belongs

| Responsibility | Application | MCP interface | Skill and style guidance |
| --- | --- | --- | --- |
| Decide what makes the short worth watching | Store plans and source references | Return compact candidate/context data | Choose hook, supporting material, payoff, and exclusions |
| Decide whether a line needs a visual | Preview and persist beat choices | Read/update beat plans | Explain, demonstrate, emphasize, or leave breathing room |
| Produce polished motion | Execute reusable, parameterized compositions | Discover templates, fill variables, render and inspect | Choose composition, hierarchy, motion purpose and restraint |
| Place and mix sound | Resolve cue timing, audio assets, gain, fades and ducking | List events/assets; propose, update and preview cues in batches | Choose which events deserve sound and the intended character |
| Maintain a recognizable style | Store versioned profiles and user overrides | Retrieve only relevant profile/examples | Apply examples, exceptions, and editorial reasons |
| Verify the result | Technical validation and media previews | Return findings and selected review windows | Judge relevance, readability, rhythm and coherence |

The MCP layer should expose application operations with compact, typed inputs and useful results. Avoid duplicating editing logic in the server or relying on long prose instructions for capabilities the app cannot execute.

## Recommended design

### A. Persist a small editorial plan

Add a versioned beat plan associated with a short or vertical project. Each beat records:

- Source word ranges and a narrative role: hook, explanation, proof, contrast, payoff, or rest.
- Intended visual purpose and treatment: presenter, presenter plus overlay, split, full-screen graphic, or footage.
- Chosen template and variables, emphasis words, asset references, and a short reason.
- Motion/sound intensity and any user-locked choices.

The agent makes semantic choices. The app resolves timing, validates ranges, builds the sequence, and maintains dependencies through edits. If source material disappears, surface an orphaned beat or cue. A template must specify whether duration changes stretch motion, add holding time, or need review; blindly scaling every entrance with the total duration will damage pacing.

Start with a transcript-row view: spoken text, treatment, preview, and a concise note. This captures the useful planning interaction without requiring a separate agent-built web application for each edit.

For podcast assembly, preserve support for ordered source ranges. `short_create` currently chooses a contiguous range; general sequence edits provide building blocks for more complex assembly. Evaluate first-class multi-range shorts only when the workflow needs them, preserving context and the speaker's meaning.

### B. Ship a focused, reusable short-form style

Build one complete style before a large catalog. A useful first collection would include:

- Presenter hook with restrained captions and selected emphasis.
- Recurring numbered progress indicator.
- Before/after or contrast comparison.
- Slider, meter, or percentage change.
- Object plus explanatory typography.
- Simple diagram, relationship/orbit, or chart.
- Split graphic with presenter.
- Concluding visual that returns to the hook's motif.

Store spacing, typography, easing, entrance/hold/exit timing, caption placement, and audio policy alongside the style. Keep colors and logos in the existing brand system. Extend typography roles where needed; the current single graphics font and independent native caption font are not a complete shared type system.

Use existing HyperFrames motion/creative knowledge. Its installed skills already describe choreography, text effects, scene transitions, reference-led direction, and seek-safe timelines. The integration work is selecting a small set of good recipes, validating them in AeroEdits, and making the successful results reusable.

Add font selection/weight, semantic emphasis runs, and a few deterministic caption animation presets to the caption subsystem. The app should create timing from the transcript; the agent should identify emphasis and style. Validate Greek and English wrapping, accents, punctuation, font coverage and mobile readability.

Some reference looks also require scene-level dimming, blur, or presenter punch-ins behind an overlay. These require native compositing controls over the actual footage. A graphic cannot blur the underlying presenter merely by blurring its own HTML background. Start with the necessary controls and verify preview/export agreement.

### C. Connect animation events to editable sound cues

The creator's linked [sound guide](https://drive.google.com/drive/folders/1WJMeuTvYZJgufE9_Xpkp6xThzQWXhR-G) distinguishes motion, discrete events and anticipation, with different synchronization points. Its transferable idea is precise visual/audio correspondence. Its fixed quotas, gain values, music restrictions and universal sound coverage are personal style choices, unsuitable as universal engine rules.

Have reusable graphics declare stable semantic event markers. Suggested fields are event ID, element, event kind, local time/range, primary peak time, importance, and suggested sound family. Built-in templates can supply these directly; freeform authors can provide explicit markers under the same contract. The app need not infer every arbitrary animation's velocity from rendered frames.

Maintain a small local audio catalog with tags, duration, transient/peak offset, loudness, true peak, variants and provenance. Audition and describe the catalog during curation; do not repeatedly ask an agent to rediscover every file on every project.

An initial cue compiler can deterministically:

1. Map each selected event through the graphic and short's clocks.
2. Match compatible catalog sounds by family, length and weight.
3. Align an audio transient to an event, a whoosh peak to a motion marker, or a riser's resolution to a transition.
4. Apply density limits, dialogue ducking, gain and fades.
5. Preserve stable cue IDs and user overrides on replanning.

For example, `cueStart = visualPeak - soundPeakOffset`. Handle negative starts, clip tails, canvas/frame-rate changes, duration edits, stale renders and removed anchors explicitly. Sound starts can use the audio sample clock; visual markers resolve to the project frame clock.

Use the native mixer and timeline. The agent should be able to select a quieter variant or remove a cue without regenerating the graphic. Keep voice, SFX and music independently editable. Absolute dB gain values alone do not guarantee a balanced mix because source loudness varies.

### D. Add a narrow MCP interface around the new data

The following are proposed interface shapes, not existing tool names:

| Operation | Compact request/result |
| --- | --- |
| `short_plan_get/set` | Beat plan, source references, expected revision, validation and diff |
| `style_get` | One relevant style, available template parameters and a few examples |
| `short_settings` | Explicitly scoped caption/layout/audio style updates |
| `events_get` | Stable animation/layout events for a selected short and time window |
| `audio_library_search` | Shortlisted compatible sounds with metadata and preview handles |
| `sound_plan` | Deterministic proposal from selected events, style and catalog |
| `sound_cues_update` | Batched edits by cue ID; preserve manual overrides |
| `preview_range` | Short audiovisual preview plus event/cue timing and mix measurements |

Extend existing actions when their scope fits. Reuse `propose_edit`, revision validation, proposals, undo, `graphics`, `graphic_check`, `storyboard`, and `check`. Do not introduce another generic timeline-edit API simply to batch calls: batching already exists.

Current-code clarification from the follow-up: `layouts` already exposes layout changes scoped to a short. Use it directly; the proposed settings interface should fill remaining caption/audio/style gaps rather than duplicate layout control.

The shipped tool descriptions should describe their individual purpose briefly. Shared workflow belongs in server instructions and the skill; template details and exemplars should be loaded on demand. Measure tool-schema and response sizes across clients before choosing further consolidation.

### E. Make the skill an editor's decision guide

Add a standalone talking-head workflow and a podcast-short workflow, with explicit routes from the existing shorts/take workflows. A standalone short should not need a longer recording merely to fit the template.

The short-form guidance should teach the agent to:

- Preserve meaning and choose a coherent hook/payoff; avoid generic “optimize for virality” instructions.
- Give a visual an explanatory job. Use presenter-only moments deliberately.
- Choose a relevant metaphor, then a matching template or asset.
- Keep a recurring visual motif and a clear reading hierarchy.
- Choose emphasis words semantically rather than styling every spoken word alike.
- Plan sound after motion timing is stable, with quieter treatments available.
- Inspect actual previews and preserve approved choices during revisions.

Use approved examples with the script, treatment, reason, preview and correction. Add a few failed examples that explain an error. Retrieve only the closest examples instead of sending a hundred-image board into every editing session. Corrections should become explicit proposed preference updates rather than silently changing every future project.

## Cost strategy

Use deterministic execution for timing, cue alignment, mixing, rendering, validation and sequence construction. Reserve strong-model attention for meaning, visual concepts and difficult review decisions.

- Reuse transcript/alignment outputs and local asset metadata.
- Make one compact creative plan, then perform batched changes. Replan affected beats only.
- Prefer validated templates and local assets. Generate a still when a scene needs an object that the library lacks, then animate it deterministically when that achieves the intended look.
- Use generated video for selected shots whose value depends on deformation, performance, complex physical interaction or camera behavior that the available assets/templates cannot deliver adequately.
- Preview layout/key poses before full-resolution renders. Invalidate only dependent renders and cues.
- Use the existing storyboard for broad inspection, then inspect motion/audio windows around important or suspicious events. Do not read every frame through a model.
- Measure wall time, model usage, generated-asset charges, render time, retries and human corrections separately. Local processing still consumes time and hardware resources.

This should reduce repeated work, but the size of the savings and visual equivalence to generated footage remain unmeasured. The flat native-audio path and reusable motion system are useful engineering hypotheses to test, not an established quality/cost result.

## Quality checks and a small evaluation

Keep the current technical checks. Add caption/style coverage, event/cue bounds, orphaned dependencies, sound density, clipping/true-peak measurements, and preview/export synchronization checks as the new features arrive.

The existing storyboard merges stretches shorter than two seconds and samples still frames. It can miss a poorly eased entrance, a one-second effect, a late sound, or a tiring mix. Add event-aware filmstrips and short rendered audiovisual windows; retain whole-short human playback. A client that cannot assess audio should provide a listening preview rather than claim it reviewed the mix.

Use three fixed 30–60 second clips: a direct instructional talking head, an abstract podcast excerpt, and a Greek-language example. Compare:

1. Current workflow.
2. Improved short-form skill using existing capabilities.
3. That skill plus the proposed template/cue integration.

Hold footage, brief, assets, output settings and revision allowance constant. Rate meaning preservation, visual relevance, readability, motion, audio balance, cohesion, editability and human correction time. Record actual costs and failures. Evaluate generated-video hero shots separately if the first comparison exposes a specific visual gap. Audience retention needs publishing evidence beyond this production benchmark.

## Recommended order

1. **Establish the baseline and draft one focused style.** Use existing tools to discover which decisions improve output and which operations remain awkward.
2. **Build one complete vertical slice.** Persist beat plans; add the required caption/layout controls; expose them through MCP; ship a small validated template pack.
3. **Implement event-driven sound.** A curated local catalog, explicit template markers, editable cue planning and targeted listening previews. Reuse the existing mixer and proposal infrastructure.
4. **Broaden after measurement.** Add additional styles, better asset search and selected generation integrations where the benchmark shows a material benefit. Defer subject matting/behind-presenter text and a broad effects marketplace until a desired style requires them.

The first product target should be one consistently good, editable short-form style with known production cost. That creates a stable base for learning the user's taste and expanding the vocabulary.
