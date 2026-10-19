# Application audit — 2026-10-18

Status: completed. Findings were appended as they were verified. **9 findings: 6 correctness issues and 3 performance improvements; 2 P1, 5 P2, 2 P3.**

Scope: the current working tree of AeroEdits, including existing uncommitted changes. This audit records bugs and concrete performance improvements; it does not change application code.

Severity: P1 = major broken workflow or data-loss risk; P2 = meaningful correctness or performance issue; P3 = smaller issue or optimization.

Start with F02 and F03: both can let export proceed without the exact graphic renders it needs. F04, F07, and F08 have isolated state reproductions. Each item below distinguishes executed evidence from source inspection.

## Findings

### F01 — P2: A late graphic response can populate the inspector for a different clip

- **Location:** `front-end/src/components/inspector/GraphicSection.tsx:41-51,63-77,119-180`.
- **Trigger:** Select graphic A, then B while A's load or save is still pending; let A's response arrive last. Even without reordered replies, A's fields remain visible while B loads.
- **Impact:** The inspector displays A's name/variables while its save callback targets B's `assetId`. Editing a shared field can overwrite B with a value based on A. A late save response has the same problem.
- **Evidence:** Both load and save call `setGraphic` unconditionally. Selection changes do not clear the old graphic, there is no request generation/identity check, and rendering does not require `graphic.assetId === assetId`. Verified by tracing the selection, response, and save paths; native UI reproduction pending.
- **Recommendation:** Clear or hide stale fields on selection changes; bind requests to project handle and asset ID, discard late responses, and refuse saves unless the displayed identity matches the selection.

### F02 — P1: Export skips identical graphics stored under different assets

- **Location:** `src-tauri/src/graphics/render.rs:468-474` (also queue deduplication at `332-344`); `src-tauri/src/export/mod.rs:924-957,2019-2025`.
- **Trigger:** Add the same template twice with the same name, variables, duration, and canvas, then export before both assets have separate cached renders.
- **Impact:** `ensure` prepares only one asset's output and reports success. The other asset can export a placeholder or stale render while rendering is started in the background.
- **Evidence:** The content hash excludes asset ID/path, so identical sources produce identical filenames. Deduplication compares only that filename, but output is stored in a separate `spec.cache` directory for each asset. An isolated Rust harness executing the actual `ensure` function with a stub renderer returned `Ok` with the first asset cached and the second absent. The export evaluator permits `Lookup::Pending` and `Lookup::Stale`. This verifies cache logic, not a full HyperFrames export.
- **Recommendation:** Deduplicate by full destination path, or use a shared content-addressed cache and explicitly materialize/reuse its output for every asset. Export should require the exact prepared render.

### F03 — P1: Rendering one graphic duration deletes another duration still needed by the edit

- **Location:** `src-tauri/src/graphics/render.rs:573-579`; `src-tauri/src/export/mod.rs:924-957,2019-2025`.
- **Trigger:** Place one graphic asset in two clips of different lengths on the same canvas, or use different lengths in the main edit and a short.
- **Impact:** Rendering the second length removes the first length's cache. Export preparation can finish with required files already missing; playback/export then use a differently timed stale animation or repeatedly render the missing version. This also wastes substantial CPU/GPU time.
- **Evidence:** `specs_for` and the filename distinguish clip lengths, but successful render cleanup deletes every other `.webm` with the same width/height prefix. An isolated Rust harness executing the actual `ensure` function and cleanup block returned `Ok` for 4-second and 8-second specs, with only the 8-second file remaining. `latest` explicitly accepts a different duration. The harness stubs rendering; it does not exercise HyperFrames itself.
- **Recommendation:** Retain all versions referenced by active documents/exports; garbage-collect unused entries separately. At minimum include duration and FPS in the cleanup grouping and pin files during export.

### F04 — P2: Switching sound during noise detection leaves the panel permanently busy

- **Location:** `front-end/src/stores/noiseStore.ts:56-77`; `front-end/src/components/cleanup/NoisesPanel.tsx:151-173,336`.
- **Trigger:** Start Find noises, then select a different sound before the scan finishes.
- **Impact:** The panel stays on “Listening…” and disables Find noises and bulk mute for the remainder of the webview session. Closing/reopening the panel does not reset the module-level store.
- **Evidence:** Executed the actual bundled Zustand store in Node: `begin(); chooseSound('second.sound0'); finish(oldGeneration, result)` leaves `busy === true`. `chooseSound` invalidates the old generation but does not clear busy; both finish/fail ignore that generation. The sound selector remains enabled during detection.
- **Recommendation:** Reset busy when invalidating a scan, and track the request context separately if the old scan continues in the backend. Add a regression test for switching sources mid-scan.

### F05 — P2 performance: Transcript playback still scans and reconciles the entire transcript

- **Location:** `front-end/src/components/transcript/TranscriptPanel.tsx:379-385,791-819`.
- **Trigger:** Play a long recording with the transcript panel open, especially near its end or during a gap with no active word.
- **Impact:** Both store selectors linearly search all words/lines on every project-store update. Each active-word change also builds React elements for every transcript line, and all words remain mounted. Memoized line components reduce child rendering but do not remove the searches, parent traversal, or large DOM.
- **Evidence:** Source inspection: two `findIndex` selectors and an unvirtualized `lines.map`. For N words and L lines, each playback update can visit N + L entries; each word change traverses L rows. No wall-clock slowdown claim is made without a browser profile.
- **Recommendation:** Build a time-sorted lookup index on transcript changes, use binary search or an advancing playback cursor, and virtualize visible lines while preserving selection and active-word scrolling. Measure a one-hour transcript before and after.

### F06 — P3 performance: Both windows and every editor panel ship in the initial JavaScript chunk

- **Location:** `front-end/src/main.tsx:3-4,25`; `front-end/src/components/layout/DockWorkspace.tsx:12-23`.
- **Trigger:** Launch either the editor or the Shorts Studio, including opening the welcome screen without a project.
- **Impact:** Startup loads/parses code for both window modes and all panels, including features the user has not opened.
- **Evidence:** Production build passed but emitted a **935.82 kB minified / 250.94 kB gzip** main JavaScript chunk and Vite's chunk-size warning. `App` and `ShortsStudio` are static imports; dock panels are also eagerly imported. Startup latency itself has not been benchmarked.
- **Recommendation:** Lazy-load the selected window root and expensive optional panels/dialogs with explicit loading fallbacks. Rebuild and compare initial bytes and cold startup time before changing vendor chunking.

### F07 — P2: Deleting the only short leaves a deleted short stuck in Shorts Studio

- **Location:** `front-end/src/stores/projectStore.ts:262-272`; `front-end/src/components/shorts/ShortsStudio.tsx:245-246,1048-1054`; `src-tauri/src/project/reader.rs:1012-1018`.
- **Trigger:** Create one short, select it, then delete it in Shorts Studio.
- **Impact:** The backend removes the short, but the studio retains its old short list and revision. Further edits target a nonexistent short or stale revision; the successful deletion never reaches the visible state.
- **Evidence:** Executed the actual bundled project store with a successful deletion response (`revision: 2`, `shorts: []`). It requests the deleted short's view, swallows the resulting rejection, and leaves revision 1 with `shorts: ['s1']`. The UI clearing `selectedId` does not help because selection falls back to the first item in that stale list.
- **Recommendation:** Check whether the viewed short still exists before requesting its view. If absent, clear the short view/edit target and apply the new project (or select a surviving short). Handle conversion failures visibly.

### F08 — P2: The Shorts Studio save queue advances before the new revision is applied

- **Location:** `front-end/src/components/shorts/ShortsStudio.tsx:333-353`; `front-end/src/stores/projectStore.ts:266-272`.
- **Trigger:** Queue two shorts changes while the short view is active, such as a pending look change followed by Pick scenes, or two rapid renames.
- **Impact:** The next save can reuse the previous revision and fail with “Stale edit revision,” losing the later requested change despite the code's promise to serialize saves.
- **Evidence:** `saveShorts` waits for `projectShortsSet`, but `applyOpenedProject` starts a second asynchronous `projectShortView` request and returns immediately. The real-store probe confirmed that applying revision 2 still exposes revision 1 until that extra request resolves. The queued callback reads that exposed revision.
- **Recommendation:** Make application of an edit result awaitable and await it inside the queue, or advance the authoritative project revision synchronously while fetching the short's presentation separately. Test two queued edits with a delayed short-view response.

### F09 — P3 performance: Graphic thumbnail images accumulate for the lifetime of the window

- **Location:** `front-end/src/components/graphics/GraphicsPanel.tsx:370-371,400-417`.
- **Trigger:** Change graphic text/colours, duration, or position repeatedly, or open successive projects in one editor session.
- **Impact:** Every distinct thumbnail key retains another base64 PNG string in a module-level `Map`. Old revisions, removed graphics, and closed projects are never evicted, so memory use grows with editing history rather than the number of visible graphics.
- **Evidence:** The only cache operations are `get` and `set`; there is no limit, delete, clear, or project-switch cleanup. Keys include the project handle and serialized variable values. This is a verified unbounded-retention path, not a measured memory total.
- **Recommendation:** Bound the cache by image bytes or an LRU entry count; discard closed-project entries and obsolete variants. Keep only visible/recent thumbnails and measure retained heap across repeated edits.

## Validation and coverage

- `npm run build`: passed (TypeScript and Vite); bundle-size result recorded in F06.
- `npm run test:cleanup`: all 6 tests passed.
- Executed an isolated probe against the real `noiseStore.ts` bundle; reproduced F04.
- Actual project-store probes reproduced F07 and the delayed revision application in F08.
- Isolated Rust harness reproduced F02 and F03 using the source's `ensure` function and cleanup block with a stub renderer.
- `cargo check --features tauri-app --offline`: passed on Windows; warnings about incremental-cache hard links and the visibility of `InterestPoint`.
- Initial full Rust run: 364 passed, 85 failed, 36 ignored. Many failures were access-denied errors in the default temporary directory, plus local network/pipe restrictions. Re-running with temporary files inside the workspace cleared many filesystem failures, but that parallel run terminated with `STATUS_INVALID_HANDLE`. A serial run also terminated with that status while executing `export::tests::captions_render_from_the_transcript_in_edited_time`. The underlying cause of that process exit is unresolved; it is **not** counted as a verified application defect.
- Targeted Rust tests with temporary files inside the workspace: **57 passed, 0 failed, 1 ignored**, covering graphics commands, graphics render/cache tests, edit persistence/revisions, sequence editing, export checks, and transcript storage. Each group ran in a separate process. This does not substitute for a passing full suite.
- Logs and the isolated Rust cache probe are under `output/audit-2026-10-18/` (ignored build/audit output). The cache probe uses the actual preparation/cleanup code with stubbed render output, so it requires no runtime download or media rendering.

Reviewed paths: frontend project state, asynchronous saves and window synchronization, Shorts Studio, transcript rendering, noise cleanup, graphics inspector/library thumbnails, export UI; backend graphics preparation/cache, export evaluation, playback, media import, edit persistence, transcript storage, and relevant tests/build configuration.

Limitations: no macOS execution, full native UI session, complete HyperFrames export, cloud-provider integration, or production performance profile. GPU/media/IPC behavior and the unexplained test-process termination still need verification in an unrestricted test environment. This is a source audit with targeted execution, not a guarantee that unreported paths are defect-free.

## Verification

Each finding was checked against the working tree on 2026-10-18 before anything changed. Where a test is listed, it was run against the old code first and failed, except where noted.

| Finding | Verdict | Fix |
| --- | --- | --- |
| F01 inspector race | Confirmed (small window: replies are local IPC) | Fixed |
| F02 identical graphics | Confirmed | Fixed, test |
| F03 render deletes other lengths | Confirmed | Fixed, test |
| F04 noise panel stuck busy | Confirmed | Fixed, test |
| F05 transcript playback scan | Real but not noticeable | Left |
| F06 initial chunk size | Real, but splitting gains little | Left (measured) |
| F07 deleted short stuck | Confirmed, wider than reported | Fixed, test |
| F08 save queue ordering | Confirmed | Fixed, test |
| F09 thumbnail accumulation | Partly right: real, but grows slowly | Fixed (bounded) |

- **F01:** Fields are drawn only for the selected graphic: `graphic` is derived from the loaded view and is undefined unless its `assetId` matches the selection (`front-end/src/components/inspector/GraphicSection.tsx:37`). Load and save replies go through `take` (`:41`), which drops a reply for a graphic or project that is no longer selected. There is no component test harness in the front-end, so this one has no automated test.
- **F02:** `ensure` deduplicated by file name, but each asset has its own cache folder. The new `missing` deduplicates by full path (`src-tauri/src/graphics/render.rs:505`). A second graphic with the same source copies the first one's render instead of rendering again (`copy_render`, `:518`, used in `ensure` at `:466`). The background queue's deduplication now also compares asset ids (`:336`). Test: `the_same_template_twice_renders_into_both_caches` (`:942`). It was written after `missing` replaced the old loop, so it was never run against the old code. The old loop's `any(|n| n == name)` check yields one entry for this input.
- **F03:** Cleanup after a render now removes only older renders of the same canvas, length *and* frame rate. Other lengths stay, capped at the newest `KEPT_LENGTHS` (8) per canvas so trimming does not fill the disk (`remove_superseded`, `render.rs:611-636`). Test: `a_render_keeps_the_other_lengths_on_its_canvas` (`:970`). It failed against the old cleanup, which left only the 8-second render.
- **F04:** `chooseSound` now clears `busy` along with invalidating the generation (`front-end/src/stores/noiseStore.ts:57`). Test: `front-end/tests/noiseStore.test.ts`.
- **F05:** The two `findIndex` selectors are real (`TranscriptPanel.tsx:379`), but playback status is polled every 50 ms (`front-end/src/hooks/useTimeline.ts:7`). Measured in Node with a one-hour transcript (9,000 words, 750 lines) at the worst case (a gap at the end): **~110 µs per poll for both selectors, ~2 ms per second of playback**. The full `lines.map` with memoized rows runs only when the active word changes (about 3 times a second). This is not noticeable, and virtualizing would put selection and scrolling at risk for no visible gain. Left.
- **F06:** The build matches the report (935.82 kB / 250.94 kB gzip before any change). Lazy-loading the two window roots was built and measured: the editor's initial JS went from 936 kB to 912 kB (−2.5%), and the Shorts window's to 754 kB. Both windows share one 551 kB chunk (dock, timeline, preview, vendor). The app loads from local disk, so the saving is a few milliseconds of parse time. That doesn't justify an extra load step and a split CSS chunk. Reverted and left.
- **F07:** It affects more than deleting the only short. Deleting the first short in the list also hit it, because the selection falls back to `shorts[0]`, which is still the deleted one. `applyOpenedProject` now applies the project and clears `viewShort` when a same-or-newer revision no longer has the short on show, instead of requesting a view that cannot exist (`front-end/src/stores/projectStore.ts:268`). Test: `front-end/tests/shortsView.test.ts` ("removing the short on show…").
- **F08:** `applyOpenedProject` now returns a promise that settles once the short's view is applied (`projectStore.ts:277`). The Shorts Studio save queue and `run` await it (`front-end/src/components/shorts/ShortsStudio.tsx:320,347`), so the next save reads the new revision. Test: `front-end/tests/shortsView.test.ts` ("applying an edit to a short's view…").
- **F09:** The growth is real but slow: one ~224 px PNG data URL of some tens of kB for each settled edit. The cache now keeps the latest 64 entries (`front-end/src/components/graphics/GraphicsPanel.tsx:373-382`, used at `:426`).

Checks after the fixes:
- Passed: `tsc --noEmit`, `npm run test:cleanup` (9 passed: 6 existing, 3 new), `npm run build`, `cargo test --lib graphics::render` (7 passed, 1 ignored) and `rustfmt --check` on `render.rs`.
- Not run: the full `cargo test` and `cargo clippy --all-targets`. The crate did not compile at the time because of another change still in progress in the same working tree (`src/window_crop.rs:294` uses the private `zoom::eval`; the `SceneEvaluator` initialisers in the `export/mod.rs` tests lack `bubble_hidden`/`window_crops`). Neither error is in a file touched here. Re-run both once that work compiles.
