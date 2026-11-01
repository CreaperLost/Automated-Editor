# Requirements for the final solution

30 Oct 2026. What AeroEdits' agent editing must do, derived from the research
([research/synthesis.md](research/synthesis.md), cited as **F1–F14**) and held against what is
built ([product.md](product.md)) and decided ([decisions.md](decisions.md)). The backlog stays
in [roadmap.md](roadmap.md); this file is the spec its items are built and checked against.

**Status:** **Have** = built and in [done.md](done.md) · **Partial** = some of it built ·
**Missing** = not built. Status was checked against the code where it says so (`check.rs`,
`mcp/`), otherwise against product.md and done.md.

---

## 1. The target

A user records a tutorial with AeroShoot, opens it, picks a preset, and their own agent edits it:
cuts from the transcript and telemetry, graphics that *show* what the speech describes in the
video's one style, sound placed on graphic events, all as editable project parts. The agent asks
once, plans per moment, proves the look on one sample clip, builds the rest, checks its own work
against the file, and stops at a state the user reviews by eye and ear. Corrections are precise;
the ones worth keeping become rules the user accepts. Success is measured as **cost per accepted
video** ([product.md](product.md#who-it-is-for)), long-form tutorials first, then tutorial shorts,
then gaming.

## 2. Fixed constraints

Settled decisions the solution must keep. They are not reopened here.

| Constraint | Source |
|---|---|
| AI only through the user's own agent over MCP / `aeroedits do`; no chat panel, no model on that path | decisions, 13 Oct |
| HyperFrames is the only renderer; GSAP vendored; no second renderer | decisions, 13 and 21 Oct |
| Built-in passes apply directly (one undo step); agent judgement calls are proposals; graphics apply as one undo step with no Accept card | decisions; GFX-8 decision 1 |
| Human in the loop: no autonomous fix-and-retry loops; never export without the user's go | Q-VERIFY |
| No anchored notes, notes lane or selection reference; feedback goes in chat; lasting context in a plan file | decisions, 24 Oct |
| No motion preview with sound for the agent; it sees stills and boards, the user listens | decisions, 24 Oct |
| Learning only through proposals the user accepts (a rule from a correction, AGT-3; patterns from data, LOOP-1); never free-text rules learned on the side | AGT-3, LOOP-1 |
| One look for every graphic in a video; references belong to the style; text cards and demonstrations are equals | decisions, 22 and 25 Oct |
| Music never needs credit; every sound carries its licence | Q-SOUND |
| Generated video and images only as user-supplied assets in a slot | decisions, Not doing |
| An edit reads about 100k tokens at most; a cheaper path stays only if the measured result holds | AI-1, cost rule |

---

## 3. The edit loop

The stages every working pipeline shares (synthesis §2), as AeroEdits runs them.

| # | Stage | What happens | Decides | Status |
|---|---|---|---|---|
| 0 | Method | The agent reads the preset's method and the user's rules, and the style's brief | — | **Partial:** method, rules and the style brief have; no style fills the brief yet (STY-2 step 2) |
| 1 | Intake | Open, scan, transcribe | App | **Have** |
| 2 | Brief | One round of questions with stated defaults; answers saved | User | **Partial:** passes are asked with numbers; graphics, style and sound are not |
| 3 | Cut | Mouth noises → silence → gaps → fillers → extra cuts | App; agent proposes extra cuts | **Have** |
| 4 | Plan | A per-moment table of graphics, footage and sound, kept in the project | Agent proposes, user replies | **Missing** (B2) |
| 5 | Sample | One representative graphic as a short clip in place | User approves frames | **Missing** (B3) |
| 6 | Build | Graphics from the style's kinds and examples; cues on graphic events | Agent | **Partial:** graphics have; style parts and cues missing |
| 7 | Check | Checks read from the project and the file | App | **Partial:** `check` has 16 kinds, none for graphics (F2) |
| 8 | Review | Boards and stills for the agent; joins and playback for the user; precise corrections | User | **Partial:** boards and joins have; correction form missing (E2) |
| 9 | Learn | A repeated correction becomes a proposed rule | User accepts | **Have** (AGT-3); patterns from data missing (A3) |
| 10 | Export | Only on the user's go; the user watches the file once | User | **Have** |

---

## 4. Requirements

Each: what it must do, why (finding), status, the item that builds it, and how we know it works.

### A. Method and learning

**A1. The method is a file the user owns and the agent reads first.** The preset's `method.md`
and `rules.md`, and the style's brief, are read before any work; the user's rules win.
*F2.* **Have** (ORG-1, AGT-3). *Accept:* an agent edit names the rule that shaped a choice.

**A2. A correction becomes a rule only through a proposal.** The agent proposes a rule with the
example that taught it; the rule says when it applies (V09's "when a number is mentioned, always
show a hero count-up"); the user accepts; autopilot never accepts rules. Look rules go to the
style, workflow rules to the preset. *F2, constraint.* **Have** for presets (`library` `op:
propose_rule`, `mcp/presets.rs`); **Missing** for style look rules (STY-2).

**A3. Learn from what the user changed, as data.** Accepted and rejected proposals, restored cuts
and redone graphics become proposed changes to presets, styles and templates. *F2.* **Missing**:
[LOOP-1](roadmap.md#loop-1). *Accept:* a pattern seen across two edits becomes one proposal.

### B. Brief, plan and sample

**B1. Ask once, with defaults, before graphics and sound.** One round, only what the user would
be upset to get wrong: which style (or make one, C2), how many graphics and where they are
welcome, the sound policy in words. Each question states the default used if skipped; answers
are saved as preset choices so "same as last time" asks nothing. *F4.* **Partial**: the preset
already asks per cleanup pass with real numbers; graphics are "only if the user wants them".
Item: [GFX-8](done.md#gfx-8). *Accept:* in EVAL-1, the agent asks at most one round before
graphics, and an auto run asks none.

**B2. A plan per moment, returned by the tool as a table.** For each moment: time (as word ids),
purpose, what the viewer *sees* (the visual, not only the words), kind, mode (full, split, over
face), sound cue, reason. `graphics` answers with the ready table; the agent keeps it in a plan
file in the project and re-reads it each turn; the user replies with specifics. Anchors are words
and recording time, never an old absolute time, so cuts re-resolve them. Behind each graphic row
sits the per-graphic contract (synthesis F13): teaching purpose, word ids for entry, emphasis,
hold and exit, approved copy and its source, real assets, target region, borrowed style traits,
fonts for English and Greek, separate 16:9 and 9:16 decisions, what a revision must preserve.
*F4, F5, F13.* **Have** (31 Oct): the plan file ([PLAN-1](done.md#plan-1-file)), its rows by
word id, placed again on every read, and the table from the tool (`graphic_add` `plan: true`,
[GFX-8](done.md#gfx-8-table)). **Missing**: the per-graphic contract behind each row. *Accept:* every planned graphic appears in the table before
any render, and a later cut moves its row.

**B3. Approve on frames: one sample before the set.** The agent renders the most typical or
hardest graphic, shows it as a short clip in place (face, captions, crop, audio, a little footage
either side), inspects it first, and the user approves the clip. Then the rest in the same look.
With an approved style or in auto mode, the agent checks the sample itself and continues. *F4.*
**Have** (31 Oct): `frame_render` `graphic` + `sample: true` and the guide's sample step; the
clip is a file path, not yet shown in the Agent panel. *Accept:* one extra render per new look,
and no graphic set is built on a look nobody has seen move.

### C. Graphics

**C1. A style holds everything a graphic needs.** Settings with fonts that resolve (or ship in the
style), motion rules and easing tokens, the kinds of graphics it uses and their 9:16 placement,
2–3 examples linked from the library, its references and the agent's breakdown, look rules
(including its "never" list), a richer shorts variant, a small licensed sound set. `library`
`kind: style` `op: show` answers with a compact brief. *F6, F11.* **Partial**: the parts, the
brief, the font rule and the check built 30 Oct (STY-2 step 1); no style has them filled in yet
(step 2), and the Style editor doesn't show them. [STY-2](roadmap.md#sty-2)
([plan](plans/styles.md)). *Accept:* STY-2's first test: an agent makes a
new long-form graphic in George's "my library" style that he would keep.

**C2. A style from references.** Intake of images, clips (read as boards, not video), a URL, a
logo or library graphics; a breakdown table first (what it takes, what it drops and why); up to
three proposals that differ in a named way, as stills on the video's own moments; the pick as one
clip in place; save with a check (fonts, examples, sounds). *F6.* **Partial**: `op: propose`
(intake and steps), previews and the saving check built 31 Oct; the pick as a clip in place waits
on GFX-8 §2; a URL comes in through the agent's own tools.

**C3. Pick per moment: demonstration or card, as equals.** Where speech describes an action or a
change, the graphic shows it (a timeline collapsing, a prompt typing, a before/after); where the
idea is a quote, number, label or punchline, a calm card. Before proposing, the agent asks "with
the text removed, does the animation still say something?". It never promises motion the graphic
won't contain. *F13, Ours 25 Oct.* **Have** as guidance (31 Oct), unmeasured. *Accept:* the 25 Oct short, redone, has at
least one graphic that passes the text-removed test.

**C4. Parts that demonstrate.** App and editor mock-ups, transformations (gaps collapsing,
before/after, messy → clean, a counter), diagram motion, word-timed emphasis, a hero count-up for
a spoken number; built as parts the agent composes, curated first from HyperFrames' registry and
the Herk kit before writing our own; all pass the gate with real English and Greek copy. *F12,
F13.* **Partial** (31 Oct): `timeline-cut`, `prompt-reply`, `count-up`, `before-after` and
`app-click`, and the diagram parts `steps-link` and `items-stack`, bundled; word-timed emphasis
and a messy → clean part not yet; Greek copy tried on `prompt-reply` only (GFX-8 §4,
[GFX-3](done.md#gfx-3)).

**C5. Author a new graphic in the academy's order.** Stills at final size with ids (checked with
`frame_render` at named times) → timing → motion; plain tween calls; the style's "never" list
applied. Hold every text for its reading time (about three words a second, counted from its first
legible frame). Name the cut (hard or eased), never leave it to default. *F11.* **Have** as guidance in
`mcp/skill/graphics.md` (30 Oct); the app already names the cut (`data-entrance`); the style's
"never" list waits on STY-2; no code checks it (F2).

**C6. Real material only.** Screens, product states and numbers come from the recording or the
user; nothing invented is presented as real; generated images enter as the user's assets in a
slot. *F6, F7.* **Have**, as a decision and in the graphics guide (30 Oct).

**C7. Revise one thing, keep the rest.** A revision changes the named graphic or element only,
with a preserve list; variants can be compared and one used as one undo step. *F5.* **Have** (1 Nov):
[GFX-4](done.md#gfx-4).

**C8. Keep graphics composable outside the app.** One graphic with alpha (ProRes 4444 / WebM),
then a cuts-only NLE handoff that reports what was flattened. *F9.* **Missing**:
[DLV-3](roadmap.md#dlv-3), later.

### D. Sound

**D1. The mix parts exist.** Clip gain and fades, music and effects roles, music held a measured
level under speech, ducking, loudness normalisation, licences on every sound, CC0 SFX and music
libraries open to the user's files. *F8.* **Have** (SND-1, SND-3).

**D2. Cues on graphic events.** Templates declare events (lands, completes, swaps); a cue
compiler places the sound so its audible peak hits the visual event
(`cueStart = visualPeak − soundPeakOffset`), with density limits and ducking; the agent picks which
events deserve a sound; the style's sound set says what it sounds like. *F8.* **Missing**:
[SND-2](roadmap.md#snd-2). *Accept:* on a board of the edit, every cue sits on its event frame;
the user hears no cue fighting speech.

**D3. Sound is directed in words.** The style carries a one-line sound policy ("minimal and
subtle, no heavy risers") and the agent follows it. *F8.* **Have** (31 Oct): `sound.policy`
and cues on a style, shown in its brief; My library has a set. Placing cues by itself is D2.

### E. Review and correction

**E1. The agent sees stills; the user hears.** Boards (`storyboard` fine/coarse), stills at
named times, the joins queue for the user's ears. *F10, constraint.* **Have** (COST-1, REV-1).

**E2. Corrections in four fields.** The agent turns any correction into **where** (time or
graphic name), **what is wrong**, **what it becomes** (target numbers, quoted words, "replace,
not add"), **what stays**, restates it in one line when it was vague, and changes only that. Fixes
in order: missing information and readability, then timing, then detail. Several precise fixes
in one reply are welcome. *F5.* **Have** as guidance in `mcp/skill/rules.md` (30 Oct); no code,
no UI (fits the "feedback in chat" decision); not yet measured. *Accept:* in an EVAL-1 revision round, no graphic other than the
named one changes.

**E3. Repair locally.** Only the changed graphic re-renders; the agent checks the frames at its
edges and the neighbouring cut. *F5.* **Have**: renders are per graphic and cached; the edge check is in
the graphics guide (30 Oct).

**E4. Small fixes by hand.** Trim, move, delete and volume stay direct on the timeline, one undo
each. *Academy.* **Have.**

### F. Verification

**F1. Checks come from the project and the file, not a model.** Duration, sync, loudness, black
frames, clipped words and the rest. *F10.* **Have**: `check`, 16 kinds (`check.rs`).

**F2. Graphics checks any agent gets.** Added to `check`: a font a graphic names that doesn't
resolve; an opaque area over the face track in a short; the caption band crossing a split line or
sitting under an opaque graphic; a graphic static for most of its time; text on screen for less
than its reading time; content outside the 9:16 safe area. *F10, F11, F13.* **Partial** (31
Oct): fonts in `check` (`graphic_font`); face covered, captions hidden or across the split, and
holding still in `frame_render` `graphic`'s answer; reading time in `check`
(`graphic_reading_time`, a hint); words or detail under the platforms' bars and buttons in a
9:16 short in `frame_render` `graphic`'s answer. *Accept:* each check fires on the 25 Oct take
as first built.

**F3. Promise against build.** Before reporting a graphic, the agent compares its own
description with its frames. *F10, Ours.* **Have** in the graphics guide (30 Oct).

**F4. Reports keep done, looked at, heard and approved apart.** *F10.* **Have** (the closing
summary).

### G. Cost and measurement

**G1. Measure cost per accepted video, by stage.** EVAL-1 after every cost or quality change;
tokens per edit against AI-1. *F14.* **Have** ([EVAL-1](plans/evaluation.md)).

**G2. Compare fairly when we compare.** Fixed brief and inputs, a control on the same machine
(no skill, or another editor), scoring from the delivered file, look judged separately by a
person and labelled, apart from brief compliance and execution; add lost or clipped speech,
graphics over UI, revision latency and spend. *F3, F10, F13.* **Missing** as a written method; [competitors.md](research/competitors.md#a-fair-comparison-when-we-run-one)
describes it.

**G3. Bounded agent work.** At most one self-check per graphic against its plan row; no
re-render loops; a long call can be stopped. *F10.* **Have** (Q-VERIFY; Stop on running calls in
the Agent panel).

### H. Tutorials and shorts

**H1. Pacing the agent can't override silently.** Graphics hold for reading time (C5); shots
carry their purpose; layout changes stay within limits (a few a minute, screen 2–8 s, camera
2–5 s, punch-in 1.5–4 s). *F11, F13.* **Partial**: layouts pick by rules; limits are
[LAY-3](roadmap.md#lay-3).

**H2. 9:16 is designed for, not resized.** Face, captions and splits checked (F2); graphics pick
their mode before the first render (have, SHT-4); a real reformat that keeps UI targets is
[DLV-5](roadmap.md#dlv-5). *F13.* **Partial.**

**H3. Outside this repo.** Web UI meant for 9:16 is best recorded in a mobile browser (V09,
synthesis F13); that is AeroShoot's job, noted for its backlog, not the editor's.

---

## 5. Build order

Matches the roadmap; the two text-only items are new and cheap enough to go first.

| Order | Work | Requirements |
|---|---|---|
| 1 | ~~**Skill text**~~ done 30 Oct ([done.md](done.md#skill-text)): four-field corrections, the authoring order and reading time, real material only, promise vs build, edge check | E2, C5, C6, F3, E3 |
| 2 | [STY-2](roadmap.md#sty-2) styles from references | C1, C2, D3, A2 (style rules) |
| 3 | [GFX-8](done.md#gfx-8) graphics an agent makes well | B1, B3, C3, C4, F2, plan table of B2 |
| 4 | [PLAN-1](done.md#plan-1) plan file and style trial (built 31 Oct) | B2 |
| 5 | [SND-2](roadmap.md#snd-2) cues at graphic events | D2 |
| 6 | [SHT-1](done.md#sht-1) (built 31 Oct), [LAY-3](roadmap.md#lay-3), [GFX-4](done.md#gfx-4), [LOOP-1](roadmap.md#loop-1), [DLV-3](roadmap.md#dlv-3), [DLV-5](roadmap.md#dlv-5) | H1, H2, C7, A3, C8 |

---

## 6. Research suggestions we are not taking

Proposed in the research write-ups (mostly the Gemini set); each conflicts with a decision or
isn't supported by evidence.

| Suggestion | Why not |
|---|---|
| Timeline comment markers the agent reads (`read_timeline_markers`), click-to-point chips | Anchored notes dropped 24 Oct; E2 gets the precision in chat |
| An "unreviewed" tint and an Approve fence on every agent change | Built-in passes apply directly; graphics have no Accept card; proposals only for judgement cuts |
| "Calibrate skill from recent edits": the agent rewrites its own method freely | Rules only as accepted proposals (A2); patterns from data (A3) |
| A Rust renderer, Remotion, or renderer portability | HyperFrames only; our graphics render in 20–40 s each, not the 23.5 min of a whole-film test |
| After Effects, Premiere or Blender driven by the agent | Decided against; a typed API owns the edit |
| Local TTS, avatars, dubbing | Re-voice paused; out of scope for tutorials |
| An inspiration hub on motionvideo's API; a marketplace | No demonstrated need |
| An 8-file harness per project (`BRIEF.md`, `STYLE.md`, `STORYBOARD.md`…) | One plan file (B2), the style (C1) and the preset carry the same content |
| Enforcing the four layers as separate agent turns | Guidance (C5), not a gate: four renders per graphic would break the cost rule |
| A fixed 3–5 s hold for all text | Hold by reading time (C5) |
| Borrowed style bans as defaults | Only as a user's own style rules |
| Mobile-viewport emulation in the editor | The editor doesn't capture; see H3 |

---

## 7. Open questions

None. The skill text (row 1 of §5) shipped before STY-2 at George's yes (30 Oct); its effect
shows in the next EVAL-1 revision round (E2's accept test).
