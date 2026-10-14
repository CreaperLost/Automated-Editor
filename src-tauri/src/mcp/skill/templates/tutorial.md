# Template: tutorial

A screen-recorded tutorial, long form (16:9): clean speech, steady pacing, the webcam and zooms
where they help, no captions, chapters at the end. Run the steps in order, with one line to the
user after each action.

## 1. Project (automatic)

- Given a recording (a `.aero` folder): `create` with the project name and `recording`.
  Otherwise work on the open project.
- `status`. Tell the user the length and what the recording has.

## 2. Transcription (ask)

- `scan` lists the transcribers that are ready and the microphones.
- Ask which transcriber to use, offering only the ready ones:
  - Parakeet: runs on this computer, free. The default.
  - ElevenLabs Scribe: cloud, on the user's key. Better with names and jargon.
  If only one is ready, say which and go on without asking.
- Two or more microphones: ask which to transcribe: the presenter's (default), or all of
  them (needed to cut gaps between words across all of them).
- `transcribe` with `provider` (and `stream` for each microphone). It can take a few minutes,
  so tell the user.

## 3. Cleanup questions (ask, one round)

Run `scan` again (now that the transcript exists). It changes nothing. Then ask, in one
message, with its numbers:

1. Mouth noises: mute the clicks, pops and breaths found clear of words? Yes (default) / no.
   Sensitivity: gentle / balanced (default) / tight.
2. Silence:
   - From the audio (default: cuts every quiet stretch), from the transcript (gaps between
     words: keeps quiet speech, cuts breaths and noises too), or both.
   - How hard: gentle / balanced (default) / tight / aggressive. Give the count and seconds
     `scan` found for the preset it used; for another preset, `silence` with `apply: false`
     reports what it would cut.
   - Two or more microphones: listen to all of them (default; cuts only where every one is
     quiet) or the presenter's only.
3. Fillers and retakes: cut the fillers and retakes the app's rules found? Yes (default) / no.
   Also the AI review (only with an AI key; the user's key, a few cents)? No (default) / yes.
4. Webcam (only with a camera): `scan` says how many cameras there are.
   - Keep the camera bubble? Yes (default) / no (hides it for the whole video).
   - Webcam focus: the camera fills the frame where the speaker talks and the mouse rests
     (`scan` gives the stretches and their total). Yes (default) / no.
5. Zooms (only with mouse data): add the zooms found (`scan` gives the count, average length
   and how far in)? Yes (default) / no / fewer and longer / shorter. "Fewer and longer" raises
   `minHoldMs` and `mergeGapMs` (default 5000 each), "shorter" lowers them. How far in:
   `clickScale` (1.6 default).

Then save the answers with `template_save` (`name: tutorial`), in this shape:

```json
{
  "transcriber": "parakeet",
  "mouthNoises": { "apply": true, "preset": "balanced" },
  "silence": { "source": "audio", "preset": "balanced", "microphones": "all" },
  "fillers": { "apply": true, "ai": false },
  "webcam": { "keep": true, "focus": true },
  "zooms": { "apply": true, "settings": { "minHoldMs": 5000, "mergeGapMs": 5000, "clickScale": 1.6 } },
  "export": { "resolution": "1080p", "quality": "high" }
}
```

## 4. Apply (automatic)

In this order, one line to the user after each:

1. `mouth_noises` with `sensitivity` (gentle 0.35, balanced 0.5, tight 0.6). It runs first,
   because muted noises then count as silence.
2. `silence`: `mode: silence` for the audio, `mode: gaps` for the transcript, or both (audio
   first), with `preset`. Microphones "all": `stream: speech`; "presenter": that sound's key.
3. `fillers` (with `ai: true` if the user wanted the AI review).
4. Webcam: if not kept, `settings` `area: layout`, `set: {"webcamEnabled": false}`, and skip
   focus. Otherwise `webcam_focus`, or `webcam_focus` with `enabled: false` if the user said
   no to focus.
5. `zooms` with `settings` from the answers, or skip it.

If a result looks wrong (more than a third of the video cut, nothing found where `scan` found
plenty), stop and tell the user before going on.

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
