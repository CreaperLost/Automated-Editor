# Competitors and tools

What other products and agent toolkits do, what we take, and what we refuse. Merged on 21 Oct
2026 from the Borumi/Tella/Premiere-MCP notes in the old backlog and the ChatCut, Gling,
Higgsfield, After Effects, DaVinci Resolve, Herk Edit Kit, Astra Premiere and Tandem reviews;
updated 27 Oct 2026 with the [awesome-claude-video-skills](evidence/claude-video-skills-2026-10-27.md)
catalogue (362 repos) and its measured sixteen-skill test.

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
| **OpenChatCut** | Open-source local-first conversational editor, multi-track timeline, Agent Skills, MCP, Remotion rendering | Agent editing on a real timeline, with skills | Local-first + skills + our own renderer | Being the same shape without our telemetry, approve-before-apply or deterministic graphics |
| **BaoCut** | AI video agent with an editable timeline: transcribe, subtitle, translate, dub, cut, animate | Transcript-as-spine editing | Transcript-driven cuts; subtitle/dub as one pipeline | — |
| **FireRed-OpenStoryline** | "Intention-driven directing" editing agent | Editorial workflow as the product | Directing by intent; editorial primitives | — |
| **The skill landscape** (362 repos) | Generated catalogue of agent-operable video skills | Everything | Their published harness: fixed brief, no-skill control, file-based scoring, published method | Reading the catalogue as an authority; it is model-graded, and 16 of 362 were measured |
| **Motion Design Studio 600** | 600 named motion briefs in 30 categories, with technique references | Our graphics vocabulary | A named, checkable vocabulary for what a graphic can *do* — the "600 ideas" are candidate brief templates | 600 one-line briefs as a library; a style is tokens and motion rules, not a list of ideas |
| **Harness Engineering 600** | 600 building blocks for long-running agents, 30 categories, one line each | Our own agent design | The approval and permission entries name our existing design better than we do | Treating a label list as a design; it is a reading of Anthropic's docs |
| **HyperFrames Studio** | HeyGen's commercial editor on the open-source runtime we pin | Same renderer, agent-first | Our own dependency's owner, selling to our buyer | Sitting on their runtime; the pin is a dependency, not a moat |
| **MotionVideo** | Open directory of 500 AI motion videos with prompts, plus a paid skills pack | Their pack ships guides for our renderer | Their motion tokens are our STY-2, sold; their feedback model is ours | A directory of from-scratch motion graphics — our gap, not their market |
| **Aloud** | "Feedback your coding agent can run" | Agent-facing review loop | The closest thing to our review board as a product | — |

**The position to defend:** in the creator videos every agent drives an NLE the user already pays
for. We *are* the editor, with a typed API, tutorial telemetry, approve-before-apply and
deterministic graphics. Resolve's native MCP is the most direct threat. The [skill
catalogue](evidence/claude-video-skills-2026-10-27.md) says the same thing from the other side:
**DaVinci Resolve appears once in 362 entries** — the script-drivable editor won the skills wave,
and the one that cannot be scripted did not.

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
- **For us:** source search first ([FTG-1](../done.md#ftg-1)), recovery
  ([SAFE-2](../done.md#safe-2)), speech matching ([AUTO-2](../done.md#auto-2)), script
  assembly ([FTG-4](../roadmap.md#ftg-4)), delivery recipes ([DLV-2](../done.md#dlv-2)).
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
  the argument for explicit target ids ([SAFE-1](../done.md#safe-1)) and checking results.
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
- **For us:** style trial ([PLAN-1](../done.md#plan-1)), curated tutorial graphics
  ([GFX-3](../done.md#gfx-3)) (selection references: dropped 24 Oct, see
  [decisions](../decisions.md#not-doing)),
  fast revisions and variants ([GFX-4](../done.md#gfx-4)), checkpoints
  ([SAFE-2](../done.md#safe-2)), guided B-roll that keeps narration intact (video-only
  streams; an empty `PlaceAsset.streams` places **all** streams including unwanted audio; avoid
  occupied magnetic tracks) ([FTG-2](../roadmap.md#ftg-2)), save-this-workflow
  ([AUTO-3](../roadmap.md#auto-3)), spotlight/redaction/magnifier ([FX-2](../done.md#fx-2)),
  seam fades (FX-4, [done 22 Oct](../done.md)), translated subtitles ([DLV-4](../roadmap.md#dlv-4)),
  delivery list ([DLV-2](../done.md#dlv-2)).
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
  ([EVAL-1](../done.md#eval-1)); SRT ([DLV-1](../done.md#dlv-1)); a scoped handoff
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
  to word ids and source clocks ([PLAN-1](../done.md#plan-1)); port validator *ideas* into
  `check`; curate 8–12 cards into our library after converting fixed 1920×1080 positions to
  responsive layout, `data-slot` to declared variables, CDN GSAP/fonts to local files, colours to
  the brand contract, then passing the gate with real English and Greek copy
  ([GFX-3](../done.md#gfx-3)).
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
- → [AGT-2](../done.md#agt-2), [REV-1](../done.md#rev-1), [FX-3](../done.md#fx-3),
  [FTG-4](../roadmap.md#ftg-4).

## HyperFrames upstream — and HyperFrames Studio

[heygen-com/hyperframes](https://github.com/heygen-com/hyperframes) is the public repo of the
runtime we pin (0.8.162). Its skills advertise GSAP, Lottie, Three.js, Anime.js, CSS, WAAPI and
TypeGPU adapters, but our gate and authoring rules allow only WAAPI/CSS with no network: "the
skills can do 3D" is not the same as "our app can" ([GFX-2](../done.md#gfx-2)). Its
`motion-blur-streak` recipe approximates blur with streaks; temporal sampling is a different
technique. Some of its creative defaults (ambient motion everywhere, two focal points, layered
backgrounds) can make busy work; let references and hierarchy override them. Stay pinned and
review upstream at each bump ([Q-HYPERFRAMES](../decisions.md#q-hyperframes)).

**But the project is now also a product.** HeyGen ships
[HyperFrames Studio](https://hyperframes.studio) — *"The video editor built for agents. Built by
the team who open sourced HyperFrames."* The page reports **73,765 stars, 6,689 forks, 121
contributors and 809,606 npm downloads a week**, and `motionvideo.xyz` lists it in its 48-entry
Editor directory as open source. Every other entry in this research is a company that *might*
build our product. This is **the owner of our own pinned dependency, already selling an
agent-first editor** to the same buyer, with the same renderer underneath. Whatever we pin, we
sit on their runtime — re-check this at each bump, not only the API.

**And it ships a free desktop app plus a free academy.** *Framey* is the HyperFrames desktop app
— free for Mac, Windows and Linux — and the **[Pit Crew Academy](https://hyperframes.studio/lessons)**
is twelve lessons, two tracks of six, that teach the method and sell the app. Full ingest:
[evidence/hyperframes-pit-crew-academy-2026-10-27.md](evidence/hyperframes-pit-crew-academy-2026-10-27.md).
200 minutes of course time, 31 minutes of video, 51 recap points, 62 vocabulary terms, 50 quiz
questions with worked answers, and the prompt for every lesson. **Vendor claim** — we ran none of
it.

Two things in it are about our decisions, not their product:

- **They solved the problem we declined.** Their review lesson teaches *Select* (click an element
  in the preview and it becomes a chip in your message) and *Draw* (circle a spot, add a note;
  each note waits in **Pending edits** so you can collect changes while you watch and send them
  together). That is anchored feedback — the thing we said *No* to on 24 Oct — shipped as a
  product, with batching. Their line: *"Three precise fixes in one reply land better than three
  rounds of vague ones."*
- **They split the work between agent and human.** *"Some fixes are quicker by hand… You do not
  need to ask the agent for a two second trim."* Trim, move, delete and volume stay direct
  manipulations on the timeline; the agent takes the rest. Undo is one press. Our
  approve-before-apply is a gate; theirs is a gate plus a fast manual path.

Their third export rule is also worth noting: *"Then watch the exported file once, from start to
end, before you send it to anyone."* Verify from the file, not from the impression — now three
independent sources.

## The skill landscape: 362 repos, and one measured test

[awesome-claude-video-skills](https://github.com/zhuyansen/awesome-claude-video-skills) is a
generated catalogue of repos that let an agent make or edit video, split into ten types. Raw
catalogue and the raw test data:
[evidence/claude-video-skills-2026-10-27.md](evidence/claude-video-skills-2026-10-27.md).
**Everything here is a maintainer or vendor claim** — none of these repos was installed by us.

**It is generated, not curated.** `data/skills.json` carries a `generated` stamp and names the
live page as its source; a decision model applies four rules (makes video; agent-operable; has a
README; ≥50 stars, or under 50 with a README quality bar). The maintainer says a repo near a
cut-off can land either side. Treat it as a wide net, not an authority.

| | |
|---|---|
| Repos | 362 — promo 69, general 64, explainer 64, editing 47, shorts 42, motion 25, story 20, music 13, craft 10, avatar 8 |
| Stars | 272,634 total; median 24. Three above 10k. A long tail, not a leaderboard |
| Grade | 353 safe, 9 caution, **0 unsafe**. The grade is of the README and install steps, not the code |
| Licence | 236 MIT, 45 NOASSERTION, 44 none, 26 Apache-2.0, 8 AGPL-3.0 |
| Language | 138 Python, 86 JavaScript, 73 TypeScript, 33 HTML. Chinese-language repos are a visible share of the catalogue |

### The peers we have not met

The 47 "editing" entries are the ones shaped like us. Six we had not seen:

| Repo | ★ | What it is | Why it matters |
|---|---:|---|---|
| [0xsline/OpenChatCut](https://github.com/0xsline/OpenChatCut) | 2,715 | Local-first conversational editor, multi-track timeline, Agent Skills, MCP, Remotion rendering | Our exact shape, open source. The closest thing to us on the list |
| [JimLiu/baocut](https://github.com/JimLiu/baocut) | 1,545 | AI video agent with an editable timeline: transcribe, subtitle, translate, dub, cut, animate | Transcript as the spine, on a real timeline |
| [FireRedTeam/FireRed-OpenStoryline](https://github.com/FireRedTeam/FireRed-OpenStoryline) | 3,568 | "Intention-driven directing" editing agent | Editorial workflow as the product, not a plugin |
| [Agentchengfeng/chengfeng-videocut-skills](https://github.com/Agentchengfeng/chengfeng-videocut-skills) | 3,142 | Claude Code Skills video-cutting agent | A skills-as-product approach to cutting |
| [cartesiancs/cartcut](https://github.com/cartesiancs/cartcut) | 818 | "Video Editor for AI agents, built on the belief that open source can beat commercial tools" | Direct challenger |
| [YeJe-cpu/SeeCut](https://github.com/YeJe-cpu/SeeCut) | 394 | "An AI editor that watches its own cut": talking head → auto-edited short, optional layered CapCut draft | Self-review as the mechanism; landing in CapCut is how we expect it to work |

Four more are worth a look because they already do what our docs ask for: [hassancs91/claude-youtube-editor](https://github.com/hassancs91/claude-youtube-editor)
(351★, the whole YouTube pipeline from a talking head), [erduo1998-cell/erduo-broll-loop-engineering](https://github.com/erduo1998-cell/erduo-broll-loop-engineering)
(232★, SRT-driven B-roll that routes HyperFrames/Remotion — our two renderers, behind a skill),
[Cassette-Editor/oh-my-cassette](https://github.com/Cassette-Editor/oh-my-cassette) (121★, editing
plugin **and** MCP server), [zenstory-ai/video-recap-skills](https://github.com/zenstory-ai/video-recap-skills)
(592★, the scene-detect → ASR → VLM → script → TTS → ffmpeg chain, with an optional editable draft).

**The Premiere MCP cluster is bigger than our note.** Besides the one we track, the list has
[hetpatel-11/Adobe_Premiere_Pro_MCP](https://github.com/hetpatel-11/Adobe_Premiere_Pro_MCP) (768★),
[leancoderkavy/premiere-pro-mcp](https://github.com/leancoderkavy/premiere-pro-mcp) (440★, a
local-first CEP bridge) and [ayushozha/AdobePremiereProMCP](https://github.com/ayushozha/AdobePremiereProMCP)
(128★, Go). **DaVinci Resolve appears once in the whole catalogue** — the Resolve MCP ecosystem is
not skill-shaped, which is why our own note on it stands alone. That asymmetry is the clearest
signal in the list: the skills wave grew on the editor that can be driven by a script, not the
one that cannot.

**Four MiniMax repos**, which we should read: [gnipbao/minimax-h3-video-reverse-skill](https://github.com/gnipbao/minimax-h3-video-reverse-skill)
(284★, H3 shot reverse-engineering with a verification tool), [killkli/minimax-h3-colab-skill](https://github.com/killkli/minimax-h3-colab-skill)
(121★), [kakaxi12/vox-director-codex](https://github.com/kakaxi12/vox-director-codex) (13★, Vox-style
collage with HyperFrames and MiniMax narration) and
[10291124jcjc-dotcom/ancient-rap-battle](https://github.com/10291124jcjc-dotcom/ancient-rap-battle)
(156★, Suno + Midjourney + H3 lip-sync).

### The measured test: sixteen skills against one brief

The maintainer also ran the 16 candidates against **one fixed brief** with a **no-skill control**
(`octocat/Hello-World`), on one machine, no API keys for video/image/voice models. This is the
controlled experiment our own creator-video research said did not exist.

The brief: a 15-second promo for a fictional app, five fixed lines of copy, H.264 1920x1080 30 fps,
"draw with code, shapes, type and colour", **sound optional**. Four checks, every one read from
the delivered file: it renders to a playable file; duration/size/fps/codec match; every line of
copy is legible in at least one frame sampled at 2/s (a VLM reader, three passes, majority vote,
calibrated on the control first); and the picture moves. *How good it looks is not scored.*

| Skill | ★ | Checks | Lines | Sound | Held frames | Minutes |
|---|---:|---:|---:|---|---:|---:|
| **octocat/Hello-World (no skill)** | 0 | 4/4 | 5/5 | yes | 3% | **4.4** |
| vincentsch/explainroo | 523 | 4/4 | 5/5 | yes | 0% | 2.0 |
| wshuyi/remotion-video-skill | 394 | 4/4 | 5/5 | no | 10% | 3.4 |
| vibe-motion/skills | 1,315 | 4/4 | 5/5 | yes | 7% | 4.4 |
| remotion-dev/skills | 5,005 | 4/4 | 5/5 | no | 34% | 4.5 |
| Barty-Bart/motion-graphics | 580 | 4/4 | 5/5 | no | 48% | 4.7 |
| digitalsamba/claude-code-video-toolkit | 2,198 | 4/4 | 5/5 | yes | 24% | 5.5 |
| diffusionstudio/lottie | 5,493 | 4/4 | 5/5 | no | 34% | 5.6 |
| hypit-ai/hypit | 20,799 | 4/4 | 5/5 | yes | 0% | 6.5 |
| iart-ai/motion-skills | 610 | 4/4 | 5/5 | yes | 10% | 6.5 |
| charlie947/motion-graphics-skills | 174 | 4/4 | 5/5 | yes | 28% | 7.0 |
| howseen-ai/claude-motion-design | 408 | 4/4 | 5/5 | yes | 14% | 7.9 |
| feitangyuan/onetake | 2,080 | 4/4 | 5/5 | yes | 45% | 8.5 |
| calesthio/OpenMontage | 64,239 | 4/4 | 5/5 | no | 14% | 8.6 |
| op7418/guizang-product-video-skill | 762 | 4/4 | 5/5 | yes | 7% | 8.6 |
| echris6/motion-video-kit | 1,080 | 4/4 | 5/5 | no | 3% | 21.8 |
| heygen-com/hyperframes | 60,291 | 4/4 | 5/5 | no | 38% | 23.5 |

**What it says.**

1. **Every run passed every check — including the no-skill control.** For a spec-driven brief,
   Claude Code with no skill at all already produces a valid file with readable copy. A skill's
   value is not mechanical correctness. The maintainer's own line: *a skill earns its place with
   a look, a workflow, or footage to edit.*
2. **The control was faster than four of the sixteen.** 4.4 min against explainroo's 2.0 min on
   one side and HyperFrames' 23.5 min on the other. Stars do not predict speed: the 64k★ project
   was the slowest, the 523★ one the fastest.
3. **Sound was optional in the brief and only 9 of 16 skills added any** — against **12 of 12**
   creator videos in our corpus, where sound is never optional. Our docs treat sound as part of
   the job; the skills catalogue treats it as a garnish.
4. **"Held frames" (the fraction of sampled frame pairs that were identical) separates the
   skills that actually move.** Onetake is one continuous shot — 45% held, the lowest motion
   score, and it passes only because a slow drop still changes the frame. HyperFrames sits at 38%
   held. The control is at 3%.
5. **The method is published and reusable.** `score_video.py`, `measure_video.py`, `evidence.py`,
   the brief and the raw results are all in `zhuyansen/agent-skills-hub` under `ops/video-runs/`,
   and `SOURCES.md` ties each check to published work (TheoremExplainAgent, Code2Video, MoVer,
   VBench, SkillsBench and two Deganutti papers). Two of those citations decide our own
   measurement rules: VLMs misjudge duration by about 6 s and classify motion type below 13%
   accuracy, so **measure duration and motion from the file, not with a model**; and a static
   video scores well on consistency alone, so **motion must be measured beside consistency**.

**What it does not say.** Sixteen of 362, picked for the test rather than at random. One brief,
one promo, fixed copy — the hardest question is whether a line survives, not whether the film is
good. One machine, one driver, no API keys for video/image/voice models, so a skill that needs a
paid model was scored on a degraded path. "How good it looks is not scored" is a deliberate
limitation, and the look column is the maintainer's eye.

## Two open knowledge bases worth reading

[beamnxw/my_works](evidence/beamnxw-my-works-2026-10-27.md) is a personal AI knowledge base with
four artefacts and no code: a **production protocol** for building launch videos with Opus 5.5 and
Remotion, the **eight-file harness** that implements it, and two 600-entry catalogues — *Harness
Engineering* (600 building blocks for long-running agents) and *Motion Design Studio* (600 motion
briefs). MIT, 27 stars, one author, two weeks of commits. **Vendor claim** throughout; we did not
run the harness.

Two things in it are worth taking.

**The motion catalogue is a graphics vocabulary.** 30 categories — text reveals, kinetic
typography, cursor and hover, buttons, bento and cards, AI interfaces, shader and distortion,
audio-reactive motion, spatial camera moves — each with 20 named briefs and technique references
to motion.dev, tympanus, GSAP, Rive, Spline, Three.js and Blender. That is a *named* list of what
a graphic can do, which is the vocabulary our style library is missing. It is not a template
library: our styles hold tokens and motion rules, and one line of prose is not a style.

**The harness catalogue names our own design.** The 30 categories are Task contracts, Project
instructions, Context retrieval, Memory, Reusable skills, Planning, Workflow orchestration, The
agent loop, Tool schemas, Tool execution, MCP and connectors, Permissions, Lifecycle hooks,
Isolation and sandboxes, Specialized roles, Delegation, Models and reasoning effort, Goals and
completion, Runtime state and checkpoints, Recovery and resuming, Validation and tests, Evidence
and citations, Evaluation datasets, Graders and scoring, Observability, Resources and budgets,
Environment and bootstrap, Deliverables and artifacts, Security and provenance, Reuse and
packaging. 21 of its 29 references are to `code.claude.com`. The entries that name *our* design
best: `action-preview`, `approval-content-binding`, `approval-staleness-check`,
`external-write-approval`, `tool-description-contract`, `structured-tool-result`,
`tool-capability-boundary`, `argument-validation`, `tool-call-deadline`, `read-write-tool-split`,
`instruction-size-budget`, `skill-review-step`, `stage-artifact-passing`, `review-owner`,
`success-evidence-map`, `memory-provenance`, `memory-write-permission`, `mcp-tool-discovery`.

Its protocol is the published form of the same pipeline the creator videos show, and it is
covered in [synthesis.md](synthesis.md) (F4, F5).

## MotionVideo, and the agent-first editor wave

[motionvideo.xyz](https://motionvideo.xyz/en) (Aniket Pawar, who also runs Shadcn Labs) is two
products sharing a domain. The **directory** is open source (MIT): 500 AI-made motion videos with
the prompt behind each, plus directories of 339 tools and 49 motion studios, in nine languages.
The **skill pack** is sold separately — $29 launch, $49 after 100 purchases, one-time — and
contains agent skills, scene starters, motion tokens and renderer guides for **Remotion,
HyperFrames, Editframe and fframes**. Full analysis:
[evidence/motionvideo-2026-10-27.md](evidence/motionvideo-2026-10-27.md). **Vendor claim**
throughout; we did not buy or run the pack.

Three things to take, one to watch.

- **Its renderer guides cover our renderer.** The pack ships setup notes for Remotion,
  HyperFrames, Editframe and fframes. A $29 skills pack that teaches an agent our pinned
  renderer is a cheaper way to raise the bar than a competitor.
- **Its "motion tokens" are our STY-2, already productised.** *"Spring, easing and timing values
  tuned for motion design, kept in one place so every scene moves with the same rhythm."* Our
  style plan says a style gains motion rules and look rules; this is that, sold as a pack.
- **Its feedback model is ours.** *"You stay in charge of the cut. Review the storyboard, ask for
  changes in plain words, and render again."* Same chat-based, plain-words review we chose on
  24 Oct, with the agent storyboarding before it writes any scene code.
- **Watch:** it is a directory of *from-scratch* motion graphics. No source footage, no
  transcript, no cut, no B-roll. That is our gap, not their market.

Its **Editor directory names eight agent-first editors we had not tracked**:
[OpenChatCut](https://github.com/0xsline/openchatcut), [Diffusion Studio](https://diffusion.studio/)
(*"the video editor your agents can drive"*), [Kinara](https://www.usekinara.com/) (*"a local video
editor for Claude and Codex"*), [Aloud](https://www.aloud.sh/) (*"feedback your coding agent can
run"* — an agent-facing review loop, and the closest thing to our review board as a product),
[Editframe](https://editframe.com/), [fframes](https://fframes.studio/), [Clapper](https://clapper.archastro.tryintern.dev/)
(*"write videos in React, render them to MP4"*) and [Powermove](https://trypowermove.com/).
Also new: `Mortiflix` (*"a local motion studio, from brief to animatic"*, open source),
`launchvideo.dev` (*"launch videos, animated by your agent"*), `PilotCut`, `CyberCut`, `Dina`,
`SmoothCut` (*"automatic zooms, without a video editor"*) and `ScreenKite` (*"record your screen.
let AI edit it"*). **Aloud** is the one to read first: it is an agent-facing feedback product, and
feedback is where our own decisions went quiet on 24 Oct.

## A fair comparison, when we run one

Same inputs in both products: a short talking head, a 3–5 min screen tutorial with telemetry, an
English/Greek sample; same goal, reference, assets and revision requests ("change one phrase",
"move this callout", "restore the approved look", "insert two video-only B-roll ranges, then cut
speech and undo"). Record time to an acceptable first cut, lost or clipped speech, graphics over
UI, manual corrections, revision latency, failures and total spend; check the middle and end, not
just the intro. Start from our own baseline ([EVAL-1](../done.md#eval-1)).

**Take the harness with it.** The measured test above is a template for a fair comparison, and
the method is published, so use it rather than inventing one: fix the brief and the inputs, run
a no-skill control on the same machine, score from the delivered file rather than from a model's
opinion, and keep quality judgement separate and labelled. Adopt the two rules the cited work
forces — measure duration and motion from the file, and never read motion off consistency alone.
Add what our own docs already ask for that the harness cannot see: speech that is lost or
clipped, graphics over UI, revision latency and total spend, and a human signing off on the
result. A skill that renders a valid file is the floor, not the product.
