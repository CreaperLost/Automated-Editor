# Jake Moran & Peter Yang: "How to Make Amazing Videos with Claude and HyperFrames" Research Report

- **Source:** [youtube.com/watch?v=MC1g2fHYC88](https://www.youtube.com/watch?v=MC1g2fHYC88)
- **Host:** Peter Yang (*Behind the Craft*)
- **Guest:** Jake Moran (Co-creator of HyperFrames, HeyGen)
- **Title:** *How to Make Amazing Videos with Claude and HyperFrames*
- **Category:** Creator Masterclass & Engine Architecture Deep Dive
- **Analyzed for:** AeroEdits AI Agent & Motion Graphics Architecture

---

## 1. Executive Summary

In this in-depth interview and live demonstration, **Jake Moran** (co-creator of **HyperFrames** at HeyGen alongside Bin Liu) joins product leader **Peter Yang** to break down the exact operational playbook for generating broadcast-quality product launches, feature explainers, and motion graphics using **Claude Code + HyperFrames**.

While HeyGen is one of the world's leading generative AI video companies, Jake explains why **diffusion video models (Sora, Runway, Pika) fundamentally fail at software launches and explainers**: they hallucinate UI, mangle typography, cannot maintain exact brand hex colors, and are non-deterministic. HyperFrames was born to make video **code-native, deterministic, and free to render locally**.

Jake reveals the **5-step file-driven agent playbook** (`frame.md` $\to$ `storyboard.md` $\to$ components $\to$ keyframe gate $\to$ render) that guides Claude from concept to finished video without hallucination.

---

## 2. Jake Moran’s 5-Step Agent Production Playbook

```
1. Gather Assets & Define Video Constraints (`frame.md`)
   • Aspect ratio (16:9, 9:16), FPS (60), brand colors, font tokens
   • Motion profile: snappy tech vs. fluid editorial
                  │
                  ▼
2. Temporal Storyboard Architecture (`storyboard.md`)
   • Scene-by-scene script, timestamps (0:00–0:40), visual metaphors
   • Pacing grid: Hook (0-3s), Problem (3-8s), Reveal (8-15s), Proof (15-30s), CTA
                  │
                  ▼
3. Modular Component Assembly
   • Reusable design patterns: 2.5D tilt card, metric counter, code terminal
   • Injects semantic HTML + GSAP seekable timeline
                  │
                  ▼
4. Static Keyframe Review Gate (Zero-Render-Cost Verification)
   • Inspects static layout cards before authoring full animations
   • Catches text wrapping bugs, contrast issues, and layout collisions
                  │
                  ▼
5. Frame-Accurate Compilation & Sound Alignment
   • Synchronizes motion to audio voiceover and transients
   • Local headless Chrome rendering $\to$ MP4 / ProRes 4444
```

---

## 3. Deep Dive: Key Technical Capabilities & Insights

### 3.1 The Purpose-Built Video Contract: `frame.md`
Jake points out that developers often make the mistake of feeding standard web `design.md` files to video agents. Web design systems fail in video because video is viewed on moving viewports, under severe H.264 compression, and for only fractions of a second.

`frame.md` translates branding specifically into **video motion constraints**:
```markdown
# Video Design System (`frame.md`)
- Resolution: 1920x1080 @ 60fps (16:9)
- Brand Colors: Background #0a0c10, Surface #161b22, Accent #00f0ff, Text #f0f6fc
- Typography: Display 64px bold, Body 32px regular (min readable: 24px)
- Motion Profile: Snappy (tweens: 0.25s–0.4s; ease: "power3.out")
- Safety Boundaries: 120px padding (never place critical copy near edges)
- Contrast Gate: Text luminance must exceed background by at least 4.5:1
```

### 3.2 Temporal Architecture: `storyboard.md`
The most common agent failure is writing code before planning time. If an agent begins writing GSAP code directly from a prompt, it almost always misjudges animation durations, creating cards that linger awkwardly or disappear before the viewer can read them.

Jake mandates that the agent generate a `storyboard.md` table first:

| Scene | Time Window | Audio Voiceover / Concept | Visual Asset / Metaphor | Motion Entrance |
| :---: | :---: | :--- | :--- | :--- |
| **01** | `00:00–00:03` | *"Building AI agents is easy. Making them reliable is hard."* | Bold kinetic typography + warning badge. | Hard cut entrance (`data-entrance="cut"`). |
| **02** | `00:03–00:08` | *"Meet AeroEdits: the first agent-native desktop NLE."* | 2.5D floating app window with light-sweep. | Camera push-in with 2.5D tilt. |
| **03** | `00:08–00:18` | *"Deterministic cuts, real telemetry, instant undo."* | 3-card Bento grid revealing features sequentially. | Staggered slide-up on transients. |
| **04** | `00:18–00:25` | *"Over 50,000 creators saving 10 hours every week."* | Metric counter spring (`0` $\to$ `50,000+`). | Scale bounce (`back.out(1.4)`). |

### 3.3 Component Reuse vs. Generative Scratch Code
Jake stresses that writing bespoke CSS layout logic from scratch for every scene produces brittle, inconsistent videos. Instead, HyperFrames projects rely on a catalog of **modular, battle-tested components**:
1. **The 2.5D Floating Tilt Card:** Gives flat software screenshots realistic 3D perspective and depth.
2. **The Kinetic Text Block:** Staggered word reveals (`SplitText`) with active word accent coloring.
3. **The Animated Metric Card:** Tabular numbers animating smoothly from zero to target.
4. **The Code / Terminal Window:** Mac-style window controls, syntax-highlighted code, and blinking caret.

### 3.4 Why Code-Driven Beats Diffusion for Explainers
Jake provides a definitive breakdown of why HyperFrames is superior to video diffusion models:
- **Zero Hallucination:** Product UI screenshots, code snippets, and graphs render with 100% vector sharpness.
- **Micro-Edits Take Seconds:** Changing a typo or metric requires changing one string in HTML or JSON; diffusion models require re-generating the entire video from scratch with unpredictable results.
- **Local & Deterministic:** 100% free local execution on standard consumer hardware, with identical pixel-for-pixel output every time.

---

## 4. AeroEdits vs. Upstream HyperFrames Workflow

| Dimension | Jake Moran's HyperFrames Workflow | AeroEdits Current Architecture |
| :--- | :--- | :--- |
| **Execution Context** | Standalone Node.js CLI (`npx hyperframes render`) via Claude Code. | Embedded inside Tauri desktop app (`src-tauri/src/graphics/`); zero PATH dependencies. |
| **Design Contract** | `frame.md` file in project directory. | Style System (`styles/`) with `--brand-*` and `--look-*` tokens. |
| **Timeline Integration**| Standalone video files assembled with external tools. | **Embedded on multi-track timeline**, anchored to spoken transcript words (`atWord`). |
| **Review Method** | Browser hot-reload preview (`npx hyperframes preview`). | Native Tauri player + contact sheets (`frame_render`). |
| **Telemetry Support** | None (pure script-to-video). | **Mouse dwell, click clusters, active window crop, typing carets.** |

---

## 5. Actionable Takeaways for AeroEdits

### 🟢 What We Should Adopt

1. **Formalize the `frame.md` Contract in AeroEdits Presets:**
   - In AeroEdits’ Style System, generate a machine-readable `frame.md` summary for agents.
   - When an agent is invoked to add graphics, it immediately reads `frame.md` to learn canvas dimensions, safe-zone margins, brand colors, and motion pacing tokens before drafting code.
2. **Mandate the `storyboard.md` Planning Step for Explainer Sequences:**
   - In `mcp/skill/graphics.md`, instruct agents that when asked to build multi-scene explainers, they must **propose a storyboard table first** (scene, time window, words spoken, visual metaphor, entrance type) for user confirmation before generating HyperFrames code.
3. **Pre-Bundle the 4 Core Jake Moran Component Primitives:**
   - Ship the 4 essential components directly in AeroEdits’ bundled graphic templates:
     - `template/tilt_card_2_5d` (floating app screenshot).
     - `template/kinetic_text` (staggered word emphasis).
     - `template/metric_counter` (smooth numeric count-up with tabular nums).
     - `template/code_terminal` (syntax-highlighted code block with typing caret).
   - This ensures agents assemble proven, cinematic layouts rather than writing raw CSS from scratch.
4. **Keyframe Gate Before Heavy Animation:**
   - Encourage agents to verify static layout cards via `frame_render` before adding multi-step GSAP timelines, drastically reducing token usage and debugging loops.

### 🔴 What We Refuse / Avoid

1. **Do NOT force users to install Node.js or the external `hyperframes` CLI:**
   - Jake assumes a developer environment with global npm packages. AeroEdits must keep its sandboxed, embedded, pre-compiled runtime inside Tauri so non-technical creators can use it out of the box with zero terminal setup.
2. **Do NOT decouple graphics from spoken audio:**
   - In Jake’s standalone workflow, timing is manually estimated. In AeroEdits, graphics are strictly pinned to real word timestamps (`atWord`), guaranteeing automatic sync with the creator's voice even after silence jump-cuts.
