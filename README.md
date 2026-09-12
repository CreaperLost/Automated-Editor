# AeroShoot Video Editor

AeroShoot Video Editor is a dedicated post-production studio application for editing multi-stream recordings created by **AeroShoot Recorder**.

## Overview

AeroShoot Video Editor takes the recording bundles output by AeroShoot Recorder and provides an interactive timeline and canvas to customize, trim, autozoom, and export the finished video.

### Core Features

1. **Multi-Track Non-Destructive Timeline**:
   - Screen video track
   - Webcamera video track
   - Microphone audio track with PCM waveforms
   - System audio track with PCM waveforms
   - Smart Zoom keyframe track
2. **Telemetry-Driven Smart Zoom**:
   - Reads `telemetry/events.jsonl` and `telemetry/geometry.jsonl` recorded during capture.
   - Automatically analyzes mouse movements, dwell points, and click clusters to generate smooth camera-director style autozooms and pans.
   - Supports user editing, acceptance, dismissal, and manual keyframe creation.
3. **Studio Inspector & Layout Customization**:
   - Responsive canvas aspect ratios: 16:9, 9:16 (vertical / reels / shorts), 4:3, 1:1.
   - Beautiful canvas wallpapers: solid colors, gradient presets, custom images.
   - Window padding, rounded corner radius, drop shadows.
   - Webcamera bubble styling: circular, squircle, or rounded rect; position (corners or custom); size and shadows.
4. **AI Silence Detection & Cuts**:
   - Audio DSP silence detection across audio tracks.
   - Ripple cut dead air with user-guided thresholds.
5. **Video Export Engine**:
   - Hardware-accelerated H.264 video and stereo AAC audio MP4 export at 720p, 1080p, or 4K.

---

## Directory Structure

```
editor/
├── README.md               # This documentation
├── front-end/              # React + Vite + TypeScript + Tailwind CSS UI
│   ├── package.json
│   ├── vite.config.ts
│   └── src/
│       ├── App.tsx         # Main Editor application
│       ├── components/     # Timeline, Canvas, Inspector, Waveform, SilenceModal
│       ├── hooks/          # useTimeline, useWindowTitle
│       ├── stores/         # projectStore, editorSettingsStore
│       └── lib/            # IPC commands and types
└── src-tauri/              # Tauri v2 backend in Rust & Swift
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── build.rs            # Builds Swift preview, playback, and export bridges
    ├── native/macos/       # Swift bridges (AeroShootPreview, AeroShootPlayback, AeroShootExport, AeroShootMedia)
    └── src/
        ├── commands/       # Tauri command handlers for project editing and playback
        ├── capture/        # Record-scene preview surface (capture sources, native preview)
        ├── dsp/            # Silence detection DSP
        ├── export/         # Export pipeline (H.264/AAC muxer)
        ├── fixtures/       # Test-only fMP4/WAV generators
        ├── media/          # BGRA frame model, decoder/encoder adapters
        ├── playback/       # Real-time playback engine and preview coordinator
        ├── project/        # Project reader, manifest, waveform, pcm, and revisions
        ├── render/         # WGPU composite shader (composite.wgsl)
        ├── session/        # Session state machine, clock, bounded queues
        ├── telemetry/      # Mouse-telemetry reader for smart-zoom generation
        ├── timeline/       # Multi-track timeline model & interval cuts
        └── zoom/           # Smart zoom generator from mouse telemetry
```

---

## Moving to a Separate Repository

This folder is completely self-contained. To move it to its own Git repository:

```bash
# 1. Copy the editor folder to your desired destination
cp -r /path/to/AeroShoot.AI/editor /path/to/aeroshoot-editor

# 2. Initialize a new git repository
cd /path/to/aeroshoot-editor
git init
git add .
git commit -m "feat: initial commit of AeroShoot Video Editor"

# 3. Add your remote repository and push
git remote add origin git@github.com:your-username/aeroshoot-editor.git
git branch -M main
git push -u origin main
```

---

## Development Setup

### Prerequisites
- Node.js >= 18 and npm
- Rust toolchain (stable)
- macOS 13+ with Xcode / Command Line Tools (for Swift bridges)

### Running the Editor in Development
```bash
cd editor/front-end
npm install

cd ../src-tauri
cargo tauri dev
```
