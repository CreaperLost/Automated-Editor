# FFmpeg-Skill Research Report

- **Source:** [github.com/kajisho5/ffmpeg-skill](https://github.com/kajisho5/ffmpeg-skill)
- **Author:** Kajisho5
- **License:** Open Source
- **Category:** Agent Skill (`SKILL.md`) & MCP Server (42 Typed Python/FFmpeg Tools)
- **Analyzed for:** AeroEdits AI Agent & Motion Graphics Architecture

---

## 1. Executive Summary

**FFmpeg-Skill** (`kajisho5/ffmpeg-skill`) is an open-source Agent Skill and MCP server designed to eliminate one of the biggest failure modes in AI coding agents: **hallucinating complex, broken FFmpeg command lines**.

Instead of asking LLMs to construct fragile 100-character shell commands containing esoteric filtergraphs (`-filter_complex "[0:v]scale=...[v0];..."`), FFmpeg-Skill provides **42 strongly typed, modular Python tools** built exclusively with the Python standard library. It pairs these tools with two crucial architectural safeguards:
1. **`doctor` (`doctor --json`):** Environment diagnostic that verifies whether the local FFmpeg build contains required compiled-in filters (e.g. `subtitles`, `loudnorm`, `scale_cuda`) before an agent attempts to invoke them.
2. **`contract` (`contract --json`):** A machine-readable schema specifying exact parameters, types, and constraints for all operations.

---

## 2. Core Architecture & Diagnostics

```
┌─────────────────────────────────────────────────────────────┐
│                 Coding Agent (Claude / Codex)               │
└──────────────────────────────┬──────────────────────────────┘
                               │
       ┌───────────────────────┼───────────────────────┐
       ▼                       ▼                       ▼
1. `doctor --json`     2. `contract --json`    3. Invokes Typed Tools
   Probes local FFmpeg    Fetches exact schema   `fit.py`, `loudnorm.py`,
   build for filters      and constraints        `caption.py`, etc.
       │                       │                       │
       └───────────────────────┼───────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                   42 Typed Python Scripts                   │
│             (Standard library only, no pip deps)            │
├─────────────────────────────────────────────────────────────┤
│ • Editorial: cut, trim, concat, speed ramp                  │
│ • Reframing: 16:9 to 9:16 vertical, pad, blur background    │
│ • Audio: 2-pass EBU R128 loudness, BGM ducking, sync        │
│ • Motion & Text: ASS subtitles, karaoke overlays, titles    │
│ • Color: HDR to SDR tonemapping, LUT color grading          │
│ • Delivery: YouTube, Shorts, TikTok, Reels presets          │
└─────────────────────────────────────────────────────────────┘
```

### 2.1 The 42-Tool Catalog by Domain

| Category | Key Tools | Functionality |
| :--- | :--- | :--- |
| **Reframing & Layout** | `fit.py`, `pad.py`, `blur_bg.py`, `crop.py` | Converts between 16:9, 9:16, 4:3, and 1:1 with intelligent content fitting, letterboxing, or blurred background fills. |
| **Captions & Overlays** | `caption.py`, `karaoke.py`, `lower_third.py`, `burn_subtitles.py` | Generates and burns stylized ASS/SRT subtitles with active word highlights and animated title cards. |
| **Audio Engineering** | `loudnorm.py`, `duck.py`, `sync_audio.py`, `denoise.py` | 2-pass EBU R128 loudness normalization (-14 LUFS target), automatic background music ducking under dialogue, cross-camera waveform audio sync. |
| **Editorial & Assembly** | `cut.py`, `concat.py`, `speed.py`, `reverse.py` | Lossless stream-copy cuts where possible, frame-accurate re-encoding cuts, ripple assembly. |
| **Color & Visuals** | `lut.py`, `hdr2sdr.py`, `color_grade.py`, `vignette.py` | 3D LUT application, Dolby Vision/HDR10 to SDR tone-mapping via mobius/hable algorithms. |
| **Inspection & Quality** | `contact_sheet.py`, `detect_scenes.py`, `probe.py` | Generates multi-frame contact sheets for visual agent inspection; extracts stream metadata. |

---

## 3. Deep Dive: Key Technical Capabilities

### 3.1 The `doctor` Environment Probing System
A major challenge with FFmpeg is fragmentation across operating systems. A command working on Ubuntu may crash on macOS or Windows because the local FFmpeg build lacks `libass` (for subtitles) or `libmp3lame`.

FFmpeg-Skill includes a `doctor` command that runs automated checks:
```bash
npx ffmpeg-skill doctor --json
```
Output:
```json
{
  "ffmpeg_found": true,
  "ffprobe_found": true,
  "version": "7.0.1",
  "tools": {
    "caption.py": { "status": "ok", "requires": ["filter:subtitles", "libass"] },
    "loudnorm.py": { "status": "ok", "requires": ["filter:loudnorm"] },
    "hdr2sdr.py": { "status": "missing_filter", "requires": ["filter:zscale"], "error": "zscale filter not compiled in" }
  }
}
```
If an agent sees `"status": "missing_filter"`, it avoids calling that tool and picks a fallback path, preventing runtime crash loops.

### 3.2 The Machine-Readable `contract`
Instead of guessing parameters, the agent inspects the tool contract:
```json
{
  "name": "loudnorm.py",
  "description": "Applies two-pass EBU R128 loudness normalization.",
  "parameters": {
    "input": { "type": "string", "required": true },
    "output": { "type": "string", "required": true },
    "integrated_lufs": { "type": "number", "default": -14.0 },
    "true_peak": { "type": "number", "default": -1.0 },
    "loudness_range": { "type": "number", "default": 7.0 }
  }
}
```

### 3.3 Zero External Dependencies
Every tool in FFmpeg-Skill is implemented in vanilla Python 3.9+ using standard libraries (`subprocess`, `json`, `sys`, `pathlib`). There is no need for `pip install` or virtual environments.

---

## 4. AeroEdits vs. FFmpeg-Skill Comparison

| Feature | FFmpeg-Skill | AeroEdits |
| :--- | :--- | :--- |
| **Engine Nature** | Python script wrappers around system FFmpeg binary. | Native Rust backend with internal sequence model + embedded HyperFrames graphics engine. |
| **Execution Model** | Batch file-to-file processing (reads input file, encodes new file to disk). | Non-destructive memory timeline; real-time preview playback; fast export. |
| **Environment Check** | `doctor` probes host FFmpeg features dynamically. | Fully bundled, pinned binaries; zero dependency on host system setup. |
| **Audio Loudness** | `loudnorm.py` (two-pass EBU R128 via FFmpeg). | Built-in audio normalization to -14 LUFS during timeline mixing/export. |
| **Vertical Reframing** | `blur_bg.py` / `fit.py` via FFmpeg filtergraphs. | Layout engine (`layouts.md`, `shorts.md`): camera bubble, split-screen, active window crop. |
| **Contact Sheets** | `contact_sheet.py` tile grid. | `frame_render` tool (produces multi-moment inspection sheets for graphics and takes). |

---

## 5. Actionable Takeaways for AeroEdits

### 🟢 What We Should Adopt

1. **Adopt the `contract --json` Pattern for AeroEdits MCP Tools:**
   - AeroEdits exposes typed tools over MCP. Providing a unified capability discovery endpoint that details exact constraints, bounds, and defaults enables agents to self-correct arguments before sending invalid payloads.
2. **Standardized Background Music Ducking Protocol (`SND-2`):**
   - FFmpeg-Skill’s `duck.py` implements sidechain audio ducking (attenuating background music by 12–18 dB whenever voiceover is active, with 200ms attack and 500ms release curves).
   - AeroEdits should formalize this sidechain ducking curve for background audio tracks when mixed against microphone or system dialogue.
3. **Explicit Reframing Presets (`blur_bg` & `smart_fit`):**
   - For user-imported horizontal 16:9 media placed into 9:16 Shorts, offer an automated blurred background fill mode (`blur_bg`) as an alternative to simple black letterboxing or extreme punch-in crops.

### 🔴 What We Refuse / Avoid

1. **Do NOT delegate real-time timeline editing to file-based Python scripts:**
   - Running Python scripts that invoke FFmpeg for every cut, crop, or zoom is slow, non-interactive, and balloons disk space with intermediate files. AeroEdits’ non-destructive Rust timeline remains vastly superior.
2. **Do NOT rely on host FFmpeg builds:**
   - kajisho5 needs a complex `doctor` utility precisely because it relies on whatever random FFmpeg binary is installed on the user's host machine. AeroEdits bundles and pins its dependencies, guaranteeing 100% feature consistency across all user machines.
