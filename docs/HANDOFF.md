# Handoff for cloud sessions

Reconciled 8 November 2026. The original PC handoff recorded George's move to phone-driven
Claude Code cloud sessions and GitHub-app review; he cannot watch video or click the editor
there. This document owns machine-only context, standing decisions and questions for George.
Setup commands: [AGENTS.md](../AGENTS.md) and [README](../README.md#development).
Implementation: [Architecture](architecture.md). Work: [Roadmap](ROADMAP.md).

## Current inspected state

- Local checkout: `claude/cloud-prep`, HEAD `be7179c`; no tracked or untracked changes before
  this consolidation. These are local facts, not a fresh remote/PR status check.
- Faceless/publish code and assets are already committed in this ancestry. Old “not committed”
  labels were removed. The original handoff recorded `AI-Automation` and PR #57 as the base;
  their current remote status has not been queried.
- [gap.md](gap.md) is the canonical issue report: the P1s (GAP-004/008/011/016/017/037) were fixed
  on 8 Nov (with GAP-015) and every other finding on 9 Nov. GAP-030/031 got a first fix (phone
  text size check; reworded-restart flag tested on made-up sentences); GAP-032 was decided (keep). GAP-005/006 pass focused tests; GAP-001/014 describe
  replaced implementations.
- [Fresh focused results](verification.md#documentation-reconciliation-8-november-2026) and
  [remaining FCL/PUB acceptance](verification.md#faceless-and-publish-acceptance) are separate.
  No new full suite, native walkthrough, listening check, paid call or live upload was run.
- Suggested work order now starts with persistence/export correctness. STY-2 still needs George's portrait references
  and over-face verdict. The pending upload and voice checks are not completed by unit tests.

## Cloud setup and access

Run `bash scripts/cloud-setup.sh`, then `bash scripts/check-env.sh`, then the touched checks
or `bash scripts/test-all.sh`. The scripts and CI workflows are authoritative. A GUI is not
available in a headless session; use the library smoke export and frame snapshots.
`aeroedits do`/`--mcp-stdio` connect to a running app; `aeroedits render <project> --out x.mp4
--frames 6` exports a saved project with no app running (CL-8).

No API key is needed to build/test; unit tests keep using mocks. **Keys for cloud sessions
(George, 8 Nov):** `OPENROUTER_API_KEY` (the main one), `TREG_API_KEY` and `ZERNIO_API_KEY` are
set in the cloud environment, so sessions may make small live calls when a task needs a real
check (transcription, review, voice, story search). Keep them few and cheap, say in the PR what
was called and roughly what it cost, and never upload or publish through Zernio unless George
asks for that upload. Variable names and scratch overrides are in [.env.example](../.env.example);
it is a template, not a complete inventory of probe variables.

For a new cloud environment, configure its setup command as `bash scripts/cloud-setup.sh`
and connect this repository to the code-session provider. Required setup downloads include
Ubuntu apt mirrors, Rust/crates registries, npm and GitHub release/codeload hosts; package
registries alone may not cover apt/toolchain/sidecar setup. Optional runtime downloads use
`huggingface.co` and its model/CDN hosts, `nodejs.org`, `storage.googleapis.com`,
`files.freemusicarchive.org`, and NVIDIA for the Windows CUDA pack. This is an access checklist,
not a claim those hosts were tested or newly allowed. Never install into George's WSL distro
as an unrequested substitute for a fresh VM.

Linux GUI builds need WebKitGTK/GTK libraries plus built `front-end/dist`. Local LLM Linux
setup is not established; Parakeet compilation does not prove a downloaded model runs.
NVENC/CUDA/DirectML, native window/audio paths, installers, macOS Swift and real recordings
still need their target environment. Smoke can use lavapipe and libx264 in CI.

The old handoff recorded pushing the WIP and local-only branches and a clean secrets scan;
no new history-wide secret scan, push, branch deletion or PR action occurred here. The obsolete
cloud-prep PR-body file was removed after retaining its setup, evidence and questions here and
in Verification. Use Git history for its original wording.

## Only on the old machine

Locations recorded by the earlier 8 Nov handoff; availability and backups were not checked
again here. Recreate what matters in scratch. None is needed for the core fixture tests.

| What | Where | Why not pushed |
|---|---|---|
| Real recordings (EVAL-1 tutorial 22 Oct, takes 25 Oct, 16 Oct test, 11 Oct, 29 Oct) | `C:\Users\George\Documents\AeroShootRec\*.aero` | Private, large media |
| Projects and review renders (GFX-8 rerun shorts, etc.) | `C:\Users\George\Documents\AeroEdits\`, `Documents\AeroEdits reviews\` | Large media |
| App data: saved styles ("My library"), presets, looks, mcp.json, faceless stories, made list, watchlist, history | `%APPDATA%\AeroEdits\` | Per-user state, partly private |
| API keys: OpenRouter, ElevenLabs (rejected), Zernio (channel Aero), Treg | Windows Credential Manager, service "AeroEdits" | Secrets; re-enter in a new machine or cloud env |
| Claude Code memory notes (the source of the decisions below) | `~/.claude/projects/C--Users-George-Desktop-Automated-Editor/memory/` | Machine config; durable parts copied here |
| UI revamp mockups (style references, not specs) | `output/ui-review/` (7.4 MB) | Git-ignored images |
| GSAP before/after demo, caption frames, quality comparisons | `output/gsap-demo`, `output/quality`, `output/*.png` | Git-ignored images |
| Installer builds | `output/installer/` (103 MB) | Build output |
| Bundled FFmpeg sidecars | `src-tauri/binaries/` | Fetched by `fetch-ffmpeg.mjs` |
| Old Python explainer pipeline (ported into FCL) | `C:\Users\George\Desktop\Explainer-Video-Automation` | Separate repo/folder; check it is on GitHub |
| Re-voice lab clips | `Documents\AeroVoiceLab\review_clips` | Paused research, media |
| Eval runner scripts (`eval3/`, `eval4/`, `gfx8-rerun/`) | Old session scratch folders | Temp; recipe in [verification.md](verification.md#agent-edit-evaluation-eval-1) |

## Standing decisions and preferences

Carried over from the PC's memory notes. Treat as settled; don't re-ask.

**How George works**
- He is an AI engineer: go technical on agents, protocols, tool design and cost.
- He hates scrolling and cognitive overload: short answers, short agent text, said once.
- "Your call" means decide and record it as Claude's call. When he says "do all of them", do
  them and list the choices made without him ("my unasked choices") for review.
- Human in the loop: no autonomous loops, nothing starts on its own (no timers, folder watches,
  unattended modes). Every step starts from his click or his agent's call.
- Run `cargo fmt --check` and fix before every commit touching Rust.

**Product direction**
- Milestones: long-form tutorials → tutorial shorts → gaming shorts and videos (21 Oct).
  Faceless explainers (FCL) and publishing (PUB) were added 7 Nov and were the active work.
- App first: improve the app, exposing each feature as an MCP tool/action as it is built.
- MCP only (no built-in chat), via `aeroedits --mcp-stdio` and a per-user pipe/socket; no HTTP
  mode (it would need a token). HyperFrames only for graphics; GSAP vendored locally.
- The app is free and open source; licences must allow commercial use by its users (no
  non-commercial models or assets). Music must never need credit (CC0/public-domain only).
- Old graphic versions and folders are kept, even when nothing on the timeline uses them: no
  automatic cleanup (9 Nov, GAP-032).
- Graphic text must be at least 2.8% of the screen's short side, Apple's 11 pt minimum on a
  phone (9 Nov, GAP-030; George asked for the published minimum).
- Never migrate or stay compatible with old project files.
- Four saved things, one rule: Style (looks), Preset (what the app/agent does for a kind of
  video, incl. method and rules), Layout (where things sit), Project (only this video's changes).
- A choice the preset holds runs as set; the agent never asks about it. Layouts and webcam are
  raised by the agent only when the user brings them up.
- Agent correctness (right tools, right arguments, right workflow) beats token savings
  (24 Oct); cost still matters ("99% of the quality for 10% of the price").
- Graphics: one look for every graphic in a video (GFX-5, "very important"); default look is his
  light "app-like" library style (white, #1D1D1F text, #0071E3 accent, rounded cards, hairlines).
  Graphics are never chopped by cuts. Text cards and demonstration graphics are equals. No Accept
  card for graphics: apply as one undo step. References belong to the style, not the preset.
  Sounds on graphics only when asked.
- Shorts vs clips are different products: `shorts` = made from a fresh take, `clips` = cut from
  the long video; never one word for both. Finding clips only adds.
- Captions: off in long-form, on in shorts; never under a full-frame graphic.
- Language: Auto (default) / English / Greek; Greek is online only. Agents transcribe
  `provider: auto` and don't change app settings. Online review fillers are cut automatically.
- Faceless (7–8 Nov): writing, reviews and storyboard run in the user's agent; the app calls only
  paid media models with the user's keys. Separate graphics on the timeline. Zernio for upload,
  package and upload in the Export dialog for every video. Python ported to Rust. Voice through
  OpenRouter with voice "Will", `elevenlabs/eleven-v4-turbo`, 1.2x atempo. X + Reddit (not
  Bluesky/Threads), TikTok + Instagram for shorts, no connected Google/YouTube accounts in Treg,
  searches only on click, $0.10 guard per search.
- Zooms glide on the edited timeline; history up to 2 GB per project, no step cap; a checkpoint
  before every agent run.

## Ideas George turned down: never re-pitch

- Word/forced alignment ("alignment is trash"); anchored notes / notes lane / "Ask AI about this";
  a motion preview or MP4 with sound for the agent.
- Learning from his corrections (unless strictly data-driven and user-approved: LOOP-1), watch
  folders and batch runs, automatic on-screen secret hiding.
- OBS/MKV import or facecam detection for gaming (he records with AeroShoot).
- Bad-take hotkey via keys.jsonl, logging UI Automation element names, speed ramps instead of
  cuts, gaming highlight finder, recording pickup lines in-app, previewing cuts with handles,
  pacing heat strip, webcam colour correction.
- Cursor/bubble micro-polish: pointer auto-hide, click ring, motion blur on camera moves,
  notification blur, hiding the bubble during silences.
- Turning down mouth noises that touch words (his words: never mention again).
- Screen OCR (too heavy); selling features on caption accuracy (he doesn't care much).
- Colour grading / LUTs (not now); a demonstration-only graphics default; per-preset references.
- Re-voicing/dubbing his voice: paused 18 Oct, open models not good enough (accent, lip sync).
- Local Greek ASR (until a good model exists).

## Engineering gotchas learned the hard way

- `cargo fmt`, `sed -i` and Python rewrites turn CRLF files into LF on the Windows checkout;
  harmless with autocrlf but noisy. Heredocs in Git Bash ate backslashes (pipe names) and Rust
  escapes. Not an issue on Linux; use the Edit tool for Rust strings with escapes anyway.
- Parakeet words swallow pauses: run `transcript::pauses::tighten_loose_words` before trusting
  word timings as guards or boundaries.
- Never subscribe big React components to `currentTimeUs`; heavy Tauri commands must be
  `async` + `spawn_blocking`; the window thread must never wait on our locks (a preview deadlock
  was fixed this way, see architecture "Threads and locks").
- HyperFrames: a measured box needs `flex: none` (the root has no height when scripts run);
  don't tween `left`/`top`; variables apply after the script; the gate renders at 1.5 s and
  2.25 s of a 3 s placement, so everything must show by then; no `'` in single-quoted variable
  defaults; test with the style's real fonts. Graphics in a project are copies: template fixes
  don't reach old ones.
- Unit tests alone missed agent behaviour (agents call `preset tutorial` before `status`): when
  a change depends on what an agent reads, run a headless agent (`claude -p`) against a dev
  build. Recipe: [verification.md](verification.md#agent-edit-evaluation-eval-1).
- `aeroedits do <tool>` can't reach tools behind `more`: use `do more --tool X --args '{…}'`.
- The old Codex client needed per-tool write approval during editing runs. Treat this as a
  dated client observation, not a promise about every client; keep the user's authorization.
- Dev build speed: `lto = "off"` and line-table debug info in `[profile.dev]` (22–50 s one-file
  rebuilds); don't bring ThinLTO back. Linking with ONNX ran a 16 GB box out of memory: use
  `CARGO_BUILD_JOBS=1` for `--features parakeet` builds on small VMs.
- archive.org licence fields are often wrong; check the artist's own page before shipping music.

## Proposal: preview builds of the front end

So George can look at UI changes on his phone without the desktop app. Steps 1, 2 and 4 are
built (CL-10, 9 Nov): `npm run build:preview` runs the UI on canned answers that the real commands
wrote for the fixture project, the preview shows a real exported frame, and CI uploads phone and
desktop screenshots as `ui-screenshots`. Hosting (step 3) is not set up: no accounts were created.

1. Add a mock for `@tauri-apps/api`'s `invoke` and `listen` in `front-end/src/lib/ipc.ts`
   behind `import.meta.env.VITE_MOCK_IPC`: canned answers for `open_project`, `project_status`,
   transcript, timeline and export lists, from JSON in `front-end/mock/` (a small fixture project
   made from `test/fixtures`). The wgpu preview area shows a still frame PNG.
2. `npm run build -- --mode preview` produces a static site.
3. Host per PR on **Cloudflare Pages** (free, preview URL per branch, comment on the PR) or
   **GitHub Pages** via an Actions artifact (no new account, one URL, no per-PR previews).
   Cloudflare needs an account and an API token stored as a repo secret: George's call.
4. A Playwright job takes screenshots of the main screens at phone and desktop widths and
   uploads them as an artifact, so even without hosting the PR shows images.

Size: one session for the mock + build mode, one for hosting, one for screenshots.

## Questions for George

These do not block documentation maintenance or unrelated mocked fixes. Finding-specific
design questions stay beside their evidence in [gap.md](gap.md), not repeated here.

1. **Priority:** confirm P1 fixes → cloud tooling → remaining faceless work, with STY-2 once
   references arrive. Does the long-form → shorts → gaming order still hold after faceless?
2. **Integration branch:** after fixes, should the recorded `AI-Automation` work merge into
   main or stay on that branch? Check current PR state before acting; this run did not publish.
3. **Acceptance environment:** can George use a Windows/macOS machine later for listening,
   native UI/installer/GPU checks, a real-script voice run and the first private upload?
   Meanwhile, continue independently with mocks and clearly scoped fixture evidence.
4. **Private inputs:** are the tutorial/take recordings and projects backed up somewhere an
   authorized session can fetch? The private media was not inspected in this run.
5. **Cloud access:** keys are settled (OpenRouter mainly, Treg, Zernio). Still open: a spending
   limit per session, and whether model/runtime hosts (huggingface.co, nodejs.org,
   storage.googleapis.com) are allowed.
6. **Preview delivery:** choose Cloudflare Pages, GitHub Pages, or CI screenshot artifacts
   only (CL-10). Accounts/secrets are needed only for the chosen hosting path.
7. **Pending creative/setup checks:** supply STY-2 references and the over-face verdict,
   listen to the music/take matching, provide longer real Greek speech, and renew ElevenLabs
   only if its direct route is wanted. FIX-9 real-client transcription consent remains a check;
   the settled policy that online fillers apply directly is not reopened.

Old branch cleanup is optional repository administration, not a documentation or P1 blocker.
No branches should be deleted merely because the old handoff listed them.
