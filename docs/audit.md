# Repository audit

Date: 2026-10-24

Status: complete. Each finding was written when confirmed; validation details were updated as checks completed. No application fixes were made during this audit.

Scope: current working tree, including existing uncommitted changes. Review covers correctness, performance, and other actionable engineering risks. This is a source audit with targeted checks, not a guarantee that every defect has been found.

Severity: P1 = high impact; P2 = normal priority; P3 = low priority.

Six findings: **one P1 and five P2**, including one performance issue. Address **A05** first because it can overwrite source media. Findings remain in discovery order. All six were fixed on 2026-10-24 (see each finding's **Fixed** line).

## Findings

### A01 — P2 — Delayed export preferences can be written to a different project

- **Category:** correctness / unintended edit.
- **Location:** `front-end/src/components/export/ExportDialog.tsx:49–64` (`savePreferences`).
- **Trigger:** change export quality in project A, close the dialog, and open project B before the 500 ms timer fires (including a project opened by an agent).
- **Evidence:** the module-level timer captures A's preferences but obtains `openedProject` only inside its callback. It therefore submits B's handle and current revision together with A's preferences. There is no project identity guard or unmount cleanup.
- **Impact:** B's saved export settings are silently changed; A's change is not saved. The request passes revision checking because it uses B's current revision.
- **Recommendation:** capture the originating project handle when scheduling and verify it before saving, or use the existing project-scoped `useDebouncedSave` hook.
- **Validation:** executed the actual `savePreferences` function in a Node/TypeScript harness with a controlled timer. Scheduling for A and switching to B before firing produced `projectExportSet("B", 9, { resolution: "4K", fps: 60, quality: "max", mbps: 16 })`.
- **Fixed (2026-10-24):** the dialog saves through `useDebouncedSave`, which drops a save once another project is open and sends a waiting one when the dialog closes.

### A02 — P2 — Concurrent WAV exports share and truncate the same temporary file

- **Category:** correctness / export corruption and data loss.
- **Location:** `src-tauri/src/export/files.rs:173–175, 205–209`; `src-tauri/src/commands/mod.rs:2606–2690`; `src-tauri/src/lib.rs:2167`.
- **Trigger:** start two audio-file exports to the same destination, for example from the UI and an agent, or reopen the export dialog while a WAV save is still running and save again.
- **Evidence:** WAV output always uses `path.with_extension("wav.part")` and `File::create`, which truncates an existing file. The command releases `command_lock` before mixing/writing, runs on `spawn_blocking`, and does not use the video export's busy guard. Both writers can therefore open the same partial file; either can rename it or remove it during the other's operation.
- **Impact:** overlapping writes can produce a mixed/corrupt WAV or a failed export. An unrelated pre-existing `.wav.part` file is also overwritten without being the requested destination.
- **Recommendation:** create a unique temporary file beside the destination using `NamedTempFile`, and coordinate publication for requests targeting the same output.
- **Validation:** source trace confirms concurrent entry points and unconditional creation/cleanup of the shared path. No user media was modified to reproduce this.
- **Fixed (2026-10-24):** `write_wav` writes to its own `NamedTempFile` beside the destination (removed on failure) and persists it over the destination. Two saves to one place now each write a whole file; the last to finish wins.

### A03 — P2 — A timeline range selection carries over when switching shorts

- **Category:** correctness / unintended edits.
- **Location:** `front-end/src/components/timeline/TimelineStudio.tsx:547–558, 844–847, 907`; `front-end/src/components/layout/DockWorkspace.tsx:71–83`.
- **Trigger:** select a time range in short A, then select another short B in the same project whose duration still contains that range.
- **Evidence:** the timeline component remains mounted between shorts. Its range reset depends only on `projectHandle`; the duration/revision effect clears only out-of-bounds ranges. Changing `shortView` therefore leaves A's range selected in B and publishes it as B's `timelineSelection`.
- **Impact:** Delete, range-based zoom, layout, or footage placement can act on an inherited time interval in the newly selected short.
- **Recommendation:** reset range and shared timeline selection when the timeline identity changes, including `shortView`, or key the timeline component by project and short.
- **Validation:** traced the persistent component, short-selection transition, and delete action. Executed the actual range-effect source with a dependency-aware hook harness: switching from a 40-second short A to a 60-second short B retained A's 2–4 second selection.
- **Fixed (2026-10-24):** the range (and the edit error) reset when `shortView` changes as well as the project; the shared selection follows.

### A04 — P2 — Edits restart pending waveform reads without cancelling the old work

- **Category:** performance / redundant audio analysis.
- **Location:** `front-end/src/components/timeline/TimelineStudio.tsx:1106–1128`; `src-tauri/src/commands/mod.rs:966–980`; `src-tauri/src/project/waveform.rs:347–389`.
- **Trigger:** edit or adjust settings repeatedly while a long recording's initial waveforms are still loading.
- **Evidence:** every project revision constructs a new `soundKeys` Map, rerunning the waveform effect even when the sounds are unchanged. Cleanup merely sets `active = false`, discarding the previous result; until a result is stored, each new effect starts another request for every missing waveform. Backend cancellation checks `waveform_epoch`, which ordinary edits do not increment, and each request can launch up to eight analysis workers.
- **Impact:** overlapping reads, hashing, and waveform construction consume CPU and disk bandwidth during editing. Completed results from superseded effects are discarded, delaying the waveform display further.
- **Recommendation:** deduplicate in-flight reads by project and sound, preserve successful results across unrelated revisions, and make the effect depend on stable sound identities. Add backend coalescing or cancellation if independent callers can request the same analysis.
- **Validation:** executed the actual memo/effect source with a dependency-aware hook harness. Three revisions with the same sound produced three simultaneous requests; only one of their three successful results was stored. Backend source confirms epoch cancellation and worker fan-out. No wall-clock performance benchmark was performed.
- **Fixed (2026-10-24):** the effect depends on the sorted list of sound keys (not a Map rebuilt each revision); one read per project and sound is in flight at a time, and its result is stored whenever that project is still open. No backend coalescing was added: the timeline is the only caller.

### A05 — P1 — Case-variant export paths bypass source-media protection on Windows

- **Category:** correctness / source-media data loss.
- **Location:** `src-tauri/src/export/mod.rs:2135–2156`; `src-tauri/src/commands/mod.rs:2678–2690`.
- **Trigger:** import an external source named `Original.wav`, then request a WAV export to `original.wav` in the same directory on a normal case-insensitive Windows volume. The agent export API accepts an explicit destination.
- **Evidence:** `resolve_destination` canonicalizes the parent directory but retains the requested filename, then compares the destination to the canonical source using case-sensitive `PathBuf` equality. These strings differ although they name the same file. Other-file exports replace their destinations (`fs::write` for text; rename for WAV), so the source protection is essential here.
- **Impact:** exporting can replace original imported media with the edited output. The original recording is not recoverable through edit undo.
- **Recommendation:** when a destination exists, compare its resolved file identity with every source before writing; use platform-appropriate comparisons for nonexistent paths and directory containment too. Retain an atomic publication policy that cannot replace source files.
- **Validation:** a Rust probe on this Windows filesystem created a disposable `Original.wav` sentinel and verified `source_path_equal=false` while `alternate_case_reads_same_file=true` for `original.wav`. The probe did not overwrite the sentinel. Source tracing confirms the rejected comparison is the one used before replacement.
- **Fixed (2026-10-24):** `resolve_destination` now canonicalizes an existing destination (true case, links followed) before the bundle and source checks. Test: `an_export_never_replaces_a_source_named_in_another_case` in `project/reader.rs`.

### A06 — P2 — Running video exports resolve mutable graphic sources again for each frame

- **Category:** correctness / inconsistent exported video.
- **Location:** `src-tauri/src/export/mod.rs:1126–1168, 2498–2519`; `src-tauri/src/graphics/render.rs:408–431, 498–513`.
- **Trigger:** edit a graphic's source/variables or restore its checkpoint while a video export is running, including through an agent while the export dialog is open.
- **Evidence:** export prepares its graphic renders with `ensure`, but the frame renderer does not retain those resolved render paths. `graphic_job` reconstructs a spec and calls the shared `lookup` on every frame. Source changes invalidate its hashes, and this final-export path accepts `Lookup::Stale` and `Lookup::Pending` exactly as preview does.
- **Impact:** an export can switch to a later graphic revision mid-clip, use an obsolete render, or encode a rendering placeholder if no fallback remains, despite having passed the graphics preparation stage. Capturing the edit document alone does not freeze external graphic source files.
- **Recommendation:** snapshot graphics for the export and pin each clip to the exact prepared render path for the job's lifetime; fail rather than use a preview placeholder if a required final render is unavailable.
- **Validation:** traced export preparation, per-frame lookup, hash invalidation, and the stale/placeholder branches. No full media render was run for this scenario.
- **Fixed (2026-10-24):** `ensure` returns each spec's render, named from the source hash taken when it was asked. `render::pin` hard-links (else copies) each into a `.export-*` folder in its cache, held for the job and removed after; the evaluator plays those (`with_pinned_graphics`) instead of looking up the cache per frame. This also covers `remove_superseded` deleting a replaced render mid-export. A render gone before pinning fails the export. Test: `a_pinned_render_outlives_the_one_that_replaces_it`.

## Validation and coverage

- Reviewed project persistence and path handling, export destinations and jobs, WAV output, graphics rendering/cache lookup, frontend project switching, short timelines, and waveform analysis. Existing application changes were preserved.
- `npx tsc --noEmit`: passed.
- `npm run test:cleanup`: all 28 tests passed.
- `cargo test --lib -- --test-threads=2`: compiled successfully. The sandboxed run reported 489 passed, 121 failed, and 49 ignored. Representative failures were temporary-file replacement permission errors and blocked localhost test-server requests.
- Reran the same compiled Rust test executable outside the sandbox with `--test-threads=2`: **610 passed, 0 failed, 49 ignored**, in 45.52 seconds. The initial failures are therefore recorded as environment restrictions, not additional application findings.
- Targeted source harnesses reproduced A01, A03, and A04; a disposable Windows filesystem probe confirmed A05's path-comparison failure. A02 and A06 were validated through source/control-flow review, without exercising concurrent full media exports.
- Limits: default Rust features only; ignored tests, optional local-model/GPU configurations, macOS behavior, and full desktop UI/media-export scenarios were not exercised. Passing existing tests does not cover the identified scenarios. Other working-tree files changed during the audit; test results describe the compiled/checked snapshot, and line references describe the source inspected.
