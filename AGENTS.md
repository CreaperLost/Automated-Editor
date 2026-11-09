# AeroEdits

AeroEdits is a desktop video editor (Tauri v2: Rust back end, React/Vite front end) for
screen-recorded tutorials made with George's own recorder (AeroShoot, `.aero` folders holding
screen, webcam, mic, system audio and mouse/keyboard telemetry). It cleans recordings up
(silence and filler cuts, mouth-noise muting, zooms from telemetry, layouts, captions, chapters),
makes shorts and clips, adds HyperFrames motion graphics, builds faceless explainer videos and
exports/uploads MP4s. It has no built-in chat: the user's own agent (Claude Code, Codex) drives
the app's passes over MCP or `aeroedits do`, and the user approves.

The owner, George, works only through Claude Code cloud sessions from his phone and reviews PRs
in the GitHub app. He cannot watch video or click the UI. Read [docs/HANDOFF.md](docs/HANDOFF.md)
first (state, standing decisions, ideas never to re-pitch, questions for him), then pick work
from [docs/ROADMAP.md](docs/ROADMAP.md).

## Editing a video with the `aeroedits` tools

If you are editing a video with the `aeroedits` tools (not working on the app), write nothing
here: no copies of graphics, drafts, plans or notes. Graphics go into the project through
`graphic_write_file` (`edits` to change part of a file); everything else stays in the project.

## Map

| Path | What |
|---|---|
| `src-tauri/src/lib.rs` | Tauri app setup and command registration (`tauri-app` feature) |
| `src-tauri/src/main.rs` | Binary: GUI, `--mcp-stdio` bridge, `aeroedits do <tool>` CLI |
| `src-tauri/src/commands/` | Every UI command as `*_impl(&AppState, …)`; MCP calls the same functions |
| `src-tauri/src/project/` | Project folder, reader, revisions, undo history, silence analysis |
| `src-tauri/src/sequence/` | Tracks/clips model, `SequenceEdit`, source↔timeline clocks |
| `src-tauri/src/media/` | FFmpeg decode/encode, audio mixer and polish; `native/macos` Swift bridges |
| `src-tauri/src/render/` | wgpu compositor (CPU path at parity), Windows window preview |
| `src-tauri/src/playback/`, `export/` | Preview engine; export jobs, delivery queue |
| `src-tauri/src/dsp/`, `transcript/`, `ai/` | Silence/noise DSP; transcripts (Parakeet, Scribe, OpenRouter); cloud/local review |
| `src-tauri/src/mcp/` | MCP server, tools, CLI, skill text (`mcp/skill/*.md`) |
| `src-tauri/src/preset/`, `style/` | Presets (what to do, `preset/methods/*.md`) and styles (how it looks) |
| `src-tauri/src/graphics/` | HyperFrames graphics: runtime download, templates, render gate |
| `src-tauri/src/faceless/`, `publish/` | Faceless explainer pipeline (FCL), package + Zernio upload (PUB) |
| `src-tauri/src/smoke.rs` | Headless end-to-end smoke tests: silence cut, mouth-noise mute + loudness, portrait captions → `aeroedits render` → ffprobe |
| `src-tauri/assets/` | Fonts, sounds, image library (compiled into the binary) |
| `front-end/src/` | React UI; `lib/ipc.ts` wraps every backend command; zustand stores |
| `test/fixtures/` | 6 s screen/voice/clip media (`scripts/make-fixtures.sh` regenerates) |
| `docs/` | HANDOFF, ROADMAP, product, architecture, verification, audit, gap (8 Nov review) |

## Setup and commands (Ubuntu, confirmed in CI)

```bash
bash scripts/cloud-setup.sh      # apt deps, Rust from rust-toolchain.toml, Node 22, npm ci, cargo fetch
bash scripts/check-env.sh        # tool versions + builds the core + renders the smoke export
bash scripts/test-all.sh         # everything below; --quick skips the GPU set and front-end build
```

| Check | Command |
|---|---|
| Rust format (gate) | `cd src-tauri && cargo fmt --check` (run `cargo fmt` before every commit touching Rust) |
| Rust tests | `cd src-tauri && cargo test` (~800 tests; media tests need FFmpeg) |
| GPU compositor/export tests | `cd src-tauri && cargo test --lib gpu -- --include-ignored` (software Vulkan via `mesa-vulkan-drivers`) |
| Smoke test | `cd src-tauri && AEROEDITS_SMOKE_OUT=/tmp/smoke.mp4 AEROEDITS_SMOKE_DIR=/tmp/smoke cargo test --lib smoke -- --nocapture` (the other smoke exports go to the folder) |
| Look at an output | `bash scripts/snapshot-frames.sh /tmp/smoke.mp4 6` then Read the PNGs / `contact-sheet.png` |
| Render a project, no app | `cd src-tauri && cargo run --bin aeroedits -- render <project> --out /tmp/x.mp4 --frames 6` (prints the MP4, stills and contact sheet) |
| Front-end | `cd front-end && npx tsc --noEmit && npm run test:cleanup && npm run build` (no ESLint configured; tests are node scripts, not vitest) |
| Look at the UI | `cd front-end && npm run build:preview && npm run screenshots` then Read `preview-screenshots/*.png` (canned backend; a new command needs an answer: `cargo test --lib preview_mock -- --ignored` rewrites `src/mock/backend.json`, odd ones go in `src/mock/backend.ts`) |
| Clippy | `cd src-tauri && cargo clippy --lib` — about 60 warnings today, so not a gate |
| Desktop shell compiles | `cd src-tauri && cargo check --features tauri-app` (needs `front-end/dist`, i.e. `npm run build`, and the WebKitGTK libs) |
| Doc links | `python3 scripts/check_doc_links.py --all` |

You cannot run the GUI in a cloud session. The smoke test plus frame snapshots are how you
show that a pipeline change works; describe what the frames show in the PR.

## Conventions

- Rust 1.92 (pinned), edition 2021; `cargo fmt` is law. Tests live beside the code in
  `#[cfg(test)] mod tests`; probes on real recordings are `#[ignore]` and read env vars.
- New UI commands: write `commands::*_impl(&AppState, …)`, wrap it thinly in `lib.rs`, add the
  call to `front-end/src/lib/ipc.ts`. Heavy commands are `async` + `spawn_blocking`.
- Never migrate or stay compatible with old project files: drop old fields freely.
- New look settings go in `style::Style`, behaviour settings in a preset part. Never add a new
  saved-settings store or call user settings "template".
- Agent-facing text (`mcp/skill/*.md`, `preset/methods/*.md`, tool descriptions) is short and
  said once; correctness of tool calls and arguments beats token savings. `the_tool_list_stays_small`
  guards the tool list size.
- Docs: update the document that owns the fact (see `docs/README.md`); keep work IDs stable.
- Prose style in docs and UI: plain short sentences, no marketing words.
- Line endings: the Windows checkout is CRLF; `.sh` files are LF (`.gitattributes`).

## Rules for cloud sessions

- Always work on a new branch and open a PR. Never push to `main`, never force-push, never
  rewrite history. One focused change per PR; no drive-by refactors.
- Run `bash scripts/test-all.sh` (or at least the parts your change touches) before finishing,
  and say in the PR what you ran and what passed. CI runs the same on every PR.
- PR description: **what changed**, **how it was verified** (commands and results, the smoke
  test's summary line), and **what George should check by eye later** (which screen or output,
  with the frame snapshots you looked at described in words).
- Don't commit secrets, `.env`, build output or media over a few MB. Test media is made by
  `scripts/make-fixtures.sh`.
- Tests mock the HTTP server as the existing tests do. The cloud environment has
  `OPENROUTER_API_KEY` (main), `TREG_API_KEY` and `ZERNIO_API_KEY`: small live calls are fine
  when a task needs a real check; list them and their rough cost in the PR. Never upload or
  publish through Zernio unless George asks for that upload. No ElevenLabs key.
- Ask nothing you can decide: make the sensible choice, list it in the PR under "Choices I made".
  If a decision only George can make blocks you, write it as a question in the PR.

## Known limits in the cloud VM

- No GUI, no window preview (the Windows preview draws with wgpu under a transparent WebView2),
  no audio playback. UI changes are type-checked, unit-tested, built and screenshotted with a
  canned backend (`npm run screenshots`).
- No real GPU: wgpu runs on lavapipe (slow, fine for tiny tests). No NVENC/AMF/QSV encoders;
  exports use libx264.
- Local models (Parakeet, YuNet, Qwen via llama.cpp/Vulkan, CUDA pack) download from
  huggingface.co / NVIDIA on first use and are behind the `parakeet` / `local-llm` features; the
  cloud network allowlist normally blocks those hosts. `cargo check --features parakeet` works.
- The graphics runtime (Node + HyperFrames + chrome-headless-shell) downloads from nodejs.org,
  registry.npmjs.org and storage.googleapis.com; `graphics_gate_real` needs it (install:
  `cargo test --lib graphics_runtime_installs -- --ignored`; `GATE_SHEETS=<dir>` keeps the sheets).
  If npm fails with "error sending request", run with `NO_PROXY=localhost` (the cloud proxy
  list sends npm direct, which Rust's client can't reach). CI runs it in `graphics-gate.yml`.
- Windows-only: installer (`scripts/build-windows.ps1`), MCP named pipe, WASAPI audio, CUDA
  pack, keychain. macOS-only: Swift bridges. Real recordings live only on George's old disk.
