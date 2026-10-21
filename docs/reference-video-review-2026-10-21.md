# Reference video review — AI video editing and motion graphics, October 2026

Reviewed: 21 October 2026. Eleven videos supplied by the project owner, reviewed against
[AeroEdits](../README.md), the MCP server and the skill layer.

Companion document: [agent-video-gaps-and-proposals-2026-10-21.md](agent-video-gaps-and-proposals-2026-10-21.md)
— what this review implies for the product.

## Method and evidence limits

Each video was located, dated and read from its published captions or the creator's own written
walkthrough. Where a creator published a companion article or repository, that was used in
preference to inference. Numbers (cost, duration, token counts) are the creator's own claims and
were not independently verified.

**Updated 21 October 2026:** full captions were since retrieved for all eleven videos and are
published in [transcript.md](transcript.md). An earlier draft of this review could not read #2's
captions; that limitation no longer applies and the caveat below has been removed. The full
transcripts also sharpen several points below — most notably Joseph's visual guide, which turns out
to have **three** parts rather than two.

View counts are as reported by the aggregator at the time of reading and move daily; treat them as
order-of-magnitude interest signals, not stable figures. Everything below is transcript- or
article-sourced.

---

## The eleven videos

Titles link to the video. Full captions for all eleven are in [transcript.md](transcript.md); the set is
reproducible via [reference-video-scripts/](reference-video-scripts/).

| # | Video | Channel | Date | Length | Interest |
|---|---|---|---|---|---|
| 1 | [How to Edit VIRAL Instagram Reels Using Opus 5.5! (New Style in 2026)](https://www.youtube.com/watch?v=5UnK3c-nlIs) | Joseph \| Video Editing | 11 Oct | 17:16 | ~67–85K |
| 2 | [Claude Opus 5.5 Changed Motion Graphics Forever (Insane Results)](https://www.youtube.com/watch?v=5XSwP0DaHF4) | Tao Prompts | 10 Oct | 20:42 | ~28–63K |
| 3 | [The NEW Way to Edit Videos Using GPT-6 Astra \| Full Guide](https://www.youtube.com/watch?v=QwzVmGov9ec) | Tao Prompts | 17 Sep | 14:41 | ~30K |
| 4 | [How I create viral animated reels using HyperFrames + Claude Code](https://www.youtube.com/watch?v=y4YupFxhAog) | Science Based AI | Sep | 14:29 | ~36K |
| 5 | [Is Astra 6 Actually Better Than Fable 5.1? (Motion Graphics Test)](https://www.youtube.com/watch?v=CgYGY0Mn1PE) | Marcin AI | 5 Sep | 9:12 | — |
| 6 | [GPT 6 Astra Is INSANE for Motion Design](https://www.youtube.com/watch?v=eyDKoGU_vHk) | Chase AI | 8 Sep | 8:41 | ~71K |
| 7 | [GPT-6 Astra + After Effects Creates Motion Graphics in Minutes!](https://www.youtube.com/watch?v=W3-RIZ-Ps64) | Chase AI / Higgsfield | Sep | — | 7-workflow breakdown |
| 8 | [GPT6 Astra one-shots motion graphics like never before (NEW Skill)](https://www.youtube.com/watch?v=Ce1Ym1HGyBA) | Jay E \| RoboNuggets | 15 Sep | 14:29 | ~47K |
| 9 | [How To Make Insane Motion Graphics With Opus 5.5 (Claude Motion)](https://www.youtube.com/watch?v=AsvN_feasyY) | kaye.creatives | 14 Oct | 12:24 | — |
| 10 | [Claude Opus 5.5 Animated Every Motion Graphic in This Video (I Didn't Keyframe a Thing)](https://www.youtube.com/watch?v=Ll45R_Z8tQc) | Matt Loui | 15 Oct | 15:23 | — |
| 11 | [GPT-6 Astra Just Unlocked Motion Design + After Effects](https://www.youtube.com/watch?v=C8dWdic-oK4) | Chase AI | 14 Sep | 8:36 | ~47K |

Sources cited in passing but **not** in the set: Chase AI's four-case blog comparison, Nate Herk's
15-case Astra-vs-Fable test, rewind@rewind02's hands-on audit of the `video-use` skill, and Vane
Motion's 24-minute squash-and-stretch and hand-rigging test of Astra inside After Effects.

---

## Part 1 — The editing videos (1, 3, 4)

### 1. Joseph — VIRAL Instagram Reels with Opus 5.5

**What it is.** Joseph claims he cut a 76-second raw take down to 50 seconds, built 12 animations,
wrote the captions and placed 106 sound effects, using only Claude Opus 5.5 — no Higgsfield, no
Premiere. Whole edit under two hours, of which he worked twenty minutes, for $1–5 on a normal plan.

**The workflow, in his order.**

1. **Train on a house style first.** One large visual guide covering three things: captions,
   animations, on-screen UI. The caption guide is not "use captions" — it is a reference image, a
   font pair (one medium sans for most words, one script font for the words that matter), a colour
   set, and a placement rule (speaker always on screen, subtitles in the lower half, captions
   highlight *the one word* that makes you keep watching). The animation guide splits every animation
   into four types — **text-centric, person-centric, object-centric, graphic-centric** — with a
   description of what each looks like, plus the background construction, a **"three-object rule"**
   (one main object centre, two supporting elements at the edges, depth elements outside), design
   basics, and **100+ reference pieces** (UIs, graphs, full scene designs) dropped in as a folder.
   A second, separate guide covers sound design and his own SFX library.
2. **Turn both guides into skills**, so he never re-trains.
3. **Cut.** Word-level transcript, remove every silence and every bad take.
4. **Plan on a page.** Claude builds a planning page from the transcript; per line Joseph picks a
   scene type — *captions* (him talking with text), *captions with UI* (text plus a small animation
   in the lower half), and fuller animation types. Fifteen minutes of his time.
5. **Build.** Claude opens Remotion and builds every animation from the training plus the plan.
6. **Sound design.** ~25 minutes to place every SFX; then the music track dragged in at −20 dB.

**What matters.** The video's real subject is not Opus 5.5 — it is that the quality lives in the
**training artefacts**, and the artefacts are of three kinds: a *style guide* (fonts, colour,
placement, banned looks), a *taxonomy* (four animation types, so planning is a choice and not a
blank page), and a *reference library* (100+ real designs to imitate). The model is the cheap part.

**Also worth noting.** He closes by naming the four "viral editing styles of 2026" — cinematic short
film, editing fundamentals, minimal animation (which he calls the one currently dominating), and
SaaS motion design. That is a segmentation of the market, not a workflow, but it is the vocabulary
he uses to pick a look per client.

### 3. Tao Prompts — The NEW Way to Edit Videos Using GPT-6 Astra

**What it is.** The most procedural of the set. GPT-6 Astra in Codex, plus HyperFrames, editing a
12-minute talking-head plus screen recording into 2:34. Total elapsed 49m30s; install alone 8m45s.

**The workflow.**

1. Install the ChatGPT desktop app, select Codex mode, create a project folder.
2. Point the agent at the HyperFrames repository and let it do the setup. **You do not install
   anything yourself.**
3. Organise the project: raw recordings, overlay clips, backgrounds.
4. **Write a brief document first.** Not a script — a brief like the one you would hand a human
   editor: the task and its type, where the footage lives, a bullet outline of what is discussed,
   which specific clips go at which parts, and visual references for the style.
5. **Upload the brief and make the agent confirm it** — that the information is clear and it can
   actually reach every asset — and fix what it finds missing.
6. Then, and only then, edit.
7. **Ask for timing and a speed analysis.** He tells the agent to log how long each step took and
   what could be sped up.

**What matters.** Two transferable ideas, both cheap. First, the **brief-as-contract** step: the
agent is not allowed to start until it has confirmed it can access everything and understands the
ask. That single gate is the difference between a reproducible workflow and a lucky render. Second,
**per-step timing** — turning the run into a performance profile of the pipeline, so the next run is
faster by construction.

**Independent corroboration.** rewind@rewind02 separately cloned and ran both skills and confirmed
the pipeline order (transcribe → cut → plan the beats → render) and the crucial guardrail that the
skill **requires approval of the strategy before it touches the cut**. Their write-up also
reproduces `video-use`'s hard rules verbatim, which are worth holding on to: subtitles applied last
in the filter chain; per-segment extract and lossless concat rather than one filtergraph pass; a 30 ms
audio fade at every cut boundary; never cut inside a word; pad every cut edge 30–200 ms to absorb
transcript drift; word-level verbatim transcript only; parallel sub-agents for multiple animations;
strategy confirmation before execution.

### 4. Science Based AI — viral animated reels with HyperFrames + Claude Code

**What it is.** A content lead at a biotech startup showing what actually drove 200,000+ short-form
views in 30 days, with the useful discipline of stating that many HyperFrames demos online are *not*
mostly made in HyperFrames.

**Three use cases, all starting from the raw material rather than from a blank canvas.**

1. **Vox-style animated data charts.** Collect reference images, have Claude find the data (his
   example: the ten most populous cities in Africa), prompt a 7-second 9:16 animated bar chart,
   then **edit it in chat until it fits inside short-form safe zones.**
2. **Text and visual explainers.** Give Claude a reference style, colours and fonts, hand it the
   script, and ask *where animations should go*. Then have it write standalone prompts so several
   Claude Code chats can build clips **in parallel**.
3. **Liquid-glass titles.** Point Claude at Apple's liquid glass design docs and get a clean pop-up
   title board you can drop a logo into.

**The six tips, which are the actual deliverable.** Use reference images. **Generate shots, not whole
videos.** Be specific about camera motion (push in, pan, static). Iterate like an editor. **Leave
timing, music and captions to post.** Borrow a named visual language — Vox, Apple, luxury fashion,
tech startup, documentary.

**What matters.** "Generate shots, not whole videos" and "leave timing, music and captions to post"
are the two that contradict the rest of this set, and they are the most useful things in it. Together
they say: generate *assets and beats*, keep *sequencing* deterministic. That is exactly the division
of labour AeroEdits already has — a deterministic timeline the app renders, with graphics supplied
as self-contained clips that never touch the edit.

The parallelism point also matters practically: splitting a short into independently-buildable beat
prompts so several agent chats can work at once is a scheduling decision, not an aesthetic one, and
it is available to us today because each of our graphics is a folder with a render cache.

---

## Part 2 — The motion-graphics videos (2, 5, 6, 7, 8, 9, 10, 11)

These split cleanly into three camps, and the camps differ in *who does the animating*, which turns
out to matter more than which model is used.

### Camp A — the model drives After Effects through a plugin

Videos 6, 7, 11, plus the wider ecosystem they belong to (Higgsfield, Skill With AI, Aidan Stanik,
Vane Motion, Bad Decisions Studio).

**The setup is identical in all of them.** Install After Effects. Install the free, open-source
**Higgsfield Motion Designer** plugin. In Codex or Claude Code invoke `higgsfield/use-after-effects`
or type "use the After Effects skill". The model then reads the AE project, writes and executes
script inside AE, builds layers, comps, expressions, rigs, and renders.

**What each adds.**

- **#6 (Chase AI, INSANE for Motion Design)** — and this one is *not* After Effects. A free GitHub
  motion-design skill plus the Higgsfield MCP. The skill holds a **motion library** — reference
  videos as a style gallery — and writes the detailed prompt; Higgsfield receives that prompt and
  runs **Seedance 2.5** to generate the motion; image generation defaults to GPT Image. Inputs can be
  text, a reference image, a reference video, or a URL. The system has a **self-correction loop**:
  generate, compare against the prompt, regenerate only the part that is off. Demonstrated on a 2D
  explainer, an exploded product film and a Canva campaign — and notably it generated the audio too,
  walking through how noise-cancelling headphones work, with a full soundscape.
- **#7 (seven workflows in minutes)** — the widest catalogue in the set: a promo film from a brief
  into **Figma storyboards** and then animated in AE; an **ASCII effect** with a custom converter the
  agent codes for you; a SaaS promo with real falling-label physics; dynamic captions and generated
  cutaways for talking-head; illustrations **rebuilt as editable shape layers and rigged**; **localising
  a finished project into another language**; and **resizing 16:9 to 9:16 without breaking the
  layout**. Higgsfield generates the visuals, Astra runs the process, you keep an editable AE project.
- **#11 (Chase AI, Unlocked Motion Design + AE)** — the explicit three-step loop: **storyboard first**
  (Astra generates the storyboard itself, because it has image generation), **let Codex write the
  prompt, then build** (22m56s for a 15-second video, zero Higgsfield credits burned), and **edit with
  prompts, not panels** — including audio: sound design and background music are prompted too.
  Codex checks its own work against the storyboard, kicks off the render, checks the final render,
  and presents the finished video inside Codex.

**What matters.** Three things, and the third is uncomfortable.

1. **Storyboard before build, self-check against the storyboard.** This is a planning artefact and a
   verification loop, not a renderer feature.
2. **Reformat and localisation are first-class operations.** A finished 16:9 project must survive a
   trip to 9:16 and to another language without being rebuilt.
3. **The model that picks the renderer is not the model that makes the pixels.** When Astra drives
   Seedance 2.5 through Higgsfield, Astra is a router and a prompt writer. The visual quality is
   Seedance's. This is exactly what #5 measures, and it is the most important measurement in the set.

### Camp B — the model writes code, a headless renderer draws it

Videos 2, 8, 10, and the wider HyperFrames / Remotion ecosystem (#3, #4, Nate Herk, humzaakhalid).

**#8 (RoboNuggets) is the clearest statement of method in the whole review**, because it is about
skills rather than output. He builds an `/animate` skill in one prompt — ask Astra to use
HyperFrames, AssemblyAI and the agent platform, and it scaffolds. Then he makes the real argument:
there are **unrefined skills and refined skills**, they do the same job, and the unrefined one only
produces the model's default taste. Refinement is three steps:

1. **Be aware of the tools.** Know exactly which three things the skill is allowed to reach for
   (transcription, the render engine, the agent platform).
2. **Review the output and give feedback, in the same working session.** Two levers: **add rules**
   (e.g. "numbers switch with a big jump animation", "use the mobile layout for the screenshot"), and
   **hand it a design system as a Markdown file copied off a website** so the agent stops defaulting
   to its own taste. Then build an **element library** — 3D graphics, browser mockups, motion
   elements, infographics and diagrams, plus reusable images, all made by the agent and kept as a
   pick-from-the-shelf library.
3. **Ask the agent to update the skill** so the corrected rules persist into every future run.

**#10 (Matt Loui)** is the same idea applied to references: build from what is real, not from what
the model imagines. He prompts Claude three ways — **record it** (a screen recording of the real
product becomes an animated UI mockup), **screenshot it** (three real screenshots become a workflow
breakdown), or **reference it** (one Pinterest motion-typography reference becomes a 25-second promo).
He is explicit about why: because every animation is built from his real screen recordings, they look
like the actual software rather than AI slop. He then renders 4K and does a five-minute finishing pass
in CapCut — chromatic edges, radial blur on the corners, and sound design with whooshes, UI pops and
line sounds.

**#2 (Tao Prompts)** is the capability showcase for the same camp: Opus 5.5 writing the animation as
code, across styles, with the canonical prompt "make a dynamic 15-second motion graphics video that
shows what an incredible motion designer you are… go all out."

**What matters.** **The reference is the input, and it must be real.** Matt Loui's recordings and
screenshots are a stronger idea than any prompt trick, because they pin the design to something that
already exists. The wider `awesome-claude-video-skills` and `awesome-opus-5-5-video-styles` catalogues
(425 prompts across 12 named visual styles; 357 motion-graphics examples with prompts) are the same
insight industrialised: a named style is a prerequisite for a repeatable look, because
*"Opus 5.5 doesn't output video: it writes code that draws every frame, so the same prompt can come out
as kinetic type one run and 3D the next. To get a look, you have to name it."*

One artefact from `motion-graphics.org` is worth stealing wholesale, because it is a style kit
expressed as a negative list: **banned — crossfades, blur-ins, brightness "developing", 3D flips,
particles, glows, holds longer than 1 s, anything that looks like a template.**

### Camp C — a product surface inside the chat app

Videos 9, and the related "Claude Motion" / Claude Artifacts push.

**#9 (kaye.creatives)** makes a point the rest of the set buries: **you do not need a coding agent
for this.** Open the ordinary Claude chatbot, go to Artifacts, click Motion. No Claude Code, no After
Effects, no terminal. Connect the Higgsfield MCP so Claude can generate visual elements and media
directly in the chat, drop in raw unedited video and the script, upload **multiple** reference clips
as the "secret to walking in your exact visual direction", attach a custom AI skill that acts as an
automated prompt generator supplying all the motion design rules and code structures, then describe
what you want in plain sentences — *"I'm not saying anything technical, I'm not naming a single tool,
I'm just talking to it exactly how I'd talk to my motion designer."* Claude maps out every phrase,
and revision is done by typing feedback into the same box.

He runs the exercise twice: once over raw footage, once building a launch intro from nothing. The
second case has no raw video, so the whole thing is generated from references plus the skill.

**What matters.** This is the **no-code surface** end of the same product, and it argues that the
skills layer, not the terminal, is what makes this reachable for non-technical creators. It also
confirms the plain-English-brief finding from #3: the brief is the interface.

### #5 — Astra vs Fable 5.1, and the finding that matters most

Video #5 is itself a head-to-head: Marcin AI, same single prompt in both models. Alongside Chase AI's
four-case comparison and Nate Herk's fifteen, there are three results in front of us, pointing in
different directions. The difference between them is the most useful thing in this review.

- **Chase AI's four-test comparison** put motion graphics in the same bracket as a browser game, a
  landing page and a 3D globe dashboard. Astra won three, and the motion-graphics test was a **tie**.
  His explanation is the important part: he connected *both* models to the Higgsfield MCP, had both
  call the same motion-graphics skill, and both routed the prompt correctly and returned polished
  15-second explainers. **The real takeaway was not about the models — it was that both are excellent
  at orchestrating external tools through MCP.** When the heavy lifting is offloaded to a specialised
  service, the model's job is routing and prompting, and both do it well.
- **Nate Herk's 15-case comparison** put Astra **ahead** on the motion-graphics sizzle reel, calling it
  more energetic, with 3D elements, shadows and a stronger sense of depth — and crediting the model
  for going and finding real material from the thing it was promoting, a better creative choice than
  keeping everything abstract. Across all 15 cases, Astra won 10 and Fable 5, at $327 against $513.

**The reconciliation is the finding.** In Chase AI's test the pixels came from Seedance 2.5, so the
model could not express its difference and both looked the same. In Nate Herk's test Astra was doing
the work itself — two prompts and 20–30 minutes for a music-synced 3D motion piece, and a 152 GB
event-recap cut in 35 minutes. **Model choice only matters when the model is the renderer.** A model
routing to an external generator is interchangeable with any other good model on the same generator.

But **video #5 in our own set pulls the other way, and it is the direct counter-test.** Marcin AI
runs the identical single prompt in both models on *ultra* mode and lands on the opposite verdict:
*"Astra 6 crushed this. I gave it the most simple prompt to see what it would actually do on its own.
The only thing it needed was Hicksfield MCP, and also remotion."* Same Higgsfield MCP that produced
Chase AI's tie. The difference is what each model chose to put on screen: Marcin credits Astra for
music, sound effects, transitions, better text, better animation, and — the tell — **video assets
rather than stills**, sourced from real demos. In the second round, given only "find the most viral
content about Astra 6 and put those demos in a new video," Astra finished and shipped while Fable 5.1
was still rendering.

So the tie and the win are not actually in conflict, and the correction to my own framing is this:
**the external generator was not what flattened Chase AI's result — the flatness came from both models
converging on the same assets.** When the model chooses what to fetch, whether real footage or stock
stills, its judgement is visible even when a third-party generator draws the frames. The stronger
claim holds in both tests, with the boundary moved: model choice is visible when the model exercises
**judgement about inputs**, not merely when it renders pixels.

This is a direct statement about where AeroEdits sits, and it is favourable: we render deterministic
compositions ourselves, and every graphic we ship is one the agent chose to place at that moment.
There is no external video model in the loop whose output would flatten every composing model to the
same result. Our output quality moves when the composing model gets better.

It also carries a warning. Nate Herk's own conclusion is that the jump in quality came *less* from
the model than from three inputs: strong reference inspiration, a **pain-person-promise** filter on
every design choice, and a purpose-built prompting skill. The models are the cheap part in both camps.

#### Three more head-to-heads, found while collecting transcripts

Outside the eleven, but they bear directly on the reconciliation above. Taken together they are less
decisive than the tests above, and the disagreement between them is the useful part.

- **Andy Diep (9 Sep)** — six pairs, model labels hidden, scored blind. Round wins were **3–3**. His
  summary scores were near-identical (Fable 3.33/5, Astra 3.50/5). But his purchasing line splits them:
  *"I would actually pay for three of Astra's render and only one of Fable's."*
- **Matt Penny (23 Sep)** — same prompt, same skill, cost and speed tracked. Fable 5.1 finished in
  11m14s at £5.90; Astra 6 in 11m09s at $15.05 — roughly double the cost for what Penny judged an
  unremarkable gain. His conclusion is the sharpest criticism in the set: Astra *"looks technically
  more impressive, but does things which just don't make a lot of artistic sense that a human would
  never do."*
- **Luke Byrne (6 Oct)** — four models on the same raw footage and the same saved editing skill.
  Opus 5.5 best overall, and his framing is the nuance worth keeping: **Opus is better consistently,
  Fable 5.1 makes the better individual graphic** — better at the animations, colours and shading. The
  two GPT models, he says, cut the footage fine but the visuals *"felt like glorified slides."*

Read together: on isolated, heavily art-directed clips Astra is competitive and sometimes better;
on **full edits driven by a skill**, the picture is consistent rather than spectacular, and cost can
double. That is a fair warning for anyone whose plan is "one prompt, whole edit" — and an argument
for AeroEdits' existing shape, where the deterministic passes do the editing and the agent supplies
graphics and direction rather than authoring the entire cut.

---

## Part 3 — The patterns underneath all eleven

Strip the tools out and the same eight things are being sold, over and over, in different words.

**1. A house style, written down, as the actual input.** Joseph's caption/colour/font/placement guide,
RoboNuggets' design-system Markdown, Matt Loui's reference folder, Nate Herk's Scrollcraft skill,
kaye's custom skill, and the 12-style prompt catalogues. Not "make it look good" — a document with
rules and references in it.

**2. A taxonomy, so planning is a choice rather than a blank page.** Joseph's four animation types
(text / person / object / graphic) and his per-line scene picker are the clearest instance. The model
does not invent a scene; it fills a slot the user has already chosen.

**3. Plan, then approve, then build.** Tao's brief-and-confirm, Chase AI's storyboard, Joseph's
planning page, and `video-use`'s hard rule that nothing touches the cut until the strategy is approved.
Every mature example has an approval gate between planning and execution.

**4. Real references over invented ones.** Record it, screenshot it, reference it (Matt Loui). "Because
every animation is built from my real screen recordings, they look like the actual software, not AI
slop."

**5. The agent verifies its own work.** Chase AI: Codex checks against the storyboard, renders, checks
the render, presents the result. Nate Herk lists "let the agent verify its own work" as one of his five
habits. Joseph reviews the edit.

**6. Sound design is half the product.** Joseph placed 106 SFX and dropped music at −20 dB. Matt Loui
finishes with whooshes, UI pops and line sounds. Chase AI's generated videos carry a full soundscape,
and #11 explicitly prompts audio. Not one of the eleven treats sound as an afterthought.

**7. Turn the result into a skill, then keep refining it.** Joseph explicitly, Nate Herk explicitly,
RoboNuggets as his entire video. The first prompt is long; every prompt after it is short. Feedback is
written back into the skill, not into the session.

**8. Reformatting is a feature, not a rebuild.** 16:9 to 9:16 without breaking the layout; localisation
into another language; safe zones for short-form. #7 and #4 both treat this as routine.

Two smaller threads are worth keeping: **timing instrumentation** (Tao asks for per-step time and a
speed analysis), and **market segmentation into named styles** (Joseph's four 2026 styles, the 12-style
catalogue, "Vox / Apple / luxury fashion / tech startup / documentary"). A style you can name is a
style you can sell.

---

## Part 4 — Numbers worth remembering

Claims are the creators'; nothing here was independently verified.

| Claim | Source |
|---|---|
| 76 s → 50 s, 12 animations, 106 sound effects, <2 h, 20 min of human work, $1–5 | Joseph #1 |
| 12 min → 2:34, 49m30s total, 8m45s of that install | Tao #3 |
| 200,000+ short-form views in 30 days | Science Based AI #4 |
| 15 s of AE motion in 22m56s, zero Higgsfield credits | Chase AI #11 |
| 152 GB event footage → 1-minute recap in 35 minutes | Nate Herk |
| Motion-graphics test: a **tie**, because both models drove the same external generator | Chase AI #5 |
| Astra 10 – Fable 5 across 15 cases; $327 vs $513 | Nate Herk |
| 25 minutes to place all sound effects; music at −20 dB | Joseph #1 |
| 25 min / $5 for a 15-second showreel, 3 revisions | Higgsfield AI, via motion-graphics.org |

The last two rows are the pair to keep in tension. When the model routes to a generator, model
choice stops mattering and the cost is whoever renders. When the model is the renderer, model choice
is the whole ballgame — and the cost is tokens and minutes, not credits.

---

## Part 5 — Where the ecosystem actually is

A scan of the adjacent landscape during this review, for orientation only.

- **HyperFrames** (HeyGen, Apache-2.0, ~68k stars) is the strongest of the widely-shared "Claude
  video skills" and is the only one of that viral list that is safe for paid work as a genuine skill.
  It ships ~21 skills agents load on demand, including `/faceless-explainer` and `/motion-graphics`,
  a router skill that picks the workflow, Node 22+ and FFmpeg, and a CLI with a lint/contrast/motion
  check. A 15-second brief reportedly took 23.5 minutes.
- **Remotion** is the React-based rival, with official AI skills and Remotion Studio for scrubbing and
  preview. It is the camp Joseph and Matt Loui are in.
- **The Higgsfield AE plugin** is the third pillar, and the only one that produces an editable AE
  project — the deliverable professionals actually ask for.
- **Seedance 2.5** via Higgsfield is the generative-video backend several skills call.
- A large body of single-purpose skills has appeared around this — beat-synced reels, ASCII, map
  animations, whiteboard explainers, dubbing pipelines, b-roll engines — almost all on HyperFrames or
  Remotion, all solving one narrow problem.

Our own [motion-graphics-research.md](motion-graphics-research.md) already reached the conclusion that
Higgsfield's only established advantage is native AE handoff and that our HyperFrames skills already
cover the advertised work. This review does not overturn that. What it adds is the **skill, reference
and sound** layer that sits *above* whichever renderer is chosen — the layer every video in this set
is actually selling.