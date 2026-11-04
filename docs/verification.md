# Verification and maintenance

Consolidated 1 November 2026 from existing logs. These are recorded results, not tests rerun
as part of the docs cleanup. [Roadmap](roadmap.md) owns follow-up work.

## Working safely

Build commands and prerequisites live in the [root README](../README.md). Test on copies of
recordings, never the originals. Use a separate dev profile/pipe and scratch project. Stop
Rust hot reload during agent runs and exports; do not race `cargo test` or `cargo build`
against `cargo tauri dev` in the same target directory.

For a code change, run the relevant behavior/regression tests. Front-end store tests live in
`front-end/tests/` (`npm run test:cleanup`); backend tests run from `src-tauri`. For changes to
media or rendering, include the appropriate real-recording probe and preview/export parity.
For docs, run `.venv\Scripts\python.exe scripts/check_doc_links.py --all`.

## Agent edit evaluation (EVAL-1)

Rerun after cost or quality changes. Keep the build, model, exact prompts, settings, reference
assets, tool trace and outputs with the run. Compare output quality and human corrections,
not just call count. Agent charges below exclude transcription/review provider spend and
human work; the simulated user judged frames and levels, not sound by ear.

### Setup and procedure

- Long-form input: a copy of `Documents/AeroShootRec/Untitled 22 Oct 2026 2.aero`, 7:10,
  screen + camera + mic + system audio + telemetry, a TX Flow/PaperTrade tutorial.
- Take input: a copy of `Untitled 22 Oct 2026 3.aero`, 1:40, camera + mic in two segments,
  about Hermes Agent. It is intentionally a different topic; unrelated tutorial footage
  should not be forced beneath it.
- App: dev build with `AEROEDITS_MCP_DEV=dev`, fresh project from the copied recording.
  Do not edit Rust while the run is in progress.
- Runner: [run.sh](plans/eval/run.sh), one headless Claude Code turn at a time, with an empty
  agent working directory and only AeroEdits MCP. The script pins the historical model
  `claude-sonnet-5-5`; record any model substitution as a changed condition.
- Set `EVAL_DIR` to an absolute scratch run folder containing `mcp.json` pointing to the
  dev executable with `--mcp-stdio`. Pass absolute prompt-file paths because the script changes
  directory. From Bash: `bash <repo>/docs/plans/eval/run.sh T1 <prompt-file>`, then pass the
  returned session ID as the third argument for subsequent turns.
- Sequence: T1 edit the tutorial automatically and stop before export; T2 add graphics;
  S1 make a short from the take, clean it, add helpful footage/graphics and stop before export.
  Preserve the exact historical prompts from run logs for strict comparisons; these descriptions
  are summaries, not verbatim prompts. Record answers to any questions and any accepted proposals.
- [tokens.py](plans/eval/tokens.py) reports context and tool-answer size, including images.
  Resumed sessions report cumulative cost: calculate each turn by subtraction.

Assess cleanup accuracy, protected word tails, correct short targeting, style coherence,
readability, meaningful graphics and revisions needed. Count redundant questions for saved
choices, unsolicited layout/webcam questions, and repeated single-frame reads where one sheet
would suffice. A missing online-upload authorization is a legitimate question. Check the
selected short before its export, even when the test itself stops earlier.

### Useful measured baselines

| Run | Tutorial T1 | Graphics T2 | Take short S1 | Combined |
|---|---|---|---|---|
| 4, 24 Oct | 13 calls, $0.191 | 5 calls, $0.080 | 13 calls, $0.122 | 31 calls, $0.393 |
| 5, 1 Nov | 18 calls, $0.237 | 13 calls, $0.161 | 26 calls, $0.293 | 57 calls, $0.691 |

Run 4 ended at 36.8k tokens after T1, 44.6k after T2 and 53.6k after S1. The tutorial was
5:46, with 114 mouth noises muted, 70.3 s of pauses removed, 12 zooms and five chapters.
Graphics were six text cards; the short stopped at an acceptance batch.

Run 5 ended at 39.8k, 56.9k and 88.4k tokens. Tutorial cleanup was similar (5:48.65), but four
of six graphics illustrated the content rather than merely naming it; three were lengthened
for reading. The short was 31.5 s, renamed from its own words, face framed, with emphasis
captions and three graphics. This is more work than run 4, not a like-for-like cost regression.
It asked about online transcription because the dev profile had not allowed it.

Run 5 found four issues fixed under QUAL-5: `check` did not address a short, the graphics guide
was read twice, a fix was reported while its render was pending, and count-up currency styling
was weak. ElevenLabs key rejection remains a local setup issue.

**S1 rerun, 1 Nov, after COST-5/QUAL-6:** 22 calls instead of 26; zero preliminary `more`
argument lookups instead of three. It made the same kind of short (100 s → 32 s, 42 noises
muted, one false start cut, three graphics, no unrelated footage). Cost was $0.307 versus
$0.293 with another online-authorization turn. Context was 49.1k versus 88.4k because this
was a fresh session; do not attribute that difference to schema compression. It stopped
before export and did not run `check`. It flagged a stray opening “Hermes” for the user.

## Speech review evaluation

Probe: `src-tauri/src/ai/review_probe.rs`. Labels and the synthetic Greek generator remain in
[plans/review-eval](plans/review-eval/). English case files refer to recordings in George's
Documents folder; they are not portable fixtures until those transcripts are supplied.
Greek EL-A and EL-B transcripts are included. The probe resolves relative transcript paths
beside the case file.

From a configured PowerShell build environment, in `src-tauri`:

```powershell
$env:AEROEDITS_REVIEW_CASE = (Resolve-Path ../docs/plans/review-eval/en-b.json).Path
$env:AEROEDITS_REVIEW_ENGINES = 'rules,local,cloud'
cargo test --release --lib --features local-llm review_probe -- --ignored --nocapture
```

Use the Vulkan/LLVM/Ninja environment from `scripts/build-windows.ps1` and a short target
path for the local-LLM build. Cloud runs use the configured provider and spend credits;
`rules` can run alone. `AEROEDITS_REVIEW_DUMP=1` dumps numbered words for blind labelling;
`AEROEDITS_REVIEW_OUT` redirects reports, which otherwise go to a temporary folder.
`AEROEDITS_REVIEW_CLOUD_MODEL` overrides the saved model.

Labels distinguish must-cut spans from optional judgement calls. A must span is missed unless
most words are cut; a cut is wrong when fewer than half its words are labelled must/optional.
Measure merged rules + model results, exactly as the app applies them.

Recorded on 21 Oct, RTX 3060 12 GB / Ryzen 5 3600, Qwen3.5 4B Q4_K_M locally versus the
configured OpenRouter `openai/gpt-6-luna`:

| Case group | Rules misses / wrong spans | Local misses / wrong spans | Cloud misses / wrong spans |
|---|---|---|---|
| EN-A/B/C, 18 must spans | 11 / 2 | 9 / 12 | 5 / 2 |
| EL-A synthetic, 19 must spans | 15 / 0 | 10 / 1 | 1 / 0 |

The cloud's two English wrong spans came from the rules; the local model added ten wrong
spans. Local review took roughly 3–12 s plus loading; cloud 12–38 s. This is a small sample,
not a general model ranking. Local candidates missed some reworded restarts and encouraged
blind acceptance of repeated phrases or meaningful “so”.

EN-D (25 Oct, fourth real English recording): cloud cut all six must fillers without a wrong
filler cut but made one wrong 12-word retake. Across four recordings, 17 must fillers supported
the decision to auto-apply cloud fillers, while retakes remain reviewable. Two-word stutter
rules improved; the defining-repeat error was fixed 26 Oct. EL-B (26 Oct) is a real 13-second
Greek sample with one retake: cloud found it, and rules found it after drawn-out-word matching.
It is too short to replace EL-A. Gemma testing was explicitly skipped, not pending implementation.

### Retakes on the labelled recordings (2 Nov)

`rules` and `flags` engines, no model. Must-cut spans across EN-A to EN-D: 33, of them 16 retakes.
A label was corrected first: EN-D's "what is, what is" named the words across the comma, so the
rule that cuts "what is," scored as a miss; it now has that span as an `alt`.

| Rules | Missed | Missed retakes | Wrong spans |
|---|---|---|---|
| Before (with the label as it was) | 12 | 7 | 0 |
| After | 9 | 4 | 0 |

The rules now cut "volume volume" and "comp... um... Complementary". Across all 19 transcripts on
this computer (34,207 words), a prototype of the two new rules fires 3 times: those two, and "high high leverage".
Of the 4 retakes they still miss, 3 are flagged for a reviewer ("they there", "see set", "to the
lower pairs to the lower perpetuals"); on "we j-" the rules cut "j-" and leave "we". All the
remaining filler misses are flagged. The new flags add 5 across 4,444 words. These were tuned on
these same recordings, so a recording they haven't seen is the real test.

<a id="fast-against-thorough"></a>
### Fast against thorough (2 Nov)

The same OpenRouter model from settings, once each on EN-A to EN-D, merged with the rules, after
the changes above. `cloud` reads every word, as a thorough review does; `cloud-fast` reads the
candidate windows, as a fast review does. An agent reviewing in-house was not measured.

| Review | Words read | Time | Missed (of 33) | Wrong spans |
|---|---|---|---|---|
| Thorough | 4,444 | 139 s | 5 | 4 |
| Fast | 2,484 | 129 s | 2 | 3 |

One run each on a small sample, so this does not show fast is better; it does show that reading
every word did not catch more here. Both still make wrong cuts, thorough on "pretty" said for weight
and "kind of", fast on two restarts it misread ("you can select purple", "So the capabilities,"),
which is why retakes stay reviewable. The 25 Oct wrong 12-word retake did not recur.

## Verified behavior and remaining limits

On 2 Nov, with a dev build on copies of real projects: `check` of the 25 Oct short took about 4 s
with renders in place and found words under the bottom band on all 4 graphics and captions hidden
on 2; a frame at 22 s bore out what it said of "Proof". A 9-minute video with 13 graphics took 18 to
26 s and was clean. With no renderer installed, it says the frames weren't checked. The export
dialog and Shorts tab show the same findings. The face finding has only unit tests on these
projects: no graphic there covered a face.

A saved workflow was proved on a second tutorial (AUTO-3): saved on a copy of `Agent test` with one
exception (no chapters) and one check (no zoom in the first 5 s), then followed from a copy of
`Untitled 29 Oct 2026` by a headless agent given only "edit the open tutorial automatically with
its preset", $0.16. It left chapters alone, reported them as open and verified the zoom rule. The
first attempt ignored both, as the agent asked for `tutorial` by name and got the built-in, which
is what `meant` fixes. That project was already part-edited, so no full edit from a raw recording
has followed a workflow yet.

Recorded Windows checks cover MCP/CLI create-to-export, proposals/accept/undo, history across
restarts and agent-run undo, checkpoints, layouts, graphic install/render/library lifecycle,
style fonts/logos in exports and cancellation. Three real Codex sessions on 25 Oct exercised
writes, the library lifecycle and a logo-bearing render/export.

On 31 Oct, the real graphics gate passed the bundled templates; a Greek `prompt-reply` was
rendered in the dev app with clean frame checks. This supersedes the old blanket claim that
Greek graphics had never been tried; it does not establish all-font/all-template glyph coverage.

On 1 Nov, dev-app checks exercised Preset Run on a real take copy, graphic versions through
Inspector/CLI and undo, export waiting for face detection, and graphic-file undo restoring
text on disk, in the Inspector and in a rendered frame (SAFE-3). SND-4's sampled take had a
quiet floor around -87 dBFS. These checks do not replace listening to real joins.

Still unverified or narrow:

- Preset Run with online steps; FIX-9 transcription consent in a real Codex session.
- Most MCP/graphics paths on macOS; local transcription is not in that build.
- Layout performance with a real `.aero` under the perf probe; matched fast/thorough review.
- Longer real Greek speech; comprehensive graphic glyph coverage and phone-size placement.
- The 30 Oct audit fixed all five recorded defects (thumbnail churn, stale edit points,
  wrong-short delayed layout saves, stale search replies and locked transcript reads).
  Equal-revision project switching still needs the desktop previous/next-edit check; no
  dedicated automated regression covered that finding. Synthetic timings are not desktop benchmarks.

Historical Codex CLI 0.163 checks required per-tool write approval; read-only tools worked.
Tool schemas loaded on demand (about 0.9k extra input tokens/turn on 26 Oct), not the whole
list. Treat those as version-specific measurements, not guarantees about every client today.

## Performance reference

The 4–5 Oct release measurements on an 11-minute recording were: project open 52 ms;
first mic/system waveform 155/220 ms (8 ms cached); find 179 pauses 106 ms and cut them 8 ms;
edit/undo 12 ms; preview rebuild about 7 ms; audio mixer 122 ms first / 13 ms cached; seek
65 ms typical / 170 ms p95. A recorded RTX 3060 export ran at 2.6× realtime (1.8× with jump cuts).
Use `src-tauri/src/perf.rs` with `AERO_PROBE=<real recording directory>` for comparisons;
these old figures are context, not performance promises for the current build.

### Dense timelines and the webview fallback (PERF-3, 2 Nov)

Synthetic project in the Vite dev build, measured from the store change through layout (not
paint), in the Browser pane while it was hidden: 8,000 clips on two tracks of 30 minutes, one
sound's waveform loaded, zoomed out, every clip moved by each edit.

| | Each clip its own block | Runs |
|---|---|---|
| Edit, median of 5 (two runs) | 829 and 843 ms | 70 and 71 ms |
| Selecting two clips | 237 and 223 ms | 40 ms |
| Elements in the page / canvases | 24,590 / 4,000 | 606 / 3 |

Clicking a run selected the clip under the pointer with its partner, and a drag on one reached
its commit. Real pointer use in the desktop app and a real dense project are not checked.

Encoders on real frames, median of 40, q82 4:2:0, one thread except where noted. libjpeg-turbo
was timed through Pillow, without its FFI or BGRA path; its files were the same size or smaller.

| Frame | `jpeg-encoder` 0.7 SIMD (app) | libjpeg-turbo 8.0 | `image` 0.25 | `jpeg-encoder`, 4 threads, 4 bands |
|---|---|---|---|---|
| Screen, 720p | 7.6 ms, 114 KB | 2.9 ms, 114 KB | 23.5 ms, 132 KB | 2.9 ms |
| Screen, 1080p | 16.8 ms, 224 KB | 6.2 ms, 224 KB | 58.6 ms, 271 KB | 6.6 ms |
| Noisy test pattern, 1080p | 26.7 ms, 646 KB | 9.4 ms, 629 KB | 93.2 ms, 963 KB | 10.1 ms |

Decision: keep `jpeg-encoder`. The fallback starts at 720p/30 fps, where it uses 7.6 of 33 ms,
and its worker never blocks the preview; wgpu is the normal path. libjpeg-turbo would save about
2.7× at the cost of a C library and NASM in every build, so revisit only if a machine that
needs the fallback measures encoding as its limit. Bands are not one JPEG file: the webview would
need restart markers or several images, so that column is only an upper bound.

Two alternating encoder threads, built after that. One thread makes the worker wait for draw +
encode per frame, and two let frames overlap. `perf_webview_encoders` (ignored test): the playback
loop on a copy of a real recording, 15 s a run, two runs each, frames shown per second:

| Setting | Release, 1 thread | Release, 2 threads | Dev build, 1 thread | Dev build, 2 threads |
|---|---|---|---|---|
| 720p, 30 fps (default) | 29.6 | 29.6 | 29.5 | 29.5 |
| 720p, 60 fps | 59.0 | 59.0 | 33.6-36.2 | 56.4-58.7 |
| 1080p, 30 fps | 29.5 | 29.5 | 18.6-19.0 | 29.3-29.4 |
| 1080p, 60 fps | 32.6-36.7 | 57.4-58.4 | 18.4-18.6 | 35.7-36.0 |

Encoding takes 7 ms (720p) and 16-22 ms (1080p) in release, 19 and 41-44 ms in the dev build, which
stands in for a slower machine. Time from drawn to shown doesn't change. At most 2 frames in a
run finished after a newer one and were dropped.

In the dev app (`AEROEDITS_PREVIEW=webview`, copy of a recording): 30 fps at 720p/30, 45-58 at
720p/60, 23-29 at 1080p/30 and 25-28 at 1080p/60, no errors. After a burst of 13 quick seeks,
the frame stored was byte for byte the one a lone seek gave, at 8 of 8 times. A real slower
machine and the Shorts Studio path, which uses the same encoders, are not checked.

### Builds and the test suite (QUAL-2, QUAL-4, 2 Nov)

One-string change in one file, `cargo build --features tauri-app`, 12-thread Ryzen 5 3600:

| File | Before | `lto = "off"` |
|---|---|---|
| `chapters.rs` | 97 and 128 s | 29 and 49 s |
| `render/mod.rs` | 101 and 102 s | 25 and 22 s |
| `commands/mod.rs` | 129 and 128 s | 49 and 50 s |

`-Z time-passes` showed `LLVM_thinlto` taking 80 of 98 s, 2 s for link and about 21 s before code
generation; the binary links in about 8 s. The test binary went from 50-56 s to 28 s. A full
dev build with the new setting took 7 min 9 s. Alternating the same source on a real recording in the dev
profile, `perf_steps_back`, `perf_look_swap` and `perf_decode_throughput` stayed within noise.
Crate splitting and sccache were not needed, and `target/debug/incremental` had reached 33 GB of
mostly old sessions, which only `cargo clean` reclaims. Release builds keep their own profile.

The installed app of 26 Oct has no MSVC Rich header, so `rust-lld` linked it, and it came from
the NSIS installer of that build. On 2 Nov `scripts/build-windows.ps1` built the final tree with
CUDA and the local model in 12 minutes: an installer of 137.79 MiB, its binary again without a
Rich header, and the binary answered MCP `initialize` and listed 32 tools. That installer was
not installed or run.

Suite: 708 tests and 53 probes before, in 26.6 s on its first run; 710 and 49 after removing
obsolete ones and adding shared-helper tests, 20.7 s first run, then 17.6 and 17.8 s. No warm
baseline was taken, so most of that gap may be the first run's scan. The removals were not
for speed: FFmpeg-backed tests dominate and take about 4 s each alone. Front-end: 43 to 52
tests in about 2 s. The `gpu_*` tests were not rerun.

## Dependency review

Review quarterly (next January 2027), or sooner for a security advisory, needed feature or
measured improvement. Exact versions live in source and lockfiles, not a second inventory here.

| Component | Authoritative location / coupling |
|---|---|
| Rust and build Node | `rust-toolchain.toml`, `.nvmrc`, CI workflows; keep them consistent |
| Front end | `front-end/package.json` and `package-lock.json` |
| Rust libraries | `src-tauri/Cargo.toml` and committed `Cargo.lock` |
| Graphics | `src-tauri/graphics/runtime/package.json` + lock and `src-tauri/src/graphics/runtime.rs`; HyperFrames remains pinned at 0.8.228; update its Node, Chrome and GSAP together as required |
| FFmpeg | `src-tauri/scripts/fetch-ffmpeg.mjs`; checksum-pinned month-end BtbN release, not moving latest |
| Models / GPU pack | Model download code and `src-tauri/src/cuda.rs`; hashes, model licences and ONNX Runtime/CUDA compatibility |

Read changelogs, update one component or coupled group on a branch, run Rust tests and
TypeScript checks, then relevant GPU and perf probes and preview/export parity on real footage. For HyperFrames,
run the gate on every bundled template and compare rerenders; for review models, run labelled
cases. A major migration is explicit work, never a side effect. Keep changes only with evidence
of compatibility and worthwhile benefit. Recheck licences, including GSAP redistribution
terms before bundling it differently. Candidate reviews include the Tailwind major
([DEP-3](roadmap.md#dep-3)), ONNX Runtime leaving release candidates ([DEP-4](roadmap.md#dep-4))
and newer HyperFrames; their existence is not a reason to upgrade during unrelated work.

**1 Nov 2026 round:**

- Vite 5.4 to 8.3 with plugin-react 6: `minify` is a boolean now, and the test runner bundles
  with esbuild, a dev dependency of its own.
- React 18.3 to 19.3, with dockview-react 8.8 and lucide-react 1.66: `useRef<T>()` needs its
  first value, the test harness reads React 19's dispatcher, and the workspace registers the
  dock api again after StrictMode's second run.
- HyperFrames 0.8.162 to 0.8.228: same Chrome, Node and GSAP. The gate passes every template,
  renders are identical (4K within SSIM 0.9995) and the gate is about 10% faster.
- Tailwind 4 was tried and not kept (DEP-3). `ort` stays at rc.13, since it is still a release
  candidate (DEP-4).
