# 17 — What the seven agree on, and the one thing they say we have not built

- **Sources:** the seven in this batch — [framejam](https://github.com/moritzkremb/framejam)
  ([05](05-style-as-data-with-a-guide.md)) ·
  [heygen-com/hyperframes](https://github.com/heygen-com/hyperframes)
  ([12](12-hyperframes-is-our-runtime.md)) ·
  [browser-use/video-use](https://github.com/browser-use/video-use)
  ([13](13-the-llm-reads-the-video.md)) ·
  [edigeh/broll-mcp](https://github.com/edigeh/broll-mcp)
  ([15](15-broll-provenance-before-the-clip.md)) ·
  [kajisho5/ffmpeg-skill](https://github.com/kajisho5/ffmpeg-skill)
  ([14](14-ffmpeg-skill-contract-and-evals.md)) ·
  [kitcut-hq/kitcut](https://github.com/kitcut-hq/kitcut)
  ([16](16-kitcut-priced-choices-checked-renders.md)) ·
  [Vincentwei1021/video-shotcraft](https://github.com/Vincentwei1021/video-shotcraft)
  ([03](03-shot-recipe-card-format.md))
- **Evidence grade:** a reading across creator claims. Where this file says "we agree", it means
  our own source and decisions say so — not that anyone has tested it.

Two of the seven were already in the folder as findings 03 and 05, so they are read here rather
than rewritten. The other five are new (12–16).

## Six things they all say

### 1. The agent must not look at the video

video-use: *"The LLM never watches the video. It reads it"* — 12 KB of packed transcript plus a
composite PNG on demand. ffmpeg-skill: `probe` first, `look.py` for the picture, contact sheet
only when the picture changed. broll-mcp: item metadata can exceed 1 MB, so it returns *one
keyframe per scene* instead. kitcut: a before/after sheet. framejam: you review the render, not
the source.

**We agree.** Our AI-1 budget is measured every turn; the transcript is the surface; the
`storyboard` tool returns an image on demand.

### 2. The check must read the output, not the plan

video-use: *"Self-evaluates the rendered output at every cut boundary … You see the preview only
after it passes."* kitcut: *"The last check reads the finished render, not the plan."* framejam:
*"you always review the render."* ffmpeg-skill: every write is probed, checked and looked at.
broll-mcp: ffprobe confirms ±0.1 s and the decode shows zero errors.

Five projects, five wordings, one rule. **We agree with the rule and have only half of it** —
our gate checks the graphic, never the cut ([11](11-the-gate-already-reads-pixels.md)).

### 3. The engine does not decide; something in front of it does

ffmpeg-skill is the clearest: *"this repo is the **hands**: it cuts, measures and exports files…
it does not decide what to cut, whether a deliverable is approvable… those are a **brain**'s job,
sitting in front of this engine, not inside it."* video-use: *"Never touch the cut without strategy
approval."* AeroEdits' Q-VERIFY: the agent shows its results, the user reads and adjusts.

**We agree, and we are the strictest.** But note the fine line, because we will otherwise
misread the next batch: video-use and kitcut both run **fix-and-re-render loops**. Ours forbids
**autonomous** loops. A bounded self-check that ends at *"here is the preview"* is not autonomy,
and it is compatible with Q-VERIFY as written. The distinction is who decides to stop, and
whether anything is published without the user.

### 4. Rights are recorded before the asset is used

broll-mcp: *"Provenance is written before the clip, so a clip never exists without its record"*
and *"nothing is guessed"*. video-shotcraft: code Apache-2.0, audio under its own licences with an
ATTRIBUTION.md. framejam: the style's `guide` line doubles as a licence and ethics guardrail.
AeroEdits' Q-SOUND: *"every track carries its licence, and a track that requires credit, forbids
monetisation or has an unknown licence is refused."*

**We agree — for music.** We have no provenance format for *footage*, and our shorts already allow
"B-roll, including outside video files".

### 5. The contract must come from the code, not sit beside it

ffmpeg-skill's SPEC: the `input_schema` is derived at run time from the argparse parser, and CI
fails if it drifts — *"There is no separate `input_schema` file to forget to update."* video-use:
12 hard rules, counted. AeroEdits: the `preset` tool returns the rules *from the app*, so they
cannot drift from the code that enforces them.

**We are right about the principle and wrong about the mechanism.** Our 38 MCP tool schemas are
hand-written.

### 6. Correctness is free; the look is the whole difference

finding [10](10-the-rest-of-the-field.md)'s benchmark: sixteen skills and a no-skill baseline all
made the same film and **all seventeen passed every mechanical check**. Only the look separated
them — six of sixteen looked like no skill at all. video-use puts it as a rule: *"12 hard rules,
artistic freedom elsewhere. Production-correctness is non-negotiable. Taste isn't."*

**This is our thesis, confirmed from outside, by strangers.** It is also the reason the rest of
this file matters less than one decision.

## Where the seven leave us alone — and it is worth something

**Nobody else in this folder holds the look in the app.**

| | where the look lives |
|---|---|
| video-use | in the **prompt** — one sub-agent per animation, each bringing its own taste |
| framejam | in a **preset file** (`style.json` + a prose `guide`) |
| hyperframes | in a **design file** (`frame.md`) and a skill reference |
| video-shotcraft | in a **recipe card** — a document an agent reads |
| iart-ai | in the **prompt pack** |
| **AeroEdits** | in the **app**, beside the brand kit, applied to every graphic |

That is not a detail. It is the only design in the folder that guarantees every graphic in one
video matches every other graphic in that video — which is exactly the failure video-use's
parallel sub-agents would produce, and exactly what finding 10's benchmark says is the only thing
that matters.

And it is getting more valuable, not less: finding [12](12-hyperframes-is-our-runtime.md) shows
our own *runtime* now shipping a look reference and a design format. The field is moving into the
look business from the prompt side. We are already on the app side. **The collision is new and it
has to be resolved deliberately** — but the direction we chose is the one the field is converging
towards from further away.

Two smaller places we are alone: **local-first with no cloud** (ffmpeg-skill and broll-mcp need no
key; we transcribe locally with Parakeet; video-use *requires* an ElevenLabs key and uploads the
audio), and **the app owns the timeline** (video-use and kitcut both say "no timeline").

## What the seven say we have not built

The most repeated specific claim in the new references is not about graphics at all. It is about
**the cut**:

- video-use: a composite at every cut boundary, and the preview is withheld until it passes.
- kitcut: a caption card on the speaker's face is **refused**, and the guard reads the render after
  every vertical cut.
- kitcut: a short whose hook arrives late is **refused, not rendered**.
- kitcut: PII in a screen recording is found and blurred in the finished cut.
- framejam: every comment points at a frame of the rendered output.

We have gates 1 and 2 (files, and the graphic alone) and **not gate 3 (the edit)**. And we have
the parts: our transcript word ids, our timeline, our render, our YuNet face tracks, our `faces`
tool. What is missing is the **comparison**.

**And the artefact for it mostly exists already.** Our `storyboard` tool
(`src-tauri/src/mcp/storyboard.rs`) is *"One small PNG contact sheet of the whole edit … a tile
per scene … Each tile is a finished frame, drawn as export draws it, over a caption strip: its
time, what the frame shows (screen, camera, split), the zoom, a graphic, a chapter, and what is
said"*. It is already described as the thing to *"check an edit after your passes and before
export"*. That is video-use's composite, already built, already aimed at the right thing.

**What it does not yet do is mark a cut.** Its scenes start where the picture changes — the
camera filling the frame, a zoom, a graphic, a chapter, or a silence of three seconds or more
between words. In a single-camera screen recording a cut usually lands *inside* one of those
three-second gaps, so it is close to a tile boundary without being one. There is no waveform
either.

So the version of this finding worth acting on is narrow: **add "where a cut starts" as a
split reason for the storyboard, and draw the waveform.** That is two changes to a tool that
already renders the finished edit for the price of one image, and it would make our gate-3 check
the same object that video-use, kitcut and framejam each built a different way to get at.

## Two licence facts to carry forward

- **kitcut has no licence file at all.** Under our rule it is refused for reuse — an idea to
  reimplement, never text to copy.
- **heygen-com/hyperframes is Apache-2.0**, with "no per-render fees or commercial-use
  thresholds" — and it is the upstream of the runtime we already ship.

## Caveats

- **Everything in this file is a reading of creator claims.** Five of the seven were not run. The
  numbers in them are theirs, on their corpora, with their graders.
- **"They all say" is a pattern, not a proof.** Six projects agreeing on a rule does not make the
  rule correct — it makes it *conventional*. Our own gate exists because conventional checks miss
  what users notice.
- Two of the seven are already in the folder, so this is not an independent sample of seven.
- The synthesis is deliberately about the **cut** because that is where five of the new references
  point. If the next wave is about graphics, this file's conclusion moves with it.
