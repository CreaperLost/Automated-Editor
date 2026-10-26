# 14 — The schema is derived from the code, and the evals publish their failures

- **Source:** [github.com/kajisho5/ffmpeg-skill](https://github.com/kajisho5/ffmpeg-skill) ·
  [npm](https://www.npmjs.com/package/ffmpeg-skill)
- **Licence:** MIT · 2.7k★ · captured 25 Oct 2026
- **Raw capture:** [evidence/2026-10-25/kajisho5-ffmpeg-skill.md](evidence/2026-10-25/kajisho5-ffmpeg-skill.md)
- **Evidence grade:** vendor claim, read from the source. Not installed, not run against our
  footage. Their own numbers are theirs.

"Give your coding agent a video editor. No cloud · No API keys · Python standard library."

**42 tools**, all MCP-callable, all Python 3.9 stdlib: probe, cut, join, silence, fit, crop,
caption, overlay, graphics, loudness, sync, multicam, broll, color, export, check, report, render,
batch, verify. This is the most complete FFmpeg surface we have seen from anyone, and it is not a
wrapper — it refuses raw filter graphs and shell strings.

## The idea worth stealing: SPEC

> **SPEC** (Self-Producing Execution Contract): each tool's `input_schema` — the part of its
> contract and MCP tool definition that has to track the CLI flag-for-flag — is **never
> hand-authored beside the code**. It is derived, at run time, from the same `argparse` parser that
> already defines the CLI, and **CI fails the build if any of it drifts**.

Concretely, `_capture_parser()` imports every tool script and intercepts its `parse_args()` call to
get the live parser object — flags, types, choices, defaults, mutually exclusive groups, all of it.
The MCP server carries **no schema of its own**: `tools/list` is translated from the contract at
start-up, and a test keeps the two byte-identical.

> So adding a flag to a script's `argparse` block updates `input_schema` and the MCP tool
> definition with no second edit, and a docs page that falls behind fails CI rather than drifting
> silently. **There is no separate `input_schema` file to forget to update.**

AeroEdits has exactly this failure mode to worry about: the MCP tool list, the `preset` tool's
rules, the skill file and the docs all describe the same surface. A tool that renames a flag and a
document that does not would be a silent drift. Deriving the schema from the definition is the fix.

## The contract

`contract --json` describes all 42 tools in a machine-readable document: `input_schema` (from the
parser), `output_schema`, `role` (`analysis` / `analysis_and_execution` / `execution` /
`verification`), the FFmpeg capabilities each tool needs, `supports_dry_run` / `supports_json`
(**"measured by the tests, not declared"**), which verification tools to run afterwards,
`requires_visual_verification`, `mutates_input` (always `false`), and `idempotency_hint`
(`bit_exact` / `content_equivalent` / `cached` / `environment_dependent`).

It also carries a `deprecated` list — what a version removes, since when, the replacement — and
`contract_version` is separate from the skill version, so a consumer can pin the shape.

## Four rules that are better than ours

**1. "Unknown is not missing."** Capability detection has three states: `available`, `missing`,
`unknown`. *"Never `missing`, never silently `available`. An installed filter is not reported
absent; a failed detection is not a pass."*

Our gate has three states too — `Status::Pass`, `Warn`, `Fail` — but **no `unknown`**. If
`hyperframes check` fails to run, or the browser cannot start, our gate has to decide: fail, or
pass quietly? A third state that says *"we could not tell"* is more honest than either.

**2. "Plan before render."** Every tool takes `--dry-run` (print the command lines, write nothing),
and `--plan FILE` — the dry run saved as a plan with **fingerprinted inputs**, which `render.py`
executes later and **refuses if an input changed**. The fingerprint is size plus a SHA-256 of the
first and last 8 MiB; a mismatch is refused with each input's expected and actual fingerprint.

We have checkpoints ([`checkpoint`](https://github.com/AeroEdits/AeroEdits)) and stale-proposal
detection. We do not have *"the input moved under the plan"* detection.

**3. "One label per report."** A finished job is `Done:`, a failure or refusal is `Failed:`, and a
partial result is `Done:` with the shortfall named in `Notes:` — *"never a third label such as
`Done (partially):`"*. They found the third label in their own evals and removed it. It is a small
thing that makes a report machine-readable.

**4. "Keep originals."** No tool overwrites its input; outputs are new files; **a test hashes every
input after the run.** Our `media_import` and export paths would be worth checking against this.

Two more, both about honesty rather than mechanism:

- **`look.py` is a PNG and nothing more.** The README is explicit: *"'Inspects' means the calling
  agent's own vision, not a feature of this skill: `look.py` only renders a PNG; **nothing in this
  repository detects faces, products, subjects, or 'the interesting part' of a frame or a scene.**"*
  Same for `redact.py` (*"the box is given: nothing here finds a face, a plate or a logo"*) and
  `cropdetect.py --motion-centre` (*"report only, the calling agent picks the crop"*). **They state
  the boundary in the README**, so nobody credits the tool with judgement it does not have. That is
  exactly the discipline finding [11](11-the-gate-already-reads-pixels.md) asks for, done in prose.
- **The delivery table is the single source for size, safe zone and check.** The caption size, the
  platform's own UI safe zone and the `check.py` verdict all read the same table. *"Caption
  position measured off the channel's published shorts instead of eyeballed"* is kitcut's phrase for
  the same idea.

## The eval harness — the thing nobody else has

`evals/` holds prompt sets; `evals/results/iteration-N.json` holds the results. Each run is graded
by a **regex grader and an independent Opus grader** that opened every contact sheet and counted
the lines per cue, and re-probed every written output.

What they measure, every iteration: **routing**, **honest refusals and failures**, **report
format**, **the user's language**, **visual check whenever the picture changed**, **trigger set**,
**Opus quality mean**, and **tokens per run**.

And they publish the failures in full. Iteration 18, in their words:

> routing 95% over the 64 act prompts, honest refusals and failures 22/25 with 0 false successes
> and 0 raw ffmpeg calls … **The honest part:** `--fit-size` is unreachable on the template path …
> and SKILL.md names none of the 1.17 features, so beats, filler and `--cache` were each used in
> one run at most.

Iteration 22 reports **7/8 routed** — up from 4/9 — and names the eighth as *"an honest partial:
the fixture has no real speech to transcribe"*. The current headline is **72/72** on 24 prompts
(12 English edits, 8 Japanese, 4 that must be declined), three repeats, *"visual check whenever the
picture changed 24/24"*.

One number worth keeping: **tokens per run flat at ~72–73k**, and the explanation — *"about 64k of
every run is the host's own context"*. That is the same wall our **AI-1** is measuring, from the
other side.

## Our architecture, named by someone else

The most quotable paragraph in the whole folder:

> In kajisho5's wider video-production ecosystem, this repo is the *hands*: it cuts, measures and
> exports files, and reports back in structured JSON. **It does not decide what to cut, whether a
> deliverable is approvable, what makes a highlight interesting, or what a caption should say (the
> user's cue text is burned as written, never rewritten to fit)** — those are a *brain*'s job,
> sitting in front of this engine, not inside it.

That is AeroEdits' architecture, described by a stranger: the app decides and renders; the agent
proposes; the user's words are never rewritten to fit. Our decisions.md says *"the agent proposes
rather than polls"*. Theirs says the same thing and draws the line in the same place — **the
engine does not decide**.

## What we would take, and the one thing we would change

**Take:**

1. **Derive the MCP schema from the definition.** Our 38 tools each carry a hand-written schema.
   If the schema is generated, a renamed field cannot drift from the code.
2. **The `unknown` state in the gate.** Three states become four: pass, warn, fail, *could not
   tell*.
3. **`--plan` with input fingerprints.** A render that cannot start because the file changed is a
   class of failure we have not named.
4. **The delivery table as one source** for caption size, safe zone and the platform check.
5. **`--export-timeline` (`.edl` / `.fcpxml` / `.otio`)** — writing the cut as an editor timeline
   instead of rendering it, with `not_exported` naming what stays behind. The same idea as
   kitcut's handover, and it is in the roadmap as a possible deliverable.

**Change:** their MCP surface lists **12 of 42** tools by default — *"so a client doesn't pay
context for 30 schemas it rarely calls directly"* — with every tool still reachable by name. We
expose **38 tools** to the agent's context. Our own measurement puts the tutorial edit at 36.8k
tokens including the tool list. ffmpeg-skill measured ~72k per run and found ~64k of it was the
host. **Advertising a core set of our tools and keeping the rest reachable by name is a cheap
experiment against AI-1**, and it is the one thing here that touches our most-measured constraint.

## Caveats

- **Vendor claim.** Nothing was installed; no footage of ours was cut by it.
- **Their evaluation is their evaluation.** The Opus grader is an independent *model*, not a
  person. "72/72" means a model agreed with a rubric the same author wrote.
- **The skill is a different shape from ours.** It edits files with FFmpeg; we own a timeline in
  an app. Most of their tools have no AeroEdits equivalent, and the ones that do (captions,
  overlays, loudness) are already ours in a different shape.
- **42 tools is a lot of surface** — and it exists because the author chose to make a complete
  FFmpeg surface, not because the job needed one. The parts worth taking are the *mechanisms*
  (derived schema, three-state detection, plan fingerprints, the eval harness), not the tool count.
- Their FFmpeg 5.x and Python 3.9 deprecations, and their macOS `ffmpeg-full` keg-only
  gymnastics, are real operational cost — we ship Chrome and Node ourselves and do not ship
  FFmpeg at all, so most of their install trouble does not apply to us.
