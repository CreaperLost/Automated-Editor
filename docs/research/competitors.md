# Competitors and tools

What other products and agent toolkits do, what we take, and what we refuse. Merged on 21 Oct
2026 from the Borumi/Tella/Premiere-MCP notes in the old backlog and the ChatCut, Gling,
Higgsfield, After Effects, DaVinci Resolve, Herk Edit Kit, Astra Premiere and Tandem reviews.

**How to read this.** Everything about another product is a **vendor or maintainer claim** read
from their pages, docs or source, unless it says "tested". None of these products was installed
and benchmarked against AeroEdits. Pricing is a snapshot from 20–21 Oct 2026.

## At a glance

| Product | What it is | Closest overlap with us | We take | We refuse |
|---|---|---|---|---|
| **DaVinci Resolve 21.1** | Pro NLE; Studio ships a native MCP | Agent editing of a real timeline | Source search, speech matching across takes, script-guided assembly, delivery recipes, project backups | Arbitrary script execution as the agent surface |
| **Premiere Pro MCP servers** (community) | Bridges into Premiere | Agent editing; footage understanding (Jumper) | Search → source range → selects → placement; explicit target ids; evidence that survives edits; progressive tool discovery | Matching a 283-tool catalogue |
| **ChatCut** | Browser/desktop agent editor | Agent editing with review | Style trial, selection references, curated graphics, fast revisions, checkpoints, guided B-roll, saved workflows, tutorial effects | Generation suite, avatars, marketplace |
| **Gling** | Speech-based rough-cut editor | Silence/filler/retake cleanup | SRT and NLE export; measure cleanup quality before adding detectors | Link-to-avatar videos |
| **Borumi** | Recorder + editor with an MCP | Screen recording + layouts + agent | Transactions with a timeline hash; guide-gating; its layout pitfalls as tests | — |
| **Tella** | Recorder + editor, public MCP, API and skills | Layouts, auto-layout, zooms from clicks | Auto-layout rules; all-or-nothing batch edits | — |
| **Higgsfield** (Motion Designer, AE plugin) | Agent that builds native After Effects layers | Motion graphics by an agent | Selling **skill + SFX library** as a pair: taste and assets travel together | AE as our engine |
| **After Effects MCP bridges** | Community bridges into AE | — | Request ids, serialized mutation, motion-review sampling ideas | An AE dependency now |
| **Tandem** (Mike Cann) | MIT, vibe-coded Mac editor with an MCP | Our thesis in miniature | **Comments on the timeline the agent reads**; transcript as the spine; "point your agent at it" | Writing a general NLE |
| **Herk Edit Kit** | Agent workspace: skills, 406 HyperFrames cards, validators | HyperFrames graphics, short-form workflow | Plan schema and validators as app checks; a small adapted graphics pack | Copying cards as-is; a second FFmpeg pipeline |
| **Astra Premiere skill** | A Codex skill driving Premiere by computer use | Editorial workflow | Executed / verified / approved kept apart; thorough review; push-ins; ordered take assembly | Computer use for our own edits |
| **invideo** | Cloud editor with its own agent | One-prompt rough cuts | — | Cloud-first |

**The position to defend:** in the creator videos every agent drives an NLE the user already pays
for. We *are* the editor, with a typed API, tutorial telemetry, approve-before-apply and
deterministic graphics. Resolve's native MCP is the most direct threat.

---

## DaVinci Resolve 21.1 and its MCP

- **Native MCP (Studio only):** File > Setup AI Assistants; the external assistant drives Resolve.
  A first-hand Windows audit reports a stdio `ResolveMCP.exe` with 14 tools: app status, scripting
  API discovery, `run_script` / `run_script_unsafe` (≤ 60 s), and colour assets (LUTs, DCTLs).
  Domain work is delegated to generated scripts. Manual p. 4318.
- **Features worth borrowing:** IntelliSearch (content search; needs model downloads and
  analysis), IntelliScript (matches a `.txt`/`.fdx` script to takes, alternatives on disabled
  tracks; picks by dialogue match, not performance), EQ/level/AI dialogue matcher across takes,
  Inspector presets, project backups. Greek is in its extended transcription pack.
- **The community MCP** (samuelgursky) is a different product: 37 compound + 390 granular tools,
  persistent source analysis, capability checks.
- **For us:** source search first ([FTG-1](../roadmap.md#ftg-1)), recovery
  ([SAFE-2](../roadmap.md#safe-2)), speech matching ([AUTO-2](../roadmap.md#auto-2)), script
  assembly ([FTG-4](../roadmap.md#ftg-4)), delivery recipes ([DLV-2](../roadmap.md#dlv-2)).
  Keep typed, reviewable tools instead of arbitrary scripts.
- Sources: [What's New](https://www.blackmagicdesign.com/products/davinciresolve/whatsnew),
  [21.1 manual](https://documents.blackmagicdesign.com/UserManuals/DaVinci_Resolve_21.1_Reference_Manual.pdf),
  [Windows MCP audit](https://github.com/dmmdea/davinci-resolve-cli/blob/main/docs/09-resolve-21-1.md),
  [community MCP](https://github.com/samuelgursky/davinci-resolve-mcp).

## Premiere Pro MCP servers

- **Jumper** ([page](https://getjumper.io/ai-agents/premiere-pro), [docs](https://docs.getjumper.io/)):
  local visual, speech and named-person search after an analysis pass, cross-project search
  including offline drives, summaries for long footage, XML export. Borrow the
  search → source range → selects → placement workflow.
- **Adobe_Premiere_Pro_MCP** ([repo](https://github.com/hetpatel-11/Adobe_Premiere_Pro_MCP)):
  283 catalogue tools over a CEP panel; a small default tool set plus `search_tools` /
  `get_tool_schema` / `invoke_tool`. Its [known issues](https://github.com/hetpatel-11/Adobe_Premiere_Pro_MCP/blob/main/KNOWN_ISSUES.md)
  include editing the wrong sequence and reporting success without a visible change, which is
  the argument for explicit target ids ([SAFE-1](../roadmap.md#safe-1)) and checking results.
- **leancoderkavy/premiere-pro-mcp** ([repo](https://github.com/leancoderkavy/premiere-pro-mcp)):
  a [context engine](https://github.com/leancoderkavy/premiere-pro-mcp/blob/main/docs/project-context-engine.md)
  with separate source and timeline revisions, media fingerprints and bounded evidence packs that
  survive timeline changes; editorial guides that separate evidence, plan preview and apply.
  The reference design for source search and scene descriptions ([FTG-3](../roadmap.md#ftg-3)).

## ChatCut

- **What it shows:** reference a look, try a 5–10 s sample, revise precisely, then apply across an
  editable timeline. Selection Mode adds references from clips, media, viewer regions, times and
  transcript selections. Styles hold references and written rules, not just branding. Skills
  save a successful process via a guided summary. Named versions beside undo. Per-graphic ProRes
  4444 export; XML/CapCut handoff that omits captions, effects and transitions. Desktop: masks,
  magnifier, local mosaic; translated captions that follow source timing; export queue with retry.
- **Tested here:** nothing. Two sponsored creator videos (Marcin AI, Brendan Jowett) were read
  from transcripts with sampled frames. Marcin's on-screen cost note was **$6.40** of host usage
  (auto-captions said "$640").
- **For us:** style trial ([PLAN-1](../roadmap.md#plan-1)), curated tutorial graphics
  ([GFX-3](../roadmap.md#gfx-3)) (selection references: dropped 24 Oct, see
  [decisions](../decisions.md#not-doing)),
  fast revisions and variants ([GFX-4](../roadmap.md#gfx-4)), checkpoints
  ([SAFE-2](../roadmap.md#safe-2)), guided B-roll that keeps narration intact (video-only
  streams; an empty `PlaceAsset.streams` places **all** streams including unwanted audio; avoid
  occupied magnetic tracks) ([FTG-2](../roadmap.md#ftg-2)), save-this-workflow
  ([AUTO-3](../roadmap.md#auto-3)), spotlight/redaction/magnifier ([FX-2](../roadmap.md#fx-2)),
  seam fades (FX-4, [done 22 Oct](../roadmap.md#done)), translated subtitles ([DLV-4](../roadmap.md#dlv-4)),
  delivery list ([DLV-2](../roadmap.md#dlv-2)).
- **Pricing (homepage, monthly):** 200 credits $45, 400 $88, 800 $160; manual editing,
  transcription and export use no credits. Pages disagreed with an older $25/100 offer.
- **Positioning lesson:** "local and agent-driven" is not unique (ChatCut Desktop runs local
  projects with an external agent over MCP). Our edge to prove is tutorial-specific behaviour:
  telemetry placement, screen/camera timing, retained speech, efficient review.

## Gling

- Speech-based rough cut: bad takes, silences, fillers, transcript editing, review/restore;
  captions, noise removal, auto zoom; long → short clipping with B-roll (sourcing unspecified);
  titles/chapters; link-to-avatar video. 24 languages including Greek. SRT export; paid timeline
  export to Premiere, Resolve and Final Cut. Free (1 h, watermark), $20/$40/$100 monthly.
- **Closest cleanup competitor.** Feature names match ours; quality doesn't follow from names.
- **For us:** measure cleanup quality on identical inputs before adding detectors
  ([EVAL-1](../roadmap.md#eval-1)); SRT ([DLV-1](../roadmap.md#dlv-1)); a scoped handoff
  ([DLV-3](../roadmap.md#dlv-3)). Avatars are a separate product.

## Borumi

Desktop recorder + editor, $79 lifetime, MCP since v0.29 (31 Jul 2026), no public docs. Known
from a community plugin that records what it verified:
[Samin12/edit-borumi-video](https://github.com/Samin12/edit-borumi-video).

- `borumi mcp` (stdio, app must run). Projects, timeline, transcript, `inspect_timeline` (a contact
  sheet including staged edits), trim/split/move/delete/speed, scenes, export.
- **Transactions:** `begin_project_edit` → edits carry a timeline hash → `commit_project_edit`
  with a one-line summary, or abort. **Guide gating:** edit tools refuse until the agent has read
  the matching guide on that connection.
- **Layouts are timeline segments.** Pitfalls we designed against (and should keep as regression
  tests): layouts drift after ripple trims; gaps flash a default layout; deleting a layout
  rebuilds others; exported audio is 44 ms behind the picture; the mix is normalised to about
  −13.7 LUFS.

## Tella

Public MCP, API and open agent skills: [layout tools](https://www.tella.com/docs/mcp-tools/layouts.md),
[developer index](https://www.tella.com/docs/_llms/developers.txt),
[tellahq/skills](https://github.com/tellahq/skills) (auto-cut, auto-layouts, remove-mistakes).

- Layout kinds: fullscreen, middle, side-by-side (regular, even, overlap, perspective),
  tv-presenter, cut-out, camera-bubble (9 positions, S/M/L), camera-only (punch-in), screen-only,
  custom; per layout a camera style, "pop-out", crop, B-roll in a slot (including a moment of the
  clip's own recording) and `hardCut` / `spring` transitions. Favourites.
- Also: zooms from clicks, mouse/key events, faces, auto crop, blurs and highlights, text
  overlays, SFX and music, AI image and SFX generation, LUT and font uploads, chapters, social
  publishing. `apply_video_edits`: up to 200 edits, all or nothing.
- **Auto-layout rules** (for [LAY-3](../roadmap.md#lay-3)): the screen for clicks, typing,
  menus, results; the speaker for a hook, opinion, reaction or sign-off while the screen is
  static; both otherwise (the base). A few changes per minute at most (the most common
  complaint is too many): screen focus 2–8 s, camera cutaway 2–5 s, punch-in 1.5–4 s. The camera
  sits on the side the speaker faces away from. Hard cuts between full-frame shots, animated moves
  for size changes.

## Higgsfield and After Effects

- **Higgsfield Motion Designer** builds native AE layers, keyframes and expressions from language
  inside an open project; the AE plugin uses no Higgsfield credits; generation does. Its
  **Editing Skill + SFX Library** is sold as one download, the clearest signal that taste and
  assets travel together. [Motion Designer](https://higgsfield.ai/ai-motion-designer),
  [editing skill](https://higgsfield.ai/s/astra-for-editing-higgsfieldai-XqLQyi).
- **AE itself** brings text animators, speed graphs, masks, effects and native editable projects;
  `aerender` renders but doesn't author; MOGRTs need AE or Adobe's Dynamic Graphics Render
  service; Adobe's AI Assistant is an AE beta (credits, no public API).
- **Bridges:** [Dakkshin/after-effects-mcp](https://github.com/Dakkshin/after-effects-mcp) (MIT,
  file-based queue keyed by command name, so concurrent requests can collide);
  [solomondivyananth/aftereffects-mcp](https://github.com/solomondivyananth/aftereffects-mcp)
  (CEP over localhost with a token, keyframe/easing, sampled motion review at 16 samples / 12 fps);
  [HeroicSwan/after-effects-mcp](https://github.com/HeroicSwan/after-effects-mcp) (request ids,
  Windows only). [cth9191/after-effects-motion-prompts](https://github.com/cth9191/after-effects-motion-prompts)
  is a prompt and revision collection, not a server.
- **Conclusion:** Higgsfield's established edge is AE handoff, not better motion from the same
  brief. Improve our own planning, templates and join review first; AE at most as an optional
  backend pilot later ([decisions](../decisions.md#not-doing)).

## Tandem

[mikecann/tandem](https://github.com/mikecann/tandem): MIT, Mac only, "point your agent at it and
make it your own" (the author takes no issues or PRs). A personal editor with an MCP server, the
transcript as the spine (words at the playhead highlighted, the agent's edits beside them) and a
**comment anchored to a time range that the agent reads and acts on**. Not a Convex product (an
earlier review got that wrong). The demo also credits an unlicensed title card beside licensed
clips: licensing needs to be a recorded property of an asset, not an afterthought.
Anchored comments were considered as NOTE-1 and dropped 24 Oct for a single user working in chat
([decisions](../decisions.md#not-doing)).

## Herk Edit Kit

[nateherkai/herk-edit-kit](https://github.com/nateherkai/herk-edit-kit) (commit `10511e5`; its
11 tests passed here). A local agent workspace: 15 skills (cleanup, whole edit, short-form,
storytelling, showreels, style selection), 406 HyperFrames draft cards (106 `vox-explainer`,
300 `kallaway`) indexed by purpose, slots and duration, transcript/EDL helpers, short-form
validators (caption correspondence, source-time mapping, scene coverage, anchors, footage ledger,
duplicate footage), showreel beat-grid and sound tools. GSAP-based; MIT for covered material,
AIS brand materials excluded, third-party notices kept. Its own notes say the 406 cards were not
all rendered or approved.

- **For us:** adapt the [plan schema](https://github.com/nateherkai/herk-edit-kit/blob/main/.claude/skills/short-form-edit/references/plan-schema.md)
  to word ids and source clocks ([PLAN-1](../roadmap.md#plan-1)); port validator *ideas* into
  `check`; curate 8–12 cards into our library after converting fixed 1920×1080 positions to
  responsive layout, `data-slot` to declared variables, CDN GSAP/fonts to local files, colours to
  the brand contract, then passing the gate with real English and Greek copy
  ([GFX-3](../roadmap.md#gfx-3)).
- The two Nate Herk videos show the same five steps (transcribe, cut, plan beats, build with
  skills, verify) and a lot of human art direction; first pass ~18 min, second ~10 min (creator
  reported).

## Astra Premiere editing skill

[per-simmons/astra-premiere-editing](https://github.com/per-simmons/astra-premiere-editing): a
Codex skill operating Premiere on macOS through accessibility and screenshots; documentation, no
engine or tests in the root listing, no licence file seen.

- **Good ideas:** keep *exposed*, *executed*, *inspected* and *approved* apart in every report;
  read the full transcript for bad takes and re-read after edits; waveform inspection and
  listening are separate evidence; restrained push-ins with pivot, easing, hold and return;
  natural ordering of numbered takes with an order audit.
- **Avoid:** UI fragility (clip bodies and edges aren't named accessibility objects); removing
  every pause as a goal; downloading arbitrary B-roll with browser cookies; turning every step
  into a mandatory approval stop.
- → [AGT-2](../roadmap.md#agt-2), [REV-1](../roadmap.md#rev-1), [FX-3](../roadmap.md#fx-3),
  [FTG-4](../roadmap.md#ftg-4).

## HyperFrames upstream

[heygen-com/hyperframes](https://github.com/heygen-com/hyperframes) is the public repo of the
runtime we pin (0.8.162). Its skills advertise GSAP, Lottie, Three.js, Anime.js, CSS, WAAPI and
TypeGPU adapters, but our gate and authoring rules allow only WAAPI/CSS with no network: "the
skills can do 3D" is not the same as "our app can" ([GFX-2](../roadmap.md#gfx-2)). Its
`motion-blur-streak` recipe approximates blur with streaks; temporal sampling is a different
technique. Some of its creative defaults (ambient motion everywhere, two focal points, layered
backgrounds) can make busy work; let references and hierarchy override them. Stay pinned and
review upstream at each bump ([Q-HYPERFRAMES](../decisions.md#q-hyperframes)).

## A fair comparison, when we run one

Same inputs in both products: a short talking head, a 3–5 min screen tutorial with telemetry, an
English/Greek sample; same goal, reference, assets and revision requests ("change one phrase",
"move this callout", "restore the approved look", "insert two video-only B-roll ranges, then cut
speech and undo"). Record time to an acceptable first cut, lost or clipped speech, graphics over
UI, manual corrections, revision latency, failures and total spend; check the middle and end, not
just the intro. Start from our own baseline ([EVAL-1](../roadmap.md#eval-1)).
