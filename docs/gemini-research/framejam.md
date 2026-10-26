# FrameJam Research Report

- **Source:** [github.com/moritzkremb/framejam](https://github.com/moritzkremb/framejam) | [framejam.ai](https://www.framejam.ai)
- **Author:** Moritz Kremb
- **License:** MIT
- **Category:** Agent Companion App, Review Loop & MCP Server for Code-Based Video & Motion Graphics
- **Analyzed for:** AeroEdits AI Agent & Motion Graphics Architecture

---

## 1. Executive Summary

**FrameJam** is an open-source companion tool and Model Context Protocol (MCP) server that connects AI coding agents (Claude Code, Cursor, ChatGPT/Codex) with code-driven video frameworks (**Hyperframes**, **Remotion**, Motion Canvas). 

It solves the primary friction points in agentic video generation:
1. **The Spatial Review Problem:** Text chats fail at communicating visual and timing tweaks (e.g., *"at 3.2s move the second card 50px left"*). FrameJam gives users a local browser interface to pause, click frames to drop coordinate pins `(x, y)`, drag time-ranges on a filmstrip, and type comments. These annotations are returned to the agent as structured JSON, markdown prompts, and annotated frame JPEGs with pin overlays.
2. **The Blueprint Problem:** Rather than letting an agent write unconstrained animations from scratch, FrameJam introduces **Playbooks** (structured recipes for video types like Product Launches and Explainers) and **Style Presets** (filtered by mood, pacing, and aspect ratio).
3. **The Multi-Round Iteration Loop:** A managed versioning workflow (`open_review` → `wait_for_feedback` → `add_version` → `resolve_comments`) that handles MCP timeouts gracefully and keeps an audit trail of fixes.

---

## 2. Core Architecture & Capabilities

```
┌────────────────────────────────────────────────────────────────────────┐
│                          Coding Agent Workspace                        │
│          (Claude Code / Cursor / Codex with framejam MCP)              │
└──────────────┬──────────────────────────────────────────▲──────────────┘
               │                                          │
       1. get_playbook / get_preset               4. wait_for_feedback
       2. writes composition (Hyperframes/Remotion)  (comments, pins (x,y),
       3. open_review / add_version (MP4)            annotated frame JPEGs)
               │                                          │
               ▼                                          │
┌─────────────────────────────────────────────────────────┴──────────────┐
│                    FrameJam Local Companion App                        │
│                     (Node / Express on :2400)                          │
├────────────────────────────────────────────────────────────────────────┤
│ • Interactive Video Player + Scrubbable Filmstrip Timeline             │
│ • Spatial Pin Marker Engine (captures frame JPEG + draws target pin)   │
│ • Storyboard / Panel Sequence Preview (pre-animation review)           │
│ • Style & Playbook Catalog Picker (`open_picker`)                      │
└────────────────────────────────────────────────────────────────────────┘
```

### 2.1 Complete MCP Toolset (14 Tools)

FrameJam exposes a clean, targeted MCP surface categorized into three layers:

| Layer | Tool | Description & Mechanics |
| :--- | :--- | :--- |
| **Review & Iteration** | `open_review` | Registers a rendered video (`.mp4`) or storyboard panel set (`panelsDir`). Opens local web UI at `http://localhost:2400/reviews/:id`. Returns URL. |
| | `wait_for_feedback` | Long-polling block. Waits for user to click **Finish Review**. Returns `status: "pending"` after ~50s to avoid MCP client timeouts, allowing agent loops. |
| | `get_feedback` | Non-blocking fetch of latest (or all) feedback comments, pin coordinates, and frame snapshots. |
| | `add_version` | Appends a newly rendered video iteration to an existing review with a revision note for the reviewer. |
| | `resolve_comments` | Allows agent to mark specific comment IDs as handled, updating review state. |
| | `list_reviews` | Lists all active review sessions and their current round states. |
| **Playbooks** | `list_playbooks` | Lists available templates (Product Launch, Explainer, Launch Teaser, Clean Cut) with input requirements, output specs, and cost tier. |
| | `get_playbook` | Returns the step-by-step narrative method, code scaffolding, music sync guidelines, and asset structure. |
| | `open_picker` | Launches interactive browser modal for user to visually choose playbook & style. |
| | `wait_for_pick` | Blocks until user selects their playbook/style in the picker UI. |
| **Style Presets** | `list_presets` | Lists visual styles filterable by `mood` (bold, calm, editorial), `pacing` (slow, medium, fast), and `format` (16:9, 9:16, 1:1). |
| | `get_preset` | Fetches design tokens, color palettes, typography, and sample animation files. |
| **Legacy** | `get_selected_*` | Backward-compatibility stubs pointing agents to `open_picker`. |

---

## 3. Deep Dive: Key Technical Innovations

### 3.1 The Spatial Pin & Time-Range Feedback Schema
When a user clicks on the player canvas, FrameJam records normalized viewport coordinates `x, y` $\in [0.0, 1.0]$ and the exact timestamp. It uses FFmpeg to extract the exact frame image and draws a visible marker at `(x, y)`.

The agent receives:
```json
{
  "status": "feedback",
  "reviewId": "rev_prod_launch_v1",
  "currentVersion": 1,
  "comments": [
    {
      "id": "cmt_01",
      "timestamp": 4.25,
      "range": [3.8, 5.1],
      "text": "The floating dashboard card overlaps with the accent badge. Move it down and left.",
      "pin": { "x": 0.38, "y": 0.42 },
      "frameImagePath": "/Users/.../.framejam/reviews/rev_prod_launch_v1/frames/cmt_01.jpg"
    }
  ]
}
```
In addition to the raw JSON, FrameJam compiles a markdown prompt with inline image references so vision-enabled models immediately understand the visual context without ambiguity.

### 3.2 Playbook Structure (Narrative Architecture)
FrameJam decouples **what** to make from **how** it is rendered. For instance, the **Product Launch (`product-launch`)** playbook enforces a strict narrative grammar:
- **Audio/Beat Grid:** All visual cuts and text reveals snap to music transients (beat drop at 0:08).
- **Structure:**
  1. *0:00–0:03 Pain Point Hook:* Bold typography, agitated pacing.
  2. *0:03–0:08 The Solution:* Product name drop on music beat, high contrast.
  3. *0:08–0:25 The Core Capabilities:* 2.5D floating UI mockups, animated cursor/dwell, badge callouts.
  4. *0:25–0:35 Proof & Metrics:* Counter animations (`countUp`), customer quote card.
  5. *0:35–0:40 CTA End Card:* Terminal command or URL with ease-in badge.
- **Input Contract ("What you bring"):** App URL / recordings, pain point summary, brand assets, target aspect ratio.

### 3.3 Storyboard / Panel Mode Prior to Full Motion
Rendering full 60fps MP4s with FFmpeg or Remotion is computationally heavy and slows the agent feedback cycle. FrameJam supports reviewing static SVG/HTML storyboard cards (`panelsDir`, `panels`) before writing GSAP or Remotion timelines. Once the copy, layout, and visual sequence are approved, the agent converts the cards into animated timelines.

---

## 4. AeroEdits vs. FrameJam Comparison

| Capability | FrameJam | AeroEdits (Current State) |
| :--- | :--- | :--- |
| **Primary Focus** | Standalone generated videos (code-to-video from prompt/script). | Desktop NLE for screen, webcam, mic recording projects with automated editorial + graphics. |
| **Review UI** | Standalone local web server (`:2400`) with video player, filmstrip, and pin drop. | Native Tauri desktop app with Inspector, timeline, and Agent review panel (`agent/proposals.json`). |
| **Motion Engine** | Delegates to Hyperframes or Remotion CLI; relies on system Node/FFmpeg. | Embedded, pinned HyperFrames + GSAP 3.15 runtime; fully sandboxed; zero external PATH dependencies. |
| **Animation Review** | Full MP4 renders reviewed in browser; frame snapshots with pin markers. | Contact sheet moments via `frame_render` (single image containing sampled keyframes). |
| **Narrative / Editorial** | Playbook recipes (script-first, music-beat synchronized). | Telemetry-first (mouse dwell, clicks, active window, speech transcript alignment). |
| **Audio & SFX** | Playbook integrates local TTS (Kokoro) and background music tracks. | Multi-track mic/PC audio waveforms, silence cuts, loudness normalization (-14 LUFS). |
| **Style Tokens** | Mood/pacing presets (`list_presets`). | Style system: `--brand-*`, `--look-*` tokens, 16:9 / 9:16 layout rules, Light/Dark styles. |

---

## 5. Actionable Takeaways for AeroEdits

### 🟢 What We Should Adopt

1. **Spatial Pin & Region Annotations on Timeline (`NOTE-1` Enhancement):**
   - **Current AeroEdits state:** Agent inspects graphics via `frame_render` contact sheets. If the user wants a fix, they must type words in chat or the notes lane.
   - **Adoption:** Add an interactive "Annotation / Pin" tool in AeroEdits' native canvas. When the user clicks or drags a box on a graphic or layout in the preview player:
     - The app captures `(norm_x, norm_y)` or `(rect_x, rect_y, rect_w, rect_h)` and timeline timestamp `t`.
     - It creates a Note pinned to the graphic asset.
     - The MCP tool `notes_get` or `graphic_get_feedback` returns the exact spatial coordinates alongside the rendered crop.
     - This completely eliminates ambiguity when instructing agents to adjust margins, font sizes, or element overlaps.

2. **Explainer & Motion Graphics Playbooks in AeroEdits:**
   - **Current AeroEdits state:** We have Presets (`Tutorial`, `Shorts`, `Clips`), but they focus on silence cut thresholds, captions, and layout splits.
   - **Adoption:** Introduce **Explainer Playbooks** for tutorial videos and feature walkthroughs:
     - *SaaS Feature Explainer:* (Hook on speech → Zoom into screen telemetry click → Full-screen HyperFrames concept card → Split layout camera+graphic).
     - *Code/Terminal Walkthrough:* (Syntax-highlighted card with typed caret → Diagrammatic workflow card).
     - These playbooks give the agent a step-by-step editorial method to propose graphics and layouts that fit proven creator patterns.

3. **Storyboard / Card-First Preview before Motion Generation:**
   - When generating complex graphics, have the agent propose the static graphic layout first (`graphic_write_file` with static CSS/HTML) and verify layout via `frame_render` before adding multi-second GSAP timelines.
   - Once layout and copy are approved, the agent injects the timeline animation. This halves agent token usage and eliminates animation debugging before layout stability is achieved.

4. **Mood & Motion Pacing Tokens in Styles:**
   - Expand AeroEdits' `--look-*` tokens to include motion pacing presets:
     - `Snappy / Tech`: Fast GSAP tweens (0.2s–0.35s), aggressive `power3.out` or `back.out(1.7)`.
     - `Editorial / Calm`: Smooth easing (0.6s–0.9s), `power2.inOut` or custom cubic beziers.
     - `Kinetic / Punchy`: Staggered character reveals (`SplitText`), overshoot bounces.

5. **MCP Polling & Timeout Pattern:**
   - FrameJam's ~50s timeout pattern for `wait_for_feedback` with a clean `pending` loop is an essential reference for long-running human-in-the-loop agent workflows.

---

### 🔴 What We Refuse / Avoid

1. **Do NOT spin up a separate web server (`:2400`) or external browser tab:**
   - FrameJam requires an external web app because it has no GUI of its own and targets CLI agents (Claude Code, Cursor).
   - AeroEdits is already a high-performance Tauri desktop app with an embedded video player and canvas. All spatial review, pin drops, and feedback must live natively on the AeroEdits canvas.
2. **Do NOT adopt Remotion as a secondary graphics engine:**
   - Remotion introduces React compilation overhead, heavy Node dependencies, and commercial licensing restrictions for end users.
   - HyperFrames + GSAP 3.15 is lightweight, offline-capable, renders faster with WebM alpha, and keeps the asset representation as transparent, portable HTML/CSS/JS.
3. **Do NOT decouple graphics from the spoken transcript:**
   - FrameJam builds videos in a vacuum from prompt/script. In AeroEdits, graphics must remain strictly anchored to spoken words (`atWord`), reinforcing real claims, numbers, and steps rather than arbitrary decorative fluff.
