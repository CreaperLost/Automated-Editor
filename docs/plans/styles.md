# Styles from references (STY-2)

**Status:** plan, 25 Oct 2026. Nothing built. Roadmap: [STY-2](../roadmap.md#sty-2), done
before [GFX-8](../roadmap.md#gfx-8) (graphics an agent makes well,
[plans/graphics-quality.md](graphics-quality.md)).

This merges three pieces that turned out to be one problem:
- GFX-5's leftover: styles the agent proposes (research in
  [research/ai-styles.md](../research/ai-styles.md)).
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
   with its licence stored, from the SFX starter set ([SND-3](../roadmap.md#snd-3)) or George's
   own files. Graphic events cue them ([SND-2](../roadmap.md#snd-2)).

**Where it lives:** everything the app uses stays in `style.json` (motion rules, kinds, look rules
and the breakdown as short text fields); references and sounds as files in the style's folder.
One file, no second Markdown copy to drift. `library` `kind: style` `op: show` answers with a
compact readable brief, which is what an agent reads before any graphic.

## Making a style from references

1. **Intake.** The user gives references: images, clips, a website URL, a logo, library graphics
   ("my library"), optionally a few words. Clips are read as frame boards, not whole videos
   ([COST-1](../roadmap.md#cost-1)). If there are no references, the agent asks for one, but can
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

## Not doing

- References on the preset (George, 25 Oct: they belong to the style).
- A demonstration-only default; text cards and demonstrations are equals.
- Copying examples into the style; they link to the library.
- A separate shorts style; shorts are a variant inside the same style.
