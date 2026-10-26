# 18 — Three more director layers on top of our own runtime

- **Sources:**
  [geekjourneyx/hyperframes-motion-director](https://github.com/geekjourneyx/hyperframes-motion-director) ·
  [gloweaseco-leo/hyperdirector](https://github.com/gloweaseco-leo/hyperdirector) ·
  [Kianzzz/xilo-opus-video](https://github.com/Kianzzz/xilo-opus-video)
- **Licence:** **AGPL-3.0 — study only, do not copy** · Apache-2.0 · MIT · captured 25 Oct 2026
- **Raw captures:** [motion-director](evidence/2026-10-25/geekjourneyx-hyperframes-motion-director.md) ·
  [hyperdirector](evidence/2026-10-25/gloweaseco-leo-hyperdirector.md) ·
  [xilo-opus-video](evidence/2026-10-25/kianzzz-xilo-opus-video.md)
- **Evidence grade:** vendor claims, read from the source. None installed.

These three were the references [10](10-the-rest-of-the-field.md) could not read — GitHub's
rate limit refused them. They are now captured, and they are the three that turned out to matter
most: **every one of them is a layer built directly on top of HyperFrames**, which is the runtime
AeroEdits already ships.

Together with the official `/hyperframes` skill from finding [12](12-hyperframes-is-our-runtime.md),
there are now **four** director layers around our runtime. That is a signal about where the
ecosystem is going, and a collision we have to look at.

## What they are

| | Stars | What it is |
|---|---|---|
| **hyperframes-motion-director** | 468 | An Agent Skill for planning, producing and reviewing HyperFrames motion videos. Chinese-first vertical promo defaults: 9:16, 1080×1920, concise screen copy, readable hold frames, a review step before delivery. |
| **hyperdirector** | 17 | A Hermes skill pack: *"a director layer **on top of** HyperFrames — not a replacement for it."* Capability judgment, brand memory, template selection, QA fix loop, delivery packaging. |
| **xilo-opus-video** | 278 | A Claude Code skill for code-composited video. Canvas / HTML·CSS·SVG / WebGL shaders / Three.js / Remotion / HyperFrames / Manim, with code-synthesised music. |

## The shape they all converge on

```
propose → confirm → design artefact → build → check each unit → render
```

**motion-director** states it as two phases: *"1. Create a brief/design proposal and wait for
confirmation. 2. Generate assets, build the composition, validate snapshots/renders, and write a
review report."* Its scaffold writes four files:

```
BRIEF_DESIGN_PROPOSAL.md   Direction, format, visual plan, motion plan
DESIGN.md                  Visual system, asset rules, layout contracts
STORYBOARD.md              Beats, screen copy, timing, transitions
REVIEW_REPORT.md           Checks, snapshots, issues, remaining risks
```

**hyperdirector** writes the same four concepts as JSON:

```
User prompt → brief.json → storyboard.json → DESIGN.md → index.html
  → npx hyperframes lint / preview / render → final.mp4 + editable HTML source
```

**xilo-opus-video** writes the same four concepts too, and is the only one of the three that
explains *why*:

> Throw one sentence at it and the result is mostly luck: you don't know how it planned, and what
> works this time may not work next time with a different topic.

Its four steps:

1. **First ask what you want to do** — an idea, an article, a product website, a codebase, or a
   reference video.
2. **Give three options** — style, technical combination, shot-by-shot visual summary, sound, and
   **estimated render time**. The three are *"deliberately spread apart"* (刻意拉开差别).
3. **Each option gets a preview** — a keyframe, or a 2–4 second sample.
4. **After you confirm, then make it** — write the confirmed option into `brief.md` (a structured,
   reusable prompt), produce it shot by shot, **produce a still per shot and self-check it**
   (逐个镜头出静帧自检), then render to MP4.

## The best planning idea in this whole folder

**Three deliberately different options, each with a preview and a price.**

Every other reference in the folder gives you *one* way to do the thing, executed well. xilo gives
you **three ways that are deliberately unlike each other**, each rendered as a keyframe or a 2–4
second sample, each with an estimated render time, and it makes **you** pick. The point is not
efficiency — it is that a single-shot agent's output depends on luck, and three visibly different
previews replace luck with a choice.

This is the direct answer to the thing finding [10](10-the-rest-of-the-field.md) measured: six of
sixteen skills looked like the default. xilo does not try to be a good default. It refuses to pick
for you, so the default never gets a chance to be the answer.

Note also that this is **the opposite of AeroEdits' presets**. We ship named presets and the agent
picks one (`preset` → *tutorial*, *shorts*, *clips*). xilo's three options are generated for the
request and deliberately unlike each other. Both are "propose, then confirm" — but ours narrows the
space, theirs widens it before narrowing.

## And the cheapest check anyone has described

**xilo: a still frame per shot, self-checked, before the render.** That is per-unit verification
of the thing being made — one picture per shot, looked at, before the whole thing is rendered.

Compare: video-use checks every **cut boundary** of the rendered output; kitcut checks the
**finished render**; framejam reviews the **render**. xilo checks **each shot before the render**.
Cheaper, earlier, and it catches the defect before the expensive part.

## The artefact names have already converged — and not on ours

| | the design artefact |
|---|---|
| HyperFrames official (`frame.md`) | *"The output is a **DESIGN.md superset** your whole toolchain can read"* |
| motion-director | `DESIGN.md` — "Visual system, asset rules, layout contracts" |
| hyperdirector | `DESIGN.md`, between `storyboard.json` and `index.html` |
| framejam | `style.json` — palette, fonts, easing, transitions, rhythm, **guide** |
| **AeroEdits** | `Look` (`src-tauri/src/graphics/look.rs`) — size, roundness, speed, energy, density, plus `ease` / `duration` / `gsap-ease` tokens |

**Three of the four projects around our runtime now call it `DESIGN.md`, and the runtime itself
says its format is a "DESIGN.md superset".** That is a naming convergence we did not cause and
cannot ignore.

It does not mean we should rename `Look`. It means the **format** is worth reading, and the
question finding [12](12-hyperframes-is-our-runtime.md) left open — *"is our `Look` a subset of
what `frame.md` expresses?"* — now has three projects' worth of evidence that `DESIGN.md` is the
name the ecosystem is converging on. A file we can read from a third party is worth more than one
we invented.

## Where this lands against AeroEdits

| | AeroEdits | the three here |
|---|---|---|
| Propose, then confirm | yes — the agent proposes, the user approves | yes — all three gate on confirmation |
| A named design artefact | `Look` + brand kit | `DESIGN.md` / `style.json` |
| A storyboard artefact | `storyboard` tool — a **PNG sheet of the finished edit**, not a plan | `STORYBOARD.md` / `storyboard.json` — a **plan** |
| Per-unit check before the render | the gate, per graphic | xilo, per shot |
| Review report | the gate's findings + the user's review | `REVIEW_REPORT.md` — "Checks, snapshots, issues, remaining risks" |
| The agent's edit loop | Q-VERIFY: **no autonomous fix-and-retry** | hyperdirector has a **QA fix loop** |

Two of those rows are ours and are stronger than theirs. Our `storyboard` tool is not a plan — it
renders **the edit as it will be exported**, one finished frame per scene, with what is said. It
is a check artefact, not a plan artefact, and it is the same idea as video-use's composite
([13](13-the-llm-reads-the-video.md)).

**hyperdirector's "QA fix loop" is the one place they are ahead, and it is exactly what we have
refused.** Our Q-VERIFY says *"No autonomous fix-and-retry loops; never export without the user's
go."* An autonomous fix loop is a fix-and-retry loop. If we ever want one, it has to be a
decision, not a drift — and the distinction to keep is *self-check then show* (video-use's, which
ends at a preview) versus *repair then publish*.

## Caveats

- **motion-director is AGPL-3.0** and therefore **refused for reuse** — the same verdict as
  OpenChatCut in finding [09](09-competitor-openchatcut.md): *study it; do not copy code*. Note
  that AGPL-3.0 is **not** the same licence as the GPL-3.0 on our allowed list, and our list names
  "GPL-3.0 (as a separate component)". If someone wants to reuse its four-file scaffold, that
  needs an explicit decision rather than an assumption.
- **hyperdirector is 17★ and v0.1.1**, calling itself an "experiment". Its one-command installer
  is `curl … | bash` — it does say the scripts can be inspected first, and a skill pack is not
  something to install by piping from the internet.
- **xilo-opus-video is Chinese-first**, and its three examples are all product-launch promos. The
  *method* is language-independent; the sample content is not.
- **All three are director layers for making promos from a product or a README.** None of them
  edits a screen recording with a speaker and a transcript — which is AeroEdits' actual subject.
  Read them for the method, not for the workflow.
- **None was installed or run.** The stars are discovery signals.
