# Glossary

The project's own words, as used in the docs and the code.

| Term | Meaning |
|---|---|
| `.aero` recording | AeroShoot's folder: screen, webcam, mics, system audio, and `telemetry/` (mouse events, geometry, keys, windows) |
| Source time / edited time | Time in a recording vs time on the edited timeline. Clocks (`sequence/clock.rs`) map between them through cuts |
| Revision | A number that goes up on every edit. Agent edits pass `expectedRevision`; a mismatch is refused ("Stale edit revision") |
| Word ID | Stable `w-N` id of a transcript word. Survives cuts and undo, since transcripts are stored in source time |
| Pass | One built-in editing step: mouth noises, silence, gaps, fillers, layouts, zooms… Each has a dry run (`apply: false`) |
| Action | An app button exposed to agents (MCP tool + `aeroedits do` command). Applies directly, one undo step |
| Proposal / batch | An agent's own judgement call (`propose_cuts`, `propose_edit`) waiting in the Agent panel; accepting is one undo step |
| Autopilot | Setting that accepts proposal batches automatically. Off by default |
| Agent panel | Dock panel listing batches, history and running calls. **Not a chat** |
| Skill | `SKILL.md` and the rules shipped in the app, installed for Claude Code / Codex, also served as MCP prompts with a preset's method |
| Preset | What the app and the agent do for one kind of video: settings, short defaults, choices, the method (`method.md`) and the user's rules (`rules.md`). Built in: Tutorial, Shorts, Clips. A project follows one and stores only its changes |
| Style | How a video looks: brand (colours, fonts, logo), graphics look, caption look, frame, 9:16 variant. Built in: Light, Dark. A project wears one |
| Choices | A preset's saved answers to its method's questions, offered as "Same as last time?" |
| Changed here | A setting a project changed from its preset (a chip with a reset) |
| `scan` | Overview of a recording plus dry-run estimates of every pass; changes nothing |
| `check` | No-model audit of the finished edit (~12 defect classes) with safe one-click fixes |
| `storyboard` | One contact sheet of the edit, a tile per scene, plus a JSON line per tile |
| Layout / section / base | A look (bubble, side by side, camera only, screen only). Sections change it for a stretch, anchored in recording time; the base fills everything else |
| Telemetry polish | Zooms, window crop, bubble auto-hide, shortcut badges, click/whoosh lane — all from recorded telemetry |
| Graphic | A HyperFrames composition in `graphics/<id>/` (`index.html`, `graphic.json`), rendered by the app to a cached WebM |
| Slot | Where a graphic plays, which sets its render size: whole canvas, or a short's camera / screen / over-screen part |
| `data-entrance` | `cut` or `ease`, set by the app on render: full-screen graphics cut in and out, overlays on camera may ease |
| Brand | A style's colours, fonts and logo; fills a graphic's `background` / `text` / `accent` |
| Graphics library | App-level saved graphics with `useWhen`, versions, and the gate's result (Template vs As is) |
| Gate | `graphic_check`: lint, layout overflow, contrast, long text, 16:9 and 9:16, first/last frames. No model |
| Take short | A short built from a new camera take, with footage from the long video under it |
| Thorough review | The tutorial template's slower review: reads every kept word for unflagged retakes and tangents and checks the joins, instead of the rule windows only ([AGT-2](roadmap.md#agt-2)) |
| **Proposed terms (not built)** | |
| Style kit | Rules for using the brand: caption style, motion, density, banned list, references ([STY-1](roadmap.md#sty-1)) |
| Beat plan | Per-moment intent anchored to words: purpose, treatment, reason, locks, status ([PLAN-1](roadmap.md#plan-1)) |
| Style trial | Building a few beats on a short range and approving the look before scaling ([PLAN-1](roadmap.md#plan-1)) |
| Event marker | A moment a graphic declares (card lands, counter ends) for sound to sync to ([SND-2](roadmap.md#snd-2)) |
| Cue compiler | App logic turning chosen events + sounds into timed, ducked audio cues ([SND-2](roadmap.md#snd-2)) |
| Motion preview | Short low-res full-rate clip with sound, so agent and user can watch, not sample ([REV-1](roadmap.md#rev-1)) |
| Checkpoint | Named restorable snapshot of edit + graphic source + caption marks ([SAFE-2](roadmap.md#safe-2)) |
| Selection reference | A serialised pointer to what the user selected, sent to the agent ("Ask AI about this") ([NOTE-1](roadmap.md#note-1)) |
| Review record | Per-revision evidence of which ranges were inspected, listened to, or decided by the user; made stale by later edits ([REV-1](roadmap.md#rev-1)) |
| Join audition queue | A list of cuts to play a few seconds around and mark keep or refine ([REV-1](roadmap.md#rev-1)) |
| Project rules | Editing rules the agent suggests and the user accepts, scoped to a project or to all videos; the "self-improving skill" ([AGT-3](roadmap.md#agt-3)) |
| Word-timed emphasis | A graphic that reveals each word of a phrase at its own word ID and re-times after cuts ([GFX-3](roadmap.md#gfx-3)) |
| Cost per accepted video | All spend (subscriptions, generation, render, human review, retries) divided by videos actually published ([product](product.md#who-it-is-for)) |
