# 07 — Motion taste as a severity-tiered checklist

- **Source:** [github.com/LottieFiles/motion-design-skill](https://github.com/LottieFiles/motion-design-skill)
- **Licence:** MIT · captured 25 Oct 2026 · 2,504★ · last push 2026-05-18
- **Raw capture:** [evidence/2026-10-25/lottiefiles-motion-design-skill.md](evidence/2026-10-25/lottiefiles-motion-design-skill.md)
- **Evidence grade:** vendor claim, read from the source. Not tested in AeroEdits.
- **Note:** this repo is already named in AeroEdits' corpus as a useful reference. It had not been
  read in full; it has now been.

## What it is

A **philosophy-first, implementation-agnostic** skill that teaches an agent to think like a motion
director before writing a line of animation code. "Works with any animation system (CSS, Framer
Motion, GSAP, Lottie, Spring)". The structure is unusually good:

```
skills/motion-design/
  SKILL.md                       # decision tree + an 8-step checklist
  director/                      # core-philosophy, decision-framework, disney-principles,
                                 # emotion-mapping, motion-personality, narrative-structure,
                                 # choreography, context-adaptation
  patterns/                      # ambient-continuous, entrance-exit, multi-element, state-feedback
  reference/                     # property-selection, quality-checklist,
                                 # timing-easing-tables, troubleshooting
```

The **quality checklist** is the finding. It is a set of taste rules written as *severity tiers*:

**CRITICAL** — linear easing on spatial movement · opacity-only for an important state change ·
exceeding the 1/3 screen rule · missing primary layer · stagger over 500 ms · layout-property
animation causing jank

**HIGH** — missing secondary layer …

and the checklist itself:

| Group | Rules |
|---|---|
| Visual | elements >40 px for motion, >100 px for detail · readable at full speed without slow-motion · clear primary / secondary / ambient layers · counter-motion for balance · natural arcs · **1/3 rule (distance)**: no unbroken motion over 1/3 of the container · **1/3 rule (density)**: at most 1/3 of elements active simultaneously |
| Technical | no linear easing on spatial movement · duration matches the element type · **ease-out entrances, ease-in exits** · duration proportional to distance · **entrance duration ≥ exit duration** · not opacity-only for important changes · **stagger total < 500 ms** · follow-through: child elements offset 50–150 ms |
| Emotional | target emotion identified before properties · personality archetype matches the brand · setup → action → resolution · intensity matches importance · same interaction = same motion · appropriate on the 100th viewing |
| Performance | primary motion uses transform + opacity · fewer than 20 animated elements per viewport · no layout-triggering properties animated · elements staggered, not simultaneous · 60 fps (30 acceptable for ambient) |
| **Accessibility** | **a `prefers-reduced-motion` alternative is provided** · no vestibular triggers without an alternative · same interaction = same animation · critical information is never motion-only · **animations over 5 s are pausable** |

## Why it matters here

**Severity tiers are the mechanism.** AeroEdits' graphics gate reports findings with severities
already (`gate.rs` has a `Status` per check and the agent's report keeps executed / inspected /
approved apart). The gate already reads the rendered pixels — coverage, timing, cut, overlay shape
— so it is not a file-level checker ([11](11-the-gate-already-reads-pixels.md)). What it cannot
check is *motion quality*: it samples three stills per run, so it can prove a graphic is
see-through at its first frame but not that the entrance *eased*. A severity-tiered taste checklist
is the way to add that without a model — every rule above is measurable from the composition's own
timeline.

Several of them are already half-present in AeroEdits' vocabulary and need only to become checks:
`Look` has `speed` and `energy`; the look has named `ease` and `duration` tokens; the gate already
knows 16:9 and 9:16. The 1/3 density rule and the stagger budget are arithmetic on the
composition.

**Two of the rules are accessibility, not taste, and AeroEdits has neither:**

- a **`prefers-reduced-motion`** alternative,
- and **nothing over 5 seconds that cannot be paused**.

Neither has been considered anywhere in the project. They are cheap to implement at the source
level — a reduced-motion variant is a second set of tokens — and they are the kind of thing that
becomes expensive to retrofit once a library of templates exists.

**The decision framework is the other half.** The `SKILL.md` opens with a decision tree —
*functional purpose? → timing rules · brand personality? → motion archetypes · story? → Disney
principles and choreography · complex scene? → the 1/3 rule and stagger patterns* — and an 8-step
checklist where **step 1 is the emotional target** and step 2 is the motion personality, before any
technical decision. "Motion Personality archetypes" (Playful, Premium, Corporate, Energetic) map
onto the `mood` field in FrameJam's styles and onto AeroEdits' `energy` token: three tools now
using three different vocabularies for the same axis.

## Caveats

- **It is written for UI motion**, not for video graphics over footage — "micro-interactions,
  loading states, page transitions" is the brief. The timing, easing and layering principles
  transfer; "readable at full speed" and "the 100th viewing" are UI-frame concerns.
- Its own README says the skill was last committed 2026-05-18 — five months stale relative to
  this capture, though it is stable by nature.
- The "Disney principles" and the specific pixel thresholds are one team's taste. The **mechanism**
  (taste as tiered, measurable findings) is the transferable part; the numbers are a starting
  point to be tuned on our own videos.
- Nothing here was tested in AeroEdits.
