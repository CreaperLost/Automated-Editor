# Presets and styles

Roadmap item: [ORG-1](../roadmap.md#org-1). Decided with George on 23 Oct 2026.

## Why

The app grew eight kinds of saved thing, and three of them were called "template": brand kits,
the graphics look, saved looks, the Templates menu, favourite layouts, the graphics library,
the agent's workflow templates (`tutorial`, `shorts`, `clips`) with their saved answers, and the
planned style kit (STY-1) and rules file (AGT-3). "My tutorial setup" was spread across four of
them, and neither George nor an agent could tell where a setting belongs.

## The rule

Every setting has one home, found by one question:

| Question | Home |
|---|---|
| How does it look? | **Style** |
| What does the app do, and when? | **Preset** |
| Where do things sit on screen? | **Layout** |
| Only this video? | **Project** |

**Decided (George, 23 Oct):** the names are **Preset** and **Style**. Caption position, words
per caption and lines per caption are behaviour, so they are in the **Preset**; how captions
look (font weight, pop / karaoke / static, emphasis colour) is in the Style. The word
"template" stops being a user-facing word for any of these; it is left only for the graphic
templates graphics are built from.

## The four things

### Style: the identity

What every graphic, caption and frame looks like. Kept app-wide, one folder per style:
`<config>/AeroEdits/styles/<name>/style.json`, with its font and logo files beside it.

- **Brand:** colour roles (background, text, accent, muted), fonts (file or installed family,
  title and body), logo.
- **Graphics:** today's look: preset base (app / calm / bold), size, roundness, speed, energy,
  density, hand-set tokens.
- **Captions look:** weight, style (pop / karaoke / static), emphasis on or off and its colour,
  text colour, outline, background box.
- **Frame look:** canvas background (solid / gradient / preset; a wallpaper image stays the
  project's file), screen corner radius and shadow, camera border colour, width and shadow.
- **Variants by canvas shape:** what changes at 9:16 (today's `Look.shorts`: bigger type, more
  energy). A short takes its style's 9:16 variant.
- **References (later, STY-2):** the images or video boards the style was made from, kept with
  it, so an agent can study them again.

A project picks one style. A graphic may pick another style (today it may pick another brand
kit) and may still override tokens of its own.

### Preset: the kind of video

How the app and the agent edit one kind of video. App-wide, one file each:
`<config>/AeroEdits/presets/<name>.json`, plus its method (below). Built in: **Tutorial**,
**Shorts**, **Clips**; **Gaming** later, as a new file with no code change.

- **Canvas:** shape (16:9, 9:16, ...), screen size, crop defaults.
- **Cleanup:** silence preset and padding, filler and retake handling, mouth-noise
  sensitivity, audio (normalise, target loudness, noise reduction, mouth clicks).
- **Captions behaviour:** on or off, position, offset, size, words per caption, lines,
  capitals.
- **Zooms:** today's zoom settings (scales, timing, how many, follow).
- **Layouts:** the base layout, which layouts may be used, and how they are picked (today's
  pick settings).
- **Screen:** cursor shown and size, shortcut badges, camera auto-hide, face framing.
- **Shorts:** the short's own caption spot and size, screen zoom and follow (today's
  `ShortLayout` minus its per-clip framing and graphic places, which stay the short's).
- **Export:** resolution, frame rate, quality, which other files to save (DLV-1).
- **Style:** the default style for this kind of video.
- **Method:** the agent's steps for this kind of video (today's `tutorial.md`, `shorts.md`,
  `clips.md`) and its **rules** (AGT-3's rules file becomes this section). What the user
  answered to the method's questions is saved into the preset's values, not a separate
  answers file, so the app and the agent read the same settings.

### Layout: the building blocks

The screen and camera arrangements: built in plus the user's favourites (today's
`layouts.json`), unchanged. A preset says which may be used and how they are picked; the style
dresses them (corners, border, shadow).

### Project: one video

Picks a preset for the video, a preset for its shorts (default Shorts) and a style. Holds only
what differs from them, plus what is only its own: media, transcripts, the edit, chapters,
shorts, graphics, the wallpaper file, hand framing, history and checkpoints.

## How a value is found

```
built-in default  ->  preset  ->  project change      (behaviour)
built-in style    ->  style   ->  project change      (looks)
```

The project stores changes only. The UI marks every changed value ("changed here" with a
reset; unmarked means "from Tutorial preset" / "from Light style"), and offers **Save changes
to the preset** / **Save as a new preset** (the same for styles). Changing a preset or style
later changes every project that uses it and hasn't overridden that value; a project that
wants to stay as it is can copy the preset into its own changes (one action).

## Agents

- One `library` tool replaces `brand_kits`, `brand_kit_set`, `brand_pick`, `graphics_look`,
  `template_save` and the saved-looks calls: `kind` (style / preset / layout), `op` (list,
  show, set, save from project, remove, use in this project). Fewer tools, fewer tokens
  (COST-2).
- `template` becomes `preset`: it serves the preset's method, its rules and its current values
  in one answer, instead of a skill file plus a separate answers file.
- `status` names the project's preset and style and how many values it changed.
- Everything is readable JSON and Markdown, so an agent (or George) can open, diff and share a
  preset or style as files.

## What it replaces

| Today | Becomes |
|---|---|
| Brand kit (`brands/`), graphics look, saved looks (`looks/`), caption look settings, frame look in `EditLayout` | **Style** |
| Templates menu (`templates.json`), agent workflow templates and their saved answers, AGT-3 rules file, cleanup/zoom/caption/export behaviour settings | **Preset** |
| Favourite layouts | **Layouts** (unchanged) |
| Project, checkpoints, history | **Project** (unchanged, now holds changes only) |
| STY-1 style kit | Folded into Style (looks) and Preset (rules) |
| UI-3 saved-settings work | Folded in: the Templates menu goes away |

No conversion: old `brands/`, `looks/`, `templates.json` and saved answers are dropped (George's
config holds none worth keeping, checked 23 Oct), and old projects are not read (see the
no-migration rule).

## Phases

Each phase ends usable, with tests, a check in the dev app, and EVAL-1 rerun when agents are
touched.

1. **Style.** The `Style` type and its folder; brand kit, look, caption look and frame look move
   into it; a bundled **Light** style (GFX-5's app-like look) and a dark one; the project picks
   one; graphics, captions and the frame read from it; a Style picker and editor in the UI;
   9:16 variant. Brand-kit and look UI and code removed.
2. **Preset values and inheritance.** The `Preset` type and folder; built-in Tutorial, Shorts and
   Clips; the project stores only its changes; "changed here" markers and reset in every
   settings panel; Save to preset / Save as new; a Preset picker in the top bar. The Templates
   menu and `templates.json` removed.
3. **Method in the preset.** The agent's steps and rules move into presets; `template` becomes
   `preset`; saved answers become preset values; the user can read and edit a preset's method
   and rules in the app (Markdown).
4. **One library tool and cleanup.** `library` replaces the old agent tools; skill text, the
   architecture doc and the roadmap updated; EVAL-1 rerun to show cost and quality hold.
5. **Later, on top:** styles from references (STY-2) write a Style; AI-proposed styles (GFX-5's
   left part) offer two or three; a Gaming preset (milestone 3); "save this workflow" (AUTO-3)
   saves a preset from an approved edit; LOOP-1 proposes changes to a preset's rules.

## Open questions (decide in the phase that needs them)

- Does a preset's method stay one Markdown file per preset, or a few (steps, rules, questions)?
- Sound later (SND-*): sound *style* (which whooshes, how loud) is Style; sound *policy* (when
  cues play, ducking) is Preset. Confirm when SND-1 starts.
- How the built-in presets and styles are updated when the app updates, if the user changed
  them: probably built-ins are read-only and "edit" makes a copy.

## As built (23 Oct 2026)

All four phases were built in one go while George was away; he asked for the decisions below to
be listed so he can change them. Each says what it means and what else could have been done.

### Style

1. **The JSON shape** is the one George approved: `brand` (colours: background, text, accent,
   **muted**; fonts: **title** and **body**, each a family and an optional file; logo),
   `graphics` (the look), `captions`, `frame`, `portrait`. Every part is optional in a file and
   falls back to Light's. *Alternative:* a flat list of tokens.
2. **Muted** is a real colour now, set on graphics as `--look-muted` unless the graphic set its
   own text or background colour (then the look's mix stays, so muted text stays readable on a
   graphic's own colours). *Alternative:* keep muted mixed from text and background, dropping
   the role.
3. **Body font**: body text uses it, titles (`.title`, `.count`, `h1`, `h2`) keep the title font;
   captions use the title font. Empty means "same as titles". *Alternative:* captions get a
   font of their own.
4. **Built-in styles are read-only.** In the editor, the first change to Light makes
   **"My light"** and dresses the project in it (a notice says so); agents get the same through
   `library` (no `saveAs` gives "My light"). *Alternatives:* edit built-ins in place (an app
   update would then have to merge), or refuse and ask for a name every time.
5. **Light** is today's look exactly (GFX-5 graphics, the Forest background, indigo camera
   border, white captions with outline, pop at 9:16); **Dark** is near-black and off-white with
   a blue accent, the Graphite background, a soft screen shadow and a white 4 px camera border.
   *Alternative:* Light with a light background (it is "the app-like look" for graphics; the
   frame kept today's dark Forest so nothing changes on screen).
6. **Editing a style saves as you go** (debounced, like the other inspector settings) instead
   of Save / Cancel, so the preview shows each change. A style change is **not an undo step**
   (it is app-wide); picking another style is. *Alternative:* a draft with Save, or style edits
   in the project's history.
7. **Where the frame and caption look live at draw time:** the old fields stay on
   `EditLayout` / `CaptionSettings` as never-stored fields, filled from the style by
   `style::dress` when a document is drawn (every preview and export goes through
   `SceneEvaluator::new_reusing`). This kept the renderer untouched. *Alternative:* pass a
   `Frame` through every render function (cleaner types, a much bigger change).
8. **A project's wallpaper image** stays the project's (Video page, "Background image"); when
   set it wins over the style's background, and "Use the style's" drops it.
9. **9:16** is any short and any 9:16 canvas. Its caption style defaults to **pop** (the old
   per-short default) and it may have its own background. A short's own background and caption
   style are gone (the style decides). *Alternative:* keep a per-short override.
10. **The style editor is a page of the Inspector** (Video, Audio, Captions, **Style**), not the
    Graphics tab, since it dresses captions and the frame too. A graphic's own style and "take
    the style's look" are on its Clip page.
11. A missing style falls back to **Light** silently; renamed styles are followed through
    `styles/renamed.json`, as brand kits were.

### Preset

12. **What a preset holds:** the edit document's parts `layout` (canvas, screen size and crop,
    cursor, badges, window crop, face framing, mirror, auto-hide), `captions` (on, position,
    offset, size, words, lines, capitals), `zoomSettings`, `audio`, `layouts` (base and
    picking); `short` (what a new short starts with); `choices` (the method's answers: cleanup
    passes, review depth, graphics, export); `style` (for a new project); `method.md` and
    `rules.md`. **Export settings and the app-wide cleanup levels are not in it yet** (export
    takes the choices' `export`). *Alternative:* move the export dialog's defaults and silence
    levels in too (a later step).
13. **How "only the changes" works:** in memory a project still has every value; on disk
    (`project.json` and history states) the preset's parts are written as `presetChanges`, a
    JSON merge patch over the preset's values, and read back over the preset as it is now. So
    every command and the renderer are unchanged. A change to a preset reaches each project the
    next time it is opened; the open project follows at once (one undo step). *Alternative:*
    every setting read through a resolver (preset, then project) at use time.
14. **What is always the project's own** is never counted as "changed" nor saved into a preset:
    the wallpaper file, hand-framed camera clips, window-crop stretches, the captioned track,
    layout sections and per-track mixes.
15. **"Changed here"** shows as chips at the top of each settings section (Video, Captions,
    Audio, Zoom), one per changed value, each a reset, plus "all back". *Alternative:* a dot
    beside every control (more code in every control).
16. **The Preset menu** replaces Templates in the top bar: follow another preset (what you
    changed stays changed), what new shorts start from, save changes to the preset (not for a
    built-in), save as a new preset (the project then follows it), take every change back,
    method and rules, presets folder, remove. Following another preset also switches the style
    when the project still wore the old preset's default style.
17. **The project picks two presets:** its own and one for its new shorts (default Shorts). The
    shorts preset's `short` values are what the first short starts with (later shorts copy the
    first, as before).
18. **Built-in presets are read-only too:** saving a method into one asks for a copy's name;
    an agent saving choices into one makes "My tutorial" and the project follows it.
19. **Presets are not renamed** (no rename in the UI; a removed or missing preset falls back to
    Tutorial). *Alternative:* renames followed like styles.

### Method, rules and agents

20. **Open question answered:** a preset's method is **two Markdown files**: `method.md` (steps
    and questions) and `rules.md` (the user's own rules, which win over the method). The shipped
    app rules (`mcp/skill/rules.md`) stay the app's. The built-in methods moved to
    `src-tauri/src/preset/methods/` and start with `# Preset: Tutorial` and so on.
21. **Saved answers are the preset's `choices`.** The old `agents/answers/*.json` and
    `templates.json` in George's config are not read and were **left on disk** (not deleted).
22. **`template` is `preset`** (same arguments); `template_save` is gone (choices are saved with
    `library` `kind: preset` `op: set`). MCP prompts are the presets (names like `tutorial`,
    `my-gaming`).
23. **One `library` tool** in the main tool list (not behind `more`): `kind` style / preset /
    layout, `op` list, show, use, set, remove, plus preview / check / bring_in_line (styles) and
    save / reset (presets). Layouts only list, show and remove there; saving and using a layout
    stays in `layouts`. A graphic's own style and tokens moved to `graphic_set` (`style`,
    `look`). It is marked destructive (it writes app-wide files).
24. **`settings`** refuses look fields (pointing at the style), and its silence-levels area is
    renamed from `presets` to **`cleanup`** so "preset" means one thing.
25. **`status`** adds one line: the preset (and how many values were changed here), the shorts'
    preset and the style.
26. **Tests write no user files:** styles and presets go to a temporary folder in test builds.

### Not done

- **EVAL-1 was not rerun** (the agents' tools and method text changed).
- A real edit with a style of George's own (font and logo files) checked in an export.
- "Changed here" for the Layouts section and a short's settings.
