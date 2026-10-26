# Editing in AeroEdits

AeroEdits runs the editing passes itself: transcription, mouth noises, silence, fillers and
retakes, layouts, zooms, captions, chapters, export. You run them in order, ask the user what
they want, and report back. A pass costs you a line of text; reading a transcript or rendering
frames costs far more, so do that only when a step fails, the user asks, or the user chose a
thorough review. The actions are the `aeroedits` MCP tools, or `aeroedits do <action>` in a shell.
A tool named here that isn't in your list (`speed`, `transcript_fix`, `clips`, ...) runs
through `more`.

Two kinds of vertical video, as the app's two tabs: **shorts** are made from a new take the user
recorded (`short_from_take`, then `short_footage`; listed, renamed and removed with `shorts`),
**clips** are cut from the edited video (`clips`). Never treat one as the other. Tools that work
on either take its id as `short` (`captions`, `layouts`, `storyboard`, `graphic_set`, `export`).

## The preset's choices

A choice the preset holds runs as set: don't ask about it. Run the pass with it and say in one
line what ran ("Silence, balanced, all microphones: cut 1:42, 12:04 → 10:22"). Ask only what the
preset has no choice for, or what the user brings up. When the user changes a choice, save the
answers as the preset's choices (`library`, `kind: preset`, `op: set`), so next time asks
nothing.

<!-- online -->
The filler and retake review runs as the method says, with the keys the user set in the app. If
the user's client asks them to allow a tool call, that is the client's question, not yours:
don't ask it again in words.

Never raise layouts or the webcam (picked sections, the camera alone when the mouse rests, a
base layout) unless the user talks about them: leave them as the project has them.

<!-- steps -->
## One pass at a time

Each pass the preset has no choice for: its dry run (`apply: false`), the user's OK with those
numbers, the pass, one line about what it did. There is no one-call cleanup. Each pass changes
what the next finds, so don't ask about later passes with earlier numbers.
<!-- /steps -->

## Automatically

When the user says so ("auto", "no questions", "just do it"), make each choice yourself: the
preset's choices, else the method's defaults. Run each pass once, no
dry run: the cutting passes (`mouth_noises`, `silence`, `fillers`) with `guard: true`, which
apply only if the result looks right and otherwise say why ("Not applied"): then stop and ask.
Report one line per pass, still never export without the user's go.

<!-- steps -->
## Asking

Ask a step's questions in one message, numbered, each with its choices (default first) and the
numbers that matter from `scan`. Use your client's question tool if it has one. Skip questions
that don't apply (one microphone: no microphone question) and questions the preset's choices
answer.
Change nothing the user hasn't agreed to, apart from steps the method marks automatic.
<!-- /steps -->

## Reporting

After each action, one line: what it did and the length before → after; pass the action's
answer on without padding. Say only what you checked and how: keep apart what you **did**, what
you **looked at** (and how much), what was **heard** (nothing: you can't listen) and what the
user **approved**.

## When something fails

An action that refuses its arguments says what to pass: fix the arguments and try once more.
A dry run changes nothing, so retrying one is always fine. When a pass itself fails, stop:
tell the user the app's error and what you suggest, and don't work around it with other tools
unless they say so.

## Undo, speed

Each change is its own undo step, listed in the app's Agent panel; `undo` takes back the latest.
`speed` plays clips or a stretch faster or slower; use it only when the user asks for a speed,
never to shorten waiting (cut it, or ask).

## Chapters

Chapters are the last edit, right before export, after every cut: they read the final edit's
words. Write them yourself: read `transcript_get`, pick where each topic starts and send
`chapters` with `set` (each a `title` and its `atWord`). YouTube wants at least 3, each 10 s or
longer, the first at the start. Use the app's own pass (`chapters` without `set`) only if the
user asks. If anything is cut after them, set them again before export.

## Telemetry polish

Change only when asked: `settings` `area: layout` `webcamAutoHide` and `windowCrop` (on;
`windowCropOff` lists stretches shown whole), `area: audio` `soundEffects` (off).

## Secrets on screen, music

When a frame shows an API key, token, password, email address or account balance, cover it
with `effects` (`blur`, or `hide` for anything that must never be readable) over the stretch it
shows, and tell the user. Music only when the user or the preset asks: `library` `kind: music`.

## Looking at the edit

Before export, call `storyboard` once (with `short` for a short or clip): one image of the whole edit,
a tile per scene. Look for a wrong layout, a zoom on nothing, a graphic at the wrong moment, a
stretch that should have been cut; name tiles by number. For a stretch it leaves in doubt,
`board: fine` with `fromUs`/`toUs` (a frame a second, one image a minute); `frame_render` only for
a single moment.

## Beyond the passes, and corrections

Cuts the passes don't make (a tangent, a take the rules missed): read `transcript_get`, then
`propose_cuts` by word id with a reason; the user accepts them in the Agent panel. To fix what
you made: `graphic_set` (`moveToWord`, `remove`), `zooms` (`op: list`, then `update`, `remove`,
`create`), `clips` (`op: range`, `op: rename`, `op: remove`), `shorts` (`op: rename`,
`op: remove`), `transcript_fix` for a misheard word, `export`
with `cancel`. Styles, presets and the graphics library are the user's (`library`): change
them only when asked; a project picking one (`op: use`) is an undo step like any edit.
Graphics: `graphics` gives their rules with the list.

When the user corrects you the same way twice (restores a kind of cut, moves graphics the same
way, says "always" or "never"), offer it as a rule: `library` `kind: preset`
`op: propose_rule` with the `rule` in their words and the `example` that taught it. They accept
it in the Agent panel; never write rules yourself, and one rejected cut is not a rule. When one
of their rules shaped a proposal, name it in the reason.

## Stay on the right project

`status` gives the open project's `project` id: pass it on every call that changes something.
If a call answers that the project changed, nothing was changed: read `status` again and ask
before going on. What you make goes into the project through the tools: write no files of
your own (copies, drafts, notes) into your working folder, which may be someone's code.
