# 23 — The style is derived by measuring the frames, and then it gets a name

- **Source:** [youtube.com/watch?v=ge1yzQ5MXmM](https://www.youtube.com/watch?v=ge1yzQ5MXmM) —
  *The Only AI Video Editing You Will Ever Need (Turned it Into A Free Skill)* · **Jad M.H | AI
  Automation** · 21 Oct 2026 · 10:50 · 42,711 views · 908 likes
- **Artefacts:** *Editor V1* — a free Claude skill, distributed as a Google Drive folder behind a
  [Skool](https://www.skool.com/nextphase-5790) community link, installed by pasting a prompt into
  Claude Code. **No GitHub repository.** The one public link is an affiliate redirect to
  `higgsfield.ai/s/video-model-showdown-kling-4-jadm-h-cJsSBE`.
- **Raw capture:** [evidence/2026-10-25/the-only-ai-video-editing-you-will-ever-need-turned-it-into.md](evidence/2026-10-25/the-only-ai-video-editing-you-will-ever-need-turned-it-into.md)
- **The kit itself was then fetched and read in full** — 79 files, with a licence, a derivation
  record and three checks: [finding 25](25-the-kit-behind-the-video.md). Two things below are
  **corrected by it**, and are marked where they appear.
- **Evidence grade:** creator claim. 302 ASR snippets; names are mangled (*"school"* for Skool,
  *"skull"* for skill, *"Ace 1.5"* for ACE-Step 1.5, *"Cloud Code"* for Claude Code, *"Zad"* for
  Jad), so every quote below is checked against the caption text. **Nothing was run or measured.**

This is the closest thing to AeroEdits that exists in the field, and it is not a competitor — **it
runs on our own runtime.** That makes it the single most useful reference in this folder, and the
one whose gaps are ours.

## 1. The method: measure the references, then name the result

The instruction he gave the model:

> "Look at every frame, frame by frame, how they do it."

And what came back:

> "It went through all three reels, it measured how often the pictures change, and in one of the
> reels, it's **175 times a minute**. It measured the colors, where the caption sits, how big the
> words gets. It even **split the voice from the music** so it could study the sound effects on
> their own. And then it turned all of it into **one style with a name**."

The description gives the count the captions do not: **5,982 frames** across three reels.

Then the invocation, which is the whole product:

> "And then it turned all of it into one style with a name. So now every time I use this style, **I
> just say 'Use editor V1.'**"

**This is GFX-5 answered by measurement rather than by prompt.** Our `roadmap.md` says GFX-5 —
*"AI-built styles: later, research first (George, 23 Oct)"* — and finding
[20](20-the-runtimes-author-on-the-record.md) gave the runtime author's answer (a board of reference
images, their brand, a tuned prompt as a skill). **This is the second, cheaper answer:** sample two
or three references frame by frame, measure them, emit thresholds, give the result a name.

The reference count is a budget claim, not a shrug: *"Find two or three videos, **not more**."*

## 2. Version 1 was bad, and the corrections produced rules he had not noticed

> "the first version of it was really bad, like really, really bad."

His notes on the first demo:

> "You're not using the sounds I gave you, use more imaginations, more clips, more sounds." And I
> emphasized on looking back to the three videos. "Every frame, second by second to the millisecond."
>
> "And that's when it went back and broke the reels down even deeper. And it **found stuff I didn't
> even notice**."

Four rules came out, and three of them are **numeric and checkable**:

| the rule, verbatim | what kind of rule |
|---|---|
| "At least half of every reel is moving clips, not still pictures" | a **floor** — ≥50 % of the runtime is moving |
| "Most shots slowly zoom in" | a **default** — the zoom is not an option |
| "Every new shot starts blurry and goes sharp" | a **universal** — every scene change gets a focus pull |
| "If a picture stays on the screen for 4 seconds, that counts actually as a mistake" | a **ceiling** — 4 s maximum hold |

> "So it set up these guardrails."

**This is the part that matters for AeroEdits, and it is not in finding 20.** A correction loop
turned *taste* into *thresholds*, and the thresholds became *guardrails the model enforces on
itself*. His closing verdict on the process is that the model was wrong, he said so, and the second
pass found rules he had never articulated. That is exactly the loop our `gaps.md` calls **AGT-3**
(*"the agent learns from the user's manual tweaks"*), except the output is **a threshold file, not
free text** — which is what the 17 Oct condition for bringing learning back asked for.

The fifth observation is about content, not thresholds, and it is the sharpest thing in the video:

> "A lot of the lines are puns. When I said 'leaked', it showed water dripping from a tap. **I never
> asked for that. It learned it from the reels.**"

**The picture is the meaning, not the noun.** That is finding [01](01-treatment-model-and-visual-lead.md)'s
thesis — graphics drawn from what the speaker actually said — arriving from the opposite direction:
not "the graphic must match the sentence", but "the graphic must be the sentence's *other half*".

## 3. It runs on our runtime

> "Claude isn't a video model here. It doesn't generate the video. **It writes code.** It uses
> **HyperFrames**, which is free and open source. Think of it like a web page that plays like a
> video. Claude writes every picture, every word, every flash as a piece of code with a time on it.
> It takes those frame by frame and turns it into an MP4 right on your Mac. So, when I change
> something, **it can change specifically that thing cuz it was frame by frame**."

Our runtime **is** HyperFrames, pinned at 0.8.162, Apache-2.0
([12](12-hyperframes-is-our-runtime.md)). So:

> **The nearest field artefact to AeroEdits' thesis is a skill written for our own engine.**

Two consequences.

**First, the editability argument, applied to a code-first host.** Finding
[21](21-agent-writes-the-project-files.md) established the principle from the CapCut side — *"what
makes a host editable by an agent is not whether it has an MCP; it is whether its project is a
writable file"*. Jad's version is the same principle about a host whose project is **code**:
frame-by-frame rendering from a project means a change is a change to that thing only. Our
`decisions.md` refuses Remotion (*"Two style systems and two failure modes"*) and refuses driving an
NLE by computer use (*"our typed API owns edits"*). HyperFrames is neither of those — it is our
runtime, and this is the field's confirmation that the choice pays.

**Second, the gap is not the engine — it is the named style.** We have the runtime, the gate, the
`Look`, the passes. What we do not have is **a style the user can name and hand to the agent.** Our
`preset` tool returns rules; the user picks a preset. His user says one sentence — *"Use the style
Editor V1."* — and gets a coherent edit.

And the one thing we have that he does not: **we keep the look in the app** (finding
[17](17-what-the-seven-agree-on.md)), which is the only way every graphic in one video matches
every other. His look lives in a Drive folder. Ours would live in the app, with a name on it.

## 4. Every rule, against our source

| the derived rule | where it stands in AeroEdits (verified in this repo) |
|---|---|
| "at least half of every reel is moving clips" | `look.rs` has a `density` control and `energy`; **nothing counts** the ratio of moving to still. The gate has no such check. |
| "most shots slowly zoom in" | we have a `zooms` pass. Whether a zoom happens is still the user's answer. |
| "every new shot starts blurry and goes sharp" | **We do not have this transition.** A search of `src-tauri/src/graphics/` for `focus`, `blur`, `ken_burns` returns **no matches** — the only "blur" in the app is a drop shadow (`shadowBlurPx`). A focus pull on every scene change is one CSS filter and one timing token. |
| "a picture on screen for 4 seconds is a mistake" | **We have no hold ceiling anywhere.** The only duration guidance in the app is prose in `preset/methods/shorts.md` (*"3 to 8 s pieces read well"*), which is a sentence in the agent's instructions, not a check. |
| "the lines are puns — 'leaked' showed a dripping tap" | our graphics are derived from the words said; the *pun* case (the meaning, not the noun) is a stronger requirement we have not stated. |
| "my face becomes one more picture in the stream around the third of the time" | matches iart-ai's three-treatments rule in finding [01](01-treatment-model-and-visual-lead.md) — speaker, graphic beside, full-frame graphic. **The rule converges; the proportion does not.** The kit marks its 25–35 % face share as *"a decision, not a measurement"* (finding 25 §3) — so ~1/3 is his choice, not an independent arrival at the same number. |
| "split the voice from the music so it could study the sound effects on their own" | we have a sound-effects lane (`sound_effects.rs`) synthesized from the edit document and a declick pass — but we do **not** separate speech from music to analyse a mix. |

### The 4-second rule is the interesting one

It is the first rule in this whole folder that a **timeline-level** check can enforce honestly.
Finding [17](17-what-the-seven-agree-on.md)'s rule is *"the check must read the output, not the
plan"* — and a hold ceiling is the one taste rule where the plan **is** the output for the thing
being checked, because the plan determines how long a picture is held.

It also catches a class the pixel gate provably cannot: **our gate reads the graphic's pixels, not
the length of time the graphic is on screen.** A correct graphic held for nine seconds passes
every check we have. Our `storyboard` renders the finished edit as one PNG — a tile per scene — and
it splits on scenes, zooms, graphics, chapters and long silences, **but not on cuts**; it has no
waveform and no durations. So the artefact that would carry this rule does not exist yet, but the
rule itself is nearly free: the plan already knows each scene's length.

## 5. Provenance, cost, and what we do not have

> "And about the pictures, since they are real, it writes down **where every single one comes from
> with its licenses**. The NASA photos are public domain. The Wikimedia ones you credit in your
> description. And **it checks every fact while it's looking for pictures**. So, it's not just
> grabbing random stuff on Google."

> "Honestly, almost zero dollars. The only thing you pay for is your Claude subscription.
> Everything else here is free. HyperFrames is free. **The transcription runs on my Mac**, which is
> free. For the music, you use your own music with no copyright one, or you use Ace 1.5 that
> generates music for you with AI. The pictures are public domain or Creative Commons."

Two verified gaps on our side.

**Provenance.** A search for `provenance` across `src-tauri/src` returns **only the telemetry
module** (mouse-event provenance, unrelated). **We record no source and no licence for any asset we
put on screen.** Finding [15](15-broll-provenance-before-the-clip.md) established that broll-mcp
writes the provenance record *before* the clip; this is the same discipline applied to stills, and
it is a record we do not keep.

**Cost.** Our `transcript/settings.rs` defaults `provider: ProviderKind::ElevenLabs` — **a paid
cloud API** — with `parakeet_model: ParakeetModel::Unified` as the *local* alternative. We ship
Parakeet and do not default to it. His entire cost argument ("almost zero dollars") rests on
transcription running on the machine. Finding [20](20-the-runtimes-author-on-the-record.md) adds
that Parakeet is *also* the better model for the cleanup passes, because timestamps decide whether
*"cut the pauses between sentences to a quarter of a second"* works at all. **Two independent
reasons now point at the same default, and ours is the one neither supports.**

## 6. The recipe, in five steps

> "Find two or three videos, not more. Give them to Claude and tell it to look at it frame by
> frame. Let it make a demo with one of your old videos. Then tell it what you don't like and be
> honest because that's literally how I got V1 to look good. And then, that's super important, you
> give it a name."

**References → measurement → a demo on your own material → an honest correction → a name.** The
third step is the one people skip: *"Let it make a demo with one of your old videos"* — the
derivation is only real once it has been applied to *your* footage and rejected.

## 7. Numbers

| | |
|---|---|
| **5,982** | frames measured across three reels (description only) |
| **175** | picture changes per minute, one reel |
| **≥ 50 %** | of every reel is moving clips, not stills |
| **4 s** | maximum hold on a picture — *"counts actually as a mistake"* |
| **1 h 15 m** | first message → demo, **including studying the reels** |
| **~ $0** | per video: HyperFrames free, transcription local, music own or ACE-Step |
| **~ 1/3** | of the time his face is one picture in the stream |
| **1–2 s** | "shots breathe" in the middle of a long video |
| full speed | at the intro and at the start of every chapter (long form) |
| "go bananas" | short form |
| "Version 2 was way better, **still not perfect**" | his own verdict |

## 8. What we would take

> **The kit behind this video was fetched afterwards and read in full —
> [finding 25](25-the-kit-behind-the-video.md).** It contains the derivation record, the device
> library, the plan format, the sound grammar and three checks, and it turns most of the items
> below from "he said it" into "here is the file and the number". Read that one for the artefact;
> this one is for the video.

1. **The derivation, not the prompt.** GFX-5 has two answers now and this is the cheap one: sample
   two or three references frame by frame, measure, emit thresholds, name it. The measurements are
   model output, not a specification — but the *method* is what we would implement.
2. **The max-hold rule as gate 3's first pixel-free rule.** We have no hold ceiling anywhere, and
   the rule catches a class our pixel gate cannot see. It needs scene durations, not pixels.
3. **The correction loop as AGT-3's mechanism.** He corrected the style twice and the second pass
   produced rules *he* had not noticed. Our checkpoints already hold the before and after of a
   user's manual tweaks; the diff is the learning signal and the output is a threshold file —
   data, not free text.
4. **Provenance for every asset.** "Where every single one comes from with its licenses" is a
   record we do not keep for any picture we show.
5. **A focus pull on every scene change.** A concrete, cheap transition we do not have.
6. **A named style, invoked by a phrase.** Our `Look` is the only artefact in the field that keeps
   the look in the app. Give it a name and a sentence — *"use the style X"* — and the derivation
   becomes a feature we could ship on top of it.
7. **Local transcription as the default.** Two independent arguments now point at Parakeet
   (timestamps for the cleanup passes; the cost story) and our default is a paid cloud API.

## What this implies outside this folder — **not applied**

This folder is the only place anything was changed. For the record, here is what a maintainer would
want to change elsewhere — **none of it has been applied**:

> **Finding 25 covers the same territory from the artefact itself** (the kit's own licence, the
> derivation record, the device library, the plan format and the three checks) and its list is the
> fuller one. Use that one for the actual edits; this one for what the video said.

- **`roadmap.md` → GFX-5** would gain a second research answer: *derive the style by measuring the
  reference frames and emit thresholds, rather than by prompting for taste* — alongside the
  brand-from-a-website answer in finding 20.
- **`gaps.md` → §7 (Style)** would gain rows: *no maximum hold on a picture*; *no focus-pull
  transition on a scene change*; *no rule counts moving against still*.
- **`gaps.md` → §5 (Intent)** would gain a row: *the style is a named artefact the user invokes; we
  have presets, not named styles.*
- **`gaps.md` → §11 (Languages and transcription)** would gain a row: *the default transcriber is
  a paid cloud API although the local model is better for timestamps.*
- **`gaps.md` → assets** would gain a row: *no source or licence is recorded for any asset we show.*
- **`research/creator-workflows.md` → artefact index** would gain *Editor V1* (Skool + Google Drive,
  **no licence stated — study the method, copy nothing**) and the five-step recipe.
- **`research/creator-workflows.md` → "Tricks worth stealing"** would gain three rows: derive a
  style by measuring two or three references; correct it with a demo on your own material and
  honest feedback; give it a name.
- **`research/creator-workflows.md` → "Corrections"** would gain one note: finding 19's *"do not
  quote a version"* caution on Kling can now be **narrowed**, because the affiliate slug
  `video-model-showdown-kling-4-jadm-h` is a URL, not an ASR transcript, and supports the creator's
  "Kling 4.0". The caution stands for any *other* model name.

## Caveats

- **Captions are ASR.** 302 snippets; the mangled names listed in the header are left as the
  capture has them, per the repo's own `evidence/scripts/README.md`, which says not to rewrite brand
  names in bulk. Every quote above was checked against the caption text; the 5,982-frame count is
  from the description only.
- **There is no artefact on GitHub**, and there is no version history: the skill ships as a Drive
  folder behind a community link. **It does have a licence**, which this file originally recorded
  as absent — the kit was fetched afterwards and it states its terms
  ([finding 25](25-the-kit-behind-the-video.md) §1): *free to use including commercially, but do
  not resell it or repackage it as a paid product.* **That is more permissive than kitcut (no
  licence) and than `video-talkcraft` (PolyForm Noncommercial), and since we are not selling
  anything, the one restriction does not bite: the kit may be run, adopted and reimplemented.**
- **The measurements are the model's output as reported by the creator.** 5,982 frames, 175 per
  minute, ≥50 % moving, 4 seconds — none was reproduced here, and we do not know the sampling
  rate the model used.
- **One correction, from the kit.** This file originally claimed the kit's "~1/3 of the time his
  face is one picture in the stream" *independently* matched iart-ai's three-treatments rule and
  landed on the same proportion. The kit itself marks that share as *"a decision, not a
  measurement"* — **the rule converges, the number does not.** Corrected in §4.
- **"Version 2 was way better, still not perfect"** is his own verdict on his own artefact.
- **One creator, one setup.** Claude Opus 5.5, Claude Code, a Mac, a Skool community. The model
  name is the creator's; it was not verified here.
- **The 1 h 15 m includes studying the reels**, and it is on the strongest model. It is not a
  per-video editing cost.
- **His own comparison** puts a human ahead: *"A real editor will always get more detail."* His
  argument is about **speed against news**, not quality — *"if the news drops this morning and your
  video comes out 3 days later, it's over."*
