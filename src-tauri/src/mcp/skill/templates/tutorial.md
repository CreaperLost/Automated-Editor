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

And two depths of review, asked once per video with the first questions (automatic: the saved
answer, else fast):

- **Fast (the default).** The cleanup passes and their reviews only look where the rules
  flagged something (an um, a "so", a phrase said twice). Quick and cheap, but a sentence the
  speaker gave up on, a restart in other words or a tangent nobody flagged is never looked at.
- **Thorough.** Also reads the whole kept transcript for those, proposes cuts for them, and
  looks at the joins (step 3b). Takes minutes and more of your tokens; worth it for a video
  that matters. Ask: "Review: fast / thorough (reads every word and checks the joins, slower)".

## 1. Project (automatic)

- Given a recording (a `.aero` folder): `create` with the project name and `recording`.
  Otherwise work on the open project.
- `status`. Tell the user the length and what the recording has.

## 2. Transcription (ask)

- `scan` gives an overview: the microphones, cameras and mouse data. Tell the user what the
  recording has in a line or two.
- Don't ask the language: `transcribe` (default `provider: auto`) runs online with Scribe,
  which detects it (English, Greek, ...), ElevenLabs first and OpenRouter when ElevenLabs has
  no key or credits. Its answer says what it heard; the later passes follow it. Use
  `provider: parakeet` (on this computer, free, English only) only when the user asks for a
  local or free transcription of an English video.
- With these questions, ask the review depth: fast (default) or thorough (see the top).
- Two or more microphones: ask which to transcribe: the presenter's (default), or all of
  them (needed to cut gaps between words across all of them).
- `transcribe` (with `stream` for each microphone). It can take a few minutes, so tell the
  user, and pass on the language it heard.

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
   - **Always `review: auto` (leave `review` out)**, fast or thorough alike: `fillers` runs the
     rules with the app's OpenRouter review when a key is set, else with its built-in local
     model when it is downloaded. Don't review the words yourself unless the user asks you to
     or `fillers` says no review could run: the app's review costs you nothing. Dry-run it
     (`apply: false`): give how many came from the review, the seconds it would cut and a few
     examples. Read the transcript around a flagged retake only when it looks like content
     (a full sentence or an item of a list), and then offer "fillers only".
   - **No review could run** (`fillers` says so): you are the AI review.
     By default `fillers_review` shows only windows of about 15 words either side of what the
     app's local pass flagged (a "like" or "so" that may be filler, a phrase said again); um
     and uh are cut by the rules already. Read every page (`page: 0`, 1, ...), mark the
     fillers and retakes by the criteria it gives (a flag that carries meaning stays), then
     save them in one `fillers_review` call with `cuts` and the pages' `stamp`. With a
     thorough review (or when the windows miss things the user points out), read every word
     with `review: "thorough"` instead. Then dry-run `fillers` the same way.
   Choices: yes / fillers only (`kinds: ["filler"]`) / skip. Then `fillers`. When the user
   wants nothing sent to OpenRouter, use `review: local`.
5. **Layouts** (only with a camera; `scan` says how many): first, the base layout: keep the
   camera bubble (default) / side by side / no camera (`layouts` `op: base` with `layout`
   "Side by side" or "Screen only"). Then **picked sections**: `layouts` `op: pick` with
   `apply: false` first (the camera alone where the speaker talks and the mouse rests:
   sections and their total). Choices: yes (`op: pick`) / skip.
6. **Zooms** (only with mouse data): `zooms` with `apply: false` (count, average length, how
   far in; add `settings` to try others: nothing is saved). Choices: yes / fewer and longer /
   shorter / skip. "Fewer and longer" raises `minHoldMs` and `mergeGapMs` (default 5000 each),
   "shorter" lowers them; how far in is `clickScale` (1.6 default). Then `zooms` with the
   same `settings`. A cut never makes a zoom jump: its glide plays whole on the edited video.

### 3b. Thorough review (only when the user chose thorough)

After the passes, on the cut edit:

1. **Read every kept word.** `transcript_get` in stretches of about two minutes (`fromUs`,
   `toUs`) until the end. Look for what the rules can't see: a sentence the speaker gave up on
   ("so what you want to- okay, let's open the settings"), a restart in other words, a tangent,
   the same explanation given twice. Keep what teaches: a deliberate repeat ("human in the
   loop. Human in the loop means..."), items of a list, and pauses while something loads or the
   viewer reads the screen. Words the transcriber got wrong are not mistakes of the speaker:
   fix them with `transcript_fix` when you are sure, never cut them.
2. **Propose those cuts** in one `propose_cuts` batch, by word id, each with a short reason
   ("abandoned sentence", "said again at 4:12"). Tell the user how many and how many seconds;
   they accept or reject each in the Agent panel.
3. **Look at the joins.** `timeline_view` (`fromUs`/`toUs` about 2 s either side) at every
   retake cut and every gap still over a second: look for a word cut short, a breath or click
   left at the join, a jump in loudness. You can see the waveform; you cannot hear it.
4. **Make a listening list** for the user: the joins you looked at that might sound wrong,
   each with its time and why, at most ten, worst first.

After the passes, save the choices with `template_save` (`name: tutorial`), in this shape:

```json
{
  "review": "fast",
  "mouthNoises": { "apply": true, "preset": "balanced" },
  "silence": { "apply": true, "preset": "balanced", "microphones": "all" },
  "gaps": { "apply": false, "preset": "balanced" },
  "fillers": { "apply": true },
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
the full-screen camera eases (the app decides; `graphics` gives the rules with its list). Ask
whether the user wants graphics (default no in automatic mode unless the saved answers say yes). If yes:

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
   for later videos; on a yes follow the graphics rules: `graphic_templatize`, rewrite, `graphic_check`
   until it passes, then `library_save` (ask about a close item it names).

## 7. Chapters (automatic, last edit)

Chapters come last, once everything else is cut, since they follow the final edit. Read
`transcript_get`, pick where each topic starts (at least 3, each 10 s or longer, the first at the
start), and send `chapters` with `set`: a `title` and the `atWord` each starts at. Use the app's
own pass (`chapters` without `set`) only if the user asks for it.

## 8. Check, status and summary (automatic)

Run `check`. Errors and warnings with a fix: apply them in automatic mode (`fix` with their
ids), otherwise list them and ask. Notes (such as loudness) are the
user's call: list them, fix only at the user's word. Then `storyboard`: one image of the whole
edit; name any tile that looks wrong (by number) and what you would do. Then `status`, and sum
up the edit. Keep four kinds of claim apart, under these headings, and never let one stand in
for another (a storyboard is not a review of the whole edit; a clean `check` reads the edit, not
the sound):

- **Done:** what each step cut or changed, with seconds where it cut; length from → to; what
  the check fixed.
- **Looked at:** what you actually read or saw, and how much of it: every kept word (thorough)
  or only the flagged windows (fast); the joins you viewed with `timeline_view`; the storyboard;
  any frames.
- **Not heard:** you cannot listen, so no join has been heard yet. List the ones to play first
  (the listening list from 3b, or with fast review: the first and last ten seconds and each
  retake cut), with times.
- **Approved by you:** which passes and proposal batches the user said yes to, and which ran
  automatically on saved answers.

Then **Still open:** what the check left (with times), proposals still waiting in the Agent
panel, and anything you were unsure about. Any step can be undone from the Agent panel's
history.

## 9. Export (ask)

Ask before exporting, then follow the export steps below.
