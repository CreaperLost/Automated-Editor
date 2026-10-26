# Video-Shotcraft Research Report

- **Source:** [github.com/Vincentwei1021/video-shotcraft](https://github.com/Vincentwei1021/video-shotcraft)
- **Author:** Vincent Wei
- **License:** Open Source
- **Category:** Agent Motion Design Studio & Shot Recipe Library (Remotion + React)
- **Analyzed for:** AeroEdits AI Agent & Motion Graphics Architecture

---

## 1. Executive Summary

**Video-Shotcraft** (`Vincentwei1021/video-shotcraft`) is an open-source AI video production skill for coding assistants (Claude Code, Cursor, Codex). Instead of using generative AI video models (which produce blurry, hallucinated, or uneditable footage), Video-Shotcraft operates as a **code-driven motion graphics design studio**.

It uses **Remotion** (React/TypeScript) and a catalog of **over 120–150+ pre-engineered "Shot Recipe Cards"** to build cinematic SaaS product launches, feature explainers, and promo videos using real product screenshots and recordings. It features an interactive **Motion Workbench** (a Vite-powered web workspace) for real-time visual adjustments, beat-synchronized pacing, and sound design layers.

---

## 2. Core Architecture & Workflow

```
[Real Product URLs / Screenshots / Recordings]
                       │
                       ▼
       [1. Agent Planning & Storyboarding]
         • Selects from 120+ Shot Recipe Cards
         • Maps narrative beats to audio transients
                       │
                       ▼
       [2. Programmatic Code Assembly]
         • Assembles typed Remotion React components (.tsx)
         • Parameterizes typography, 2.5D camera angles, easing
                       │
                       ▼
       [3. Interactive Motion Workbench]
         • Real-time Vite preview player + Inspector
         • Live tweaking of text, brand colors, shot swapping
         • Sound design layer (whooshes, UI clicks, risers)
                       │
                       ▼
       [4. Final Render & Multi-Target Export]
         • Frame-accurate MP4 export via Remotion CLI
         • Timeline export to CapCut / JianYing
```

---

## 3. Deep Dive: Key Technical Capabilities

### 3.1 The 120+ "Shot Recipe Card" Library
The greatest strength of Video-Shotcraft is that the agent **does not invent motion from scratch**. Writing physics springs, camera perspectives, and CSS transitions on the fly almost always leads to awkward, amateurish animations.

Instead, Video-Shotcraft provides a massive, categorized library of structured **Shot Cards**:
- **2.5D Camera Moves:** CSS 3D transforms (`rotateX`, `rotateY`, `translateZ`, `perspective(1000px)`) that turn flat UI screenshots into sleek, angled, floating dashboard cards.
- **Micro-Interactions & Cursors:** Simulated smooth mouse glides, click ripples, and active button depressions.
- **Metric Reveals:** Spring-animated counters (`countUp`), bar chart expansions, and progress rings.
- **Kinetic Split Layouts:** Asymmetrical split cards (left 60% code block, right 40% animated terminal result).
- **Transitions:** Seamless 3D flip-cards, camera push-ins, and directional parallax wipes.

Each shot card contains:
- Pre-tuned spring physics (`mass`, `damping`, `stiffness`).
- Standardized duration boundaries (e.g. 2.0s to 4.5s).
- Explicit slot props (e.g. `title`, `screenshotPath`, `accentColor`, `cameraAngle`).

### 3.2 Beat-Synchronized Editing & Sound Design (SFX)
Video-Shotcraft treats motion and audio as unified:
1. **Audio Transients:** Cut points, shot entrances, and text reveals are clamped to musical beats.
2. **Integrated Sound Design:** Each shot recipe declares companion SFX cues:
   - Floating card entry $\to$ gentle whoosh (`whoosh_soft.wav`).
   - Feature badge pop $\to$ subtle click/blip (`ui_tap.wav`).
   - Metric finish $\to$ chime or accent impact.

### 3.3 The Interactive Motion Workbench
Alongside agent command-line editing, Video-Shotcraft spins up a Vite web workbench where creators can:
- Scrub the visual timeline in real-time.
- Click elements to edit copy or swap brand hex codes directly.
- Swap one shot recipe for another without asking the agent to rewrite code.

---

## 4. AeroEdits vs. Video-Shotcraft Comparison

| Capability | Video-Shotcraft | AeroEdits |
| :--- | :--- | :--- |
| **Motion Technology** | Remotion (React, TypeScript, CSS-in-JS). | **HyperFrames + GSAP 3.15** (Semantic HTML, CSS, JavaScript). |
| **Animation Paradigm** | React spring math (`spring({ frame, fps })`). | GSAP Timelines (`window.__timelines["graphic"]`) & Web Animations API. |
| **Shot Templating** | 120+ modular Shot Recipe Cards (.tsx). | Bundled graphic templates + user style tokens (`--look-*`, `--brand-*`). |
| **3D / 2.5D Capabilities** | Native 3D CSS transforms & camera angles. | 2D layouts and cards; currently lacks pre-tuned 2.5D perspective templates. |
| **Sound Effects (SFX)** | Sound cues declared inside shot recipes. | Roadmap item `SND-2` (sound cues at graphic events); audio loudness -14 LUFS. |
| **Primary Footage** | Synthetic compositions built around screenshots. | **Real recording sessions** (multi-track screen, webcam, mic, mouse/keyboard telemetry). |

---

## 5. Actionable Takeaways for AeroEdits

### 🟢 What We Should Adopt

1. **Port the 2.5D Floating UI Card Recipes to HyperFrames/GSAP:**
   - Video-Shotcraft’s most visually impressive feature is its 2.5D perspective cards (giving flat screen recordings or app screenshots a premium Apple-style floating slant using `transform: perspective(1200px) rotateY(-12deg) rotateX(6deg)`).
   - We should build a dedicated set of **2.5D perspective graphic templates** in HyperFrames for AeroEdits. When an agent creates a graphic illustrating a software feature, it can render the screenshot in a floating 2.5D card with realistic box shadows and smooth camera drift.
2. **Adopt the "Shot Recipe" Design Pattern:**
   - Expand AeroEdits’ graphic template catalogue by introducing structured **Shot Cards** in `mcp/skill/graphics.md`:
     - *Hero Metric Callout* (counter + badge).
     - *2.5D Window Tilt* (slanted app preview).
     - *Before vs. After Split* (divider wipe).
     - *Workflow Stepper* (3-step progress pipeline).
   - This prevents agents from inventing raw CSS layouts from scratch and ensures broadcast-level aesthetic consistency.
3. **Sound FX Markers in Templates (`SND-2` Implementation):**
   - In HyperFrames `graphic.json`, allow templates to declare sound cues:
     ```json
     {
       "sfx": [
         { "atFrame": 12, "cue": "whoosh" },
         { "atFrame": 45, "cue": "click" }
       ]
     }
     ```
   - When the graphic plays on the AeroEdits timeline, the app automatically drops the corresponding SFX into the audio mix.

### 🔴 What We Refuse / Avoid

1. **Do NOT adopt Remotion or React runtime for graphics:**
   - Remotion requires full React compilation, bundlers (Webpack/Vite), and heavy Node processes that would dramatically increase memory overhead in a Tauri desktop app. HyperFrames + GSAP 3.15 executes instantly in headless Chrome with zero build step.
2. **Do NOT replace real tutorial footage with synthetic mockups:**
   - Video-Shotcraft makes standalone promo teasers from static screenshots. AeroEdits is built for authentic tutorials and screencasts. Our graphics must complement real human recordings, active window telemetry, and spoken dialogue.
