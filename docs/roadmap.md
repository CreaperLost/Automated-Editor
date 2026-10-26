# Roadmap

What is still to do, in order. What shipped is in [done.md](done.md); each item's gap is in
[gaps.md](gaps.md), the decisions that shape it in [decisions.md](decisions.md).

**Sizes:** S ≈ up to two days, M ≈ up to a week, L = more than a week.
**Rules for this file:** take items top-down unless George says otherwise. When an item starts,
link its plan (in `plans/`); when it ships, add it to [done.md](done.md) with the date and its
ID as an anchor, and delete it here. Keep IDs stable; never renumber or reuse one.

## Where things stand (26 Oct 2026)

**Milestones (decided 21 Oct):** (1) a great long-form tutorial, (2) tutorial shorts, (3) gaming
shorts and gaming videos. Gaming work is not scheduled yet.

Milestone 1 is close: everything built up to 24 Oct was tried in the dev app and in EVAL-1 run
4, and 25–26 Oct added music, screen effects, take matching and cleaner filler cuts. What it
still lacks is **graphics an agent makes well**, which is why Next is STY-2, then GFX-8. Shorts
from a take (milestone 2's first half) are built; idea-first clips (SHT-1) follow.

## Standing preferences

<a id="ai-budget"></a><a id="ai-1"></a>
- **AI-1 · An edit reads about 100k tokens at most** (George, 24 Oct). What an agent reads in a
  whole edit (the tool list, the guides and every tool answer, images included) should stay at
  about 100k tokens or less: a preference to steer by, not a limit any test enforces; our
  source code doesn't count. Correct tools, arguments and workflows matter more than size
  (24 Oct, later). Measured by [plans/eval/tokens.py](plans/eval/tokens.py), run by
  [plans/eval/run.sh](plans/eval/run.sh): EVAL-1 run 4 ended the tutorial edit at 36.8k, with
  graphics at 44.6k, a short from a take after them at 53.6k. Biggest answers: `preset` 3.8k,
  `storyboard` 3.6k, `transcript_get` 2.1k, `graphics` 2.1k. Next levers: transcripts read once,
  and images.
- **Rerun EVAL-1** ([plans/evaluation.md](plans/evaluation.md)) after each cost or quality
  change. Don't edit Rust while a run is going.

---

## Next, in order

<a id="sty-2"></a>
### 1. STY-2 · Styles from references — L, plan: [plans/styles.md](plans/styles.md)

Bad graphics aren't a shorts-only problem, and a style today holds only look settings, so agents
fall back on stock templates (George, 25 Oct). Merges GFX-5's AI-proposed styles
([research](research/ai-styles.md)), the old STY-2 and GFX-8's worked examples. A style gains
motion rules, the kinds of graphics it uses (with their 9:16 placement), 2–3 examples linked from
the library, its references and the agent's breakdown of them, look rules, a richer shorts
variant and a small sound set; fonts must resolve or ship with it. `library` `kind: style`
`op: propose` makes one from references: breakdown first, stills to narrow, one clip in place for
the pick. First test: a "my library" long-form style from George's own graphics.

<a id="gfx-8"></a>
### 2. GFX-8 · Graphics an agent makes well — L, plan: [plans/graphics-quality.md](plans/graphics-quality.md)

On top of STY-2. The 25 Oct test (one short take, three agents) gave generic text cards every
time: stock templates, approval on a written plan, no sample, uninstalled fonts, cards over the
face, captions cut by splits. To build: picking per moment what to show (text cards and
demonstrations are equals), one sample clip before the set, the plan always shown as a table,
checks the agent runs (face, captions, fonts), honest style knobs. George's decisions are in the
plan; the same test's bugs (FIX-8 to FIX-10) are fixed.

<a id="sht-1"></a>
### 3. SHT-1 · Idea-first clips — L, plan: [plans/shorts.md](plans/shorts.md)

A clip is an idea plus pieces gathered from anywhere in the video, found with source search
(FTG-1, built). **Decided 22 Oct:** Shorts and Clips are two tabs that share their parts.
**Shorts** start from a take recorded outside the app (built: SHT-2 to SHT-7); **Clips** cut the
best moments out of the long video, and SHT-1 makes them idea-first.

---

## Waiting on George

Small checks only George can do; nothing else is blocked on them.

- <a id="fix-9-run"></a>**FIX-9 with Codex:** turn on Transcription settings → AI agents →
  Allow, then let a Codex session transcribe a project ([done.md](done.md#fix-9)).
- <a id="cost-2"></a>**COST-2 · Tools on demand: the Claude app.** Say whether the Claude app
  sends every tool upfront. Claude Code (tool search, about 1.7k tokens for the names) and Codex
  (about 0.9k a turn; schemas load when used, measured 26 Oct) need nothing. Only if the Claude
  app sends the whole list: change the list to the open project's preset
  (`notifications/tools/list_changed`, the shorts' tools only in a shorts edit) rather than put
  more behind `more`.
- **Listening:** the 38 built-in music tracks ([SND-3](done.md#snd-3); Komiku's moods are guessed
  from titles), and take matching on real pickups ([AUTO-2](done.md#auto-2); its EQ strength,
  60 % of the difference and 4 dB at most, is a first guess). Feedback whenever, no rush.
- **A longer Greek recording** for the review eval: EL-B is 13 seconds, so Greek still rests
  mostly on the scripted EL-A ([LNG-4](done.md#lng-4),
  [local-review-eval](plans/local-review-eval.md)).

---

## Later, by area

Order within an area is the suggested order.

### Direction: plan, style, method

<a id="plan-1"></a>
- **PLAN-1 · Plan file and style trial (M).** The agent writes and keeps a plan in the project
  (a Markdown file it reads back: per moment the purpose, treatment and reason, and what the
  user locked), so the context survives between turns and runs without new storage or UI. Then
  a **style trial**: build 2–3 beats on a 5–10 s range, approve the look, then scale.
  (Graphics already keep their word: cuts before a graphic move it along and no cut goes
  through one, `sequence/edit.rs`.)
<a id="auto-1"></a>
- **AUTO-1 · Run a preset on request after opening (S).** After opening a project the user picks
  a preset (Tutorial, Shorts, Clips…) and its method runs up to its first question, with only
  free local steps unless the user confirms. Replaces folder watch, which was turned down.
  Decision: [Q-WATCH](decisions.md#q-watch).
<a id="auto-3"></a>
- **AUTO-3 · "Save this workflow" (M).** From an approved edit: instructions, defaults,
  exceptions and checks as a named, versioned workflow shown as a readable summary first. Proven
  only when reused on a second tutorial with different footage.
<a id="loop-1"></a>
- **LOOP-1 · The user and the agent improve the app together (L).** A loop where each edit makes
  the next one better: the app, the bundled material (presets, styles, graphic templates) and
  the taste. Records what the user changed after the agent (proposals accepted or rejected,
  cuts restored, graphics redone) and turns patterns into proposed changes to a preset's rules
  ([AGT-3](done.md#agt-3)), graphic templates and styles, each accepted by the user.
  Data-driven, never free-text rules learned on the side (the 17 Oct condition for bringing
  learning back).

### Graphics

<a id="gfx-3"></a>
- **GFX-3 · Templates for real tutorial jobs (M + design).** Word-timed emphasis (each word at
  its own word id, re-timed after cuts); highlight a real UI target; step sequence; before/after;
  a lower-half overlay prototype. Purpose tags and search in the library. 8–12 total, all
  passing the gate with real English and Greek copy. Likely shaped by STY-2 and GFX-8.
<a id="gfx-4"></a>
- **GFX-4 · Graphic variants (M).** Independent variant folders with source snapshots;
  Create / Revise / Compare / Use; "Use this version" is one undo step; revision scope (element /
  graphic / scene) so a colour fix never regenerates a sequence.

### Sound

<a id="snd-2"></a>
- **SND-2 · Cues at graphic events (M/L).** Templates declare event markers (card lands, counter
  completes); a small procedural palette (tap, tick, pop, chime, airy whoosh, impact) plus the
  sound library; a deterministic cue compiler (`cueStart = visualPeak − soundPeakOffset`, density
  limits, ducking); the agent picks which events deserve sound; the style's sound set (STY-2)
  says how it sounds.

### Footage

<a id="ftg-2"></a>
- **FTG-2 · Selects and B-roll from own footage (M).** Named source ranges with notes; a reviewed
  placement batch: video-only streams, a free overlay track, narration untouched, word-anchored
  so later cuts re-resolve it.
<a id="ftg-3"></a>
- **FTG-3 · Scene descriptions (M).** Agent-written descriptions and tags from sparse contact
  sheets, cached against source fingerprints; OCR for UI labels if keyword search misses.
<a id="ftg-4"></a>
- **FTG-4 · Ordered take and script-guided assembly (M).** Natural ordering of take names
  (`1.2`, `1.10`), missing/duplicate takes shown; match a supplied outline to source
  transcripts, alternatives shown, one proposal batch.
<a id="ftg-6"></a>
- **FTG-6 · Stock B-roll providers (M).** Pexels / Pixabay with the user's key, licence recorded,
  download only on accept.
<a id="ftg-5"></a>
- **FTG-5 · Import an outside transcript (S).** Deepgram / AssemblyAI / Rev JSON → our word model.
  Not needed (21 Oct); do it only if it stays this small.

### Layouts — plan: [plans/layouts.md](plans/layouts.md)

<a id="lay-3"></a>
- **LAY-3 · Auto layouts with limits (M).** Tella-style rules on top of `pick`: a few changes per
  minute at most, length windows (screen 2–8 s, camera 2–5 s, punch-in 1.5–4 s), camera on the
  side the face looks away from; the app proposes, the agent reviews with `list` + storyboard.
<a id="lay-4"></a>
- **LAY-4 · Presenter cut-out layout (M).** Needs FX-1's matte.

### Camera and colour

<a id="fx-1"></a>
- **FX-1 · Webcam background removal, blur, replacement (L).** A segmentation matte computed once
  per recording and cached like face tracks, composited in wgpu. Also enables text drawn behind
  the presenter, and LAY-4.
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
<a id="dlv-4"></a>
- **DLV-4 · Translated subtitles (M).** Greek ↔ English cues with stable source anchors and their
  own revision history; glossary for product names. Dubbing stays out (YouTube auto-dubs).
<a id="dlv-3"></a>
- **DLV-3 · Graphic export with alpha, then a constrained NLE handoff (M → L).** ProRes 4444 /
  WebM alpha for one graphic; then a cuts-only timeline (FCP7 XML or OTIO) tested in the
  destination editor with a report of what was flattened.
<a id="dlv-5"></a>
- **DLV-5 · Real reformat 16:9 → 9:16 (L).** One operation that keeps UI targets, cursor
  relationships and reading time, not just the canvas size.

### Agent cost and quality — 99 % of the quality for 10 % of the price

Every item here is measured against [EVAL-1](done.md#eval-1): keep a cheaper path only when the
edit stays as good. (COST-2's last check is under [Waiting on George](#cost-2).)

<a id="cost-3"></a>
- **COST-3 · Cheaper reviewers (M).** Hand narrow judgement jobs (looking at a board, filler
  windows, chapter titles, checking a graphic) to a smaller model or a lower reasoning effort,
  through a subagent or the app's own OpenRouter pass with a cheap model; the main agent keeps
  the decisions.

### Verification

<a id="ver-1"></a>
- **VER-1 · macOS (M), later.** The verification done on Windows (24–25 Oct: the whole bridge
  from create to export, three real Codex sessions;
  [product.md](product.md#what-is-verified-and-what-is-not)) again on macOS, when the macOS
  version is worked on (George, 26 Oct: not now).

### Code health and performance

<a id="qual-1"></a>
- **QUAL-1 · Code reuse check (M).** The same logic is often written again instead of shared,
  which costs time and causes bugs fixed in one copy only (asked for 22 Oct). Sweep the Rust
  passes, the MCP actions vs the UI commands, and the front-end panels and dialogs; move repeats
  into shared helpers with tests; note the patterns to reuse in `architecture.md`. Seen so far:
  the same Tauri event-listener effect in `App.tsx` and several panels; line-ending-safe edits;
  Clips and Shorts sharing one set of short panels.
<a id="qual-2"></a>
- **QUAL-2 · Review the tests (M), not urgent.** Drop tests that guard behaviour that is gone
  (old project formats, removed alignment, superseded passes), merge near-duplicates, keep the
  ones that pin decisions; note how long the suite takes before and after (asked for 23 Oct).
<a id="perf-3"></a>
- **PERF-3 · Dense timelines and the JPEG fallback (M).** Draw sub-pixel clip runs as one strip
  per track at full view (O01); two alternating encoder threads for the non-wgpu preview path
  (O02).
<a id="qual-4"></a>
- **QUAL-4 · Faster builds, round two (M), only if builds feel slow again.** After
  [QUAL-3](done.md#qual-3) a one-file change builds in 15–28 s. Left: split the crate (a
  refactor); sccache helps only clean builds and the extra target folders; a whole installer
  build with `rust-lld` not yet run (a plain release build links and runs).
<a id="dep-2"></a>
- **DEP-2 · Recurring dependency review (S each).** Follow the cadence in
  [dependencies.md](dependencies.md): read changelogs, update on a branch, run tests, `perf.rs`
  and the parity probes, record the result. Candidates: React 18 → 19, Vite 5 → current,
  Tailwind 3 → 4, `ort` leaving release candidates, HyperFrames past 0.8.162.

### Ideas not yet scheduled

Kept so they aren't lost; promote one into a numbered item when a real edit needs it.
Alternate full-length timelines for review; record a narration fix at the playhead (or import one
from AeroShoot); media bins and relink repair; copy scenes and clips between projects; export an
arbitrary range as a reference clip; cover frame / thumbnail selection; title and description
drafts; a reduced-motion style; safe zones as versioned delivery presets with phone-size
inspection; storyboard tiles keyed to music beats.

### External: the AeroShoot recorder

Not this repo, but they affect edits: write exact segment boundaries instead of overlapping by up
to a frame (O03), and remove the 10 ms system-audio gap at each segment boundary (O04). The
editor already tolerates both.

---

## Dropped or merged

Reasons are in [decisions.md](decisions.md#not-doing).

| ID | What | When | Why |
|---|---|---|---|
| <a id="gfx-5"></a>GFX-5 | Styles proposed by the agent | 25 Oct | Merged into [STY-2](#sty-2); the look, shorts look and bring-in-line [shipped](done.md) |
| NOTE-1 | Anchored notes, "Ask AI about this" | 24 Oct | The chat, the agent's history and "the graphic titled X" already do it |
| — | Finding secrets on screen automatically | 26 Oct | The agent covers what it sees in frames; the user draws the rest |
| LNG-1, LNG-2, LNG-3 | Greek word aligner, more languages, more transcription models | 23 Oct | Not needed |
| — | Folder watch (start editing when a recording lands) | 21 Oct | Replaced by [AUTO-1](#auto-1) |
