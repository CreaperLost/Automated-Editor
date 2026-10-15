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
course lesson: `tutorial`). If none fits, say so and ask which is closest. Then follow its
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

## Export

Never export until the user says go. Follow the export steps (`export.md` beside this file,
and at the end of every template).

## Beyond the passes

When the user asks for cuts the passes don't make (a tangent, a wrong take the rules missed),
read `transcript_get`, then `propose_cuts` by word id with a reason. The user accepts it in the
Agent panel.
