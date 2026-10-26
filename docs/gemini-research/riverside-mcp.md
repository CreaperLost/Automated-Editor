# Riverside MCP Research Report

- **Source:** [riverside.com/mcp](https://riverside.com/mcp) | [riversidefm/riverside-mcp-public](https://github.com/riversidefm/riverside-mcp-public)
- **Author:** Riverside.fm
- **License:** Proprietary SaaS (Grow / Pro Tier) with Open MCP Interface
- **Category:** Cloud Podcast & Studio NLE Model Context Protocol (MCP) Server
- **Analyzed for:** AeroEdits AI Agent & Motion Graphics Architecture

---

## 1. Executive Summary

**Riverside MCP** (`https://mcp.riverside.com/mcp`) is the official Model Context Protocol server provided by Riverside.fm. It allows AI assistants (Claude, ChatGPT, Cursor) to directly query, edit, cut, package, and distribute high-fidelity studio recordings through conversational natural language.

Its architecture is one of the most mature commercial video MCP implementations in the industry, exposing **over 60 tool methods** across 5 distinct namespaces (`editing_`, `search_`, `social_`, `platform_`, `media_`). Its standout strengths are **cross-library spoken transcript search**, **transactional batch editing (`editing_batch`)**, and **direct multi-platform social scheduling** without leaving the chat interface.

---

## 2. Core Architecture & Namespace Taxonomy

```
┌─────────────────────────────────────────────────────────────┐
│                 AI Assistant (Claude / ChatGPT)             │
└──────────────────────────────┬──────────────────────────────┘
                               │ JSON-RPC 2.0 over HTTPS
                               ▼
┌─────────────────────────────────────────────────────────────┐
│              Riverside Production MCP Server                │
│                 `https://mcp.riverside.com/mcp`             │
├─────────────────────────────────────────────────────────────┤
│ • `search_`   : Query spoken words across entire studio archive
│ • `editing_`  : Batch edits, transcript reading, clip cuts  │
│ • `platform_` : Studio projects, participants, tracks       │
│ • `social_`   : Direct scheduling to YouTube, TikTok, Reels │
│ • `media_`    : Asset handles and file export statuses      │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                   Riverside Cloud Infrastructure            │
│       • High-fidelity 4K local recording tracks             │
│       • AI silence & filler removal engine                  │
│       • Automated vertical shorts layout compiler           │
└─────────────────────────────────────────────────────────────┘
```

### 2.1 The 5 Tool Namespaces

| Namespace | Key Methods | Description & Operational Purpose |
| :--- | :--- | :--- |
| **`search_`** | `search_riverside`, `search_recording_transcripts_exact` | Semantic and exact-match search across all recordings in an account. Returns matching timestamps, speaker identities, and surrounding sentence context. |
| **`editing_`** (~40 tools) | `editing_read_aligned_transcript`, `editing_batch`, `editing_create_edit_from_segments`, `editing_reorder_timeline` | High-precision editorial control. `editing_read_aligned_transcript` streams word-aligned transcripts; `editing_batch` executes multiple edits atomically in a single network round-trip. |
| **`platform_`** (~13 tools) | `platform_get_studio`, `platform_list_projects`, `platform_get_tracks` | Manages multi-track session assets (separate microphone feeds, local 4K camera tracks, screen shares). |
| **`social_`** (~4 tools) | `social_schedule_clip`, `social_get_upload_status` | Direct distribution: takes an edited vertical short and schedules it directly to YouTube Shorts, TikTok, Instagram Reels, and LinkedIn. |
| **`media_`** (~3 tools) | `media_get_download_url`, `media_export_status` | Manages final rendering jobs and asset downloads. |

---

## 3. Deep Dive: Key Technical Capabilities

### 3.1 Transactional Batch Editing (`editing_batch`)
Individual MCP tool calls incur network latency and token costs. If an agent tries to make 25 individual jump-cuts by calling `cut_segment` 25 times, the session becomes sluggish and prone to mid-sequence network failures.

Riverside solves this with **`editing_batch`**:
- The agent prepares a complete array of editorial actions (cuts, layout changes, caption toggles).
- It passes the entire batch in a single call.
- The server executes the batch as an atomic transaction. If any segment is invalid, the entire operation rolls back safely.

### 3.2 Spoken Dialogue Discovery Across Archives
Most video editors only know about the file currently open. Riverside's MCP allows an agent to search across years of historical recordings:
- Query: *"Find where we discussed vector databases in any recording from last month."*
- Tool: `search_riverside(query: "vector databases")`
- Returns: Project ID, timestamp, and transcript snippet. The agent can immediately open that project and extract a clip.

### 3.3 Direct-to-Social Distribution Pipeline
Riverside removes the friction of downloading MP4s and manually re-uploading them to social media. An agent workflow can:
1. Search a 60-minute podcast for the most exciting 45-second hook.
2. Trim pauses and burn kinetic captions.
3. Call `social_schedule_clip` to publish it to YouTube Shorts and TikTok at optimal engagement hours.

---

## 4. AeroEdits vs. Riverside MCP Comparison

| Feature | Riverside MCP | AeroEdits |
| :--- | :--- | :--- |
| **Architecture** | Remote cloud SaaS (OAuth, HTTPS, cloud storage). | **Local-first desktop application** (Tauri + Rust on macOS/Windows). |
| **Privacy & Storage** | Media stored on Riverside cloud servers; paid tier required. | **100% private, local storage on user's disk** (`.aero` project bundles). |
| **Batch Editing** | `editing_batch` tool over JSON-RPC. | AeroEdits batch tools: `graphic_add` with `items`, all-or-nothing proposals. |
| **Recording Modality** | Remote studio (webcam, browser guest recording). | **Local screencast & camera** with mouse and keyboard telemetry. |
| **Motion Graphics** | Basic caption layouts and speaker cards. | **Full HyperFrames + GSAP 3.15 motion graphics engine.** |
| **History & Undo** | Cloud revision snapshots. | Full multi-gigabyte persistent local undo/redo (`history/`). |

---

## 5. Actionable Takeaways for AeroEdits

### 🟢 What We Should Adopt

1. **Transactional Batch Execution for All Timeline Tools:**
   - Riverside’s `editing_batch` is the industry gold standard for agent NLE control.
   - Ensure AeroEdits’ MCP server supports compound atomic batches across all tools (e.g. `timeline_batch` that applies cuts, smart zooms, layout splits, and graphic placements in a single transactional step with one undo checkpoint).
2. **Cross-Project Transcript Search (`search_recordings`):**
   - In AeroEdits, allow agents to search across all `.aero` projects on the user’s local machine. When a creator says: *"Where did I explain the auth hook last week?"*, the agent can locate the exact take in an older project, preview it, and pull that take or graphic into the current project.
3. **Automated Social Packaging & Delivery Recipes (`DLV-2`):**
   - Expand AeroEdits’ export tools so that when an agent creates a 9:16 Short, it also generates the accompanying YouTube/TikTok package: title, description, transcript-derived hashtags, and chapter timestamps.

### 🔴 What We Refuse / Avoid

1. **Do NOT become a cloud-dependent SaaS:**
   - Riverside requires costly monthly subscriptions, uploads gigabytes of raw video over consumer internet, and locks features behind account tiers. AeroEdits’ local-first, zero-subscription desktop architecture is a massive competitive moat.
2. **Do NOT hide tool methods or rely on opaque server-side black boxes:**
   - Riverside hides several internal editing primitives from public introspection. AeroEdits maintains fully open, strongly-typed contracts where every tool’s inputs and outputs are transparently visible to both user and agent.
