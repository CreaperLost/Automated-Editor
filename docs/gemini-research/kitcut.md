# KitCut Research Report

- **Source:** [github.com/kitcut-hq/kitcut](https://github.com/kitcut-hq/kitcut) | [kitcut.ai](https://kitcut.ai)
- **Author:** KitCut HQ
- **License:** Open Source
- **Category:** Timeline-Free Local Video Post-Production & MCP Server
- **Analyzed for:** AeroEdits AI Agent & Motion Graphics Architecture

---

## 1. Executive Summary

**KitCut** (`kitcut-hq/kitcut`) is an open-source, local-first post-production system designed to eliminate the manual timeline editor entirely. Connected to AI coding agents (Claude Code, Cursor, ChatGPT) via the Model Context Protocol, KitCut acts as an autonomous digital assistant that ingests multi-device session media, syncs multicam feeds, cuts dead air, blurs sensitive personal data (PII), burns captions and graphics, and prepares vertical shorts and exports.

Its key architectural philosophy is **"Timeline-Free & Asset-Centric"**: creators speak or prompt their intent, and the agent executes structured post-production pipelines backed by GPU hardware acceleration (NVENC, AMF, QSV) and an immutable markdown audit trail (`journal.md`).

---

## 2. Core Architecture & Pipeline

```
[Local Disks & Connected Phones]
                │
                ▼
      [1. Asset Ingestion Engine]
         • Auto-discovers new session takes
         • De-duplicates identical files
         • Aligns chronologically via shoot timestamps & audio sync
                │
                ▼
      [2. Agent MCP Orchestration]
         • Connected via MCP stdio/HTTP
         • Maintains `project.json` (state) & `journal.md` (audit)
                │
                ▼
      [3. Smart Production Passes]
         ├── Multicam Speaker Switching (audio-driven angle cuts)
         ├── PII & Privacy Redaction (face/screen/text blurring)
         ├── Kinetic Captions & Chapter Marker Generation
         ├── Vertical Reframing (horizontal 16:9 $\to$ 9:16 Shorts)
         └── Graphics & Lower-Thirds Composition
                │
                ▼
      [4. Review Stops & Checkpoints] (Human approval before export)
                │
                ▼
      [5. Hardware-Accelerated Export & Direct Publish (YouTube)]
```

---

## 3. Deep Dive: Key Technical Capabilities

### 3.1 Automated Multi-Device Ingestion & Audio Sync
KitCut recognizes that modern creators record on multiple devices simultaneously (e.g. OBS on PC, high-res selfie camera on an iPhone, podcast mic on an audio recorder). 
Instead of requiring manual import and alignment:
1. KitCut scans designated project folders and attached mobile devices.
2. It groups media by session timestamp and hashes file headers to eliminate duplicates.
3. It performs cross-correlation waveform analysis on audio tracks to align camera and microphone feeds down to the sample level.

### 3.2 Smart Post-Production Passes

#### A. Automated PII & Sensitive Information Blurring
Tutorial and screencast creators constantly risk exposing API keys, email addresses, passwords, and client names. KitCut integrates an automated redaction pass that detects sensitive text strings and patterns (regex / OCR) or user-specified regions, applying tracking blurs across the timeline.

#### B. Audio-Driven Multicam Speaker Switching
For two-person interviews or multi-camera setups, KitCut analyzes individual microphone tracks. When Speaker A talks, the pipeline cuts to Camera A; when both speak or pause, it defaults to a wide or split-screen shot.

#### C. State & Journal Architecture (`project.json` + `journal.md`)
Every video project is backed by two files:
- `project.json`: Machine-readable snapshot of timeline clips, effects, and assets.
- `journal.md`: Human-readable markdown audit log documenting every action taken by the agent (e.g. *"14:22 - Trimmed 3.2s silence; applied warm LUT; blurred email on frame 412"*).

### 3.3 Hardware-Accelerated Local Encoding
To avoid slow CPU rendering bottlenecks, KitCut probes the host machine for hardware encoders:
- NVIDIA NVENC (`h264_nvenc`, `hevc_nvenc`)
- AMD AMF (`h264_amf`)
- Intel QuickSync QSV (`h264_qsv`)
- Apple Silicon VideoToolbox (`h264_videotoolbox`)
This delivers near real-time rendering on consumer laptops and desktop workstations.

---

## 4. AeroEdits vs. KitCut Comparison

| Capability | KitCut | AeroEdits |
| :--- | :--- | :--- |
| **User Interface** | Timeline-free; driven purely by agent chat and preview windows. | Hybrid: Professional desktop GUI with multi-track timeline + AI Agent panel. |
| **Input Material** | Multi-device video files, phone recordings, separate audio files. | Native multi-track `.aero` recording bundle (screen, webcam, mic, system audio, mouse & keyboard telemetry). |
| **Telemetry Awareness**| None (blind video/audio processing). | **Deep telemetry integration** (clicks, mouse dwell, active window, caret typing). |
| **Redaction / Blurring**| Built-in PII and text blurring pass. | Active window crop and camera auto-hide (`webcamAutoHide`); roadmap item `FX-2` (spotlight/redaction). |
| **Motion Graphics** | Burned static cards and labels. | Embedded **HyperFrames + GSAP 3.15** motion engine with responsive 16:9 / 9:16 layout slots. |
| **History & Undo** | `journal.md` audit trail + `project.json`. | Full non-destructive undo/redo surviving app restarts (`history/`, up to 2GB) + named checkpoints. |

---

## 5. Actionable Takeaways for AeroEdits

### 🟢 What We Should Adopt

1. **Automated PII & Secret Redaction Pass (`FX-2` Acceleration):**
   - In screencast and coding tutorials, creators frequently expose `.env` secrets, tokens, or browser cookies.
   - AeroEdits should introduce an agent-driven redaction tool (`redact_region` or `redact_text`). Because AeroEdits knows the active window coordinates from telemetry, the agent can easily apply a localized mosaic or blur over designated coordinate rectangles across a time window.
2. **The `journal.md` Creator Audit Trail:**
   - In addition to binary checkpoint snapshots, write a high-level, human-readable `journal.md` inside the `.aero` project folder. When a creator reviews an edit after an agent run, they can read a clean list of exactly what cuts were made, what silences were trimmed, and what graphics were added.
3. **Hardware Acceleration Auto-Detection:**
   - Ensure AeroEdits' export pipeline auto-detects macOS VideoToolbox and Windows NVENC/AMF/QSV for lightning-fast export times on high-resolution 4K timeline exports.

### 🔴 What We Refuse / Avoid

1. **Do NOT throw away the visual timeline:**
   - KitCut takes a radical "no timeline" stance. However, professional creators demand visual verification: seeing waveforms, clip boundaries, and graphic overlays on a timeline is indispensable for confidence. AeroEdits’ dual approach (agent automation + visual timeline) is far more practical.
2. **Do NOT compromise determinism:**
   - KitCut relies heavily on ad-hoc scripts and external encoders. AeroEdits’ strict architecture (render hashes, pinned HyperFrames runtime, deterministic WebM caches) guarantees that an edit looks identical regardless of machine or OS.
