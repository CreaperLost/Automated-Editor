# Open questions

Two lists: **contradictions** found while reading the docs against each other and the code, and
**decisions** the research leaves for George. Answer a question here, then move the answer into
[vision.md](vision.md) or the relevant plan.

## Contradictions to fix

| # | What | Where | Suggested fix |
|---|---|---|---|
| C1 | `SKILL.md` § Telemetry polish says the three touches are "on by default … leave them on", then says "Both are off by default; switch them on only when the user asks". Sound effects are off by default since 20 Oct; bubble hide is on. An agent reading this can't tell which is which. | [`SKILL.md:139–147`](../../src-tauri/src/mcp/skill/SKILL.md) | Say per item: `webcamAutoHide` on, `windowCrop` on (when telemetry exists), `soundEffects` off |
| C2 | Decision 4 says "No keystroke capture. The recorder is unchanged." Keyboard telemetry v1 now ships (typing stretches and shortcuts, never text). | [ai-readiness § Decisions](../plans/ai-readiness.md#decisions-2026-10-13) vs [keyboard-telemetry.md](../keyboard-telemetry.md) | Amend the decision: "no typed text is ever captured" |
| C3 | Graphics v1 says a placement "stores an anchor in the recording's time (a word id when there is one)". The code resolves `atWord` to an edited time once and stores a plain clip (`mcp/actions.rs` `graphic_add`). Cuts before it still ripple it along, but the word is not kept. | [ai-readiness § Graphics v1](../plans/ai-readiness.md#graphics-v1-agreed-2026-10-15) vs [short-form review](../short-form-reference-review-2026-10-21.md#what-the-current-code-already-provides) | Either store the anchor (needed for the beat plan anyway) or update the plan text |
| C4 | Principle 6, "a brand kit feeds every graphic **and caption**". Captions use one embedded font and ignore the kit. | [ai-readiness § Principles](../plans/ai-readiness.md#principles) vs [herk § 4](../herk-edit-kit-research-2026-10-21.md#4-extend-captions-and-sound-as-native-editable-systems) | Mark as not yet true, or fold into T5 |
| C5 | Motion research says our HyperFrames skills cover "3D, shaders"; the gate forbids network and the authoring rules allow only WAAPI/CSS. | [motion-graphics-research](../motion-graphics-research.md#current-conclusion) vs [gaps #4](../agent-video-gaps-and-proposals-2026-10-21.md#4-graphics-are-capped-at-web-animations-and-css) | Test one three.js / GSAP composition under the gate, then update both docs |
| C6 | `phase-2-handoff.md` still says branch `ai-readiness` and "nothing is committed". | [phase-2-handoff.md](../plans/phase-2-handoff.md) | Add a "historical, superseded by ai-readiness.md" header |
| C7 | Backlog task 1 says the cut-out layout "needs task 10"; layouts.md says it needs task 9's matte. | [future-work #1](../future-work.md#1-more-layouts) vs [layouts.md](../plans/layouts.md) | Fix the backlog line |
| C8 | The reference review and gaps doc contain errors the [second opinion](../second-opinion-video-research-2026-10-21.md#a-errors-and-overreach) checked against the captions: Tao's 49m30s is a first draft (3–4 h with feedback); "106 SFX" and Joseph's "12 hours / 4.5B tokens" aren't in his track; the text/person/object/graphic subjects were confused with his per-line scene choices; the tie is labelled "Chase AI #5" (#5 is Marcin); "six AE videos" (captions show two); £5.90 compared with $15.05 as "double". | [reference review](../reference-video-review-2026-10-21.md), [gaps](../agent-video-gaps-and-proposals-2026-10-21.md) | Correct the numbers and gap #3 in place, or add an errata note linking the second opinion |
| C9 | The ten-videos review names agent tools and files that don't exist or aren't agent-facing: `project_view`, `project_save_as`, `proposals_undo` (it's `undo_batch`), `project_diagnose`, `transcript_get(word_times_external)`, an `aeroedits://` handler, `commands/check.rs` (it's `src/check.rs`). `transcript_cut_words*` and `transcript_caption_edit` are UI commands only. It also says "whisper alignment" (it's wav2vec2), "five" templates (six), "single audio track per source" and "timeline: no write" (`propose_edit` places, moves and trims clips). | [ten-ai-editor-videos](../ten-ai-editor-videos-2026-10-21.md) vs the code | Correct the code references before using its proposals |
| C10 | Ten-videos #10 (Convex) had no captions or description; its section is inferred from the company's product. | same | Mark as unverified or drop |
| C11 | The style banned list ("no holds over 1 s, no blur-ins") conflicts with tutorial needs (reading time) and with Ben's own advice. | [gaps #2](../agent-video-gaps-and-proposals-2026-10-21.md#2-there-is-no-house-style-as-a-first-class-object) vs [2nd § F](../second-opinion-video-research-2026-10-21.md#f-recommendations-a-competent-editor-would-challenge) | Don't ship a borrowed banned list as defaults |

## Decisions waiting for George

**Q1. Style kit: what is it, and where does it live?**
Extend the brand kit, or a separate object that *references* a brand kit? App-level like brand
kits, or per project? One profile, or separate long-form and shorts profiles (chatcut and sf2
both argue for separate)?

**Q2. One plan model or two?**
The beat plan (graphics, captions, sound for long-form) and the idea-first short
([shorts-redesign Phase C](../plans/shorts-redesign.md#phase-c-the-idea-first-short-the-big-one))
both store "intent anchored to words". Should they be one model with a scope (video / short)?

**Q3. Sound before or after the plan?**
See [roadmap.md](roadmap.md#a-reconciled-order). A cheap interim exists: the agent places
imported SFX files with `propose_edit` today, with no new tools.

**Q4. The animation ceiling.**
Vendor a pinned GSAP into the graphics runtime (like Node, Chrome and HyperFrames), or keep
WAAPI/CSS and say so plainly in the skill?

**Q5. Where does the user approve a plan?**
The Agent panel is deliberately not a chat. Does it grow a Plan tab, or is the plan its own
panel next to the transcript?

**Q6. What is the near-term product focus?**
The research is heavy on short-form reels; the vision says tutorials first, gaming next.
Is the next milestone "a great long-form tutorial", "a great tutorial short", or both?
The second opinion argues for **accurate, revisable screen tutorials** and against using
viral-reel polish as the yardstick.

**Q7. Who owns judgement that the app's own AI already does?**
Chapters and short suggestions call a cloud model inside the app; the agent could do both
from the transcript at no extra cost. Keep both paths, or let the agent own them when connected?

**Q8. Evaluation set.**
Every doc proposes a fixed comparison (English tutorial, Greek clip, talking-head short; time,
corrections, cost per stage). Should building that be Phase 0, before any feature?

**Q10. Fast or thorough by default?**
The tutorial template reviews fillers in candidate windows. Should a full-transcript thorough
pass be offered every time, be the default, or stay opt-in?

**Q11. Where do the user's editing rules live, and who may write them?**
Creators let the agent rewrite its own skill. Ours is installed and overwritten by the app.
Options: project rules, app-level rules, or both; the agent suggests, the user accepts.
Is a self-editing rule file something you want at all?

**Q12. How autonomous should verify be?**
Jason's agent loops "until the cut is clean"; Chase AI warns against autonomous loops. Should
`check` fixes and re-renders retry by themselves (bounded), or always stop for you?

**Q13. Colour, folder watch, outside transcripts — in scope?**
The ten-videos review asks for a colour/LUT action, a folder watch for unattended runs, and
importing Deepgram-style transcripts. Each is small-to-medium; which matter for your channel?

**Q9. Organising the originals.**
This folder summarises without moving anything. Should the dated research files move into
`docs/research/` (and `plans/` stay as is)? Links inside the docs would need updating.
