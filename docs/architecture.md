# Architecture: how AeroEdits is built

The system as it stands on 21 Oct 2026, condensed from the finished plans (Premiere-style
tracks, the 4 Oct performance pass, AI readiness phases 0–3, layouts). Repository
layout, build and test commands are in the [root README](../README.md). Why things are the way
they are is in [decisions.md](decisions.md).

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
`agent/proposals.json`, `graphics/<id>/` and caches. Recordings play from their own folders,
mounted at `recordings/<asset id>/`.

- **Assets and streams.** Everything playable is an asset (`Recording`, `Video`, `Image`, `Audio`,
  `Graphic`) with streams (`screen`, `camera`, `audio:N`, `picture`). All streams of an asset share
  one clock. The project's own recording is just the first Recording asset. Stream keys are
  `<asset>.<stream>`.
- **Tracks and clips.** Video tracks V1… and audio tracks A1…, all the same kind of thing. A clip
  is `{source: StreamRef, start_us, in_us, duration_us, link}`. Video clips never carry sound;
  sound is a linked clip on an audio track. Track roles: Screen, Camera, Overlay (video); Speech,
  Background (audio). Mix (volume, denoise, ducking) is per audio track; clips have no gain or fades.
- **Magnetic** ripples every unlocked track; off, cuts leave gaps and moves overwrite. Empty time
  is black.
- **Edits** are a typed `SequenceEdit` enum (17 kinds: add/move/remove track, place asset, move,
  trim, ripple trim, split, delete, delete range, link, unlink, set clip, set magnetic, set speed,
  restore cuts…), applied to a copy and committed as one revision (`project/revision.rs`).
- **Clocks: one mapping primitive.** `sequence/clock.rs` builds, from the clips that play a stream
  or asset, a map between source time and timeline time (a moment can appear twice; the earlier
  clip owns it). Transcripts, captions, word cuts, silence suggestions, zooms, layout sections,
  chapters and shorts all live in **source time** and are mapped through a clock on read, so cuts,
  reorders and undo never move them off their content.
- **Revisions and undo.** Every edit bumps `revision`; edits pass `expected_revision` and a
  mismatch is refused ("Stale edit revision"). `EditHistory` keeps up to 64 undo steps **in
  memory only** (gap: [SAFE-2](roadmap.md#safe-2)).
- **Shorts** are windows anchored in source time that follow the video's edits until edited on
  their own (`ShortEdit`, an own sequence); take shorts carry a take (camera + voice) with footage
  from the video under it (`take_short.rs`). Their own layout tracks, captions and graphic slots.

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
  no-footage (take shorts). Plan and follow-ups: [plans/layouts.md](plans/layouts.md).
- **Zooms** from clicks, dwell, click clusters and typing (`zoom/`), eased in recording time.
- **Telemetry** (`telemetry/`): `events.jsonl` + `geometry.jsonl` (mouse), `keys.jsonl`
  ([contract](contracts/keyboard-telemetry.md)), `windows.jsonl`
  ([contract](contracts/window-telemetry.md)). Drives zooms, layout picks, window crop, bubble hide,
  shortcut badges and the click/whoosh lane (`sound_effects.rs`).
- **Faces:** YuNet tracks per camera asset in source time; `focus_uv` frames each camera slot on
  the face, lining up every 30 s rather than following; per-clip hand framing overrides.

## The agent connection (MCP)

`src-tauri/src/mcp/`. Full history and measurements in [plans/ai-readiness.md](plans/ai-readiness.md).

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
- **The skill** (`mcp/skill/`: `SKILL.md`, `export.md`, `templates/{tutorial,shorts,take-shorts}.md`)
  is compiled in, served by the `template` tool, as MCP prompts and server instructions, and
  installed into Claude Code / Codex skill folders (refreshed at launch while marked "Installed
  by AeroEdits"). Saved answers: `<config>/AeroEdits/agents/answers/<template>.json`.
- **Events:** agent changes emit `aeroedits-project-updated`, `aeroedits-proposals-updated`,
  `aeroedits-agent-activity`, `aeroedits-agent-opened` so the UI refreshes.

## Graphics

`src-tauri/src/graphics/`. Decided in Graphics v1 ([ai-readiness](plans/ai-readiness.md#graphics-v1-agreed-2026-10-15)).

- **Asset:** a HyperFrames project in `graphics/<id>/` (`index.html`, `graphic.json` with
  variables, brand, length). Variables are declared with `data-composition-variables`.
- **Slots, not safe zones:** where a graphic plays decides its render box (whole canvas; a short's
  camera, screen or over-screen part). Full-screen graphics cut in and out (`data-entrance`); an
  overlay on the camera may ease.
- **Never chopped:** passes skip the time a graphic covers, a cut inside one is refused, cuts
  before it move it along.
- **Render** (`render.rs`): one background job at a time, pinned HyperFrames 0.8.162 with its own
  Node 22 and chrome-headless-shell (`runtime.rs`, hash-checked, ~200 MB download, ~500 MB on
  disk); WebM VP9 alpha decoded with `libvpx-vp9` forced; cache key over source files, canvas,
  length, fps, density, brand and version. The preview plays a **draft** (30 fps, 1920 px long
  side); an export renders **final** ones at its own fps, and at twice the density (HyperFrames
  `--resolution landscape-4k` / `portrait-4k`, same layout) when larger than 1080p. Renders stage in `<runtime>/work/<hash12>/` (long project paths
  broke Windows' 260-char limit). Export waits for pending renders. A render takes ~16–28 s.
- **Brand kits** (`brands.rs`): app-level `<config>/AeroEdits/brands/<name>/brand.json` (+ logo,
  font file) → a `<style id="aeroedits-brand">` with `--brand-*` variables at render time.
  Projects and graphics store a kit's **name**, so a rename is recorded in `brands/renamed.json`
  (old → new, chains collapsed) and every lookup (`load`, `current_name`) follows it; deleting a
  kit drops its old names. A save builds the new folder beside the old one and swaps it in, the
  old one set aside and put back if the swap fails; a non-kit folder in the way is refused.
- **Agent file writes** (`write_file`): plain relative paths only, and no symlink or junction
  anywhere in the path (the graphic's folder included), like the project's `safe_path`.
- **Library** (`library.rs`) and **gate** (`gate.rs`): saving to the library runs a no-model check
  (lint, network refs, kit colours, layout overflow and contrast at 16:9 and 9:16 with long text,
  first/last-frame coverage, variable changes visible) in parallel headless Chromes, ~26–30 s, cached
  by source hash; pass → "template", fail → only "as is" at the user's word.

## Check, storyboard, export

- **`check`** (`check.rs`) reads the edit only (no decode, no model, ~3 ms): missing media, black
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

## Performance baseline and rules

From the 4–5 Oct pass (R01–R20; all of the earlier P01–P07 findings fixed). The benchmark is
`src-tauri/src/perf.rs` (needs `AERO_PROBE=<dir>` with a real recording).

11-minute recording, release build:

| Operation | Time |
|---|---|
| Open the project | 52 ms |
| First waveform, mic / system | 155 / 220 ms (8 ms cached) |
| Find pauses (179) / cut them all | 106 ms / 8 ms |
| An edit or undo | 12 ms |
| Rebuild the preview after an edit | ~7 ms |
| Build the audio mixer, all effects, first / again | 122 / 13 ms |
| Seek | 65 ms typical, 170 ms p95 |

Rules that keep it fast (break one and it shows):

- Recording video crosses the decoder pipe as **NV12**; the shader converts. Imported files stay
  BGRA (they may carry alpha).
- Pixels live in a shared copy-on-write `PixelBuffer`; stills are decoded no larger than the
  preview needs.
- Lookahead decoders start before clip **and** segment boundaries (at most 4 at once).
- Clocks are built once per recording per summary (binary search + local merge), never per zoom.
- Timeline clips are memoised components, drawn only near the view; drags use an index map and a
  selection set, one redraw per animation frame.
- Panels reload only when what they depend on changes (sequence, transcript, their own settings),
  never on every revision.
- Long reads (transcript, cues, waveforms) run on the blocking pool, not Tauri's main thread.
- The preview worker never blocks on the JPEG encoder (fallback path); on Windows the preview is
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
  race: a false "cannot find function run"). Stop the dev app before editing Rust.
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
- Front-end behaviour tests are `front-end/tests/*.test.ts` (`npm run test:cleanup`, also run by
  CI): stores with a mocked `api`, no browser. A change of a store's async flow (queued saves,
  late replies) gets a test there.
