# Herk Edit Kit and the two Nate Herk video guides

Research date: 21 October 2026.

The useful opportunity for AeroEdits is to adapt the kit's editorial workflow, selected designs, and validation ideas to our existing editor. We already have the central rendering and editing foundations. The largest remaining benefits are explicit creative planning, reusable directing preferences, better graphic selection, event-linked sound, and inspection of the encoded result.

## Evidence and scope

- Obtained and reviewed both English auto-generated YouTube transcripts and the second video's description and chapters. Transcripts can misrecognize names; they establish what the creator says, not independent output-quality measurements.
- Downloaded and inspected the public repository at commit `10511e5af86fc47756ea89803b397422e2d89284`. Read its playbook, skill instructions, catalog, a representative card, plan/footage validators, selected audio tools, verification notes, package metadata, and licensing notices.
- Ran `node --test tests/*.test.mjs` in that checkout: **11 passed, 0 failed**. These are behavior/fixture tests; no dependencies were installed, no paid API was called, and no media render or audiovisual benchmark was performed.
- Compared against AeroEdits at local HEAD `92f80d3` and its current working tree, including prior motion/short-form research. Source findings do not certify shipped behavior. Existing unrelated working-tree changes were preserved.
- Did not continuously watch/listen to the complete videos or independently assess their showcased outputs. Video-specific findings below are transcript/description evidence. The public kit is a later consolidated version, not necessarily the exact snapshot used in either demonstration.

## What the videos actually show and claim

### GPT-6 Astra guide

[Video](https://www.youtube.com/watch?v=o3IEkKXXXvo).

- **1:27–3:04:** Describes cleaning separate face-camera and screen recordings, reducing a 14-minute tutorial to nine minutes, inserting concept cards, and switching to the presenter when the screen is inactive. This closely overlaps our existing tutorial workflow and activity-based layouts.
- **3:06–9:15:** Presents reels and an ad that combine presenter footage, full-screen animations, supporting footage, captions, and sound. The creator describes gathering relevant material from his own videos and event assets. Relevance to the spoken idea is more valuable to us than adding generic decorative B-roll.
- **11:26–13:39:** Explains the sequence: transcribe, cut, plan beats, build with skills, verify and iterate. Word timestamps provide anchors for picture and text.
- **18–23 minutes:** Gives detailed direction for the intro, including which side elements enter from, caption treatment, camera placement, background, and playing video panels in perspective. The result has substantial human art direction.
- **23:43–29:14:** Reviews and revises a first draft. He reports an approximately 18-minute first pass and a ten-minute second pass, then requests another opening variation. Problems include overlapping 3D panels, small text, and an unsatisfying opening. These are creator-reported timings for one example, not our performance forecast.

Lesson: preserve precise creative decisions and make local revisions cheap. The demonstration does not establish that an unspecified prompt reliably produces a finished edit.

### Opus guide

[Video](https://www.youtube.com/watch?v=7jHXoPGnA4c).

- **0:48–4:23:** Explains a detailed intro prompt with word-triggered logos, glass backing for readability, highlighted phrases, and supporting assets. He praises additional creative choices made by the agent.
- **4:20–6:58:** Turns analysis of a reference reel into a reusable `motion-showreel` skill, then applies it to YouTube branding. Some inserts use image generation followed by Kling video generation; those shots are separate from HTML-authored animation.
- **6:58–10:40:** Describes a branded showreel and a story assembled from an event-resource folder reportedly containing 105 GB. The event example used an initial prompt, iteration, and a fix; he also acknowledges awkward speech cuts.
- **10:40–14:53:** Applies three treatments to the same avocado-toast recording: whiteboard, restrained course graphics, and fast short-form. This supports offering distinct styles for the same source instead of one universal editing recipe.
- **14:53–16:12:** Shows a speculative brand ad. His explanation of precisely how the agent sourced some assets is partly conjectural; it should not be treated as an audited asset pipeline.
- **16:12–19:01:** Repeats the five-step workflow and explains saving feedback into skills. A “one-prompt” example can still contain multiple internal render/review cycles, existing skills, and paid generated assets.

Lesson: build a repeatable directing system with examples and feedback. The claim that one model has better taste is the creator's judgment, not a controlled model comparison.

## What the GitHub project provides

[Repository](https://github.com/nateherkai/herk-edit-kit), formerly the HyperFrames Student Kit.

It is a local workspace template for a coding agent, with Markdown workflows, Node helper scripts, media examples, and HTML motion assets. HyperFrames renders the compositions, FFmpeg handles media operations, and GSAP supplies much of the animation. It is not a separate desktop timeline editor or a fine-tuned editing model.

The checked-out inventory includes:

- **15 skill directories**, covering cleanup, whole-video editing, short-form editing, storytelling, showreels, style selection, and framework use. The verification document still mentions 14 in one section; the current directory inventory and README say 15.
- **406 draft cards:** 106 `vox-explainer` and 300 `kallaway`, indexed by purpose, tier, editable slots, duration hints, and previews in `style-library/registry.json`.
- **Two scene starting points:** dark graph paper and a left glass popout.
- **Transcript/EDL helpers:** transcription, silence cuts, mistake candidates, approved cuts, retiming, and HTML EDL review.
- **Short-form validators:** caption correspondence, source-time mapping, scene coverage, frame alignment, anchors, footage ledger completeness, hashes, and duplicate/overlapping footage selections.
- **Showreel helpers:** reference analysis, beat-grid measurement, music splicing, sound generation, and mixing. The music-grid script estimates period near a supplied BPM and phase from low-frequency energy; it is not a demonstrated universal tempo/beat tracker.

The framework and starter can run locally; agent usage and optional cloud transcription, music, image/video, and sound generation can still incur costs. ElevenLabs transcription has a supplied helper. Whisper is an alternative workflow rather than an equivalently packaged provider in that helper. Source: [tools guide](https://github.com/nateherkai/herk-edit-kit/blob/main/docs/TOOLS-AND-API-KEYS.md).

The kit's own [verification notes](https://github.com/nateherkai/herk-edit-kit/blob/main/docs/VERIFICATION.md) explicitly say the 406 cards were not all rendered and visually approved. Showcase MP4s also do not supply all private source projects, transcripts, or production decisions; see [export scope](https://github.com/nateherkai/herk-edit-kit/blob/main/docs/EXPORT-SCOPE.md).

## Comparison with our current application

| Area | AeroEdits already has | Useful addition |
| --- | --- | --- |
| Cleanup | Word editing, silence/gap passes, fillers/retakes, alignment, proposals, undo, shared timeline mapping | Phrase-aware review guidance and encoded join checks; retain our native editing pipeline |
| Picture/layout | Screen, camera, split layouts, punch-ins, face framing, activity telemetry, imported footage, take-based shorts | Choose treatments from meaning and evidence as well as activity |
| Graphics | HyperFrames source, six tutorial templates plus blank, transparent overlays, short slots, brand kits, reusable library | Curated visual recipes and purpose-based retrieval |
| Planning | Transcript ranges, short reasons, actual timeline, storyboard inspection | Persist the intended hook, proof, payoff, shot purpose, word anchors, and cue dependencies |
| Captions | Timed groups, word highlighting, outlines/background, wrapping, position | Configurable typefaces, emphasis styling, placement per layout, rendered contrast/face checks |
| Sound | Imported audio, gain/ducking, procedural clicks/whooshes linked to interaction and layout | Editable semantic cues for graphic actions, a restrained sound palette, final-file measurement |
| QA | Edit-structure checks, frame rendering, storyboards, graphics gate and contact sheets | Contiguous encoded frames around joins, speech/audio review, evidence tied to the actual output hash |

Local evidence: [MCP actions](../src-tauri/src/mcp/actions.rs), [shorts workflow](../src-tauri/src/mcp/skill/templates/shorts.md), [take shorts](../src-tauri/src/take_short.rs), [graphics library](../src-tauri/src/graphics/library.rs), [graphics gate](../src-tauri/src/graphics/gate.rs), [captions](../src-tauri/src/captions.rs), [edit checks](../src-tauri/src/check.rs), and [sound effects](../src-tauri/src/sound_effects.rs).

The current source supports transparent overlays and `full`, `camera`, `screen`, and `overScreen` short slots. It would be inaccurate to describe overlays, brand kits, or graphics validation as wholly missing.

## Recommended integration

### 1. Adapt the directing workflow into our existing skill system

Use the kit's [short-form workflow](https://github.com/nateherkai/herk-edit-kit/blob/main/.claude/skills/short-form-edit/SKILL.md) as a reference for a focused AeroEdits short-form recipe:

1. Read the transcript and available assets.
2. Choose a truthful hook and a complete payoff.
3. Plan each shot's explanatory purpose and exact spoken anchor.
4. Build a low-cost moving draft with speech and rough sound.
5. Refine the weak sequences, then polish and inspect the encoded output.
6. Offer corrections as versioned style preferences with examples and a reason.

Keep app operations in our native actions. Do not add a second FFmpeg editing pipeline that flattens screen/camera relationships, telemetry, undo, or source mapping. Load only the needed references instead of injecting all 15 skills or the entire catalog into every agent context.

### 2. Persist a small editorial plan and validate it

Borrow the concepts in [plan-schema.md](https://github.com/nateherkai/herk-edit-kit/blob/main/.claude/skills/short-form-edit/references/plan-schema.md), adapting them to our asset IDs, transcript word IDs, source clocks, microseconds, and edit revision.

A beat should store its purpose, source/word anchors, chosen layout, graphic/library ID, asset references, visual events, expected payoff, and user overrides. Store creative intent alongside actual timeline objects; do not create a second timing engine. Update dependent cues when anchors move and flag a beat if its source words disappear.

Port validation concepts into app checks rather than calling the scripts against incompatible schemas. The supplied plan validator assumes complete caption coverage and specific filenames/layout names. Its event-gap threshold, allowed anticipation, and “no repeated B-roll” policy are style decisions that should be configurable.

### 3. Curate and adapt a small graphics pack

Start with roughly 8–12 useful treatments: a hook, concept definition, comparison, three-step reveal, quote/source card, evidence crop, warning, diagram, and payoff. Prefer treatments suited to Greek AI tutorials. This is a proposed starting size, not a measured optimum.

The sampled `t1-section-circle.html` demonstrates why catalog files cannot be copied directly:

- Fixed 1920×1080 positioning needs responsive layout and explicit portrait treatment.
- `data-slot` fields need conversion to our declared composition variables.
- Relative `tokens.css`, CDN GSAP, and fonts need self-contained dependencies. Our current authoring guidance prefers paused finite CSS/Web Animations; GSAP cards need adaptation or an explicitly supported local GSAP path.
- Colors need our brand-variable contract; placeholders and AIS references need replacement.
- Timing must follow clip duration and our cut/ease entrance behavior.
- Each selection must pass `graphic_check`, followed by visual inspection with real English/Greek copy in the actual slot dimensions.

Use the existing library/versioning system. Add purpose/style metadata and previews where helpful. Keep draft cards separate from approved assets.

### 4. Extend captions and sound as native editable systems

The native caption renderer currently embeds Liberation Sans Bold. A graphics brand font does not change it. Add caption font selection and shared typography roles, then layout-aware placement and optional emphasis runs. Test Greek diacritics, long words, and real phone-size readability.

Add cue events linked to visual actions or word IDs: reveal, connect, success, warning, impact. Resolve them to native audio clips with gain, trim, fades, and ducking. Keep graphics and sound separately editable; graphics currently become picture assets, so embedded HTML audio is not a native SFX lane.

The kit itself contains differing music/loudness advice between the house playbook and the broader short-form skill. Choose one explicit profile, audition it, and measure the encoded output instead of copying all thresholds as universal defaults.

### 5. Extend output review and final rendering quality

Keep our cheap structural `check`, storyboard, and graphics gate. Add a targeted encoded-output pass: opening frames, contiguous strips around each cut, caption readability, audio joins, sync, final syllable, loudness and true peak. Fresh ASR can flag missing words but cannot prove consonants are intact; listening remains necessary. Track what was actually inspected and invalidate the report when the final file changes.

Our graphics source currently fixes renders at **30 fps and a 1920-pixel long side**. A 4K or higher-cadence video export therefore does not add graphic detail or motion samples. Add explicit preview/final graphics settings and include the chosen cadence/dimensions in preparation and lookup, preserving the existing cache identity. Verify small text, thin lines, fast motion, transparency, and preview/export agreement before calling it an improvement.

## Adoption order and evaluation

1. **First useful experiment:** one focused short-form recipe, a small persistent beat plan, and 3–5 adapted designs, using existing footage and native audio capabilities.
2. **Product implementation:** editable event cues, typography, plan validation, and targeted final-output checks.
3. **Quality/performance:** final-resolution graphics policy, better retrieval, and dependency-aware revision.
4. **Optional later:** asset generation/sourcing and music-grid tools for showreels. Tutorial speech should not be forced onto a music grid.

Use the same short English and Greek recordings for a current-workflow baseline and the adapted workflow. Keep source, brief, model settings, and available assets equal. Record time to acceptable draft, human corrections, incorrect cuts, word-to-visual drift, readability, sync, cost, and the time required for one local revision. Include a meaning-preservation review and moving/audio review. Audience retention remains unmeasured.

This continues the existing [short-form recommendations](short-form-reference-review-2026-10-21.md) and [part 2](short-form-reference-review-part-2-2026-10-21.md). The new concrete inputs are inspectable workflows, a searchable card corpus, validator implementations, and explicit caveats about what the public kit does not verify.

## Reuse notices

The root [LICENSE](https://github.com/nateherkai/herk-edit-kit/blob/main/LICENSE) supplies an MIT grant for covered material and excludes AIS brand materials from reuse. Imported pipeline resources retain a separate [use permission](https://github.com/nateherkai/herk-edit-kit/blob/main/licenses/PIPELINE-USE-PERMISSION.txt). [Third-party notices](https://github.com/nateherkai/herk-edit-kit/blob/main/THIRD_PARTY_NOTICES.md) retain separate terms for frameworks, GSAP, fonts, and assets. Preserve applicable notices for adapted material and use our own branding/media. This assessment does not establish redistribution rights for every included asset or dependency.

No application code, installed skills, or runtime dependencies were changed during this research.
