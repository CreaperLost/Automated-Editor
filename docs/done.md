# Done

What shipped, most recent first, a line or two each. What is still to do is in
[roadmap.md](roadmap.md). The details are in git history (commit messages and the plans deleted
once built), the design in [architecture.md](architecture.md), the choices in
[decisions.md](decisions.md).

When a roadmap item ships, add it at the top here with the date and its ID as an anchor
(`<a id="xxx-n"></a>`), and delete it from the roadmap.

- **1 Nov 2026** — <a id="qual-5"></a>**QUAL-5 what EVAL-1 run 5 found**: `check` with
  `short` checks a short or clip as its export plays it (its own timeline, captions and
  graphics; its fixes stay with the short's own tools), and the export steps say so;
  `graphics` with `rules: false` leaves out the rules an agent read earlier in the
  conversation (about 2.5k tokens; the shorts method says when); the frame tools
  (`frame_render`, its sheets, `storyboard`) render and wait for a graphic that plays there
  and is out of date, so a fix shows as made, never the old render; `count-up`'s sign
  ("$", "€", "%": two characters at most) is the number's size and colour, right against it.
  Tried in the dev app: "$25" drawn straight after changing "$10", and "€5M".
- **1 Nov 2026** — <a id="eval-1-run-5"></a>**EVAL-1 run 5**
  ([plan](plans/evaluation.md#run-5-1-nov-after-gfx-8-plan-1-versions-auto-1-and-dlv-2)): run 4's
  three messages on the 31 Oct–1 Nov work. Same cleanup; graphics that show what is said (four
  demonstrations of six, against six text cards); a short with captions, emphasis and three
  graphics. 57 calls and $0.69 against 31 and $0.39, the edit's context ending at 88k. Its
  findings are fixed ([QUAL-5](#qual-5)).
- **1 Nov 2026** — <a id="fx-3"></a>**FX-3 camera push-ins**, off by default (George): a short's
  Look panel has "Push in at jump cuts", and a preset's `short` defaults take `pushIns` for new
  shorts. At a jump cut in the camera the framing goes 1.15x closer, and back out at the next,
  centred on the face; a framing holds at least 2.5 s (a cleaned take is cut every few tenths
  of a second, and changing at each would strobe); footage or a gap in between starts it over.
  Preview and export alike. Tried on the 25 Oct take short: in at 6 s, out at 12 s, in at 17 s.
- **1 Nov 2026** — <a id="dlv-2"></a>**DLV-2 delivery list**: one export at a time stays the
  default (George). While one runs, the Export dialog offers "Add to queue", and exporting
  ticked shorts queues them all; the queue runs in order on one worker. The dialog's Exports
  list (kept in the project, `exports.json`) shows what waits, runs and went out (videos,
  shorts, subtitles, transcripts, sound), the file and when, "Older than the edit" when an
  export now would differ (judged by what the export contains, so an undo and redo, or an edit
  to another short, doesn't mark it), and per line: take out of the queue, stop, show in the
  folder, export again from the edit as it is now (beside the old file), take off the list.
  Queued exports never take a name another one will write. Agents keep one at a time. Tried in
  the dev app: a second export refused without the queue, queued with it, both written as (4)
  and (5); after an edit, "export again" made (6).
- **1 Nov 2026** — <a id="sty-3"></a>**STY-3 a style from references in the Style panel**:
  "New style from references…" (the Style page's header): pick pictures or clips, say what
  you like, optionally a name. The app keeps the request (`style-request.json` in its
  settings) and copies it as a prompt; an agent's `status` names it, even with no project
  open, until it runs `library` `op: propose`, which takes it up. Nothing is sent anywhere by
  the app. Tried in the dev app.
- **1 Nov 2026** — <a id="gfx-8"></a>**GFX-8 graphics an agent makes well**, closed by
  George: the two 31 Oct rerun shorts (Claude Code, Codex) are OK. The over-face panel stays
  where it is. His note: full-camera moments in those review renders weren't centred on the
  face. They came from a dev build without face detection (the installed app has it), but it
  showed a real gap: an export started before a camera's face track was found (it is found
  in the background after opening) used the centre crop. An export now waits for the face
  first ("Finding the face in the camera"); tried by exporting right after opening.
- **1 Nov 2026** — <a id="gfx-4"></a>**GFX-4 graphic versions**: a redo of a graphic goes into a
  version beside it (its own folder and renders, marked as a version of the first), so the one
  the user saw is kept. Inspector: the Version row (1, 2 · what changed, + New) plays another
  version in the same places, on the video and every short's own timeline; + New copies the
  graphic into a version that plays at once, so changes go into the copy. Each is one undo step.
  Agents: `graphic_set` `newVersion` (a copy that doesn't play yet), `frame_render` `graphic`
  on it shows it in place (with `sample: true`, a clip), `graphic_set` `use: true` plays it;
  `graphics` lists `versionOf` and the note. A word or a colour still changes in place. Tried in
  the dev app.
- **1 Nov 2026** — <a id="auto-1"></a>**AUTO-1 run a preset after opening**: Preset → Run
  "Tutorial…" lists the steps the app runs itself (scan, transcription unless done, then the
  passes the preset's choices turn on: mouth noises, pauses, gaps, fillers only, zooms with
  mouse data), each one undo step through the same tools an agent calls, `guard: true` on the
  cuts. Only free steps on this computer run unless the user ticks online steps for that run.
  It stops at a pass that waits for the user ("would cut more than a quarter"), an error, or
  where the agent takes over (retakes, graphics, chapters, export), and says so; `status` tells
  an agent later which passes the app already ran. Tried in the dev app on a copy of the 25 Oct
  take (mouth noises muted, the pause pass held).
- **1 Nov 2026** — <a id="gfx-3"></a>**GFX-3 templates for tutorial jobs**, closed by George:
  what GFX-8 built covers it (`step`, `steps-link`, `items-stack`, `before-after`,
  `app-click`, `timeline-cut`, `prompt-reply`, `count-up`, the diagram parts, 14 templates in
  all, each through the gate). Not built and not wanted now: a word-timed emphasis template, a
  lower-half overlay prototype, purpose tags and search in the library.
- **31 Oct 2026** — <a id="plan-1"></a>**PLAN-1 style trial**: `frame_render` with `moments`
  and `sample: true` gives a playable clip from the first moment to the last (3–20 s, with
  sound) beside the sheet; the rules' step-by-step part says to dress 5–10 s first when the
  look is new and scale after the user likes the clip. With the plan file below, PLAN-1 is done.
- **31 Oct 2026** — <a id="gfx-8-table"></a>**GFX-8 the plan table from the tool**
  ([plan](plans/graphics-quality.md#as-built-31-oct)): `graphic_add` with `plan: true` checks
  the items and answers with the table (start, end, the words said there, template or library
  item, overlaps, what it shows), adding nothing; the graphics guide and the Tutorial method
  use it.
- **31 Oct 2026** — <a id="plan-1-file"></a>**PLAN-1 plan file**: `plan` (behind `more`)
  keeps the agent's plan as `plan.md` in the project: `write`, `edit` (each old text found
  once) and `read`, which ends with where each word id in it plays now (or that it was cut), so
  a later cut moves a row without the file changing. `status` names a plan; the rules say to
  read it first, save the table once the user agrees, and follow what it locks. Tried in the
  dev app.
- **31 Oct 2026** — <a id="sht-1"></a><a id="sht-1-find"></a>**SHT-1 idea-first clips, last
  parts** ([plan](plans/shorts.md#as-built-31-oct)): the cloud `find` (Clips tab "Find with AI",
  `clips` `op: find`) now picks ideas, each one stretch or up to four pieces in play order, and
  a gathered pick gets its own timeline like one an agent gathers. "Take the video's cuts again"
  rejoins a gathered clip's pieces as the video has them now (it played everything from the
  first piece to the last before); agents: `clips` `op: resync`. A piece the video trimmed
  plays what is kept, one cut whole is left out.
- **31 Oct 2026** — <a id="sht-1-arrange"></a>**SHT-1 arranging pieces**
  ([plan](plans/shorts.md#as-built-31-oct)): a gathered clip's pieces in the Clips tab, each
  with its place in the video and earlier, later, remove; agents: `clips` `op: arrange` with
  `order`, and `list` shows each piece. A piece takes its own cuts and graphics with it.
- **31 Oct 2026** — <a id="sht-1-gather"></a>**SHT-1 Gather**
  ([plan](plans/shorts.md#as-built-31-oct)): select words in the video's transcript, then "Add
  to clip…": a new clip of them, or they play after what a clip plays (its own cuts kept).
  Agents: `clips` `op: add`. Words a clip already has are refused. Tested in the dev app.
- **31 Oct 2026** — <a id="gfx-8-rerun"></a>**GFX-8 rerun**
  ([plan](plans/graphics-quality.md#the-rerun-31-oct)): the 25 Oct take again with Claude
  Code (Opus 5.5, 55 calls, $1.43) and Codex (gpt-6-astra, 84 calls). Both made colourful,
  moving graphics that show what is said (linked steps, a typed prompt, gaps closing, the apps
  side by side) instead of white text cards. Fixed from the runs: captions moved out of a part
  a graphic fills, the shared text fit no longer shrinks a font's overhanging letters to a dot,
  `prompt-reply` keeps both messages in a short's part.
- **31 Oct 2026** — <a id="sht-1-pieces"></a>**SHT-1 first part, clips gathered from anywhere**
  ([plan](plans/shorts.md#as-built-31-oct)): `clips` `op: create` takes an `idea` and `pieces`
  (word ranges from anywhere, in play order); the clip gets its own timeline joining them as
  the video has them now (`shorts::join_slices`), overlapping pieces are refused, layouts are
  picked inside each piece only. The Clips method starts from ideas; the Clips tab shows the
  pieces and the idea. Tested in the dev app: a later stretch then an earlier one, 24.9 s.
- **31 Oct 2026** — <a id="gfx-8-diagrams"></a>**GFX-8 diagram parts**
  ([plan](plans/graphics-quality.md#as-built-31-oct)): bundled `steps-link` (up to five steps
  appear from `cue`, each joined by a drawn line, the current one ringed; across in 16:9, down
  in portrait) and `items-stack` (up to five items land one on another, a line under them).
  Both pass the real gate first time and render clean in 16:9 and a 9:16 clip. The frame
  checks now wait for a graphic still rendering instead of measuring its placeholder.
- **31 Oct 2026** — <a id="gfx-8-safe-area"></a>**GFX-8 9:16 safe area**
  ([plan](plans/graphics-quality.md#as-built-31-oct)): in a short, `frame_render` with
  `graphic` names words or detail under the bars and buttons Shorts and TikTok draw once posted
  (top 8%, bottom 20%, right 12% below the middle, 5% side edges), found from the graphic's own
  pixels row by row. All eleven templates clean in a 9:16 clip; planted words flagged.
- **31 Oct 2026** — <a id="gfx-8-toolkit"></a>**GFX-8 toolkit, first three demonstrations**
  ([plan](plans/graphics-quality.md#as-built-31-oct)): bundled templates `timeline-cut` (gaps
  close on `cue`, length before → after), `prompt-reply` (types, sends on `cue`, reply
  streams) and `count-up` (digits roll up from `cue`). Seek-safe GSAP, the look's tokens and the
  style's colours; the real gate passes all nine templates; rendered in the dev app in English
  and Greek. Later the same day: `before-after` (a line sweeps the before into the after) and
  `app-click` (a cursor clicks a named button, the result appears), and a `check` hint when a
  graphic's words take longer to read than it is on screen.
- **31 Oct 2026** — <a id="gfx-8-frame-checks"></a>**GFX-8 frame checks**
  ([plan](plans/graphics-quality.md#as-built-31-oct)): `frame_render` with `graphic` now ends
  with checks measured from the scene's pixels at the sheet's moments: a face partly covered
  (through the face track and the camera's crop), captions hidden under a graphic, captions
  across a short's split line, and a hint when a graphic holds still. Full cards are exempt.
  Live: TX Flow clean, a bad panel over the bubble flagged on all three. Sounds on graphics
  became opt-in the same day.
- **31 Oct 2026** — <a id="gfx-8-first"></a>**GFX-8, first part**
  ([plan](plans/graphics-quality.md#as-built-31-oct)): guidance (show or card per moment, the
  text-removed test, a plan table with what the viewer sees, honest look knobs), the **sample
  clip in place** (`frame_render` `graphic` + `sample: true`: frames for the agent, a 720p clip
  with sound for the user) and a `graphic_font` check. Toolkit, the frame-based checks, UI-8
  and the rerun remain.
- **31 Oct 2026** — <a id="sty-2-sound"></a>**STY-2 step 5, a style's sound set**
  ([plan](plans/styles.md#as-built-step-5-31-oct-sound-set)): the brief shows each cue's sound
  (name, length) and how to place it; My library gets four quiet CC0 cues (soft whoosh, soft
  click, pluck pop, soft hit) and a one-line policy. Step 4 (shorts variant) skipped until
  George brings references.
- **31 Oct 2026** — <a id="sty-2-propose"></a>**STY-2 step 3, `op: propose`**
  ([plan](plans/styles.md#as-built-step-3-31-oct-op-propose)): up to 8 references (images,
  clips read as one board at two frames a second, library graphics as their contact sheet and
  source) shown to the agent with the steps (breakdown first, up to three previewed proposals,
  save only the pick). Saves nothing itself. Not yet run by an agent; the pick as a clip in
  place waits on GFX-8 §2.
- **30 Oct 2026** — <a id="sty-2-my-library"></a>**STY-2 step 2, the "My library" style**
  ([plan](plans/styles.md#as-built-step-2-30-oct-my-library)), in George's styles folder: the
  look tuned to his four white library graphics, seven motion rules, five kinds, the four
  graphics as examples, six look rules and a breakdown; `check` clean. Tested 31 Oct: a fresh
  Opus agent made a new graphic in it ("TX Flow prize pool"); George: "It's cool", not kept.
- **30 Oct 2026** — <a id="sty-2-parts"></a>**STY-2 step 1, a style's new parts**
  ([plan](plans/styles.md#as-built-step-1-30-oct)): motion rules, kinds of graphics with their
  short slot, library examples, look rules, references (images and clips in the style's folder),
  a breakdown and a sound policy with cues, plus a 9:16 motion/kinds/examples part.
  `library` `kind: style` `op: show` is now a text brief agents read before graphics; `op: set`
  refuses a font the style doesn't carry; `op: check` reports the style's own problems without a
  project. Steps 2–6 (the "my library" style, `op: propose`, shorts variant, sound set, GFX-8)
  remain.
- **30 Oct 2026** — <a id="skill-text"></a>**Skill text from the research** (requirements.md
  §5 row 1): the graphics guide (`mcp/skill/graphics.md`) asks for real material only, builds a
  composition in three passes (stills with ids, timing, motion), holds text for its reading time,
  compares what was promised with the frames before reporting, and keeps a fix to its one graphic
  with an edge check; the rules (`rules.md`) read every correction as where / what is wrong /
  what it becomes / what stays, and change only what was named. Not yet run in EVAL-1.
- **26 Oct 2026** — <a id="online-fillers"></a>**Online review fillers cut at once** (George's
  call): in the app, Review with the OpenRouter/OpenAI review cuts the fillers it and the rules
  found straight away (one undo step) and leaves the retakes in the list; the tutorial method
  tells agents to cut fillers with `kinds: ["filler"]` and dry-run only the retakes. A local or
  in-house review still lists everything first. **EL-B**, George's first real Greek recording
  (13 s): the online review cut exactly the retake and the optional opening "Λοιπόν"; the rules
  missed the retake ("γιααα" drawn out) until retakes matched a held letter as one, and now
  catch it, with the other five cases unchanged. Scribe direct rejected the saved ElevenLabs
  key, so Scribe via OpenRouter transcribed it.
- **26 Oct 2026** — <a id="qual-3-debuginfo"></a>**QUAL-3** dev builds keep line tables only
  (`debug = "line-tables-only"` for the app and its dependencies): a one-file change 47 s →
  15–28 s, the PDB 285 → 79 MB; backtraces keep file and line, a debugger loses locals (set
  `debug = true` for that). A release build links with `rust-lld` (6 min from a warm cache)
  and its MCP server answers.
- **26 Oct 2026** — <a id="fx-2-finished"></a>**FX-2 finished:** **blur behind a graphic**, a
  second switch beside "Dim behind it" on a graphic clip (and `effects` `kind: blur_behind`):
  the screen and camera under the graphic's rect blur while it plays (blocks of 12–40 px at
  1080p, eased with the clip; a round or rounded camera only when the graphic covers it whole,
  so its shape stays). Boxes **move and resize** while paused: drag a box, or a corner, and it
  is saved on release as one undo step (`project_effect_move`, `effects` `op: move`).
- **26 Oct 2026** — <a id="lng-4"></a><a id="lng-4-repeat"></a>**LNG-4** a phrase ending one sentence and opening
  the next is meant ("what we call human in the loop. Human in the loop essentially means"):
  neither the rules' retakes (`transcript/edit.rs`) nor the review's candidates flag it when
  the phrase doesn't open its sentence and the sentence doesn't end on a small word ("open up
  the." is still a retake). Rules rerun: EN-A's one wrong cut gone; EN-B, EN-C, EN-D and EL-A
  unchanged.
- **26 Oct 2026** — <a id="snd-3-komiku"></a>**SND-3** 14 tracks by Komiku (CC0 on all 266 of
  their FMA track pages, no other licence found), so the library is no longer one composer's:
  calm, focus, happy, nostalgic, upbeat and game, pinned and downloaded like the rest. Built-in
  tracks now carry their artist.
- **25 Oct 2026** — <a id="cost-1"></a>**COST-1** review boards: `storyboard` `board: fine` (a
  frame a second, one JPEG a minute, 10×6 tiles of 148 px, about 1.0k image tokens by Claude's
  pixel count) or `coarse` (every 5 s, ten minutes an image, about 120 tokens a minute); amber
  edge on graphics, blue on zooms; 5 or 3 images a call, then "call again with fromUs". The
  rules point at `fine` for a stretch the scene storyboard leaves in doubt. Not measured yet:
  what an agent still catches on a board (needs an EVAL run).
- **25 Oct 2026** — <a id="lng-5"></a>**LNG-5** custom vocabulary: the key terms (Transcription
  settings, or `settings` `area: transcription` `keyterms`) bias Scribe as before, and now every
  transcript respells them afterwards: spacing and case for any transcriber ("chat gpt" →
  ChatGPT), and for Parakeet, runs of two or more words that sound like a term ("cloud code" →
  Claude Code); one word that only sounds alike ("codecs") is left. OpenRouter answering 400 to
  key terms transcribes without them instead of failing; terms are checked against Scribe's
  limit (5 words, 50 characters). The field now shows for Parakeet too.
- **25 Oct 2026** — <a id="qual-3"></a><a id="qual-3-done"></a>**QUAL-3** faster builds. Measured with `cargo build
  --timings` on the dev profile: a one-file change took **6 min 23 s**, 338 s of it the library
  writing a 1.5 GB static library and a DLL that nothing links (Tauri's mobile template asks for
  `staticlib` and `cdylib`). Now `crate-type = ["rlib"]`: **1 min 1 s**. Linking with Rust's own
  `rust-lld` (`src-tauri/.cargo/config.toml`) took the binary's link from 19.5 s to 7.6 s:
  **41 s**. A warm `cargo check` after a change is 9 s.
- **25 Oct 2026** — <a id="snd-3"></a><a id="snd-3-music"></a>**SND-3 (music)** 24 CC0 tracks by mood (calm,
  focus, happy, dreamy, nostalgic, upbeat, game) by HoliznaCC0, each licence read on its own
  FMA page; Loyalty Freak Music was turned down (archive.org says CC0, the artist's Bandcamp
  says CC BY 4.0). The app doesn't ship them: each is pinned by size and SHA-256 and downloaded
  the first time it is played or placed (`music/built-in` in the app folder). A **Music** page
  in the Media panel (moods, play, + fills from the playhead to the end, add your own) and
  `library` `kind: music`. Placing fills the stretch on a "Music" track (repeating the track,
  faded in, out and at each repeat, brought to −16 LUFS) and switches the music under speech
  on (18 dB) when it was off: one undo step. The user's own music is refused unless its licence
  needs no credit and allows monetised use (new licence kind `nonCommercial`); `check` flags
  music and effects like that anywhere in the edit.
  Later the same day (George): **Sounds** and **Music** are dock panels of their own in Edit,
  Cleanup, Shorts and Clips (no longer pages of the Media panel); placing goes into the timeline
  shown, so in Shorts or Clips into that short's own timeline (the video is left as it is).
- **25 Oct 2026** — <a id="fx-2"></a><a id="fx-2-done"></a>**FX-2** screen effects: blur (the picture reduced to a
  coarse grid of colour, 8–32 recorded pixels a block, never faded), hide (a solid block),
  spotlight (the screen dims around the box), magnifier (a round lens, 2× by default) and dim
  behind a graphic (a switch on the graphic clip). An effect is a box of the recording's own
  picture over a stretch of its own clock, so it stays on what it covers through crops, zooms,
  layouts and cuts, in the preview and the export (one drawing path). Drawn with the tools over
  the preview (five seconds from the playhead; while paused the boxes show with "end here" and
  ×), or by agents with `effects` (a box as `frame_render` shows the frame). GPU and CPU agree
  (`gpu_cpu_screen_effects_within_tolerance`); effect layers reuse the screen's texture rather
  than upload it again. The agent rules say to cover keys, tokens, passwords, emails and
  balances when a frame shows one, and to tell the user.
- **25 Oct 2026** — <a id="auto-2"></a><a id="auto-2-done"></a>**AUTO-2** matching speech across takes ("Match takes"
  in the Audio panel, on for new projects and the built-in presets). Each sound file of speech
  is measured once with the rest of its analysis: the loudness of its mostly-spoken 100 ms
  blocks and its spectrum in seven octave bands. Takes more than 1 dB from the edit's level
  (weighted by how long each plays) are moved to it (9 dB at most); a take whose tone is 2 dB
  or more off in a band gets 60 % of the difference as shelves and peaks (4 dB at most). The EQ
  runs from a 100 ms warm-up before each read, so chunked playback and the export match.
  Normalisation counts the matched gains.
- **25 Oct 2026** — <a id="ui-8"></a><a id="ui-8-done"></a>**UI-8** the Shorts tab is transcript first: the Transcript
  panel sits under the shorts list (no longer behind it), a take short's transcript carries the
  take, its steps and **Clean the take**, Look's section is only "What plays under the take",
  and a **Style** tab sits beside Look. Saved Shorts layouts without the new tab fall back to
  the new arrangement.
- **25 Oct 2026** — **SND-3 (SFX starter set)** 17 CC0 sounds built into the app (4 whooshes, 2
  risers, 3 hits, 3 clicks, 3 pops, a ding, a toggle) from Kenney's packs and Freesound, each
  licence checked on its page (`src-tauri/assets/sounds/LICENSES.md`); written once to the app
  folder's `sounds/built-in`. The user's own go in `sounds/mine` with tags and a licence. A
  **Sounds** page in the Media panel (search, tag filters, play, + at the playhead, add your
  own); `library` `kind: sound` (`list` with `query`, `use` at `atUs`/`atWord`). Placing
  imports the sound once (role effects, its licence) onto a free effects track or a new
  "Sound effects" track, one undo step, never moving the video. `check` now also flags effects
  whose licence asks for credit.
- **25 Oct 2026** — <a id="fix-8"></a>**FIX-8** cuts in a take short stay in the short: when
  the video plays the same take, `fillers` with `short` found and cut on the video's timeline.
  `fillers`, `fillers_review` and the AI review now read the short's kept words and cut its
  own timeline; `propose_cuts` takes `short` (a cuts batch remembers it, and the Agent panel
  shows "Show the short").
- **25 Oct 2026** — <a id="fix-9"></a>**FIX-9** online transcription is the user's recorded
  choice: Transcription settings → AI agents → Allow stamps the date (only the app sets it;
  `settings` refuses `agentsOnline` from an agent). The preset states it with that date, or,
  while off, tells the agent to ask once (or use Parakeet). Not yet tried with Codex.
- **25 Oct 2026** — <a id="fix-10"></a>**FIX-10** no Accept card for graphics: `short_footage`
  `op: add` and `library` style `bring_in_line` apply at once as one undo step, listed in the
  Agent panel with Undo; overlapping graphics are refused. The `under` and `restyle` batch kinds
  are gone; proposal cards stay for cuts, timeline edits and rules.
- **24 Oct 2026** — <a id="lay-2"></a>**LAY-2** layout keys: the shortcuts dialog has a
  Layouts tab (built-ins and favourites, no keys by default); a key puts that layout from the
  playhead to the next layout change. While playing it shows at once (LAY-1's swap), and every
  press until playback stops is one undo step: history steps carry a `group`, and a step of the
  top step's group joins it (`Join` line in the log, so it survives reopening). MCP `layouts op:
  switch` was already there.
- **24 Oct 2026** — <a id="ui-1"></a>**UI-1** thumbnails on video clips: small JPEGs along the
  clip (keyframe seek, one FFmpeg process each; images and the native backend decode), kept in
  `cache/thumbs/<asset>/` by file, time and height; times rounded to a step that suits the zoom,
  so small zooms and moves reuse them. Only when the video lane is 42 px or taller, only for the
  drawn part of the timeline, at most 64 a call, one call at a time; pictures scrolled away
  before their turn are never asked for. Graphics show none.
- **24 Oct 2026** — <a id="ui-2"></a>**UI-2** Basic and Advanced settings: File → Advanced
  settings (off by default). In Basic, each section keeps its fine-tuning behind a "More
  settings" link at its foot (per section, not remembered); in Advanced everything shows and the
  Advanced parts of Export, Jump cuts and Transcript settings start open. Under More settings:
  cursor size, room around the window, crop (Screen); per-clip camera framing (Camera); exact
  bubble size, roundness, screen fit, overlapping card, section transition, move length
  (Layouts); noise and duck amounts, mouth-click strength, music and loudness levels (Audio);
  distance from edge, words at once, lines (Captions); every auto-zoom setting but the amount,
  the follow speed (Zoom); graphics look sliders, caption weight, emphasis colour, outline, box,
  the frame's corners, shadows and camera border, and 9:16 graphics sliders (Style). The rest
  stays in Basic.
- **24 Oct 2026** — <a id="rev-1"></a>**REV-1** listen to the joins: a **Joins** panel (behind
  the Inspector) steps through cuts, plays ±2 s around each and marks keep or fix with a note;
  `joins` (list, find, add, clear; `agent/joins.json`) lets the agent hand over cuts and read the
  marks. Joins are held by recording times, so they move with later edits and show "changed"
  once fixed. `find` scores retakes (words cut out), words cut into or ending at a cut, unsure
  words, mid-sentence cuts and `check`'s blips. `playhead` `op: show` moves the user's preview
  (`playhead_get` is now `playhead`, behind `more`). The export guide asks for it before export.
- **24 Oct 2026** — <a id="ftg-1"></a>**FTG-1** `source_search` (behind `more`) and a **Search**
  panel (behind the transcript): every transcript, cut parts and imported takes too; whole
  words in order, the last a prefix, long words one letter off, Greek accents folded; each hit
  says plays (and where), cut (and the join over it) or not placed.
- **24 Oct 2026** — <a id="snd-1"></a>**SND-1** clips' own gain and fades (`Clip.mix`,
  `setSound` edit, the clip inspector; split keeps fades at the outer ends); **music** and
  **effects** roles; `musicUnderSpeechDb` keeps music lanes that far under the speech, measured
  (Audio page); a licence on imported files (`media_import` `licences`, the clip inspector), and
  `check` warns about music whose licence needs credit or isn't known.
- **24 Oct 2026** — <a id="agt-3"></a>**AGT-3** `library` `kind: preset` `op: propose_rule`
  (`rule` + the `example` that taught it) makes a **Rule** batch in the Agent panel; accepting
  adds a bullet to the preset's `rules.md` (a built-in gets its "My" copy); autopilot never
  accepts rules, and the rules guide says when to propose one and to name the rule that shaped a
  proposal. Designed from the creator-workflows summary, not the three skills' originals.
- **24 Oct 2026** — <a id="agt-4"></a>**AGT-4** the diagnose part, as `check` findings: linked
  sound and picture out of sync, and clips running past their media. Not built, on purpose:
  resuming a stopped job, a state screenshot (`storyboard`, `frame_render` and `timeline_view`
  cover it) and per-stage time budgets (the agent's own pacing).
- **24 Oct 2026** — <a id="lay-1"></a>**LAY-1** look-only swap: an edit that changes only the
  layout sections bumps a look generation; the engine swaps them into the running scene, keeping
  the mixer, decoders and sound playing. `perf_look_swap` on the 7:10 tutorial with 30 sections:
  the frame after a change 7.9 ms p50 swapped vs 12.1 ms rebuilt; playing with a change every
  second p50 11.5, p95 17.7, one 363 ms frame. The drag preview followed 26 Oct: dragging a
  section's edge scrubs the preview to it (one seek a frame), saved on release.
- **24 Oct 2026** — **Shorts and clips told apart for agents.** `clips` (list by default,
  create, find, range, rename, remove) replaces `short_create` and the old `shorts` find; `shorts`
  is now only for shorts made from a take (list by default, rename, remove); each refuses the
  other kind and names its tool. Finding clips (agent and the Clips tab's Find with AI) only
  adds clips: it used to replace the whole list, take shorts included. New clips and new
  shorts follow their own presets (`clipsPreset` beside `shortsPreset`; `library` `op: use`
  `for: shorts|clips`) and copy the look of their own kind. `export` takes `short: "shorts"` or
  `"clips"`. Methods, rules and the export guide say which is which.
- **24 Oct 2026** — <a id="sht-2"></a>**SHT-2** a short's caption splits, joins, hidden words and
  emphasis are its own (`Short.caption_marks`, over the video's; undo steps), text fixes stay
  shared; the Shorts tab counts them and can drop them ("Use the video's"); MCP `captions` gains
  `split`, `join` and `hide`. <a id="fix-7"></a>**FIX-7** dropping on a new track is one undo
  step (`onNewTrack` edit); undo names say "Cut 0.04 s" and leave out zooms a cut only moved, and
  the blip fix reads "Removed 3 blips"; `export` to a missing folder says it did not start.
- **24 Oct 2026** — **Everything built so far, tried** in the dev app and EVAL-1 run 4
  (tutorial 18 calls, $0.271, −28 %; a short from a take 13 calls, $0.122). Fixed from it:
  UI-5's drag row moving the tracks under the pointer; on a take short, `mouth_noises` using
  the take's voice, `fillers` taking `short`, an answer when every noise is next to words, and
  the shorts tools sending the agent to the Shorts preset. Docs cleaned up (built plans folded
  in and deleted, fixed gaps removed).
- **24 Oct 2026** — <a id="gfx-7"></a>**GFX-7** graphics ready before export: every render is
  at the export's frame rate, export renders queue in the background (2 Chrome workers, below
  normal priority, fast capture), so an export of unchanged graphics starts at once (tried:
  frames after about 15 s).
- **24 Oct 2026** — **AGT-4** stopping a long call (the client cancels, hangs up, or the user
  presses Stop in the Agent panel); **COST-2** 8 more tools behind `more` (38 → 30, 60,100 →
  41,240 characters; measured: Claude Code with tool search pays only the names); **AI-1** the
  guides trimmed (tutorial −10 %, `graphics` −45 %).
- **24 Oct 2026** — **The fixes from George's 23 Oct edit**:
  <a id="fix-4"></a>**FIX-4** dead air before the first word, after the last and around
  recording pauses is cut; <a id="fix-5"></a>**FIX-5** a filler review runs once and is
  reused; <a id="agt-5"></a>**AGT-5** the preset's choices run without a question;
  <a id="gfx-6"></a>**GFX-6** graphics proposed as a table; <a id="cost-4"></a>**COST-4**
  `frame_render` sheets of moments or a graphic; <a id="ui-7"></a>**UI-7** a streamed export
  log; <a id="ui-6"></a>**UI-6** Medium / High / Ultra, Advanced, **Remove all blips**;
  <a id="ui-4"></a>**UI-4** the transcript review panel runs Review on its button;
  <a id="ui-5"></a>**UI-5** add-track buttons, and a drop row only while dragging;
  <a id="fix-6"></a>**FIX-6** the styles and presets folders open before anything was saved.
- **24 Oct 2026** — **A second audit's six findings fixed** (A01-A06): export preferences saved
  to the wrong project, two WAV saves sharing a temp file, a range selection carried between
  shorts, waveform reads restarted on every edit, a case-variant export path that could replace
  a source file (P1), and exports re-reading graphic sources each frame (renders now pinned).
- **23 Oct 2026** — <a id="org-1"></a><a id="sty-1"></a><a id="ui-3"></a>**ORG-1 presets and styles**, all four phases:
  Style (brand, graphics look, caption look, frame, 9:16 variant; Light and Dark built in),
  Preset (settings by part, cleanup levels, export settings, the method, the user's rules, the
  choices; Tutorial, Shorts, Clips built in), projects storing only their changes, the Preset
  menu and the Style page, one `library` tool for agents. Replaced brand kits, saved looks, the
  Templates menu and saved answers. Choices: [decisions](decisions.md#presets-and-styles-as-built-23-oct).
- **23 Oct 2026** — <a id="dlv-1"></a>**DLV-1** subtitles (SRT, VTT), the transcript (text,
  Markdown) and the sound (WAV) as files, of the video or a short, from the caption cues.
- **23 Oct 2026** — **AGT-4 readiness**: `status` with no project says what this computer is
  ready for (keys, models, GPU pack, renderer, FFmpeg, the skill per agent).
- **23 Oct 2026** — <a id="safe-2"></a>**SAFE-2** history on disk (no step limit up to 2 GB),
  an agent's run undone as one step, checkpoints of the edit, graphic files and caption marks
  (one before every agent run). Checked with a real agent in EVAL-1 run 3.
- **23 Oct 2026** — <a id="cap-1"></a>**CAP-1** captions as typography: the style's font (file, installed or
  Inter Display; .woff/.woff2 too), one emphasis word per caption in the accent, pop / karaoke /
  static, captions giving way to full-frame graphics.
- **23 Oct 2026** — **GFX-5 leftovers and SHT-4's footage sound**: saved looks (now styles),
  the shorts look multiples, **Bring in line** by rule; a take's footage sound on its own track,
  always ducked under the voice.
- **23 Oct 2026** — <a id="sht-7"></a><a id="sht-7-done"></a>**SHT-7** a split graphic knows its box and renders
  with its portrait rules; on a take short the camera keeps its own part.
- **23 Oct 2026** — <a id="sht-4"></a><a id="sht-3"></a>**SHT-4** graphic modes in shorts (full, split, over the face, picked
  before the render) and footage at its own speed ([Q-SHORT-GFX](decisions.md#q-short-gfx)), the take picker, the
  footage sound control. **SHT-3** the take is cut before footage and graphics, and the app warns
  when graphics go on an uncut take.
- **23 Oct 2026** — **A smaller agent context and fewer calls**: 18 rarely needed tools behind
  `more`, a shorter automatic guide, `guard: true` instead of dry runs in automatic mode,
  `graphic_add` with `items`, graphics on a take short movable (tutorial 34 → 20 calls).
- **23 Oct 2026** — <a id="eval-1"></a>**EVAL-1 baseline** ([plans/evaluation.md](plans/evaluation.md)):
  a real agent edited the 7-min tutorial for $0.41 in 34 calls and made a short from a take
  for about $0.30.
- **23 Oct 2026** — <a id="sht-6b"></a>**SHT-6b** the agent's picks under a take are one batch;
  the Shorts tab's picker places video, graphics and files under a take by hand.
  <a id="sht-6"></a>**SHT-6** graphics on a take short's own timeline, copies of the video's
  graphics, B-roll from imported files. <a id="sht-5"></a>**SHT-5** Shorts and Clips as two
  tabs over one model.
- **23 Oct 2026** — <a id="gfx-2"></a>**GFX-2** GSAP in graphics (3.15.0, fetched once,
  sha256-pinned, CDN links rewritten to the local file); the tool list's schemas trimmed 14 %.
- **22 Oct 2026** — **GFX-5** one look for every graphic in a video (preset, five controls,
  tokens) and the templates rewritten in George's light library style.
- **22 Oct 2026** — **Quick wins**: <a id="fx-4"></a>**FX-4** 16 ms crossfades at cuts;
  loudness reaches its target (-14 LUFS by default); **GFX-1** graphics at the export's frame
  rate and 2x density above 1080p; small words said twice and broken-off words cut by the rules.
- **22 Oct 2026** — **An audit's findings fixed**: the shorts preview freeze (a deadlock with
  the window thread; the rule is in [architecture](architecture.md#threads-and-locks-the-window-thread-must-never-wait-on-us)),
  stale short edits, brand-kit saves and renames, all-or-nothing `graphic_set` and
  `transcript_fix`, and a dozen smaller ones.
- **22 Oct 2026** — <a id="perf-1"></a>**PERF-1** the transcript panel indexed and windowed
  (9,500 words: reload 583 → 310 ms, DOM 12,344 → 517 nodes). <a id="perf-2"></a>**PERF-2**
  lazy loading (start-up script 966 → 236 KB).
- **21 Oct 2026** — <a id="agt-2"></a>**AGT-2** fast or thorough review and an honest report
  (Done / Looked at / Not heard / Approved by you / Still open).
  <a id="agt-1"></a>**AGT-1** agent parity for cleaning up and correcting (zooms, graphic
  moves, chapters, shorts, word fixes, roles, export cancel); calls bound to another project
  refused under the command lock.
- **21 Oct 2026** — <a id="fix-1"></a>**FIX-1** the render queue keeps renders of one graphic
  at two lengths; <a id="fix-2"></a>**FIX-2** the skill's on/off defaults; <a id="fix-3"></a>**FIX-3**
  honest read-only and destructive hints, a real `propose_edit` schema;
  <a id="safe-1"></a>**SAFE-1** agent calls bound to their project; <a id="dep-1"></a>**DEP-1**
  Rust 1.92.0, Node 22, FFmpeg pinned with checksums, `Cargo.lock` committed.
- **21 Oct 2026** — Layouts: base layout + timed sections, favourites, pick rules, face framing
  ([plan](plans/layouts.md)). Local vs cloud filler review measured
  ([eval](plans/local-review-eval.md)). Clip speed 0.25×–8×. Docs consolidated.
- **20 Oct 2026** — Greek (online), Parakeet Unified EN + Verbatim, the CUDA 13 GPU pack, local
  Qwen3.5 4B review on Vulkan; sound effects off by default; fillers use rules + AI by default.
- **18 Oct 2026** — Shorts as a workspace tab; take shorts (`short_from_take`, `short_footage`).
- **17 Oct 2026** — Face framing (YuNet); fast switching between shorts.
- **13–16 Oct 2026** — AI readiness phases 0–3: MCP bridge, proposals and Agent panel, app actions
  and CLI, skill and templates, `check`, shorts scenes, graphics v1, library + gate.
- **4–5 Oct 2026** — Premiere-style tracks (one sequence model, clocks); performance pass R01–R20.

## Old task numbers → IDs

For anything (memory notes, commit messages, old chats) that still says "task N".

| Old | New | Old | New |
|---|---|---|---|
| #1 More layouts | Done; follow-ups LAY-1…LAY-4 | #10 Number keys | LAY-2 |
| #2 More languages | Removed (was LNG-1, LNG-2) | #11 Auto layouts | LAY-3 |
| #3 Measure local review | Done; LNG-4 | #12 Transcription models | Removed (was LNG-3) |
| #4 B-roll | FTG-2, FTG-6 | #13 Version history | SAFE-2 |
| #5 Music and SFX | SND-1, SND-2 | #14 Source search | FTG-1, FTG-3 |
| #6 Templates | UI-3 | #15 NLE handoff | DLV-3 |
| #7 Clip thumbnails | UI-1 | #16 MCP discovery | AGT-4 |
| #8 Simpler UI | UI-2 | #17 Project binding | SAFE-1 |
| #9 Background removal | FX-1 | Known issues F05 / F06 / render queue | PERF-1 / PERF-2 / FIX-1 |
