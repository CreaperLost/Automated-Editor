# 12 — HyperFrames is our runtime, and it now ships agent guidance

- **Source:** [github.com/heygen-com/hyperframes](https://github.com/heygen-com/hyperframes) ·
  [hyperframes.dev](https://www.hyperframes.dev/) · [hyperframes.heygen.com](https://hyperframes.heygen.com/introduction)
- **Licence:** Apache-2.0 — *"no per-render fees or commercial-use thresholds"* · 71.8k★ · captured 25 Oct 2026
- **Raw capture:** [evidence/2026-10-25/heygen-com-hyperframes.md](evidence/2026-10-25/heygen-com-hyperframes.md)
- **Evidence grade:** vendor claim, read from the source. **Nothing was installed or rendered here.**

This is not another reference in the field. It is the codebase our graphics are rendered by.

## What it is

> Write HTML. Render video. Built for agents.

A composition is an HTML file with `data-start` / `data-duration` / `data-track-index` attributes;
animation comes from GSAP, CSS, Lottie, Three.js, Anime.js, WAAPI or a custom adapter; the renderer
seeks each frame in headless Chrome and encodes with FFmpeg. **"Same input, same frames, same
output"** — deterministic, which is the property AeroEdits' gate leans on.

Verified in this repo: `src-tauri/src/graphics/runtime.rs` pins **HyperFrames 0.8.162** from npm,
alongside **Node 22.23.4**, **Chrome 152.0.7977.30** (the version HyperFrames 0.8.162 itself pins
in its browser manager) and **GSAP 3.15.0** fetched from npm with a SHA-256. A test asserts the
lockfile's integrity hash matches the pin, so the version cannot drift silently.

Our gate already speaks to it: `hyperframes check <stage> --json` (whose report carries `lint`,
`runtime`, `contrast` and `layout` sections) and `hyperframes lint <dir>`; the renderer calls
`hyperframes render`. See [11](11-the-gate-already-reads-pixels.md).

## The part that matters: it now ships seven skills, and one of them is about *look*

HyperFrames publishes **7 skills**, installable as a Claude Code plugin (`claude plugin
marketplace add heygen-com/hyperframes`) or standalone (`npx skills add`).

| Skill | What it holds |
|---|---|
| `/hyperframes` | **Read first** for any request to make, edit, animate or render a video. *"Holds the render contract and the work loop, and points to the references for format, motion, look, captions, whole films and storyboards."* |
| `/hyperframes-cli` | The CLI by job — scaffold, check, **look at frames**, edit the timeline, preview, render, publish, project history, and *"recovering from build or render failures"* |
| `/hyperframes-registry` | **~450 ready-made blocks** installed as editable source — transitions, overlays, titles, kinetic type, captions, charts, maps, 3D, camera moves |
| `/media-use` | Get or make any media for a video — music, sound effects, images, icons, logos, voiceover, transcripts, cutouts, colour grades and looks — and mix placed audio |
| `/figma`, `/slideshow`, `/remotion-to-hyperframes` | On demand |

**Three consequences for AeroEdits.**

**1. There are now two sources of truth about how a graphic should look.** Our app owns the look
([05](05-style-as-data-with-a-guide.md)) and the agent reads our rules. HyperFrames' `/hyperframes`
skill now also carries a **look** reference, plus captions, overlays and motion references — the
exact four areas we decided to own. If a user's agent has the HyperFrames plugin installed, the
agent is being told two different things about the same graphic. We install exactly one file into
the agent's profile; HyperFrames wants its skills in the same place. **This is a real collision that
has to be resolved deliberately, and it is new.** It did not exist when our engine was chosen.

**2. `/hyperframes-registry` is ~450 blocks of taste-correct motion we do not have.** Our graphics
are hand-authored HTML. Transitions, overlays, titles, kinetic type, captions, charts, camera moves
— installed as *editable source*, not black-box components. This is a supply of motion primitives
that would otherwise have to be written one at a time.

**3. `/media-use` is the field's answer to our sound problem.** Our **SND-3** (sound and music
libraries the user extends) is on the roadmap and unsourced. `/media-use` covers "music, sound
effects, images, icons, logos, voiceover… with HeyGen or free local tools, and mix placed audio".
The hosted half is out of reach (our gate allows no network); the local-tools half is not.

## `frame.md` — a design-system format for video

> Every brand has a `design.md`. None of them were written for a camera. `frame.md` is the missing
> translation layer: it takes your web-context design spec and inverts it for the frame — the same
> tokens, the same rules, but rewritten so an AI agent can compose a promo video without guessing
> at scale or reaching for web chrome.
>
> The output is a `DESIGN.md` superset your whole toolchain can read. **Atoms stay sacred.
> Composition stays free. Numbers come from the script.**

Read that against [05](05-style-as-data-with-a-guide.md). FrameJam's style was `style.json` plus a
prose `guide`; ours is `Look` (`src-tauri/src/graphics/look.rs`) plus named tokens. `frame.md` is
the same idea written as a **file format a whole toolchain can read**, and it states the three
rules we have been arriving at separately:

- *Atoms stay sacred* — the look's tokens are not negotiable per graphic. That is our "one look
  for every graphic in a video".
- *Composition stays free* — the agent may compose inside it. That is the line between the app
  owning the look and the agent owning the layout.
- *Numbers come from the script* — which is our rule that a number on screen comes from what was
  said, not from the agent's imagination.

Named looks are published at `hyperframes.dev/design` (Biennale Yellow, BlockFrame, Blue
Professional, Bold Poster, Broadside, Capsule, Cartesian, Cobalt Grid, Coral, Creative Mode).

**Two open questions, and they are cheap to answer.** Is our `Look` a subset of what `frame.md`
expresses — in which case adopting the format is a mapping, not a rewrite? And what licence covers
the published look templates? Apache-2.0 covers the *code*; the README does not say the design
templates are covered by it. Under our repo's rule (MIT, Apache-2.0, GPL-3.0 as a separate
component, OpenRAIL) we would need the text, not the SPDX tag.

## Two more things worth knowing

**`/remotion-to-hyperframes` does not change our Remotion decision.** It brings Remotion (also
Motion Canvas, Lottie, Three.js, canvas) compositions into HyperFrames, *"verified frame by
frame"*. It is tempting to read that as a way around Remotion's revenue-capped licence for
[03](03-shot-recipe-card-format.md)'s shotcraft. It is not: our decision against Remotion is
**maintenance, not licence** — `docs/decisions.md` says *"Two style systems and two failure modes.
A maintenance choice, not a claim that HyperFrames is better."* A converter would not give us one
style system. What it does establish is that the *licence* boundary is crossable — which matters
if we ever want to reuse Remotion-authored motion, not Remotion as our renderer.

**The CLI may now do what our gate does by hand.** `/hyperframes-cli` documents *"check, look at
frames"* — and our gate builds its own contact sheet from its own screenshots. The upstream
maintains that code and the version drift is ours to carry. Worth one experiment: run
`hyperframes look` (or whatever the command is called at 0.8.162) on one of our fixtures and see
whether it beats the contact sheet we wrote. We should not be re-implementing our runtime's own
tooling.

## Caveats

- **Everything here is a vendor claim**, read from a README. The 7 skills, the 450 blocks and
  `frame.md` were not installed, listed or rendered.
- **This is the newest reference we have** and it is the upstream of our own dependency — it moves
  faster than anything else in this folder, and the skills it ships today may not be the skills
  our pinned 0.8.162 was cut against. **We pin 0.8.162; the README describes `main`.**
- `frame.md`'s licence for the published templates is **not stated** in the README. Treat the
  templates as unlicensed until the text says otherwise.
- `/media-use` names HeyGen as one source of media. HeyGen is a hosted service, and our gate
  allows no network — the skill is not usable as-is for us.
- The 71.8k★ count is a discovery signal, not a ranking, and it says nothing about whether the
  skills are good.
