# Handoff: AI readiness, Phase 2 (agents propose, George reviews)

You are continuing George's AeroEdits work on branch `ai-readiness`. George is an AI engineer:
be technical, give him a recommendation rather than a survey, and discuss design choices with
him before building anything large. He prefers to agree on a plan first.

Read these first:
- `docs/plans/ai-readiness.md`: the whole plan, the decisions, and what each phase did.
- `src-tauri/src/mcp/` (mod.rs, tools.rs, ipc.rs, bridge.rs, clients.rs, frames.rs): the MCP
  server built in phases 0 and 1.
- `audit/README.md`: another session audited the tree and fixed findings on top of this work.

## Decisions already made (don't reopen them)

- AI comes only from the user's own agent (Claude app, Claude Code, Codex, Gemini CLI, Cursor)
  connecting over MCP. No chat panel and no model calls inside the app for this path.
- **Transport:** agents start `aeroedits.exe --mcp-stdio`, which reaches the running app over a
  named pipe that only George's Windows account can open (`\\.\pipe\aeroedits-mcp-<SID>`).
  There's no HTTP, no port and no token. Keep it that way; an HTTP mode would need a token again.
- **Graphics:** HyperFrames only. The app runs the renders. WebM VP9 with alpha is the default
  render cache, ProRes 4444 for macOS and for handing a graphic to another editor.
- **Fullscreen graphics** hide the webcam bubble and the captions by default.
- **No keystroke capture.**
- **The existing AI passes stay:** transcription (ElevenLabs or local Parakeet) and the
  OpenAI/OpenRouter filler, retake, chapter and shorts passes. The agent becomes a second
  source of the same suggestions, landing in the same review lists.
- Focus is tutorials and educational videos now, gaming later. The app is free and open source.

## Where things stand

- **Phase 0 (done):** transparent video decodes as BGRA (NV12 would drop the alpha), with
  `libvpx-vp9` forced for WebM. A transparent import gets the Overlay role. Checked in the
  preview (~60 fps) and in a 1080p export.
- **Phase 1 (done):** six read-only tools: project_info, transcript_get (packed lines with
  stable word ids `w-N`), timeline_get, playhead_get, frame_render (PNG), layout_get (layer
  rectangles in canvas pixels). File > AI agents… has the on/off switch, one-click
  Connect/Disconnect for the Claude app and Codex, copy text for other agents, the connected
  agents and an activity list.
- **Verified with a real client:** on 2026-10-13 George's Claude Code session (the desktop
  app's Code tab) had `mcp__aeroedits__*` tools and `playhead_get` answered from his open
  project. A Codex session hasn't been tried yet.
- **Not yet built from phase 1:** `telemetry_get` (clicks and scrolls on the edited
  timeline) and `timeline_view` (filmstrip, waveform and words for a range, as a PNG).
- **Nothing is committed.** The working tree also holds the audit session's fixes across
  transcript, sequence, playback and UI files, plus an untracked `audit/` folder. Ask
  George how he wants commits split before committing anything.

## Phase 2 scope

1. **Write tools that propose, never apply directly.** For example `propose_cuts` (word ids or
   edited ranges, plus a reason) and `propose_edit` (one or more `SequenceEdit`s, plus a reason).
   Every write tool takes `expected_revision` and returns a short diff and the new state. Mark
   them `read_only: false` in `tools.rs`, so `Host::project_changed` fires and both windows
   refresh through the `aeroedits-project-updated` event.
2. **A review panel** (not a chat) listing proposal batches with the agent's name and its
   reasons, with accept, reject and undo for each. An accepted batch is **one undo step**.
3. **An "autopilot" setting** that accepts batches automatically. It's off by default.
4. **The two missing read tools:** telemetry_get and timeline_view.
5. **`proposals_list`, `proposal_withdraw`, `undo_batch`** for the agent's own batches.

## Code to build on

- `project/revision.rs`:
  - `EditHistory::commit_next(expected_revision, root, next_document)` commits a whole new
    document as one undo step.
  - Apply several `sequence::edit::apply_reserving` calls to a clone, then commit once, to
    get a single undo step for a batch.
  - A wrong revision fails with "Stale edit revision".
- `sequence/edit.rs` `SequenceEdit`:
  - Tagged by `kind`.
  - Variants: addTrack, placeAsset, moveClips, trimClip, split, delete, deleteRange,
    rippleTrim, link, setClip, …
- `ripple_cuts` in revision.rs and reader.rs is how jump cuts and word cuts shorten the
  video.
- Transcript suggestions (`transcript/edit.rs`):
  - `TranscriptCutSuggestion` has `source: SuggestionSource { Rules, Ai }` and a `reason`.
  - AI spans are stored by word id in `Transcript.ai_suggestions` (`AiSpan`).
  - Agent cut proposals could reuse this, probably with a new `Agent` source, so they appear
    in the transcript review George already uses.
- Zoom suggestions show an accept/dismiss pattern:
  - `project_zoom_accept`, `project_zoom_dismiss`, `dismissed_zoom_ids`.
- Commands are thin wrappers over `commands::*_impl(&AppState, …)`. MCP tools call the same
  functions under the same locks (`command_lock`, then `opened_project`).

## Questions for George before building

- **Where proposals live:**
  - Option A: in memory, so they're lost when the app closes.
  - Option B: saved in the project (a `proposals` list in the edit document or beside it).
  - Recommend one.
- **Where the review panel goes:**
  - A dock panel.
  - Part of File > AI agents.
  - Inline on the timeline as ghost clips or striped ranges.
  - The audit and the UI mockups in `output/ui-review` (style references, not specs) may
    help with this choice.
- **Cut proposals:**
  - Do they go into the existing transcript suggestion list, a separate list, or both?

## How to work here (lessons from earlier sessions)

- **Line endings and backslashes:**
  - The working tree uses CRLF and Git Bash mangles backslashes.
  - Use the Write/Edit tools for source changes, not sed, Python or heredocs.
  - Check with `file <path>`.
  - Restore CRLF with `sed -i 's/\r\?$/\r/' <path>` if a shell rewrite slipped in.
  - Pipe names like `\\.\pipe\...` lost backslashes twice when written through the shell.
- **Builds:**
  - Don't run `cargo test` or `cargo build` while `cargo tauri dev` is building. They share
    `target/debug`, and the race produced a false "cannot find function run" error.
  - Stop the dev app before editing Rust; it rebuilds on every save.
- **Driving the dev app:** see the memory note "Driving the dev app".
  - The page is `http://127.0.0.1:1420/`.
  - Use DevTools on port 9222 with a separate `WEBVIEW2_USER_DATA_FOLDER`.
  - Keep each `Runtime.evaluate` call on one line.
  - The project handle is `useProjectStore.getState().openedProject.projectHandle`.
  - `sequence.tracks` lists video tracks bottom-first.
  - For webview screenshots use CDP `Page.captureScreenshot`. A window capture can grab
    another window when the dev window doesn't come to the front.
- **Testing MCP without touching George's settings:**
  - Use the official TypeScript MCP SDK client (`@modelcontextprotocol/sdk`, installed in a
    scratch folder) with `StdioClientTransport({ command: <exe>, args: ["--mcp-stdio"] })`.
  - Start the dev app with `CODEX_HOME=<scratch folder>` when trying Connect, so it writes to
    a scratch `config.toml`.
- **Projects:**
  - Test on copies of recordings from `C:\Users\George\Documents\AeroShootRec\*.aero`
    (delete `project.json` in the copy), never on originals.
  - Your own session may also have the `mcp__aeroedits__*` tools, connected to whatever
    George has open. Use them read-only. Once write tools exist, test them only on a scratch
    project.
- **Notes to keep:** record each phase's status and measurements in
  `docs/plans/ai-readiness.md` as you go, and update the project memory when decisions change.

## Done means

- The write tools, review panel, single-step undo per batch and the two read tools work.
- Unit tests cover them.
- An end-to-end test drives a proposal through the bridge and the pipe and accepts it.
- They're checked in the real dev app, with a screenshot of the review panel holding an
  agent's batch.
- The full test suite passes and `npx tsc --noEmit` is clean in `front-end`.
- The report says what wasn't verified (macOS, a real Codex app session).
