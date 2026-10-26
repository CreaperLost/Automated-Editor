# 24 — The vendor's own numbers, and three defect classes only a human caught

- **Source:** [youtu.be/kOoC3yhUyDQ](https://youtu.be/kOoC3yhUyDQ) — *Did AI Change Editing Forever?
  (GPT-6 Astra + DaVinci Resolve)* · **Higgsfield AI** · presenter **Adil** · 2 Oct 2026 · 11:36 ·
  171,327 views · 1,887 likes
- **Artefacts:** [Higgsfield MCP for editing](https://higgsfield.ai/s/astra-for-editing-higgsfieldai-KDnryF)
  · [Editing Skill + SFX Library](https://higgsfield.ai/s/astra-for-editing-higgsfieldai-XqLQyi)
  (both published free in the description) · Discord, X, Instagram
- **Raw capture:** [evidence/2026-10-25/did-ai-change-editing-forever-gpt-6-astra-davinci-resolve.md](evidence/2026-10-25/did-ai-change-editing-forever-gpt-6-astra-davinci-resolve.md)
- **Evidence grade:** vendor claim. 301 ASR snippets; the brand names are mangled (*"Hixel"* and
  *"Higgsfield"*, *"Astro"* for Astra, *"Chad GPT"* for ChatGPT, *"Claude"* for GPT-6), so every
  quote below is checked against the caption text. **Nothing was run here.**

> **This reference is already in the repo.** The full transcript was captured in the earlier wave
> ([`research/evidence/transcripts-wave2-2026-10-21.md` §11](../research/evidence/transcripts-wave2-2026-10-21.md))
> and the product is already written up in
> [`research/competitors.md`](../research/competitors.md) (the *Higgsfield and After Effects*
> section). **Nothing in this file corrects that write-up.** What this wave adds is the part the
> earlier note did not carry: **the presenter reports his own numbers**, and the numbers name
> three defect classes that our gate cannot see at all.
>
> **The artefact behind the video is now covered separately.**
> [Finding 27](27-the-personal-skill-higgsfield-publishes.md) is the *Editing Skill + SFX Library*
> the description points at — which turned out to be one editor's personal process, in Russian,
> with no sounds in it and no licence on it.

## 1. The metric: two clocks, not one

> "I want to see how much editing work I can hand over. **I'm tracking two things. How long the
> whole process takes, and how much time I actually spend on prompts, reviews, and final tweaks.**"

**That is a better metric than wall-clock, and it is the metric the product is judged on.** Finding
[21](21-agent-writes-the-project-files.md) reached the same conclusion from Matt Loui's side: *"The
number that matters for us is the ten minutes. Three hours is Claude's; ten minutes is the human's.
The product is not the speed of the edit — it is how little of the human's attention the edit
costs."*

Two independent creators, one month apart, measuring the same thing.

**We have no such metric.** Nothing in our repo records how much of the user's attention an edit
costs. We measure the gate's pass rate, the render's length and the transcript's word error rate.
The number a customer would feel — *"how long was I in this?"* — we do not measure at all.

## 2. The numbers

| | |
|---|---|
| **49 s → 37 s** | the commercial, after **one manual pass** trimming the gaps between reactions — 12 s of air removed, ~24 % of the runtime |
| **30 min active / ~2 h total** | the commercial, *"including generation time and rendering"* |
| **20 min hands-on / ~2 h total** | the YouTube tutorial rebuilt from raw files, *"my hands-on time so far comes to about 20 minutes"* |
| **3 days** | *"The original episode took 3 days to edit"* |
| **~80 %** | *"I'd say Astra got about 80% of it right"* |
| **10 or 20** | short clips from one timeline: *"Ask for 10, ask for 20, then just review the full batch"* |
| **60 s** | the Oneiric trailer, cut from a film with *"over 2 million views"* |

**The human share is between a sixth and a quarter of wall-clock.** For comparison, Matt Loui's
workflow (finding 21) was **about 10 minutes of active time in 3 hours — under 6 %**. His host was
CapCut's project files; Adil's hosts are DaVinci Resolve and After Effects. **The more closed the
host, the more human attention the edit costs** — which is the argument for our *"no computer use"
decision, priced*.

## 3. The 49 → 37 number is the most useful data point in this wave

> "Look at these tiny gaps between reactions. I'll get rid of them real quick. I made one pass
> myself in Resolve, trimming those micropauses from the edit Astro Build. **Astro's version was
> about 49 seconds, minus 37. Same shots in sequence, just stripped of all that air.**"

A professional editor, with a frontier agent driving DaVinci Resolve, on a 49-second commercial,
**still had to trim the pauses by hand** — and the pauses were **a quarter of the runtime**.

This is direct external evidence for the gap our own repo already names. `gaps.md` §7 records the
filler/retake problem from our side: *"The filler/retake rules cut a deliberate repeat… restarts,
single-word repeats and cut-off words open no candidate window."* Finding
[20](20-the-runtimes-author-on-the-record.md) records it from Jake Moran's side: on bad timestamps
*"you'll have like half your word cut off"*.

**Three sources now say the same thing: the pause-and-filler pass is where this breaks.** Ours is
the only one of the three with a measured magnitude attached — and the magnitude is a quarter of
the runtime.

## 4. Three defect classes, and every one of them is invisible to a pixel gate

The trailer review is the most valuable forty seconds in the video, because he names what was
wrong in a way a machine could not have reported:

> "the overall structure works, but there are three things I want to change. First, **this moment
> doesn't carry through the cut.** Then, **these three shots pass way too quickly.** I'd also hold
> each shot a little longer so the action reads clearly. And now listen closely to this audio cut."
>
> "**Hear that pop? Terrible.**"

| the defect | what it is | can our gate see it? |
|---|---|---|
| "this moment doesn't carry through the cut" | a **visual motif that does not survive the cut** — the thing that made the previous shot legible is gone in the next one | **no** |
| "these three shots pass way too quickly" | a **duration** defect — the shot is too short to read, judged against what precedes and follows it | **no** |
| "Hear that pop? Terrible" | an **audio discontinuity at a transition** | **no** |

**All three are cross-boundary.** A defect that lives *in* a shot is a shot's problem; a defect
that lives *between* two shots is the edit's problem. That is the whole distinction between gate 2
and gate 3, and our gate is entirely gate 2.

**Verified in our source.** `graphics/gate.rs` (`GATE_VERSION = 2`) checks a **graphic in
isolation**: frame coverage (`SEE_THROUGH_MAX 0.005`), opacity (`SHOWN_MIN 0.02`), full-frame fill
(`FILLS_MIN 0.9`), overlay share (`OVERLAY_MAX 0.6`), the graphic's own cut-in and cut-out frames
(`CUT_SHOWN_SHARE 0.5`), hard-coded colours and fonts, fixed text, variables, and network
references. It runs the graphic at two sizes (`WIDE`, `TALL`), three Chrome instances at a time,
180 s timeout. **It knows nothing about the timeline it lands in, how long it is held, what
happens at the cut on either side of it, or what the export sounds like.**

And we have an instrument for the third class that we never point at the output: we **synthesise**
our own sound effects (`sound_effects.rs` — clicks and whooshes, *"the sounds are synthesized
here, so the app ships no sound files and nothing needs a licence"*) and we **repair** mouth clicks
(`media/declick.rs`). **We can generate the audio of an export and we never listen to it.**

## 5. The context, decomposed

> "Here's the secret to making this work. **The context you give it.** If you want the output to
> match your personal style, you have to feed it references. I handed over our **past After Effects
> projects, fonts, and audio packs**. The **script** tells it what we're explaining. **Our YouTube
> editor skill tells it how we actually work.**"

Four inputs, and they are cleanly separable:

| input | what it carries | do we have it? |
|---|---|---|
| **the script** | the content | yes — the transcript |
| **source files** | the footage | yes |
| **references** — past AE projects, fonts, audio packs | the taste | **partly**: our `Look` holds skin, not structure ([20](20-the-runtimes-author-on-the-record.md)) |
| **the skill file** | the method | yes — `rules.md` — **but ours is compiled into the app and the user cannot own it** (AGT-3) |

Note what the reference is: **past projects.** Not screenshots — *project files*. That is finding
[20](20-the-runtimes-author-on-the-record.md)'s *"somebody else's hyperframes project where you
have the code to use"*, confirmed from the other side: a past AE project is source, and it is the
highest-fidelity reference there is.

## 6. The other side of the computer-use question

Finding [21](21-agent-writes-the-project-files.md) has Matt Loui refusing the UI: *"Do not use
computer use or open CapCut."* This wave has the vendor doing the opposite, and warning about it:

> "Next, **computer use**. This lets Astra open and operate Resolve on my Mac. And a quick note
> before enabling computer use, **Astra can make real changes in your apps, and mistakes can
> happen. Back up your projects, review permissions, and keep sensitive apps closed.**"

**The same wave contains both answers.** Our `decisions.md` refuses *"Driving Premiere or another
NLE by computer use — No | Fragile UI, high upkeep; our typed API owns edits."* Adil is the vendor
saying the same thing in softer words, after shipping it anyway.

And the note is the strongest argument for our position, because it is written by the person whose
product it is: **an agent with computer use can change anything in your apps, and the vendor's own
remedy is "back up your projects".** Our position is not caution — it is the difference between a
tool that can damage your work and one that cannot.

## 7. Two things worth stealing from the mechanism

**The script becomes a file inside the host.** *"It's writing and executing a custom script directly
inside After Effects to build the entire graphic… It automatically exports the animation with an
alpha channel or a transparent background, brings it into Resolve, and places it over the footage…
without me touching a single key frame."* The graphic is generated **in the host's own format**, so
the handoff is a file, not a screenshot. Finding [21](21-agent-writes-the-project-files.md) noted
that the Higgsfield motion path is reachable over a plain MCP URL; this is the same principle
applied to *our* shape: **a graphic should arrive as a thing the host can keep and edit, not as an
image it has to import.**

**Colour as saved nodes, not as a baked look.**

> "Astra gives each adjustment its own node, and **those nodes are saved with the project**. So, if
> I need bigger changes later, I can just jump in and tweak them myself. It nailed the contrast and
> my skin tone."

Contrast with ours: our `Look` holds colour as **tokens** — a small set of global controls
(size, roundness, speed, energy, density) that every graphic in the video obeys. His node graph
holds colour as **a saved structure** the user can open and push. **Both are editable afterwards,
but a node graph is diffable and a token set is not** — and a diffable representation is what the
checkpoint-diff learning loop in finding 21 needs.

**And one capability overlap we should know about:** *"if you need them, you can also ask Astra to
record the demonstrations first. Just give it the project and ask it for that step."* An agent that
drives the screen recorder. **AeroEdits is a screen recorder, and our MCP exposes editing, not
recording** — there is no tool that starts or stops a recording. The field can already record the
demo for you.

## 8. What we would take

1. **Measure the human's attention, not the wall clock.** Adil's two clocks and Matt's ten minutes
   are the same idea, and we measure neither. A number like *"the user spent 12 minutes on this
   edit"* is the product.
2. **Treat the pause-and-filler pass as the fragile one, with a number attached.** A quarter of the
   runtime in one professional's edit. Our `silence` and `fillers` passes are the equivalent, and
   the honest move is to measure them on real material rather than to assume.
3. **Three gate-3 rule classes to try, in this order:** *duration* (a shot too short to read — the
   cheapest, and it needs no pixels), *cross-cut continuity* (the motif that does not carry), and
   *audio at the transition* (the pop). The last one is new to us: **we synthesise the audio of an
   export and never listen to it.**
4. **A cross-cut marker in the storyboard.** All three defects live at a boundary, and our
   storyboard renders the finished edit as one PNG but **does not mark where a cut starts**. That is
   the single artefact change that would make gate 3 possible at all.
5. **Prefer references that are project files over screenshots.** Past AE projects, past
   HyperFrames projects. Finding 20 already noted this; a vendor shipping it confirms it.
6. **Consider a colour node graph alongside the token set.** Not instead of it — the token set is
   what keeps every graphic in one video matching every other, which is our one advantage
   ([17](17-what-the-seven-agree-on.md)). But a graph is diffable, and diffable is what learning
   from checkpoints needs.
7. **Look at whether the agent should be able to record.** We are the recorder. Right now it cannot
   start one.

## What this implies outside this folder — **not applied**

This folder is the only place anything was changed. For the record, here is what a maintainer would
want to change elsewhere — **none of it has been applied**:

- **`research/competitors.md` → Higgsfield section** would gain: the product **editing** (not just
  motion) end to end — script + references + a skill file driving Resolve by computer use, with the
  vendor's own warning about it; the **~80 % self-assessment**; the **two-clock metric**; and the
  **49 s → 37 s** result, which is the only published magnitude for the pause pass failing.
- **`research/competitors.md` → at-a-glance row for Higgsfield** would gain the editing side
  (Resolve by computer use, batch-N shorts from one timeline, AE script → alpha → Resolve handoff).
- **`research/creator-workflows.md` → artefact index** would gain the *Editing Skill + SFX Library*
  as a **free download published in the description** — **no licence stated: study the shape, copy
  nothing** — alongside the existing note that taste and assets are sold as a pair.
- **`research/creator-workflows.md` → "Tricks worth stealing"** would gain two rows: *ask for 10 or
  20 cuts from one timeline and review the batch*; *colour adjustments as nodes saved in the
  project, so the user can push them later*.
- **`gaps.md` §7** would gain a row: *no rule for how long a shot is held or how long a graphic is
  held* — now supported by a published magnitude.
- **`gaps.md` §12 (performance)** would gain a row: *no metric for the human's attention cost of
  an edit.*

## Caveats

- **Captions are ASR.** 301 snippets; the brand names are mangled as listed in the header and left
  as captured, per the repo's own `evidence/scripts/README.md`. Every quote above was checked
  against the caption text.
- **This is a vendor promoting its own product**, on its own channel, with its own affiliate links
  in the description. Every claim about Astra's quality is the vendor's.
- **The numbers are the presenter's own estimates** of his own time. "30 minutes actually working on
  this" and "about 20 minutes" hands-on are self-reported and not instrumented.
- **The 49 s → 37 s figure is one commercial, one manual pass, 49 seconds long.** It is a striking
  magnitude, not a rate over many edits.
- **"Astra got about 80% of it right"** is a self-assessment with no rubric attached, and the same
  presenter made three structural notes on the trailer.
- **The earlier wave's write-up stands.** Nothing here corrects
  [`research/competitors.md`](../research/competitors.md); this file adds the numbers and the
  defect list to it.
- **The generator classes do not apply to us.** "The agent's on the wrong side of the desk" is a
  defect in *generated video*, and our `decisions.md` refuses generative video as a core
  dependency. It is recorded here because it is the clearest statement in this wave of why the
  agent's output has to be watched by a person — not because we would ship the generator.
