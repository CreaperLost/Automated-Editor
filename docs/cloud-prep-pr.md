# PR description: Prepare repo for cloud development

The automated run on 8 Nov could not open this PR (gh was not signed in on the PC). Open it from
`claude/cloud-prep` into `AI-Automation` with the text below, then delete this file in that PR.

---

## ROTATE THESE

Nothing to rotate. Scanned the working tree and every ref's history for `sk-`, `sk-ant-`, `AKIA`, `ghp_`, `github_pat_`, `xox?-`, private keys, `AIza`, and `api_key=` / `token=` / `Bearer` literals, plus any `.env` files. The only hits were public star-history badge `sealed_token`s in third-party READMEs copied into old research notes (commit 463dc94). Those are not credentials. Your keys live in the Windows keychain and never appear in files.

## Summary

This gets the repo ready for Claude Code cloud sessions started from your phone. A session that knows nothing can now set up a fresh Ubuntu VM, build, and run the real pipeline headless (recording → silence cut → export → ffprobe). It can also look at frames of the output and follow written rules. The base is `AI-Automation`: it is 4 commits ahead of `main` and still unmerged in #57, and it now includes your uncommitted faceless/publish work.

Before this branch:
- Your uncommitted work on `AI-Automation` (FCL/PUB, image library, fonts, templates: 563 new files, 28 MB) was committed as two WIP commits and pushed.
- Local-only branches were pushed: `claude/clip-speed` (one commit that was on no remote), `claude/clip-speed-port-db0455`, `claude/layouts-970278`, `dep-2-review`.
- There were no stashes.

## Audit highlights (full version in docs/HANDOFF.md)

- On Windows, the WIP builds and passes: `cargo test` 787 passed, 0 failed, 54 ignored. Front-end tsc is clean and 52/52 tests pass.
- The crate already built and tested on Linux in CI. Windows/macOS code is behind `cfg`. There are no hardcoded `C:\Users\George` paths in source code.
- There was no headless render CLI, and `aeroedits do` needs the running app. The library API is enough for headless work, so the smoke test uses it.
- `docs/gap.md` (8 Nov review) lists 4 P1 bugs in the faceless/publish work. They should be fixed before #57 merges. It also says two tests fail; both pass.
- Clippy reports about 60 warnings, so it is reported but doesn't fail the build. There is no ESLint config.
- Things that can't run in the cloud: the GUI and preview, the real GPU and NVENC, local models (huggingface.co), and the graphics runtime (nodejs.org, storage.googleapis.com) unless you allow those hosts.

## What changed, by commit

1. **Fixtures + smoke test.** `test/fixtures/` holds a 6 s screen, voice (tone / 2 s silence / tone) and clip, 904 KB, made by `scripts/make-fixtures.sh`. `src-tauri/src/smoke.rs` builds an `.aero` recording from them, opens it, runs the silence pass, applies the ripple cut, exports, and checks the result with ffprobe. `.gitattributes` keeps `.sh` files LF.
2. **Scripts:** `scripts/cloud-setup.sh` (idempotent: apt packages, Rust from `rust-toolchain.toml`, Node 22, `npm ci`, `cargo fetch`), `scripts/check-env.sh`, `scripts/snapshot-frames.sh` (N PNGs plus a contact sheet), `scripts/test-all.sh` (one command for every check).
3. **Docs and config.** `AGENTS.md` is rewritten for cloud sessions and keeps the "an agent editing a video writes nothing here" rule. `CLAUDE.md` is `@AGENTS.md`. Also added: `.env.example`, a `.gitignore` exception for it, and the `docs/roadmap.md` → `docs/ROADMAP.md` rename (git mv, links updated). New workflow `.github/workflows/cloud-check.yml` runs `cloud-setup.sh` and then `test-all.sh` on every PR and on pushes to `claude/**`, and uploads the smoke MP4 and its frames.
4. **Cloud check also runs on `claude/**` pushes**, so a session gets feedback before it opens a PR.
5. **`docs/HANDOFF.md`** covers state, git, the audit, what stays only on the old machine, decisions and preferences from the memory notes, ideas you turned down (never to re-pitch), gotchas, a proposal for preview builds, and questions for you. **`docs/ROADMAP.md`** now starts with CL-1..CL-15: one-session tasks with acceptance criteria a session can check headless.

## How it was verified

- On Windows (real GPU), the smoke test passes: `6.000 s recording -> cut 1.890 s of silence -> 4.133 s, 124 frames`, h264 320x180 with an AAC track.
- I looked at the frame snapshots. testsrc's burned-in clock reads 0.367, 1.033, 1.733 and then jumps to 4.300, 4.567, 5.667. So the 2 s silence (from ~2.05 s to ~3.94 s) really was cut, and the screen sits inside the default style's padded frame on its green wallpaper.
- The **Cloud check** run on a fresh `ubuntu-24.04` runner used only the scripts: **green** (run 42442268439: rustfmt, cargo test 383 s, GPU tests on lavapipe, smoke `6.000 s -> cut 1.890 s -> 4.133 s, 124 frames`, front-end typecheck/tests/build, doc links). The first run failed only because HANDOFF.md hadn't been pushed yet, so the doc-link check failed. https://github.com/CreaperLost/Automated-Editor/actions?query=branch%3Aclaude%2Fcloud-prep
- `scripts/check_doc_links.py --all` reports 0 broken links.
- I did not do a local WSL cold start: it would have installed packages into your WSL distro as root, and the CI job is the same fresh-VM test.

## Code changes needing your review

- `src-tauri/src/smoke.rs` is new and test-only (`#[cfg(test)] mod smoke;` in `lib.rs`). It skips with a message when FFmpeg or a wgpu adapter is missing, unless `AEROEDITS_REQUIRE_FFMPEG` / `AEROEDITS_REQUIRE_GPU` are set.
- Two doc-comment link fixes in `faceless/mod.rs` and `mcp/faceless.rs` (`roadmap.md` → `ROADMAP.md`).
- No app behaviour changed.

## What to check by eye later

- Open the `smoke-output` artifact of the Cloud check run on your phone and look at `frames/contact-sheet.png`. You should see six test-pattern frames, and the clock in the corner should jump from about 1.7 s to about 4.3 s between the third and fourth frame.
- Read `AGENTS.md` and the "Standing decisions" and "Never re-pitch" sections of `docs/HANDOFF.md`. Correct anything I carried over wrong from the memory notes.

## Questions for George

1. Is the order of tasks in ROADMAP "Cloud sessions: start here" right? It's my guess: the P1 bug fixes, then tooling, then FCL-7/9, then STY-2 once you have references. Does long-form → shorts → gaming still hold after faceless?
2. Should #57 (AI-Automation → main) merge after the P1 fixes, or should `AI-Automation` stay the long-lived branch?
3. FCL-4 voice, PUB-2 upload and FCL-6 visuals have never run for real. Should sessions keep building on them blind, or wait?
4. Which API keys may a cloud session use, and with what spending limit? The default is none, with everything mocked.
5. May cloud sessions reach huggingface.co, nodejs.org and storage.googleapis.com? That unlocks the graphics gate and the face/ASR probes.
6. Will you have any Windows/macOS machine with a GPU, even occasionally?
7. Are the test recordings backed up anywhere a session could fetch them?
8. Are the `AeroShoot.AI` and `Explainer-Video-Automation` repos pushed to GitHub?
9. Should old merged branches be deleted? I deleted nothing.
10. The gap.md P1 entries have open questions inside. Will you answer them, or should sessions pick the safe option?
11. Preview builds: Cloudflare Pages (needs your account), GitHub Pages, or skip?
12. Still open from before: the over-face panel verdict, `apply: true` for cloud fillers, listening to the music, a longer Greek recording, a new ElevenLabs key, and the STY-2 references.

## Final checklist for George

- [ ] **Cloud environment setup script** (claude.ai/code → environment settings): `bash scripts/cloud-setup.sh`
- [ ] **Env vars:** none are required. Optional: `AEROEDITS_REQUIRE_FFMPEG=1`. API keys only if you answer question 4 (names are in `.env.example`).
- [ ] **Network:** "package registries" is not enough on its own. Allow `archive.ubuntu.com`, `security.ubuntu.com`, `static.rust-lang.org`, `sh.rustup.rs`, `crates.io`, `index.crates.io`, `static.crates.io`, `registry.npmjs.org`, `github.com`, `objects.githubusercontent.com`, `codeload.github.com`. Optional, for models and graphics: `huggingface.co`, `cdn-lfs.huggingface.co`, `nodejs.org`, `storage.googleapis.com`, `files.freemusicarchive.org`.
- [ ] **Secrets:** nothing to rotate. Add keys to the cloud environment only if you want live calls.
- [ ] **GitHub:** connect the repo in claude.ai/code (the GitHub app, or `/web-setup`).
- [ ] **Merge this PR** into `AI-Automation` (or tell a session to).
- [ ] **First test session from your phone:** "Do CL-6 from docs/ROADMAP.md, part 1 only: fix the clippy warnings in src-tauri/src/zoom/ and src-tauri/src/media/, no behaviour change, open a PR." Check that it ran `scripts/test-all.sh` and that the Cloud check went green.
- [ ] **What you can't do remotely:** check the GUI or preview, use the GPU (NVENC/CUDA), run local models (Parakeet, YuNet, Qwen), build or try the Windows installer by hand (only `package.yml` in CI), record, listen to audio, or use anything on the old disk (recordings, `%APPDATA%\AeroEdits`, keychain keys, `output/` mockups).

🤖 Generated with [Claude Code](https://claude.com/claude-code)
