# After Effects and MCP: implications for AeroEdits

Investigated 21 October 2026. Discussion paper; no integration implemented.

## Recommendation

Improve AeroEdits' motion planning, reusable compositions and moving-preview review now.
Evaluate After Effects as an optional authoring/rendering backend for work that benefits
from Adobe typography, effects, compositing or native project delivery. Keep the editor,
recording, transcript anchors and final assembly in AeroEdits.

An MCP connection supplies control of an engine. It does not by itself supply art direction,
good choreography, readable typography or reliable revisions. Assess those separately.

This extends the earlier [motion research](motion-graphics-research.md) and
[tutorial workflow review](tutorial-motion-workflows.md) with AE-specific findings and a
closer look at the actual app integration points.

## Evidence and limits

- Read the two supplied repositories, selected source files, two alternative bridges,
  Adobe documentation, and AeroEdits' current graphics implementation.
- Source observations below mean code/documentation inspection, not execution in AE.
- Did not install bridges, execute downloaded code, launch AE, render comparative films,
  or inspect generated AEP projects. Did not watch the example films end to end.
- No After Effects application appeared in the standard `/Applications` inventory checked
  here; this does not establish whether Adobe access or another installation exists.
- Repository `main` branches are mutable. Pin an audited commit before any pilot.

## What After Effects adds

AE's useful distinction is the authoring and finishing environment: text animators and
selectors, detailed velocity/easing controls, masks and shape paths, effects, layered
compositing, and native editable projects. Text animators can target characters or words;
speed controls give a designer detailed influence over motion between keys.
Sources: [Adobe text animation](https://helpx.adobe.com/after-effects/desktop/animating-text/text-animation/animating-text.html),
[speed controls](https://helpx.adobe.com/after-effects/desktop/animate-in-after-effects/speed-between-keyframes/speed.html).

These are production controls, not proof that an agent uses them effectively. Browser
animation can also produce complex type, diagrams and camera-like movement. A comparison
must establish the practical quality and revision advantage for our tutorial work.

Adobe distinguishes scripts that automate application operations from expressions that
drive properties. AE scripting uses ExtendScript; its scripting interface can also expose
controls and export Motion Graphics templates.
Source: [Adobe scripting documentation](https://helpx.adobe.com/ca/after-effects/desktop/automate-in-after-effects/automate-animation/scripts.html).

Rendering can be automated with `aerender`, using a saved project, composition and output
path. It can start a separate instance or reuse a running instance. This is a render
interface, not an authoring or live-preview API. Scripts/bridges still need to create and
save the project. Source: [Adobe automated rendering](https://helpx.adobe.com/after-effects/desktop/render-and-export/automate-rendering/automated-rendering-network-rendering.html).

MOGRTs expose selected controls for use in Premiere; they are not a portable browser
animation runtime. AeroEdits currently has no MOGRT interpreter. Supporting AE templates
would require a compatible rendering service or a local AE path, plus our own controls.
Source: [Adobe Essential Graphics and MOGRTs](https://helpx.adobe.com/after-effects/desktop/motion-graphics/work-with-motion-graphics-templates/creating-motion-graphics-templates.html).

Adobe also documents Dynamic Graphics Render, a cloud service that populates and renders
AE-authored MOGRTs from structured data. It is worth considering for template variation at
scale, rather than treating desktop MCP as the only route. API access, commercial terms,
supported effects and upload requirements would need a separate assessment.
Source: [Adobe DGR guidance](https://developer.adobe.com/audio-video-firefly-services/guides/dgr/dgr-best-practices).

### A current development worth tracking

Adobe's AI Assistant is now documented as an **After Effects beta** feature, with its
overview updated 16 October 2026. It supports project operations, expressions, text/shape
work, template controls and rendering, and accepts custom skills. Access depends on region
and enterprise settings; Adobe says most creative AI actions use credits.
This is another benchmark candidate. The page does not establish a public API through
which AeroEdits can drive that assistant.
Source: [Adobe AI Assistant overview](https://helpx.adobe.com/after-effects/desktop/work-with-after-effects-ai-assistant/after-effects-ai-assistant-overview.html).

## The repositories

### Supplied: cth9191/after-effects-motion-prompts

This is a production-reference collection, not an MCP server. Its three examples pair
finished previews with briefs, storyboards and revision material. They were made through a
separate Higgsfield bridge. The reusable workflow separates planning, construction and
timestamped revisions; object identity and connected transitions are central ideas.
Sources: [repository](https://github.com/cth9191/after-effects-motion-prompts),
[prompt template](https://github.com/cth9191/after-effects-motion-prompts/blob/main/PROMPT-TEMPLATE.md).

Its scope notes explicitly exclude native AEPs and execution logs. KITE combines generated
imagery with editable AE elements and represents a fixed view with shallow depth, rather
than an orbitable 3D scene. The notes distinguish local bridge work from paid cloud
generation. Treat these as creator disclosures, not a verified billing guarantee.
Source: [sources and scope](https://github.com/cth9191/after-effects-motion-prompts/blob/main/SOURCES.md).

The linked `higgsfield-ai/fnf-local-pluging-bridge-mcp` repository and its raw README both
returned 404 during this investigation. The cause is unknown. We therefore cannot currently
reproduce the documented bridge setup from that public link or audit its implementation.
Reuse permissions for example assets also need checking before incorporating them.

**Use for us:** adopt the planning and revision structure; do not treat the example prompts
as an installable animation engine or a reproducible quality benchmark.

### Supplied: Dakkshin/after-effects-mcp

The README documents an MIT-licensed Node MCP server controlling AE through an open
ScriptUI bridge panel. It covers compositions, several layer types, masks, expressions,
keyframes and raw scripts. Source: [repository](https://github.com/Dakkshin/after-effects-mcp).

The server writes a shared `ae_command.json` and reads `ae_mcp_result.json`. Several tools
return a queued acknowledgement and require a later result read. Command matching in the
wait helper uses a command name rather than a unique request ID. **Inference:** rapid or
concurrent requests need serialization to avoid overwriting pending work or associating
the wrong result; the inspected protocol is insufficient as our production job contract.
Source: [server implementation](https://github.com/Dakkshin/after-effects-mcp/blob/main/src/index.ts).

The keyframe helper searches a limited set of property groups, calls `setValueAtTime`,
and may insert an initial key at the current composition time. It does not expose temporal
ease or spatial tangent controls in that helper. Advanced animation can still be authored
through raw scripting, but this shifts more work and error handling to the agent.
Source: [AE bridge implementation](https://github.com/Dakkshin/after-effects-mcp/blob/main/src/scripts/mcp-bridge-auto.jsx).

**Assessment:** useful learning/reference implementation and possible manual prototype;
not my preferred foundation for a dependable AeroEdits integration without substantial work.

### Alternative: solomondivyananth/aftereffects-mcp

The README documents macOS/Windows support, a CEP panel reached through localhost HTTP,
project inspection, keyframe/easing editing, frame capture, templates, and background video
render jobs. Its breadth makes it the first candidate I would test on this Mac. It remains
an early project: the README contains conflicting statements about tests/CI and says
authoring/rendering have less mileage. Source: [repository](https://github.com/solomondivyananth/aftereffects-mcp).

The package actually declares `test` and `test:live` scripts; their existence does not mean
they pass here. Source: [package manifest](https://github.com/solomondivyananth/aftereffects-mcp/blob/main/package.json).

Its motion-review tool creates a sampled MP4/contact sheet. Defaults are 16 samples and
12 fps playback, so do not confuse this diagnostic with full-rate, correctly timed final
playback. Mechanical proof checks also explicitly defer visual feel to review.
Source: [MCP implementation](https://github.com/solomondivyananth/aftereffects-mcp/blob/main/mcp/ae-mcp.js).

The security design documents loopback binding, token authentication, refusal of browser
requests, and a project write guard. It also explicitly says raw scripting is not fully
contained by that guard. These are useful design elements to inspect, not a security audit.
Source: [security model](https://github.com/solomondivyananth/aftereffects-mcp/blob/main/SECURITY.md).

### Alternative: HeroicSwan/after-effects-mcp

The README describes request IDs/timeouts, a Startup-script bridge, frame capture, rendering,
motion blueprints and reusable branded templates. Its “headless” wording means avoiding a
manually opened control panel in normal operation; the workflow still requires AE running.
Source: [repository](https://github.com/HeroicSwan/after-effects-mcp).

The installation guide explicitly requires Windows 10/11. It also tells users to open a
status panel during setup. **Assessment:** useful architecture reference and possible
Windows pilot; not an established macOS option for us.
Source: [installation guide](https://github.com/HeroicSwan/after-effects-mcp/blob/main/docs/INSTALLATION.md).

## What AeroEdits already has, and the actual gaps

| Local observation | Implication |
| --- | --- |
| Editable `index.html` plus `graphic.json`, typed variables, project-local assets | We already retain source and expose useful revisions. |
| Pinned HyperFrames 0.8.162; bundled templates use paused, finite Web Animations | Upstream engine capability and what our app currently supports must be assessed separately. |
| `graphic_write_file` instructions specify Web Animations/CSS and no network | A broader animation-library workflow needs explicit compatibility work and local packaging. |
| Cached transparent VP9 WebM; 30 fps and 1920 px long side | Native 4K/60 fps graphics require changes here even if AE can render them. |
| Brand kits, reusable library, transcript-word placement | Keep these strengths in any AE integration. |
| Template gate checks layouts, long text, variables, durations and sampled stills | Good technical foundation; does not establish motion quality, continuity or narration alignment. |
| Graphic source assumes HTML; render hashing includes HyperFrames version | Adding AE needs an explicit backend/source type and different dependency fingerprinting. |

Local evidence: [graphics model](../src-tauri/src/graphics/mod.rs),
[renderer](../src-tauri/src/graphics/render.rs), [gate](../src-tauri/src/graphics/gate.rs),
[MCP actions](../src-tauri/src/mcp/actions.rs),
[runtime manifest](../src-tauri/graphics/runtime/package.json),
[title composition](../src-tauri/graphics/templates/title/index.html).

## Proposed improvement path

1. **Make motion intent explicit.** Store a small beat plan: narration anchor, exact copy,
   start/end states, focal object, movement, transition, reading hold and sound cue.
   Reuse object identities across beats. Choose motion to explain a concept.
2. **Build a focused tutorial library.** Prioritize a screen callout, connected workflow
   diagram, kinetic keyword emphasis, before/after reveal, result spotlight and chapter
   transition. Test Greek glyphs, long copy and portrait layouts from the beginning.
3. **Add moving review.** Render a full-rate preview of the graphic and its neighboring
   footage. Keep contact sheets for inspection, but evaluate pacing, handoffs and sound
   through playback. Make timestamped revisions easy to target and compare.
4. **Separate preview and delivery settings.** Assess configurable resolution/fps and
   engine-specific quality settings without forcing every edit to render at delivery cost.
5. **Pilot AE behind a narrow adapter.** AeroEdits owns placement, variables, assets,
   render jobs and final assembly. AE owns its native layers, effects and AEP source.

The optional adapter should track backend, project/comp identity, exposed variables,
dependency manifest, render spec, source revision and preview provenance. Preserve AEP and
required assets separately from disposable render caches. Include request IDs, serialized
mutation, progress, cancellation and clear failure states. Undo grouping alone is not a
transaction or a crash-recovery strategy.

For initial integration, render an opaque clip or an alpha-capable intermediate/PNG
sequence, then normalize it into a format the editor supports. Verify alpha interpretation,
color, fonts and audio before relying on that path. An MP4 preview does not carry the
transparency required for an overlay. AE layers remain editable in AE; they do not become
native AeroEdits layer controls merely because we retain the AEP.

This is a proposed design, not code already available. Do not promise bidirectional
conversion between arbitrary HTML compositions and arbitrary AE projects.

## A practical comparison before commitment

Use one brand kit and the same Greek/English copy for three short tasks:

| Task | What it tests |
| --- | --- |
| 8-second kinetic title and logo reveal | Typography, easing, blur and readable holds |
| 12-second connected three-step explanation | Object continuity, transitions and camera framing |
| Tutorial screen callout with a result reveal | Integration, alpha, caption clearance and narration timing |

Compare our present pipeline, an improved HyperFrames workflow and the selected AE bridge.
Use matched 1080p/30 fps output first; evaluate 4K/60 separately. Record active work time,
render time, failures, manual repairs and the effort for the same requested revision.
Review complete playback, native source editability and portrait behavior. No measured
winner exists yet.

The discussion decision is whether the immediate priority is **better tutorial graphics
inside AeroEdits**, **native Adobe production/handoff**, or both. My recommendation is the
first path immediately, with a contained AE pilot to establish where the second earns
its setup and maintenance cost.
