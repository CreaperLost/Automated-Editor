# moritzkremb/framejam

- **Source:** https://github.com/moritzkremb/framejam
- **Kind:** github
- **Captured:** 2026-10-25T14:58:52
- **Evidence grade:** vendor or creator claim. Not tested in this repo. Quotes from a caption are what a creator *said*, not a measurement.

## Repository

- **Repo:** moritzkremb/framejam
- **Description:** Make videos with your coding agent: pick a style, then point at the frame and say what to change. Works with Hyperframes, Remotion or any video tool, over MCP.
- **Homepage:** https://www.framejam.ai
- **Stars:** 59
- **Forks:** 5
- **Open issues:** 5
- **Licence (SPDX):** MIT
- **Language:** TypeScript
- **Default branch:** main
- **Last push:** 2026-10-25T10:25:35Z
- **Archived:** False
- **Topics:** ai-agents, claude-code, codex, cursor, gsap, hyperframes, mcp, mcp-server, remotion, video, video-review

## README

```
<div align="center">

# FrameJam

**Make videos with your coding agent, and tell it what to change by pointing at the screen.**

Pick a style, let the agent build the video, then click the frame that's wrong and say why.
<br>
Your agent gets the exact timestamp and a frame image with your pin on the spot you clicked.

[Website](https://www.framejam.ai) · [Join the Discord](https://discord.gg/7bW9tP2ySK) · [Get updates](https://www.framejam.ai/#updates) · [Report an issue](https://github.com/moritzkremb/framejam/issues)

<br>

<img src="docs/images/review.jpg" alt="The FrameJam review page: a video with a timeline, a range selected on the filmstrip and a comment in the sidebar" width="860">

</div>

<br>

## Why FrameJam

Coding agents are surprisingly good at making videos, with [Hyperframes](https://github.com/heygen-com/hyperframes),
[Remotion](https://www.remotion.dev), [Motion Canvas](https://motioncanvas.io), [ffmpeg](https://ffmpeg.org) or
whatever your project already uses. The hard part is everything around it. Describing a look in words is slow, and
"the thing at around three seconds should be bigger" is a terrible way to give feedback. FrameJam is a plugin for
Claude, ChatGPT and Cursor, with a small app that sits next to your agent, and it handles both.

**Pick a style instead of describing one.** Name one of 30 example videos when you ask for a video, or type
`/framejam:style` to choose on a page. Your agent gets the look:
palette, textures, fonts, easing, transitions, text animations and pacing, and builds whatever your video is about in
it.

**Give feedback like you would to a person.** Pause and type, click the thing that's off, or drag across the
timeline to mark a range. Press **Finish review** and your agent gets every comment with its timestamp and a frame
from the render, with your pin drawn where you clicked.

**Works with any video tool.** Anything that renders an mp4 can be reviewed, and you always review the render.

**Go round by round.** Each new version from the agent is a fresh round. Old versions keep their comments, so
you can always see what changed and why.

**Runs on your machine.** One local Node process, no account, no upload. Your projects are plain JSON files in
`~/.framejam`. Your videos and comments never leave your machine. The Styles and Playbooks pages stream their example
videos from framejam.ai, the app downloads playbook updates from framejam.ai (see below), and the **Feedback**
button sends the note you write; nothing else goes out.

## How it works

1. **Ask your agent** for a video, for example *"make a 7-second launch teaser with FrameJam and open it for review"*.
2. **Optionally pick a playbook and a style**: name them in your request, or type `/framejam:playbook` or
   `/framejam:style` to choose on a page.
3. **Review it** in the browser pane next to your chat: click, type, drag, then press **Finish review**.
4. **Get version 2.** The agent applies your notes and the page switches to the new version on its own. Repeat
   until you love it.

Not ready to animate yet? Ask for a **storyboard** first: one still per shot, which you review the same way.

<div align="center">
<img src="docs/images/styles.jpg" alt="The Styles page: a grid of example videos, each with a Use button" width="860">
</div>

## Quick start

FrameJam is a plugin for **Claude Code** (terminal and the Claude desktop app), **ChatGPT** (Work and Codex in the
ChatGPT desktop app) and **Cursor**. You need **Node 22+** and **ffmpeg** (`brew install ffmpeg` on macOS). Paste this
into your agent (shown for Cursor; use `claude` or `codex` for the other apps):

> Run `npx -y framejam install cursor` and open the link it prints in your built-in browser.

Or run `npx -y framejam install` in a terminal yourself; without an app name it sets up every app it finds. It
installs the FrameJam plugin in each app (the tools and the skill), checks Node and ffmpeg, and starts the app in the
background at `http://localhost:2400`. Then start a new session in Claude, reopen the ChatGPT app, or reload the
Cursor window, so the plugin loads. FrameJam shows up in each app's plugin list, where you can turn it off or remove
it.

**Updating.** In Claude Code the plugin updates on its own. Run `npx -y framejam@latest install` any time to update
everywhere: it updates the plugin and replaces an older FrameJam UI that is still running in the background with the
new one. The Projects page shows a notice with this command when a newer version is on npm (it asks the npm registry
at most every 6 hours; set `FRAMEJAM_NO_UPDATE_CHECK=1` to turn that off).

After that, typing `/framejam:start` in your agent is all it takes: the agent opens FrameJam and picks up where you
are (a video you're already editing, feedback waiting, or a new video). Until an agent has connected, the start page shows a
short setup checklist (it stays available at `/setup`). After that, it lists your projects and shows whose turn it is
on each one.

To run from a clone instead:

```bash
git clone https://github.com/moritzkremb/framejam.git
cd framejam
npm install
npm run build
npm start                 # http://localhost:2400  (MCP at http://localhost:2400/mcp)
node dist/server/cli.js install   # points your apps at this clone instead of npm (MCP server + skill, no plugin)
```

## Connect your agent

`npx -y framejam install` does all of this for you. The plugin lives in [`plugin/`](plugin): one folder that Claude
Code, ChatGPT and Cursor all read, listed in this repo's marketplaces (`.claude-plugin/marketplace.json` for Claude,
`.agents/plugins/marketplace.json` for ChatGPT). To install it by hand, then run `npx -y framejam start` to start the
UI:

<details open>
<summary><b>Claude Code</b></summary>

<br>

In Claude Code:

```
/plugin install framejam --marketplace moritzkremb/framejam
```

Or in a terminal:

```bash
claude plugin marketplace add moritzkremb/framejam
claude plugin install framejam@framejam
```

In the Claude desktop app you can also add the marketplace under Customize → Plugins → Add → Add marketplace → Add
from a repository. Start a new session so it loads. Claude Code has no built-in browser, so open
`http://localhost:2400` yourself.

</details>

<details>
<summary><b>ChatGPT (Work and Codex)</b></summary>

<br>

```bash
codex plugin marketplace add moritzkremb/framejam
codex plugin add framejam@framejam
```

The `codex` command comes with the ChatGPT desktop app. Reopen the app; FrameJam is listed under Plugins.

</details>

<details>
<summary><b>Cursor</b></summary>

<br>

Cursor also loads plugins you installed in Claude Code, so if you have FrameJam there, you're done. Otherwise copy the
[`plugin`](plugin) folder to `~/.cursor/plugins/local/framejam` and reload the window.

</details>

<details>
<summary><b>Any other MCP client (without the plugin)</b></summary>

<br>

Each app needs the MCP server and the skill. Desktop apps launched from the Dock may not see the `PATH` from your
shell (nvm, fnm): if the server fails to start because `npx` isn't found, use the full path from `which npx` as the
command. When running from a clone, replace `npx -y framejam` with
`node /absolute/path/to/framejam/dist/server/cli.js` and `moritzkremb/framejam` with the clone's path.

For example, in Cursor's `~/.cursor/mcp.json` (or `.cursor/mcp.json` in a project):

```json
{
  "mcpServers": {
    "framejam": {
      "command": "npx",
      "args": ["-y", "framejam", "--stdio"]
    }
  }
}
```

```bash
npx -y skills add moritzkremb/framejam -g -a cursor -y
```

Claude Code: `claude mcp add -s user framejam -- npx -y framejam --stdio`. Codex: `codex mcp add framejam -- npx -y
framejam --stdio`. Use `-a claude-code` or `-a codex` for the skill.

</details>

<details>
<summary><b>ChatGPT chat mode (or any remote MCP client)</b></summary>

<br>

ChatGPT's chat connectors use MCP over HTTP. Run `npm start`, turn on developer mode for connectors in ChatGPT's
settings, and add a custom connector with this URL:

```
http://localhost:2400/mcp
```

If your ChatGPT build requires an https URL, expose the port with a tunnel (for example
`cloudflared tunnel --url http://localhost:2400`) and set `FRAMEJAM_PUBLIC_URL` to the tunnel URL so the review links
it hands out are reachable.

</details>

<details>
<summary><b>Teach the agent the loop</b></summary>

<br>

[`skills/framejam/SKILL.md`](skills/framejam/SKILL.md) is the skill. The plugin ships it as `start` (so the command
reads `/framejam:start`), next to `playbook` and `style`. Without the plugin, `npx skills add moritzkremb/framejam`
installs it for most agents (there it's `/framejam`); you can also drop it into `AGENTS.md`. It walks the agent through: ask what to make → pick a preset →
build with the project's tool → render → `open_review` → `wait_for_feedback` (and call it again while the result is
`pending`, up to 12 times in a row, about 10 minutes) → edit → `add_version` with a note, until you say it's done. `.cursor/rules/framejam.mdc` is the same text
as a Cursor rule; regenerate it with `npm run sync:rule` after editing the skill.

</details>

### Try the whole loop without an agent

```bash
npm start                 # terminal 1
npm run e2e               # terminal 2 — plays the agent: picks a preset, opens a review, blocks on wait_for_feedback
```

Open the printed review URL, add a few comments and press **Finish review**. The script prints what the agent
received, then posts v2, and you can watch the page move to it live. `npx tsx scripts/ui-demo.ts` does the browser
side automatically (headless Chrome) and saves screenshots.

## Styles

<table>
<tr>
<td><img src="presets/acid-chrome/poster.jpg" alt="Acid Chrome" width="200"><br><sub><b>Acid Chrome</b></sub></td>
<td><img src="presets/cyanotype-mac/poster.jpg" alt="Cyanotype Mac" width="200"><br><sub><b>Cyanotype Mac</b></sub></td>
<td><img src="presets/doodle-mascot/poster.jpg" alt="Doodle Mascot" width="200"><br><sub><b>Doodle Mascot</b></sub></td>
<td><img src="presets/case-file/poster.jpg" alt="Case File" width="200"><br><sub><b>Case File</b></sub></td>
</tr>
<tr>
<td><img src="presets/dot-matrix/poster.jpg" alt="Dot Matrix" width="200"><br><sub><b>Dot Matrix</b></sub></td>
<td><img src="presets/ink-wash/poster.jpg" alt="Ink Wash" width="200"><br><sub><b>Ink Wash</b></sub></td>
<td><img src="presets/synthwave-grid/poster.jpg" alt="Synthwave Grid" width="200"><br><sub><b>Synthwave Grid</b></sub></td>
<td><img src="presets/parchment-epic/poster.jpg" alt="Parchment Epic" width="200"><br><sub><b>Parchment Epic</b></sub></td>
</tr>
</table>

FrameJam ships with 30 styles:

**Neon Terminal**, **Noir Quote**, **Navy Teal**, **Mono Plex**, **Paper Marker**, **Bauhaus Grid** (1:1),
**Dot Matrix**, **Red Band Title**, **Midnight Glow**, **Cream Serif**, **Airy UI**,
**Dot Field**, **Blueprint**, **Topo Credits**, **Parchment Epic**, **Case File**, **Ink Wash**,
**Acid Chrome**, **Soft Outline**, **Split Flap**, **Label Collage**, **Cyanotype Mac**, **Doodle Mascot**,
**Synthwave Grid**, **Pop Zine**, **UI Microstudy**, **Motion Principles**, **Dither Serif**, **Color Wipe** and
**Storybook Lantern**. Want a style that isn't here? Ask for it in the
[FrameJam Discord](https://discord.gg/7bW9tP2ySK).

Many were studied from the Skillry Opus 5.5 gallery, What Ships launch films and Opus 5.5 videos shared on X, with
original copy and no brand assets. Every one passes `hyperframes lint` with no errors.

<details>
<summary><b>How presets are stored, and making your own</b></summary>

<br>

Presets live in `presets/<id>/` (built-in) and `~/.framejam/presets/<id>/` (yours):

```
presets/neon-terminal/
  style.json          # the look: palette, fonts, easing, transitions, textAnimations, rhythm, guide, plus example (what the preview shows)
  composition/        # the example video as a Hyperframes project; agents look here for techniques, not a starting point
  preview.mp4         # rendered with `npx hyperframes render`
  poster.jpg
```

The npm package leaves out the built-in `preview.mp4` files to stay small; the Styles page streams them from
`https://www.framejam.ai/presets/<id>/preview.mp4` (override with `FRAMEJAM_PREVIEW_URL`). To add a style pack, unzip
it into `~/.framejam/presets/`.

To re-render the previews:

```bash
npm run render:presets            # or: npm run render:presets -- noir-quote
```

</details>

## Playbooks

A style sets the look; a playbook sets how a kind of video gets made: what you bring, the steps, the tools it needs
and the quality bar, all as defaults your request can override. Name one when you ask for a video, or type
`/framejam:playbook` to choose on a page. Some playbooks come with styles made for them, each with its own example
video. FrameJam ships with 16:

| Playbook | What goes in, what comes out |
| --- | --- |
| **Kinetic Music Video** | A song goes in, a music video where the lyrics, graphics and illustrated characters move on every beat comes out. |
| **Slopcore Music Video** | A song full of a scene's in-jokes goes in, a music video with one star, every lyric on screen and a graphic explaining every line comes out. Made the way anabology made ESCAPE VELOCITY with Claudia (claudia.gallery). |
| **Slopcore Cartoon** | A song about a scene's memes goes in, a cartoon music video that explains every joke comes out, from still illustrations only, for a few dollars. |
| **Split Screen Short** | Raw talking-head clips go in, a tight vertical short that shows what you talk about while you say it comes out. Comes with two styles made for it: Midnight Glow and Paper Marker. |
| **Dynamic Layout Short** | A raw talking-head recording goes in, a vertical short that switches layout with what you say comes out. Comes with three styles made for it: Pop Zine, Midnight Glow and Mono Plex. |
| **Anchor and Overlay Reel** | A raw talking-head recording goes in, a vertical reel where you stay on screen and a picture pops up for every thing you name comes out. |
| **Editorial Reel** | A raw talking-head recording goes in, a vertical reel with magazine-style serif titles, chat pop-ups and word-by-word captions comes out. |
| **Whiteboard Reel** | A raw talking-head recording goes in, a vertical reel comes out where every idea you explain is drawn on a white board above you. |
| **Clean Cut** | A raw recording goes in, the same video as a tight, fast-paced jump cut without pauses, fillers and retakes comes out. |
| **Product Launch** | Your product goes in, a polished launch video with the real app on screen comes out. |
| **Founder Launch** | A founder talking to camera and screen recordings go in, a tight launch video comes out. |
| **Launch Teaser** | A name, one promise and a few real clips go in, a 15-second teaser that lands the name on the beat comes out. |
| **Explainer** | A topic, article or script goes in, a narrated animated explainer comes out. |
| **Math Animation** | A math or science idea goes in, a calm animated explainer in the style of 3Blue1Brown comes out. Comes with two styles made for it: Blueprint and Soft Outline. |
| **How-to** | A process goes in, a short step-by-step video that's clear even with the sound off comes out. |
| **Timeline** | Dates and milestones go in, a fast story told one milestone at a time comes out. Comes with a style made for it: Parchment Epic. |
| **YouTube Hook** | Your intro script or recording goes in, the 20–60 second opening that keeps people watching your long video comes out. |
| **Motion Design Showreel** | Your logo and a few things to show off go in, a short motion piece cut to the beat comes out. Comes with two styles made for it: Bauhaus Grid and Pop Zine. |

Each playbook page says what you need (what's included, what runs free on your computer, and which accounts cost
money), whether it starts a new project or works in the one you have, and how your style combines with it.

<details>
<summary><b>How playbooks are stored, and making your own</b></summary>

<br>

Playbooks live in `playbooks/<id>/` (built-in) and `~/.framejam/playbooks/<id>/` (yours). A playbook is an agent
skill plus a `playbook.json` for the gallery:

```
playbooks/clean-cut/
  playbook.json   # name, tagline, creator, bring, get, needs, project, styles, steps
  SKILL.md        # the method the agent follows
  references/     # read on demand
  scripts/        # helpers the agent copies into the project
  poster.jpg
```

Like styles, the npm package leaves out `preview.mp4`; the Playbooks page streams it from
`https://www.framejam.ai/playbooks/<id>/preview.mp4`. The full format is on
[framejam.ai/docs/playbooks](https://www.framejam.ai/docs/playbooks).

**Playbook and style updates.** Playbooks and styles also come from a signed catalog at
`https://www.framejam.ai/catalog/v1/`, so a new or fixed playbook (scripts included) or style reaches you without a new
FrameJam release. The app checks at start and every 6 hours, verifies the signature and every file against keys built
into the app, and keeps the result in `~/.framejam/catalog/`. Offline, or if anything fails to verify, it uses the last
good copy and then the ones that came with the app. Your own copy in `~/.framejam/playbooks/<id>` or
`~/.framejam/presets/<id>` always wins; for playbooks, when a newer version exists, the Playbooks page says so and
nothing in your folder changes. Some playbooks and styles are catalog-only: they aren't in the npm package and show up
once the app has been online. Set `FRAMEJAM_CATALOG_URL=off` or `FRAMEJAM_NO_UPDATE_CHECK=1` to
turn it off. How it's built and published: [docs/catalog.md](docs/catalog.md).

</details>

## The review page

<details>
<summary><b>Rounds, comments and what the agent sees</b></summary>

<br>

- **One round per version.** You add comments to the latest version, then press **Finish review**. That locks the
  version and hands its comments to the agent. The agent's next version starts with an empty list, and older versions
  show their comments greyed out.
- **Changed your mind?** After finishing, **Edit comments** reopens the round. If the agent hadn't picked the comments
  up yet they're simply withdrawn; if it had, finishing again sends a revised list (`revised: true`) that replaces the
  old one.
- **Layout.** The video, timeline and comment box sit on the left; the comments are a list in a sidebar on the right
  that collapses to a rail. Clicking a comment anywhere (pin, timeline marker, corner note) opens the sidebar on it.
  Below 720px wide the sidebar slides over the video instead of taking a column.
- **The comment box** sits under the timeline. It's attached to the current time by default; click the video to attach
  a spot (the agent gets that frame with your pin drawn on it), drag across the filmstrip to attach a range, or choose *Whole video*. Enter adds the comment.
- **On the video**, a comment shows up only while the playhead is at it: pinned comments as a bubble on their spot,
  time and range comments as a note in the top-left corner. The cursor over the video is a comment bubble.
- **Agent listening.** The page says "Agent listening" while the agent is inside `wait_for_feedback` (and for 15
  seconds between its calls). That only happens while the agent's turn is still running: once it ends its turn, nobody
  is listening, and after you finish the page shows one line to paste into the chat ("Apply my FrameJam feedback for
  rev_…"). It switches to "Your agent is making version N" once the agent picks the comments up.
- **What plays.** Always the render. Each version keeps a snapshot of its mp4, so older versions look as they did then.
- **Timeline zoom.** Pinch on the timeline, `⌘`+scroll (`Ctrl`+scroll), or use the zoom buttons to pick ranges on long
  videos. Scroll sideways or drag the bar under the filmstrip to move along.
- **Transcript.** For videos with speech, the words run along under the timeline and as text in the sidebar's
  *Transcript* tab. Click a word to jump there; select words to comment on them, and the agent gets the quote with the
  comment. The agent can attach a transcript (`transcriptPath`); otherwise FrameJam transcribes each version on your
  computer with [whisper.cpp](https://github.com/ggml-org/whisper.cpp) if it's installed (`brew install whisper-cpp`).
  It uses a whisper.cpp model you already have, or downloads one (574 MB) from Hugging Face the first time you press
  **Transcribe**. Your video is never uploaded.
- **Playback speed and full screen.** The speed button next to the time cycles 1×, 1.5×, 2× and 0.5×. Full screen
  shows the video with the comments sidebar, and moves the comment box into it.
- **Download.** The download button next to **Finish review** (or *Download video* in the version menu) saves a copy of
  the version's video wherever you choose.
- **Keys.** `Space` play/pause, `←/→` step one frame, `Shift+←/→` step one second, `C` focus the comment box,
  `S` playback speed, `F` full screen, `⌘B` show or hide the comments, `⌘↩` (`Ctrl+Enter`) finish review, `Esc` clear, `?` shortcuts.
- **Copy comments as text** (version menu) copies a version's comments as markdown for harnesses without MCP.

</details>

<details>
<summary><b>Storyboards</b></summary>

<br>

Before anything is animated, an agent can open a storyboard: a set of still panels, one per shot, each with an
optional title and caption (action, camera move, voiceover line, duration).

```ts
open_review({
  title: "Launch storyboard",
  panelsDir: "/abs/project/storyboard",          // every image in the folder, sorted by file name
  panels: [{ path: "01.png", title: "Cold open", caption: "Slow push in. VO: 'Every team ships faster.'" }, "02.png"],
})
```

The review page shows the panels as a **board** (a grid with titles and captions) and a **panel** view (one panel
large, with a strip of all panels underneath). Click a panel on the board to open it, click inside it to pin a spot,
or just type to comment on the whole panel. On the board, the comment box is for the whole storyboard. Keys: `←/→`
previous/next panel, `B` board, `C` comment, `⌘↩` finish review.

Comments reach the agent by panel: `at: "panel 3 (Cold open)"`, with the panel's number, title, caption and image path,
the pin position, and an image of the panel with the pin drawn on it. Rounds, Finish review, Edit comments and versions
work exactly as for videos. Each version keeps its own copy of the panel images, so the agent can overwrite the files
for v2 and v1 still shows what it was.

</details>

<details>
<summary><b>MCP tools</b></summary>

<br>

| Tool | What it does |
| --- | --- |
| `open_review({ title?, videoPath?, panels?, panelsDir?, transcriptPath?, reviewId?, note? })` | Opens a review and returns `{ reviewId, url, version }`. `videoPath` is a render from any tool. Pass `panels`/`panelsDir` instead of a video for a storyboard. `transcriptPath` (an `.srt`, `.vtt` or word-timings JSON such as Whisper's) shows what's said under the timeline and in the sidebar. Calling it again with the same title adds v2, v3, and so on. |
| `wait_for_feedback({ reviewId, timeoutSeconds? })` | Blocks until the user presses **Finish review**. Returns `{ status: "pending" }` after about 50s; call it again, up to 12 times in a row, then stop and tell the user to say "apply my FrameJam feedback". Sends progress notifications while waiting. |
| `get_feedback({ reviewId?, include? })` | Returns the newest round of comments right away. Without `reviewId` it picks the review whose comments haven't reached the agent yet; unsent comments are sent (and their version locked). Use it when the user says "apply my FrameJam feedback". |
| `list_reviews()` | Lists reviews with their URL and where each round stands (`awaiting_user`, `user_commenting`, `sent_not_delivered`, `delivered_to_agent`). |
| `add_version({ reviewId, videoPath?, panels?, panelsDir?, transcriptPath?, note? })` | Attaches a new render (or new storyboard panels) as the next round. It starts with no comments; the `note` is shown to the user. With no media, a storyboard re-reads its `panelsDir`. |
| `resolve_comments({ reviewId, ids, note? })` | Optional bookkeeping for the agent. The UI shows each version as one round instead. |
| `list_presets({ mood?, pacing?, format?, query? })` | Lists the style presets. |
| `get_preset({ id, includeTemplate? })` | Returns the look (`style.json` and its guide of look rules), what the example video shows, and the example's file list. `includeTemplate: true` adds the example's source. |
| `list_playbooks({ query? })` | Lists the playbooks with what to bring, what you get, the cost and the creator. |
| `get_playbook({ id, styleId? })` | Returns `playbook.json`, the full `SKILL.md` method, the playbook's local folder and file list, how a style combines with it, and the styles made for it (with their ready settings when `styleId` is one of them). |
| `open_picker({ kinds?, title? })` | Opens a page where the user picks a playbook and/or style for this video, or none. Returns `{ pickId, url }`. |
| `wait_for_pick({ pickId })` | Waits for the user's pick and returns the playbook and style (or `null` for none). Returns `pending` after about 50s; call it again. |
| `get_selected_preset()`, `get_selected_playbook()` | Kept for older skills; they point the agent to `open_picker`. |

Feedback comes back three ways: as JSON (`comments[]` with `at`, `time`, `endTime`, `position`, `thumbnailPath`), as a
markdown prompt, and as inline JPEG frames with the pin drawn on them.

</details>

## Development

```bash
npm run dev        # API on :2400 (tsx watch) + Vite UI on :2401 with a proxy
npm test           # vitest: MCP tools (in-memory, streamable HTTP, stdio) + REST API
npm run typecheck
npm run lint
```

<details>
<summary><b>Project layout and configuration</b></summary>

<br>

| Path | Contents |
| --- | --- |
| `src/server/cli.ts` | CLI entry. `framejam install [cursor\|claude\|codex]` installs the FrameJam plugin in each app (`src/server/install.ts`; from a clone it adds the MCP server and skill instead) and starts the UI; `framejam start` starts the UI in the background (log in `~/.framejam/server.log`) and prints the URL, `--browser` also opens it; `framejam` starts HTTP in the foreground; `framejam --stdio` runs MCP on stdio and also hosts the UI when the port is free. |
| `src/core/` | Code that runs anywhere (no Node or filesystem): shared types, playbook display helpers, feedback formatting, the MCP tool definitions (`mcp.ts`, given a store and base URL) and the `Store` interface (`store.ts`: everything FrameJam keeps, such as projects, comments, picks, state, styles and playbooks). |
| `src/server/mcp.ts` | The core MCP tools with this computer's skill version. |
| `src/server/json-store.ts` | `JsonStore`, the JSON file implementation of `Store`. Every call re-reads from disk, so several processes can share `~/.framejam`. |
| `src/server/app.ts` | Hono routes: REST, SSE change feed, media with HTTP range support, `/mcp`. |
| `src/web/` | React front end styled by the design system (`src/web/styles/`, generated from `design/`). |
| `design/` | The FrameJam design system: brand book, tokens, component CSS and screen mockups. Open `design/index.html`. |
| `skills/framejam/` | The agent skill (also mirrored as the Cursor rule in `.cursor/rules/framejam.mdc`). |
| `plugin/` | The plugin for Claude Code, ChatGPT and Cursor: both manifests, `mcp.json` and the skills `start`, `playbook` and `style`. `npm run sync:plugin` regenerates the manifests and `start` from `package.json` and the skill. |

Configuration: `--port` / `FRAMEJAM_PORT` (default 2400), `--host` / `FRAMEJAM_HOST` (default 127.0.0.1),
`--data-dir` / `FRAMEJAM_HOME` (default `~/.framejam`), `FRAMEJAM_PUBLIC_URL` (the base URL used in links), and
`FRAMEJAM_PREVIEW_URL` (where the Styles page streams built-in previews from), `FRAMEJAM_PLAYBOOK_PREVIEW_URL` (the
same for the Playbooks page), `FRAMEJAM_CATALOG_URL` (where playbook and style updates come from; `off` turns them off, see
[docs/catalog.md](docs/catalog.md)), `FRAMEJAM_WHISPER` and `FRAMEJAM_WHISPER_MODEL` (the whisper.cpp program and
model for transcripts, when they aren't found on their own), and `FRAMEJAM_NO_UPDATE_CHECK=1` (don't look for a newer
version on npm or for playbook updates).

</details>

<details>
<summary><b>Known limitations (v1)</b></summary>

<br>

- **One UI host per port.** If several harnesses each start `framejam --stdio`, the first one hosts the web UI and the
  others reuse it through the shared data directory. If the hosting process exits, restart one of them (or run
  `npx -y framejam start`).
- **No MCP Apps embedding yet.** The UI is a plain web page, ready to be embedded later.

</details>

## Join the community

<table>
<tr>
<td>

### Join the FrameJam Discord

FrameJam is free and open source, and so is the community. Join the
**[FrameJam Discord](https://discord.gg/7bW9tP2ySK)**:

- **Get help.** Ask questions and get unstuck on your videos.
- **Share what you make.** Show your videos and see what others are making.
- **Shape what's next.** Ask for styles and features, and hear about new ones first.

**[Join the Discord →](https://discord.gg/7bW9tP2ySK)**

Just want the news? [Sign up for updates](https://www.framejam.ai/#updates) for new styles and releases.

</td>
</tr>
</table>

<br>

<div align="center">

Created by [@moritzkremb](https://x.com/moritzkremb).
Questions, ideas or bugs? Use the **Feedback** button in the app or [open an issue](https://github.com/moritzkremb/framejam/issues).

</div>
```

## Licence file

```
MIT License

Copyright (c) 2026 Moritz Kremb

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

> The repo's licence rule allows MIT, Apache-2.0, GPL-3.0 (as a separate component) and OpenRAIL. Non-commercial, research-only and revenue-capped licences are refused. Read the text, not the SPDX tag alone.

## File tree (637 files, filtered)

- `.cursor/rules/framejam.mdc`
- `AGENTS.md`
- `CLAUDE.md`
- `LICENSE`
- `README.md`
- `design/README.md`
- `design/components/AgentStatus/README.md`
- `design/components/AgentStatus/preview.html`
- `design/components/Badge/README.md`
- `design/components/Badge/preview.html`
- `design/components/Button/README.md`
- `design/components/Button/preview.html`
- `design/components/CodeBlock/README.md`
- `design/components/CodeBlock/preview.html`
- `design/components/Comment/README.md`
- `design/components/Comment/preview.html`
- `design/components/Cover/preview.html`
- `design/components/EmptyState/README.md`
- `design/components/EmptyState/preview.html`
- `design/components/FeedbackDock/README.md`
- `design/components/FeedbackDock/preview.html`
- `design/components/FeedbackPanel/README.md`
- `design/components/FeedbackPanel/preview.html`
- `design/components/FilterBar/README.md`
- `design/components/FilterBar/preview.html`
- `design/components/HandoffCard/README.md`
- `design/components/HandoffCard/preview.html`
- `design/components/Header/README.md`
- `design/components/Header/preview.html`
- `design/components/Menu/README.md`
- `design/components/Menu/preview.html`
- `design/components/Pin/README.md`
- `design/components/Pin/preview.html`
- `design/components/Player/README.md`
- `design/components/Player/preview.html`
- `design/components/ReviewAfterSend/preview.html`
- `design/components/ReviewCard/README.md`
- `design/components/ReviewCard/preview.html`
- `design/components/ReviewFirstLook/preview.html`
- `design/components/ReviewHorizontal/preview.html`
- `design/components/ReviewOldVersion/preview.html`
- `design/components/ReviewVertical/preview.html`
- `design/components/ReviewWide/preview.html`
- `design/components/ReviewWideCollapsed/preview.html`
- `design/components/ReviewsScreen/preview.html`
- `design/components/Segmented/README.md`
- `design/components/Segmented/preview.html`
- `design/components/SetupSteps/README.md`
- `design/components/SetupSteps/preview.html`
- `design/components/StatesScreen/preview.html`
- `design/components/StyleCard/README.md`
- `design/components/StyleCard/preview.html`
- `design/components/StyleDetailScreen/preview.html`
- `design/components/StylesScreen/preview.html`
- `design/components/Swatches/README.md`
- `design/components/Swatches/preview.html`
- `design/components/Tabs/README.md`
- `design/components/Tabs/preview.html`
- `design/components/Timeline/README.md`
- `design/components/Timeline/preview.html`
- `design/components/Toast/README.md`
- `design/components/Toast/preview.html`
- `design/components/WelcomeScreen/preview.html`
- `design/index.html`
- `docs/catalog.md`
- `index.html`
- `playbooks/anchor-overlay/SKILL.md`
- `playbooks/anchor-overlay/references/clean-cuts.md`
- `playbooks/anchor-overlay/references/format.md`
- `playbooks/anchor-overlay/references/reference-breakdown.md`
- `playbooks/anchor-overlay/references/transcription.md`
- `playbooks/anchor-overlay/scripts/anchorlib.py`
- `playbooks/anchor-overlay/scripts/check_plan.py`
- `playbooks/anchor-overlay/scripts/check_render.py`
- `playbooks/anchor-overlay/scripts/make_captions.py`
- `playbooks/anchor-overlay/scripts/make_music.py`
- `playbooks/anchor-overlay/scripts/prepare_edit.py`
- `playbooks/anchor-overlay/scripts/render.py`
- `playbooks/anchor-overlay/scripts/textlayer.py`
- `playbooks/cartoon-music-video/SKILL.md`
- `playbooks/cartoon-music-video/assets/scene.template.js`
- `playbooks/cartoon-music-video/assets/smoke.template.js`
- `playbooks/cartoon-music-video/assets/timeline.template.js`
- `playbooks/cartoon-music-video/engine/core.js`
- `playbooks/cartoon-music-video/engine/index.html`
- `playbooks/cartoon-music-video/engine/lib.js`
- `playbooks/cartoon-music-video/engine/main.js`
- `playbooks/cartoon-music-video/engine/plate.js`
- `playbooks/cartoon-music-video/engine/roto.js`
- `playbooks/cartoon-music-video/engine/scenes/hud.js`
- `playbooks/cartoon-music-video/engine/typekit.js`
- `playbooks/cartoon-music-video/references/agent_brief_template.md`
- `playbooks/cartoon-music-video/references/engine_api.md`
- `playbooks/cartoon-music-video/references/examples/which-frame/STYLE.md`
- `playbooks/cartoon-music-video/references/examples/which-frame/TREATMENT.md`
- `playbooks/cartoon-music-video/references/examples/which-frame/scene_verse1.js`
- `playbooks/cartoon-music-video/references/examples/which-frame/timeline.js`
- `playbooks/cartoon-music-video/references/kit_api.md`
- `playbooks/cartoon-music-video/references/lessons.md`
- `playbooks/cartoon-music-video/references/motion_library.md`
- `playbooks/cartoon-music-video/references/pipeline.md`
- `playbooks/cartoon-music-video/references/providers.md`
- `playbooks/cartoon-music-video/scripts/audio_analysis.py`
- `playbooks/cartoon-music-video/scripts/av_sync_check.py`
- `playbooks/cartoon-music-video/scripts/beatwarp.py`
- `playbooks/cartoon-music-video/scripts/dance_sync_check.py`
- `playbooks/cartoon-music-video/scripts/fixmatte.py`
- `playbooks/cartoon-music-video/scripts/lipsync_check.py`
- `playbooks/cartoon-music-video/scripts/lyrics_align.py`
- `playbooks/cartoon-music-video/scripts/lyrics_fix.py`
- `playbooks/cartoon-music-video/scripts/matte_fallback.py`
- `playbooks/cartoon-music-video/scripts/matte_keyed.py`
- `playbooks/cartoon-music-video/scripts/plot_lyrics.py`
- `playbooks/cartoon-music-video/scripts/prep_stills.py`
- `playbooks/cartoon-music-video/scripts/prep_video.py`
- `playbooks/cartoon-music-video/scripts/providers/elevenlabs_song.py`
- `playbooks/cartoon-music-video/scripts/providers/kie_seedance.py`
- `playbooks/cartoon-music-video/scripts/sheet.py`
- `playbooks/cartoon-music-video/scripts/synth_beat.py`
- `playbooks/cartoon-music-video/scripts/track.py`
- `playbooks/clean-cut/SKILL.md`
- `playbooks/clean-cut/references/clean-cuts.md`
- `playbooks/clean-cut/references/transcription.md`
- `playbooks/clean-cut/scripts/export_cutlist.py`
- `playbooks/clean-cut/scripts/find_retakes.py`
- `playbooks/clean-cut/scripts/prepare_edit.py`
- `playbooks/editorial-reel/SKILL.md`
- `playbooks/editorial-reel/references/clean-cuts.md`
- `playbooks/editorial-reel/references/graphics.md`
- `playbooks/editorial-reel/references/reference-breakdown.md`
- `playbooks/editorial-reel/references/transcription.md`
- `playbooks/editorial-reel/scripts/check_pacing.py`
- `playbooks/editorial-reel/scripts/make_captions.py`
- `playbooks/editorial-reel/scripts/make_cutouts.py`
- `playbooks/editorial-reel/scripts/make_sounds.py`
- `playbooks/editorial-reel/scripts/make_starter.py`
- `playbooks/editorial-reel/scripts/prepare_edit.py`
- `playbooks/editorial-reel/template/build.py`
- `playbooks/explainer/SKILL.md`
- `playbooks/explainer/references/hyperframes.md`
- `playbooks/explainer/references/other-tools.md`
- `playbooks/explainer/references/sound.md`
- `playbooks/explainer/references/story.md`
- `playbooks/explainer/references/visuals.md`
- `playbooks/explainer/scripts/build_timing.py`
- `playbooks/explainer/scripts/fetch_fonts.py`
- `playbooks/explainer/scripts/mix.py`
- `playbooks/explainer/scripts/sfx.py`
- `playbooks/explainer/scripts/tts.py`
- `playbooks/explainer/scripts/word_times.py`
- `playbooks/explainer/template/build_html.py`
- `playbooks/explainer/template/index.template.html`
- `playbooks/explainer/template/panels.py`
- `playbooks/founder-launch/SKILL.md`
- `playbooks/founder-launch/references/edit.md`
- `playbooks/founder-launch/references/recording.md`
- `playbooks/founder-launch/references/structure.md`
- `playbooks/founder-launch/scripts/launch_bed.py`
- `playbooks/founder-launch/scripts/pauses.py`
- `playbooks/how-to/SKILL.md`
- `playbooks/launch-teaser/SKILL.md`
- `playbooks/launch-teaser/references/example-framejam.md`
- `playbooks/launch-teaser/references/teaser.md`
- `playbooks/launch-teaser/scripts/launch_bed.py`
- `playbooks/launch-teaser/template/hyperframes/index.html`
- `playbooks/layout-switch-short/SKILL.md`
- `playbooks/layout-switch-short/references/clean-cuts.md`
- `playbooks/layout-switch-short/references/layouts.md`
- `playbooks/layout-switch-short/references/reference-breakdown.md`
- `playbooks/layout-switch-short/references/transcription.md`
- `playbooks/layout-switch-short/scripts/check_pacing.py`
- `playbooks/layout-switch-short/scripts/make_captions.py`
- `playbooks/layout-switch-short/scripts/make_cutouts.py`
- `playbooks/layout-switch-short/scripts/prepare_edit.py`
- `playbooks/layout-switch-short/template/build.py`
- `playbooks/math-animation/SKILL.md`
- `playbooks/math-animation/references/manim-cookbook.md`
- `playbooks/math-animation/references/other-tools.md`
- `playbooks/math-animation/references/setup.md`
- `playbooks/math-animation/references/visual-language.md`
- `playbooks/math-animation/template/helpers.py`
- `playbooks/math-animation/template/narration.py`
- `playbooks/math-animation/template/palette.py`
- `playbooks/math-animation/template/scenes.py`
- `playbooks/math-animation/template/scripts/panels.py`
- `playbooks/math-animation/template/scripts/render_all.py`
- `playbooks/math-animation/template/scripts/tts.py`
- `playbooks/math-animation/template/storyboard.py`
- `playbooks/motion-showreel/SKILL.md`
- `playbooks/motion-showreel/references/beat-grid.md`
- `playbooks/motion-showreel/references/reference-breakdown.md`
- `playbooks/motion-showreel/references/tricks.md`
- `playbooks/motion-showreel/scripts/beat_grid.py`
- `playbooks/motion-showreel/scripts/sync_check.py`
- `playbooks/motion-showreel/template/index.html`
- `playbooks/product-launch/SKILL.md`
- `playbooks/product-launch/references/assets.md`
- `playbooks/product-launch/references/example-notion.md`
- `playbooks/product-launch/references/hyperframes.md`
- `playbooks/product-launch/references/music.md`
- `playbooks/product-launch/references/shots.md`
- `playbooks/product-launch/references/story.md`
- `playbooks/product-launch/scripts/launch_bed.py`
- `playbooks/product-launch/template/hyperframes/index.html`
- `playbooks/slopcore-cartoon/SKILL.md`
- `playbooks/slopcore-cartoon/references/example.md`
- `playbooks/slopcore-cartoon/scripts/gen_stills.py`
- `playbooks/slopcore-music-video/SKILL.md`
- `playbooks/slopcore-music-video/references/board.md`
- `playbooks/slopcore-music-video/references/generation.md`
- `playbooks/slopcore-music-video/references/method.md`
- `playbooks/slopcore-music-video/references/overlay_system.md`
- `playbooks/slopcore-music-video/references/pipeline.md`
- `playbooks/slopcore-music-video/references/song.md`
- `playbooks/slopcore-music-video/references/style_anatomy.md`
- `playbooks/slopcore-music-video/scripts/audio_analysis.py`
- `playbooks/slopcore-music-video/scripts/base_edit.py`
- `playbooks/slopcore-music-video/scripts/lyrics_align.py`
- `playbooks/slopcore-music-video/scripts/lyrics_fix.py`
- `playbooks/slopcore-music-video/scripts/plot_lyrics.py`
- `playbooks/slopcore-music-video/scripts/sheet.py`
- `playbooks/slopcore-music-video/scripts/shot_plan.py`
- `playbooks/talking-head-short/SKILL.md`
- `playbooks/talking-head-short/references/clean-cuts.md`
- `playbooks/talking-head-short/references/layout.md`
- `playbooks/talking-head-short/references/transcription.md`
- `playbooks/talking-head-short/scripts/make_captions.py`
- `playbooks/talking-head-short/scripts/prepare_edit.py`
- `playbooks/timeline/SKILL.md`
- `playbooks/whiteboard-reel/SKILL.md`
- `playbooks/whiteboard-reel/references/clean-cuts.md`
- `playbooks/whiteboard-reel/references/graphics.md`
- `playbooks/whiteboard-reel/references/reference-breakdown.md`
- `playbooks/whiteboard-reel/references/transcription.md`
- `playbooks/whiteboard-reel/scripts/check_pacing.py`
- `playbooks/whiteboard-reel/scripts/make_captions.py`
- `playbooks/whiteboard-reel/scripts/make_sounds.py`
- `playbooks/whiteboard-reel/scripts/make_starter.py`
- `playbooks/whiteboard-reel/scripts/prepare_edit.py`
- `playbooks/whiteboard-reel/template/build.py`
- `playbooks/youtube-intro/SKILL.md`
- `playbooks/youtube-intro/references/formula.md`
- `playbooks/youtube-intro/references/graphics.md`
- `playbooks/youtube-intro/scripts/framed.py`
- `playbooks/youtube-intro/scripts/tighten.py`
- `playbooks/youtube-intro/scripts/transcribe.py`
- `plugin/README.md`
- `plugin/skills/playbook/SKILL.md`
- `plugin/skills/start/SKILL.md`
- `plugin/skills/style/SKILL.md`
- `presets/acid-chrome/composition/index.html`
- `presets/agent-ui-demo/composition/index.html`
- `presets/bauhaus-grid/composition/index.html`
- `presets/blueprint-explainer/composition/index.html`
- `presets/case-file/composition/index.html`
- `presets/cream-serif-launch/composition/index.html`
- `presets/cyanotype-mac/composition/index.html`
- `presets/data-story/composition/index.html`
- `presets/dither-serif/composition/index.html`
- `presets/doodle-mascot/composition/index.html`
- `presets/dot-field-showreel/composition/index.html`
- `presets/dot-matrix/composition/index.html`
- `presets/ink-wash/composition/index.html`
- `presets/label-collage/composition/index.html`
- `presets/life-timeline/composition/index.html`
- `presets/midnight-launch/composition/index.html`
- `presets/mono-changelog/composition/index.html`
- `presets/motion-principles/composition/index.html`
- `presets/neon-terminal/composition/index.html`
- `presets/noir-quote/composition/index.html`
- `presets/paper-marker/composition/index.html`
- `presets/parchment-epic/composition/index.html`
- `presets/pop-zine/composition/index.html`
- `presets/recipe-steps/composition/index.html`
- `presets/red-band-title/composition/index.html`
- `presets/split-flap/composition/index.html`
- `presets/storybook-lantern/composition/index.html`
- `presets/synthwave-grid/composition/index.html`
- `presets/topo-credits/composition/index.html`
- `presets/ui-microstudy/composition/index.html`
- `scripts/catalog-keygen.ts`
- `scripts/catalog.ts`
- `scripts/e2e.ts`
- `scripts/lib/catalog-build.ts`
- `scripts/lib/catalog-keygen.ts`
- `scripts/render-presets.ts`
- `scripts/sync-plugin.ts`
- `scripts/sync-rule.ts`
- `scripts/ui-demo.ts`
- `skills/framejam/SKILL.md`
- `skills/framejam/versions.json`
- `src/core/feedback.ts`
- `src/core/mcp.ts`
- `src/core/playbooks.ts`
- `src/core/presets.ts`
- `src/core/review-status.ts`
- `src/core/store.ts`
- `src/core/transcript.ts`
- `src/core/types.ts`
- `src/server/app.ts`
- `src/server/catalog-format.ts`
- `src/server/catalog-keys.ts`
- `src/server/catalog.ts`
- `src/server/cli.ts`
- `src/server/files.ts`
- `src/server/install.ts`
- `src/server/json-store.ts`
- `src/server/mcp.ts`
- `src/server/paths.ts`
- `src/server/playbooks.ts`
- `src/server/presets.ts`
- `src/server/save-dialog.ts`
- `src/server/skill.ts`
- `src/server/thumbs.ts`
- `src/server/transcribe.ts`
- `src/server/update.ts`
- `src/server/version.ts`
- `src/web/components/community.tsx`
- `src/web/components/feedback.tsx`
- `src/web/components/filter-dropdown.tsx`
- `src/web/components/header.tsx`
- `src/web/components/menu.tsx`
- `src/web/components/pick-banner.tsx`
- `src/web/components/review/comment-list.tsx`
- `src/web/components/review/dock.tsx`
- `src/web/components/review/filmstrip.tsx`
- `src/web/components/review/player.tsx`
- `src/web/components/review/review-shell.tsx`
- `src/web/components/review/storyboard.tsx`
- `src/web/components/review/transcript.tsx`
- `src/web/components/review/use-review.ts`
- `src/web/components/setup-steps.tsx`
- `src/web/components/states.tsx`
- `src/web/components/update-notice.tsx`
- `src/web/components/video-backdrop.tsx`
- `src/web/lib/api.ts`
- `src/web/lib/data-source.ts`
- `src/web/lib/pin.ts`
- `src/web/lib/use-active-pick.ts`
- `src/web/lib/utils.ts`
- `src/web/main.tsx`
- `src/web/pages/not-found-page.tsx`
- `src/web/pages/pick-page.tsx`
- `src/web/pages/playbook-detail-page.tsx`
- `src/web/pages/playbooks-page.tsx`
- `src/web/pages/projects-page.tsx`
- `src/web/pages/review-page.tsx`
- `src/web/pages/setup-page.tsx`
- `src/web/pages/style-detail-page.tsx`
- `src/web/pages/styles-page.tsx`
- `tests/anchor-overlay.test.ts`
- `tests/catalog-build.test.ts`
- `tests/catalog-client.test.ts`
- `tests/catalog-format.test.ts`
- `tests/catalog-helpers.ts`
- `tests/catalog-keys.test.ts`
- `tests/catalog-library.test.ts`
- `tests/catalog-styles.test.ts`
- `tests/catalog-surfaces.test.ts`
- `tests/core.test.ts`
- `tests/helpers.ts`
- `tests/http.test.ts`
- `tests/install.test.ts`
- `tests/json-store.test.ts`
- `tests/mcp-tools.test.ts`
- `tests/playbooks.test.ts`
- `tests/plugin.test.ts`
- `tests/presets.test.ts`
- `tests/review-status.test.ts`
- `tests/skill-rule.test.ts`
- `tests/start.test.ts`
- `tests/stdio.test.ts`
- `tests/store-contract.ts`
- `tests/transcribe.test.ts`
- `tests/transcript.test.ts`
- `tests/update.test.ts`
- `tests/web/playbook-notices.test.tsx`
- `tests/web/review-data-source.test.tsx`
- `tsconfig.app.json`
- `tsconfig.json`
- `tsconfig.node.json`
- `tsup.config.ts`
- `vite.config.ts`
- `vitest.config.ts`
