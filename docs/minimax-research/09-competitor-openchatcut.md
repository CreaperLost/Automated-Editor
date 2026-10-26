# 09 — A new competitor: OpenChatCut

- **Source:** [github.com/0xsline/OpenChatCut](https://github.com/0xsline/OpenChatCut) ·
  [openchatcut.com](https://openchatcut.com)
- **Licence:** **AGPL-3.0** · captured 25 Oct 2026 · 2,668★ · last push 2026-10-21 · TypeScript
- **Raw capture:** [evidence/2026-10-25/0xsline-openchatcut.md](evidence/2026-10-25/0xsline-openchatcut.md)
- **Evidence grade:** vendor claim, read from the source. Not installed, not benchmarked against
  AeroEdits.
- **Licence:** AGPL-3.0 is not on AeroEdits' allowed list. **Study it; do not copy code.**

## What it is

**"Open-source ChatCut alternative · agent-native · local-first AI video editor."** A desktop
editor — local video projects, a multitrack timeline, AI agents, MCP and production-ready exports
— with a built-in conversational agent *and* an external Streamable HTTP MCP for Codex and Claude
Code. AGPL-3.0, active development, 2,668★.

Its own comparison table, with the columns it chose:

| Capability | Traditional timeline editor | One-shot AI video generation | OpenChatCut |
|---|---|---|---|
| Track- and clip-level precision | ✅ | ❌ | ✅ |
| Modify a project with natural language | ❌ | ✅ | ✅ |
| **Inspectable and undoable changes** | ✅ | usually unavailable | ✅ |
| **Linked transcript and visuals** | partial | ❌ | ✅ |
| Direct control from Codex / Claude Code | ❌ | ❌ | ✅ MCP |
| Built-in and external agent collaboration | ❌ | ❌ | ✅ shared tools |
| Local projects and BYOK | product-dependent | usually cloud-based | ✅ |

## Why it is worth a file

AeroEdits' corpus knows **ChatCut** — the commercial product, reviewed on 21 Oct with its style
trial, selection references, curated graphics and masked desktop tools. It does not know
**OpenChatCut**, which is the open-source version of the same idea, and which now has its own
audience. Two of its claims map exactly onto AeroEdits' differentiators:

- **"Inspectable and undoable changes"** — AeroEdits' approve-before-apply, one undo step per
  batch, checkpoints before every agent run. Being claimed as a table row means it is now
  **expected**, not differentiating.
- **"Linked transcript and visuals"** — AeroEdits' word-anchored graphics, the transcript as the
  spine. Also now expected.

The interesting part is what it has that AeroEdits does **not**, and it is a long list:
WebGL effects, LUTs, chroma key, transitions and custom shaders · multiple audio tracks with
sound effects, background music, voice-over recording, loudness, auto-ducking and vocal isolation
· automatic captions with named styles, **translation**, timeline overlays and SRT export ·
AI generation of image, video, speech, music and sound effects with progress tracking · online
media search · **FCPXML export** · a **resource-aware export queue** · and **agent-driven publishing
to TikTok, Instagram, YouTube, LinkedIn, Facebook, X, Threads, Pinterest and Bluesky**, with a
preview and explicit confirmation first.

It also takes the Codex/ChatGPT-subscription route: "OpenAI · Codex" in the agent model picker,
with the official CLI owning credential storage and OAuth tokens never exposed to the browser.
AeroEdits' own position — the user's own agent over MCP or CLI, paid by the user's own
subscription — is arriving at a competitor with the same shape.

## The two things that are genuinely worth taking

1. **The publishing step.** Agent-driven publishing to nine platforms with a preview and explicit
   confirmation is a small feature with a large perceived value, and it is the obvious end of
   AeroEdits' delivery list (DLV-2). It also happens to be the one place where "no autonomous
   loops, never publish without the user's go" needs a hard edge.
2. **The comparison table itself** is worth reading as a market statement: it puts
   *"inspectable and undoable changes"* and *"linked transcript and visuals"* in the same row as
   *"MCP"*, as things a user should now expect from an agent editor. That is AeroEdits' thesis,
   stated as a baseline rather than a differentiator.

## Caveats

- **AGPL-3.0.** Study it; do not copy code into AeroEdits.
- It is a **React/TypeScript web app** with a Remotion rendering path — a different architecture
  from AeroEdits' Rust core and pinned HyperFrames runtime. Feature parity in a different stack is
  not a like-for-like comparison, and their "Motion Graphics · built-in templates, a secure
  sandbox, custom templates, and video rendering" is a thinner claim than AeroEdits', which
  renders editable HyperFrames compositions.
- Its telemetry, its real-world quality and its agent behaviour were not measured here. Its own
  feature table is marketing copy.
- It ships a **built-in chat agent** as well as MCP. AeroEdits' settled position is no chat panel
  in the app. If that holds, OpenChatCut's users can have a chat panel and AeroEdits' cannot — worth
  knowing what that costs in the market.
