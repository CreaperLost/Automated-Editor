# Future work

The backlog of features to pick from, one task at a time. Each entry says what we want, what we
already have to build on, and what to settle before starting. When a task is picked up, link
its plan here; when it ships, move it to **Done**.

Started 20 Oct 2026 from George's list and a review of two competitors, Borumi and Tella (see
[Competitor notes](#competitor-notes)).

## Done

- **Greek, local English, the GPU pack and a local AI review** (20 Oct 2026; tasks 2, 3 and 12).
  - **Language spoken** (English or Greek) in transcription settings. Greek goes online only
    (Scribe or Scribe via OpenRouter); a mismatch with what Scribe heard is reported. Filler
    sounds and words (εε, μμ, λοιπόν, δηλαδή, ας πούμε, βασικά, ξέρεις...), accent folding,
    the Greek `;`, caption capitals without accents and the AI prompts follow it; alignment
    stays English only.
  - Local English is now Parakeet Unified EN 0.6B (4.0% FLEURS WER against v3's 4.8%), with
    TDT v3 kept as **Verbatim** in settings (21 Oct): Unified drops stutters, repeats and
    most ums, which the filler and retake passes cut. Both download at full precision on
    CUDA builds.
  - The NVIDIA GPU pack (CUDA 13, cuDNN 9, 1.9 GB) downloads from settings, so Parakeet and
    the aligner run on CUDA; the CUDA installer ships ONNX Runtime's CUDA provider.
  - Fillers and retakes can be reviewed by Qwen3.5 4B inside the app (llama.cpp on Vulkan).
    Since task 3 (21 Oct) it is only the fallback without a key: the app and MCP `fillers`
    `review: auto` prefer OpenRouter/OpenAI; `review: local` forces it.
  - Research behind the choices (20 Oct): Handy's per-language FLEURS tables
    (models.handy.computer). Greek: Cohere Transcribe 9.0%, Whisper large-v3 11.5%, Canary
    1B v2 26%, Parakeet v3 35%.

- **Sound effects off by default** (20 Oct 2026). The clicks and whooshes were spammy, so new
  projects start with both off. They stay in the Audio panel and in `settings` (`area: audio`,
  `soundEffects`) for anyone who wants them, and the skill only switches them on when asked.
- **Fillers and retakes use rules and AI by default** (20 Oct 2026). The MCP `fillers` action
  defaults to `review: auto`: the rules plus the app's OpenRouter/OpenAI review whenever a key
  is set, else the rules plus the agent's own in-house review (`fillers_review`). If the
  OpenRouter review fails it falls back to in-house and says why. `review: in-house` or
  `openrouter` forces one. Later the same slot takes a local model (task 3).

## Known issues

Still open from the application audit of 18 Oct 2026, checked against the code on 20 Oct.
The other seven findings were fixed and their file was removed; it is in Git history as
`docs/application-audit-2026-10-18.md`.

- **Transcript panel work grows with the transcript during playback** (audit F05, P2
  performance). `TranscriptPanel.tsx` runs a `findIndex` over every word and every line on
  each playback update, and keeps every line mounted.
  - **Fix:** build a time-sorted index when the transcript changes, then use binary search
    or an advancing cursor.
  - Virtualize the lines, keeping selection and active-word scrolling.
  - Measure on a one-hour transcript before and after.
- **Every editor panel is in the initial JavaScript bundle** (audit F06, P3 performance). The
  separate Shorts window is gone, but `DockWorkspace.tsx` still imports every panel eagerly.
  - **Fix:** lazy-load optional panels and dialogs, then compare the size of the first
    bundle and the cold start time.
- **Check: a newer graphic render can drop a waiting one for another length** (follow-up to
  audit F03). In `graphics/render.rs` the queue replaces a waiting job for the same graphic
  and canvas even when its length differs. If the main edit and a short use the same graphic
  at two lengths, one render may never be queued. Verify, and if so include the length in
  that match.

## Tasks

### 1. More layouts

A small set of named layouts that work for both long-form (16:9) and shorts (9:16), changed
over time on the timeline instead of one look for the whole edit.

**Plan:** [docs/plans/layouts.md](plans/layouts.md). Decided and built 21 Oct 2026; still to
do: the look-only playback swap (needed by task 10) and a perf measurement.

- **Kinds:**
  - camera bubble (position, size, shape)
  - side by side (camera left or right, or top or bottom in portrait; even or overlapping)
  - camera only (fullscreen, or punched in)
  - screen only
  - presenter cut out over the screen (needs task 9's matte)
- **Saved favourites:** your own layouts, reused across projects.
- **Timed layout sections** that stay attached to their content through cuts. The base layout
  fills every gap, so there is never a flash of a default layout.
- **Transitions:** a hard cut or a short animated move into each section.
- **Fold in what exists:** today the look is one global layout (`project/layout.rs`
  `EditLayout`), plus webcam-focus stretches (`webcam_focus.rs`), plus the shorts'
  camera/screen/split scenes (`short_scenes.rs`). These should become one system: webcam
  focus and shorts scenes are just layout sections.
- **MCP:** list, add, update and remove layouts, save a favourite, and an auto-layout step (see
  task 11).
- **Reference:** Tella's layout kinds and auto-layout rules ([notes](#tella)), and Borumi's
  layout pitfalls to avoid ([notes](#borumi)).

### 2. More languages

English and Greek are done (see Done). Next:

- A Greek word aligner, so Greek cuts get 20 ms word edges too: MMS forced alignment (it
  romanises the text, so English tech words inside Greek speech align as well) or a Greek
  XLSR CTC model, on the same ONNX Runtime path as wav2vec2.
- Further languages: each needs its filler words, its question mark, and an online provider
  that knows it.

### 3. Measure the local review

The local Qwen3.5 4B review is built (see Done). Compare it with the OpenRouter review on the
same English and Greek transcripts, counting misses and wrong cuts, before trusting it with
`apply: true` by default. Try Gemma 4 E4B too if Greek is weak.

- **Measured 21 Oct 2026:** [Local vs OpenRouter review](plans/local-review-eval.md). The
  probe is `ai/review_probe.rs`; the labelled cases are in `plans/review-eval`.
  - English, 3 recordings and 3,244 words: the local review missed 9 of 18 clear cuts and
    made 10 wrong cuts of its own (45 words). OpenRouter (`gpt-6-luna`) missed 5 and made 0.
    The rules alone miss 11 and make 2 wrong retake cuts, which every engine inherits.
  - Greek (a scripted case; no Greek recording yet): local missed 10 of 19, OpenRouter 1.
  - Local runs on the GPU through Vulkan (about 3.4 GB VRAM): 2–3 s to load and 3–12 s to
    review, or about 65 s on the first run after a build. OpenRouter took 12–38 s.
  - Its wrong cuts are candidate flags it approved as they were: phrases said twice taken
    as retakes, and "so" meaning "therefore".
  - **Decided (George, 21 Oct):** OpenRouter/OpenAI is the default, and local runs only when
    no key is set. This applies to MCP `review: auto` and to the app's Find with AI, whose
    saved default is now `cloud`. `apply: true` is not the default for either review yet.
- **Still to do:** a real Greek recording, and the candidate-pass and rules fixes listed in
  the plan. Gemma 4 E4B (4.98 GB GGUF, needs a Gemma chat format in the probe) is skipped for
  now.

### 4. Automatic B-roll from free libraries

Add B-roll video automatically from free, openly licensed sources. To discuss: which sources
(for example Pexels, Pixabay, or Wikimedia Commons: their APIs, licences and attribution
rules), how to pick moments and search terms from the transcript, and whether it is
suggested for review or placed directly.

- **Also from your own recording:** Tella can show an earlier moment of your own screen
  recording as B-roll while you keep talking. That needs no download and may be the
  cheapest first step.
- **Fits with:** graphic slots (the Graphics v1 design) and the layout sections in task 1
  (B-roll fills the screen slot for a range).
- **Find the right source moment first:** the Premiere MCP review below shows a missing
  step before placement: searchable source footage, including parts cut from the current
  edit. Build on task 14, then let the agent propose B-roll from returned source ranges.
  `short_footage` already places the edited video's footage under a take short; extend that
  workflow rather than building placement again.

### 5. Music and sound effects

Background music and sound effects. To discuss:

- where they come from: free libraries, AI generation, or the user's own files;
- music ducking under speech (the ducking code exists for background lanes);
- loop and fade rules;
- placing sound effects on chosen events, not on every click (see the Done entry above for
  why the automatic ones were turned off).

Both Borumi and Tella ship a curated library; Tella also generates sound effects from a
prompt.

### 6. Edit, save over, remove and add templates

Make templates reusable: load a template, change things, then click **Save** on the
template to replace it with the current settings. Also remove a template and add a new
one from the current project.

- **Look at:** `templates.rs`, the Templates menu (`front-end/.../TemplatesMenu.tsx`) and the
  MCP `template_save`. Decide which settings a template holds (layout, audio, captions,
  zooms, cleanup choices, brand kit) so saving over one is predictable.

### 7. Image previews on track clips

Each clip on the timeline shows a small preview image.

- **Very low quality** (tiny thumbnails, a few per clip), so they never slow the app down or
  hold up loading.
- **Only when the track is tall enough to show them.** Otherwise they stay hidden and are
  not even loaded.
- **To decide:** where thumbnails come from (decoded once and cached per source file, like
  the waveform overview), how many per clip at each zoom level, and how they are cancelled
  when you scroll away. Follow the preview-performance rules.

### 8. A simpler UI with advanced settings behind a switch

Show the few settings most people need, and let advanced users open the rest ("Advanced"
sections or a global switch). Start with an inventory of every panel's settings, split into
basic and advanced.

### 9. Webcam background removal, blur and virtual backgrounds

- **Remove the background:** a matte, like a green screen, so the presenter can be cut out
  over the screen (the cut-out layout in task 1).
- **Blur the background.**
- **Replace it** with a colour, image or wallpaper.
- **To decide:** the person-segmentation model (it must run in real time in the preview, or
  be computed once per recording and cached like face tracks), and how it fits the wgpu
  compositor.
- Both Borumi ("Green Screen" layout, background blur, virtual backgrounds) and Tella
  (cut-out, pop-out) have this.

### 10. Multi-camera: switch layouts with number keys

A manual way to cut between layouts quickly, for gaming videos as well as tutorials.

- Press 1, 2, 3, ... 9 while playing or scrubbing. The layout with that number applies to
  the active track at the playhead, or to the track under the pointer.
- Each available layout (task 1) gets a number.
- The same switching is open to AI through the MCP (task 11).
- Needs task 1 first (timed layout sections). Remember the hotkey settings (`hotkeyStore`)
  and that playback must not stutter while switching.

### 11. Auto layouts and multi-scene changes from the skill and MCP

How the agent, through the MCP and the skill, plans layout changes and scene switches
automatically. To discuss.

- **Starting point:** Tella's auto-layout skill ([notes](#tella)). The agent picks a base
  layout, then adds a few changes per minute at most:
  - screen focus during dense UI;
  - a camera cutaway on human moments;
  - a punch-in on the strongest line;
  - the camera on the side opposite the speaker's gaze.
- **We have better signals than Tella:**
  - clicks, keys and windows from `.aero` telemetry;
  - mouse rest, already used by webcam focus;
  - speech;
  - face tracks, for the speaker's position and gaze side.
- So the app can propose the plan itself and the agent reviews it.

### 12. More transcription models

- **Local Greek:** Cohere Transcribe 03-2026 (9% Greek WER on FLEURS, 2.25 GB) once it can
  run here (transcribe.cpp or an ONNX export) and a Greek aligner exists (task 2): it gives
  no word times. Test it on Greek speech with English tech words first.
- **More accurate local English:** Qwen3-ASR 1.7B (3.2% FLEURS WER, about 6x slower than
  Parakeet), if Parakeet Unified's mistakes show up in practice.
- **Online:** other OpenRouter speech models (Voxtral, GPT-4o Transcribe) behind the same
  provider seam.

### 13. Project version history

Every change saved as a version you can go back to, even after closing the app, with
"undo what the agent did" as one step. Today undo is lost when a project is reopened. Borumi
shipped this together with its MCP, so users can roll back an AI edit they don't like. This
was turned down once as "named versions/compare", and is back on the list as plain
persistent history.

### 14. Search and summarize source footage

The largest gap found in the [Premiere MCP review](#premiere-pro-mcp-servers): an agent can
inspect our edit, but cannot ask the app to find "where I open the export dialog" across
recordings. The general media index in [AI readiness](plans/ai-readiness.md#b-roll-and-media-library)
is still a plan; the shipped `library_save` is for reusable graphics.

- **First step:** search saved source transcripts by words and phrases, with a bounded
  result count. Return asset/stream IDs, source in/out times, transcript excerpts and word
  IDs; optionally include their occurrences on the edited timeline. Searching only the kept
  words would miss useful material already cut from the edit.
- **Next:** scene descriptions and tags from sparse contact sheets, written by the user's
  agent and cached against the source identity. Add OCR for tutorial UI labels if it proves
  useful. Keep this on the existing "user's agent supplies understanding" path; a bundled
  semantic model is a separate decision, after measuring keyword search misses.
- **Reuse the evidence:** keep source-analysis versions separate from timeline revisions,
  as in leancoderkavy's context engine below. Moving/trimming a clip should retain its source
  descriptions; replacing the source should invalidate them. Store evidence IDs, source
  ranges and provenance, then return a bounded context pack for the agent's current task.
  Existing saved transcripts already survive edits; extend that principle to shot notes and
  audio observations. Offer a way to clear the index.
- **Navigate long recordings:** a compact source summary with timed sections and a way to
  read just one section's evidence. `storyboard` already summarizes the finished picture;
  this is an index of the source, including material outside the current edit.
- **Scope:** current project's sources first, then folders and cross-project reuse. Persist
  analysis status and file fingerprints; distinguish an unavailable source from no matches.
  Analysis must report progress, support cancellation and resume without redoing unchanged
  files. Offline results may stay searchable, but unavailable files cannot be placed/exported.
- **Finish the workflow:** saved selects (named source ranges with notes), then a reviewed
  placement batch for task 4 or a take short. Preserve source time separately from edited
  time, and recheck the target revision before applying.
- **Verify:** retrieve labelled moments from a real tutorial, including one removed by a
  cut and one in an imported take; return playable source ranges and rebuild the same
  selects after unrelated timeline edits. Measure retrieval misses and latency before
  expanding to embeddings or people recognition. Face centering already exists; identifying
  named people across a footage library is a different, lower-priority feature.

### 15. Export a handoff for another editor

Today MCP exports a rendered video or shorts. Consider an editable handoff for someone who
wants to finish in Premiere, alongside the transcript/audio exports already listed below.

- Start by choosing a target format and testing a minimal cuts-only sequence with source
  paths, source in/out, frame rate and linked audio. Decide how unavailable media is reported.
- Layouts, telemetry zooms, captions, speed changes and HyperFrames graphics need an explicit
  mapping or a rendered fallback; report what was flattened or omitted. Do not promise a
  lossless round trip before it has been tested in the receiving editor.
- Individual source selects with a timecode manifest could be a smaller first deliverable.
- **Priority:** after source search if an actual Premiere finishing workflow needs it.
  Verify by importing a fixture and checking frame boundaries and audio sync in Premiere.

### 16. MCP discovery and connection diagnostics

The current server registers 53 tools (2 guide tools, 14 reading/proposal tools and 37 app
actions) and returns their full schemas in `tools/list`. Before the catalog grows further:

- Measure schema/context cost in supported clients. If material, offer a small default set
  with searchable tool descriptions and on-demand schemas/invocation, while retaining a
  full-catalog option for clients that discover native tools themselves. Existing tool names
  and CLI commands should keep working.
- Add one read-only readiness report usable without an open project: app/bridge versions,
  connected state, open project and revision if any, available actions, missing runtime/model
  dependencies and actionable errors. Reuse `status` and `scan` checks where possible.
  MCP's handshake already reports a server version; it does not provide this full report.
- Distinguish "connected but no project", "model/runtime missing" and "unsupported here"
  from a failed operation. Return structured fields alongside the short human explanation.
- Run a disposable-project workflow through the real bridge on Windows and macOS: inspect,
  propose, accept, inspect the changed timeline, undo, render, export and check the output.
  Existing unit/bridge tests remain useful; record host-verified coverage separately.
- Publish a generated capability ledger from tool registration, with platform/dependency
  requirements and what each check establishes. Distinguish timeline readback, rendered-frame
  inspection and audio/playback review. Our exporter already checks output duration and
  decodes the first/last frames to check dimensions (`export/mod.rs`); broader delivery QA
  would add sampled frames around edit boundaries and audio checks, not replace those checks.

### 17. Bind agent edits to the intended project

The additional Premiere MCP review highlighted a concrete guard to add to our proposal
path. `propose_cuts` and `propose_edit` take `expectedRevision`, but their shared MCP helper
calls `with_proposals(state, None, ...)`, which selects whichever project is open when the
call runs. The UI path already supplies a project handle. A matching numeric revision alone
does not identify a project; local clip IDs can coincide too.

- Return and require the inspected project/session identity alongside the revision for
  agent-planned mutations; check both under the command lock. Carry the short/timeline
  identity explicitly where relevant. Audit the other direct-action paths for the same race.
- **Verify:** inspect project A, switch to B with the same revision and colliding clip IDs,
  then submit A's proposal. It must refuse without saving a batch or changing B. Repeat with
  a close/reopen so the intended lifetime of a session handle is explicit.
- This is separate from task 13's persistent undo. Prioritize the guard before adding more
  agent mutation tools; the existing revision and proposal checks remain useful.

## More ideas from the competitor review

Smaller things worth keeping:

- **Hide sensitive areas:** blur a region (an email address, a key) or highlight one for a
  time range.
- **Crop to the visible content automatically.** Window crop does part of this already.
- **Transcript export** as SRT, Markdown or plain text, and an audio-only export.
- **Custom vocabulary** for transcription (product names, jargon).
- **LUT import** for colour.
- **"Behind the subject" text:** captions or titles drawn behind the presenter. Needs the
  matte from task 9.
- **A sound on layout changes,** optional. Only with task 1, and only if it stays subtle.
- **Copy scenes and clips between projects.**

## Competitor notes

Borumi and Tella gathered 20 Oct 2026; Premiere MCP review added 21 Oct 2026.

### Premiere Pro MCP servers

Reviewed 21 Oct 2026 against the current AeroEdits source. These are research findings and
proposed backlog items, not decisions to integrate or a claim that the external tools were
tested locally. The three references cover different parts of the workflow.

**Jumper — footage understanding.** Its [Premiere page](https://getjumper.io/ai-agents/premiere-pro)
documents local visual, speech and named-person search, active-sequence reading, timeline
delivery to Premiere, XML export and individual clip exports. Search requires prior analysis;
the agent can start it. Its [documentation](https://docs.getjumper.io/) also describes
cross-project search, including indexed media on disconnected drives, and summaries for
navigating long footage. These are vendor-documented capabilities, not retrieval results
measured here. We should borrow the search → source range → selects → placement workflow.

**Adobe_Premiere_Pro_MCP — editing control.** The
[repository](https://github.com/hetpatel-11/Adobe_Premiere_Pro_MCP) reports 283 catalog tools,
covering projects/media, timeline edits, effects/keyframes, captions, proxies, multicam,
color, audio and export. Its supported bridge is a live CEP panel; UXP is experimental.
It provides readiness/capability checks and advertises a small tool set by default, with
`search_tools` / `get_tool_schema` / `invoke_tool` for the rest. The
[dispatcher source](https://github.com/hetpatel-11/Adobe_Premiere_Pro_MCP/blob/main/src/tools/index.ts)
confirms catalog discovery and schema validation before dispatch. Tool count is not a
quality measure or evidence that every operation works on every host.

The repository's [known issues](https://github.com/hetpatel-11/Adobe_Premiere_Pro_MCP/blob/main/KNOWN_ISSUES.md)
qualify those claims: scripting has host limits, render-queue status needs Media Encoder,
some MOGRT behavior remains unverified, and a live sweep mutates its test project. Earlier
bugs included editing the wrong sequence and reporting success without a visible change.
For us, that argues for explicit target IDs and checking resulting state in real-host tests.

**leancoderkavy/premiere-pro-mcp — reusable context and explicit verification.** This is a
[separate project](https://github.com/leancoderkavy/premiere-pro-mcp), published as
`premiere-pro-mcp`; the previous repository uses `adobe-premiere-pro-mcp`. Both declare the
same executable name, so identify the repository/package when comparing them. Its README
describes a CEP path plus capability-gated UXP additions, project-target guards and bounded
capability discovery. Catalog/version figures differ between the retrieved README and
[generated manifest](https://github.com/leancoderkavy/premiere-pro-mcp/blob/main/public-product-manifest.json);
treat them as source snapshots, not a verified installed feature count.

The most relevant design is its
[context engine](https://github.com/leancoderkavy/premiere-pro-mcp/blob/main/docs/project-context-engine.md):
capture a bounded timeline snapshot, attach explicit transcript/shot/audio evidence, retrieve
matching evidence, and retain source analysis across timeline changes. It does not itself
perform vision or transcription. The
[implementation](https://github.com/leancoderkavy/premiere-pro-mcp/blob/main/src/tools/project-context.ts)
defines separate source/timeline revisions, media fingerprints and bounded enrichment input.
This gives task 14 a concrete implementation reference compatible with our agent-led design.

Its [editorial workflow guide](https://github.com/leancoderkavy/premiere-pro-mcp/blob/main/docs/ai-editorial-workflows.md)
separates evidence packs and plan previews from applying a supported operation. It also
separates structural readback from playback/render verification. AeroEdits already checks
proposals on a copy and provides review/undo; the additions to pursue are reusable evidence,
explicit project binding (task 17), and a capability/verification ledger (task 16).
These are documented/source-inspected contracts; no Premiere host was exercised here.

**What AeroEdits already has, and what is actually missing:**

| Area | Current AeroEdits evidence | Gap / next step |
|---|---|---|
| Inspect and review an edit | `project_info`, `timeline_get`, `transcript_get`, `frame_render`, `timeline_view`, `storyboard` | Keep these; source search and source summaries are task 14. A finished-frame sheet is not an indexed footage library. |
| Find footage by meaning | Agents can read a transcript and inspect frames; graphics have a reusable library | No general footage-search tool or persistent searchable scene index. Begin with source transcript search and cached descriptions (task 14). |
| People | `faces` detects faces for framing | No named-person retrieval across sources. Defer unless multi-person footage becomes a real use case. |
| Build an edit | `media_import`, `propose_edit` with ordered `SequenceEdit` operations, word-based cuts, linked clips and `speed` | Basic editing is already covered. Named source selects and reusable B-roll placement need task 14 + task 4. |
| Agent workflow guidance | `template`, saved answers, shipped skill, three MCP prompts | Already present; add a search/selects recipe when those tools exist. |
| Export and quality review | `check`, `export`, `export_status`, rendered shorts; storyboard verification | Editable NLE handoff and individual selects are missing (task 15). Export progress and readiness checking already exist. |
| Connection and discovery | Private stdio bridge to the running app, version handshake, client setup, `status`, `scan`; full tool list | Add a project-independent readiness report; measure catalog cost before progressive discovery (task 16). |
| Recover from edits | Revision-checked proposals, preview diff, accept/reject, undo/redo and agent history | Preserve these guarantees. Persistent history remains task 13; a larger tool catalog would not solve it. |
| Bind a request to its target | UI commands can require a project handle; MCP proposals check the current project's revision | Require project/session identity too, so a project switch cannot redirect a stale request (task 17). |
| General finishing | Layout transitions, telemetry zooms, caption settings, graphics, speech cleanup and audio settings | No generic effects/Lumetri/keyframe surface or full proxy/multicam workflow found in the MCP catalog. These are broader editor scope, not immediate tutorial priorities. |

**Recommended order:** add the project-identity guard before expanding mutation tools. For
footage features, source transcript search and saved selects first; source descriptions
and summaries next; use those results for B-roll; then add handoff if needed. Connection
diagnostics can be a small separate task. Keep the existing proposal/revision/undo path for
agent-planned edits, while the built-in actions retain their current direct-action behavior.
We do not need to match Premiere's catalog to improve the tutorial workflow.

Local evidence checked: [`mcp/tools.rs`](../src-tauri/src/mcp/tools.rs),
[`mcp/actions.rs`](../src-tauri/src/mcp/actions.rs),
[`mcp/mod.rs`](../src-tauri/src/mcp/mod.rs),
[`mcp/skill.rs`](../src-tauri/src/mcp/skill.rs),
[`sequence/edit.rs`](../src-tauri/src/sequence/edit.rs),
[`proposals.rs`](../src-tauri/src/proposals.rs), and
[`graphics/library.rs`](../src-tauri/src/graphics/library.rs). Additional review:
[`mcp/propose.rs`](../src-tauri/src/mcp/propose.rs),
[`commands/proposals.rs`](../src-tauri/src/commands/proposals.rs) and
[`export/mod.rs`](../src-tauri/src/export/mod.rs).

### Borumi

[borumi.com](https://borumi.com). A desktop recorder and editor for macOS and Windows, $79
lifetime. It has an MCP server since v0.29 (31 Jul 2026), but no public docs. What follows
comes from a community plugin that drives it and records what it verified on live projects:
[Samin12/edit-borumi-video](https://github.com/Samin12/edit-borumi-video) (its
`references/borumi-mcp.md`). Related:
[proof-first-video-editor](https://github.com/Samin12/proof-first-video-editor) and
[opencutagent-borumi](https://github.com/Samin12/opencutagent-borumi).

- **Server:** the app binary run as `borumi mcp` (stdio). The app must be running.
- **Tools:**
  - **Projects:** `list_open_projects`, `open_project`, `duplicate_project`,
    `get_project_overview`, `get_scenes`
  - **Reading:** `get_timeline`, `get_transcript`, `detect_speech`, and `inspect_timeline`,
    which returns a contact sheet of frames, including staged edits
  - **Editing:** `trim_timeline`, `untrim_segments`, `split_segments`,
    `move_timeline_segments`, `delete_segments`, `add_segments`, `update_segments`,
    `create_scenes`, `set_segment_speed`, `import_media`
  - **Export:** `export_video`, `export_audio`, `export_transcript`, `get_export_status`
- **Transactions:** `begin_project_edit` returns a transaction id. Edits carry a timeline
  hash, and nothing lands until `commit_project_edit` with a one-line summary, or
  `abort_project_edit`.
- **Guide gating:** edit tools refuse to run until the agent has read the matching guide on
  that connection.
- **Layouts are timeline segments** with JSON properties:
  - `fullscreen` (camera or screen, cover fit);
  - `custom`: each source has x/y/width/height ratios, corner shape (squircle), shadow,
    face-tracking override and overflow.
- **Named layouts:** only "Green Screen" and "Squircle". Otherwise "side layouts", "custom
  layouts", layouts "without background", and "Favorite layouts". The homepage grid is just
  an image.
- **Other layout features:**
  - shadow and stroke controls;
  - 100+ wallpapers and custom backgrounds;
  - background blur and virtual backgrounds;
  - layout transitions you can switch off, with an optional sound;
  - portrait, square and custom canvas sizes.
- **Pitfalls to avoid** (from the plugin's notes):
  - Layouts drift away from their content after ripple trims.
  - Gaps between layouts flash the default layout (a fullscreen camera).
  - Deleting or untrimming a layout can rebuild layouts elsewhere in the project.
  - Exported audio is 44 ms behind the picture.
  - The mix is normalized to about −13.7 LUFS, which is loud.
- **Also has:** recording scene by scene with multiple takes, a teleprompter that follows
  your voice, automatic face centering, transcript-based editing, silence removal with
  parameters, captions with per-section styling, and per-clip speed.

### Tella

Tella publishes a full MCP and API, and its auto-layout guidance as an open agent skill:

- [MCP tools: layouts](https://www.tella.com/docs/mcp-tools/layouts.md)
- [developer docs index](https://www.tella.com/docs/_llms/developers.txt)
- [tellahq/skills](https://github.com/tellahq/skills): auto-cut, auto-layouts, remove-mistakes

**Layout kinds:**
- `fullscreen`, `middle`
- `side-by-side`: left/right, or top/bottom in portrait; regular, even, overlap or perspective
- `tv-presenter`
- `cut-out`: the presenter matted over the screen
- `camera-bubble`: 9 positions, sizes S/M/L, circle/square/landscape/portrait shapes
- `camera-only`: fullscreen, punch-in, or middle
- `screen-only`
- `custom`: pixel geometry, shape, corners, order, hidden

**On each layout:**
- a camera style override: circle, square, portrait, cutout, blob;
- "pop-out": the presenter rises out of a frame behind their head;
- a crop for that layout only;
- B-roll media in the screen or camera slot: an image, a video, or a moment of the clip's own
  recording;
- the transition into it: `hardCut` or `spring`.

**Favourites:** `list_saved_layouts`, `apply_saved_layout`, `save_layout`.

**Other tool groups:**
- clips: cut by transcript, remove fillers, remove silences
- zooms, including generating them from clicks
- mouse events, key events, camera faces
- auto crop
- blurs and highlights
- overlays and text overlays
- sound effects and background music
- AI image and sound-effect generation
- backgrounds, LUT and font uploads
- chapters
- `apply_video_edits`: up to 200 edits in one call that apply all or nothing
- social publishing and analytics

**Auto-layout rules** from their skill, worth adopting for task 11:
- Choose what the viewer should look at, moment by moment:
  - **the screen** for clicks, typing, menus, results, or "as you can see";
  - **the speaker** for a hook, an opinion, a reaction or a sign-off while the screen is
    static;
  - **both** for everything else, which is the base layout.
- **Base layout:** a bubble at size M for demos, side by side or presenter for talks.
- **A few changes per minute at most.** Too many is the most common complaint.
  - Screen focus: 2-8 s.
  - Camera cutaway: 2-5 s.
  - Punch-in: 1.5-4 s.
- **Placement:** the camera goes on the side the speaker faces away from, and stays there.
- **Transitions:** hard cuts between full-frame shots, animated moves for size and split
  changes.
