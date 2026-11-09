# AeroEdits

AeroEdits is a desktop app for automated video editing, built for Windows and macOS. It opens
multi-track `.aero` recording projects (screen, webcam, microphone and system audio, plus
mouse telemetry) and edits them with smart zoom, silence cuts and layout presets before
exporting an MP4.

## Working with AI agents

AeroEdits has no built-in chat. AI comes from your own agent (Claude Code, Codex, the Claude
app…) over MCP or the `aeroedits do` command line. The app runs the editing passes; the agent
runs them in order, asks you with real numbers, and you approve. Agent proposals wait in the
Agent panel, and timeline edits are undoable. Publish packages and production-stage records
are stored separately from edit undo.

The agent can keep a word-anchored project plan, trial a style in context and make
editable graphics. It reviews stills and boards; you judge motion, listen and approve export.

Start with [docs/](docs/README.md) for product direction, architecture, the roadmap and verification.

## Features

- Multi-track non-destructive editing, clip speed, linked cuts, persistent undo/redo and checkpoints.
- English/Greek transcription, noise/silence/filler/retake cleanup, audio mix and loudness.
- Telemetry-driven zooms, window crops, layouts, captions and editable HyperFrames graphics.
- Separate workflows for shorts from a fresh take and clips from the edited long video.
- Faceless researched explainers with generated narration, Broadsheet graphics and an image library.
- Video/subtitle/transcript/audio export, delivery history, YouTube packages and Zernio uploads.

The [Product](docs/product.md#what-is-built) document owns current capabilities and constraints;
[Architecture](docs/architecture.md) owns implementation and recorder contracts. Faceless/publish
paths still need real-production acceptance, and [gap.md](docs/gap.md) records known correctness
and performance limitations. Built features are not a claim that every path is verified.

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

### Headless Ubuntu / cloud sessions

Start with [AGENTS.md](AGENTS.md) and [HANDOFF](docs/HANDOFF.md). On a fresh Ubuntu VM:

```bash
bash scripts/cloud-setup.sh
bash scripts/check-env.sh
bash scripts/test-all.sh
```

The fixture smoke test renders through the library without a running GUI. Keep an output with
`AEROEDITS_SMOKE_OUT=/tmp/smoke.mp4`, then inspect it with
`bash scripts/snapshot-frames.sh /tmp/smoke.mp4 6`. Missing FFmpeg/adapter skips unless the
require flags in `.env.example` are set. No API key is required for tests. `aeroedits render
<project> --out x.mp4 --frames 6` exports a saved project with no app; `aeroedits do` connects to
the running editor.

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
npm ci

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

Transcripts are saved in the editor project at `transcripts/<asset>.<stream>.json`, with
word times in source time. The shared clock maps them through edits; repeated/muted speech
caption limitations remain GAP-022/023.

**Word timing:** forced alignment was removed on 22 Oct. There is no `align` tool or
wav2vec2 model download in the current source. Parakeet groups token spans without extending
them through punctuation/whitespace. Cleanup uses quiet-gap tightening and optional word-edge
refinement to protect spoken words; this is not forced alignment and does not retime every
saved word after transcription. See [Architecture](docs/architecture.md#transcription).

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
Swift bridges. Frontend behavior checks: `npm run test:cleanup`; production build: `npm run build`.
Doc links: `python scripts/check_doc_links.py --all` (file targets only; inspect changed anchors).
Recorded results and checks not yet performed are in [Verification](docs/verification.md).
