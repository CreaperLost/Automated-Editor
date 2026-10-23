# Gaps and improvements

Everything the research and our own build logs found missing, merged, de-duplicated and
checked against the code on 21 Oct 2026. Each gap says what is wrong, the evidence, and the
roadmap item that fixes it ([roadmap.md](roadmap.md)). What the product does today is in
[product.md](product.md).

**Evidence grades.** **Code:** checked in `src-tauri/src` or `front-end/src` on 21 Oct.
**Log:** from our own build or measurement logs. **Research:** from competitor or creator
material ([research/](research/README.md)); creator claims are not measurements.

## The short version

1. **Correctness debt: fixed 21 Oct.** The render-queue bug, the self-contradicting skill
   paragraph, the wrong MCP safety hints, the schema-less `propose_edit` and agent calls that
   didn't know which project they belonged to are all fixed (see section 1). What remains is the
   Tauri `template_save` name (UI-3) and the direct actions' project check moving under the lock
   (AGT-1).
2. **The agent can now clean up and correct (fixed 21 Oct, AGT-1).** It can move and remove
   graphics, author zooms, fix mis-heard words, write chapters, edit and remove shorts, set media
   roles and cancel exports. What stays UI-only is mostly playback (see REV-1).
3. **Nothing survives a reopen.** Undo is in memory (64 steps), graphic source edits have no
   undo at all, and there are no checkpoints. Every creator in the research duplicates the
   project before letting an agent touch it; we give them nothing to do that with.
4. **The agent is blind to time.** Every review tool is a still; tools return text and PNG
   only. Pacing, easing, joins and the mix are invisible to the agent and unrecorded for the user.
5. **There is nowhere to put intent.** No comments, no notes, no plan, no user-owned rules. The
   strongest pattern in the research (feedback anchored to a time range; a method file the agent
   improves) has no home in our data model.
6. **The craft surfaces are thin:** one caption font, no sound placement, graphics locked to
   30 fps / 1920 px, no search over the source footage.
7. **Some things are absent from the app itself,** not just from the agent: colour grading,
   subtitle files, NLE handoff, tutorial effects (spotlight, redaction).

---

## 1. Bugs and correctness debt

| Gap | Evidence | Fix |
|---|---|---|
| **A newer graphic render drops a waiting one for another length.** The queue replaces a waiting job when asset and canvas match, but the match key (`Spec::prefix`) is only `WxH`, so the same graphic at two lengths (main edit and a short) can leave one render never queued | Code: `graphics/render.rs` (`retain` on `asset_id` + `prefix()`; `prefix()` = `"{w}x{h}-"`) | Fixed 21 Oct: [FIX-1](roadmap.md#fix-1) |
| **The skill contradicts itself on telemetry polish.** It says the touches are "on by default … leave them on", then "Both are off by default". Truth: `webcamAutoHide` on, `windowCrop` on when window telemetry exists, `soundEffects` off | Code: `mcp/skill/SKILL.md` § Telemetry polish | Fixed 21 Oct: [FIX-2](roadmap.md#fix-2) |
| **MCP safety hints are wrong.** `export`, `graphic_render` and `graphic_check` are flagged `read_only: true` but write files; `destructiveHint` is hard-coded `false` for every tool, including `export` (overwrites) and `library_save` | Code: `mcp/actions.rs`, `mcp/mod.rs` (`.destructive(false)`) | Fixed 21 Oct: [FIX-3](roadmap.md#fix-3) |
| **`propose_edit` has no schema.** `edits: Vec<Value>`; the description lists 13 of the 17 `SequenceEdit` kinds (`SetMagnetic`, `SetSpeed`, `RippleTrim`, `RestoreCuts` work but are undocumented) | Code: `mcp/propose.rs`, `sequence/edit.rs` | Fixed 21 Oct: [FIX-3](roadmap.md#fix-3) |
| **The app froze when a short played in the window preview** (Windows closed it as not responding, `AppHangB1`, three times on 18 Oct). A deadlock: the playback engine held the `preview` lock through an `if let` while drawing, drawing asked Tauri for the window's handle (answered by the window thread), and the window thread waited for that lock in the synchronous `preview_layout` command. Only the short's draw paths held it | Reproduced 22 Oct in the dev app (four clips, switched every 4 s while playing: frozen within seconds, CPU flat); stacks with `cdb` | Fixed 22 Oct: the lock is let go before drawing, and the presenter keeps the window's handle. Same test after, three runs: 45 switches, no freeze, slowest window-thread reply 72 ms. Rule in [architecture](architecture.md#threads-and-locks-the-window-thread-must-never-wait-on-us) |
| **A queued short-layout change could land on another short or project.** It read the project when it ran, not when it was asked for | Code: `stores/shortsStore.ts` `shortOp` | Fixed 22 Oct: it keeps the project and short it was asked for, and is dropped when another shows ([Done](roadmap.md#done)) |
| **A late short view replaced the video's timeline** after leaving Shorts, while edits went to the video | Code: `stores/projectStore.ts` `applyOpenedProject` | Fixed 22 Oct: a view of a short not on show is ignored |
| **Saving a brand kit deleted the old folder before the new one was in place** (errors ignored), so a failed swap lost the kit; a non-kit folder with the new name was deleted too | Code: `graphics/brands.rs` `save` | Fixed 22 Oct: the old folder is set aside and put back on failure; a folder in the way is refused |
| **Renaming a brand kit broke every graphic and project using it** (they store the name) | Code: `graphics/brands.rs` | Fixed 22 Oct: `renamed.json` in the kits folder leads old names to the kit, in every project; views show the name now |
| **`graphic_set` with a move and a rename moved, then failed** on the stale revision; `transcript_fix` saved earlier words before a bad one failed | Code: `mcp/actions.rs` | Fixed 22 Oct: checked before anything changes, each step on the latest revision, a late failure says what went in; word fixes go in together or not at all |
| **Slider changes were lost** when the Zoom panel or Layouts section closed within the save delay | Code: `ZoomPanel.tsx`, `LayoutsSection.tsx` | Fixed 22 Oct: both use `useDebouncedSave`, which saves on close |
| **A short's captions stayed stale after a word fix** when only the short had captions; the short-frame cache ignored transcript saves | Code: `commands/transcript.rs`, `commands/mod.rs` | Fixed 22 Oct |
| **Sound effects were silent in the preview** of a project with no audible audio track, though export had them | Code: `playback/mod.rs` `has_sound` | Fixed 22 Oct |
| **A 10 fps export could fail its own duration check** (a whole last frame runs up to 100 ms past the edit; the slack was 80 ms) | Code: `export/mod.rs` | Fixed 22 Oct: up to one frame is allowed |
| **A video with 16+ audio streams failed to import** after extracting them all (picture + 16 > `MAX_STREAMS`) | Code: `media_bin.rs` | Fixed 22 Oct: only the sounds that fit beside the picture |
| **`graphic_write_file` followed links** (symlinks, junctions) out of the graphic's folder | Code: `graphics/mod.rs` `write_file` | Fixed 22 Oct: a link anywhere in the path is refused, like the project's `safe_path` |
| **Two different things are called `template_save`.** The MCP tool saves skill *answers*; the Tauri command saves a *look* | Code: `mcp/skill.rs`, `templates.rs` | Open: moved to [UI-3](roadmap.md#ui-3) (rename the whole saved-look command family) |

## 2. Safety and recovery

| Gap | Evidence | Fix |
|---|---|---|
| **Agent proposals were not bound to a project.** They carried `expectedRevision` but the MCP helper called `with_proposals(state, None, …)`, i.e. whichever project was open | Code: `mcp/propose.rs` | Fixed 21 Oct: [SAFE-1](roadmap.md#safe-1). Left: direct actions check the `project` id just before running rather than under the command lock ([AGT-1](roadmap.md#agt-1)) |
| **Undo dies on reopen.** `EditHistory` is in memory, capped at `MAX_UNDO = 64`; only the current document is saved. "Undo what the agent did yesterday" has no substrate | Code: `project/revision.rs` | [SAFE-2](roadmap.md#safe-2) |
| **Graphic source edits have no undo.** `graphic_set` (variables, brand) and `graphic_write_file` write source directly; library "versions" don't keep old files | Code: `commands/graphics.rs`, `graphics/library.rs` | [SAFE-2](roadmap.md#safe-2), then [GFX-4](roadmap.md#gfx-4) |
| **No checkpoints.** No "clean cut" / "approved look" snapshot covering the edit, graphic source and caption marks (transcripts sit outside the edit's undo) | Research: ChatCut, every Resolve creator duplicates the project first | [SAFE-2](roadmap.md#safe-2) |

## 3. Agent surface

### Missing operations (the agent can create but not finish)

The human UI has 137 Tauri commands; 42 have no agent path. The ones that block real sessions:

| The agent cannot… | Human path today | Fix |
|---|---|---|
| ~~Remove or move a graphic; remove a short; remove a library item; write a brand kit~~ | Graphics panel, Shorts list, BrandKits | Fixed 21 Oct ([AGT-1](roadmap.md#agt-1)) |
| ~~Add, edit or delete a zoom~~ | ZoomPanel, zoom lane | Fixed 21 Oct ([AGT-1](roadmap.md#agt-1)) |
| ~~Fix a mis-transcribed word~~ (`transcript_fix`; not an undo step, as in the UI) | Transcript panel | Fixed 21 Oct ([AGT-1](roadmap.md#agt-1)) |
| ~~Edit chapters, edit a short's range, set media roles, cancel its own export~~ | Chapters panel, Shorts, Inspector, export dialog | Fixed 21 Oct ([AGT-1](roadmap.md#agt-1)) |
| Move the user's playhead to show them a moment (`playback_*` is UI-only) | Preview bar | [REV-1](roadmap.md#rev-1) |

And the other way round: `scan` and `telemetry_get`, the two best "read the recording" surfaces,
have no UI; `fillers_review`, `graphic_templatize` and `graphic_check` can't be run or audited
from the UI. Low priority, but worth knowing when a user asks "what did the agent see?".

### Surface hygiene

| Gap | Evidence | Fix |
|---|---|---|
| 55 tools' schemas are sent on every connection: 72,722 characters after the 23 Oct trim; a further cut would mean shorter descriptions or on-demand schemas | Code: `mcp/mod.rs` `lean` | [AGT-4](roadmap.md#agt-4) |
| No readiness report without a project (versions, runtimes, why something is unavailable); "not connected", "no project", "runtime missing" and "failed" look alike | Log; research: Resolve, Premiere MCPs | [AGT-4](roadmap.md#agt-4) |
| No long-job contract: progress exists, cancellation and resumable analysis don't | Code: actions dispatch blocking work with a progress callback | [AGT-4](roadmap.md#agt-4) |
| No per-run report of time and cost (0 matches for `cost|usd|token_usage` in `src/ai`) | Code | [EVAL-1](roadmap.md#eval-1) |
| `proposals_list` reports a rejection but not the user's reason | Code | [NOTE-1](roadmap.md#note-1) |
| API keys are out of the agent's reach by design, so in a fresh setup `transcribe`, `fillers`, `chapters` and `shorts` depend on state the agent can't fix. Keep it that way; `status`/readiness must say so plainly | Code | [AGT-4](roadmap.md#agt-4) |

## 4. Review: the agent can't see motion or hear

| Gap | Evidence | Fix |
|---|---|---|
| Every review tool is a still (`frame_render`, `timeline_view`, `storyboard`); MCP output is text and PNG only. Easing, pacing, overlaps, joins and the mix are invisible | Code: `mcp/tools.rs` | [REV-1](roadmap.md#rev-1) |
| `check` reads the edit without decoding media: a clean report can't certify clean audio or footage (frozen or black frames inside clips, measured loudness) | Code: `check.rs` | [REV-1](roadmap.md#rev-1) (encoded-output pass) |
| Reports blur what was *executed*, *inspected*, *listened to* and *approved*; nothing records which joins anyone actually heard, and edits don't invalidate that | Research: Astra Premiere skill | Report: fixed in the skill 21 Oct ([AGT-2](roadmap.md#agt-2)). Records that survive edits: [REV-1](roadmap.md#rev-1) |
| Speed over coverage: the filler review reads only candidate windows and the skill discouraged reading transcripts, so an unflagged tangent or abandoned sentence may never be considered | Code: `ai/candidates.rs`, `SKILL.md`; Log: local-review eval missed restarts outside windows | Thorough mode added to the skill 21 Oct ([AGT-2](roadmap.md#agt-2)); not yet measured on a real recording |

## 5. Intent: notes, plans and the user's method

| Gap | Evidence | Fix |
|---|---|---|
| **No comments or notes anchored to the edit.** Zero matches for `comment|annotation|marker`. The user can't say "that zoom at 2:14 is wrong" except in chat; the agent can't leave "unsure about this cut" for the user | Code; research: Tandem (comment on the timeline), Tao (timestamped feedback, a drawn region beat words) | [NOTE-1](roadmap.md#note-1) |
| **No selection reference.** The user can't hand the agent "this graphic / these words / this rectangle"; they describe timestamps in chat | Research: ChatCut selection mode | [NOTE-1](roadmap.md#note-1) |
| **No plan before the build.** `storyboard` reviews the edit that exists, not the one to come; graphics are proposed as a chat list; `graphic_add(atWord)` resolves the word once and forgets it | Code: `mcp/actions.rs` `graphic_add` | [PLAN-1](roadmap.md#plan-1) |
| **The user's method can't be saved.** Saved answers remember settings for 3 fixed templates. `SKILL.md` is compiled into the binary and reinstalled, so the user can't own it and the agent can't improve it. Every strong creator workflow (a 1,400-line skill the agent rewrites, a published prompt doc, a skill zip) keeps the method in a user-owned file | Code: `mcp/skill.rs` (`include_str!`); research | [AGT-3](roadmap.md#agt-3) |
| **Style is only colours, a font and a logo.** No caption rules, motion character, density, sound policy or references | Code: `graphics/brands.rs` | Visual look: [GFX-5](roadmap.md#gfx-5); written rules: [STY-1](roadmap.md#sty-1) after [AGT-3](roadmap.md#agt-3) |

## 6. Captions

| Gap | Evidence | Fix |
|---|---|---|
| ~~The brand kit's font doesn't reach captions~~ | **Fixed** 23 Oct: kit font file (.ttf/.otf) or the installed family, else Inter Display; .woff/.woff2 kit files still fall back | [CAP-1](roadmap.md#cap-1) |
| ~~No per-word emphasis, no animation presets~~ (**fixed** 23 Oct); caption ownership by stretch **fixed** 23 Oct: captions give way to full-frame graphics | Code; research: every short-form reference | [CAP-1](roadmap.md#cap-1) |
| Greek glyph coverage in graphics never tested (captions: tested 23 Oct, every bundled weight) | Log | [GFX-3](roadmap.md#gfx-3) |
| No subtitle files (SRT/VTT) or transcript export; burned-in captions are the only output | Code: 0 matches for `srt|webvtt` | [DLV-1](roadmap.md#dlv-1) |

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
| Graphics render at **30 fps and 1920 px long side** whatever the export: a 4K/60 export gets no extra detail or motion samples | Code: `graphics/render.rs` (`RENDER_FPS`, `RENDER_LONG_SIDE`) | Fixed 22 Oct: exports render final graphics at their fps, 2x density above 1080p ([Done](roadmap.md#done)) |
| Six full-screen 4–6 s cards. No template for the most common tutorial jobs: highlight real UI, a word-timed emphasis, a sequence of steps, before/after | Code: `src-tauri/graphics/templates/` | [GFX-3](roadmap.md#gfx-3) |
| No variants: a revision overwrites; no Compare / Use; ~20 s re-render with no draft preview | Code; log | [GFX-4](roadmap.md#gfx-4) |
| **A video's graphics don't share a look.** Each template hard-codes its sizes, corners, spacing and motion; the kit's three colours and font are only used where a graphic chooses to, and nothing checks that. Changing the style of every graphic means editing each one | Code: `graphics/brands.rs` (`variables()`: background, text, accent), `src-tauri/graphics/templates/`; George, 22 Oct | Fixed 22 Oct: one look per video, every template and render takes it ([Done](roadmap.md#done)) |
| A graphic can't dim or blur the footage under it (a transparent HTML clip can't sample what is composited later); that is compositor work | Research | [FX-2](roadmap.md#fx-2) |

## 9. Footage and shorts

| Gap | Evidence | Fix |
|---|---|---|
| **No search over the source.** Every read is on the *edited* timeline; the agent can't ask "where do I open the export dialog?", including material already cut | Code: `mcp/tools.rs` | [FTG-1](roadmap.md#ftg-1) |
| No saved selects, no guided B-roll from own footage, no stock providers | Code | [FTG-2](roadmap.md#ftg-2), [FTG-6](roadmap.md#ftg-6) |
| A short is still one continuous stretch (`Short.source_start_us..source_end_us` or a copied `ShortEdit`): it can't combine a setup at minute 3 with the payoff at minute 17 | Code: `shorts.rs` | [SHT-1](roadmap.md#sht-1) |
| Caption text, breaks and emphasis can't be shaped inside a short | Code | [SHT-2](roadmap.md#sht-2) |
| Take shorts: no speed/freeze fit modes, no per-short ducking amount | Log: `plans/shorts.md` | [SHT-4](roadmap.md#sht-4) |
| No outside-transcript import, no ordered take assembly, no script-guided assembly; no one-step "run this template now" after opening a project (automatic folder watch is turned down) | Code | [FTG-4](roadmap.md#ftg-4), [FTG-5](roadmap.md#ftg-5), [AUTO-1](roadmap.md#auto-1) |

## 10. Finishing and delivery: absent from the app, not just the agent

| Gap | Evidence | Fix |
|---|---|---|
| No colour grade or LUT (0 matches for `lut|grade|temperature`) | Code | [FX-5](roadmap.md#fx-5), parked: not needed now |
| No NLE handoff (0 matches for `otio|fcpxml|aaf|edl`); no standalone alpha graphic export | Code | [DLV-3](roadmap.md#dlv-3) |
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
| The filler/retake **rules** cut a deliberate repeat ("human in the loop. Human in the loop essentially means…") — every review engine inherits that wrong cut (parallel statements and lists that open alike are kept since 22 Oct 2026); restarts, single-word repeats and cut-off words open no candidate window | Log: [local-review-eval](plans/local-review-eval.md) | [LNG-4](roadmap.md#lng-4) |
| The review evaluation has no real Greek recording (the Greek case is scripted) | Log | [LNG-4](roadmap.md#lng-4) |
| No custom vocabulary (product names, jargon) | Code | [LNG-5](roadmap.md#lng-5) |

## 12. Performance (open items)

| Gap | Evidence | Fix |
|---|---|---|
| A layout edit rebuilds the playback evaluator *and* the audio mixer; fine for occasional edits, wrong for dragging a section edge or live layout keys | Log: `plans/layouts.md` | [LAY-1](roadmap.md#lay-1) |
| With the whole timeline in view, every clip is a separate element (8,000 clips: ~1.5 s per shifting edit in dev) | Log: O01 | [PERF-3](roadmap.md#perf-3) |
| The JPEG preview transport (non-wgpu fallback path) is near budget at 1080p (~30 ms per detailed frame) | Log: O02 | [PERF-3](roadmap.md#perf-3) |
| ~~A graphic waiting for its render listed its cache folder on every frame; sound-effect markers were all drawn whatever the view; `transcript_fix` loaded and saved the whole transcript per word; a small preview of a big image decoded it at full size first~~ | Code | Fixed 22 Oct: fallback found once per render generation, markers only in the drawn time, one save per batch, decoded straight to the preview size with size limits and reused for smaller sizes |
| Recorder segment boundaries overlap by up to a frame; system audio has a 10 ms gap at each segment boundary | Log: O03, O04 — **in the AeroShoot repo**, not here | Recorder backlog |

## 13. Verification debt

Built but never checked end to end; see [product.md](product.md#what-is-verified-and-what-is-not).
macOS for the MCP and graphics paths; a real Codex session using write tools; the agent's
templatize → check → save loop over MCP; a brand kit's font and logo in a real render; layout
performance on a real `.aero` with `perf.rs`. → [VER-1](roadmap.md#ver-1).

## 14. Contradictions found while consolidating

| # | Contradiction | Status |
|---|---|---|
| C1 | `SKILL.md` telemetry polish on/off | **Fixed** 21 Oct ([FIX-2](roadmap.md#fix-2)) |
| C2 | "No keystroke capture" vs keyboard telemetry v1 | **Fixed** in [ai-readiness.md](plans/ai-readiness.md#decisions-2026-10-13): no typed text is ever captured |
| C3 | Graphics v1 says a placement stores a word anchor; the code resolves the word to a time once | **Fixed** in the plan text; storing the anchor is part of [PLAN-1](roadmap.md#plan-1) |
| C4 | "A brand kit feeds every caption" | **Fixed** 23 Oct: font and accent ([CAP-1](roadmap.md#cap-1)) |
| C5 | Research said our HyperFrames skills cover "3D, shaders"; our gate and authoring rules allowed only WAAPI/CSS | GSAP allowed 23 Oct ([GFX-2](roadmap.md#gfx-2)); 3D and shaders still not |
| C6 | Phase 2 handoff still said "nothing is committed" | Removed (historical doc deleted) |
| C7 | Backlog said the cut-out layout needs task 10; it needs the matte | Fixed: [LAY-4](roadmap.md#lay-4) depends on [FX-1](roadmap.md#fx-1) |
| C8–C10 | Factual errors in the video research (Tao's 49 min is a first draft, 3–4 h total; "106 SFX", "12 hours / 4.5B tokens" not in the captions; two taxonomies conflated; tools named that don't exist; Convex is really Mike Cann's Tandem) | Fixed: [research/creator-workflows.md](research/creator-workflows.md) uses only the corrected facts |
| C11 | A borrowed "banned list" (no holds over 1 s, no blur-ins) conflicts with tutorials, where viewers need time to read UI | Settled: don't ship borrowed bans as defaults ([decisions.md](decisions.md)) |
