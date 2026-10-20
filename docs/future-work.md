# Future work

The backlog of features to pick from, one task at a time. Each entry says what we want, what we
already have to build on, and what to settle before starting. When a task is picked up, link
its plan here; when it ships, move it to **Done**.

Started 20 Oct 2026 from George's list and a review of two competitors, Borumi and Tella (see
[Competitor notes](#competitor-notes)).

## Done

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

**Plan:** [docs/plans/layouts.md](plans/layouts.md) (21 Oct 2026, waiting on George's
decisions).

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

### 2. Greek, then more languages

- **First:** transcription, captions, filler and retake rules, and the AI review prompts in
  Greek.
- **Then:** a language setting per project (or per sound) that every pass reads, so adding a
  language means adding its models and word lists, not new code paths.
- **To check:**
  - Which passes are English-only today: the filler word lists, retake detection, caption
    line breaking, and the wav2vec2 word aligner (an English model).
  - Greek needs its own aligner, or must skip alignment.
- Ties into task 12 (local Greek transcription).

### 3. Local LLM for fillers and retakes

Run the filler and retake review on a local model instead of OpenRouter: free, private, and
no key needed.

- **Where it plugs in:** the same slot as the OpenRouter review (`ai/fillers.rs`, the `auto`
  review mode). `auto` would then pick local when a model is installed.
- **Budget:** the RTX 3060 has 12 GB, shared with transcription and face detection. Look at
  small instruction models (around 3-8B, quantized) that can return the same JSON spans.
- **Measure before switching:** compare against the OpenRouter review on the same
  transcripts, counting both misses and wrong cuts.

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

- **Local English, better than Parakeet:** must run in about 6-8 GB of VRAM (the PC has an
  RTX 3060 with 12 GB, and no cuDNN, so check CUDA, DirectML or CPU fallbacks). Compare on
  word error rate and timing accuracy, because cuts depend on word timings (Parakeet's words
  swallow pauses).
- **Local Greek:** a model that transcribes Greek well (task 2).
- **Online:** ElevenLabs Scribe today, plus Scribe through OpenRouter. Add other providers
  through OpenRouter later.

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
