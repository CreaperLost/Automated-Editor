---
name: aeroedits
description: Edit videos in AeroEdits (screen-recorded tutorials first) with its own editing passes, through the aeroedits MCP tools or the `aeroedits do` command. Use when the user asks to edit, clean up, cut, tighten, add zooms or chapters to, caption, or export a recording or project in AeroEdits.
---
<!-- Installed by AeroEdits, which keeps this folder up to date. Remove this line to keep your own changes. -->

# Editing in AeroEdits

AeroEdits runs the editing passes itself: transcription, mouth noises, silence, fillers and
retakes, webcam focus, zooms, captions, chapters, export. You run them in the right order, ask
the user what they want, and report back. A pass costs you a line of text. Reading a transcript
or rendering frames costs far more, so do that only when a step fails or the user asks.

The actions are MCP tools (`aeroedits` server) and shell commands:
`aeroedits do <action> --option value` (`aeroedits do <action> --help` lists the options).

## Start with a template

A template is the plan for one kind of video: the steps, what to ask, and the defaults.
`template` lists them; `template` with a name returns that template and the user's saved
answers. Pick the template that fits what the user said (a tutorial, a screen recording, a
course lesson: `tutorial`; shorts cut from the video: `shorts`; a short the user recorded a
new take for, with footage from the video: `take-shorts`). If none fits, say so and ask which
is closest. Then follow its
steps in order. The user's own instructions win over the template.

## Saved answers

When `template` shows saved answers, ask once: "Same settings as last time?" and list them in a
line or two. On yes, run the template automatically with them (below). After the user answers
the questions, save the answers with `template_save` so next time is one question.

## One pass at a time

Run each editing pass on its own: first its dry run (`apply: false`), then the user's OK with
those numbers, then the pass, then one line about what it did. There is no one-call cleanup.
Each pass changes what the next one finds, so don't ask about later passes with earlier
numbers.

## Automatically

When the user says to do it automatically ("auto", "no questions", "just do it", or yes to
"same as last time?"), make each choice yourself: the saved answers, else the template's
defaults. Still run each dry run first and report one line per pass. Still stop and ask when a
dry run looks wrong (the template says when). Still never export without the user's go. The
user can switch either way at any point.

## Asking

- Ask all of a step's questions in one message, numbered. For each, give the choices with the
  default first and the numbers that matter from `scan` (for example "Silence: 112 pauses,
  1:42 in all"). If your client has a question tool (AskUserQuestion), use it, up to 4
  questions per call; otherwise plain text.
- Skip questions that don't apply: no camera means no webcam questions, one microphone means no
  microphone question, no AI key means no AI questions.
- Change nothing the user hasn't agreed to, apart from steps the template marks as automatic
  and passes run automatically at the user's word.

## Reporting

After each action, one line: what it did and the length before → after. The actions answer in
a line or two; pass that on without padding it.

## When something fails

Stop. Tell the user the app's error and what you suggest. Don't work around it with other
tools or edits unless the user says so.

## Undo

Each change is its own undo step and is listed in the app's Agent panel, where the user can
undo it. `undo` takes back the latest change.

## Chapters

Chapters are the last edit, right before export, after every cut: they read the final edit's
words. Never run `chapters` earlier; if the user asks for them sooner, say why they come last.
If anything is cut after them, run `chapters` again before exporting.

## Graphics

Motion graphics are HyperFrames compositions the app renders and plays full screen over
everything (screen, camera, captions); in a 9:16 short the same graphic renders at 9:16. Cuts
never go through a graphic, so add them after the cuts are done and checked, before chapters.
Look first at what exists: `graphics` lists the bundled templates and the user's library, each
library item with a `useWhen` line; pick one whose `useWhen` fits and fill in its variables.
Write a composition of your own (`graphic_add` with `blank`, then `graphic_write_file` or the
folder directly) only when nothing there does the job. Ask before `graphics_install`. After
adding, `graphic_render`, then look with `frame_render`.

When the user likes a graphic you wrote, offer to keep it for later projects. If they say yes:
`graphic_templatize` (its source and the rules: text as variables, brand kit colours, timing
from `data-duration`, a portrait block, a fading card, no network), rewrite the files, then
`graphic_check` until it passes (fix what it names; it is the app's own check, no model). Then
`library_save` with a `label`, a one-line `useWhen` and `lengthUs`. If it answers with a close
library item or template, ask whether to update that item (`update`, its next version) or keep
both (`asNew`). Never save without the user's yes, and save a failing graphic as is (`asIs`)
only at their word.

Brand kits (`brand_kits`) are the user's colours, font and logo. The project has a default kit
(`brand_pick`) and a graphic may pick another (`graphic_set` `brand`). A kit fills the
`background`, `text` and `accent` variables a graphic has not set, so with a kit leave those
unset; a composition of your own can use `var(--brand-accent)`, `var(--brand-background)`,
`var(--brand-text)`, `var(--brand-font)` and `var(--brand-logo)` (a CSS `url()`). The user's
library (in `graphics`) holds graphics saved from other projects: `graphic_add` with `library`
(its id or name) copies one in. Items marked "as is" were kept although they fail the check:
look at one with `frame_render` before relying on it.

In a short, a graphic plays over the whole frame unless it is given a slot there
(`graphic_set` with `short` and `slot`): `camera` or `screen` (that part of the split), or
`overScreen` (over the screen part, under the captions; its see-through parts show the
screen). It renders for that space, so the templates fit any slot.

## Telemetry polish

Three touches come from the recording's telemetry, on by default and subtle; leave them on
unless the user asks. In `settings` `area: layout`: `webcamAutoHide` fades the bubble out while
clicks, drags, scrolling, a resting pointer or typing are under it (it never moves); `windowCrop`
crops the screen to the front window when it is clearly smaller than the display (needs a
recording with window telemetry), `windowCropPaddingPct` is the room around it, and
`windowCropOff` lists recording stretches (`{media, sourceStartUs, sourceEndUs}`) shown whole,
for a part the user wants uncropped. In `area: audio`, `soundEffects` (`clicks`, `whooshes`,
`muted`, `volumeDb`) is a quiet lane of click sounds and a few whooshes on zooms and layout
switches; when the user wants silence or no clicks, switch it there.

## Looking at the edit

After your passes and before export, call `storyboard` once (with `short` for a short): one
small image of the whole edit, a tile per scene, with a JSON line per tile (time, what the
frame shows, zoom, graphic, chapter, words). Look for a wrong layout, a zoom on nothing, a
graphic over the wrong moment, a stretch that should have been cut; refer to tiles by number.
It replaces grabbing many frames: use `frame_render` only for a moment the storyboard leaves
in doubt, and `fromUs`/`toUs` for a closer look at one stretch.

## Export

Never export until the user says go. Follow the export steps (`export.md` beside this file,
and at the end of every template).

## Beyond the passes

When the user asks for cuts the passes don't make (a tangent, a wrong take the rules missed),
read `transcript_get`, then `propose_cuts` by word id with a reason. The user accepts it in the
Agent panel.
