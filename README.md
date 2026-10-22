# AeroEdits

AeroEdits is a desktop app for automated video editing, built for Windows and macOS. It opens
multi-track `.aero` recording projects (screen, webcam, microphone and system audio, plus
mouse telemetry) and edits them with smart zoom, silence cuts and layout presets before
exporting an MP4.

## Working with AI agents

AeroEdits has no built-in chat. AI comes from your own agent (Claude Code, Codex, the Claude
app…) over MCP or the `aeroedits do` command line. The app runs the editing passes; the agent
runs them in order, asks you with real numbers, and you approve. Agent proposals wait in the
Agent panel, and every change is one undo step.

![How the user's agent works with AeroEdits today](docs/images/ai-collaboration-today.svg)

Where this is heading: the agent moves from running passes to directing the edit, with a plan
you approve, a style trial, moving previews with sound, and rules it learns from your
corrections. This is a proposal from the 21 Oct 2026 research, not a commitment.

![The AeroEdits AI loop: today and tomorrow](docs/images/ai-loop-today-vs-tomorrow.svg)

Start with [docs/](docs/README.md) for the vision, the current state, the
gaps and the roadmap.

## Features

- **Multi-track timeline:** screen and webcam video, microphone and system audio waveforms,
  and a smart zoom track. Edits are non-destructive and support undo and redo.
- **Smart zoom from mouse telemetry:** reads `telemetry/events.jsonl` and
  `telemetry/geometry.jsonl` and suggests zooms and pans from clicks, dwell points and click
  clusters. Suggestions can be accepted, dismissed, edited or added by hand.
- **Jump cuts:** finds pauses in one sound or two chosen sources, then ripple-cuts the
  selected pauses from the timeline. Two-source mode preserves either voice (for example,
  microphone and voice-mode system audio). **Remove silence** finds very quiet audio,
  optionally protecting saved word timings. Balanced uses -42 dB, 250 ms pauses, and
  40 ms margins. Gentle, Tight, and Aggressive presets and the Advanced controls allow
  shorter gaps and margins down to zero. Automatic sensitivity is optional and shows
  the measured cutoff for each source; higher sensitivity can cut soft untranscribed speech.
  **Gaps without words** uses the primary source's saved transcript to suggest gaps
  containing breaths, saliva noises, or clicks, regardless of loudness. In two-source mode,
  the microphone is the default primary source; PC speech, music, video audio, and other
  sounds veto cuts. The PC transcript is optional and adds word protection when present.
  PC-only quiet footage never introduces cuts. Manual word/filler/retake editing keeps its
  existing behavior. Gap detection uses the chosen microphone margin, while PC sound
  protection keeps at least 80 ms regardless of the preset, including beyond trimmed
  or moved PC clip boundaries on the timeline. Optional word-edge refinement
  uses 5 ms audio windows to trim up to 200 ms of measured quiet at either word boundary,
  without changing saved transcript or caption timings. Leading/trailing audio stays
  protected. Sounds the transcriber tagged between words (`[keyboard clacking]`, `[laughs]`)
  are cut with the gap unless **Keep tagged sounds** is on. Suggestions
  start unchecked: transcription can miss words and tutorial clicks can be intentional.
  Noises inside recognized words are kept. **Transcript** editing removes selected words,
  fillers, and retakes. The retake rules also catch a small word said twice ("we we", "it's a
  it's a") and a word broken off ("j- just"); words doubled for weight ("very very", "pretty
  pretty") stay. Each pass has its own settings and review and can run independently,
  in any order. All cuts share timeline mapping and undo; timeline or transcript changes
  invalidate stale suggestions, including analyses still running.
- **Keyboard activity:** recordings from AeroShoot with keyboard recording on carry
  `telemetry/keys.jsonl` ([format](docs/contracts/keyboard-telemetry.md)): typing stretches with a key
  count (never the text) and shortcuts pressed. Typing counts as screen activity, so the
  picked layouts keep the screen in view while you type with a
  resting mouse. Long typing gets a zoom on the text caret (or the click into the field), and
  the zoom follows the caret. Shortcuts show as a badge ("Ctrl + S") in a corner of the screen,
  switched by **Show shortcut badges** in the canvas settings (`shortcutBadges` for agents),
  and `telemetry_get` lists both. Recordings without the file behave as before.
- **Style:** 16:9, 9:16, 4:3 and 1:1 canvases, wallpapers, padding, corner radius, shadows,
  and the camera's border, shadow and mirror. Every camera clip is centred on the face (found
  once per recording on this computer, YuNet) unless framed by hand or switched off.
- **Layouts** ([plan](docs/plans/layouts.md)): camera bubble (a 3x3 grid of spots or anywhere,
  Small/Medium/Large or any size, rect/circle/squircle), side by side (camera left or right,
  top or bottom in 9:16, any share, an even split or a card over the screen, the screen fitted
  or filled), camera only (with a punch-in) and screen only. Each timeline (the video, every
  short) has a base layout and sections that change it, anchored in the recording's time so
  cuts never move them; the base shows everywhere else. Changes cut between full-frame shots
  and on cuts, and move otherwise (or as each section says). Favourites are kept for every
  project. **Pick from the recording** fills sections in, no AI: the camera while the mouse
  rests as you talk, and in shorts the opening line and the screen while you click a lot;
  sections you set yourself stay. The timeline's Layouts lane shows them; agents use `layouts`.
- **Bubble out of the way:** when a click, drag, scroll, resting pointer or the typing caret
  lands where the webcam bubble sits (after layout, window crop and zoom), the bubble fades
  out for that stretch and back in afterwards; it never jumps to another corner. Activity near
  it keeps it hidden, and minimum hidden and shown times stop it flickering. On by default:
  **Hide while you work under it** in the Camera settings (`webcamAutoHide` for agents). Only
  while a bubble layout sits over a fitted screen.
- **Crop to the active window:** recordings from AeroShoot with window recording on carry
  `telemetry/windows.jsonl` ([format](docs/contracts/window-telemetry.md)). When the window in front is
  clearly smaller than the recorded display, the screen shows just that window with room
  around it and glides to the next one when you switch; quick switches, menus, popups and
  tiny windows never move it. Zooms work inside the crop and keep their targets, in shorts
  too. **Crop to the active window** in the canvas settings switches it for the video, and the
  Clip page switches it for one clip (`windowCrop`, `windowCropPaddingPct`, `windowCropOff`).
- **Sound effects:** a quiet lane of soft clicks on recorded mouse clicks and a light whoosh
  when a zoom goes in or out or the layout switches, with its own mute and volume on the
  Audio page and lowered under speech. Never busy: at most one click per 180 ms, a double
  click clicks once, no whoosh on small zooms or back-to-back moves, nothing inside a graphic
  or for a click a cut removed. The sounds are synthesized by the app, so there are no sound
  files to license (`soundEffects` in the audio settings for agents).
- **Mouth-click cleanup:** Audio → Reduce mouth clicks repairs short impulses on tracks
  marked Speech using FFmpeg's `adeclick`. Strength is adjustable; switch it off to hear
  the original. Playback and export share timing-preserving repaired audio, with undo/redo
  and unchanged original recordings and transcripts. The first preparation can take a few
  minutes for long recordings; temporary copies are cached for seeks and repeated clips.
  Requires FFmpeg with `adeclick`, including on macOS. Strong settings can soften consonants;
  this is general click repair, not a classifier for every mouth noise.
- **Loudness and cuts:** new projects normalize to YouTube's -14 LUFS. A leveller turns down
  only the 100 ms stretches whose peaks would reach the limiter, so quiet recordings reach the
  target (two real edits went from -18.4 and -19.4 LUFS to -14.3 and -14.7, peaks at -1 dBFS).
  A cut between two clips with sound to spare crossfades over 16 ms at equal power instead of
  dipping to silence; elsewhere it fades out and in.
- **Export:** H.264 video with AAC stereo audio at 720p, 1080p or 4K. Graphics render for the
  export: at its frame rate, and at twice the density (the same layout, sharper) for 1440p
  and 4K; the preview plays 30 fps drafts.
- **Shorts layouts:** in the Shorts Studio a short is split (camera above the screen) by
  default, with its own sections on its own timeline, as above. Agents use the `short_create`
  (a short from two word ids), `layouts` with `short` and `export` `short` actions, and a
  `shorts` template where they pick the moments themselves.
- **Shorts from a take:** a new camera take with the long video's footage, outside clips and
  graphics under it (the Shorts tab's "Take and what plays under it", or the agent's
  `short_footage`). Each graphic shows one of three ways, chosen as it is added so it renders
  once: **Full** (the whole frame), **Split** (your face above, the graphic below, footage or
  not) or **Over face** (your face fills the frame and the graphic is drawn over it, the
  templates as a panel at the bottom). Footage plays at its own **speed** (0.25x to 4x) to fit
  its words; trim by choosing a shorter stretch, jump-cut with several pieces; where it ends,
  the camera comes back. Its sound is only the video's background, never its microphone.
- **Check before export:** the export dialog lists what a viewer would notice, read from the
  edit itself (nothing is decoded): missing files, black frames, no sound, blips left between
  two cuts, words cut partway (beyond what word-edge refinement trims), many cuts close
  together, zooms that flash by or jump because a cut took their easing, captions over the
  webcam bubble, chapters that were cut or that YouTube would drop (fewer than 3, or one under
  10 s), loudness not normalized to YouTube's -14 LUFS, and agent proposals still waiting.
  Each finding jumps to its moment; safe ones have a Fix (cut the blip, close the gap, delete
  a zoom that flashes, move a zoom so its easing plays, move the captions clear, remove the
  chapter, normalize), each its own undo step.
  Agents run the same check with the `check` action.
- **Storyboard:** one small picture of the whole edit, a tile per scene: a finished frame
  (layout, webcam, zoom and graphics as exported) over its time, what it shows, the zoom, a
  graphic or chapter and a line of what is said. Scenes split where the camera or a short's
  scene, a zoom, a graphic, a chapter or a long silence starts; short stretches merge, and the
  sheet stays under 48 tiles and 2048 px. The export dialog shows it, and clicking a tile jumps
  there. Agents get the same sheet, with a JSON line per tile, from the `storyboard` action
  (`short` for a short's 9:16 one), instead of grabbing many frames.
- **Motion graphics:** the Media panel's Graphics page adds full-screen graphics from
  templates made for tutorials (section title, step card, keyboard shortcut, key point, recap
  checklist, end card) at the playhead, on a Graphics track, in a clean light app style
  (rounded cards, hairlines, soft shadows). A graphic plays over everything (screen, camera,
  captions) and cuts in and out (it eases only as an overlay over the full-screen camera), and
  cuts never go through one:
  silence, filler and other passes skip the time it covers, a cut wholly inside it is refused,
  and cuts before it move it along. Its text and colours are edited in the Inspector. Each
  graphic is a [HyperFrames](https://github.com/heygen-com/hyperframes) composition kept as
  source in `graphics/<id>/` (`index.html` and `graphic.json`) and rendered by the app in the
  background for the canvas it plays on, so the same graphic fits a 16:9 video and a 9:16
  short; renders are cached in `assets/graphics/` and export waits for them. The renderer
  (HyperFrames 0.8.162 with its own Node 22 and chrome-headless-shell, each pinned and checked
  by hash; npm packages installed with `npm ci` from the shipped lock file) is a 200 MB
  download on first use into the app's data folder; nothing on `PATH` is used or changed.
  Agents use the `graphics`, `graphic_add` (at a time or a transcript word), `graphic_set`,
  `graphic_write_file`, `graphic_render` and `graphics_install` actions.
- **One look for every graphic:** the Graphics page's Look section sets the video's look, a
  preset (App, Calm, Bold) and five sliders (size, roundness, speed, energy, density), with
  the brand kit's colours. Preview draws a few of the video's own graphics with a change before
  Apply (one undo step) restyles them all. Templates take every size, corner, gap and timing
  from the look's `--look-*` tokens; `graphic_check` flags a graphic's own colours, fonts,
  radii and timings. Agents use `graphics_look` (behind `more`).

## Platform status

| | Windows | macOS |
|---|---|---|
| Open and edit projects, smart zoom, silence detection | Yes | Yes |
| Export (H.264/AAC MP4) | Yes (FFmpeg) | Yes (Swift/AVFoundation bridge) |
| Preview and playback | Yes (FFmpeg, preview drawn in the app window, WASAPI audio) | Yes (Swift/AVFoundation bridge) |

The shared media backend (`src-tauri/src/media/ffmpeg.rs`) decodes and encodes through
FFmpeg on every platform. On macOS the Swift bridges in `src-tauri/native/macos/` stay the
default until the shared backend does everything they do; set
`AEROEDITS_MEDIA_BACKEND=ffmpeg` to try the shared one there.

Without the Swift preview view, the backend composites each preview frame at up to 1280px,
JPEG-encodes it, and the webview fetches it with the `preview_frame` command.

## Repository layout

```
front-end/                 React + Vite + TypeScript + Tailwind UI
  src/App.tsx              Main editor window
  src/components/          Timeline, canvas preview, inspector, waveform, silence dialog
  src/hooks/, src/stores/  Timeline hooks and zustand stores
  src/lib/                 IPC wrappers (one per backend command) and shared types
src-tauri/                 Tauri v2 backend (Rust)
  tauri.conf.json
  build.rs                 Builds the Swift bridges on macOS
  native/macos/            Swift preview, playback, media and export bridges
  src/commands/            Tauri command implementations
  src/dsp/                 Silence detection
  src/export/              Export pipeline
  src/fixtures/            Synthetic media and projects for tests
  src/media/               Frame model, decoder and encoder adapters
  src/playback/            Playback engine and native preview
  src/project/             Project reader, manifest, journal format, waveforms, edit revisions
  src/render/              wgpu compositor (composite.wgsl)
  src/telemetry/           Mouse telemetry reader
  src/timeline/            Timeline model and interval mapping
  src/zoom/                Smart zoom generator
```

## Development

### Prerequisites

- Node.js 22 (the version in `.nvmrc`), and npm
- Rust: rustup installs the version pinned in `rust-toolchain.toml` on first build.
  Then the Tauri CLI: `cargo install tauri-cli --version "^2"`
- **Windows:** Microsoft C++ Build Tools (the "Desktop development with C++" workload) and
  the WebView2 runtime, which ships with Windows 10 and 11.
- **macOS:** macOS 13 or newer with Xcode or the Command Line Tools, for the Swift bridges.
- **FFmpeg** (with `ffprobe`) for decoding and export on Windows and Linux. Either fetch the
  copy the app bundles (see [Bundled FFmpeg](#bundled-ffmpeg)) or install it: on Windows run
  `winget install Gyan.FFmpeg`, then open a new terminal. AeroEdits looks for it in
  `AEROEDITS_FFMPEG` and `AEROEDITS_FFPROBE`, then next to the app, then on `PATH`.

### Run the app

```bash
cd front-end
npm install

cd ../src-tauri
cargo tauri dev --features tauri-app
```

Without `--features tauri-app` the binary builds the editor core only, with no window.

### Installer builder scripts

The VibeRunner **Build** action calls a builder for the current platform:

| Platform | Command from the repository root | Installer output |
|---|---|---|
| Windows | `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\build-windows.ps1` | `C:\aeb\release\bundle\nsis\` (the short target folder the local model needs; with `-NoLocalLlm`, `src-tauri/target/release/bundle/nsis/`) |
| macOS | `bash scripts/build-macos.sh` | `src-tauri/target/release/bundle/dmg/` |
| Linux | `bash scripts/build-linux.sh` | `src-tauri/target/release/bundle/deb/` and `appimage/` |

Each script installs the locked frontend dependencies, including TypeScript, before
Tauri builds the frontend and packages the installer. Windows and Linux also fetch
and bundle FFmpeg. The prerequisites listed above must already be installed;
Linux additionally needs the native Tauri/WebKit and packaging dependencies.

The Windows builder enables CUDA local transcription by default
(`tauri-app,parakeet-cuda`). For CPU transcription, append `-Transcription cpu`
to the Windows command; for DirectML, append `-Transcription directml`.
CUDA runtime prerequisites are listed under Transcription below. The macOS and
Linux builders currently build without local transcription.

### Bundled FFmpeg

Release builds ship `ffmpeg` and `ffprobe` as Tauri sidecars, so users need no separate
install. Fetch them once per machine, then pass the extra config to Tauri:

```bash
cd src-tauri
node scripts/fetch-ffmpeg.mjs             # into src-tauri/binaries/ (git-ignored)
cargo tauri build --features tauri-app --config tauri.ffmpeg.conf.json
cargo tauri dev --features tauri-app --config tauri.ffmpeg.conf.json   # optional in dev
```

The script downloads the GPL 8.1 build from
[BtbN/FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds) for the Rust host target
(`--target <triple>` to cross-fetch; Windows and Linux, x64 and ARM64) and checks it against
the release's SHA-256 sums. FFmpeg runs as a separate program, so its GPL license covers
only the bundled binaries, whose license text and download source are installed with the app
under `licenses/`. `--lgpl` fetches the LGPL build instead (no libx264). macOS keeps using
the Swift bridges, with FFmpeg from Homebrew when the shared backend is forced.

Export uses a GPU H.264 encoder when one works on the machine (NVIDIA NVENC, then AMD AMF,
then Intel Quick Sync, each checked with a one-frame test encode), otherwise libx264. Set
`AEROEDITS_H264_ENCODER` to an FFmpeg encoder name, such as `libx264`, to force one.

### Transcription

Transcripts drive word-based editing: delete words to cut them, remove filler sounds, and
drop abandoned retakes. The language is English or Greek (**Language spoken** in
transcription settings); filler words, retakes, the AI review and caption capitals follow it.
Three providers are supported:

- **Parakeet (local, English only):** NVIDIA
  [Parakeet Unified EN 0.6B](https://huggingface.co/nvidia/parakeet-unified-en-0.6b) (an
  RNN-T) through ONNX Runtime. On FLEURS it makes 4.0% word errors where the multilingual
  Parakeet TDT 0.6B v3 used before made 4.8%, at the same speed. It is opt-in at build time,
  with one feature per accelerator:

  ```bash
  cargo tauri dev --features tauri-app,parakeet-cuda      # NVIDIA GPU (with the GPU pack below)
  cargo tauri dev --features tauri-app,parakeet-directml  # any DirectX 12 GPU on Windows, no extra installs
  cargo tauri dev --features tauri-app,parakeet-webgpu    # Metal on macOS
  cargo tauri dev --features tauri-app,parakeet           # CPU only
  ```

  Unified tidies speech, though: on a 15-minute tutorial it dropped stutters and repeats
  ("see see", "we can can") and six of nine "uh"s, which the filler and retake passes would
  have cut. So settings also offer **Verbatim**: Parakeet TDT 0.6B v3, which writes them down.

  Each model downloads from transcription settings (checked against its SHA-256): full
  precision (about 2.5 GB, fastest on the GPU) in CUDA builds, INT8 (about 0.7 GB) otherwise.
  Unified comes from
  [bobNight/parakeet-unified-en-0.6b-onnx](https://huggingface.co/bobNight/parakeet-unified-en-0.6b-onnx),
  Verbatim from
  [istupakov/parakeet-tdt-0.6b-v3-onnx](https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx).
- **ElevenLabs Scribe (cloud):** enter an API key in settings. It is stored in Windows
  Credential Manager or the macOS Keychain, never in the project; `ELEVENLABS_API_KEY`
  overrides it for development.
- **Scribe via OpenRouter (cloud):** the same Scribe v2, billed to the OpenRouter key.

Greek is transcribed online only: no local model is good enough yet (Parakeet v3 gets about a
third of Greek words wrong on FLEURS, Whisper large-v3 about 12%, Cohere Transcribe 9% but
without word times).

**NVIDIA GPU pack:** ONNX Runtime's CUDA provider (shipped with CUDA builds) needs CUDA 13 and
cuDNN 9. They are about 1.9 GB, so they are not in the installer: **Download** next to the
accelerator in transcription settings fetches NVIDIA's redistributable archives (each checked
against NVIDIA's SHA-256), keeps only the 14 DLLs into `%APPDATA%/AeroEdits/runtime`, and
loads them by full path before the first session. Without them everything runs on the CPU.

**Local AI review (fillers and retakes):** Qwen3.5 4B (Q4_K_M, 2.7 GB, Apache-2.0) runs inside
the app through llama.cpp on the GPU via Vulkan (the `local-llm` feature, on by default in
`scripts/build-windows.ps1`). It reads only the windows around what the rules flagged, in
English or Greek, answers under a JSON grammar and decodes greedily, so the same words always
give the same cuts. Building it needs the [Vulkan SDK](https://vulkan.lunarg.com/)
(`VULKAN_SDK`), LLVM (`LIBCLANG_PATH`), Ninja (`CMAKE_GENERATOR=Ninja`) in the Visual Studio
developer environment, and a short `CARGO_TARGET_DIR` such as `C:\aeb`: under
`src-tauri\target` llama.cpp's nested shader-generator build passes Windows' path limit. The
build script sets all of that up. Users need nothing, since Vulkan comes with GPU drivers. Choose it or OpenRouter / OpenAI under **Filler and retake review**.

Transcripts are saved per audio track in `<project>.aero/transcripts/<track id>.json`, with
word times in source time, so they stay valid across cuts and undo.

**Word alignment:** transcribers time words loosely. Parakeet uses an 80 ms grid and stretches
words over the pauses beside them; ElevenLabs sometimes runs a word over seconds of pause or
noise. After transcribing (either provider), AeroEdits re-times every word against the audio
with [wav2vec2-base-960h](https://huggingface.co/facebook/wav2vec2-base-960h) (Apache-2.0,
English, 20 ms frames, about 378 MB, downloaded from transcription settings). It runs on
Parakeet's ONNX Runtime, so it needs a `parakeet` build. Each stretch of speech is
force-aligned letter by letter, then each word's edges are snapped to where its sound starts
and stops. Guardrails keep it safe: a word moves at most 120 ms past where the transcriber put
it, words and stretches the model is unsure of keep their old times, and text, word ids and cuts
already made never change. **Align words** in the transcript panel (`align` for agents) aligns
transcripts made before. It is switched by **Align words to the audio** in transcription
settings.
Parakeet groups raw tokens into words without extending their speech span through
separately timed punctuation or whitespace, and keeps consecutive repeated words.
Its native token timestamp grid remains 80 ms; quiet-edge refinement helps existing
transcripts, but does not identify every mouth noise inside a spoken word.

### Tests

```bash
cd src-tauri
cargo test

cd ../front-end
npx tsc --noEmit
```

Media tests skip when FFmpeg is missing unless `AEROEDITS_REQUIRE_FFMPEG=1` is set, as CI
does. Tests whose names contain `gpu` need a GPU adapter and are ignored off macOS; run
them with `cargo test --lib gpu -- --include-ignored`. The native preview tests need macOS. On macOS, set `AEROEDITS_SKIP_SWIFT=1` to build against stubbed
Swift bridges.
