# 22 — The closest thing to AeroEdits that is open source (and MIT)

- **Source:** [youtube.com/watch?v=dXz1qkLc9PQ](https://www.youtube.com/watch?v=dXz1qkLc9PQ) — *I
  Replaced 8 Hours of Video Editing with This (Open Source)* · **Sirio** · 21 Oct 2026 · 20:17 ·
  19,036 views · 411 likes
- **Artefacts:**
  [sirioberati/YouTube-AI-Video-Editing-System](https://github.com/sirioberati/YouTube-AI-Video-Editing-System)
  (**MIT**, 59★, 397 files) ·
  [Genjustsu-Open-Source-Workflow](https://github.com/sirioberati/Genjustsu-Open-Source-Workflow)
  (MIT, 1.2k★) ·
  [Emotion-Control-Open-Source-Workflow](https://github.com/sirioberati/Emotion-Control-Open-Source-Workflow)
  (NOASSERTION)
- **Raw captures:** [the video](evidence/2026-10-25/i-replaced-8-hours-of-video-editing-with-this-open-source.md) ·
  [main repo](evidence/2026-10-25/sirioberati-youtube-ai-video-editing-system.md) ·
  [Genjutsu](evidence/2026-10-25/sirioberati-genjustsu-open-source-workflow.md) ·
  [Emotion Control](evidence/2026-10-25/sirioberati-emotion-control-open-source-workflow.md)
- **Evidence grade:** creator claim plus source code we read but **did not run**. The 3 % is his
  number, on his material, with his definition of an error.

This is **Eyeline**: a teleprompter, an editing app, seven per-stage skills, JSON schemas, a
design pack, and an editor handoff — all MIT, all local, no API key. Two Python web apps
(`localhost:5317` teleprompter, `localhost:5320` editor), Chrome for speech recognition and
rendering, FFmpeg for the cut.

## 1. The number nobody else in this folder has

> "it brings the error rate of like the clipping and assembly from like traditional ways of
> clipping your content through AI **down to around 3%**. That's normally **20 to 30%** if you just
> use AI to clip things and ask Codex or Claude to take a video and transcribe it and start
> clipping."

**The first measured error rate for an AI editing pipeline in this whole folder.** And it lands
exactly on the metric our `gaps.md` has no number for: our LNG-4 entry says *"The filler/retake
rules cut a deliberate repeat… restarts, single-word repeats and cut-off words open no candidate
window"* — a qualitative complaint where Sirio has 3 % against 20–30 %.

The reason for the gap is the important part:

> "the reason that all of these things are saved as JSON… is that **the agent can read it exactly
> with no guessing. So that means that the cut is actually coming from predetermined data instead
> of an AI watching like an hour of footage and transcribing it and then you hope that AI is going
> to make the right cut.**"

**The cut is deterministic. The AI is not in the cut loop.** That is AeroEdits' thesis — the app
decides, the agent proposes — said by someone who shipped it and measured the difference.

## 2. The trick: the recorder is the data collector

This is the AeroShoot recorder, MIT, with the data contract written down. And it is the best idea
in the repo:

- The teleprompter follows your voice — **using Chrome's built-in speech recognition, so there is
  nothing extra to install** — and the iPad shows the words, synced over the same local server.
- It records **mic audio separately** from the camera, so the two can be synced later.
- **It marks a retake by itself.** Live in the demo: *"it catches that I repeated the line twice,
  and it marks it as retake, all by itself. I didn't press any buttons."*
- **`take.json`** — *"the diary of our takes"*: every try, when each started and stopped.
- **`editplan.json`** — which take to use for every line.
- **`agents.md`** — a note for Claude *and for the human editor, or your friends* setting up the
  same project.

**And the take rule is trivial and deterministic:**

> "this part is not AI because the rule is very simple, it's **deterministic**. If I say a line
> twice, three times, four times, whatever that is, **it keeps the last one**."

**This is the answer to our LNG-4.** Our problem is a word-level rule misfiring on a *deliberate*
repeat (*"human in the loop. Human in the loop essentially means…"*). Sirio's rule works at **take**
granularity, where a deliberate repeat *inside* one take is not a repeat at all. **Take-level
selection sidesteps word-level repeat detection entirely** — and our roadmap lists the AeroShoot
recorder as *"External"*. This is an MIT implementation of it.

## 3. The second transcription pass — gate 3, in a transcript

> "it uses Whisper, which is also free, to transcribe the **final cut** that the agents put
> together. So it **double checks that it actually makes sense and that every word can be heard.
> It's not like cut in the middle.** And then it finds all the pauses, all the ums, and removes
> everything that does not fit my instructions. And he does so by simply **analyzing the audio
> waves**."

**Transcribe the output, not the input.** This is finding [11](11-the-gate-already-reads-pixels.md)'s
gate 3 — the gate that knows about the *edit* — implemented with a transcript instead of pixels.
It is also the cheapest possible version: you already have the audio.

It exploits the sync: *"since the audio is synced to the camera footage, if you make a cut to the
audio, that cut is also going to happen to the video, too. **Everything is linked together.**"*

**And it does not flatten.** *"It does not just merge and cut everything into one file. **It puts
all the takes on a timeline**, so if something's off, the human editor can still fix it, even if
it's word by word… if you highlight and delete a word, it does the same thing in the timeline."*
That is our proposal model, our word ids and source-linked editing — and it is the same answer as
finding 14's `--plan` with fingerprinted inputs: **the candidate never replaces the source.**

## 4. The board question — finding 01, independently implemented

> "the boards are planned by a skill. And the skill reads the whole final transcript… **line by
> line, and figures out which lines need visuals and what type of visuals**… And **for every line
> that I speak, the skill asks one question, which for me, in this case, is, 'Can you understand
> this just by listening, or do you need extra layers?'**"

**That is finding [01](01-treatment-model-and-visual-lead.md)'s rule, arrived at independently** —
finding 01 came from `iart-ai/b-roll-skills` (MIT). Two implementations, the same question, the
same scope: **one question per line.**

And the difference is instructive. Finding 01's answer is **three treatments** (the speaker · a
graphic beside them · a full-frame graphic). Sirio's wording is *"extra layers"* — **less specific
than ours**, and his board brief is *"simply instructions for the agent to start designing. It's not
the actual thing."* A brief is a plan; ours resolves to a treatment per line.

## 5. `window.seek(seconds)` — the answer to our gate's sampling limit

> "Each HTML board exposes `window.seek(seconds)` so **preview and export use the same
> deterministic state**."

Finding 11 said our gate *"samples three stills per run, so it can prove a graphic is see-through
at its first frame but not that the entrance **eased**. **That is a sampling limit, not a
philosophy**, and it changes what the fix looks like: more frames, or a rule about the timing
tokens themselves."*

**A seekable board is the clean answer.** Any frame, deterministically, and preview and export are
the same state. Our HyperFrames runtime already seeks each frame in headless Chrome
([12](12-hyperframes-is-our-runtime.md)) — so the capability is ours; **the board API is the
pattern we are missing.**

The render step is the same shape as ours, implemented independently:

> "remember that all the boards are still HTML, they are code. So, now what's going to happen is
> that **Chrome is going to open each animation, each board, and it's going to record it frame by
> frame and FFmpeg is going to turn all of it into an MP4**."

HTML → Chrome → frame by frame → FFmpeg → MP4. **That is our pipeline**, in Python, with no
runtime dependency.

## 6. The structure is a mirror of ours

| Eyeline | AeroEdits |
|---|---|
| `agents/{recording,cutting,review,design,animation,editor}.md` | `mcp/skill/{rules,graphics,export}.md` + the `preset` tool |
| **`skills/*/SKILL.md` — seven, one per stage** | one 23-line `SKILL.md` router |
| `schemas/{take,cut-plan,edit-plan,board-timeline}.schema.json` | our typed API and proposal shapes |
| **`config/workflow.json` — workflow stages** | **our presets** |
| `config/preferences.example.json` | our `Look` and style presets |
| **`qa_cut.py`, `qa_edges.py`** | **our gate** |
| `assemble_cut.py` | our export |
| `ingest_sync.py`, `ingest_proxy.py`, `ingest_words.py` | our camera sync, proxies, word ids |
| `templates/board/index.html` + `board-timeline.schema.json` | our graphics and timeline |
| `templates/design/` — 267 boards | our graphic templates |
| `templates/design-brief.md` + `claude-design-prompt.md` | our style |
| `plan.candidate.json` vs `plan.json` | our proposals and checkpoints |
| `AGENTS.md`, `CLAUDE.md` | our `AGENTS.md` |

**Seven skills, one per stage, each a `SKILL.md`** — the router pattern
[12](12-hyperframes-is-our-runtime.md) discussed, done properly, where ours is 23 lines pointing
at `preset`.

## 7. The one thing they have that we don't: placeholder skills the user fills in

> "Everything in `skills/` is a **placeholder**, written for this public repository. Fill in your
> audience, voice, editorial rules, design tokens, animation rules, and examples. Customizing these
> Markdown files is **prompt configuration; it does not train a model**."

**That answers the repo's gap #4 in one sentence**: *"Every creator publishes a skill/prompt as a
file. We ship a `skill.rs`; the user can't author their own."* → Ship placeholder skills the user
fills in. MIT.

## 8. And the one thing they have differently: the design system lives in files

> "Step four is Claude Code that's going to **design every board for my own design system**, which
> I spent a long time putting together. Now, there's more than a hundred boards here that have
> design styles, charts, graphs, prompts, and all the animations. And **the more boards or
> templates that you have, the easier it is to turn the new scripts… into graphics**."

The look is **in the project**, read by the agent: a design brief, a design system, and **267
restyled HTML reference boards, 246 of them animated**. Their consistency comes from *a plan plus a
large hand-built library*.

**Ours comes from tokens.** Same promise — *"every board follows a plan and they have the same
design rules and nothing is left to chance"* — different mechanism. **This is the sharpest
evidence yet that the trade we made is real: you cannot get consistency from a template library
alone without a large hand-made one.** Their 267 boards are the cost of the other route. Our `Look`
is the claim that tokens are cheaper.

What they got right that we don't: the boards are **HTML with the animation timings preserved** —
*"Its animation timing and effects are preserved, while personal media is replaced with neutral
placeholders. **Style the boards yourself before production.**"* That is **a catalogue of motion
primitives in our medium**, MIT-licensed. Finding 12 noted HyperFrames' `/hyperframes-registry`
has ~450 blocks; this has 246 animated boards in HTML.

## 9. The honest limits, which match ours

> "This is an early public release, **not an unattended one-click editor. Cutting decisions need
> human review.** Speech recognition can turn a mispronounced word into a correctly spelled
> transcript, and **waveform silence is not proof that a word is finished**."

**"Waveform silence is not proof that a word is finished"** is finding
[20](20-the-runtimes-author-on-the-record.md)'s Parakeet point, stated by a builder who shipped:
timestamp accuracy is what decides whether pause-capping works.

And its mirror — *a correctly spelled transcript can be wrong about what was said* — is the
counter-argument to our "names and numbers" reason for preferring the Unified model. **Sirio's
defence is not a better first-pass model; it is a second pass over the output.** That is a better
answer than ours, because it is checked against the thing that ships.

## 10. The role shift, and the strongest challenge to us in this folder

> "your job is spotting the boards if they **land too early**, if the cuts feels off, if the sound
> sounds flat. All these minor little details that are extremely important for creating quality
> video."

*"Land too early"* is the visual lead rule ([01](01-treatment-model-and-visual-lead.md)) as a **QA
item** — done by a human, in the last 10 % of the work.

And the conclusion:

> "I did **not** wait for an app to add a feature or someone to release an MCP cuz everything that
> I need was already available right here, right in front of me. And what made the difference was
> **how I architected this entire thing**."
>
> "do not just watch tutorials on how to use someone else's software. **Start building your own
> solution.**"

The argument: the app is a component; the **system** is what you assemble. AeroEdits' counter is
the one finding [17](17-what-the-seven-agree-on.md) made: **we keep the look in the app so every
graphic in one video matches every other**, and no arrangement of files reproduces that guarantee.
Sirio's own wording concedes the shape of his answer — consistency by **plan and library**, ours by
**tokens**.

## Numbers

| | |
|---|---|
| **3 %** | clipping and assembly error rate, deterministic pipeline |
| **20–30 %** | the same, *"ask Codex or Claude to take a video and transcribe it and start clipping"* |
| 1.5 h | to produce 15 minutes of motion graphics |
| **~40 ¢** | AI tokens for that (Claude plan $200/mo ≈ 27 ¢/hour) |
| $750–$900 | what a human editor charges for the same, in 6–8 hours |
| ~20 min | to render a 15-minute video to 2K on an M3 |
| 5–6 h → 1.5 h | an editor's time per video; one a day → three |
| **10 %** | the human input: *"the last 10% is finding the mistakes, fixing them, and directing AI"* |
| 397 files | in the repo |

## What we would take

1. **The take-level rule.** Keep the last take. It sidesteps word-level repeat detection entirely,
   which is exactly where our LNG-4 rule misfires — and it is cheap to test against our own
   material.
2. **The second transcription pass over the assembled cut.** Transcribe the *output*, check every
   word is audible and nothing is cut mid-word, find the pauses by waveform. **That is gate 3 in
   a transcript, and we already have the audio.**
3. **The recorder as a data collector.** `take.json` is a data contract an editor — ours or anyone
   else's — can read. Our roadmap calls the AeroShoot recorder "External"; this is MIT with the
   contract written down.
4. **`window.seek(seconds)`** on our boards, so preview and export are the same deterministic state
   and our gate can ask for *any* frame, not three.
5. **Placeholder skills the user fills in.** The repo's gap #4, answered.
6. **The 267-board library as a motion catalogue** — HTML, timings preserved, MIT, our medium.

## The companion repos

- **Genjustsu** (MIT, 1.2k★): replace video subjects using depth maps, SAM 3 masks and Seedance
  via the Enhancor API. *"Draft first; approve before 1080p. The final export restores the entire
  original source audio, not generated audio."* Two patterns worth taking: **draft-then-approve
  before the expensive render**, and **the original audio is never replaced.**
- **Emotion Control** (**NOASSERTION → refused** under our rule): a clay-mask facial-performance
  workflow — *"A performance starts with a face. Keep the expression. Change the appearance."*
  *"This repository builds the performance masks; it does not bundle a video-generation model or
  make paid API calls."* Both are generative, which we refuse as a core dependency, and both need
  paid hosted APIs. They are adjacent to our `FX-1` / `LAY-4` work, not alternatives to it.

## Caveats

- **Nothing was run.** 397 files of Python we did not execute.
- **"3 %" is not defined.** Errors in what — clip boundaries, word alignment, both? The repo has
  `qa_cut.py` and `qa_edges.py`, so there is a definition in the code we did not read. **The
  number is the most valuable thing here and the least pinned down.**
- **His "human editor" is a role he employed.** The business case is his arrangement, not a market
  rate; $750–$900 for 6–8 hours is what *he* was charged.
- **MIT, but read the grant:** *"code and newly authored templates are MIT. TEIN branding is
  excluded from the MIT grant… Supplied demonstration images and GIFs are documented separately in
  media credits. Third-party software and model weights retain their own licenses."* The boards
  are newly authored templates (MIT); the demo GIFs and images are separately credited; the model
  weights are not ours to redistribute.
- **Whisper, not Parakeet.** His ASR is local Whisper, so his word timings are a different accuracy
  regime from ours — and finding 20's Parakeet question does not transfer directly.
- **Chrome's built-in speech recognition** drives the teleprompter's voice-following, which may send
  audio to the browser vendor. His own privacy note says so.
- **He is selling a community and a waitlist** (tein.ai, PublicAI on Skool) and claims *"last year
  we made about $6 million"*. The video is a funnel as well as a tutorial.
- **The two companion repos are generative** and need paid APIs; they sit beside our work, not
  against it.

## What this implies outside this folder — **not applied**

This folder is the only place anything was changed. For the record, here is what a maintainer would
want to change elsewhere — **none of it has been applied**:

- **`gaps.md` → §11 (Languages and transcription):** our LNG-4 entry describes the filler/retake
  rule misfiring on a deliberate repeat but has **no error rate**. A take-level rule (*"keep the last
  take"*) sidesteps word-level repeat detection, and there is a **3 % vs 20–30 %** measurement to
  aim at.
- **`gaps.md` → a new entry for the missing output pass:** we check graphics, but we do not
  re-transcribe the assembled cut to confirm every word is audible and nothing is cut mid-word.
  That is gate 3 in a transcript, and it is the cheapest gate in this file.
- **`research/competitors.md`:** Eyeline (MIT, 397 files) is the closest thing to us that is open
  source, and it is a **structural mirror** — seven per-stage skills, JSON schemas, a
  `config/workflow.json` of workflow stages, `qa_cut.py`/`qa_edges.py`, and 267 HTML reference
  boards. It should be in the competitor table, not only in this folder.
- **`research/creator-workflows.md` → "Tricks worth stealing":** two rows — *the agent diffs your
  manual tweaks and writes itself a skill* (already noted from Matt Loui, and here again as
  "placeholder skills the user fills in"); *transcribe the output, not the input*.
- **`roadmap.md` → the AeroShoot recorder**, listed as *External*, now has an MIT implementation
  with a written data contract (`take.json`, `editplan.json`, `agents.md`).
