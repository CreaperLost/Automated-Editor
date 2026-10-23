# AeroEdits: what it is and what it does today

Status as of 21 Oct 2026 (branch `Research-AI-readiness`, last merge `92f80d3`). This is the
one description of the product. How it is built is in [architecture.md](architecture.md); what
is missing is in [gaps.md](gaps.md); what we do next is in [roadmap.md](roadmap.md).

## In one paragraph

AeroEdits is a free, open-source desktop editor (Windows first, macOS second) for
**screen-recorded tutorials**. It opens `.aero` recordings made by AeroShoot: screen, webcam,
microphones and system audio, **plus mouse, keyboard and window telemetry**. It turns them into
a clean, well-framed long video and vertical shorts. The AI is **the user's own agent** (Claude
Code, Codex, the Claude app…) connected over MCP or the `aeroedits do` command line and paid by
the user's own subscription. The app is the tool server, the agent is the editor, the user
approves.

![How the user's agent works with AeroEdits today](images/ai-collaboration-today.svg)

## Who it is for

George's own channel first: AI tutorials in English and Greek, long form and shorts. Gaming is
next. Success means a viewer can follow the action, understand the concept and see the result,
with predictable revisions and reasonable production time. A pretty showreel is not the target.

**The acceptance test for any feature:** does it make tutorials more accurate, readable and
revisable, at a lower **cost per accepted video** (subscriptions, generation, rendering, human
review and retries, divided by videos actually published)? And for agent features: *does it make
the agent's output more checkable without the user watching every frame?*

## Principles

1. **The app is a tool server, the agent is the editor.** The agent decides; the app does
   everything deterministic (cuts, rendering, alpha, compositing, mapping time through edits).
2. **The agent proposes, the app checks, the user accepts.** Agent judgement calls arrive as
   batches, one undo step each. Built-in passes apply directly, like pressing the button.
3. **Text first, pixels when needed.** Transcript and telemetry (~10–20 KB) are the working
   surface; frames only at decision points.
4. **Check the output, not the plan.** Graphics are checked over real footage, cuts against the
   transcript.
5. **Pin to meaning, not to time.** Things hang off words, clicks and recording time, so cuts
   don't knock them out of place.
6. **One source for the look.** The project's style (brand colours, fonts and logo, the
   graphics look, the caption look and the frame) dresses every graphic, caption and frame.

## Where AeroEdits wins, and must keep winning

These are capabilities we *have*. That a competitor's video doesn't show one is not proof they
lack it, so we say "we have", not "only we have".

- **Tutorial signals no transcript-only editor has:** clicks, typing, shortcuts, the front
  window, a resting pointer, the face position. They drive zooms, layouts, window crop and the
  bubble hiding.
- **Cut accuracy:** word-edge refinement, per-source
  vetoes in two-source mode, tagged sounds.
- **Approve-before-apply:** revision-checked proposals, one undo step per batch.
- **Verification without a model:** `check` before export, the graphics gate, `storyboard`.
- **Editable, deterministic graphics:** HyperFrames source the agent can rewrite and the app
  re-renders, so a better composing model shows up as a better edit.
- **Low running cost:** one line of text per pass; the rules come once, from `template`.
- **A typed API instead of computer use:** no OS accessibility permissions, no screen scraping.

## Features today

| Area | What it does | Driven by |
|---|---|---|
| Timeline | Tracks of clips (V1…, A1…), linked picture and sound, magnetic ripple, non-destructive, undo/redo | Clocks map recording time ↔ edited time |
| Import | `.aero` recordings, video, audio, images, folders; any recording or import is just an asset | — |
| Transcription | Online by default: Scribe detects the language (ElevenLabs, then OpenRouter); Parakeet Unified EN or Verbatim (TDT v3) locally for English | The user's key, or the local model |
| Cleanup | Mouth noises (mute), mouth-click repair (`adeclick`), silence (audio), gaps without words (transcript), fillers and retakes (rules + AI review), manual word cuts | Rules; AI review by OpenRouter/OpenAI, local Qwen 4B without a key |
| Speed | 0.25×–8× on clips or ranges, pitch kept | Manual, or the agent on request |
| Zooms | Suggested from clicks, dwell, click clusters and typing (caret follow) | Telemetry |
| Layouts | Bubble (3×3 grid, S/M/L, shapes), side by side (fit/fill, any share), camera only (punch-in), screen only; a base layout plus sections anchored in recording time; favourites; **Pick from the recording** (rest, busy, hook rules) | Telemetry, face tracks |
| Camera | Face centring on every camera slot (YuNet, found once per recording), hand framing per clip | Local model |
| Telemetry polish | Window crop to the front window, bubble hides under activity, shortcut badges, click/whoosh lane (off by default) | `windows.jsonl`, `keys.jsonl`, `events.jsonl` |
| Captions | The style's title font (else Inter Display, bundled) at its caption weight; one emphasis word per caption in the accent, picked by the agent; pop-in, karaoke or static; English and Greek, 16:9 and 9:16 | Transcript |
| Audio | Track gain, denoise, speech ducking, loudness normalisation to -14 LUFS by default with a peak leveller, crossfades at cuts | DSP |
| Graphics | HyperFrames compositions: 6 tutorial templates + blank in a light app style, one style per video (brand, graphics look with presets and five controls, preview), an app-level library behind a no-model quality gate, short slots; exports render them at their fps and 2× density above 1080p | Pinned runtime (200 MB download on first use) |
| Shorts | A Shorts workspace tab; word-range shorts that follow the video's edits or get their own timeline; **take shorts** (a new camera take with footage from the long video under it, each piece at its own speed; graphics full frame, split beside the face or over it); own layouts | Agent or the app's cloud AI picks moments |
| Chapters | YouTube chapters from the transcript, checked before export | The app's cloud AI |
| Review | `check` (≈12 defect classes, one-click fixes, each its own undo step) and `storyboard` (one contact sheet of the edit) | No model |
| Export | H.264/AAC, 720p–4K, GPU encoders (NVENC, AMF, QSV, else x264); shorts at 1080×1920 | FFmpeg (Windows/Linux), AVFoundation (macOS) |

## Three kinds of "AI" in the app

Most research docs blurred these. Keep them apart.

```mermaid
flowchart LR
    subgraph A["① Deterministic app logic (no model)"]
      A1[silence / gaps]
      A2[zooms from telemetry]
      A3[layout pick]
      A4[check · storyboard · graphics gate]
      A5[render · composite · export]
    end
    subgraph B["② In-app models (local or the user's keys)"]
      B1[Transcription<br/>Parakeet · Scribe]
      B3[Filler/retake review<br/>OpenRouter · local Qwen 4B]
      B4[Chapters · short suggestions<br/>cloud]
      B5[Face detection<br/>YuNet]
    end
    subgraph C["③ The user's agent (MCP / CLI)"]
      C1[Follows the skill + template]
      C2[Asks the user, runs passes]
      C3[Judgement: extra cuts, short picks,<br/>graphics choice and authoring]
    end
    C -->|calls| A
    C -->|triggers| B
```

- **①** is where most of the editing quality comes from: cheap and repeatable.
- **②** exists because a subscription agent can't turn audio into words, and shouldn't pay
  Opus prices for filler review.
- **③** orchestrates and makes the judgement calls. It never touches project files directly,
  except graphic source.

## How an agent edits a tutorial today

The agent reads the shipped skill (`src-tauri/src/mcp/skill/`), then follows the Tutorial
preset's method:

1. `preset("tutorial")` → steps, rules, the user's own rules and the preset's choices ("Same as
   last time?").
2. `create` / `status` / `scan` → mics, cameras, mouse data, transcribers, dry-run estimates.
3. `transcribe` (online, the language detected).
4. For each pass (mouth noises → silence → gaps → fillers → layouts → zooms): dry run
   (`apply: false`), ask the user with real numbers, then run. One line back, one undo step.
5. Captions off (unless a short). Graphics only if the user wants them: `graphic_add(atWord)` →
   `graphic_render` → `frame_render`.
6. `chapters` last, then `check` (+ fixes) and `storyboard`.
7. Export only on the user's go.

**Automatic mode** ("auto", "same as last time") skips the questions but still dry-runs every
pass, and stops when a pass would cut more than a quarter of the video, all passes more than a
third, or the numbers are far from `scan`.

**Fast or thorough review** is asked once per video (and saved). Fast looks only where the
rules flagged something; thorough also reads every kept word for abandoned sentences,
restarts and tangents, proposes those cuts, and checks the joins. Either way the closing
summary keeps what was done, looked at, heard and approved apart.

Where judgement sits:

| Step | Who decides |
|---|---|
| Transcribe, mouth noises, silence, gaps, layouts, zooms | The app (models and rules); the agent relays numbers |
| Fillers and retakes | App rules + the app's AI review; the agent reviews only without a key (`fillers_review`) |
| Extra cuts (tangents, missed retakes) | **The agent** → `propose_cuts` → the user accepts |
| Which shorts | **The agent** (`shorts` template) or the app's cloud AI |
| Which graphics, where, with what text | **The agent** |
| Chapters | The app's cloud AI |
| Quality review | `check` + one `storyboard` (stills only) |

So today the agent is an **operator with good manners**: it sequences passes, asks with numbers
and reports in one line. Its creative surface is narrow: extra cuts, short picks and graphics.

**What the user sees:** an **Agent panel** (docked, tabbed with the Inspector; deliberately not a
chat) with proposal batches, reasons, per-item play buttons, Apply / Reject / Undo, a history
of direct actions and running calls; **File > AI agents** for on/off, one-click connect for the
Claude app and Codex, skill install and the autopilot switch.

**Reuse loops that exist:** saved template answers; `graphic_templatize` → `graphic_check` →
`library_save`; brand kits; saved looks in the Templates menu. Not reusable yet: the user's
editing *method*, style rules beyond colours, sound choices, a plan of the edit.

## The agent surface: 56 tools

Registered in [`mcp/skill.rs`](../src-tauri/src/mcp/skill.rs) (2),
[`mcp/tools.rs`](../src-tauri/src/mcp/tools.rs) (14) and
[`mcp/actions.rs`](../src-tauri/src/mcp/actions.rs) (40). Every tool is also a CLI command,
`aeroedits do <tool> --key value`. Tools return **text and PNG only**.

| Group | Tools | Applies how |
|---|---|---|
| Guide | `template`, `template_save` | — |
| Read | `project_info`, `transcript_get`, `timeline_get`, `playhead_get`, `telemetry_get`, `layout_get` | — |
| Look | `frame_render`, `timeline_view`, `storyboard` | — |
| Propose | `propose_cuts`, `propose_edit`, `proposals_list`, `proposal_withdraw`, `undo_batch` | Waits for the user in the Agent panel (unless autopilot) |
| Project | `status`, `scan`, `open`, `create`, `undo` (`run: true`: the agent's whole run), `redo`, `checkpoint`, `settings`, `media_import` | Directly, one undo step each; the app saves a checkpoint before each agent run |
| Speech | `transcribe`, `mouth_noises`, `silence`, `fillers`, `fillers_review`, `speed`, `transcript_fix` | Directly; `apply: false` is a dry run; word fixes are not undo steps |
| Picture | `layouts`, `zooms` (automatic and single zooms), `faces` | Directly; dry run available |
| Shorts | `shorts`, `short_create`, `short_footage`, `short_from_take` | Directly |
| Graphics | `graphics`, `graphics_install`, `graphic_add`, `graphic_set` (incl. move, remove), `graphic_write_file`, `graphic_render`, `graphic_check`, `graphic_templatize`, `library_save`, `library_remove`, `brand_kits`, `brand_kit_set`, `brand_pick` | Directly (source edits, kits and the library have no undo) |
| Finish | `chapters`, `check`, `export`, `export_status` | Directly; export only on the user's go |

The human UI has 137 Tauri commands. Since 21 Oct the agent can also author zooms, edit chapters
and shorts, fix words, manage brand kits and the library, set media roles and cancel exports;
what stays UI-only is mostly playback, preview hit-testing and API keys. Every writing tool
takes the project's `project` id. Two agent tools have no UI (`scan`, `telemetry_get`). Details in [gaps.md](gaps.md#3-agent-surface).

## Platforms

| | Windows | macOS |
|---|---|---|
| Edit, zooms, cleanup | Yes | Yes |
| Preview and playback | wgpu preview drawn under a transparent webview, WASAPI audio | Swift/AVFoundation bridge |
| Export | FFmpeg (bundled) | Swift/AVFoundation bridge (`AEROEDITS_MEDIA_BACKEND=ffmpeg` to try the shared one) |
| Local transcription | CUDA (with the GPU pack), DirectML or CPU | Not in the macOS build yet |

## What is verified and what is not

From the build logs. Know this before trusting a feature in a real session.

- **Verified on Windows:** the MCP bridge and pipe end to end; proposals → accept → undo; actions
  over the CLI; graphics install and render in the dev app; `check` on a real recording; layouts
  in the dev app; the filler-review evaluation.
- **Not verified:** macOS for most of the MCP and graphics path; a real Codex session using write
  tools; the agent's full templatize → check → save loop over MCP; a brand kit's font and logo in
  a real render; how MCP clients treat very long calls.

## The AI loop we are heading to

Today the agent is an operator; the proposals in [roadmap.md](roadmap.md) make it a director:
it reads the user's rules, writes a plan the user approves, has the app build that plan
deterministically, watches the result move, and proposes what to remember. The app's job grows
from "run passes" to "keep creative decisions durable": notes, plans, cues, variants and
checkpoints that survive cuts, undo and reopening. This is a direction, not a decided design.

![The AeroEdits AI loop: today and tomorrow](images/ai-loop-today-vs-tomorrow.svg)

What does **not** change: no chat panel in the app; cleanup passes with dry runs, proposals,
revision checks and single-step undo; HyperFrames as the only renderer; "a pass costs a line
of text".
