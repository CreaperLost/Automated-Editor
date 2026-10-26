# 20 — The runtime's author, on the record

- **Source:** [youtube.com/watch?v=MC1g2fHYC88](https://youtu.be/MC1g2fHYC88) — *How to Make Amazing
  Videos with Claude and HyperFrames* · **Peter Yang's channel** · guest **Jake Moran** · 25 Oct 2026
  · 41:41 · 252 views · 14 likes
- **Artefacts:** [hyperframes.dev](https://hyperframes.dev) (HyperFrames Studio, Mac + Linux) ·
  [Parakeet TDT 0.6B v2 model card](https://build.nvidia.com/nvidia/parakeet-tdt-0_6b-v2/modelcard) ·
  [Jake on X](https://x.com/jake11moran)
- **Raw capture:** [evidence/2026-10-25/how-to-make-amazing-videos-with-claude-and-hyperframes-jake.md](evidence/2026-10-25/how-to-make-amazing-videos-with-claude-and-hyperframes-jake.md)
- **Evidence grade:** creator claim. 1,144 ASR snippets; names are mangled, so quotes below are
  checked against the caption text and the description's chapter list. **It is an opinion
  interview, not a specification.** Nothing was run.

Every other reference in this folder is *about* the field. This one is **the person who builds
the runtime we ship, explaining how he uses it in anger** — Jake Moran is a HeyGen employee and
a member of the HyperFrames team. It is the closest thing in this folder to the vendor's own
account of how the thing we depend on is actually used.

## 1. Parakeet is not a transcription detail — it is an edit-quality detail

> "sometimes it misses on the exact timestamp. It's a bit hard for LLMs to figure out exactly
> when something said down to the millisecond, but it's key for, let's say, Instagram or
> TikTok-based editing where you really really matters that you're super concise and that the
> pacing is really high."
>
> "With a bad transcription model, the LLM will think, 'Okay, cool.' And then it might be
> operating on some bad timestamps, and then you'll have like **half your word cut off**."
>
> "**Parakeet for me works 98% of the time** to getting the right amount of time in between
> words or whatever the case you need."

**We ship Parakeet.** `src-tauri/src/transcript/parakeet.rs` carries two models: **Unified** (the
default) and **Verbatim** (TDT 0.6B v3). Our repo chose Unified because *"on FLEURS it makes 4.0%
word errors where TDT v3 makes 4.8%, and it gets names and numbers right more often"* — but it
*"smooths speech over: on a 15-minute tutorial it dropped 'see see', 'we can can' and six of nine
'uh's."*

**There is a real tension, and our repo measured the wrong metric.** Jake's metric is **timestamp
accuracy**, because that is what decides whether an instruction like *"cut the pauses in between
sentences to a quarter of a second"* works at all. Our repo measured **word error rate** and
**content** (names, numbers). Nobody here has measured timestamp accuracy — and *"half your word
cut off"* is precisely what a pause-cap instruction does on bad timestamps.

**And note the direction of the argument.** We picked Unified partly *because* it drops the "uh"s.
But if it drops them, **the filler pass has nothing to cut and the pause timing is computed from
smoothed speech.** Our own repo says this about TDT: it *"writes stutters, repeats and filler
sounds down, which the filler and retake passes cut."* Jake, from the field, says the same thing
from the other side. **Our default may be wrong for the cleanup passes even if it is right for
names and numbers.**

Two supporting facts. He uses **TDT 0.6B v2** — our Verbatim model is **v3**, so we have the newer
one. And **HyperFrames ships its own smaller local transcription model**, which "sometimes misses
on the exact timestamp" — so our runtime's own model is *worse* than Parakeet on timestamps, and
we already chose Parakeet over it. That choice now has an external reason behind it.

## 2. The vocabulary is the interface

> "it's important to **use language that both you and the LLM will recognize** because that'll help
> you get to results faster, not waste tokens. We all know that AI is expensive."

The terms he teaches: rough cut · jump cut · punch-in · B-roll · hook · zoom-in (push-in, pull-back,
cut-in) · transition · whip pan · speaker cards · overlay · table of contents.

**We have this vocabulary in the app, and the user cannot see it.** Our `rules.md` is 107 lines and
the `preset` tool returns the rules — but nobody teaches the user the words, so the user cannot
use them. Jake's move is to *teach the vocabulary* so the user can prompt better and spend fewer
tokens. That is a surface we do not have: **the vocabulary of our own editor.**

## 3. "Energy" words are the right granularity — and we already ship them

> "I want this to be high-paced. I want this to be kind of crazy. Or I want this to be refined,
> professional, sleek. These types of more general, I like to say **energy type terms** work
> really well for helping these models get there."

Our `Look` has `energy` as one of **five global controls — size, roundness, speed, energy,
density** (`src-tauri/src/graphics/look.rs`). The field's working vocabulary and our token set are
the same five adjectives. Our granularity is right; what makes it ours is that the **app** holds
them ([17](17-what-the-seven-agree-on.md)).

## 4. A brand kit is what stops the default

> "If you just say 'my brand', probably **nine times out of 10**, you're going to get some crazy
> colours and then **imagined surface**. So it does matter to add the context."
>
> "we have a feature now that allows you to **create a brand kit just from your website**… the
> agent just pulls them down from your website or you can upload them."

**"Nine times out of 10"** is the field's own estimate of what finding
[10](10-the-rest-of-the-field.md)'s benchmark measured as "six of sixteen looked like the
default". Two independent estimates of the same rate, from opposite ends.

And the feature: **brand kit from a URL.** We make the user assemble a brand kit by hand.
HyperFrames reads the website. That is one input, cheap, and it is exactly what our `brandColours`
gate check already assumes exists (*"background, text or accent is not declared as a colour
variable"*).

## 5. The style gap has two halves, and we hold one of them

This is the chapter the video is built around, and it **answers the question finding
[12](12-hyperframes-is-our-runtime.md) left open**.

> "if you have styles that you really want to emulate, you can either watch them really slowly and
> try and describe them… **there's a lot of little nuances in what just happened in this hook.
> Including how the words enter, how there's background removal so that some of the items can be
> slightly behind the character. There's a lot that needs to be described.**"
>
> "Yesterday, we added this as a native capability to **if you just upload a reference** — so I
> uploaded this 5 seconds and then I described in the prompt just what I wanted to change… you can
> see where we're drawing inspiration from but at the end of the day this feels like a very
> different scene because there's different font different colors but it's **still structurally the
> same idea**."
>
> "there are only probably **10 total things** that happen in this hook, it's more about **getting
> those right**. It's easier if you can start with references — maybe that be screenshots, maybe
> that be previous videos, maybe that be **somebody else's hyperframes project where you have the
> code to use**."

**The field now has a way to copy a look that does not go through words.** Upload five seconds; the
model reverse-engineers it into code; then you restyle it.

**The decomposition that matters:**

| | what it carries | who has it |
|---|---|---|
| **Structure** | which elements, in what order, with what relationships; how the words enter | HyperFrames' reference reverse-engineering; framejam's per-element map; video-shotcraft's recipe cards |
| **Skin** | font, colour, brand, easing | **AeroEdits' `Look`** |

HyperFrames' capability moves **structure** across. Our `Look` holds **skin**. **Neither holds
both.** A `Look` that also carried a per-element map — *which element, in what order, with what
relationship* — would be the complete artefact, and finding
[05](05-style-as-data-with-a-guide.md) already recorded that the per-element map is the thing
framejam has and we have no home for.

Note the third reference path: **"somebody else's hyperframes project where you have the code to
use."** A HyperFrames project is a *legal, structured, reusable* reference — and HyperFrames is
Apache-2.0. That is a better reference than a screenshot, and it is a licence-clean one.

And the line for people who already have taste: *"you now have a library that can be referenceable
if you reverse engineer it into code"* — *"all of these things are dialed down through years of
creation… you have your style, you have your colors, you have your formats."* **That is what a
`Look` is supposed to be, and nobody has automated the derivation.**

## 6. Four sections, and a library of them

> "I normally think of that in four sections. **Hook, kind of like the context building, the maybe
> the call to action or the output, and then coming back around to like, 'Hey, here's our
> brand.'**"
>
> "we have these things in our desktop app that we call **moments**… all of these are available to
> anyone if they use our desktop app. And all you have to do is add them and then add a little bit
> of additional information… **it'll recreate within your brand context** because my brand kit was
> selected."

**HyperFrames has a library of reusable structural sections, in the same medium as the output.**
Our presets carry rules and steps but no named structural template, and no library — though
`shorts_energy` and `shorts_speed` are the beginnings of one.

## Three more that concern us directly

**Drawing on the frame.**
> "You can just **draw on what you want to change**, leave the note, and then they'll queue."
>
> "up until 2 weeks ago… I'm watching it and **typing in Claude chat at the same time**. And that's
> difficult."

This is our **NOTE-1**, with drawing added. The repo's trick #1 was *"anchor feedback to a time
range, not a chat message"* (Tandem; Tao). This is a third confirmation, and the *drawing* is the
new part.

**PII blur as a skill.** Peter: *"I've built a skill called video blur that blurs out all the
sensitive information… I'm usually like trying to screen share like Claude Code or something and
there's some PII stuff in there."*

**We have no redaction capability** — the only "blur" in our code is a drop shadow
(`shadowBlurPx`). For a screen-recording editor this is the highest-value missing feature in this
folder: kitcut has it ([16](16-kitcut-priced-choices-checked-renders.md)), Peter has it as a
skill, and we would otherwise publish a tutorial that shows a card number.

**Segmenting long videos to avoid reprocessing.**
> "anything longer than let's say 15, 20, 30 minutes, I like to split it into different segments…
> if you want to make a change at like 13 minutes in to an hour-long video, it has to reprocess the
> whole thing which is going to like just **burn a bunch of tokens**."

**We do not have this.** Our `chapters` tool sets chapter titles; the timeline is one document.
The reason he gives is exactly our **AI-1**.

And the closing tip, which is the repo's open **AGT-3**:
> "all that is is asking at the end of the session, 'Hey, let's remember exactly what the steps
> here are so that I can reference it later for when I'm creating more videos.' … You definitely
> want to save things as a reference so you don't have to do the same work twice."

**The agent writing the skill at the end of the session.** The repo's gap — *"Every creator
publishes a skill/prompt as a file. We ship a `skill.rs`; the user can't author their own."* —
has a concrete answer here, and it is a prompt, not a feature.

## What we would take

1. **Measure timestamp accuracy per model, on our own material.** It decides whether the filler
   and pause passes work at all. We have measured word error rate and never timestamps — and our
   own default model is the one that drops the "uh"s the filler pass is looking for.
2. **Brand kit from a URL.** One input; it is what stops "crazy colours and imagined surface".
3. **Per-element structure in the `Look`.** The missing half of the artefact.
4. **A named structural template per preset** — hook / context / payoff / brand — and eventually a
   library of them, the way HyperFrames has `moments`.
5. **The vocabulary surface.** Teach the user our editing words so they can prompt in them.
6. **Redaction.** Screen recordings leak PII and we would publish it.
7. **Segments as a first-class unit**, for token cost as much as for editing.

## What this implies outside this folder — **not applied**

This folder is the only place anything was changed. For the record, and because the substance
belongs with the finding, here is what a maintainer would want to change elsewhere — **none of it
has been applied**:

- **`roadmap.md` → GFX-5** (*"AI-built styles: later, research first (George, 23 Oct)"*) asks
  *"research how they do it before building this"*. This video is that research. The three inputs
  are confirmed one by one, and **the brand-from-a-URL input is the one we cannot obtain.**
- **`gaps.md` → §5 (Intent)** would gain a row: *a brand is assembled by hand; the reference
  workflow reads it from the website.*
- **`gaps.md` → §11 (Languages and transcription)** would gain a row: *the transcript model is
  chosen for word error rate, but the cleanup passes need timestamps — and our default is the
  model that drops the fillers they exist to cut.*
- **`research/creator-workflows.md` → artefact index** would gain *HyperFrames Studio
  (hyperframes.dev, Mac + Linux)*, with the note that the brand kit from a website, `moments`
  and draw-on-frame review live in the **desktop app, not in the runtime we ship**.
- **`research/creator-workflows.md` → "Tricks worth stealing"** would gain four rows: make the
  brand obtainable rather than typed; a reference can be another project in the same medium;
  redact as a pass over the render; split a long video into segments so a late change does not
  reprocess everything.
- **`research/creator-workflows.md` → "Corrections"** would gain two notes: Jake Moran's account
  of HyperFrames is **first-party** (a HeyGen employee on the team), and the 24 Oct video has
  **no verification step at all** — which is the argument for our gate being a throughput
  feature.

## Caveats

- **Captions are ASR**, 1,144 snippets; names come out mangled ("Astra", "MIDI", "Muse Wonder",
  "Paweł Lipski"). The description and its chapter list are the reliable parts.
- **It is an opinion interview, not a spec.** 41 minutes with a host, narrated demos, and
  impressions like *"the models seem to be maybe more conservative than I normally am"*.
- **Jake is a HeyGen employee and on the HyperFrames team.** This is a vendor's account of its
  own product — the most credible account available, and still not neutral.
- **"98% of the time" is unsourced**, and is about Parakeet **v2** on his material, not v3 on ours.
- **The reference-uploading capability was unlocked the day before the recording** (*"we unlocked
  this new tool internally and we shared it yesterday"*). It is brand new, documented only here,
  and may not exist in the **0.8.162** we pin.
- **Several features are in HyperFrames Studio, not in the runtime** — the brand kit from a
  website, `moments`, and draw-on-frame review. We ship the CLI and the runtime, not the app. This
  is exactly the distinction finding 12 flagged and finding 18's three director layers rely on.
  Studio is Mac and Linux, which is where we run.
- **252 views on day one.** Not a signal of anything.
