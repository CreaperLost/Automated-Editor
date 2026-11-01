# Styles from references (STY-2)

**Status:** plan, 25 Oct 2026. Steps 1 and 2 built 30 Oct, step 2 tested and steps 3 and 5
built 31 Oct (see "As built" below). Step 4 waits on George's shorts references; step 6,
GFX-8, closed 1 Nov.
Roadmap: [STY-2](../roadmap.md#sty-2), done
before [GFX-8](../done.md#gfx-8) (graphics an agent makes well,
[plans/graphics-quality.md](graphics-quality.md)).

This merges three pieces that turned out to be one problem:
- GFX-5's leftover: styles the agent proposes (research in
  [research/synthesis.md](../research/synthesis.md), F6).
- STY-2: a style made from references.
- The worked examples from GFX-8: a few graphics done well that the agent learns from.

**Why first (George, 25 Oct):** the bad graphics in the 25 Oct test are not a shorts-only
problem. A style today only holds look settings, so when an agent is asked for a look it has
nothing to go on but the stock templates and a number like `energy`. Long form looks right
because it reuses George's own library graphics, which carry his taste. A style made from
references gives every video, long or short, the same thing: the look, how it moves, which
kinds of graphics fit, and examples of it done well. GFX-8 then builds graphics on top of it.

## What a style holds today

`style/mod.rs`, kept in `<config>/AeroEdits/styles/<name>/style.json` with its font and logo
files beside it:
- **Brand:** four colour roles, title and body fonts (a font file may sit in the folder), a logo.
- **Graphics look** (`graphics/look.rs`): a base preset (app, calm, bold) and multipliers for
  size, roundness, speed, energy, density, plus hand-set `--look-*` tokens.
- **Captions, frame** (background, screen, camera) and **portrait** (multipliers for 9:16, an
  optional caption style and background).

All of it is about how things look. Nothing says how graphics move beyond scaling motion that
already exists, which kinds of graphics fit, or shows one done well.

## What a style will hold (decided 25 Oct)

1. **Settings,** as today. Fonts must be usable: a font that isn't bundled in the style's folder
   must resolve in the renderer, with the weights the style uses, or the style check refuses it
   and says what is available. The style can't name a font it doesn't carry.
2. **Motion language:** a few written rules plus easing tokens. How things enter and leave,
   what moves and what holds, how long a graphic holds, how one graphic hands over to the next,
   and one signature move. Replaces "energy" as the way to say "more motion"; `energy` stays as a
   multiplier and its description says plainly what it does.
3. **Kinds of graphics:** which kinds this style uses and when (for example "app mock-ups for
   steps, calm cards for quotes and numbers, no lower-thirds"), each with where it sits in 9:16
   (full frame, split, beside the face). The agent picks per moment within this list. Text cards
   and demonstration graphics are equals (decided 25 Oct).
4. **Examples:** 2–3 graphics **linked from the library** (decided), one per main kind. For the
   long-form style they are George's own library graphics. The agent reads their source and
   adapts it. One source of truth; a style is not shareable on its own.
5. **References:** the images and clips the style came from, kept in the style's folder, plus
   the agent's written breakdown: what it took from them and what it left out.
6. **Look rules:** rules about the look, like "never an opaque card over my face". Workflow rules
   ("render one sample first") stay in preset rules.
7. **Shorts variant, richer** (decided): same style and references; the portrait part gets its
   own motion rules, kinds and examples on top of today's multipliers.
8. **A small sound set** (decided): a whoosh, a click, a pop and a hit for graphic moments, each
   with its licence stored, from the SFX starter set ([SND-3](../done.md#snd-3)) or George's
   own files. Graphic events cue them ([SND-2](../roadmap.md#snd-2)).

**Where it lives:** everything the app uses stays in `style.json` (motion rules, kinds, look rules
and the breakdown as short text fields); references and sounds as files in the style's folder.
One file, no second Markdown copy to drift. `library` `kind: style` `op: show` answers with a
compact readable brief, which is what an agent reads before any graphic.

## Making a style from references

1. **Intake.** The user gives references: images, clips, a website URL, a logo, library graphics
   ("my library"), optionally a few words. Clips are read as frame boards, not whole videos
   ([COST-1](../done.md#cost-1)). If there are no references, the agent asks for one, but can
   still work from words.
2. **Breakdown.** The agent writes what it sees, as a short table: palette, type, motion moves,
   kinds of graphics, what it drops and why. This is shown to the user before anything is made.
3. **Proposals.** Up to three styles that differ in a named way (never three shades of one),
   each previewed as **stills on the video's own moments** (`op: preview`). With a tight
   reference, like George's library, one proposal is enough.
4. **The pick, in motion.** For the chosen style, one example graphic rendered as a **short clip
   in place** in the user's video (face, captions, audio, a little footage either side). The agent
   inspects it first; the user approves the clip, not a description. Stills narrow the choice;
   motion is judged only in motion.
5. **Save.** The style, its examples (made as library graphics, tagged with the style), its
   references and breakdown, its sounds. The style check runs: fonts resolve, example graphics
   render, sounds have licences.

**Tools:**
- `library` `kind: style` `op: propose`: `references` (files, URLs, library graphic ids),
  optional `brief`, `logo`; answers with the breakdown table and up to three previews. Only the
  pick is saved.
- `op: show` answers with the compact brief: settings, motion rules, kinds, look rules, examples.
- `op: set` takes the new parts; `op: check` gains fonts, examples and sounds.
- Graphics tools (`graphic_add`, `graphics`) point the agent at the style brief and its examples
  for the kind it is making, so no agent starts from a blank page or a stock template by default.

**UI:** the Style editor shows references, examples, sounds, motion rules and kinds as plain
lists the user can edit; no new window.

## Build order

1. **The style's new parts** (motion rules, kinds, examples, references, look rules, richer
   portrait, sounds) in `style.json`, `op: show`, `op: set`, `op: check` with the font rule.
2. **"My library" style.** Build the long-form style by hand with the agent from George's four
   library graphics (White Skill Hub/Focus, White Chat Reset, White Task Router): breakdown,
   motion rules, kinds, the graphics as examples. **Test:** an agent makes a new long-form
   graphic in this style that isn't in the library, and George would keep it.
3. **`op: propose`** (intake, breakdown, proposals as stills, the pick as a clip, save).
4. **Shorts variant** from references George brings: its own motion rules, kinds and examples.
   **Test:** the 25 Oct take again, graphics only from the style, compared with
   [graphics-quality.md](graphics-quality.md).
5. **Sound set** once the SFX starter set exists (SND-3).
6. Then [GFX-8](graphics-quality.md): picking what to show per moment, the sample clip, the
   checks (face, captions), the plan table.

## As built (step 1, 30 Oct)

- `style.json` gains `motion`, `kinds` (`name`, `when`, `shorts`: a short's graphic slot),
  `examples` (`graphic`: a library id or name, `kind`), `rules`, `references` (`file`, `note`),
  `breakdown` and `sound` (`policy`, `cues` of `role` + sound library id); `portrait` gains its
  own `motion`, `kinds` and `examples`. Empty parts stay out of the file. Limits: 12 lines or
  kinds or references or cues, 6 examples, 240 characters a line, 2,000 for the breakdown.
- Motion tokens are the look's existing tokens (`ease`, `gsap-ease`, `duration`, `stagger`,
  `out`, `distance`, `entry`); `motion` holds the words. No separate easing field.
- References are images (png, jpg, webp, gif, svg) or clips (mp4, mov, webm, mkv, up to
  300 MB) copied in as `reference-N.<ext>`; a save hard-links the style's own files into the
  new folder where the disk allows, so clips aren't copied on every edit.
- `op: show` answers with a text brief (colours, fonts and whether each is there, the look and
  every token, then motion, kinds, examples with their source path, look rules, sound,
  references, breakdown, the 9:16 part), then the fields as JSON for `set`. A style with no
  motion, kinds or examples says it is only a look.
- **Font rule:** a font is there when it is the style's file, or installed on this computer at
  each weight the style draws it at (the title at the look's title weight and the caption
  weight, the body at 400; within 100). `op: set` refuses a changed font that isn't there and
  names installed files with a name like it; `op: check` reports any font that isn't. The
  Style editor still saves any family (the user's own choice).
- `op: check` answers the style's own problems (fonts, examples missing from the library or
  saved as is, cue sounds missing, reference files gone) without a project; with one, also the
  graphics not in line as before. Example graphics are not rendered by the check.
- The graphics guide tells agents to read the brief before the first graphic.
- Not built: the Style editor's lists for the new parts (the editor keeps them as they are).
- `op: show` lists the new parts once, as text; its "Fields" JSON leaves them out (half the
  tokens for a filled style).

## As built (step 2, 30 Oct): "My library"

Saved through the dev app's `library` `set` (Light with `saveAs`) into the user's styles folder,
`%APPDATA%\AeroEdits\styles\My library\`; `op: check` reports no problems. Made from reading the
four graphics' source (White Skill Hub, White Skill Focus, White Chat Reset, White Task Router):
- **Look tuned to the examples:** speed 0.75 (entrances 933 ms, stagger 120 ms, as theirs run
  0.9–1.25 s), roundness 1.4 (radius 3.64vmin, their cards 3.6), energy 1.5 (travel 2.7vmin,
  theirs 2.4–3.5), title tracking −0.045em. Colours and system font as Light (already theirs).
- **Seven motion rules** (rise and settle, reading-order build, drawn connectors, the hub
  slide-in as signature move, state hand-over, one quiet hold, no frame fades or gimmicks),
  **five kinds** (hub, router, narrowing, state change, statement card; all full frame in a
  short), **the four graphics as examples** (statement card has none), **six look rules**, and
  the breakdown (what was taken, what was left out: their own scene fade, fixed SVG coordinates,
  the demo copy).
- No references (the examples are the source), no sound (step 5) and no 9:16 part (step 4).
- **The test, run 31 Oct.** George: "It's cool", not kept in the library; go on to step 3. A fresh headless Claude Opus 5.5
  (`claude -p`, only the AeroEdits tools and Read) on a scratch copy of the 29 Oct airdrops
  tutorial wearing My library, told only "one new graphic where it helps most, written
  yourself, in the project's style". 20 turns, $1.08, 7 min. It read the brief, then one
  example's source (the router), and wrote "TX Flow prize pool" (0:23.40–0:41.80, 18.4 s): three
  cards (no minimum, community volume, your volume) rising on their words, curved connectors
  drawing into a "Prize pool" box that hands over to "Up to 18,000 USDC" on "eighteen thousand".
  It wears the style (white cards, hairlines, tinted line icons, accent eyebrow, semibold
  headline, rise-and-settle, reading order) and is a router. Against the style: the headline
  ("More volume, bigger rewards.") is its own wording, which the look rules forbid (it flagged
  it); one card label wraps to two lines while the others don't; 18.4 s over the screen, which
  no rule limits. It reported its doubts honestly and placed times from the waveform.

## As built (step 3, 31 Oct): `op: propose`

- `library` `kind: style` `op: propose` with `references` (full paths of images or clips, or
  graphics library ids/names; at most 8) and an optional `brief` (the user's words). It
  saves nothing: it shows each reference to the agent and gives the next steps.
  - **An image** comes back as itself, at most 1280 px on its long side (FFmpeg reads png, jpg,
    webp, gif, bmp; an SVG is named for the agent to read as a file).
  - **A clip** comes back as one board: two frames a second up to 24 frames, spread evenly over
    a longer clip, rows of 6 (FFmpeg's `tile`; no time labels, the text gives the spacing).
  - **A library graphic** comes back as its gate contact sheet and its source path.
- The steps it gives: the breakdown table first, shown to the user; up to three proposals that
  differ in a named way, each a `set` patch previewed with `op: preview`; save only the pick
  (`set` with `saveAs`, `references` with notes, library graphics as `examples`, the
  `breakdown`), then `op: check`. Saving copies reference files in (step 1).
- `op: preview` on a video with no graphics now says to add one at a typical moment first,
  so proposals are always seen on the video's own moments.
- The graphics guide points agents at `op: propose` when the user brings references.
- **Not built here:** the pick as a clip in place (GFX-8 §2's sample clip, which this will
  reuse); a website URL as a reference (the app reaches no network; the agent can fetch one
  with its own tools and pass images); a separate `logo` argument (`set` takes
  `brand.logo` as a full path already).
- Not yet run by an agent.

## As built (step 5, 31 Oct): sound set

Step 4 (the shorts variant) is skipped until George brings references for it (31 Oct).
- A style's `sound` (step 1) is now shown in full in the brief: the policy, each cue's role,
  sound id, name and length (or "not in the sound library"), and how to place one (`library`
  `kind: sound` `op: use`, `name` = the id, `atWord` on the graphic's event; not every event).
  Licences stay with the sound library, where every sound carries one.
- **My library's set** (chosen without asking, from the CC0 starter set): whoosh `whoosh-soft`,
  click `click-soft`, pop `pop-pluck`, hit `hit-soft`; policy "quiet UI sounds under the voice
  … no risers, booms or punches". `check` clean.
- Placing cues on graphic events by themselves is [SND-2](../roadmap.md#snd-2); until then the
  agent places them by hand.
- **31 Oct, George on the first clip with the cues** (TX Flow prize pool in place): "it works,
  even though the effects are a bit bad". **Decided:** sounds on graphics are opt-in, added only
  when the user asks (or the preset's rules or choices say so); the guide, the edit rules and
  the brief say so. My library's set stays as the set to use when asked; better sounds are open.

## Not doing

- References on the preset (George, 25 Oct: they belong to the style).
- A demonstration-only default; text cards and demonstrations are equals.
- Copying examples into the style; they link to the library.
- A separate shorts style; shorts are a variant inside the same style.
