# Performance review of the current system

Status: completed performance pass. Findings and measurements were saved incrementally.

Reviewed revision: `642e77e` (2026-10-04), including the new backend/UI preview-performance commits. Focus: playback/export throughput, decoding, mixing, UI rendering, allocation, I/O and contention. This is a review, not an implementation pass. Concurrent working-tree edits appeared during review in the timeline, sequence/store helpers, export profiling and the built-in performance harness. They were preserved. The measured hot-path logic was rechecked against those edits; the frontend references below use the later working-copy line positions.

The earlier review reports are no longer present in this checkout. Prior findings from this chat will be checked against the new code before being carried forward.

## What to optimize first

Seven bottlenecks are supported by timing, cache-identity or geometry probes. Priorities depend on the workload; the largest observed costs were:

| Priority | Workload / bottleneck | Observed cost | First change to try |
| --- | --- | --- | --- |
| High | P04: 4,000 cut clips, 30 zooms | 827 ms to derive zoom ranges | Build each source clock once per revision/summary; then fix interval scaling |
| High | P05: dragging 4,000 selected clips in an 8,000-clip timeline | 185–190 ms per math evaluation | Cache ID maps, selection sets and snap edges |
| High | P01: four 4K still layers at 720p preview | 42 ms scene construction; p95 53 ms | Share immutable pixels and cache preview-sized variants |
| High for large zoom | P06: one long waveform at 64× / DPR 2 | 169 MiB requested RGBA capacity | Tile/crop backing canvases to the viewport |
| Medium; higher for long recordings | P02/P03: polish setup and cache eviction | 114 ms for an irrelevant two-minute file; 257/257 repeated analyses | Filter source ranges and retain the active analysis set |
| High for full-HD/high-detail previews | P07: 1080p JPEG transport | 28–50 ms encoding alone | Fit transport quality to a measured budget; evaluate faster transport/encoding |

These are separate measurements with different fixtures. Do not sum the rows into an estimate of application latency or assume the same speedup on every project.

## Verified findings

### P01 — Still images bypass preview resolution limits and deep-copy full pixel buffers every frame

**Measured and source-confirmed.** `SceneEvaluator::picture_job` sends images to `decode_image_cached` without its `DecodeLimit` (`src-tauri/src/export/mod.rs:925`). The cache returns a deep clone of a `VideoFrame` whose pixels are a `Vec<u8>` (`src-tauri/src/media_bin.rs:237`; `src-tauri/src/media/mod.rs:114`). A 3840×2160 still therefore copies 33.2 MB per layer per frame even on a 1280×720 preview. Four such layers represent 132.7 MB/frame before output readback and JPEG encoding. The current code does assign stable texture keys to still overlays: repeated GPU uploads are already avoided. The remaining problem is CPU copying, oversized decoded buffers and texture memory. The deep copy also occurs while holding the process-wide image-cache mutex, which can serialize preview/export cache hits.

**Optimization:** cache preview-sized still variants, keyed by file identity and effective sampling needs, and share immutable pixel buffers rather than cloning them. Preserve full-resolution export and zoom quality, and retain the newly implemented GPU texture caching. The new byte-bounded image cache fixes repeated decoding for a few images, but does not remove these per-frame CPU copies.

**Measured on this checkout:** 15 warmed scene evaluations at a 1280×720 preview size took 0.993 ms for one 720p still, 15.292 ms for one 4K still, 3.868 ms for four 720p layers and **42.412 ms for four 4K layers** (p95 52.944 ms). The decoder still returned 3840×2160 pixels despite the 720p limit. Deep-cloning one 4K `VideoFrame` averaged 7.199 ms; cloning an `Arc` handle was below the probe's displayed 0.001 ms precision. GPU composition of the already-built four-4K-layer scene averaged 2.468 ms with the existing texture cache. These are separate stages, not end-to-end playback FPS. Four layers use the same immutable image deliberately, showing avoidable copies even when only one image is decoded/cached.

### P02 — Noise-reduction initialization analyzes recording segments outside the used clip

**Measured and source-confirmed.** `PolishPlan::build` flattens every segment attached to every speech clip before checking for a denoiser (`src-tauri/src/media/polish.rs:255`). A recording clip holds the full recording stream's segment list. No intersection with `clip.in_us..clip.in_us+clip.len` is performed in this denoise pass. A short excerpt can therefore trigger analysis of a long recording that it never plays, synchronously during mixer creation on the playback worker.

**Optimization:** collect and deduplicate only segments intersecting audible clip source ranges before analysis. Prepare source analysis asynchronously where possible; retain completed analyses and derived plans across unrelated visual changes.

**Measured:** building noise reduction for a one-second clip took 2.435 ms with its one-second WAV. Keeping a completely unused 120-second WAV in the source's segment list made the same plan take **113.541 ms**; the unused file's subsequent analysis lookup took 0.153 ms because that unnecessary analysis was now cached. Both files were generated locally. This is initialization cost, not per-audio-chunk cost.

### P03 — The audio-analysis cache flushes all entries at 256 files

**Measured with repeat-hit verification.** `analyze_cached` calls `cache.clear()` when inserting beyond its 256-entry limit (`src-tauri/src/media/polish.rs:75`). A sequential working set of 257 distinct segments does not retain its analyses for the next traversal. The same file may be analyzed repeatedly while constructing denoise, ducking and normalization plans, and again after edits.

**Optimization:** retain analyses needed by the current mixer/plan and use a byte-aware eviction policy for reusable idle analyses. Simply replacing clear-all with a same-size LRU still thrashes a sequential 257-entry working set; the active set must survive the pass.

**Measured:** two sequential passes over 128 quarter-second WAV files retained all 128 analysis objects (93.517 ms cold, 5.934 ms repeat). At 257 files, **all 257 analysis objects were recomputed** on the repeat pass (201.967 ms cold, 167.136 ms repeat), verified by `Arc::ptr_eq` while retaining the original objects. Longer real segments make recomputation more expensive.

### P04 — Source-clock construction scales quadratically after many jump cuts

**Measured with scaling and integration probes.** The overlap fix in `src-tauri/src/sequence/clock.rs` scans all earlier disjoint source ranges in `unplaced` and allocates/rebuilds the complete merged range vector in `place` for each clip (`:55-92`). Contiguous source clips collapse to one range, but jump-cut clips preserve many gaps, making both routines grow with every clip. These clocks feed captions, transcripts, zoom ranges, chapters and short summaries.

**Optimization:** binary-search the first possibly overlapping interval, merge only neighboring ranges in place, and cache clocks per sequence revision and stream/role. Include a sorted-source fast path for the common jump-cut sequence and preserve first-occurrence semantics for reordered/overlapping clips.

**Measured:** source-clock construction for jump-cut clips averaged 2.571 / 8.023 / 28.268 / **114.878 ms** at 1,000 / 2,000 / 4,000 / 8,000 clips. At 8,000 contiguous source clips it was 3.141 ms. `EditDocument::zooms_with_ranges` constructs a clock again for each zoom, and following-short summaries construct one for each short, amplifying this cost. All measured sequences passed validation.

**Integration measurement:** the real `zooms_with_ranges()` on a validated document with 4,000 cut clips and 30 zooms averaged **826.627 ms** (maximum 880.578 ms). Mapping all 30 ranges using a single already-built clock took 0.260 ms. Constructing that clock once still costs approximately 28 ms on this fixture; the 0.260 ms comparison excludes that one-time cost. Reuse within a summary call is therefore valuable even before changing the interval algorithm.

### P05 — Multi-clip dragging performs quadratic searches before React renders

**Measured using the current frontend functions.** `moveClipDrag` calls `findClip` separately for every selected ID and rebuilds snap points on every pointer event (`front-end/src/components/timeline/TimelineStudio.tsx:964-972`). `findClip` linearly scans clips; `snapPoints` tests `ignore.includes(c.id)` for every clip (`:608`). Large selections therefore cost O(clips × selected clips). It then calls `setDrag`, rerendering the parent timeline and mapping every clip again; there is no animation-frame coalescing or viewport clipping of clip blocks (`:1369`). Concurrent changes add memoized clip components and preserve unchanged object identities, which helps child rendering but does not remove these searches or parent iteration.

**Measured:** 4,000 clips with 2,000 selected took **43.177 ms** per drag-math evaluation; 8,000 clips with 4,000 selected took **184.831 ms** (maximum 197.035 ms). An equivalent calculation with prebuilt ID-map and selected-ID-set lookups took 0.467 / 0.994 ms. The index construction is excluded because it can be cached once per revision/selection; the repeated snap-point construction is included. These timings exclude React, DOM/layout and pointer hit testing, so the UI performs additional work beyond this measured cost.

**Optimization:** memoize clip/asset/track indices and a selection set, cache eligible snap edges until the sequence/selection changes, and coalesce drag state updates to one animation frame. Render only visible clip blocks with overscan, keeping drag ghosts separate from the full timeline tree.

### P06 — Waveform backing canvases grow with the entire zoomed clip, not the viewport

**Verified by running the production geometry effect with canvas stubs.** The timeline can zoom to 64× (`front-end/src/components/timeline/timelineShared.tsx:12`) and sets its full content width accordingly. `WaveformRenderer` sizes the display canvas to the entire clip rectangle multiplied by device pixel ratio, then allocates two equally large cached canvases (`front-end/src/components/waveform/WaveformRenderer.tsx:59-64,94`). Offscreen portions are still included, and zoom/resizing rebuilds the buffers.

**Requested geometry:** with a 1,200 px timeline viewport, one full-length audio clip, 48 px waveform height and DPR 2, the three canvases request 2.64 MiB of RGBA pixel capacity at 1×, 42.19 MiB at 16× and **168.75 MiB at 64×**. At 64× each is 153,600×96 pixels. Multiple tracks multiply this request. These are calculated backing-pixel capacities from actual assignments, not measured browser RAM or proof that a particular webview successfully allocates that width.

**Optimization:** render only the visible time window into viewport-sized canvases, with small overscan or fixed-size tiles. Reuse tiles while scrolling, cap each backing surface, and retain the two-color caching strategy within those bounds. This also bounds the cost of the playback clear/draw operations.

### P07 — CPU JPEG transport can set the preview throughput ceiling

**Measured encoding; backpressure path confirmed in source.** Every webview frame goes through `encode_webview_frame` (`src-tauri/src/playback/preview.rs:156`) on one encoder thread. That thread is fed by a zero-capacity synchronous channel (`src-tauri/src/playback/engine.rs:49,331`). Moving encoding off the media worker allows overlap, but when encoding takes longer than frame production, sending the next frame blocks the media worker; audio replenishment also runs on that worker. Browser JPEG decoding/drawing adds further work after this stage.

**Measured with the production encoder:** 720p encoding averaged 12.192 ms for a flat frame and 21.697 ms for a detailed synthetic frame; 1080p averaged **27.927–50.239 ms** for those two cases. The detailed 1080p case alone exceeds a 30 fps frame budget. Its JPEG was 972,839 bytes, or roughly 29 MB/s at 30 fps before transport copies. These are deliberately simple low/high-detail fixtures, not claims about average recordings. The zero-capacity channel is bounded and helps prevent stale-frame buildup; replacing it with a deep FIFO would introduce latency rather than solve throughput.

**Optimization:** profile encoder time independently from decode/composite time in the app, choose transport resolution/rate to fit the frame budget, and benchmark alternative fast encoder implementations on representative screen recordings. Consider a bounded latest-frame handoff so a slow preview encoder cannot stall audio production, while deliberately dropping superseded preview frames. A GPU-backed or hardware-compressed presentation path is a larger follow-up; reducing GPU draw-call overhead alone will not remove this CPU encoding ceiling.

## Improvements already present

The new commits preserve playback evaluators across seeks, reuse WAV readers, binary-search active audio spans, avoid identical playback-status updates, isolate playhead subscriptions, cache still textures, and avoid cloning whole documents just to calculate independent-short lengths. Those earlier issues are not being reported as if they were unfixed. The concurrent changes also add memoized clip components and sequence structural sharing. P01, P04 and P05 identify remaining costs around those improvements.

## Follow-up candidate: decoder warm-up and eviction

Source inspection shows an eight-stream global FFmpeg cache (`src-tauri/src/media/ffmpeg.rs:23,877`) but no bound on distinct pending prefetch jobs (`:914-951`). `SceneEvaluator::prefetch` can visit multiple upcoming edges on multiple video tracks, and completed warm-ups enter the same eviction list as active decoders. At nine simultaneous video sources, the ready-cache capacity is also smaller than the working set. This can create process churn and evict sources needed for current playback. This was **not benchmarked** because FFmpeg/video fixtures were not available through this session's PATH; it is not included among the seven verified bottlenecks. Instrument process starts, active/pending counts and eviction reasons; then bound warm-ups and protect active sources before increasing cache capacity. Do not replace a memory limit with unlimited decoders.

## Validation and reusable probes

The probe files named below were removed on 20 Oct 2026 in a cleanup of `output/`. The
ongoing benchmark is `src-tauri/src/perf.rs`.

- `output/performance-probes-2026-10-04.rs`: five focused Rust tests completed successfully. They measure still resolution/copies/compositing, audio analysis overreach, analysis-cache reuse, clock scaling, repeated zoom clocks and JPEG encoding. They use generated temporary media, not user recordings.
- `output/performance-ui-probe-2026-10-04.cjs`: completed successfully; executes the production snap-point closure and sequence helpers, compares indexed lookups, and executes the waveform geometry effect against canvas stubs. The repeat run confirmed similar drag scaling (41 ms at 4,000/2,000 and 190 ms at 8,000/4,000).
- GPU timings used an NVIDIA GeForce RTX 3060. Rust used the repository's optimized development/test profile: application optimization level 1, dependencies level 3. The frontend probe used Node 22.19.0 and the installed TypeScript compiler. Runs were sequential to avoid overlapping cache/timing probes. Sample counts are small (3–30 depending on cost), and timings are illustrative rather than a statistical benchmark suite.
- The temporary integration-test shim was removed after verification. This pass did not modify application source or the concurrent edits. No full application test suite was run because this was a review with focused measurements, not a code fix. A pre-existing `InterestPoint` visibility warning appeared during compilation.

To rerun, create `src-tauri/tests/performance_review_probe.rs` with:

```rust
include!("../../output/performance-probes-2026-10-04.rs");
```

Then run from `src-tauri`:

```text
cargo test --offline --test performance_review_probe -- --nocapture --test-threads=1
```

Run the frontend probe from the repository root:

```text
node output/performance-ui-probe-2026-10-04.cjs
```

## Method and limits

This pass measured backend functions and frontend calculations, not a full Tauri recording/playback/export session. It did not collect browser flamegraphs, actual browser/GPU memory use, full export throughput, hardware audio underruns, or macOS-native performance. Synthetic clips and detailed images expose scaling behavior but may differ from typical projects. Existing application performance tests require a recording fixture via `AERO_PROBE`; none was supplied for this pass. Before shipping optimizations, validate representative recordings with p50/p95 frame time, dropped-frame count, seek latency, edit latency, peak memory and export frames/second. Recheck current line positions if the concurrent edits continue.
