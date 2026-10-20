# Shorts redesign: plan v1 (2026-10-17)

Feedback that started this: switching between shorts is very slow; the camera framing is off
(the opening shows half a face); captions can't be edited in a short; and a short is just one
continuous stretch of the video. What we want instead: **start from an idea, gather every part
of the video that serves it, cut it down to what is essential, then dress it with motion
graphics.**

## Where it stands today (from the code)

| Area | Today | Why it hurts |
|---|---|---|
| What a short is | `Short` has one `source_start_us..source_end_us` range (`shorts.rs`). It follows the video's edits until it is edited on its own (`ShortEdit` = a copied sequence) | Built for "cut a clip out", not "build from many places". The own-edit copy also stops following the video's cleanup |
| Picking | `ai/shorts.rs` and the MCP `shorts.md` template ask for contiguous `from..to` line ranges, 20–60 s | The AI can't combine a setup from minute 3 with the payoff from minute 17 |
| Camera framing | Camera layers use `cover_uv` (`render/mod.rs:1800`): a fixed **centre** crop. A 16:9 webcam cut to 9:16 keeps only the middle 32% of its width | Anyone sitting off-centre is cut in half, worst in camera-only scenes, which `short_scenes::suggest` puts on the opening line |
| Captions | Text comes from the shared transcript; the Studio only has style sliders and says "edit caption text on the timeline below" | No place to see and fix caption text, line breaks or emphasis inside a short |
| Switching | Each switch runs `project_short_view` (clones the whole `OpenedProject` summary and builds the short's timeline), replaces the store's project, refetches `project_current`, then `playback_focus_short` re-points the engine | Several full round trips and full re-renders per click; not measured yet |
| Graphics | Graphic slots exist per short (`graphic_slots`, `GraphicSlot`) | Placing them isn't part of a guided flow |

## The new flow

Shorts Studio becomes four steps for the selected short, shown as tabs over the same preview:

1. **Idea**: a title and a one-line premise/hook ("Why your exports look washed out").
2. **Gather**: search the whole video's transcript and add passages to the short, in any order.
3. **Tighten**: the short's own transcript; strike words to cut them, fix caption text, set
   line breaks and emphasis. Scenes (camera / screen / split) are adjusted here too.
4. **Dress**: graphics in slots, caption style, background, then export.

The agent (MCP) works the same steps: proposes ideas with their passages, assembles, tightens,
then authors graphics through HyperFrames.

---

## Phase A: switching speed (small, do first)

**Measure before changing.** Add timing spans to the switch path and record a baseline on the
real test project (see `perf.rs` and the dev-app driving notes):

- `project_short_view` time and serialised size;
- front-end time from IPC result to the store applied and the first paint;
- `playback_focus_short` until the first frame is presented;
- the extra `project_current` refetch and any caption reloads it triggers.

Likely fixes, in order of expected payoff (confirm with the numbers):

1. **Slim short view.** Return only what changes per short (sequence, duration, zooms, webcam
   focus, short id) instead of the whole `OpenedProject`; the store merges it over the main
   project it already holds.
2. **Cache views** by `(revision, short id)` in the reader; switching back is then free.
3. **Keep the engine warm.** Shorts of one recording share decoders; switching should seek,
   not rebuild. Prefetch the first frame of the shorts next to the selected one.
4. **Drop the duplicate fetch.** `project_current` runs on every revision; only refetch when the
   main project changed, not when the view did.
5. Don't re-render the whole Studio on selection: memoise the shorts list and inspector.

Target: under 150 ms from click to the new short's first frame, and no visible flash.

## Phase B: face-aware camera framing

**Detect.** Run a small face detector (YuNet, ~230 KB ONNX, through the `ort` crate the
aligner already uses, CPU is fine) on the webcam recording at ~4 frames per second, in the
background after import. Store a **face track** per camera asset in source time: the main face's
box, confidence, and gaps where none was found. A 10-minute recording is ~2,400 detections,
a few seconds of CPU.

**Smooth like a camera operator, not a tracker.** Keep the crop still while the face stays in
a centre dead zone; when it leaves, ease to the new position over ~0.6 s. Low-pass the box
size so leaning in doesn't pump the framing. No face: hold the last position, else centre.

**Frame.** Replace the centre `cover_uv` for camera layers with `focus_uv(face, dest)`: the crop
window centred on the face horizontally, with the eyes about a third from the top, clamped
inside the frame. One function used by preview and export (parity holds), for the camera crops
in shorts: the short's camera part and camera-only scenes. The main video keeps its centre crop
for now (decision 4). No separate camera asset means no face track (decision 6).

**Control.** A per-short "Camera framing: Auto / Manual" with drag-to-reframe on the preview
for manual, plus an offset that nudges auto. The MCP `status` reports whether a face track
exists.

Tests: unit tests for smoothing and `focus_uv` clamping; a parity probe that renders a frame
with an off-centre face in preview and export and compares crops.

## Phase C: the idea-first short (the big one)

### Data model

No migration or compatibility with old projects (standing rule): replace the model outright.

```rust
pub struct Short {
    pub id: String,
    pub title: String,
    pub idea: String,            // the premise / hook, also shown to the agent
    pub pieces: Vec<Piece>,      // in play order
    pub cuts: Vec<WordRange>,    // words removed inside pieces (Tighten)
    pub layout: ShortLayout,     // unchanged: split, scenes, captions, graphic slots, framing
    pub reason: String,
}

pub struct Piece {
    pub id: String,
    pub media: Option<String>,   // recording clock, as today
    pub source_start_us: u64,    // snapped to word edges
    pub source_end_us: u64,
}
```

**The short's timeline** is built, not stored: for each piece in order, the parts of that
source range the main edit keeps (so filler, silence and mouth-noise cleanup carries over), minus
the short's own `cuts`, joined end to end. Zooms, webcam focus, scenes and graphics are anchored
in source time or to words, so they land in the right place whatever the order.

A short only uses what the main edit keeps (decision 2): a piece over a passage the video cut
plays only the kept parts. Gather shows cut passages greyed out and doesn't let them be added.

`ShortEdit` (the copied sequence) goes away; extra tracks a short needs (graphics, music) live
in a small per-short overlay sequence anchored to words.

### Gather (UI)

- A search box over the **whole video's transcript** (text match, word-boundary, fuzzy on
  typos), results as passages with a few words of context and their time. Click plays it in
  the preview; "Add" appends it as a piece; drag on the transcript to widen it to whole
  sentences.
- The piece list: reorder by drag, remove, play one piece.
- Meaning-based search ("everything about colour grading") comes from the agent over MCP; the app
  ships no model (AI-readiness rule). The old OpenAI/OpenRouter pass in `ai/shorts.rs` is
  rewritten to return ideas with pieces too.

### Tighten (UI)

The short's transcript (shared component with the main transcript panel), with word
strike-through to cut, sentence-level "drop", and a running length readout against a target
(default 30–60 s). Scene strips sit under it, as they do now on the timeline.

### MCP

- `short_create` takes `title`, `idea` and `pieces: [{fromWord, toWord}]`.
- `short_pieces` with `op: add | remove | move | set`; `short_cut` with word ranges.
- `transcript_search` (text) so the agent can check its picks cheaply.
- `shorts.md` template rewritten: (1) read the transcript, (2) propose 3–5 **ideas**, each with its
  supporting passages from anywhere in the video and the hook line first, (3) user picks, (4)
  assemble, (5) tighten to the essential (drop setup, repeats, asides), (6) graphics through
  HyperFrames into the short's slots, (7) export waits for the user.

### Checks

`shorts::validate` gains: pieces non-empty, word-aligned, no piece overlapping another in the
same short, total length within limits, cuts inside pieces. Unit tests for building the
timeline from pieces through a main edit (kept parts, raw pieces, cuts, reordering), and that
scenes and zooms map onto reordered pieces.

## Phase D: caption editing in shorts

Lives in the Tighten step, on the same transcript view.

- **Text fixes** edit the shared transcript word (fixing "Pyhton" fixes it in the video and in
  every short). Said plainly in the UI.
- **Per-short caption shaping**, stored on the short: forced line/caption breaks
  (`caption_breaks: Vec<word id>`), emphasised words (`emphasis: Vec<word id>`, drawn in the
  brand accent), and hidden words (shown in audio, left out of captions).
- The view groups words into the caption chunks as they will appear, so what you see is what
  renders.
- MCP: `short_captions` with the same three operations.

## Phase E: graphics as the last step

The Dress tab lists the short's graphic slots (full frame, camera part, screen part, over
screen), lets the user drop a library graphic or brand-kit template on a word, and shows the
agent's proposed graphics as a review batch, as in the main editor. Graphics hang off words, so
Tighten cuts never knock them out of place (graphics v1 rules: never chopped, slots not safe
zones).

---

## Order and parallel work

| Phase | Size | Depends on | Can run alongside |
|---|---|---|---|
| A: switching speed | small | none | B |
| B: face framing | medium | none | A, C |
| C: idea-first model + Gather/Tighten + MCP | large | none (A's slim view helps) | B |
| D: caption editing | medium | C's Tighten view | E |
| E: graphics step | small–medium | C | D |

A and B touch different code (view/IPC/store vs. render/detection), so they can be two
sessions in their own worktrees. C reworks `shorts.rs`, `ShortsStudio.tsx` and the MCP shorts
tools, so it should not run alongside anything else that edits those.

## Decisions (2026-10-17)

1. The slow switching is **between shorts** in the Shorts Studio list. Phase A targets that.
2. Shorts use **only what the main edit keeps**. No `raw` pieces: a passage cut from the video
   can't go into a short (restore it in the video first).
3. Caption text fixes are **shared**: they edit the transcript word, for the video and every short.
4. Face framing is for **shorts only** for now. The main video's bubble and full-frame webcam keep
   the centre crop; `focus_uv` is written so they can switch over later.
5. Length: **one default target, 30–60 s**, changeable per short. No per-platform presets.
6. Gaming footage without a separate camera recording: **no face framing** for now; revisit with
   the gaming work.
