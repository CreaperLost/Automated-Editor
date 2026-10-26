# 10 — The rest of the field, and what is left to read

- **Sources:**
  [zhuyansen/awesome-claude-video-skills](https://github.com/zhuyansen/awesome-claude-video-skills) ·
  [frankxai/awesome-motion-design-agent-skills](https://github.com/frankxai/awesome-motion-design-agent-skills) ·
  [iart-ai/motion-skills](https://github.com/iart-ai/motion-skills) ·
  [iart-ai/motion-design-skills](https://github.com/iart-ai/motion-design-skills) ·
  [iart-ai/explainer-video-skills](https://github.com/iart-ai/explainer-video-skills)
- **Licence:** CC0-1.0 (both lists' own text) · MIT (all three iart-ai packs) · captured 25 Oct 2026
- **Raw captures:** [zhuyansen](evidence/2026-10-25/zhuyansen-awesome-claude-video-skills.md) ·
  [frankxai](evidence/2026-10-25/frankxai-awesome-motion-design-agent-skills.md) ·
  [motion-skills](evidence/2026-10-25/iart-ai-motion-skills.md) ·
  [motion-design-skills](evidence/2026-10-25/iart-ai-motion-design-skills.md) ·
  [explainer-video-skills](evidence/2026-10-25/iart-ai-explainer-video-skills.md)
- **Evidence grade:** creator claims and one self-run end-to-end test. **Nothing here was installed,
  run or benchmarked against AeroEdits.** The zhuyansen test is the field's own harness on the
  field's own machine, judged by the author's eye — a measurement of other people's work, not of
  ours.

This file is the sweep: the two lists that map the field, the pack family that sits behind them,
and the references that are still uncaptured.

## 1. Two people keep the map, and they keep it differently

| | [zhuyansen/awesome-claude-video-skills](https://github.com/zhuyansen/awesome-claude-video-skills) | [frankxai/awesome-motion-design-agent-skills](https://github.com/frankxai/awesome-motion-design-agent-skills) |
|---|---|---|
| Stars | 922 | 80 |
| Last push | 2026-10-24 | 2026-10-24 |
| Licence | CC0-1.0 (custom dedication, not the SPDX tag) | CC0-1.0 |
| What it holds | **340 repos**, grouped by type, each read and security-graded | **5 first-party skills**, plus a curated catalogue and "earned" third-party packs |
| Grading | SAFE / CAUTION / *pending*, from a decision model reading each README | "Pulse" — stars, licence, maintenance date |
| Doctrine | none written | *"static hierarchy first. Animate only to clarify state or narrative."* |

Both are worth keeping. zhuyansen is the map; frankxai is the taste and the safety rules. Neither
is a licence we can lean on for the *projects* they point at — each project keeps its own licence,
which is the rule the zhuyansen licence file states in as many words: *"The list itself (the
selection, the grouping and the text written for it) is dedicated to the public domain under CC0
1.0. Repository names and descriptions belong to their authors and are quoted from GitHub. Each
listed project has its own license; check it before reuse."*

## 2. The most valuable evidence in this whole folder: sixteen skills, one brief

The zhuyansen list contains a controlled comparison nobody else in this folder has — and it is the
single most useful thing we read.

**The setup** (11 Oct 2026). Sixteen video skills plus **Claude Code with no skill at all** were
each asked to make the same film: a 15-second promo for a made-up app, 1920×1080 at 30 fps, five
fixed lines of on-screen copy, in a throwaway sandbox **with no API keys for video, image or voice
models**.

**The checks.** Four per file: it plays; it is the length and size asked for; every line of copy can
be read in frames sampled twice a second; the picture moves. How good a film *looks* was not
scored by machine.

**The result.**

| | |
|---|---|
| Passed all four checks | **17 of 17** — including the no-skill baseline |
| Lines of copy missing or added | none, in any film |
| Time taken | 2.0 → 23.5 min; the **no-skill baseline was 4.4 min** |
| Added sound | **9 of 16** |
| Looked like the default | **6 of 16** ("deep teal water, waves, white centred type") |
| Looked like it had a look of its own | **10 of 16** |

Their conclusion, in their words: *"A skill earns its place with a look, a workflow, or footage to
edit."*

**Why this matters to AeroEdits.** This is the field's own controlled test, and it says something
sharp: **correctness is free**. Every one of seventeen runs produced a playable film with every line
of copy present. Nothing failed. The only axes that separated them were time, sound and *the look*.

That is AeroEdits' thesis, arrived at from the opposite end. "One look for every graphic in a
video" is not a nicety we chose for taste's sake — on this benchmark it is the **only** axis on
which an agent-made video is better or worse than another agent-made video. Six of sixteen shipped
films that a viewer could not tell from no skill at all, with a skill installed and claiming
otherwise.

The lesson for us is sharper than "taste is good": **"we installed a skill" is not evidence of
taste.** Six of the sixteen failed the only test that mattered, and they failed it while looking
successful on all four mechanical checks. Our gate is a mechanical check too. It is necessary and it
is not sufficient, and finding [11](11-the-gate-already-reads-pixels.md) is about exactly where
ours sits between the two.

**The caveat that matters most.** This brief is a 15-second promo for an app that does not exist.
No talking head, no screen recording, no transcript, no word-anchored graphics — the zhuyansen
note says so directly: *"motion-graphics (built for talking-head footage, which this brief did not
have)."* AeroEdits' territory is precisely what this test did not cover. Read it as a baseline for
**code-rendered promo work**, not as a measure of the talking-head case.

## 3. The portable mechanism is the deliver-and-verify loop, and it is MIT

Behind both lists sits the **iart-ai** family: a hub of eighteen installable packs
([motion-skills](https://github.com/iart-ai/motion-skills), MIT), each a `SKILL.md` plus
`references/`, aimed at one audience — `b-roll-skills` (already read, [01](01-treatment-model-and-visual-lead.md)),
`explainer-video-skills`, `motion-design-skills`, `youtube-video-skills`, `lower-thirds-skills`,
`manim-skills`, `webgl-animation-skills` and so on.

The mechanism they all share, in their words:

> Skills that produce a visual artifact ship a **deliver-and-verify loop** plus a small `scripts/`
> toolkit (freeze a frame, tile a contact sheet, probe the encoded MP4), so the agent confirms its
> own output — web skills render a standalone HTML, video skills render via Remotion or Manim.

That loop is the field converging on the thing AeroEdits already has. But two details are worth
taking literally:

1. **The toolkit is small and named**: freeze a frame · tile a contact sheet · probe the encoded
   MP4. That is a three-tool recipe, not a framework. Our gate already makes a contact sheet
   (`Report::sheet`) — the *probe the encoded MP4* half is the one we do not have, and it is the
   half that catches what a still cannot.
2. **"So the agent confirms its own output"** — the agent, not the app. The field's loop runs inside
   the agent's loop; ours runs inside the app. Ours is stronger in that the user sees the report,
   but theirs is the shape that travels with the user's own agent.

**Do not quote their numbers.** The hub's description says "50 open-source skills … 14 installable
packs"; its README says "55 open-source skills … Eighteen installable packs"; the explainer pack's
README says "a 14-pack open-source collection". Star counts in the captures also differ between
files captured minutes apart (998★ for the hub in one, 984★ in the list). These are marketing
counts, not measurements.

## 4. The field's taste is an *art-director role*, and it lives in the prompt

[iart-ai/motion-design-skills](https://github.com/iart-ai/motion-design-skills) (MIT, 161★) is the
clearest statement of how this field thinks about taste. Nine skills, and the names are the point:

| Skill | What it claims |
|---|---|
| `motion-art-direction` | "Senior creative direction — motion language, tone, pacing, hierarchy, and restraint" |
| `animation-principles` | timing, easing, and the 12 principles for natural-feeling motion |
| `shot-composition` | grids, safe areas, focal hierarchy, 2D/3D camera framing across aspect ratios |
| `color-motion` | palettes, gradients, perceptual interpolation (OKLCH/Lab), grading, render colour management |
| `beat-sync-editing` | cut to the beat — editing rhythm, transitions, retiming, speed ramps |

Compare [05](05-style-as-data-with-a-guide.md) and [08](08-taste-neutral-skills-and-video-mcp.md).
Those two files bracket the question — FrameJam holds taste as **data** beside the brand kit,
Remotion 2.0 removed design instructions so the model brings its own. iart-ai sits **closer to
AeroEdits' pole than to Remotion's**: these packs ship opinionated taste, as an agent-readable
artefact.

That is a third data point on the question finding 08 leaves open — *is the look a property of the
video, or of the agent?* Remotion says the agent. iart-ai says the **pack**. AeroEdits says the
**app** — which is the only one of the three that guarantees every graphic in one video matches
every other graphic in that video. Same instinct in all three; the owner differs. Worth noting
that the field's most-installed packs carry taste, because it weakens the "skills must be
taste-neutral or the model is constrained" argument.

## 5. Safety and licence hygiene worth copying

frankxai's two hard lines are better than anything AeroEdits has written down:

> *"Do not install unsigned ZIP/S3 skill blobs or mass-dump skill collections into a live profile."*
>
> *"Scan any third-party pack with `NVIDIA/SkillSpector` before it reaches a live profile."*

And the method behind their "earned" packs: *"Operators get leverage from **about 5–7 named
workflows**, not bulk dumps."* Their contribution standard is a good template for any registry
AeroEdits might grow: a primary URL, one sentence of distinct value, current maintenance evidence,
licence posture, and a safety caveat.

This maps onto AeroEdits because **we install exactly one file into the user's agent** —
`src-tauri/src/mcp/skill/SKILL.md`, 23 lines, and it contains no rules at all. The rules live in
the app (`rules.md`, `export.md`, `graphics.md`) and are served on demand by the `preset` and
`graphics` tools; the app's own code says the folder holds *"only the pointer to `template`, so an
agent reads the rules once, from the app"*. Older versions installed `export.md` and
`templates/*.md` beside `SKILL.md`, and the app now **deletes them at launch** so no agent reads
them.

Two things follow:

- The "scan before install" discipline applies to **our own** skill file. It is the one file in
  the user's agent profile that we wrote, and it has no validation step of its own.
- AeroEdits already ships the router pattern finding 08 praises in Remotion 2.0 — and goes one
  step further. Remotion's 2.0 skills still carry their own guidance behind a router; ours carry
  **none**, because the app serves them. That is the same instinct (keep the agent's context
  small, keep the knowledge in one place) arrived at from the other end, and it is worth knowing
  we have it before copying anything from Remotion.

## 6. What is left to read

> **Updated 25 Oct 2026, later the same day.** The three references below were re-captured once
> the rate limit reset, and they turned out to be the three most relevant in the field: **all
> three are director layers built on HyperFrames, the runtime AeroEdits already ships.** They are
> now read in [18](18-three-more-director-layers.md). `heygen-com/hyperframes` was captured at the
> same time and is [12](12-hyperframes-is-our-runtime.md). Two of the three giants below are
> still uncaptured.

Three queued references **failed to capture** on the first attempt — GitHub's unauthenticated API
returned `403 rate limit exceeded` for all three. They were the three most relevant to our stack:

| Reference | What it is | Why we wanted it | Now |
|---|---|---|---|
| [geekjourneyx/hyperframes-motion-director](https://github.com/geekjourneyx/hyperframes-motion-director) | a motion director **for HyperFrames** | our pinned runtime, directly | [18](18-three-more-director-layers.md) |
| [gloweaseco-leo/hyperdirector](https://github.com/gloweaseco-leo/hyperdirector) | "hyperdirector" | a rival *director* concept — is it a skill, a service, a prompt pack? | [18](18-three-more-director-layers.md) |
| [Kianzzz/xilo-opus-video](https://github.com/Kianzzz/xilo-opus-video) | a video skill | video generation over MCP, against [08](08-taste-neutral-skills-and-video-mcp.md)'s motionvid and Motion | [18](18-three-more-director-layers.md) |

To retry a failed capture, set `GITHUB_TOKEN` and pass `--refetch` (without it, a cached failure
is skipped):

```bash
python3 docs/minimax-research/scripts/intake.py --refetch \
  https://github.com/geekjourneyx/hyperframes-motion-director \
  https://github.com/gloweaseco-leo/hyperdirector \
  https://github.com/Kianzzz/xilo-opus-video
```

**Three much larger repos were never queued at all**, and they are the most-starred in the field:

| Reference | Stars | Note | Status |
|---|---|---|---|
| [calesthio/OpenMontage](https://github.com/calesthio/OpenMontage) | 71.3k | "12 production pipelines, 100+ tools, 700+ agent skill and production-knowledge files" | **still uncaptured** |
| [heygen-com/hyperframes](https://github.com/heygen-com/hyperframes) | 71.8k | *"the most-starred renderer here took 23.5 minutes on a 15-second brief"* — and it is **HyperFrames** | captured — [12](12-hyperframes-is-our-runtime.md) |
| [hypit-ai/hypit](https://github.com/hypit-ai/hypit) | 25.2k | clones a viral video; swaps face, words, B-roll; 100 variants in one command | **still uncaptured** |

`heygen-com/hyperframes` was the one to capture first, and it is now read
([12](12-hyperframes-is-our-runtime.md)). It is not "a different, much larger HyperFrames
project" than the runtime we pin — **it is the same project**, and we pin its npm package at
0.8.162. The 23.5-minute figure still stands as the thing worth explaining: 23.5 minutes of
rendering for 15 seconds of output is about **94× real time**, and nothing in our own measurements
covers that.

`OpenMontage` and `hypit` are the two still uncaptured, and both are large enough to matter:
OpenMontage claims 700+ agent skill files, and hypit's "100 variants in one command" is a
generation model we have explicitly refused as a core dependency (*"No editability or
determinism"*) — so it is worth reading to be sure we are refusing the right thing.

Two more worth a capture when there is time: `kmilo93sd`'s
[agent-video-editor](https://github.com/kmilo93sd/agent-video-editor) — *"Editing judgment for AI
agents: pacing, cuts, captions, audio, color and thumbnails … plus 128 CC0 sounds, 26 CC0 graphics"*
— because 128 licence-clean sounds is directly the asset class AeroEdits' **SND-3** has to source
by hand, and `garrytan/gstack` (MIT, 134k★), listed by frankxai for "design / motion QA passes".

## Caveats

- **Nothing here was installed, run or benchmarked against AeroEdits.** Every performance number,
  star count and quality judgement is the author's own.
- The zhuyansen test is **one run on one brief on one machine**, with look judged by eye and
  explicitly not machine-scored. "6 of 16 looked like the default" is a judgement, not a metric.
  The finding survives the noise because it is a *negative* result — the baseline passed too — but
  the exact ratio is one author's eye.
- The zhuyansen brief has **no talking head and no transcript**, so it says little about the
  talking-head b-roll case that is AeroEdits' actual subject.
- **CC0 on the lists is not CC0 on the projects.** Both lists are dedicated to the public domain;
  every project they list keeps its own licence, and most of the interesting ones are not CC0.
- Star counts are "dated discovery signals, not rankings" — frankxai says so in its own research
  method, and the counts in our captures already disagree with each other.
- iart-ai's own skill counts disagree between its description, its README and its pack READMEs.
  Do not cite a number from this family.
