# Agent video gaps and proposals — from the October 2026 reference review

Reviewed: 21 October 2026.
Source: [reference-video-review-2026-10-21.md](reference-video-review-2026-10-21.md) — eleven videos on
AI video editing and motion graphics, plus a read of the current MCP, skill and graphics surface.
Full captions for all eleven are in [transcript.md](transcript.md); several proposals below are quoted
from those transcripts rather than from the review's summaries.

This document is the "so what". It states where AeroEdits already beats every workflow in the review,
where it is behind, and what we would build, in what order.

---

## The one finding that matters most

Three head-to-heads, pointing in different directions, and the differences between them are the most
useful thing in the whole review.

- **Chase AI (four cases)** put motion graphics in the **tie** column. Both models were wired to the
  Higgsfield MCP, both called the same skill, both routed the prompt and returned polished 15-second
  explainers. His own conclusion was that *both* are excellent at orchestrating external tools.
- **Nate Herk (15 cases)** put Astra **ahead** — more energetic, 3D, shadows, depth — and credited it
  for going and finding real material from the thing it was promoting. 10–5 overall, $327 vs $513.
- **Marcin AI — video #5 in the set itself** ran the identical single prompt in both models on ultra
  mode and got the opposite verdict to Chase: *"Astra 6 crushed this."* Using the **same** Higgsfield
  MCP.

Three results, two apparently contradictory. The reconciliation: what flattened Chase AI's pair was
**not** the external generator — it was both models converging on the same assets. Marcin's Astra
differed by reaching for **video instead of stills**, sourced from real demos, and by shipping while
Fable was still rendering.

So the rule that survives all three is narrower than the one we started with:

> Model choice becomes visible when the model exercises **judgement about inputs** — what to fetch,
> what to show, when to use motion. It stays invisible when the model is only routing a brief to a
> generator that draws the frames.

**That is the shape AeroEdits is already in.** We render deterministic compositions ourselves, and
every graphic we ship is one the agent chose to place at that moment. There is no external video model
in the loop whose output could flatten every composing model to the same result.

The corollary is the risk, and the four further comparisons collected with the transcripts sharpen it.
On isolated, heavily art-directed clips Astra is competitive or better. On **full edits driven by a
skill**, the result is consistent rather than spectacular, and cost can double — one creator had Fable
finish for £5.90 against Astra's $15.05 and preferred Fable's output, judging Astra *"technically more
impressive"* but doing things *"a human would never do."* Another found the GPT edits *"felt like
glorified slides"* while Opus 5.5 was best overall and Fable 5.1 best on individual graphics. Nate
Herk's conclusion stands: his quality jump came *less* from the model than from real reference
inspiration, a named brand filter, and a purpose-built skill. We have the renderer. We are thinner on
the other three.

---

## Where we already win

Worth saying plainly, because it shapes which proposals are worth making.

| Capability | Ours vs the videos |
|---|---|
| Cut accuracy | Word alignment (wav2vec2, 120 ms guardrails), word-edge refinement, per-source vetoes, "keep tagged sounds". The videos' best rule is *"never cut inside a word, pad edges 30–200 ms"* — we do that with measured audio and guardrails. |
| Verification | `check` audits the finished edit against ~12 classes of viewer-visible defect, each with a one-click fix, each its own undo step. No video in the set has an equivalent. |
| Graphic QA | `graphic_check` is a no-model gate: lint, layout overflow and contrast at 16:9 **and** 9:16, own and long text per variable, stills at 3 s and 8 s, cut-in/out vs eased, overlay coverage. |
| Approve-before-apply | `propose_cuts` / `propose_edit` stage a batch the user accepts in the Agent panel, with `expectedRevision` and a single undo. `video-use` calls its equivalent "the single best guardrail in this whole workflow"; we shipped it. |
| Cost of operation | Qwen3.5 4B on-device for filler/retake review, greedy with a JSON grammar so the same words give the same cuts. No key, no cloud, no spend. |
| Context cost | One action returns one line. The skill explicitly tells agents a pass "costs a line of text" and that reading transcripts or rendering frames is expensive. |
| Reformat | Graphics render per canvas and per short slot, layouts adapt, the same composition fits 16:9 and 9:16. Pattern 8 of the review. |

---

## Where we are behind, ranked

Ordered by leverage, not by effort.

### 1. There is no sound design surface — only ambient telemetry polish

**The gap.** Sound is half the product in this review. Joseph placed 106 sound effects and set music
at −20 dB; Matt Loui finishes every video with whooshes, UI pops and line sounds; Chase AI's
generations carry a full soundscape and #11 prompts audio explicitly. Not one of the eleven treats
sound as an afterthought.

We have `soundEffects` (`clicks`, `whooshes`, `muted`, `volumeDb`) — a synthesized lane placed
*automatically from mouse telemetry*, with careful anti-busy rules. That is genuinely good and nobody
else in this set has it. But it is ambient polish on the one source we are strongest on. There is no
way for an agent to place a cue at a chosen time, no music track, no beat grid, and no short-scoped
audio cue API — a gap our own [chatcut review](chatcut-competitive-review-2026-10-21.md) already named.

**Proposal.** A `sound_design` action and a sound event model: cues with a kind (whoosh, pop, click,
line, riser, impact, sting), a time, a gain, and a source (agent-chosen, or derived from a graphic's
`data-entrance`). Paired with `audio_cues` on a short so a vertical cut gets its own score. Music
follows as a second step: a music track with loudness relative to speech, which we already understand
via the −14 LUFS check.

### 2. There is no house style as a first-class object

**The gap.** Every one of the eleven videos sells the same thing in different words: the quality lives
in the **training artefacts**, not the model. Joseph's visual guide (fonts, colours, placement, the one
word to highlight), RoboNuggets' design-system Markdown and element library, Matt Loui's real reference
folder, Nate Herk's Scrollcraft skill, kaye's custom skill. Our nearest equivalents are the graphic
library (picture only) and brand kits (colours, font, logo).

The missing thing is the **decision policy**: caption style, animation taxonomy, graphic density, hook
treatment, motion tempo — and a **banned list**. `motion-graphics.org` states one outright, and it is
the sharpest style artefact in the review: *banned — crossfades, blur-ins, brightness "developing", 3D
flips, particles, glows, holds longer than 1 s, anything that looks like a template.*

**Proposal.** A **style kit**: a versioned, in-app-editable, agent-readable document carrying caption
rules, an animation taxonomy, graphic density and placement, safe zones, a banned list, and reference
images. `library_save` already teaches agents to write rules when templating a graphic — a style kit is
the same mechanism one level up, and the SKILL.md section that explains it is a natural home. Brand
kits stay the *values* (colours, font, logo); the style kit is the *rules for using them*.

### 3. Two of the four scene archetypes have no templates

**The gap.** Joseph splits every animation into four types — **text-centric, person-centric,
object-centric, graphic-centric** — then plans a scene by picking one per line. That taxonomy is what
makes planning a choice instead of a blank page.

Mapping our six bundled templates against it: text-centric is covered (`title`, `key-point`),
graphic-centric is covered (`step`, `shortcut`, `recap`, `end-card`). **Person-centric and
object-centric have no template family at all.** Everything we ship is a full-screen card of 4–6 s.

**Proposal.** Add template families for the two missing archetypes — lower-thirds and callouts built on
the person; object/UI treatments (browser mockup, phone frame, numbered stamp, cursor demo) built on a
thing. Separately, the `blank` + Web Animations path can express these but only if an agent writes
the HTML, which is the expensive path.

The full transcript sharpened this into something more specific and more actionable. Joseph's visual
guide has **three** parts, and the third is the one we have no equivalent of at all: **on-screen
animations** — the small things that pop up over him while he talks, governed by exactly two rules.
*"The background behind it darkens and blurs, and the animation sits in the lower half of the
screen."* That is a **lower-half overlay with a dimmed, defocused backdrop** — a one-line rule, and
precisely the composition that recurs across this review and that our full-screen card templates
cannot produce, since we ship nothing that sits *over* the frame except by hand-writing HTML.

Proposal, tightened: a **lower-half overlay family** — `data-overlay` is already supported and
already the gate's tested path for "eases in and out", the mechanics exist; what is missing is a
template, a darkened-blurred backdrop standard, and a placement rule in the skill. This is close to
a day's work and it closes the most-repeated gap in the set.

### 4. Graphics are capped at Web Animations and CSS

**The gap.** `graphic_write_file` states the rule plainly: *"no network (no CDN scripts or web fonts);
Web Animations or CSS, finite and paused."* And the gate flags CDN scripts. `graphic_write_file` writes
`index.html`, or an image or font — **no sanctioned way to vendor a JavaScript animation library.**

Everything in this review's output leans on what that excludes: GSAP timelines with real easing
curves, squash and stretch, physics ("real falling-label physics"), three.js and WebGL scenes, shaders,
character rigs. Two videos are built specifically on rigging and keyframe-equivalent precision inside
After Effects, and Vane Motion's test is entirely about whether the model can reproduce squash-and-stretch
easing and hand rigging correctly.

Our own [motion-graphics-research.md](motion-graphics-research.md) recorded that our HyperFrames skills
already cover "3D, shaders" — but that was read off the skills' documentation, not tested under our
own no-network gate. **This is the one place where prior research may be out of date with our own
constraints, and it should be verified before it is relied on.**

**Proposal.** Either vendor a pinned local animation runtime into the graphics folder the same way we
pin Node, Chrome and HyperFrames, or state the ceiling explicitly in the skill so agents stop
attempting physics and reach for what we can actually do well. Cheapest version of the second option
costs nothing and removes a whole class of agent failure.

### 5. No plan the user edits before the edit happens

**The gap.** Every mature example has an approval gate between planning and execution — Tao's brief
that must be confirmed before work starts, Chase AI's storyboard that the build is checked against,
Joseph's planning page where he picks a scene type per line for fifteen minutes.

We have the *guardrail* (proposals, per-pass dry runs, `check`). We do not have the *artefact*. And
`storyboard`, despite the name, is a review of the edit that exists, not a plan for the one to come.
The closest thing is `take-shorts.md`'s beat split, which lives in the template's prose rather than in
the app.

**Proposal.** A `plan` action producing a scene list — time, scene type, what it shows, planned zoom,
planned graphic, words — that the user edits in a small panel before anything is applied, and that the
graphics pass then reads from. This is the piece that makes "plan, approve, build" a real loop here,
and it is what the storyboard's sibling should be.

### 6. Captions are not a designed system

**The gap.** `settings area: captions` is a settings patch, not a design surface. Captions are the
first thing every short-form video in this set talks about — animated word by word, italics on the key
word, a specific font pair, a specific position, highlight the one word that makes you keep watching —
and we have no agent-facing way to express any of that. We can turn captions on. We cannot style a
performance.

**Proposal.** Caption style presets plus word-level emphasis. A `captions` action taking a style
(weight, size, position, colour pairing, max words per line), a per-word emphasis rule, and a safe-zone
reference — plus a check rule for overlap with the webcam bubble, which `check` already reports but
cannot fix.

Joseph's captions specification is small enough to build verbatim, and every element of it is a
setting we do not have: **one medium sans for most words and one script font for the special words**;
**white for normal words and one accent colour for the words that want to pop**; a stated position; and
an animation of **word-by-word pop-up fade-in**. On top of that, one rule that is arguably the single
most valuable line in the whole transcript set — *"the captions highlight the one word that makes you
keep watching."* That is a per-word emphasis the agent chooses by meaning, which no template and no
static style can express, and it is exactly the kind of decision the skill layer should be making.

### 7. Agents can create graphics but not remove them

**The gap.** `graphic_add`, `graphic_set`, `graphic_write_file`, `graphic_render`, `graphic_templatize`,
`graphic_check`, `library_save` — and **no `graphic_remove`, no move, no reorder.** The UI has
`graphics_library_remove`; the agent surface does not. There is also no way to delete a short, no way
to edit or remove a library item, and no way to author a brand kit from an agent (kits are "made by the
user in the Graphics tab").

Every practical agent session ends by cleaning up. Ours cannot.

**Proposal.** `graphic_remove`, `graphic_move`, `library_remove`, `short_remove`, and a
`brand_kit_set` that writes colours, font and logo the way `graphic_set` writes variables. Small,
mechanical, and it removes a standing source of agent frustration.

### 8. There is no motion review

**The gap.** The videos verify by watching — "Codex checks its own work against the storyboard, kicks
off the render, checks the final render." We verify with stills: `frame_render` returns one PNG,
`graphic_render` reports lint and status, `storyboard` returns a contact sheet, `layout_get` returns
rects. Every one of those is a still. **Pacing, easing and overlap are invisible.**

Our own [short-form review, part 2](short-form-reference-review-part-2-2026-10-21.md) found that
`RENDER_FPS` is fixed at 30 and `RENDER_LONG_SIDE` at 1920 with no settings knob, so a higher export
rate buys no extra motion samples. A still-frame review loop over a fixed-rate renderer is structurally
unable to judge motion.

**Proposal.** `motion_preview`: a short, low-resolution, full-rate clip of one graphic or one timeline
range, so the agent and the user can watch the thing rather than sample it. Follow with the separate
preview/final fps and resolution policies the part-2 review already recommended.

### Below the line

Real gaps, not this quarter's work, and all already known:

- **Footage search** — Nate Herk cut 152 GB into a one-minute recap; Tao hands the agent specific clips
  to place. `future-work.md` #14 already names search and summarise over source footage as the biggest
  gap found in the Premiere MCP review. Still true.
- **Reformat as an operation** — our graphics and layouts adapt to the canvas, but there is no pass
  that takes a finished 16:9 edit and produces the vertical version. Pattern 8 of the review treats it
  as routine; we treat it as manual.
- **Per-step timing** — Tao asks for it explicitly. Our actions return one line each; nothing
  accumulates a profile of where a long run actually spent its time.
- **Render settings are not agent-controllable** — fps and long side fixed (see #8).
- **Proposals are not bound to project identity** — `expectedRevision` alone does not identify a
  project (`future-work.md` #17).
- **53 tools is a context-cost concern** already on the backlog, and adding seven more makes it worse.
  Anything added above should be considered against whether it can be merged into an existing action.

---

## What we should not do

The videos tempt us in four directions we have already decided against. Nothing here overturns those
decisions; the videos are evidence for them, not against.

- **Do not add Remotion as a second renderer.** Joseph and Matt Loui are in the Remotion camp, HyperFrames
  is the better-supported one, and we already ship a pinned, integrity-checked HyperFrames. Two renderers
  means two style systems and two sets of failure modes.
- **Do not chase After Effects.** Six of these videos route through the Higgsfield AE plugin, and that is
  the strongest pull in the set. Our [AE investigation](after-effects-mcp-investigation-2026-10-21.md)
  already concluded that an MCP connection supplies control of an engine, not art direction. Joseph's
  own evidence is the argument: his twelve hours and 4.5 billion tokens went into a *skill trained on his
  past work*, not into After Effects.
- **Do not add generative video.** Seedance 2.5 appears twice in this review. It produces attractive
  motion with no editability, no determinism and no place in a timeline. Our product is the opposite
  promise, and Chase AI's own tie result shows what delegating the pixels costs you.
- **Do not build a marketplace or a public skill store.** The ecosystem is full of narrow single-purpose
  skills; we do not win by being the eleventh of them.

---

## Recommended order

**Tier 1 — do now.** Each is small, each unblocks the others, and each is referenced by most of the
eleven videos.

1. **Sound design surface** (gap 1) — the largest functional gap, and the one every video leads with.
2. **House style as a style kit** (gap 2) — compounding; it is the artefact every video is actually
   selling, and it makes templates, captions and graphics consistent for one owner.
3. **Missing template families + captions design** (gaps 3, 6) — the visible output; two archetypes and
   the element every short-form video is built around.
4. **Agent CRUD cleanup** (gap 7) — mechanical, and it stops every session ending in a broken state.

**Tier 2 — then.** 5. Plan/approve artefact (gap 5). 6. Motion review plus preview/final render
policies (gap 8). 7. Settle the graphics animation ceiling either way (gap 4) — resolving it is
cheaper than leaving it ambiguous, and if the skills really do cover 3D and shaders under our own gate,
saying so converts a suspected limitation into a capability.

**Tier 3 — backlog.** Footage search, reformat, timing profile, render settings, project-bound
proposals, tool-count consolidation.

---

## The test to apply to each

Every proposal above should pass this, taken from the review's most repeated sentence — plan, approve,
build, then **have the agent verify its own work**:

> Does this make the agent's output more checkable without the user watching every frame?

The pass/floor settings on silence and fillers pass it — the dry run reports measurable numbers and the
user approves those numbers. `check` passes it — findings jump to their moment, each with a fix. A style
kit passes it only if the banned list becomes checkable; a document nobody enforces is a preference, not
a guardrail, and should be built as rules in `graphic_check` or not built at all.