# Dependencies: what we pin, and when we recheck

Decided 21 Oct 2026: keep toolchains and key libraries current **on purpose**. Pin them so a
build is reproducible, review them on a schedule so we pick up speed and quality gains, and
update only after a compatibility check so an update never turns into an unplanned migration.
Work items: [DEP-1](roadmap.md#dep-1) (pin what isn't pinned, first review) and
[DEP-2](roadmap.md#dep-2) (the recurring review).

## Inventory (21 Oct 2026)

"Pinned" means a build gets exactly this version. A caret range (`^5.4`) is pinned in practice by
`package-lock.json`; a moving target (`stable`, `latest`) is not pinned.

### Toolchains

| Component | Version in use | Where it is set | Pinned? | Recheck |
|---|---|---|---|---|
| Rust | 1.92.0 on George's PC | CI: `dtolnay/rust-toolchain@stable` | **No** (moving `stable`); no `rust-toolchain.toml` | DEP-1: pin it |
| Rust edition | 2021 | `src-tauri/Cargo.toml` | Yes | When moving to edition 2024 |
| Node.js (front-end build) | 22.19 locally, 20 in CI | README says "18 or newer"; `.github/workflows/*.yml` say 20 | **No**, and three different answers | DEP-1: one version everywhere |
| TypeScript | 5.9.3 | `front-end/package.json` `^5.4.5` + lock | Lock file | Quarterly |
| Tauri | 2.x (`@tauri-apps/api` 2.11.1) | `Cargo.toml` `tauri = "2.0.0"`, `package.json` | Lock files | Quarterly |

### Front end (locked by `front-end/package-lock.json`)

| Package | Locked | Notes |
|---|---|---|
| React / React DOM | 18.3.1 | React 19 is the next major |
| Vite | 5.4.21 | Two majors behind current |
| Tailwind CSS | 3.4.19 | Tailwind 4 changes configuration |
| dockview-react | 8.4.0 | Workspace layouts; layout keys are versioned |
| zustand | 4.5.7 | v5 is the next major |

### Back end crates (`src-tauri/Cargo.toml`; `Cargo.lock` is git-ignored)

| Crate | Requirement | Notes |
|---|---|---|
| `wgpu` | 27.0.1 | Compositor and preview; parity tests guard updates |
| `rmcp` | 3.5 | MCP server; protocol version answered by the bridge |
| `ort` (ONNX Runtime) | `=2.0.0-rc.13` (exact) | Parakeet, wav2vec2, YuNet; ONNX Runtime 1.28 CUDA build |
| `llama-cpp-2` | `=0.1.160` (exact) | Local filler review on Vulkan; lacks Gemma's chat template |
| `tokio` | 1.38 | — |

`Cargo.lock` is ignored, so two machines can resolve different patch versions of the
caret-ranged crates. Deciding whether to commit it is part of DEP-1 (it is normally committed for
applications).

### Runtimes the app downloads (exact, checked by hash)

| Component | Version | Where | Notes |
|---|---|---|---|
| **HyperFrames** | **0.8.162** | `src-tauri/graphics/runtime/package.json` + lock, `graphics/runtime.rs` | Stays pinned (decided 21 Oct). Recheck at each review: changelog, our gate on every bundled template, render determinism |
| Node.js (graphics runtime) | 22.23.3 | `graphics/runtime.rs` `NODE_VERSION` | Separate from the build's Node |
| chrome-headless-shell | 152.0.7977.30 | `graphics/runtime.rs` `CHROME_VERSION` | Must match what HyperFrames pins; bump together |
| GSAP | not yet | — | To be vendored and pinned by [GFX-2](roadmap.md#gfx-2) |
| FFmpeg (bundled) | BtbN 8.1 branch, `latest` release | `src-tauri/scripts/fetch-ffmpeg.mjs` | **Not reproducible**: `latest` moves; checksums come from the same moving release. DEP-1: pin a dated release |
| CUDA / cuDNN (GPU pack) | CUDA 13.4 (cudart 13.4.92, cuBLAS 13.8), cuFFT 12.4, cuDNN 9.27 | `src-tauri/src/cuda.rs` | Must match the ONNX Runtime CUDA build |

### Models (downloaded on demand, checked by SHA-256)

| Model | Use | Licence |
|---|---|---|
| Parakeet Unified EN 0.6B (ONNX) and Parakeet TDT 0.6B v3 (Verbatim) | Local English transcription | See the model cards (recheck at each review) |
| wav2vec2-base-960h | Word alignment | Apache-2.0 |
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
   comparison; for models, the relevant probe (`review_probe`, alignment probe).
4. Keep the update only if nothing regresses, or if the gain is worth a planned migration; then
   write it here with the date.
5. A major version that needs code changes becomes a roadmap item with its own size, never a
   side effect of another task.

## Review log

| Date | Component | From → to | Result |
|---|---|---|---|
| — | — | — | First review is part of DEP-1 |
