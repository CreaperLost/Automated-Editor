# 03 — The shot recipe-card format

- **Source:** [github.com/Vincentwei1021/video-shotcraft](https://github.com/Vincentwei1021/video-shotcraft)
- **Licence:** Apache-2.0 · captured 25 Oct 2026 · 13,522★ · last push 2026-10-17
- **Raw capture:** [evidence/2026-10-25/vincentwei1021-video-shotcraft.md](evidence/2026-10-25/vincentwei1021-video-shotcraft.md)
- **Evidence grade:** vendor claim, read from the source. Not tested in AeroEdits.

## What it is

A library of **124 recipe cards in 10 categories, 150 shots**, for cinematic product video. Each
card is a Markdown file at `references/shots/<category>/<card>.md`, with a working Remotion
implementation beside it in `demos/<category>/<card>/`. Categories: camera, data, effects,
interaction, opening, outro, rhythm, transition, typography, ui-entrance.

The cards themselves are the obvious thing to copy. **The format is the actual finding.**

## The anatomy of a card

```yaml
---
name: card-flip-reveal
一句话: 功能卡 3D 翻面揭示——卡片沿 Y 轴翻 180°…      # one line: what it does
适用: "功能→成果"的成对叙事                              # when to use
时长: 全程 150f（~5s，含 hold）；4f 预备 + 16f 翻 + ~10f 弹簧落定，三卡错峰 12f
能量: 中高                                              # energy
标签: data
---
```

then five sections:

| Section | What it holds |
|---|---|
| **意图** (intent) | which question the card answers — here "so what?", the causal pair — and how it differs from its sibling cards in the library |
| **动效核心** (motion core) | the mechanics, exactly: perspective and transform values, the three phases with frame counts and named easing curves, a lighting model that is physically linked to the angle, the stagger delay, the camera move |
| **参数表** | a table of *typical value* and **"调节手感" — how it feels to tune**, i.e. what **wrong** looks like |
| **已知坑** (known pitfalls) | the demo was tuned on grey placeholder material, so the parameters are a starting point and must be re-validated on real assets; semantic rules; performance notes |
| **参考实现** | the path to the working code |

## Why it matters here

**The "what wrong looks like" column is the transferable part.** From the card:

> flip duration: under 14f you don't see the edge moment; over 28f it reads as slow; cutting the
> prep reads as *flung away* rather than *flipped*

That is a sentence you can act on. "Use an ease-out" cannot be acted on by a model — it will
produce something plausible. "If the flip is under 14 frames the viewer misses the edge moment,
and if you cut the 4-frame wind-up it reads as thrown rather than flipped" *can* be acted on,
because it describes the symptom rather than the recipe. AeroEdits' graphics templates are
currently compositions and variables; the missing half is prose that says what bad looks like.

Three more things in the format map onto AeroEdits' existing vocabulary:

- **能量 (energy)** — a card-level attribute with values like 中高 (mid-high). AeroEdits' `Look`
  already has an `energy` token, so the two scales can be aligned rather than invented twice.
- **适用 (when to use)** and **标签 (tags)** — the purpose tags and library search that GFX-3
  already plans.
- **已知坑** — a card that says "the parameters were tuned on placeholders; re-validate on real
  material" is the same discipline as AeroEdits' "check the output over real footage", written
  into the artefact itself.

And a rule worth stealing outright: **the two sides must be semantically paired** (interface → its
outcome). A flip that reveals unrelated content is just a fancy crossfade.

## The card that also carries sound

Each card declares a **sound candidate position** — in this one, *"a light 'snap' on each card's
settle frame; three snaps double the rhythm"*. That is the cue-at-a-visual-event idea, expressed
inside the template rather than in a separate sound plan. It is the natural home for AeroEdits'
pending SND-2 work: the graphic already knows the frame where its element lands, so the cue does
not have to be guessed after the fact.

## Caveats

- **Remotion, not HyperFrames.** The cards describe techniques — timing, easing, choreography — not
  code, so the format transfers; the 150 implementations do not. This is also the same author's
  `mg-styles-15`, already in AeroEdits' corpus, so the author is a known quantity.
- Their acknowledgement states the recipes were distilled by studying the motion language of
  outstanding official product films, and that the cards document techniques re-implemented from
  scratch with no footage or brand assets. The Apache-2.0 licence covers the card text.
- 150 shots for cinematic product video is a much wider brief than AeroEdits' tutorial graphics.
  The format is worth copying; the catalogue is not ours.
