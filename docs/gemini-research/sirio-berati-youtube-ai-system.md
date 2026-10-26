# Sirio Berati: "How to ACTUALLY Edit your YouTube Videos w Claude" Research Report

- **Source:** [youtube.com/watch?v=dXz1qkLc9PQ](https://www.youtube.com/watch?v=dXz1qkLc9PQ) | [github.com/sirioberati/YouTube-AI-Video-Editing-System](https://github.com/sirioberati/YouTube-AI-Video-Editing-System)
- **Creator:** Sirio Berati
- **Title:** *How to ACTUALLY Edit your YouTube Videos w Claude (for real)*
- **Category:** Open-Source Automated Video Pipeline & Script-Guided Assembly
- **Analyzed for:** AeroEdits AI Agent & Motion Graphics Architecture

---

## 1. Executive Summary

In this video and companion open-source release, creator and developer **Sirio Berati** presents the **YouTube AI Video Editing System** (`sirioberati/YouTube-AI-Video-Editing-System`). It is an end-to-end programmatic pipeline that reduces a standard 6-to-8-hour YouTube editing session to under an hour using **Claude Code**, **Whisper**, and code-driven motion graphics.

The system's core innovation is solving the **"semantic gap"** in automated editing: rather than forcing an AI model to guess why a creator made a mistake from raw audio waveforms, Sirio integrates recording with a smart teleprompter (**Eyeline Teleprompter**) that logs script text, sentence IDs, and take restarts as metadata during the shoot. Downstream AI agents use this ground-truth data to perform deterministic take selection, ripple cuts, visual board design, and word-synchronized animation.

---

## 2. The 6-Stage Programmatic Editing Pipeline

```
[1. Recording with Eyeline Teleprompter]
    • Creator reads script sentence-by-sentence
    • Teleprompter logs: { sentence_id, take_num, start_time, end_time }
                  │
                  ▼
[2. Whisper Transcription & Verification]
    • High-precision word-level alignment
    • Cross-references spoken words against expected script text
                  │
                  ▼
[3. Autonomous Cutting via Claude Code]
    • Deterministic retake selection (e.g. keep final take per sentence_id)
    • Strips pauses, dead air, and filler words
                  │
                  ▼
[4. Claude Design (Visual Boards)]
    • Evaluates sentence meaning to design on-screen boards
    • Adheres to a structured design system (tokens, colors, typography)
                  │
                  ▼
[5. Programmatic Animation & Word Sync]
    • Motion graphics triggered precisely at spoken word timestamps
    • Kinetic text, lower-thirds, and concept callouts
                  │
                  ▼
[6. Final Assembly, Export & Metadata Packaging]
    • Renders final MP4 + generates YouTube title, description, and chapters
```

---

## 3. Deep Dive: Key Technical Capabilities & Insights

### 3.1 Eliminating the Semantic Gap via Teleprompter Metadata
Most automated editing tools (Gling, Descript, AutoCut) operate blind: they analyze audio waveforms and transcripts after the fact, struggling to differentiate between an intentional pause for dramatic effect and an awkward silence, or between a deliberate repetition for weight and an accidental retake.

Sirio solves this at the **point of capture**:
- The creator reads from the teleprompter.
- Whenever the creator stumbles and restarts a line, pressing a key or clicking resets the sentence counter, tagging the previous attempt as `retake_abandoned` and logging the new attempt as `take_2`.
- When Claude Code ingests the shoot data, it does not need to guess: it simply keeps the final valid take for every sentence ID.

### 3.2 Script-Guided Assembly (`script.json` Spine)
Instead of treating video as an opaque media file, Sirio’s pipeline structures the entire project as a typed JSON document:
```json
{
  "scenes": [
    {
      "id": "scene_01_hook",
      "script_text": "Stop editing your YouTube videos manually. Here is how to do it with Claude.",
      "selected_take": "take_3",
      "audio_range": [14.2, 19.8],
      "visual_board": {
        "template": "bold_kinetic_callout",
        "highlight_word": "Claude",
        "entrance": "punch_cut"
      }
    }
  ]
}
```
This structured spine allows the agent to reason mathematically about timing, motion entrance, and narrative pacing.

### 3.3 Claude Design: The "Visual Board" Architecture
For visual engagement, Sirio employs a "Visual Board" model. Rather than scattering random stock B-roll over the timeline, Claude generates branded cards and layout splits:
- Lower-third concept labels.
- Structured 3-item bullet cards.
- Code blocks with active highlights.
- Metric cards.
Each board is mapped to a specific spoken sentence and animated to hit on the core emphasis word.

---

## 4. AeroEdits vs. Sirio Berati’s System Comparison

| Dimension | Sirio Berati’s System | AeroEdits |
| :--- | :--- | :--- |
| **Intake Modality** | Smart teleprompter logging (`sentence_id`, takes). | Multi-track `.aero` recording (screen, webcam, mic, system audio, mouse/keyboard telemetry). |
| **Retake Detection** | Script-aligned take logging. | Speech transcript repetition rules ("we we", "j- just") + silence cuts; roadmap item `FTG-4` (script assembly). |
| **Visual Animation** | Programmatic visual boards. | Embedded **HyperFrames + GSAP 3.15** compositions anchored to `atWord`. |
| **Telemetry Support** | None (talking-head teleprompter only). | **Full mouse dwell, click clusters, active window crop, typing carets.** |
| **Interface** | CLI / scripts terminal pipeline. | Full desktop NLE with timeline GUI + AI Agent MCP panel. |
| **Safety & History** | Script re-runs. | Persistent local undo (`history/`, up to 2GB) + named checkpoints. |

---

## 5. Actionable Takeaways for AeroEdits

### 🟢 What We Should Adopt

1. **Script-Guided Take Assembly (`FTG-4` Implementation):**
   - Sirio’s workflow demonstrates the immense power of **script-guided assembly**.
   - In AeroEdits, allow creators to import a script (`script.txt` or `script.md`).
   - When the agent reviews raw footage, it matches the spoken transcript to the script lines. When a creator attempts a sentence 3 times, the agent automatically selects the best/final take and drops retakes 1 and 2 onto disabled/alternative lanes.
2. **The "Visual Board" Concept for Technical Tutorials:**
   - Formalize Sirio’s "Visual Board" concept inside AeroEdits’ graphic template library.
   - When a tutorial video introduces a complex topic, the agent deploys a full or split "Visual Board" (e.g. 3 key bullet takeaways, or a syntax card) that remains visible while the creator talks through the points.
3. **Automated YouTube Metadata Packaging on Export (`DLV-2`):**
   - Adopt Sirio’s automated publishing metadata generator. On final timeline export, generate a companion `publish_metadata.md` file containing:
     - Suggested YouTube titles (curated for click-through rate).
     - Full video description.
     - Accurate timestamped chapter markers derived directly from the timeline cuts and graphic titles.
     - Relevant search tags.

### 🔴 What We Refuse / Avoid

1. **Do NOT require a teleprompter setup as a prerequisite for editing:**
   - Sirio’s system only works if the creator reads a pre-written teleprompter script verbatim. AeroEdits is built for organic software tutorials, live coding, and natural technical screencasts where creators speak extemporaneously while working with tools.
2. **Do NOT force creators into terminal script executions:**
   - Sirio requires running terminal Python commands and managing raw scripts. AeroEdits packages all agent intelligence behind standard MCP tools and a clean native desktop GUI.
