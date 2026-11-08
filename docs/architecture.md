# Architecture: how AeroEdits is built

Implementation invariants and recorder contracts, consolidated 1 November 2026.
Build and setup: [root README](../README.md). Product constraints: [Product](product.md).
Measured checks and dependency policy: [Verification](verification.md).

## Stack

- **Front end:** React + Vite + TypeScript + Tailwind, zustand stores, dockview workspaces
  (Edit, Cleanup, Shorts). `front-end/src/lib/ipc.ts` wraps every backend command.
- **Back end:** Tauri v2, Rust. Every Tauri command is a thin wrapper over
  `commands::*_impl(&AppState, …)`; the MCP server calls the same `_impl` functions under the same
  locks (`command_lock`, then `opened_project`), so revision checks and undo behave identically.
- **Media:** FFmpeg (bundled sidecar on Windows/Linux) behind `media/ffmpeg.rs`; Swift
  AVFoundation bridges on macOS (`native/macos/`). Compositing in wgpu (`render/composite.wgsl`),
  with a CPU path kept at parity by tests.
- **Models in the app:** Parakeet on ONNX Runtime (CUDA / DirectML / CPU; CUDA needs
  the downloadable GPU pack), YuNet face detection, Qwen3.5 4B through llama.cpp on Vulkan
  (`local-llm` feature).

## The project and the sequence model

A project folder holds `project.json` (the edit document), `transcripts/<asset>.<stream>.json`,
`agent/proposals.json`, `graphics/<id>/`, `history/` (undo history and checkpoints) and caches. Recordings play from their own folders,
mounted at `recordings/<asset id>/`.

- **Assets and streams.** Everything playable is an asset (`Recording`, `Video`, `Image`, `Audio`,
  `Graphic`) with streams (`screen`, `camera`, `audio:N`, `picture`). All streams of an asset share
  one clock. The project's own recording is just the first Recording asset. Stream keys are
  `<asset>.<stream>`.
- **Tracks and clips.** Video tracks V1… and audio tracks A1…, all the same kind of thing. A clip
  is `{source: StreamRef, start_us, in_us, duration_us, link}`. Video clips never carry sound;
  sound is a linked clip on an audio track. Track roles: Screen, Camera, Overlay (video); Speech,
  Background, Music, Effects (audio). Mix (volume, denoise, ducking) is per audio track; clips
  have their own gain and fades (SND-1). The mixer (`media/audio.rs`) reads each sound file
  through its polish (`media/polish.rs`): denoise, then take matching (AUTO-2, `dsp/takes.rs`:
  a gain and gentle biquads per file of speech, run from a 100 ms warm-up so any chunking gives
  the same samples), ducking, the music's level under speech, normalisation.
- **Libraries** (`sound_library.rs`, `music_library.rs`): built-in sounds are compiled in and
  written once to the app folder; built-in music is too big for that, so each track is pinned
  by size and SHA-256 and downloaded from the Free Music Archive on first use. Placing either
  imports it into the project with its licence; music that needs credit is refused for the
  library and flagged by `check`.
- **Magnetic** ripples every unlocked track; off, cuts leave gaps and moves overwrite. Empty time
  is black.
- **Edits** are a typed `SequenceEdit` enum (including: add/move/remove track, place asset, move,
  trim, ripple trim, split, delete, delete range, link, unlink, set clip, set magnetic, set speed,
  restore cuts…), applied to a copy and committed as one revision (`project/revision.rs`).
- **Clocks: one mapping primitive.** `sequence/clock.rs` builds, from the clips that play a stream
  or asset, a map between source time and timeline time (a moment can appear twice; the earlier
  clip owns it). Transcripts, captions, word cuts, silence suggestions, zooms, layout sections,
  chapters and shorts all live in **source time** and are mapped through a clock on read, so cuts,
  reorders and undo never move them off their content.
- **Revisions and undo.** Every edit bumps `revision`; edits pass `expected_revision` and a
  mismatch is refused ("Stale edit revision").
- **History on disk** (`project/history.rs`, SAFE-2). Each edit state is saved gzip-compressed
  in `history/states/<sha256>.json.gz`, named by its JSON with `revision` zeroed (so undo and
  redo write nothing new); `history/log.jsonl` is append-only (commit, undo, redo, drop,
  checkpoint lines) and is replayed, checked against `project.json` and rewritten on open. A
  crash between saving the edit and the log comes back as a "Change made outside AeroEdits"
  step. `EditHistory` holds steps, not documents; up to 2 GB of states per project, oldest steps
  first, no step limit. Steps are named from what changed (`history::describe`) and marked as
  an agent's while its MCP call runs (`history::as_agent`, a thread-local set in `tools::call`).
- **Agent runs.** An agent's steps carry a run id while the same agent keeps editing within 15
  minutes with nothing of the user's between; `preset` starts a new run. Undoing a run
  (Agent panel, MCP `undo` `run: true`) restores the state from before it as one new step and
  marks its batches undone.
- **Checkpoints** (`project/checkpoint.rs`): the edit's state, each graphic's source files
  (`history/files/<sha256>.gz`) and each transcript's caption marks with a stamp of its word ids.
  Saved by the user (top bar History), the agent (`checkpoint` behind `more`) and the app before
  every agent run (newest five kept). Going back is one step (when the edit differs), puts the
  graphic files and caption marks back (marks only onto the same words), asks for the graphics'
  renders, withdraws waiting batches, and saves what was there first as a checkpoint. Graphic
  edits are undo steps (SAFE-3): the edit records each graphic's files by content
  (`graphicFiles`, bytes in `history/files`), each step lists the files it changed so pruning
  keeps them, and undo or redo puts back only the graphics that step changed
  (`EditHistory::restore_graphic_files`). A redo can also go into a version (GFX-4).
- **Shorts** are windows anchored in source time that follow the video's edits until edited on
  their own (`ShortEdit`, an own sequence); take shorts carry a take (camera + voice) with footage
  from the video under it (`take_short.rs`), each piece at its own speed. Their own layout
  tracks, captions and graphic slots. A graphic split beside the face (`screen`) or over it
  (`overFace`) adds a layout section for its time when the short's timeline is built
  (`shorts::graphic_sections`, never stored), and its render gets `data-mode`
  (`full`/`split`/`over`) on the root.

## Transcription

- Providers behind one seam: Parakeet (local, English only; Unified by default, TDT v3 as
  **Verbatim**), ElevenLabs Scribe, Scribe via OpenRouter. The language setting is `auto`
  (default), `en` or `el`. Online is a chain: the saved route first (ElevenLabs by default), the
  other when the first has no key or fails (no credits). On `auto` no language is sent and the
  transcript keeps the one Scribe heard most (by words, over the chunks); fillers, retakes, the AI
  prompts and caption capitals follow the transcript's language. Agents transcribe with
  `provider: auto` and never change the app's saved settings.
- Uploads: ElevenLabs takes 10-minute chunks (~19 MB of 16-bit WAV), OpenRouter 5-minute chunks
  (~10 MB; its multipart limit is 25 MB and the provider gets about a minute), split at the
  quietest point near the limit.
- Word alignment (wav2vec2) was removed on 22 Oct 2026: it did not help in practice.
  `tighten_loose_words` handles Parakeet's loose words (it stretches words over pauses on its
  80 ms grid).
- Filler/retake review: rules flag candidates (`ai/candidates.rs`), then OpenRouter/OpenAI reads
  every word in 300-word chunks, or local Qwen reads only the candidate windows under a JSON
  grammar, greedy. Probe: `ai/review_probe.rs`, labelled cases in [plans/review-eval](plans/review-eval/).

## Picture: layouts, zooms, telemetry

- **Layouts** (`layouts/`): a timeline has a base layout and sections
  `{media, source_start_us, source_end_us, layout, transition, auto, rule}`. `resolve` maps them
  through clocks once per evaluator (sliver rules: base gaps under 1 s absorbed, pieces under 1 s
  hidden but kept), `arrange` turns a layout into slot rects for a canvas, `lerp` blends two
  arrangements for moves. One drawing path for the video and shorts in
  `export::SceneEvaluator::scene_at`. `pick` rules: rest (camera while the mouse rests and you
  talk), busy (screen while clicks/typing come fast), hook (camera on a short's opening line),
  no-footage (take shorts). Plan and follow-ups: [roadmap](ROADMAP.md#lay-3).
- **Zooms** from clicks, dwell, click clusters and typing (`zoom/`), eased in recording time.
- **Telemetry** (`telemetry/`): `events.jsonl` + `geometry.jsonl` (mouse), `keys.jsonl`
  ([contract](#keyboard-telemetry)), `windows.jsonl`
  ([contract](#window-telemetry)). Drives zooms, layout picks, window crop, bubble hide,
  shortcut badges and the click/whoosh lane (`sound_effects.rs`).
- **Screen effects** (`effects.rs`, FX-2): a box of a screen recording's own picture (0..1) over
  a stretch of its own clock, so crops, zooms, layouts and cuts never move it. `scene_at` draws
  the ones showing right after the pointer (`Scene::push_screen_effect` maps the box through the
  screen layer's rect and uv); a dim or a blur behind goes in just before its graphic
  (`Scene::push_blur_behind` blurs the screen and camera layers under the graphic's rect). Blur and the spotlight are
  shader passes (`pass_kind` 2 and 3 in `composite.wgsl`, twinned on the CPU in `blit_effect`);
  blur and magnifier layers draw the screen's texture again (`Layer::shares`) instead of
  uploading the frame twice. A blur or hide never fades.
- **Faces:** YuNet tracks per camera asset in source time; `focus_uv` frames each camera slot on
  the face, lining up every 30 s rather than following; per-clip hand framing overrides.

## The agent connection (MCP)

`src-tauri/src/mcp/`. Measurements of real agent edits are in
[Verification](verification.md#agent-edit-evaluation-eval-1).

The editing contract and approval rules live in [Product](product.md#editing-contract).

The app runs a preset's first steps itself through the same tools (`first_steps.rs`, AUTO-1:
Preset → Run), as author "AeroEdits", so its steps are undo steps like an agent's and `status`
can tell a later agent what already ran.

- **Transport.** Agents start `aeroedits --mcp-stdio` (`bridge.rs`). The bridge answers
  `initialize`, `ping` and `tools/list` itself (so the client sees tools while the app is closed)
  and forwards everything else over `\\.\pipe\aeroedits-mcp-<user SID>` (`ipc.rs`: DACL for this
  account only, remote clients rejected, first instance only; the bridge checks the pipe server
  runs as the same account). macOS: `agents/mcp.sock` in a 0700 folder, peer uid checked. Served
  by `rmcp`. Off by default (`mcp.json`). Debug builds: `AEROEDITS_MCP_DEV=<name>` suffixes the
  pipe so a dev copy runs beside the installed app.
- **Two doors, one set of tools.** `aeroedits do <tool>` (`cli.rs`) speaks MCP over the same pipe;
  `tools/list` drives both, so every tool has a CLI form.
- **Proposals** (`proposals.rs`, `mcp/propose.rs`): batches of `Cuts` (word spans or edited ranges)
  or `Edits` (`SequenceEdit`s), checked on a copy when proposed, stored in
  `<project>/agent/proposals.json` (no revision used, survives restart). Accepting is one undo
  step; word cuts are re-resolved against the current edit, ranges and edits only at their base
  revision. Autopilot accepts automatically.
- **Project binding.** Each open gets a fresh `project_handle` (a UUID), reported to agents as
  `project` by `project_info`, `status` and `proposals_list`. `tools::call` (the one entry the
  server uses) takes a `project` argument out of any call and refuses it when another project is
  open; `propose_cuts`, `propose_edit` and `undo_batch` require it, and proposals check it again
  under the command lock (`with_proposals(state, Some(handle), …)`), as the UI's commands do.
  `tools::takes_project` adds the argument to every writing tool's schema.
- **Tool hints.** `read_only` means nothing changes, files included; `tools::DESTRUCTIVE` lists
  the tools that overwrite without undo. Both become MCP `readOnlyHint` / `destructiveHint`.
  `propose_edit`'s schema is derived from `SequenceEdit`, so it always lists every edit kind.
- **Actions** apply directly like a button, one undo step each, listed in the Agent panel
  history. Detection actions take `apply: false`. Long jobs send MCP progress notifications.
  Requests on one connection run concurrently; agents that need order wait for each reply.
- **Tools on demand** (COST-2): rarely needed tools (the shorts tools, `faces`, `layouts`,
  `checkpoint`, ...) are behind `more`, which gives a tool's arguments or runs it, so every
  session's tool list stays small. Not `tools/list_changed`: Codex doesn't refresh a list
  mid-session. Long calls stop when the client cancels them or the user presses Stop in the
  Agent panel (`tools::Stop`, checked between steps).
- **The skill** (`mcp/skill/`: `SKILL.md`, `rules.md`, `export.md`, `graphics.md`) is compiled
  in; with the preset's method (`preset/methods/*.md` for the built-ins, `method.md` for the
  user's) it is served by the `preset` tool, as MCP prompts and server instructions, and installed
  into Claude Code / Codex skill folders (refreshed at launch while marked "Installed by
  AeroEdits"). The user's answers are the preset's `choices`, saved with `library`.
- **`library`** (`mcp/library.rs`, `mcp/presets.rs`): one tool for styles, presets and favourite
  layouts (`kind`, `op`), instead of a tool per kind and operation.
- **Events:** agent changes emit `aeroedits-project-updated`, `aeroedits-proposals-updated`,
  `aeroedits-agent-activity`, `aeroedits-agent-opened` so the UI refreshes.

## Graphics

`src-tauri/src/graphics/`. Decided in Graphics v1 (15 Oct; see [Product](product.md#settled-constraints)).

- **Asset:** a HyperFrames project in `graphics/<id>/` (`index.html`, `graphic.json` with
  variables, its own style if any, its own look tokens, length). Variables are declared with `data-composition-variables`.
- **Versions** (`versions.rs`, GFX-4): a version is a graphic of its own whose `graphic.json`
  names the family's first (`versionOf`); "use" swaps the clips' asset on the video's and every
  short's own timeline (clip ids stay, so a short's slots stay). Frame tools draw an unused
  version in place through `frames::trying`, which swaps a copy of the document.
- **Slots, not safe zones:** where a graphic plays decides its render box (whole canvas; a short's
  camera, screen or over-screen part). Full-screen graphics cut in and out (`data-entrance`); an
  overlay on the camera may ease.
- **Never chopped:** passes skip the time a graphic covers, a cut inside one is refused, cuts
  before it move it along.
- **Render** (`render.rs`): one background job at a time, pinned HyperFrames 0.8.228 with its own
  Node 22 and chrome-headless-shell (`runtime.rs`, hash-checked, ~200 MB download, ~500 MB on
  disk); WebM VP9 alpha decoded with `libvpx-vp9` forced; cache key over source files, canvas,
  length, fps, density, style (settings and files) and version. Every render of a graphic in an
  edit, the preview's too, is at the export settings' frame rate (60 at most), so a 1080p export
  plays the preview's renders; a larger export renders at twice the density (HyperFrames
  `--resolution landscape-4k` / `portrait-4k`, same layout). Whenever the edit changes, each
  graphic's export render is queued in the background behind the preview's
  (`render::request_export`; 2 Chrome workers, below-normal priority, fast capture), so an
  export of unchanged graphics starts writing frames at once (GFX-7). An export pins the renders
  it starts with (`render::pin`, hard links in a `.export-*` folder) and fails rather than play a
  placeholder. Renders stage in `<runtime>/work/<hash12>/` (long project paths broke Windows'
  260-char limit). An 8 s graphic at 1080p60 takes about 40 s on an RTX 3060.
- **The look** (`look.rs`): a style's graphics look (preset, five global controls, hand-set
  tokens; scaled at 9:16), injected into every render after the composition's own defaults as `--look-*`
  properties on `[data-composition-id]`, where HyperFrames puts the colour variables the roles
  mix from; part of the render hash. Find the root by `graphics::root_at` (the attribute with a
  value), never the first `data-composition-id` in the file: the look's selector comes first.
- **Styles** (`src-tauri/src/style/`): built in (Light, Dark) or app-level
  `<config>/AeroEdits/styles/<name>/style.json` (+ font and logo files). The brand becomes a
  `<style id="aeroedits-brand">` with `--brand-*` variables at render time (`style/brand.rs`);
  `style::dress` puts the frame and caption look onto a document as it is drawn (the
  `#[serde(skip)]` fields of `EditLayout` and `CaptionSettings`: never stored). Projects and
  graphics store a style's **name**, so a rename is recorded in `styles/renamed.json` and every
  lookup follows it. A save builds the new folder beside the old one and swaps it in.
- **Presets** (`src-tauri/src/preset/`): built in (Tutorial, Shorts, Clips) or app-level
  `<config>/AeroEdits/presets/<name>/` (`preset.json`, `method.md`, `rules.md`). A document is
  stored with `preset::stored` (the parts a preset sets written as `presetChanges`, a JSON merge
  patch over the preset's values) and read with `preset::read`, in `project.json` and in history
  states alike.
- **Agent file writes** (`write_file`): plain relative paths only, and no symlink or junction
  anywhere in the path (the graphic's folder included), like the project's `safe_path`.
- **Library** (`library.rs`) and **gate** (`gate.rs`): saving to the library runs a no-model check
  (lint, network refs, kit colours, layout overflow and contrast at 16:9 and 9:16 with long text,
  first/last-frame coverage, variable changes visible) in parallel headless Chromes, ~26–30 s, cached
  by source hash; pass → "template", fail → only "as is" at the user's word.

### Image library

Still pictures for graphics, built into the app (`src-tauri/src/image_library.rs`).

- **Files:** `src-tauri/assets/images/<look>/...` as WebP (SVG for flags, maps, wordmarks), and
  one `catalog.json`: the looks, topics and groups (with the words a beat matches on), and per
  picture its look, what it depicts, where it goes, size and licence. `build.rs` embeds every
  file. The repo is the only copy; `LICENSES.md` says where each set came from.
- **Looks:** one set per look, because a video keeps one look (GFX-5): `broadsheet` (faceless
  explainers, 245), `light`, `dark`, `editorial`, `neon` (the same 60 subjects each), and
  `sourced` (real flags, maps, wordmarks, portraits; shown with every look, marked `*`). A style
  takes the look whose `styles` list names it, else the nearest by background and accent colour.
- **Agent:** `library` `kind: image`: `list` (the project style's look unless `look`; `query`
  matches words), `show` (a board of a picture or a group), `use` (copies the picture into a
  graphic as `images/<look>-<name>`, one undo step).
- **Check:** flags a graphic carrying a picture that needs credit or a rights check
  (`ImageLicence`) or one from another look than its style's (`ImageLook`).
- **Adding a set:** made in Flova from the rules and approved tile in
  `src-tauri/assets/image-looks/README.md`, then `python scripts/add_images.py <look>=<folder>`
  (WebP quality 80, transparency 70, objects ≤ 1024 px, figures' feet at y=950).

## Check, storyboard, export

- **Exports** (`export/deliveries.rs`, DLV-2): one worker runs the queued exports in order; a
  busy owner refuses an export unless it is queued; every export that ends is recorded in the
  project's `exports.json` with a fingerprint of the document it was made from, so the list
  can tell what is older than the edit by content, not revision.
- **`check`** (`check.rs`) reads the selected video or short edit only (no decode, no model, ~3 ms): missing media, black
  frames, no sound, blips between cuts, words cut partway, flashing zooms, captions over the
  bubble, chapter rules, loudness not normalised, pending proposals. Safe
  fixes, each its own undo step.
- **`storyboard`** (`mcp/storyboard.rs`, agents only since 22 Oct 2026): a tile per scene (split
  where camera, zoom, graphic, chapter or a long silence starts; ≤ 48 tiles, ≤ 2048 px) plus a
  JSON line per tile.
- **Zoom glides** are timed on the edited video: `zoom::kept_stretches` gives each zoom the parts
  of it the edit keeps, and `zoom/eval.rs` eases in and out over those, so a cut through a glide
  never makes it jump (the old "zoom jumps" check and its fix are gone).
- **Export** (`export/mod.rs`): decode at the export fps, composite on the GPU, convert to NV12 on
  the GPU, encode with NVENC/AMF/QSV (one-frame test) or libx264; writer thread with a two-frame
  queue; the finished file is checked (duration, first/last frame). 1080p30 export ran at 2.6×
  real time (1.8× with jump cuts) on an RTX 3060.

## Performance rules

Rules that keep it fast (break one and it shows):

- Recording video crosses the decoder pipe as **NV12**; the shader converts. Imported files stay
  BGRA (they may carry alpha).
- Pixels live in a shared copy-on-write `PixelBuffer`; stills are decoded no larger than the
  preview needs.
- Lookahead decoders start before clip **and** segment boundaries (at most 4 at once).
- Clocks are built once per recording per summary (binary search + local merge), never per zoom.
- Timeline clips are memoised components, drawn only near the view; drags use an index map and a
  selection set, one redraw per animation frame. Neighbouring clips thinner than 3 px of one
  stream draw as one `ClipRun` (one waveform canvas across them), and cut marks under a pixel
  apart as one band. A press acts on the clip under the pointer; selected and missing clips
  stay on their own. A drag must not change a run's key: it holds the pointer capture.
- Panels reload only when what they depend on changes (sequence, transcript, their own settings),
  never on every revision.
- Long reads (transcript, cues, waveforms) run on the blocking pool, not Tauri's main thread.
- The preview worker never blocks on the JPEG encoders (fallback path; two threads take turns and
  a frame finishing after a newer one isn't shown); on Windows the preview is
  drawn by wgpu under a transparent webview.
- A graphic waiting for its render shows its newest older render, found once per render
  generation, not by listing the cache folder every frame.
- Sound-effect markers, like clips, are drawn only in the timeline's drawn time.
- `transcript_fix` changes all its words in one load and one save.

## Threads and locks: the window thread must never wait on us

A Tauri command without `async` runs **on the window's own thread** (Tauri 2). So do a few Tauri
calls made from other threads: `window_handle()` (and anything that asks the window for its size or
position) sends a message to that thread and waits for the answer. Together these make a deadlock
easy, and it froze the app whenever a short played in the window preview (fixed 22 Oct): the
playback engine held the `preview` lock while drawing, drawing asked Tauri for the window's handle,
and the window thread was waiting for the `preview` lock in the synchronous `preview_layout`
command. Windows then reports the app as not responding and closes it (`AppHangB1`).

- Never hold an app lock (`preview`, `playback`, `opened_project`, `command_lock`) across a call
  that may wait on the window thread, a draw, a decode or a file write.
- `if let Some(x) = state.preview.lock().f() { … }` keeps the guard for the **whole block**; take
  the value in its own statement first (`let x = state.preview.lock().f();`). `let … else` and plain
  `let` statements drop it at the end of the line.
- The window presenter (`render/present.rs`) gets the window's Win32 handle once, when it is made,
  and measures the window with `GetClientRect` on the drawing thread.
- Heavy synchronous commands belong in `async` + `spawn_blocking`.
- To see a hang: `cdb -pv -p <pid> -c "~*k 40; qd"` (Windows SDK debuggers) prints every
  thread's stack without stopping the app; look for two threads each waiting on what the other
  holds. A hang shows as a window that stops responding while the process's CPU time stays flat.

## Working on the code: notes that saved time

- The working tree uses **CRLF**, and Git Bash mangles backslashes (pipe names lost them twice).
  Edit source with an editor tool, not `sed`/heredocs; restore CRLF with
  `sed -i 's/\r\?$/\r/' <path>` if a shell rewrite slipped in.
- Don't run `cargo test`/`cargo build` while `cargo tauri dev` is building (shared `target/debug`
  race: a false "cannot find function run"). Stop the dev app before editing Rust: a save
  rebuilds and restarts it, ending a running agent turn or export.
- Test on **copies** of recordings from `Documents\AeroShootRec\*.aero` (delete `project.json` in
  the copy), never originals. Use your own session's `mcp__aeroedits__*` tools read-only; test write
  tools on a scratch project. To try one-click connect without touching real settings, start the
  dev app with `CODEX_HOME=<scratch>`.
- Test MCP with the official TypeScript SDK client: `StdioClientTransport({ command: <exe>, args:
  ["--mcp-stdio"] })`.
- Driving the dev app: page at `http://127.0.0.1:1420/`, DevTools on port 9222 with a separate
  `WEBVIEW2_USER_DATA_FOLDER`; the project handle is
  `useProjectStore.getState().openedProject.projectHandle`; webview screenshots via CDP
  `Page.captureScreenshot`.
- The local-LLM build needs a short `CARGO_TARGET_DIR` (`C:\aeb`), the Vulkan SDK, LLVM and Ninja;
  `scripts/build-windows.ps1` sets it up.
- After any docs change, run `.venv\Scripts\python.exe scripts/check_doc_links.py`.
- Shared helpers to reach for before writing one (QUAL-1): `ids::free_id` / `free_name` for a
  new id or name beside the ones in use (`base`, `base-2`...), `ids::is_plain_id` to check one;
  `shorts::video_sequence` for what a clip plays of the video now (its stretch or its pieces);
  `shorts::edited_span` for a source stretch on the edited timeline; `files::write_atomic`,
  `write_durable`, `save_json`, `load_json` for files replaced whole; `ranges::merge`,
  `merge_near`, `subtract` for stretches of time; `own_library` for a library's own files, tags,
  search and import; `proposals::load_file` / `save_file` in the agent folder;
  `telemetry::stream` for recorder streams and `read_once` for what a recording's files give;
  `commands::sounds::place_on_shown` and `on_shown` for a change on the timeline shown.
  In the window: `lib/format` (`formatTime`, `errorMessage`), `listenTo` for app events,
  `useBusyRun`, `useEscapeKey`, `DialogBackdrop` and `media/libraryParts`.
- Front-end behaviour tests are `front-end/tests/*.test.ts` (`npm run test:cleanup`, also run by
  CI): stores with a mocked `api`, no browser. A change of a store's async flow (queued saves,
  late replies) gets a test there.

## Recent implementation details

- **Graphic file history (SAFE-3):** content-addressed files participate in undo/redo; restore
  only the graphics changed by that step so unrelated manual edits remain untouched.
- **Room tone (SND-4):** `media/room_tone.rs` chooses the quietest eligible 0.4 s (at most
  -40 dBFS, excluding silence), plays it back and forth under muted noises, and falls back to
  silence when no candidate exists. Preview and export use the same path.
- **Plan (PLAN-1):** `plan` behind `more` reads/writes/edits the project's `plan.md`. Each edit
  match must be unique. Reads append current positions for referenced word IDs, including cut
  words, without rewriting the file. Status advertises it.
- **Style brief:** `style.json` includes motion, kinds, examples linked by library ID, rules,
  reference files/notes, breakdown, sound cues and portrait overrides. Motion uses existing
  look tokens. `library` style `propose` returns references and guidance without saving;
  save only the selected proposal. Style `check` checks references, fonts and library entries,
  but does not render examples. Website references must be fetched by the agent into files.
- **Graphic review:** `graphic_add` with `plan: true` validates and returns the table without
  adding. `frame_render` with `sample: true` writes an in-context clip with sound for the user,
  alongside the agent's sheet. Sampled frame checks flag face coverage, caption conflicts,
  portrait platform bands and long static holds; unavailable face tracks are “not checked”.
  These checks live in `mcp/graphic_checks.rs`. `findings` runs the conflicts across every
  graphic for `check`: 3 to 6 frames each, after up to 3 minutes for renders and face tracks, past
  which the report says what wasn't looked at. `check.rs` stays free of decoding, so `project_check`
  answers at once and `project_check_frames` follows in the panel; agents get both as one report.
  Wait for the current render before claiming a graphic fix is visible.
- **Retakes:** `transcript/edit.rs` cuts only what is nearly always a slip (a word said again after
  a 0.3 s pause with no comma, a word given up on before the same word whole). `ai/candidates.rs`
  flags more for a reviewer (`slips`: said twice, given up on, said wrong and corrected); a flag
  costs a look at its words, a wrong rule costs a word of the video. Words said for weight are
  `EMPHASIS_WORDS`.
- **Saved workflows (AUTO-3):** a preset with `exceptions`, `checks` and `approved`
  (`preset/workflow.rs`), not a second kind of thing. `preset/store.rs` counts a `version` on each
  save that changes something and keeps what it replaced in `versions/<n>/` (at most 50); restoring
  is a new version. The agent's guide carries the exceptions and checks. Agents ask for a guide by
  kind before they read `status`, so `preset` with a built-in's name gives the user's preset that the
  open project follows for that kind (`mcp/skill.rs` `meant`), saying so.
- **Seek-safe graphics:** all visible changes must follow deterministic timeline state. Avoid
  callback-driven text updates or measurements before HyperFrames sets the root height.
  Use transform-based movement; inspect the actual render and short-duration placements.
- **Timeouts (QUAL-7):** the agent pipe gives up after 10 s of a busy app; the AI client's test
  server bounds its waits. Do not filter libtest's notice of tests running over 60 s.

## Recorder contracts

These specifications are retained in full because a different application writes the files.
They are not subject to the editor's no-project-migration rule.

<a id="keyboard-telemetry"></a>
## Keyboard telemetry (`telemetry/keys.jsonl`), version 1

The contract between the AeroShoot recorder (writer) and AeroEdits (reader) for keyboard
activity in an `.aero` recording. It sits next to the mouse stream (`telemetry/events.jsonl`)
and uses the same clock.

**What the editor does with it:** typing and shortcuts count as screen activity, so the
webcam no longer fills the frame while you type with a resting mouse, and short scenes keep
the screen. Typing stretches become zoom targets. Shortcuts can show as a small badge
("Ctrl + Shift + P").

**Privacy rule:** the file never holds typed text. Typing is stored as stretches with a key
count only. Only shortcuts name a key, and only keys pressed with Ctrl, Alt or Win/Cmd held,
or F-keys.

### File

- Path: `<project>.aero/telemetry/keys.jsonl`. UTF-8, one JSON object per line, `\n` endings.
- Written incrementally and flushed on the same cadence as `events.jsonl`, so a crash loses at
  most the last flush. The editor ignores a truncated final line.
- No file means "keyboard not recorded": the editor falls back to mouse-only behaviour.
  The recorder writes no file when keyboard recording is switched off.
- Typical size: a few KB per hour.

### Common fields

Every record has:

| Field | Type | Meaning |
|---|---|---|
| `version` | `1` | Format version. The editor ignores the whole file for any other value. |
| `seq` | u64 | 0 for the first record, then +1 per record. Its own counter, separate from `events.jsonl`. |
| `t_us` | u64 | Microseconds on the recording clock: the same clock as `events.jsonl` `t_us` (0 = recording start after the countdown; paused time is included and written as a `gap`). |
| `kind` | string | One of the kinds below. The editor skips kinds it does not know. |

### Kinds

#### `capture` (exactly one, the first record)

What this recorder captures. Written at recording start (`t_us` 0, `seq` 0).

```json
{"version":1,"seq":0,"t_us":0,"kind":"capture","platform":"windows","shortcuts":true,"typing":true,"caret":true}
```

| Field | Type | Meaning |
|---|---|---|
| `platform` | `"windows"` \| `"macos"` | Lets the editor draw Win vs ⌘ and Alt vs ⌥. |
| `shortcuts` | bool | `shortcut` records are written. |
| `typing` | bool | `typing` records are written. |
| `caret` | bool | The recorder tries to report caret positions. Some apps never expose one, so `true` still allows records without them. |

#### `shortcut`

One shortcut press, at its key-down time.

```json
{"version":1,"seq":12,"t_us":8412000,"kind":"shortcut","modifiers":["ctrl","shift"],"code":"KeyP","label":"P","focus_inside_source":true}
```

| Field | Type | Meaning |
|---|---|---|
| `modifiers` | string[] | Held modifiers, in the order `ctrl`, `alt`, `shift`, `meta` (Win on Windows, Cmd on macOS). Left/right not distinguished. |
| `code` | string \| null | The physical key as a W3C `KeyboardEvent.code` (`KeyP`, `Digit1`, `Slash`, `Enter`, `F5`, `ArrowUp`, `Escape`, …). `null` for a modifier tapped alone. |
| `label` | string | What the key shows in the active layout, ready to draw: letters in upper case (`P`), symbols as typed without Shift (`/`), named keys in English (`Enter`, `Esc`, `Tab`, `Space`, `Backspace`, `Delete`, `Up`, `Down`, `Left`, `Right`, `Home`, `End`, `PgUp`, `PgDn`, `F5`). Empty for a modifier tapped alone. |
| `focus_inside_source` | bool, optional | The window receiving the key is in the recorded source: on the recorded display, or the recorded window itself. Omit if unknown. The editor ignores shortcuts with `false`. |

A key-down is written as a `shortcut` when, and only when:

1. a non-modifier key goes down while `ctrl`, `alt` or `meta` is held (Shift may be held too), or
2. F1–F24 go down (alone or with modifiers), or
3. `meta` is pressed and released with no other key in between, within 1 s (opening the
   Start menu or Spotlight). It is written at the release with `modifiers: ["meta"]`,
   `code: null`, `label: ""`.

Never a `shortcut`:

- **AltGr.** Windows reports AltGr as LeftCtrl plus RightAlt; keys typed with it are typing.
  This matters on Greek, German and other layouts.
- Shift plus a character (typing).
- Auto-repeat: a held key writes one `shortcut` at its first down only.
- The recorder's own hotkeys (start, stop, pause): nothing is written for them.
- Anything during the countdown or while paused.

#### `typing`

A run of typing, written once the run ends.

```json
{"version":1,"seq":13,"t_us":15230000,"kind":"typing","start_us":12010000,"end_us":15230000,"keys":38,"caret_start":{"geometry_id":"g1","norm_x":0.412,"norm_y":0.633},"caret_end":{"geometry_id":"g1","norm_x":0.598,"norm_y":0.633},"focus_inside_source":true}
```

| Field | Type | Meaning |
|---|---|---|
| `start_us` | u64 | Key-down time of the run's first key. |
| `end_us` | u64 | Key-down time of the run's last key (not the idle timeout after it). Equals `t_us`. |
| `keys` | u32 | Key-downs in the run, auto-repeat excluded. Never the keys themselves. |
| `caret_start`, `caret_end` | object, optional | The text caret at the first and the last key, when the OS reports one. `geometry_id`, `norm_x` and `norm_y` mean exactly what they mean in `events.jsonl` (normalized to the recorded source under that geometry). Omit when the caret is unknown or outside the source. |
| `focus_inside_source` | bool, optional | As for `shortcut`, taken at the first key. |

Typing keys: characters (with or without Shift or AltGr), `Space`, `Enter`, `Tab`,
`Backspace`, `Delete`, arrows, `Home`, `End`, `PageUp`, `PageDown` and `Insert`, pressed
without Ctrl, Alt or Win/Cmd.

- A run ends when no typing key goes down for **500 ms**, or at pause, stop, or a `gap`.
  The editor joins runs itself where it wants longer ones, so keep this short.
- A `shortcut` in the middle of a run does not end the run.
- A run with a single key is still written (`keys: 1`).
- Records stay in `seq` order. A `typing` record's `t_us` is its `end_us`, so a `shortcut`
  pressed during the run can have a later `seq` but an earlier `t_us` than the run. The
  editor sorts by time.

#### `gap`

A stretch where the keyboard state is unknown. The editor treats it as "might be typing",
never as "no typing". The exception is `hook_unavailable`: there the editor uses the mouse
alone, as if the keyboard had not been recorded, so a missing permission never blocks the
webcam for a whole recording.

```json
{"version":1,"seq":20,"t_us":61000000,"kind":"gap","reason":"recording_paused","start_us":42000000,"end_us":61000000,"dropped_events":0}
```

| `reason` | When |
|---|---|
| `recording_paused` | The recording was paused (written on resume or at stop). |
| `hook_unavailable` | The keyboard hook or event tap could not be installed or was removed. Written from that time to the end (or to when it came back). On macOS this includes missing Input Monitoring permission. |
| `queue_overflow` | Events were dropped between the hook and the writer. `dropped_events` is how many. |
| `secure_input` | macOS secure event input was on (password fields), so keys were not seen. |

### Platform notes for the writer

- **Windows:** `WH_KEYBOARD_LL` on the same kind of hook thread as the mouse hook
  (`capture/windows/record/mouse.rs`), with the same `RecordingClock`, pause flag and
  dropped-event counter pattern. `LLKHF_INJECTED` keys are recorded like any other key.
  Caret: `GetGUIThreadInfo` on the foreground thread (`rcCaret`, converted to screen
  coordinates, then normalized like the mouse). Many apps (Chromium, Electron) report no
  caret; then omit it.
- **macOS:** a listen-only `CGEventTap` for `keyDown` and `flagsChanged`, beside the mouse tap
  in `MouseHookMac.swift`. Caret: the focused element's `AXSelectedTextRange` bounds through
  Accessibility, when permitted; otherwise omit it.
- Labels come from the active layout at the time of the press (`ToUnicodeEx` with modifiers
  cleared on Windows, `UCKeyTranslate` on macOS).

### Versioning

New optional fields may be added to version 1; readers ignore fields they do not know.
A change to the meaning of an existing field, or to a rule above, needs `version: 2`.


<a id="window-telemetry"></a>
## Window telemetry (`telemetry/windows.jsonl`), version 1

The contract between the AeroShoot recorder (writer) and AeroEdits (reader) for the window in
front during an `.aero` recording. It sits next to the mouse stream (`telemetry/events.jsonl`)
and uses the same clock. `telemetry/geometry.jsonl` only describes the recorded display, so
the editor needs this file to know where the window you work in is.

**What the editor does with it:** when the front window is clearly smaller than the recorded
display (a big or second monitor), the screen is cropped to that window with a little room
around it, and the crop glides to the next window when you switch. Quick app switches,
popups, menus and tiny windows never move it. The user can switch it off for the whole video
or for any stretch.

**Privacy rule:** the file never holds window titles or window contents. Only an opaque id,
the executable name, the window's kind and its box.

### File

- Path: `<project>.aero/telemetry/windows.jsonl`. UTF-8, one JSON object per line, `\n`
  endings.
- Written incrementally and flushed on the same cadence as `events.jsonl`. The editor ignores
  a truncated final line.
- No file means "windows not recorded": the editor never crops. The recorder writes no file
  when it records a single window or an area (the crop only makes sense for a display).
- Typical size: a few KB per hour.

### Common fields

| Field | Type | Meaning |
|---|---|---|
| `version` | `1` | Format version. The editor ignores the whole file for any other value. |
| `seq` | u64 | 0 for the first record, then +1 per record. Its own counter. |
| `t_us` | u64 | Microseconds on the recording clock, the same clock as `events.jsonl` `t_us`. |
| `kind` | string | One of the kinds below. The editor skips kinds it does not know. |

### Kinds

#### `capture` (exactly one, the first record)

```json
{"version":1,"seq":0,"t_us":0,"kind":"capture","platform":"windows"}
```

#### `front`

From `t_us` on, this window is in front. Written at recording start, whenever the foreground
window changes, and when the front window's box changes (after a move or resize settles; at
most 4 per second while it is being dragged).

```json
{"version":1,"seq":7,"t_us":8412000,"kind":"front","window":"0x000A12F4","app":"Code.exe","role":"window","bounds":{"norm_x":0.052,"norm_y":0.041,"norm_w":0.611,"norm_h":0.808},"maximized":false}
```

| Field | Type | Meaning |
|---|---|---|
| `window` | string \| null | An opaque id, stable while the window lives (the HWND in hex on Windows, the CGWindowID on macOS). `null`: no app window in front on the recorded display (the desktop, or the front window is on another display). |
| `app` | string, optional | The executable or bundle id. Informational; the editor does not use it yet. |
| `role` | string | `window` (an ordinary top-level window), `dialog` (an owned window such as Save As), `fullscreen`, or a short-lived one: `popup`, `menu`, `tooltip`. The editor never crops to short-lived ones. |
| `bounds` | object | The window's visible frame (no invisible resize borders or drop shadow) normalized to the recorded source exactly like `norm_x`/`norm_y` in `events.jsonl`: `norm_x`, `norm_y` its top-left, `norm_w`, `norm_h` its size. It may reach past 0..1 where the window is partly off the display; the editor cuts it to the display. |
| `maximized` | bool, optional | The window is maximized (shown whole, never cropped to). |

#### `gap`

A stretch where the front window was not seen (the recording was paused, or the hook failed).
The editor keeps the crop it had.

```json
{"version":1,"seq":20,"t_us":61000000,"kind":"gap","reason":"recording_paused","start_us":42000000,"end_us":61000000}
```

### Platform notes for the writer

- **Windows:** `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)` plus `EVENT_OBJECT_LOCATIONCHANGE`
  filtered to the foreground window, on the hook thread beside the mouse hook. Bounds from
  `DwmGetWindowAttribute(DWMWA_EXTENDED_FRAME_BOUNDS)`, converted to the recorded display and
  normalized like the mouse. Role: `menu` for `#32768`, `tooltip` for `tooltips_class32`,
  `popup` for `WS_POPUP` windows without a caption, `dialog` for owned windows,
  `fullscreen` when the box covers the display.
- **macOS:** `NSWorkspace.didActivateApplicationNotification` plus the front window's
  `kAXPositionAttribute`/`kAXSizeAttribute` (or `CGWindowListCopyWindowInfo` layer 0), in the
  display's points converted like the mouse.

### Versioning

New optional fields may be added to version 1; readers ignore fields they do not know.
A change to the meaning of an existing field, or to a rule above, needs `version: 2`.
