# 02 — Model-free graphics checks and rhythm thresholds

- **Source:** [github.com/iart-ai/b-roll-skills](https://github.com/iart-ai/b-roll-skills)
- **Licence:** MIT · captured 25 Oct 2026
- **Raw capture:** [evidence/2026-10-25/iart-ai-b-roll-skills.md](evidence/2026-10-25/iart-ai-b-roll-skills.md)
- **Evidence grade:** vendor claim, read from the source. Not tested in AeroEdits.

## What it is

`render.mjs --check` reads **the plan, the transcript and the pixels of every frame**, and reports
what it finds. This is the part of the reference that matters most to AeroEdits, because it is
exactly the "check the output, not the plan" principle applied to graphics rather than to cuts.

## The checks it makes

**Compositional:**

- every full-frame graphic **hides the speaker completely** once it is in;
- **no overlay reaches into the box around the speaker's face**;
- original speech anchors **start and end in gaps between words** (graphics may lead them);
- **nothing is drawn outside a planned scene**.

**Rhythm (warnings, not errors):**

- more than about **8 seconds of only the speaker**;
- a full-frame graphic **shorter than 3** or **longer than 8 seconds**;
- **two of them back to back**;
- **not opening or closing on the speaker**.

## Why it matters here

AeroEdits' graphics gate checks a **file** (lint, runtime errors, layout overflow, contrast) and
then renders the graphic and measures its **pixels** — coverage, timing, cut behaviour, whether
`data-overlay` matches the frame, whether a variable changes the picture
(`src-tauri/src/graphics/gate.rs`, see [11](11-the-gate-already-reads-pixels.md)). What it checks
nothing about is whether the graphic is **well placed in the edit** — which is exactly the class of
defect that a tutorial viewer notices and that no amount of linting will catch.

And the pieces to do it are already in the repo:

| The check needs | AeroEdits has |
|---|---|
| the box around the speaker's face | YuNet face tracks, cached per recording (`graphics/`-adjacent face module) |
| gaps between words | the transcript's word ids |
| a planned scene | graphics anchored to word ids, and planned beats |
| durations and adjacency | the timeline |

So "no overlay reaches into the face box" is a **model-free** check with everything it needs
already present — it compares a rectangle against a rectangle. The rhythm warnings are equally
cheap: they are durations and a count of consecutive Covers.

This also fixes a blind spot the project has already written down. `docs/gaps.md` records that our
review tools are stills and that the agent "cannot see motion or hear". The pixel checks above are
not motion review, but they are the first class of finding that reads the *rendered* output rather
than the source, and they need no model at all.

## The rhythm thresholds are a taste decision, and a cheap one

The numbers (8 s of speaker, 3–8 s for a full-frame graphic, no two in a row, never open or close
on the speaker) are one author's taste, not a law. Their value is that they are **written down and
measurable**, so they can be argued with and changed — unlike "keep it minimal and subtle", which
is the instruction our research already found repeated across creators.

Treat them as a starting point to be tuned on our own videos, not as a spec to copy. The
mechanism — thresholds live in the style, the gate reads them — is the transferable part.

## Caveats

- The thresholds are calibrated for **talking-head video at their pacing**. AeroEdits' milestone is
  a long-form screen-recorded tutorial, where a full-screen graphic over a screen recording has a
  different natural rhythm, and where "8 seconds of only the speaker" may be completely wrong.
- "no overlay reaches the face box" assumes a **face in a known box**. AeroEdits has face tracks
  for the camera, but a graphic over the *screen* layer has no face box — the equivalent check
  there is "does the graphic cover the UI target the narrator is talking about", which is a harder
  question and may need the agent.
- Their check reads every frame of the render. Ours renders in the background at export frame rate
  (`graphic_render`); a per-frame pass over the export is affordable once, not per preview.
