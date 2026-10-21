# DaVinci Resolve: features, MCP and AeroEdits gaps

Research date: 21 October 2026. Compared Blackmagic's current Resolve 21 feature page,
the Resolve 21.1 reference manual and MCP implementation reports with this checkout.
This is a product/code review, not a live Resolve integration test. No Resolve MCP was
installed or connected. Existing uncommitted backlog/research changes were left intact.

## Recommendation

Build **source search first**, then **consistent speech across takes**, then
**script-guided assembly**. Support these with persistent edit recovery and clearer MCP
diagnostics. AeroEdits already has agent editing, multiple layouts, shorts, graphics,
pitch-preserving speed changes and export checks. Adding those again would duplicate work.

Resolve is a useful reference for finding footage, assembling it and finishing sound.
Its Photo page, immersive delivery and advanced face effects have much less relevance
to our screen-recorded tutorial workflow.

## What Resolve contributes

Blackmagic's [What's New page](https://www.blackmagicdesign.com/products/davinciresolve/whatsnew)
describes content search, script-based timeline assembly, reference-based audio matching,
reusable effect chains, HTML/Lottie graphics, richer animation controls, media ratings,
speech generation and vertical delivery. These are the relevant feature leads;
the page does not enumerate the native MCP tools.

The [21.1 manual](https://documents.blackmagicdesign.com/UserManuals/DaVinci_Resolve_21.1_Reference_Manual.pdf),
page 4318, confirms built-in MCP support in **Studio**. Its examples include timeline
inspection, clip renaming, marker export, LUT application, project-setting comparisons,
vertical reframing and multiple render formats. These examples describe workflows;
they do not establish that every Resolve UI control has a corresponding scripting API.

Further manual checks refine the feature leads:

- **IntelliSearch, p. 396:** requires downloaded models and media analysis. Build our
  index explicitly, with progress and cache invalidation; search is not free at ingest.
- **IntelliScript, pp. 1003–1004:** matches spoken dialogue from `.txt`/`.fdx` scripts and
  places alternative takes on disabled tracks. Its choice of the best take is based on
  dialogue matching, not performance quality or good-take metadata. Our proposal should
  show alternatives rather than claim to judge delivery from words alone.
- **EQ/Level/AI Dialogue Matcher, pp. 3886–3887:** distinguish tone, loudness and room
  ambience. Start our work with the first two; ambience matching is a separate feature.
- **Inspector presets, p. 753:** can capture a whole parameter tab or a subset. This is a
  good reference for predictable saved audio/layout settings.
- **Project backups, p. 87:** recoverable independent project copies provide a practical
  model for our persistent recovery task.
- **Transcription, p. 1003:** Greek is included in the extended transcription package.
  AeroEdits already supports Greek online, but local Greek transcription/alignment remain
  meaningful gaps under existing backlog tasks 2 and 12. Accuracy on mixed Greek/English
  tutorial speech needs testing before choosing a model.

## How the native MCP works

The manual's setup is **File > Setup AI Assistants**. Resolve confirms which installed
assistants it found; interaction then happens in the assistant's own chat interface.
The assistant may need restarting. The extension can opt to launch Resolve when needed.
This is an integration with an external agent, rather than a new editing model embedded
in Resolve. The manual explicitly notes that generated scripts share the assistant's
access permissions to disk, network and other resources.

The important conceptual flow is:

```text
User request → external assistant → MCP tool call
             → discover scripting API / run script → Resolve project
             ← results and state ← inspect / verify
```

An [independent, first-hand Windows 21.1 audit](https://github.com/dmmdea/davinci-resolve-cli/blob/main/docs/09-resolve-21-1.md#3-the-native-mcp-server-is-a-stdio-zipapp-not-a-listener-measured)
reports a stdio `ResolveMCP.exe`, a Claude `.mcpb` bundle and these 14 tools:

| Purpose | Reported native tool names |
|---|---|
| Application | `launch_resolve`, `get_resolve_status`, `get_whats_new` |
| API discovery | `get_scripting_api`, `search_scripting_api`, `get_scripting_docs` |
| Execute | `run_script`, `run_script_unsafe` |
| Color assets | `list_dctls`, `list_luts`, `update_dctl`, `delete_dctl`, `delete_lut`, `generate_lut` |

That audit describes restricted execution for `run_script` and unrestricted execution for
`run_script_unsafe`. Its observed script timeout is at most 60 seconds. This inventory is
reported for the audited Windows build, not independently reproduced here or guaranteed
for every platform/update. The architecture delegates domain operations to scripts;
14 tools do not mean only 14 editing operations.

### What that opens up in practice

Through the scripting methods available in the installed version, an assistant can inspect
project state, compose repeatable media/timeline operations and queue delivery work.
API documentation discovery helps it generate the right calls for that version.
The manual's concrete examples make three useful patterns clear:

- **Inspect before changing:** describe offline media, markers and timeline contents.
- **Apply a consistent operation to many items:** naming, looks and settings checks.
- **Produce several deliverables from one instruction:** multiple formats/resolutions.

Do not equate an advertised Neural Engine feature with proven MCP automation support.
Check the installed API, required models and actual result for the desired operation.
Color scripting also does not imply that every color-page control is writable.

### Community MCP is a separate product

The [samuelgursky server's own documentation](https://github.com/samuelgursky/davinci-resolve-mcp#server-modes)
currently lists 37 compound tools, 390 granular tools and a separate 18-tool offline server.
Its live capabilities span project/media management, analysis, timeline operations,
review markers, grading, Fusion and delivery. It documents persistent source-analysis
artifacts and capability checks. These are maintainer claims, not tests performed here.

Its API-coverage percentages refer to scripting methods, not the whole Resolve interface.
Its free-edition in-app bridge and offline file/database editing should not be confused
with Blackmagic's native Studio MCP.

## What AeroEdits already has

The code registers **53 tools**: two guide tools in
[`skill.rs`](../src-tauri/src/mcp/skill.rs), 14 inspection/proposal tools in
[`tools.rs`](../src-tauri/src/mcp/tools.rs) and 37 actions in
[`actions.rs`](../src-tauri/src/mcp/actions.rs).

| Area | Existing implementation | Implication |
|---|---|---|
| Agent connection | Stdio bridge to private local IPC; one-click Claude Desktop/Codex configuration | Basic MCP integration is already built. |
| Inspect the edit | Project, transcript, timeline, playhead, frame, layout, telemetry, filmstrip/waveform and storyboard | Keep these domain-specific tools; they provide useful context without scripts. |
| Make edits | Proposed cuts/sequence edits, stable word IDs, revision checks, batch undo; direct workflow actions | Extend the review model to new workflows rather than replacing it. |
| Picture-in-picture / vertical | Bubble and split layouts, timed sections, face centering, shorts and multiple canvas ratios | Resolve's PIP and vertical features are overlap, not missing fundamentals. |
| Motion graphics | HyperFrames source, templates, custom files, rendering, brands and library tools | Missing interchange/animation polish is narrower than “add graphics.” |
| Speech cleanup | Silence/filler/retake passes, mouth-click repair, spectral denoise, normalization, ducking | The audio gap is matching and shaping sound, not basic cleanup. |
| Delivery | H.264/AAC, frame-rate/bitrate controls, main video and shorts export, status and preflight checks | Improve deliverable management before broad codec expansion. |

Evidence: [`mcp/mod.rs`](../src-tauri/src/mcp/mod.rs),
[`bridge.rs`](../src-tauri/src/mcp/bridge.rs), [`clients.rs`](../src-tauri/src/mcp/clients.rs),
[`project/audio.rs`](../src-tauri/src/project/audio.rs),
[`sequence/edit.rs`](../src-tauri/src/sequence/edit.rs) and
[`export/mod.rs`](../src-tauri/src/export/mod.rs).

## Gaps worth addressing

Priorities below are recommendations for tutorial editing, not measured user demand.
Proposed tool names are illustrative; none are implemented by this research.

| Priority | Gap and value | Current evidence / smallest useful addition |
|---|---|---|
| 1 | **Search the source, including removed material.** Find “where I open the export dialog” and reuse it under narration. | Media search filters asset names and kinds; `transcript_get` presents kept words. Start with source-transcript keyword/phrase search returning asset/stream, source ranges, excerpts and word IDs. Then add cached scene descriptions and OCR. Existing backlog task 14. |
| 1 | **Match speech across takes.** A re-recorded sentence should not suddenly become louder or sound like a different microphone. | Audio settings expose normalization, denoise, ducking and track gain, but no reference-based level/EQ matching. Start with measured speech-level matching, bounded gain, bypass/A-B and reported clipping risk; add gentle EQ matching only after listening tests. |
| 1 | **Recovery after a whole agent workflow.** Restore the previous edit after reopening the app. | `EditHistory::new` starts empty undo/redo vectors. Persistent proposal history exists, but is not general persistent edit history. Save a recoverable checkpoint before a workflow, with source references and schema version. Existing task 13. |
| 2 | **Script-guided assembly.** Turn a supplied tutorial outline into an ordered rough cut. | Transcript cuts and retake detection exist, but no dedicated supplied-script matching workflow was found. Search source transcripts first; show candidate takes, unmatched lines and ambiguity; apply accepted source selections as one proposal batch. |
| 2 | **Reusable delivery sets.** Queue the tutorial, selected shorts and accompanying files together. | Shorts can already be exported in a batch; the broader gap is named multi-output recipes, persistent queue/retry and per-output status. Start with existing H.264/AAC outputs plus transcript/subtitle/chapter sidecars, then add codecs only for a concrete workflow. |
| 2 | **Reusable sound treatment.** Apply the same voice settings to new recordings. | Existing project settings/templates provide a starting point. Add a small saved speech preset covering current cleanup plus any new EQ/compression. Do not require a general plugin host. |
| 2 | **Selects and review notes.** Mark good demonstrations, rejected takes and changes without turning them into chapter headings. | Chapters and agent proposal history exist; general source-range ratings, notes and saved selects were not found in the inspected model/tool surface. Start with named ranges and reasons linked to search results. |
| 3 | **Animation authoring and interchange.** Make callouts easier to reuse and adjust. | Zoom/layout easing and HTML compositions exist; no native Lottie/OGraf import or general parameter-curve editor was found. First add a few exposed timing/easing controls and transparent overlay templates; consider Lottie only with a tested alpha/seeking/render adapter. HTML support alone does not establish OGraf compatibility. |
| 3 | **Voiceover replacements.** Repair a sentence without re-recording the whole segment. | Resolve advertises speech generation. Our revoice lab is an experiment, explicitly separate from the app. Start with imported replacement audio and alignment/review before integrating voice generation; verify word timing, pronunciation and camera sync. |

Specific source-search evidence:
[`MediaPanel.tsx`](../front-end/src/components/media/MediaPanel.tsx) filters `asset.name`;
[`tools.rs`](../src-tauri/src/mcp/tools.rs) describes and builds the edited transcript.
Recovery evidence: [`revision.rs`](../src-tauri/src/project/revision.rs).
Voice experiment: [`revoice lab README`](plans/revoice-lab-scripts/README.md).
The [existing backlog](future-work.md) already covers source search, history, backgrounds,
media reuse and MCP discovery. This review supports those entries rather than creating
duplicate tasks.

The audio recommendation draws on Resolve's reference-matching features; its broader
[Fairlight documentation](https://www.blackmagicdesign.com/products/davinciresolve/fairlight)
also provides a useful reference for clip/track processing and reusable sound workflows.
The [Edit page](https://www.blackmagicdesign.com/products/davinciresolve/edit) is a reference
for editing, animation and delivery workflows. The proposed scope above is deliberately
smaller and tailored to our recordings.

## MCP improvements to take from this research

1. **Connection and capability report.** A read-only report should work without a project:
   app/bridge versions, connection state, installed transcription/graphics runtimes,
   supported operations and actionable failures. Existing `status`, `scan` and handshake
   versioning are foundations, not the complete report. This overlaps backlog task 16.
2. **Discover only what is needed.** Measure the 53-tool schema cost before changing the
   interface. Consider searchable descriptions/on-demand schemas where clients benefit.
   Resolve's docs-first scripting surface and the community compound mode are two different
   approaches; neither requires us to expose arbitrary script execution.
3. **Search evidence that survives edits.** Proposed `source_search` and `source_describe`
   should return stable evidence references, file fingerprints and explicit source time.
   Existing edited-time tools should retain their current meaning.
4. **Review complete workflows.** Proposed script assembly/audio match should return
   evidence and a concrete preview, then use revision-checked batch application.
   Our proposals are a useful foundation. Direct action tools also exist today, so it would
   be inaccurate to claim every MCP mutation already waits for approval.
5. **Manage long jobs.** Export already supports background start/status and tools report
   progress. The inspected dispatcher uses blocking work with a progress callback, without
   a general job/cancellation contract passed into tool implementations. Add resumable
   source-analysis jobs and explicit cancellation; a client timeout alone is insufficient.
6. **Verify through the real bridge.** On Windows and macOS, exercise inspect → propose →
   accept → inspect → undo → render/export on disposable projects. Tool count and unit tests
   do not establish end-to-end agent reliability.

Keep the current bounded editing tools. Resolve's arbitrary-script model is useful for
its enormous API, but source search, script assembly and audio matching can have typed,
reviewable inputs in AeroEdits.

## Suggested order and acceptance checks

1. **Source retrieval:** find labelled moments in a real tutorial, including a cut-out
   moment and an imported take. Verify source playback and stable results after unrelated
   timeline edits. Measure misses/latency before adding embeddings.
2. **Recovery and diagnostics:** restore after restart; distinguish app closed, no project,
   unavailable runtime and unsupported action. Reuse existing backlog tasks 13 and 16.
3. **Audio consistency:** compare original/matched consecutive takes by listening and
   measured speech level; preserve duration, avoid clipping and support undo.
4. **Script assembly:** match an outline with repeated lines and missing material; keep
   uncertainty visible and verify source boundaries/audio sync after acceptance.
5. **Delivery recipes:** export a main video and multiple shorts, fail one output, retry it
   without losing completed work and retain which revision each output rendered.

Defer photo editing, immersive video, face aging/reshaping, cinema-grade color management
and a full Fusion-style node editor. Webcam background removal remains relevant, but is
already backlog task 9. A minimal editable handoff into Resolve could extend task 15;
test source ranges, time bases and flattened layouts/graphics in Resolve before promising
a round trip.

## Research limits

- The official manual confirms setup and example workflows; it does not publish the
  complete 14-tool schema on page 4318. The exact inventory above comes from an explicitly
  identified implementation audit, not our own running Resolve session.
- The current marketing page presents Resolve 21 features and links to a 21.1 video;
  these should not all be labelled new in 21.1.
- Repository findings reflect inspected models, registrations and implementations, not a
  UI acceptance test. Backlog entries and experimental scripts are not shipped features.
- No application code, client settings or installed integrations were changed.
