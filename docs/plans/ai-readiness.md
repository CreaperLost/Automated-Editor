# AI readiness: plan v2 (2026-10-13)

**Status 21 Oct 2026:** phases 0–3 built (MCP bridge, proposals, app actions + CLI, skill and
templates, `check`, shorts scenes, graphics v1, brand kits, library + gate). This file is the
design record and build log of the agent system; the current summary is in
[architecture.md](../architecture.md#the-agent-connection-mcp) and the open work in
[roadmap.md](../roadmap.md). Phases 4–6 below ("brand kit … recipes", "library, providers",
"screen-pinned callouts") are superseded by roadmap items STY-1, FTG-2/FTG-6 and FX-2.

AeroEdits is a free, open-source editor. AI comes from the **user's own agent** (Claude Code, Codex,
Gemini CLI, OpenCode with MiniMax or GLM, Cursor, …) connecting to the app over **MCP**, paid by
the user's own subscription. The app has no model, no chat panel, and no AI keys on this path.
Focus now: tutorials and educational videos. Next: gaming.

## Principles

1. **The app is a tool server, the agent is the editor.** The agent decides; the app does
   everything deterministic (cuts, rendering, alpha, compositing, mapping times through edits).
2. **The agent proposes, the app checks, the user accepts.** Agent edits arrive as proposal
   batches: one undo group each, unchecked until accepted (as jump cuts already work). An
   "autopilot" setting can accept them automatically.
3. **Text first, pixels when needed** (from browser-use/video-use). The packed transcript and
   telemetry are the main working surface (~10–20 KB); frames and filmstrips only at decision points.
4. **Check the output, not the plan** (from video-use, adukhan98/video-edit, kamgasimo/ai-video-editor):
   graphics are checked over the real footage; cuts are checked against the transcript.
5. **Pin to meaning, not to time.** Graphics and B-roll hang off transcript words or click
   events, so later cuts don't knock them out of place.
6. **One source for the look.** A brand kit feeds every graphic and caption.

## Why MCP only, and what that requires

It works with any agent platform, costs the app nothing, stays inside every provider's terms
(DaVinci Resolve 21.1 and OpenChatCut do the same), keeps secrets out of the app, and keeps the app
testable. What it requires:

| Gap | Answer |
|---|---|
| The app is a running GUI, but clients start their servers themselves | Agents start `aeroedits --mcp-stdio` (the only way in), which reaches the running app over a private pipe, like Roblox Studio's built-in server |
| Anything local could reach a localhost port (other accounts, web pages) | No port at all (decided 2026-10-13): a named pipe whose DACL allows only this account, remote clients refused, the app must create it first; the bridge checks the pipe's server runs as the same account. macOS: socket in a 0700 folder, peer uid checked. No token. No shell or script tool |
| Clients differ: coding agents have a shell, files and skills; chat apps often don't | Coding agents are the main target. The app renders graphics itself and offers `graphic_write_file`, so a client without a shell still works |
| MCP gives tools, not editing craft | Ship an `aeroedits` skill (SKILL.md, the standard most agents now read) that hands graphic authoring to the HyperFrames skills; repeat the essentials in the server's `instructions` and as MCP prompts (recipes) |
| The user can't see what the agent is doing | An **Agent** review panel (not a chat): connection status, proposal batches with reasons, accept/reject/undo. A **Connect an agent** page with config to copy for each client and an "install skill" button |
| MCP sampling (the server borrowing the client's model) would be handy | Clients support it unevenly; don't depend on it |

The existing passes stay (decision 5): transcription in `transcript/` and the OpenAI/OpenRouter passes
in `ai/`. MCP adds the agent as a second source of the same suggestions.

## Graphics (HyperFrames only)

### The graphic asset

A new `AssetKind::Graphic`:

- `dir`: `<project>/graphics/<id>/`, a HyperFrames project (index.html, assets).
- `variables`: JSON passed to `hyperframes render --variables-file` (template inputs: text, numbers, colours).
- `mode`: `overlay` (transparent, adds to the video) or `fullscreen` (opaque, plays on its own over the voice).
- A render cache keyed by a hash of (files, variables, canvas size, fps, format).

**The app runs the render**, not the agent. Then it can re-render when inputs change, when the
canvas changes (16:9 to a 9:16 short), and at export resolution. Pin the HyperFrames version (as
ai-video-editor does) and check Node 22+, Chrome and FFmpeg with `hyperframes doctor` on the
Connect page.

### Alpha: decided automatically

- `overlay` renders `--format mov` (ProRes 4444) or `webm` (VP9 alpha); `fullscreen` renders `mp4`.
- For any imported or rendered file: read the stream's alpha (`yuva*` pixel format; for VP9 WebM the
  `alpha_mode=1` tag, since ffprobe reports `yuv420p`). If it has alpha, sample the alpha plane
  across the clip. **All opaque → drop the alpha** and treat it as fullscreen (decode as NV12, which is
  cheap). Mostly transparent → overlay. The bounding box of the visible pixels feeds the safe-zone check.
- Decoding fixes needed (found in the code): preview decode uses NV12 whenever `limit.yuv` is set
  (`media/ffmpeg.rs:750`, `:854`), which throws away alpha, so streams with alpha must take the BGRA
  path. WebM alpha needs `-c:v libvpx-vp9` *before* `-i`, because FFmpeg's built-in VP9 decoder ignores
  alpha. On macOS, AVFoundation decodes ProRes 4444 but not VP9 alpha.
- Default format: **open, see "Overlay format" below.** The rendered file is a cache that can be
  rebuilt from the HyperFrames source, so this can change later without breaking projects.

### Overlay format (measured 2026-10-13, bundled FFmpeg 8.1)

Test clip: a synthetic 6 s 1080p30 transparent lower third (gradient card sliding in, text), mostly empty frame.

| | ProRes 4444 (.mov) | VP9 alpha (.webm, crf 30) |
|---|---|---|
| Size | 22.2 MB (~30 Mb/s) | 63 KB |
| Encode | 10.4 s | 4.4 s |
| Decode all 180 frames to BGRA | 0.77 s (4.3 ms/frame) | 0.42 s (2.3 ms/frame) |
| Seek to 5.5 s | 0.14 s | 0.16 s |
| Alpha with FFmpeg's default decoder | kept (`yuva444p12le`) | **lost** (alpha min 255); kept only with `-c:v libvpx-vp9` |

The test was synthetic and mostly transparent. Re-measure in the spike with real HyperFrames
renders (full-frame motion, 60 fps, 4K, several overlays at once).

### Placement modes

1. **Overlay:** a transparent clip on a video track above screen and camera, below captions. It
   must stay out of the safe zones the app already knows: the webcam bubble's rectangle and the
   caption band.
2. **Fullscreen (cutaway):** opaque; replaces the picture while the voice continues. Layout flag:
   hide the webcam bubble or keep it on top.
3. **Later, screen-pinned callouts:** an arrow or highlight on a UI element has to follow smart
   zoom and pan, so it is drawn in the *screen layer's* coordinates, not the canvas's. This is
   unique to tutorials and where AeroEdits wins: no generic editor knows where the click was.
4. Later: split or inset layouts (the screen shrinks beside a graphic).

### Anchors

A clip carries `anchor: { start_word, end_word, lead_ms, tail_ms }` or `{ click_event, … }`. The
clip's position on the timeline is worked out from the anchor through the edit mapping (`sequence/clock.rs`).
If edits remove all the anchored words, the clip is flagged as orphaned in the review panel, never
silently deleted. A graphic's length comes from a `duration` variable, so it re-renders to fit
its span. **Needs a design pass:** how `sequence/edit.rs` ripples anchored clips.

### Brand kit and templates

- `brand.json` (fonts, colours, logo, caption style) → `brand.css` added to every graphic.
- A tutorial template pack (HyperFrames compositions with `data-composition-variables`):
  lower third, chapter or section title, "Step N" card, keyboard-shortcut chip, callout/arrow,
  checklist/recap, code snippet, terminal, before/after, definition/term card, stat count-up,
  end card. Look in the HyperFrames registry (~400 items) before building one.
- With a template, the agent only fills in variables, which is cheap and keeps the look
  consistent. A freeform composition only when asked for (hero moments).

## B-roll and media library

- An app-level SQLite index (FTS5) over folders and providers. Each record: path, source
  (local | pexels | pixabay | graphic | generated), provider ID and URL, licence and its URL, author,
  download date, duration, size, fps, has_alpha, description, tags, contact-sheet thumbnail.
- **Descriptions come from the user's agent**: `library_pending` hands over contact sheets, the agent
  looks at them, and `library_describe` stores the result. No model in the app. Local embeddings
  (SigLIP via ONNX) only if keyword search turns out not to be enough.
- Providers: Pexels and Pixabay, with the user's own free key in the OS keychain (`secrets.rs`).
  Search local first, then the providers; download only on accept; record the licence. For
  tutorials B-roll matters less than graphics, so it comes later.

## MCP tools, v1 (about 20, and they report what changed)

Reading:
- `project_info`: canvas, fps, length, tracks, layout, safe zones, `revision`
- `transcript_get`: packed lines with word IDs, times on the edited timeline, pauses
- `telemetry_get`: clicks and scrolls (normalized positions) and zoom segments, on edited time
- `timeline_get`: a compact list of clips per track
- `frame_render(t)`: a PNG of the finished frame (graphics and captions included)
- `timeline_view(range)`: filmstrip + waveform + word strip PNG (video-use's best idea)
- `graphic_list`, `library_search`

Changing (take `expected_revision`; return a diff and the new revision):
- `graphic_create(template?, mode)` → folder path; `graphic_write_file` (for clients without a shell)
- `graphic_render(id, variables?)` → lint/check results + 3 frames over the real footage at the anchor
- `propose_graphic`, `propose_broll`, `propose_cuts` (reuses the silence, filler and retake passes)
- `library_fetch`, `library_describe`, `proposals_list`, `proposal_withdraw`, `undo_batch`

Accepting is the user's job in the UI unless autopilot is on.

## Recipes (prompt, template or setting)

A recipe is markdown guidance plus JSON limits: graphics per minute, allowed templates, B-roll on/off,
caption style, how hard to cut. Picked in the UI, readable with `recipe_get`, and published as MCP
prompts. The app enforces the hard limits (density, safe zones) when it checks proposals.

## Gaming (later; keep the design open)

"Moments" come from detectors: game-audio peaks, mic excitement, kill-feed OCR/YOLO with a profile per game
(see Montage Maker, Highlightly, Auto-Clipper). Combining signals beats any single one. Keep
markers/moments generic (`markers_get(kind)`), not tied to mouse telemetry.

## Notes from the code (for phase 1)

- Every Tauri command is a thin wrapper over `commands::*_impl(&AppState, …)`, and edits already
  take `expected_revision` and a typed `SequenceEdit` (add track, place asset, move, trim, split,
  delete range, link, set clip). The MCP server can call the same `_impl` functions with the
  same `AppState`, so the revision checks and undo behave exactly as for the UI.
- The UI only learns about backend changes from command results. Agent edits need a
  `project-changed` event that makes the front end reload the project summary.
- No HTTP server crate yet. Options: `rmcp` (the official Rust MCP SDK, Streamable HTTP over
  axum) or a small hand-written JSON-RPC endpoint (no server-sent events needed for v1).

## Phases

0. **Spike: done 2026-10-13 (Windows).** `media/ffmpeg.rs` reads alpha in `probe_video`
   (`Alpha::{None, InFormat, Vp9}`); a stream with alpha always decodes as BGRA, never on a GPU decoder,
   with `-c:v libvpx-vp9` forced for VP9. `media_bin::import` gives a transparent video the
   Overlay role. Tests: probe, decode with the NV12 preview limit, CPU composite, import.
   In the dev app: a HyperFrames lower third (hyperframes 0.8.152) on a new top track composites
   correctly in the wgpu preview at ~60 fps and in a 1080p export.
   Measured: rendering a 5 s 1080p30 graphic takes 25–28 s (~6 s setup, ~13 s Chrome capture
   at ~11 fps), so renders need a background queue. Decoding the real render: WebM 2.7 ms/frame,
   ProRes 6.2 ms/frame; a seek costs ~150 ms either way (mostly starting the process). HyperFrames
   WebM had 2 keyframes in 150 frames, which doesn't matter for clips this short.
   **Not checked: macOS** (AVFoundation path; it needs ProRes or the FFmpeg backend).
   Seen in the preview: the lower third covers the webcam bubble, so safe zones are needed.
1. **MCP skeleton: done 2026-10-13 except telemetry_get and timeline_view.** `src-tauri/src/mcp/`:
   - Transport (changed the same day from localhost HTTP + token, after comparing Roblox, Resolve,
     Figma, JetBrains, Blender and CVE-2025-49596): agents start `aeroedits --mcp-stdio`
     (`bridge.rs`, copies bytes), which opens `\\.\pipe\aeroedits-mcp-<user SID>` (`ipc.rs`):
     DACL `D:P(A;;GA;;;<SID>)`, `reject_remote_clients`, `first_pipe_instance`; the bridge
     refuses a pipe whose server process runs as another account. macOS: `agents/mcp.sock` in
     the config dir (0700), peer uid checked both ways (compiles untested: `ring` needs a macOS
     toolchain). `rmcp` 3.5 serves each connection over the stream; no axum.
   - Off by default (`mcp.json` `{enabled}`). `connected` lists agents by their handshake name;
     activity records which agent called what.
   - One-click connect (`clients.rs`): Claude app (`<config>/Claude/claude_desktop_config.json`)
     and Codex (`$CODEX_HOME` or `~/.codex/config.toml`, via toml_edit so comments stay), backup
     `*.aeroedits-backup` first, Disconnect removes only our entry. Others get copy text.
   - Tools (read-only): project_info, transcript_get, timeline_get, playhead_get, frame_render,
     layout_get. Agent edits will emit `aeroedits-project-updated` via `Host::project_changed`.
   - Checked: pipe DACL read back through Windows (one protected allow entry, this account), a
     second owner refused, bridge + pipe + tools end to end, the official TypeScript MCP SDK
     client launching the debug exe over stdio (35 ms connect), Connect/Disconnect on a scratch
     `CODEX_HOME`. **Not checked:** a real Claude app / Codex app session, macOS.
2. **Proposals, review panel, revisions: built 2026-10-13** (decisions agreed with George the same day):
   - **Storage:** `<project>/agent/proposals.json` (`src-tauri/src/proposals.rs`), beside the edit and outside
     it, so proposing takes no revision or undo step and survives a restart. All pending batches are kept, plus
     the newest 50 decided ones; at most 50 waiting.
   - **Batches:** `Cuts { stream, items }` (each item a word span `fromWord..toWord` or an edited range, with a
     kind and reason) or `Edits { edits: Vec<SequenceEdit> }`. Checked on a copy when proposed; the summary
     (cut count and seconds, or a clip diff: `+ V2 c-9 …`, `~ A1 c-3 … (was …)`, `Length a → b`) is returned.
   - **Accepting** is one undo step (`ripple_cuts` for cuts, `ProjectReader::commit_document` for edits).
     Word cuts are worked out again from the current edit (they survive later edits); ranges and edits only at
     their base revision, otherwise the batch shows as out of date and the agent must re-propose.
   - **Undo** of a batch only while its accept is the latest revision. The user's undo puts the items back to
     waiting; the agent's `undo_batch` withdraws the batch.
   - **Autopilot:** `mcp.json` `{autopilot}`, off by default, switch in the Agent panel header. Auto-accepted
     batches stay listed with Undo.
   - **Tools (13):** + `telemetry_get` (clicks, scrolls merged within 0.5 s, zooms, on edited time),
     `timeline_view` (PNG: filmstrip, waveform, words with ids, pending cuts shaded), `propose_cuts`,
     `propose_edit`, `proposals_list`, `proposal_withdraw`, `undo_batch`. Tools get a `Ctx` with the agent's
     handshake name; changes emit `aeroedits-project-updated` / `aeroedits-proposals-updated`.
   - **UI:** an **Agent** dock panel (tabbed with Inspector; tab title shows the waiting count): batch cards
     with agent, reason, summary, ticked cut items with play buttons, Apply N cuts / Reject / Undo; History.
     Agent cut items also appear in the transcript review list (`source: agent`, `kind: cut`, a badge with the
     agent's name), decided through their batch, and striped on the transcript words and across the timeline.
   - Requests on one MCP connection run concurrently (found in the e2e test); agents that need order must wait
     for each reply.
   - Debug builds only: `AEROEDITS_MCP_DEV=<name>` suffixes the pipe so a dev copy runs beside the installed app.
   - **Checked:** 7 tests in `proposals.rs` (wait → accept as one undo step → undo, word cuts surviving an edit,
     ranges and edit batches going stale, item-by-item decisions, agent ownership, merging, pruning), 2 more over
     the bridge and the pipe in `mcp/mod.rs` (propose → user accepts → proposals_list/transcript_get →
     undo_batch; autopilot with a bad edit refused), 2 for timeline_view drawing, `cargo test --lib` 351 passed, `tsc --noEmit` clean, cleanup tests 5/5, `vite build`. In the dev
     app on a scratch copy of a real 5:13 recording, through the official MCP TS SDK client: two batches proposed
     (revision unchanged), panel screenshot in `docs/plans/phase-2-agent-panel.png`, Apply 2 cuts (5:13.48 →
     5:02.27, one revision), the edit batch then shown out of date, Undo from History, keep/cut single items from
     the transcript review list. timeline_view on the real footage took 1.4 s for 32 s at 1600 px.
   - **Not checked:** macOS; a real Codex or Claude app session calling the write tools; autopilot in the dev
     app (it shares `mcp.json` with the installed app; covered by the pipe test). "Cut all" in the transcript list
     cuts agent items as plain word cuts, so their batch then shows them as "already cut" until decided.
2b. **App actions: the agent presses the app's own buttons (started 2026-10-14).** George's goal: first let the
   agent do what he does by hand, using the built-in passes, which cost nothing next to an Opus-class model
   reading transcripts. Graphics wait for phase 3. Agreed:
   - **One set of actions, two doors.** Every action is an MCP tool *and* a CLI command,
     `aeroedits do <action> --key value` (talks MCP over the same private pipe). The CLI costs agents with a
     shell (Claude Code, Codex) no tool list in context; MCP stays for the Claude and Codex desktop chats.
   - **Actions apply directly**, one undo step each, like a click. They are listed in the Agent panel's history
     with Undo. The proposal flow stays for the agent's own judgment calls.
   - **Settings:** each action takes its settings as optional arguments (missing ones: the app's saved values or
     the Balanced preset). `settings` reads and patches the captions, layout, audio, zoom, webcam focus,
     transcription and AI-pass settings, and lists the presets. API keys are never readable or settable.
   - **Feedback:** each action returns a one- or two-line summary (what changed, length before → after, undo
     step), errors say what to do next; long jobs report progress (MCP progress notifications; the CLI prints
     them to stderr) and the Agent panel shows what is running.
   - **Actions v1:** `status`, `open`, `create`, `undo`, `redo`, `transcribe`, `mouth_noises`, `silence`
     (audio silence or transcript gaps, presets gentle/balanced/tight/aggressive), `fillers` (built-in rules,
     optionally the OpenAI/OpenRouter pass), `webcam_focus`, `zooms`, `settings`, `chapters`, `shorts`, `export`,
     `export_status`, and `cleanup` (transcribe if needed → mouth noises → silence → gaps → fillers → webcam →
     zooms, one call, a line per step). Detection actions take `apply: false` to only report what they found.
   - **Built 2026-10-14** (`src-tauri/src/mcp/actions.rs`, `mcp/cli.rs`): the 17 actions above wrap the same
     `commands::*_impl` functions the UI calls; the jump-cut presets are copied from
     `front-end/src/lib/jumpCutConfig.ts` (keep in step). `settings` merges a JSON patch through the settings'
     own types and refuses unknown fields by name. Changes are listed in the Agent panel as `action` entries
     (status Done / Undone, Undo while latest). Tools get a `Ctx` with the host (transcription state, window
     events) and a progress callback; MCP progress notifications go out when the client sends a progressToken;
     the panel shows running calls with progress (`McpStatus.working`, event `aeroedits-agent-activity`). An
     agent opening or creating a project emits `aeroedits-agent-opened` and the editor loads it.
     CLI: `aeroedits do` lists, `do <action> --help` shows options with their choices, `--kebab-case value`,
     `--no-x`, bare flags, comma lists, `--json '{...}'`, plain words fill required options
     (`do settings captions`); exit 0 / 1 (action failed) / 2 (usage or app unreachable); images are saved to
     the temp folder; the caller is named from `AEROEDITS_AGENT`, else `CLAUDECODE` / Codex env, else `cli`.
     File > AI agents has a "Command line" tab with notes to paste into CLAUDE.md / AGENTS.md.
   - **Checked:** `cargo test --lib` 360 passed (new: 5 action tests on a test recording with tone "speech",
     pauses and a mouth click: fillers cut and undone, silence and gaps with presets and overrides, the click
     splitting a pause until mouth_noises mutes it, settings patch and unknown-field refusal, cleanup step order,
     export to a real MP4 and create; 3 CLI parsing tests; the CLI over the pipe with progress on stderr and
     exit codes), clippy clean on the new code, `tsc` clean, cleanup tests 5/5, `aeroedits do status` with no app
     says what to do and exits 2.
   - **Bridge that outlives the app (2026-10-14).** First real test: George's Claude app chat showed no
     AeroEdits tools after he reinstalled AeroEdits. The old bridge connected once at start; with the app
     closed it answered every request with an error, so the client marked the server failed until the
     Claude app restarted. Now `bridge::relay` answers `initialize` (echoing the client's protocol version
     when it has a handshake, else 2025-11-25), `ping` and `tools/list` itself from the same build's tool
     list, connects to the pipe on the first other request (repeating the handshake under the agent's
     name), forwards responses and progress, answers calls in flight with "AeroEdits closed during this
     call" when the pipe drops, and connects again on the next call. With the app closed a call returns
     the tool error "AeroEdits is not running, or AI agents are off…" (or the pipe's own refusal reason).
     Test: one bridge kept through app closed → open → closed → open, tools listed throughout, the agent
     named in activity.
   - **Not checked yet:** anything in the running dev app (George asked to hold the dev server); `transcribe`
     for real (needs the Parakeet model or a key), `webcam_focus` and `zooms` (the test recording has no camera or
     mouse data), `chapters`/`shorts` (need an OpenAI/OpenRouter key), how MCP clients treat a long call
     (transcription, export: progress notifications should keep it alive), the release build's console attach.
2c. **Skill and templates (built 2026-10-14).** George ran a full tutorial edit from the Claude app with a
   hand-written prompt; it worked. His feedback: the steps belong in a template the agent reads by itself, it
   should ask before each pass with real numbers (transcriber; mouth noises; silence from audio or transcript,
   one mic or both; how many cameras and whether to keep them; how many zooms and how long), captions off unless
   it's a short, chapters last, and the export flow in the skill rather than each prompt. Agreed the same day:
   one question round after a `scan`; all three ways in; answers remembered ("same as last time?").
   - Text in `src-tauri/src/mcp/skill/` (`SKILL.md`: rules for asking, reporting, failing, undo; `export.md`;
     `templates/tutorial.md`), compiled into the app by `mcp/skill.rs`.
   - Ways in: the `template` tool (no name lists them; with a name returns rules + template + export + saved
     answers), and the server instructions say to call it first on any edit request; MCP prompts per template
     (`tutorial` with `recording`/`name` arguments; answered by the bridge too, so no app needed); an
     Install/Update/Remove skill row for Claude Code (`~/.claude/skills/aeroedits`, `CLAUDE_CONFIG_DIR`) and
     Codex (`$CODEX_HOME/skills/aeroedits`) in File > AI agents. Installed copies are refreshed at launch while
     their SKILL.md keeps the "Installed by AeroEdits" line; removing only takes out our files.
   - `template_save` keeps answers in `<config>/AeroEdits/agents/answers/<template>.json`.
   - New `scan` action: microphones, other sounds, cameras (and whether shown), mouse events, transcriber
     readiness, AI key, and dry runs of mouth noises, silence, gaps, fillers, webcam focus (stretches and total)
     and zooms (count, average and range of length, scale) for one preset; changes nothing.
   - Actions: `transcribe provider: parakeet|elevenlabs` (saved as the choice; refuses with the reason when not
     ready), `silence stream: speech` for every microphone (gaps too, which needs each transcribed),
     `webcam_focus apply: false` (`commands::webcam_focus_find_impl` / `webcam_focus_store_impl` split out of
     detect), `zooms apply: false` and zoom summaries with lengths. CLI: one plain word fills the only option of
     a tool with no required ones (`aeroedits do template tutorial`).
   - **Checked:** `cargo test --lib` 365 passed (new: guide order and saved answers, prompts, skill
     install/refresh/remove, template tools, scan changes nothing, prompts over the pipe and through the bridge
     with the app closed, CLI word), clippy clean on the new code, `tsc` clean.
   - **Not checked:** the dialog's skill rows and a real run in the Claude app with the prompt from the + menu
     (dev server on hold); `scan` on a real recording with a camera and mouse data (timing: silence and gaps
     each scan the audio once); whether the Claude app shows MCP prompt arguments.
2d. **Pass-by-pass approval (2026-10-15).** George's findings from real runs: chapters must come last,
   right before export; the app's OpenRouter filler/retake review works very well and should be the
   default; the agent called `cleanup` at once instead of following the template. Agreed: no one-call
   cleanup; approve each pass with its own dry run; "auto" makes the choices for him.
   - `cleanup` (and the project pinning only it used) is removed. `scan` is an overview whose numbers
     are a first estimate; each pass is asked with its own `apply: false` dry run just before it runs.
   - Template `tutorial`: step by step by default, automatic on the user's word ("auto", "no
     questions", yes to "same as last time?"): saved answers or defaults, dry run, one line per pass,
     stop when a dry run looks wrong (a pass over a quarter of the video, all over a third, far from
     `scan`, any warning). Gaps between words are their own pass (default no). Fillers: dry run with
     the AI review, then run with `ai: false`, which keeps the review's saved finds (no second charge).
   - `fillers` `ai` defaults to on when a key is set (`Host::ai_review_ready`; tests' hosts say no, so
     tests never spend the key); without a key the rules run and say so. `scan`'s dry run is rules only.
   - Chapters: the tool description, SKILL.md, the server instructions and the export steps say last;
     `status` and `export` warn when the agent's chapters predate a later change (warn, not refuse, so
     hand-edited chapters are never forced over).
   - **Checked:** `cargo test --lib` 370 passed (new: the AI default without a key, chapters reported
     out of date after a cut, no `cleanup` in the tool list), clippy clean on the new code.
2e. **Check before export (2026-10-15).** Picked by George from a list of ten ideas (goal: app first,
   AI-friendly as built). `src-tauri/src/check.rs` reads the edit only (no decode, no model): missing
   media (error), black frames, no sound, blips between two cuts (no word in them), words cut more
   than word-edge refinement trims (>200 ms and >30%), choppy stretches (4 cuts in 3 s), zooms under
   0.8 s on screen or whose easing a cut took, captions over the webcam bubble (real cue sizes via
   `captions::cue_size`, bubble via `render::webcam_bubble_rect`), chapters cut / under 10 s / fewer
   than 3, loudness not normalized or under -16 LUFS, pending proposals. Safe fixes (`check::Fix`):
   cut the blip, close an empty gap, delete a flashing zoom, move a jumping zoom into the first/last
   kept stretch that holds its whole easing (zooms ease in recording time, so a cut in the easing
   really jumps), raise captions clear, remove the chapter, normalize to -14. On a copy of "Tutorial
   14 Oct 2026": 3 ms, no warnings, 6 zoom jumps (all fixable) + 2 choppy notes + loudness; after
   the fixes only the 2 choppy notes remain. Commands `project_check` / `project_check_fix` (ids or `["all"]`, latest
   first so ripples never move a pending one, each its own undo step); `CheckPanel` in the export
   dialog (Show seeks and closes it); MCP/CLI action `check` with `fix`; tutorial template and export
   steps run it before asking to export. Not in v1: chapters-stale (any revision change, a fix
   included, would trip it; `status` keeps that warning), shorts, frame decoding (black or frozen
   footage inside clips), measured loudness.
2f. **Shorts scenes and agent-picked shorts (2026-10-15).** Ideas 1-3 of the ten. `short_scenes.rs`:
   `ShortLayout.scenes` (camera | screen, anchored in source time with `media`, `auto` flag) and
   `scene_transition_ms` (default 350, 0 = cut). `split_scene` eases the camera's share of the height
   (`camera_share`: base → 1 or 0, scenes that meet go straight across) and moves captions toward a
   full-frame spot (70% height). `suggest`: hook (camera, first 2.5 s unless busy at once), busy
   screen (4+ clicks/scrolls within 1.5 s of each other over 2.5 s+), talking with the mouse at rest
   (webcam-focus detection with an 8 s rest), screen wins, scenes under 1.5 s dropped, near ones meet.
   Commands `project_short_scenes_pick`, `project_short_scene_add` (short time → source via
   `shorts::scenes_at`/`with_scene`); UI positions via `shownStartUs` in `attach_edited`. Studio:
   Scenes section (Pick scenes, Camera/Screen here, strip, per-scene mode/remove, change speed;
   pending look saves flush before scene ops). MCP: `short_create` (word ids, copies the first short's
   look, picks scenes), `short_scenes` (list/pick/set/clear), `export` `short` (id/title/"all"),
   template `shorts` (the agent picks moments from `transcript_get`; no API key). Real recording
   probe: mostly split, a hook per short, a few screen bursts and camera rests. Not done: a scene
   lane on the timeline, graphic scenes (phase 3), removing shorts by action.
3. Graphic assets: see "Graphics v1 (agreed 2026-10-15)" below; it replaces the placement modes,
   anchors and safe zones of the older "Graphics" section.
4. Brand kit, tutorial template pack, `aeroedits` skill, recipes.
5. Library, providers, agent-written descriptions.
6. Screen-pinned callouts that follow zoom; click anchors.

## Graphics v1 (agreed 2026-10-15)

George's direction: graphics are one of the last passes, after the cuts and timeline are checked;
they must never be chopped; one kind of asset; the source stays editable and reusable. Placement
was left to us.

**One asset, a slot decides the size.** `AssetKind::Graphic`: a HyperFrames project in
`<project>/graphics/<id>/` (index.html, assets), `variables.json`, a brand kit name, and the canvas
it was authored for. Where it plays decides the box it is rendered into (the *slot*), so there
are no safe zones to check: the slot belongs to the graphic alone.

| Where | Slot | Drawn | Render |
|---|---|---|---|
| 16:9 video | the whole canvas | above everything: screen, webcam, captions | opaque MP4 |
| Short, fullscreen | the whole 9:16 canvas | above everything | opaque MP4 |
| Short, split | the camera's or the screen's part of the split (at the base share) | in that part; captions as for the split | opaque MP4 |
| Short, over the screen | the screen's rectangle | over the screen, under the camera and captions | WebM VP9 alpha |

Renders are a cache keyed by (source files, variables, brand, slot size, fps); the source is never
touched by a render. The composition reads its size from the slot, so a template written with
relative units fits any slot.

**Placement: anchored, rigid, protected.**
- *(As built, `graphic_add(atWord)` resolves the word to a time once and stores a plain clip
  that ripples with cuts; keeping the word anchor is part of [PLAN-1](../roadmap.md#plan-1).
  The design below is the intent.)*
- A placement stores an anchor in the recording's time (a word id when there is one, else source
  µs) and its own length. Its start is mapped through `sequence::clock`, then it plays its full
  length in timeline time. Shown on a **Graphics lane** above the video tracks (like the zoom lane);
  dragging moves the anchor, trimming sets the length (and the `duration` variable, which
  re-renders).
- **Never chopped (default, no switch needed):** the time a graphic covers is protected. Silence,
  gaps, fillers, mouth noises, `check` fixes and agent cut proposals skip it (like protected
  words). A manual cut that reaches into it is refused with "A graphic plays here: move or remove
  it first". Cuts before it move it with its anchor; cuts after it don't touch it.
- If its anchor is cut anyway (a cut from before protection, a hand edit of the clips under it),
  `check` reports it as orphaned; it is never deleted silently.
- Shorts pick up the placements whose anchor falls inside the short's range. Each short
  placement chooses its slot (fullscreen, split top/bottom, over the screen); a short `graphic`
  scene kind covers this (closes 2f's open item).

**Converting 16:9 → 9:16.** "Convert for shorts" re-renders in the 9:16 slot when the composition
is responsive (all bundled templates are). Otherwise it makes a variant
(`graphics/<id>/variants/<slot>/`, a copy of the source) and hands it to the agent:
`graphic_adapt` returns the source, the target slot and frames of the original, and the agent
rewrites the layout. The variant is listed under the same graphic.

**Editability and reuse.**
- Variables form in the Inspector, generated from `data-composition-variables`; a change
  re-renders in the background. "Open folder" for hand edits; the app watches the folder.
- An app-level **graphics library** (`<config>/AeroEdits/graphics/`): Save to library from a
  project, Use in project copies it in (a project never points outside itself).
- Bundled tutorial templates + library items show in a **Graphics** tab of the Media panel:
  pick, fill the form, place at the playhead. Works with no agent.

**Brand kits.** Several, app-level (`<config>/AeroEdits/brands/<name>/brand.json` plus logo and
fonts) → `brand.css` added to the graphic at render time. A project has a default kit; each
graphic can pick another. Changing a kit re-renders the graphics that use it.

**Rendering.** A background queue runs a pinned `hyperframes render` per (graphic, slot); the
preview shows the graphic's poster frame with a "rendering" badge until the file lands, export
waits for pending renders. Compositing: a new top layer role above captions for full and split
slots; `push_overlay` limited to the screen rect for the over-screen slot.

**Agents.** Actions (MCP + `aeroedits do`): `graphic_templates`, `graphic_create` (template or
blank → folder), `graphic_write_file`, `graphic_set` (variables, brand, length), `graphic_render`
(lint + 3 frames at the placement), `graphic_place` (word id, slot), `graphic_adapt`,
`brand_kits`. The tutorial and shorts templates get a graphics pass after the cuts and checks,
before chapters.

**Runtime: downloaded on first use (decided 2026-10-15),** like the Parakeet model. The first time
graphics are used (Graphics tab, or an agent's first graphic action, which returns "graphics
runtime not installed" with the size), the app downloads into `<data>/AeroEdits/graphics-runtime/`:
a pinned portable Node 22 (nodejs.org zip, checked against its SHASUMS256), a pinned
`hyperframes` installed with that Node's npm, and a pinned chrome-headless-shell (Chrome for
Testing, checked by hash). About 200 MB, with progress and a Remove button. Nothing on `PATH` is
used or changed. To check in the spike: how `hyperframes render` is pointed at our Chrome
(`hyperframes browser` / a Puppeteer executable path) and that it needs no other global tools.

**Open:** live preview of the HTML while a render runs (later, if the placeholder feels slow).

**Built 2026-10-15 (uncommitted on `shorts`, not run in the dev app yet).**
- `src-tauri/src/graphics/`: `runtime.rs` (Node 22.23.3, chrome-headless-shell 152.0.7977.30 =
  the version HyperFrames 0.8.162 pins, SHA-256 pinned for win x64/arm64, mac arm64/x64,
  linux x64; `npm ci` from `src-tauri/graphics/runtime/package-lock.json`; system `tar` unpacks;
  `installed.json` written last; `AEROEDITS_GRAPHICS_RUNTIME` overrides the folder), `render.rs`
  (one background render at a time; file `<w>x<h>-<ms>-<fps>-<hash>.webm`, hash over every
  source file + canvas + length + fps + version; long side 1920; 30 fps; the last render for the
  canvas stands in while a new one is made, else a placeholder; `ensure` for export;
  `generation()` joins the preview's frame key so a finished render redraws; HyperFrames env:
  `HYPERFRAMES_BROWSER_PATH`, `_FFMPEG_PATH`, `_FFPROBE_PATH`, `_NO_TELEMETRY`), `mod.rs`
  (source model, variables checked against `data-composition-variables`, `size_composition`
  sets the staged copy's root size/duration and viewport, `place` = add + place on a free
  "Graphics" track or a new top one, one undo step), `templates.rs`.
- Templates in `src-tauri/graphics/templates/` (title, step, shortcut, key-point, recap,
  end-card, blank): Web Animations, transparent page with a fading card (so the WebM's alpha
  crossfades over the video), vw/vh/vmin + portrait block, colours `background`/`text`/`accent`
  in every one (ready for brand kits). All lint clean; rendered at 16:9 and 9:16 and checked.
  The six are full-screen versions of the earlier list: lower third → key point, callout →
  shortcut card.
- `AssetKind::Graphic` (edited as a still; folder found from the project root by id, so a moved
  project keeps them). Editor rules in `sequence/edit.rs`: `DeleteRange` keeps only what lies
  outside graphics (all inside → "A graphic plays here: move or remove it first"), split-all
  passes graphics by, splitting one is refused, and any edit that changes a graphic it was not
  about is refused (`graphics_kept`).
- Scene: graphics drawn last (above captions), in the 16:9 scene and the short's split scene.
- UI: Media panel Media | Graphics pages (install with progress/cancel, templates, graphics in
  the project with render state, renderer remove/retry); Inspector Graphic section (name,
  variables form, length, open folder, render again, render error).
- Agents: `graphics`, `graphic_add` (`atUs` or `atWord`), `graphic_set`, `graphic_write_file`
  (returns lint), `graphic_render` (lint + waits), `graphics_install`; tutorial template step 6
  "Graphics (ask)" before chapters; SKILL.md "Graphics"; server instructions.
- Checked: `cargo test --lib` 403 passed (editor rules, variables, sizing, templates, placement
  in one step, actions with `atWord` and a filler cut refused under a graphic), the real install
  into a scratch folder (~70 s, 490 MB on disk) and real renders at 1280x720 and 540x960 canvases
  (16-17 s each, transparent first frame, opaque card), clippy clean on the new code, rustfmt,
  `tsc`, `vite build`.
- Not done: convert-for-shorts variants for non-responsive graphics, undo for variable changes
  (they are source, not edit), macOS run.
- **Run in the dev app (2026-10-15, first pass only, from a snapshot of the tree)** on a copy
  of "Untitled 15 Oct 2026" (5:19): the Graphics page, a Section title added at 0:11 on a new
  "V3 · Graphics" track, the placeholder until the renderer was installed from the page, the
  render landing and the preview redrawing by itself, the fade-in crossfading over screen and
  camera, a title change in the Inspector re-rendering in ~20 s, a cut inside the graphic
  refused, a cut across it keeping its 4 s (11 s removed, graphic moved along), split-all
  passing it by, and a 20 s 1080p export (10 s) with the graphic fading in and out.
  **Bug found and fixed:** renders inside a long project path failed with ENAMETOOLONG
  (HyperFrames' temp name on top of the stage/`.part` paths in the project cache passed 260
  characters); renders now stage in `<runtime>/work/<hash12>/` and land via
  `.landing-<id>.webm`. Known: a replaced render the preview's decoder still holds is not
  deleted until the next render for that canvas; the fade-out shows the text over the video
  for a moment (the card and text fade together), which could be tuned.

**Built 2026-10-15, second pass (uncommitted on `shorts`, not run in the dev app yet).**
- Brand kits (`graphics/brands.rs`): `<config>/AeroEdits/brands/<name>/brand.json` (background,
  text, accent, font family, optional `fontFile` and `logo` copied into the kit's folder as
  `font.<ext>`/`logo.<ext>`; `AEROEDITS_BRANDS_DIR` overrides). Names are folder names (letters,
  digits, space, `-`, `_`; `none`/`default` reserved); saves go through a staged folder and
  swap in, renames included. The project default is `EditDocument.graphics_brand` (an edit, one
  undo step; a no-op pick commits nothing); a graphic's own choice is `brand` in `graphic.json`
  (absent = project's, `none` = no kit). At render time the kit fills `background`/`text`/
  `accent` where the composition declares them and the graphic left them unset, and a
  `<style id="aeroedits-brand">` goes last in `<head>` (font on `body`, `@font-face` from the
  kit's file, `--brand-background/-text/-accent/-font/-logo`), with the files in
  `aeroedits-brand/` of the staged copy. The kit's settings and file bytes join the render hash;
  a missing kit is no kit. Saving or removing a kit calls `sources_changed`.
- Library (`graphics/library.rs`): `<config>/AeroEdits/graphics/lib-<id>/` = a copy of the
  graphic's folder plus `library.json` (length it played, saved at); `AEROEDITS_GRAPHICS_LIBRARY`
  overrides. "Use" copies it into `graphics/<new asset id>/` without `library.json`
  (`create_from_folder`) and places it like a template, one undo step.
- Short slots: `ShortLayout.graphic_slots` (clip id → `full`/`camera`/`screen`/`overScreen`;
  clip ids survive `slice_sequence`, so a short's stretch and its own edit both work; an unlisted
  clip is full). `shorts::graphic_rect` = the split at the short's own share (scenes don't move
  it; with no camera the camera slot is still the top/bottom part). `Spec::for_clip` sizes the
  render from the render canvas (1080x1920) split, so preview and export share renders.
  `push_graphics` runs twice in the split scene: `overScreen` after the overlays and before the
  captions, the rest after the captions; `Scene::push_graphic` takes the rect. `short_create`
  and "use this look for all" keep each short's own slots.
- UI: Graphics page sections Templates, Your library (+ use at playhead, remove), In this
  project, Brand kits (project's kit picker, kits with swatches, make/change/remove form with
  logo and font file pickers via `pick_brand_file`). Inspector: brand kit picker (project's /
  none / a kit), colours the kit fills shown "from the kit" with "Use kit" to drop an override,
  Save to library. Shorts Studio: a Graphics section listing the short's graphics with a
  Full / Camera / Screen / Over screen switch.
- Agents: `brand_kits` (list + project default), `brand_pick` (project default, or one
  graphic's with `graphic`), `graphic_set` `brand` and `short` + `slot`, `graphic_add`
  `library`, `graphics` lists the library and each graphic's brand. SKILL.md, the tutorial
  (asks for a kit; saved answer `graphics.brand`) and shorts templates, server instructions.
- Checked: `cargo test --lib` (kits save/rename/remove and checks, filling only declared unset
  colours, the style last in head, the hash following the kit and the graphic's `none`, library
  round trip as a copy, slot rects and JSON, a short scene drawing the graphic in each slot with
  renders at the slot's size, agent `brand_pick` and slot errors), clippy on the new code,
  rustfmt, `tsc`, `vite build`. Not checked: a real render with a kit's font file and logo, the
  UI in the dev app.

**Built 2026-10-16, third pass: a library that grows from real work (uncommitted on `shorts`,
not run in the dev app yet).** George's aim: when an agent makes a graphic he likes, "keep it"
turns it into a general template in the app-level library for every later project. The model
generalizes it; the app checks it, with no model and no free-text rules.
- Gate (`graphics/gate.rs`). Checks on the files: no `http(s)`/protocol-relative references
  (XML namespaces aside), `background`/`text`/`accent` declared as colours (brand kits apply),
  a portrait block (warning), and text no `data-var-*` or script fills (warning). Then staged
  copies in `<runtime>/work/gate-<hash>/`, with the values baked into the declared defaults
  (`with_defaults`; `check`/`snapshot` take no variables file), three Chromes at a time:
  `hyperframes check --json --frame-check` at 1920x1080 for 3 s and 1080x1920 for 8 s, with its
  values and with long values (each line about twice as long, made of its own words, the same
  number of lines). `snapshot --describe false` (never to a vision model; `GEMINI_API_KEY`
  removed) at 0, ½, ¾ and the last frame. One more still pair per string variable set to another
  value.
- What fails: lint/runtime/contrast errors (warnings warn); any layout overflow or layout error,
  and a layout HyperFrames never measured (it skips the browser when lint has errors); a first
  frame more than 0.5% covered (alpha ≥ 96); nothing shown (under 2% at alpha ≥ 200) at ½ (and
  ¾ at 8 s); a last frame still covered, unless the root says `data-holds-end` (the end card
  holds to the end, fading in only); a string variable whose other value changes fewer than
  100 pixels; network; missing kit colours.
- Answer: pass/warn/fail per check with why, and a contact sheet (16:9 with its text and long
  text, 9:16 the same, over a checkerboard, labelled), kept in `<cache>/gate/<source hash>.json/
  .png` so an unchanged graphic answers at once; one gate at a time.
- Library (`library.rs`): `library.json` holds name (a slug for agents, unique, never a bundled
  template's), label, useWhen, default length, version, project (name, folder), created/updated
  dates, `general` (passed the gate) and the checks failed when saved as is. The gate's report
  and sheet are kept as `.check.json`/`.check.png` (dot files never travel into a project). An
  update replaces the item whole through a staged folder, as its next version. `closest()` finds
  a library item or bundled template close by label (word Jaccard ≥ 0.5) or variable ids (kit
  colours left out, Jaccard ≥ 0.75 with 2+ shared). `graphic_add library` takes an id or a name.
- Agents: `graphic_templatize` (source, what the app sees already, a close item, and the rules:
  text as variables, kit colours, timing from `data-duration`, viewport layout with a portrait
  block, a fading card or `data-holds-end`, no network); `graphic_check` (the gate, with the
  sheet as an image); `library_save` (gate → close match → save; stops at a close item until
  `update` or `asNew`; `asIs` only at the user's word; needs `useWhen`). `graphics` lists library
  items with label, useWhen, template/as is and version. SKILL.md: library first, offer to keep a
  graphic the user likes; tutorial step 6 prefers fitting library items and offers to keep one;
  server instructions say the same.
- UI: Inspector Save to library opens a form (label, use when) → Check and save, with progress;
  pass saves as a template; a close item offers "Update it (version n)" / "Save as new"; a
  failure lists why with the sheet and offers "Save as is", marked not general. Graphics page:
  each library item shows a Template / As is badge, its version, its use-when line, and its
  check's sheet on demand.
- Checked: `cargo test --lib`, the ignored `graphics_gate_real` against the installed runtime
  (every bundled template passes in 26-30 s, the end card through `data-holds-end`; a one-off
  graphic fails on network, kit colours, fitting 9:16, both lengths and an unshown variable; the
  second run answers from the cache), clippy clean on the new code, rustfmt, `tsc`, `vite build`.
  Not checked: the UI in the dev app, an agent's full templatize → check → save loop over MCP.

## Decisions (2026-10-13)

1. **The app runs HyperFrames renders.** The agent writes the files and calls `graphic_render`.
2. **WebM VP9 alpha is the default render cache** (decoded with `libvpx-vp9` forced, a keyframe every
   second). **ProRes 4444** for the macOS native path and for exporting a graphic to another editor.
3. **Fullscreen graphics hide the webcam bubble and the captions by default**, with a per-clip override.
4. **No typed text is ever captured.** (Amended 17–18 Oct: originally "no keystroke capture".)
   Keyboard telemetry v1 records typing stretches with a key count and shortcuts only; see
   [keyboard-telemetry.md](../contracts/keyboard-telemetry.md).
5. **The existing AI passes stay**: transcription (ElevenLabs or local Parakeet), fillers, retakes,
   chapters, shorts. The agent is a second way to produce the same suggestions over MCP, landing
   in the same review lists. Transcription stays in the app in any case, because a subscription
   agent can't turn audio into words.

## Inspiration (repos)

- browser-use/video-use: packed transcript, `timeline_view` PNGs, self-check on the render, one sub-agent per animation
- adukhan98/video-edit: brand intake → brand.json/brand.css, approval of a storyboard, ProRes 4444 graphics checked over footage, cuts that use the audio envelope
- kamgasimo/ai-video-editor: pinned HyperFrames, cut lists that keep their evidence, checks by re-transcribing, a measured voice/music margin, FCPXML out
- zhuyansen/awesome-claude-video-skills: ~180 repos rated for security
