# 0xsline/OpenChatCut

- **Source:** https://github.com/0xsline/OpenChatCut
- **Kind:** github
- **Captured:** 2026-10-25T15:07:58
- **Evidence grade:** vendor or creator claim. Not tested in this repo. Quotes from a caption are what a creator *said*, not a measurement.

## Repository

- **Repo:** 0xsline/OpenChatCut
- **Description:** Open-source, local-first conversational AI video editor with a professional multi-track timeline, Agent Skills, MCP integration, and Remotion rendering.
- **Homepage:** https://openchatcut.com
- **Stars:** 2668
- **Forks:** 408
- **Open issues:** 24
- **Licence (SPDX):** AGPL-3.0
- **Language:** TypeScript
- **Default branch:** main
- **Last push:** 2026-10-21T13:09:03Z
- **Archived:** False
- **Topics:** agent-skills, agentic-ai, ai-agent, ai-video-editor, capcut, chatcut, claude-code, codex, desktop-app, electron, ffmpeg, local-first, mcp, open-source, openchatcut, remotion, text-based-video-editing, timeline-editor, video-editing, video-editor

## README

```
<p align="center">
  <img src="public/openchatcut-icon.png" width="96" alt="OpenChatCut" />
</p>

<h1 align="center">OpenChatCut</h1>

<p align="center">
  <a href="README_ZH.md">简体中文</a> · <strong>English</strong>
</p>

<p align="center">
  <strong>Open-source ChatCut alternative · agent-native · local-first AI video editor</strong>
</p>

<p align="center">
  Let Codex, Claude Code, and the built-in agent read, edit, and export real video projects that remain fully editable.
  Website: <a href="https://openchatcut.com">openchatcut.com</a>
</p>

<p align="center">
  <a href="https://openchatcut.com/pro"><strong>OpenChatCut Pro</strong></a> · Desktop edition: install it and start cutting
</p>

<p align="center">
  <a href="#what-is-openchatcut">Introduction</a> ·
  <a href="#product-tour">Product Tour</a> ·
  <a href="#quick-start">Quick Start</a> ·
  <a href="#using-openchatcut-with-codex--claude-code">Agent / MCP</a> ·
  <a href="#community">Community</a> ·
  <a href="#sponsors">Sponsors</a> ·
  <a href="#changelog">Changelog</a> ·
  <a href="#star-growth">Star Growth</a> ·
  <a href="#contributing">Contributing</a>
</p>

<p align="center">
  <a href="https://github.com/0xsline/OpenChatCut"><img alt="GitHub Repository" src="https://img.shields.io/badge/GitHub-Repository-181717?style=flat&logo=github" /></a>
  <a href="https://discord.gg/bSGUAeWYkh"><img alt="Discord Community" src="https://img.shields.io/badge/Discord-Join_Community-5865F2?style=flat&logo=discord&logoColor=white" /></a>
  <img alt="Status" src="https://img.shields.io/badge/status-active_development-FF8A3D?style=flat" />
  <img alt="Local First" src="https://img.shields.io/badge/data-local_first-111827?style=flat" />
  <img alt="TypeScript" src="https://img.shields.io/badge/TypeScript-6-3178C6?style=flat&logo=typescript&logoColor=white" />
  <img alt="React" src="https://img.shields.io/badge/React-19-149ECA?style=flat&logo=react&logoColor=white" />
  <img alt="Remotion" src="https://img.shields.io/badge/Remotion-4-0B84F3?style=flat" />
  <img alt="Electron" src="https://img.shields.io/badge/Electron-43-47848F?style=flat&logo=electron&logoColor=white" />
  <img alt="MCP" src="https://img.shields.io/badge/MCP-Agent_native-7C3AED?style=flat" />
  <a href="https://atomgit.com/Oxsline/openchatcut"><img alt="AtomGit G-Star" src="https://img.shields.io/badge/AtomGit-G--Star-DA203E?logo=gitcode&logoColor=white" /></a>
</p>

<p align="center">
  <a href="https://linux.do" alt="LINUX DO"><img src="https://shorturl.at/ggSqS" /></a>
</p>

<p align="center">
  <a href="https://www.producthunt.com/products/openchatcut?embed=true&amp;utm_source=badge-featured&amp;utm_medium=badge&amp;utm_campaign=badge-openchatcut" target="_blank" rel="noopener noreferrer"><img alt="OpenChatCut - Open-source AI agent video editor with a real timeline | Product Hunt" width="250" height="54" src="https://api.producthunt.com/widgets/embed-image/v1/featured.svg?post_id=1201995&amp;theme=light&amp;t=1784645557617" /></a>
</p>

<p align="center">
  <img src="assets/readme-pic/01-editor-overview.png" alt="OpenChatCut editor overview with the agent workspace, media pool, preview, and multitrack timeline" />
</p>

<p align="center">
  <sub>From a single instruction to a real timeline: agents, media, previews, motion graphics, transitions, effects, and multitrack audio all work together in one project.</sub>
</p>

---
## Sponsors

<p align="center">
  <a href="https://ofox.ai/?utm_source=github&utm_medium=sponsorship&utm_content=openchatcut"><img src="assets/readme-pic/ofox-logo.svg" width="300" alt="OfoxAI" /></a>
</p>

**OfoxAI: Text, image, and video AI in one platform**

OfoxAI is a unified API platform for text, image, and video AI, with video models from the Seedance, MiniMax (Hailuo), and Wan families. Choose models for AI applications, agents, creative ideation, image generation, and video assets through one platform.
[Explore OfoxAI models and APIs →](https://ofox.ai/?utm_source=github&utm_medium=sponsorship&utm_content=openchatcut)

<br />

If OpenChatCut helps you, you can support its continued development on Ko-fi or Afdian.

<p align="center">
  <a href="https://ko-fi.com/Y5N2241IP5">
    <img alt="Support me on Ko-fi" src="https://img.shields.io/badge/Support_me_on-Ko--fi-72a4f2?logo=kofi&amp;logoColor=white" />
  </a>
  <a href="https://www.ifdian.net/a/sline?utm_source=copylink&amp;utm_medium=link">
    <img alt="Support OpenChatCut on Afdian" src="https://img.shields.io/badge/Support_on-Afdian-946CE6" />
  </a>
</p>

---

## What is OpenChatCut?

OpenChatCut is an **open-source ChatCut alternative**: a video editor that brings **conversational agents** and **professional timeline editing** into the same workspace. It is independent open source (AGPL), not affiliated with the commercial ChatCut product.

**OpenChatCut = local video projects + multitrack timeline + AI agents + MCP + production-ready exports.**

It does not merely generate a video that can no longer be changed. Every edit is written to real tracks, clips, transitions, captions, effects, and media inside the project. You can continue editing manually, undo or redo changes, save versions, or hand the project to another agent.

OpenChatCut is built for creators and developers who want AI to participate in the actual editing workflow without giving up professional control, rather than starting over from an empty chat box or an immutable generated result.

- Website: [https://openchatcut.com](https://openchatcut.com)
- Open-source ChatCut alternative guide: [https://openchatcut.com/blog/open-source-chatcut-alternative](https://openchatcut.com/blog/open-source-chatcut-alternative)
- ChatCut vs OpenChatCut: [https://openchatcut.com/blog/chatcut-vs-openchatcut](https://openchatcut.com/blog/chatcut-vs-openchatcut)

- 🤖 **Agent-native**: the built-in agent and external MCP agents share the same editing tools.
- 🎞️ **Real timeline**: multiple video and audio tracks, transitions, effects, LUTs, zooms, and keyframes.
- 📝 **Transcript-driven editing**: word-level transcription, text-based cuts, pause handling, speakers, and linked captions.
- ✨ **Generation and media**: images, video, speech, music, sound effects, and online media search.
- 🧩 **Motion Graphics and WebGL**: editable motion templates, custom shaders, visual effects, and transitions.
- 👁️ **Visual geometry**: in-browser person segmentation and face-safe zones — captions avoid the speaker automatically, reframe follows the subject, and overlay graphics land in empty space.
- 📦 **Production-ready exports**: MP4, audio, captions, FCPXML, and complete project data.
- 🖥️ **Local-first**: projects and media stay on your machine by default, while API keys remain server-side.

## Community

Join the OpenChatCut community to discuss workflows, share feedback, and follow development:

- [Join Discord](https://discord.gg/bSGUAeWYkh)
- Scan the QR code below with WeChat.

<p align="center">
  <br />
  <img src="assets/readme-pic/wechat-community.jpg" width="220" alt="OpenChatCut WeChat community QR code" />
</p>

---

## Product Tour

The following screenshots show real OpenChatCut projects and editor states, not static mockups.

<table>
  <tr>
    <td width="50%" valign="top" align="center">
      <img src="assets/readme-pic/02-project-dashboard.png" alt="OpenChatCut local project dashboard" /><br />
      <sub><b>Local project management</b> — Create, import, duplicate, export, and continue editing multiple real projects.</sub>
    </td>
    <td width="50%" valign="top" align="center">
      <img src="assets/readme-pic/03-agent-transitions.png" alt="An agent generating music and editing transitions on a multitrack timeline" /><br />
      <sub><b>Complete agent-driven editing</b> — Generate music, invoke tools, and write transitions, captions, and multitrack media directly to the timeline.</sub>
    </td>
  </tr>
  <tr>
    <td width="50%" valign="top" align="center">
      <img src="assets/readme-pic/04-motion-graphics.png" alt="Agent tool execution history and the Motion Graphics library" /><br />
      <sub><b>Motion Graphics and agents</b> — Browse motion templates or let an agent generate and assemble editable MG clips.</sub>
    </td>
    <td width="50%" valign="top" align="center">
      <img src="assets/readme-pic/05-effects.png" alt="OpenChatCut WebGL visual effects library" /><br />
      <sub><b>WebGL visual effects</b> — Apply pixelation, duotone, fisheye, kaleidoscope, softening, light leaks, and more directly to clips.</sub>
    </td>
  </tr>
  <tr>
    <td width="50%" valign="top" align="center">
      <img src="assets/readme-pic/06-zoom.png" alt="OpenChatCut camera movement and zoom effects library" /><br />
      <sub><b>Camera movement and zoom</b> — Push, pull, slow zoom, fast zoom, and eased camera effects work directly with the timeline.</sub>
    </td>
    <td width="50%" valign="top" align="center">
      <img src="assets/readme-pic/07-lut.png" alt="Comparing LUT color styles in OpenChatCut using a Tokyo Tower reference image" /><br />
      <sub><b>LUTs and color styles</b> — Compare camera transforms and film looks in real time against a consistent reference frame.</sub>
    </td>
  </tr>
</table>

---

## Why OpenChatCut?

Traditional editors excel at precise control. One-shot AI video generators excel at producing results quickly. OpenChatCut connects both approaches through one continuously editable project:

| Capability | Traditional timeline editor | One-shot AI video generation | **OpenChatCut** |
|---|:---:|:---:|:---:|
| Track- and clip-level precision | ✅ | ❌ | **✅** |
| Modify a project with natural language | ❌ | ✅ | **✅** |
| Inspectable and undoable changes | ✅ | Usually unavailable | **✅** |
| Linked transcript and visuals | Partial | ❌ | **✅** |
| Direct control from Codex / Claude Code | ❌ | ❌ | **✅ MCP** |
| Built-in and external agent collaboration | ❌ | ❌ | **✅ Shared tools** |
| Local projects and BYOK | Product-dependent | Usually cloud-based | **✅** |

The core editing loop:

```text
Describe the goal → Agent reads the project → Produces verifiable edits → Writes to the timeline
                  → Preview / adjust / undo → Captions and mixing → Export
```

---

## Core Capabilities

| Area | Implemented capabilities |
|---|---|
| Timeline | Multitrack editing, move, trim, split, ripple edits, snapping, keyframes, markers, undo, and redo |
| Visuals | WebGL effects, LUTs, chroma key, zoom, transitions, and custom shaders |
| Audio | Multiple audio tracks, sound effects, background music, voice-over recording, loudness, auto-ducking, and vocal isolation |
| Transcript | Transcription jobs, word-level editing, pause compression, search, speakers, and clip views |
| Captions | Automatic captions, named styles, translation, timeline overlays, and SRT export |
| Motion Graphics | Built-in templates, a secure sandbox, custom templates, and video rendering |
| AI generation | Image, video, speech, music, and sound-effect jobs with progress tracking |
| Media | Uploads, folders, online image/video/audio search, and Firecrawl visual-media fallback |
| Export | MP4, audio, captions, FCPXML, project import/export, export history, hardware-aware H.264 acceleration, and resource-aware export queueing |
| Publishing | Agent-driven publishing of finished renders to TikTok, Instagram, YouTube, LinkedIn, Facebook, X, Threads, Pinterest, and Bluesky via Upload-Post, with a preview and explicit confirmation first |
| Agent | Built-in conversational agent, skills, proposal-based edits, and external Streamable HTTP MCP |

---

## Community Resources

The [OpenChatCut resource library](https://openchatcut.com/resources) is a shared catalog for reusable MG animations, sound effects, transitions, visual effects, zooms, and LUTs.

<p align="center">
  <a href="https://openchatcut.com/resources">
    <img src="assets/readme-pic/08-community-resources.en.png" alt="OpenChatCut community resource library" />
  </a>
</p>

### Discover and install

- Hover visual cards to watch the complete result, or play audio resources before downloading.
- Copy a resource's install URL into OpenChatCut's Extension Center, or download its original package.
- Browse the website catalog inside the editor and manage installed extensions locally.

### Contribute a resource

1. Open [Contribute a resource](https://openchatcut.com/resources/submit) and choose its category.
2. Upload the resource and the category-specific preview inputs; the site renders the published preview.
3. Add the creator and license information, then submit it for review and publication.

Installable visual resources use the editor's `openchatcut-plugin@1` format and runtime validation. Official OpenChatCut resources use the MIT license; community contributors select a license during submission, and each published card identifies its creator and license.

---

## Use Cases

- **Talking-head and interview editing**: transcribe audio or video, remove mistakes, pauses, and repetition through text, then generate captions automatically.
- **Fast multiformat assembly**: import video, images, and audio, then let an agent create the rough cut, transitions, soundtrack, and pacing.
- **Short-form and social content**: reframe the canvas and generate titles, captions, voice-over, music, and visual packaging.
- **Motion Graphics**: use built-in templates or ask an agent to create editable motion-graphics clips.
- **Developer automation**: use MCP to let Codex, Claude Code, or another compatible client inspect and modify a real project.

## Workflow

1. Create a project and import local media.
2. Edit manually on the timeline or describe the result you want.
3. The agent reads project context and invokes editing tools.
4. Review the proposal and preview the result, then apply, adjust, or undo it.
5. Finish captions, audio, effects, and color.
6. Export video, audio, captions, FCPXML, or the complete project.

---

## Quick Start

### Desktop installers

Download the latest macOS, Windows, and Linux builds from [GitHub Releases](https://github.com/0xsline/OpenChatCut/releases/latest). The release currently includes DMG installers for Apple Silicon and Intel Macs, an x64 Windows installer, and an x64 Linux AppImage.

These are early builds. The macOS packages are not yet signed or notarized, so the operating system may require manual approval on first launch.

### Run from source

Requires Node.js 24.x and npm. The supported Node.js range is enforced by `package.json`, and `.nvmrc` selects the matching major version for Node version managers.

On Linux x64, `npm install` also downloads ONNX Runtime's optional CUDA provider (several hundred MB from NuGet). For a CPU-only install, run `ONNXRUNTIME_NODE_INSTALL=skip npm install` instead.

```bash
git clone https://github.com/0xsline/OpenChatCut.git
cd OpenChatCut
npm install
cp .env.example .env.local
npm run dev
```

Open:

```text
http://localhost:5199
```

The editor needs a browser [secure context](https://developer.mozilla.org/en-US/docs/Web/Security/Defenses/Secure_Contexts): open OpenChatCut at `http://localhost` or `http://127.0.0.1` on the machine running it, or serve it over HTTPS. Over plain HTTP from another device (a LAN IP or reverse proxy), browsers disable Web Crypto, WebCodecs, and the clipboard, so the project list still loads but the editor shows this requirement instead of opening.

Only add the model or media-service credentials you actually use to `.env.local`. Features without configured third-party credentials report the missing key explicitly; local timeline editing, built-in media, and other configured capabilities continue to work.

Development launches are isolated per Git checkout/worktree by default. `npm run dev` and
`npm run desktop:dev` keep that checkout's projects, imported media, generation jobs,
credentials, settings, and local authorization state under its own profile in
`~/.openchatcut/dev-profiles/`. Use `npm run dev:shared` only when you intentionally need
the legacy shared development store.

### Built-in Agent authentication

- **API keys:** open **Settings → Agent model**, choose a provider, and save its API key and model. Keys remain server-side.
- **ChatGPT subscription:** install the official Codex CLI 0.146.0 or newer, then open **Settings → Agent model → OpenAI · Codex**. Sign in through the browser or device-code flow, load the account's models, choose a model-specific reasoning effort (or keep its default), and select Codex from the chat model picker. OpenChatCut uses a dedicated Codex profile; the official CLI owns credential storage, token renewal, and logout, while OAuth tokens are never exposed to the browser.
- **Claude subscription:** OpenChatCut does not collect Claude OAuth credentials. Use Claude Code through the local MCP connection below. The built-in Agent can use Anthropic through an API key.

The built-in Agent always runs the model loop on the local server. Chat, drafts, and proposals survive page refreshes and local service restarts. Timeline changes still go through the active editor's validated, undoable commands.

### Agent local-path access (advanced)

Desktop agents can browse any local directory allowed by the operating system by default, including mounted media drives. `browse_local_media` lists folders and supported media, searches by filename or relative path and media type, and returns paginated results without importing. The agent can then use `import_assets` for up to 100 selected paths, `import_asset` for one path, or `import_folder` for a whole folder. Imports reuse media probing and reference registration, preserve source files, and skip duplicate content, including duplicates within one batch. Document files such as TXT, Markdown, DOCX, and PDF should be attached to chat instead. In the desktop app these tools are enabled by default; in a browser session they also work, but only for directories listed in `AGENT_IMPORT_ROOTS` and while the editor is served from the same machine that runs OpenChatCut.

To restrict local access, set a comma-separated value such as `AGENT_IMPORT_ROOTS=/Volumes/Media,D:\Projects` in `.env.local`; leave it empty for default unrestricted directory access in the desktop app (a browser session requires the allowlist and refuses these tools while it is empty). Explicit restrictions apply to both browsing and import. Source checkouts read the repository file at startup. Packaged desktop builds read it from Electron's user-data directory: `~/Library/Application Support/OpenChatCut/` on macOS, `%APPDATA%\OpenChatCut\` on Windows, and `$XDG_CONFIG_HOME/OpenChatCut/` (normally `~/.config/OpenChatCut/`) on Linux. Restart after editing the file.


Local H.264 exports automatically prefer VideoToolbox on macOS and NVENC on compatible Windows systems, then fall back to software encoding. Tune render concurrency and the heavy-export limit with `OPENCHATCUT_RENDER_CONCURRENCY` and `OPENCHATCUT_MAX_ACTIVE_EXPORTS`, disable hardware encoding with `OPENCHATCUT_DISABLE_HARDWARE_ENCODING`, or override FFmpeg-side encoder selection with `OPENCHATCUT_H264_ENCODER`; see [`.env.example`](.env.example).

On Apple Silicon, more render workers can slow down video-heavy exports even when VideoToolbox encoding is active. See [macOS render tuning](MACOS-RENDER-TUNING.md) for an M4 Max benchmark and a way to test your own workload. These measurements do not establish a default for other Macs or Windows.

### Desktop development

```bash
npm run desktop:dev
```

The desktop app uses an Electron shell with the same embedded services. The web development build and desktop build share project, agent, generation, and export logic.

---

## Project Status

OpenChatCut is under active development. The editor, project format, and agent tools will continue to evolve. Prebuilt macOS, Windows, and Linux installers are published on [GitHub Releases](https://github.com/0xsline/OpenChatCut/releases); running from source remains the most transparent option for development and troubleshooting.

The core timeline, local projects, built-in media, and manual editing do not depend on cloud services. Connected features such as AI models, online media, generation, and transcription are enabled only after you configure the corresponding services.

---

## Using OpenChatCut with Codex / Claude Code

Install the single OpenChatCut Agent Skill:

```bash
npx skills add 0xsline/OpenChatCut
```

Then tell the agent `Set up OpenChatCut`. The installed router registers the
local MCP connection and loads the editor's 26 specialized skills on demand,
so the agent's skill list stays compact.

Custom skills live in a user-visible directory with the same layout as
`~/.codex/skills` / `~/.claude/skills`:

```text
~/.openchatcut/skills/<slug>/SKILL.md     (Windows: %USERPROFILE%\.openchatcut\skills\...)
```

Ask the agent to `manage_skill create` (or use the bundled *skill-creator*
workflow) and it writes the SKILL.md there — editable and shareable by hand.
Dropping a SKILL.md into that directory makes it available to the agent, and
selecting a workflow from the chat's `/` command or the creative-mode picker
activates it for the next message.

OpenChatCut exposes a Streamable HTTP MCP endpoint:

```text
http://localhost:5199/api/external-mcp/mcp
```

The repository's root `.mcp.json` already contains the local connection. Before using timeline tools, run OpenChatCut and open the target project. Project listing, creation, and navigation tools do not require the editor to remain open.

Codex app/CLI and Claude Code use the same session workflow:

1. Call `begin_edit_session` and keep the returned `editSessionId`. Set `approvalMode` to `manual` (the default) or `auto`.
2. Pass that id to every project read/edit tool. Changes are applied only to an isolated draft.
3. Call `review_edit_session` when the draft is ready.
4. In `manual` mode, review, preview, select, and approve or reject the proposal inside the open OpenChatCut project. In `auto` mode, `review_edit_session` applies the complete draft immediately. The client can poll `get_edit_session` for `applied`, `rejected`, or `discarded` status.

Approval in Codex or Claude controls whether the client may call a tool. OpenChatCut project approval is required only for `manual` sessions; `auto` sessions explicitly bypass that review. Either mode commits the applied operations atomically as one undo step.
If an `auto` session becomes stale, it returns an error instead of falling back to manual approval; discard it and start a new session.
Only draft-safe project read/edit tools are exposed in these sessions. Generation, export, project deletion, and other immediate side-effect tools stay unavailable because a rejected proposal could not roll them back.

### Codex

Add the following to your Codex configuration:

```toml
[mcp_servers.openchatcut]
url = "http://localhost:5199/api/external-mcp/mcp"
```

### Claude Code

```bash
claude mcp add --transport http openchatcut \
  http://localhost:5199/api/external-mcp/mcp
```

You can then describe an editing task directly to the agent:

```text
Start an OpenChatCut edit session. Read the draft, add a scratch sound effect
at 8 seconds on the second audio track, and add a glitch transition between
the adjacent video clips. Submit the draft for review and wait for the user to
approve it in OpenChatCut before reporting that the edit was applied.
```

External agents invoke the same internal editing tools and `EditorCore` commands as the editor itself. There are no separate project formats that can drift apart, and the live timeline is not changed while an external draft is being prepared.

### Qoder

Qoder is one-click too: the guide's **Connect** button writes the endpoint and
bearer token into Qoder's own settings file, keeping any unrelated settings and
MCP servers already there.

```text
~/.qoder/settings.json        (international build)
~/.qoder-cn/settings.json     (China build — written as well when installed)
```

```json
{
  "mcpServers": {
    "openchatcut": {
      "type": "http",
      "url": "http://localhost:5199/api/external-mcp/mcp",
      "headers": { "Authorization": "Bearer <token>" }
    }
  }
}
```

Restart Qoder afterwards to load the server.

### Qwen Work (千问办公)

千问办公 keeps custom MCP servers inside its own connector store rather than a
config file, so OpenChatCut cannot write this one for you. In the guide, use
**Copy config** on the 千问办公 card, then in 千问办公 open
连接器 → 自定义 MCP → 添加, paste the JSON, and save:

```json
{
  "mcpServers": {
    "openchatcut": {
      "type": "streamable-http",
      "url": "http://localhost:5199/api/external-mcp/mcp",
      "headers": { "Authorization": "Bearer <token>" }
    }
  }
}
```

### Protecting MCP access

When exposing the MCP endpoint yourself, configure:

```bash
OPENCHATCUT_MCP_TOKEN=your-token
OPENCHATCUT_EDITOR_URL=https://your-editor.example.com
```

Clients must send `Authorization: Bearer <token>`. The current bridge is designed for a single user on one machine, not as a multi-tenant service.

---

## Architecture

<p align="center">
  <img src="assets/readme-pic/openchatcut-runtime.en.svg" alt="OpenChatCut runtime architecture connecting agents, MCP, EditorCore, local storage, preview, rendering, and export" />
</p>

<p align="center">
  <sub>One shared set of agent tools and EditorCore commands connects the built-in agent, external MCP clients, the real timeline, local data, and production exports.</sub>
</p>

| Layer | Technology |
|---|---|
| Frontend | React 19, TypeScript 6, Vite 8 |
| Editing core | Immutable timeline state, command layer, and proposal-based application |
| Agent | Vercel AI SDK 7 (Anthropic, OpenAI, Gemini, Kimi, Qwen, GLM, DeepSeek, MiniMax, Xiaomi MiMo, Mistral, xAI Grok by API key or SuperGrok/X Premium+ subscription sign-in, OpenRouter, OrcaRouter, OFox, Requesty, Cheaper Inference, and compatible APIs), Agent Skills, MCP SDK |
| Preview and visuals | Remotion Player, WebGL / GLSL |
| Server | Dual-host Vite / Electron plugins and a server-side keystore |
| Persistence | Shared local project store under `~/.openchatcut`, IndexedDB cache, configurable local media directory, optional Cloudflare R2 |
| Desktop | Electron 43 |
| Export | Remotion, FFmpeg, FCPXML, SRT |

### Directory Overview

| Directory | Responsibility |
|---|---|
| `src/editor/` | Timeline state and commands, independent of UI and LLMs |
| `src/agent/` | Agent assembly, tools, skills, progress, and settings |
| `src/library/` | Library UI for Motion Graphics, sound effects, transitions, effects, LUTs, and more |
| `src/transcript/` | Transcription, word-level editing, and transcript UI |
| `src/captions/` | Caption models, styles, controls, and preview layer |
| `src/gl/` | WebGL effects, transitions, and shader runtime |
| `src/generate/` | Clients for image, video, speech, music, and sound-effect generation |
| `src/persist/` | Project, chat, version, and media persistence |
| `server/plugins/` | Generation, transcription, media, export, and storage services |
| `desktop/` | Electron main process and embedded services |
| `remotion/` | Headless rendering and export pipeline |

---

## Data and Privacy

- Projects, chat history, and version data are stored in the shared local store under `~/.openchatcut`; IndexedDB provides a browser-side cache and legacy-data migration.
- User media is stored in a local media directory that you can back up or migrate.
- Whether AI requests leave your machine depends on the model, generation, or media services you configure.
- Unconfigured cloud features do not affect the local timeline or editing of existing media.
- When exposing MCP externally, configure a Bearer Token and restrict network access to the editor endpoint.

---

## Security Model

- API keys are only available to server-side configuration; vendor credentials must never be exposed to the browser through `VITE_`.
- LLM output, plugin packages, template code, and user input are validated at trust boundaries.
- Motion Graphics and shader code run in a restricted sandbox, with dedicated checks blocking malicious templates.
- Agents modify projects only through `EditorCore` commands, making edits traceable and undoable.
- MCP binds locally by default; public endpoints support Bearer Token authentication.
- Local media directories and R2 credentials are managed by the server and never written into project JSON.

---

### Fal.ai image and video generation

Fal.ai is an optional generation provider with an explicit image/video model catalog.
Configure it under **Settings → AI Generation → Image or Video → Fal.ai**.
See [Fal setup, supported models, and extension guide](FAL.md) for supported modes,
server-only credentials, and checks that do not spend generation credits.

## Development and Verification

```bash
# Type checking and production build
npm run build

# Core regression checks
npm test

# Static analysis
npm run lint

```

After changing the agent, timeline, preview, or export paths, run at least:

```bash
npx tsc -b --force
npm test
npm run build
```

---

## Technical Foundations

OpenChatCut is built with the following core projects and specifications:

| Project / specification | Role in OpenChatCut |
|---|---|
| [ChatCut-Inc/agent-plugin](https://github.com/ChatCut-Inc/agent-plugin) | Agent Skills foundation. OpenChatCut adapts the plugin's skill structure and workflows for its local editor, storage, MCP, and tool architecture. See the [Agent Skills attribution notice](src/agent/skills/NOTICE.md). |
| [Remotion](https://www.remotion.dev/) | Core foundation for React-based video preview, composition, and server-side rendering. |
| [Model Context Protocol](https://modelcontextprotocol.io/) | Protocol foundation that enables Codex, Claude Code, and other external agents to access projects and timeline tools. |
| [Vercel AI SDK](https://ai-sdk.dev/) | Provider-neutral model streaming and tool calling for the built-in Agent. |
| [tt-a1i/archify](https://github.com/tt-a1i/archify) | Tool used to define, validate, and generate the README's runtime architecture diagram. |

JianYing / CapCut draft export is powered by [capcut-cli](https://github.com/renezander030/capcut-cli).

This list covers the project's major technical foundations. It does not replace the licenses bundled with individual dependencies, fonts, or binaries. See `package-lock.json` for JavaScript dependency versions and [`assets/fonts/LICENSES.md`](assets/fonts/LICENSES.md) for font licenses.

---

## Changelog

See the bilingual [`CHANGELOG.md`](CHANGELOG.md) for notable changes, or browse all published packages on [GitHub Releases](https://github.com/0xsline/OpenChatCut/releases).

---

## Star Growth

<p align="center">
  <a href="https://www.star-history.com/?type=date&repos=0xsline%2FOpenChatCut">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/chart?repos=0xsline/OpenChatCut&type=date&theme=dark&legend=top-left&sealed_token=KKfeYtGGCjyG1QN9_Ev6Tvyyrcp5LW6bzOT8ZKED1EE0qNRqM3KrThzzbXWdcP6K-sr3vKbmoFZYDviSMtf8SI5UqAPYQf9v8qXCpM04S2C4LQTAKPbexT66SI3Q8pcHJJoMT7VCZnGp93LqIXZchAyYfTMmKy_y_LFOJ-_ruEq8GP1kVESXshaFzJfC" />
      <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/chart?repos=0xsline/OpenChatCut&type=date&legend=top-left&sealed_token=KKfeYtGGCjyG1QN9_Ev6Tvyyrcp5LW6bzOT8ZKED1EE0qNRqM3KrThzzbXWdcP6K-sr3vKbmoFZYDviSMtf8SI5UqAPYQf9v8qXCpM04S2C4LQTAKPbexT66SI3Q8pcHJJoMT7VCZnGp93LqIXZchAyYfTMmKy_y_LFOJ-_ruEq8GP1kVESXshaFzJfC" />
      <img alt="OpenChatCut Star History Chart" src="https://api.star-history.com/chart?repos=0xsline/OpenChatCut&type=date&legend=top-left&sealed_token=KKfeYtGGCjyG1QN9_Ev6Tvyyrcp5LW6bzOT8ZKED1EE0qNRqM3KrThzzbXWdcP6K-sr3vKbmoFZYDviSMtf8SI5UqAPYQf9v8qXCpM04S2C4LQTAKPbexT66SI3Q8pcHJJoMT7VCZnGp93LqIXZchAyYfTMmKy_y_LFOJ-_ruEq8GP1kVESXshaFzJfC" />
    </picture>
  </a>
</p>

---

## License

OpenChatCut is licensed under the [GNU Affero General Public License v3.0 or later](LICENSE).
Third-party components and assets remain subject to their respective licenses.

---


## Contributing

1. Create a branch from `main`.
2. Add an executable check for non-trivial logic.
3. Run `npm test`, `npm run lint`, and `npm run build` before committing.
4. Open a Pull Request and include screenshots or acceptance evidence for UI or video-behavior changes.

Use [GitHub Issues](https://github.com/0xsline/OpenChatCut/issues) for bug reports and feature requests.
```

## Licence file

```
GNU AFFERO GENERAL PUBLIC LICENSE
                       Version 3, 19 November 2007

 Copyright (C) 2007 Free Software Foundation, Inc. <https://fsf.org/>
 Everyone is permitted to copy and distribute verbatim copies
 of this license document, but changing it is not allowed.

                            Preamble

  The GNU Affero General Public License is a free, copyleft license for
software and other kinds of works, specifically designed to ensure
cooperation with the community in the case of network server software.

  The licenses for most software and other practical works are designed
to take away your freedom to share and change the works.  By contrast,
our General Public Licenses are intended to guarantee your freedom to
share and change all versions of a program--to make sure it remains free
software for all its users.

  When we speak of free software, we are referring to freedom, not
price.  Our General Public Licenses are designed to make sure that you
have the freedom to distribute copies of free software (and charge for
them if you wish), that you receive source code or can get it if you
want it, that you can change the software or use pieces of it in new
free programs, and that you know you can do these things.

  Developers that use our General Public Licenses protect your rights
with two steps: (1) assert copyright on the software, and (2) offer
you this License which gives you legal permission to copy, distribute
and/or modify the software.

  A secondary benefit of defending all users' freedom is that
improvements made in alternate versions of the program, if they
receive widespread use, become available for other developers to
incorporate.  Many developers of free software are heartened and
encouraged by the resulting cooperation.  However, in the case of
software used on network servers, this result may fail to come about.
The GNU General Public License permits making a modified version and
letting the public access it on a server without ever releasing its
source code to the public.

  The GNU Affero General Public License is designed specifically to
ensure that, in such cases, the modified source code becomes available
to the community.  It requires the operator of a network server to
provide the source code of the modified version running there to the
users of that server.  Therefore, public use of a modified version, on
a publicly accessible server, gives the public access to the source
code of the modified version.

  An older license, called the Affero General Public License and
published by Affero, was designed to accomplish similar goals.  This is
a different license, not a version of the Affero GPL, but Affero has
released a new version of the Affero GPL which permits relicensing under
this license.

  The precise terms and conditions for copying, distribution and
modification follow.

                       TERMS AND CONDITIONS

  0. Definitions.

  "This License" refers to version 3 of the GNU Affero General Public License.

  "Copyright" also means copyright-like laws that apply to other kinds of
works, such as semiconductor masks.

  "The Program" refers to any copyrightable work licensed under this
License.  Each licensee is addressed as "you".  "Licensees" and
"recipients" may be individuals or organizations.

  To "modify" a work means to copy from or adapt all or part of the work
in a fashion requiring copyright permission, other than the making of an
exact copy.  The resulting work is called a "modified version" of the
earlier work or a work "based on" the earlier work.

  A "covered work" means either the unmodified Program or a work based
on the Program.

  To "propagate" a work means to do anything with it that, without
permission, would make you directly or secondarily liable for
infringement under applicable copyright law, except executing it on a
computer or modifying a private copy.  Propagation includes copying,
distribution (with or without modification), making available to the
public, and in some countries other activities as well.

  To "convey" a work means any kind of propagation that enables other
parties to make or receive copies.  Mere interaction with a user through
a computer network, with no transfer of a copy, is not conveying.

  An interactive user interface displays "Appropriate Legal Notices"
to the extent that it includes a convenient and prominently visible
feature that (1) displays an appropriate copyright notice, and (2)
tells the user that there is no warranty for the work (except to the
extent that warranties are provided), that licensees may convey the
work under this License, and how to view a copy of this License.  If
the interface presents a list of user commands or options, such as a
menu, a prominent item in the list meets this criterion.

  1. Source Code.

  The "source code" for a work means the preferred form of the work
for making modifications to it.  "Object code" means any non-source
form of a work.

  A "Standard Interface" means an interface that either is an official
standard defined by a recognized standards body, or, in the case of
interfaces specified for a particular programming language, one that
is widely used among developers working in that language.

  The "System Libraries" of an executable work include anything, other
than the work as a whole, that (a) is included in the normal form of
packaging a Major Component, but which is not part of that Major
Component, and (b) serves only to enable use of the work with that
Major Component, or to implement a Standard Interface for which an
implementation is available to the public in source code form.  A
"Major Component", in this context, means a major essential component
(kernel, window system, and so on) of the specific operating system
(if any) on which the executable work runs, or a compiler used to
produce the work, or an object code interpreter used to run it.

  The "Corresponding Source" for a work in object code form means all
the source code needed to generate, install, and (for an executable
work) run the object code and to modify the work, including scripts to
control those activities.  However, it does not include the work's
System Libraries, or general-purpose tools or generally available free
programs which are used unmodified in performing those activities but
which are not part of the work.  For example, Corresponding Source
includes interface definition files associated with source files for
the work, and the source code for shared libraries and dynamically
linked subprograms that the work is specifically designed to require,
such as by intimate data communication or control flow between those
subprograms and other parts of the work.

  The Corresponding Source need not include anything that users
can regenerate automatically from other parts of the Corresponding
Source.

  The Corresponding Source for a work in source code form is that
same work.

  2. Basic Permissions.

  All rights granted under this License are granted for the term of
copyright on the Program, and are irrevocable provided the stated
conditions are met.  This License explicitly affirms your unlimited
permission to run the unmodified Program.  The output from running a
covered work is covered by this License only if the output, given its
content, constitutes a covered work.  This License acknowledges your
rights of fair use or other equivalent, as provided by copyright law.

  You may make, run and propagate covered works that you do not
convey, without conditions so long as your license otherwise remains
in force.  You may convey covered works to others for the sole purpose
of having them make modifications exclusively for you, or provide you
with facilities for running those works, provided that you comply with
the terms of this License in conveying all material for which you do
not control copyright.  Those thus making or running the covered works
for you must do so exclusively on your behalf, under your direction
and 

[... truncated at 8000 chars ...]
```

> The repo's licence rule allows MIT, Apache-2.0, GPL-3.0 (as a separate component) and OpenRAIL. Non-commercial, research-only and revenue-capped licences are refused. Read the text, not the SPDX tag alone.

## File tree (2804 files, filtered)

- `.github/ISSUE_TEMPLATE/config.yml`
- `CHANGELOG.md`
- `FAL.md`
- `LICENSE`
- `MACOS-RENDER-TUNING.md`
- `README.md`
- `README_ZH.md`
- `assets/README.md`
- `assets/agent/openchatcut-tool-schemas.json`
- `assets/fonts/LICENSES.md`
- `cli/args.ts`
- `cli/cli.verify.ts`
- `cli/clip-ref.ts`
- `cli/commands/common.ts`
- `cli/commands/edit.ts`
- `cli/commands/export.ts`
- `cli/commands/item.ts`
- `cli/commands/media.ts`
- `cli/commands/project.ts`
- `cli/commands/render.ts`
- `cli/commands/run-op.ts`
- `cli/commands/timeline.ts`
- `cli/commands/tools.ts`
- `cli/commands/where.ts`
- `cli/errors.ts`
- `cli/main.ts`
- `cli/output.ts`
- `cli/profile.ts`
- `cli/router.ts`
- `cli/session.ts`
- `cli/store.ts`
- `cli/summary.ts`
- `cli/tool-catalog.ts`
- `config/.oxlintrc.json`
- `config/README.md`
- `config/vite.config.ts`
- `config/vite.config.verify.ts`
- `desktop/agent-local-media.verify.ts`
- `desktop/agent-path-import.e2e.verify.ts`
- `desktop/agent-path-import.verify.ts`
- `desktop/bootstrap.ts`
- `desktop/chdir-first.ts`
- `desktop/context-menu.ts`
- `desktop/context-menu.verify.ts`
- `desktop/desktop.verify.ts`
- `desktop/directory-watch-controller.ts`
- `desktop/directory-watch-controller.verify.ts`
- `desktop/directory-watch-errors.ts`
- `desktop/directory-watch-errors.verify.ts`
- `desktop/directory-watch-import.verify.ts`
- `desktop/directory-watch-ipc.ts`
- `desktop/directory-watch.verify-fixtures.ts`
- `desktop/directory-watch.verify.ts`
- `desktop/editor-auth-ipc.ts`
- `desktop/embedded-port.ts`
- `desktop/embedded-port.verify.ts`
- `desktop/embedded-project-store-http.verify.ts`
- `desktop/embedded-server.ts`
- `desktop/embedded-server.verify.ts`
- `desktop/env-file.ts`
- `desktop/export-directory-state.ts`
- `desktop/export-directory-state.verify.ts`
- `desktop/export-reveal.ts`
- `desktop/export-reveal.verify.ts`
- `desktop/local-media-bridge.ts`
- `desktop/local-media-import.verify.ts`
- `desktop/main.ts`
- `desktop/mini-connect.ts`
- `desktop/native-asr-pcm.ts`
- `desktop/native-asr-pcm.verify.ts`
- `desktop/native-asr-service.ts`
- `desktop/native-asr-service.verify.ts`
- `desktop/native-asr-utils.ts`
- `desktop/native-asr-worker-lifecycle.ts`
- `desktop/native-asr-worker-lifecycle.verify.ts`
- `desktop/native-asr-worker.ts`
- `desktop/native-asr-worker.verify.ts`
- `desktop/native-clap-service.ts`
- `desktop/native-clap-worker.ts`
- `desktop/native-hardware-profile.ts`
- `desktop/native-hardware-profile.verify.ts`
- `desktop/native-inference-budget.ts`
- `desktop/native-inference-budget.verify.ts`
- `desktop/native-inference-ipc.ts`
- `desktop/native-inference-policy.ts`
- `desktop/native-inference-residency.ts`
- `desktop/native-inference-residency.verify.ts`
- `desktop/native-rhythm-service.ts`
- `desktop/native-rhythm-worker.ts`
- `desktop/native-semantic-service.ts`
- `desktop/native-semantic-worker.ts`
- `desktop/native-worker-priority.ts`
- `desktop/packaged-runtime.ts`
- `desktop/page-origin.ts`
- `desktop/page-origin.verify.ts`
- `desktop/preload.ts`
- `desktop/project-store-ipc.ts`
- `desktop/remotion-binaries.ts`
- `desktop/remotion-binaries.verify.ts`
- `desktop/runtime-preflight.ts`
- `desktop/runtime-preflight.verify.ts`
- `desktop/runtime-profile.ts`
- `desktop/runtime-profile.verify.ts`
- `desktop/single-instance.ts`
- `desktop/single-instance.verify.ts`
- `desktop/smoke-lifecycle.ts`
- `desktop/smoke-mcp-recovery.ts`
- `desktop/smoke-probe.ts`
- `desktop/smoke-renderer-recovery.ts`
- `desktop/static-files.ts`
- `desktop/ui-scale-migration.ts`
- `desktop/ui-scale-migration.verify.ts`
- `desktop/update-ipc.ts`
- `desktop/update-packaging.verify.ts`
- `desktop/update-service.ts`
- `desktop/update-service.verify.ts`
- `desktop/window-frame.ts`
- `desktop/window-frame.verify.ts`
- `desktop/window-recovery.ts`
- `desktop/window-scale.ts`
- `desktop/window-scale.verify.ts`
- `index.html`
- `remotion/Root.tsx`
- `remotion/index.ts`
- `server/agent-local-media.ts`
- `server/agent-runs/acceptance-loop.ts`
- `server/agent-runs/acceptance-loop.verify.ts`
- `server/agent-runs/browser-tool.ts`
- `server/agent-runs/claude-code-turn.ts`
- `server/agent-runs/claude-code-turn.verify.ts`
- `server/agent-runs/codex-turn.ts`
- `server/agent-runs/codex-turn.verify.ts`
- `server/agent-runs/context.ts`
- `server/agent-runs/copilot-turn.ts`
- `server/agent-runs/copilot-turn.verify.ts`
- `server/agent-runs/execution-input.ts`
- `server/agent-runs/executor-events.ts`
- `server/agent-runs/executor.ts`
- `server/agent-runs/executor.verify.ts`
- `server/agent-runs/llm-retry.ts`
- `server/agent-runs/llm-retry.verify.ts`
- `server/agent-runs/model.ts`
- `server/agent-runs/model.verify.ts`
- `server/agent-runs/request.ts`
- `server/agent-runs/routes.ts`
- `server/agent-runs/routes.verify.ts`
- `server/agent-runs/sse.ts`
- `server/agent-runs/store-approval.verify.ts`
- `server/agent-runs/store-business-failure.verify.ts`
- `server/agent-runs/store-capability.verify.ts`
- `server/agent-runs/store-event-window.verify.ts`
- `server/agent-runs/store-events.ts`
- `server/agent-runs/store-metrics.ts`
- `server/agent-runs/store-record.ts`
- `server/agent-runs/store-recovery.ts`
- `server/agent-runs/store-settle.ts`
- `server/agent-runs/store-tool-requests.verify.ts`
- `server/agent-runs/store-tools.ts`
- `server/agent-runs/store-types.ts`
- `server/agent-runs/store-values.ts`
- `server/agent-runs/store.ts`
- `server/agent-runs/store.verify.ts`
- `server/agent-runs/tool-catalog-generation.ts`
- `server/agent-runs/tool-catalog-generation.verify.ts`
- `server/agent-runs/tool-policy.ts`
- `server/agent-runs/tool-policy.verify.ts`
- `server/agent-runs/unexecuted-tool-calls.ts`
- `server/claude-code/claude-code-agent.verify.ts`
- `server/claude-code/command.ts`
- `server/claude-code/installation.ts`
- `server/claude-code/models.ts`
- `server/claude-code/resume-store.ts`
- `server/claude-code/turn-runner.ts`
- `server/codex/app-server.ts`
- `server/codex/codex-agent.verify.ts`
- `server/codex/command.ts`
- `server/codex/installation.ts`
- `server/codex/turn-manager.ts`
- `server/copilot/client.ts`
- `server/copilot/copilot-agent.verify.ts`
- `server/copilot/installation.ts`
- `server/copilot/turn-manager.ts`
- `server/data-dir.ts`
- `server/data-dir.verify.ts`
- `server/derivative-queue.ts`
- `server/directory-watch-handle.ts`
- `server/directory-watch-import.ts`
- `server/directory-watch.ts`
- `server/editor-auth.ts`
- `server/env-text.ts`
- `server/error-scrub.ts`
- `server/error-scrub.verify.ts`
- `server/export-destinations.ts`
- `server/export-media-sources.ts`
- `server/external-agent/agent-runtime-persistence.ts`
- `server/external-agent/agent-runtime-persistence.verify.ts`
- `server/external-agent/broker-poll-refresh.verify.ts`
- `server/external-agent/broker-registry.ts`
- `server/external-agent/broker-types.ts`
- `server/external-agent/broker-waiters.ts`
- `server/external-agent/broker.ts`
- `server/external-agent/broker.verify.ts`
- `server/external-agent/builtin-approval-mode.ts`
- `server/external-agent/builtin-approval-mode.verify.ts`
- `server/external-agent/capcut-command.ts`
- `server/external-agent/capcut-command.verify.ts`
- `server/external-agent/client-connect.ts`
- `server/external-agent/client-connect.verify.ts`
- `server/external-agent/edit-session-ownership.ts`
- `server/external-agent/import-token.ts`
- `server/external-agent/import-token.verify.ts`
- `server/external-agent/jianying-export-commands.verify.ts`
- `server/external-agent/jianying-export.ts`
- `server/external-agent/jianying-export.verify.ts`
- `server/external-agent/mcp-binding.ts`
- `server/external-agent/mcp-check.verify.ts`
- `server/external-agent/mcp-controls.ts`
- `server/external-agent/mcp-instructions.ts`
- `server/external-agent/mcp-prompts.ts`
- `server/external-agent/mcp-recovery-race.verify.ts`
- `server/external-agent/mcp-result.ts`
- `server/external-agent/mcp-session-status.ts`
- `server/external-agent/mcp-session-verifier.ts`
- `server/external-agent/mcp-tool-catalog.ts`
- `server/external-agent/mcp-tool-exposure.ts`
- `server/external-agent/mcp-tool-exposure.verify.ts`
- `server/external-agent/mcp.ts`
- `server/external-agent/mcp.verify.ts`
- `server/external-agent/offline-catalogs.ts`
- `server/external-agent/offline-executor.ts`
- `server/external-agent/offline-jianying-export.ts`
- `server/external-agent/offline-mcp-process-recovery.verify.ts`
- `server/external-agent/offline-mcp.verify.ts`
- `server/external-agent/offline-outcome.ts`
- `server/external-agent/offline-path-import.ts`
- `server/external-agent/offline-project-store.ts`
- `server/external-agent/offline-project-store.verify.ts`
- `server/external-agent/offline-review-outcome.ts`
- `server/external-agent/offline-run-recovery.ts`
- `server/external-agent/offline-runtime-safety.verify-scenarios.ts`
- `server/external-agent/offline-runtime-support.ts`
- `server/external-agent/offline-runtime.ts`
- `server/external-agent/offline-runtime.verify-fixtures.ts`
- `server/external-agent/offline-runtime.verify.ts`
- `server/external-agent/offline-session-view.ts`
- `server/external-agent/offline-tool-authorization.ts`
- `server/external-agent/offline-tools.ts`
- `server/external-agent/project-edit-ownership-codec.ts`
- `server/external-agent/project-edit-ownership.ts`
- `server/external-agent/projects.ts`
- `server/file-name.ts`
- `server/frame-grid.ts`
- `server/key-probe-local.ts`
- `server/key-probe-result.ts`
- `server/key-probes.ts`
- `server/key-probes.verify.ts`
- `server/keystore-default-profile.verify.ts`
- `server/keystore-names.ts`
- `server/keystore-profile.verify.ts`
- `server/keystore.ts`
- `server/keystore.verify.ts`
- `server/llm-config.ts`
- `server/llm-proxy.verify.ts`
- `server/local-media-import.ts`
- `server/local-path-import.ts`
- `server/loopback-address.ts`
- `server/mcp-token.ts`
- `server/mcp-token.verify.ts`
- `server/media-acceleration.ts`
- `server/media-acceleration.verify.ts`
- `server/media-binaries.ts`
- `server/media-binaries.verify.ts`
- `server/media-dir-profile.verify.ts`
- `server/media-dir.ts`
- `server/media-normalization-admission.ts`
- `server/media-normalization-admission.verify.ts`
- `server/media-normalization-reason.verify.ts`
- `server/media-normalization-reference.verify.ts`
- `server/media-normalization-runner.ts`
- `server/media-normalization.ts`
- `server/media-process.ts`
- `server/media-process.verify.ts`
- `server/media-references.ts`
- `server/media-references.verify.ts`
- `server/media-stream.verify.ts`
- `server/media-timecode.ts`
- `server/media-timecode.verify.ts`
- `server/mobile-upload-service.ts`
- `server/mobile-upload-service.verify.ts`
- `server/net.ts`
- `server/outbound-proxy.ts`
- `server/outbound-proxy.verify.ts`
- `server/plugins/agent-artifact-budget.verify.ts`
- `server/plugins/asr-model-downloads.verify.ts`
- `server/plugins/asr-models.ts`
- `server/plugins/asr-models.verify.ts`
- `server/plugins/assemblyai-upload.ts`
- `server/plugins/assemblyai-upload.verify.ts`
- `server/plugins/auto-grade.ts`
- `server/plugins/auto-grade.verify.ts`
- `server/plugins/claude-code-agent.ts`
- `server/plugins/codex-agent.ts`
- `server/plugins/copilot-agent.ts`
- `server/plugins/cross-origin-isolation.ts`
- `server/plugins/e2b.ts`
- `server/plugins/export-destination.ts`
- `server/plugins/export-destination.verify.ts`
- `server/plugins/export-http.ts`
- `server/plugins/export-job-routes.ts`
- `server/plugins/export-media-plan.ts`
- `server/plugins/export-media-plan.verify.ts`
- `server/plugins/export-media-sources.ts`
- `server/plugins/export-media-sources.verify.ts`
- `server/plugins/export-params.verify.ts`
- `server/plugins/export-plan.ts`
- `server/plugins/export-product-assets.verify.ts`
- `server/plugins/export-qa.ts`
- `server/plugins/export-qa.verify.ts`
- `server/plugins/export-reference-materialization.ts`
- `server/plugins/export-reference-materialization.verify.ts`
- `server/plugins/export-render-routes.ts`
- `server/plugins/export-rendering.ts`
- `server/plugins/export-runtime.ts`
- `server/plugins/export-runtime.verify.ts`
- `server/plugins/export-stage.ts`
- `server/plugins/export-stage.verify.ts`
- `server/plugins/export-submission.ts`
- `server/plugins/export-submission.verify.ts`
- `server/plugins/export.ts`
- `server/plugins/extension-store.ts`
- `server/plugins/extension-store.verify.ts`
- `server/plugins/external-agent-bridge-routes.ts`
- `server/plugins/external-agent.ts`
- `server/plugins/external-agent.verify.ts`
- `server/plugins/extract-audio.ts`
- `server/plugins/extract-audio.verify.ts`
- `server/plugins/extract-frames.ts`
- `server/plugins/extract-frames.verify.ts`
- `server/plugins/fal-catalog-input.ts`
- `server/plugins/fal-catalog-input.verify.ts`
- `server/plugins/fal-client.ts`
- `server/plugins/fal-client.verify.ts`
- `server/plugins/fal-provider.ts`
- `server/plugins/fal-provider.verify.ts`
- `server/plugins/fal-routing.verify.ts`
- `server/plugins/firecrawl-crawl.ts`
- `server/plugins/firecrawl-scrape.ts`
- `server/plugins/firecrawl-shared.ts`
- `server/plugins/firecrawl.ts`
- `server/plugins/firecrawl.verify.ts`
- `server/plugins/generation-job-lifecycle.ts`
- `server/plugins/generation-job-store.ts`
- `server/plugins/generation-job-types.ts`
- `server/plugins/generation-jobs.ts`
- `server/plugins/generation-jobs.verify-fixtures.ts`
- `server/plugins/generation-jobs.verify-groups.ts`
- `server/plugins/generation-jobs.verify.ts`
- `server/plugins/grok-video-provider.ts`
- `server/plugins/hf-download-transport.ts`
- `server/plugins/hf-integrity.ts`
- `server/plugins/hf-proxy.ts`
- `server/plugins/hf-proxy.verify.ts`
- `server/plugins/image-provider-clients.ts`
- `server/plugins/image.ts`
- `server/plugins/image.verify.ts`
- `server/plugins/import-probe.ts`
- `server/plugins/import-probe.verify.ts`
- `server/plugins/import-url-errors.ts`
- `server/plugins/import-url-errors.verify.ts`
- `server/plugins/index.ts`
- `server/plugins/isolate-voice.ts`
- `server/plugins/isolate-voice.verify.ts`
- `server/plugins/llm-proxy.ts`
- `server/plugins/local-media.ts`
- `server/plugins/local-media.verify.ts`
- `server/plugins/media-preview-transforms.ts`
- `server/plugins/media-preview.ts`
- `server/plugins/media-provider-config.ts`
- `server/plugins/minimax-video.ts`
- `server/plugins/mobile-upload.ts`
- `server/plugins/mobile-upload.verify.ts`
- `server/plugins/model-pack-install.ts`
- `server/plugins/model-pack-install.verify.ts`
- `server/plugins/model-packs.ts`
- `server/plugins/model-persistence-restart.verify.ts`
- `server/plugins/music-atlas.ts`
- `server/plugins/music-media.ts`
- `server/plugins/music-media.verify.ts`
- `server/plugins/music-minimax.ts`
- `server/plugins/music-mureka.ts`
- `server/plugins/music-sonilo.ts`
- `server/plugins/music-sonilo.verify.ts`
- `server/plugins/music-types.ts`
- `server/plugins/music-validation.ts`
- `server/plugins/music.ts`
- `server/plugins/music.verify.ts`
- `server/plugins/normalize-media-verify-helpers.ts`
- `server/plugins/normalize-media.ts`
- `server/plugins/normalize-media.verify.ts`
- `server/plugins/ofox-video-provider.ts`
