# Bugs and performance gaps

Reconciled 8 November 2026 against local HEAD `be7179c` on `claude/cloud-prep`.
The working tree was clean, including untracked files, before this documentation-only pass.
That pass made no application fixes. This is the canonical detailed issue report;
[Roadmap](ROADMAP.md) owns feature work and scheduling, not a second bug list.

**3 findings remain open: 0 P1, 1 P2, 2 P3** (GAP-030/031/032, which need George's judgement
or real recordings). GAP-001/014 are superseded; GAP-005/006 are closed after current focused
checks; every other finding was fixed on 8–9 Nov on branch `claude/project-thread-sc2z5d`. Records stay in place for their anchors.

P1 means serious correctness, data loss or security risk; P2 means a normal-priority bug,
material performance issue or validation gap; P3 means lower impact. **Code-traced** means
the current implementation was inspected, not that the desktop scenario ran. **Prior probe**
means an executed result recorded by the earlier 8 Nov review; it was not rerun here.
Scratch paths under `src-tauri/target/` in those records are local evidence, not portable tests.
Performance work counts are not measured desktop latency. Test scope and fresh results are in
[Verification](verification.md#documentation-reconciliation-8-november-2026).

## Open findings

| Area | Finding IDs |
|---|---|
| Review and graphics follow-ups | [030](#gap-030), [031](#gap-031), [032](#gap-032) |

<a id="gap-002"></a>
### GAP-002 — Fixed: Release changes are silently treated as retries of the old upload

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** An upload with the same file, channel and title but a different release or package (description, tags, AI label, category, thumbnail) is refused with what to do: change it in YouTube Studio or the Zernio dashboard, or change the title for a new upload. Each upload records a `package_digest`. Updating an existing post is not built. Test: `a_changed_release_or_package_is_not_taken_for_a_retry`. The record below is the original finding.

**Location:** `src-tauri/src/publish/mod.rs`, `upload_key` and the early-return branch in `upload`; `front-end/src/components/export/PublishDialog.tsx`, release selector and `upload`.

**Evidence / cause (static confirmation):** Upload identity includes only file contents, channel, and title. `upload` returns an existing non-failed upload, optionally refreshing its status, before using the newly requested `Release`. Neither a changed release nor changed description/tags/thumbnail is compared with the original request. There is no post-update operation in the client.

**Reproduction:** Upload a video as Draft, reopen Publish for the same export, select Scheduled and click Schedule with the same title/channel. The existing draft is returned and no scheduling request is sent. Private-to-scheduled and changing a scheduled date follow the same branch. Saving improved metadata and uploading again also leaves the remote copy unchanged.

**Prior probe (8 Nov):** Called the production `publish::upload` twice through its injectable HTTP interface, with a local fixture file and an in-memory fake service: Draft followed by Scheduled. The second call returned `release=Draft`, `status=draft`; only one post-creation request was made. No remote request or real upload occurred.

**Impact:** A user can believe a release action succeeded while the video remains a draft/private or retains its previous schedule and metadata. The dialog does not explain that these changes were ignored.

**Possible solution:** Separate retry identity for one submission from an explicit update to an existing post. Compare the requested release and metadata with the persisted submission snapshot; implement an update where supported, or refuse with a clear instruction and link to edit the existing post. Do not simply include the release in the key and silently create another upload.

**Regression checks:** Draft → scheduled, private → scheduled, schedule A → B, and a metadata-only change must either update the existing post or produce an explicit actionable refusal. An exact retry must still avoid a second post.

**Questions for the fix:** Should AeroEdits support publishing a saved draft and changing an existing release, or should those operations be directed to the external dashboard?

<a id="gap-003"></a>
### GAP-003 — Fixed: Late faceless-status responses can restore the wrong project's state

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** `facelessStore` applies only its latest refresh and clears on a path change, including `refresh(null)`. Test: `front-end/tests/raceFixes.test.ts`. The record below is the original finding.

**Location:** `front-end/src/stores/facelessStore.ts`, `refresh`; `front-end/src/App.tsx`, `refreshFaceless` effect; `front-end/src/components/faceless/FacelessPanel.tsx`, refresh effect.

**Evidence / cause (static confirmation):** Every `refresh(path)` unconditionally writes `{ production, path }` after its asynchronous request. There is no request generation, cancellation, or check that this remains the selected project. Calling `refresh(null)` clears the store but does not invalidate an earlier in-flight request. `api.facelessStatus()` receives no project identity, while the backend reads whichever project is currently open. App's `active` flag only guards the workspace change after the store has already been mutated. Consumers render `production` without checking its stored `path` against the open project.

**Reproduction:** Start loading faceless project A, switch to B or close the project before A's status completes, and let the old response finish last. The store again contains A's production even though B/no project is selected. Two overlapping refreshes can also regress the stages shown within one project.

**Prior probe (8 Nov):** Transpiled the actual `facelessStore.ts` into a Node VM with real Zustand and a deferred IPC stub. Resolving B before A left `{path:"A", topic:"A"}`; clearing the store before resolving A also left `{path:"A", topic:"A"}`. No project or external service was touched.

**Impact:** The Faceless tab can appear for an ordinary project, show another project's topic/stages, or enable actions using stale readiness information. Server-side stage validation may reject an inappropriate action, but it does not correct the displayed state.

**Possible solution:** Invalidate earlier refresh generations, including on `refresh(null)`, and apply responses only to the current project handle/path and latest request. Pass a project handle through the status IPC and validate it in the backend. Clear or explicitly mark old state while switching projects.

**Regression checks:** Resolve A after B; resolve A after clear; resolve an older same-project request after a newer one. None may replace the current state.

**Questions for the fix:** No product decision is needed for rejecting stale responses. Decide whether a refresh error should preserve the last known state with an error indicator instead of looking like an ordinary non-faceless project.

<a id="gap-004"></a>
### GAP-004 — Fixed: Regenerating a voice take can keep the old audio and transcript on the timeline

**Fixed (8 Nov, branch `claude/project-thread-sc2z5d`):** The joined narration is now named by the takes' audio bytes (with pace and gaps), not the requested words, so a redo that returns different audio is a new narration: new asset, new transcript, acceptance cleared. Exact reuse still pays for nothing and old files stay for undo. Later graphics are not retimed: the visuals stage goes stale through the stage chain. Test: `voice::tests::a_take_voiced_again_is_a_new_narration`. The record below is the original finding.

**Location:** `src-tauri/src/faceless/voice.rs`, `fingerprint` and `make`; `src-tauri/src/commands/faceless.rs`, `faceless_voice_impl` (`same_file` branch).

**Evidence / cause (static confirmation):** Explicit `redo` forces a new paid speech request and replaces `rawN.mp3`. However, the joined narration filename is derived only from text/model/voice fingerprints and speed, not the returned audio bytes or take identity. A redo of unchanged words therefore writes the same narration path even if pronunciation or duration changes. `faceless_voice_impl` treats equality of that path plus the presence of the old asset on the timeline as proof that the audio is unchanged. It then skips `place_narration`, reuses the old asset/sound, loads the old transcript, and preserves previous acceptance. `place_narration` imports a separate media asset, so rewriting the intermediate narration does not replace that imported timeline asset.

**Reproduction:** Voice a script, then ask the agent to redo a scene whose pronunciation is wrong without changing its text. The provider returns a different take, but `same_file` is true. The new take is generated and assembled while playback/export still uses the old imported narration. New scene boundaries can now disagree with the reused transcript/audio.

**Prior probe (8 Nov):** Linked a standalone scratch probe to the freshly built production Rust library. A fake speech provider returned two different local tones for unchanged words, with the second explicitly requested through `redo`. The actual `voice::make` returned the same narration identity although bytes changed and duration changed from 1,531,958 to 2,363,917 microseconds. Audio extracted by the actual media importer after the first take remained unchanged. No speech service was called. The subsequent `same_file` decision is confirmed by tracing its caller.

**Impact:** Paid regeneration fails to fix the audible mistake; alignment checks and previous user acceptance can apply to the wrong take. This directly affects the rendered output.

**Possible solution:** Identify a narration by the actual audio contents or immutable take IDs, including all selected raw takes and pacing settings. A changed take must produce a new asset, new transcription, and a reset of acceptance. Keep old audio immutable for undo. Do not use a filename derived solely from requested text as an audio equality test.

**Regression checks:** Use a fake speech provider returning different audio/durations on consecutive calls for identical text. Force redo and assert a new narration identity, new imported asset and transcript, updated scene times, and cleared acceptance; exact reuse without redo should still avoid paid calls.

**Questions for the fix:** When a new take changes duration, should later graphics be retimed automatically or marked stale for manual/agent repair?

<a id="gap-007"></a>
### GAP-007 — Fixed: Export-list polling repeatedly hashes the whole edit for every historical export while holding project locks

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** `export_deliveries` keeps each fingerprint between polls while the revision, short, transcripts and graphics stay the same, so a poll no longer rehashes every edit. The list never overlaps its own polls and drops answers for an earlier project. The record below is the original finding.

**Location:** `src-tauri/src/commands/mod.rs`, `export_deliveries_impl`; `src-tauri/src/export/deliveries.rs`, `fingerprint` and `list`; `front-end/src/components/export/DeliveryList.tsx`, 500 ms polling effect.

**Evidence / cause (static confirmation):** Each list request holds `opened_project` and then `export` locks while evaluating all records. For every record with a fingerprint, `list` calls `now(request)`. Main-video requests each clone the complete edit document, serialize it, and SHA-256 hash it again; short requests additionally rebuild the short document. These values depend on the current target/document, not the individual historical export, but there is no per-call memoization. Up to 100 persisted records are retained, and the UI polls twice per second while an export is active. The current implementation computes `older` even for unfinished records, although that result is then masked out.

**Reproduction / scale:** Keep 100 exports of one large edit, then start another export with the list open. One unchanged polling interval can clone/serialize/hash that same edit roughly 100 times; two polls mean roughly 200 full-document passes per second. This is an operation count derived from the loops, not a measured desktop latency.

**Prior probe (8 Nov):** A scratch probe called the production delivery writer/list implementation with 100 same-target completed exports. One `list` invocation called its current-fingerprint callback **100 times**. No real export was run for this check.

**Impact:** Avoidable CPU/allocation load grows with both project size and export history. Holding the project and export locks during the work can delay edits, playback commands, cancellation, and status requests.

**Possible solution:** Compute one fingerprint per distinct target per list request, only for completed entries that need comparison. Cache across polls by the relevant revision/content generation. Snapshot needed state under locks and perform expensive serialization outside them. Avoid overlapping frontend polls if a previous request has not completed.

**Regression checks:** Populate 100 exports of the same target and instrument fingerprint calls; expect one, not 100. Include multiple shorts, a changed revision, and an unchanged poll. Measure command/lock latency on a large real edit before and after.

**Questions for the fix:** Should export freshness also account for transcript/caption/graphic files that change outside the edit revision? That affects cache keys and freshness semantics.

<a id="gap-008"></a>
### GAP-008 — Fixed: Accepting an old narration can incorrectly certify it against a newly reviewed script

**Fixed (8 Nov, branch `claude/project-thread-sc2z5d`):** `voice.json` records the voice stage's inputs (the reviewed script's fingerprint) captured when generation starts. Completion records the stage only if those inputs are still current (`faceless::record_stage_for`); acceptance refuses narration made from an older script and, from the app, a take other than the one the user heard. The button is disabled while the voice stage is out of date. Test: `commands::faceless::tests::old_narration_cannot_certify_a_newer_script`. The record below is the original finding.

**Location:** `src-tauri/src/commands/faceless.rs`, `faceless_voice_accept_impl` and `finish_voice`; `src-tauri/src/faceless/mod.rs`, `record_stage`; `front-end/src/components/faceless/VoiceView.tsx`, “It sounds right” handler.

**Evidence / cause (static confirmation):** Acceptance loads any existing `voice.json`, sets `accepted = true`, then calls `record_stage("voice")`. That function records the hash of the review file that exists *now*. Acceptance never verifies that the loaded audio was generated from that review/script, and the voice record contains no script/review fingerprint to compare. The UI's acceptance button is disabled only while busy, even when the upstream review is stale. The long-running voice-generation path also records the current review hash on completion without checking that it matches the input used before network/transcription work began.

**Reproduction:** Generate narration A that has an issue requiring acceptance. Change the script to B and complete its review. The old voice file remains. Click “It sounds right” without regenerating B. The old audio is now recorded as satisfying B's current review and the voice stage can become Done. Similarly, change/review the script during a long generation and let the old generation finish afterward.

**Prior probe (8 Nov):** A scratch project with synthetic stage A/B files and an old voice record was opened with the production `ProjectReader`. Before calling `faceless_voice_accept_impl`, the voice stage was **Stale** after the script/review changed. After that production acceptance command, it was **Done**, without regenerating narration. This isolates stage binding; it is not an end-to-end spoken-content test.

**Impact:** The stage system can green-light visuals/export preparation while the spoken facts belong to an older script. Acceptance of pronunciation should not substitute for generating the current content.

**Possible solution:** Store immutable script/review fingerprints with generated narration and capture them at job start. Acceptance must match those fingerprints to current inputs; completion must reject or mark stale an output whose inputs changed. Pass an expected narration identity to acceptance so a late click cannot accept another take.

**Regression checks:** A → B script/review change followed by accepting A must fail or remain stale. Repeat with a delayed generation and a concurrent review change. Accepting the current narration should still clear pronunciation/length warnings.

**Questions for the fix:** Can users deliberately retain narration after purely visual/script annotation changes? If so, define a spoken-content fingerprint separately from review metadata; do not allow changed spoken words to reuse old acceptance.

<a id="gap-009"></a>
### GAP-009 — Fixed: Export freshness ignores transcript changes that alter rendered captions

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** Fingerprints include the transcript words (when captions or a file show them) and the graphic files. Subtitle, transcript and sound exports record one too. Test: `outputs_are_recorded_newest_first_and_marked_older_than_the_edit`. The record below is the original finding.

**Location:** `src-tauri/src/export/deliveries.rs`, `fingerprint` and `list`; `src-tauri/src/commands/transcript.rs`, `modify_transcript` / `transcript_run_impl`; `src-tauri/src/export/mod.rs`, caption-cue loading.

**Evidence / cause (static confirmation):** Video delivery fingerprints hash only `EditDocument` with the revision zeroed. Subtitle/transcript file deliveries have no fingerprint and fall back to comparing edit revisions. Transcription and transcript modifications save the separate transcript file without committing a new edit document. Export's caption renderer reads that separate file. Thus the same fingerprint/revision can produce different visible words and timings. Refreshing the delivery list does not fix this because the omitted input remains omitted.

**Reproduction:** Export a captioned video, then correct a transcript word or replace its transcription without changing the timeline/caption settings. Reopen the export list. The existing file is still reported as current even though another export would render different captions. Transcript/subtitle deliveries have the same dependency problem.

**Impact:** The UI can encourage publishing an export containing an old transcription error because its stale-output indicator is false.

**Possible solution:** Include hashes or durable generations of the transcripts/caption marks actually consumed by the target in an export-input fingerprint. Capture those identities with the export snapshot and use the same scheme for current comparisons. Apply equivalent treatment to other mutable external render inputs rather than assuming the edit JSON fully describes output.

**Regression checks:** Export, change a word/timing in the captioned sound, and assert stale status; change an unrelated sound not consumed by the output and define whether it should remain current. Cover subtitle/transcript files and shorts.

**Questions for the fix:** Should “current” mean identical rendered content or any changed project input? The former avoids false positives but requires target-specific dependency tracking.

<a id="gap-010"></a>
### GAP-010 — Fixed: Audio adjustments have no save queue and collide on the same edit revision

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** Audio changes go through one queue (`front-end/src/lib/audioQueue.ts`) and each applies to the latest audio settings and revision. Test: `front-end/tests/raceFixes.test.ts`. The record below is the original finding.

**Location:** `front-end/src/lib/trackMix.ts`, `saveTrackMix`; `front-end/src/components/audio/AudioSection.tsx`, `setPolish`, `update`, and `setVolume`; `src-tauri/src/commands/mod.rs`, `project_audio_update_impl`.

**Evidence / cause (static confirmation):** Every call to `saveTrackMix` immediately reads the currently displayed revision and sends an entire audio-settings snapshot. Calls are neither queued nor rebased. The component also has independent 250 ms debounces for track volumes and global audio settings, both using that same revision. Backend audio updates enforce the expected revision. Two requests sent before either response arrives therefore compete: the first advances the revision and the second is rejected. There is no automatic retry of the user's rejected intent. The controls remain enabled during these saves.

**Reproduction:** Delay the first audio-update response, change noise reduction/ducking on another lane, or let a global-settings debounce and a volume debounce fire together. Both requests carry revision N. One succeeds; the other receives a stale-revision error and its adjustment is not persisted. Volume drafts are cleared in `finally`, including after a failed save.

**Impact:** Rapid, otherwise valid mixer adjustments require the user to notice an error and repeat them. This is especially likely when an audio preparation/save takes longer than the debounce interval. The error is visible, so this is not characterized as silent data loss.

**Possible solution:** Use a project/timeline-owned serialized audio-change queue, merging patches into the latest acknowledged settings and revision at send time. Coalesce repeated slider values while retaining independent track changes. Include global settings and per-track mix edits in the same queue; preserve unsaved drafts on error. Await `applyOpenedProject` before advancing the queue when short views require an extra fetch.

**Regression checks:** Hold request A pending, issue B for a different track and C for global sound effects, release A, and assert all intended changes survive on successively current revisions. Cover failures, switching projects/shorts, and a newer slider draft while an earlier save resolves.

**Questions for the fix:** Should a failed save leave the controls showing the unsaved value with a Retry action, or restore the last saved value with a clear notice?

<a id="gap-011"></a>
### GAP-011 — Fixed: Publish commands are not bound to the project whose export the dialog shows

**Fixed (8 Nov, branch `claude/project-thread-sc2z5d`):** Every project-specific publish command takes the project handle and is refused once another project is open; package and thumbnail writes happen with the project held. An upload already started finishes for the project it began in. The agent's `publish` tool fixes the open project at the start of each call. The dialog closes and resets on a project change. Test: `commands::publish::tests::publish_calls_for_another_project_are_refused`. The record below is the original finding.

**Location:** `src-tauri/src/commands/publish.rs`, `project_root` and package/thumbnail/upload commands; `front-end/src/lib/ipc.ts`, publish API methods; `front-end/src/components/export/DeliveryList.tsx`, `publishing` state; `front-end/src/components/export/PublishDialog.tsx`, initialization and upload sequence.

**Evidence / cause (static confirmation):** Publish IPC requests carry a target such as `video`, but no project handle. Each backend call resolves the currently open project. The dialog captures the export file and draft but is not keyed to a project handle; its load effect depends only on `target`. `DeliveryList` does not clear `publishing` when `projectHandle` changes. App can receive an agent-opened-project event while a dialog is open. A delayed image picker or the multi-step save → upload sequence can consequently resume against a different project.

**Reproduction:** Open Publish for project A's export, then have a connected agent open B while the dialog remains active (or while its file picker is pending). Save/choose the thumbnail from the old dialog. The backend writes into B's `publish/` folder. If an upload proceeds, the captured file can still be A's export while its package/journal is resolved from B.

**Impact:** Metadata can overwrite another project's package; the wrong combination of video and metadata can be uploaded. This is a project-identity error, independent of the draft-reset issue in GAP-001.

**Possible solution:** Require a project handle on every project-specific publish command, validate it before resolving a root, and retain that root/identity throughout asynchronous work. Key/reset the dialog and its requests on project changes. Reject a stale action instead of retargeting it. Bind the export/delivery ID and metadata target to that same project, with explicit handling for a file supplied outside the delivery list.

**Regression checks:** Switch A → B during package loading, file picking, save, thumbnail extraction, and immediately before upload dispatch. Assert no B files change and no A file is submitted using B's package.

**Questions for the fix:** Should an already-started upload continue for A after the editor switches to B? Continuing can be supported safely by retaining A's captured context; new actions from A's stale dialog should still be rejected.

<a id="gap-012"></a>
### GAP-012 — Fixed: Export-history writes race with Forget and subtitle/audio exports

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** `exports.json` is written atomically under one lock for record and forget; a failed record is written to the export's log. The record below is the original finding.

**Location:** `src-tauri/src/export/deliveries.rs`, `work`, `record`, `forget`, and `write`; `src-tauri/src/commands/mod.rs`, `export_forget_impl` and `export_file_impl`.

**Evidence / cause (static confirmation):** The background video worker records completion after releasing its shared jobs lock. Other commands independently call `record`/`forget` on the same `exports.json`. These functions do a read → modify → write without a shared per-project lock. `write` also uses one fixed temporary path, `exports.json.tmp`. The command-side project/export locks do not protect the worker's filesystem transaction. Completion recording ignores write errors with `let _ = record(...)`.

**Reproduction:** While a video export finishes, forget an older output or export subtitles/audio for the same project. Both transactions can read the old list. Whichever writes last drops the other's change, or one rename fails because the other writer already renamed the shared temporary file. Forgetting an item can appear to succeed and then have it reappear; a successfully created output can be absent from history.

**Impact:** Export history can lose records or resurrect removed entries. The actual finished media files are not deleted by this race, but the UI loses the record needed to find/re-export/publish them.

**Possible solution:** Serialize all export-history transactions under a per-project mutex or a single writer, including the worker and file-export commands. Use unique temporary files via the existing atomic-write helper, but retain transaction locking because atomic replacement alone cannot prevent lost updates. Report persistence failures instead of silently discarding them.

**Regression checks:** Use barriers to overlap two records, and a record with Forget, then assert both independent changes survive. Inject a write/rename failure and require a visible/recoverable history error.

**Questions for the fix:** Should failure to save history mark a successful media export as “completed with warning,” or retain a pending history record for retry? The file's successful creation should remain accurately represented.

<a id="gap-013"></a>
### GAP-013 — Fixed: Research URL validation does not bind the connection to the address it checked

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** Each hop is fetched with a client pinned to the addresses its public check resolved (`resolve_public`, `resolve_to_addrs`). A proxy set on the computer still looks hosts up itself; the proxy is then what keeps requests off the local network. The record below is the original finding.

**Location:** `src-tauri/src/faceless/evidence.rs`, `check_public`, `WebPages::new`, and `WebPages::page`.

**Evidence / cause (static confirmation; no live exploit attempted):** `check_public` resolves a hostname with `to_socket_addrs`, rejects private/local addresses, and returns only the URL. The validated address list is discarded. `WebPages::page` then sends that hostname to a separately configured reqwest client, which has no pinned resolution from the check. Consequently the lookup used to decide “public” and the lookup used to connect are separate operations. Checking every redirect does not close this gap within each hop.

**Trigger:** An attacker-controlled hostname returns a public address during validation and a private/loopback address when the HTTP client resolves it. This requires control over DNS responses/timing or equivalent resolver behavior; an ordinary static private-IP URL is already rejected.

**Impact:** The code's explicit promise that research URLs cannot reach the user's local network is not enforced at connection time. HTTP research URLs can be used to make unintended local-network GET requests if the hostname resolves differently between checks. HTTPS certificate validation may restrict some variants, but HTTP is allowed.

**Possible solution:** Resolve and validate once per hop, then connect only to the validated addresses while retaining the URL hostname for Host/SNI and certificate validation. Apply the same rule to retries and redirects, and define how proxies affect this guarantee. Reuse a tested outbound-request policy across research/discovery clients where applicable.

**Regression checks:** Use an injected resolver/transport that returns a public address at validation and a loopback/private address on a second lookup. Assert that there is no unvalidated second resolution/connection. Include redirects and mixed public/private address answers.

**Questions for the fix:** Are corporate HTTP proxies supported? A proxy may perform DNS resolution itself, so the intended local-network isolation policy must specify that case.

<a id="gap-015"></a>
### GAP-015 — Fixed: Closing the new package editor cancels the last unsaved keystrokes

**Fixed (8 Nov, branch `claude/project-thread-sc2z5d`):** Package saves belong to a project/target save queue (`front-end/src/lib/packageSaver.ts`), not to the editor. Closing the editor or opening Accounts sends the last change to its own project; a failed draft is kept and shown again. A save cut off by an agent opening another project is refused (GAP-011) and its draft stays in memory only. Test: `front-end/tests/packageSaver.test.ts`. The record below is the original finding.

**Location:** `front-end/src/components/export/PackageEditor.tsx`, `change` and unmount cleanup; `front-end/src/components/export/DeliveryList.tsx`, switching from Publish to Accounts.

**Evidence / cause (static confirmation, continuation pass):** The newly added shared package editor promises to save as the user types, but only sends a save after a 600 ms timer. Its cleanup calls `clearTimeout(pending.current)` and has no flush, persistent draft, or unsaved-changes warning. Closing the export/publish dialog, or opening Accounts from Publish (which unmounts that dialog), cancels a pending save. This differs from the existing `useDebouncedSave` hook, whose cleanup flushes pending work.

**Reproduction:** Edit a title or description and immediately close the dialog or click Accounts before 600 ms elapses. Reopen it: the last edit was never sent and the previous saved text returns.

**Prior probe (8 Nov):** A hook harness mounted the actual `PackageEditor` with mocked IPC, typed a new title, and unmounted after 100 ms. After advancing beyond the debounce, the save call count remained **0**. This verifies the lost-save path without a desktop UI walkthrough.

**Impact:** Fast navigation silently loses valid metadata despite the autosave promise. This is a new lifecycle issue in the shared editor; the older thumbnail-reset issue GAP-001 is tracked separately with a continuation status note.

**Possible solution:** Give pending drafts a project/target-owned save controller that survives the component, or explicitly flush and await pending work on close/navigation. Preserve recoverable drafts when saving fails. Do not flush against whichever project happens to be open; also apply the ownership fix in GAP-011.

**Regression checks:** Type, advance less than 600 ms, then unmount through Close and Accounts. Require exactly one save of the latest valid draft to its original target. Cover a save failure and a project switch.

**Questions for the fix:** Should closing wait for autosave, or should the save continue with a visible global status/retry indicator?

<a id="gap-016"></a>
### GAP-016 — Fixed: Upload can publish old metadata because it does not await or validate autosave

**Fixed (8 Nov, branch `claude/project-thread-sc2z5d`):** Upload locks the fields, flushes and awaits every queued save (saves run one after another), re-reads the saved package and stops on a failed save or a package problem. No fixed wait remains. Test: `front-end/tests/packageSaver.test.ts`. The record below is the original finding.

**Location:** `front-end/src/components/export/PublishDialog.tsx`, `upload`; `front-end/src/components/export/PackageEditor.tsx`, `change`.

**Evidence / cause (static confirmation, continuation pass):** The upload handler waits an unconditional 700 ms and then calls `publishUpload`. The child editor starts saving only after a 600 ms debounce. The parent receives no save promise, pending state, failure status, or current draft. Waiting 700 ms is not synchronization with the save completing. The Upload button checks the last persisted `view.problem`, and text fields remain editable during the wait/upload. A failed autosave is reported only inside the editor while the parent's upload proceeds using the previous valid package on disk.

**Reproduction:** Begin with a valid saved package, edit its title/description, and immediately click Upload. Delay or reject `publishPackageSave`; advance the 700 ms timer. `publishUpload` still runs using the old metadata. Typing again during the wait can move the debounce beyond upload dispatch even without a slow backend.

**Prior probe (8 Nov):** A hook harness exercised the actual `PublishDialog` and `PackageEditor` with deferred mocked IPC. At 600 ms there was one pending save and zero uploads; at 700 ms there was **one upload while that save was still pending**. No remote upload was made.

**Impact:** A private or scheduled remote post can receive an older title, description, tags, or disclosure value than the user just entered. Once the old-title submission exists, the retry behavior in GAP-002 further complicates correction.

**Possible solution:** Provide a shared save controller or explicit `flushAndSave()` operation that Upload awaits. Freeze a consistent package/release/target snapshot at submission, surface validation/save failures to the parent, and send no upload if persistence fails. Serialize outstanding saves so an earlier response/write cannot overwrite the submission snapshot.

**Regression checks:** Keep a save pending beyond 700 ms, fail it, and type again during upload preparation. Assert zero upload calls until the intended snapshot is successfully saved; on failure require an actionable error and no remote submission.

**Questions for the fix:** Should fields be locked during submission preparation, or should edits made after clicking Upload become a draft for a later update? Either policy needs an explicit snapshot boundary.

<a id="gap-017"></a>
### GAP-017 — Fixed: Restoring a cut can insert the wrong source audio into manually linked clips

**Fixed (8 Nov, branch `claude/project-thread-sc2z5d`):** Each linked partner ending at the cut restores its own removed source range. A partner cut differently (no cut, other pauses, another length or speed) makes the restore refuse with a message saying to unlink it, rather than guessing source time. Tests: `sequence::edit::tests::restoring_a_cut_keeps_each_linked_sources_own_clock`, `restoring_refuses_a_partner_cut_differently`. The record below is the original finding.

**Location:** `src-tauri/src/sequence/edit.rs`, `SequenceEdit::Link`, `cut_after`, and `restore_cuts` (around lines 784 and 887–1001).

**Evidence / cause:** Link accepts clips from different assets and different source offsets. `restore_cuts` computes missing source ranges from the initiating clip, gathers linked partners that end at the same timeline position and speed, and creates every partner's restored piece with the same `in_us: a`. That timestamp belongs to the initiating asset, not necessarily to the partner. Validation accepts the incorrect piece if that other asset has media at the substituted time.

**Prior probe (8 Nov):** A production-library probe created a video playing source 0–2 s then 3–5 s, and synchronized external audio playing source 10–12 s then 13–15 s. Both cuts meet at timeline 2 s. Linking their left clips and restoring the video's cut correctly restored video 2–3 s, but inserted **audio source 2–3 s instead of 12–13 s**. The operation returned success and produced a valid sequence. Probe: `src-tauri/target/gap-sequence-probe.rs`.

**Impact:** Restoring an edit can introduce unrelated dialogue or sound and break synchronization without reporting an error. The same implementation can fail validation for a partner whose substituted range crosses a pause or lies outside its media, preventing an otherwise valid restoration.

**Possible solution:** Determine each partner's own removed source range and preserve its source/timeline mapping. Only restore partners whose neighboring pieces establish a compatible cut; handle different pause structures explicitly. If that cannot be determined, refuse the ambiguous operation or restore only the unambiguous clip with a clear explanation instead of guessing source time.

**Regression checks:** Link different assets with a 10-second source offset, ripple-delete a shared interval, and restore from either track. Require the original content mapping on both tracks. Include unequal source offsets within one asset, different pause locations, locked partners, and different speeds. Existing same-recording restoration tests do not exercise independent source clocks.

**Questions for the fix:** Should restoration automatically include arbitrary manually linked clips, or only partners with a demonstrably matching cut? What should happen when a linked partner has no removed source media at that boundary?

<a id="gap-018"></a>
### GAP-018 — Fixed: An export's captured edit does not capture its mutable caption and graphic inputs

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** An export captures the transcripts its captions read when Export is clicked (`ExportInputs`) and fails with a clear message if a placed graphic's files changed before it finished. The record below is the original finding.

**Location:** `src-tauri/src/export/mod.rs`, `CapturedExport`, `prepare_job`, `run_export_logged`, and `SceneEvaluator::caption_at` (around lines 205, 2316, 2577, 2655, and 1803).

**Evidence / cause (static confirmation):** Preparing a job captures an `EditDocument`, output settings, and a project-directory lease. It does not capture transcript contents or graphic source files. The worker discovers graphic render specifications from the live project directory after waiting for face analysis; graphic renders are pinned only after that work finishes. Caption cues are loaded from the live transcript store when the first caption is evaluated. The directory lease is not a versioned snapshot of those files. Thus the document revision shown for the export does not identify all of the content it will render.

**Reproduction scenario:** Start an export with captions enabled, then correct transcript text while the job is waiting or preparing graphics. When caption evaluation first runs, it reads the newer transcript together with the older captured timeline. Similarly, a graphic changed before the worker resolves its render specifications can appear in an export of the previous edit. This scenario was traced through the worker; a full rendered-video comparison has not been performed.

**Impact:** Export contents depend on timing and may mix versions of a project. Repeating an export of the same captured document need not reproduce its captions or graphics. This is distinct from GAP-009, which concerns whether an already completed export is labeled current.

**Possible solution:** Capture an immutable export-input manifest at submission: document, relevant transcripts/marks, and versioned graphic source or render identities. Pass those inputs through the renderer instead of resolving live files later. If snapshotting is too expensive, detect changed dependencies before rendering and restart or fail with a specific message. Reuse content-addressed inputs to avoid unnecessary copies.

**Regression checks:** Pause a worker after capture, mutate a transcript and a graphic, resume, and compare output against the submitted snapshot. Repeat with a mutation during preparation. Require either the original content or an explicit invalidation result, never an unreported mixture.

**Questions for the fix:** Does Export mean the project as it was when clicked or the latest content when rendering begins? The UI, job identity, input capture, and freshness checks should implement the same policy.

<a id="gap-019"></a>
### GAP-019 — Fixed: Editor shortcuts bypass modal and transcript interaction boundaries

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** `DialogBackdrop` moves focus in, keeps Tab inside, stops its keys from reaching the editor and returns focus on close. Timeline shortcuts ignore handled keys and every key while a dialog is open; a transcript cut stops its key. Escape still closes every open dialog listening for it. Test: `front-end/tests/dialogKeys.test.ts`. The record below is the original finding.

**Location:** `front-end/src/components/ui/index.tsx`, `DialogBackdrop`; `front-end/src/components/timeline/TimelineStudio.tsx`, global keydown listener around lines 1170–1188; `PublishDialog.tsx`, `AccountsDialog.tsx`, and other backdrop consumers.

**Evidence / cause (static confirmation):** `DialogBackdrop` supplies a visual overlay and pointer dismissal only: no focus placement, focus containment, restoration, or inert background. Several callers supply `role="dialog"`/`aria-modal`, but Publish and Accounts do not; none of those attributes alone isolates keyboard input. The timeline's window-level listener excludes only inputs, textareas, selects, and editable text. It does not check whether a modal is open, whether an event originated in a dialog button, or whether another handler already prevented it.

**Reproduction scenario:** Select a timeline clip, open Export or Publish, move focus to a dialog button, and press Delete or a configured split/ripple shortcut. The event passes the timeline's target filter and dispatches the underlying editing action. Tab can also move through background controls because the overlay is not a keyboard boundary. This remains code-traced; a native WebView keyboard walkthrough remains to be performed.

**Additional affected path:** The transcript panel handles Delete/Backspace by calling `preventDefault()` and cutting selected words, but does not stop propagation. The timeline's global handler ignores `defaultPrevented`. When transcript word selection coexists with timeline clip selection, the same key can therefore dispatch both a word cut and a clip deletion. Their revision checks can reject one of the competing edits; they do not make double dispatch correct. Cover this with a mounted integration test that asserts one editing action per handled key.

**Impact:** A user can alter the timeline while working in a modal, and keyboard/screen-reader users cannot reliably navigate the dialog as an isolated interaction. Space on a button can be intercepted as playback rather than activating the button. Dialogs without roles/names are additionally difficult to identify through assistive technology.

**Possible solution:** Introduce one shared accessible modal primitive with an accessible name, initial focus, focus containment, background inertness, and restoration to the trigger. Track the active modal/interaction scope and suspend editor shortcuts outside that scope; honor `defaultPrevented`. Coordinate Escape with the topmost modal rather than broadcasting dismissal to every listener.

**Regression checks:** In a real DOM/browser test, open a dialog with selected clips, cycle Tab/Shift+Tab, press Delete, Space, split, and undo, and verify only appropriate dialog actions occur. Test Escape and focus return for stacked dialogs and dropdowns. Add accessible role/name assertions for every modal entry point.

**Questions for the fix:** Which global shortcuts, if any, should remain active while dialogs are open? Should a long-running upload/export dialog be dismissible without cancelling the work? Define those separately from keyboard focus behavior.

<a id="gap-020"></a>
### GAP-020 — Fixed: Late source suggestions can add an account to the wrong platform's watchlist

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** Source suggestions are kept per platform, stale answers are dropped, and Add uses the suggestion's own platform. The record below is the original finding.

**Location:** `front-end/src/components/faceless/SourcesPanel.tsx`, `suggest`, platform-change effect, and suggestion Add handler.

**Evidence / cause (static confirmation):** `suggest` captures the platform used for a request but unconditionally writes its result to `candidates` when it completes. Changing platform clears the list once, without invalidating the request, and the platform selector remains enabled during discovery. The Add handler uses the currently selected `platform` rather than the returned candidate's own platform. Candidate records already include their originating platform, but that field is ignored here.

**Reproduction:** Start Suggest accounts on X, switch to Instagram before the response completes, then wait. The X candidates appear under Instagram; clicking Add calls `facelessWatchAdd("instagram", xHandle, ...)`. A switch to YouTube has the same risk even though that tab has no discovery button.

**Prior probe (8 Nov):** The actual component in a hook harness received a deferred candidate with `platform: "x"` after switching to Instagram. Clicking its Add control sent **`platform: "instagram"`** to mocked IPC. Probe: `src-tauri/target/gap-ui-probe.mjs`.

**Impact:** The saved research watchlist can contain a nonexistent account or an unrelated account on another service. Future research may waste paid requests or use the wrong creator as evidence.

**Possible solution:** Associate each request and result with its platform and a generation token; ignore stale results after switching. Use the candidate's validated platform when adding, and render candidates only in their originating tab. Retain a per-platform result cache if useful, with clear loading/error state.

**Regression checks:** Hold an X discovery response, switch to each other platform, resolve it, and assert no X candidates appear there. Verify Add sends the candidate's actual platform. Also resolve two platform requests in reverse order and test unmount during discovery.

**Questions for the fix:** Should discovery continue and cache results after the user leaves a tab, or should switching cancel it when the provider supports cancellation?

<a id="gap-021"></a>
### GAP-021 — Fixed: Graphic playback rereads and reparses source metadata for every frame

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** The scene evaluator keeps each graphic clip's spec per graphics generation. A graphic or style change bumps the generation; a style file edited by hand outside the app is picked up at the next change, as renders already were. Test: `a_graphic_spec_is_built_once_until_graphics_change`. The record below is the original finding.

**Category:** Performance improvement opportunity; redundant work is confirmed in source, but its contribution to dropped frames has not been measured.

**Location:** `src-tauri/src/export/mod.rs`, `SceneEvaluator::graphic_job`; `src-tauri/src/graphics/render.rs`, `Spec::for_clip` and `Spec::dress`; `src-tauri/src/graphics/mod.rs`, `read_source`.

**Evidence / cause:** Every evaluated graphic frame builds a new `Spec`. That builds default style JSON, resolves the effective style, reads `graphic.json` with `std::fs::read_to_string`, deserializes it, builds look CSS, and serializes the selected style again. The renderer caches hashes and render lookups, but this preparation occurs before those caches and even before checking pinned export renders. Spec properties usually remain constant throughout a clip.

**Work estimate:** A single continuously visible graphic across an eight-minute 30 fps export incurs at least **14,400 source-metadata reads/parses** through this path, before any extra preview/prefetch evaluations. Multiple graphic layers multiply the work. The OS may cache file bytes, but open/read calls, JSON parsing, allocation, and style serialization still occur.

**Impact:** Avoidable synchronous filesystem and allocation work sits in the frame-generation path, making preview/export more sensitive to file-system latency and large graphic metadata. This is not a measured claim of a particular frame-rate or export-time regression.

**Possible solution:** Cache resolved specs in the scene evaluator by clip, canvas, quality, document/style identity, and graphic-source generation. Build stable export specs from the captured manifest (GAP-018), and explicitly invalidate preview specs on source/style changes. Cache source metadata separately when many clips reuse a graphic.

**Regression checks:** Instrument metadata read/parse counts for hundreds of unchanged frames; require work proportional to unique specs, not frame count. Change source look/style, canvas, duration, and export fps and verify invalidation. Benchmark cold and warm preview/export with several graphic layers before setting a latency target.

**Questions for the fix:** Which file-watcher or command events are authoritative for externally edited graphics and styles? Cache lifetime must preserve that live-editing behavior.

<a id="gap-022"></a>
### GAP-022 — Fixed: Automatic captions include muted speech that the output does not play

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** Automatic captions read only tracks that are heard (not muted, gain above zero). A sound chosen for captions is still read on a muted track, as a guide track. Clip gain and noise mutes do not hide words. Test: `captions_follow_the_speech_that_plays`. The record below is the original finding.

**Location:** `src-tauri/src/export/mod.rs`, `caption_candidates` and `caption_track_cues`; `src-tauri/src/sequence/clock.rs`, `stream_entries`; compare `src-tauri/src/media/audio.rs`, `lanes`.

**Evidence / cause:** Automatic caption candidate selection visits all audio tracks, and the transcript clock includes their clips without checking track mute state. The audio mixer explicitly excludes muted tracks and zero-gain tracks. Caption selection therefore does not follow the audible timeline. Filtering candidate keys alone is insufficient when the same source has both muted and audible instances: the clock must also select the correct instances.

**Prior probe (8 Nov):** A production-library probe used one muted microphone-role track with a valid one-word transcript. `media::audio::lanes` returned **zero audible lanes**, while `caption_track_cues` returned **one caption cue** for the muted speech. Probe: `src-tauri/target/gap-caption-probe.rs`; no audio device or external API was used.

**Impact:** Captions can show discarded dialogue or an unused voice take over silence or replacement narration. Muting a track to exclude its content does not exclude its transcript from automatic caption output.

**Possible solution:** Define caption eligibility from audible speech instances in automatic mode and use a clock built from those instances. Respect track mute and the chosen zero-gain policy. Preserve an explicit opt-in if users intentionally want captions from a silent guide track; distinguish that mode in the UI.

**Regression checks:** Mute the only speech track; mute one of two different voice takes; place the same source on muted and audible tracks; set track gain to zero. Verify burned captions and subtitle export use the intended source and timing. Cover unmute and explicit-source overrides.

**Questions for the fix:** Should an explicitly selected caption source remain active when muted? Should moment-level noise mutes or clip gain also suppress words, or only whole-track mute?

<a id="gap-023"></a>
### GAP-023 — Fixed: Repeated source footage loses captions after its first timeline occurrence

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** `sequence::clock::layers` places every play of a moment; captions read each layer, so a repeated line is captioned each time. A caption edit applies to every play of that word. Test: `captions_follow_the_speech_that_plays`. The record below is the original finding.

**Location:** `src-tauri/src/sequence/clock.rs`, `entries` and its `used` source-range set; `src-tauri/src/export/mod.rs`, `caption_track_cues`; `src-tauri/src/project/revision.rs`, `transcript_clock`.

**Evidence / cause:** The shared source-to-timeline mapper intentionally gives the first placement of each source moment precedence and removes later placements using `unplaced(&used, ...)`. That policy can make sense for a single seek target, but the same mapper drives caption cue generation. Later audible repeats receive no mapping for the words already mapped once.

**Prior probe (8 Nov):** A production-library probe placed a one-second speech clip at timeline 0 s and repeated the same source at 2 s. A word at source 0.1–0.4 s generated only one cue, starting at **0.1 s**; there was no corresponding cue at **2.1 s**. Both clips remained in the sequence. Probe: `src-tauri/target/gap-caption-probe.rs`.

**Impact:** Replays, repeated quotations, and repeated hooks can contain audible speech with missing burned-in or exported subtitles. This is a product limitation of the shared clock, not a timing correction that can be solved by retranscribing the source.

**Possible solution:** Keep a first-occurrence mapper for navigation where that is useful, but generate captions per audible clip instance. Map/clamp source words through every instance, preserve speed and gaps, and identify cues by source word plus clip/occurrence so repeated words do not collide in editing/UI state.

**Regression checks:** Repeat the same source interval at different positions and speeds, including partially overlapping source ranges. Require cues for each audible occurrence, correct clipping at cut boundaries, and deterministic behavior where two speech clips overlap.

**Questions for the fix:** Should editing one repeated caption change every instance or only the selected occurrence? How should competing simultaneous speakers be presented?

<a id="gap-024"></a>
### GAP-024 — Fixed: Frontend checks do not exercise real React lifecycle, focus, or browser interaction

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** `front-end/tests/dom.ts` mounts the real React DOM renderer on a jsdom page, in StrictMode, with keys that bubble to the window. The first suite covers dialog focus and keys and choice-group arrows. More suites (package autosave with Upload, project switching) can now be added on it. Runs in `npm run test:cleanup` and so in CI. The record below is the original finding.

**Category:** Test-quality improvement; this is a coverage limitation, not a claim that the existing unit tests are invalid.

**Location:** `front-end/tests/hooks.ts`, `front-end/tests/run-cleanup-tests.mjs`, `front-end/package.json`, and `.github/workflows/ci.yml`.

**Evidence:** The frontend test command bundles TypeScript tests and runs them under Node. Component tests use a custom hook dispatcher that replaces private React internals. State setters do not schedule renders; callers render manually. `useSyncExternalStore` reads a snapshot but ignores subscriptions, and child function components are not mounted by the element traversal helper. There is no DOM/browser test runner in the frontend package or CI workflow. The app itself mounts a real React tree under StrictMode.

**Impact:** Passing these tests establishes selected logic and state transitions, but cannot establish actual rerender timing, parent/child integration, portal behavior, focus order, event bubbling/default actions, or StrictMode setup/cleanup behavior. GAP-015/016 require coordinated parent/child save behavior, and GAP-019 requires real keyboard/focus behavior; those boundaries currently lack representative coverage.

**Possible solution:** Keep the fast pure-logic tests and add a small real-renderer component/browser suite with mocked Tauri IPC. Prioritize package autosave plus Upload/Close, project switching with deferred responses, modal keyboard isolation, transcript scrolling after resize, and preview generation changes. Use public React testing APIs for lifecycle-sensitive cases instead of extending the private dispatcher to imitate more of React.

**Acceptance checks:** Run the new suite in CI, exercise actual mounted parents and children, dispatch user-like keyboard/pointer events, and assert persisted effects as well as visible state. Include StrictMode in at least one lifecycle suite. Demonstrate that the relevant regression test fails against the current buggy path before claiming coverage.

**Questions for implementation:** Which smoke flows need the actual desktop WebView, and which can run in a normal browser with IPC mocks? Keep native rendering/OS integration checks separate from browser-only behavior.

<a id="gap-025"></a>
### GAP-025 — Fixed: A preview frame can become stale during image decoding and still be painted after a seek

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** A decoded preview frame is checked again against the current generation before it is painted. The record below is the original finding.

**Location:** `front-end/src/components/canvas/NativePreviewHost.tsx`, `useEngineFrames`, `offer`, and the asynchronous `draw` loop.

**Evidence / cause:** `offer` compares the incoming frame's generation with `playbackGeneration` before decoding, then stores only a `Blob` in `latest`. `draw` awaits `decodeFrame` and subsequently checks cancellation/canvas availability, but no longer knows the frame's generation and does not recheck the acceptance predicate. A seek or view change can make the image obsolete during that await without restarting the preview-surface effect.

**Prior probe (8 Nov):** A hook probe supplied a generation-1 frame, held `createImageBitmap` unresolved, advanced the project store to playback generation 2, and then resolved the old decode. The canvas's `drawImage` was called **once with the obsolete frame**. Probe: `src-tauri/target/gap-ui-probe.mjs`; no native preview or video decoder was used.

**Impact:** Rapid scrubbing or switching playback targets can briefly show an old frame after the playhead has moved. If the next frame is delayed, the obsolete image remains visible longer. The incoming-frame filter does not cover this asynchronous boundary.

**Possible solution:** Carry frame generation and relevant view identity through the queued blob and decode. Recheck them immediately before painting, including the current `accept` predicate, and close discarded image bitmaps. Preserve the latest valid pending frame while discarding obsolete decoded output.

**Regression checks:** Delay decode, seek, and resolve; require zero stale draws and proper bitmap cleanup. Repeat for a short/video switch and unmount, then confirm the next valid frame is painted. Test both `createImageBitmap` and image-element fallback paths.

**Questions for the fix:** Should the old image be retained or cleared while waiting for the first frame of a new target? Whichever policy is chosen, a newly completed obsolete decode should not overwrite the intended state.

<a id="gap-026"></a>
### GAP-026 — Fixed: Numeric evidence checks miss negative values and do not preserve units or magnitude

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** `numbers` compares quantities: sign, scale (thousand, million, billion, k, bn), percent and percentage points. 1,000 and 1k are equal. Units, currency and what is counted are not compared, so a pass still does not prove the sentence. Test: `a_number_keeps_its_sign_scale_and_kind`. The record below is the original finding.

**Category:** Confirmed validation gap, with an additional content-quality improvement for quantity semantics. Passing this check should not be presented as factual verification of a claim.

**Location:** `src-tauri/src/faceless/evidence.rs`, `numbers` and `verify`; `src-tauri/src/faceless/script.rs`, `number_set` and scene/title checks.

**Evidence / cause:** `numbers` ignores digit matches preceded by `-` to avoid treating model names such as GPT-5 as quantities. That also ignores standalone negative quantities entirely. For positive values, verification compares only digit strings; it does not bind them to sign, unit, scale, subject, or period. Script validation reuses the same extraction. `number_context` is descriptive metadata rather than an enforced quantity comparison.

**Prior probe (8 Nov):** Against the quote “The trial included 40 people and raised $6.34 million,” production `verify` accepted all three statements without a rejection: “The trial included 40 percent of people,” “The trial raised $6.34 billion,” and “The trial lost -999 dollars.” The first two change the meaning/magnitude of supported digits; the third introduces a completely new quantity that the extractor omits. Probe: `src-tauri/target/gap-validation-probe.rs`.

**Impact:** Unsupported or materially misstated quantities can enter the fact dossier and narration despite the numeric gate. An editorial review may still catch them, but the deterministic check does not supply the protection its successful result might suggest.

**Possible solution:** Parse signed quantities separately from embedded product/version identifiers. Normalize supported unit and magnitude forms into structured values, retaining context such as currency, percent, count, and time period. Require review when transformations cannot be checked safely, and label the existing quote-presence check precisely. Do not claim that matching quantities proves the entire sentence is entailed by the source.

**Regression checks:** Negative numbers versus GPT-5/H100, million versus billion, counts versus percentages, currency changes, percentage points versus relative percentages, decimal/grouping separators, and equivalent formats such as 1,000 versus 1k. Include both false acceptances and false rejections.

**Questions for the fix:** Which numerical transformations may an author make without a new source or explicit derivation? Should derived values be represented as calculations with cited inputs rather than ordinary quoted facts?

<a id="gap-027"></a>
### GAP-027 — Fixed: Stage completion ignores the recorded output hash and can certify a replaced or corrupt result

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** A stage is done only when its own file still hashes to the recorded output. Voicing refuses a script whose review is stale or blocked, before any paid call. Test: `a_stage_whose_own_file_changed_is_not_done`. The record below is the original finding.

**Location:** `src-tauri/src/faceless/mod.rs`, `StageRecord.output`, `record_stage`, and `stage_states`; `src-tauri/src/commands/faceless.rs`, review prerequisite in `faceless_voice_impl`.

**Evidence / cause:** Completing a stage records both input and output hashes. Later `stage_states` computes the current output hash only to check that the file can be read; it never compares it with `record.output`. It compares the previous stage's file with `record.inputs` instead. Replacing a stage's own output therefore leaves that stage Done while making its successors stale. The voice prerequisite trusts the review stage's Done status without loading/validating that review again.

**Prior probe (8 Nov):** A scratch production recorded research, script, and review outputs, then replaced `review.json` with text that was not valid JSON. `faceless::summary` still returned **Review: Done**. This isolates receipt validation rather than editorial schema validation; the synthetic original outputs were used only to exercise `record_stage`/`stage_states`. Probe: `src-tauri/target/gap-validation-probe.rs`.

**Impact:** External/manual edits, partial writes, or file corruption can leave a stage falsely certified. In particular, a review file with newly added blockers or invalid content may still satisfy the gate for paid voice generation. GAP-008 addresses accepting stale narration; this issue is the missing verification of the stage's own recorded result.

**Possible solution:** Compare the current output digest with the recorded digest and mark a mismatch stale/unverified. Revalidate the result schema and semantic completion criteria before recording a new receipt. If direct edits are supported, provide an explicit validate-and-record operation rather than inferring success from file existence.

**Regression checks:** Change only the current stage's output, replace it with invalid JSON, remove it, and restore the exact recorded bytes. Verify the stage and its successors transition correctly. Add a valid review with open blockers and ensure voice generation is rejected before any paid call.

**Questions for the fix:** Which direct file edits should be automatically revalidated, and which require an explicit review/acceptance step? Should corruption be a distinct error state rather than ordinary Stale?

<a id="gap-028"></a>
### GAP-028 — Fixed: Shared tab and segmented controls expose selection semantics without matching keyboard navigation

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** `Segmented` and `Tabs` are one Tab stop; arrows, Home and End move and pick at once, skipping disabled options. Tabs do not own their panels, so no panel ids were added. Test: `front-end/tests/keyboard.test.ts`, `dialogKeys.test.ts`. The record below is the original finding.

**Category:** UI/accessibility improvement, confirmed by inspection of the shared controls; no assistive-technology walkthrough was performed.

**Location:** `front-end/src/components/ui/index.tsx`, `Segmented` and `Tabs`.

**Evidence:** `Segmented` renders a radio group of buttons and `Tabs` renders a tab list, but every enabled item remains a separate Tab stop and neither component handles arrow keys or Home/End. Tabs expose selected state without IDs or panel relationships. Native buttons still support activation, so these controls are not entirely keyboard-inaccessible; their behavior does not match the grouped navigation implied by their roles. In the editor, unhandled arrows can also reach the timeline shortcuts described in GAP-019.

**Impact:** Keyboard users must Tab through every option rather than enter the group once and move within it, and arrow navigation can unexpectedly move the playhead. Screen-reader users have less information connecting a tab to its content. Fixing this once in the shared kit benefits release choices, platform selection, format selection, and panel tabs.

**Possible solution:** Implement roving focus, selection-aware arrow navigation that skips disabled items, and deliberate automatic or manual activation. Add stable tab/panel IDs and relationships where those components own panels. Stop handled navigation events from reaching global editor actions. Alternatively use native radio inputs for simple single-choice groups.

**Acceptance checks:** One group entry point in the Tab order, correct selected/focused state after arrows and Home/End, disabled options skipped, and no timeline action on handled keys. Verify horizontal/vertical and RTL behavior where supported, plus accessible panel relationships.

**Questions for implementation:** Should arrowing through tabs immediately activate their panels, or should Enter/Space activate after focus moves? Expensive panel loading may favor manual activation.

## Retained follow-ups from the roadmap and completed audit

These were previously scattered review targets or limitations. They remain actionable without
claiming a new desktop reproduction. Feature requests such as resumable jobs and spend reports
remain in [Roadmap](ROADMAP.md#unscheduled-capabilities).

<a id="gap-029"></a>
### GAP-029 — Fixed: Checkpoint save lacks a locked project-identity recheck

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** The checkpoint save checks the project handle again under the lock. Test: `a_checkpoint_for_another_project_is_refused_under_the_lock`. The record below is the original finding.

**Status / affected paths:** Code-traced race window, not executed. AGT-1 follow-up in
`src-tauri/src/mcp/tools.rs` (`call`), `mcp/actions.rs` (`checkpoint`, save branch),
`commands/proposals.rs` (the guarded path to reuse).

**Evidence / reproduction:** `call` checks the supplied project before dispatch. Checkpoint
save later acquires `command_lock` and `opened_project` and calls `save_checkpoint` without
checking that handle again. Switch projects between the first check and lock acquisition to
test whether A's request writes a checkpoint in B. Many direct actions use `opened(ctx)` and
handle-checking commands; the old blanket claim that all direct actions are unguarded is too broad.

**Impact:** A checkpoint can belong to the wrong project and misrepresent the recovery point.
Other direct actions still need a bounded call-site audit, not an assumed defect classification.

**Potential solution:** Validate the supplied handle under the same lock as the checkpoint
write, using the proposal/command ownership pattern. Retain captured context for long jobs.

**Validation:** Barrier-controlled A→B switch before checkpoint lock acquisition must refuse
the stale call and leave B's files unchanged. Check current-project save and automatic checkpoints.

**Question / dependency:** No product decision is needed to refuse stale writes. A deterministic
barrier or injected dispatcher is needed to exercise the race; do not rely on sleep timing.

<a id="gap-030"></a>
### GAP-030 — P3: Sampled graphic checks cannot establish phone readability or continuous clearance

**Status / affected paths:** Confirmed coverage limit, not a newly reproduced overlap.
`src-tauri/src/mcp/graphic_checks.rs` (`check_moments`, `bands_with_words`),
`mcp/frames.rs`, `front-end/src/components/agent/AgentPanel.tsx`.

**Evidence / reproduction:** Checks spread 3–6 samples over each graphic. A conflict between
samples can be missed; platform-band detection is not a minimum readable type-size test.
The Agent panel does not embed the generated sample clip. Earlier real-frame observations are
in [Verification](verification.md#verified-behavior-and-remaining-limits).

**Impact:** A clean check cannot certify continuous face/caption clearance or 9:16 phone-size
legibility. STY-2's real over-face sample remains necessary.

**Potential solution:** Sample around known entry/action boundaries, add a defined type-size
heuristic, and surface the existing sample artifact where useful. Keep human motion/listening
approval; do not add the rejected agent motion-review pipeline.

**Validation:** A fixture with a brief overlap between uniform samples should be detected or
explicitly reported as unexamined; compare portrait frames at a declared phone display size.

**Questions / dependencies:** George must judge the over-face sample and agree what display
size is acceptable. Native frame/runtime access is needed for the visual checks. No minimum
font size or continuous-clearance guarantee has been agreed.

<a id="gap-031"></a>
### GAP-031 — P2: Reworded restarts remain a review-coverage gap

**Status / affected paths:** Historical labelled case plus current code inspection; not rerun
on real audio. `src-tauri/src/ai/candidates.rs`, `transcript/edit.rs`, `ai/review_probe.rs`,
`docs/plans/review-eval/en-b.json`.

**Evidence / reproduction:** EN-B already contains the optional retake “what you need to do
is” → “you need to go” (words 456–461). The old roadmap instruction to add a label first was
stale. Candidate restart rules rely on shared opening words; the previous evaluation reported
this rewording unflagged. The referenced English transcript is outside the repo.

**Impact:** Fast review can omit a retake that thorough review sees; optional labels are not
must-cut failures, and neither mode is established as universally better.

**Potential solution:** Extract a portable labelled fixture, evaluate a narrow paraphrase/slip
candidate rule, and retain manual review for ambiguous cases. Protect deliberate repetition
and meaningful connective words rather than broadening automatic cuts blindly.

**Validation:** Run rules/flags on that fixture and unseen rewordings, then compare wrong-cut
and missed-span counts with the existing labels. Repeat matched fast/thorough runs before
changing defaults ([procedure](verification.md#fast-against-thorough)).

**Questions / dependencies:** Supply or reconstruct the missing transcript/audio with stable
word timing. Decide whether this optional rewording should merely be flagged; no new blanket
auto-cut policy is authorized by the old observation.

<a id="gap-032"></a>
### GAP-032 — P3: Unplaced graphic-folder retention needs a safe ownership check

**Status / affected paths:** Retained investigation, not a newly measured leak.
`src-tauri/src/project/history.rs`, `project/revision.rs`, `graphics/mod.rs`, `graphics/versions.rs`.

**Evidence / reproduction:** The earlier roadmap reported graphic folders remaining after undo.
History restores versioned files, and unused versions may still be intentional. On a scratch
project, add/render a graphic, undo placement, prune history, then enumerate files and references
from the current edit, undo/redo, versions and checkpoints. That lifecycle was not rerun here.

**Impact:** Potential project-storage growth; neither a byte leak rate nor a broken undo path
has been established. The separate 20–40 s render observation is a dated baseline, not evidence
that deleting source folders is safe (see [GAP-021](#gap-021)).

**Potential solution:** Add reachability/retention accounting and, only if measured need exists,
an explicit recoverable cleanup of demonstrably unreferenced files. Keep old renders playable.

**Validation:** Quantify retained bytes after repeated add/undo cycles, prove every retained
history/checkpoint/version still restores, and preserve manually supplied or unclassified files.

**Questions / dependencies:** Define whether unused versions are user assets and when cleanup
may remove them. A source file being absent from today's timeline is insufficient ownership proof.

## Closed or superseded IDs

Fixed on 8 Nov, with their records left where they were: [GAP-004](#gap-004),
[GAP-008](#gap-008), [GAP-011](#gap-011), [GAP-015](#gap-015), [GAP-016](#gap-016),
[GAP-017](#gap-017), [GAP-037](#gap-037).

<a id="gap-001"></a>
### GAP-001 — Superseded: thumbnail responses reset unsaved package fields

Current `PackageEditor.tsx` calls `show(..., false)` for thumbnail responses and
`PublishDialog.tsx` no longer resets its child draft on status refresh. The original handler
is gone. Autosave loss and submission ordering remain [GAP-015](#gap-015) and
[GAP-016](#gap-016); comprehensive mounted-React dirty-draft coverage remains GAP-024.

<a id="gap-005"></a>
### GAP-005 — Closed: stale built-in style assertion

`src-tauri/src/style/store.rs`, `styles_are_saved_listed_renamed_and_removed`, now expects
Light, Dark, Broadsheet and the saved user style. Current focused test: **1 passed** with
TEMP/TMP redirected inside the workspace. The initial default-temp attempt failed on sandbox
file access, before reaching this assertion; that was not evidence of a catalog regression.

<a id="gap-006"></a>
### GAP-006 — Closed: tool schema exceeded the former budget

`src-tauri/src/mcp/mod.rs`, `the_tool_list_stays_small`, now enforces **<51,000 UTF-8 bytes**.
Its comment records the deliberate increases for `actionWord` and app-wide `publish`
(50,741 bytes at that change). The current guard passes. The historical 47,697 vs 47,500
failure applies to the earlier budget. This pass did not measure client tokens or latency.

<a id="gap-014"></a>
### GAP-014 — Superseded: absent package/publish stage wiring

`faceless::sync_package` is called by package/thumbnail saves; `record_upload` is called by
upload and refresh in `commands/publish.rs`. The current package-stage test passes. Publish
receipt creation requires a YouTube ID; a remote draft alone does not satisfy that branch.
Full release/status semantics still need the PUB-2/FCL-9 acceptance run. Stage receipt integrity
remains [GAP-027](#gap-027); the removed missing-wiring report is not proof of end-to-end success.

## Additional findings from the deeper review

<a id="gap-033"></a>
### GAP-033 — Fixed: Refreshing a corrected source keeps facts whose evidence is no longer present

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** Fetching a page again rechecks its facts: one the page no longer supports is taken out and listed as refused (a script citing it then fails its check). A page that cannot be read again keeps what was read before. Test: `a_corrected_page_drops_the_facts_it_no_longer_supports`. The record below is the original finding.

**Location:** `src-tauri/src/faceless/evidence.rs`, `fetch`, `save`, and `add_facts`; `front-end/src/components/faceless/ResearchView.tsx`, checked-facts presentation.

**Evidence / cause:** Facts are verified when added. A later `fetch(..., again: true)` overwrites the source's saved text and metadata but retains all existing facts. `save` recalculates origins, corroboration, and grade without revalidating each fact against its source. No source-content fingerprint or immutable source-version reference is stored with the fact.

**Executed reproduction:** A fake page supplied a quote, and `add_facts` accepted it. The same URL was refreshed with a correction that removed the quote. The dossier still contained **one accepted fact**, while calling the production verifier against the now-saved page returned **`quote not found in the page`**. Probe: `src-tauri/target/gap-validation-probe.rs`; no network request occurred.

**Impact:** Research can continue to present stale claims as checked facts after the source changes or is corrected. Scripts can cite the retained fact, and the earlier evidence bytes have been overwritten, weakening reproducibility of what was actually verified.

**Possible solution:** Version fetched page content and bind each accepted fact to the exact content digest and retrieval time. On refresh, revalidate affected facts and mark changed/unsupported ones stale or needing review; invalidate dependent script/review stages as appropriate. Keep the prior evidence snapshot if historical reporting remains allowed rather than silently treating it as current evidence.

**Regression checks:** Refresh with unchanged text, a removed quote, altered quantities, an explicit correction, and a fetch failure. Verify affected facts, dossier grade, and downstream stages reflect the chosen validity policy. Ensure the original evidence remains inspectable when a fact is retained for historical context.

**Questions for the fix:** Should a fact supported by an older source snapshot remain usable with an explicit as-of date, or must all production facts match the latest successful fetch? How should an unavailable source differ from a corrected one?

<a id="gap-034"></a>
### GAP-034 — Fixed: An unrelated edit during media import discards the completed extraction and requires starting over

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** An import refuses a stale revision before extracting, then commits its assets onto the latest revision instead of failing after the work is done. The record below is the original finding.

**Category:** Performance/workflow improvement opportunity. Stale-revision rejection protects consistency, but it makes otherwise independent import work unnecessarily fragile.

**Location:** `src-tauri/src/commands/mod.rs`, `project_media_import_impl`; `src-tauri/src/project/reader.rs`, `add_assets`; `front-end/src/components/media/MediaPanel.tsx`, `importFrom`.

**Evidence / cause:** The frontend captures a revision before the file picker. The backend checks project identity, then probes/extracts the entire batch without checking that revision. At final commit, `add_assets` rejects any revision change and deletes the newly extracted files. Therefore a canvas change, timeline edit, or agent edit during a long import can invalidate all completed work even though importing adds assets rather than replacing the existing timeline.

**Executed check:** A production-command probe supplied an already stale revision with a one-second WAV. It completed the extraction path and returned `Stale edit revision`. The extracted-file directory was empty afterward: cleanup is implemented correctly, so **an orphan-file leak was investigated and ruled out**. The unnecessary work/retry remains. Probe: `src-tauri/target/gap-import-probe.rs`; its original leak assertion failed and was corrected to check the observed cleanup behavior. The corrected probe was recompiled and rerun successfully.

**Impact:** Large files or multi-file batches can consume minutes of decoding and disk I/O, then leave the media bin unchanged and require the same import again. The picker delay widens the window even before extraction starts.

**Possible solution:** Reject an already stale request before expensive work. For a revision that changes during extraction, safely append the prepared assets to the latest document under the command lock after rechecking project identity, asset limits, and uniqueness; preserve the new asset IDs. If automatic rebasing is unsuitable, retain a bounded prepared-import result and offer a cheap commit retry instead of repeating extraction.

**Regression checks:** Change an unrelated layout setting during a held import and require either a safe append to the latest document or a retry that reuses extracted files. Also test project closure/switch, import failure, capacity limits, and cleanup of abandoned prepared results.

**Questions for implementation:** Should importing media be allowed to commit after unrelated edits? How long should prepared results survive if a user postpones the retry?

<a id="gap-035"></a>
### GAP-035 — Fixed: Unreadable publish metadata is silently treated as empty and overwritten by the next save

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** Write paths and the upload journal read package and upload files strictly: a damaged file is reported and never overwritten. Publish shows the problem. Test: `a_damaged_package_file_is_never_overwritten`. The record below is the original finding.

**Location:** `src-tauri/src/files.rs`, `load_json`; `src-tauri/src/publish/mod.rs`, `packages`, `save_package`, and similarly the upload-history reader.

**Evidence / cause:** The generic JSON loader returns `None` for absence, read failure, and invalid JSON alike. `packages` converts all three cases into an empty map. `save_package` then inserts one target and atomically replaces the whole file, without warning, backup, or recovery. Atomic replacement avoids a partially written new file; it does not protect the recoverable old data from this read-error path.

**Executed reproduction:** A scratch `publish/package.json` held recognizable main-video and short drafts followed by a truncated JSON field. `packages` reported zero targets. Saving metadata for the main video succeeded and replaced the file; the earlier short's draft text was gone. Probe: `src-tauri/target/gap-persistence-probe.rs`.

**Impact:** A transient read problem or damaged/externally edited metadata file can become permanent loss of all other targets' metadata on the next autosave. The UI may initially present an empty package rather than explaining that existing data could not be loaded. Applying the same default-to-empty policy to upload records also erases useful reconciliation history when those records are subsequently rewritten.

**Possible solution:** Use `Result<Option<T>, Error>` for user-authored and transaction-history stores: only NotFound means an empty store. Block replacement after parse/read errors, retain the original bytes, and offer a recoverable copy/backup plus a clear error. Audit callers of `load_json` to distinguish disposable caches from durable user content.

**Regression checks:** Missing file initializes normally; malformed or unreadable existing data yields a visible error and unchanged bytes; recovery preserves unaffected targets. Cover package, upload-history, and other user-content stores individually rather than changing cache fallback behavior indiscriminately.

**Questions for the fix:** Should the app keep one last-known-good version or a small rotating backup? Which settings are safe to reset automatically, and which require explicit recovery?

<a id="gap-036"></a>
### GAP-036 — Fixed: An unrelated project revision discards an unfinished chapter title

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** Chapter drafts reset on a project change and stay until their title is saved or the chapter is gone. The record below is the original finding.

**Location:** `front-end/src/components/chapters/ChaptersPanel.tsx`, `titles` state, revision effect, and `commitTitle`.

**Evidence / cause:** The panel clears every local title draft whenever `openedProject.revision` changes. Chapter titles save on blur, so a user can still be typing when a background import, agent edit, or other command advances the project revision. Even if the chapter itself is unchanged, the effect replaces the input with its persisted title. The draft is not in edit history and Undo cannot recover it. The effect also uses revision alone rather than project identity, so switching between projects with equal revisions need not reset drafts.

**Executed check:** Mounted the actual component with the existing hook harness, typed `Unfinished new title`, then supplied the same project and chapters with revision 2 instead of 1. After the effect, the input read `Original`. Probe: `src-tauri/target/gap-chapter-probe.mjs`. This is a deterministic state/effect reproduction, not a real DOM typing or focus test.

**Impact:** Unrelated background work can erase text while it is being entered. Blanket revision resets also prevent the UI from distinguishing a harmless refresh from a conflicting edit to the same chapter.

**Possible solution:** Keep dirty drafts keyed by project and chapter identity. Preserve them across unrelated revisions, reset on project change, and remove drafts only after an acknowledged save or explicit cancellation. If the stored title changes while dirty, surface or resolve that specific conflict instead of silently resetting every input.

**Regression checks:** Retain a dirty title through a timeline/layout revision, a background command completion, and a same-project refresh. Test external changes to the same title, save failure, chapter deletion, and project switches with equal revisions. Include a mounted React/browser case for blur plus a concurrent refresh.

**Questions for the fix:** Should a conflicting external chapter rename preserve the user's draft with a warning or require choosing a version? Should title edits autosave after an idle interval or continue saving on blur?

<a id="gap-037"></a>
### GAP-037 — Fixed: Overlapping voice jobs share scratch files and can attach another job's audio to a script

**Fixed (8 Nov, branch `claude/project-thread-sc2z5d`):** `voice::make` holds a per-production job guard before deciding what to voice; a second job for the same production is refused before any paid request. Test: `voice::tests::one_voice_job_at_a_time_per_production`. The record below is the original finding.

**Location:** `src-tauri/src/faceless/voice.rs`, `make`; `src-tauri/src/commands/faceless.rs`, `faceless_voice_impl`; desktop `faceless_voice` and MCP voice dispatch.

**Evidence / cause:** Voice generation has no per-production job guard before provider requests or file writes. Each invocation reads its own manifest but writes shared `rawN.mp3`, `voN.wav`, and `manifest.json` paths. Narration identity derives from the invocation's text/settings, while assembly reads whichever bytes currently occupy those shared paths. The desktop command runs in `spawn_blocking`; an independent MCP call can enter the same backend concurrently. The transcription guard is acquired later, after generation and timeline placement, so it cannot protect these files. A local button's busy state does not serialize independent callers.

**Executed reproduction:** Used the production `voice::make` with fake speech and explicit channels, without paid requests or sleep timing. Job A generated a one-second take and paused at the existing “Setting the pace” progress callback. Job B generated a two-second take for different text and completed in the same production directory. Resuming A produced a two-second scene under A's distinct narration filename. A therefore consumed B's raw audio while retaining A's text-derived identity. Probe: `src-tauri/target/gap-voice-concurrency-probe.rs`. This proves generation-layer corruption; the full desktop/MCP overlap was traced, not run against a provider.

**Impact:** A returned narration can contain another script's speech, with misleading identity and checks. Overlaps can also duplicate billable scene requests, overwrite manifest updates, contend on paced files, or remove intermediates another job is using. This is separate from GAP-004's single-job redo identity and GAP-008's stale review binding.

**Possible solution:** Acquire a per-production generation guard before deciding which scenes to request, and reject or queue concurrent starts with visible job ownership. Give each run unique working paths and immutable/content-addressed completed takes; commit its manifest only after validating captured script/review/settings versions. Preserve reusable successful takes after failure. If parallel jobs are a product requirement, isolate their artifacts and explicitly choose which result becomes active.

**Regression checks:** Keep the deterministic A/B interleaving above and require either a rejected/queued second job or correct isolated audio for both. Also overlap same-text redo, different voice settings, provider failure, and project switching. Assert no duplicate request for a guarded start, no deletion of another run's files, and no stage receipt for mismatched inputs.

**Questions for the fix:** Should a second request join the existing job, queue, or fail with its status? Are independent alternative takes intended, and who may choose the active one? The current single-production output should not silently intermix them.

<a id="gap-038"></a>
### GAP-038 — Fixed: Thumbnail replacement deletes the existing image before the replacement commits

**Fixed (9 Nov, branch `claude/project-thread-sc2z5d`):** A new thumbnail is written under a new name, the package is saved, then old files are removed; a failed save removes the new file. Test: `a_failed_thumbnail_replacement_keeps_the_old_one`. The record below is the original finding.

**Location:** `src-tauri/src/publish/mod.rs`, `set_thumbnail`, especially removal of both extension variants before `write_atomic` and package save.

**Evidence / cause:** After validating the incoming image, the function deletes both the old JPG and PNG paths, writes the new image, and finally updates package metadata. A failure writing the new file occurs after deletion of the previous one. If the extension changes and metadata persistence fails, the saved package can still point to the deleted old filename. Atomicity of an individual new-file write does not make this multi-file sequence transactional.

**Executed reproduction:** In a scratch publish directory, created a package referencing an existing JPG path and blocked the new PNG destination with a directory. Called `set_thumbnail` with a valid generated 640×360 PNG. The call returned an error; the old JPG path was gone and package metadata still referenced it. The prior JPG contents were placeholder bytes because this test concerns preservation of the existing file, not image decoding. Probe: `src-tauri/target/gap-thumbnail-probe.rs`. The injected destination failure is deterministic; disk-full and permission-failure variants were not run.

**Impact:** A failed replacement can remove the user's working thumbnail rather than leaving it intact for retry. If its original external source is no longer available, recovery requires a backup or recreation. Subsequent publishing sees missing thumbnail data.

**Possible solution:** Write and validate a new image under a unique filename first, atomically commit metadata referencing it, and only then retire the previously referenced file. On metadata failure, retain the old image and package and clean up only the uncommitted new artifact. Handle outstanding readers and concurrent replacements under the same project ownership/serialization rules.

**Regression checks:** Inject failures creating the new file, replacing it, and committing metadata; after each failure require the old referenced bytes and package to remain usable. Cover JPG→PNG, PNG→JPG, same-extension changes, successful cleanup, and two overlapping replacements.

**Questions for the fix:** Should the previous thumbnail remain as a recoverable version after success, or be removed once the new reference is durable? Transactional preservation on failure does not depend on that retention choice.

## Review coverage and validation

### Deeper source review, 8 November 2026

The continuation examined sequence restoration, caption mapping, export capture, graphic-frame
work, preview decoding, evidence refresh/quantity checks, stage receipts, media imports, publish
persistence, keyboard routing, chapter draft state, and concurrent narration generation. Findings
were recorded individually as they were established; application code was not changed. Existing
documentation consolidation and its closed/superseded findings were preserved. Newly appended
IDs that overlapped the consolidation's retained follow-ups were reassigned to GAP-033 onward;
the final report has 38 unique IDs, 34 open entries, and four closed/superseded entries.

Executed evidence for this review is recorded in each finding. Local Rust probes call the compiled
application library; frontend probes bundle the actual components with mocked IPC and the existing
hook harness. They are focused reproductions, not new committed regression tests or substitutes
for native WebView, audio-listening, or rendered-output verification.

| Check | Result and limit |
|---|---|
| Linked-cut restoration and caption mapping probes | Reproduced wrong linked source offset, captions on muted speech, and missing captions on repeated source intervals. |
| Evidence and stage probes | Reproduced unsupported quantities passing numeric checks, replaced review output retaining Done, and refreshed sources retaining invalidated facts. |
| Preview and source-panel probes | Reproduced a stale decoded frame painting after a generation change and an X suggestion saved as Instagram; rerun successfully at the end of this review. |
| Import rejection/cleanup probe | Recompiled and rerun: stale revision rejected; extracted files cleaned up. No orphan leak claimed. |
| Publish persistence and thumbnail probes | Reproduced overwrite of damaged metadata and loss of the prior thumbnail on replacement failure. |
| Chapter draft probe | Actual component state/effect path discarded the unfinished title on an unrelated revision; rerun successfully. |
| Concurrent voice probe | Channel-controlled interleaving reproduced A's narration using B's two-second take; fake speech only, no provider calls. |
| Focused Rust faceless tests | `cargo test --lib faceless::tests --offline -- --test-threads=2`: 6 passed, 0 failed, 1 live test ignored, 835 filtered out; TEMP/TMP redirected into `target/gap-review-tmp`. |
| Report integrity | Unique finding IDs and anchors, matching priority totals, and no unresolved internal GAP links. |

Scratch probes live under ignored `src-tauri/target/` and may disappear on a clean build.
Each proposed fix should bring the relevant minimal regression into the maintained test suite.
No live upload, paid generation, new broad suite run, or desktop performance benchmark was
performed in this continuation. Performance estimates remain code-derived opportunities unless
an entry explicitly says otherwise.

### Documentation consolidation coverage

All documents and support files under `docs/` were read or parsed, including the labelled
evaluation data and runners. Root instructions, README, shipped editing methods/skill text,
asset licence notes, test/build scripts and CI workflows were inspected. Source review traced
the listed findings through callers and tests, with narrower coverage of unrelated modules;
this is not an exhaustive audit of every line or platform.

Current focused checks and the completed AUD-001..008 audit summary live in
[Verification](verification.md#documentation-reconciliation-8-november-2026). Earlier 8 Nov
probes preserved above remain historical evidence. The earlier review suite reported
773 passed / 12 failed / 54 ignored after redirecting TEMP/TMP: two then-stale guards and ten
environment-limited HTTP/pipe failures. That snapshot does not describe today's full suite.

No live API call, paid generation, upload, DNS-rebinding attempt, native UI walkthrough,
listening check, full rendered comparison, new performance benchmark, full Rust suite,
ignored GPU/runtime probe, or Linux/macOS run was performed in this consolidation.
Do not infer a finding is resolved from a passing unrelated test or missing reproduction.
