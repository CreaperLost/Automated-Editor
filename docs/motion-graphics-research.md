# Motion graphics research

Research started: 20 October 2026.

This is a living record of findings about motion graphics tools, creative workflows, and
capabilities relevant to Automated Editor. Keep verified capabilities, vendor claims,
inferences, and results from hands-on tests distinct. Add dated findings as research continues.

The scope now also includes adjacent editing competitors. The original filename is retained
so the research stays together.

- [Higgsfield and HyperFrames](#current-conclusion)
- [Gling and AeroEdits](#gling-and-aeroedits)

## Current conclusion

Higgsfield Motion Designer's clearest documented advantage over our HyperFrames setup is
its connection to After Effects and the production workflow around it. The research so far
does not establish that its AI produces better motion design from the same brief.

Our installed HyperFrames skills already cover much of the advertised work: kinetic
typography, logo reveals, product promos, transitions, camera movement, charts, maps,
3D, shaders, reusable components, and editable compositions.

The differences worth investigating are native Adobe project delivery, the convenience
and quality of finishing tools, integrated asset generation, creative direction, and the
effort required to produce and revise a polished result.

## Research scope and evidence

The initial research reviewed Higgsfield's product pages, launch documentation, plugin
overview, and integration guide; HyperFrames' public repository; Adobe documentation;
and the locally installed HyperFrames skills.

This was a documentation and capability assessment. We have not run Higgsfield, inspected
its generated project files, measured its performance, or completed a controlled visual
comparison. Promotional examples do not establish typical output quality or how much
human finishing was involved.

The local skill audit describes the installed instructions available to the agent. It does
not prove that every capability is configured, available on this machine, or equally mature.
It also does not audit Automated Editor's own MCP implementation.

## What Higgsfield Motion Designer does

Higgsfield describes Motion Designer as an agent that builds native After Effects layers,
keyframes, expressions, and timing from natural-language instructions inside an open
project. Its product FAQ says text, colors, layers, timing, and animation remain editable.
The same FAQ also claims Premiere Pro support. Availability through MCP means it is not
limited to a single chat client. These are vendor claims, not independently tested findings.

Sources: [Motion Designer product page][higgsfield-designer],
[launch documentation][higgsfield-launch].

| Dimension | Higgsfield Motion Designer | Our HyperFrames setup |
| --- | --- | --- |
| Agent authors | Native Adobe project elements, according to Higgsfield | HTML, CSS, animation code, and media |
| Animation environment | After Effects; product FAQ also claims Premiere Pro support | GSAP and supported alternative runtimes |
| Editable deliverable | Adobe project elements | Composition source and Studio timeline |
| Human finishing | Adobe editing tools | Studio and code |
| Workflow advantage | Working inside an existing Adobe project | Integration with software, data, and reusable code |

HyperFrames documents deterministic rendering, a reusable catalog, agent skills, and an
evolving browser editor. Its source files remain editable; it is not merely a flattened
video generator. Source: [HyperFrames repository][hyperframes-repo].

## Advantages and differences worth testing

### Native Adobe finishing and handoff

Building inside After Effects removes a handoff for someone who already works there.
An editable Adobe project can also be a client or team requirement in its own right.
The reviewed HyperFrames workflows do not provide an equivalent native Adobe project
handoff. Exporting a video or consuming a Lottie asset is not the same deliverable.

### Established animation controls

After Effects provides text animators, selectors, motion blur, and detailed speed-graph
editing. These are useful controls for producing and refining polished animation. Their
availability does not demonstrate that Higgsfield's agent uses them well on every task.

Sources: [Adobe text animation documentation][adobe-text],
[Adobe speed controls][adobe-speed].

### Motion blur

The installed HyperFrames `motion-blur-streak` recipe explicitly approximates motion blur
using directional blur or ghost trails. That differs from integrating movement across a
shutter interval. Fast type, rotating shapes, and camera moves are useful cases to compare.

This is a limitation of the documented recipe, not proof that browser rendering can never
support temporal sampling. We have not established whether another current HyperFrames
render path implements it.

Local evidence: `hyperframes-animation/rules/motion-blur-streak.md` in the installed skills.

### Connected asset generation

Higgsfield advertises image and video generation directly into Adobe timelines, alongside
background removal, reframing, and upscaling. HyperFrames also has media workflows;
the practical difference may be provider access, configuration, asset transfer, and the
number of steps needed to continue editing.

Source: [Higgsfield Adobe plugin overview][higgsfield-adobe].

### Setup and cost

Higgsfield's launch guide describes active Higgsfield, GPT, and After Effects subscriptions
for its ChatGPT workflow. It says generations consume Higgsfield credits and client usage
depends on the plan. Its integration guide says unlimited access and free generations
apply only on the Higgsfield website, not the integrations. Verify current terms before
making a purchasing decision; no cost comparison has been measured.

The integration guide documents Windows installation via a ZXP installer and Adobe 2024
or newer. Installation and bridge connectivity remain untested here.

Sources: [launch documentation][higgsfield-launch],
[integration guide][higgsfield-integrations].

## Capabilities already present in our skills

| Installed skill | Relevant capability |
| --- | --- |
| `hyperframes-core` | Composition timing, clips, tracks, source editing, deterministic rendering contract |
| `hyperframes-animation` | Atomic motion rules, scene blueprints, transitions, choreography audits |
| `hyperframes-animation` runtime adapters | GSAP, Lottie, Three.js, Anime.js, CSS, WAAPI, TypeGPU/WebGPU |
| `hyperframes-creative` | Typography, palette, composition, brand direction, pacing |
| `hyperframes-registry` | Discovery and reuse of blocks and components |
| `motion-graphics` | Titles, logo reveals, charts, maps, lower thirds, webpage and UI animation |
| `media-use` | Asset sourcing, generation, and media preparation workflows |

Prompt-based creation, brand assets, editable text and timing, complex code-authored
visuals, and reusable animation patterns are therefore areas of overlap. Their presence
in a feature list alone does not distinguish output quality or reliability.

## Creative defaults may affect quality

The installed `hyperframes-creative/references/video-composition.md` includes guidance
to give decorative elements ambient motion, use at least two focal points, and vary
entrance eases and directions. The house-style guidance also encourages layered backgrounds.

These rules can help avoid empty, webpage-like compositions. Applied mechanically,
they may encourage busy or formulaic work. A strong animation can instead use one focal
point, deliberate stillness, consistent easing, and a single well-designed transition.

Working hypothesis: some perceived quality gaps may come from art direction and recipe
selection even when the underlying engine can achieve the desired result. This has not
been demonstrated by a benchmark, and no skills were changed during this research.

Potential improvements to evaluate:

- Let a reference and the intended visual hierarchy override generic density rules.
- Treat stillness and negative space as deliberate options.
- Plan the relationship between movements, not just individual entrance effects.
- Assess a complete moving sequence as well as proof snapshots and technical checks.
- Distinguish passing validation from achieving the desired visual quality.

## Distinguish two similarly named Higgsfield offerings

The linked **AI Motion Designer** page describes an Adobe-connected agent.

A separate **AI Motion Design** page describes a code-to-video workspace with motion
presets, a visual inspector, and adjustable parameters. That description is conceptually
closer to HyperFrames. Its public page does not establish which rendering framework it
uses, and this research does not assume the two offerings share an implementation.

Sources: [AI Motion Designer][higgsfield-designer],
[AI Motion Design][higgsfield-motion-design].

## Proposed controlled comparison

Use identical assets, copy, fonts, colors, duration, aspect ratio, references, and output
settings. Record the model/client and tool versions. Compare a first output and one
equivalent revision, using a consistent time or effort allowance.

| Test | What to inspect |
| --- | --- |
| Kinetic typography | Hierarchy, spacing, timing, easing, legibility, motion blur |
| Logo reveal | Shape choreography, continuity, restraint, final hold |
| Product UI sequence | Camera movement, framing, continuity, text legibility |

Record time to usable result, human corrections, reference fidelity, revision accuracy,
editability, rendering failures, and actual usage costs. Judge output without identifying
the tool where practical. Preserve prompts, source projects, previews, and revisions so
findings can be reviewed and reproduced.

## Open questions

- Which specific qualities in Higgsfield's examples are most valuable to reproduce?
- How consistently does its agent match a reference and preserve a requested design?
- What requires manual finishing, and how much?
- How much of a measured gap comes from the engine, assets, instructions, or art direction?
- Can more selective HyperFrames creative rules improve output without adding technology?
- Does the intended delivery workflow require native Adobe projects?
- Which capabilities of Automated Editor's MCP are exposed and reliable enough to support
  the same tasks? This requires a separate implementation audit.

## Research log

- **20 October 2026:** Initial documentation review and installed-skill comparison.
  Identified Adobe integration, finishing controls, asset workflow, motion-blur treatment,
  and creative defaults as investigation areas. No hands-on benchmark completed.

## Sources

[higgsfield-designer]: https://higgsfield.ai/ai-motion-designer
[higgsfield-launch]: https://higgsfield.ai/blog/ai-motion-designer-after-effects-gpt
[higgsfield-adobe]: https://higgsfield.ai/plugins/after-effects
[higgsfield-integrations]: https://higgsfield.ai/creator-hub/help-center/integrations/external-integrations-higgsfield
[higgsfield-motion-design]: https://higgsfield.ai/ai-motion-design
[hyperframes-repo]: https://github.com/heygen-com/hyperframes
[adobe-text]: https://helpx.adobe.com/after-effects/desktop/animating-text/text-animation/animating-text.html
[adobe-speed]: https://helpx.adobe.com/after-effects/desktop/animate-in-after-effects/speed-between-keyframes/speed.html

Local skill evidence was read from `C:/Users/George/.codex/skills/` on 20 October 2026.
These skills live outside this repository and may change independently. Relevant files:

- `hyperframes/SKILL.md`
- `hyperframes-core/SKILL.md`
- `hyperframes-animation/SKILL.md`
- `hyperframes-animation/rules/motion-blur-streak.md`
- `hyperframes-creative/SKILL.md`
- `hyperframes-creative/references/house-style.md`
- `hyperframes-creative/references/video-composition.md`
- `hyperframes-registry/SKILL.md`
- `motion-graphics/SKILL.md`

## Gling and AeroEdits

Research date: 20 October 2026. Here, "us" means the AeroEdits application in this
repository, its MCP/CLI actions, and its integrated graphics workflow. General capabilities
of an external agent are distinguished from features built into the app.

### Assessment

Gling is a closer competitor to AeroEdits than Higgsfield Motion Designer. Its core is a
speech-based rough-cut workflow: import a recording, transcribe, remove unwanted material,
review the result, and export. It also advertises tools for repurposing and creating content.
Source: [Gling overview](https://www.gling.ai/).

Our working tree already implements many comparable editing operations. Gling's clearest
documented differences are professional-editor handoff, wider language coverage, a web
app, publishing assistance, and link-to-avatar video generation. Our strongest technical
differentiators are recorded interaction telemetry, screen/webcam coordination, and direct
agent access to the edit.

These are capability findings and product hypotheses. Neither cleanup accuracy nor user
experience superiority has been established by a hands-on comparison. Local findings
include uncommitted work present during the review; they do not certify a released build.

### What Gling offers

Gling's cleanup page describes bad-take, silence, and filler detection with transcript-based
editing. Users can review and restore cuts rather than accepting every suggestion. This is
an assisted rough-cut workflow, not evidence that it reliably chooses the best performance.
Source: [cleanup workflow](https://www.gling.ai/save-time).

Its quality tools advertise captions, background-noise removal, and automatic zooming.
Source: [quality features](https://www.gling.ai/quality-content).

Its clipping page advertises long-video-to-short conversion with captions, B-roll, and
zooms. The reviewed page does not explain B-roll sourcing, licensing, matching, or editing
controls, so these remain unverified implementation details.
Source: [AI clipping](https://www.gling.ai/ai-viral-video-clipping).

It also advertises YouTube titles, chapters, and future-video suggestions. Claims about
improving views or algorithm performance are marketing claims, not measured outcomes.
Source: [publishing assistance](https://www.gling.ai/maximize-success).

The link-to-video tool is a separate creation workflow: written content becomes an
AI-presented talking video with a selected avatar/face and voice. It should not be
confused with extracting clips from an existing recording.
Source: [link-to-video](https://www.gling.ai/generate-video-from-any-link).

### Comparison with the current implementation

| Area | Gling documentation | AeroEdits working tree | Assessment |
| --- | --- | --- | --- |
| Silences and fillers | Detection, removal, review | Audio and transcript-gap passes; filler candidates and review | Broad overlap; compare wrong cuts and misses |
| Retakes | Bad-take and repeated-line cleanup | Restart/retake suggestions with rules and AI review | Broad overlap; best-delivery selection is unproven |
| Transcript editing | Text edits affect the cut | Word-based cuts with shared timeline mapping | Overlap; boundary quality needs listening tests |
| Captions | Automatic captions | Word highlighting, positioning, sizing, colors, outlines, backgrounds | Present in both; styling depth unbenchmarked |
| Noise reduction | Advertised enhancement | Spectral denoising, loudness normalization, ducking, mouth-click repair | Present in both; algorithms and quality are not equivalent by assumption |
| Zoom and framing | Automatic emphasis zooms | Click/dwell zooms, caret following, active-window crop, webcam focus and face framing | Our approach has explicit tutorial interaction signals |
| Shorts | Long-video clipping | AI moment suggestions, word-range shorts, scene selection, take-based shorts | Already implemented; do not classify shorts as missing |
| Chapters | Transcript-based chapter generation | AI chapter generation and validation | Already implemented |
| B-roll | Advertised in clipping | Imported media and own-recording footage under take shorts; automatic stock sourcing is backlog | Partial overlap; sourcing workflow is a gap |
| Publishing assistance | Titles and next-video suggestions | Short titles exist; no dedicated full-video title or channel-idea workflow found | Agent can draft text, but app integration is absent |
| Link-to-avatar video | Advertised | No equivalent built-in workflow found | Distinct product expansion, not a cleanup prerequisite |
| Agent control | No public official MCP/API documentation found in this research | MCP and CLI actions, proposals, undo, inspection, storyboard and checks | Concrete capability on our side; Gling absence is unconfirmed |
| Motion graphics | Reviewed pages do not establish a comparable source-authoring system | Editable HyperFrames source, graphics templates and background rendering | Documented capability on our side; Gling capability unknown |

App evidence: [MCP actions](../src-tauri/src/mcp/actions.rs),
[inspection and proposals](../src-tauri/src/mcp/tools.rs),
[transcript edits](../src-tauri/src/transcript/edit.rs),
[short suggestions](../src-tauri/src/ai/shorts.rs),
[captions](../src-tauri/src/captions.rs),
[audio settings](../src-tauri/src/project/audio.rs),
[audio processing](../src-tauri/src/media/polish.rs),
[README](../README.md), and [backlog](future-work.md).

### Export, platform, and language differences

Gling documents finished media export and paid desktop timeline export for Premiere,
Resolve, and Final Cut. Its guide cautions users to choose the matching format, retain
original media, and check the imported sequence. It does not establish that every caption,
zoom, effect, or generated element survives as an editable equivalent.
Source: [editor handoff guide](https://www.gling.ai/blog/integrating-gling-with-your-existing-video-editing-workflow).

In AeroEdits, MP4 export is implemented. No SRT exporter or professional-editor timeline
exporter was found in the reviewed source searches. Burned-in captions do not replace
subtitle-file export. Treat these as missing in the inspected code, subject to rechecking
as concurrent development continues.

Gling's FAQ lists 24 unique speech languages, including Greek. Its multicamera and
separate-microphone workflow is restricted to Resolve/Final Cut exports; Premiere and
direct media export require pre-synced input. This is not evidence of universal multicamera
support. The FAQ confirms SRT export.
Source: [pricing and FAQ](https://www.gling.ai/pricing).

Our configured languages are English and Greek. English has local transcription and
alignment; Greek uses online Scribe and has no local word aligner. The working tree also
contains optional local filler/retake review. Chapters and automatic short suggestions
still use the configured cloud AI provider. Do not describe the entire workflow as offline.
Evidence: `transcript/language.rs`, `transcript/settings.rs`, `ai/mod.rs`, and `ai/local.rs`
under `src-tauri/src/`.

Gling documents web, Windows, and macOS availability, plus optional YouTube connection
for requested title suggestions using recent upload titles. AeroEdits is a desktop app.
A local desktop UI alone does not establish local inference or a privacy advantage.
Source: [Gling help](https://www.gling.ai/help/gling).

### Pricing snapshot

On the research date, Gling lists Free with one edited hour/month and watermarked export;
Plus, Pro, and Elite at $20/$40/$100 monthly for 10/30/100 edited hours. Annual billing
shows $10/$20/$50 monthly equivalents. Paid allowances also cover specified link-video
word budgets. The flattened pricing grid does not reliably express feature exclusions;
use the FAQ's explicit restriction on free timeline export. Recheck before purchasing.
Source: [current plans](https://www.gling.ai/pricing).

### What to learn from Gling

1. **Measure cleanup quality before adding more detection features.** Use the same speech
   recordings and compare clipped syllables, lost meaning, filler misses, and incorrect take
   choices. A matching feature name does not establish a matching result.
2. **Make review faster.** Evaluate a simple import → cleanup → review → export path with
   before/after listening and easy restoration. Our many passes and options may create
   friction; this is a hypothesis, not a measured UI comparison.
3. **Consider SRT and editor handoff.** SRT is a focused portability improvement. NLE export
   needs a separate specification for cuts, source offsets, tracks, speed, transitions,
   captions, and graphics; test each destination rather than promising generic XML support.
4. **Keep the tutorial-specific advantage visible.** Clicks, keyboard activity, windows,
   screen content and camera framing offer signals a transcript alone cannot supply.
   The reviewed Gling documentation does not establish equivalent telemetry handling.
5. **Separate publishing support from video editing.** Title alternatives and descriptions
   could start as an agent-assisted output before adding a channel integration.
6. **Treat avatar generation as a scope decision.** It does not have to be built to compete
   on editing existing tutorial recordings.

### Proposed benchmark and unanswered questions

Use four identical inputs in both tools: an English talking-head recording with restarts;
Greek speech with English technical words; a tutorial with meaningful pauses and PC audio;
and a camera take with a separate microphone. Test generic-file onboarding separately from
`.aero` telemetry workflows so input differences remain visible.

Record import and processing time, corrections needed, cut-boundary quality, retained
meaning, audio quality, caption timing, short coherence, review effort, and export fidelity.
Listen to complete outputs and inspect imported timelines. Record plans, settings, versions,
and costs. No benchmark has been run yet.

Still unresolved: Gling's underlying models and local/cloud split, its B-roll implementation,
actual Greek quality, consistency of retake decisions, exact timeline-export fidelity,
and whether it offers an official automation interface not surfaced by this research.

### Research log

- **20 October 2026:** Reviewed official Gling product, help, pricing, and handoff pages.
  Compared against the current AeroEdits source and backlog. Added the capability matrix,
  limitations, possible priorities, and test plan. No application code changed.
