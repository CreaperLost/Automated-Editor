# Zane Hoyer: "How to Automate Your ENTIRE Editing Job" Research Report

- **Source:** [youtube.com/watch?v=RXN9ci0SvIo](https://www.youtube.com/watch?v=RXN9ci0SvIo)
- **Creator:** Zane Hoyer
- **Title:** *How to Automate Your ENTIRE Editing Job ($49,540/Mo)*
- **Category:** Creator Workflow & Automated AI Video Agency Architecture
- **Analyzed for:** AeroEdits AI Agent & Motion Graphics Architecture

---

## 1. Executive Summary

In this video, creator and video editor **Zane Hoyer** breaks down how professional freelance video editors and post-production agencies can scale to $50,000/month by replacing manual editing grunt work with an **orchestrated multi-agent assembly line**.

Rather than editing inside a traditional NLE manually, Zane decomposes the editing lifecycle into specialized agent tasks:
1. **Rough-cut & A-roll grading** via ChatGPT + DaVinci Resolve scripts.
2. **Motion graphics & kinetic typography** generated programmatically via **Claude Code** (Remotion / code-driven animation).
3. **Contextual AI B-roll generation** via OpenArt.
4. **Client style & brand containment** via isolated reference directories (`brand.json`, fonts, color tokens).

While Zane proves the massive creator demand and commercial viability of agentic video editing, his setup suffers from severe **"Frankenstein fragmentation"** (juggling ChatGPT, terminal Claude Code, web OpenArt, and DaVinci Resolve with manual file handoffs). This report details how **AeroEdits** unifies this entire workflow into a single, cohesive desktop platform.

---

## 2. Zane Hoyer's 4-Agent Assembly Line

```
Raw Multi-Take Footage + Script
               │
               ▼
┌─────────────────────────────────────────────────────────────┐
│  Agent 1: Rough Cut & Grading (ChatGPT + DaVinci Resolve)   │
│  • Reads speech transcript; identifies retakes & dead air   │
│  • Emits cut lists / EDL to assemble primary A-roll track   │
│  • Applies default camera LUTs and primary color grade      │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│  Agent 2: Motion Graphics Engine (Claude Code + Remotion)   │
│  • Prompted with script sentences requiring visual hooks    │
│  • Writes React/Remotion code for bouncy spring text,       │
│    kinetic cards, and 2.5D floating UI mockups              │
│  • Renders alpha-channel video clips (ProRes 4444)          │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│  Agent 3: Contextual B-Roll Generation (OpenArt AI)         │
│  • Analyzes script metaphors; prompts image/video models    │
│  • Generates high-retention contextual B-roll cuts          │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│  Agent 4: Multi-Client Brand Vault & Hook Testing           │
│  • Enforces client brand rules (hex palettes, fonts, logos) │
│  • Generates 3 hook variations for 9:16 Shorts / Reels      │
│  • Final assembly in DaVinci Resolve timeline               │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. Deep Dive: Key Technical Capabilities & Insights

### 3.1 Code-Driven Motion Graphics vs. After Effects Templates
Zane emphasizes that traditional motion graphics templates (.mogrts or After Effects compositions) are rigid and fail to adapt dynamically to variable sentence lengths, numbers, or brand colors. 

By using **Claude Code to write code-driven motion graphics**, the editor achieves:
- **Dynamic text wrapping & layout:** Cards scale automatically based on the length of the sentence.
- **Parametric spring physics:** Motion velocity, overshoot, and damping are calculated mathematically rather than manually keyframed.
- **Instant client restyling:** Swapping a client's accent hex code instantly updates all rendered graphics.

### 3.2 The Multi-Client Brand Isolation Architecture
To service multiple creator clients without brand contamination, Zane isolates client assets into strict configuration bundles:
- `clients/<id>/brand.json`: Primary, secondary, and accent colors; body and display font files; logo files.
- `clients/<id>/motion_profile.json`: Pacing rules (e.g. snappy 0.3s springs for tech vs. smooth 0.8s eases for corporate).
- `clients/<id>/rules.md`: Do's and Don'ts (e.g. *"Never use emojis; always highlight metrics in gold"*).

This mirrors AeroEdits' **Preset and Style architecture** (`styles/` and `presets/`).

### 3.3 Hook Testing & Short-Form Variations
Retention in modern short-form video (TikTok, YouTube Shorts, Reels) is determined in the first 3 seconds. Zane uses agents to generate **3 alternative opening hooks** (varying text phrasing, layout punch-in, and opening graphic) while keeping the body of the video identical.

---

## 4. The Critical Flaws in Zane's Workflow (The "Frankenstein" Problem)

While Zane's methodology is conceptually sound, his daily execution is plagued by severe friction:

| Friction Point in Zane's Setup | Real-World Impact | How AeroEdits Solves It |
| :--- | :--- | :--- |
| **Tool Juggling Across 4 Apps** | Constantly switching between terminal (Claude Code), browser (ChatGPT, OpenArt), and NLE (DaVinci Resolve). | **Single unified desktop application:** All agent passes, timeline playback, and graphic renders occur in AeroEdits. |
| **Manual File Moving & Ingest** | Manually copying rendered ProRes files from Remotion build folders into Resolve media pools and onto video tracks. | **Direct timeline integration:** `graphic_add` places compositions directly on the timeline; renders cache automatically. |
| **Frame Drift & Cut Mismatches** | Fragile EDL/XML exports between ChatGPT transcripts and Resolve playheads cause slight audio sync drift. | **Sample-accurate timeline spine:** Cuts happen directly inside AeroEdits' native sequence model with zero translation loss. |
| **Lack of Telemetry Awareness** | Only works on talking-head footage; completely incapable of handling screencasts, software tutorials, or code walks. | **Full telemetry engine:** Mouse clicks, dwell points, active window bounds, and typing carets automatically drive smart zooms and layout framing. |
| **No Atomic Undo / Checkpoints** | If an agent's run makes an error, undoing requires reversing changes across 3 separate software tools. | **Persistent local checkpoints:** Named checkpoints and single-step undo (`history/`) for entire agent runs. |

---

## 5. Actionable Takeaways for AeroEdits

### 🟢 What We Should Adopt

1. **Multi-Hook Variation Generator for Shorts (`SHT-8`):**
   - Adopt Zane's hook-testing strategy for AeroEdits' Shorts workflow (`shorts.md`).
   - When generating a vertical Short from a take, allow the agent to propose **3 hook variants** for the first 3–5 seconds:
     - *Variant A:* Camera punch-in with kinetic emphasis caption.
     - *Variant B:* Full-screen 2.5D floating graphic card with stat callout.
     - *Variant C:* Split layout with animated diagram.
   - The creator can preview all three with one click and export three alternative `.mp4` variations for A/B testing on social platforms.
2. **Standardized Client Brand Vaults (`library kind: style`):**
   - Zane's client folder structure confirms that AeroEdits' Style system (`--brand-*` and `--look-*` tokens) is exactly what professional creators need. We should provide an explicit UI command or tool: `client_brand_create` to scaffold a complete brand bundle from a URL or logo file.
3. **Automated A-Roll Camera LUT & Primary Grade Pass:**
   - Add an initial color-enhancement pass for webcam clips (auto-exposure, contrast, white balance, or custom `.cube` 3D LUT application).

### 🔴 What We Refuse / Avoid

1. **Do NOT force users into an external NLE (DaVinci Resolve / Premiere):**
   - Zane must use DaVinci Resolve because he doesn't have an integrated agent-first editor. AeroEdits *is* the editor. Keeping the timeline, audio waveforms, telemetry, and agent proposals in one window eliminates 90% of the friction in Zane's workflow.
2. **Do NOT adopt external Remotion build dependencies:**
   - Zane relies on Remotion, requiring Node.js, Webpack/Vite bundlers, and heavy React compilation. AeroEdits' embedded **HyperFrames + GSAP 3.15** runtime is faster, lighter, executes offline, and requires zero user configuration.
3. **Do NOT use slow cloud image generators (OpenArt) during interactive editing:**
   - Generating diffusion B-roll from web services introduces high latency ($10\text{–}30\text{s}$ per prompt) and high costs. For explainers, prioritized deterministic vector graphics, telemetry-driven zooms, and curated archival B-roll (via `broll-mcp` / Archive.org) yield higher information density and immediate turnaround.
