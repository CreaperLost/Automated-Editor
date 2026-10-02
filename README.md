# AeroEdits

AeroEdits is a desktop app for automated video editing, built for Windows and macOS. It opens
multi-track `.aero` recording projects (screen, webcam, microphone and system audio, plus
mouse telemetry) and edits them with smart zoom, silence cuts and layout presets before
exporting an MP4.

## Features

- **Multi-track timeline:** screen and webcam video, microphone and system audio waveforms,
  and a smart zoom track. Edits are non-destructive and support undo and redo.
- **Smart zoom from mouse telemetry:** reads `telemetry/events.jsonl` and
  `telemetry/geometry.jsonl` and suggests zooms and pans from clicks, dwell points and click
  clusters. Suggestions can be accepted, dismissed, edited or added by hand.
- **Silence detection:** finds dead air on the audio tracks and ripple-cuts it with
  user-chosen thresholds.
- **Layout:** 16:9, 9:16, 4:3 and 1:1 canvases, wallpapers, padding, corner radius, shadows
  and a styled webcam bubble.
- **Export:** H.264 video with AAC stereo audio at 720p, 1080p or 4K.

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

- Node.js 18 or newer, and npm
- Rust (stable) and the Tauri CLI: `cargo install tauri-cli --version "^2"`
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

### Bundled FFmpeg

Release builds ship `ffmpeg` and `ffprobe` as Tauri sidecars, so users need no separate
install. Fetch them once per machine, then pass the extra config to Tauri:

```bash
cd src-tauri
node scripts/fetch-ffmpeg.mjs             # into src-tauri/binaries/ (git-ignored)
cargo tauri build --features tauri-app --config tauri.ffmpeg.conf.json
cargo tauri dev --features tauri-app --config tauri.ffmpeg.conf.json   # optional in dev
```

The script downloads the LGPL 8.1 build from
[BtbN/FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds) for the Rust host target
(`--target <triple>` to cross-fetch; Windows and Linux, x64 and ARM64) and checks it against
the release's SHA-256 sums. The LGPL build keeps AeroEdits' MIT/Apache license intact; it has
no libx264, so Windows exports use Media Foundation (`h264_mf`) or OpenH264. `--gpl` fetches
the GPL build with libx264 instead, which is fine for personal builds but must be
distributed under the GPL. The license text and download source are installed with the app
under `licenses/`. macOS keeps using the Swift bridges, with FFmpeg from Homebrew when the
shared backend is forced.

### Transcription

Transcripts drive word-based editing: delete words to cut them, remove filler sounds, and
drop abandoned retakes. Two providers are supported, chosen in transcription settings:

- **Parakeet (local):** NVIDIA Parakeet TDT 0.6B v3 through ONNX Runtime. It is opt-in at
  build time, with one feature per accelerator:

  ```bash
  cargo tauri dev --features tauri-app,parakeet-cuda      # NVIDIA GPU (needs CUDA 12 and cuDNN 9)
  cargo tauri dev --features tauri-app,parakeet-directml  # any DirectX 12 GPU on Windows, no extra installs
  cargo tauri dev --features tauri-app,parakeet-webgpu    # Metal on macOS
  cargo tauri dev --features tauri-app,parakeet           # CPU only
  ```

  The model (about 670 MB, INT8) downloads from Hugging Face from the settings dialog, or
  can be placed by hand in any folder holding `vocab.txt`, `encoder-model*.onnx` and
  `decoder_joint-model*.onnx` from
  [istupakov/parakeet-tdt-0.6b-v3-onnx](https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx).
- **ElevenLabs Scribe (cloud):** enter an API key in settings. It is stored in Windows
  Credential Manager or the macOS Keychain, never in the project; `ELEVENLABS_API_KEY`
  overrides it for development.

Transcripts are saved per audio track in `<project>.aero/transcripts/<track id>.json`, with
word times in source time, so they stay valid across cuts and undo.

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
