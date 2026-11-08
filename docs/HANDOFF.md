# Handoff: from the Windows machine to cloud sessions

Written 8 Nov 2026 by an automated Claude Code run on George's Windows PC, the day before he
loses access to it. From now on all work happens in Claude Code cloud sessions (fresh Ubuntu VM,
repo cloned, package registries only) started from his phone; he reviews PRs in the GitHub app
and cannot watch video or click the UI.

This file holds what existed only on that machine (memory notes, local state) plus an audit of
the repo for headless work. Setup and rules for sessions are in [AGENTS.md](../AGENTS.md);
work to pick from is in [ROADMAP.md](ROADMAP.md).

Contents: [State](#state-on-8-nov-2026) · [Git](#git-state) · [Audit](#audit) ·
[Tests](#tests-and-results) · [Only on the old machine](#only-on-the-old-machine) ·
[Standing decisions](#standing-decisions-and-preferences) · [Never re-pitch](#ideas-george-turned-down-never-re-pitch) ·
[Gotchas](#engineering-gotchas-learned-the-hard-way) · [Preview builds](#proposal-preview-builds-of-the-front-end) ·
[Questions](#questions-for-george)

## State on 8 Nov 2026

- **Branch `AI-Automation`** (PR [#57](https://github.com/CreaperLost/Automated-Editor/pull/57) into
  `main`, open) holds everything up to AUTO-3 plus, as of today, two WIP commits with the
  faceless + publish work that was uncommitted on the PC: FCL-0..4, FCL-5 (Broadsheet style,
  four templates, image library of 509 pictures), FCL-6, FCL-8, PUB-1, PUB-2, Treg research,
  watchlist/demand. Status of each item is in [ROADMAP.md](ROADMAP.md#fcl).
- **It builds and its tests pass on Windows:** `cargo test` 787 passed, 0 failed, 54 ignored;
  front-end `tsc --noEmit` clean, `npm run test:cleanup` 52/52 (run 8 Nov before committing).
  Linux results: the Cloud check workflow on `claude/cloud-prep` (see the PR).
- **Not tried in the running app:** most of FCL-3..6 and PUB-1/2 were unit-tested only. No
  live Zernio upload has been made (George was to do the first private upload himself from the
  Export dialog). FCL-4 voice has not run on a real script.
- **Known bugs:** [gap.md](gap.md) is a code review of that work dated 8 Nov: 4 P1 findings
  (GAP-004 voice regenerate keeps old audio, GAP-008 old narration certified against a new script,
  GAP-011 publish commands not bound to the dialog's project, GAP-016 upload doesn't await
  autosave, GAP-017 restoring a cut can insert the wrong source audio) and ~20 P2. It also says
  two regression guards fail (GAP-005 style store, GAP-006 tool-list budget); both passed in the
  8 Nov run, so that part looks stale. Fix the P1s before merging #57.
- **Next on the roadmap** before the faceless work: STY-2 step 4 (shorts style from George's
  references; blocked on references he has to supply).

## Git state

Done on 8 Nov before this branch:

- The uncommitted work on `AI-Automation` was committed as `WIP: image library, image looks,
  fonts and graphic templates (FCL)` and `WIP: faceless + publish (FCL/PUB)` and pushed.
  563 new files, 28 MB, the largest a 436 KB SVG; nothing secret or over 20 MB.
- Branches that existed only locally were pushed: `claude/clip-speed` (one commit, "Clip speed on
  timeline clips", not on any remote before), `claude/clip-speed-port-db0455`,
  `claude/layouts-970278`, `dep-2-review`. All other local branches were already pushed and
  level with their remotes. No stashes existed. No other worktrees.
- Secrets scan of the working tree and all history (`sk-`, `sk-ant-`, `AKIA`, `ghp_`,
  `github_pat_`, `xox?-`, private keys, `AIza`, `api_key=`/`token=`/`Bearer` literals, `.env`
  files): **nothing of George's.** The only hits are public star-history badge `sealed_token`s in
  third-party READMEs copied into old research notes (commit 463dc94), which are not credentials.
  Keys live in the Windows keychain, never in files.

## Audit

### What the app is

A Tauri v2 desktop editor. `src-tauri/` is one Rust crate (`aeroedits_lib` + the `aeroedits`
binary); `front-end/` is React 19 + Vite 8 + TypeScript + Tailwind 3, zustand stores, dockview
workspaces (Edit, Cleanup, Shorts, Clips, Faceless). Every Tauri command is a thin wrapper over
`commands::*_impl(&AppState, …)`; the MCP server calls the same functions under the same locks.
Details: [architecture.md](architecture.md).

**Entry points.** `src-tauri/src/main.rs`: no args → GUI (`--features tauri-app`; without the
feature it prints a "headless mode" line and exits); `--mcp-stdio` → MCP bridge to the running
app; `do <tool> …` → CLI over the same MCP connection (needs the app running). There is no
standalone headless render CLI; the headless path is the library API, as `src/smoke.rs` uses it.

**Pipeline, input → output.**
1. *Input:* an AeroShoot `.aero` folder (`manifest.json`, `journal.jsonl`, rolling ~55 s
   segments in `media/screen|webcam|mic|system`, `telemetry/*.jsonl`) or imported files.
   `project::folder::create_project_folder` makes a project that mounts it;
   `commands::open_project_impl` opens it (assets, streams, sequence of tracks/clips).
2. *Analysis:* silence (`project/silence.rs`, `dsp/silence.rs`), transcripts
   (`transcript/`: Parakeet local, ElevenLabs Scribe or OpenRouter online), fillers/retakes
   (`transcript/edit.rs` rules, `ai/` reviews), mouth noises (`dsp/noises.rs`), zooms from
   telemetry (`zoom/`), layouts (`layouts/`), faces (`face/`, YuNet).
3. *Edit:* `SequenceEdit`s (`sequence/edit.rs`, magnetic ripple) committed as revisions with
   on-disk undo history (`project/history.rs`); everything time-based lives in source time and
   maps through `sequence/clock.rs`.
4. *Render/export:* `playback/` evaluates scenes; `render/` composites in wgpu (CPU path kept at
   parity); `media/ffmpeg.rs` decodes and encodes (libx264 or NVENC/AMF/QSV/MF on Windows,
   AVFoundation on macOS); `media/audio.rs` mixes; `export/` runs jobs and keeps a delivery list.
   Graphics are HyperFrames HTML rendered by a downloaded Node + Chrome runtime (`graphics/`).
5. *Publish:* `publish/` (package, thumbnail, Zernio upload with a journal).

**Automation / agent logic:** `src-tauri/src/mcp/` (tools, actions, CLI, bridge, skill text in
`mcp/skill/*.md`), presets with agent methods in `src-tauri/src/preset/methods/*.md`,
`mcp/first_steps.rs` (Preset → Run), proposals (`proposals.rs`), the faceless tools
(`mcp/faceless.rs`), publish (`mcp/publish.rs`).

### Commands on a clean Ubuntu VM

| Command | Result |
|---|---|
| `bash scripts/cloud-setup.sh` | Installs apt deps (FFmpeg, lavapipe, Tauri libs), Rust 1.92.0, checks Node 22, `npm ci`, `cargo fetch` |
| `bash scripts/check-env.sh` | Versions + builds the lib + runs the smoke test |
| `bash scripts/test-all.sh` | fmt, cargo test, GPU tests on lavapipe, smoke, front-end tsc/tests/build, doc links; clippy reported |
| `cargo check --features tauri-app` | Works on Linux with the Tauri libs and `front-end/dist` (ci.yml job "Tauri app build check") |
| `cargo check --features parakeet` | Compiles (ci.yml); running Parakeet needs a model from huggingface.co |
| `--features local-llm` | Needs CMake, Vulkan SDK, libclang; not set up for Linux. Windows build script only |
| `cargo tauri dev` / any GUI | Impossible in a cloud session |
| `scripts/build-linux.sh` | Builds .deb/AppImage; needs `cargo tauri` CLI and GitHub (BtbN FFmpeg build); untested here |

Things the repo needs that aren't in it: FFmpeg (apt in the VM; Windows/Linux installers fetch a
pinned BtbN build via `src-tauri/scripts/fetch-ffmpeg.mjs`), a Vulkan driver for wgpu (lavapipe
in the VM), model files and the graphics runtime (downloaded at run time, see below).

### Machine dependencies a clean Linux VM lacks

- **System tools:** ffmpeg/ffprobe with libx264, Mesa lavapipe (`mesa-vulkan-drivers`), build
  tools, WebKitGTK 4.1/GTK 3/librsvg/appindicator/xdo/patchelf for the Tauri shell. All installed
  by `cloud-setup.sh`. Python 3 only for `scripts/check_doc_links.py` and
  `src-tauri/scripts/add_images.py` (Pillow) and the old eval scripts.
- **Models, downloaded on first use into the app's data folder:** Parakeet Unified EN and TDT v3
  ONNX (huggingface.co, pinned commits, SHA-256), YuNet face model (huggingface.co/opencv), Qwen3.5
  4B GGUF (huggingface.co/unsloth), the CUDA 13 pack (developer.download.nvidia.com, Windows).
- **Graphics runtime:** Node 22 (nodejs.org), HyperFrames 0.8.228 and GSAP (registry.npmjs.org),
  chrome-headless-shell (storage.googleapis.com). Needed by `graphics_gate_real` and any
  graphic render.
- **Music:** built-in tracks download from files.freemusicarchive.org (SHA-256 pinned).
- **OS-specific code**, all behind `cfg`: Windows named-pipe MCP transport with a DACL, WASAPI
  audio (cpal), keychain (keyring), window preview drawn by wgpu under a transparent WebView2
  (`render/present.rs`), CUDA pack loading, `rust-lld` linker config
  (`src-tauri/.cargo/config.toml`, Windows target only); macOS Swift bridges (`native/macos`).
  Linux: keys in owner-only files, MCP over a Unix socket path like macOS (not exercised).
- **Hardware:** NVENC/AMF/QSV encoders, CUDA, real GPU preview: Windows PC only (RTX 3060 12 GB).
- **Hard-coded paths:** none in source code. `C:\Users\George\…` appears only in docs and eval
  data (`docs/plans/review-eval/*.json` name recordings on his disk) and the old recovery ZIP note.
- **Local services:** none besides the app itself (the MCP server lives inside the running app).

### Secrets, env vars and external hosts

Every variable is in [.env.example](../.env.example). Keys: `OPENROUTER_API_KEY`,
`ELEVENLABS_API_KEY`, `OPENAI_API_KEY`, `ZERNIO_API_KEY`, `TREG_API_KEY`; each overrides the
keychain/file key. None is needed for tests: cloud-service tests use a local fake server.
Runtime hosts: api.elevenlabs.io, openrouter.ai, api.openai.com, zernio.com, treg.to,
huggingface.co, nodejs.org, registry.npmjs.org, storage.googleapis.com, cdn.jsdelivr.net (rewritten
to local files by the graphics gate), files.freemusicarchive.org, developer.download.nvidia.com,
github.com (BtbN FFmpeg builds), plus ~60 news/RSS hosts the faceless discovery reads
(`src-tauri/src/faceless/aero-ai-news.json`).

### Tests

- Rust: ~840 unit tests in `#[cfg(test)]` modules (787 run, 54 ignored on Windows). Ignored ones
  are probes on real recordings (env-driven), the real graphics gate (needs the runtime, ~10 min)
  and GPU tests that CI runs with `--include-ignored` on lavapipe.
- New: `src/smoke.rs`, the end-to-end smoke test on `test/fixtures` (6 s recording → silence
  pass finds the 2 s gap → ripple cut → 320x180 export → ffprobe checks duration ±0.15 s,
  codec, size, frames ±3, sound). On Windows: 6.000 s → cut 1.890 s → 4.133 s, 124 frames. The
  test pattern's burned-in clock in the snapshot frames jumps from 1.733 s to 4.300 s at the cut.
- Front-end: `front-end/tests/*.mjs` node scripts via `npm run test:cleanup` (52). They test pure
  logic, not React rendering (GAP-024). No ESLint config exists.
- CI: `.github/workflows/ci.yml` (path-filtered: front-end typecheck, Linux/Windows/macOS Rust
  tests, Parakeet check, Tauri shell check), `cloud-check.yml` (new: setup script + test-all on
  every PR and on pushes to `claude/**`), `package.yml` (installers), `cache-warm.yml`.
- Clippy: about 60 warnings on the lib (8 Nov), so it is reported, not gated.

### Large files

No Git LFS. The biggest tracked content is the image library (`src-tauri/assets/images`, 25 MB of
WebP/SVG, compiled into the binary by `build.rs`; the repo is now its only copy) and the fonts.
`test/fixtures` is 0.9 MB. Real recordings were never in the repo.

### Instruction files

- `AGENTS.md` rewritten for cloud sessions (kept the rule that an agent editing a video writes
  nothing into the source tree). `CLAUDE.md` added as `@AGENTS.md` so Claude Code loads it.
- `README.md` is a long feature list for users; its build notes are Windows/macOS-centred. Left
  as is; AGENTS.md is the developer entry.
- `docs/README.md` pointed at a recovery ZIP on the PC and a Windows venv; both noted.
- `docs/roadmap.md` renamed to `docs/ROADMAP.md`; cloud-sized tasks added at the top.
- `.claude/settings.json` is empty; `.claude/launch.json` holds dev-server entries (one uses the
  Windows `.venv`).
- Contradictions found: `docs/gap.md` says two tests fail, they pass; the roadmap's FCL items say
  "not committed", they are now committed as WIP.

## Only on the old machine

Not pushed, and why. Ask a session to recreate what matters; nothing here is needed to build.

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
| AeroShoot recorder (writes `.aero`, keys.jsonl) | `C:\Users\George\Desktop\AeroShoot.AI` | Separate repo; check it is pushed |
| Re-voice lab clips | `Documents\AeroVoiceLab\review_clips` | Paused research, media |
| Pre-cleanup docs ZIP | `C:/Users/George/.codex/visualizations/2026/11/01/.../docs-before-cleanup.zip` | Use Git history instead |
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
- Codex `exec` refuses non-read-only MCP tools unless each has `approval_mode="approve"`.
- Dev build speed: `lto = "off"` and line-table debug info in `[profile.dev]` (22–50 s one-file
  rebuilds); don't bring ThinLTO back. Linking with ONNX ran a 16 GB box out of memory: use
  `CARGO_BUILD_JOBS=1` for `--features parakeet` builds on small VMs.
- archive.org licence fields are often wrong; check the artist's own page before shipping music.

## Proposal: preview builds of the front end

So George can look at UI changes on his phone without the desktop app. Not set up (no accounts
were created).

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

1. **Priorities:** is the order in [ROADMAP.md](ROADMAP.md#cloud-sessions-start-here) right? It
   is my guess: fix the P1 bugs in the faceless/publish work first, then the cloud tooling, then
   FCL-7/9, then STY-2 when you have references. Do you still want long-form → shorts → gaming
   after faceless?
2. **PR #57 (AI-Automation → main):** merge after the P1 fixes, or keep `AI-Automation` as the
   long-lived branch? This prep PR targets `AI-Automation`.
3. **Half-done work:** FCL-4 voice, PUB-2 upload and FCL-6 visuals were never run for real. Do
   you want a cloud session to keep building on them blind (unit tests only), or wait until
   you can run them somewhere?
4. **Keys in the cloud:** which keys (OpenRouter, Zernio, Treg, ElevenLabs) may a cloud session
   use, and with what spending limit? Default: none; everything is mocked.
5. **Network:** may cloud sessions reach huggingface.co, nodejs.org, storage.googleapis.com
   (models and the graphics runtime)? That unlocks the real graphics gate and face/ASR probes.
6. **A machine with a GPU/GUI later?** Is there any Windows/macOS machine you will have, even
   occasionally? Otherwise Windows installers come only from `package.yml` in CI and nothing is
   ever looked at in the running app.
7. **Recordings:** are any test recordings or projects backed up somewhere a session could get
   (Drive, a release asset)? Without them EVAL-1 and the real-data probes can't run.
8. **Other repos:** are `AeroShoot.AI` and `Explainer-Video-Automation` pushed to GitHub?
9. **Branches to clean up:** many old branches exist (`expansion`, `improvements`,
   `improving-ai-style`, `Research-AI-readiness`, `short-form-content-creation`,
   `Improving-shorts`, `claude/*`). Delete the merged ones? Nothing was deleted.
10. **gap.md P1s:** each has open questions inside (for example how a regenerated narration
    should replace the old clip). Answer them in the PR or let sessions pick the safe option?
11. **Preview builds:** set up Cloudflare Pages (needs your account) or GitHub Pages, or skip?
12. **Still open from before:** your verdict on the over-face panel (it now raises `check`
    warnings), `apply: true` for cloud fillers, the music listening pass, a longer Greek
    recording, a new ElevenLabs key, STY-2 shorts references.
