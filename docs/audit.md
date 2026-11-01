# Performance and application bug audit

Last updated: 30 October 2026. Baseline: `f3233e0` plus the existing working-tree
changes. This is a living record of investigated defects, not a claim that the
whole app has been audited. All five findings of the first pass were fixed the
same day; see [Fixes](#fixes-30-october).

Evidence labels: **Code** means the relevant execution path was traced;
**Reproduced** means a controlled executable check exercised the production
logic. Synthetic checks are not desktop benchmarks. P1 means serious disruption
or incorrect edits; P2 means a narrower correctness or performance problem.
Keep IDs stable, add new evidence as it is found, and mark a finding fixed only
after its regression check passes.

## Open findings

None.

## Fixed findings

| ID | Priority | Finding | Evidence | Status |
|---|---|---|---|---|
| AUD-003 | P1 | A delayed layout edit can be saved to a different short | Reproduced; code | Fixed 30 Oct; regression test |
| AUD-001 | P2 | Thumbnail demand above the cache capacity never settles | Reproduced; code | Fixed 30 Oct; regression test |
| AUD-002 | P2 | Switching projects can retain the previous project's edit points | Reproduced; code | Fixed 30 Oct; no automated test |
| AUD-004 | P2 | Clearing search does not invalidate an in-flight result | Reproduced; code | Fixed 30 Oct; regression test |
| AUD-005 | P2 | Every search synchronously reloads transcripts under the project lock | Code; synthetic timing | Fixed 30 Oct; unit tests + timing |

The sections below keep the original analysis; each ends with what was changed.

### AUD-001: Visible thumbnails are evicted and immediately requested again

- **Location:** [ClipThumbs.tsx](../front-end/src/components/timeline/ClipThumbs.tsx),
  `KEEP`, `flush`, and `schedule` (lines 12, 42–86).
- **Trigger:** More than 2,000 distinct thumbnail keys remain wanted by mounted
  clips. Dense timelines can mount many short clips, each requesting a picture;
  the 400-slot limit is per clip, not a global limit.
- **Cause:** `flush` loads missing wanted keys, then unconditionally removes the
  oldest entries until `made.size <= KEEP`, including keys still wanted. Every
  completion schedules another pass, which asks for the evicted visible keys.
  There is no possible settled cache state when demand exceeds capacity.
- **Impact:** Repeated IPC, disk reads and base64 transfers while the timeline is
  idle, plus a global subscriber notification on every completion. Disk caching
  avoids repeat decoding of successfully cached pictures; it does not stop this
  loop. Thumbnails can disappear and return as their entries rotate.
- **Evidence, 30 Oct:** Executed the actual module's cache/queue code, transpiled
  with the installed esbuild, with a stub thumbnail API and controlled timers.
  With 2,001 wanted keys, 101 requests fetched 2,070 pictures for only 2,001 unique
  keys. The last ten batches each fetched one picture, another timer remained
  pending, and only 2,000 keys were retained. No wall-clock speed claim is made.
  Controls with 1,999 and 2,000 keys each settled after 32 requests; clearing
  wanted keys stopped the 2,001-key loop without another request.
- **Fix direction:** Keep wanted entries resident and evict only unused entries,
  or impose a global visible-thumbnail budget and stop requesting beyond it.
  Notify only clips whose results changed if large-timeline redraw cost remains.
- **Regression check:** Demand just below, at, and above capacity; drain the queue
  and require no further API calls without a viewport/source change. Then scroll
  away and verify unused entries can be reclaimed.

### AUD-002: Edit-point memoization omits project identity

- **Location:** [TimelineStudio.tsx](../front-end/src/components/timeline/TimelineStudio.tsx),
  `edges` (line 536), `jumpToEdit`, and `rippleTrim`;
  [DockWorkspace.tsx](../front-end/src/components/layout/DockWorkspace.tsx),
  `Timeline`; [App.tsx](../front-end/src/App.tsx), `openPath`.
- **Trigger:** Open project B directly from project A in the same workspace, where
  both have the same revision and `shortView` (for example two new projects at
  revision 0), but different clip boundaries.
- **Cause:** `edges` depends on revision and short view only. The workspace and
  timeline stay mounted when the project changes. Other timeline indexes follow
  the new sequence, but this memo can keep A's boundaries until B's revision
  changes.
- **Impact:** Previous/next-edit navigation can seek to A's cut points in B. The
  seek after a previous-side ripple trim can also use A's boundary; the backend
  still computes the actual trim against B, so this finding does not assert that
  the backend trims using the stale edge list.
- **Evidence, 30 Oct:** Evaluated the exact production memo expression with React's
  dependency comparison behavior. A at revision 0 had edges `[0, 5000000,
  10000000]`; B at revision 0 had `[0, 2000000, 20000000]`. The memo returned A's
  edges after switching to B. Desktop interaction remains to be verified.
- **Fix direction:** Depend on the sequence object (already structurally shared)
  or include project identity along with the existing dependencies.
- **Regression check:** Switch directly between equal-revision projects with
  different cuts, then exercise previous/next edit before making any edit in B.

### AUD-003: Debounced layout changes follow the new short instead of their owner

- **Location:** [useDebouncedSave.ts](../front-end/src/hooks/useDebouncedSave.ts),
  `schedule` (lines 26–37);
  [LayoutsSection.tsx](../front-end/src/components/layouts/LayoutsSection.tsx),
  `useLater`, `editLayout`, and `setPick`;
  [layoutsStore.ts](../front-end/src/stores/layoutsStore.ts), `run` and
  `changeLayouts` (lines 41–59).
- **Trigger:** Change short A's base-layout slider, then select short B within
  the 200 ms debounce period and let B's view arrive before the save fires.
  Both shorts belong to the same project.
- **Cause:** The debounce captures only `projectHandle`. The delayed closure
  calls `changeLayouts`, which reads the *current* `openedProject.shortView`.
  Switching shorts passes the handle check, so the call targets B. The short
  save queue protects the owner captured at enqueue time, but by then the
  delayed callback has already selected B as its owner.
- **Impact:** A's base-layout edit changes B instead. Layout-picking settings use
  the same delayed route. A section-specific update can instead fail if its ID
  does not exist in B. This is an incorrect-edit risk, not just a stale preview.
- **Evidence, 30 Oct:** Executed the production debounce with controlled hook
  refs/timers, plus bundled production `layoutsStore`, `shortsStore`, and
  `projectStore`. Scheduled a base-layout change while `shortView = A`, changed
  the store to B under the same handle/revision, then fired the timer. The mocked
  backend received `projectLayoutsChange(..., shortId: "B", {op: "base", ...})`.
  No real project was written.
- **Fix direction:** Capture both project and timeline identity when scheduling
  a timeline-specific save. Either save explicitly to that original timeline
  using the latest project revision, or flush it before changing views. Do not
  resolve the destination from whichever short happens to be visible later.
- **Regression check:** Edit A and immediately select B, leave Shorts for the
  video, and switch projects. Assert that B/the video never receives A's change;
  define and test whether the original change is flushed or deliberately canceled.

### AUD-004: A completed search can restore results after the query is cleared

- **Location:** [SearchPanel.tsx](../front-end/src/components/review/SearchPanel.tsx),
  search effect (lines 47–64).
- **Trigger:** Type at least two characters, allow the 200 ms timer to start the
  request, then clear the field or shorten it to one character before it returns.
- **Cause:** The short-query branch clears results and returns before incrementing
  `asked.current`. Cleanup only clears the debounce timer, which cannot invalidate
  a request already sent. The old response still passes the request-ID check.
- **Impact:** Results for the old query reappear under an empty/one-character
  search field. A late failure can likewise set an irrelevant error.
- **Evidence, 30 Oct:** Executed the exact production effect with controlled
  timers and a deferred search promise. Clearing the query set results to `null`;
  resolving the old promise then restored `{total: 1, hits: [...]}`.
- **Fix direction:** Invalidate the request generation on every effect run or
  cleanup, including early-return branches. Clear stale errors when clearing
  the query as well.
- **Regression check:** Resolve and reject an old request after clearing or
  shortening the query; results must remain empty and no stale error may appear.

### AUD-005: Search does disk reads and full transcript processing in a blocking command

- **Location:** [lib.rs](../src-tauri/src/lib.rs), `source_search` (lines 1197–1215);
  [source_search.rs](../src-tauri/src/source_search.rs), `search` (lines 188–275);
  [transcript/store.rs](../src-tauri/src/transcript/store.rs), `load_transcript`
  (lines 49–70); [SearchPanel.tsx](../front-end/src/components/review/SearchPanel.tsx),
  search effect.
- **Trigger:** Search a project with large or numerous saved transcripts. Each
  typing pause of at least 200 ms can issue another request; project revisions
  and transcript changes also rerun the search.
- **Cause:** The Tauri command is synchronous and retains `opened_project.lock()`
  throughout `source_search::search`. Every request opens, reads, deserializes and
  validates every eligible transcript, normalizes all its words, maps matches,
  and allocates the complete hit list before truncating it to the requested limit.
  There is no search index or cancellation of work superseded by a newer query.
- **Impact:** File I/O and CPU work block the command handler and hold off other
  operations requiring the project lock. Returning at most 100 hits does not
  bound the search work or temporary hit allocations. A small result limit is
  therefore not a safeguard against long-recording stalls.
- **Evidence, 30 Oct:** Traced the full command-to-file-read path and the installed
  `tauri-macros` 2.7.1 implementation matching `Cargo.lock`: an ordinary synchronous
  command defaults to `ExecutionContext::Blocking` and invokes the function
  inline. In contrast, this app's `transcript_get` already uses
  `tauri::async_runtime::spawn_blocking` for long-transcript work. This is a code
  finding; no search duration, UI freeze duration, or memory peak was measured.
- **Fix direction:** Snapshot the needed project data under the lock, release it,
  and do search on a blocking worker. Cache normalized transcripts with explicit
  invalidation on transcript changes, avoid retaining every formatted hit just
  to return the first 100, and discard/cancel obsolete searches.
- **Regression check:** With a large synthetic transcript fixture and multiple
  streams, measure cold/warm search latency and project-lock hold time while
  issuing successive queries. Confirm that edits/status handling remain
  responsive, only the latest query is displayed, and transcript edits invalidate
  the index. Verify exact-before-fuzzy ordering and total-hit counts survive
  bounded result storage.

## Fixes, 30 October

- **AUD-001:** `flush` in [ClipThumbs.tsx](../front-end/src/components/timeline/ClipThumbs.tsx)
  now evicts only pictures no mounted clip wants, oldest first, until at most
  `KEEP` remain. Wanted pictures stay resident even above `KEEP`, so demand above
  capacity settles; unwanted ones are reclaimed on the next load (a scroll to new
  clips). Global notification per batch is unchanged.
- **AUD-002:** `edges` in [TimelineStudio.tsx](../front-end/src/components/timeline/TimelineStudio.tsx)
  now depends on the (structurally shared) `sequence` object, like `clipIndex`
  and `clipEdges` beside it.
- **AUD-003:** [useDebouncedSave.ts](../front-end/src/hooks/useDebouncedSave.ts)
  captures the project handle *and* `shortView` when scheduling. It subscribes to
  the project store and flushes a waiting save as soon as the window asks for
  another timeline (`viewShort` changes) or another project/view is shown, while
  the original timeline is normally still loaded. At fire time the save runs only
  if the same project and timeline are shown; otherwise it is dropped, never
  redirected. Defined behaviour: a pending change is **flushed** on a short
  switch or leaving Shorts for the video; it is **dropped** when another project
  opens or the other view is already in place. This covers every
  `useDebouncedSave` user (layouts, captions, audio, zoom, inspector, export).
- **AUD-004:** the [SearchPanel.tsx](../front-end/src/components/review/SearchPanel.tsx)
  effect increments the request generation on every run, including the short-query
  branch, which also clears any error.
- **AUD-005:** `source_search` in [lib.rs](../src-tauri/src/lib.rs) is now async:
  it clones the current document and root under the lock, releases it, and
  searches with `spawn_blocking`. [source_search.rs](../src-tauri/src/source_search.rs)
  caches each transcript's words and normalized forms per file, invalidated
  explicitly by `save_transcript`/`delete_transcript` (`forget`) and by a size/mtime
  stamp for writers in other processes; only the last-searched project's entries
  are kept. Hits past the limit are counted, never formatted (`Kept`), keeping
  exact-before-fuzzy order and the true total. A newer search from the same
  window stops one still running between streams (`Latest`); the panel already
  ignores its "replaced" answer. The MCP `source_search` tool keeps its locked,
  synchronous path but gains the cache and bounded hits.

Verification:

- `npm run test:cleanup` (front-end): **32 passed**, 0 failed. New
  [auditFixes.test.ts](../front-end/tests/auditFixes.test.ts) runs the real
  components/hooks through a small hook harness ([hooks.ts](../front-end/tests/hooks.ts))
  with fake timers: AUD-001 with ~2,700 wanted keys (settles, no idle requests,
  reclaims after scrolling away), AUD-003 (switch short, switch view, leave for the
  video, switch project, plus `changeLayouts` reaching the backend with short A),
  AUD-004 (resolve and reject after clearing and after shortening). With the three
  original source files restored, all four new tests fail; with the fixes they pass.
- `npx tsc --noEmit`: clean.
- `cargo test --lib -- source_search transcript::store`: 8 passed (new: hit
  bounding/order/total, cache reuse and same-length save invalidation, per-window
  replacement). `cargo check --features tauri-app`: builds. `cargo fmt --check`: clean.
- Ignored `search_cost` test (100,000-word synthetic transcript, test profile):
  cold read + search 121 ms, warm 1.0–1.2 ms, 12,500 hits counted, 100 kept.
  Not a desktop measurement; project-lock hold time is now a document clone.
- Not done: AUD-002 has no automated test (the change is a dependency list; check
  previous/next edit after switching between equal-revision projects in the app),
  and no live desktop run of any fix.

## Scope and verification

Completed the first targeted pass over frontend project lifecycle, timeline
navigation and thumbnails, debounced settings saves, source search, and their
backend entry points. Existing performance observations remain in
[gaps.md](gaps.md#12-performance); they are not new measurements from this audit.

- Four controlled Node checks reproduced AUD-001 through AUD-004 using the current
  production source, mocked API responses, and controlled timers/hook state. They
  were run in memory; they neither launched the editor nor changed recordings.
- `cd front-end; npm run test:cleanup`: **28 tests passed**, zero failures. These
  existing tests do not cover the four newly reproduced cases. In particular,
  `shortsQueue.test.ts` protects a short operation *after* enqueueing; AUD-003
  occurs in the earlier debounce phase.
- `.venv/Scripts/python.exe scripts/check_doc_links.py`: one existing broken link,
  `docs/research/synthesis.md` → `../requirements.md`. This unrelated document was
  not changed. The same checker with `--path docs/audit.md` passed: 13 relative
  link targets, zero broken links.
- No application fixes, full native build, live desktop reproduction, export
  validation, or macOS run was performed. The code-only performance finding
  needs a representative timing/memory run before assigning a measured budget.

## Next audit targets

These are investigation targets, not confirmed defects:

- Flush/ownership behavior of pending settings saves when closing a project,
  switching workspaces, or undoing while a save is in flight.
- Thumbnail queue cancellation and cold-cache load time on a dense timeline;
  global thumbnail notifications versus actual affected-clip redraws.
- Playback/export behavior across relinked media and repeated imports, including
  cache invalidation and memory retention during long editing sessions.
