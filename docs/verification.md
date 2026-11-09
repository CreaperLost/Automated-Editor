# Verification and maintenance

Historical measurements consolidated 1 November; documentation reconciled 8 November 2026.
The fresh focused checks below are separate from retained historical logs.
[Roadmap](ROADMAP.md) owns feature work; [gap.md](gap.md) owns detailed open defects.

## Documentation reconciliation, 8 November 2026

Snapshot: `be7179c`, `claude/cloud-prep`, clean tracked/untracked tree before documentation
edits. Entire `docs/` inventory retained or consolidated, including runners and labelled JSON.
Source inspection covered current callers/tests for GAP-002..028 and the retained follow-ups;
it was targeted, not a new exhaustive code audit. No source or user media was changed.

| Fresh check | Result and limit |
|---|---|
| `cargo test --lib --offline styles_are_saved_listed_renamed_and_removed -- --nocapture` | 1 passed after workspace TEMP/TMP override; closes GAP-005. First default-temp attempt failed on access before the catalog assertion. |
| `cargo test --lib --offline the_tool_list_stays_small -- --nocapture` | 1 passed; current budget <51,000 UTF-8 bytes. Closes GAP-006's old-budget failure; no client token/latency measurement. |
| `cargo test --lib --offline faceless::tests -- --nocapture` | 6 passed, 1 ignored (`live_voice`). Includes package-stage wiring, narration placement and visual gap fill; does not cover same-text redo or stale acceptance. |
| `cargo test --lib --offline publish::tests -- --nocapture` | 6 passed, including fake-service lost-reply retry, request modes and thumbnail validation. Does not cover release changes, concurrent UI saves, wrong-project dialogs or a real upload. |
| `npm run test:cleanup` in `front-end` | 52 passed, zero failed/skipped. Includes AUD thumbnail/layout/search regression checks. Node hook harness only; GAP-024 remains open. |

The Rust runs emitted existing private-interface/dead-code warnings and incremental hard-link
fallback. No full Rust suite, clippy count, native UI, listening, GPU/runtime gate, new render,
Linux/macOS run, live provider call or paid-service acceptance was performed. Prior GAP probes
and historical suite/build/CI results below are not fresh runs. Documentation checks passed:
`python scripts/check_doc_links.py --all` (zero broken file targets), a supplemental check of
local Markdown fragments/exact path case (zero errors), all 32 stable GAP IDs and required issue
fields, evaluation JSON parsing/ranges, and `git diff --check`. Only Markdown files changed.

### Retired audit material

`audit.md` contained only fixed AUD-001..008 narratives and earlier test counts. Its useful
contracts are now in Architecture; tests remain in `front-end/tests/auditFixes.test.ts`,
`auditRecovery.test.ts`, `src-tauri/src/source_search.rs`, `commands/thumbs.rs` and
`transcript/store.rs`. Current frontend tests pass; this is not a new native walkthrough.

| Stable IDs | Retained fix / remaining verification |
|---|---|
| AUD-001/006/008 | Wanted thumbnail entries are protected; transient failures retry with bounded backoff; released demand prunes unused entries. Regression checks pass in the Node suite. |
| AUD-002 | Edit-point memo depends on sequence identity. Current source retains this; no dedicated automated test or fresh equal-revision desktop-switch check. |
| AUD-003/007 | Debounced settings capture project/short ownership; layout saves serialize/rebase, preserve pending drafts and invalidate stale replies. Node regressions pass. Other save paths still have GAP-010/015/016. |
| AUD-004 | Search clear/short query invalidates pending responses. Node regression passes. |
| AUD-005 | UI search snapshots the document then runs on a blocking worker; transcript cache invalidates on saves and file stamps; results are bounded. Existing Rust checks/timing are historical, not rerun in this pass. MCP search still uses its locked synchronous path. |

The old 100k-word search timing (121 ms cold, 1.0–1.2 ms warm) was synthetic, not a desktop
benchmark. Original investigations remain in Git history; their already-fixed defect text
must not become a new backlog. The audit's still-useful lifecycle, thumbnail-load and
relinked/imported-media cache investigations remain [open coverage below](#verified-behavior-and-remaining-limits)
and [GAP-032](gap.md#gap-032).

### Historical cloud-prep evidence

The earlier handoff recorded Windows `cargo test` 787 passed / 0 failed / 54 ignored and
frontend 52/52 on 8 Nov. It also recorded Cloud check run **42442268439** green on fresh
Ubuntu 24.04: format, Rust, lavapipe GPU, smoke, frontend typecheck/tests/build and doc links.
That remote result was not fetched or revalidated here. Workflows are in
[cloud-check.yml](../.github/workflows/cloud-check.yml) and [ci.yml](../.github/workflows/ci.yml).

Recorded smoke summary: **6.000 s recording → cut 1.890 s of silence → 4.133 s, 124 frames**,
H.264 320×180 plus AAC. The prior six-frame inspection saw clock values 0.367, 1.033, 1.733,
4.300, 4.567, 5.667 in a padded screen over green wallpaper. This is retained observation,
not a newly viewed output. `src-tauri/src/smoke.rs` skips missing FFmpeg/adapter unless
`AEROEDITS_REQUIRE_FFMPEG` / `AEROEDITS_REQUIRE_GPU` is set; CI sets them for the cloud run.

9 Nov (CL-7/CL-8), cloud VM, lavapipe: the smoke test exports through `aeroedits render`, and
two more smoke tests pass. A quiet voice (−30 LUFS) with a click at 3.0 s: the mouth-noise pass
finds it, muting it leaves peak 0.013 there (room 0.022; 0.147 unmuted), and normalize gives
−13.6 LUFS. A 9:16 project asked for 1280×720 exports 720×1280; “HELLO” is 4939 white pixels
at 0.5 s and none at 3.0 s. Viewed frames: white outlined caption text low in the frame over
the test pattern, “HELLO” then “HELLO WORLD”, none after.

## Faceless and publish acceptance

Implementation present is not production acceptance. No live calls were made in this pass.

| Work | Evidence already recorded | Still required |
|---|---|---|
| FCL-0/1/2 | Prior dev-app production creation at 9:16; 7 Nov discovery 712 stories/25 s, 8/8 funnels; one real quote accepted and a planted number refused | Rebuild a complete research dossier on a new story; labelled grouping calibration. GAP-013/026 limit address/quantity assurances. |
| FCL-3 | Script/review validation and MCP exist; no Script panel | Full new-story script/review by an agent; UI is CL-13. Hash integrity remains GAP-027. |
| FCL-4 | One historical speech call checked model/voice, plus unit/fixture tests | After GAP-004/008/027, voice a real script, regenerate a changed take, check new asset/transcript/cues and have George listen. No current paid pricing guarantee. |
| FCL-5 | Earlier notes report four Broadsheet templates passing 16:9, 9:16 and split gate; 509-image catalogue built in | Finish dedicated parts/browser and inspect a phone-size contact sheet. Full current Linux gate remains CL-9. Assets and licences remain in their catalogue/LICENSES notes. |
| FCL-6 | Word/action cues and visuals gap-fill tests | One complete agent-built film: verify coverage, pacing, word starts, reading holds, variety and render frames. Phone clearance limits are GAP-030. |
| FCL-7 | General music, ducking and normalization exist | Faceless preset bed/cues and measured −14 LUFS export with bed ≥15 dB below narration, plus listening. |
| FCL-8/PUB-1 | Shared package editor, thumbnail constraints and current package-stage test | Fix save/ownership races; generation from a brief is CL-14. |
| PUB-2/FCL-9 | Mocked lost-reply upload test passes; package/upload stage hooks exist; old handoff recorded Zernio channel connection | Approved private upload by George, real visibility/status/Studio check; source/time-word checks at release and made-list behavior, including failed/uncertain replies. GAP-002/011/016/027 remain. |
| FCL-10 | MCP methods/operations exist | Long and short idea→approved private upload EVAL-1 with corrections, app/provider/agent costs and elapsed time. Do not report the old Python pipeline as required runtime. |

Historical Treg measurements from 8 Nov (not rerun): discovery with X/Reddit $0.0099 in one
run; later filtered search $0.015, demand for ten stories including first channel refresh
$0.040, X suggestions $0.0036. These are example invoices, not a price promise or a total-video
budget. Current per-call/search guards live in source. No own-channel outcome calibration exists.

## Working safely

Build commands and prerequisites live in the [root README](../README.md). Test on copies of
recordings, never the originals. Use a separate dev profile/pipe and scratch project. Stop
Rust hot reload during agent runs and exports; do not race `cargo test` or `cargo build`
against `cargo tauri dev` in the same target directory.

For a code change, run the relevant behavior/regression tests. Front-end store tests live in
`front-end/tests/` (`npm run test:cleanup`); backend tests run from `src-tauri`. For changes to
media or rendering, include the appropriate real-recording probe and preview/export parity.
For docs, run `python scripts/check_doc_links.py --all` (or `python3` on Linux).
That checker validates local file targets, not fragments or remote URLs; inspect changed
anchors too. On this Windows sandbox, redirect TEMP/TMP to a writable workspace folder
for persistence tests; the default sandbox temp path denied an atomic folder operation.

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

- Pending-save flush/ownership while closing projects, switching workspaces or undoing;
  thumbnail queue cancellation/cold dense-timeline loading and notification redraw cost;
  relinked/repeated-import preview/export cache invalidation and long-session memory. These
  are retained investigation targets, not confirmed new bugs. Reproduce on a scratch project,
  measure work/retained bytes, and add a gap.md finding only when evidence identifies a fault.

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
([DEP-3](ROADMAP.md#dep-3)), ONNX Runtime leaving release candidates ([DEP-4](ROADMAP.md#dep-4))
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
