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
| **Milestones, in order:** (1) a great long-form tutorial, (2) tutorial shorts, (3) gaming shorts and gaming videos. Later tiers are ordered by this | 21 Oct | Q-FOCUS |
| **User-owned artefacts, in order:** project rules first, then the style kit, then the sound library | 21 Oct | Q-RULES |
| **Chapters and short suggestions:** without an agent the app's own passes run; when an agent is connected, the agent writes them (it is usually smarter), accepting the higher cost | 21 Oct | Q-AI-OWNER |
| **Verification keeps a human in the loop:** the agent shows its results (previews, findings, what it changed); the user reads, watches and adjusts. No autonomous fix-and-retry loops; never export without the user's go | 21 Oct | Q-VERIFY |
| **Animation:** GSAP first, since it is HyperFrames' default runtime, vendored locally so the no-network gate still holds; other runtimes (Lottie, three.js…) only when a template needs one | 21 Oct | Q-ANIM, [GFX-2](roadmap.md#gfx-2) |
| **Sound needs two libraries the user can extend:** a built-in SFX library of specific, licence-clean sounds, and an open-source, copyright-free music library, both open to the user's own files and imports. Scope and order still to discuss ([Q-SOUND](#q-sound)) | 21 Oct | Q-SOUND |
| **HyperFrames stays pinned** (0.8.162); recheck it at each dependency review | 21 Oct | Q-HYPERFRAMES |
| **Dependencies are kept current on purpose:** toolchains (Rust, Node, TypeScript) and key libraries are pinned, reviewed on a schedule, and updated only after a compatibility check, so we get speed and quality gains without surprise migrations | 21 Oct | [dependencies.md](dependencies.md), [DEP-1](roadmap.md#dep-1) |
| **One anchored-note type** (decided by Claude at George's request): comments, agent notes, plan beats, review marks, spoken editor directions and short ideas are all one record, `{anchor (word ids or a recording time range, mapped through clocks), kind, text, author, status, revision}`, with one timeline lane and one MCP tool. Features add a `kind`, not a new system | 21 Oct | Q-PLAN, [NOTE-1](roadmap.md#note-1) |
| **Fast or thorough cleanup review** (decided by Claude at George's request): in step mode the agent asks once per video and saves the answer; auto mode uses the saved answer, fast if there is none. Thorough = read the whole kept transcript for unflagged retakes and tangents, inspect the remaining gaps and joins, and report what was only read vs actually listened to. Revisit the default after EVAL-1: if fast mode misses cuts George cares about on the long-form baseline, thorough becomes the default for long-form | 21 Oct | Q-THOROUGH, [AGT-2](roadmap.md#agt-2) |
| **Sound order:** clip gain, fades and separate SFX/music tracks first ([SND-1](roadmap.md#snd-1)), then a small SFX starter set (~15–20 hand-picked, licence-clean sounds: whooshes, UI clicks and pops, two risers, two hits), then the music library. Both libraries take the user's own files | 21 Oct | Q-SOUND, [SND-3](roadmap.md#snd-3) |
| **Music must never need credit.** Only tracks whose licence allows monetised use with no attribution (e.g. CC0 / public domain or an equivalent no-attribution licence) enter the built-in library. The app enforces it: every track carries its licence, and a track that requires credit, forbids monetisation or has an unknown licence is refused (user imports are flagged instead of silently accepted) | 21 Oct | Q-SOUND |
| **No automatic folder watch.** Nothing runs when a recording finishes: each video may be a tutorial or something else. Instead, after opening a project, the user can pick a template (tutorial, shorts…) and have its saved method run, on request ([AUTO-1](roadmap.md#auto-1)) | 21 Oct | Q-WATCH |

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
| Colour grading and LUTs | Not now (21 Oct) | Not needed for the channel today ([FX-5](roadmap.md#fx-5) parked) |
| Automatic folder watch (start editing when a recording lands) | No (21 Oct) | Each video may be a different kind; the user starts a template after opening instead |
| Importing an outside transcript | Only if it stays trivially small (21 Oct) | Not needed; our own transcription covers it ([FTG-5](roadmap.md#ftg-5)) |

## Waiting for George

Nothing is waiting. Add new questions here with a recommendation.

### Answered 21 Oct

<a id="q-focus"></a>**Q-FOCUS** long-form tutorial → tutorial shorts → gaming shorts and videos ·
<a id="q-rules"></a>**Q-RULES** project rules → style kit → sound library ·
<a id="q-anim"></a>**Q-ANIM** GSAP first, vendored locally ·
<a id="q-ai-owner"></a>**Q-AI-OWNER** the agent writes chapters and short picks when connected ·
<a id="q-verify"></a>**Q-VERIFY** human in the loop, no autonomous loops ·
<a id="q-scope"></a>**Q-SCOPE** colour not now; outside transcripts only if trivial ·
<a id="q-hyperframes"></a>**Q-HYPERFRAMES** pinned, rechecked at dependency reviews ·
<a id="q-plan"></a>**Q-PLAN** one anchored-note type (Claude's call) ·
<a id="q-thorough"></a>**Q-THOROUGH** ask once per video, fast by default until EVAL-1 says otherwise (Claude's call) ·
<a id="q-sound"></a>**Q-SOUND** knobs → SFX starter set → music; music never needs credit ·
<a id="q-watch"></a>**Q-WATCH** no automatic folder watch; run a template on request after opening.
All recorded in [Settled](#settled).

### Old question numbers

Q1, Q11, Q14, Q15 → Q-RULES · Q2, Q5 → Q-PLAN · Q3, Q16 → Q-SOUND · Q4 → Q-ANIM · Q6 → Q-FOCUS ·
Q7 → Q-AI-OWNER · Q8 → adopted as EVAL-1 · Q9 → done 21 Oct · Q10 → Q-THOROUGH · Q12 → Q-VERIFY ·
Q13 → Q-SCOPE and Q-WATCH · Q17 → Q-HYPERFRAMES.
