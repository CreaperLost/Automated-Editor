# Matt Loui: "Automate Your CapCut Editing With Claude" Research Report

- **Source:** [youtube.com/watch?v=U-AdnIz1jIk](https://www.youtube.com/watch?v=U-AdnIz1jIk)
- **Creator:** Matt Loui (CapCut Lab)
- **Title:** *Automate Your CapCut Editing With Claude (Step by Step + Free Skill)*
- **Category:** Creator Workflow & CapCut Draft Reverse-Engineering
- **Analyzed for:** AeroEdits AI Agent & Motion Graphics Architecture

---

## 1. Executive Summary

In this tutorial, creator and editor **Matt Loui** demonstrates how to connect **Claude** (via Claude Desktop and AI agent skills) to **CapCut Desktop** to automate video assembly, timeline cuts, B-roll placement, motion graphics, and audio mixing.

The technical breakthrough powering Matt's workflow is the programmatic manipulation of CapCut's internal timeline state file: **`draft_content.json`** (or `draft_info.json` on macOS). Because CapCut Desktop lacks an official API or MCP server, the agent directly reads, generates, and writes this JSON file inside the local draft directory (`com.lveditor.draft`). When the creator launches CapCut, the project opens completely populated with cuts, synchronized voiceover, kinetic text overlays, and keyframe zooms.

While this workflow showcases massive creator demand for agent-driven editing in mainstream tools, it is notoriously fragile (every CapCut software update risks breaking undocumented JSON schema tags). This report evaluates how AeroEdits can capitalize on this trend—both by offering a native, robust alternative and by supporting CapCut draft export as a delivery format.

---

## 2. Matt Loui's CapCut Automation Pipeline

```
Raw Footage / Script + Pinterest Moodboard
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│             Claude Agent (with CapCut Skill)                │
│  • Reads script and audio timestamps                        │
│  • Analyzes Pinterest reference images for visual aesthetic │
│  • Generates asset references (Higgsfield images / voice)   │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│         `draft_content.json` Generator / Manipulator        │
│  • Calculates clip start/end boundaries on video tracks     │
│  • Generates keyframe coordinate curves (scale 1.0 -> 1.08) │
│  • Injects text elements, transitions, and audio ducking    │
│  • Writes directly to `~/.../com.lveditor.draft/:id/`       │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│                 CapCut Desktop Application                  │
│  • Creator opens local draft; timeline is fully assembled   │
│  • Final manual review, color tweaks, and 1-click export    │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. Deep Dive: Key Technical Capabilities & Insights

### 3.1 The `draft_content.json` Reverse-Engineering Pattern
CapCut stores all project state in a structured JSON schema:
- **`tracks`**: Ordered array of timeline tracks (video, audio, text, sticker, filter, effect).
- **`segments`**: Timeline clips with `source_timerange` (in-point, duration) and `target_timerange` (placement on timeline in microseconds).
- **`keyframes`**: Parametric animations (scale, position X/Y, rotation, opacity).
- **`materials`**: Metadata definitions for media assets, font styles, text colors, and transition shaders.

By instructing Claude to write this schema directly, Matt achieves "hands-free" assembly. The creator avoids hours of dragging clips on a timeline.

### 3.2 Moodboard-Driven Motion & Aesthetic Direction
Matt demonstrates feeding **Pinterest / visual moodboard boards** to Claude:
- Claude extracts color palettes, text shadow treatments, and typographic personality (e.g. bold serif vs. minimalist sans) from the reference images.
- It translates these aesthetic attributes into the font, color, and animation parameters of the CapCut text tracks.
- This gives generated videos a curated, high-end "human" feel rather than generic template styles.

### 3.3 Dynamic Keyframe Camera Drift
A staple technique in CapCut social video is the **continuous slow zoom**:
- During static talking-head or screencast segments, a slow scale push-in from `1.0` to `1.06` over 3 to 5 seconds prevents visual monotony and boosts viewer retention.
- The agent automates this by writing start and end keyframes onto segment layers.

---

## 4. The Critical Weaknesses of CapCut Draft Hacking

While Matt's workflow is innovative, relying on reverse-engineered CapCut drafts has severe flaws:

| Problem in CapCut Hack | Root Cause | AeroEdits Native Advantage |
| :--- | :--- | :--- |
| **Silent Corruption on App Updates** | CapCut frequently changes internal schema properties (`app_version`, `new_version`), causing projects to fail to open or crash. | **Stable, typed MCP contracts:** AeroEdits has an official, first-party MCP server backed by strict Rust schemas. |
| **No Two-Way Review Loop** | Claude writes the JSON blind; if an element looks misaligned, Claude cannot see the visual result inside CapCut. | **Integrated visual review:** AeroEdits provides `frame_render` contact sheets and native canvas spatial pin feedback (`NOTE-1`). |
| **Blind to Telemetry** | CapCut has no concept of screen recording telemetry (mouse clicks, active windows, keyboard typing). | **Full telemetry awareness:** Smart zooms follow real clicks and typing carets automatically. |
| **No Undo or Rollback in Claude** | If the agent corrupts the draft, the creator has to rebuild the project manually. | **Persistent local undo:** Checkpoints in `history/` allow instant one-step rollback of entire agent runs. |

---

## 5. Actionable Takeaways for AeroEdits

### 🟢 What We Should Adopt

1. **CapCut Draft Export (`DLV-5` Delivery Target):**
   - Millions of creators love CapCut for its mobile app, viral music library, and native social effects.
   - We should build a native **Export to CapCut Draft** feature in AeroEdits:
     - The creator records and edits in AeroEdits (leveraging our automatic silence cuts, mouse telemetry zooms, and HyperFrames motion graphics).
     - On completion, AeroEdits can export a clean `draft_content.json` bundle that opens seamlessly inside CapCut Desktop for creators who want to add CapCut-exclusive trending stickers or mobile filters.
2. **Moodboard Image Input for Style Generation (`PLAN-1`):**
   - Adopt Matt's Pinterest technique: allow creators to supply 1 to 3 reference images (screenshots, moodboards, posters) to the Style System.
   - The agent analyzes the images, extracts dominant colors, typography style, and spacing rhythm, and generates an AeroEdits Style (`styles/`) automatically.
3. **Subtle Camera Drift / Push-In Automation (`FX-5`):**
   - Implement an automated "Subtle Camera Drift" pass for talking-head camera sections: applying a gentle 3% to 5% continuous scale drift during long continuous speech segments to elevate production value.

### 🔴 What We Refuse / Avoid

1. **Do NOT build our core editor around an undocumented third-party file format:**
   - Building a business on hacking CapCut's undocumented `draft_content.json` is suicidal; a single CapCut update can break the entire product overnight. AeroEdits’ core sequence model must remain 100% native, open, and owned by us.
2. **Do NOT adopt CapCut's cloud tracking and proprietary locks:**
   - CapCut Desktop increasingly forces cloud logins, uploads project telemetry to ByteDance servers, and locks features behind CapCut Pro subscriptions. AeroEdits remains strictly local, private, and offline-first.
