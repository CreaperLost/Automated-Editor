# Roadmap

The one ordered plan. It replaces the old backlog (`future-work.md`), the reconciled roadmap and
the six competing "recommended orders" in the research docs. Each item names its gap in
[gaps.md](gaps.md); decisions that block an item are in [decisions.md](decisions.md).

**Sizes:** S ≈ up to two days, M ≈ up to a week, L = more than a week.
**Rules for this file:** pick items top-down within a tier unless George says otherwise. When an
item starts, link its plan (in `plans/`); when it ships, move it to [Done](#done) with the date
and delete its details here. Keep IDs stable; never renumber.

## The logic of the order

**Milestones (decided 21 Oct):** (1) a great long-form tutorial, (2) tutorial shorts, (3) gaming
shorts and gaming videos. Everything in Now and Next serves milestone 1; shorts work (SHT-*)
follows; gaming work is not scheduled yet.

1. **Done by 24 Oct: what exists is safe, complete and measured, and settings have one home.**
   Project binding, agent parity, history and checkpoints, the EVAL-1 baseline, presets and
   styles, one look for all graphics, caption type, graphics ready before export.
2. **Next: the joins by ear, and the craft surfaces still missing.** The agent hands the user
   the risky cuts to listen to, rules the agent proposes, sound clips and source search.
3. **Later: build the director loop on those foundations.** A plan file and style trial;
   idea-first shorts on top of source search; cues on top of sound clips; finishing and
   delivery.

```mermaid
flowchart LR
    SAFE1[SAFE-1 project binding] --> AGT1[AGT-1 agent parity]
    SAFE2[SAFE-2 history + checkpoints] --> GFX4[GFX-4 graphic variants]
    ORG1[ORG-1 presets + styles] --> PLAN1[PLAN-1 plan file + style trial]
    ORG1 --> STY2[STY-2 styles from references]
    ORG1 --> AUTO1[AUTO-1 run a preset after opening]
    FTG1[FTG-1 source search] --> SHT1[SHT-1 idea-first shorts]
    FTG1 --> FTG2[FTG-2 selects + B-roll]
    SND1[SND-1 sound clips] --> SND2[SND-2 cues at graphic events]
    FX1[FX-1 background matte] --> LAY4[LAY-4 cut-out layout]
```

---

## Now

Milestone 1 (a great long-form tutorial) is close: everything built up to 24 Oct was tried in
the dev app and in EVAL-1 run 4 (see [Done](#done)). Rerun EVAL-1 from
[plans/evaluation.md](plans/evaluation.md) after each cost or quality change.

<a id="ai-budget"></a>
### AI-1 · What an agent reads in an edit: about 100k tokens at most (a standing preference)

George, 24 Oct: what an agent reads in a whole edit (the tool list, the guides and every tool
answer, images included) should stay at about 100k tokens or less. A preference to steer by,
not a hard limit: no test fails on it. Our source code doesn't count.
- **Measured** by [plans/eval/tokens.py](plans/eval/tokens.py), run by
  [plans/eval/run.sh](plans/eval/run.sh) after each turn. EVAL-1 run 4 (24 Oct): the tutorial
  edit ends at 36.8k tokens, adding graphics at 44.6k, a short from a take after them at 53.6k,
  Claude Code's own prompt included. Biggest answers: `preset` 3.8k, `storyboard` 3.6k (with
  its image), `transcript_get` 2.1k, `graphics` 2.1k.
- **Next levers:** transcripts read once, and images ([COST-1](#cost-1)).

---

## Next

REV-1, AGT-3, AGT-4, SND-1, FTG-1 and LAY-1 were built 24 Oct (see [Done](#done)). Left in this
tier, in order (George, 25 Oct): **STY-2** styles from references (it takes in GFX-5's
AI-proposed styles), then **GFX-8** graphics an agent makes well on top of it, and what VER-1
couldn't cover here. Next up after them: SHT-1 (idea-first shorts, on FTG-1). The SFX starter
set (SND-3), review boards (COST-1) and custom vocabulary (LNG-5) were built 25 Oct. LAY-2, UI-1 and UI-2 were built 24 Oct.
NOTE-1 (anchored notes, "Ask AI about this") was dropped 24 Oct: the chat, the agent's history
and descriptions like "the graphic titled X" already do its job
([decisions](decisions.md#not-doing)).

<a id="sty-2"></a>
### STY-2 · Styles from references — L, plan: [plans/styles.md](plans/styles.md)

First, ahead of GFX-8 (George, 25 Oct): bad graphics aren't a shorts-only problem, and a style
today holds only look settings, so agents fall back on stock templates. Merges GFX-5's
AI-proposed styles, the old STY-2 and GFX-8's worked examples. A style gains motion rules, the
kinds of graphics it uses (with their 9:16 placement), 2–3 examples linked from the library,
its references and the agent's breakdown of them, look rules, a richer shorts variant and a
small sound set; fonts must resolve or ship with it. `library` `kind: style` `op: propose` makes
one from references: breakdown first, stills to narrow, one clip in place for the pick. First
test: a "my library" long-form style from George's own graphics.

<a id="gfx-8"></a>
### GFX-8 · Graphics an agent makes well — L, plan: [plans/graphics-quality.md](plans/graphics-quality.md)

After STY-2. The 25 Oct test (one short take, three agents) gave generic text cards every time:
stock templates, approval on a written plan, no sample, uninstalled fonts, cards over the face,
captions cut by splits. On top of the style: picking per moment what to show (text cards and
demonstrations are equals), one sample clip before the set, the plan always shown as a table,
checks the agent runs (face, captions, fonts), honest style knobs. George's decisions are in
the plan.

The bugs from the same test, FIX-8 to FIX-10, were fixed 25 Oct (see [Done](#done)); FIX-9
still needs a real Codex run.

<a id="gfx-5"></a>
### GFX-5 · Styles proposed by the agent — merged into STY-2

The look, the shorts look and bring-in-line shipped (see [Done](#done)). The leftover, styles the
agent proposes (research: [research/ai-styles.md](research/ai-styles.md)), is now part of
[STY-2](#sty-2) (George, 25 Oct).

<a id="ver-1"></a>
### VER-1 · Pay the verification debt — M

- **Done on Windows, 24 Oct:** on a disposable project (the 1:40 take), through the real bridge
  with a stdio MCP client: create → transcribe → silence → check (and its fix) → propose_edit →
  accept in the Agent panel → undo → export (1280×720, 49.75 s, −14.2 LUFS, checked with
  ffprobe/ebur128), plus every tool built that day.
- **Done on Windows, 25 Oct:** three real Codex sessions (`codex exec`) against the dev app:
  write tools, the templatize → check → save → remove loop, and a style's logo in a graphic in
  a real 720p export. Found: `codex exec` refuses tools not marked read-only unless each has
  `approval_mode = "approve"` in Codex's config (see
  [product.md](product.md#what-is-verified-and-what-is-not)).
- **Left:** the same on **macOS**.

---

## Later

Grouped by area. Order within a group is the suggested order.

### Direction: plan, style, method

<a id="plan-1"></a>
- **PLAN-1 · Plan file and style trial (M).** The agent writes and keeps a plan in the project
  (a Markdown file it reads back: per moment the purpose, treatment and reason, and what the
  user locked), so the context survives between turns and runs without new storage or UI. Then
  a **style trial**: build 2–3 beats on a 5–10 s range, approve the look, then scale.
  Rewritten 24 Oct without anchored notes. (Graphics already keep their word: cuts before a
  graphic move it along and no cut goes through one, `sequence/edit.rs`.)
<a id="auto-3"></a>
- **AUTO-3 · "Save this workflow" (M).** From an approved edit: instructions, defaults,
  exceptions and checks as a named, versioned workflow shown as a readable summary first. Proven
  only when reused on a second tutorial with different footage.

### Graphics

<a id="gfx-3"></a>
- **GFX-3 · Templates for real tutorial jobs (M + design).** Word-timed emphasis (each word at
  its own word id, re-timed after cuts); highlight a real UI target; step sequence; before/after;
  a lower-half overlay prototype. Purpose tags and search in the library. 8–12 total, all
  passing the gate with real English and Greek copy.
<a id="gfx-4"></a>
- **GFX-4 · Graphic variants (M).** Independent variant folders with source snapshots;
  Create / Revise / Compare / Use; "Use this version" is one undo step; revision scope (element /
  graphic / scene) so a colour fix never regenerates a sequence. Depends on SAFE-2.

### Sound

<a id="snd-2"></a>
- **SND-2 · Cues at graphic events (M/L).** Templates declare event markers (card lands, counter
  completes); a small procedural palette (tap, tick, pop, chime, airy whoosh, impact) plus
  imported files; a deterministic cue compiler (`cueStart = visualPeak − soundPeakOffset`,
  density limits, ducking); the agent picks which events deserve sound; sound style in words
  from STY-1. Depends on SND-1. **Was:** backlog #5 (second half).
<a id="snd-3"></a>
- **SND-3 · Sound and music libraries the user extends (M/L).** The SFX starter set is built
  (25 Oct, see [Done](#done)). Left: a copyright-free music library on the same pattern
  (browse, preview, search by tag, one-click placement, the user's own files). **Music must never need
  credit (enforced):** every track stores its licence; the built-in library holds only tracks
  allowed for monetised use without attribution (CC0 / public domain or equivalent); a track that
  requires credit, forbids monetisation or has an unknown licence is refused for the library, and
  a user import like that is flagged in the project and by `check` before export. Comes after
  the project rules and style kit (decided order). Depends on SND-1.
  Decision: [Q-SOUND](decisions.md#q-sound).

### Footage

<a id="ftg-2"></a>
- **FTG-2 · Selects and B-roll from own footage (M).** Named source ranges with notes; a reviewed
  placement batch: video-only streams, a free overlay track, narration untouched, word-anchored
  so later cuts re-resolve it. Depends on FTG-1. **Was:** backlog #4 (own footage first).
<a id="ftg-3"></a>
- **FTG-3 · Scene descriptions (M).** Agent-written descriptions and tags from sparse contact
  sheets, cached against source fingerprints; OCR for UI labels if keyword search misses.
  **Was:** backlog #14 next steps.
<a id="ftg-4"></a>
- **FTG-4 · Ordered take and script-guided assembly (M).** Natural ordering of take names
  (`1.2`, `1.10`), missing/duplicate takes shown; match a supplied outline to source transcripts,
  alternatives shown, one proposal batch.
<a id="ftg-5"></a>
- **FTG-5 · Import an outside transcript (S).** Deepgram / AssemblyAI / Rev JSON → our word model,
  Not needed (21 Oct); do it only if it stays this small.
<a id="ftg-6"></a>
- **FTG-6 · Stock B-roll providers (M).** Pexels / Pixabay with the user's key, licence recorded,
  download only on accept. **Was:** backlog #4.

### Shorts — plan: [plans/shorts.md](plans/shorts.md)

**Decided 22 Oct:** two features in two tabs that share their parts. **Shorts** start from a take
recorded outside the app (camera and voice); everything else goes under it. **Clips** cut the best
moments out of the long video (what the Shorts tab does today besides takes). Shorts from a take
came first (SHT-5, SHT-6, SHT-4, SHT-7, SHT-3, SHT-2, all done); Clips (SHT-1) next, after FTG-1.

<a id="sht-1"></a>
- **SHT-1 · Idea-first clips (L).** A clip is an idea plus pieces gathered from anywhere in the
  video. Depends on FTG-1. Comes after the take-first Shorts work.

### Layouts — plan: [plans/layouts.md](plans/layouts.md)

<a id="lay-3"></a>
- **LAY-3 · Auto layouts with limits (M).** Tella-style rules on top of `pick`: a few changes per
  minute at most, length windows (screen 2–8 s, camera 2–5 s, punch-in 1.5–4 s), camera on the
  side the face looks away from; the app proposes, the agent reviews with `list` + storyboard.
  **Was:** backlog #11.
<a id="lay-4"></a>
- **LAY-4 · Presenter cut-out layout (M).** Depends on FX-1's matte.

### Finishing and effects

<a id="fx-1"></a>
- **FX-1 · Webcam background removal, blur, replacement (L).** A segmentation matte computed once
  per recording and cached like face tracks, composited in wgpu. Also enables text drawn behind
  the presenter. **Was:** backlog #9.
<a id="fx-2"></a>
- **FX-2 · Tutorial effects (M).** Spotlight, redaction/blur of a region, magnifier, dim/blur
  under a graphic: time-bounded rectangles anchored to the screen layer, correct through crop,
  zoom, layout changes and cuts, in preview and export.
<a id="fx-3"></a>
- **FX-3 · Camera push-ins (M).** Start/end scale, pivot, easing, hold, return; matched framing
  across jump cuts; separate from screen zooms.
<a id="fx-5"></a>
- **FX-5 · Colour: LUT import and a simple grade (M). Parked 21 Oct: not needed now.** Per clip,
  preview/export parity, `clip_grade` op.

### Delivery

<a id="dlv-2"></a>
- **DLV-2 · Delivery list (M).** Video + shorts + sidecars as one queue; per-output status,
  retry, the revision each output rendered.
<a id="dlv-3"></a>
- **DLV-3 · Graphic export with alpha, then a constrained NLE handoff (M → L).** ProRes 4444 /
  WebM alpha for one graphic; then a cuts-only timeline (FCP7 XML or OTIO) tested in the
  destination editor with a report of what was flattened. **Was:** backlog #15.
<a id="dlv-4"></a>
- **DLV-4 · Translated subtitles (M).** Greek ↔ English cues with stable source anchors and their
  own revision history; glossary for product names. Dubbing stays out (YouTube auto-dubs).
<a id="dlv-5"></a>
- **DLV-5 · Real reformat 16:9 → 9:16 (L).** One operation that keeps UI targets, cursor
  relationships and reading time, not just the canvas size.

### Languages and transcription

LNG-1 (Greek word aligner), LNG-2 (more languages) and LNG-3 (more transcription models) were
removed 23 Oct 2026: not needed.

<a id="lng-4"></a>
- **LNG-4 · Filler/retake rules fixes and a real Greek eval (S/M).** Done: list items and
  parallel statements (22 Oct), single-word repeats and cut-off words (22 Oct), two-word
  restarts and "they, they've" (25 Oct), a fourth labelled English recording, EN-D (25 Oct:
  cloud cut all 6 must fillers, no wrong filler cut). Left: the rules still cut "human in the
  loop. Human in the loop essentially means" (needs more examples); **a real Greek recording**
  from George to label; George's call on `apply: true` for cloud fillers (only fillers; the
  evidence now backs it). Results: [local-review-eval](plans/local-review-eval.md).
  **Was:** task 3 leftovers.

### Automation

<a id="auto-1"></a>
- **AUTO-1 · Run a preset on request after opening (S).** Replaces folder watch, which was
  turned down: nothing starts when a recording finishes. After opening a project the user picks a
  preset (Tutorial, Shorts, Clips…) and its method runs up to its first question, with only
  free local steps unless the user confirms. After [ORG-1](#org-1) phase 3. Decision:
  [Q-WATCH](decisions.md#q-watch).
<a id="auto-2"></a>
- **AUTO-2 · Speech matching across takes (M).** Measured speech-level matching (then gentle EQ)
  so a re-recorded sentence doesn't jump.

### UI

Built: UI-1 and UI-2 (see [Done](#done)).

<a id="ui-8"></a>
- **UI-8 · Shorts tab: transcript first (S).** Add a Style tab; move "Clean the take" and its
  steps out of Look into the Transcript panel, which is the main thing; keep "what plays under
  it" short. From the 25 Oct test ([plans/graphics-quality.md](plans/graphics-quality.md#bugs)).

### Dependencies

<a id="dep-2"></a>
- **DEP-2 · Recurring dependency review (S each).** Follow the cadence in
  [dependencies.md](dependencies.md): read changelogs, update on a branch, run tests, `perf.rs`
  and the parity probes, record the result. Candidates already visible: React 18 → 19,
  Vite 5 → current, Tailwind 3 → 4, `ort` leaving release candidates, HyperFrames past 0.8.162.

### Performance

<a id="perf-3"></a>
- **PERF-3 · Dense timelines and the JPEG fallback (M).** Draw sub-pixel clip runs as one strip
  per track at full view (O01); two alternating encoder threads for the non-wgpu preview path
  (O02).

### Agent cost and quality — 99 % of the quality for 10 % of the price

Raised 23 Oct as long-term direction. Every item here is measured against
[EVAL-1](#eval-1): keep a cheaper path only when the edit stays as good.

<a id="cost-2"></a>
- **COST-2 · Tools on demand (S/M).** Measured and cut 24 Oct (see Done): 30 tools, 41,240
  characters (about 16k tokens); Claude Code with `ToolSearch` pays about 1.7k for the names.
  Left: measure Codex and the Claude app; then, if a client sends the whole list, change the
  list to the open project's preset (`notifications/tools/list_changed`: the shorts' tools only
  in a shorts edit) rather than more behind `more`.
<a id="cost-3"></a>
- **COST-3 · Cheaper reviewers (M).** Hand narrow judgement jobs (looking at a board, filler
  windows, chapter titles, checking a graphic) to a smaller model or a lower reasoning effort,
  through a subagent or the app's own OpenRouter pass with a cheap model; the main agent keeps
  the decisions. Each swap kept only if EVAL-1 says the result is as good.
- **STY-2 · Styles from references:** moved to [Next](#sty-2) on 25 Oct, plan
  [plans/styles.md](plans/styles.md).
<a id="loop-1"></a>
- **LOOP-1 · The user and the agent improve the app together (L).** Aim: a loop where each edit
  makes the next one better: the app, the bundled material (presets, styles, graphic
  templates) and the taste. Records what the user changed after the agent (proposals accepted
  or rejected, cuts restored, graphics redone) and turns patterns into proposed changes to a
  preset's rules ([AGT-3](#agt-3)), graphic templates and styles, each accepted by the user. Data-driven, never
  free-text rules learned on the side (the 17 Oct condition for bringing learning back).

### Code health

<a id="qual-1"></a>
- **QUAL-1 · Code reuse check (M), later.** Asked for 22 Oct: the same logic is often written
  again instead of shared, which costs time and causes bugs that are fixed in one copy only.
  Sweep the Rust passes, the MCP actions vs the UI commands, and the front-end panels and dialogs
  for repeated logic; move it into shared helpers with tests; note the patterns to reuse in
  `architecture.md` so new work starts from them. Seen so far: the same Tauri event-listener
  effect written out in `App.tsx` and several panels; line-ending-safe edits; Clips and Shorts
  must share one set of short panels (SHT-5).
<a id="qual-2"></a>
- **QUAL-2 · Review the tests (M), later, not urgent.** Asked for 23 Oct. Go through the Rust
  and front-end tests: drop ones that guard behaviour that is gone or was replaced (old
  project formats, removed alignment, superseded passes), merge near-duplicates, and keep the
  ones that pin decisions. Note how long the suite takes before and after.
<a id="qual-3"></a>
- **QUAL-3 · Build times and caching (S/M), later, not urgent.** Asked for 23 Oct: builds feel
  slow (a `cargo check` of the app takes about 3 minutes after a small change). Measure a clean
  build, a one-line change and `cargo tauri dev` rebuilds (`cargo build --timings`); look at
  what is missing: a compiler cache (sccache), a faster linker, splitting the big crate so a
  change rebuilds less, dev profile settings (`opt-level` and debug info per package), and
  caching in the release/installer build.

### Ideas not yet scheduled

Kept so they aren't lost; promote one into a numbered item when a real edit needs it.
Alternate full-length timelines for review; record a narration fix at the playhead (or import one
from AeroShoot); media bins and relink repair; copy scenes and clips between projects; export an
arbitrary range as a reference clip; cover frame / thumbnail selection; title and description
drafts; a reduced-motion style; safe zones as versioned delivery presets with phone-size
inspection; storyboard tiles keyed to music beats.

---

## External: the AeroShoot recorder

Not this repo, but they affect edits: write exact segment boundaries instead of overlapping by up
to a frame (O03), and remove the 10 ms system-audio gap at each segment boundary (O04). The
editor already tolerates both.

## Done

Most recent first, a line or two each. The details are in git history (commit messages and the
plans deleted once built), the design in [architecture.md](architecture.md), the choices in
[decisions.md](decisions.md).

- **25 Oct 2026** — <a id="cost-1"></a>**COST-1** review boards: `storyboard` `board: fine` (a
  frame a second, one JPEG a minute, 10×6 tiles of 148 px, about 1.0k image tokens by Claude's
  pixel count) or `coarse` (every 5 s, ten minutes an image, about 120 tokens a minute); amber
  edge on graphics, blue on zooms; 5 or 3 images a call, then "call again with fromUs". The
  rules point at `fine` for a stretch the scene storyboard leaves in doubt. Not measured yet:
  what an agent still catches on a board (needs an EVAL run).
- **25 Oct 2026** — <a id="lng-5"></a>**LNG-5** custom vocabulary: the key terms (Transcription
  settings, or `settings` `area: transcription` `keyterms`) bias Scribe as before, and now every
  transcript respells them afterwards: spacing and case for any transcriber ("chat gpt" →
  ChatGPT), and for Parakeet, runs of two or more words that sound like a term ("cloud code" →
  Claude Code); one word that only sounds alike ("codecs") is left. OpenRouter answering 400 to
  key terms transcribes without them instead of failing; terms are checked against Scribe's
  limit (5 words, 50 characters). The field now shows for Parakeet too.
- **25 Oct 2026** — **SND-3 (SFX starter set)** 17 CC0 sounds built into the app (4 whooshes, 2
  risers, 3 hits, 3 clicks, 3 pops, a ding, a toggle) from Kenney's packs and Freesound, each
  licence checked on its page (`src-tauri/assets/sounds/LICENSES.md`); written once to the app
  folder's `sounds/built-in`. The user's own go in `sounds/mine` with tags and a licence. A
  **Sounds** page in the Media panel (search, tag filters, play, + at the playhead, add your
  own); `library` `kind: sound` (`list` with `query`, `use` at `atUs`/`atWord`). Placing
  imports the sound once (role effects, its licence) onto a free effects track or a new
  "Sound effects" track, one undo step, never moving the video. `check` now also flags effects
  whose licence asks for credit. Music library still to come.
- **25 Oct 2026** — <a id="fix-8"></a>**FIX-8** cuts in a take short stay in the short: when
  the video plays the same take, `fillers` with `short` found and cut on the video's timeline.
  `fillers`, `fillers_review` and the AI review now read the short's kept words and cut its
  own timeline; `propose_cuts` takes `short` (a cuts batch remembers it, and the Agent panel
  shows "Show the short").
- **25 Oct 2026** — <a id="fix-9"></a>**FIX-9** online transcription is the user's recorded
  choice: Transcription settings → AI agents → Allow stamps the date (only the app sets it;
  `settings` refuses `agentsOnline` from an agent). The preset states it with that date, or,
  while off, tells the agent to ask once (or use Parakeet). Not yet tried with Codex.
- **25 Oct 2026** — <a id="fix-10"></a>**FIX-10** no Accept card for graphics: `short_footage`
  `op: add` and `library` style `bring_in_line` apply at once as one undo step, listed in the
  Agent panel with Undo; overlapping graphics are refused. The `under` and `restyle` batch kinds
  are gone; proposal cards stay for cuts, timeline edits and rules.
- **24 Oct 2026** — <a id="lay-2"></a>**LAY-2** layout keys: the shortcuts dialog has a
  Layouts tab (built-ins and favourites, no keys by default); a key puts that layout from the
  playhead to the next layout change. While playing it shows at once (LAY-1's swap), and every
  press until playback stops is one undo step: history steps carry a `group`, and a step of the
  top step's group joins it (`Join` line in the log, so it survives reopening). MCP `layouts op:
  switch` was already there.
- **24 Oct 2026** — <a id="ui-1"></a>**UI-1** thumbnails on video clips: small JPEGs along the
  clip (keyframe seek, one FFmpeg process each; images and the native backend decode), kept in
  `cache/thumbs/<asset>/` by file, time and height; times rounded to a step that suits the zoom,
  so small zooms and moves reuse them. Only when the video lane is 42 px or taller, only for the
  drawn part of the timeline, at most 64 a call, one call at a time; pictures scrolled away
  before their turn are never asked for. Graphics show none.
- **24 Oct 2026** — <a id="ui-2"></a>**UI-2** Basic and Advanced settings: File → Advanced
  settings (off by default). In Basic, each section keeps its fine-tuning behind a "More
  settings" link at its foot (per section, not remembered); in Advanced everything shows and the
  Advanced parts of Export, Jump cuts and Transcript settings start open. Under More settings:
  cursor size, room around the window, crop (Screen); per-clip camera framing (Camera); exact
  bubble size, roundness, screen fit, overlapping card, section transition, move length
  (Layouts); noise and duck amounts, mouth-click strength, music and loudness levels (Audio);
  distance from edge, words at once, lines (Captions); every auto-zoom setting but the amount,
  the follow speed (Zoom); graphics look sliders, caption weight, emphasis colour, outline, box,
  the frame's corners, shadows and camera border, and 9:16 graphics sliders (Style). The rest
  stays in Basic.
- **24 Oct 2026** — <a id="rev-1"></a>**REV-1** listen to the joins: a **Joins** panel (behind
  the Inspector) steps through cuts, plays ±2 s around each and marks keep or fix with a note;
  `joins` (list, find, add, clear; `agent/joins.json`) lets the agent hand over cuts and read the
  marks. Joins are held by recording times, so they move with later edits and show "changed"
  once fixed. `find` scores retakes (words cut out), words cut into or ending at a cut, unsure
  words, mid-sentence cuts and `check`'s blips. `playhead` `op: show` moves the user's preview
  (`playhead_get` is now `playhead`, behind `more`). The export guide asks for it before export.
- **24 Oct 2026** — <a id="ftg-1"></a>**FTG-1** `source_search` (behind `more`) and a **Search**
  panel (behind the transcript): every transcript, cut parts and imported takes too; whole
  words in order, the last a prefix, long words one letter off, Greek accents folded; each hit
  says plays (and where), cut (and the join over it) or not placed.
- **24 Oct 2026** — <a id="snd-1"></a>**SND-1** clips' own gain and fades (`Clip.mix`,
  `setSound` edit, the clip inspector; split keeps fades at the outer ends); **music** and
  **effects** roles; `musicUnderSpeechDb` keeps music lanes that far under the speech, measured
  (Audio page); a licence on imported files (`media_import` `licences`, the clip inspector), and
  `check` warns about music whose licence needs credit or isn't known.
- **24 Oct 2026** — <a id="agt-3"></a>**AGT-3** `library` `kind: preset` `op: propose_rule`
  (`rule` + the `example` that taught it) makes a **Rule** batch in the Agent panel; accepting
  adds a bullet to the preset's `rules.md` (a built-in gets its "My" copy); autopilot never
  accepts rules, and the rules guide says when to propose one and to name the rule that shaped a
  proposal. Designed from the creator-workflows summary, not the three skills' originals.
- **24 Oct 2026** — <a id="agt-4"></a>**AGT-4** the diagnose part, as `check` findings: linked
  sound and picture out of sync, and clips running past their media. Not built, on purpose:
  resuming a stopped job, a state screenshot (`storyboard`, `frame_render` and `timeline_view`
  cover it) and per-stage time budgets (the agent's own pacing).
- **24 Oct 2026** — <a id="lay-1"></a>**LAY-1** look-only swap: an edit that changes only the
  layout sections bumps a look generation; the engine swaps them into the running scene, keeping
  the mixer, decoders and sound playing. `perf_look_swap` on the 7:10 tutorial with 30 sections:
  the frame after a change 7.9 ms p50 swapped vs 12.1 ms rebuilt; playing with a change every
  second p50 11.5, p95 17.7, one 363 ms frame. Drag-preview on the front end is not done.
- **24 Oct 2026** — **Shorts and clips told apart for agents.** `clips` (list by default,
  create, find, range, rename, remove) replaces `short_create` and the old `shorts` find; `shorts`
  is now only for shorts made from a take (list by default, rename, remove); each refuses the
  other kind and names its tool. Finding clips (agent and the Clips tab's Find with AI) only
  adds clips: it used to replace the whole list, take shorts included. New clips and new
  shorts follow their own presets (`clipsPreset` beside `shortsPreset`; `library` `op: use`
  `for: shorts|clips`) and copy the look of their own kind. `export` takes `short: "shorts"` or
  `"clips"`. Methods, rules and the export guide say which is which.
- **24 Oct 2026** — <a id="sht-2"></a>**SHT-2** a short's caption splits, joins, hidden words and
  emphasis are its own (`Short.caption_marks`, over the video's; undo steps), text fixes stay
  shared; the Shorts tab counts them and can drop them ("Use the video's"); MCP `captions` gains
  `split`, `join` and `hide`. <a id="fix-7"></a>**FIX-7** dropping on a new track is one undo
  step (`onNewTrack` edit); undo names say "Cut 0.04 s" and leave out zooms a cut only moved, and
  the blip fix reads "Removed 3 blips"; `export` to a missing folder says it did not start.

- **24 Oct 2026** — **Everything built so far, tried** in the dev app and EVAL-1 run 4
  (tutorial 18 calls, $0.271, −28 %; a short from a take 13 calls, $0.122). Fixed from it:
  UI-5's drag row moving the tracks under the pointer; on a take short, `mouth_noises` using
  the take's voice, `fillers` taking `short`, an answer when every noise is next to words, and
  the shorts tools sending the agent to the Shorts preset. Docs cleaned up (built plans folded
  in and deleted, fixed gaps removed).
- **24 Oct 2026** — <a id="gfx-7"></a>**GFX-7** graphics ready before export: every render is
  at the export's frame rate, export renders queue in the background (2 Chrome workers, below
  normal priority, fast capture), so an export of unchanged graphics starts at once (tried:
  frames after about 15 s).
- **24 Oct 2026** — **AGT-4** stopping a long call (the client cancels, hangs up, or the user
  presses Stop in the Agent panel); **COST-2** 8 more tools behind `more` (38 → 30, 60,100 →
  41,240 characters; measured: Claude Code with tool search pays only the names); **AI-1** the
  guides trimmed (tutorial −10 %, `graphics` −45 %).
- **24 Oct 2026** — **The fixes from George's 23 Oct edit**:
  <a id="fix-4"></a>**FIX-4** dead air before the first word, after the last and around
  recording pauses is cut; <a id="fix-5"></a>**FIX-5** a filler review runs once and is
  reused; <a id="agt-5"></a>**AGT-5** the preset's choices run without a question;
  <a id="gfx-6"></a>**GFX-6** graphics proposed as a table; <a id="cost-4"></a>**COST-4**
  `frame_render` sheets of moments or a graphic; <a id="ui-7"></a>**UI-7** a streamed export
  log; <a id="ui-6"></a>**UI-6** Medium / High / Ultra, Advanced, **Remove all blips**;
  <a id="ui-4"></a>**UI-4** the transcript review panel runs Review on its button;
  <a id="ui-5"></a>**UI-5** add-track buttons, and a drop row only while dragging;
  <a id="fix-6"></a>**FIX-6** the styles and presets folders open before anything was saved.
- **24 Oct 2026** — **A second audit's six findings fixed** (A01-A06): export preferences saved
  to the wrong project, two WAV saves sharing a temp file, a range selection carried between
  shorts, waveform reads restarted on every edit, a case-variant export path that could replace
  a source file (P1), and exports re-reading graphic sources each frame (renders now pinned).
- **23 Oct 2026** — <a id="org-1"></a><a id="sty-1"></a><a id="ui-3"></a>**ORG-1 presets and styles**, all four phases:
  Style (brand, graphics look, caption look, frame, 9:16 variant; Light and Dark built in),
  Preset (settings by part, cleanup levels, export settings, the method, the user's rules, the
  choices; Tutorial, Shorts, Clips built in), projects storing only their changes, the Preset
  menu and the Style page, one `library` tool for agents. Replaced brand kits, saved looks, the
  Templates menu and saved answers. Choices: [decisions](decisions.md#presets-and-styles-as-built-23-oct).
- **23 Oct 2026** — <a id="dlv-1"></a>**DLV-1** subtitles (SRT, VTT), the transcript (text,
  Markdown) and the sound (WAV) as files, of the video or a short, from the caption cues.
- **23 Oct 2026** — **AGT-4 readiness**: `status` with no project says what this computer is
  ready for (keys, models, GPU pack, renderer, FFmpeg, the skill per agent).
- **23 Oct 2026** — <a id="safe-2"></a>**SAFE-2** history on disk (no step limit up to 2 GB),
  an agent's run undone as one step, checkpoints of the edit, graphic files and caption marks
  (one before every agent run). Checked with a real agent in EVAL-1 run 3.
- **23 Oct 2026** — <a id="cap-1"></a>**CAP-1** captions as typography: the style's font (file, installed or
  Inter Display; .woff/.woff2 too), one emphasis word per caption in the accent, pop / karaoke /
  static, captions giving way to full-frame graphics.
- **23 Oct 2026** — **GFX-5 leftovers and SHT-4's footage sound**: saved looks (now styles),
  the shorts look multiples, **Bring in line** by rule; a take's footage sound on its own track,
  always ducked under the voice.
- **23 Oct 2026** — <a id="sht-7"></a><a id="sht-7-done"></a>**SHT-7** a split graphic knows its box and renders
  with its portrait rules; on a take short the camera keeps its own part.
- **23 Oct 2026** — <a id="sht-4"></a><a id="sht-3"></a>**SHT-4** graphic modes in shorts (full, split, over the face, picked
  before the render) and footage at its own speed ([Q-SHORT-GFX](decisions.md#q-short-gfx)), the take picker, the
  footage sound control. **SHT-3** the take is cut before footage and graphics, and the app warns
  when graphics go on an uncut take.
- **23 Oct 2026** — **A smaller agent context and fewer calls**: 18 rarely needed tools behind
  `more`, a shorter automatic guide, `guard: true` instead of dry runs in automatic mode,
  `graphic_add` with `items`, graphics on a take short movable (tutorial 34 → 20 calls).
- **23 Oct 2026** — <a id="eval-1"></a>**EVAL-1 baseline** ([plans/evaluation.md](plans/evaluation.md)):
  a real agent edited the 7-min tutorial for $0.41 in 34 calls and made a short from a take
  for about $0.30.
- **23 Oct 2026** — <a id="sht-6b"></a>**SHT-6b** the agent's picks under a take are one batch;
  the Shorts tab's picker places video, graphics and files under a take by hand.
  <a id="sht-6"></a>**SHT-6** graphics on a take short's own timeline, copies of the video's
  graphics, B-roll from imported files. <a id="sht-5"></a>**SHT-5** Shorts and Clips as two
  tabs over one model.
- **23 Oct 2026** — <a id="gfx-2"></a>**GFX-2** GSAP in graphics (3.15.0, fetched once,
  sha256-pinned, CDN links rewritten to the local file); the tool list's schemas trimmed 14 %.
- **22 Oct 2026** — **GFX-5** one look for every graphic in a video (preset, five controls,
  tokens) and the templates rewritten in George's light library style.
- **22 Oct 2026** — **Quick wins**: <a id="fx-4"></a>**FX-4** 16 ms crossfades at cuts;
  loudness reaches its target (-14 LUFS by default); **GFX-1** graphics at the export's frame
  rate and 2x density above 1080p; small words said twice and broken-off words cut by the rules.
- **22 Oct 2026** — **An audit's findings fixed**: the shorts preview freeze (a deadlock with
  the window thread; the rule is in [architecture](architecture.md#threads-and-locks-the-window-thread-must-never-wait-on-us)),
  stale short edits, brand-kit saves and renames, all-or-nothing `graphic_set` and
  `transcript_fix`, and a dozen smaller ones.
- **22 Oct 2026** — <a id="perf-1"></a>**PERF-1** the transcript panel indexed and windowed
  (9,500 words: reload 583 → 310 ms, DOM 12,344 → 517 nodes). <a id="perf-2"></a>**PERF-2**
  lazy loading (start-up script 966 → 236 KB).
- **21 Oct 2026** — <a id="agt-2"></a>**AGT-2** fast or thorough review and an honest report
  (Done / Looked at / Not heard / Approved by you / Still open).
  <a id="agt-1"></a>**AGT-1** agent parity for cleaning up and correcting (zooms, graphic
  moves, chapters, shorts, word fixes, roles, export cancel); calls bound to another project
  refused under the command lock.
- **21 Oct 2026** — <a id="fix-1"></a>**FIX-1** the render queue keeps renders of one graphic
  at two lengths; <a id="fix-2"></a>**FIX-2** the skill's on/off defaults; <a id="fix-3"></a>**FIX-3**
  honest read-only and destructive hints, a real `propose_edit` schema;
  <a id="safe-1"></a>**SAFE-1** agent calls bound to their project; <a id="dep-1"></a>**DEP-1**
  Rust 1.92.0, Node 22, FFmpeg pinned with checksums, `Cargo.lock` committed.
- **21 Oct 2026** — Layouts: base layout + timed sections, favourites, pick rules, face framing
  ([plan](plans/layouts.md)). Local vs cloud filler review measured
  ([eval](plans/local-review-eval.md)). Clip speed 0.25×–8×. Docs consolidated.
- **20 Oct 2026** — Greek (online), Parakeet Unified EN + Verbatim, the CUDA 13 GPU pack, local
  Qwen3.5 4B review on Vulkan; sound effects off by default; fillers use rules + AI by default.
- **18 Oct 2026** — Shorts as a workspace tab; take shorts (`short_from_take`, `short_footage`).
- **17 Oct 2026** — Face framing (YuNet); fast switching between shorts.
- **13–16 Oct 2026** — AI readiness phases 0–3: MCP bridge, proposals and Agent panel, app actions
  and CLI, skill and templates, `check`, shorts scenes, graphics v1, library + gate.
- **4–5 Oct 2026** — Premiere-style tracks (one sequence model, clocks); performance pass R01–R20.

## Old numbers → new IDs

For anything (memory notes, commit messages, old chats) that still says "task N".

| Old | New | Old | New |
|---|---|---|---|
| #1 More layouts | Done; follow-ups LAY-1…LAY-4 | #10 Number keys | LAY-2 |
| #2 More languages | Removed (was LNG-1, LNG-2) | #11 Auto layouts | LAY-3 |
| #3 Measure local review | Done; LNG-4 | #12 Transcription models | Removed (was LNG-3) |
| #4 B-roll | FTG-2, FTG-6 | #13 Version history | SAFE-2 |
| #5 Music and SFX | SND-1, SND-2 | #14 Source search | FTG-1, FTG-3 |
| #6 Templates | UI-3 | #15 NLE handoff | DLV-3 |
| #7 Clip thumbnails | UI-1 | #16 MCP discovery | AGT-4 |
| #8 Simpler UI | UI-2 | #17 Project binding | SAFE-1 |
| #9 Background removal | FX-1 | Known issues F05 / F06 / render queue | PERF-1 / PERF-2 / FIX-1 |
