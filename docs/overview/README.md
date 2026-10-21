# AeroEdits docs: start here

Written 21 Oct 2026 as a map over everything in `docs/`. The documents in this folder are
**summaries and cross-references**; the detail and evidence stay in the original files, which
are linked from every section. When an original changes, update the summary here.

## The picture in two diagrams

How the user's agent works with the app today ([details](ai-process-today.md)):

![How the user's agent works with AeroEdits today](images/ai-collaboration-today.svg)

How each stage changes if the proposals land ([details](ai-process-tomorrow.md)):

![The AeroEdits AI loop: today and tomorrow](images/ai-loop-today-vs-tomorrow.svg)

Both are plain SVG files in [images/](images/) with light and dark colours; edit them by hand
when a stage changes.

## Read in this order

| # | File | What it answers |
|---|---|---|
| 1 | [vision.md](vision.md) | What AeroEdits is for, the principles, and the decisions not to reopen |
| 2 | [current-state.md](current-state.md) | What is built today: features, the three kinds of "AI", the 53 agent tools |
| 3 | [ai-process-today.md](ai-process-today.md) | How the user's agent and the app collaborate now, step by step |
| 4 | [themes.md](themes.md) | The gaps every research doc found, merged into 12 themes, with sources and the second opinion's corrections |
| 5 | [ai-process-tomorrow.md](ai-process-tomorrow.md) | How the AI process changes if the proposals land |
| 6 | [roadmap.md](roadmap.md) | One order of work, reconciled from the six different orders the docs propose |
| 7 | [open-questions.md](open-questions.md) | Contradictions between docs, and decisions waiting for George |
| — | [glossary.md](glossary.md) | The project's own words (revision, batch, slot, gate, beat plan…) |

## Map of every original document

Kinds: **Plan** (design + build log for our own work), **Research** (external products or
videos compared with our code), **Contract** (file formats shared with AeroShoot),
**Evidence** (raw material behind research), **Backlog**.

### Plans and build logs (`docs/plans/`)

| Doc | Kind | Status | One line |
|---|---|---|---|
| [ai-readiness.md](../plans/ai-readiness.md) | Plan | Living, phases 0–3 built | **The architecture.** MCP-only AI, proposals, app actions, skill, check, shorts scenes, graphics v1, library gate |
| [phase-2-handoff.md](../plans/phase-2-handoff.md) | Plan | Historical | Handoff brief for phase 2; superseded by the ai-readiness log |
| [layouts.md](../plans/layouts.md) | Plan | Built 21 Oct | Base layout + timed sections replace global layout, webcam focus and shorts scenes |
| [premiere-style-tracks.md](../plans/premiere-style-tracks.md) | Plan | Built 4 Oct | Timeline rewritten as tracks of clips with one mapping primitive (clocks) |
| [shorts-redesign.md](../plans/shorts-redesign.md) | Plan | Decided 17 Oct, partly built | Idea → Gather → Tighten → Dress; face framing; caption editing in shorts |
| [shorts-workspace-and-voiceover-shorts.md](../plans/shorts-workspace-and-voiceover-shorts.md) | Plan | V1 built 18 Oct | Shorts as a workspace tab; shorts from a new take with footage from the long video |
| [word-alignment.md](../plans/word-alignment.md) | Plan | Built | wav2vec2 forced alignment, 20 ms word edges, guardrails |
| [local-review-eval.md](../plans/local-review-eval.md) | Plan + data | Measured 21 Oct | Local Qwen 4B vs OpenRouter filler review → cloud is default |
| [performance-review-2026-10-04.md](../plans/performance-review-2026-10-04.md), [review.md](../plans/review.md) | Plan | Mostly fixed | Performance findings P01–P07, R01–R20; open O01–O04 |
| [revoice-learnings.md](../plans/revoice-learnings.md) | Plan | **Paused** 18 Oct | Re-voicing accent + lip sync: no local, commercially usable stack yet |

### Research (all dated 20–21 Oct 2026)

| Doc | Looks at | Headline for us |
|---|---|---|
| [agent-video-gaps-and-proposals](../agent-video-gaps-and-proposals-2026-10-21.md) | The 11 reference videos → our product | 8 ranked gaps; "the model is cheap, the training artefacts are the product" |
| [reference-video-review](../reference-video-review-2026-10-21.md) | 11 YouTube videos on AI editing / motion | 8 patterns: house style, taxonomy, plan→approve→build, real references, self-verify, sound, skill refinement, reformat |
| [short-form-reference-review](../short-form-reference-review-2026-10-21.md) + [part 2](../short-form-reference-review-part-2-2026-10-21.md) | 6 short-form tutorials | Persistent beat plan, event-driven sound cues, style profile, graphics render quality is capped at 30 fps / 1920 px |
| [chatcut-competitive-review](../chatcut-competitive-review-2026-10-21.md) | ChatCut (closest agent-editor competitor) | Style trial before scaling, "Ask AI about this" selection references, checkpoints, guided B-roll |
| [herk-edit-kit-research](../herk-edit-kit-research-2026-10-21.md) | Nate Herk's open kit (406 cards, validators) | Adapt the plan schema and validators into app checks; curate 8–12 graphics |
| [dan-kieft-production-workflow-review](../dan-kieft-production-workflow-review-2026-10-21.md) | A generation-heavy production workflow | A Production panel of timed tasks; manual asset round trip before any provider |
| [prompt-to-motion-workflow-review](../prompt-to-motion-workflow-review-2026-10-21.md) | Remotion prompt-to-motion demo | Graphic variants A/B with source snapshots; Create / Revise / Compare / Use |
| [after-effects-mcp-investigation](../after-effects-mcp-investigation-2026-10-21.md) | 4 AE MCP bridges | AE only as an optional backend later; improve our own motion first |
| [davinci-resolve-research](../davinci-resolve-research-2026-10-21.md) | Resolve 21.1 + its native MCP | Source search, speech matching across takes, script-guided assembly |
| [motion-graphics-research](../motion-graphics-research.md) | Higgsfield, HyperFrames, Gling | Higgsfield's edge is AE handoff, not better motion; Gling is the closest cleanup competitor |
| [tutorial-motion-workflows](../tutorial-motion-workflows.md) | 5 X posts + linked repos | Three tutorial graphic workflows and a per-graphic production contract |
| [astra-premiere-skill-review](../astra-premiere-skill-review-2026-10-21.md) | A Codex skill driving Premiere by computer use | Separate *executed*, *verified* and *approved*; a thorough review mode; review evidence stored per revision; word-timed emphasis; camera push-ins |
| [second-opinion-video-research](../second-opinion-video-research-2026-10-21.md) | A critique of the reference review and gaps doc, checked against the captions | Several factual errors and overreaches; put revision safety and playable review ahead of ornament; judge by tutorial comprehension |
| [ten-ai-editor-videos](../ten-ai-editor-videos-2026-10-21.md) | 10 more videos: Resolve 21.1 MCP, Premiere/AE by computer use, Claude Code stacks, invideo | Resolve's vendor MCP is the direct threat; self-improving skill files; music/SFX tracks; folder watch. **Several claims about our code are wrong** ([open-questions C9](open-questions.md#contradictions-to-fix)) |
| [nine-ai-editing-videos](../nine-ai-editing-videos-2026-10-21.md) | 9 videos: Higgsfield/Astra, Resolve MCP vs invideo, Cooperson presets, Tandem (Convex), HyperFrames | Timeline comments the agent reads; personal SFX library mapping; style presets; run cost reports. **Built from descriptions only — captions were IP-blocked** |

### How much to trust each research doc

Added 21 Oct after the second opinion. Summaries in this folder already apply these corrections.

| Doc | Caveat |
|---|---|
| reference-video-review, agent-video-gaps | Factual errors found by the [second opinion](../second-opinion-video-research-2026-10-21.md#a-errors-and-overreach): Tao's 49m30s is a first draft (3–4 h total), "106 SFX" and Joseph's "12 hours / 4.5B tokens" aren't in the captions, two taxonomies conflated |
| ten-ai-editor-videos | Video 10 (Convex) has no captions or description; its section is inference. Many code references are wrong or are UI commands, not MCP tools |
| nine-ai-editing-videos | No transcripts (YouTube IpBlocked on 21 Oct); every *(unverified)* line needs a caption check |
| All video reviews | Creator claims (times, costs, "one prompt") are unverified; captions can't show visual quality |

### Contracts, backlog and evidence

| Doc | Kind | One line |
|---|---|---|
| [future-work.md](../future-work.md) | Backlog | 17 numbered tasks, known issues, Borumi / Tella / Premiere-MCP notes |
| [keyboard-telemetry.md](../keyboard-telemetry.md) | Contract | `keys.jsonl`: typing stretches and shortcuts, never the typed text |
| [window-telemetry.md](../window-telemetry.md) | Contract | `windows.jsonl`: front window box, never titles or contents |
| [transcript.md](../transcript.md) + [reference-video-scripts/](../reference-video-scripts/) | Evidence | Captions of the 11 reference videos and the script that fetches them |
| [plans/review-eval/](../plans/review-eval/) | Evidence | Labelled transcripts for the filler-review evaluation |
| [plans/revoice-lab-scripts/](../plans/revoice-lab-scripts/) | Evidence | The paused re-voice lab |

## Keeping this folder honest

- Each claim here links to the doc it came from. If you can't find the source, the summary is wrong.
- Research docs mix vendor claims, creator claims and our source findings. The summaries keep
  that distinction: "the video claims", "the code shows", "we propose".
- Nothing in this folder is a decision unless [vision.md](vision.md) or a plan says George made it.
