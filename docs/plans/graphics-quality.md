# Graphics an agent makes well (GFX-8)

**Status:** notes and direction, 25 Oct 2026. Nothing built yet. Roadmap:
[GFX-8](../roadmap.md#gfx-8), with the bugs from the same test as
[FIX-8 to FIX-10 and UI-8](../roadmap.md#fix-8). **Comes after
[STY-2, styles from references](styles.md)** (George, 25 Oct): the style carries the look,
motion, kinds of graphics and examples this plan builds on.

Graphics are the part we have worked on least as a *craft*. We built the plumbing (one Graphic
asset, slots, the render queue, the library, one look, shorts modes), but not what makes an
agent produce graphics worth keeping. On 25 Oct George edited the same short take with three
agents. All three produced generic text cards. This file keeps what they said, what George saw,
and what we change.

## The test, 25 Oct

Same take ("AI can edit your videos", ~23 s, 9:16), same AeroEdits build, three agents.

| Agent | Cuts | Graphics | What George saw |
|---|---|---|---|
| Claude Code, Opus 5.5 | Retake at ~0:18 found but the cut landed on the 16:9 video (bug, undone twice) | 4 cards from stock templates (title, recap, key-point), a "Fancy-Short" style with energy 1.9 | Dark purple box filling the lower half, caption cut at the split line, serif fallback font, graphics plan not shown until asked again, then an Accept card |
| Codex | Stopped: Codex's own approval reviewer blocked online transcription ("sharing your audio with those providers wasn't explicitly authorized") | — | Autopilot broke at step one, again |
| Astra | Retake removed correctly | 4 cards (title, step, key-point), shown as a clear table first | Same boring white cards: "AI can edit your videos" box over the chin, a split with two plain buttons "Codex / Claude Code" |

The plan table Astra gave (the good part, keep this format):

| Time | Graphic | Mode |
|---|---|---|
| 0–1.9s | "AI can edit your videos" — title | Overlay |
| 4.2–6.1s | "Open Codex or Claude Code" — step | Split |
| 14–16.3s | "Cut silences. Add motion graphics." — key point | Split |
| 20–22.8s | "This video was edited by AI" — key point | Overlay |

## What the agents said went wrong

Both agents, asked afterwards, landed on the same causes. In their words, condensed:

**Opus 5.5**
1. Reached for the stock templates and never went back on that. They are made for calm tutorial
   cards. When George asked for fancy colours and lots of motion, it changed the style instead
   of dropping the templates. Our guidance says "templates first, write your own only when
   nothing does the job", and this was exactly that case, but it didn't see it.
2. Oversold the style. Energy 1.2 → 1.9 only makes the templates' existing motion travel further
   and overshoot more. It doesn't add motion. It promised "lots of motion" without checking what
   the number changes.
3. Never showed anything before committing: designed the style, saved it, applied it and
   proposed all four graphics without rendering one sample.
4. Didn't check fonts. Bahnschrift, then Inter Display; neither installed, so titles fell back
   to a serif.
5. Didn't think through the 9:16 frame: the opaque hook card covers the lower face; the two
   splits give half the frame to a mostly static box; captions are cut where the split begins.

**Astra**
- Nothing technical restricted it. It read "start with the templates" too literally and added
  small decorations instead of designing visuals that *show* the message.
- What it should have built for this short:
  - Opening: a messy timeline snapping into a finished edit.
  - Tools: a video file dropping into an editor, then a prompt being typed.
  - Editing: silent gaps visibly collapsing, then animated graphics appearing in the preview.
  - Payoff: a quick before/after of this very short.
- A written plan can't establish visual quality. George approved descriptions that sounded more
  expressive than what got built.

## The root causes, as we see them

1. **Our guidance anchors on templates.** "Look first at existing templates" is read as "use
   them". The templates are all text cards, so every agent makes text cards.
2. **No demonstration.** Nothing tells the agent that a motion graphic should *show* the action
   (a timeline collapsing, a prompt typing, a before/after), with text supporting it.
3. **Approval happens on words, not frames.** The agent plans, the user approves a table, and
   the first frames anyone sees are the finished set.
4. **Knobs that sound bigger than they are.** `energy` scales existing motion; agents describe
   it as "more motion".
5. **No checks the agent can run on its own work**: fonts installed, face covered, captions
   clipped by a split, "does it communicate with the text removed".
6. **The 9:16 frame isn't designed for.** Splits take half the frame for a static box; the
   caption band and the split line collide; overlay cards are opaque.

## Direction

### 1. Demonstration-led by default

The default brief for any graphic, written into the skill and the shorts and tutorial presets:

> Pick, for each moment, the graphic that adds understanding. Where the speech describes an
> action or a change, show it with animated timelines, interfaces, objects or diagrams; text
> supports the visual. Where the idea is a quote, a number, a label or a punchline, a calm card
> is the right graphic. Neither is the default; templates are starting points, not a limit:
> reject them when they can't express the idea. Describe exactly what you will build, and never
> promise motion the graphic won't contain.

**The text-removed test:** "If we removed the text, would the animation still say something?"
For a short like this one, the answer should usually be yes. The agent asks this of every
graphic before proposing it.

### 2. One sample before the set

- The agent renders **one representative graphic** (the hardest or most typical one) and shows
  it as a **short playable clip in place**: the graphic in the short with the face, captions,
  crop and audio, plus a little footage before and after (decided 25 Oct; costs one render).
  It inspects the clip itself first.
- When the look is new, or the user asked to review, the user approves the *sample*, not a
  description. Only then the remaining graphics, in the same look. With an approved look or an
  autopilot request, the agent checks the sample itself and carries on (see decision 1).
- This is PLAN-1's "style trial" applied to graphics; build it here first, PLAN-1 reuses it.

### 3. Show the plan, always, as a table

Every agent should present planned graphics the way Astra did: time, what it shows (the visual,
not just the words), mode. Make this the tool's job, not the agent's memory: `graphics` (plan or
propose) answers with a ready table the agent passes on. Opus 5.5 described graphics George never
saw, which means the plan lived only in a card.

### 4. A graphics toolkit that isn't text cards

The template set needs pieces that demonstrate, especially for tutorial and AI-tool content:
- App and editor mock-ups: a timeline with clips, a prompt box typing, a chat reply streaming,
  a file dropping into a window, a cursor clicking.
- Transformations: gaps collapsing, before/after wipe, a messy → clean snap, a counter.
- Diagram motion: steps linking, an arrow path drawing, items stacking.
- Built as parts the agent composes (GSAP is decided, 21 Oct), not as finished cards. This
  replaces most of [GFX-3](../roadmap.md#gfx-3)'s list; keep its word-timed emphasis.
- The six text-card templates stay, as equals to the new parts (decided 25 Oct): the agent picks
  per moment, by what the idea needs.
- The HyperFrames skills we have (`motion-graphics`, `hyperframes-animation`,
  `hyperframes-registry` with ~400 blocks) already hold a lot of this. Check what to reuse
  before writing our own.

### 5. Checks the agent runs before proposing

Added to `check` (or a `graphics` check) so any agent gets them, not just the careful one:
- **Fonts:** every font a graphic names is installed or bundled; otherwise refuse with the list
  of what is available.
- **Face:** in a short, an opaque area over the face track (we have face tracks, `focus_uv`) is
  flagged. Over-face graphics must be see-through around the face or sit beside it.
- **Captions:** the caption band must not cross a split line or sit under an opaque graphic;
  flag the frames where it does.
- **Motion:** a graphic that is static for most of its time is flagged ("mostly static box").
- **Promise vs. build:** the agent's description of a graphic is checked against its frames
  (it looks at them; we can't verify words automatically).

### 6. Honest style knobs

`energy` and similar style fields say in their tool description exactly what they change
("scales distance and overshoot of existing motion; adds no new motion"). An agent asked for
"more motion" must change the graphics, not only the number.

### 7. The 9:16 frame

- Split: only when the graphic moves for most of its time and the face still reads; the face
  half gets the captions, never the line between.
- Over face: the graphic frames the face (around, beside, behind cut-out), not a box over the
  chin.
- Full frame: fine when the graphic is the point (decided 22 Oct), and the best place for a
  big demonstration.

### 8. Where user taste lives

From the agents' own advice, the order of places to keep rules:
1. **Preset rules** in AeroEdits (proposed by the agent, accepted by George). They load with the
   preset in every agent. Candidates from this test:
   - "Shorts: don't use the bundled text-card templates; write animated graphics that fit the
     short."
   - "Render one graphic and show it to me before adding the rest."
   - "Never cover my face with an opaque card."
   - "Only use fonts installed on this computer."
   - "When I ask for a look ('fancy', 'more motion'), show a preview before saving or applying a
     style."
2. The user's own `CLAUDE.md` / `AGENTS.md` for habits across all work ("sample before committing
   to a creative direction").
3. **Not the skill file**: the app overwrites it on update. Our defaults go there; the user's
   go in preset rules.

A reference (a short or a screenshot George likes) beats words like "fancy". References live
**with the style**, not the preset (decided 25 Oct): the style is the look, the preset picks a
style, so every preset using that style gets them. The style flow should ask for one when it
has none, but must not *need* one to avoid generic cards.

## Bugs and UI found in the same test

<a id="bugs"></a>
- **FIX-8 · A retake cut in a short lands on the 16:9 video.** Opus 5.5 found "And the only thing
  you've gotta do after that is to da--" at ~0:18 in the short; applying the cut changed the
  16:9 timeline instead. Astra's cut on the same take worked, so the path differs by tool or
  argument (likely the `project`/target of `fillers`/`propose_cuts` when a take short is open).
  **Fixed 25 Oct:** `fillers` resolved the take's voice but cut with no short, and the take
  was also on the video, so the cut fell back to the video's timeline. `propose_cuts` had no
  `short` at all. Both now cut the short's own timeline.
- **FIX-9 · Codex blocks online transcription on autopilot.** Codex's approval reviewer (not
  ours) refuses because sending audio to an online provider "wasn't explicitly authorized". We
  decided 22 Oct that agents transcribe automatically. The fix is on our side: the preset or
  `transcribe` answer must state the user's own choice plainly (provider picked by the user in
  Settings, on what date), so a reviewer sees an explicit authorization. Verify with a real
  Codex run. **Built 25 Oct, per the review note below:** the app recorded no such choice, so
  there is now one (Transcription settings → AI agents → Allow, dated by the app); the preset
  quotes it, or tells the agent to ask once while it is off. Codex run still to do.
- **FIX-10 · Graphics wait for Accept.** Adding graphics shows a proposal card ("Accept /
  Reject / Show the short"). George: why would I need to accept graphics? They are one undo
  step like every other pass. Decided 25 Oct: graphics apply directly as one undo step, and
  the Agent panel shows a light "added 4 graphics" line with Undo. Proposal cards stay for
  risky cuts only. **Built 25 Oct** (`short_footage` add and style bring-in-line).
- **UI-8 · Shorts tab: transcript first, Style tab, no duplicate clean.** The Shorts tab has no
  Style tab; its Look tab holds "Take and what plays under it" with a "Clean the take" button
  and steps that overlap the Transcript panel, which should be the main thing. Move cleaning into
  the transcript, add Style, keep "what plays under it" short.

## Decisions, 25 Oct

1. **No Accept card for graphics.** They apply as one undo step; the Agent panel shows a light
   "added N graphics" line with Undo. Proposal cards stay for risky cuts. The sample stop
   happens only when the look is new or the user asked to review; otherwise the agent checks
   its own sample and carries on.
2. **Text cards and demonstration parts are equals.** The six templates stay; the agent picks
   per moment by what the idea needs (a quote, number or punchline can be a card).
3. **The sample is a short playable clip in place**: one graphic in the short with face,
   captions and audio, a little footage either side. One render.
4. **References belong to the style**, not the preset. A preset picks a style and gets its
   references with it.

### What a style contains (talked through 25 Oct; built in [STY-2](styles.md))

Today a Style (`style/mod.rs`, `graphics/look.rs`) is only look settings: brand colours, fonts
and logo; a base look with size, roundness, speed, energy and density multipliers and hand-set
tokens; captions; frame; portrait multipliers. Nothing says how graphics move, which kinds fit,
or shows one done well. Proposed layers, with George's answers:

1. **Settings** as today, plus the **font files inside the style**, so a style can't name a font
   it doesn't carry.
2. **Motion language:** a few written rules plus easing tokens: entrances and exits, what moves
   and what holds, hold length, hand-off between graphics, one signature move.
3. **Kinds of graphics: in the style** (decided). Which kinds it uses and when, and its 9:16
   habits (full frame, split, beside the face). The agent picks per moment within the list.
4. **Examples: linked to the library** (decided), 2–3 per style; for the long-form style they
   are George's library graphics. One source of truth; a style isn't shareable on its own.
5. **References** with the agent's written breakdown: what it took, what it dropped.
6. **Shorts: a richer variant inside the same style** (decided). Same references; the portrait
   part gets its own motion rules, kinds and examples, not only multipliers.
7. **A small sound set** (decided): whooshes, clicks and hits for graphic moments, matched to
   the style (ties into [SND-2](../roadmap.md#snd-2) and SND-3).

Look rules ("no opaque card over my face") go in the style; workflow rules ("render one sample
first") stay in preset rules.

## Suggested order

0. [STY-2](styles.md) first: the style's motion rules, kinds, examples and references.
1. FIX-8, FIX-9, FIX-10 (small, they break the flow today).
2. Guidance: the demonstration-led brief, the text-removed test, the plan table, honest knobs
   (skill, presets, tool descriptions). Cheap and affects every agent at once.
3. Checks: fonts, face, captions vs split, mostly-static.
4. Sample-first flow in the graphics tools.
5. The demonstration toolkit (replacing GFX-3's list), reusing HyperFrames blocks.
6. UI-8.
7. Rerun the same take with all three agents and compare against this file. Done when George
   would keep the graphics without asking for a redo.

## Review notes and suggested refinements, 25 Oct

These are additions from reviewing this report against the current source, not decisions or
an independent visual assessment of the failed outputs. The original renders and agent traces
were not inspected in this review.

### Separate missing capability from guidance that was ignored

The template anchoring is real: [the graphics instructions](../../src-tauri/src/mcp/skill/graphics.md)
explicitly say to write a composition only when nothing existing does the job. But the same
instructions already require a plan table, rendering followed by frame inspection, and a
preview before applying a requested style change. Adding those instructions again will not
necessarily fix the failure. Capture which tools each agent actually called, what images it
received, and whether those images reached the user. Distinguish an omitted inspection, an
inspection of the wrong target, and an agent accepting a visibly poor result.

Root cause 5 should read **no checks for these specific placement and quality failures**, not
no checks at all. [The existing gate](../../src-tauri/src/graphics/gate.rs) already checks
layout overflow, contrast, runtime errors, multiple aspect ratios, variable substitution and
entrance behavior. It is primarily a reusable-template gate. Add a separate check of the
actual timeline placement, duration, style and composite; do not require every one-off graphic
to pass a library template's long-text and alternate-duration tests.

The sample instructions should also use the existing **single** `frame_render` call with
`graphic` and `short`, or a sheet of `moments`, rather than 3–4 separate calls. That tool already
renders the finished composite, including captions and layout.

### Better graphics do not necessarily mean more graphics or more movement

The central failure is that the visuals repeat the narration without adding understanding.
Demonstration-led is a good default for this take, but a blanket ban on text cards is too broad:
a quote, number, label or typographic punchline can be exactly the right graphic. Keep calm
cards as an explicit option. Use the text-removed test as a diagnostic, with exceptions for
ideas whose meaning is inherently verbal or numerical.

Likewise, do not make “moves for most of its time” a pass condition. A clear transformation
followed by a readable hold is often better than continuous motion. Pixel changes can be
manufactured with a wobble or particle loop without improving the explanation. Review the
meaningful change, its timing with the speech, and whether the result has time to register.
Treat static-duration detection as a review hint, not a quality score or hard rejection.

Four graphics in 23 seconds is not a target. First ask which moments benefit from a visual;
leave the others with the speaker. A file drop, typed prompt and editor transformation may be
too much for a 1.9-second slot. Simplify the action or give it more time. Judge at normal speed
and phone viewing size, not only from full-resolution stills.

### Make the sample a complete, timed example

For this test, the strongest sample is probably **silent gaps collapsing in a timeline**. It
tests an actual transformation, narrative timing, small-screen readability and the choice of
full-frame versus split. Specify the visible start state, action and final state, plus the
spoken word that cues the change. A plan row should describe those, not just “animated timeline”.

Render it with the actual face, captions, crop and audio, including a little footage before
and after the slot. A contact sheet catches layout errors; a playable clip is needed to judge
pacing and motion. The agent should inspect and revise it before showing it. User approval is
not a substitute for the agent's own quality control.

There is a tension between “only continue after sample approval” and FIX-10's removal of
graphics approval cards. Resolve it explicitly: request a sample decision when establishing
a new look or when the user requests review; with an approved look or an autopilot request,
inspect the sample and continue within that authorization. Keep additions reversible and show
the result with Undo. Avoid turning every graphics pass into another mandatory stop.

### Give the agent useful constraints and examples

Before authoring, return a compact placement brief: target short and slot, actual render-box
dimensions, duration, nearby transcript, caption bounds, face bounds if available, style
tokens, and usable fonts. The agent should not have to reconstruct those from separate replies.

Start with a handful of excellent worked examples rather than hundreds of discoverable blocks.
Each should include a playable result, source, when to use it, timing structure, and a portrait
or split variant. Include a weak text-card version beside a strong demonstration with a short
explanation of why the latter works. Otherwise a larger catalog may simply create a new set of
defaults the agents overuse.

Reuse HyperFrames parts only after checking their fit with AeroEdits' existing contract:
offline assets, paused seekable timelines, style tokens, variable duration, alpha and the
actual split box. A block count does not establish suitability. Keep UI mock-ups simple and
recognizable; use the actual footage for a claimed before/after of this short so the payoff
demonstrates a real edit.

### Make checks precise enough to trust

- **Fonts:** check the font actually resolved in the renderer, including needed weights and
  glyphs, rather than only an OS installation list. Prefer a known bundled family with an
  explicit fallback. CSS generic families are valid; do not reject every fallback name in a
  stack. A missing preferred font should produce an actionable replacement or bundling choice.
- **Face:** `focus_uv` describes the crop, not an occlusion verdict. Transform detected face
  bounds through the crop and layout into output coordinates, then compare with the graphic's
  visible coverage over time. If detection is unavailable, report “not checked”, not a pass.
  A transparent background alone does not prevent an opaque child element covering the face.
- **Captions:** test the final composite at caption changes and graphic transitions, including
  the actual split position. A fixed caption band is only an approximation. Check text clipping
  and readability as well as geometric overlap.
- **Results:** distinguish objective failures, heuristic warnings and checks not performed.
  Passing technical checks must not be presented as proof of good design.

### Tighten the experiment and implementation order

The report records **two graphics failures and one blocked run**, not three completed graphics
evaluations. Agent explanations after the fact are useful hypotheses, not proof of causality.
Keep the exact prompt, model identifier, build, preset, source take and tool trace with each
run. Preserve rendered outputs for comparison; repeat runs before attributing differences to
a model.

Use the same take as a regression case, then try a different topic and aspect ratio so the
solution is not just tuned to these four sentences. Compare baseline, revised guidance alone,
and revised guidance plus worked examples. Assess relevance, hierarchy, legibility, timing,
placement and coherence across the set. Record first-pass keep/rework decisions and render
time; George's willingness to keep the result remains the final creative criterion.

Move one excellent worked example and the in-context preview loop ahead of a large toolkit
or sophisticated motion detection. That gives the next test something concrete to learn from
and makes defects visible sooner. Fix the wrong-target edit independently, but do not assume
FIX-8, FIX-9 and FIX-10 are all small before tracing their paths.

For FIX-9, report only consent the app actually recorded. Choosing a provider does not by
itself prove that the user authorized uploading this audio, and a tool response cannot create
that authorization after the fact. If a prior choice explicitly covered automatic uploads,
surface its real scope and provenance; otherwise obtain that choice through the settings or
onboarding flow. Do not invent a consent date or promise that wording alone will satisfy an
external approval reviewer.
