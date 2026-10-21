# Tutorial motion graphics: research findings and workflows

Updated: 21 October 2026.

Reading guide:

- [Findings at a glance](#findings-at-a-glance)
- [Evidence and reading scope](#evidence-and-reading-scope)
- [Findings from all five posts](#findings-from-the-five-supplied-posts)
- [Additional linked implementations](#additional-linked-implementations-we-read)
- [Earlier project research](#findings-carried-forward-from-earlier-project-research)
- [What the findings change for AeroEdits](#synthesis-what-the-research-changes-for-this-app)
- [Existing app capabilities](#what-the-app-already-provides)
- [Tutorial workflows and evaluation](#three-workflows-to-compare)

## Target

The creator makes shorts and long videos about AI tutorials and Greek gaming. The immediate
priority is polished AI tutorials with useful motion graphics. Gaming remains a secondary
use case; its pacing and visual identity should not dictate tutorial defaults.

Success means a viewer can follow the demonstrated action, understand an abstract concept,
and see the result. Visual polish, predictable revisions and reasonable production time
matter alongside clarity. A good-looking standalone showreel is insufficient evidence.

## Findings at a glance

The research points to a production system with separate creative direction, assets,
timing, rendering and review. The supplied material contains concrete implementations of
that system, not just prompt advice: seekable browser renderers, reusable character rigs,
style libraries with source, frame inspection tools and sound-generation scripts.

For AeroEdits, the strongest immediate direction is to improve how an agent plans and
authors graphics inside the existing HyperFrames pipeline. Brand kits, editable source,
library reuse and technical validation already exist in the app. The missing evidence is
whether the proposed creative process produces consistently better tutorial sequences.
This is an implementation-informed recommendation, not a measured comparison of engines.

Three particularly useful findings are:

- **Style can be packaged with working code.** The fifteen-style project publishes prompts,
  films and source; Lemo-Opuscar provides a larger style system. These are more actionable
  references than a folder of impressive social posts.
- **Inspection can be part of the toolchain.** Hanif's animation project includes contact
  sheets, transition strips, out-of-order frame verification and staged rendering. Our
  existing gate covers other technical checks; neither establishes editorial quality alone.
- **Interactive explainers and video assets are different deliverables.** Hairline and
  Anatomy offer useful visual systems, but their interaction needs an explicit timeline
  before it becomes a reusable narrated-video component. Anatomy Reveal already includes
  a film pipeline, with its own style, cost and silent-output constraints.

## Evidence and reading scope

All five supplied X posts were accessed in the browser. Three opened as full X articles;
their article bodies were read. The main linked tool pages, repository READMEs and selected
documentation were also read. Direct web fetching of X returned 403, while the browser
could access the posts. The TypingMind link initially failed but its `www` page was read
successfully. The Openverse API root returned 403 and was not read successfully.

This was not an exhaustive source-code audit of every repository. Embedded films were not
all watched, and linked collections were not traversed entry by entry. No comparative films
were rendered or external skills installed. Model attribution, completion times and demo
quality claims remain creator reports unless explicitly described otherwise below.

Evidence labels used in this document:

- **Source finding:** what a read article, README or documentation actually describes.
- **Local finding:** behavior or configuration found in AeroEdits source.
- **Interpretation:** what that evidence suggests for our tutorial workflow.
- **Untested:** a capability, integration or quality claim still requiring a practical test.

## Findings from the five supplied posts

### 1. Movez: ten tools for a motion studio

[Original post](https://x.com/0xMovez/status/2111841847408517282).

This is a mixed stack: creative instructions, interactive figure builders, runtime libraries,
a Blender integration and an audio API. They do different jobs and are not interchangeable
video generators. Reading the destinations revealed important distinctions:

| Linked resource | What its documentation establishes | Relevance and limits for AeroEdits |
| --- | --- | --- |
| [Anatomy Reveal](https://www.typingmind.com/skills/annnguyentt-anatomy-reveal) | Builds a physical object from convex parts, then produces an interactive dissection and a product-reveal MP4. Documents 3–60 s films, 30 fps, multiple ratios, a paper look and adaptive temporal sampling for blur. | A ready-made product teardown/reveal workflow. The documented film is silent. Its own estimates are hours for object construction and roughly two render minutes per film second; these are source estimates, not measured here. |
| [Anatomy](https://skills.wheresryan.sh/anatomy) | Generates live isometric machines whose moving parts explain an idea. The mechanical-watch example supports winding, slowing and exploding the mechanism. | Useful for a mechanism that benefits from spatial explanation. This is an interactive explanatory object, not an established drop-in video template. |
| [Hairline](https://hairline.lucasmarkes.com/skills) | Actually exposes two skills: `hairline-create` for an idea and `hairline-adopt` for an existing SVG. Produces a standalone HTML figure with an interaction/tour engine and checks. | A focused source of clean line-based explanatory graphics. Requires a deliberate time-driven playback adaptation for our renderer. The post's generic `/hairline` name is less precise than the destination. |
| [LottieFiles motion-design-skill](https://github.com/LottieFiles/motion-design-skill) | Provides motion principles, timing/easing tables, choreography, UI patterns and review guidance across animation systems. | Useful as creative direction, without replacing HyperFrames. The repository does not claim to be an MP4 renderer. |
| [Motion AI Kit](https://github.com/motiondivision/ai-kit) | Provides Motion documentation, animation advice and CSS spring generation. Additional example/source access and MotionScore auditing are associated with Motion+. The README identifies it as a Cursor plugin. | Some workflow ideas transfer; the full advertised package is not simply a free, agent-neutral rendering engine. Integration and access need checking before adoption. |
| [Scenario Blender plugin](https://github.com/scenario-labs/blender-plugin) | Describes an experimental Blender 5.0+ extension for Scenario generation and a local MCP server. Requires Scenario API credentials; generations consume Creative Units. | The destination is more specific than the post's “full AI 3D team” description. Useful for selected 3D assets or scenes, with external setup and usage cost. |
| [React Three Rapier](https://github.com/pmndrs/react-three-rapier) | Wraps the Rapier WASM physics engine for React Three Fiber, including bodies, colliders, joints, stepping and snapshots. Documents React/Fiber version compatibility. | A physics runtime, not a creative skill. Simulation needs an explicit reproducibility/seeking strategy for frame-based export; a live physics demo alone is insufficient. |
| [React Postprocessing](https://github.com/pmndrs/react-postprocessing) | Wraps postprocessing for React Three Fiber, with effects such as bloom, depth of field, noise and vignette. | A finishing layer for a 3D scene; it does not plan the scene or improve the teaching content automatically. |
| [GSAP](https://github.com/greensock/GSAP) | Offers sequencing, easing, morphs, motion paths and other animation tools. The README confirms the toolset and bonus plugins are free, including commercial use. | Relevant to controlled transitions and connected states. It still needs composition, assets and video rendering around it. |
| [Openverse API root](https://api.openverse.org/v1/) | Access failed with 403 in this session. | The post describes it as an audio source. Its catalog-size claim and asset permissions were not verified here; record it as a lead, not a validated audio integration. |

**Interpretation:** use a small core pipeline and add specialist tools for a specific visual
need. Installing all ten does not produce a coherent tutorial style.

### 2. Movez: twelve-step studio course

[Full article](https://x.com/0xMovez/status/2104216919033192746).

**Source findings:** the article separates model-authored animation code from the system
that captures frames and encodes video. Its twelve stages are:

| Stage | Concrete method |
| --- | --- |
| Pixels | Query an exact frame with `seek(t)` or `draw(t)`. |
| Setup | Keep runtime, renderer and house rules in a reusable project. |
| One-line prompt | Use a short showreel as a setup exercise. |
| Brand | Gather real logos, fonts and screenshots. |
| Reference | Extract visual rules into a style guide before animation. |
| Specification | Describe states and transitions rather than adjectives. |
| Engine | Capture frames in a browser and encode with FFmpeg, or use a framework. |
| Springs | Evaluate motion from time; sum responses to successive target changes. |
| Sound | Share timing between picture, score and sound effects. |
| Director brief | Preserve references, character rules, beats and deliverables. |
| Critique | Inspect contact sheets, transition strips and phone-size frames. |
| Ship | Recompose aspect ratios and package reusable source. |

It provides sample rendering, spring, beat-analysis and sound-synthesis code. Its XML
example retains one container across successive UI states; its long-production example
uses staged stills and an animatic. The published render example averages subframes for
blur. These are concrete implementation references, not benchmarked AeroEdits features.

**Interpretation:** for tutorials, drive explanatory states from speech and real actions.
The course's showreel tempo, 120 BPM examples and frequent visual changes are optional
creative choices. Preserve its reference → state plan → render → inspect → repair method.

### 3. AI Edge: motion engineering guide

[Full article](https://x.com/aiedge_/status/2112552093483671966).

**Source findings:** its brief has eight explicit dimensions: role, format, story, visual
style, typography, motion language, audio and exclusions. It proposes an agent interview
to fill the brief, a reusable brand skill, real local assets and scene-by-scene timings.

The examples include a launch promo, talking-head captions/overlays, post-to-video and
website-to-video. The website workflow gathers actual screenshots, colors, typography and
copy before building. Its review loop samples twenty frames, scores readability,
continuity, audio alignment and polish, then repairs the three largest weaknesses.
Human revision advice is specific: use timestamps, protect successful sections and save
versions separately.

**Interpretation:** the talking-head/overlay and real-asset capture processes fit tutorials
better than the launch-promo framing. Preserve revision intent explicitly: changing a label
should not redesign a successful sequence. A self-assigned score is a diagnostic aid,
not proof of audience comprehension or production quality.

### 4. rari: building a repeatable motion studio

[Full article](https://x.com/0xwhrrari/status/2105643919119696297).

**Source findings:** the five layers are director, reference, timeline, renderer and critic.
The article makes each shot accountable to what the viewer should understand and describes
entry/exit states, exact product assets and seekable frames. It suggests choosing the
smallest renderer that supports the intended reuse.

Its motion guidance distinguishes quick controls, settling panels, restrained cameras,
stable headline holds and playful mascots. It specifically cautions against making
everything overshoot. It asks for a contact sheet plus transition strips and defects with
timestamps, evidence and local repairs. Landscape and portrait preserve the story while
changing composition.

It also specifies a durable project structure: brief, style guide, shot list, assets, source,
reviews and exports. Long runs progress through references/assets, planning, stills, rough
cut, repairs and final checks.

**Interpretation:** use those artifacts as the agent's working memory and revision contract.
Spring-only rules from other examples are not universal: motion should suit each object
and the tutorial's message. Rendering successfully is a technical milestone, not completion.

### 5. Kin: eight repositories

[Original Chinese post](https://x.com/KinGao476942/status/2112729924943634602).

The destinations were read directly. They divide into discovery collections, style systems
and executable examples. Star counts in the post were not used to rank suitability.

| Repository | Finding from the README | Useful next use |
| --- | --- | --- |
| [yihui-dev/awesome-opus5-5-videos](https://github.com/yihui-dev/awesome-opus5-5-videos) | Collects original posts and shared prompts; some entries link the post instead when no prompt was published. Documents 703 prompts and 418 live remakes in its dated snapshot. | Search by intended visual result, then inspect a selected example's actual provenance. A remake does not establish the original creator's complete process. |
| [lemomo-ai/lemo-opuscar](https://github.com/lemomo-ai/lemo-opuscar) | Publishes 51 film styles with code-created samples, guides and tools. Documents Canvas/WebGL rendering, sample-based scores, TTS, a style-customization workflow and an optional storyboard step. The author explicitly says styles are tuned for Opus 5.5. | Study a small number of relevant styles and adapt one into a channel identity. Equivalent quality with another model remains untested. |
| [mexicat/pdoom-video](https://github.com/mexicat/pdoom-video) | Implements word-synchronized karaoke typography with each frame defined by song time. Documents browser preview and 1080p60/4K60 export. | Study word-level emphasis and shared text/audio timing. A music-video timeline needs adaptation to tutorial speech. |
| [JohnHeibel/PDoomVideo](https://github.com/JohnHeibel/PDoomVideo) | Publishes nine chapter files, shared rigs/timeline, p5.js/p5.brush composition and a Chrome/FFmpeg renderer. The README reports two generations and includes model-written `ANIMATION_GUIDE.md` and `STORYBOARD.md`. | Concrete evidence of shared style/code instructions and modular scene production. The existence of these files is verified; their entire implementation was not audited. |
| [Vincentwei1021/mg-styles-15](https://github.com/Vincentwei1021/mg-styles-15) | Provides fifteen ten-second films, style notes, prompts and full source. Documents `window.renderAt(t)`, headless capture, FFmpeg, Python audio and a Blender exception for 3D. Reports two revision rounds and AI scores of 7.57–8.21. | Especially useful for a reference-to-source study. Published scores are creator-reported AI evaluations, not our independent assessment. |
| [JohnHeibel/ClaudeAnimationBase](https://github.com/JohnHeibel/ClaudeAnimationBase) | Provides a p5.js/p5.brush cartoon starter with Clawd, 31 emotions, a model guide and rendering tools. Documents storyboard/contact-sheet use and warns that watercolor fills can take seconds per frame without a dedicated GPU. | Candidate for occasional character explanations. Avoid committing the whole tutorial pipeline to expensive brush rendering before timing it locally. |
| [athemeroy/awesome-claude-5-5-videos](https://github.com/athemeroy/awesome-claude-5-5-videos) | Catalogs source-linked workflows with model attribution and production evidence. Its 11 October snapshot reports 424 distinct source posts, including 301 Opus entries, and nine-frame sampling for 199. Explicitly says sampling cannot verify full motion or model calls. | A stronger provenance/research index than an unqualified demo list. Its own limitations should carry into our conclusions. |
| [joeseesun/opus-video-prompts](https://github.com/joeseesun/opus-video-prompts) | Publishes eighteen prompt documents and 54 public cases across nine technical routes. Distinguishes code-rendered graphics from workflows orchestrating video-generation models. | Reusable starting briefs and route examples. A copyable prompt alone is not a reproducible production environment. |

Numbers above describe the read documentation snapshot, not stable catalog sizes or
independently recounted inventories. The earlier Movez course links an older athemeroy
repository name; Kin links the broader current collection read here.

## Additional linked implementations we read

| Resource | Concrete finding | Implication |
| --- | --- | --- |
| [buildwithhanif/claude-animation-skill](https://github.com/buildwithhanif/claude-animation-skill) | Uses Node Canvas and FFmpeg, with brush/watercolor drawing, rigs, scene assets and synthesized audio. Its harness offers `sheet`, a twelve-frame `strip`, `verify` for out-of-order determinism, and staged rendering that preserves an existing successful output. | Borrow these inspection and recovery mechanisms for the authoring workflow. Their presence is more concrete than generic “self-critique” advice. |
| [WinterArc21/Battle-of-Austerlitz-Film](https://github.com/WinterArc21/Battle-of-Austerlitz-Film) | Documents a WebGL2 film with real terrain data, procedural assets, time-driven formations/effects, Kokoro narration, synthesized sound and resumable render chunks. | Shows how longer code-rendered explainers separate data, narration, direction, rendering and sound. It is a specialized implementation, not a general template already integrated with our app. |
| [guanmo-ai/awesome-ai-motion](https://github.com/guanmo-ai/awesome-ai-motion) | A browsable collection spanning product promos, explainers, short motion, characters, interactive work, narrative and music. Some entries explicitly have incomplete supporting materials. | Discover relevant examples by category; verify whether source and production notes actually exist for each selected reference. |
| [Remotion core idea](https://www.remotion.dev/docs/idea) and [agent skills](https://www.remotion.dev/docs/ai/skills) | Documents frame-number-driven React compositions and skills for creation, markup, preview, render, captions and other tasks. | An alternative composition architecture worth comparing only for a concrete unmet need. The documentation does not prove better results than HyperFrames. |
| [HyperFrames overview](https://github.com/heygen-com/hyperframes/blob/main/docs/developers/overview.mdx) and [README](https://github.com/heygen-com/hyperframes) | Documents editable HTML with timing attributes and seekable animation runtimes, plus CLI, SDK, Player and rendering surfaces. Current README distinguishes the core skills from the complete published set. | The rendering architecture already fits AeroEdits. Do not adopt upstream-main commands or capabilities without checking the app's pinned runtime. |

## Findings carried forward from earlier project research

The [20 October motion graphics research](motion-graphics-research.md) predates these five
posts. It remains relevant and should not be replaced by this tutorial-focused document.
Its evidence was documentation and an installed-skill review on the Windows machine,
not a hands-on Higgsfield benchmark on this machine.

- **Higgsfield's clearest documented distinction was Adobe handoff.** Its Motion Designer
  advertises native editable After Effects elements and an Adobe-connected workflow.
  That establishes a different delivery route, not superior output from the same brief.
- **The existing skill stack already described broad capabilities.** Typography, diagrams,
  transitions, charts, maps, 3D, shaders, media and reusable components were present in the
  reviewed HyperFrames instructions. Availability in a skill is distinct from a tested
  feature exposed by AeroEdits.
- **Motion blur needs a specific comparison.** The inspected HyperFrames streak recipe
  approximated blur. The newer sources above demonstrate temporal sampling as a separate
  technique. We have not established that the app's current render path uses it or measured
  its cost and benefit.
- **Creative defaults may constrain quality.** Earlier instructions encouraged ambient
  motion, multiple focal points and layered backgrounds. The research proposed testing
  whether selective use, deliberate stillness and reference-led hierarchy improve results.
  That remains a hypothesis, not a demonstrated defect in every generated graphic.

## Synthesis: what the research changes for this app

| Evidence | Resulting workflow decision | Remaining uncertainty |
| --- | --- | --- |
| Real-asset gathering appears in all three long guides. | Capture actual screens and approved copy before animation; use diagrams explicitly as explanations. | Target tracking through AeroEdits crop/zoom/layout changes needs testing. |
| Concrete code examples use known-time frame evaluation. | Preserve seekable, reproducible compositions and editable source. | Specialist physics and interactive figures need adaptation. |
| Style libraries publish prompts alongside working implementations. | Select references with inspectable source and retain the useful design rules. | Model portability and local rendering performance are unmeasured. |
| rari differentiates motion by object; LottieFiles provides choreography guidance. | Define motion for text, panels, pointers and diagrams separately. | No universal spring or pacing preset has been validated for our tutorials. |
| Source projects include strips, determinism checks and revision artifacts. | Review transitions and moving sequences, preserve versions and repair named defects. | The best amount of review per graphic needs a practical time/quality comparison. |
| Local brand/library/gate infrastructure already exists. | Improve authoring and reusable graphic selection before introducing another general renderer. | A controlled comparison could reveal a genuine renderer limitation. |
| Most showcased audio examples target reels or music videos. | Synchronize tutorial explanation to speech; treat beat-driven editing as a deliberate exception. | Automatic internal cue timing and graphic audio integration have not been audited. |

The evidence supports testing **a better production workflow around the current engine**
first. It does not yet justify claiming a best model, a best renderer, an automatic quality
score, or a specific production-speed improvement.

## What the app already provides

These observations come from local source, rather than upstream capability lists:

| Capability | Evidence | Implication |
| --- | --- | --- |
| Six tutorial templates plus a blank composition | `src-tauri/src/graphics/templates.rs` | Titles, steps, shortcuts, key points, recaps and end cards already have a baseline. |
| Editable HTML rendered through pinned HyperFrames | `src-tauri/graphics/runtime/package.json`, `src-tauri/src/graphics/render.rs` | Evaluate new compositions in the existing pipeline first. Upstream main documentation does not establish compatibility with the pinned release. |
| Cached VP9 WebM with transparency; 30 fps, 1920 px long side | `src-tauri/src/graphics/render.rs` | Judge integration at these actual settings. A 60 fps or native 4K external demo is not a comparable app result. |
| Brand colors, a font and local logo/font assets | `src-tauri/src/graphics/brands.rs` | Establish a Greek-capable tutorial identity through existing kits. |
| Versioned library copies with a `useWhen` description | `src-tauri/src/graphics/library.rs` | Successful compositions can become reusable choices for agents. |
| Static checks, rendered layout checks, long-text and duration variants | `src-tauri/src/graphics/gate.rs` | Technical validation already exists; extend evidence with motion and editorial review. |
| Transcript-word placement, source writing, rendering, frames and storyboard | `src-tauri/src/mcp/skill/SKILL.md`, `templates/tutorial.md` | The authoring workflow can use existing agent actions. |
| Short graphics can occupy camera/screen slots or overlay the screen | `src-tauri/src/mcp/skill/SKILL.md` | Test explanations beside the recording as well as full-frame interruptions. |

The current tutorial instructions recommend few full-screen graphics, roughly one per
minute, and inspecting one or two examples. That is a conservative editorial policy, not
a measured optimum. A higher density may be appropriate for useful small annotations;
adding more full-screen cards is a different decision.

Graphics use app-controlled entrance behavior. Full-frame graphics cut; easing is allowed
in the documented camera-overlay case. New compositions must respect that contract.
Do not import a generic fade-out recipe unchanged.

## Three workflows to compare

### 1. Direct the viewer through a real screen

Use for finding a control, choosing a setting, entering a prompt or noticing an output.

1. Identify the exact action and corresponding spoken words after the edit is stable.
2. Capture the actual application state and specify the intended target region.
3. Select the smallest intervention: zoom alone, a highlight, a short label, or a detail inset.
4. Keep the target visible while the action happens; remove the annotation when it has served
   its purpose. Preserve the exact wording of English UI controls in Greek narration.
5. Review with the real screen, camera, captions, zoom and layout changes present.

Prototype: “Επίλεξε το μοντέλο” beside the real model selector, followed by a focused view
of the resulting setting. No invented application screenshot.

Unverified integration issue: screen-relative annotations need a clear coordinate contract
through crop, zoom, layout and aspect-ratio changes. Short overlay slots exist; this audit
does not establish automatic target tracking or equivalent long-form placement behavior.
Test those transformations before promising a reusable highlight component.

### 2. Explain a concept with a small animated sequence

Use when the recording cannot show the mechanism: an agent loop, retrieval, context,
tool calls, or a multi-step automation.

1. Write one sentence the viewer should understand at the end.
2. Build a literal state sequence before choosing effects. Example:
   question → search → retrieved passages → answer, with each stage named accurately.
3. Choose a visual metaphor only if it helps; a simple flow can outperform an elaborate
   mechanical model. Distinguish explanatory simplifications from real product behavior.
4. Reveal each state at the corresponding spoken phrase. Hold the completed relationship
   long enough to inspect it. Speech controls explanatory timing; music may support it.
5. Use one consistent set of text, connection, emphasis and transition behaviors.

Prototype: a 10–15 second Greek retrieval explanation, with a genuine source excerpt and
an explicitly illustrative diagram. Compare a simple SVG/HTML sequence against one
adapted specialist figure. The duration is a test specification, not a general rule.

### 3. Demonstrate a result or comparison

Use for the payoff: original versus processed image, prompt versions, two model outputs,
or manual versus automated steps.

1. Collect real inputs and results from the tutorial.
2. Identify the difference the viewer should notice and write concise labels.
3. Compare under equal framing and scale. Use a wipe, aligned panels or sequential detail
   views according to what makes the difference easiest to see.
4. Use measured values only; never invent a time saving or quality score.
5. For portrait, try a stacked comparison or sequential close-ups rather than shrinking
   the landscape composition.

Prototype: an image-editing result with Greek labels “Πριν” and “Μετά”, timed to the spoken
explanation and tested with both a face camera and captions.

## Production contract

For each graphic, record:

- Its teaching purpose and why a graphic helps at this moment.
- Transcript word IDs and desired entry, emphasis, hold and exit times.
- Approved copy, real assets and source of factual claims.
- Layout mode: replacement, overlay, inset or short slot; target region if needed.
- Style reference and the exact traits to borrow.
- Greek/English font and wording requirements.
- Separate landscape and portrait layout decisions.
- Editable fields and what a later revision should preserve.
- Evidence from technical checks, representative stills and moving playback.

Create a rough still or animatic before polishing. Reuse a library item when it serves the
purpose; create a custom composition when it cannot. Apply the app's technical checks,
then watch the moving result in context. Fix timestamped defects locally. Save a reusable
library version only after it proves useful and under the existing library-save workflow.

## Greek and format checks

Use real Greek copy from the script, including tonos, diaeresis, final sigma, punctuation,
and mixed English product names. Check the chosen font actually renders those glyphs with
consistent weight. For any case-conversion effect, review the resulting Greek rather than
assuming English typography rules transfer.

Try long Greek headings, multi-line labels and actual code or UI strings. Existing long-text
checks are useful but do not establish deliberate Greek coverage. Bundle the intended font;
do not judge a system-font fallback as if it were the final identity.

Shorts require their own composition: fewer simultaneous elements, explicit camera/caption
clearance and readable detail. Long videos can sustain a longer explanatory hold. Do not
force a meaningful visual change every two seconds when the viewer needs to read or act.

## First controlled evaluation

Use three small fixtures, one per workflow above, with the same approved script, assets,
style reference and output settings in each comparison. These may use synthetic tutorial
material initially; final validation needs a representative real recording.

Compare:

1. Existing templates, zooms and layouts where applicable.
2. A custom HyperFrames composition with the production contract above.
3. A specialist approach only where it adds a relevant capability (for example an adapted
   explanatory figure). Record adaptation work and dependencies.

Record model/tool versions, agent time, render time, failures, human corrections and any
paid usage. Preserve the first result and one bounded revision, such as changing the Greek
heading or extending a spoken explanation while retaining the established look.

Evaluate clarity, factual accuracy, screen readability, temporal continuity, speech alignment,
visual polish, portrait quality and revision reliability separately. A factual error, covered
control or unreadable label blocks acceptance regardless of aggregate score. Prefer blind
comparisons where practical. AI critique is diagnostic; it does not establish viewer learning
or justify “8/10” as proof of quality.

The first decision is whether custom authoring and better direction materially improve
these examples within the existing renderer. Only then decide which components, agent
instructions or rendering features deserve implementation.
