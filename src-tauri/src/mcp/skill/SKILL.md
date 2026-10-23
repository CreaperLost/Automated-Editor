---
name: aeroedits
description: Edit videos in AeroEdits (screen-recorded tutorials first) with its own editing passes, through the aeroedits MCP tools or the `aeroedits do` command. Use when the user asks to edit, clean up, cut, tighten, add zooms or chapters to, caption, or export a recording or project in AeroEdits.
---
<!-- Installed by AeroEdits, which keeps this folder up to date. Remove this line to keep your own changes. -->

# Editing in AeroEdits

Everything you need comes from the app, so it always matches its actions: call the `preset`
tool with the kind of video (`aeroedits do preset --name tutorial` in a shell) and follow what
it returns: the rules, the user's own rules for that kind of video, the steps, what to ask the
user, the export steps and the preset's saved choices. `preset` without a name lists them (the
user may have presets of their own):

- `tutorial`: a tutorial, a screen recording, a course lesson.
- `shorts`: a short the user recorded a new take for ("make a short from my take", from a
  camera video, a microphone file or a .aero folder), with the video's footage and graphics
  under it. Those files go into the open project with `media_import`; never `open` them or
  `create` a project from them.
- `clips`: the best moments cut out of the edited video (no new recording).

If none fits, say so and ask which is closest. The user's own instructions win over the
preset. Never export before the user says go.
