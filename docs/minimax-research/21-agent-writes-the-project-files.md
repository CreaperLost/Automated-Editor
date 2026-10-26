# 21 — The agent writes the host's project files, then writes itself a skill

- **Source:** [youtube.com/watch?v=U-AdnIz1jIk](https://www.youtube.com/watch?v=U-AdnIz1jIk) —
  *Automate Your CapCut Editing With Claude (Step by Step + Free Skill)* · **Matt Loui** · 23 Oct 2026
  · 9:39 · 60,844 views · 1,135 likes
- **Artefacts:** [Higgsfield MCP](https://higgsfield.ai/mcp) (`mcp.higgsfield.ai/mcp`) · a free Claude
  skill (Google Drive) · a full prompts list (Google Doc) · [mattloui.com](https://mattloui.com)
  (*CapCut Lab*)
- **Raw captures:** [the video](evidence/2026-10-25/automate-your-capcut-editing-with-claude-step-by-step-free-s.md)
  · [the Higgsfield MCP page](evidence/2026-10-25/higgsfield-mcp-ai-image-video-generation-for-any-agent.md)
- **Evidence grade:** creator claim. 296 ASR snippets — **"Higgsfield" comes out as "Higsfield"**,
  which the repo's own `evidence/scripts/README.md` warns about and says not to fix in bulk. The
  description is the reliable source.

Matt Loui is already in the repo — trick #3 (*"Ask before the long job"*), trick #5, archetype C,
and a *Prompt kit* artefact. **This is a different video with a different artefact**, and it
contains the clearest external statement of a principle we already decided on.

## 1. The mechanism: no MCP, no computer use, no UI. Just files.

The instruction inside his prompt:

> "**Do not use computer use or open CapCut. Build the timeline by writing the project files
> directly through file access to my Mac**, and finally, ask me any questions to confirm."

**Claude writes CapCut's project files directly.** CapCut is closed while it works; the user
reopens it and the timeline is there — 117 cuts, a screen recording synced to the camera, a colour
adjustment layer, zooms, music. *"I didn't edit a single thing."*

**This corrects a note the repo already carries.** `docs/research/creator-workflows.md` says:
*"**CapCut has no MCP** in Jason's setup ('Claude clicks through the UI'); three MCP hosts, one UI
host."* The first half is right — there is no CapCut MCP here either. **The second half is wrong
as a general statement: Matt clicks through nothing.** CapCut does not need an MCP because its
project *is a file format you can write*.

**That is the generalisable lesson, and it is the strongest external evidence anyone has given
for AeroEdits' shape.** Our `decisions.md` refuses *"Driving Premiere or another NLE by computer
use — No | Fragile UI, high upkeep; our typed API owns edits."* Matt arrives at the same place by
a different road: **an NLE whose project is a writable file is editable by an agent with no API and
no UI.** An NLE with neither is not.

> **What makes a host editable by an agent is not whether it has an MCP. It is whether its project
> is a file.**

## 2. The generation MCP writes code, not pixels

> "Claude, using **Higgsfield Motion Editor MCP**, used **no credits because it wrote all the
> code**."

The repo's competitor notes already knew the shape of this: *"Higgsfield Motion Designer builds
native AE layers, keyframes and expressions from language inside an open project; **the AE plugin
uses no Higgsfield credits; generation does.**"* What Matt adds is the part that matters for us:
**it is reachable over a plain MCP URL** — `mcp.higgsfield.ai/mcp`, *"30+ models for image and
video generation, no API key required"* — so an agent can ask for motion **without After Effects
installed**.

**This is a different kind of generator from OpenArt MCP** ([19](19-the-folder-is-the-prompt.md)),
and the difference is the whole argument:

| | what comes back | editable | credits | gateable |
|---|---|---|---|---|
| OpenArt MCP | image / video files | no | yes | no |
| **Higgsfield Motion Editor MCP** | **code (native AE layers)** | **yes** | **no** | **yes** |

Our `decisions.md` refuses *"Generative video as a core dependency — No | No editability or
determinism. Generated clips are fine as **user-supplied assets** in a slot."* The Higgsfield
motion path is the case where that reasoning does not bite — **because it is not generating pixels
at all, it is writing a project.** That is the shape a generator would have to have to ever be
admissible to us, and it already exists in the field.

**Caveat:** the public Higgsfield MCP page advertises generation ("30+ models for image and video
generation, no API key required") and product areas including Motion Design, *For After Effects*
and *For Blender*. **It does not document a Motion Editor tool or its schema.** The "writes code,
spends no credits" behaviour is supported by the repo's own competitor notes and by Matt; I could
not verify the tool surface from the public page.

## 3. The agent reads your corrections and writes itself a skill

This is the answer to the repo's sharpest structural gap.

The repo: *"The method lives in a file the user owns and the agent improves… Jason and Zane: the
prompt is a versioned, compiled artefact. **Our `SKILL.md` is compiled into the app and reinstalled;
the user can't own it.** This is the sharpest structural difference between our loop and the strong
examples → AGT-3."*

Matt's loop, verbatim:

> "ask it to **reanalyze that project now that you've made those changes**. What Claude will do
> incredibly is the before and after and **make note of any changes that you made** so that it
> understands the style of editing that you like. And now that Claude understands how you like your
> videos edited and how you like your motion graphics, the music, the sound effects, the pacing,
> ask it to use this chat and the workflow that you established to create a skill that you can use
> for future projects. **Claude will take all the context, the editing, the prompts, the changes
> that you made in CapCut, and write a skill for itself**."

**The mechanism is a diff.** The agent has the project before the user's manual tweaks and the
project after; it reads the difference and writes itself a skill.

**We already hold the raw material.** Our checkpoints and history (SAFE-2) keep the project state
across edits, and the user's manual changes are edits too. **Matt's diff is, for us, the
difference between two checkpoints.** That is a concrete and cheap AGT-3 — and it is data, not free
text, which is what the 17 Oct condition for bringing learning back requires.

## 4. What the human review caught — and it is gate 3, live

> "if we see here, we see a **jump cut and then the motion graphic kicks in, which just isn't
> professional**. So, I'm going to go ahead and move that motion graphic up a little bit so that we
> have a cut that looks like this."
>
> "There may be a **moment that Claude missed removing a pause or a stutter**."

Two defect classes, both caught by a person looking at the result:

1. **A graphic landing badly at a cut** — a *placement* defect.
2. **A missed pause or stutter** — a *cleanup* defect.

**The first is exactly finding [11](11-the-gate-already-reads-pixels.md)'s gate 3** — the gate
that knows about the edit rather than about the graphic alone. The second is the class our
`gaps.md` already names: *"The filler/retake rules cut a deliberate repeat… restarts, single-word
repeats and cut-off words open no candidate window."*

**So the field's top workflows still do gate 3 by hand.** Findings 13, 16 and 17 established that
the field's *tooling* has converged on *"the check must read the output"*; this is the *workflow*
side — the check happens, but it happens in the human's eyes, inside ten minutes of active time.

## 5. The rest of the method, which is mostly things we already knew

- **Approve one, then the rest:** *"Generate the first animation for my approval, asking me
  questions about the visual and creative direction. **Once approved, move on to the rest applying
  the same design and thinking.**"* — PLAN-1's style trial, confirmed a third time.
- **Ask before the long job:** the prompt ends *"ask me any questions to confirm"*. Trick #3's
  source is Matt Loui; here it is on the record.
- **A folder is a parameter:** *"Add music from my creative folder… You can direct Claude to
  whatever folder or creative folder that you have on your computer that you want it to use."* —
  the same move as Zane's folder ([19](19-the-folder-is-the-prompt.md)), and the repo's trick #2's
  warning to *"give the agent concrete asset files, not an inspiration board it can't resolve."*
- **A style reference is an image:** a Pinterest video, downloaded and uploaded. Third instance —
  Zane's style folder, Jake's reference upload, this.
- **A compatibility constraint belongs in the prompt:** *"Any generated assets need to work in
  CapCut correctly for CapCut to be able to access these files."* The agent must know the host's
  file requirements. **That is a `rules` item, not a taste item** — and we have no equivalent.
- **The agent places, the user approves:** the model put the graphic after the jump cut; the user
  moved it three frames earlier.

## 6. Numbers

| | |
|---|---|
| **117** | cuts on a talking head |
| **58 min** | for Claude to edit inside CapCut |
| **51 min → 18 min** | a long recording cut down, *"perfectly synced"*; also tested on a screen recording |
| **~3 h** | for a fresh project end to end — of which **about 10 minutes is his active time** |
| **$0** | generation credits (the motion was code) |
| $17/mo | Claude Pro on the annual plan ($20 month-to-month) |
| $15/mo | Higgsfield Starter, for the MCP |
| "20 to 30 hours per project" | his estimate of the time saved |
| ~600,000 views | for the video whose motion graphics Claude made, in a week and a half |

**The number that matters for us is the ten minutes.** Three hours is Claude's; ten minutes is the
human's. **The product is not the speed of the edit — it is how little of the human's attention the
edit costs.** Finding 19's argument (*a check is what makes a setup amortisable*) is the same
thing seen from the other side.

## 7. What he sells

[mattloui.com](https://mattloui.com) — **CapCut Lab**: *"The CapCut system behind a 200k+ subscriber
channel and 15 years of client work, for creators, client editors and business owners. Lifetime
access."* 800+ students.

**He sells the system, not the software** — the repo's archetype F (*"the vendor ships the whole
loop"*) and Zane's $700 prompt, same shape: the method is the product.

## What we would take

1. **Write our principle down in the form this gives it:** *an NLE is editable by an agent when its
   project is a writable file format* — and, more usefully, the inverse: **it is not editable when
   it is only a UI.** That is the real content behind our "no computer use" decision, and it is
   the first external evidence for it.
2. **The before/after diff as AGT-3's mechanism.** We already keep checkpoints; the diff between
   two of them is the learning signal, and it is data rather than free text.
3. **The generator-that-writes-code test.** If a generation dependency is ever admitted, the bar is
   *"writes a project we can gate"*, not *"writes a file we can import"*.
4. **The compatibility constraint as a rule.** "Generated assets must satisfy the host's file
   requirements" is checkable, so it belongs in the skill the agent reads — not in taste.

## Caveats

- **Captions are ASR.** 296 snippets; "Higgsfield" is transcribed as "Higsfield". The repo's own
  `evidence/scripts/README.md` says not to rewrite mangled brand names in bulk — quoting a
  timestamp and checking it is the workflow. The description is the reliable source.
- **One creator, one setup.** CapCut desktop on a Mac, Claude Pro, a Higgsfield subscription, and
  a paid course behind it. **Nothing was run or measured here.**
- **"Claude writes the project files" is a claim about a closed format.** CapCut's project format is
  undocumented and may change. We would never ship this — it is exactly the fragility our
  `decisions.md` cites against computer use — but the *principle* survives the implementation.
- **The Higgsfield MCP page does not document a Motion Editor tool.** It advertises generation and
  product areas; the "writes code, spends no credits" behaviour rests on the repo's competitor
  notes plus Matt.
- **"Perfectly synced"** is his word for the 51→18 minute cut, not a measurement.
- **The ten minutes is his estimate of his own active time**, not a measurement of anyone's
  attention.
- The free skill and the prompts Doc are his artefacts; **the prompts Doc is a different document
  from the "Prompt kit (Matt Loui)" already in the repo's artefact index.**

## What this implies outside this folder — **not applied**

This folder is the only place anything was changed. For the record, here is what a maintainer would
want to change elsewhere — **none of it has been applied**:

- **`research/creator-workflows.md` → "Corrections that change conclusions":** the note *"CapCut has
  no MCP in Jason's setup ('Claude clicks through the UI')"* is right about the MCP and wrong about
  the UI. Matt Loui edits CapCut by **writing its project files** — no MCP, no computer use, no UI.
  The generalisable correction: **what makes a host editable by an agent is not an MCP server, it is
  that its project is a writable file.**
- **`research/creator-workflows.md` → artefact index:** *Higgsfield MCP*
  (`mcp.higgsfield.ai/mcp`, 30+ models, no API key), *Matt Loui's CapCut Lab* (the system sold, not
  the software), and his free Claude skill + prompts Doc.
- **`research/creator-workflows.md` → "Tricks worth stealing":** two rows — *the agent diffs your
  manual tweaks and writes itself a skill*; *a generated asset must satisfy the host's file
  requirements, so that is a rule, not a taste item*.
- **`research/competitors.md` → Higgsfield section:** the motion path is reachable **over a plain
  MCP URL without After Effects installed**, which is a different claim from the AE-plugin one
  recorded there.
