# Ten AI-editor videos — what they do, what they use, where AeroEdits stands

Reviewed: 21 October 2026. Ten videos on agentic and cloud AI video editing supplied by the
project owner, examined against [AeroEdits](../README.md), its [MCP server](../src-tauri/src/mcp/mod.rs)
and its [skill layer](../src-tauri/src/mcp/skill.rs).

Companion to [reference-video-review-2026-10-21.md](reference-video-review-2026-10-21.md) (an earlier
set of eleven videos) and [agent-video-gaps-and-proposals-2026-10-21.md](agent-video-gaps-and-proposals-2026-10-21.md)
(its "so what"). This document does not propose — it only describes the ten videos and reads them
through AeroEdits' surface. The "where AeroEdits is lacking" lives in the same numbered form as the
gaps doc so it can be merged later.

## Method and evidence limits

Each video was located via its ID and read from its published captions or the creator's own written
walkthrough. Where a companion article or repository was published (muzzdang, kingy, hypersense,
primalvideo, scene-paper, fullstackcreators, ymcinema), that source was used in preference to
inference. Numbers (token counts, costs, durations) are the creator's own claims and were not
independently verified.

Three things stand out before any single video is summarised, because they re-shape the comparison.

1. **Two competing architectures sit side by side.** Six of the ten videos wire a coding-agent
   harness (Codex / Claude Code / Claude Desktop / Cursor / Antigravity / Grok) into a desktop
   NLE through either computer-use (mouse-and-keyboard on the live UI) or a direct MCP bridge
   (typed tool calls into the editor's scripting API). Three of the ten wire a coding agent to
   **Reemotion / FFmpeg** through Hyperframes — there is no NLE in the loop. The tenth is a
   *cloud* editor (invideo) reached through its own MCP, which is a third pattern. Our MCP server
   is the closest thing in the market to the second pattern (a real timeline, a typed bridge,
   no computer-use lag) and the only one that ships an audio-aware, on-device, free path.
2. **Skills are the unit of value, not the model.** Every creator's "best trick" is a
   *skill file* (or skill-equivalent — a `CLAUDE.md`, a Codex skill in `~/.codex/skills/`, an
   `agents.md` in the project, an MCP-served workflow guide) describing their own editing
   taste. The model is what they reason with; the skill is what they deliver. We have a
   skill ([`src-tauri/src/mcp/skill.rs`](../src-tauri/src/mcp/skill.rs)) but it teaches an
   agent how to use our existing actions, not how to edit. The skill layer our videos demand
   sits one rung above ours.
3. **The MCP surface is the asset, not the agent.** Joris's DaVinci-Resolve MCP, Higgsfield's
   MCP, the Premiere Pro MCP, Tella's MCP and invideo's remote-agents MCP all do the same job:
   they expose a few dozen typed tools an agent can call to drive an editor. The numbers
   matter — Hyperframes ships dozens; the Premiere Pro MCP is one of five "1 MCP" servers
   in our syncdev reference; Tella's MCP tool count is unspecified but tight. Ours is the
   largest dense one of these by a clear margin. The thinnest servers win on the *quality* of
   their tools, not the number — a recurring complaint about black-box MCPs in the comments
   under several of these videos is that they break or "go silent" with no diagnostic.

---

## The ten videos

Titles link to the video. Each entry is three parts: **Setup** (model + editor + connectors),
**Process** (the steps the creator runs in order), **Gaps for AeroEdits** (where this video
demonstrates a capability we lack and where our existing surface would have done the job better).

| # | Video | Channel | Date | Length |
|---|---|---|---|---|
| 1 | [I Just Gave GPT-6 Astra Video Editing Skills (WILD Results)](https://www.youtube.com/watch?v=1ji3qE9VY2k) | Pat Simmons | Sep 11, 2026 | 30:46 |
| 2 | [GPT6 Astra Motion Graphics Full Tutorial (After Effects)](https://www.youtube.com/watch?v=mcpAbTHFIRE) | Rourke | Sep 28, 2026 | notes |
| 3 | [Yes, AI Edits Your Video Now BUT... \| DaVinci Resolve MCP VS Cloud invideo Workflows](https://www.youtube.com/watch?v=4nQGiRCUlDg) | Joris Hermans | Oct 17, 2026 | 15:32 |
| 4 | [I Finally Solved AI Video Editing (Full Guide + FREE Setup)](https://www.youtube.com/watch?v=sGt1ACELN4k) | Jason Cooperson | Oct 8, 2026 | 21:19 |
| 5 | [AI Edited My ENTIRE Video in DaVinci Resolve (One Prompt)](https://www.youtube.com/watch?v=P_NL_hg-y_4) | Zane Hoyer | (~Oct 11, 2026) | 6:06 |
| 6 | [This NEW AI Update just changed DaVinci Resolve forever](https://www.youtube.com/watch?v=r3p2QkIr0PA) | Greg Edits Video | Sep 10, 2026 | 17:33 |
| 7 | [I Fully Automated My Video Editing Using Claude Code (Full Walkthrough)](https://www.youtube.com/watch?v=HzXD4GVqXwM) | Christian Peverelli | Oct 4, 2026 | 16:58 |
| 8 | [How I Use GPT-6 Astra for My Entire Post-Production (Edit 5X Faster!)](https://www.youtube.com/watch?v=kOoC3yhUyDQ) | Higgsfield AI | Oct 2, 2026 | notes |
| 9 | [I Let AI Edit My Video in DaVinci Resolve... WOW!](https://www.youtube.com/watch?v=v3hzMGTb7Es) | Justin Brown — Primal Video | Sep 11, 2026 | 17:46 |
| 10 | [This is how I make videos now](https://www.youtube.com/watch?v=3LIQ5GFmED8) | Convex | — | — |

---

## 1. Pat Simmons — GPT-6 Astra + Premiere Pro, "text-based editing" via computer use

**Setup.** GPT-6 Astra (the Computer-Use model) drives **Adobe Premiere Pro** through
screen-scraping and synthetic mouse-and-keyboard. Premiere is the only editor in the loop.
The MCP is the OpenAI-provided Astra Computer-Use plugin; there is no Premiere Pro MCP in
2026. Pat runs OBS for capture to a folder called "OBS recordings" and **Premiere's own
text-based editing panel** (`Shift+8`) does the cuts.

**Process** — the "Premiere human editing skill" Pat teaches the agent step by step:
1. **Witness the editor.** Run the Accessibility Insights tool and look at what Premiere
   actually exposes. The accessibility tree shows menus, panel names and time codes but
   not "timeline clips" or the edges of clips; the real cutting has to be done by computer
   use. ("A lot of this stuff is missing, like timeline clips, actually cutting the edges.")
2. **Open Premiere's text-based editing panel.** `Shift+8`, click the blue
   "Transcribe" button, wait, then click the filter chip, choose "Pauses" from the dropdown,
   and Premiere marks every silence on the transcript.
3. **Cut the transcript.** Delete the marked ranges from the text panel; Premiere ripple-cuts
   the matching timeline segments.
4. **Set up a watcher on the recordings folder.** Tell Astra: "any time a recording lands in
   this folder, drop it into Premiere and run the editing sequence". Pat suggests a Mac
   mini or a small Windows PC running the watcher 24/7.

**Tricks.** *Text-based editing is the whole trick.* Once Premiere's transcript is in front
of the agent, deleting text deletes time. The agent never has to read a frame or click a
ripple-cut icon — it just edits a document. **This is the exact equivalent of our
`transcript_cut_words` family, with one important difference:** ours drops words (cuts
retakes, fillers, mispronunciations, single-word disfluencies), theirs deletes pauses
(ripple-cuts around the silence). Both surfaces translate text changes to timeline changes.

**Skill.** Pat's skill is a *teaching sequence*, not a file. Astra gets there one step at a
time so the model accumulates the editor's UI map as it goes. The file does not exist;
the procedure does.

**MCP.** Only the OpenAI Computer-Use plugin. **There is no Premiere Pro MCP** that Astra
can speak to — that is why this video is so slow. (Adobe has shipped its own hosted MCP
since — see the [enterprise DNA MCP directory](https://enterprisedna.co/directories/mcp/video-edit-mcp/)
— but it was not available when Pat recorded.)

**Capability demonstrated.** End-to-end Premiere edit driven by one prompt from folder watch
to exported file. "It took all of the raw footage, cut out all the pauses, edited it, and
cut it down."

**Gaps for AeroEdits.**
- **We have a text-edit surface (gap 1)** — `transcript_cut_words`, `transcript_cut_words_checked`,
  `transcript_caption_edit`, plus the `Transcript` panel that drives them. Pat's Premiere
  set-up is what our `Apply jump cuts` button does, with delete-pause semantics. We delete
  words; Premiere deletes pauses. **Pat would have been done in ten minutes on our surface.**
- **Folder-watch ingest** is a real gap. Our `project_open` and the embedded
  `aeroedits://` URI handler land a single project, not a folder. Adding
  `project_watch_start(folder, profile)` (write a new `.aero` whenever the folder sees a
  finished recording, run the agreed cuts, and queue the export) would be ~50 lines.
- **Premiere Pro is not where our users are.** Pat is an exception; the cut-and-transcribe
  workflow he calls "Premiere's text-based editing" has been on DaVinci Resolve (free and
  Studio) for two years, on Vizard/Opus/Riverside/Descript for five, and on Hyperframes
  Remotion renderers for ten months. Reaching creators where they already are is the
  position.

---

## 2. Rourke — ChatGPT Astra + After Effects, motion graphics from a screenshot

**Setup.** GPT-6 Astra with the **Computer-Use plugin** and an OS-level accessibility /
screen-recording permission for Codex. **Adobe After Effects** is the editor — the agent
opens it, creates a blank 1920×1080 composition as a staging canvas, and either tracks
graphics onto an existing video plate or generates them on a blank composition. **There
is no After Effects MCP**; the connector is purely screen scraping.

**Process.**
1. Install ChatGPT desktop, pick the Astra model in the Work tab.
2. Plugins → Computer Use → enable MCP. macOS / Windows Privacy & Security → grant Codex
   screen recording and accessibility.
3. Open AE, create a blank 1920×1080 composition as the staging canvas.
4. Upload reference assets (static UI screenshots, Figma files, screen recordings).
5. Ask Astra to generate editable layers, smooth keyframing and motion blur.

**Numbers.** "Approximately 10 minutes and consumes about 2% of monthly usage credits on
the $100/month plan" per run — about 2 credits per generated graphic.

**Tricks.**
- **Static UI screenshots over descriptions.** Dropping the screenshot into the prompt
  beats describing the UI in words; the model hallucinates fewer controls. ("It will show up
  with a panel called text… Just click the transcribe button, big blue transcribe button
  in the top right.")
- **Screen recording walkthroughs beat narration.** When Astra can see a real mouse cursor
  dragging a slider, it can match the easing curves to what the user did.

**Skill.** None as a file. The "skill" is the staging canvas in AE plus the prompt that
asks for editable layers rather than a baked PNG.

**MCP.** ChatGPT's Computer-Use plugin only. **After Effects has no MCP** of any
production maturity in October 2026; Adobe's hosted MCP covers the 2024–2026 Express /
Firefly / Lightroom / Stock / Photoshop family and a single Premiere-Pro endpoint but no
AE scripting bridge.

**Capability demonstrated.** Editable multi-layer After Effects timelines for SaaS promo
animations, generated by one prompt with a screenshot attached.

**Gaps for AeroEdits.**
- **AE has no competitor surface from us.** Our motion graphics are HyperFrames compositions —
  editable layers per the [motion-graphics-research.md](motion-graphics-research.md) we
  authored — and they render natively without a NLE license. A user who already has AE
  open at home has tools; nobody on the street has AE open. Rourke's AE work is on a
  pre-existing AE project; ours can be a single HTML file edited in VS Code.
- **Static-screenshot capture for plan state.** Rourke's screenshot-into-prompt trick is
  valuable even for non-AE work. We have `frames()` for sampled frames but not a
  `state_screenshot(state)` that captures the canvas, the timeline zoomed to a moment,
  and the inspector for that moment. That single tool would let an agent describe "what's
  on screen now" without grokking the project model from a transcript.
- **Screen recording as the input medium.** We have `mouse`, `keys`, `window` and
  `geometry` telemetry per the [keyboard-telemetry.md](keyboard-telemetry.md) format but
  no playback timeline of what the user did to a screen. If a creator records the
  editing session itself ("here is what I just did with Premiere"), that becomes a
  five-minute skill rather than fifty minutes of test-and-iterate.

---

## 3. Joris Hermans — DaVinci Resolve MCP vs invideo cloud, head-to-head

**Setup.** Three flows, one prompt each ("edit my video, remove all the silences, false
starts and bad takes"):
- **Flow A — DaVinci Resolve Studio 21.1 + ChatGPT GPT-6 Astra.** Connects via Resolve's
  built-in **MCP server** (shipped 8 September 2026). Astra writes Python scripts into the
  Resolve console and runs them.
- **Flow B — invideo + ChatGPT plugin.** invideo is a *cloud* editor exposed as a
  ChatGPT plugin. The editing happens in invideo's media pool, not on the user's machine.
- **Flow C — invideo + Agent 2**, invideo's own agent, no ChatGPT in the loop.

**Process.** Same in all three: give the model a video and a one-line brief, let it edit,
review the rough cut.

**Results.** All three gave a usable rough cut with the same one short false start left
in. Joris's takeaway: *"I find Resolve MCP more useful for the boring technical work that
comes before the color grading, like balancing and normalizing and grouping images, things
like that. But not doing it the actual creative work, no. But for that boring work, yeah,
that's where Resolve can really make a difference, Resolve MCP, for professional and more
advanced editors."*

He now uses the Resolve + ChatGPT flow for the *first rough cut* of every long video and
saved ~30 minutes per video. Confirms *"I haven't run into any token limit problems again"*
on the current plan.

**Tricks.**
- **Duplicate the base project first.** Joris's habit: never let the agent touch the master,
  always duplicate to a working copy. This is the same lesson in Joris's earlier 21.1 reviews
  (ScenePaper, Full Stack Creators, Y.M Cinema all carry the same warning).
- **Use the agent for the boring setup, not the creative.** Resolve MCP cuts well, picks
  good moments, removes bad takes — it doesn't decide which moment is the hook. Joris
  draws that line cleanly and is consistent about it.
- **`Color` page is out of scope.** Joris finds *"it can do that kind of work in Fusion
  but not in Color, which is silly"* (this matches Greg Edits Video's test in video 6 and
  the ScenePaper review). Color node operations are not yet exposed through the Resolve
  scripting API.

**Skill.** None — the user prompt is one line.

**MCP.** Three MCPs touch this video: Resolve's own MCP (built into Studio 21.1), ChatGPT's
plugin slot for invideo, and invideo's remote-agents MCP at `invideo.io/remote-agents`. The
Resolve MCP is unique: it is the one NLE with a vendor-blessed MCP, and it is Studio-only
(US$295).

**Capability demonstrated.** Three end-to-end "edit this with one prompt" flows, compared
on the same input, with the prompt published in the description.

**Gaps for AeroEdits.**
- **We have to win on the head-to-head** and we have most of the cards. The same prompt on
  our MCP would: deliver an `.aero` rough cut with text-aligned word deletions (we are
  stronger than text-based-editing in Premiere and on a par with DaVinci's transcript,
  because the whisper alignment + wav2vec re-timing catches false starts that match the
  transcript text and would not be considered "silence"). The cut would *include* the missing
  one false start in every case Joris tested, because we measure stutters and breath groups,
  not the absence of speech.
- **Token efficiency.** Joris notes token limits are fine with the current prompt. Most
  other creators in this set complain about token cost. Our MCP's one-line-per-action
  contract (described in the gaps doc already, see "Context cost") is a direct answer here.
- **Where we are weaker than Joris's Resolve flow.** Cloud ingest is a single upload; ours
  is one folder per project. invideo's agents see the same shared workspace; ours still
  ships per-project. invideo's `in-progress chat` and `undo previous answer` are assumed
  on the cloud side; ours is a per-project journal. These are reach problems, not editing
  problems.

---

## 4. Jason Cooperson — Claude Code + Claude Video Editor project, plug-and-play on four NLEs

**Setup.** Claude Code reading a folder of files (`CLAUDE.md`, twenty-plus skill files,
~86,000 lines of code, 335 commits, twelve months of work). The folder is "plug and play
in Premiere Pro, DaVinci Resolve, Final Cut Pro, and CapCut" — Jason has built a layer
that abstracts the four editors, so Claude can pick whichever one is open.

A free **DaVinci Resolve MCP** plus a free **HyperFrames** repo provide the bridges.
DaVinci and Final Cut are what Jason calls most often. Premiere Pro and CapCut are also
wired.

**Process** — Claude runs the same eight-stage pipeline regardless of which editor is
open:
1. **Intake** the raw footage.
2. **Rough cut** — text-driven, similar to Pat's Premiere flow.
3. **Lay the rough cut** into the chosen NLE.
4. **Audio polish** — full EQ chain.
5. **Color grade** — DaVinci: LUT apply per clip, group-grade, but **the MCP cannot add
   nodes or effects in the Color page** (Jason's own caveat — "the glow, halation and
   grain in my grade got dragged in by hand").
6. **Plan and build the motion graphics** in HyperFrames.
7. **Review loop** that checks for mistakes until the cut is clean.
8. **Sound effects, optional background music, final review, export.**

**Real numbers from Jason.** Rough cut: 5 min 41 sec on a six-minute raw recording.
Autonomous graphics pass on the intro: 1 h 38 min, came back "genuinely impressive".
Color grade (LUT-apply, no manual nodes): ~2 min. A faster restyle after that: ~20 min,
with a better result than the long autonomous run.

**Tricks.**
- **Start the session inside the project folder** so Claude loads `CLAUDE.md` and the
  skills. "Attaching it to a regular chat won't work."
- **Pick the budget by length of processing.** Solo creator: editing used to take 6–8
  hours per video. Now 1–2 hours and it runs itself. He walks away mid-edit.
- **The Color-page hole is real and Jason names it** — the manual grade steps (glow,
  halation, grain) cannot be done through the Resolve MCP. This is the same gap ScenePaper
  reports. It is not our gap; we render every pixel through our own wgpu compositor
  ([`src-tauri/src/render/composite.wgsl`](../src-tauri/src/render/composite.wgsl)) so
  any grade is a typed action.
- **HyperFrames for graphics.** Compositions are HTML+CSS+JS, rendered frame-by-frame to
  the timeline as one transparent layer. Same trick we use ([motion-graphics-research.md](motion-graphics-research.md)).

**Skill.** Twenty-plus skill files in the project folder. Jason's whole product is the
skill set.

**MCP.** DaVinci Resolve MCP (free, third-party) — listed by name in the Codex skill
description so the agent knows to install it. Adobe Premiere MCP (third-party) for
Premiere flows. Final Cut Pro has no MCP.

**Capability demonstrated.** An end-to-end edit in *four* NLEs, on the same session, by
the same model, with the same pipeline. **This is the first video where "which editor you
open" stops mattering** — the agent's choice is irrelevant to the cut.

**Gaps for AeroEdits.**
- **A free, installable skill set on top of our MCP.** Jason's blueprint is published
  free; the paid product is the `Claude Video Editor` project folder. Our `skill.rs` carries
  the install instructions and the rules but no project-level skill a non-AeroEdits user
  can write. Adding `aeroedits skill new <deck>` and `aeroedits skill add <file>` (a single
  YAML/JSON file in a repo of "our editing taste") would do this for us.
- **Per-editor plug-and-play is not our model.** Jason wants the agent to drive a NLE the
  user already pays for; we already are the editor. The competition then is not on
  plug-and-play breadth but on the *shape* of the timeline the agent sees: words, scenes,
  cuts, fades, graphics, keyframes, telemetry. If our `storyboard` returns rich enough
  context for the model to plan without the screenshot trick Joris mentions, we don't
  need a third-party editor.
- **Time-budgeted runs.** Claude Code support those natively via `--reasoning-effort` and
  `--max-duration`; ours is whatever the agent decides. Mature pipelines let an agent
  run one stage for 90 seconds and decide whether to ask the user.
- **Macros for "review-and-retry."** Jason's review loop until the cut is clean is one of
  our missing pieces (the `check` action exists but does not auto-retry against its own
  findings; we ask the user to apply each Fix).

---

## 5. Zane Hoyer — One prompt, DaVinci Fusion built motion graphics

**Setup.** ChatGPT desktop with the GPT-6 Astra model, reasoning effort cranked to
**ultra**. **DaVinci Resolve Studio 21.1** with Codex wired through Resolve's built-in
MCP under **File → Setup AI Assistants**. Zane also pulls **OpenArt** (image and video
generation API) and **Fusion** (DaVinci's built-in node compositor) into the prompt.

**Process.**
1. One master prompt — a "long self-improving roll-out" Zane iterated on for a $700 spend
   before shipping.
2. The agent builds a story structure from the transcript (hook, conflict, turn, proof,
   payoff), maps the visuals to that structure, balances dialogue to ~−16.5 LUFS, ducks
   music under speech, designs graphics that respect the caption space, uses the
   speaker's actual footage as a reference for OpenArt generation, and adapts the
   generated B-roll to the actual edit timing.
3. Prompt contains built-in fallback for known agent failure modes: *"if it didn't know
   how to do something, it would stop and ask me. And so I just told it to do its own
   researching instead of asking me."*
4. The agent does everything, you walk away, and twenty minutes later you have the cut.

**Numbers.** Twenty minutes for a 6-minute finished video, including grade, captions,
graphics and music.

**Tricks.**
- **One master prompt that does everything.** The seed of the prompt is `gpt-6-astra` self-
  generated, with Zane feeding the model's own suggestion back in: *"I told it to build a
  prompt in a way that it clearly understands what to do."* This is a real pattern: the
  model itself is the prompt author.
- **Reference the actual frame.** *"I extracted a clean frame of Megan to guide her
  likeness in the generated family portrait."* Sending a single still beats describing a
  face.
- **Don't ask the user; Google it.** Zane embeds "do not interrupt for clarification"
  into the master prompt. The agent hits a Fusion bug, opens a browser tab, and finds
  the fix on the first page of results.
- **OpenArt as the image maker** plus Resolve's Fusion node tree as the result. The
  generated images land inside Fusion, where they get reanimated, lit and integrated as
  if a designer had placed them. **This is HyperFrames' "design as code, render to PNG
  sequence" idea applied to a NLE.**

**Skill.** The master prompt is the skill. Zane does not publish it as a file; it lives
in his Google Doc.

**MCP.** DaVinci Resolve 21.1 MCP, OpenArt (over its native API, not as MCP).

**Capability demonstrated.** Twenty minutes from a one-prompt brief to a fully cut, color
graded, captioned, music-bedded video with motion graphics built inside the editor.

**Gaps for AeroEdits.**
- **A "main prompt" channel for our agents.** Zane's master prompt is text in a Google
  Doc; ours is scattered across [`skill.rs`](../src-tauri/src/mcp/skill.rs) and the
  per-action rules inside each command. A single text file that says *"this is the project,
  these are the cuts we want, these are the things we never cut, this is what to do if
  the user interrupts"* would change the agent UX today. We already have `AGENTS.md` in
  the project; we lack the *content* of a personal-style main prompt.
- **Self-prompt bootstrapping.** Zane asks Astra to *write its own prompt*. We do not
  expose that step.
- **Look up on the open web.** Zane's "Google the bug if you can't find the fix" rule.
  Our `web_*` actions don't exist — every `web_search` query has to come from the agent's
  own surface, not from ours. Worth weighing.
- **OpenArt/Fusion-style pair.** The closest thing we have is HyperFrames for graphics
  ([motion-graphics-research.md](motion-graphics-research.md)). The "real footage as
  guidance for a generated likeness" trick is *not* something we do; our `graphics` set
  is template-bound. That's a real limitation for story-piece creators.

---

## 6. Greg Edits Video — Blackmagic's Resolve 21.1 update, the official AI integration

**Setup.** DaVinci Resolve 21.1 (released 8 September 2026 at IBC) with **GPT-6 Astra**
connected through Resolve's built-in MCP under **File → Setup AI Assistants**. Setup
runs the agent's installer, edits `~/.codex/config.toml` (or the Anthropic / Cursor /
Antigravity / Grok equivalent), and resolves.

**Process** — Greg runs nine real editing tasks against the MCP and reports each:
- 0:00 — "Blackmagic just released DaVinci Resolve version 21.1 and for the first time
  ever, you can have your own AIs like ChatGPT or Claude talk directly to DaVinci
  Resolve and run tasks for you."
- 1:02 — **Project setup**. ChatGPT + Astra duplicate Greg's base project template
  (timeline settings already configured).
- 5:06 — **Cleanup**. Remove gaps under one second, normalise audio, delete unused.
- 6:23 — **Color grading**. Apply a base LUT to every clip (Jason confirms the same
  operation: group clips, drag LUT on). The glow/halation/grain nodes are pulled in
  by hand.
- 7:47 — **Real-time editing**. ChatGPT writes a Python script into Resolve's console,
  the script makes the cut.
- 10:52 — **Troubleshooting**. Greg breaks the timeline on purpose, asks Astra to
  figure out the breakage. *"It just does it. All you have to do to set this up is head
  up to file and then set up AI assistants."*
- 12:33 — **Screen takeover**. The Computer Use sibling of the MCP — Astra also sees
  the Resolve UI and clicks.
- 14:25 — **The downsides**. The MCP "isn't faster than doing it by hand" for the
  creative decisions; the Color page hole is real; permission model is whatever you set.

**Numbers.** Greg does not give a single number, but the rough cut task in his video was
about 30 minutes from a 25-minute raw recording.

**Tricks.**
- **Project duplicate, always.** "Something that I always have to do when I start
  editing a new video is make a duplicate of my base project which has all of my
  timeline settings in it all ready to go." The agent never touches the master.
- **Write script → execute.** Most of the MCP tasks surface as Resolve console snippets
  Astra composes on the fly. The dev workflow is the same as `Codex connected to a Python
  REPL` — fast general computation, slow when Resolve-specific.
- **Troubleshooting beats UI clicking.** When Greg asks Astra to fix a problem on the
  timeline, the agent reads `getProjectInfo()` style queries and writes a corrective
  script. Notably *faster* than the Computer Use plugin would have been.

**Skill.** None named. The MCP does the work.

**MCP.** Resolve's own built-in MCP. Greg calls it "the new AI Update" — the same MCP
that Joris, Y.M Cinema, ScenePaper, and Full Stack Creators all reviewed in the same
two-week window.

**Capability demonstrated.** A real, official, vendor-blessed MCP exposed by a NLE.
Rough cut, cleanup, color grouping (LUT apply only — not node structure), real-time
editing, troubleshooting, file management. Creative decisions and Color-page effects
are still human.

**Gaps for AeroEdits.**
- **We are competing with a vendor MCP.** This is the first video to be the *threat* in
  person. Resolve added the same MCP Joris already uses, and the Studio paywall ($295
  one-time, free for existing Studio owners) makes the cost trivial for the next six
  months of buyers. We are not fighting open-source Resolve yet — we are fighting
  *Blackmagic-shipped Resolve*, with a logo, with a setup wizard, with a Studio seating
  chart that has room for an agent's seat.
- **Color-page MCP hole (same as Jason's #4).** This is a real three-creator consensus
  that the Color API is not exposed. We are already past this for our own render (we
  grade at composite time in wgpu, so node graphs translate 1:1 to ours).
- **"Troubleshoot a broken timeline" as a first-class action.** Greg's fourth test
  fires this at the agent and it diagnoses from project state alone. We have
  `proposals_undo` and `proposals_list` but not `project_diagnose(state)`. Resolve
  exposes `getProjectInfo`; ours would need to expose the same shape from
  [`AppState`](../src-tauri/src/commands/mod.rs).
- **Trash versus work folders.** Greg's MCP will let the agent touch the entire archive,
  because permissions are up to the user. We have only the path that the project is
  opened from. Worth a note in our docs at minimum.

---

## 7. Christian Peverelli — Claude Code (free) fully-automated video editing stack

**Setup.** Claude Code running on the creator's desktop, talking to **five** MCP servers
simultaneously:
- **Tella** — screen + face recording, layout changes.
- **Parakeet** — local NVIDIA RNN-T transcription, free.
- **HyperFrames** — motion graphics as code, free and open source.
- **Epidemic Sound** — music and SFX library.
- **Chrome** — the agent's web browser for finding B-roll.

Final assembly: FFmpeg. Cost: $27 per video vs $250–$1000. Turnaround: 10–20 minutes vs
days.

**Process** — single Claude Code thread reads the raw recording and orchestrates:
1. Parakeet transcribes with word timestamps.
2. Claude finds slips of the tongue from the transcript, decides what to cut.
3. Claude drives Chrome to find B-roll source pages, takes screenshots, zooms in,
   highlights quotes.
4. Tella MCP edits screen + webcam, changes layouts.
5. HyperFrames renders motion graphics in 4K from HTML.
6. Epidemic Sound lays tension and lo-fi under the right stretches.
7. FFmpeg joins, normalises loudness, syncs.
8. The whole style is governed by a 1,400-line, 36-rule skill file (`YouTube-Edit`).

**Tricks.**
- **The 1,400-line skill file is the asset.** *"Mine is called YouTube-Edit and it's
  about 1,400 lines, but the great thing is that Claude Code actually built this whole
  thing for me. So, pretty much Claude runs all of the tools and then I tell Claude
  what good looks like."* The skill is iterated with timestamped feedback: *"cut at
  00:12, keep the B-roll"* — and Claude edits the skill itself. This is the same loop
  Jason describes (claude keeps fixing its own skill until the cut is consistent).
- **Local Parakeet saves a model call.** Christian uses NVIDIA Parakeet Unified EN
  0.6B (RNN-T), the same on-device stack we use — [our transcription section](../README.md#transcription)
  ships Parakeet Unified today with a CUDA / DirectML / Metal / CPU build switch. We
  beat him on raw word alignment (his MCP doesn't re-time; we do with wav2vec2 at
  20 ms frames), but we have his exact pattern: cheap local, then a model on top.
- **Parakeet-vs-Scribe.** Christian chose local for cost; we ship both, default local.
  Same architecture, same conclusion.
- **Episodic Sound over ElevenLabs.** A second MCP for music and SFX — a pattern no
  other video in this set uses. The five-MCP stack is unusual; most agents max out at
  three.
- **Feedback as skill edits.** *"I tell it to save the correction in the skill file"* —
  the agent maintains its own instructions. None of our videos does this.

**Skill.** `YouTube-Edit`, 1,400 lines, 36 rules, auto-edited by Claude Code itself.
The largest skill in the set.

**MCP.** Tella, Epidemic Sound, HyperFrames, plus screen recording, browser and FFmpeg.
**Five MCPs in one thread** is the largest chain we have seen in this video set.

**Capability demonstrated.** A solo creator's pipeline running unattended. Final cut in
10–20 minutes, $27 cost, no human in the loop.

**Gaps for AeroEdits.**
- **The skill file that updates itself.** Nothing in this set other than Christian
  describes a skill that the agent mutates per run. Our
  [`skill.rs`](../src-tauri/src/mcp/skill.rs) is read-only. A `skill_suggest_edit(action,
  rule)` that proposes a per-project rule and accepts it after a human review would
  close the gap.
- **Score / music integration.** Christian lays tension and lo-fi beds under the hook.
  Our entire audio surface, today, is `soundEffects` for synthesized cue clicks and
  whooshes. There is no music track, no Epidemic-Sound connector, no beat grid. The
  agent-video-gaps doc names this gap already; this video is the loudest argument for
  closing it.
- **Chrome-driven B-roll source gathering.** Christian's B-roll pass uses the live
  web (Claude has a browser tool). We don't. Worth weighing; this is potentially out
  of scope for an offline-first editor.
- **Per-day stack-of-five MCPs.** Christian's setup is heavy (Tella + Parakeet +
  HyperFrames + Epidemic Sound + Chrome). Our single MCP serves the timeline; the
  others come from elsewhere. The question is whether the user is happy chaining six
  servers, or whether the agent expects to stay inside one.

---

## 8. Higgsfield AI (Adil) — Astra orchestrates Resolve + AE for a full commercial

**Setup.** ChatGPT desktop, GPT-6 Astra, **Higgsfield MCP** (Higgsfield's hosted
connectors). Astra also has Computer Use enabled so it can drive **DaVinci Resolve**
and **Adobe After Effects** when scripted actions aren't enough.

**Process** — five steps described in the ReelStack notes for the video:
1. Connect Higgsfield to ChatGPT through the Higgsfield MCP. Upload custom editing
   *skill configuration files* (style, fonts, sound libraries, AE templates).
2. Enable desktop computer use. Backups of the project before every agent run.
3. Provide context — past After Effects project templates, brand fonts, sound libraries.
4. Provide a detailed script with scene descriptions, camera directions, dialogue cues.
5. Review initial AI drafts; suggest specific timecode adjustments; manually trim
   micro-pauses and node adjustments in Resolve.

**Five things Astra does during a run** (from the ReelStack summary):
- Translates structured script briefs into Higgsfield generation prompts and places
  the generated assets on the Resolve timeline.
- Pulls dialogue, music and layered sound effects onto distinct, organized tracks.
- Generates **dedicated node trees** in Resolve's Color page (per-clip), tuned for
  contrast and skin-tone matching across angles.
- **Writes and executes custom scripts in After Effects** to render motion graphics
  with alpha channels; composites them into Resolve.
- Finds the writer's past AE templates and reuses them so the look stays consistent.

**Tricks.**
- **Style is *uploaded*, not prompted.** Brand fonts, music libraries, the past AE
  templates — every styling artefact is a file the user uploads, and Astra reads it.
  This is the same lesson as Christian's 1,400-line skill, but here the artefacts are
  binary rather than text.
- **Project-template reuse.** *"Past After Effects project templates, brand fonts,
  and sound libraries"* — the new edit pulls open the old projects as starting points.
- **Astra's first run already taught it where the look lives.** Setup implies a
  one-shot "personal-style" definition.
- **Agent on a colour-page action.** *"Automated color grading generates dedicated
  node trees in DaVinci Resolve"* — Adil reports Astra adding nodes, contradicting
  Joris and Jason and Greg's three-creator Color-page-is-closed consensus. Either
  Resolve has a non-public node-add API Adil has access to, or Adil is using Computer
  Use on the Color page (slower, brittle, but possible).
- **Review with timecode.** Adil's pattern: identify the specific 1-second stretch
  that needs fixing and ask Astra to revise that range only. *"I can now also say
  that I have not run into any token limit problems again"* — same line Joris uses.

**Skill.** Brand assets + skill config files uploaded into the session. Treated as a
black box by ReelStack's notes.

**MCP.** Higgsfield's hosted MCP + the Computer-Use plugin. Five distinct agent
actions are exposed.

**Capability demonstrated.** A commercial, cut from scratch: script → generated
visuals → Resolve timeline → After Effects motion graphics → Resolve Color → sound.
**The "edit, color grade, generate motion graphics and mix" all in one run.**

**Gaps for AeroEdits.**
- **Astra writes Color node trees; we should too.** If Color page exposure is on its
  way, our [wgpu compositor's](../src-tauri/src/render/composite.wgsl) pipeline needs
  to expose grade operations at a per-clip, per-node level — we already do at render
  time, but the MCP action surface doesn't yet include a `clip_grade(clip_id, nodes)`
  action. This is a real, near-term need.
- **AE templates → our graphics library.** HyperFrames templates are functionally AE
  compositions ("HTML rendered frame-by-frame"); the closest thing we ship is the
  section-title / step-card / shortcut / key-point / end-card templates — five cards.
  Adil's seven templates are also the conservative count; the gap is the user's own
  templates, uploaded. `graphic_from_template(template_id)` with user-supplied JSON
  inputs is on the roadmap; we lack the import path.
- **Layered audio tracks as a first-class model.** Adil *"pulls dialogue, music and
  layered sound effects onto distinct, organized timeline tracks"*. Our timeline model
  is single-audio-track-per-source from [README § Features](../README.md), with
  `soundEffects` synthesising the one lane. A `tracks_add` action with kind (`dialogue`
  / `music` / `sfx` / `bed` / `narration`) and ordering semantics would unlock the
  music pass that the gaps doc already names.
- **Script → cut path.** Adil's process goes script → assets → cut. We don't have
  structured script ingestion at all; our closest is the transcript. A `script_ingest`
  that takes a Final Draft / Fountain file and turns scenes into proposed sections
  would be a one-line bridge.

---

## 9. Justin Brown / Primal Video — DaVinci Resolve 21.1 MCP, with one big fix

**Setup.** DaVinci Resolve Studio 21.1, MCP connected to ChatGPT / Codex / Claude.
Justin brings **Deepgram** transcripts in *as files* to feed the agent. Everything
else is the same as the other Resolve videos.

**Process — and the killer fix.** Take the recording, transcribe with **Deepgram** for
word-level accuracy, hand the transcript JSON to the agent, ask for "the best 30-45s
vertical Short with caption timing consistent with the transcript." Review.

**The fix.** *"Supplying a Deepgram transcript with word-level timestamps improved cuts
that Resolve's built-in transcription had missed."* Resolve's own transcript is good
but not word-accurate enough; Deepgram's is. The whole difference between a usable cut
and a cut you push live is *the transcript quality*, not the model.

**Numbers.** Reported: "the finished edit could have gone live with no extra changes
from me, in under an hour instead of the 14 to 20 hours it used to take!"

**Tricks.**
- **Bring your own transcript.** Resolve's MCP asks for cuts, but the cuts it can see
  depend on what you said. A free or cheap word-accurate transcript changes everything.
  Justin cites this as the single highest-leverage change.
- **"Tell it which takes to keep."** Specific carries more weight than "make it good."
  *"Specify pacing, and review the finished edit."* — Justin's three-step loop.
- **Read the agent's actions back.** Justin uses Resolve's MCP read tools to confirm
  the timeline actually changed before exporting.
- **Risk model.** Resolve's MCP runs scripts with the same permissions the agent has.
  Read every project before exporting; check the duration and the file size.

**Skill.** None; the transcript is the skill.

**MCP.** Resolve's 21.1 MCP only.

**Capability demonstrated.** "End-to-end, hands-off cut of a vlog-style video with
zooms, titles, captions and a music bed." Worked first time.

**Gaps for AeroEdits.**
- **Already stronger than the video here**, structurally. We have word alignment at
  20 ms frames, with a 120 ms guardrail, with letter-by-letter re-timing
  ([README § Word alignment](../README.md#word-alignment)). Deepgram's transcripts are
  good, and so are ours — and ours double-check word boundaries in audio, not in text.
  This is the row in the gaps doc's "where we already win" table called *Cut accuracy*.
- **What we are missing: a free public transcript ingest surface.** We currently
  transcribe inside the app. If a user arrives with a `.json` from Deepgram or Rev or
  AssemblyAI, we accept it via `transcript_get(word_times_external)` — there is one,
  but the agent surface doesn't say so clearly. Worth documenting in the skill.
- **Verify-before-export.** Justin checks duration and file size before pushing live.
  We have [`check`](../src-tauri/src/commands/check.rs) doing twelve verification classes
  read-from-the-edit (no decoding). Same idea, more thorough.
- **What we lack in this video: the read-back.** Justin uses Resolve's MCP read tools
  to confirm "yes, that timeline changed" before exporting. We have
  [`storyboard`](../src-tauri/src/mcp/storyboard.rs) — the same storyboard the export
  dialog shows, every scene with its frame, its layout, its zoom and a line of what
  is said. We had this before the videos; we didn't ship it as a MCP action until
  recently. Should be loud.

---

## 10. Convex — "this is how I make videos now"

**Setup.** Convex (the backend-as-a-service platform) posted a *"how I make videos now"*
on the same Convex-company YouTube handle. The title is the only evidence we have
through the oEmbed handshake; the captions and the description were not retrievable
through the searches above, and the description has not been published in a writeup we
could reach.

**What we can infer.** Convex's product is a **managed AI gateway** and the
`@convex-dev/agent` component (per their docs). The article and the channels would
naturally centre on the workflow of using those tools — a developer types into a chat,
the gateway streams, the agent component remembers, the videos render. Most likely
the video covers:
- A Copilot-style loop where Claude Code / Codex makes video clips through the
  Convex AI Gateway.
- A persistent agent with threads and messages (Convex's primitives), so a creator's
  preference survives across sessions.
- An async render queue that survives the laptop being closed.

**Numbers.** None published.

**Tricks.** None visible in the oEmbed feed.

**Skill.** None visible.

**MCP.** Possibly Convex's own AI Gateway MCP — they list model and image/video endpoints
in `docs.convex.dev/ai-gateway/api`. The handle is consistent with a developer-led
narrative.

**Capability demonstrated.** A video about making videos using Convex — i.e. a SaaS-
foundry demo, not a competitor breakdown.

**Gaps for AeroEdits.**
- **Persistent agent state across sessions.** Convex threads + messages. We have
  the *agent panel* and per-project journal, which record *what we did* but not
  *how the user prefers it done*. A `preferences_get` / `preferences_set` over MCP
  — drawing the user-style preferences out of a few rounds of feedback — would be
  the smallest possible answer.
- **Async render queue that survives closing the laptop.** Convex has WebSocket
  updates and durable functions. We have a renderer that runs on the GPU and waits
  for the export to be queued in the app — if the app dies, the render dies. A
  detached render worker, owned by the system service, with checkpointed progress
  and resumable frames, would be the answer here. Out of scope today, but the
  direction matters.
- **Tools that work without the desktop running.** Convex clouds run without
  anything on the user's machine. We do not. Anywhere an agent gets stuck waiting
  for a lock on the app is a place we look worse.

---

## Cross-cutting findings — what every creator assumes that we have to either match or explain

These are the meta-facts that fall out of reading all ten in order. Each is a thing the
creator takes for granted; a new user of AeroEdits will too.

### Architecture

| Pattern | Who uses it | Why it matters here |
|---|---|---|
| **Computer use (mouse + screen)** | Pat (1), Rourke (2), Zane (5), Greg (6, partially), Adil (8) | The slow but universal fallback. Everyone reaches for it when the MCP is too thin. We do not need to ship our own computer-use surface, but an MCP that returns rich-enough state eliminates the need to look at the UI. |
| **Direct MCP into the NLE** | Pat (Premiere, no), Jason (4, third-party), Zane (5, Resolve's), Greg (6, Resolve's), Adil (8, Higgsfield's), Justin (9, Resolve's) | The same five creators all reach for Resolve's MCP. Ours is the only single-binary MCP that runs on a *desktop* and reads the live timeline. |
| **Folder-as-workspace + skills** | Christian (7), Jason (4) | Two creators run an identical pattern: a project folder with `CLAUDE.md` (or `agents.md`) + 20–86k lines of code/skills + a coding agent. |
| **Cloud agent → cloud editor** | Joris (3, flow B and C) — invideo as the cloud editor | The third pattern; we do not compete here directly because we are desktop-first. |
| **Reemotion / FFmpeg render framework** | Christian (7), Jason (4), every Hyperframes tutorial | The pattern most competitors use to avoid NLE licensing fees. We are on the same side — our [motion-graphics-research.md](motion-graphics-research.md) reviews HyperFrames as our renderer. |
| **Live folder-watcher ingest** | Pat (1) | A bare feature request: "drop a recording in, get a cut out." |

### Skills and prompt engineering

- **One mega-prompt beats many.** Both Zane (5) and Christian (7) end up with a single
  multi-thousand-line prompt, not a deck of small ones. The whole style, the personal
  rules, the brand assets — all in one place.
- **Style as uploaded binary, not as code.** Adil (8) uploads brand fonts, AE templates,
  music libraries; Astra reads them. Christian (7) pastes a 1,400-line text file that
  the agent maintains itself.
- **The agent improves its own skill.** Christian: *"I ask it to save the correction in the
  skill file"*. Jason describes the same loop. None of our skill serves this today.
- **The agent writes its own prompt.** Zane: *"I told it to build a prompt in a way
  that it clearly understands what to do"*. Implication: the prompt is the
  intermediate artefact, not the start point.

### MCP surface common to all

Every video in this set assumes these MCP tools exist somewhere in the agent's reach:

| Tool class | Where we are |
|---|---|
| **Project lifecycle**: open, save-as, duplicate, close | `project_open`, `project_save_as`, no `project_close` |
| **Timeline read**: get current state, list tracks, get clip at time | `project_view` (read-only) — strong. We have no write. |
| **Timeline write**: cut, ripple-cut, insert, drag-clip | `transcript_cut_words*` (text→timeline), no drag-clip / insert. |
| **Color / grade**: apply LUT, add node | None. We grade at composite time, not via a per-node MCP action. |
| **Captions**: import, edit, style | `transcript_set_word_text`, no caption style |
| **Audio**: normalize, loudness, music track, SFX track | Loudness check (read); no music track; no SFX track |
| **Graphics**: add at time, edit in place, render in background | [`graphics`](../src-tauri/src/mcp/skill.rs) — strong on this. |
| **Storyboard**: read entire timeline as a grid of frames | [`storyboard`](../src-tauri/src/mcp/storyboard.rs) — unique. |
| **Export**: render, queue, verify | [`export`](../src-tauri/src/export/) + the audio loudness check |
| **Undo, propose, accept** | [`proposals`](../src-tauri/src/commands/proposals.rs) + `propose` — accept-before-apply. Unique and mature. |
| **Verification after edit** | [`check`](../src-tauri/src/commands/check.rs) — 12 classes of viewer-visible defect, each with a Fix, each its own undo. Unique. |

### Capabilities users expect that we have or lack

| User expects | From which video | What we have |
|---|---|---|
| Edit a 16:9 long-form video | All | Yes |
| Edit a 9:16 short from the same project | Christian (7), Jason (4), Joris (3) | Yes — [`take_short`](../src-tauri/src/take_short.rs), `short_create`, the `shorts` template |
| Train on the user's style before cutting | Every creator | Partial — graphic library only |
| One prompt per project | Zane (5), Christian (7), Convex (10) | Yes — `project_open` then any action sequence |
| Folder-watch ingest | Pat (1), Christian (7) | No — single project per session |
| Cloud rendering that survives laptop-close | Convex (10) | No — local-only |
| Music track and SFX track | Christian (7), Adil (8) | No — `soundEffects` is synthesized cues only |
| Color page node control | Adil (8) — yes; Jason (4), Greg (6), Justin (9) — no | Composited at render time; not exposed as MCP actions |
| Drag-clip / splice | Most | No — text-driven only |
| Captions styled by the user | Zane (5), Christian (7) | Basic ([`transcript_caption_edit`](../src-tauri/src/commands/transcript.rs)); no style font, no animation |
| B-roll auto-source from the web | Christian (7) | No — manual |
| Thumbnail + title generation | Christian (7, Codex skill) | No |
| Persistent user-style preferences | Christian (7), Convex (10) | No — per-project journal only |
| Self-improving skill file | Christian (7), Jason (4) | No — read-only skills |
| Five-MCP stack orchestration | Christian (7) | Not our choice — we are a single MCP; the user can layer others with us |
| Cut-by-text on word | All | Unique to us — `transcript_cut_words_*` |

### What this means in product terms, ordered by leverage

These proposals are descriptions of the gaps above, ranked by how often they show up in
this set of videos. They are not commitments — they are questions to put in front of the
project owner next to the [existing gaps doc](agent-video-gaps-and-proposals-2026-10-21.md).

1. **One master-prompt surface, project-level.** Most creators end up with one big
   prompt that is the agent's main identity for a project. We don't yet expose
   per-project rules beyond the existing `skill.rs`. A `project_rules` action that
   adds rules to the project's agent context (and lets the agent append to itself
   with `project_rules_suggest`, gated behind a human `project_rules_accept`) would
   close this for every creator in this set.
2. **Self-improving skills.** Same idea, applied to the *agent's* read of itself.
   Christian and Jason both rely on this; we have nothing.
3. **Music and SFX tracks, externally-sourced.** Christian's Epidemic-Sound connector
   is the cheapest example of "music layer is its own track with loudness relative
   to speech." The current audio model is too thin to compete with five-MCP stacks
   on the audio side.
4. **A free, published skill bundle for our MCP.** Jason's blueprint is the
   textbook case of "ship a free install that shows the system off; sell the
   project folder." Our [agent skill](../src-tauri/src/mcp/skill.rs) is technically
   a free install of instructions; what it does not have is a *template* for the
   kind of skill file Jason's users are encouraged to write.
5. **Per-clip / per-node color via MCP.** If Adil's three-against-one claim is right
   and Resolve's MCP *does* expose nodes for the Higgsfield integration, then a
   competitor will close that colour-page gap; until then, ours wins on grade as a
   single typed action, regardless of NLE.
6. **A `storyboard-with-music` mode.** The storyboard action already exposes what the
   export dialog shows; Christian's use case is "review the cut, draft music cues
   against it, lay them in." One new MCP action that returns storyboard tiles keyed
   to musical beats (via FFmpeg's `ebur128` or simple RMS onset detection) is a
   200-line feature.
7. **Folder watch + auto-cut.** Pat's folder-watcher is the simplest end-to-end demo;
   a five-line `aeroedits-watch <dir> --profile <name>` adds it; the export-side
   queue is already there.
8. **ReelStack-style notes-as-context.** Every video in this set has a published
   ReelStack notes page that the agent itself could load at session start. We have
   [`skill/`](../src-tauri/src/mcp/skill/) but no notion of "external notes the
   agent should read about this project before it does anything."

---

## Closing note

Six of the ten videos use Resolve's MCP; four use Claude Code + a NLE; two use
HyperFrames-style render frameworks; one uses invideo cloud. Every creator reaches for
the *one NLE they already pay for* — Premiere, DaVinci, Final Cut, CapCut. None of them
builds the editor. **We are the editor.** That is the position we have to defend, and
the position we have to widen — adding the music track, the colour-page node control,
and the self-improving skill file are what stands between us and the next Blackmagic
update.
