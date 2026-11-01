# beamnxw/my_works — an AI knowledge base of motion and harness engineering

Pulled 27 October 2026 from <https://github.com/beamnxw/my_works>.
Commit `00d9b1cce52c` (2026-10-25T12:48:49Z) — Add fourth knowledge base entry: Opus 5.5 Motion Studio Harness. Repo created 11 Oct 2026, last pushed 25 Oct 2026,
27 stars, 6 forks, MIT. Self-described as *"AI knowledge base | guides, tutorials"*.
The author is **beamnwx**, an independent creator (@beamnxw) selling launch videos.

The repo has no source code — it is **four authored artefacts**: three PDFs and one ZIP of
markdown files. It is the clearest written statement of the workflow the twelve creator
videos in [synthesis.md](../synthesis.md) only showed by example.

## The four entries

| # | Artefact | What it is | Size |
|---|---|---|---|
| 1 | `beamnxw-harness-engineering-600.pdf` | 35 pp. **600 building blocks** for long-running agents, in 30 categories | 272 KB |
| 2 | `motion-engineering-opus55-paper.pdf` | 14 pp. **Production protocol** for building launch videos with Opus 5.5 + Remotion, with worked example, prompts and handoff | 138 KB |
| 3 | `beamnxw-motion-design-studio-600.pdf` | 38 pp. **600 motion briefs** in 30 categories, with 46 technique references | 718 KB |
| 4 | `opus55-motion-studio-harness.zip` | The eight-file harness that implements the protocol | 19 KB |

---

## 2. The production protocol (the important one)

*Motion Engineering with Opus 5.5: Build Launch Videos You Can Sell.* Original article on the
author's X account (<https://x.com/beamnxw/status/2108616740846420354>). The PDF is the source.

Its own abstract is careful: *"the example supplies design choices rather than measured
performance or sales results"*, and the closing note says it reports *"no completed video
benchmark, client sale, or measured conversion result"*.

### The twelve steps

| # | Step | Artefact |
|---|---|---|
| 1 | Choose an action | — |
| 2 | Inspect assets | `public/` |
| 3 | Study a reference | `REFERENCE.md` |
| 4 | Map shot events | event frames |
| 5 | Define the layout | layer order, coordinates |
| 6 | Write the brief | `BRIEF.md` |
| 7 | Stage the opening | stills + preview |
| 8 | Inspect and repair | scoped corrections |
| 9 | Align the sound | waveform |
| 10 | Verify the export | ffprobe |
| 11 | Package the clip | MP4 + cover |
| 12 | Review economics | scope and price |

### The five-part brief

*Goal* (one product action, its viewer, the opening outcome, the closing destination) ·
*Delivery and assets* (composition, dimensions, frame rate, duration, required assets) ·
*Layout* (coordinate origin, safe area, type limits, fitted screen, measured interaction
anchors, layer order) · *Timeline* (initial and final states, event frames, curves, reading
holds) · *Implementation and review* (deterministic rendering, editable constants, fixed seeds,
the first stage, required stills, feedback checkpoints).

### Frame conventions

```
f = round(t x fps)                 # seconds to frames; state the rounding rule
frame_count = end - start + 1      # inclusive global intervals
duration_s = frame_count / fps
global_frame = sequence_start + local_frame
```

The worked example is a 20-second 1080x1080 composition at 30 fps, 600 frames, with events at
**120** (report settled), **150–180** (cursor approaches), **195** (press registers), **210**
(file appears), **234** (label fully readable), **420** (closing begins), **438** (settles),
**599** (end).

### The four-field correction format

This is the sharpest single idea in the protocol, and it is the exact shape of the feedback
the twelve creator videos all converge on.

| Field | Worked example |
|---|---|
| **Location** | Global frame 210 |
| **Observed defect** | Cursor overlaps result label |
| **Change** | Departure 204–210; 24 px clearance |
| **Preserve** | Press 195, reveal 210, card, copy, sound |

The rule around it: *"Repair missing information and readability first, then timing, then
visual detail"*, and *"Update only the implicated values, check their affected frames and
neighboring boundary, and confirm the edit at normal speed."* The appendix carries the whole
repair as a reusable prompt (A.3):

```text
Observed defect: the cursor overlaps the result label at global frame 210.
Change only the cursor's post-press path:
  Start the departure at 204 and reach its resting point by 210.
  Choose the resting point inside the card, with 24 pixels of clearance from the result label.
  Use a path that stays clear of the label throughout the move.
Preserve the press at 195, reveal at 210, card geometry, copy, and audio cues.
Hold the cursor at its destination through 419.
Render stills at 195, 210, 222, 240 and a preview of 180 through 249.
```

### The reading-hold rule

*"Count the reading window from the first frame where the words can be read. Eight frames at
30 fps give about 0.27 seconds. That interval can be too short even when an entrance looks
smooth."* The worked example gives the result a hold of **234 through 419 = 186 frames =
6.2 seconds**. It also asks for a *"check interface labels at phone size; a large preview can
hide an illegible product crop"*, and says to *"play the scene at normal speed and read
aloud"*.

### The staged gate

The opening is built first and approved before anything else is made:

```text
npx remotion still Launch out/opening.png --frame=60
npx remotion still Launch out/product.png --frame=240
npx remotion still Launch out/closing.png --frame=540
npx remotion render Launch out/transition.mp4 --frames=90-149 --codec=h264
```

*"Render global frames 0, 18, 42, 60, and 119 plus its moving preview … Return the files
and checks, then obtain feedback on this stage. After direction is approved, build the product
and closing scenes."* An optional *"independent critic"* receives the brief, images and
source frames and may return *"up to five specific corrections"*.

### Reference analysis

```text
ffmpeg -i reference.mp4 -vf "fps=2,scale=960:-2" frames/ref-%04d.jpg
```

*"Two images per second provide an overview. Extract consecutive source frames around the
entrance, landing, press, and cut before assigning exact event timing. Sparse samples can
miss both overshoot and the true cut."* Measurements are labelled **measured**, **estimated**
or **proposed**, and an uncertain estimate must name the frames needed to verify it. It also
warns against coordinate confusion: *"An overview resized to 960 pixels wide does not have the
coordinate system of the original video"*, with the conversion
`source_x = viewed_x * source_w / viewed_w`.

### Sound

*"Locate the click transient and reveal hit with waveform inspection and playback. Offset each
clip so the audible event lands on the visible press at 195 or reveal at 210."* Music and
effects stay on separate tracks with editable placement and volume, and the ending is checked
through headphones **and** a phone speaker.

### Export verification

```text
npx remotion render Launch out/launch.mp4 --codec=h264
ffprobe -v error -select_streams v:0 -show_entries stream=width,height,avg_frame_rate,duration \
  -of json out/launch.mp4
```

*"A successful render command alone does not establish that the message or interaction is
readable."* The delivery is the accepted MP4, one cover still, the approved brief, the cue
list, the commands and a verification report — and *"If a check cannot be performed, state
that limitation."*

### Commercial framing

The last section packages the method as a product: *"Launch Clip", one feature, one approved
script, a 20-second 1080x1080 MP4, one cover image and **one revision round**". The worked
arithmetic is labelled illustrative — *"A $150 pilot with $20 direct costs leaves $130 before
tax and overhead. Four hours of work imply $32.50 per hour."*

---

## 4. The eight-file harness

`opus55-motion-studio-harness.zip` — the protocol as a project the agent can be pointed at.

| File | Stage | Role |
|---|---|---|
| `CLAUDE.md` | 0 | the method file: stage sequence, project structure, what each file owns |
| `BRIEF.md` | 1 | film brief, default delivery contract, fillable production input |
| `STYLE.md` | 1 | visual tokens, light and material, "specify the scene as a designed frame" |
| `ASSETS.md` | 1 | asset manifest; product states; missing inputs; replaceable content contract |
| `STORYBOARD.md` | 2 | global storyboard: starting layout, worked event schedule, scene record format |
| `MOTION.md` | 3 | shared motion system: frames and endpoints, movement direction, track helpers |
| `SOUND.md` | 4 | sound cue sheet: align the audible hit, mix in context, cue record |
| `DELIVERY.md` | 5 | render and handoff; review the exported file; return useful files |

Plus `START.txt`, `LAUNCH-PROMPT.txt`, `FULL-PROMPT.txt` and `SOURCES.txt`.

### The clauses that matter

From `CLAUDE.md`:

- *"Markdown guidance is context, **not an enforcement mechanism**."*
- *"Ask up to three questions together only when missing answers would materially change the
  film. Record assumptions for ordinary creative decisions and keep working."*
- *"Never present an invented screenshot, feature, metric, or customer result as real."*
- *"If rendering is blocked, keep the sources and report the specific missing dependency or
  access."*
- *"Keep decisions in the relevant file: the brief owns the outcome; storyboard owns event
  frames; style owns visual tokens; motion owns curves; sound references the event frames.
  Avoid conflicting copies of the same value."*
- *"A revision should change the responsible file and dependent tracks together."*

From the paper's handoff prompt:

- *"If this chat cannot write files or render video, explain the missing tool access and
  produce the brief and shot plan only. **Do not claim to have rendered an output.**"*
- *"Do not publish the video, send a pitch, accept terms, spend money, or claim a sale
  without my authorization."*
- *"Adapt these values only with an updated brief and a stated reason."*

### Setup it assumes

```text
npx create-video --yes --blank launch-video
cd launch-video
npx remotion skills add
npm run dev
claude --model claude-opus-5-5
```

Node.js, Claude Code (2.1.280+), FFmpeg and FFprobe, Chromium. The default harness asks for a
20-second 1920x1080 60fps film; the paper's worked example is 1080x1080 30fps 600 frames.

---

## 1. Harness Engineering — 600 building blocks

35 pages, 600 numbered entries in 30 categories.
Each entry is a compact label plus one plain-language sentence. The reference links cover the
underlying concepts; the labels and summaries are authored for the collection.

### Categories

| # | Category | Entries |
|---|---|---|
| 01 | Task contracts | 001–020 |
| 02 | Project instructions | 021–040 |
| 03 | Context retrieval | 041–060 |
| 04 | Memory | 061–080 |
| 05 | Reusable skills | 081–100 |
| 06 | Planning | 101–120 |
| 07 | Workflow orchestration | 121–140 |
| 08 | The agent loop | 141–160 |
| 09 | Tool schemas | 161–180 |
| 10 | Tool execution | 181–200 |
| 11 | MCP & connectors | 201–220 |
| 12 | Permissions | 221–240 |
| 13 | Lifecycle hooks | 241–260 |
| 14 | Isolation & sandboxes | 261–280 |
| 15 | Specialized roles | 281–300 |
| 16 | Delegation | 301–320 |
| 17 | Models & reasoning effort | 321–340 |
| 18 | Goals & completion | 341–360 |
| 19 | Runtime state & checkpoints | 361–380 |
| 20 | Recovery & resuming | 381–400 |
| 21 | Validation & tests | 401–420 |
| 22 | Evidence & citations | 421–440 |
| 23 | Evaluation datasets | 441–460 |
| 24 | Graders & scoring | 461–480 |
| 25 | Observability | 481–500 |
| 26 | Resources & budgets | 501–520 |
| 27 | Environment & bootstrap | 521–540 |
| 28 | Deliverables & artifacts | 541–560 |
| 29 | Security & provenance | 561–580 |
| 30 | Reuse & packaging | 581–600 |

### The entries closest to our own architecture

Sampled from the categories that describe an agent harness rather than a video technique.
Each is a one-line pattern with a name worth borrowing.

| Entry | Pattern |
|---|---|
| `action-preview` | Show the proposed mutation before authorization. |
| `approval-content-binding` | Bind approval to the exact content being changed. |
| `approval-staleness-check` | Recheck authorization when the action changes. |
| `external-write-approval` | Review the destination and content of a remote write. |
| `tool-description-contract` | Explain the operation and when to use it. |
| `structured-tool-result` | Return fields the next step can inspect. |
| `tool-capability-boundary` | State the operation's limitations. |
| `argument-validation` | Check a proposed call before executing it. |
| `tool-call-deadline` | Bound how long an operation may run. |
| `read-write-tool-split` | Distinguish retrieval from state-changing connector actions. |
| `instruction-size-budget` | Limit always-loaded guidance to useful content. |
| `skill-review-step` | Include a defined verification stage. |
| `stage-artifact-passing` | Transfer saved outputs between workflow stages. |
| `review-owner` | Assign responsibility for accepting the result. |
| `success-evidence-map` | Connect each success claim to supporting evidence. |
| `memory-provenance` | Record the origin of each durable note. |
| `memory-write-permission` | Control where persistent notes can be written. |
| `mcp-tool-discovery` | Inspect the tools exposed by a server. |

### Provenance

29 distinct URLs: **21 to `code.claude.com`**, 5 to `anthropic.com`, and one each to
`modelcontextprotocol.io`, `opentelemetry.io` and `x.com`. The catalogue is a reading of
Anthropic's own documentation, not original research.

---

## 3. Motion Design Studio — 600 motion briefs

38 pages, 600 numbered entries in 30 categories,
each a motion brief with technique references. *"Original editorial ideas with primary
technique references and inspiration links. Every entry includes an explanation and clickable
source IDs."* Resource links were checked 16 Oct 2026.

### Categories

| # | Category | Entries |
|---|---|---|
| 01 | Text reveals | 001–020 |
| 02 | Kinetic typography | 021–040 |
| 03 | Scroll storytelling | 041–060 |
| 04 | Cursor & hover | 061–080 |
| 05 | Buttons & calls to action | 081–100 |
| 06 | Navigation & tabs | 101–120 |
| 07 | Bento & cards | 121–140 |
| 08 | Lists & grids | 141–160 |
| 09 | Page transitions | 161–180 |
| 10 | Onboarding | 181–200 |
| 11 | SaaS product demos | 201–220 |
| 12 | AI interfaces | 221–240 |
| 13 | Dashboards & data | 241–260 |
| 14 | Loading & progress | 261–280 |
| 15 | Notifications & feedback | 281–300 |
| 16 | Forms & inputs | 301–320 |
| 17 | Commerce & checkout | 321–340 |
| 18 | Mobile gestures | 341–360 |
| 19 | Animated icons | 361–380 |
| 20 | Logo & brand systems | 381–400 |
| 22 | Material transformations | 421–440 |
| 23 | Particle systems | 441–460 |
| 24 | Shader & distortion | 461–480 |
| 25 | Gradients & light | 481–500 |
| 26 | Editorial collage | 501–520 |
| 27 | Social & launch loops | 521–540 |
| 28 | Transitions & cuts | 541–560 |
| 29 | Audio-reactive motion | 561–580 |
| 30 | Spatial camera moves | 581–600 |

### Entry anatomy

One sentence of motion, with source IDs in brackets. Four sampled entries:

- **001 baseline-lift** — [S01] [S03] Lift each word through a tight baseline mask, then let punctuation arrive one beat late.
- **041 pinned-chapter-stack** — [S04] [S05] Pin a chapter title while successive story cards climb underneath and replace its subtitle.
- **221 prompt-token-ribbon** — [S18] [S09] [S21] Break a submitted prompt into a ribbon of word tiles that feeds a compact response panel.
- **541 hinged-frame-cut** — [S35] [S36] [S38] Rotate an image plane on its central hinge and reveal the next scene on its reverse side.

### Provenance

46 distinct URLs: **16 to `motion.dev`**, 6 `tympanus.net`, 4 `gsap.com`, 3 each
`docs.derivative.ca`, `helpx.adobe.com`, `rive.app`, `threejs.org`, 2 each `docs.spline.design`
and `studiodumbar.com`, and one each `buck.co`, `docs.blender.org`, `awwwards.com`,
`ordinaryfolk.co`. These are inspiration and technique references, not cited research.

---

## How this maps onto what we already know

The protocol is the **codified form of the same pipeline** the twelve creator videos showed.
Stage for stage:

| Creator-video stage | Protocol | Harness file |
|---|---|---|
| 0 · the method lives in a file | the brief owns the outcome, each file owns one value | `CLAUDE.md` + 7 files |
| 1 · reference + brief | reference analysis, then a five-part brief | `REFERENCE.md`, `BRIEF.md`, `STYLE.md`, `ASSETS.md` |
| 2 · transcribe for timing | *absent* — this protocol builds from scratch, it does not cut footage | — |
| 3 · cut | *absent* | — |
| 4 · plan before rendering | global storyboard, event schedule | `STORYBOARD.md` |
| 5 · generate | frame-driven Remotion components | `MOTION.md` |
| 6 · render deterministically | `npx remotion render`, fixed seeds | — |
| 7 · sound | cue sheet, waveform-aligned | `SOUND.md` |
| 8 · assemble | one composition, scene sequence | — |
| 9 · review and iterate | staged gate, four-field correction, replay | `DELIVERY.md` |

### Where it sharpens our notes

- **The four-field correction** is the cleanest written form of *"feedback is a spec, not an
  adjective"* — the finding we drew from video 04 (*"the text is boring"* is not feedback),
  video 01 (a comment on the clip) and video 09 (bullet rules for an editor).
- **The reading-hold rule** answers the question our [decisions](../../decisions.md) row on
  borrowed style bans left open: it is not "no holds over 1 s" but **count from the first
  legible frame**, and extend the hold if the line is not finished.
- **2 fps reference sampling** converges with the [awesome-claude-video-skills](claude-video-skills-2026-10-27.md)
  harness (frames sampled at 2 per second) and with Deganutti et al. (arXiv 2605.16223,
  OCR on frames at 2 fps). Three independent sources, same sampling rate.
- **"Measure the file, not the impression"** matches that harness's file-based scoring, and
  its own rule — *"A successful render command alone does not establish that the message is
  readable"* — is the same instinct as our `check`.

### Where it does not transfer cleanly

- **It builds from scratch.** No source footage, no transcript, no cut, no B-roll. Our product
  edits existing takes; three of the twelve video stages have no counterpart here.
- **It is Remotion-only.** We decided *"Remotion (or any second renderer): No"* on 21 Oct —
  two style systems and two failure modes — and pinned HyperFrames 0.8.162. The transferable
  parts are frame semantics, the correction format, the reading-hold rule and the staged gate,
  none of which need Remotion.
- **Its approval gate is a chat checkpoint**, not a product surface. We have approve-before-apply
  and review boards ([COST-1](../../done.md#cost-1)); this adds no telemetry and no scrubbable
  timeline.
- **Its corrections are frame-anchored**, which is the shape we declined on 24 Oct (*anchored
  notes: No*). See the correction below — this is evidence, not a reversal.

## Corrections that change conclusions

- **"Anchored notes: No" (24 Oct) is not what this protocol does.** Our decision was about a
  *product surface* — a notes lane, a selection reference, *"Ask AI about this"* — for one user
  working live in chat. This protocol anchors corrections **in the chat message itself**, as four
  text fields naming a global frame number. It needs no notes lane, and it costs the agent
  almost nothing because the agent never receives a video: it renders named stills on demand.
  The decision stands; the *format* is a sharper way to say the same thing.
- **"A motion preview with sound for the agent: No" (24 Oct)** was decided on cost — 5 s at
  30 fps as frames is ~45k tokens. This protocol sidesteps it by never sending the agent a
  frame *sequence*: the agent gets single stills at named frames, and the moving preview with
  sound is for the **human**. Same saving, different mechanism.
- **"Borrowed style bans as defaults: No" (21 Oct)** — the protocol argues the same way but
  gives a measurement: count the hold from the first legible frame, and extend it if the line
  is not finished. That is a check, not a ban.

## Caveats

- **No measurements.** The paper reports no benchmark, no sale, no conversion. The costs and
  prices are labelled arithmetic assumptions to replace with real inputs.
- **One author's practice, not a survey.** Four artefacts, one person, two weeks of commits.
- **The two 600 catalogues are labels and one-liners**, not evidence. The harness catalogue is a
  reading of Anthropic's documentation; the motion catalogue is original editorial ideas with
  inspiration links. Neither is a citable result.
- **The protocol is unexecuted.** We did not run the harness; the renders and the $150 sale
  are the author's account.

## Re-fetching

```bash
git clone --depth 1 https://github.com/beamnxw/my_works.git
# PDFs: text extracted with pypdf; the harness: unzip opus55-motion-studio-harness.zip
```
