# Roadmap: one order, reconciled

Six documents each propose their own order of work. They mostly agree; where they don't, it's
called out below. **This is a proposal for discussion**, not a decision.

## What each doc says to do first

| Doc | First | Then | Later |
|---|---|---|---|
| [gaps](../agent-video-gaps-and-proposals-2026-10-21.md#recommended-order) | Sound design, style kit, templates + captions, agent CRUD | Plan artefact, motion review, animation ceiling | Footage search, reformat, timing, render settings, project binding, tool count |
| [chatcut](../chatcut-competitive-review-2026-10-21.md#implementation-order) | Style trial | Selection references, curated graphics + supplied-file B-roll | Faster revisions + checkpoints, NLE handoff |
| [short-form 1](../short-form-reference-review-2026-10-21.md#recommended-order) | Baseline + one style | Beat plan vertical slice (captions, MCP, template pack) | Event-driven sound, then broaden |
| [short-form 2](../short-form-reference-review-part-2-2026-10-21.md#updated-priorities) | One style + acceptance criteria | Beat plan, graphics render policy | Sound, captions, matting/3D |
| [herk](../herk-edit-kit-research-2026-10-21.md#adoption-order-and-evaluation) | Short-form recipe + beat plan + 3–5 designs | Cues, typography, plan validation, output checks | Render policy, generation |
| [resolve](../davinci-resolve-research-2026-10-21.md#suggested-order-and-acceptance-checks) | Source search | Recovery + diagnostics, speech matching | Script assembly, delivery recipes |
| [backlog (Premiere notes)](../future-work.md#premiere-pro-mcp-servers) | **Project binding before any new mutation tool** | Source search, selects | Descriptions, B-roll, handoff |
| [second opinion](../second-opinion-video-research-2026-10-21.md#e-second-opinion-on-the-ranked-gaps) | Revision operations (CRUD) + project-bound proposals | Plans + playable motion/audio previews; captions + real reformat | Style + asset kit, *minimal* sound, targeted templates, then runtime |
| [astra-premiere](../astra-premiere-skill-review-2026-10-21.md#suggested-delivery-sequence) | Thorough template option + honest review summary | Review records + join audition | Word-emphasis graphic, camera push-ins, ordered take assembly |
| [ten videos](../ten-ai-editor-videos-2026-10-21.md#what-this-means-in-product-terms-ordered-by-leverage) | Project rules / master prompt, self-improving skill | Music and SFX tracks, a published skill bundle | Colour via MCP, storyboard with music, folder watch |

**Where they agree:** a style + a plan come before more graphics; review must start to move;
project binding comes before more write tools; generation and AE come last.

**Where they disagree:**
- *Sound first* (gaps) vs *sound after the plan* (short-form 1/2, herk). The second opinion
  resolves most of this: build a **minimal** sound surface early (imported files, fades, gain,
  ducking, preview, undo) and leave the event-driven cue compiler for later. → Q3.
- *Self-improving rules first* (ten videos) vs *safety and review first* (second opinion,
  astra). Both are cheap; rules need an accept gate and a home outside SKILL.md. → Q11.

## A reconciled order

Each phase ends with something usable and measurable. Theme numbers refer to [themes.md](themes.md).

### Phase 0 — Clear the ground (small, unblocks everything)

- Project/session binding on agent mutations (T9, backlog #17).
- Agent CRUD: `graphic_remove`, `graphic_move`, `short_remove`, `library_remove`, `brand_kit_set` (T11).
- State the graphics animation ceiling in the skill; decide whether to vendor GSAP (T3).
- Fix the contradictions in agent-facing text ([open-questions.md](open-questions.md) C1–C3)
  and the factual errors in the research docs (C8–C10).
- **Thorough mode** in the tutorial template and a summary that separates executed / inspected /
  listened / approved — uses existing tools only ([astra P0](../astra-premiere-skill-review-2026-10-21.md#p0-thorough-editorial-workflow-using-existing-tools--small)).
- Build the **evaluation set** every doc asks for: a fixed English tutorial, a Greek clip, a
  30–60 s talking-head short; record time, corrections and **cost per accepted video**.

### Phase 1 — Style + plan, one vertical slice

- Style kit v1: rules (some checkable by `graphic_check`, not all), files and references,
  long-form and shorts profiles (T1).
- Project rules the agent suggests and the user accepts, stored outside SKILL.md (T10).
- Beat plan v1 stored in the project, transcript-row panel, `plan_get/set` (T2).
- 3–5 templates for **observed tutorial jobs** (highlight UI, key command, sequence, before/after),
  including a lower-half overlay prototype and the word-timed emphasis graphic (T3).
- Style trial on a 5–10 s range (T2).
- **Measure** against the Phase 0 baseline before going further.

### Phase 2 — Make it watchable (and hearable)

- Draft/final graphics render policy (fps, size) (T7).
- `motion_preview` with sound for a range or a beat, plus review records and a join audition queue (T7).
- Caption typography: font choice, emphasis words, a few animation presets (T5).
- **Minimal sound surface**: imported licensed SFX and music on their own tracks, trim, fades,
  gain, ducking, licence record (T6).

### Phase 3 — Event-driven sound

- Event markers in templates, procedural palette, cue compiler, density policy from the style kit (T6).

### Phase 4 — Revisions and recovery

- Graphic variants with source snapshots, Compare / Use (T4).
- Persistent history and named checkpoints (T9, backlog #13).

### Phase 5 — Footage

- Source-transcript search → selects → guided B-roll → script-guided assembly (T8).
- Outside transcript import, ordered take assembly, optional folder watch (T8).
- Speech matching across takes (T6, resolve).

### Phase 6 — Learn and deliver

- "Save this workflow" with examples and scopes (T10).
- Real reformat 16:9 → 9:16 keeping UI targets; delivery presets for safe zones (T12).
- Delivery queue, SRT/VTT, translated subtitles, native tutorial effects, camera push-ins,
  colour/LUT actions, NLE handoff (T12).

## What's already in flight (from the backlog)

Not re-ordered here; they continue on their own track: layouts follow-ups (backlog #1, look-only
playback swap), number-key layout switching (#10), auto layouts from the skill (#11), more
languages and local Greek (#2, #12), clip thumbnails (#7), simpler UI (#8), the performance
open items O01–O04 and the three known issues in [future-work.md](../future-work.md#known-issues).
