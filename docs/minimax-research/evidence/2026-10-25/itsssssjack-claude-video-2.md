# ItsssssJack/claude-video-2

- **Source:** https://github.com/ItsssssJack/claude-video-2
- **Kind:** github
- **Captured:** 2026-10-25T17:38:35
- **Evidence grade:** vendor or creator claim. Not tested in this repo. Quotes from a caption are what a creator *said*, not a measurement.

## Repository

- **Repo:** ItsssssJack/claude-video-2
- **Description:** Claude Video 2.0: make studio-quality motion graphics with Claude Code. Free skill with a gallery of 21 looks.
- **Stars:** 44
- **Forks:** 10
- **Open issues:** 0
- **Licence (SPDX):** MIT
- **Language:** HTML
- **Default branch:** main
- **Last push:** 2026-10-21T17:30:59Z
- **Archived:** False

## README

```
# Claude Video 2.0

A free skill for Claude Code that makes motion graphics that look like a studio made them.

- **Pick a look** from a gallery of 21 styles, each one playing as a real video.
- **Answer 5 short questions.**
- **Get your video.** Claude builds it in code, shows you 3 stills to approve, renders an MP4,
  and checks its own work from the real frames.

Pure graphics, or with your own photos and clips, or with AI clips from Higgsfield (it shows you the
plan and the credit cost first, and waits for your "yes"). Works on the normal Claude Pro plan.

## Install in 3 steps

1. Copy this folder to `~/.claude/skills/`, so you have `~/.claude/skills/claude-video-studio/SKILL.md`.
2. Only for AI clips: get a key at https://console.higgsfield.ai and add one line to a file called `.env`
   in your project folder: `HIGGSFIELD_API_KEY=your_key_id:your_key_secret`. Never paste a key into the chat.
3. Open Claude Code and say: **make me a motion graphic**

You also need Node 22 or newer (https://nodejs.org) and ffmpeg (`brew install ffmpeg`).
Claude checks for you and tells you the one command to fix anything missing.

## How it works

**A. Pick a look.** A gallery opens in your browser. Every tile is a real video. Flip a studio piece to
"With media" to see the same design with real footage inside it. Press **Use this style** and paste
the name into Claude Code. Or paste links or images of your own reference.

**B. Five questions**, in one message, each with a default:

1. What is it for? (YouTube intro, Short, launch, explainer, ad) and how long? [12 s, 16:9]
2. What should it say? (the words on screen) [I'll write it from your topic]
3. Does it loop? [no]
4. Your brand: colours, fonts, logo? [use the style's]
5. Real media? Your photos or clips, AI clips from Higgsfield, or none (pure graphics) [none]

The questions live in `questions.md`, so you can change them.

**C. Build.** A storyboard (shots, timings, assets, and the credit cost if there are AI clips),
3 stills for you to approve, the render, then a pacing check and, for loops, a pixel loop check.

## The 21 looks

Studio pieces: `paper-world`, `ink-line`, `weather-widget`, `data-story`, `swiss-kinetic`,
`particle-field`, `growing-city`, `explainer-diagram`, `time-travel`, `editor-ui`, `motion-app-ui`,
`design-app-ui`, `audio-app-ui`.

Code styles: `swiss-grid`, `editorial-paper`, `midnight-grid`, `paper-cut`, `risograph`, `blueprint`,
`graphite-ui`, `poster-pop`.

Each one is a folder in `styles/` with a `STYLE.md` (colours, fonts, easing numbers, motion rules, a ban list)
and, for most, a full working piece to start from. To see the gallery any time:
`python3 ~/.claude/skills/claude-video-studio/scripts/gallery.py`

## Why it looks good: the 5 hacks

1. Pick one style first and stick to it.
2. Real references beat adjectives.
3. Write the brief in numbers.
4. Loops: one clock, and the last frame must equal the first.
5. Check frames, not code.

`references/hacks.md` explains each one and where it comes from.

## What is inside

- `SKILL.md` the steps Claude follows
- `gallery/` the style gallery and its videos
- `styles/` the 21 looks
- `questions.md` the questions (edit freely) and `config.json` (an optional link Claude mentions at the end)
- `references/` the 5 hacks, motion numbers, engine rules, Media mode, how to write a style
- `templates/` the starter page and the one-clock helper
- `scripts/` gallery, render, stills, pacing check, loop check, storyboard, Higgsfield
- `examples/` a small brief and a storyboard

Made with [HyperFrames](https://hyperframes.heygen.com) and [GSAP](https://gsap.com). All fonts are free
(Google Fonts, and Switzer loaded from Fontshare at render time). MIT licence.
```

## Licence file

```
MIT License

Copyright (c) 2026 Claude Video Studio contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

> The repo's licence rule allows MIT, Apache-2.0, GPL-3.0 (as a separate component) and OpenRAIL. Non-commercial, research-only and revenue-capped licences are refused. Read the text, not the SPDX tag alone.

## File tree (111 files, filtered)

- `LICENSE`
- `NOTION-PAGE.md`
- `README.md`
- `SKILL.md`
- `config.json`
- `examples/loop-brief.md`
- `gallery/index.html`
- `gallery/looks.js`
- `questions.md`
- `references/hacks.md`
- `references/hyperframes.md`
- `references/loops.md`
- `references/media.md`
- `references/motion-craft.md`
- `references/style-format.md`
- `scripts/_media.py`
- `scripts/gallery.py`
- `scripts/higgsfield.py`
- `scripts/loop_check.py`
- `scripts/pacing_check.py`
- `scripts/storyboard.py`
- `styles/audio-app-ui/STYLE.md`
- `styles/blueprint/STYLE.md`
- `styles/blueprint/index.html`
- `styles/blueprint/studio.js`
- `styles/data-story/STYLE.md`
- `styles/data-story/index.html`
- `styles/design-app-ui/STYLE.md`
- `styles/editor-ui/STYLE.md`
- `styles/editorial-paper/STYLE.md`
- `styles/editorial-paper/index.html`
- `styles/editorial-paper/studio.js`
- `styles/explainer-diagram/STYLE.md`
- `styles/explainer-diagram/index.html`
- `styles/graphite-ui/STYLE.md`
- `styles/graphite-ui/index.html`
- `styles/graphite-ui/studio.js`
- `styles/growing-city/STYLE.md`
- `styles/growing-city/index.html`
- `styles/ink-line/STYLE.md`
- `styles/ink-line/index.html`
- `styles/midnight-grid/STYLE.md`
- `styles/midnight-grid/index.html`
- `styles/midnight-grid/studio.js`
- `styles/motion-app-ui/STYLE.md`
- `styles/paper-cut/STYLE.md`
- `styles/paper-cut/index.html`
- `styles/paper-cut/studio.js`
- `styles/paper-world/STYLE.md`
- `styles/paper-world/index.html`
- `styles/particle-field/STYLE.md`
- `styles/particle-field/index.html`
- `styles/poster-pop/STYLE.md`
- `styles/poster-pop/index.html`
- `styles/poster-pop/studio.js`
- `styles/risograph/STYLE.md`
- `styles/risograph/index.html`
- `styles/risograph/studio.js`
- `styles/swiss-grid/STYLE.md`
- `styles/swiss-grid/index.html`
- `styles/swiss-grid/studio.js`
- `styles/swiss-kinetic/STYLE.md`
- `styles/swiss-kinetic/index.html`
- `styles/time-travel/STYLE.md`
- `styles/time-travel/index.html`
- `styles/weather-widget/STYLE.md`
- `styles/weather-widget/index.html`
- `templates/motion/index.html`
- `templates/motion/studio.js`
