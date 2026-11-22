# Roadmap

Reconciled 8 November 2026 against `be7179c`. This is the feature and scheduling backlog.
[gap.md](gap.md) owns detailed bugs/performance gaps, evidence, fix questions and validation.
[Product](product.md) owns settled scope; [Architecture](architecture.md) owns built behavior;
[Verification](verification.md) owns measured results and acceptance runs still missing.
Keep existing IDs. A task listed in two tables below is one task, not two implementations.

<a id="cloud-sessions-start-here"></a>
## Cloud sessions: start here

Suggested order: persistence/export correctness (the P1s, CL-1..CL-4, are fixed), then cloud tooling
and features.
George's priority and branch-policy questions remain in [HANDOFF](HANDOFF.md#questions-for-george).
For implementation PRs, run the touched checks and `scripts/test-all.sh` where supported;
mock paid services.

| ID | Task / canonical detail | Acceptance or dependency |
|---|---|---|
| <a id="cl-1"></a>CL-1 | Publish project binding: [GAP-011](gap.md#gap-011) | Done 8 Nov. Reject A→B stale package/thumbnail/upload actions; no writes to B. |
| <a id="cl-2"></a>CL-2 | Package close/autosave/upload: [GAP-015](gap.md#gap-015), [GAP-016](gap.md#gap-016) | Done 8 Nov (save queue tests; mounted dialog tests added 20 Nov in CL-15). Latest draft persists to its owner; Upload awaits successful validation/save. |
| <a id="cl-3"></a>CL-3 | Narration identity/input binding: [GAP-004](gap.md#gap-004), [GAP-008](gap.md#gap-008) | Done 8 Nov, with GAP-037's one-job guard. Changed fake take imports new audio/transcript and clears acceptance; stale script cannot certify old narration. |
| <a id="cl-4"></a>CL-4 | Linked-cut restoration: [GAP-017](gap.md#gap-017) | Done 8 Nov. Restore independently offset audio/video source clocks from either track without substituting source time. |
| <a id="cl-5"></a>CL-5 | Remaining correctness follow-ups | Done 9 Nov: every remaining gap.md finding is fixed or decided (see each record). |
| <a id="cl-6"></a>CL-6 | Remove lib clippy warnings, then gate | Done 9 Nov: 64 warnings to 0 (`clippy --fix` plus a few by hand; `too_many_arguments`, `type_complexity`, `needless_range_loop`, `result_large_err` allowed in `lib.rs`); `test-all.sh` fails on any new one, so Cloud check does. |
| <a id="cl-7"></a>CL-7 | Broaden `src-tauri/src/smoke.rs` | Done 9 Nov: silence cut; mouth-noise found and muted plus −14 LUFS ±1 normalize; 9:16 project exports 720×1280 with caption pixels. About 2 s each on lavapipe. |
| <a id="cl-8"></a>CL-8 | Standalone headless render CLI | Done 9 Nov: `aeroedits render <project> [--out x.mp4] [--short ID] [--size WxH] [--fps N] [--frames N]` (`src-tauri/src/render_cli.rs`); the smoke test exports through it. |
| <a id="cl-9"></a>CL-9 | Linux real graphics gate in CI | Done 9 Nov: `graphics-gate.yml` (graphics PRs or by hand) caches the runtime and gates all 17 templates (about 5 min); contact sheets as `graphics-gate-sheets`. Linux unzips Chrome with `unzip` (GNU tar reads no zip). |
| <a id="cl-10"></a>CL-10 | Mocked-IPC frontend preview/screenshots | Done 9 Nov: `npm run build:preview` (canned backend in `front-end/src/mock/`, written by the real commands) and `npm run screenshots`; CI's front-end job uploads `ui-screenshots` (start, editor, Export; phone and desktop). Hosting is a separate George decision. |
| <a id="cl-11"></a>CL-11 | Execute FCL-7 | Done 17 Nov: both faceless presets keep music 18 dB under the narration and choose a calm bed plus a `hit-soft` opening; the method has a Sound step. `smoke_faceless_bed_sits_under_the_narration` exports −14.1 LUFS with the bed 18.3 dB under speech. Listening is still George's. |
| <a id="cl-12"></a>CL-12 | Execute FCL-9 | Done 17 Nov, mocked: Upload of a faceless video (private or scheduled) refuses time words past their window ("today", "just announced" 12 h; "this week" 72 h after the script) and reads every cited page again (unreadable, quote gone, new correction notice). Retries and drafts skip it. `faceless/release.rs`. The approved private upload stays PUB-2. |
| <a id="cl-13"></a>CL-13 | Execute FCL-3's Script panel | Done 20 Nov: the faceless tab's Script stage shows length against the format, the check, each scene with its cited facts (exact quotes, folded), the current review's findings and when this revision was accepted (`faceless_script_get`). Mounted test `scriptView.test.ts`; phone screenshot checked. |
| <a id="cl-14"></a>CL-14 | Execute PUB-1 thumbnail generation | Done 20 Nov: Export page "Draw it" and `publish op: thumbnail` with `brief` draw one in the project's style (colours, `thumbnail` rules, references marked `thumbnail`) with `google/gemini-3.1-flash-image` on OpenRouter (about 7 cents), cut to 16:9 at 1280×720 JPEG, flagged when flat at 160 px. Mocked tests; one live call. |
| <a id="cl-15"></a>CL-15 | Real React regression coverage: [GAP-024](gap.md#gap-024) | Done 20 Nov: `front-end/tests/dom.ts` (jsdom + real React renderer, StrictMode) with keyboard isolation tests, and `publishDialog.test.ts` with mocked IPC: Close right after typing saves to its project, Upload waits for the save and a failed save stops it, switching projects keeps each draft with its own project. |

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
| FCL-3 | Script panel: scenes, cited facts, duration estimate, review findings and accepted revision | Built 20 Nov (CL-13). Agent writes/reviews, no in-app script-writing model. |
| FCL-4 | Real-script voice acceptance after CL-3 and stage-integrity fixes | No new TTS implementation needed. Follow the voice run in Verification; George listens and approves. GAP-004/008/027 are fixed. |
| FCL-5 | Finish the dedicated Broadsheet parts | Image-library browser built 21 Nov (Images panel). 21 Nov: `dates` (timeline) and `chart` (a line over time) built, gated at 16:9, 9:16 and the split part, rendered in Broadsheet at both shapes. Covered by existing templates: source `quote`, claim `verdict`, comparison `bars`, flow `steps-link`, objects a picture in a graphic of the agent's own. Left to decide: artifact (a screenshot with its line marked), gates (pass/fail checks), tiles (a grid of names), anchor and flip3d have no definition here; build one when a real story needs it. STY-2 reference approval and runtime compatibility constrain visual work. |
| FCL-6 | Agent-run real-film visual acceptance; generated-image and screenshot intake workflow | `faceless/visuals.rs` and action-word cues already exist. Intake built 21 Nov: `library` `kind: image` `use` takes a screenshot or the user's picture (`file`, credit `source`) and `draw` makes an illustration in the graphic's look at the video's shape (OpenRouter image model, about 7 cents); each is recorded in the graphic's `pictures.json`, listed per beat in `beats.json`, and `check` asks for the credit in the description and the AI disclosure. The `picture` part places one on a beat (empty, it blocks the visuals stage). Left: verify gap fill, word/action cues, reading time and visual variety on a complete production. Requires accepted voice and source assets. |
| FCL-7 | Faceless openings/bed in library, enabled by these presets and ducked | Built 17 Nov (CL-11); measured in the smoke test. Left: George listens to a real production's bed and opening before the defaults stay. Library licences are CC0 (`assets/music`, `assets/sounds` LICENSES). |
| FCL-9 | Release-time wording/source recheck, made-list/status acceptance | Recheck built 17 Nov (CL-12, mocked). Left: an approved private upload (PUB-2) showing the made-list and status outcomes, and a scheduled one. |
| FCL-10 | Complete Preset Run integration for app-owned steps, then EVAL-1 for long and short | Faceless MCP operations/methods already exist behind `more`. 21 Nov: Preset Run follows a production's stage (reports read, visuals check with fill, opening sound, music bed; paid narration and agent steps listed, never run). Left: EVAL-1. Both formats reach approved private upload with correction/time/cost evidence. Requires preceding FCL/PUB acceptance. |

FCL-0 is built foundation; FCL-2's validator/network defects GAP-013/026 are fixed;
FCL-8's remaining thumbnail generation is PUB-1. These IDs stay reserved; their completed
implementation narratives and duplicate tasks were removed.

<a id="pub-1"></a>
### PUB-1 — Generated thumbnails

Built 20 Nov (CL-14): `publish/generate.rs`. A style's thumbnail look is `Style.thumbnail` (rules)
and its references marked `thumbnail` (examples sent with the brief), so no new settings store.
Broadsheet has rules; no style has example thumbnails yet. Left: George judges real generated
thumbnails, and a style gets example thumbnails once he has ones he likes.

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
| Per-video time/provider-spend report (`spend.rs`, job/provider boundaries, UI) | 21 Nov: each paid call the app makes for a project (narration, Treg searches and page reads, release checks, drawn pictures, thumbnails, online transcription) goes to `spend.json`; `status` gives the total and `spend: true` lists it by provider and step. A cost the service didn't report stays unknown. Left: no UI; time spent, the agent's tokens, story discovery and the AI review passes are not counted. |
| Remaining “changed here” chips (`components/layouts/`, short settings, `preset/`) | 21 Nov: the video's Layouts section shows its chips (hidden while a short is in view), and a short's settings show what its look changed from its own kind's preset (shorts or clips), each taken back for that short only (`presetMarks.test.ts`). Left: none known; reopen if another settings panel lacks them. |

## Deferred work

Order within each group is the suggested order. S ≈ two days, M ≈ a week, L > a week.
None supersedes the shorts style or George's next real edit.

| ID | Work and acceptance boundary | Size |
|---|---|---|
| <a id="loop-1"></a>LOOP-1 | Use accepted/rejected proposals, restored cuts and redone graphics to propose improvements to methods, styles and templates. User accepts each change; no hidden learning. 21 Nov, first part (`corrections.rs`): `proposals_list` `corrections: true` counts what the user did with the agent's work on a project (cut items accepted and rejected by kind with examples, timeline edits turned down, actions undone, their own steps since: time put back, graphic files, zooms, layouts), and the skill sends a repeated correction to `propose_rule` for the user to accept. Left: across projects, and proposals for style and template changes (their own review step), once George has corrections to learn from. | L |
| <a id="snd-2"></a>SND-2 | Opt-in cues at graphic events. Built 21 Nov (`graphic_cues.rs`): `library` `kind: sound` `op: cues` places the style's `sound.cues` (role to library sound) where each graphic comes on (`in`, else `whoosh`), at moments its composition marks (`data-cues="0.4:pop"`) and where one held 2 s goes off (`out`). Each sound's loudest moment lands on the moment; at least 1.5 s apart and 6 a minute (entrances first); its own "Graphic cues" track ducked 8 dB under speech; one undo step that replaces the last. Left: no style has cues yet and the first ones were judged weak, so George listens and picks the sounds before a style or preset uses them. | M/L |
| <a id="ftg-2"></a>FTG-2 | Saved selects and B-roll from own footage: reviewed batch, video-only streams, free overlay track, narration untouched, word anchors. Built 21 Nov: `propose_edit` `broll` items (`media`, `fromS` or `select` = the words of a FTG-3 notes moment, `lengthS` default 4, `atWord` or `atS`) become placements of the media's picture streams only, each on the first video track above the bottom one that is free there or a new one, checked to move nothing, in the one batch the user accepts. Left: B-roll's own sound (off by design), and a real edit to judge it. | M |
| <a id="ftg-3"></a>FTG-3 | Cache source descriptions/tags from sparse boards against fingerprints. Built 21 Nov (`source_notes.rs`): `source_search` `media` shows a board of 12 frames spread over it and any notes; `media_import` `notes` (description, tags, moments) keeps them app-wide by a fingerprint of the media's bytes (size, first and last 256 KB), so a copy anywhere is never described again; `source_search` `query` finds footage by its notes beside the transcripts. Left: OCR, only if keyword search misses on real footage. | M |
| <a id="ftg-4"></a>FTG-4 | Natural take ordering, missing/duplicate checks and script-guided assembly with alternatives in one proposal. Built 21 Nov (`assemble.rs`): `propose_edit` `assemble` puts imported takes together in one batch from `atS` (default the end). With `script`, each line comes from the take that says most of its words in order (60% at least; the later take when as good), trimmed to them with 0.15 s room; the batch lists the other takes that say it, the lines none says and the takes the script doesn't use. Without, the takes go whole in name order (2 before 10), and a take a later one says again is left out. Left: a real multi-take shoot to judge the matching. | M |
| <a id="ftg-6"></a>FTG-6 | Stock providers using the user's key; record licences and download only on acceptance. Built 21 Nov (`stock.rs`), Pexels only (free, its licence needs no credit): the key goes in Settings → Accounts (`PEXELS_API_KEY` overrides it); `source_search` `stock` shows numbered previews and nothing is downloaded; `media_import` `stock` proposes ids as a batch only the user accepts (never autopilot), which downloads them into `media/stock/` and imports them with their licence and source, one undo step. Left: a live check once George has a key, and other providers only if Pexels runs short. | M |
| <a id="ftg-5"></a>FTG-5 | Import external word transcripts only if trivially small; not currently needed. | S |
| <a id="lay-3"></a>LAY-3 | Bounded auto layouts on top of `pick`. Built 21 Nov: the `paced` pick rule (off by default) holds screen 2–8 s and camera 2–5 s, carries a long rest on as a punch-in 1.5–4 s, and starts at most `shotsPerMinute` (default 3) shots in any minute, keeping the opening line first, then busy screen, then rests. One undoable step; the dry run lists the shots and `storyboard` shows them once applied. Left: George judges the pacing on a real edit before any preset turns it on. | M |
| <a id="fx-1"></a>FX-1 | Cached per-recording camera segmentation for background removal/blur/replacement; composite in wgpu. 21 Nov, first part (`face/matte.rs`): MediaPipe selfie segmentation (Apache-2.0, 450 KB ONNX, downloaded on first use and checked by hash) finds the person 10 times a second of a camera stream in the background (about 11 ms a matte on the CPU), smoothed over time, kept as `mattes/<asset>.<stream>.bin` (256² deflated, about 4 KB a matte) in the asset's own time. 21 Nov, second part: the style's camera `background` (`none`, `blur`, `remove`, `color` with `backgroundColor`; Style panel, Frame, "Behind the person") draws the person over a blurred room, nothing (what is behind the camera shows; its ring and shadow go) or one colour, from the matte at the camera's own time, in the wgpu shader and the CPU path alike (`gpu_cpu_camera_backdrops_within_tolerance`). Mattes start the first time a frame asks for them; an export waits for them. Left: the blur takes a little of the person's edge into the room (a dark halo round hair); George judges it on his own camera. | L |
| <a id="lay-4"></a>LAY-4 | Presenter cut-out layout, dependent on FX-1's matte. Built 21 Nov: the `presenter` layout (built-in "Presenter"; `right`, `sizePct` 40–90 of the height, `screen`) stands the camera's person, cut out of their room by the matte, on the bottom edge in front of the screen, with no ring or shadow; a move into or out of it takes the room away gradually. Mattes start the first time it shows and an export waits for them; until they exist the camera shows whole. Left: George judges it on his own camera. | M |
| <a id="dlv-3"></a>DLV-3 | Standalone alpha graphics, then constrained FCP7 XML/OTIO handoff. Test in the destination editor and report what was flattened. 21 Nov, first part (`export/graphics_out.rs`): `export` `file: graphics` (Export page, Graphics) writes each graphic on a shown track, as the export renders it (its slot, size and frame rate), as a QuickTime ProRes 4444 with alpha into `<name> graphics/`, named in timeline order with where it starts (`02 1m13.00s Section title.mov`). Left: the XML/OTIO handoff, once George says which editor it goes to. | M→L |
| <a id="dlv-5"></a>DLV-5 | Real 16:9→9:16 reformat preserving UI targets, cursor relationships and reading time. 22 Nov, first part (`reframe.rs`): where a screen fills a part narrower than the recording (a short, a 9:16 video), the view goes where the work is, planned once from the telemetry like a camera operator: still while clicks, the typing caret and where the pointer lands stay well inside it; an eased move (0.3 to 0.7 s) that arrives 0.6 s before a click or typing; at least 1.2 s held after each move to read. Zooms take over from it as they start. On by default (`followAction` in the short's look, Shorts → "Follow the work"). 22 Nov, second part: the whole video as a 9:16 version in one step (Export page, "9:16 version"; `export` `vertical: true`; `shorts::vertical_document`): the same edit, both pictures stacked as in a short where the video shows both, its sections kept, captions on when transcribed, as `<project> (vertical).mp4`, in the list of exports like the others. Left: George judging it on a real recording. | L |
| <a id="snd-5"></a>SND-5 | Neural noise removal. Built 22 Nov (`media/declick.rs`): Audio → Noise reduction → Neural (`noiseModel: neural`) runs RNNoise (`nnnoiseless`, BSD-3, model built in, about 100× real time on one core) once over each speech file where noise reduction is on, at 48 kHz and back, keeping every frame; mixed back so noise falls by the track's dB at most. Loudness and the other analyses measure the cleaned copy. Left: George compares it with Steady on a real recording. | S-M |
| <a id="snd-6"></a>SND-6 | Speech guard for silence cuts. Built 22 Nov (`dsp/speech.rs`): with `speechGuard: true` on the silence pass, RNNoise's voice probability (every 10 ms, the noise model already built in; Silero would need a download) marks speech, held 150 ms either side, and no silence cut overlaps it, so quiet words under the level threshold stay. It never cuts by itself. Off by default: it also hears steady tones as voice, and it changed eight tests built on tone fixtures. Test: quiet flite speech under -38 dBFS is cut without it and kept with it. Left: George tries it on a real recording before a preset turns it on. | S-M |
| <a id="ftg-7"></a>FTG-7 | Shot changes in imported footage. Built 22 Nov (`media/shots.rs`): FFmpeg's `scdet` (score 10, on a 256 px copy; cached per file) finds hard cuts in a video file; `source_search` `media` then says where each shot starts and its board shows one frame per shot (the longest 12 when there are more) instead of 12 evenly spread. TransNet V2 only if `scdet` misses on real footage. | S |
| <a id="snd-7"></a>SND-7 | Voice chain. Built 22 Nov (same cleaned copy as SND-5 and the mouth-click repair): Audio → Voice chain (`voiceChain`) puts speech through FFmpeg's high-pass at 80 Hz, a 3:1 compressor over -20 dBFS and a de-esser; the leveller and normalization still set the level. Off by default. Left: George listens. | S-M |
| <a id="ftg-8"></a>FTG-8 | Videos from a link. Built 22 Nov (`web_video.rs`): `media_import` `url` (`fromS`/`toS` for part) and the Media panel's link button download with yt-dlp (fetched on first use from its GitHub release and checked against its sums; updated once when a download fails) into `media/downloads/`, H.264 up to 1080p first, then import it with the licence the site states (Creative Commons links; else `unknown` with the link). YouTube needs a JS runtime: the graphics runtime's Node is passed when installed. Left: a live YouTube check from George's PC (the cloud VM is asked to sign in); progress in the panel. | S |
| <a id="clip-2"></a>CLIP-2 | Clip finding from excitement, Clipper's idea without chat: where the voice runs 6 dB over its usual level for the last minutes, and reaction phrases in the transcript, as signals beside the clips method's own picks; a clip opens on the loudest 3 s before it, then plays whole. Promote when clips from long videos fall short. | M |
| <a id="ui-2"></a>UI-2 | Timeline upgrades seen in OpenCut. Built 22 Nov: Duplicate (toolbar, Ctrl+Shift+D; `duplicate` edit) copies the selected clips with their partners, sound, look and speed to right after them; a gain line drawn on each sound clip, with its fades as ramps, dragged up or down for gain (double-click: 0 dB; the selection moves together). Box-select was already there. Left: markers to drop and jump between (they need to follow ripple cuts), eased curves for zoom and graphic motion. | M |
| <a id="enc-1"></a>ENC-1 | Software export at x264 `medium`, as HandBrake's quality presets. Done 22 Nov: 7% smaller than `fast` at the same SSIM on an upscaled screen recording, about as fast; CRF values and the GPU encoders unchanged. | S |
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
