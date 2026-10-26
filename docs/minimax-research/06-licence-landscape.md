# 06 — Licence landscape: what we may actually reuse

- **Sources:** every capture in [evidence/](evidence/), licences read from each repository's licence
  file on 25 Oct 2026
- **Evidence grade:** licence files as committed in each repository on the capture date. Not legal
  advice. Read the licence text before relying on any of it.

## The rule

AeroEdits' licence rule, from `docs/decisions.md`: a bundled model or component may be **MIT,
Apache-2.0, GPL-3.0 (as a separate component) or OpenRAIL**. **Never** non-commercial,
research-only or revenue-capped.

Two questions the rule does not answer yet: is **CC0** acceptable (no attribution required, so it
looks compatible in spirit, but it is not on the list), and does the rule apply to *content* —
recipe cards, styles, prompt text — as well as to code.

## What each reference actually allows

| Reference | Licence | Verdict |
|---|---|---|
| `Vincentwei1021/video-shotcraft` | Apache-2.0 | **usable** — code, cards, text |
| `iart-ai/b-roll-skills` | MIT | **usable** |
| `iart-ai/explainer-video-skills` | MIT | **usable** |
| `iart-ai/motion-design-skills` | MIT | **usable** |
| `iart-ai/motion-skills` (hub) | MIT | **usable** |
| `moritzkremb/framejam` | MIT | **usable** — including the 30 styles, MIT with original copy and no brand assets |
| `LottieFiles/motion-design-skill` | MIT | **usable** |
| `frankxai/awesome-motion-design-agent-skills` | CC0-1.0 | probably fine — **needs a decision** on CC0 |
| `zhuyansen/awesome-claude-video-skills` | CC0-1.0 for the list; each project keeps its own | the list is free; the projects are not automatically so |
| **`Vincentwei1021/video-talkcraft`** | **PolyForm Noncommercial 1.0.0** | **not usable** — noncommercial. Study the method, copy nothing |
| **`0xsline/OpenChatCut`** | **AGPL-3.0** | **not on the list.** Study it as a competitor; do not copy code |
| **Remotion** | custom — free for individuals and companies of up to 3 employees; larger companies need a licence | **revenue-capped → refused.** Already decided: HyperFrames is our only engine |
| **`remotion-dev/skills`** | **no licence file** (default copyright) | **not usable.** The skills teach an API we have refused anyway |

## The two findings that matter

**1. The best method is the one we cannot use.** `video-talkcraft` is the closest match to
AeroEdits' thesis we found — script-as-truth trimming, the anti-slideshow camera policy, the
schematic-companion rule, per-shot pipeline state. Its licence forbids commercial use, so the
implementation cannot be reused. Its sibling `video-shotcraft`, by the same author, is Apache-2.0
and freely adaptable — and the **format** is the part worth taking anyway (see
[03](03-shot-recipe-card-format.md)). So the practical answer is: take the *shape* of the method
from the non-commercial repo, take the *artefacts* from the Apache-2.0 one.

**2. Licence risk sits in the reference, not the code.** The most copied thing in this field is not
code but **content** — recipe cards, style tokens, prose guides. FrameJam's 30 `style.json` files
are MIT and cleanly reusable. The LottieFiles skill's principles are MIT. But the same *idea*
reproduced by a different author under a different licence is not, and `video-talkcraft` shows how
fast that changes across one author's own repos.

## Open questions for George

1. **Is CC0-1.0 inside the rule?** It is public-domain-equivalent and requires no attribution, so
   it is compatible with the rule's intent (no non-commercial clause, no attribution requirement,
   no revenue cap). But it is not on the list, and the list was written deliberately.
2. **Does the rule cover content, or only code?** A style token set, a recipe card and a prose
   guide are not code. If the rule is about what we *ship inside the app*, it should cover them.
3. **Do we need an attribution file?** Apache-2.0 requires a notice. Any app that ships adapted
   cards needs a place to keep them — which does not exist today.

## A note on the reminder to check licences

Several of these repositories declare `NOASSERTION` or `Custom license` in GitHub's metadata while
committing a licence file that says something else entirely — PolyForm, and Remotion's
revenue-capped terms. **The SPDX tag is not the licence.** Every entry above was read from the
file.
