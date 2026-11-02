# Roadmap

Updated 2 November 2026. This is the single backlog. Keep work IDs stable; remove shipped
items and put lasting behavior in [Product](product.md) or [Architecture](architecture.md).
Validation and measured results live in [Verification](verification.md).

## Next: STY-2 shorts style

<a id="sty-2"></a>
**STY-2 — Finish the portrait variant from George's references.** The style fields, My library
long-form style, reference proposals and sound set are built. GFX-8 is complete. Step 4 is the
remaining work, waiting for shorts references; it is not a request to rebuild the style system.

1. Break down the references into palette, typography, motion, useful graphic kinds and what
   to leave out. Keep the breakdown in the style.
2. Add portrait motion rules, kinds and linked library examples to the same style. Check fonts
   and files. The current style sound set remains opt-in; the first cues were judged weak.
3. Try one representative graphic on the 25 Oct take with the real face, captions and audio.
   Resolve the over-face placement tradeoff: lifting a low panel can clear platform UI while
   obscuring the mouth. Show the sample before building a full set.
4. Compare relevance, readability at phone size, timing and coherence. George must want to keep
   it. Then test another topic/aspect ratio and rerun EVAL-1 for workflow or quality changes.

The existing Style panel can start reference intake. The richer motion/kinds/examples fields
are available through the tools; dedicated editor lists for all these fields are still absent.

## Checks waiting on George

These do not block unrelated work.

- **FIX-9:** enable Transcription settings → AI agents → Allow, then verify a real Codex
  transcription session. Verify recorded authorization, not just response wording.
- <a id="cost-2"></a>**COST-2:** determine whether the Claude desktop app sends every tool
  upfront. Claude Code and Codex measurements already support on-demand schemas. Change tool
  exposure only if this client needs it; do not assume Codex refreshes lists mid-session.
- Listen to built-in music and real take matching. EQ strength (60% of the measured difference,
  capped at 4 dB) is still a tuning choice, and mood tags need ears.
- Renew the rejected ElevenLabs key in settings, or continue using OpenRouter fallback.
- Supply a longer real Greek recording: EL-B is only 13 seconds, so EL-A's synthetic case
  still carries most of the Greek evaluation.

## Unresolved engineering and review issues

Carry these into a relevant change rather than keeping a second gaps catalogue. Older reports
are evidence to reproduce, not newly verified defects.

| Issue | Next check or constraint |
|---|---|
| Direct actions check the project handle before acquiring the command lock | Recheck project switching under concurrent calls; proposals already validate under the lock. AGT-1 follow-up. |
| Stopping long analysis is supported; resuming is not | Add only for a job that proves expensive to restart. |
| No in-app time/provider-spend report | EVAL-1 measures agent spend; do not report that as the total cost of a video. |
| Thorough review has no matched real-recording baseline | Compare it with fast review before changing the long-form default. |
| Frame-level graphic checks are separate from `check` | Sample face/caption/platform conflicts with `frame_render`; the Agent panel does not embed the sample clip. |
| Over-face layouts can hide captions or mouth or enter platform bands | Resolve in the STY-2 sample; a transparent composition root is not proof the face is visible. |
| Review candidate windows can miss single-word repeats, cut-off words and reworded restarts | Reproduce with labelled cases before changing rules; guard deliberate repetition and meaningful connective words. |
| Some settings lack “changed here” chips (layouts and short settings) | Address with the relevant panel work. |
| Graphic rerenders can take 20–40 s; orphaned unplaced graphic folders can remain after undo | Keep old renders playing; establish a real need before adding draft rendering or cleanup. |

## Deferred work

Order within each group is the suggested order. S ≈ two days, M ≈ a week, L > a week.
None supersedes the shorts style or George's next real edit.

| ID | Work and acceptance boundary | Size |
|---|---|---|
| <a id="auto-3"></a>AUTO-3 | Save an approved workflow as named/versioned instructions, defaults, exceptions and checks; show a readable summary and prove reuse on a second tutorial. | M |
| <a id="loop-1"></a>LOOP-1 | Use accepted/rejected proposals, restored cuts and redone graphics to propose improvements to methods, styles and templates. User accepts each change; no hidden learning. | L |
| <a id="snd-2"></a>SND-2 | Opt-in cues at graphic events: event markers, sound-peak offsets, density limits and ducking, using the style's sound set. Improve the sounds before scaling. | M/L |
| <a id="ftg-2"></a>FTG-2 | Saved selects and B-roll from own footage: reviewed batch, video-only streams, free overlay track, narration untouched, word anchors. Not urgent (1 Nov). | M |
| <a id="ftg-3"></a>FTG-3 | Cache source descriptions/tags from sparse boards against fingerprints; OCR only if keyword search misses. | M |
| <a id="ftg-4"></a>FTG-4 | Natural take ordering, missing/duplicate checks and script-guided assembly with alternatives in one proposal. | M |
| <a id="ftg-6"></a>FTG-6 | Stock providers using the user's key; record licences and download only on acceptance. | M |
| <a id="ftg-5"></a>FTG-5 | Import external word transcripts only if trivially small; not currently needed. | S |
| <a id="lay-3"></a>LAY-3 | Bounded auto layouts on top of `pick`: few changes/minute, screen 2–8 s, camera 2–5 s, punch-in 1.5–4 s, review with list and storyboard. | M |
| <a id="fx-1"></a>FX-1 | Cached per-recording camera segmentation for background removal/blur/replacement; composite in wgpu. | L |
| <a id="lay-4"></a>LAY-4 | Presenter cut-out layout, dependent on FX-1's matte. | M |
| <a id="dlv-3"></a>DLV-3 | Standalone alpha graphics, then constrained FCP7 XML/OTIO handoff. Test in the destination editor and report what was flattened. | M→L |
| <a id="dlv-5"></a>DLV-5 | Real 16:9→9:16 reformat preserving UI targets, cursor relationships and reading time. | L |
| <a id="cost-3"></a>COST-3 | Cheaper narrow reviewers, retained only if EVAL-1 quality holds. Not urgent (1 Nov). | M |
| <a id="ver-1"></a>VER-1 | Repeat Windows bridge-to-export verification on macOS when macOS work resumes. | M |
| <a id="dep-2"></a>DEP-2 | Quarterly dependency review, next January 2027, or sooner for an advisory or measured gain; follow Verification. | S |
| <a id="fx-5"></a>FX-5 | LUT import/simple grade with preview/export parity. Parked: not needed for the channel. | M |

AeroShoot work belongs in the recorder repo: exact segment boundaries (O03, overlap up to one
frame) and removal of the 10 ms system-audio gap at boundaries (O04). The editor tolerates both.

Promote unscheduled ideas only when a real edit needs them: alternate review timelines,
narration fixes, relinking/media bins, cross-project copying, arbitrary reference-range export,
cover frames, title/description drafts, reduced motion, versioned delivery safe zones and
music-beat boards. Do not resurrect dropped scope listed in [Product](product.md#out-of-scope).
