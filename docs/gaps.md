# Gaps and improvements

What is still missing or wrong, checked against the code. Each gap says what is wrong, the
evidence, and the roadmap item that fixes it ([roadmap.md](roadmap.md)). What the product does
today is in [product.md](product.md). A gap that is fixed leaves this file (24 Oct 2026: the
fixed rows went; the roadmap's [Done](roadmap.md#done) and git keep them).

**Evidence grades.** **Code:** checked in `src-tauri/src` or `front-end/src`. **Log:** from our
own build or measurement logs. **Research:** from competitor or creator material
([research/](research/README.md)); creator claims are not measurements.

## The short version

1. **The agent is blind to time.** Every review tool is a still; tools return text and PNG
   only, and the agent can't hear. The user's ears check the joins; the app doesn't yet hand
   them the risky ones.
2. **No plan before the build.** The agent keeps no plan between turns.
3. **The craft surfaces are thin:** no sound placement, no clip gain or fades, few tutorial
   graphic templates, no search over the source footage.
4. **Shorts are one continuous stretch** (clips) or one take: no idea-first short built from
   pieces across the video.
5. **Some things are absent from the app itself,** not just from the agent: NLE handoff,
   tutorial effects (spotlight, redaction), background removal.

---

## 1. Small bugs

None open (24 Oct).

## 2. Safety and recovery

| Gap | Evidence | Fix |
|---|---|---|
| Graphic source edits (`graphic_set` variables, `graphic_write_file`) are not undo steps; checkpoints keep the files, and the app saves one before every agent run | Code: `commands/graphics.rs`, `project/checkpoint.rs` | [GFX-4](roadmap.md#gfx-4) |
| Direct actions check the `project` id just before running rather than under the command lock | Code: `mcp/tools.rs` | [AGT-1](roadmap.md#agt-1) follow-up |

## 3. Agent surface

| Gap | Evidence | Fix |
|---|---|---|
| The agent can't move the user's playhead to show them a moment (`playback_*` is UI-only) | Code | [REV-1](roadmap.md#rev-1) |
| `scan` and `telemetry_get` have no UI; `fillers_review`, `graphic_templatize` and `graphic_check` can't be run or audited from the UI. Low priority, but worth knowing when a user asks "what did the agent see?" | Code | — |
| No resumable analysis: progress and stopping a call exist, resuming a stopped job doesn't | Code: actions run blocking work with a progress callback | [AGT-4](roadmap.md#agt-4) |
| No in-app report of time and provider spend per run (EVAL-1 measures the agent's side only) | Code: 0 matches for `cost|usd|token_usage` in `src/ai` | [EVAL-1](roadmap.md#eval-1) |

## 4. Review: the agent can't see motion or hear

| Gap | Evidence | Fix |
|---|---|---|
| Every review tool is a still (`frame_render`, `timeline_view`, `storyboard`); MCP output is text and PNG only, and Claude takes no sound. Joins and the mix can only be checked by the user's ears | Code: `mcp/tools.rs` | [REV-1](roadmap.md#rev-1) (the agent picks the joins, the user listens); [COST-1](roadmap.md#cost-1) for cheaper sight |
| No list of risky cuts to listen to after the edit (the Jump cuts panel auditions pauses only before they are cut) | Code: `JumpCutsPanel.tsx` | [REV-1](roadmap.md#rev-1) |
| Thorough review mode is in the skill but not yet measured on a real recording | Log | [AGT-2](roadmap.md#agt-2) |

## 5. Intent: plans and the user's method

Anchored notes and a selection reference were dropped 24 Oct: feedback goes in chat
([decisions](decisions.md#not-doing)).

| Gap | Evidence | Fix |
|---|---|---|
| **No plan before the build.** `storyboard` reviews the edit that exists, not the one to come; the agent keeps no plan between turns | Code: `mcp/storyboard.rs` | [PLAN-1](roadmap.md#plan-1) |
| A style has no references (images, a video) and no sound policy; agent-proposed styles are not built | Code: `style/` | [GFX-5](roadmap.md#gfx-5) (proposed styles), [STY-2](roadmap.md#sty-2) |

## 6. Captions

| Gap | Evidence | Fix |
|---|---|---|
| Greek glyph coverage in graphics never tested (captions: tested 23 Oct, every bundled weight) | Log | [GFX-3](roadmap.md#gfx-3) |

## 7. Sound and music

| Gap | Evidence | Fix |
|---|---|---|
| The only SFX is a synthesized click/whoosh lane with four settings, off by default, no import path | Code: `sound_effects.rs` | [SND-1](roadmap.md#snd-1) |
| Clips have no gain and no fades; a music bed can only be an imported asset placed by hand, with track-level ducking | Code: `sequence/mod.rs` `Clip`; `project/audio.rs` `TrackMix` | [SND-1](roadmap.md#snd-1) |
| No loudness *target* for a mix (we measure and normalise output, but have no speech-relative music level) | Code | [SND-1](roadmap.md#snd-1) |
| No cue placed on purpose at a visual event (graphics are code, so the exact frame is knowable); no licence record per sound | Research | [SND-2](roadmap.md#snd-2) |
| Graphics are picture-only: audio in a HyperFrames composition never plays | Code: `graphics::asset_for` makes a picture stream only | [SND-2](roadmap.md#snd-2) |

The research is clear that sound is **placement and art direction, not library breadth**
("keep it minimal and subtle, no heavy risers"). Start small.

## 8. Graphics

| Gap | Evidence | Fix |
|---|---|---|
| Few templates for the most common tutorial jobs: highlight real UI, a word-timed emphasis, a sequence of steps, before/after | Code: `src-tauri/graphics/templates/` | [GFX-3](roadmap.md#gfx-3) |
| No variants: a revision overwrites; no Compare / Use; a re-render takes 20-40 s with no draft preview | Code; log | [GFX-4](roadmap.md#gfx-4) |
| A graphic can't dim or blur the footage under it (a transparent HTML clip can't sample what is composited later); that is compositor work | Research | [FX-2](roadmap.md#fx-2) |
| GSAP is allowed in graphics (23 Oct); 3D and shaders are not | Code: the gate | Only when a style needs them ([decisions](decisions.md)) |

## 9. Footage and shorts

| Gap | Evidence | Fix |
|---|---|---|
| **No search over the source.** Every read is on the *edited* timeline; the agent can't ask "where do I open the export dialog?", including material already cut | Code: `mcp/tools.rs` | [FTG-1](roadmap.md#ftg-1) |
| No saved selects, no guided B-roll from own footage, no stock providers | Code | [FTG-2](roadmap.md#ftg-2), [FTG-6](roadmap.md#ftg-6) |
| A clip is one continuous stretch (`Short.source_start_us..source_end_us` or a copied `ShortEdit`): it can't combine a setup at minute 3 with the payoff at minute 17 | Code: `shorts.rs` | [SHT-1](roadmap.md#sht-1) |
| No ordered take assembly, no script-guided assembly; no one-step "run this preset now" after opening a project | Code | [FTG-4](roadmap.md#ftg-4), [AUTO-1](roadmap.md#auto-1) |

## 10. Finishing and delivery: absent from the app, not just the agent

| Gap | Evidence | Fix |
|---|---|---|
| No colour grade or LUT | Code | [FX-5](roadmap.md#fx-5), parked: not needed now |
| No NLE handoff (no `otio|fcpxml|aaf|edl`); no standalone alpha graphic export | Code | [DLV-3](roadmap.md#dlv-3) |
| Export is H.264/AAC only; fps from a closed set (10, 15, 24, 25, 30, 60) | Code: `export/mod.rs` | Fine for now; revisit with [DLV-3](roadmap.md#dlv-3) |
| No tutorial effects: spotlight, redaction/blur of a region (an email, a key), magnifier | Code | [FX-2](roadmap.md#fx-2) |
| No webcam background removal or blur (also blocks the cut-out layout) | Code | [FX-1](roadmap.md#fx-1) |
| No real reformat of a finished 16:9 edit into 9:16 that keeps UI targets and cursor relationships | Code | [DLV-5](roadmap.md#dlv-5) |
| One export job at a time, no delivery list (video + shorts + sidecars, which revision was exported, retry) | Code: `ExportOwner` | [DLV-2](roadmap.md#dlv-2) |
| No translated subtitles (Greek ↔ English) | Code | [DLV-4](roadmap.md#dlv-4) |

## 11. Languages and transcription

| Gap | Evidence | Fix |
|---|---|---|
| Greek is online-only (Parakeet is English only) | Code: `transcript/language.rs` | — |
| The filler/retake **rules** cut a deliberate repeat ("human in the loop. Human in the loop essentially means…"), and every review engine inherits that wrong cut; restarts, single-word repeats and cut-off words open no candidate window | Log: [local-review-eval](plans/local-review-eval.md) | [LNG-4](roadmap.md#lng-4) |
| The review evaluation has no real Greek recording (the Greek case is scripted) | Log | [LNG-4](roadmap.md#lng-4) |
| Key terms fix only what a term spells or (Parakeet) what two or more words sound like; a single sound-alike word ("codecs" for Codex) stays | Code: `transcript/vocabulary.rs` | — |

## 12. Performance

| Gap | Evidence | Fix |
|---|---|---|
| A layout edit rebuilds the playback evaluator *and* the audio mixer; fine for occasional edits, wrong for dragging a section edge or live layout keys | Log: `plans/layouts.md` | [LAY-1](roadmap.md#lay-1) |
| With the whole timeline in view, every clip is a separate element (8,000 clips: ~1.5 s per shifting edit in dev) | Log: O01 | [PERF-3](roadmap.md#perf-3) |
| The JPEG preview transport (non-wgpu fallback path) is near budget at 1080p (~30 ms per detailed frame) | Log: O02 | [PERF-3](roadmap.md#perf-3) |
| Recorder segment boundaries overlap by up to a frame; system audio has a 10 ms gap at each segment boundary | Log: O03, O04, **in the AeroShoot repo**, not here | Recorder backlog |

## 13. Verification debt

Built but never checked end to end; see [product.md](product.md#what-is-verified-and-what-is-not).
macOS for the MCP and graphics paths; layout performance on a real `.aero` with `perf.rs`.
(Codex write tools, the templatize → check → save loop and a style's logo in an export were
checked 25 Oct.) → [VER-1](roadmap.md#ver-1).
