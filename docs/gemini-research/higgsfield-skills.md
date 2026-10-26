# Higgsfield Skills Research Report

- **Source:** [github.com/higgsfield-ai/skills](https://github.com/higgsfield-ai/skills) | [higgsfield.ai](https://higgsfield.ai)
- **Author:** Higgsfield AI
- **License:** Open Source
- **Category:** Generative AI Video Agent Skills, Consistent Character IDs & NLE Extensions
- **Analyzed for:** AeroEdits AI Agent & Motion Graphics Architecture

---

## 1. Executive Summary

**Higgsfield Skills** (`higgsfield-ai/skills`) is a suite of AI agent skills created by Higgsfield AI to integrate their foundation models—notably **Seedance 2.0 / 2.5** (multi-shot video generation and style transfer), **Soul ID** (consistent character generation), and product photography engines—directly into coding assistants like Claude Code, Cursor, and Codex.

In addition to command-line skills, Higgsfield ships an **official Adobe After Effects & Premiere Pro extension** that can be driven autonomously by AI agents over the Model Context Protocol. Its key insight is that in professional video, **"taste and assets travel together"**: a successful motion design agent cannot be just a prompt writer; it must come equipped with sound libraries, curated visual reference kits, and strict composition gates.

---

## 2. Core Architecture & Skill Suite

```
┌─────────────────────────────────────────────────────────────┐
│              Coding / Creative Agent (Claude / Cursor)      │
└──────────────────────────────┬──────────────────────────────┘
                               │
       ┌───────────────────────┼───────────────────────┐
       ▼                       ▼                       ▼
`higgsfield-generate`    `higgsfield-soul-id`    `higgsfield-explainer`
Multi-model generation   Consistent character    Multi-shot narrative
(Seedance, Kling, Veo)   actor identities        sequencing & scoring
       │                       │                       │
       └───────────────────────┼───────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                   Higgsfield Model Mesh                     │
│        • Generative video transforms  • Character locks     │
│        • Motion design layers         • Audio/SFX cues      │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│             NLE Bridges (After Effects / Premiere)          │
│   • CEP / UXP Extension (`Window -> Extensions -> Higgsfield`)
│   • Autonomous MCP driving of native NLE timeline layers    │
└─────────────────────────────────────────────────────────────┘
```

### 2.1 The Skill Modules

| Skill | Focus | Behavior |
| :--- | :--- | :--- |
| `higgsfield-generate` | Model abstraction layer | Single unified interface routing requests to Seedance, Soul, Kling, or Veo depending on prompt requirements. |
| `higgsfield-soul-id` | Reusable character reference | Pins a character's face, body, and style across different scenes and camera angles, solving the character consistency problem. |
| `higgsfield-product-photoshoot` | Commercial studio shots | Places real product models into lighting setups, reflections, and 3D studio environments. |
| `higgsfield-video-explainer` | Narrative explainer compilation | Plans multi-shot sequences, generates background b-roll, and aligns footage with a voiceover script. |
| `Window -> Extensions -> Higgsfield` | Native After Effects Bridge | Agent generates native timeline layers, masks, and generative video clips directly inside Adobe NLEs. |

---

## 3. Deep Dive: Key Technical Capabilities

### 3.1 Consistent Identity Anchoring (`Soul ID`)
Generative video models notoriously fail at keeping faces and objects identical across cut boundaries. Higgsfield’s `Soul ID` provides an omni-reference embedding vector. When an agent directs a multi-scene explainer featuring a presenter or product mascot, passing the `soul_id` ensures the character looks identical across shots 1, 2, and 5.

### 3.2 "Taste and Assets Travel Together"
A critical architectural lesson from Higgsfield's motion designer workflows is that an agent with great prompts will still produce terrible videos if it lacks high-end raw materials. 
Higgsfield bundles:
1. **Curated Motion Presets:** Camera moves, light-sweeps, and wipe angles engineered by professional motion designers.
2. **Companion SFX Packs:** Whooshes, risers, and impact hits matched to specific camera velocities.
When the agent invokes a transition or title card, the sound effect is injected alongside the visual layer automatically.

### 3.3 The NLE Bridge Strategy
Rather than building an entire NLE from scratch, Higgsfield chose to inject its agent capabilities into existing editors (After Effects and Premiere) via extensions. This allows pro editors to retain their existing keyboard shortcuts and rendering pipelines while letting the agent automate tedious generative tasks.

---

## 4. AeroEdits vs. Higgsfield Comparison

| Capability | Higgsfield | AeroEdits |
| :--- | :--- | :--- |
| **Motion Approach** | Generative diffusion video (Seedance, Kling) + After Effects. | **Deterministic Code-as-Video** (HyperFrames + GSAP 3.15). |
| **NLE Position** | Plugin / Bridge for Adobe After Effects and Premiere. | **Self-contained, native desktop NLE** (Tauri + React + Rust). |
| **Telemetry Support** | None. | **Deep mouse, keyboard, and window telemetry.** |
| **Character Presence** | Generated AI avatars (`Soul ID`). | **Real human presenter webcam** with YuNet face tracking & auto-framing. |
| **Execution Cost** | Cloud GPU-heavy ($$$ credits per video generation). | **100% local, free, offline execution on consumer hardware.** |
| **Sound Design** | Bundled SFX matched to camera motion. | Roadmap item `SND-2` (sound cues triggered at graphic events). |

---

## 5. Actionable Takeaways for AeroEdits

### 🟢 What We Should Adopt

1. **"Taste and Assets Travel Together" (Bundle High-End SFX with Graphic Templates):**
   - As observed in Higgsfield, a motion graphic without sound design feels amateur.
   - For every graphic template in AeroEdits (metric cards, bento grids, 2.5D tilts, diagram reveals), bundle a **curated, royalty-free audio cue** (e.g. `sfx/card_land.wav`, `sfx/pop_high.wav`). When the agent places the graphic, AeroEdits automatically places the audio cue on the audio timeline.
2. **Preset "Camera Motion & Light-Sweep" Styles:**
   - In HyperFrames, introduce CSS gradient light-sweeps (`background: linear-gradient(90deg, transparent, rgba(255,255,255,0.15), transparent)`) moving across cards on entrance. This gives code-driven cards the cinematic gloss of After Effects motion graphics with zero render overhead.

### 🔴 What We Refuse / Avoid

1. **Do NOT adopt After Effects or Premiere as our engine:**
   - Higgsfield requires users to purchase expensive Adobe Creative Cloud subscriptions and deal with bloated CEP/UXP extension crashes. AeroEdits is proud to be a standalone, lightning-fast native desktop application.
2. **Do NOT replace real footage with generative AI video diffusion:**
   - Diffusion-generated video is unstable, hallucination-prone, and cannot accurately represent real software products, code syntax, or user interfaces. AeroEdits remains committed to real recordings and crisp, deterministic code graphics.
