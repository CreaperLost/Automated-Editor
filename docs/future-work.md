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

- **Kinds:**
  - camera bubble (position, size, shape)
  - side by side (camera left or right, or top or bottom in portrait; even or overlapping)
  - camera only (fullscreen, or punched in)
  - screen only
  - presenter cut out over the screen (needs task 10)
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

Gathered 20 Oct 2026.

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
