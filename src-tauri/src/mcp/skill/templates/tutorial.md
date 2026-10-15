# Template: tutorial

A screen-recorded tutorial, long form (16:9): clean speech, steady pacing, the webcam and zooms
where they help, no captions, chapters at the end. Run the steps in order, with one line to the
user after each action.

Two ways to run it:

- **Step by step (the default).** Before each cleanup pass, show its dry run and get the
  user's OK.
- **Automatic**, when the user says so ("do it automatically", "auto", "no questions",
  "same as last time"), at the start or at any point. You then make each choice yourself: the
  saved answers if there are any, otherwise the defaults below. You still run each pass's dry
  run first, report one line per pass, and stop to ask when a result looks wrong (see step 4).
  Export still waits for the user's go.

## 1. Project (automatic)

- Given a recording (a `.aero` folder): `create` with the project name and `recording`.
  Otherwise work on the open project.
- `status`. Tell the user the length and what the recording has.

## 2. Transcription (ask)

- `scan` gives an overview: the transcribers that are ready, the microphones, cameras and
  mouse data. Tell the user what the recording has in a line or two.
- Ask which transcriber to use, offering only the ready ones (automatic: the saved one, else
  Parakeet):
  - Parakeet: runs on this computer, free. The default.
  - ElevenLabs Scribe: cloud, on the user's key. Better with names and jargon.
  If only one is ready, say which and go on without asking.
- Two or more microphones: ask which to transcribe: the presenter's (default), or all of
  them (needed to cut gaps between words across all of them).
- `transcribe` with `provider` (and `stream` for each microphone). It can take a few minutes,
  so tell the user.

## 3. Cleanup, one pass at a time

The passes below, in this order: each changes what the next finds (muted noises count as
silence; cut silence changes the gaps), so each is measured right before it runs. For every
pass:

1. Dry run: the action with `apply: false` and the default (or saved) settings.
2. Ask one question with its numbers and choices, default first, for example: "Silence
   (balanced): 112 pauses, 1:42 in all, 12:04 → 10:22. Cut? yes / gentler / tighter / skip".
   Always offer "do the rest automatically". If the user picks other settings, dry-run those
   and ask again in one line, unless they said to just do it.
3. Run the pass with the chosen settings, then one line: what it did, length before → after.

Automatic: steps 1 and 3 only, with the saved answers or the defaults.

The passes:

1. **Mouth noises:** `mouth_noises` with `sensitivity` (gentle 0.35, balanced 0.5 default,
   tight 0.6). Mutes the clicks, pops and breaths clear of words; the length stays. Choices:
   yes / gentle / tight / skip.
2. **Silence from the audio:** `silence` with `preset` (gentle / balanced default / tight /
   aggressive). Two or more microphones: ask once whether to listen to all of them (default,
   `stream: speech`: cuts only where every one is quiet) or the presenter's only (that
   sound's key). Choices: yes / gentler / tighter / skip.
3. **Gaps between words:** `silence` with `mode: gaps` and `preset`: cuts the breaths and
   noises between transcribed words that the audio pass left. Choices: no (default) / yes /
   gentler / tighter.
4. **Fillers and retakes:** `fillers`. With an AI key the dry run includes the app's AI review
   (the user's key, a few cents), which catches far more than the rules; give how many came
   from it. Choices: yes / rules only / skip. Then run it with `ai: false`: the review's
   finds from the dry run are kept, so it isn't paid for twice. Without a key, say only the
   rules can run.
5. **Webcam** (only with a camera; `scan` says how many): first, keep the camera bubble? Yes
   (default) / no (`settings` `area: layout`, `set: {"webcamEnabled": false}`, and skip focus).
   Then **webcam focus**: `webcam_focus` (the camera fills the frame where the speaker talks
   and the mouse rests: stretches and their total). Choices: yes / skip (`webcam_focus` with
   `enabled: false`).
6. **Zooms** (only with mouse data): `zooms` (count, average length, how far in). Choices:
   yes / fewer and longer / shorter / skip. "Fewer and longer" raises `minHoldMs` and
   `mergeGapMs` (default 5000 each), "shorter" lowers them; how far in is `clickScale`
   (1.6 default). Pass them as `settings`.

After the passes, save the choices with `template_save` (`name: tutorial`), in this shape:

```json
{
  "transcriber": "parakeet",
  "mouthNoises": { "apply": true, "preset": "balanced" },
  "silence": { "apply": true, "preset": "balanced", "microphones": "all" },
  "gaps": { "apply": false, "preset": "balanced" },
  "fillers": { "apply": true, "ai": true },
  "webcam": { "keep": true, "focus": true },
  "zooms": { "apply": true, "settings": { "minHoldMs": 5000, "mergeGapMs": 5000, "clickScale": 1.6 } },
  "export": { "resolution": "1080p", "quality": "high" }
}
```

## 4. When a result looks wrong (also in automatic)

Stop and ask before running a pass when its dry run:
- would cut more than a quarter of the video, or the passes together more than a third;
- finds nothing where `scan` found plenty, or far more than `scan` estimated;
- reports an error or a warning.

## 5. Captions (automatic)

Tutorials have no captions: if `status` shows them on, `settings` `area: captions`,
`set: {"enabled": false}`. Turn them on only if the user says this is a short or vertical video.

## 6. Chapters (automatic, last edit)

Chapters come last, once everything else is cut, since they follow the final edit. Run
`chapters`. If `scan` said there is no AI key, skip this and say so.

## 7. Status and summary (automatic)

`status`, then sum up the edit in a few lines:
- What each step cut or changed, with seconds where it cut.
- Length from → to.
- What to check by hand:
  - the first and last ten seconds (cuts at the edges),
  - each retake cut (that the kept take is the right one),
  - places with many cuts close together (they can feel jumpy),
  - the webcam focus stretches and zooms on the timeline,
  - mouth noises left alone next to words.
- Any step can be undone from the Agent panel's history.

## 8. Export (ask)

Ask before exporting, then follow the export steps below.
