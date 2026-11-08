# Code review: bugs, errors, and performance gaps

Review date: 2026-11-08.

This review examines the current working tree, including uncommitted and newly added source files. Existing edits are preserved. Findings are added individually as they are verified; this document is a review, not an implementation of fixes.

Priorities: **P1** = serious correctness, data-loss, or security issue; **P2** = normal-priority bug or material performance issue; **P3** = lower-impact issue. Static confirmation means the relevant implementation and callers were traced; it does not imply the desktop application was exercised. Questions identify decisions to resolve before implementing a fix.

**Result: 15 open findings — 4 P1, 10 P2, 1 P3 — plus one superseded finding (GAP-001).** Each finding was written when identified, then supplemented with reproduction evidence where available. No fixes to application source were made.

Suggested fix order: address narration identity/input validation (GAP-004, GAP-008), publish project ownership (GAP-011), and upload/save synchronization (GAP-016) first; then publishing draft/release handling (GAP-002, GAP-015), save/history races (GAP-003, GAP-010, GAP-012), and the outbound-network boundary (GAP-013). Export freshness/polling and stage bookkeeping follow (GAP-007, GAP-009, GAP-014). Restore the two failing regression guards before merging (GAP-005, GAP-006).

## Findings

### GAP-001 — P2: Choosing a publish thumbnail silently discards unsaved metadata

**Continuation status (2026-11-08):** The publishing UI was refactored between review passes. `PublishDialog` now delegates to `PackageEditor`, whose thumbnail responses use `show(..., false)` and preserve draft fields; the old status-refresh reset is also removed. The original specific reset path below is therefore **superseded in the current source**, not an open finding against the refactored handlers. This was not an assistant code fix. The new autosave lifecycle and upload ordering problems are GAP-015 and GAP-016. Full dirty-draft regression coverage has not yet been established.

**Location:** `front-end/src/components/export/PublishDialog.tsx`, `show`, the image/frame handlers, and the upload-status refresh handler.

**Evidence / cause (static confirmation):** Title, description, disclosure, and tags live in local `draft`/`tags` state until Save or Upload. `show(v)` replaces all of that state with the package returned by the backend. Both thumbnail handlers call `show(await api.publishThumbnail...)` without first saving the draft. The backend thumbnail functions update only the thumbnail in the package already on disk. Refreshing an earlier upload also reloads the package through `show`.

**Reproduction:** Open Publish, type a new title and description without clicking Save, then choose an image or use a frame as the thumbnail. The fields revert to their last saved values. The same loss occurs when refreshing an earlier upload's status. Fields remain editable during requests, so edits made while Save/Upload is in flight can also be overwritten by its response.

**Impact:** Ordinary preparation of a video loses entered copy without a warning or an undo path.

**Possible solution:** Separate persisted package/thumbnail/upload state from the editable metadata draft. Thumbnail and status responses should update their own fields only. Track a draft generation or dirty fields so a save response cannot replace changes made after that save started. Alternatively serialize an explicit draft save before thumbnail operations, while still preserving subsequent typing.

**Regression checks:** Enter metadata, choose both types of thumbnail, and refresh an earlier upload; assert all unsaved fields survive. Hold a save response, type again, then resolve it and assert the later text remains.

**Questions for the fix:** Should metadata autosave, or remain explicitly saved? If explicit, should closing a dirty dialog prompt to save?

### GAP-002 — P2: Release changes are silently treated as retries of the old upload

**Location:** `src-tauri/src/publish/mod.rs`, `upload_key` and the early-return branch in `upload`; `front-end/src/components/export/PublishDialog.tsx`, release selector and `upload`.

**Evidence / cause (static confirmation):** Upload identity includes only file contents, channel, and title. `upload` returns an existing non-failed upload, optionally refreshing its status, before using the newly requested `Release`. Neither a changed release nor changed description/tags/thumbnail is compared with the original request. There is no post-update operation in the client.

**Reproduction:** Upload a video as Draft, reopen Publish for the same export, select Scheduled and click Schedule with the same title/channel. The existing draft is returned and no scheduling request is sent. Private-to-scheduled and changing a scheduled date follow the same branch. Saving improved metadata and uploading again also leaves the remote copy unchanged.

**Executed confirmation:** Called the production `publish::upload` twice through its injectable HTTP interface, with a local fixture file and an in-memory fake service: Draft followed by Scheduled. The second call returned `release=Draft`, `status=draft`; only one post-creation request was made. No remote request or real upload occurred.

**Impact:** A user can believe a release action succeeded while the video remains a draft/private or retains its previous schedule and metadata. The dialog does not explain that these changes were ignored.

**Possible solution:** Separate retry identity for one submission from an explicit update to an existing post. Compare the requested release and metadata with the persisted submission snapshot; implement an update where supported, or refuse with a clear instruction and link to edit the existing post. Do not simply include the release in the key and silently create another upload.

**Regression checks:** Draft → scheduled, private → scheduled, schedule A → B, and a metadata-only change must either update the existing post or produce an explicit actionable refusal. An exact retry must still avoid a second post.

**Questions for the fix:** Should AeroEdits support publishing a saved draft and changing an existing release, or should those operations be directed to the external dashboard?

### GAP-003 — P2: Late faceless-status responses can restore the wrong project's state

**Location:** `front-end/src/stores/facelessStore.ts`, `refresh`; `front-end/src/App.tsx`, `refreshFaceless` effect; `front-end/src/components/faceless/FacelessPanel.tsx`, refresh effect.

**Evidence / cause (static confirmation):** Every `refresh(path)` unconditionally writes `{ production, path }` after its asynchronous request. There is no request generation, cancellation, or check that this remains the selected project. Calling `refresh(null)` clears the store but does not invalidate an earlier in-flight request. `api.facelessStatus()` receives no project identity, while the backend reads whichever project is currently open. App's `active` flag only guards the workspace change after the store has already been mutated. Consumers render `production` without checking its stored `path` against the open project.

**Reproduction:** Start loading faceless project A, switch to B or close the project before A's status completes, and let the old response finish last. The store again contains A's production even though B/no project is selected. Two overlapping refreshes can also regress the stages shown within one project.

**Executed confirmation:** Transpiled the actual `facelessStore.ts` into a Node VM with real Zustand and a deferred IPC stub. Resolving B before A left `{path:"A", topic:"A"}`; clearing the store before resolving A also left `{path:"A", topic:"A"}`. No project or external service was touched.

**Impact:** The Faceless tab can appear for an ordinary project, show another project's topic/stages, or enable actions using stale readiness information. Server-side stage validation may reject an inappropriate action, but it does not correct the displayed state.

**Possible solution:** Invalidate earlier refresh generations, including on `refresh(null)`, and apply responses only to the current project handle/path and latest request. Pass a project handle through the status IPC and validate it in the backend. Clear or explicitly mark old state while switching projects.

**Regression checks:** Resolve A after B; resolve A after clear; resolve an older same-project request after a newer one. None may replace the current state.

**Questions for the fix:** No product decision is needed for rejecting stale responses. Decide whether a refresh error should preserve the last known state with an error indicator instead of looking like an ordinary non-faceless project.

### GAP-004 — P1: Regenerating a voice take can keep the old audio and transcript on the timeline

**Location:** `src-tauri/src/faceless/voice.rs`, `fingerprint` and `make`; `src-tauri/src/commands/faceless.rs`, `faceless_voice_impl` (`same_file` branch).

**Evidence / cause (static confirmation):** Explicit `redo` forces a new paid speech request and replaces `rawN.mp3`. However, the joined narration filename is derived only from text/model/voice fingerprints and speed, not the returned audio bytes or take identity. A redo of unchanged words therefore writes the same narration path even if pronunciation or duration changes. `faceless_voice_impl` treats equality of that path plus the presence of the old asset on the timeline as proof that the audio is unchanged. It then skips `place_narration`, reuses the old asset/sound, loads the old transcript, and preserves previous acceptance. `place_narration` imports a separate media asset, so rewriting the intermediate narration does not replace that imported timeline asset.

**Reproduction:** Voice a script, then ask the agent to redo a scene whose pronunciation is wrong without changing its text. The provider returns a different take, but `same_file` is true. The new take is generated and assembled while playback/export still uses the old imported narration. New scene boundaries can now disagree with the reused transcript/audio.

**Executed confirmation:** Linked a standalone scratch probe to the freshly built production Rust library. A fake speech provider returned two different local tones for unchanged words, with the second explicitly requested through `redo`. The actual `voice::make` returned the same narration identity although bytes changed and duration changed from 1,531,958 to 2,363,917 microseconds. Audio extracted by the actual media importer after the first take remained unchanged. No speech service was called. The subsequent `same_file` decision is confirmed by tracing its caller.

**Impact:** Paid regeneration fails to fix the audible mistake; alignment checks and previous user acceptance can apply to the wrong take. This directly affects the rendered output.

**Possible solution:** Identify a narration by the actual audio contents or immutable take IDs, including all selected raw takes and pacing settings. A changed take must produce a new asset, new transcription, and a reset of acceptance. Keep old audio immutable for undo. Do not use a filename derived solely from requested text as an audio equality test.

**Regression checks:** Use a fake speech provider returning different audio/durations on consecutive calls for identical text. Force redo and assert a new narration identity, new imported asset and transcript, updated scene times, and cleared acceptance; exact reuse without redo should still avoid paid calls.

**Questions for the fix:** When a new take changes duration, should later graphics be retimed automatically or marked stale for manual/agent repair?

### GAP-005 — P3: The style-store regression test no longer matches the built-in catalog

**Location:** `src-tauri/src/style/store.rs:488`, `styles_are_saved_listed_renamed_and_removed`; built-in Broadsheet definition in `src-tauri/src/style/mod.rs`.

**Evidence (reproduced):** With temporary files redirected to a writable workspace directory, the Rust test fails with actual names `["Light", "Dark", "Broadsheet", "Acme Dark"]` versus expected `["Light", "Dark", "Acme Dark"]`. Broadsheet is an intentional new built-in, so this is a stale test expectation, not evidence that listing styles is broken.

**Impact:** The library suite is red on the current working tree. Assertions after this failure (invalid/reserved names and removal behavior) do not execute in that test.

**Possible solution:** Update the expected catalog deliberately, or derive the built-in portion from the canonical registry while independently asserting its expected contents in a catalog test. Include Broadsheet in reserved-name/immutability coverage.

**Regression check:** Run `cargo test --lib styles_are_saved_listed_renamed_and_removed` with writable temporary storage and verify all assertions execute.

**Questions for the fix:** No clarification needed if Broadsheet is intended to ship.

### GAP-006 — P2: The advertised MCP tool schema exceeds its enforced context-size budget

**Location:** `src-tauri/src/mcp/mod.rs:1494–1523`, `the_tool_list_stays_small`, and the tool definitions returned by `tool_list`.

**Evidence (reproduced):** The serialized tool list is **47,697 bytes**, exceeding the strict `< 47,500` assertion. The test's message calls these characters, but the measurement is Rust string `.len()` (UTF-8 bytes). This test fails independently of filesystem/network permissions.

**Impact:** A performance/cost regression guard is red. Clients that consume the full schema receive more context on connection. The overrun is small (197 bytes above the nominal boundary); no claim is made that it alone creates noticeable latency or a specific token cost.

**Possible solution:** Inspect the existing `TOOL_SIZES` breakdown, remove redundant descriptions/schema fields, or place infrequently used operations behind the existing `more` mechanism. If the added context is an intentional tradeoff, document and approve a new budget rather than silently weakening the guard.

**Regression check:** `cargo test --lib the_tool_list_stays_small`; record both serialized bytes and an actual client/token measurement if the budget is revised.

**Questions for the fix:** Is the current 47,500-byte budget still required, or should the added capabilities justify an explicitly increased limit?

### GAP-007 — P2: Export-list polling repeatedly hashes the whole edit for every historical export while holding project locks

**Location:** `src-tauri/src/commands/mod.rs`, `export_deliveries_impl`; `src-tauri/src/export/deliveries.rs`, `fingerprint` and `list`; `front-end/src/components/export/DeliveryList.tsx`, 500 ms polling effect.

**Evidence / cause (static confirmation):** Each list request holds `opened_project` and then `export` locks while evaluating all records. For every record with a fingerprint, `list` calls `now(request)`. Main-video requests each clone the complete edit document, serialize it, and SHA-256 hash it again; short requests additionally rebuild the short document. These values depend on the current target/document, not the individual historical export, but there is no per-call memoization. Up to 100 persisted records are retained, and the UI polls twice per second while an export is active. The current implementation computes `older` even for unfinished records, although that result is then masked out.

**Reproduction / scale:** Keep 100 exports of one large edit, then start another export with the list open. One unchanged polling interval can clone/serialize/hash that same edit roughly 100 times; two polls mean roughly 200 full-document passes per second. This is an operation count derived from the loops, not a measured desktop latency.

**Executed confirmation:** A scratch probe called the production delivery writer/list implementation with 100 same-target completed exports. One `list` invocation called its current-fingerprint callback **100 times**. No real export was run for this check.

**Impact:** Avoidable CPU/allocation load grows with both project size and export history. Holding the project and export locks during the work can delay edits, playback commands, cancellation, and status requests.

**Possible solution:** Compute one fingerprint per distinct target per list request, only for completed entries that need comparison. Cache across polls by the relevant revision/content generation. Snapshot needed state under locks and perform expensive serialization outside them. Avoid overlapping frontend polls if a previous request has not completed.

**Regression checks:** Populate 100 exports of the same target and instrument fingerprint calls; expect one, not 100. Include multiple shorts, a changed revision, and an unchanged poll. Measure command/lock latency on a large real edit before and after.

**Questions for the fix:** Should export freshness also account for transcript/caption/graphic files that change outside the edit revision? That affects cache keys and freshness semantics.

### GAP-008 — P1: Accepting an old narration can incorrectly certify it against a newly reviewed script

**Location:** `src-tauri/src/commands/faceless.rs`, `faceless_voice_accept_impl` and `finish_voice`; `src-tauri/src/faceless/mod.rs`, `record_stage`; `front-end/src/components/faceless/VoiceView.tsx`, “It sounds right” handler.

**Evidence / cause (static confirmation):** Acceptance loads any existing `voice.json`, sets `accepted = true`, then calls `record_stage("voice")`. That function records the hash of the review file that exists *now*. Acceptance never verifies that the loaded audio was generated from that review/script, and the voice record contains no script/review fingerprint to compare. The UI's acceptance button is disabled only while busy, even when the upstream review is stale. The long-running voice-generation path also records the current review hash on completion without checking that it matches the input used before network/transcription work began.

**Reproduction:** Generate narration A that has an issue requiring acceptance. Change the script to B and complete its review. The old voice file remains. Click “It sounds right” without regenerating B. The old audio is now recorded as satisfying B's current review and the voice stage can become Done. Similarly, change/review the script during a long generation and let the old generation finish afterward.

**Executed confirmation:** A scratch project with synthetic stage A/B files and an old voice record was opened with the production `ProjectReader`. Before calling `faceless_voice_accept_impl`, the voice stage was **Stale** after the script/review changed. After that production acceptance command, it was **Done**, without regenerating narration. This isolates stage binding; it is not an end-to-end spoken-content test.

**Impact:** The stage system can green-light visuals/export preparation while the spoken facts belong to an older script. Acceptance of pronunciation should not substitute for generating the current content.

**Possible solution:** Store immutable script/review fingerprints with generated narration and capture them at job start. Acceptance must match those fingerprints to current inputs; completion must reject or mark stale an output whose inputs changed. Pass an expected narration identity to acceptance so a late click cannot accept another take.

**Regression checks:** A → B script/review change followed by accepting A must fail or remain stale. Repeat with a delayed generation and a concurrent review change. Accepting the current narration should still clear pronunciation/length warnings.

**Questions for the fix:** Can users deliberately retain narration after purely visual/script annotation changes? If so, define a spoken-content fingerprint separately from review metadata; do not allow changed spoken words to reuse old acceptance.

### GAP-009 — P2: Export freshness ignores transcript changes that alter rendered captions

**Location:** `src-tauri/src/export/deliveries.rs`, `fingerprint` and `list`; `src-tauri/src/commands/transcript.rs`, `modify_transcript` / `transcript_run_impl`; `src-tauri/src/export/mod.rs`, caption-cue loading.

**Evidence / cause (static confirmation):** Video delivery fingerprints hash only `EditDocument` with the revision zeroed. Subtitle/transcript file deliveries have no fingerprint and fall back to comparing edit revisions. Transcription and transcript modifications save the separate transcript file without committing a new edit document. Export's caption renderer reads that separate file. Thus the same fingerprint/revision can produce different visible words and timings. Refreshing the delivery list does not fix this because the omitted input remains omitted.

**Reproduction:** Export a captioned video, then correct a transcript word or replace its transcription without changing the timeline/caption settings. Reopen the export list. The existing file is still reported as current even though another export would render different captions. Transcript/subtitle deliveries have the same dependency problem.

**Impact:** The UI can encourage publishing an export containing an old transcription error because its stale-output indicator is false.

**Possible solution:** Include hashes or durable generations of the transcripts/caption marks actually consumed by the target in an export-input fingerprint. Capture those identities with the export snapshot and use the same scheme for current comparisons. Apply equivalent treatment to other mutable external render inputs rather than assuming the edit JSON fully describes output.

**Regression checks:** Export, change a word/timing in the captioned sound, and assert stale status; change an unrelated sound not consumed by the output and define whether it should remain current. Cover subtitle/transcript files and shorts.

**Questions for the fix:** Should “current” mean identical rendered content or any changed project input? The former avoids false positives but requires target-specific dependency tracking.

### GAP-010 — P2: Audio adjustments have no save queue and collide on the same edit revision

**Location:** `front-end/src/lib/trackMix.ts`, `saveTrackMix`; `front-end/src/components/audio/AudioSection.tsx`, `setPolish`, `update`, and `setVolume`; `src-tauri/src/commands/mod.rs`, `project_audio_update_impl`.

**Evidence / cause (static confirmation):** Every call to `saveTrackMix` immediately reads the currently displayed revision and sends an entire audio-settings snapshot. Calls are neither queued nor rebased. The component also has independent 250 ms debounces for track volumes and global audio settings, both using that same revision. Backend audio updates enforce the expected revision. Two requests sent before either response arrives therefore compete: the first advances the revision and the second is rejected. There is no automatic retry of the user's rejected intent. The controls remain enabled during these saves.

**Reproduction:** Delay the first audio-update response, change noise reduction/ducking on another lane, or let a global-settings debounce and a volume debounce fire together. Both requests carry revision N. One succeeds; the other receives a stale-revision error and its adjustment is not persisted. Volume drafts are cleared in `finally`, including after a failed save.

**Impact:** Rapid, otherwise valid mixer adjustments require the user to notice an error and repeat them. This is especially likely when an audio preparation/save takes longer than the debounce interval. The error is visible, so this is not characterized as silent data loss.

**Possible solution:** Use a project/timeline-owned serialized audio-change queue, merging patches into the latest acknowledged settings and revision at send time. Coalesce repeated slider values while retaining independent track changes. Include global settings and per-track mix edits in the same queue; preserve unsaved drafts on error. Await `applyOpenedProject` before advancing the queue when short views require an extra fetch.

**Regression checks:** Hold request A pending, issue B for a different track and C for global sound effects, release A, and assert all intended changes survive on successively current revisions. Cover failures, switching projects/shorts, and a newer slider draft while an earlier save resolves.

**Questions for the fix:** Should a failed save leave the controls showing the unsaved value with a Retry action, or restore the last saved value with a clear notice?

### GAP-011 — P1: Publish commands are not bound to the project whose export the dialog shows

**Location:** `src-tauri/src/commands/publish.rs`, `project_root` and package/thumbnail/upload commands; `front-end/src/lib/ipc.ts`, publish API methods; `front-end/src/components/export/DeliveryList.tsx`, `publishing` state; `front-end/src/components/export/PublishDialog.tsx`, initialization and upload sequence.

**Evidence / cause (static confirmation):** Publish IPC requests carry a target such as `video`, but no project handle. Each backend call resolves the currently open project. The dialog captures the export file and draft but is not keyed to a project handle; its load effect depends only on `target`. `DeliveryList` does not clear `publishing` when `projectHandle` changes. App can receive an agent-opened-project event while a dialog is open. A delayed image picker or the multi-step save → upload sequence can consequently resume against a different project.

**Reproduction:** Open Publish for project A's export, then have a connected agent open B while the dialog remains active (or while its file picker is pending). Save/choose the thumbnail from the old dialog. The backend writes into B's `publish/` folder. If an upload proceeds, the captured file can still be A's export while its package/journal is resolved from B.

**Impact:** Metadata can overwrite another project's package; the wrong combination of video and metadata can be uploaded. This is a project-identity error, independent of the draft-reset issue in GAP-001.

**Possible solution:** Require a project handle on every project-specific publish command, validate it before resolving a root, and retain that root/identity throughout asynchronous work. Key/reset the dialog and its requests on project changes. Reject a stale action instead of retargeting it. Bind the export/delivery ID and metadata target to that same project, with explicit handling for a file supplied outside the delivery list.

**Regression checks:** Switch A → B during package loading, file picking, save, thumbnail extraction, and immediately before upload dispatch. Assert no B files change and no A file is submitted using B's package.

**Questions for the fix:** Should an already-started upload continue for A after the editor switches to B? Continuing can be supported safely by retaining A's captured context; new actions from A's stale dialog should still be rejected.

### GAP-012 — P2: Export-history writes race with Forget and subtitle/audio exports

**Location:** `src-tauri/src/export/deliveries.rs`, `work`, `record`, `forget`, and `write`; `src-tauri/src/commands/mod.rs`, `export_forget_impl` and `export_file_impl`.

**Evidence / cause (static confirmation):** The background video worker records completion after releasing its shared jobs lock. Other commands independently call `record`/`forget` on the same `exports.json`. These functions do a read → modify → write without a shared per-project lock. `write` also uses one fixed temporary path, `exports.json.tmp`. The command-side project/export locks do not protect the worker's filesystem transaction. Completion recording ignores write errors with `let _ = record(...)`.

**Reproduction:** While a video export finishes, forget an older output or export subtitles/audio for the same project. Both transactions can read the old list. Whichever writes last drops the other's change, or one rename fails because the other writer already renamed the shared temporary file. Forgetting an item can appear to succeed and then have it reappear; a successfully created output can be absent from history.

**Impact:** Export history can lose records or resurrect removed entries. The actual finished media files are not deleted by this race, but the UI loses the record needed to find/re-export/publish them.

**Possible solution:** Serialize all export-history transactions under a per-project mutex or a single writer, including the worker and file-export commands. Use unique temporary files via the existing atomic-write helper, but retain transaction locking because atomic replacement alone cannot prevent lost updates. Report persistence failures instead of silently discarding them.

**Regression checks:** Use barriers to overlap two records, and a record with Forget, then assert both independent changes survive. Inject a write/rename failure and require a visible/recoverable history error.

**Questions for the fix:** Should failure to save history mark a successful media export as “completed with warning,” or retain a pending history record for retry? The file's successful creation should remain accurately represented.

### GAP-013 — P2: Research URL validation does not bind the connection to the address it checked

**Location:** `src-tauri/src/faceless/evidence.rs`, `check_public`, `WebPages::new`, and `WebPages::page`.

**Evidence / cause (static confirmation; no live exploit attempted):** `check_public` resolves a hostname with `to_socket_addrs`, rejects private/local addresses, and returns only the URL. The validated address list is discarded. `WebPages::page` then sends that hostname to a separately configured reqwest client, which has no pinned resolution from the check. Consequently the lookup used to decide “public” and the lookup used to connect are separate operations. Checking every redirect does not close this gap within each hop.

**Trigger:** An attacker-controlled hostname returns a public address during validation and a private/loopback address when the HTTP client resolves it. This requires control over DNS responses/timing or equivalent resolver behavior; an ordinary static private-IP URL is already rejected.

**Impact:** The code's explicit promise that research URLs cannot reach the user's local network is not enforced at connection time. HTTP research URLs can be used to make unintended local-network GET requests if the hostname resolves differently between checks. HTTPS certificate validation may restrict some variants, but HTTP is allowed.

**Possible solution:** Resolve and validate once per hop, then connect only to the validated addresses while retaining the URL hostname for Host/SNI and certificate validation. Apply the same rule to retries and redirects, and define how proxies affect this guarantee. Reuse a tested outbound-request policy across research/discovery clients where applicable.

**Regression checks:** Use an injected resolver/transport that returns a public address at validation and a loopback/private address on a second lookup. Assert that there is no unvalidated second resolution/connection. Include redirects and mixed public/private address answers.

**Questions for the fix:** Are corporate HTTP proxies supported? A proxy may perform DNS resolution itself, so the intended local-network isolation policy must specify that case.

### GAP-014 — P2: Successful packaging/publishing never completes the corresponding faceless stages

**Deep-review status (2026-11-08):** The current working tree now contains `sync_package` and `record_upload`, calls from package/thumbnail saves, and upload/refresh integration. The original missing-wiring finding is **superseded**, not open against this newer implementation. This change was made outside this review. Its success/status semantics and full end-to-end behavior still need validation; the original evidence below describes the earlier snapshot.

**Location:** `src-tauri/src/faceless/mod.rs`, `STAGES` and `file_hash`; `src-tauri/src/commands/publish.rs`; `src-tauri/src/publish/mod.rs`, package/upload persistence; `front-end/src/components/faceless/FacelessPanel.tsx`.

**Evidence / cause (static confirmation):** The stage machine expects `faceless/package.json` and `faceless/publish.json` plus corresponding `record_stage` entries. The implemented publishing workflow stores metadata in `publish/package.json` and upload records in `publish/uploads.json`. Its save/upload/refresh commands never write the expected faceless files or call `record_stage` for either stage. A source search finds no production caller recording these two stages. The Faceless panel explicitly directs users to the Export dialog to perform Package and Publish.

**Reproduction:** Complete a faceless production through Visuals, save its full package, and successfully upload it through the supported Publish dialog/tool. Refresh the Faceless stages. Package/Publish still remain Todo and the next-stage pointer does not represent the work just completed.

**Impact:** The production checklist cannot reach completion through the offered workflow, and agents/users receive misleading guidance about what still needs doing. This is an integration gap in the new feature, not a claim that remote upload itself fails.

**Possible solution:** Make stage state derive from the canonical publish package/upload records, or write a small faceless stage receipt containing the target, metadata fingerprint, exported-content identity, and upload outcome. Avoid duplicating mutable metadata into two independent sources of truth. Changes to the relevant package/render/release should invalidate only the affected stages.

**Regression checks:** A valid package completes Package; an accepted upload outcome completes Publish according to the chosen semantics. Failed/uncertain uploads must not mark successful completion. Later metadata/content changes must invalidate the appropriate receipt.

**Questions for the fix:** Does Publish count as done for a remote draft, a private upload, a scheduled post, or only a public video? Should the Package stage require a thumbnail, or is an explicitly chosen automatic thumbnail acceptable?

### GAP-015 — P2: Closing the new package editor cancels the last unsaved keystrokes

**Location:** `front-end/src/components/export/PackageEditor.tsx`, `change` and unmount cleanup; `front-end/src/components/export/DeliveryList.tsx`, switching from Publish to Accounts.

**Evidence / cause (static confirmation, continuation pass):** The newly added shared package editor promises to save as the user types, but only sends a save after a 600 ms timer. Its cleanup calls `clearTimeout(pending.current)` and has no flush, persistent draft, or unsaved-changes warning. Closing the export/publish dialog, or opening Accounts from Publish (which unmounts that dialog), cancels a pending save. This differs from the existing `useDebouncedSave` hook, whose cleanup flushes pending work.

**Reproduction:** Edit a title or description and immediately close the dialog or click Accounts before 600 ms elapses. Reopen it: the last edit was never sent and the previous saved text returns.

**Executed confirmation:** A hook harness mounted the actual `PackageEditor` with mocked IPC, typed a new title, and unmounted after 100 ms. After advancing beyond the debounce, the save call count remained **0**. This verifies the lost-save path without a desktop UI walkthrough.

**Impact:** Fast navigation silently loses valid metadata despite the autosave promise. This is a new lifecycle issue in the shared editor; the older thumbnail-reset issue GAP-001 is tracked separately with a continuation status note.

**Possible solution:** Give pending drafts a project/target-owned save controller that survives the component, or explicitly flush and await pending work on close/navigation. Preserve recoverable drafts when saving fails. Do not flush against whichever project happens to be open; also apply the ownership fix in GAP-011.

**Regression checks:** Type, advance less than 600 ms, then unmount through Close and Accounts. Require exactly one save of the latest valid draft to its original target. Cover a save failure and a project switch.

**Questions for the fix:** Should closing wait for autosave, or should the save continue with a visible global status/retry indicator?

### GAP-016 — P1: Upload can publish old metadata because it does not await or validate autosave

**Location:** `front-end/src/components/export/PublishDialog.tsx`, `upload`; `front-end/src/components/export/PackageEditor.tsx`, `change`.

**Evidence / cause (static confirmation, continuation pass):** The upload handler waits an unconditional 700 ms and then calls `publishUpload`. The child editor starts saving only after a 600 ms debounce. The parent receives no save promise, pending state, failure status, or current draft. Waiting 700 ms is not synchronization with the save completing. The Upload button checks the last persisted `view.problem`, and text fields remain editable during the wait/upload. A failed autosave is reported only inside the editor while the parent's upload proceeds using the previous valid package on disk.

**Reproduction:** Begin with a valid saved package, edit its title/description, and immediately click Upload. Delay or reject `publishPackageSave`; advance the 700 ms timer. `publishUpload` still runs using the old metadata. Typing again during the wait can move the debounce beyond upload dispatch even without a slow backend.

**Executed confirmation:** A hook harness exercised the actual `PublishDialog` and `PackageEditor` with deferred mocked IPC. At 600 ms there was one pending save and zero uploads; at 700 ms there was **one upload while that save was still pending**. No remote upload was made.

**Impact:** A private or scheduled remote post can receive an older title, description, tags, or disclosure value than the user just entered. Once the old-title submission exists, the retry behavior in GAP-002 further complicates correction.

**Possible solution:** Provide a shared save controller or explicit `flushAndSave()` operation that Upload awaits. Freeze a consistent package/release/target snapshot at submission, surface validation/save failures to the parent, and send no upload if persistence fails. Serialize outstanding saves so an earlier response/write cannot overwrite the submission snapshot.

**Regression checks:** Keep a save pending beyond 700 ms, fail it, and type again during upload preparation. Assert zero upload calls until the intended snapshot is successfully saved; on failure require an actionable error and no remote submission.

**Questions for the fix:** Should fields be locked during submission preparation, or should edits made after clicking Upload become a draft for a later update? Either policy needs an explicit snapshot boundary.

### GAP-017 — P1: Restoring a cut can insert the wrong source audio into manually linked clips

**Location:** `src-tauri/src/sequence/edit.rs`, `SequenceEdit::Link`, `cut_after`, and `restore_cuts` (around lines 784 and 887–1001).

**Evidence / cause:** Link accepts clips from different assets and different source offsets. `restore_cuts` computes missing source ranges from the initiating clip, gathers linked partners that end at the same timeline position and speed, and creates every partner's restored piece with the same `in_us: a`. That timestamp belongs to the initiating asset, not necessarily to the partner. Validation accepts the incorrect piece if that other asset has media at the substituted time.

**Executed reproduction:** A production-library probe created a video playing source 0–2 s then 3–5 s, and synchronized external audio playing source 10–12 s then 13–15 s. Both cuts meet at timeline 2 s. Linking their left clips and restoring the video's cut correctly restored video 2–3 s, but inserted **audio source 2–3 s instead of 12–13 s**. The operation returned success and produced a valid sequence. Probe: `src-tauri/target/gap-sequence-probe.rs`.

**Impact:** Restoring an edit can introduce unrelated dialogue or sound and break synchronization without reporting an error. The same implementation can fail validation for a partner whose substituted range crosses a pause or lies outside its media, preventing an otherwise valid restoration.

**Possible solution:** Determine each partner's own removed source range and preserve its source/timeline mapping. Only restore partners whose neighboring pieces establish a compatible cut; handle different pause structures explicitly. If that cannot be determined, refuse the ambiguous operation or restore only the unambiguous clip with a clear explanation instead of guessing source time.

**Regression checks:** Link different assets with a 10-second source offset, ripple-delete a shared interval, and restore from either track. Require the original content mapping on both tracks. Include unequal source offsets within one asset, different pause locations, locked partners, and different speeds. Existing same-recording restoration tests do not exercise independent source clocks.

**Questions for the fix:** Should restoration automatically include arbitrary manually linked clips, or only partners with a demonstrably matching cut? What should happen when a linked partner has no removed source media at that boundary?

### GAP-018 — P2: An export's captured edit does not capture its mutable caption and graphic inputs

**Location:** `src-tauri/src/export/mod.rs`, `CapturedExport`, `prepare_job`, `run_export_logged`, and `SceneEvaluator::caption_at` (around lines 205, 2316, 2577, 2655, and 1803).

**Evidence / cause (static confirmation):** Preparing a job captures an `EditDocument`, output settings, and a project-directory lease. It does not capture transcript contents or graphic source files. The worker discovers graphic render specifications from the live project directory after waiting for face analysis; graphic renders are pinned only after that work finishes. Caption cues are loaded from the live transcript store when the first caption is evaluated. The directory lease is not a versioned snapshot of those files. Thus the document revision shown for the export does not identify all of the content it will render.

**Reproduction scenario:** Start an export with captions enabled, then correct transcript text while the job is waiting or preparing graphics. When caption evaluation first runs, it reads the newer transcript together with the older captured timeline. Similarly, a graphic changed before the worker resolves its render specifications can appear in an export of the previous edit. This scenario was traced through the worker; a full rendered-video comparison has not been performed.

**Impact:** Export contents depend on timing and may mix versions of a project. Repeating an export of the same captured document need not reproduce its captions or graphics. This is distinct from GAP-009, which concerns whether an already completed export is labeled current.

**Possible solution:** Capture an immutable export-input manifest at submission: document, relevant transcripts/marks, and versioned graphic source or render identities. Pass those inputs through the renderer instead of resolving live files later. If snapshotting is too expensive, detect changed dependencies before rendering and restart or fail with a specific message. Reuse content-addressed inputs to avoid unnecessary copies.

**Regression checks:** Pause a worker after capture, mutate a transcript and a graphic, resume, and compare output against the submitted snapshot. Repeat with a mutation during preparation. Require either the original content or an explicit invalidation result, never an unreported mixture.

**Questions for the fix:** Does Export mean the project as it was when clicked or the latest content when rendering begins? The UI, job identity, input capture, and freshness checks should implement the same policy.

### GAP-019 — P2: Modal dialogs do not isolate keyboard focus or suspend destructive timeline shortcuts

**Location:** `front-end/src/components/ui/index.tsx`, `DialogBackdrop`; `front-end/src/components/timeline/TimelineStudio.tsx`, global keydown listener around lines 1170–1188; `PublishDialog.tsx`, `AccountsDialog.tsx`, and other backdrop consumers.

**Evidence / cause (static confirmation):** `DialogBackdrop` supplies a visual overlay and pointer dismissal only: no focus placement, focus containment, restoration, or inert background. Several callers supply `role="dialog"`/`aria-modal`, but Publish and Accounts do not; none of those attributes alone isolates keyboard input. The timeline's window-level listener excludes only inputs, textareas, selects, and editable text. It does not check whether a modal is open, whether an event originated in a dialog button, or whether another handler already prevented it.

**Reproduction scenario:** Select a timeline clip, open Export or Publish, move focus to a dialog button, and press Delete or a configured split/ripple shortcut. The event passes the timeline's target filter and dispatches the underlying editing action. Tab can also move through background controls because the overlay is not a keyboard boundary. This was traced in source; a native WebView keyboard walkthrough remains to be performed.

**Impact:** A user can alter the timeline while working in a modal, and keyboard/screen-reader users cannot reliably navigate the dialog as an isolated interaction. Space on a button can be intercepted as playback rather than activating the button. Dialogs without roles/names are additionally difficult to identify through assistive technology.

**Possible solution:** Introduce one shared accessible modal primitive with an accessible name, initial focus, focus containment, background inertness, and restoration to the trigger. Track the active modal/interaction scope and suspend editor shortcuts outside that scope; honor `defaultPrevented`. Coordinate Escape with the topmost modal rather than broadcasting dismissal to every listener.

**Regression checks:** In a real DOM/browser test, open a dialog with selected clips, cycle Tab/Shift+Tab, press Delete, Space, split, and undo, and verify only appropriate dialog actions occur. Test Escape and focus return for stacked dialogs and dropdowns. Add accessible role/name assertions for every modal entry point.

**Questions for the fix:** Which global shortcuts, if any, should remain active while dialogs are open? Should a long-running upload/export dialog be dismissible without cancelling the work? Define those separately from keyboard focus behavior.

### GAP-020 — P2: Late source suggestions can add an account to the wrong platform's watchlist

**Location:** `front-end/src/components/faceless/SourcesPanel.tsx`, `suggest`, platform-change effect, and suggestion Add handler.

**Evidence / cause (static confirmation):** `suggest` captures the platform used for a request but unconditionally writes its result to `candidates` when it completes. Changing platform clears the list once, without invalidating the request, and the platform selector remains enabled during discovery. The Add handler uses the currently selected `platform` rather than the returned candidate's own platform. Candidate records already include their originating platform, but that field is ignored here.

**Reproduction:** Start Suggest accounts on X, switch to Instagram before the response completes, then wait. The X candidates appear under Instagram; clicking Add calls `facelessWatchAdd("instagram", xHandle, ...)`. A switch to YouTube has the same risk even though that tab has no discovery button.

**Executed confirmation:** The actual component in a hook harness received a deferred candidate with `platform: "x"` after switching to Instagram. Clicking its Add control sent **`platform: "instagram"`** to mocked IPC. Probe: `src-tauri/target/gap-ui-probe.mjs`.

**Impact:** The saved research watchlist can contain a nonexistent account or an unrelated account on another service. Future research may waste paid requests or use the wrong creator as evidence.

**Possible solution:** Associate each request and result with its platform and a generation token; ignore stale results after switching. Use the candidate's validated platform when adding, and render candidates only in their originating tab. Retain a per-platform result cache if useful, with clear loading/error state.

**Regression checks:** Hold an X discovery response, switch to each other platform, resolve it, and assert no X candidates appear there. Verify Add sends the candidate's actual platform. Also resolve two platform requests in reverse order and test unmount during discovery.

**Questions for the fix:** Should discovery continue and cache results after the user leaves a tab, or should switching cancel it when the provider supports cancellation?

### GAP-021 — P2: Graphic playback rereads and reparses source metadata for every frame

**Category:** Performance improvement opportunity; redundant work is confirmed in source, but its contribution to dropped frames has not been measured.

**Location:** `src-tauri/src/export/mod.rs`, `SceneEvaluator::graphic_job`; `src-tauri/src/graphics/render.rs`, `Spec::for_clip` and `Spec::dress`; `src-tauri/src/graphics/mod.rs`, `read_source`.

**Evidence / cause:** Every evaluated graphic frame builds a new `Spec`. That builds default style JSON, resolves the effective style, reads `graphic.json` with `std::fs::read_to_string`, deserializes it, builds look CSS, and serializes the selected style again. The renderer caches hashes and render lookups, but this preparation occurs before those caches and even before checking pinned export renders. Spec properties usually remain constant throughout a clip.

**Work estimate:** A single continuously visible graphic across an eight-minute 30 fps export incurs at least **14,400 source-metadata reads/parses** through this path, before any extra preview/prefetch evaluations. Multiple graphic layers multiply the work. The OS may cache file bytes, but open/read calls, JSON parsing, allocation, and style serialization still occur.

**Impact:** Avoidable synchronous filesystem and allocation work sits in the frame-generation path, making preview/export more sensitive to file-system latency and large graphic metadata. This is not a measured claim of a particular frame-rate or export-time regression.

**Possible solution:** Cache resolved specs in the scene evaluator by clip, canvas, quality, document/style identity, and graphic-source generation. Build stable export specs from the captured manifest (GAP-018), and explicitly invalidate preview specs on source/style changes. Cache source metadata separately when many clips reuse a graphic.

**Regression checks:** Instrument metadata read/parse counts for hundreds of unchanged frames; require work proportional to unique specs, not frame count. Change source look/style, canvas, duration, and export fps and verify invalidation. Benchmark cold and warm preview/export with several graphic layers before setting a latency target.

**Questions for the fix:** Which file-watcher or command events are authoritative for externally edited graphics and styles? Cache lifetime must preserve that live-editing behavior.

### GAP-022 — P2: Automatic captions include muted speech that the output does not play

**Location:** `src-tauri/src/export/mod.rs`, `caption_candidates` and `caption_track_cues`; `src-tauri/src/sequence/clock.rs`, `stream_entries`; compare `src-tauri/src/media/audio.rs`, `lanes`.

**Evidence / cause:** Automatic caption candidate selection visits all audio tracks, and the transcript clock includes their clips without checking track mute state. The audio mixer explicitly excludes muted tracks and zero-gain tracks. Caption selection therefore does not follow the audible timeline. Filtering candidate keys alone is insufficient when the same source has both muted and audible instances: the clock must also select the correct instances.

**Executed reproduction:** A production-library probe used one muted microphone-role track with a valid one-word transcript. `media::audio::lanes` returned **zero audible lanes**, while `caption_track_cues` returned **one caption cue** for the muted speech. Probe: `src-tauri/target/gap-caption-probe.rs`; no audio device or external API was used.

**Impact:** Captions can show discarded dialogue or an unused voice take over silence or replacement narration. Muting a track to exclude its content does not exclude its transcript from automatic caption output.

**Possible solution:** Define caption eligibility from audible speech instances in automatic mode and use a clock built from those instances. Respect track mute and the chosen zero-gain policy. Preserve an explicit opt-in if users intentionally want captions from a silent guide track; distinguish that mode in the UI.

**Regression checks:** Mute the only speech track; mute one of two different voice takes; place the same source on muted and audible tracks; set track gain to zero. Verify burned captions and subtitle export use the intended source and timing. Cover unmute and explicit-source overrides.

**Questions for the fix:** Should an explicitly selected caption source remain active when muted? Should moment-level noise mutes or clip gain also suppress words, or only whole-track mute?

### GAP-023 — P2: Repeated source footage loses captions after its first timeline occurrence

**Location:** `src-tauri/src/sequence/clock.rs`, `entries` and its `used` source-range set; `src-tauri/src/export/mod.rs`, `caption_track_cues`; `src-tauri/src/project/revision.rs`, `transcript_clock`.

**Evidence / cause:** The shared source-to-timeline mapper intentionally gives the first placement of each source moment precedence and removes later placements using `unplaced(&used, ...)`. That policy can make sense for a single seek target, but the same mapper drives caption cue generation. Later audible repeats receive no mapping for the words already mapped once.

**Executed reproduction:** A production-library probe placed a one-second speech clip at timeline 0 s and repeated the same source at 2 s. A word at source 0.1–0.4 s generated only one cue, starting at **0.1 s**; there was no corresponding cue at **2.1 s**. Both clips remained in the sequence. Probe: `src-tauri/target/gap-caption-probe.rs`.

**Impact:** Replays, repeated quotations, and repeated hooks can contain audible speech with missing burned-in or exported subtitles. This is a product limitation of the shared clock, not a timing correction that can be solved by retranscribing the source.

**Possible solution:** Keep a first-occurrence mapper for navigation where that is useful, but generate captions per audible clip instance. Map/clamp source words through every instance, preserve speed and gaps, and identify cues by source word plus clip/occurrence so repeated words do not collide in editing/UI state.

**Regression checks:** Repeat the same source interval at different positions and speeds, including partially overlapping source ranges. Require cues for each audible occurrence, correct clipping at cut boundaries, and deterministic behavior where two speech clips overlap.

**Questions for the fix:** Should editing one repeated caption change every instance or only the selected occurrence? How should competing simultaneous speakers be presented?

### GAP-024 — P2: Frontend checks do not exercise real React lifecycle, focus, or browser interaction

**Category:** Test-quality improvement; this is a coverage limitation, not a claim that the existing unit tests are invalid.

**Location:** `front-end/tests/hooks.ts`, `front-end/tests/run-cleanup-tests.mjs`, `front-end/package.json`, and `.github/workflows/ci.yml`.

**Evidence:** The frontend test command bundles TypeScript tests and runs them under Node. Component tests use a custom hook dispatcher that replaces private React internals. State setters do not schedule renders; callers render manually. `useSyncExternalStore` reads a snapshot but ignores subscriptions, and child function components are not mounted by the element traversal helper. There is no DOM/browser test runner in the frontend package or CI workflow. The app itself mounts a real React tree under StrictMode.

**Impact:** Passing these tests establishes selected logic and state transitions, but cannot establish actual rerender timing, parent/child integration, portal behavior, focus order, event bubbling/default actions, or StrictMode setup/cleanup behavior. GAP-015/016 require coordinated parent/child save behavior, and GAP-019 requires real keyboard/focus behavior; those boundaries currently lack representative coverage.

**Possible solution:** Keep the fast pure-logic tests and add a small real-renderer component/browser suite with mocked Tauri IPC. Prioritize package autosave plus Upload/Close, project switching with deferred responses, modal keyboard isolation, transcript scrolling after resize, and preview generation changes. Use public React testing APIs for lifecycle-sensitive cases instead of extending the private dispatcher to imitate more of React.

**Acceptance checks:** Run the new suite in CI, exercise actual mounted parents and children, dispatch user-like keyboard/pointer events, and assert persisted effects as well as visible state. Include StrictMode in at least one lifecycle suite. Demonstrate that the relevant regression test fails against the current buggy path before claiming coverage.

**Questions for implementation:** Which smoke flows need the actual desktop WebView, and which can run in a normal browser with IPC mocks? Keep native rendering/OS integration checks separate from browser-only behavior.

### GAP-025 — P2: A preview frame can become stale during image decoding and still be painted after a seek

**Location:** `front-end/src/components/canvas/NativePreviewHost.tsx`, `useEngineFrames`, `offer`, and the asynchronous `draw` loop.

**Evidence / cause:** `offer` compares the incoming frame's generation with `playbackGeneration` before decoding, then stores only a `Blob` in `latest`. `draw` awaits `decodeFrame` and subsequently checks cancellation/canvas availability, but no longer knows the frame's generation and does not recheck the acceptance predicate. A seek or view change can make the image obsolete during that await without restarting the preview-surface effect.

**Executed reproduction:** A hook probe supplied a generation-1 frame, held `createImageBitmap` unresolved, advanced the project store to playback generation 2, and then resolved the old decode. The canvas's `drawImage` was called **once with the obsolete frame**. Probe: `src-tauri/target/gap-ui-probe.mjs`; no native preview or video decoder was used.

**Impact:** Rapid scrubbing or switching playback targets can briefly show an old frame after the playhead has moved. If the next frame is delayed, the obsolete image remains visible longer. The incoming-frame filter does not cover this asynchronous boundary.

**Possible solution:** Carry frame generation and relevant view identity through the queued blob and decode. Recheck them immediately before painting, including the current `accept` predicate, and close discarded image bitmaps. Preserve the latest valid pending frame while discarding obsolete decoded output.

**Regression checks:** Delay decode, seek, and resolve; require zero stale draws and proper bitmap cleanup. Repeat for a short/video switch and unmount, then confirm the next valid frame is painted. Test both `createImageBitmap` and image-element fallback paths.

**Questions for the fix:** Should the old image be retained or cleared while waiting for the first frame of a new target? Whichever policy is chosen, a newly completed obsolete decode should not overwrite the intended state.

### GAP-026 — P2: Numeric evidence checks miss negative values and do not preserve units or magnitude

**Category:** Confirmed validation gap, with an additional content-quality improvement for quantity semantics. Passing this check should not be presented as factual verification of a claim.

**Location:** `src-tauri/src/faceless/evidence.rs`, `numbers` and `verify`; `src-tauri/src/faceless/script.rs`, `number_set` and scene/title checks.

**Evidence / cause:** `numbers` ignores digit matches preceded by `-` to avoid treating model names such as GPT-5 as quantities. That also ignores standalone negative quantities entirely. For positive values, verification compares only digit strings; it does not bind them to sign, unit, scale, subject, or period. Script validation reuses the same extraction. `number_context` is descriptive metadata rather than an enforced quantity comparison.

**Executed reproduction:** Against the quote “The trial included 40 people and raised $6.34 million,” production `verify` accepted all three statements without a rejection: “The trial included 40 percent of people,” “The trial raised $6.34 billion,” and “The trial lost -999 dollars.” The first two change the meaning/magnitude of supported digits; the third introduces a completely new quantity that the extractor omits. Probe: `src-tauri/target/gap-validation-probe.rs`.

**Impact:** Unsupported or materially misstated quantities can enter the fact dossier and narration despite the numeric gate. An editorial review may still catch them, but the deterministic check does not supply the protection its successful result might suggest.

**Possible solution:** Parse signed quantities separately from embedded product/version identifiers. Normalize supported unit and magnitude forms into structured values, retaining context such as currency, percent, count, and time period. Require review when transformations cannot be checked safely, and label the existing quote-presence check precisely. Do not claim that matching quantities proves the entire sentence is entailed by the source.

**Regression checks:** Negative numbers versus GPT-5/H100, million versus billion, counts versus percentages, currency changes, percentage points versus relative percentages, decimal/grouping separators, and equivalent formats such as 1,000 versus 1k. Include both false acceptances and false rejections.

**Questions for the fix:** Which numerical transformations may an author make without a new source or explicit derivation? Should derived values be represented as calculations with cited inputs rather than ordinary quoted facts?

### GAP-027 — P2: Stage completion ignores the recorded output hash and can certify a replaced or corrupt result

**Location:** `src-tauri/src/faceless/mod.rs`, `StageRecord.output`, `record_stage`, and `stage_states`; `src-tauri/src/commands/faceless.rs`, review prerequisite in `faceless_voice_impl`.

**Evidence / cause:** Completing a stage records both input and output hashes. Later `stage_states` computes the current output hash only to check that the file can be read; it never compares it with `record.output`. It compares the previous stage's file with `record.inputs` instead. Replacing a stage's own output therefore leaves that stage Done while making its successors stale. The voice prerequisite trusts the review stage's Done status without loading/validating that review again.

**Executed reproduction:** A scratch production recorded research, script, and review outputs, then replaced `review.json` with text that was not valid JSON. `faceless::summary` still returned **Review: Done**. This isolates receipt validation rather than editorial schema validation; the synthetic original outputs were used only to exercise `record_stage`/`stage_states`. Probe: `src-tauri/target/gap-validation-probe.rs`.

**Impact:** External/manual edits, partial writes, or file corruption can leave a stage falsely certified. In particular, a review file with newly added blockers or invalid content may still satisfy the gate for paid voice generation. GAP-008 addresses accepting stale narration; this issue is the missing verification of the stage's own recorded result.

**Possible solution:** Compare the current output digest with the recorded digest and mark a mismatch stale/unverified. Revalidate the result schema and semantic completion criteria before recording a new receipt. If direct edits are supported, provide an explicit validate-and-record operation rather than inferring success from file existence.

**Regression checks:** Change only the current stage's output, replace it with invalid JSON, remove it, and restore the exact recorded bytes. Verify the stage and its successors transition correctly. Add a valid review with open blockers and ensure voice generation is rejected before any paid call.

**Questions for the fix:** Which direct file edits should be automatically revalidated, and which require an explicit review/acceptance step? Should corruption be a distinct error state rather than ordinary Stale?

### GAP-028 — P3: Shared tab and segmented controls expose selection semantics without matching keyboard navigation

**Category:** UI/accessibility improvement, confirmed by inspection of the shared controls; no assistive-technology walkthrough was performed.

**Location:** `front-end/src/components/ui/index.tsx`, `Segmented` and `Tabs`.

**Evidence:** `Segmented` renders a radio group of buttons and `Tabs` renders a tab list, but every enabled item remains a separate Tab stop and neither component handles arrow keys or Home/End. Tabs expose selected state without IDs or panel relationships. Native buttons still support activation, so these controls are not entirely keyboard-inaccessible; their behavior does not match the grouped navigation implied by their roles. In the editor, unhandled arrows can also reach the timeline shortcuts described in GAP-019.

**Impact:** Keyboard users must Tab through every option rather than enter the group once and move within it, and arrow navigation can unexpectedly move the playhead. Screen-reader users have less information connecting a tab to its content. Fixing this once in the shared kit benefits release choices, platform selection, format selection, and panel tabs.

**Possible solution:** Implement roving focus, selection-aware arrow navigation that skips disabled items, and deliberate automatic or manual activation. Add stable tab/panel IDs and relationships where those components own panels. Stop handled navigation events from reaching global editor actions. Alternatively use native radio inputs for simple single-choice groups.

**Acceptance checks:** One group entry point in the Tab order, correct selected/focused state after arrows and Home/End, disabled options skipped, and no timeline action on handled keys. Verify horizontal/vertical and RTL behavior where supported, plus accessible panel relationships.

**Questions for implementation:** Should arrowing through tabs immediately activate their panels, or should Enter/Space activate after focus moves? Expensive panel loading may favor manual activation.

## Review coverage and validation

### Scope

This was a broad, targeted risk review of the current working tree, not an exhaustive proof that every line and platform is defect-free. The existing `docs/audit.md` was read to avoid presenting its already-fixed thumbnail/layout bugs as new findings.

The traced paths included:

- Frontend project/workspace state, debounced saves, track mixing, faceless views, publishing dialogs, export-list polling, caption updates, thumbnail and waveform rendering.
- Backend project ownership, file persistence, selected history/checkpoint and sequence-edit paths, transcript/caption persistence, narration generation/import, stage tracking, export history/fingerprints, publish retries, and research fetching.
- Build/test integration and selected graphics/media cache and playback code. These received a narrower inspection than the paths with findings; passing tests are not a substitute for visual/audio verification.

### Checks performed

| Check | Result |
| --- | --- |
| Frontend `npm run build` | Passed TypeScript and Vite build. Vite reported a 648.54 kB minified workspace chunk and ineffective dynamic imports for ChaptersPanel, StylePanel, and MediaPanel because of static imports elsewhere. These warnings were observed, but no user-visible startup regression was measured, so they are not separately classified as confirmed performance bugs. |
| Frontend `npm run test:cleanup` | 52 passed, 0 failed. |
| Backend `cargo check --lib --features tauri-app --offline` | Passed. Warnings included the public `plan_zooms` signature exposing private `InterestPoint` and sandbox incremental-cache hard-link fallback. |
| Backend `cargo build --lib --offline` | Passed; used the resulting current production library for standalone probes. |
| Backend `cargo test --lib --offline -- --test-threads=4` with writable TEMP/TMP | 839 total: **773 passed, 12 failed, 54 ignored**. Two deterministic regressions are GAP-005 and GAP-006. The other 10 failures involve local HTTP/named-pipe access and remain inconclusive in this restricted environment. |
| Faceless store with real Zustand and deferred IPC responses | Reproduced GAP-003 for out-of-order responses and clear-while-loading. |
| Production Rust library, fake speech/audio fixtures | Reproduced unchanged narration identity despite changed audio for GAP-004; verified the already imported audio stayed unchanged. |
| Production stage/acceptance commands with synthetic A/B stage files | Reproduced Stale → Done acceptance of old narration for GAP-008. |
| Production export-history list with 100 same-target records | Reproduced 100 current-fingerprint callback invocations for one list request (GAP-007). |
| Production publishing function with fake HTTP | Reproduced Draft being returned for a subsequent Scheduled request without a new/update post request (GAP-002). |
| Continuation: actual package/publish components with hook harness, fake timers, and deferred IPC | Reproduced zero saves after early unmount (GAP-015), and upload dispatch while autosave remained pending (GAP-016). Probe: `src-tauri/target/gap-package-probe.mjs`. |

The first Rust suite run used the sandbox's default temporary directory and produced 164 failures, including `Access is denied` on file persistence. Redirecting TEMP/TMP to `src-tauri/target/gap-review-tmp` removed that confounder. The later 12-failure result above supersedes the initial count; the permission failures were not recorded as application defects.

The remaining environment-limited failures were two `ai::client` HTTP tests, the Parakeet checksum-download HTTP test, the ElevenLabs upload-cancellation HTTP test, one `mcp::ipc` pipe-access test, and five `mcp` bridge/CLI/proposal tests. Local HTTP requests failed before the expected exchanges; pipe connections reported unavailable app/agent access. These results require a normal local environment to establish whether any additional code bug exists.

### Limits and retained evidence

No live publishing, paid speech generation, external-account modification, or DNS-rebinding attempt was performed. No desktop UI walkthrough, long-recording playback benchmark, full export visual/audio comparison, macOS/Linux build, CUDA/DirectML/WebGPU transcription run, or ignored GPU/runtime test was performed. Consequently race findings without executed evidence remain code-traced scenarios, and performance claims are limited to demonstrated work counts rather than measured interactive latency.

Scratch probes and logs were kept under the ignored `src-tauri/target/` tree (`gap-review-probe.rs`, `gap-review-tests.log`, `gap-review-check.log`, and `gap-review-build.log`). Their dependencies and local fixture inputs isolate the reported behaviors; they are not committed regression tests. The intended review deliverable is this file, `docs/gap.md`; pre-existing working-tree edits were preserved.

The build and full-suite results above belong to the initial review snapshot. Publishing UI changes observed during the continuation pass superseded GAP-001 and introduced the code-traced findings GAP-015 and GAP-016. Those newer changes have not received a fresh full-suite run.
