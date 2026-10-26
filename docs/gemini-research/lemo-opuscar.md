# Lemo-Opuscar Research Report

- **Source:** [github.com/lemomo-ai/lemo-opuscar](https://github.com/lemomo-ai/lemo-opuscar)
- **Author:** Lemomo AI
- **License:** Open Source
- **Category:** Code-Driven Animation & Procedural Film Studio (Canvas + WebGL + FFmpeg)
- **Analyzed for:** AeroEdits AI Agent & Motion Graphics Architecture

---

## 1. Executive Summary

**Lemo-Opuscar** (`lemomo-ai/lemo-opuscar`) is an open-source animation and mini-film studio skill designed for AI coding agents (specifically Claude Code powered by Claude Opus). 

Instead of relying on AI video diffusion models (Sora, Runway Gen-3, Pika)—which produce hallucinated, non-deterministic video clips that cannot be edited or adjusted—Lemo-Opuscar builds films **entirely through code**. The agent acts as an autonomous director, writing programmatic scenes using **HTML5 Canvas, WebGL, and Three.js**, capturing frame sequences via a headless browser (Puppeteer/Playwright), and compiling them alongside local offline text-to-speech (TTS) and music using FFmpeg.

Its standout feature is an extensive catalog of **procedural artistic styles** (ranging from Chinese Ink Wash and Watercolor to Whiteboard Explainers and 3D Game Cinematics).

---

## 2. Core Architecture & Pipeline

```
[User Story / Topic Brief] ──► [Agent Director (Claude)]
                                      │
                                      ▼
                      [1. Storyboard & Shot Planning]
                                      │
                                      ▼
                 [2. Procedural Code Generation (HTML5/WebGL)]
                   • HTML5 Canvas 2D procedural rendering
                   • Three.js 3D meshes & camera animation
                   • Custom GLSL shaders for artistic styles
                                      │
                                      ▼
                 [3. Frame Capture (Headless Browser)]
                   • Steps timeline frame-by-frame (24fps / 60fps)
                   • Captures exact lossless PNG buffers
                                      │
                                      ▼
                 [4. Audio Synthesis & Sound Design]
                   • Local offline TTS (Kokoro) for narration
                   • Background music & ambient sound alignment
                                      │
                                      ▼
                 [5. FFmpeg Muxing & Assembly]
                   • Encodes image sequence + audio into final MP4
```

---

## 3. Deep Dive: Key Technical Capabilities

### 3.1 Code-Driven vs. Generative Diffusion
Lemo-Opuscar demonstrates why programmatic code is superior to video diffusion models for explainers and motion graphics:

| Dimension | Video Diffusion (Sora, Runway) | Code-Driven (Lemo-Opuscar / HyperFrames) |
| :--- | :--- | :--- |
| **Determinism** | Non-deterministic; impossible to reproduce exact frame. | **100% deterministic; frame-for-frame identical every run.** |
| **Typography** | Text warps, blurs, or turns into gibberish. | **Crisp, subpixel-rendered vector typography.** |
| **Editability** | Regenerating a scene changes everything completely. | **Changing a single variable updates the render instantly.** |
| **Compute & Cost** | Heavy GPU servers ($$$ per generation). | **Runs locally on consumer CPUs/GPUs with standard web tech.** |
| **Alpha Transparency**| Extremely difficult to extract clean alpha channels. | **Native alpha transparency out of the box.** |

### 3.2 Procedural Artistic Styles (Shaders & Canvas 2D)
Lemo-Opuscar provides reusable algorithmic templates for distinct visual aesthetics:
- **Whiteboard Explainer:** Simulated hand-drawn vector paths with procedural jitter and stroke drawing (`stroke-dasharray` / Canvas `stroke()`).
- **Chinese Ink Wash & Watercolor:** Procedural Perlin noise shaders simulating fluid ink bleeding on textured paper.
- **Isometric Tech Diagrams:** Three.js orthographic camera views with clean geometric blocks and animated connecting conduits.
- **Retro Pixel Art / Chiptune:** Quantized color palettes and low-resolution canvas buffers upscaled with `image-rendering: pixelated`.

### 3.3 Offline Speech & Sound Alignment
Lemo-Opuscar pairs its animation code with local, offline text-to-speech engines (such as **Kokoro TTS**). The duration of the synthesized audio file dictates the exact frame length of the Canvas animation, ensuring speech and visuals stay synchronized.

---

## 4. AeroEdits vs. Lemo-Opuscar Comparison

| Feature | Lemo-Opuscar | AeroEdits |
| :--- | :--- | :--- |
| **Rendering Framework** | Raw HTML5 Canvas, WebGL, Three.js via Puppeteer script. | Embedded **HyperFrames + GSAP 3.15** runtime in Tauri. |
| **Animation Paradigm** | RequestAnimationFrame / custom step loop in JS. | GSAP seekable timelines (`window.__timelines["graphic"]`). |
| **Audio Spine** | Synthesized offline TTS narration. | **Real human spoken audio**, transcribed with word-level timestamps. |
| **Timeline Integration**| Standalone mini-film generator. | Multi-track timeline; graphics anchor to spoken words (`atWord`). |
| **Output Codecs** | Standard H.264 MP4. | WebM VP9 Alpha (real-time preview) + ProRes 4444 (master export). |

---

## 5. Actionable Takeaways for AeroEdits

### 🟢 What We Should Adopt

1. **Procedural "Whiteboard / Hand-Drawn" Explainer Style:**
   - Whiteboard explainers remain one of the most engaging tutorial formats.
   - We can build a dedicated HyperFrames template using SVG path animations (`stroke-dashoffset` driven by GSAP) with subtle organic noise to simulate code concepts or architectural diagrams being drawn live as the speaker explains them.
2. **Three.js Isometric 3D Diagram Templates:**
   - Support Three.js inside HyperFrames via the `window.__seek(t)` hook.
   - This allows agents to generate rotating 3D server architectures, database clusters, or hardware visualizations that smoothly animate into place during technical explainers.
3. **Local Offline TTS for Style Trials & Scratch Tracks:**
   - For creator workflows where a user wants the agent to draft a short-form video before they record their own voice, integrate lightweight local TTS (e.g. Kokoro) to generate a timed scratch track.

### 🔴 What We Refuse / Avoid

1. **Do NOT generate full synthetic films from scratch:**
   - Lemo-Opuscar creates standalone mini-movies. AeroEdits is an editor for real human creators (screencasts, tutorials, talks). Our graphics must serve as overlays and chapter breaks for real footage, not replace the creator.
2. **Do NOT abandon GSAP for ad-hoc Canvas `requestAnimationFrame` loops:**
   - Writing raw Canvas loop math on the fly is prone to timing bugs and performance stutters. GSAP's timeline model provides robust tweening, easing curves, and frame-accurate seeking out of the box.
