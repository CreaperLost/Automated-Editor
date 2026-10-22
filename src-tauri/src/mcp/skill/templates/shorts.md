# Template: shorts from a take

A vertical short (9:16) told again for shorts: the user recorded a new take, a camera video
(often with its own microphone), maybe a separate microphone file, or an AeroEdits recording,
retelling one part of the long video (one myth, one tip, one step). The take is the short's
camera, voice and captions. It goes through the same process as any recording (transcribe,
clean), then you pick the footage from the long video that shows what each sentence talks
about, so the action comes from the video while the take does the talking. Then graphics.

Step by step by default; automatic when the user says so ("auto", "no questions"): then use
the defaults below, report in one message and go on. Export still waits for the user's go.

## 1. Project (automatic)

- `status`. The long video needs a transcript (that is how you find its footage): if it has
  none, `transcribe` it (online, the language detected; as in the tutorial template). The
  long video should be edited first: footage only comes from what its edit keeps.
- `status` says how many shorts are already made.

## 2. The take (ask only if unclear)

- If the user gave file paths, `media_import` them (all the take's files at once) into the
  project that is open. A .aero recording folder is a take too (its camera and microphone come
  in together): import it, never `open` it or `create` a project from it.
  Otherwise the take is already in Media: `project_info` lists the media; the take is what
  the user names, else the newest imported files.
- `short_from_take` with a `title` (under 60 characters, also the file name), `media` (the
  take's file ids: the app tells the camera from the microphone; a separate microphone wins
  over the camera's own) and `idea` (one line: what the user said the take is about). With no
  title given, make one from that. Pass on what it says about lining the camera
  up with the voice; if the match was weak, tell the user to check the lip sync.
- `faces`, so the camera is framed on the face (it waits until it is found).

## 3. Transcribe and clean the take (automatic)

The same passes as the long video's cleanup, on the take short's own timeline:

- `transcribe` with the take's voice as `stream`.
- `mouth_noises` with that `stream`.
- `silence` with `short` (the short's id), `mode: gaps` and `preset: tight`: the pauses
  between the take's words. A short is cut tighter than a long video: Balanced leaves a
  pause you hear at every cut.
- `fillers` with that `stream` (after `fillers_review` if the user wants a careful pass).

Report one line per pass. The user can review each in the Shorts tab (Transcript, Jump cuts,
Mouth noises) and undo.

## 4. Read (automatic)

- `transcript_get` with the take's voice as `stream`: the take's words with ids, at their
  times on the short.
- `transcript_get` for the long video, and `chapters` if it has them, to find where it
  talks about the same thing. Its times are the video's timeline (what `videoS` means).

## 5. Pick the footage (ask)

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
- Not only the video: the user may give outside clips (a trailer, a news clip, B-roll). They
  go into the project with `media_import`; place them with `media` (and `mediaS`) instead of
  `sourceWord`.

Show the plan as a list: the take's words (a few), the video time picked and what it shows.
Ask for an OK or changes (automatic: go).

## 6. Dress (ask)

Pick graphics for the take's words too. First the **long video's own graphics** (`graphics`
lists them with where they play): where a beat retells a part of the video that had a graphic,
reuse it (`graphic`, its id: a copy made for 9:16, its text changeable without touching the
video's). Then new ones (`template` or `library`): a title card on the hook, a label per beat
("MYTH #1", "TRUE"/"FALSE" stamp, a name), news or a new feature the video lacks. Add them to
the plan; ask for an OK (automatic: go).

## 7. Propose (automatic)

One `short_footage` call with `op: add`, a `reason`, and every item at once; the user accepts
the batch in the Agent panel (under autopilot it lands at once). Footage: `fromWord`/`toWord`
from the take, and `sourceWord` (a word id from the video's transcript where the footage
starts; it must be one the video keeps) or `videoS`; its game or desktop sound plays ducked
under the voice, `sound: false` where it distracts. Graphics: `fromWord` (and `toWord`, else
its own length) with `template`, `library` or `graphic`, plus `name` and `variables`. Report
the summary it answers and wait for the user's decision (`proposals_list`). Once accepted:
each graphic plays over the whole frame, or in the screen's part (`graphic_set` with `short`
and `slot: screen`); `storyboard` with `short` to look at the frames, and fix what reads badly
(`op: remove` with `fromS`/`toS` takes footage off).

## 8. Export (ask)

Ask "Export the short now?". On the go: `export` with `short` (its id). It is written next to
the project as "<project> - <title>.mp4", 1080x1920. Answer with the path.
