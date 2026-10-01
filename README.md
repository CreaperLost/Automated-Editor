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
| Preview, playback and export | Not yet | Yes (Swift/AVFoundation bridge) |

Preview, playback and export currently go through the Swift bridges in
`src-tauri/native/macos/`. A shared media backend for both platforms is planned; the Swift
path stays until the shared one does everything it does.

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

### Run the app

```bash
cd front-end
npm install

cd ../src-tauri
cargo tauri dev --features tauri-app
```

Without `--features tauri-app` the binary builds the editor core only, with no window.

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

Some Rust tests need macOS (native preview) or a GPU adapter (compositor parity) and fail
on machines without them. On macOS, set `AEROEDITS_SKIP_SWIFT=1` to build against stubbed
Swift bridges.
