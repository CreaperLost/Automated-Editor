# 13 — The LLM reads the video, it never watches it

- **Source:** [github.com/browser-use/video-use](https://github.com/browser-use/video-use) ·
  [video-use.com](https://video-use.com) (waitlist)
- **Licence:** MIT · 29.8k★ · captured 25 Oct 2026
- **Raw capture:** [evidence/2026-10-25/browser-use-video-use.md](evidence/2026-10-25/browser-use-video-use.md)
- **Evidence grade:** vendor claim, read from the source. Not installed, not benchmarked here.

This is the most architecturally similar reference in the folder, and the second most-starred
project we have seen. It is also the clearest statement of a rule we have been arguing towards
without ever writing down.

## The one idea

> **The LLM never watches the video. It *reads* it** — through two layers that together give it
> everything it needs to cut with word-boundary precision.

**Layer 1 — the transcript, always loaded.** One transcription call per source gives word-level
timestamps, speaker diarization and audio events (`(laughter)`, `(applause)`). Every take packs
into a single ~12 KB `takes_packed.md`:

```
## C0103  (duration: 43.0s, 8 phrases)
  [002.52-005.36] S0 Ninety percent of what a web agent does is completely wasted.
  [006.08-006.74] S0 We fixed this.
```

**Layer 2 — a visual composite, on demand.** `timeline_view` renders a filmstrip + waveform +
word-labels PNG for any time range. It is called *only at decision points* — ambiguous pauses,
retake comparisons, cut-point sanity checks.

> Naive approach: 30,000 frames × 1,500 tokens = **45M tokens of noise**.
> Video Use: **12KB text + a handful of PNGs**.

Their framing of it: *"the same idea as browser-use giving an LLM a structured DOM instead of a
screenshot — but for video."*

## The pipeline, and the loop at the end of it

```
Transcribe ──> Pack ──> LLM Reasons ──> EDL ──> Render ──> Self-Eval
                                                              │
                                                              └─ issue? fix + re-render (max 3)
```

> The self-eval loop runs `timeline_view` on the **rendered output** at every cut boundary —
> catches visual jumps, audio pops, hidden subtitles. **You see the preview only after it passes.**

That is finding [11](11-the-gate-already-reads-pixels.md)'s missing **gate 3** — the gate that
knows about the edit — closed in the most concrete way we have seen it stated. Not by reasoning
about the plan, but by *looking at the rendered cut, at every boundary, and refusing to show it
until it passes*. The loop is bounded at three fixes.

Their five design principles, the last two being the sharp ones:

1. **Text + on-demand visuals.** No frame-dumping. The transcript is the surface.
2. **Audio is primary, visuals follow.** Cuts come from speech boundaries and silence gaps.
3. **Ask → confirm → execute → self-eval → persist.** Never touch the cut without strategy approval.
4. **Zero assumptions about content type.** Look, ask, then edit.
5. **12 hard rules, artistic freedom elsewhere.** Production-correctness is non-negotiable. Taste
   isn't.

Also in the box: cuts filler words and false starts, auto colour grades each segment, **30 ms
audio fades at every cut**, burns subtitles in your style (2-word UPPERCASE chunks by default),
and persists session memory in `project.md`.

## Where we are ahead

**Transcription.** video-use needs an **ElevenLabs API key** and sends the audio to the cloud.
AeroEdits already has both paths — `src-tauri/src/transcript/`: **Parakeet locally** through ONNX
Runtime, and ElevenLabs Scribe for cloud. Our local model choice is directly aimed at their
headline feature: the *Verbatim* Parakeet TDT model "writes stutters, repeats and filler sounds
down, which the filler and retake passes cut" — our filler and retake passes. So our filler
cutting works with the user's footage **never leaving the machine**, and theirs needs a key and a
network. For a product that sells "your footage stays on your machine", that is not a small
difference.

**Token budget.** Their argument is the same one as our **AI-1**: what an agent reads in an edit
should be small. Theirs measures 12 KB of packed text; ours is measured by
`plans/eval/tokens.py` — the tutorial edit ends at **36.8k tokens**, **44.6k** with graphics,
**53.6k** with a short, against a ~100k preference. Same shape, different scale, and ours is
already measured every turn.

## Where we are behind — and where we are better

**We are behind on one thing: the self-eval loop over the rendered cut.** We gate every graphic;
we do not look at the cut. Theirs runs the composite at every boundary and blocks the preview.
This is the concrete, implementable version of the missing gate, and it costs one PNG per
boundary rather than a new subsystem.

**We are better on one thing, and it is our thesis.** video-use generates animation overlays
"via HyperFrames, Remotion, Manim, or PIL — **spawned in parallel sub-agents, one per
animation**". One sub-agent per animation means each one brings its own taste: four graphics from
four sub-agents is four looks. AeroEdits' decision — the app owns the look, the agent proposes
the content, one look for every graphic in a video ([05](05-style-as-data-with-a-guide.md)) — is
the direct answer to that design. We would not want their approach for our graphics, and we can
say why: it is the failure mode AeroEdits exists to avoid.

Their principle 2 — *audio is primary, visuals follow* — is worth flagging as **not ours**. For a
screen-recorded tutorial the screen is the content and the audio is the commentary; cutting on
speech boundaries is the right instinct for a talking head and the wrong one for a screen
recording. Their README claims tutorials, so it is worth seeing where that lands.

## What we would take

1. **The composite at the cut boundary.** `timeline_view` — filmstrip, waveform, word labels, one
   PNG per boundary — is exactly the artefact our gate's contact sheet is not. It answers "did
   the cut land" and "did the subtitle land" in one picture.
2. **The rule count.** *12 hard rules* is a tight, checkable number. Our `rules.md` is 107 lines.
   A rule that cannot be counted cannot be audited.
3. **The bounded fix loop.** Max three fix-and-re-render cycles, then it shows you the preview
   anyway. Bounded loops are honest loops.

## Caveats

- **Vendor claim.** The skill was not installed and no video was edited with it. The token
  arithmetic (45M vs 12 KB) is their comparison, not a measurement against AeroEdits.
- **29.8k★ is a discovery signal**, and this project is actively commercialising: *cloud-hosted
  video-use is coming* (waitlist at video-use.com) and **Browser Use Box** for always-on editing
  from a VPS or Telegram. The open-source part may not stay the whole product.
- **Their EDL and their `timeline_view` are not specified** at the level we would need to adopt
  them — we have the idea, not the format.
- "Works for any content — talking heads, montages, tutorials, travel, interviews" is a claim
  about four very different editing problems solved by one pipeline.
- The ElevenLabs dependency means their pipeline cannot run offline at all. Ours can, entirely.
