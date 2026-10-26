# 01 — Three treatments per line, and visual lead frames

- **Source:** [github.com/iart-ai/b-roll-skills](https://github.com/iart-ai/b-roll-skills)
- **Licence:** MIT · captured 25 Oct 2026 · last push 2026-10-17 · 1★ at capture
- **Raw capture:** [evidence/2026-10-25/iart-ai-b-roll-skills.md](evidence/2026-10-25/iart-ai-b-roll-skills.md)
- **Evidence grade:** vendor claim, read from the source. Not tested in AeroEdits.

## What it is

An agent skill that adds motion graphics to a talking-head video. It transcribes the recording,
then assigns **every line** one of three treatments, draws the graphics from the transcript, and
renders the MP4 with the original sound. The recording itself is never cut.

The three treatments:

| Treatment | What the viewer sees | Used for |
|---|---|---|
| **Speaker** | the recording alone | the opening, opinions and stories, the call to action, the closing line |
| **Overlay** | the speaker, with a small graphic beside them: a key word, a number counting up, a short list | one concrete thing worth seeing while the speaker is still worth watching |
| **Cover** | a full-frame graphic while the voice carries on: a chart, a comparison, a diagram, a sequence of steps | lines that are better shown than said by a face |

The rule that makes it usable rather than decorative: *graphics are drawn from the transcript — the
speaker's own numbers, names and words, never invented ones*; *overlays stay off the face*; and
*the speaker comes back between full-frame graphics so the video never turns into a slideshow with
a voice-over*.

## The part worth taking: visual lead frames

The agent chooses timing per visual beat, and the rule is specific:

- **structure** lands about **six frames early**,
- **key numbers and final values** land **on their spoken word**,
- **withheld conclusions** land **at their reveal**.

Six frames is 240 ms at 25 fps or 200 ms at 30 fps. One switch (`VISUAL_LEAD_FRAMES = 0`) turns it
into exact sync. Audio and captions keep their original timing regardless.

This is the first reference we found that distinguishes *structure* from *data* when anchoring a
graphic to speech. AeroEdits anchors graphics to a word id (`graphic_add` at a word), which is
correct for a number but wrong for the shape of a sentence that sets up a point — that wants to
arrive first and be understood by the time the word lands.

## And the two clocks

The composition keeps **an advanced structure clock** (`s.now`) and **the unchanged playback clock**
(`s.audioNow`). Structure may lead; audio never moves. That is the same problem AeroEdits solves
with its recording-time ↔ edited-time clocks, solved one layer down, in the composition.

## Why it matters here

AeroEdits already has the primitives this needs: word ids with their gaps, YuNet face boxes cached
per recording, graphic durations, and the three graphic modes decided on 22 Oct (full, split beside
the face, over the face). The missing piece is a **rubric** that decides the treatment per beat
without a model, plus the lead rule that decides *when*.

That rubric is worth having as a plain table, because it is checkable: a beat that has a number in
it and the viewer would benefit from seeing the number is an Overlay; a beat that is a comparison
is a Cover; a beat that is an opinion is Speaker. It also draws a line our current model does not —
**Overlay** is only for "one concrete thing", so a beat cannot accumulate three of them.

The lead rule is the part that would change output quality on its own. Right now a graphic that
starts on a word arrives *at* the word; a graphic that shows the shape of a sentence wants to
arrive slightly before and be understood by the word. Six frames is a number, not a feeling, which
means it can go into a gate, a template or a style token rather than into a prompt.

## Caveats

- One star at capture and pushed two weeks before: a young project. The method is sound; the
  implementation is unproven at scale.
- It renders with Playwright + Chrome and loads fonts from Google Fonts, so **it needs network at
  render time**. AeroEdits' gate allows no network, and its fonts are local files in the style —
  adopting the method means keeping our font path, not theirs.
- It is built for a **single talking-head recording**, not a screen-recorded tutorial with
  telemetry. The treatment rubric survives that change; the asset model does not.
- "Built and tested with Claude Opus 5.5" — a vendor claim, and the pack says other models are
  untested.
