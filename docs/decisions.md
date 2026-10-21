# Decisions

Two lists: what George has **decided** (don't reopen without a reason), and what is **waiting
for a decision**, each with a recommendation. When a question is answered, move it to the
settled table with the date and update the roadmap item it blocks.

## Settled

| Decision | Date | Where it came from |
|---|---|---|
| AI only through the user's own agent over MCP / CLI. No chat panel in the app, no model calls on that path | 13 Oct | [ai-readiness](plans/ai-readiness.md#decisions-2026-10-13) |
| Transport: `aeroedits --mcp-stdio` bridge → a named pipe only this OS account can open (macOS: 0700 socket, peer uid checked). No port, no token, no script tool | 13 Oct | ai-readiness |
| The existing in-app AI passes stay (transcription, fillers/retakes, chapters, shorts); the agent is a second source of the same suggestions | 13 Oct | ai-readiness |
| HyperFrames is the only graphics engine; the app runs renders; WebM VP9 alpha is the render cache, ProRes 4444 for macOS and for handing a graphic to another editor | 13 Oct | ai-readiness |
| Keyboard telemetry never holds typed text: only typing stretches with a key count, and shortcuts (replaces 13 Oct "no keystroke capture") | 17–18 Oct | [keyboard-telemetry](contracts/keyboard-telemetry.md) |
| Window telemetry never holds titles or contents | 18 Oct | [window-telemetry](contracts/window-telemetry.md) |
| Agent judgement calls are proposals the user accepts; built-in passes apply directly (one undo step each) | 14 Oct | ai-readiness 2b |
| One set of actions, two doors: every action is an MCP tool and an `aeroedits do` command | 14 Oct | ai-readiness 2b |
| No one-call cleanup: each pass is dry-run, approved, then run; "auto" makes the choices but still dry-runs | 15 Oct | ai-readiness 2d |
| Chapters last, right before export | 15 Oct | ai-readiness 2d |
| Graphics come after the cuts, are never chopped, play full screen over everything (or in a short's slot); slots, not safe zones | 15 Oct | ai-readiness § Graphics v1 |
| Graphics runtime (Node, Chrome, HyperFrames, pinned and hash-checked) downloads on first use; nothing on `PATH` is used or changed | 15 Oct | ai-readiness |
| A graphic is saved to the library only at the user's word; the gate decides "template" vs "as is" | 16 Oct | ai-readiness |
| **No migration:** old project files are never converted or kept compatible; old fields are deleted | 4 Oct | premiere-tracks; repeated in every plan |
| Shorts use only what the main edit keeps; caption text fixes are shared with the video; one default length target, 30–60 s | 17 Oct | [plans/shorts.md](plans/shorts.md) |
| Take shorts: footage only from what the video keeps, ducked under the voice, split look by default, the take goes through the normal cleanup | 18 Oct | plans/shorts.md |
| Sound effects (clicks, whooshes) off by default | 20 Oct | backlog |
| Fillers/retakes default to rules + AI review; OpenRouter/OpenAI when a key is set, local Qwen 4B only without one; `apply: true` is not the default for either | 21 Oct | [local-review-eval](plans/local-review-eval.md) |
| Greek is transcribed online only until a local model is good enough | 20 Oct | backlog |
| Layouts: one shared style per timeline, layouts only arrange; sections anchored in recording time; favourites copied when applied; default transition Auto; moves play inside the section; face centring on every camera slot; shorts keep their own base and sections; bubble on a 3×3 grid with S/M/L; side by side has fit and fill; layouts bound to keys by the user, no fixed slots | 21 Oct | [plans/layouts.md](plans/layouts.md#decisions-21-oct-2026) |
| Re-voicing and lip sync paused until a local, commercially licensed stack is good enough | 18 Oct | [research/revoice.md](research/revoice.md) |
| Licence rule for any bundled model: MIT, Apache-2.0, GPL-3.0 (as a separate component), OpenRAIL. Never non-commercial, research-only or revenue-capped | 18 Oct | research/revoice.md |

### Not doing

Proposed by the research and consistent with everything above. The reasons are the corrected
ones (the second opinion softened several).

| Idea | Status | Why |
|---|---|---|
| Remotion (or any second renderer) | No | Two style systems and two failure modes. A maintenance choice, not a claim that HyperFrames is better |
| After Effects integration | Not now; at most an optional backend pilot later | MCP gives control of an engine, not art direction. AE's editable-handoff advantage is real |
| Generative video as a core dependency | No | No editability or determinism. Generated clips are fine as **user-supplied assets** in a slot (`media_import` already works) |
| A public skill / template marketplace | Not now | No demonstrated demand |
| Driving Premiere or another NLE by computer use | No | Fragile UI, high upkeep; our typed API owns edits |
| Writing our own recorder features into the editor | No | Corrections come from AeroShoot or a small narration recorder, not screen capture in the editor |
| Borrowed style bans as defaults (no holds over 1 s, no blur-ins) | No | Tutorials need reading time; ship bans only as a user's own style rules |
| Avatars, live co-editing, a big model catalogue, cloud rendering | Deferred | Scope before proving tutorial outcomes |
| Gaze-aware cutaways, 3D scenes | Only when a chosen style needs them | — |

## Waiting for George

Ordered by what they block. Each has a recommendation; a one-word "yes" adopts it.

<a id="q-focus"></a>
**Q-FOCUS · What is the next milestone: a great long-form tutorial, a great tutorial short, or both?**
The research is heavy on short-form reels; the vision says tutorials first.
*Recommendation:* the long-form tutorial first, measured by EVAL-1 (accurate cuts, readable UI
and captions, cost per accepted video). Shorts improve as a side effect (captions, sound, search
are shared) and get their own push with SHT-1. Blocks: the order inside Next.

<a id="q-rules"></a>
**Q-RULES · Which user-owned artefact first: project rules, a style kit, or a sound library?**
All three are the same shape: a file the user owns that the agent reads and proposes edits to.
*Recommendation:* **project rules first** (AGT-3): cheapest, covers method and taste in words,
and the style kit (STY-1) later becomes structured fields in the same store. Read the three
published skills before designing it (a day). Rules at two scopes (project, all projects); the
agent only proposes, the user accepts; the installed `SKILL.md` stays app-owned.

<a id="q-plan"></a>
**Q-PLAN · One plan model or several?** The beat plan, the idea-first short, production tasks
(editor directions, asset requests), review records and timeline comments all store "intent or
evidence anchored to words / source time".
*Recommendation:* **one anchored-note primitive** (NOTE-1) with a `kind`, and build the beat plan,
review records and short ideas on it. Blocks: NOTE-1's data model.

<a id="q-sound"></a>
**Q-SOUND · How small is "minimal sound", and does music come first?**
*Recommendation:* SND-1 = clip gain + fades, SFX and music as their own tracks, a
speech-relative music level and a loudness target, a licence field. No library browser, no
generation, no beat grid. Cue placement at graphic events (SND-2) only after captions and the
render policy. Music is a user-supplied track until a tutorial proves it needs more.

<a id="q-anim"></a>
**Q-ANIM · The animation ceiling: vendor a pinned GSAP into the graphics runtime, or keep
Web Animations/CSS and say so?**
*Recommendation:* run the GFX-2 spike (half a day) and decide on its result. WAAPI/CSS already
does easing and squash-and-stretch; only adopt GSAP if a real template needs it and it passes
the gate deterministically.

<a id="q-ai-owner"></a>
**Q-AI-OWNER · Who owns chapters and short suggestions when an agent is connected?**
Both call the app's cloud model today; the agent could do both from the transcript at no extra cost.
*Recommendation:* keep the app's passes for users without an agent; when an agent is
connected, the templates let the agent write them (chapters through the AGT-1 `chapters set` op).

<a id="q-thorough"></a>
**Q-THOROUGH · Fast or thorough by default?**
*Recommendation:* ask every time in step mode ("fast or thorough?"), remember the answer as a
saved answer, default to fast in auto mode. (AGT-2.)

<a id="q-verify"></a>
**Q-VERIFY · How autonomous should verification be?** Some creators loop "until the cut is
clean"; others warn against autonomous loops.
*Recommendation:* bounded: the agent may apply `check`'s safe fixes and re-check **once**, must
report what it changed, and always stops for the user before export.

<a id="q-scope"></a>
**Q-SCOPE · Colour/LUT, folder watch, outside-transcript import: in scope for your channel?**
*Recommendation:* outside-transcript import is small (FTG-5) and fine any time; folder watch
(AUTO-1) after AGT-3 so it can run a saved method; colour (FX-5) only if your camera footage
needs it. All stay in Later.

<a id="q-hyperframes"></a>
**Q-HYPERFRAMES · Track HyperFrames upstream?** The runtime we pin (0.8.162) is HeyGen's public repo.
*Recommendation:* stay pinned; review the upstream changelog at each deliberate bump; keep our
gate and templates in this repo (they encode our product rules, not the engine's).

### Old question numbers

Q1 → Q-RULES (style kit part) · Q2 → Q-PLAN · Q3, Q16 → Q-SOUND · Q4 → Q-ANIM · Q5 (where to
approve a plan) → answered by Q-PLAN: the Notes lane and the Agent panel · Q6 → Q-FOCUS ·
Q7 → Q-AI-OWNER · Q8 (evaluation set) → adopted as EVAL-1 in Now · Q9 (organise the docs) →
done 21 Oct · Q10 → Q-THOROUGH · Q11, Q14, Q15 → Q-RULES · Q12 → Q-VERIFY · Q13 → Q-SCOPE ·
Q17 → Q-HYPERFRAMES.
