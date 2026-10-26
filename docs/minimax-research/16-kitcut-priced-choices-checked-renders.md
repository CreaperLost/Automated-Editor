# 16 — Every choice priced before the render, every render checked after

- **Source:** [github.com/kitcut-hq/kitcut](https://github.com/kitcut-hq/kitcut)
- **Licence:** **none found** — no SPDX licence, no `LICENSE` file in the tree · 41★ · last push
  2026-10-25 · captured 25 Oct 2026
- **Raw capture:** [evidence/2026-10-25/kitcut-hq-kitcut.md](evidence/2026-10-25/kitcut-hq-kitcut.md)
- **Evidence grade:** vendor claim, read from the source. Windows-only, so not runnable here.

"Raw footage in. Finished video out. On your own machine." You say what you want in plain words,
Claude does the editing. Nothing is uploaded until you say *"publish"*.

## The licence is the first fact

**There is no licence file and no SPDX licence.** Under our repo rule (MIT, Apache-2.0, GPL-3.0 as
a separate component, OpenRAIL) kitcut is **refused for reuse**. We cannot copy its code, its
prompts or its text.

That does not make it useless — it makes it a **reading**, not a source. Everything below is
something we would have to write ourselves, which is worth knowing before anyone starts drawing
from it.

It is also Windows-only (*"Not yet: Colour grading. Mac."*), so nothing could have been run here
in any case.

## The principle, stated in one sentence

> **Every choice is priced before anything renders, and every render is checked afterwards against
> what was asked for. That applies to all of it.**

Both halves are things we do not do.

## What we would take

### 1. The caption guard

> **Never lands on a face** or across the video's own lower third.
>
> A caption card sitting on the speaker's face is refused. **The guard reads the render and runs
> after every vertical cut.**

This is finding [11](11-the-gate-already-reads-pixels.md)'s missing **gate 3**, in production, on
the feature we are worst at. We already have **YuNet face tracks** (`src-tauri/src/face/`) and a
`faces` MCP tool — the machine is there. The gate does not use it. kitcut's rule is one sentence:
*read the render, and if a caption is on a face, refuse it.*

### 2. The hook gate

> **Opens on the hook** — a clip whose point arrives late is **refused, not rendered**.
>
> Hook gate: A clip whose hook arrives late is refused rather than rendered. It had shipped twice.

Our `shorts` tool proposes short picks and our **SHT-5** builds shorts from a take. We have no
rule that a short must *earn its first second*. This is a cheap, checkable gate: measure where the
clip's point arrives; if it is late, refuse the pick rather than render it.

### 3. Shortlist with reasons, before any cut

> **Finds the moments** in a long video, **shortlisted with reasons before a cut**.
>
> Picking the moments became its own stage: candidates shortlisted with reasoning, quotes resolved
> and hook timing **priced BEFORE anything is cut**. Two rejected picks would have died on paper.

We have a `shorts` tool that proposes picks. kitcut separates *finding and pricing candidates*
from *cutting*, so a bad candidate dies before it costs a render. Ours proposes and renders in one
step, which is the thing they explicitly restructured to avoid.

### 4. PII blurring that follows the content

> **Finds and blurs** card numbers, CVVs, IBANs, phones, emails and addresses, **following each
> one as it scrolls**.
>
> `film-redact` — when the secrets ARE the film, redact the finished cut instead of mapping back
> through it.

We are a screen-recording editor. A tutorial that shows a card number, a CVV, an IBAN or a
customer's email is a video we helped ruin, and nothing in AeroEdits touches it today. Note the
method: *"redact the finished cut instead of mapping back through it"* — you cannot reliably map
a secret's coordinates back through a timeline, but you can find it in the pixels that will ship.

### 5. "The last check reads the finished render, not the plan"

Stated as a requirement, for the silent-screen-recording path. This is the same sentence as
video-use's self-eval ([13](13-the-llm-reads-the-video.md)) and the same conclusion as our own
finding 11 — arrived at independently by three projects.

### 6. The handover, and saying what cannot travel

> **Give the edit away, not just the file**: the cut exports as an OpenTimelineIO timeline, an EDL
> or FCP7 XML that **DaVinci Resolve opens in its free edition** — two video tracks, the audio,
> the media paths, plus an SRT of the words.
>
> **The reasoning travels with it**: every removal, name label and end card arrives as a marker
> carrying the note that explains it.
>
> **It says what it cannot carry** before you promise anyone an editable file — the composite, the
> crop and the caption look are pixels, and pixels stay in the render.

We export a video. kitcut exports an *edit* — and states in advance which parts of it are pixels
and will not survive. Our **NOTE-1** anchored-note type is already exactly "a marker carrying the
note that explains it", and we have an export path that could carry them.

### 7. The one-line reason to measure off the real thing

> Caption position measured off the channel's published shorts instead of eyeballed — **a still
> frame cannot tell a caption box from a black turtleneck.**

That is a genuinely sharp observation: a still frame of a channel's video cannot tell you whether
that black rectangle is their caption box or their shirt. So you must measure off the published
video, not a frame you picked.

## Two things to be careful about

**1. Its multicam scoring is a method we should copy, even though the feature is not ours.**
*"Scores itself against professionally edited videos, frame by frame — and will tell you whether it
can reproduce a given channel's style."* Its update history gives the numbers: *"rebuilding
somebody else's finished film and scoring the re-cut frame by frame. Six films, stage 1 exact on
all six"*, *"held-out score 49.5% → 63.9%, and it needed no tuning"*, and *"Stage 2 chooses the cut
from the soundtrack alone — 73–87% of the timeline on the same camera as the human editor."*

That is **EVAL-1 with a human ground truth**: rebuild a professionally edited film and score the
re-cut against it. We measure our agent's token cost and routing; they measure whether the output
matches what a professional actually shipped. Both are needed, and theirs is the one that says
whether the edit is *good*.

**2. Its "price before rendering" is about *time*, not money.** kitcut's "Every stage timed, so
the next film can be priced before it starts" and "Every choice is priced before anything renders"
mean **render minutes**. We price agent tokens (COST-1..3). Both are costs, but the one we lack
is the user's: *"every decision is remembered, so any video can be changed later in one
sentence"* and *"Change one thing later in one sentence — only that re-renders"* — a user-facing
cost of changing your mind, which we never state.

Its memory model is close to ours and worth restating because it is complete: *"Every render has a
record: what is on it, which setting controls it, where it went, and a journal of why"* plus
*"Stale renders flagged when a decision changed after them."* Ours has checkpoints and stale
proposals. Theirs has the same two ideas and a render history.

## The parts we would refuse

- **Windows-only**, and no macOS yet. We are a Mac app.
- **"Claude does the editing"** with a `project.json` per video and a journal — the project.json
  shape is ours, but they hand the decisions to the agent wholesale. We decided the agent
  *proposes* and the app decides, and our **Q-VERIFY** keeps a human in the loop.
- **"Nothing is uploaded until you say publish"** is right, but **YouTube publishing** and a
  **premium dub voice** are account-gated, and they are the only part that leaves the machine.
- **Auto colour grading** is not a feature we would take either way — our **Q-SCOPE** says colour
  is not now.

## Caveats

- **No licence.** Read everything above as an idea to reimplement, never as code or text to copy.
  That constraint is absolute and it is the first thing to check on any future capture of this
  project.
- **Windows-only, so unrunnable here.** Every claim is a README claim.
- **41★ and pushed the day of capture.** Early. The update history shows a maintainer shipping
  features and finding their own bugs within days of each other — which is a good sign for the
  project's pace and a reason to distrust any single number in it.
- **The multicam percentages are their own, on their own corpus**, with a corpus they chose. The
  method is the transferable part, not the numbers.
- Its "no timeline" framing (the description says *"Local, no timeline"*) means the user never
  sees a timeline. We are a timeline app. That is a different product, not a better one.
