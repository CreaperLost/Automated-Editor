# HyperFrames Research Report

- **Source:** [github.com/heygen-com/hyperframes](https://github.com/heygen-com/hyperframes) | [hyperframes.app](https://hyperframes.app)
- **Author:** HeyGen
- **License:** Open Source (Apache 2.0 / Modular Packages)
- **Category:** Agent-Native Video-as-Code Framework (HTML/CSS/JS $\to$ MP4/WebM/ProRes)
- **Analyzed for:** AeroEdits AI Agent & Motion Graphics Architecture

---

## 1. Executive Summary

**HyperFrames** is HeyGen's open-source framework designed specifically for **turning web standards (HTML, CSS, GSAP, WebGL, media) into frame-accurate, deterministic video files**. 

While traditional programmatic video tools like Remotion require a React runtime and heavyweight bundling, HyperFrames treats the web page itself as the timeline and scene graph. AI coding agents (Claude Code, Cursor, Codex) can generate standard semantic HTML and GSAP code, lint it for structural and contrast issues, and render MP4, WebM (with alpha transparency), and ProRes 4444 files.

AeroEdits already uses HyperFrames as its embedded graphics engine (`src-tauri/src/graphics/`). This investigation evaluates the upstream repository's latest CLI tools, quality gates, parameterized templates, and agent skills.

---

## 2. Architecture & Modular Package Structure

HyperFrames is organized into modular packages wrapped by a unified CLI:

```
┌──────────────────────────────────────────────────────────────┐
│                     HyperFrames CLI                          │
│               `npx hyperframes <command>`                    │
└──────────────────────────────┬───────────────────────────────┘
                               │
       ┌───────────────────────┼───────────────────────┐
       ▼                       ▼                       ▼
┌──────────────┐       ┌──────────────┐        ┌──────────────┐
│  @core       │       │  @engine     │        │  @producer   │
│ HTML parser, │       │ Headless     │        │ Node.js API  │
│ parameters,  │       │ browser      │        │ programmatic │
│ timeline     │       │ capture +    │        │ batch        │
│ seek hooks   │       │ FFmpeg mux   │        │ orchestration│
└──────────────┘       └──────────────┘        └──────────────┘
                               ▲
                               │
                       ┌───────┴──────┐
                       │  @studio     │
                       │ Web-based    │
                       │ live visual  │
                       │ workbench    │
                       └──────────────┘
```

### 2.1 Core CLI Commands & Agent Workflows

The CLI defaults to non-interactive, machine-friendly output (`--human-friendly` enables spinners and interactive prompts):

| Command | Purpose | Agent Suitability |
| :--- | :--- | :--- |
| `npx hyperframes init <dir>` | Scaffolds a new composition project with template HTML, CSS, and timeline bindings. | High (scaffolding starter files). |
| `npx hyperframes preview` | Launches a local hot-reload web server for live inspection. | Interactive preview during development. |
| `npx hyperframes lint` | Fast static structure check (DOM tags, timeline registration, missing assets). | Essential automated pre-render step. |
| `npx hyperframes check` | Comprehensive gate: runs headless verification, console errors, motion glitches, and WCAG contrast check. | Essential gate before showing to user. |
| `npx hyperframes render` | Headless Chrome frame capture encoded via FFmpeg into output media. | Produces MP4, WebM, ProRes 4444, GIF, PNG sequences. |

---

## 3. Deep Dive: Key Technical Capabilities

### 3.1 Seekable Timeline Protocol
HyperFrames does not record the screen in real time. Instead, it seeks the page frame-by-frame:
1. **GSAP Timelines:** A global paused timeline registered on `window.__timelines["graphic"]` or `window.__timeline`. HyperFrames seeks this timeline to $t = \frac{\text{frame}}{\text{fps}}$ and captures the buffer.
2. **Web Animations API:** If `data-no-timeline` is set on the document root, HyperFrames steps through native CSS animations using `document.getAnimations()`.
3. **Custom Canvas/WebGL:** A hook `window.__seek(timeInSeconds)` allows Three.js, Canvas 2D, and GLSL shaders to render deterministically at any arbitrary framerate.

### 3.2 Automated Quality Gates (`lint` vs. `check`)
HeyGen designed HyperFrames to prevent agents from outputting visually broken media:
- **`lint`:** Pure AST/static check. Catches unclosed tags, malformed CSS, references to external CDN assets that violate offline-first constraints, and missing `data-duration` attributes.
- **`check`:** Boots a headless Chrome instance. Evaluates:
  - Runtime JavaScript exceptions.
  - Missing fonts or layout shifts (FOUC).
  - Motion glitches (e.g. elements jumping before animation start).
  - WCAG contrast ratios: evaluates text color against dynamic background elements to ensure legibility.

### 3.3 Transparency & Codecs
HyperFrames supports rendering alpha channels natively:
- **WebM VP9 Alpha:** Ultra-lightweight format for real-time overlay previews on desktop/web players.
- **Apple ProRes 4444:** Broadcast-grade alpha transparency for export or handoff to Final Cut Pro / Premiere / DaVinci Resolve.
- Leaving `body { background: transparent }` unpainted automatically triggers alpha plane capture during FFmpeg muxing.

### 3.4 Parameterized Templates
HyperFrames compositions can declare schema variables:
```json
{
  "variables": {
    "title": { "type": "string", "default": "Clean Architecture" },
    "stat": { "type": "string", "default": "+42%" },
    "accentColor": { "type": "color", "default": "#3b82f6" }
  }
}
```
At render time, the agent or app can pass overrides (`--params '{"stat": "+99%"}'`) without editing HTML or re-writing GSAP animation logic.

---

## 4. AeroEdits vs. Upstream HyperFrames

| Capability | Upstream HyperFrames | AeroEdits Implementation |
| :--- | :--- | :--- |
| **Runtime Environment** | Node CLI / npm dependency (`npx hyperframes`). | Embedded, pinned Node + Chrome runtime inside Tauri app; zero `PATH` dependencies. |
| **Animation Engine** | GSAP 3.15, Web Animations, WebGL. | Pinned GSAP 3.15 offline (`gsap.min.js`), Web Animations API, custom tokens. |
| **Output Formats** | MP4, WebM, ProRes 4444, GIF, HLS, PNG sequence. | WebM VP9 Alpha (real-time timeline playback cache) & ProRes 4444 (master export). |
| **Integration Model** | Standalone video generation tool. | Composition clips embedded in `.aero` project (`graphics/<id>/`), strictly anchored to spoken words (`atWord`). |
| **Layout Context** | Fixed viewport (16:9, 9:16, 1:1). | Dynamic responsive render boxes (`data-mode="split"`, `data-width`, `data-height`, orientation media queries for Shorts). |
| **Styling Consistency** | Per-project CSS variables. | Global Style Tokens (`--brand-*`, `--look-*`) applied across all graphics in the project. |

---

## 5. Actionable Takeaways for AeroEdits

### 🟢 What We Should Adopt

1. **Adopt Upstream's Automated Contrast & Accessibility Checks (`hyperframes check`):**
   - Incorporate contrast-checking logic into `graphic_check`. If an agent sets text color `#94a3b8` on a light background or over dynamic video footage, `graphic_check` should fail with an actionable warning: *"Contrast ratio 2.8:1 below minimum 4.5:1. Switch text to --brand-text or darken background."*
2. **Explicit Parameter Schema (`variables.json`):**
   - When templatizing graphics via `graphic_templatize`, adopt a standardized `variables.json` schema. This enables the agent to reuse existing graphic templates by simply changing JSON values rather than rewriting HTML strings.
3. **WebGL / Canvas Deterministic Seek Hook (`window.__seek`):**
   - Formalize support for `window.__seek(t)` in AeroEdits' render engine. This unlocks complex explainer graphics: particle simulations, 3D rotating product models (Three.js), and procedural shaders that cannot be animated via standard GSAP DOM properties.

### 🔴 What We Refuse / Avoid

1. **Do NOT adopt external npm CLI dependencies on user machine:**
   - Upstream HyperFrames assumes users have Node 20+, Chrome, and FFmpeg installed on `PATH`. AeroEdits must keep its pinned, bundled, sandboxed runtime to prevent environment mismatch bugs across macOS and Windows.
2. **Do NOT decouple graphics from the video timeline:**
   - HyperFrames standalone is a full video builder. In AeroEdits, HyperFrames is a graphics overlay layer. Timing must remain anchored to the project audio and spoken word timestamps (`atWord`), with hard cuts for full-screen and ease transitions for overlays (`data-entrance`).
