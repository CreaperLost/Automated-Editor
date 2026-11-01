# Dependencies: what we pin, and when we recheck

Decided 21 Oct 2026: keep toolchains and key libraries current **on purpose**. Pin them so a
build is reproducible, review them on a schedule so we pick up speed and quality gains, and
update only after a compatibility check so an update never turns into an unplanned migration.
Work items: [DEP-1](done.md#dep-1) (pin what isn't pinned, first review) and
[DEP-2](roadmap.md#dep-2) (the recurring review).

## Inventory (21 Oct 2026)

"Pinned" means a build gets exactly this version. A caret range (`^5.4`) is pinned in practice by
`package-lock.json`; a moving target (`stable`, `latest`) is not pinned.

### Toolchains

| Component | Version in use | Where it is set | Pinned? | Recheck |
|---|---|---|---|---|
| Rust | 1.92.0 | `rust-toolchain.toml` (rustup installs it), CI `dtolnay/rust-toolchain@1.92.0` | Yes (21 Oct) | Quarterly; change both places together |
| Rust edition | 2021 | `src-tauri/Cargo.toml` | Yes | When moving to edition 2024 |
| Node.js (front-end build) | 22 (LTS) | `.nvmrc`; CI reads it (`node-version-file`); README | Major pinned (21 Oct); CI was on Node 20, past end of life | Quarterly; Node 24 is the next LTS |
| TypeScript | 5.9.3 | `front-end/package.json` `^5.4.5` + lock | Lock file | Quarterly |
| Tauri | 2.x (`@tauri-apps/api` 2.11.1) | `Cargo.toml` `tauri = "2.0.0"`, `package.json` | Lock files | Quarterly |

### Front end (locked by `front-end/package-lock.json`)

| Package | Locked | Notes |
|---|---|---|
| React / React DOM | 19.3.1 (1 Nov) | The test harness's hook dispatcher reads React 19's internals (`tests/hooks.ts`) |
| Vite | 8.3.5 (1 Nov), `@vitejs/plugin-react` 6.1.2 | Minifies with its own minifier; `esbuild` 0.28.2 is a dev dependency of its own for the test runner |
| Tailwind CSS | 3.4.19 | Tailwind 4 is its own item, [DEP-3](roadmap.md#dep-3) |
| dockview-react | 8.8.3 (1 Nov) | Workspace layouts; layout keys are versioned |
| lucide-react | 1.66.0 (1 Nov) | Icons; 0.344 did not accept React 19 |
| zustand | 4.5.7 | v5 is the next major |

### Back end crates (`src-tauri/Cargo.toml`, locked by `src-tauri/Cargo.lock`)

| Crate | Requirement | Notes |
|---|---|---|
| `wgpu` | 27.0.1 | Compositor and preview; parity tests guard updates |
| `rmcp` | 3.5 | MCP server; protocol version answered by the bridge |
| `ort` (ONNX Runtime) | `=2.0.0-rc.13` (exact) | Parakeet, YuNet; ONNX Runtime 1.28 CUDA build. Still release candidates on 1 Nov (rc.15); moving is [DEP-4](roadmap.md#dep-4) |
| `llama-cpp-2` | `=0.1.160` (exact) | Local filler review on Vulkan; lacks Gemma's chat template |
| `tokio` | 1.38 | — |
| `serde_json` | 1.0 with `float_roundtrip` (locked 1.0.151) | Exact float parsing since 23 Oct: the default parser can read a number back one bit off, which made a reopened edit look changed to the undo history |
| `flate2` | 1.1 (locked 1.1.10, pure-Rust `miniz_oxide` back end) | Compresses the edit history's saved states (SAFE-2, 23 Oct). Was already in the tree through `png`, so adding it changed no locked version |
| `wuff` | 0.2.9 (MIT, pure Rust) | Unpacks a style's .woff/.woff2 font for captions (CAP-1, 23 Oct). A port of Google's WOFF2 decoder; checked on two KaTeX .woff2 files. Brings a second, older `brotli-decompressor` (5.0.3 beside 6.0.1) and `alloc-no-stdlib` 2.0.4; drop the duplicate when `wuff` moves to 6.x |

`Cargo.lock` is committed (since 21 Oct; it used to be git-ignored), so every machine and CI
build with exactly the same crates. `cargo update` happens only in a review.

### Runtimes the app downloads (exact, checked by hash)

| Component | Version | Where | Notes |
|---|---|---|---|
| **HyperFrames** | **0.8.228** (1 Nov) | `src-tauri/graphics/runtime/package.json` + lock, `graphics/runtime.rs` | Stays pinned (decided 21 Oct). Recheck at each review: changelog, our gate on every bundled template, render determinism. It pins `undici` 7.25.0 exactly, which has high-severity advisories (SOCKS5 proxies, cookies, shared caches) that our offline renders don't reach; 0.8.162 had the same version |
| Node.js (graphics runtime) | 22.23.4 | `graphics/runtime.rs` `NODE_VERSION` | Separate from the build's Node |
| chrome-headless-shell | 152.0.7977.30 | `graphics/runtime.rs` `CHROME_VERSION` | Must match what HyperFrames pins; bump together |
| GSAP | 3.15.0 (sha256-pinned npm tarball) | `graphics/runtime.rs` `GSAP_VERSION`, `graphics/gsap.rs` | The version HyperFrames (0.8.162 and 0.8.228) links from its CDN (`GSAP_CDN_VERSION`); bump together. Fetched from npm onto the user's computer when a graphic first loads it (and at runtime install), never shipped in the repo or installer: its licence is GreenSock's "Standard 'no charge' licence" ("free, even for commercial use" per its README; the full terms at gsap.com/standard-license did not load on 22 Oct, so redistribution terms are unread) |
| GSAP | not yet | — | To be vendored and pinned by [GFX-2](done.md#gfx-2) |
| FFmpeg (bundled) | `n8.1.3-9-g29e619e767`, BtbN release `autobuild-2026-09-30-13-08` | `src-tauri/scripts/fetch-ffmpeg.mjs` (`RELEASE`, `VERSION`, `SHA256`) | Yes, checksums in the script (21 Oct). BtbN prunes daily builds after ~2 weeks but keeps month-end ones, so pin month-end releases only |
| CUDA / cuDNN (GPU pack) | CUDA 13.4 (cudart 13.4.92, cuBLAS 13.8), cuFFT 12.4, cuDNN 9.27 | `src-tauri/src/cuda.rs` | Must match the ONNX Runtime CUDA build |

### Models (downloaded on demand, checked by SHA-256)

| Model | Use | Licence |
|---|---|---|
| Parakeet Unified EN 0.6B (ONNX) and Parakeet TDT 0.6B v3 (Verbatim) | Local English transcription | See the model cards (recheck at each review) |
| YuNet | Face detection | MIT |
| Qwen3.5 4B Q4_K_M (GGUF) | Local filler review | Apache-2.0 |

Any model added must pass the licence rule in [decisions.md](decisions.md#settled).

## The review

**When:** once a quarter (next: January 2027), and additionally whenever a security advisory,
a needed feature or a measured performance gain points at one component.

**How, per component:**

1. Read the changelog between our version and the target; note breaking changes and gains.
2. Update on a branch, one component (or one tightly coupled group: Tauri + `@tauri-apps/api`;
   HyperFrames + chrome-headless-shell + its Node; ONNX Runtime + the CUDA pack) at a time.
3. Run `cargo test`, `npx tsc --noEmit`, the GPU tests, the `perf.rs` benchmark and the parity
   probes on a real `.aero`; for HyperFrames, the gate on every bundled template and a re-render
   comparison; for models, the relevant probe (`review_probe`).
4. Keep the update only if nothing regresses, or if the gain is worth a planned migration; then
   write it here with the date.
5. A major version that needs code changes becomes a roadmap item with its own size, never a
   side effect of another task.

## Review log

| Date | Component | From → to | Result |
|---|---|---|---|
| 21 Oct 2026 | Rust | `stable` (unpinned, 1.92.0 here) → 1.92.0 pinned | Full suite passes (538), `cargo fmt` clean |
| 21 Oct 2026 | Node.js | 18+ / 20 / 22 → 22 everywhere | Front end unchanged; CI moved off Node 20 (end of life) |
| 21 Oct 2026 | FFmpeg | BtbN `latest` (8.1.3-20, moving) → 30 Sep build 8.1.3-9 | Pinned with checksums; a real download verified. Same 8.1 branch, a month older than `latest` |
| 21 Oct 2026 | Crates | `Cargo.lock` ignored → committed | No version changes |
| 1 Nov 2026 | Vite + `@vitejs/plugin-react` | 5.4.21 + 4.3 → 8.3.5 + 6.1.2 | `minify: "esbuild"` is gone in Vite 8 (boolean now); the test runner bundles with esbuild, now a dev dependency of its own. `tsc`, the 32 front-end tests and the build pass |
| 1 Nov 2026 | React + React DOM (+ types, lucide-react, dockview-react) | 18.3.1 → 19.3.1 | Typing: `useRef<T>()` needs its first value (5 places). The test harness reads React 19's dispatcher. StrictMode's second effect run left the dock's api unset in development; the workspace registers it again. Dev app: every panel opens, no console errors, playback plays |
| 1 Nov 2026 | Tailwind CSS | 3.4.19 → 4 tried, not kept | The upgrade tool's 266 changes in 45 files include rewriting an `EffectKind` value; its own item, [DEP-3](roadmap.md#dep-3) |
| 1 Nov 2026 | `ort` / parakeet-rs | rc.13 / 0.3.8, unchanged | `ort` is still a release candidate (rc.15); parakeet-rs 0.4 needs exactly rc.15 and a new ONNX Runtime set-up; [DEP-4](roadmap.md#dep-4) |
| 1 Nov 2026 | HyperFrames | 0.8.162 → 0.8.228 | 64 releases, nothing breaking; Windows fixes (a locked Chrome log no longer fails a render, audio scratch files, child processes left by a killed CLI, a 45 s wait for a graphic with no timeline). Same Chrome (152.0.7977.30), Node 22 and GSAP 3.15.0, so only the npm packages change. A fresh install from the lock passes; the gate passes all 13 templates and the GSAP graphic, and the one-off fails for each reason it should, as on 0.8.162. Gate 801 s against 895 s on 0.8.162; the 4K final render 63 s against 74 s. The two draft renders (16:9 and 9:16) are bit-identical, 4K at 60 fps SSIM 0.9995 (PSNR 63.5 dB, alpha identical). The app's installed runtime shows "Update the graphics renderer"; each graphic renders once again (the cache key holds the version) |
| Next | Everything else in the inventory | — | January 2027, or sooner when a gain or advisory appears |
