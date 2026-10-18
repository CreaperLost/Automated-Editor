# Template: shorts

Vertical shorts (9:16) cut from the edited video: you read the transcript and pick the moments
yourself (no AI key needed), the app builds each short with the split-screen look and picks
its scenes (camera only for the opening line and while the speaker talks without the mouse,
screen only while the clicking is busy, split otherwise). Run it after the video's own edit, so
the shorts come from the cleaned-up talk.

Step by step by default; automatic when the user says so ("auto", "no questions"): then use
the defaults below, show the picks in one message and go on. Export still waits for the user's
go.

## 1. Project (automatic)

- `status`. Shorts need a transcript: if nothing is transcribed, ask which transcriber (as in
  the tutorial template; automatic: the saved one, else Parakeet) and `transcribe`.
- `short_scenes` without `short` lists the shorts already made. Ask whether to keep them
  (default) or start over (remove them in the Shorts Studio; there is no action for it).

## 2. Read (automatic)

`transcript_get` for the speech. It is the edited video's words with their ids and times.

## 3. Pick (ask)

Ask how many (default 3, at most 5). Pick moments that:
- make sense on their own, with no "as I said", "like before", "this one" pointing elsewhere;
- open with a hook in the first sentence: a question, a bold claim, a result, "here's how to";
- run 20 to 60 seconds and start and end on whole sentences, ending on the payoff;
- show something on screen while they talk, when there is a choice;
- don't overlap each other or the shorts already made.

Show them as a numbered list: time in the video, length, a catchy title (under 60 characters,
it is also the file name) and one line on why it works. Ask for an OK or changes (automatic: go).

## 4. Make (automatic)

For each pick, `short_create` with `title`, `fromWord`, `toWord` and `reason`. It copies the
look of the shorts already made and picks the scenes. Report one line per short with its
scenes. To change a short's scenes: `short_scenes` with `op: set` (seconds on the short's own
timeline) or `op: clear`. Graphics in a short's stretch play over its whole frame; a title or
step card often reads better in the screen's part (`graphic_set` with `short` and `slot:
screen`), or `overScreen` for one meant to sit on the screen picture. Then look at each short
once with `storyboard` and `short` (its id): a tile per scene of its 9:16 frame. Fix a scene
that shows the wrong part, then report.

## 5. Export (ask)

Ask "Export the shorts now?" (all, or which). On the go: `export` with `short: "all"` (or a
short's id). They are written next to the project as "<project> - <title>.mp4", 1080x1920.
Answer with the paths.
