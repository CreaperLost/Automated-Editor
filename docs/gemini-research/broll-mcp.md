# B-Roll MCP Research Report

- **Source:** [github.com/edigeh/broll-mcp](https://github.com/edigeh/broll-mcp)
- **Author:** Edigeh
- **License:** MIT
- **Category:** MCP Server for Automated B-Roll Retrieval, Scene Detection & Legal Provenance
- **Analyzed for:** AeroEdits AI Agent & Motion Graphics Architecture

---

## 1. Executive Summary

**Broll-MCP** (`edigeh/broll-mcp`) is an open-source Model Context Protocol server that allows AI coding and video agents (Claude Code, Cursor, Codex) to search, preview, cut, and download public-domain B-roll footage directly from the **Internet Archive (archive.org)** with automated legal provenance tracking.

Finding relevant B-roll to overlay on speech during explainers and tutorials is typically tedious and legally risky. Broll-MCP solves this by pairing natural-language search with **remote scene detection (PySceneDetect)**, **frame-accurate cut extraction**, and **automatic JSON license sidecars**.

---

## 2. Core Architecture & Workflow

```
[Agent Prompts: "Find 1960s NASA control room footage"]
                     │
                     ▼
         1. `search_footage` (Brief $\to$ Archive.org query)
                     │
                     ▼
         2. `inspect_item` (Scene detection over HTTPS)
                     │
                     ▼
         Keyframe snapshots returned as MCP images to Vision Model
                     │
                     ▼
         3. Agent selects exact scene timestamp & calls `extract_clip`
                     │
                     ▼
    ┌────────────────┴────────────────┐
    │                                 │
    ▼                                 ▼
[H.264 Faststart MP4]       [Provenance JSON Sidecar]
(Verified via ffprobe)      (MD5, CC0/PD license, attribution line)
```

### 2.1 Complete MCP Toolset

| Tool | Parameters | Description & Behavior |
| :--- | :--- | :--- |
| **`search_footage`** | `query`, `collection_tier`, `bw_preference`, `page` | Translates user brief into Internet Archive queries via a 2-stage filter (mandatory terms $\to$ optional terms). Caches results in local SQLite WAL database to prevent redundant upstream requests. |
| **`inspect_item`** | `identifier`, `mode` (`scenes` \| `keyframes`) | Runs PySceneDetect (with FFmpeg fallback) over remote HTTPS streams to identify shot cuts. Returns visual keyframe snapshots as MCP images for agent review without downloading full multi-gigabyte source files. |
| **`extract_clip`** | `identifier`, `start_time`, `end_time`, `output_path` | Downloads only the requested range via HTTP range requests, cuts frame-accurately, re-encodes to H.264 `faststart` MP4, verifies duration via `ffprobe` ($\pm 0.1\text{s}$ tolerance), and returns 3 check frames. |
| **`check_provenance`** | `clip_path` | Reads the companion JSON sidecar to verify license status, commercial use eligibility, and attribution text. |

---

## 3. Deep Dive: Key Technical Capabilities

### 3.1 Remote Scene Detection Without Full Ingest
Downloading 4GB archival video files just to check if a 5-second scene fits an explainer is completely impractical. Broll-MCP streams video headers and byte-ranges over HTTPS, runs **PySceneDetect** across the target segment, and returns lightweight image snapshots of each shot boundary. Vision agents inspect these snapshots and choose the exact segment before triggering an extraction.

### 3.2 The Strict Provenance Schema (`.json` Sidecars)
Commercial video creators face severe copyright and licensing risks when incorporating external B-roll. Broll-MCP guarantees legal compliance by writing an immutable JSON sidecar next to every clip before final output:

```json
{
  "clip_file": "nasa_control_room_1969.mp4",
  "source_url": "https://archive.org/details/nasa_apollo11_mission_control",
  "source_md5": "e4d909c290d0fb1ca068ffaddf22cbd0",
  "start_time": "00:14:22.500",
  "end_time": "00:14:28.000",
  "duration_seconds": 5.5,
  "license": {
    "type": "Public Domain Mark 1.0",
    "url": "https://creativecommons.org/publicdomain/mark/1.0/",
    "commercial_use_allowed": true,
    "attribution_required": false
  },
  "attribution_text": "Footage courtesy of NASA / Internet Archive (Public Domain Mark 1.0)"
}
```

### 3.3 Verification Checks on Cut Durations
Video decoders often struggle with remote keyframe seeking, leading to blank frames, freeze frames, or audio desync. Broll-MCP runs an autonomous verification step after each cut:
1. Executes `ffprobe` to ensure packet timestamps match requested start and end points within $\pm 0.1\text{s}$.
2. Samples 3 check frames (start, midpoint, end) to verify no decode corruption or blank frames occurred.

---

## 4. AeroEdits vs. Broll-MCP Comparison

| Capability | Broll-MCP | AeroEdits |
| :--- | :--- | :--- |
| **Purpose** | External archival B-roll search, preview, and extraction. | Desktop editor for multi-track screen, webcam, mic recordings + HyperFrames motion graphics. |
| **Integration** | Standalone Python MCP server via `uvx`. | Rust-based MCP server (`src-tauri/src/mcp/`) integrated with native project database and timeline. |
| **B-Roll Handling** | Downloads standalone MP4 clips to a directory. | `PlaceAsset` / `short_footage` timeline placement on B-roll video tracks. |
| **Audio Treatment** | Strips or keeps source audio without speech awareness. | Video-only stream placement (`PlaceAsset.streams`) preventing external audio from overriding voice tracks. |
| **Legal Provenance** | JSON sidecar with verified Creative Commons / Public Domain records. | Project asset metadata (`assets/` manifest). |

---

## 5. Actionable Takeaways for AeroEdits

### 🟢 What We Should Adopt

1. **Native B-Roll Search & Placement via Archive.org (`FTG-2` Enhancement):**
   - Incorporate an Archive.org / public-domain search capability into AeroEdits’ asset tools (`asset_search_archive` or `broll_find`).
   - When an explainer or tutorial discusses historical events, concepts, or real-world phenomena (e.g. *"Apollo guidance computer"*, *"early mainframe computers"*, *"satellite telemetry"*), the agent can automatically locate, preview, and drop relevant public-domain archival footage onto the timeline.
2. **Automated Provenance & Credit Generation (`DLV-2`):**
   - Adopt the `.json` sidecar provenance schema for all imported assets. On export, AeroEdits can automatically generate an end-credits card or YouTube video description section listing required Creative Commons attributions without manual creator effort.
3. **Stream-Safe B-Roll Placement Rule:**
   - As documented in AeroEdits' `FTG-2` roadmap, external B-roll must always be placed with **video-only streams** (`streams: ["video"]`). Never allow imported archival sound or hums to override the primary voice track or trigger unwanted silence-cut vetoes.
4. **Three-Frame Verification Check:**
   - Whenever an agent imports or cuts external footage, sample 3 check frames (head, middle, tail) to confirm the clip decoded cleanly before presenting it to the user.

### 🔴 What We Refuse / Avoid

1. **Do NOT run a separate Python process runtime (`uvx`) inside AeroEdits:**
   - Keep AeroEdits lightweight and self-contained. Any HTTP API requests to Archive.org metadata endpoints can be executed natively in Rust (`reqwest`) or through the frontend without requiring Python 3.12 dependencies on user systems.
2. **Do NOT place uninspected B-roll over critical screen telemetry:**
   - Archival B-roll must never obscure the active mouse click cluster or typing caret during technical tutorial steps. B-roll belongs over conversational speech stretches or high-level concept explanations.
