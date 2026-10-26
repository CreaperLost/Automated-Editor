# 19 — The folder is the prompt

- **Source:** [youtube.com/watch?v=RXN9ci0SvIo](https://youtu.be/RXN9ci0SvIo) — *How to Automate
  Your ENTIRE Editing Job ($49,540/Mo)* · **Zane Hoyer** · 24 Oct 2026 · 7:18 · 34,494 views · 882 likes
- **Artefact:** [OpenArt MCP](https://openart.ai/mcp) (`mcp.openart.ai`) — the only link that is
  not OpenAI or Anthropic
- **Raw capture:** [evidence/2026-10-25/how-to-automate-your-entire-editing-job-49-540-mo.md](evidence/2026-10-25/how-to-automate-your-entire-editing-job-49-540-mo.md)
- **Evidence grade:** creator claim. Captions are ASR — names come out mangled; the description is
  the reliable source. **The folder structure was shown on screen, not published.** Nothing was
  installed or tested.
- **Lineage:** this is the sequel to Zane's **8 Oct 2026** video, *AI Edited My ENTIRE Video in
  DaVinci Resolve (One Prompt)* (`P_NL_hg-y_4`, 184,640 views), which is already in the earlier
  wave at `docs/research/evidence/video-dossier-2026-10-21.md` #15. Same channel, same stack,
  sixteen days later.

This is the only finding in the folder that is a **creator workflow** rather than a tool. It is
also the only one with no verification step at all — which turns out to be the most interesting
thing about it.

## What changed in sixteen days

The 8 Oct video's artefact was **one master prompt in a Google Doc** — our own artefact index
still lists it as *"Master prompt for a DaVinci edit (Zane)"*, and the earlier wave priced it:
*"a prompt Google Doc (Zane Hoyer, 'refined for $700')"*.

**This video publishes no file at all.** Its three links are OpenArt, Codex and Claude. What it
publishes instead is a **folder structure**, and with it a change in where the method lives:

> "usually if you wanted to just press one button and then walk away, you'd have to give it a super
> long prompt. But **since we have everything built out in our folders**, we can essentially just
> say the following."

The method moved **out of the prompt and into the folder next to the footage**, and the prompt
collapsed to one sentence.

**That is AeroEdits' architecture, arrived at from outside.** Our `preset` tool returns the rules,
the steps, the export steps and the user's own saved choices *from the app*, so the agent's prompt
can be short — the app's own code calls our skill folder *"only the pointer to `template`, so an
agent reads the rules once, from the app"*. Zane gets the same effect by putting the context in
files on disk next to the footage. Different mechanism, same conclusion:

> **The prompt should be short because the context lives somewhere else.**

Our `AGENTS.md` says the same thing about our own repo: graphics go into the project through
`graphic_write_file`; everything else stays in the project.

## The folder — and what each part is really for

```
<client>/
  raw footage/
  subject assets/               "super clear face shots, as well as like your client's clothing"
  motion graphic style references/
  brand kit/                    brand colours, logo, a font
  (a document)                  the client's editing styles, fonts and brand identity
  (a document)                  different types of motion animations
```

His reason for naming everything: *"it's really important we name everything because remember,
we're directing an AI agent to come here, and it has to figure out what is what, essentially."*

**The two reference folders are not documentation. They are seed inputs for two different
generators.**

- **`subject assets/`** — clear face shots and clothing, so the image generator keeps the client's
  likeness. The earlier wave already caught this from the 8 Oct video (*"it used her actual
  footage as an OpenArt reference"*) and filed it as gap #8:
  `graphic_generate(prompt, seed_path, brand_id)`. **This folder is the `seed_path`.**
- **`motion graphic style references/`** — screenshots of other people's edits, so the motion
  graphics match them. His claim at the top of the video: *"the majority of people on this planet
  don't realize you can take a screenshot of any editing style, throw it into a chatbot, and it
  will animate it into an advanced motion graphic."*

**This is the sharpest thing in the video, and it is about pixels.** The field's conditioning input
for a look is a **picture**, not a description. So there are three ways to express a look in this
folder, and they are not the same thing:

| | form | what it promises |
|---|---|---|
| framejam's `guide`, hyperframes' `frame.md`, Zane's "motion movement style" prompt | **words** | an agent can *build in* the look — but each graphic is a fresh interpretation |
| Zane's `motion graphic style references/` | **pictures** | the look is *borrowed* — one shot, and the next graphic will not match it |
| **AeroEdits' `Look`** | **tokens** | size, roundness, speed, energy, density, colour roles, fonts, easing — **reproducible and checkable** |

Ours is the only one of the three that can promise *"every graphic in this video matches every
other"*. It is also the only one that **cannot start from a screenshot**. That is the real shape of
the gap, and it is narrower than "we have no style references": we are not missing a picture, we
are missing the ability to *derive a look from* one — which is a model, and models are outside our
no-network gate.

## The three things that decide quality — in his words

> "the quality of your edit really comes down to three things we need to focus on: **motion graphic
> references**, **clear shots of the subject** for advanced B-roll, and then **a really solid brand
> package**. This will include things like main color theme, font, logos, and color grade. And
> then everything else will be taken care of by the prompt, which will include things like cutting
> up the footage."

**This is our thesis, stated by a freelancer.** Quality = style + brand + subject. The prompt
handles the mechanics. Finding [10](10-the-rest-of-the-field.md)'s benchmark said the same thing
from the other end: mechanical checks were free, and only the look separated seventeen runs. Two
independent sources, same conclusion.

And note what is **not** in the three: the model, the prompt, the NLE, the tool.

## The prompt

> "I have DaVinci Resolve open. I want you to edit my video inside of DaVinci Resolve. Go to my
> downloads and find the folder labeled Zane. Extract my raw footage, cut out the spaces, color
> grade my footage, and balance my audio. Analyze everything I say in the video and generate me
> multiple pieces of realistic B-roll that I can use later. Use Kling 4.0 inside of OpenArt. Place
> the B-roll clips in my media pool."

The shape is: *state what is open, name the folder, name the steps.* No parameters, no flags, no
file lists. **The folder is the parameter list.**

## What this workflow does not have: a check

There is no verification step anywhere in the seven minutes. No gate, no self-eval, no reading of
the render, no re-render loop. The one-shot prompt produces *"a fully cut color-graded video"* and
a pile of B-roll, and that is the deliverable.

**That is the one thing every other reference in this folder has and this one does not.** Findings
[13](13-the-llm-reads-the-video.md), [16](16-kitcut-priced-choices-checked-renders.md) and
[17](17-what-the-seven-agree-on.md) established that the field's tooling has converged on *"the
check must read the output, not the plan"* — video-use at every cut boundary, kitcut on the
finished render, framejam on the render, ffmpeg-skill on every write. The top creator workflow in
this batch has none of it.

He can afford that, and the reason is the interesting part: **his quality comes from the setup,
not from a check.** The setup is done once per client and reused; a check would have to run every
time.

That reframes our **VER-1** ("pay the verification debt") and our gate. The gate is not primarily
a quality feature — **it is a throughput feature.** It is what lets a client folder be set up once
and then edited for months without anyone re-reading the brief. If every run needed a human
looking at it, his own words would be false: *"you only have to set it up once and then you're
done forever."* **A check is what makes the setup amortisable.**

## The variant idea — new to this repo

> "You can have Claude basically take the same reel and change little tiny things, and then you can
> spam **10 to 20 different versions** of that same video on Instagram trial reels. You then see
> which one performs the best for the client, and then you post that winner to the main feed."

**N variants of one edit, differing by tiny changes, tested against each other.** This is new in
this folder — and it is the feature our architecture is closest to being able to do:

- kitcut's *"Change one thing later in one sentence — only that re-renders"* and its *"Stale
  renders flagged when a decision changed after them."*
- our own checkpoints, stale-proposal detection and `revision` field.

We have the machinery to re-render only what changed. We have no notion of a variant. **REV-1**
("moving review") is adjacent but is about review, not A/B.

And there is a taste lesson in it that connects to the whole folder: **he does not pick a winner by
eye — he picks it by what the audience did.** That is a measurement replacing a judgement, the
same move as kitcut scoring a re-cut against a professional edit frame by frame, and as the
zhuyansen benchmark in finding [10](10-the-rest-of-the-field.md). The field's hardest problem —
which look — is being answered by data wherever the creator can get at data.

## The business layer (and what the title does not show)

The title claims **$49,540/Mo**. **The video never derives it.** No revenue breakdown, no client
count, no rate card. What it does give:

- "$80 to $100 a reel" for someone with taste, editing skill and AI skills
- his own model: **monthly retainers**, and *"you guarantee virality"*
- *"1 to 200 videos a month"* as the extreme offer
- the lead machine: post AI motion graphics on Instagram, *"tell people to comment down below for
  the prompt"*, automate sending the prompt, collect the email
- *"I've been doing this for 4 or 5 years"*, and *"I'm on like week number three of trying
  different agents"*

The honest reading: the title number is a marketing claim, and the mechanism described — guarantee
virality by shipping 10–20 variants — is a **retention play**, not a pricing proof.

## The one artefact, and why we cannot use it

**OpenArt MCP** — `https://openart.ai/mcp`, endpoint `mcp.openart.ai`. A remote MCP server and
CLI: *"Generate AI images and video from Claude, ChatGPT, Cursor, or any agent… **Connect with one
URL, sign in with OAuth. No API keys.**"* It fronts 100+ hosted models (Seedance, Kling, Sora,
GPT Image, Nano Banana, Seedream) plus **Motion Control** (*"apply precise motion from reference
videos"*) and a character builder for consistency across scenes. The description's link is an
affiliate redirect (`tolt.link/linktomcp` → `openart.ai` with an influencer tag).

Two things about it worth keeping:

1. **"Connect with one URL, sign in with OAuth. No API keys."** For a consumer tool that is the
   right onboarding shape — no key to copy, no secret to paste into an agent's config file. If we
   ever expose a hosted generator, that is the shape to copy.
2. **Motion Control from a reference video** is the moving twin of *"screenshot of a look"* — the
   reference is a **moving** reference.

And the reason we cannot use it: **it is a hosted service, and our gate allows no network.**
`docs/decisions.md` refuses generative video as a core dependency (*"No editability or
determinism"*), and no-network is a gate invariant. OpenArt stays outside the app. The underlying
need is still open, and the repo already filed it as gap #8 —
`graphic_generate(prompt, seed_path, brand_id)` — and Zane's `subject assets/` folder is exactly
the `seed_path` that action needs. **This video confirms the shape of that gap.**

## What we would take

1. **A style-reference folder** — not as a new kind of look, but as *seed material for the agent*:
   a handful of reference frames it can look at while it composes. We have `frame_render`; we have
   no way to hand the agent a look to imitate. This is the piece of the field's picture-based
   conditioning we can adopt **without a model**.
2. **A client scope above the video.** Zane separates *clients* by folder; we separate *videos* by
   project. Our rule is *"one look for every graphic **in a video**"*, so a client-level scope sits
   above it without breaking it.
3. **The variant.** N near-identical renders, only the changed thing re-rendered. We have
   checkpoints and stale detection for exactly this, and no notion of a variant.
4. **Confirmation of the short prompt.** Our `preset` tool already does this; the video is
   evidence that the pattern is right and worth keeping rather than drifting away from.

## What this implies outside this folder — **not applied**

Nothing outside `docs/minimax-research/` was changed. For the record, the earlier wave's
`research/creator-workflows.md` would want three things from this video:

- **Artefact index:** *OpenArt MCP* — `openart.ai/mcp`, endpoint `mcp.openart.ai`: remote MCP + CLI,
  OAuth, **no API keys**, 100+ hosted image/video models. The service behind Zane's B-roll, and
  hosted, so outside our no-network gate. And the **Zane master-prompt Google Doc should be marked
  superseded** — his 24 Oct video publishes no file at all.
- **"Tricks worth stealing":** three rows — keep the method in a folder next to the footage so the
  prompt can be one sentence; ship 10–20 near-identical variants and pick the winner by what the
  audience did, not by eye; keep style references as images the agent looks at, beside the words
  that describe them.
- **"Corrections":** the 24 Oct video has **no verification step at all** — the one-shot prompt
  produces the deliverable — which is the argument for our gate being a **throughput** feature
  rather than a quality one. It also breaks the *"the artefact is the product"* rule: a method
  with no file behind it is now something he teaches and sells access to.

## Caveats

- **Captions are ASR.** "GPT-6 Astra", "DaVinci Resolve", "OpenArt" and "Kling" are transcribed,
  not read — the description is the reliable source for names. The video says *"Kling 4.0"*;
  OpenArt's public model list at capture time says **Kling 3.0**. Either the site lags, the
  creator misspoke, or the ASR is wrong. **Do not quote a version.**
- **The folder structure was shown on screen, not published.** There is no prompt, no skill file
  and no document behind this video — so unlike his 8 Oct video (whose artefact was a $700
  master-prompt Google Doc, still in our artefact index), this one publishes a method with no file
  behind it. Our capture is the description plus 239 ASR snippets.
- **The business numbers are marketing.** $49,540/Mo, "six-figure business", *"paid more than a
  brain surgeon"*, "$80–100 a reel", "1 to 200 videos a month" — none is derived in the video.
- **34,494 views against 184,640 for the 8 Oct video.** This is a how-to sequel, not a stunt.
  Views are not a measure of the method.
- **No verification means we cannot tell how good the output actually is.** Nothing in the video
  shows the finished result being checked, so every quality claim rests entirely on his reasoning
  about the setup.
- **His stack is a Mac with multiple laptops, DaVinci Resolve and hosted tools.** Ours is a
  Rust/Tauri app with a pinned local runtime and a no-network gate. The *shape* is comparable; the
  stack is not, and his one-line prompt is only possible because DaVinci exposes a rich API to an
  agent — which is exactly the risk our own decisions.md records under *"Driving Premiere or
  another NLE by computer use: No — fragile UI, high upkeep; our typed API owns edits."*
