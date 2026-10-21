# Prompt-to-motion workflow for AeroEdits

Reviewed: 21 October 2026.

## Evidence and recommendation

Reference: [Roboverse — Claude Code Can Now Automate Your Videos (Remotion + Opus 5.5)](https://www.youtube.com/watch?v=6_rCyryA6hg).
Read the full exported English auto-generated transcript and expanded description/chapter
list, and inspected a chart-result frame at 3:51 in the browser. This is a workflow review,
not a complete visual-quality assessment or a reproduction of the examples. Model names,
generation times and quality claims are the creator's claims; they were not independently
tested. The on-screen prompts were not fully transcribed, so the briefs below are proposals.

Recommendation: bring the brief → editable composition → preview → independent revision
→ reuse → export loop into AeroEdits. Start with the existing HyperFrames pipeline.
The reference does not establish that Remotion is required to reproduce this workflow.
No application code or rendering dependencies were changed for this review.

## What the video does

| Time | Demonstrated workflow | Product lesson |
| --- | --- | --- |
| [0:15](https://www.youtube.com/watch?v=6_rCyryA6hg&t=15s) | A local coding agent sets up a project, dependencies and Remotion instructions, then opens Studio. | Setup and a persistent project sit behind a simple creative entry point. |
| [2:00](https://www.youtube.com/watch?v=6_rCyryA6hg&t=120s) | Describe a podcast name card by purpose, text and second-by-second behavior; preview it. | A useful brief specifies content and timing, not just an aesthetic. |
| [2:43](https://www.youtube.com/watch?v=6_rCyryA6hg&t=163s) | Request changes to the background and design, explicitly on a copy. | Revisions preserve a usable original. |
| [3:26](https://www.youtube.com/watch?v=6_rCyryA6hg&t=206s) | Create an animated comparison chart, then revise bounce and colors while preserving a liked glow. | Feedback records what to change and what to preserve. |
| [5:17](https://www.youtube.com/watch?v=6_rCyryA6hg&t=317s) | Generate a 30-second app ad and an AI explainer. | The same loop expands from a graphic to a sequence of scenes. |
| [7:29](https://www.youtube.com/watch?v=6_rCyryA6hg&t=449s) | Build a 60-second intro using four earlier compositions. | Approved work becomes material for larger compositions. |
| [9:03](https://www.youtube.com/watch?v=6_rCyryA6hg&t=543s) | Render originals and revisions to seven named MP4s, then number them. | A render queue packages multiple deliverables in a deliberate order. |

The workflow authors animation code and renders it; the description does not establish a
generative-video model as its core. Nor does it demonstrate automatic editorial improvement
of an existing screen recording. Our opportunity is to connect this authoring loop to the
recording, transcript and telemetry already available in AeroEdits.

## Current app versus missing product flow

| Area | Existing local implementation | Addition to evaluate |
| --- | --- | --- |
| Setup | App-managed Node/Chrome and pinned HyperFrames runtime | Show runtime readiness in the creation flow. |
| Authoring | Blank/template/library creation; `graphic_write_file`, variables and brand kits | A structured creation brief and persistent job state. |
| Placement | `graphic_add` supports a timeline time or transcript word | Create from selected words, retaining their context. |
| Preview | Background rendering, playback, `frame_render`, storyboard | Draft preview and synchronized original/revision comparison. |
| Revisions | Library version numbers and project timeline history | Independent project graphic variants and source snapshots. |
| Reuse | Library copies, `useWhen`, brand kits and validation gate | Promote an approved result into a reusable choice. |
| Longer sequences | Timeline graphics and shorts provide integration points | Explicit scene plans, composition references and timing rules. |
| Delivery | Graphic render cache and finished video/short export | A queue for selected graphic variants and standalone sequences. |

Evidence: [graphics actions](../src-tauri/src/mcp/actions.rs),
[graphics commands](../src-tauri/src/commands/graphics.rs),
[library](../src-tauri/src/graphics/library.rs),
[renderer](../src-tauri/src/graphics/render.rs),
[Graphics panel](../front-end/src/components/graphics/GraphicsPanel.tsx),
[Agent panel](../front-end/src/components/agent/AgentPanel.tsx).

Important distinctions:

- The Agent panel reviews actions and proposals; it is not an embedded creative chat.
  Existing external-agent MCP/CLI access can support an initial pilot without building
  an in-app agent host first.
- `project_graphic_set_impl` explicitly changes variables and brand source without an undo
  step. `graphic_write_file` writes source directly. Timeline undo is not source rollback.
- Library `version` metadata does not by itself establish retained, selectable historical
  source versions. A project revision system must preserve the actual files.
- The inspected graphic renderer uses 30 fps and a 1920-pixel long side with cached WebM
  output. MP4 video export and transparent overlay delivery are separate deliverables.
- Full-frame graphics use the app's cut entrance contract. Camera overlays and short slots
  have specific rules; generic fades and a transparent background do not establish arbitrary
  overlay support everywhere. Preserve these rules during the first implementation.

## Proposed user experience

In Graphics, add **Create with AI**. From a transcript selection, offer **Explain this
visually**, prefilled with the selected speech and placement. A compact brief contains:

- Purpose: lower third, comparison, process explanation, product sequence or intro.
- Exact copy or structured chart data, plus the chosen brand and local assets.
- Duration, aspect ratio, placement and scene beats.
- Reference/style notes, with separate instructions for what must remain unchanged.

Example for a Greek AI tutorial:

> Explain “prompt → model → answer” in an eight-second graphic at the selected sentence.
> Use Greek labels and our brand kit. Reveal each stage as it is mentioned, highlight the
> active stage, and hold the completed diagram for reading.

Generate a draft, play it in the actual edit, and allow a follow-up such as “reduce the
bounce, preserve the colors and timing.” That creates variant B with its own source folder;
A remains available. Compare both at the same relative time. **Use this version** replaces
the selected placement as one undoable timeline action. Keep library saving as a separate
explicit operation under the existing workflow.

Internal narration cues need explicit timing. A starting word anchor places a whole graphic;
it does not automatically synchronize every internal animation beat. Initially store beat
offsets relative to the graphic. Later derive them from transcript word timings and define
what happens when cuts or duration changes invalidate those cues.

## Implementation order

1. **Make revisions reliable.** Add an independent graphic-copy operation, immutable source
   snapshots and parent/variant metadata. Copy required local assets and variable values.
   Stage source writes, validate, then publish a complete candidate. A cancelled or failed
   job must leave the current placement usable. Reuse the renderer's existing content-based
   cache rather than treating a variant label as proof of identical content.
2. **Pilot the creative loop through existing MCP actions.** Build one lower third, one
   data-driven chart and one Greek process explainer. Persist briefs, feedback, source
   snapshots and preview references. An agent can orchestrate this before a full embedded
   chat is available. Do not claim that an existing action already performs variant creation.
3. **Expose Create / Revise / Compare / Use in Graphics.** Introduce job progress, cancel,
   failure/retry and revision selection. Check the placement's project revision before
   applying a completed result; stale work remains a draft for review.
4. **Add scene assembly and batch delivery.** Store ordered scene references with pinned
   graphic versions, durations, transitions and audio decisions. Begin with flat sequences;
   nested compositions need dependency tracking and cycle checks. Add queue status,
   cancellation, output naming and per-item errors. Export transparent overlays only through
   a format that supports alpha; ordinary H.264 MP4 is for opaque delivery.

Likely code touchpoints: `graphics/` for source/version storage, `commands/graphics.rs`
and `mcp/actions.rs` for operations, `GraphicsPanel.tsx` and `GraphicSection.tsx` for UI,
and the existing export pipeline for queued delivery. These are proposed changes.

## First acceptance exercise

Use one real AeroEdits tutorial and a fixed brand kit. Produce the three pilot graphics,
revise each once, then assemble a short intro using the approved results. Check that:

- Original source and rendering remain reproducible after revision and reopening.
- Using a variant can be undone without losing either source version.
- Requested changes happen while preserved colors, wording and timing stay intact.
- Greek copy and long labels fit both 16:9 and 9:16.
- Graphics remain readable with the actual recording, captions and camera present.
- Motion plays coherently; still-frame technical checks alone cannot establish this.
- Failed, cancelled or stale jobs do not replace the current edit.

Record time to acceptable output, manual corrections, revision accuracy and render failures.
Only introduce a Remotion adapter if a matched practical test identifies a specific benefit
that warrants another runtime and composition contract. No engine benchmark was run here.

This extends the existing [tutorial motion workflow research](tutorial-motion-workflows.md)
with the reference video's emphasis on preserved variants and composition reuse.
