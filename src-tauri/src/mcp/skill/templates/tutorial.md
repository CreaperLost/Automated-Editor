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
- The language: English (default) or Greek, in `settings` `area: transcription`
  (`set: {"language": "el"}` or `"en"`). Ask only when the user has not said. Greek is
  transcribed online only, with ElevenLabs Scribe or Scribe via OpenRouter: Parakeet is
  English only, and so is word alignment.
- Ask which transcriber to use, offering only the ready ones (automatic: the saved one, else
  Parakeet):
  - Parakeet: runs on this computer, free. The default. Its model (`settings` `area:
    transcription`, `parakeetModel`) is `unified` (the most accurate words, the default) or
    `verbatim` (keeps every um, stutter and repeat for the filler and retake passes).
  - ElevenLabs Scribe: cloud, on the user's key. Better with names and jargon.
  If only one is ready, say which and go on without asking.
- Two or more microphones: ask which to transcribe: the presenter's (default), or all of
  them (needed to cut gaps between words across all of them).
- `transcribe` with `provider` (and `stream` for each microphone). It can take a few minutes,
  so tell the user. When the word alignment model is downloaded, transcribing also aligns
  the words to the audio (its result says how many). A transcript made earlier and not aligned
  yet: run `align` on it before the cleanup passes, since they cut at word edges.

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
   tight 0.6), after transcribing (the transcript's words are never muted). Mutes the clicks,
   pops and breaths clear of words; the length stays. Noises close together count as one
   place; give the places and how many are left next to words. The user reviews and unmutes
   them in the app's Mouth noises panel. Choices: yes / gentle / tight / skip.
2. **Silence from the audio:** `silence` with `preset` (gentle / balanced default / tight /
   aggressive). Two or more microphones: ask once whether to listen to all of them (default,
   `stream: speech`: cuts only where every one is quiet) or the presenter's only (that
   sound's key). Choices: yes / gentler / tighter / skip.
3. **Gaps between words:** `silence` with `mode: gaps` and `preset`: cuts the breaths and
   noises between transcribed words that the audio pass left. Choices: no (default) / yes /
   gentler / tighter.
4. **Fillers and retakes (rules and AI):** always both, in English or Greek: the rules and the
   review follow the language the track was transcribed in. `status` says whether an AI key is
   set.
   - **The default, `review: auto`:** `fillers` runs the rules with the OpenRouter review when
     a key is set, else with the app's built-in local model when it is downloaded (free,
     nothing leaves the computer, but it makes more wrong cuts). Dry-run it (`apply: false`):
     give how many came from the review and a few examples.
   - **Neither, or the review could not run** (`fillers` says so): you are the AI review.
     By default `fillers_review` shows only windows of about 15 words either side of what the
     app's local pass flagged (a "like" or "so" that may be filler, a phrase said again); um
     and uh are cut by the rules already. Read every page (`page: 0`, 1, ...), mark the
     fillers and retakes by the criteria it gives (a flag that carries meaning stays), then
     save them in one `fillers_review` call with `cuts` and the pages' `stamp`. Read every
     word with `review: "thorough"` only when the user asks for a thorough pass or the
     windows miss things they point out. Then dry-run `fillers` the same way.

   Choices: yes / rules only (`review: in-house` without saving a review) / skip. Then
   `fillers`. When the user wants nothing sent to OpenRouter, use `review: local` (or
   `review: in-house` when the local model is not downloaded).
5. **Layouts** (only with a camera; `scan` says how many): first, the base layout: keep the
   camera bubble (default) / side by side / no camera (`layouts` `op: base` with `layout`
   "Side by side" or "Screen only"). Then **picked sections**: `layouts` `op: pick` with
   `apply: false` first (the camera alone where the speaker talks and the mouse rests:
   sections and their total). Choices: yes (`op: pick`) / skip.
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
  "fillers": { "apply": true, "review": "auto" },
  "webcam": { "keep": true, "focus": true },
  "zooms": { "apply": true, "settings": { "minHoldMs": 5000, "mergeGapMs": 5000, "clickScale": 1.6 } },
  "graphics": { "apply": false, "brand": "the project's" },
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

## 6. Graphics (ask)

Graphics come after the cuts, on the final timeline: they play full screen over everything and
no cut goes through one. A full-screen graphic cuts in and out, never eases; only one laid over
the full-screen camera eases (the app decides, see Graphics in the skill). Ask whether the user wants graphics (default no in automatic mode
unless the saved answers say yes). If yes:

1. `graphics`: the templates, the user's library and their variables. If the renderer is not
   installed, ask before `graphics_install` (a 200 MB download). `brand_kits`: if the user has
   kits and the project uses none, ask which (`brand_pick`); the kit then gives every graphic
   its colours and font, so leave `background`, `text` and `accent` unset.
2. Read `transcript_get` and propose a short list, one line each with its time, template and
   text. The user's library comes first: an item whose `useWhen` fits the moment beats a
   bundled template (it is how they like it). Otherwise a `title` where a new section starts,
   `step` cards for numbered steps, `shortcut` when a key combination is named, `key-point` for
   a rule worth remembering, `recap` near the end, `end-card` last. Few and short (about one a
   minute at most, 3–5 s each), never over a part where the screen must be seen. Ask: add them
   all / pick / skip.
3. `graphic_add` each with `atWord` (the word it starts at) and its `variables` (`library` with
   the item's name for a library one), then `graphic_render` and look at one or two with
   `frame_render` inside them. Report one line.
4. If you wrote a graphic of your own and the user likes it, offer to keep it in their library
   for later videos; on a yes follow SKILL.md: `graphic_templatize`, rewrite, `graphic_check`
   until it passes, then `library_save` (ask about a close item it names).

## 7. Chapters (automatic, last edit)

Chapters come last, once everything else is cut, since they follow the final edit. Read
`transcript_get`, pick where each topic starts (at least 3, each 10 s or longer, the first at the
start), and send `chapters` with `set`: a `title` and the `atWord` each starts at. Use the app's
own pass (`chapters` without `set`) only if the user asks for it.

## 8. Check, status and summary (automatic)

Run `check`. Errors and warnings with a fix: apply them in automatic mode (`fix` with their
ids), otherwise list them and ask. Notes (loudness, choppy stretches, zoom jumps) are the
user's call: list them, fix only at the user's word. Then `storyboard`: one image of the whole
edit; name any tile that looks wrong (by number) and what you would do. Then `status`, and sum
up the edit in a few lines:
- What each step cut or changed, with seconds where it cut.
- Length from → to.
- What the check found and fixed, and what it left (with times).
- What to check by hand:
  - the first and last ten seconds (cuts at the edges),
  - each retake cut (that the kept take is the right one),
  - the layout sections and zooms on the timeline,
  - mouth noises left alone next to words.
- Any step can be undone from the Agent panel's history.

## 9. Export (ask)

Ask before exporting, then follow the export steps below.
