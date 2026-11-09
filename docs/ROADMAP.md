# Roadmap

Reconciled 8 November 2026 against `be7179c`. This is the feature and scheduling backlog.
[gap.md](gap.md) owns detailed bugs/performance gaps, evidence, fix questions and validation.
[Product](product.md) owns settled scope; [Architecture](architecture.md) owns built behavior;
[Verification](verification.md) owns measured results and acceptance runs still missing.
Keep existing IDs. A task listed in two tables below is one task, not two implementations.

<a id="cloud-sessions-start-here"></a>
## Cloud sessions: start here

Suggested order: the five P1s, persistence/export correctness, then cloud tooling and features.
George's priority and branch-policy questions remain in [HANDOFF](HANDOFF.md#questions-for-george).
For implementation PRs, run the touched checks and `scripts/test-all.sh` where supported;
mock paid services.

| ID | Task / canonical detail | Acceptance or dependency |
|---|---|---|
| <a id="cl-1"></a>CL-1 | Publish project binding: [GAP-011](gap.md#gap-011) | Reject A→B stale package/thumbnail/upload actions; no writes to B; coordinate with CL-2. |
| <a id="cl-2"></a>CL-2 | Package close/autosave/upload: [GAP-015](gap.md#gap-015), [GAP-016](gap.md#gap-016) | Latest draft persists to its owner; Upload awaits successful validation/save; mounted parent/child tests via CL-15. |
| <a id="cl-3"></a>CL-3 | Narration identity/input binding: [GAP-004](gap.md#gap-004), [GAP-008](gap.md#gap-008) | Changed fake take imports new audio/transcript and clears acceptance; stale script cannot certify old narration. |
| <a id="cl-4"></a>CL-4 | Linked-cut restoration: [GAP-017](gap.md#gap-017) | Restore independently offset audio/video source clocks from either track without substituting source time. |
| <a id="cl-5"></a>CL-5 | Remaining correctness follow-ups | GAP-005/006 checks are complete; do not repeat their old fixes. Split remaining GAP-002/003/010/012 into focused changes; all other open findings stay in gap.md. |
| <a id="cl-6"></a>CL-6 | Remove lib clippy warnings, then gate | Mechanical changes only; `cargo clippy --lib -- -D warnings` and relevant tests pass. Historical ~60 is not a fresh warning count. |
| <a id="cl-7"></a>CL-7 | Broaden `src-tauri/src/smoke.rs` | Done 9 Nov: silence cut; mouth-noise found and muted plus −14 LUFS ±1 normalize; 9:16 project exports 720×1280 with caption pixels. About 2 s each on lavapipe. |
| <a id="cl-8"></a>CL-8 | Standalone headless render CLI | Done 9 Nov: `aeroedits render <project> [--out x.mp4] [--short ID] [--size WxH] [--fps N] [--frames N]` (`src-tauri/src/render_cli.rs`); the smoke test exports through it. |
| <a id="cl-9"></a>CL-9 | Linux real graphics gate in CI | Cache runtime; every bundled template passes `graphics_gate_real`, frames as artifacts. Needs runtime hosts/cache access. |
| <a id="cl-10"></a>CL-10 | Mocked-IPC frontend preview/screenshots | [Design](HANDOFF.md#proposal-preview-builds-of-the-front-end); start/editor/Export at phone and desktop widths as CI artifacts. Hosting is a separate George decision. |
| <a id="cl-11"></a>CL-11 | Execute FCL-7 | Same sound-bed task below; measured output loudness/ducking, no separate backlog copy. |
| <a id="cl-12"></a>CL-12 | Execute FCL-9 | Same release/source-validation task below, mocked service first. |
| <a id="cl-13"></a>CL-13 | Execute FCL-3's Script panel | Same UI task below; typecheck and mounted tests, screenshot when CL-10 exists. |
| <a id="cl-14"></a>CL-14 | Execute PUB-1 thumbnail generation | Same task below; mocked image reply, size/preview checks. |
| <a id="cl-15"></a>CL-15 | Real React regression coverage: [GAP-024](gap.md#gap-024) | Small mounted suite in CI with mocked IPC; start with package lifecycle and project switching, then keyboard isolation. Dependency review applies. |

Other open findings are not deprioritized by their absence from a CL row. Use gap.md's issue
index for export consistency, captions, outbound requests, stage integrity and UI races.
GPU/native UI, listening, real recordings, paid-service acceptance and the first private upload
require their actual environment; cloud unit success does not satisfy those checks.

## Next: STY-2 shorts style

<a id="sty-2"></a>
**STY-2 — Finish the portrait variant from George's references.** The style fields, My library
long-form style, reference proposals and sound set are built. GFX-8 is complete. Step 4 is the
remaining work, waiting for shorts references; it is not a request to rebuild the style system.

1. Break down the references into palette, typography, motion, useful graphic kinds and what
   to leave out. Keep the breakdown in the style.
2. Add portrait motion rules, kinds and linked library examples to the same style. Check fonts
   and files. The current style sound set remains opt-in; the first cues were judged weak.
3. Try one representative graphic on the 25 Oct take with the real face, captions and audio.
   Resolve the over-face placement tradeoff: lifting a low panel can clear platform UI while
   obscuring the mouth. Show the sample before building a full set.
4. Compare relevance, readability at phone size, timing and coherence. George must want to keep
   it. Then test another topic/aspect ratio and rerun EVAL-1 for workflow or quality changes.

The existing Style panel can start reference intake. The richer motion/kinds/examples fields
are available through the tools; dedicated editor lists for all these fields are still absent.

## Next: FCL faceless videos

<a id="fcl"></a>
The Rust faceless foundation, research/script validators, narration import, visual checks,
package integration and Broadsheet assets are already in this commit. They are not uncommitted
plans. See [Architecture](architecture.md#faceless-productions-and-publishing) for the current
data flow and [Verification](verification.md#faceless-and-publish-acceptance) for untried paths.
The Python explainer repository is a historical design input, not a runtime dependency.

| ID | Remaining work only | Acceptance / dependencies |
|---|---|---|
| FCL-1 | Improve event grouping and calibrate fit/attention only from labelled examples or our own outcomes | Preserve manual join/separate/triage and visible score parts; low-shared-word event splits remain a tuning limitation, not a failed new test. Needs labelled groups/outcome data. |
| FCL-3 | Script panel: scenes, cited facts, duration estimate, review findings and accepted revision | `front-end/src/components/faceless/` plus existing `faceless/script.rs` IPC; mounted rendering test and screenshot. Agent writes/reviews, no in-app script-writing model. CL-13. |
| FCL-4 | Real-script voice acceptance after CL-3 and stage-integrity fixes | No new TTS implementation needed. Follow the voice run in Verification; George listens and approves. GAP-004/008/027 remain blockers. |
| FCL-5 | Finish the dedicated Broadsheet parts and a built-in image-library browser | Style and four templates (`headline`, `quote`, `verdict`, `bars`) exist; do not rebuild them. Inventory reuse vs dedicated timeline/flow/artifact/gates/tiles/anchor/objects/flip3d parts. Gate each at 16:9, 9:16 and actual split boxes; phone-size contact sheet. Imported-image filter already exists; it is not a browser for the built-in catalogue. STY-2 reference approval and runtime compatibility constrain visual work. |
| FCL-6 | Agent-run real-film visual acceptance; generated-image and screenshot intake workflow | `faceless/visuals.rs` and action-word cues already exist. General image import exists; missing work is faceless generation/intake integration, provenance and beat placement. Verify gap fill, word/action cues, reading time and visual variety on a complete production. Requires accepted voice and source assets. |
| FCL-7 | Faceless openings/bed in library, enabled by these presets and ducked | `preset/mod.rs`, `sound_library.rs`, `media/audio.rs`; fixture export −14 LUFS ±1 with bed ≥15 dB under speech. Check licences and listen before keeping defaults; no paid generation in implementation tests. CL-11. |
| FCL-9 | Release-time wording/source recheck, made-list/status acceptance | `commands/publish.rs`, `faceless/script.rs`, `faceless/evidence.rs`: reject stale “today/just” at scheduled release and recheck sources; mocked retry/status paths, then an approved private upload. Made-list and stage hooks exist; test outcomes instead of adding duplicate wiring. Depends PUB-2 and GAP-002/011/016/027. CL-12. |
| FCL-10 | Complete Preset Run integration for app-owned steps, then EVAL-1 for long and short | Faceless MCP operations/methods already exist behind `more`; extend `mcp/first_steps.rs` only where steps are still manual. Stop at agent/user decisions and explicit paid-step consent. Both formats reach approved private upload with correction/time/cost evidence. Requires preceding FCL/PUB acceptance. |

FCL-0 is built foundation; FCL-2's remaining validator/network defects are GAP-013/026;
FCL-8's remaining thumbnail generation is PUB-1. These IDs stay reserved; their completed
implementation narratives and duplicate tasks were removed.

<a id="pub-1"></a>
### PUB-1 — Generated thumbnails

The shared package editor, chapter insertion, image/frame thumbnails and 160 px preview exist.
Remaining: generate from a brief with the user's OpenRouter image key, style-owned templates
and references. Relevant paths: `src-tauri/src/publish/`, `style/`, `commands/publish.rs`,
`front-end/src/components/export/PackageEditor.tsx`. Test mocked replies: at least 1280×720,
under 2 MB, preview readable at 160 px; request 1536×864. Decide the home/schema for style
thumbnail references before adding another settings store. Coordinate lifecycle fixes CL-1/2.
CL-14 is this task. Per-target metadata already covers videos, shorts and clips.

<a id="pub-2"></a>
### PUB-2 — Validate the existing upload workflow

Zernio account/channel selection, private/scheduled/draft requests, journal and lost-reply
lookup exist in `publish/{mod,zernio}.rs` and `commands/publish.rs`. Remaining: a real private
upload approved and performed by George, with saved package, correct disclosure/channel,
real visibility, Studio link, retry and status behavior. Do not infer success from a returned
post alone. [GAP-002](gap.md#gap-002) owns release/metadata-change policy; GAP-011/016 own safe
submission. FCL-9 owns faceless release-time/source checks. No live upload was performed here.

## Checks waiting on George

These do not block unrelated work.

- **FIX-9:** enable Transcription settings → AI agents → Allow, then verify a real Codex
  transcription session. Verify recorded authorization, not just response wording.
- <a id="cost-2"></a>**COST-2:** determine whether the Claude desktop app sends every tool
  upfront. Claude Code and Codex measurements already support on-demand schemas. Change tool
  exposure only if this client needs it; do not assume Codex refreshes lists mid-session.
- Listen to built-in music and real take matching. EQ strength (60% of the measured difference,
  capped at 4 dB) is still a tuning choice, and mood tags need ears.
- Renew the rejected ElevenLabs key in settings, or continue using OpenRouter fallback.
- Supply a longer real Greek recording: EL-B is only 13 seconds, so EL-A's synthetic case
  still carries most of the Greek evaluation.

## Engineering follow-ups

The former issue table is merged into [gap.md](gap.md): direct-action ownership → GAP-029,
sampled graphic/phone checks → GAP-030, reworded restarts → GAP-031, orphan folders → GAP-032.
Over-face design belongs to STY-2; matched fast/thorough evaluation belongs to
[Verification](verification.md#fast-against-thorough). These links replace duplicate reports.

<a id="unscheduled-capabilities"></a>
### Unscheduled capabilities

Promote only when a real job needs them. These are missing features, not newly confirmed bugs.

| Capability / affected paths | Acceptance and dependency |
|---|---|
| Resume stopped long analysis (`mcp/tools.rs`, job-owning command) | First identify a job expensive to restart; persist inputs/progress safely and prove resume equals a fresh run. Cancellation already exists. |
| Per-video time/provider-spend report (`faceless/`, job/provider boundaries, UI) | Capture app/provider/agent categories separately with missing values explicit; EVAL-1's agent cost alone is insufficient. Decide accounting scope before UI. |
| Remaining “changed here” chips (`components/layouts/`, short settings, `preset/`) | Show actual preset differences and reset only the intended owner; enumerate absent fields and test reset after project/short switching. |

## Deferred work

Order within each group is the suggested order. S ≈ two days, M ≈ a week, L > a week.
None supersedes the shorts style or George's next real edit.

| ID | Work and acceptance boundary | Size |
|---|---|---|
| <a id="loop-1"></a>LOOP-1 | Use accepted/rejected proposals, restored cuts and redone graphics to propose improvements to methods, styles and templates. User accepts each change; no hidden learning. | L |
| <a id="snd-2"></a>SND-2 | Opt-in cues at graphic events: event markers, sound-peak offsets, density limits and ducking, using the style's sound set. Improve the sounds before scaling. | M/L |
| <a id="ftg-2"></a>FTG-2 | Saved selects and B-roll from own footage: reviewed batch, video-only streams, free overlay track, narration untouched, word anchors. Not urgent (1 Nov). | M |
| <a id="ftg-3"></a>FTG-3 | Cache source descriptions/tags from sparse boards against fingerprints; OCR only if keyword search misses. | M |
| <a id="ftg-4"></a>FTG-4 | Natural take ordering, missing/duplicate checks and script-guided assembly with alternatives in one proposal. | M |
| <a id="ftg-6"></a>FTG-6 | Stock providers using the user's key; record licences and download only on acceptance. | M |
| <a id="ftg-5"></a>FTG-5 | Import external word transcripts only if trivially small; not currently needed. | S |
| <a id="lay-3"></a>LAY-3 | Bounded auto layouts on top of `pick`: few changes/minute, screen 2–8 s, camera 2–5 s, punch-in 1.5–4 s, review with list and storyboard. | M |
| <a id="fx-1"></a>FX-1 | Cached per-recording camera segmentation for background removal/blur/replacement; composite in wgpu. | L |
| <a id="lay-4"></a>LAY-4 | Presenter cut-out layout, dependent on FX-1's matte. | M |
| <a id="dlv-3"></a>DLV-3 | Standalone alpha graphics, then constrained FCP7 XML/OTIO handoff. Test in the destination editor and report what was flattened. | M→L |
| <a id="dlv-5"></a>DLV-5 | Real 16:9→9:16 reformat preserving UI targets, cursor relationships and reading time. | L |
| <a id="cost-3"></a>COST-3 | Cheaper narrow reviewers, retained only if EVAL-1 quality holds. Not urgent (1 Nov). | M |
| <a id="ver-1"></a>VER-1 | Repeat Windows bridge-to-export verification on macOS when macOS work resumes. | M |
| <a id="dep-2"></a>DEP-2 | Quarterly dependency review, next January 2027, or sooner for an advisory or measured gain; follow Verification. | S |
| <a id="dep-3"></a>DEP-3 | Tailwind 3 to 4, by hand or checking every change: the upgrade tool changes 266 places in 45 files and also rewrote the saved `EffectKind` value `"blur"`. Then compare every panel with the old build. | M |
| <a id="dep-4"></a>DEP-4 | `ort` 2.0 and parakeet-rs 0.4, when `ort` leaves release candidates: one ONNX Runtime environment handed to every model (seven call sites), a CUDA pack matching the newer ONNX Runtime, then the Parakeet, face detection and pack probes. | S-M |
| <a id="fx-5"></a>FX-5 | LUT import/simple grade with preview/export parity. Parked: not needed for the channel. | M |

AeroShoot work belongs in the recorder repo: exact segment boundaries (O03, overlap up to one
frame) and removal of the 10 ms system-audio gap at boundaries (O04). The editor tolerates both.

Promote unscheduled ideas only when a real edit needs them: alternate review timelines,
narration fixes, relinking/media bins, cross-project copying, arbitrary reference-range export,
reduced motion, versioned delivery safe zones and
music-beat boards. Do not resurrect dropped scope listed in [Product](product.md#out-of-scope).
