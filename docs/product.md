# Product and direction

Status consolidated 1 November 2026. [Roadmap](ROADMAP.md) owns unfinished work;
[Verification](verification.md) distinguishes built behavior from measured results.

## Purpose and success

AeroEdits is a free, open-source desktop editor for screen-recorded tutorials, Windows first,
macOS second. It opens AeroShoot `.aero` recordings with screen, camera, microphone, system
sound and mouse/keyboard/window telemetry. George's English and Greek channel is the first
customer. The milestones stay in order: a great long-form tutorial, tutorial shorts, then gaming.

The user's own agent edits through MCP or `aeroedits do`; AeroEdits owns the timeline,
deterministic editing and rendering. Existing in-app transcription and review models remain.
There is no in-app chat. Success means accurate, readable, revisable tutorials at a lower
cost per accepted video, including rendering, corrections and human review.

## What is built

| Area | Current behavior |
|---|---|
| Edit and recovery | Non-destructive linked picture/sound tracks, ripple editing, clip speed with pitch preserved, persistent undo/redo, undo an agent run, named checkpoints. Graphic text, look and source-file edits are undoable, including files added through `graphic_write_file` (SAFE-3). |
| Speech | Online Scribe through ElevenLabs/OpenRouter with fallback; local English Parakeet. Noise muting, mouth-click repair, silence/gap cuts, filler and retake review. Muted noises use the recording's quiet room tone when available (SND-4). Key terms can correct nearby one-word mishearings while protecting common words (LNG-6). |
| Picture | Telemetry zooms, layouts, window crop, shortcut badges, bubble hiding, face centring, screen blur/redaction, spotlight, magnifier and dim/blur behind graphics. Optional jump-cut push-ins in shorts. |
| Sound | Clip gain/fades, track mix, denoise, ducking, loudness normalisation and take matching; extensible SFX/music libraries with licence checks. Graphics themselves are picture-only; sounds are separate assets. |
| Direction | Presets hold method, choices and rules; one approved on a real edit is a saved workflow, with the user's exceptions and checks, a readable summary and restorable versions. Styles hold the look, motion rules, kinds, examples, references, sound policy and portrait variant. Reference intake and preview exist. `plan` keeps a word-referenced `plan.md` inside the editing project. |
| Graphics | Editable HyperFrames compositions, shared style, library reuse, font/reading-time checks, sampled composite checks, in-context sample clips and reversible versions. Templates include timeline cuts, prompt/reply, counters, before/after, app clicks, linked steps and stacks. GFX-8 was kept by George on 1 Nov. |
| Shorts and clips | Shorts start from a separate take, cleaned normally, with chosen footage and graphics beneath it. Clips select ideas from the main video's kept words, including gathered pieces with their own timeline and resync. Captions support emphasis and corrections. |
| Review and delivery | `check` for the video or a selected short, including graphics over a face or the captions and words under a platform's bars, storyboard and graphic sheets; H.264/AAC video, subtitle/transcript/audio files, optional export queue and delivery history. Default-path repeat exports choose an unused numbered filename (FIX-11); explicit destinations keep their chosen path. |
| Preset Run | The app runs initial scan/transcription/cleanup steps on request, guarded, with local steps by default and online steps only when allowed for the run. It stops where the method asks or the agent takes over; it does not start an agent session. |

## Editing contract

The shipped preset and skill are the executable workflow; do not duplicate their tool list here.

- Read the preset, saved choices, user rules and any existing project plan. Text and telemetry
  come first; use images at decision points. Do not ask again for choices already recorded.
- Run cleanup in order: mouth noises, silence, gaps, fillers, then any extra cuts. Step mode
  previews passes with real numbers; automatic mode uses guards. Chapters come last.
- Built-in passes apply directly, one undo step each. Extra judgement cuts and timeline proposals
  await acceptance unless autopilot is authorized. Graphics are added directly as an undoable
  batch; they do not require an extra proposal card.
- Graphics follow the cuts. Show a plan table with words, timing, kind and what the viewer sees:
  start state, change with cue word, end state. Save agreed direction with the project's `plan`
  tool. For a new look, make one representative sample in context and get the user's response
  before scaling; an approved look or automatic request need not stop at every graphic.
- Fast review checks flagged moments; thorough review also reads every kept word for missed
  restarts and tangents and inspects joins. Report what was read, seen, heard and approved
  separately. The agent sees stills/boards; the user judges motion and listens.
- Run `check` on the correct video or short and inspect the result. Export only on the user's go.

## Settled constraints

These are product decisions, not invitations to reopen old proposals.

- One typed action surface for MCP, CLI and UI implementations; no arbitrary script tool or
  computer-use control of another NLE. Local account-only IPC, no network port or shared token.
- HyperFrames is the only graphics renderer. GSAP is available locally; other runtimes only
  when a chosen style needs them. Runtime downloads are pinned and hash-checked; do not use or
  alter tools on the user's PATH. Generated media may be imported as user-supplied assets.
- No project-format migrations or backwards-compatibility layers: replace obsolete fields.
- Source-time and word anchors survive cuts. Graphics are never chopped. Full-frame graphics
  hide the camera and captions; short graphics use full, split or over-face slots chosen before
  rendering. Text cards and demonstrations are equally valid when they explain the moment.
- One style dresses the video's graphics, captions and frame. References belong to the style;
  examples link to the graphic library. Shorts use a portrait variant of that same style.
  Fonts must resolve at the required weights or be supplied as files.
- Built-in styles/presets are read-only; edits create user copies. Projects keep their own
  overrides. App-wide style edits save immediately; selecting a style is undoable.
- Clips use only what the main video keeps; caption text corrections are shared. Take shorts
  may use imported outside footage, but footage drawn from the main video uses its kept edit.
  Each piece can have its own speed; the camera returns where footage ends. Under-take footage
  does not use the main video's microphone. Default short target: 30–60 seconds.
- Captions default off in long-form and on in shorts. Camera push-ins, click/whoosh effects and
  sounds on graphic events are opt-in. Better sound placement matters more than a bigger library.
- Cloud filler review is preferred when a key exists; local Qwen is the fallback without one.
  Online fillers apply directly; retakes and local-model suggestions are listed for review.
  Drinking sounds Scribe tagged (`[drinking]`) are listed too, and agents cut them with the fillers.
  Greek transcription remains online-only. Provider selection alone is not upload consent.
- Bundled music must permit monetised use without attribution; unknown or incompatible licences
  are refused or flagged on user imports. Every sound carries its licence. New bundled models
  must meet the commercial-use licence policy (MIT, Apache-2.0, GPL-3.0 as a separate component,
  or acceptable OpenRAIL terms); never non-commercial, research-only or revenue-capped.
- Feedback stays in chat and the project plan, not a new notes lane. Learning must be a proposed
  rule or a data-backed change the user accepts, never silent free-text learning. No autonomous
  creative fix-and-retry loops or agent motion/audio review pipeline.
- Aim for about 100k tokens read per edit, a preference rather than an enforced limit. Correct
  tools and output quality matter more than schema size. Keep cheaper paths only when measured
  quality holds. Source-code work is outside this editing budget.

## Acceptance criteria

A useful graphic explains the actual sentence, has a readable hold, fits its real render box,
and leaves the face, captions and platform UI usable. More motion or more graphics is not a
quality score. Technical checks distinguish faults, heuristics and checks not performed;
passing them is not proof of good design. George's keep/rework decision is the creative test.

Preserve editable source, stable anchors and undo. Inspect real composites rather than a plan
or an isolated browser render. Compare evaluation runs with the same inputs and record
corrections, call count, context, time and cost; vendor demonstrations are inspiration, not
measured evidence that AeroEdits lacks something.

## Out of scope

No second renderer, public marketplace, screen recorder inside the editor, automatic folder
watch, anchored notes, automatic secret detection or translated subtitles (DLV-4, dropped
1 Nov). Colour/LUT work, avatars, live collaboration, cloud rendering and an expanded model
catalogue are deferred. Re-voicing/lip sync stay paused until a suitable local, commercially
licensed stack exists. After Effects integration is not scheduled.
