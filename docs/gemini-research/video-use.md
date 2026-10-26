# Video-Use Research Report

- **Source:** [github.com/browser-use/video-use](https://github.com/browser-use/video-use)
- **Author:** Browser Use Team
- **License:** Open Source
- **Category:** Agentic Video Editing Skill & Pipeline (`SKILL.md` + Python + FFmpeg)
- **Analyzed for:** AeroEdits AI Agent & Motion Graphics Architecture

---

## 1. Executive Summary

**Video-Use** (`browser-use/video-use`) is an open-source Agent Skill and pipeline that empowers coding agents (Claude Code, Cursor, Codex) to perform end-to-end video editing through conversational natural language.

Instead of scrubbing video frames via costly vision tokens, Video-Use adopts an **audio-first, transcript-centric approach**. It ingests raw takes, transcribes speech down to word-level timestamps using **ElevenLabs Scribe**, and allows the agent to reason about cuts, retakes, and pacing directly from text. It pairs this reasoning with modular Python/FFmpeg scripts that apply 30ms anti-pop audio crossfades, segment-based color grading, stylized subtitles, and multi-engine animation overlays (Remotion, HyperFrames, Manim, PIL).

---

## 2. Architecture & Pipeline

```
Raw Video Footage (.mp4 / .mov)
         │
         ▼
[1. ElevenLabs Scribe Transcription] ──► Generates `takes_packed.md` (word-level timestamps)
         │
         ▼
[2. Agent Editorial Strategy] ─────────► Identifies filler words, false starts, retakes, dead air
         │                              Drafts plan; requests user approval (Human-in-the-loop)
         ▼
[3. Segment Processing Engine]
   ├── Audio Engineering ──────────────► 30ms cosine/linear crossfades at every cut boundary
   ├── Color Grading ──────────────────► ASC CDL chains applied per segment (warm, punchy, neutral)
   ├── Kinetic Subtitles ──────────────► 2-word uppercase synchronized chunks
   └── Graphic / Overlay Generation ───► Parallel sub-agents (HyperFrames, Remotion, Manim, PIL)
         │
         ▼
[4. Autonomous Self-Evaluation] ───────► Verifies cut boundaries, A/V sync, loudness
         │
         ▼
[5. Lossless Concat & Export] ─────────► Outputs `final.mp4` + persists session to `project.md`
```

---

## 3. Deep Dive: Key Technical Capabilities

### 3.1 Audio-First Token Efficiency
Feeding raw video frames into vision models is extremely token-heavy and slow. Video-Use converts hours of footage into a structured markdown document (`takes_packed.md`) with word-level timestamps. The agent can read 10 minutes of footage in ~3,000 text tokens, identify speech mistakes, retakes, and pauses, and plan cuts with mathematical precision before ever invoking an external tool.

### 3.2 The 30ms Anti-Pop Audio Crossfade
A universal problem in automated jump-cuts is "audio clicking" or "popping" when audio waveforms are cut abruptly at non-zero crossings. Video-Use systematically injects a **30ms audio fade/crossfade** (`afade=t=in:d=0.030`, `afade=t=out:d=0.030`) at every cut boundary. This guarantees clean, professional dialogue transitions without auditory artifacts.

### 3.3 Multi-Engine Motion Graphics & Overlays
Video-Use delegates motion graphics to specialized sub-agents depending on the subject matter:
- **Remotion / HyperFrames:** For web UI mockups, browser window recordings, and product feature cards.
- **Manim:** For mathematical formulas, algorithmic graphs, and educational diagrams.
- **PIL / Python Scripts:** For rapid static overlays, badges, and lower thirds.

### 3.4 Conversational Human-in-the-Loop Protocol
Video-Use codifies a strict protocol in its `SKILL.md`:
1. **Never make destructive edits without confirmation:** The agent analyzes the transcript and outputs an plain-English edit proposal (e.g. *"I found 3 false starts in Take 2 and 4.2 seconds of dead air at 01:14. Shall I cut them?"*).
2. **Autonomous Verification Before Delivery:** The agent inspects the final cut with `ffprobe` and sample frames to confirm that audio sync did not drift before declaring completion.
3. **Persistent State in `project.md`:** All editorial decisions, color choices, and subtitle preferences are logged in a project journal so subsequent sessions can build on previous work.

---

## 4. AeroEdits vs. Video-Use Comparison

| Capability | Video-Use | AeroEdits |
| :--- | :--- | :--- |
| **Interface** | Pure CLI / Agent chat; no visual GUI. | Desktop GUI app (Tauri + React) with multi-track timeline, waveform display, and Inspector. |
| **Cut Engine** | Python scripts executing complex FFmpeg filtergraphs. | Rust backend with native timeline sequence model; cuts are non-destructive and instantaneous. |
| **Telemetry** | None (pure audio/speech based). | **Rich mouse & keyboard telemetry** (`telemetry/events.jsonl`, `geometry.jsonl`, `keys.jsonl`). |
| **Smart Zooms** | Basic digital punch-ins. | Telemetry-driven smart zooms following clicks, dwell points, and typing carets. |
| **Audio Cuts** | 30ms FFmpeg audio fades on cut segments. | Timeline jump-cuts with configurable margins (40ms default) and PC audio veto. |
| **Graphics** | Dynamic sub-agents generating Remotion/Manim/PIL. | Embedded HyperFrames compositions with GSAP 3.15, global style tokens, and live preview. |

---

## 5. Actionable Takeaways for AeroEdits

### 🟢 What We Should Adopt

1. **Explicit 30ms Audio Seam Fades:**
   - While AeroEdits protects margins around words (40ms default), cutting between words can still occasionally hit non-zero waveform crossings. Adopting Video-Use’s strict **30ms micro-fade / equal-power crossfade** at every ripple-cut boundary guarantees broadcast-clean dialogue playback with zero pop/click noise.
2. **The `takes_packed.md` Transcript Compact Pattern for Agents:**
   - In AeroEdits, agents currently retrieve transcripts via `transcript_get`. Structuring the transcript into a compact, phrase-and-take overview allows the agent to grasp the entire flow of a 15-minute tutorial in a single prompt, drastically reducing context window overhead.
3. **Multi-Discipline Graphics Routing (HyperFrames + SVG/Diagrams):**
   - For mathematical or algorithmic explanations in tutorials, provide agent templates or generators that output clean SVG path animations, expanding beyond standard UI cards.

### 🔴 What We Refuse / Avoid

1. **Do NOT abandon non-destructive timeline editing for destructive FFmpeg script chops:**
   - Video-Use cuts video by running FFmpeg commands that re-encode or split files on disk. AeroEdits' native sequence model is non-destructive, instantaneous, and allows multi-level undo/redo without disk bloat or re-render latency.
2. **Do NOT rely on cloud transcription APIs (ElevenLabs Scribe) as a hard dependency:**
   - Video-Use requires an external paid API key for ElevenLabs. AeroEdits supports local offline transcription (Whisper.cpp / local models), preserving user privacy and offline operation.
3. **Do NOT discard telemetry:**
   - Video-Use operates solely on voice. AeroEdits’ superpower is combining voice with mouse dwell, typing carets, and active window bounds.
