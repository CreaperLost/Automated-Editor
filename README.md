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
  without changing saved transcript or caption timings. Leading/trailing audio and
  recognized audio events stay protected. Suggestions
  start unchecked: transcription can miss words and tutorial clicks can be intentional.
  Noises inside recognized words are kept. **Transcript** editing removes selected words,
  fillers, and retakes. Each pass has its own settings and review and can run independently,
  in any order. All cuts share timeline mapping and undo; timeline or transcript changes
  invalidate stale suggestions, including analyses still running.
- **Layout:** 16:9, 9:16, 4:3 and 1:1 canvases, wallpapers, padding, corner radius, shadows
  and a styled webcam bubble.
- **Mouth-click cleanup:** Audio → Reduce mouth clicks repairs short impulses on tracks
  marked Speech using FFmpeg's `adeclick`. Strength is adjustable; switch it off to hear
  the original. Playback and export share timing-preserving repaired audio, with undo/redo
  and unchanged original recordings and transcripts. The first preparation can take a few
  minutes for long recordings; temporary copies are cached for seeks and repeated clips.
  Requires FFmpeg with `adeclick`, including on macOS. Strong settings can soften consonants;
  this is general click repair, not a classifier for every mouth noise.
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

### Installer builder scripts

The VibeRunner **Build** action calls a builder for the current platform:

| Platform | Command from the repository root | Installer output |
|---|---|---|
| Windows | `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\build-windows.ps1` | `src-tauri/target/release/bundle/nsis/` |
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
