# Preset: Clips

Vertical clips (9:16) cut from the edited video, the app's Clips tab: you read the transcript
and pick the moments yourself (no AI key needed), the app builds each clip with the look of the
others (a split-screen base layout by default) and picks its layout sections (camera only for
the opening line and while the speaker talks without the mouse, screen only while the clicking
is busy, the base otherwise). Run it after the video's own edit, so the clips come from the
cleaned-up talk. Clips are not shorts: a short is made from a new take the user recorded
(`preset` `shorts`, the Shorts tab); never touch those here.

Step by step by default; automatic when the user says so ("auto", "no questions"): then use
the defaults below, show the picks in one message and go on. Export still waits for the user's
go.

`clips`, `faces`, `captions` and `layouts` are behind `more`: `more` with `tool` alone gives
one's arguments, with `args` it runs it (`more` `tool: clips` `args: {"op": "create", ...}`).

## 1. Project (automatic)

- `status`. Clips need a transcript: if nothing is transcribed, `transcribe` (online, the
  language detected; as in the Tutorial preset).
- `status` says how many clips are already made. If there are some, `clips` (`op: list`, the
  default) lists them; ask whether to keep them (default) or start over (`clips` `op: remove`
  with `clip`, one per clip). Shorts made from a take are never removed here.
- With a camera, `faces` (it waits until the face is found in every camera; clips centre
  each camera clip on it). Pass on a camera with little or no face found.

## 2. Read (automatic)

`transcript_get` for the speech. It is the edited video's words with their ids and times.

## 3. Pick (ask)

Ask how many (default 3, at most 5). Pick moments that:
- make sense on their own, with no "as I said", "like before", "this one" pointing elsewhere;
- open with a hook in the first sentence: a question, a bold claim, a result, "here's how to";
- run 20 to 60 seconds and start and end on whole sentences, ending on the payoff;
- show something on screen while they talk, when there is a choice;
- don't overlap each other or the clips already made.

Show them as a numbered list: time in the video, length, a catchy title (under 60 characters,
it is also the file name) and one line on why it works. Ask for an OK or changes (automatic: go).

## 4. Make (automatic)

For each pick, `clips` with `op: create`, `title`, `fromWord`, `toWord` and `reason`. It copies
the look of the clips already made (else the project's clips preset) and picks its layout
sections. Report one line per clip with its sections (`layouts` with `short`: the clip's id).
To fix a pick: `op: range` with `clip`, `fromWord`, `toWord`; `op: rename` with `clip` and
`title`. The other tools take a clip's id as `short`. To change a clip's sections: `layouts` with `short` and `op: add`
(`sections` [{fromS, toS, layout}], seconds on the clip's own timeline; layouts like "Camera
only", "Screen only", "Side by side"), `update`, `remove` or `clear`; `op: base` changes the
split itself (e.g. {"kind": "sideBySide", "cameraFirst": false, "cameraPct": 40}). Graphics in a clip's stretch play over its whole frame; a title or
step card often reads better in the screen's part (`graphic_set` with `short` and `slot:
screen`), or `overScreen` for one meant to sit on the screen picture. Then look at each clip
once with `storyboard` and `short` (its id): a tile per scene of its 9:16 frame. Fix a section
that shows the wrong part. If the clip shows captions, shape them (`more` `tool: captions` with
`short`: the clip's id, which lists its lines): `emphasis`, per line the one word that makes a
viewer keep watching, never a small word, none on a flat line; `split` a line that reads as
two thoughts, `join` a line too short to read alone, `hide` a stray word. These stay the
clip's own: the video's captions don't change. Then report.

## 5. Export (ask)

Ask "Export the clips now?" (all, or which). On the go: `export` with `short: "clips"` (every
clip; never "all", which also exports the shorts made from a take) or a clip's id. They are
written next to the project as "<project> - <title>.mp4", 1080x1920. Answer with the paths.
