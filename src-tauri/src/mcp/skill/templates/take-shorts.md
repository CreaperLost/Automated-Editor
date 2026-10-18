# Template: shorts from a take

A vertical short (9:16) told again for shorts: the user recorded a new take, a camera video
(often with its own microphone) and maybe a separate microphone, retelling one part of the
long video (one myth, one tip, one step). The take is the short's camera, voice and captions;
you pick the footage from the long video that shows what each sentence talks about, so the
action comes from the video while the take does the talking. Then dress it with graphics.

Step by step by default; automatic when the user says so ("auto", "no questions"): then use
the defaults below, report in one message and go on. Export still waits for the user's go.

## 1. Project (automatic)

- `status`. The long video needs a transcript (that is how you find its footage): if it has
  none, ask which transcriber (as in the tutorial template; automatic: the saved one, else
  Parakeet) and `transcribe` it.
- Short ids and titles: `short_scenes` without `short` lists the shorts already made.

## 2. The take (ask only if unclear)

- If the user gave file paths, `media_import` them. Otherwise the take is already in Media:
  `project_info` lists the media; the take is the camera video and microphone file the user
  names, else the newest imported ones.
- `short_from_take` with a `title` (under 60 characters, also the file name), `camera`, and
  `voice` when there is a separate microphone (else the camera's own sound is the voice) and
  `idea` (one line). Pass on what it says about lining the camera up with the voice; if the
  match was weak, tell the user to check the lip sync.
- `transcribe` with the take's voice as `stream` (the line it answers says which).

## 3. Read (automatic)

- `transcript_get` with the take's voice as `stream`: the take's words with ids.
- `transcript_get` for the long video, and `chapters` if it has them, to find where it
  talks about the same thing.

## 4. Pick the footage (ask)

Split the take into beats: the hook, each claim or step, the verdict or payoff. For each beat
find the stretch of the long video that **shows** it:

- Search the video's transcript for the same names and things (a place, a mission, a menu, a
  setting). Where the video talked about it is usually where it was on screen.
- Look before you pick: `frame_render` at a few seconds of the candidate (or `storyboard`) to
  check the screen shows the thing, and favour action over menus, loading screens and still
  desktop.
- Each beat gets footage as long as its words; 3 to 8 s pieces read well, and a new piece every
  sentence or two keeps a short moving. Don't reuse the same seconds twice in one short.
- The hook may stay on the camera alone (the user's face sells the claim): leave it without
  footage unless the user wants otherwise.

Show the plan as a list: the take's words (a few), the video time picked and what it shows.
Ask for an OK or changes (automatic: go).

## 5. Place (automatic)

`short_footage` with `op: add` and all the items at once: `fromWord`/`toWord` from the take,
`sourceWord` (a word id from the video's transcript where the footage starts) or `media` and
`sourceS`. Add `sound: true` for game footage whose sound helps (explosions, engines), never
for footage where the video's own voice talks. Report the list it answers. Fix a piece with
`op: remove` (`fromS`/`toS`) and add again.

## 6. Dress (ask)

Graphics go on the take's words, through HyperFrames, as in the shorts template: a title card
on the hook, a label per beat ("MYTH #1", "TRUE"/"FALSE" stamp, a name), in the screen's part
(`graphic_set` with `short` and `slot: screen`) or over the whole frame. Then `storyboard`
with `short` to look at the frames, and fix what reads badly.

## 7. Export (ask)

Ask "Export the short now?". On the go: `export` with `short` (its id). It is written next to
the project as "<project> - <title>.mp4", 1080x1920. Answer with the path.
