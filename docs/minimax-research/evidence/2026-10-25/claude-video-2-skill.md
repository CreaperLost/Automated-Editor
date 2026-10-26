# Claude Video 2.0 — the `claude-video-studio` skill (Jack Roberts)

- **Source:** [github.com/ItsssssJack/claude-video-2](https://github.com/ItsssssJack/claude-video-2) ·
  also published as a Notion page,
  `https://app.notion.com/p/Motion-Video-free-Claude-skill-400e8d6bd137810897c2ea505cbf29b8`
  (the repo carries the page's source as `NOTION-PAGE.md`)
- **Video:** [youtu.be/KphrV76DJoI](https://youtu.be/KphrV76DJoI) — *Claude Video 2.0 Just Changed
  Everything (FREE Skill)* · **Jack Roberts** · 24 Oct 2026 · 13:00 · 35,518 views · 645 likes
- **Machine captures:** [the repo](itsssssjack-claude-video-2.md) (README, licence, tree) ·
  [the Notion page](notion.md) ·
  [the video](claude-video-2-0-just-changed-everything-free-skill.md) ·
  [Motion Lab](novusedge-motion-lab.md) (the cited origin) ·
  [Artimind](how-to-make-motion-graphics-with-claude-opus-5-5-without-the.md) (the cited origin)
- **Captured:** 2026-10-25T17:38Z — 111 files, README + licence + tree via the GitHub API; the
  documents below via `raw.githubusercontent.com`
- **Evidence grade:** **a published artefact, read in full.** **MIT** — the licence text was read,
  not the SPDX tag. **Not installed and not run.** No render was made and no check was executed.

**What it is.** A Claude Code skill that turns Claude into a motion-graphics studio: a gallery of
**21 looks**, five questions, a brief in numbers, a storyboard, **3 stills to approve**, a render,
and **two automatic pixel checks** — a pacing check and a loop check. Built on **HyperFrames**
(Apache-2.0) and GSAP.

## The tree (111 files, filtered)

```
LICENSE  MIT                    README.md
NOTION-PAGE.md                   the Notion page's own source, with a "[REPO LINK GOES HERE]" placeholder
SKILL.md                         the steps Claude follows
config.json  questions.md        the 5 questions (editable) and an optional closing link
gallery/      index.html, looks.js, videos/
styles/       21 looks, each STYLE.md (+ index.html, studio.js for most)
references/   hacks.md · style-format.md · motion-craft.md · hyperframes.md · loops.md · media.md
scripts/      doctor.sh · gallery.py · new_project.sh · stills.sh · render.sh ·
              pacing_check.py · loop_check.py · storyboard.py · higgsfield.py · _media.py
templates/    motion/ (index.html, studio.js — the one loop clock)
examples/     loop-brief.md, storyboard.json
```

## SKILL.md — the workflow, verbatim extracts

> **Three stages: A. pick a look (gallery) → B. five questions → C. build (storyboard, stills,
> render, checks).** Every step follows the 5 hacks in `references/hacks.md`: one style, real
> references, a brief in numbers, one loop clock, and checking frames, not code.

> **Stage A — pick a look.** … A style id (like `paper-world`): read `$SKILL/styles/<id>/STYLE.md`.
> **That spec is now law for this piece.** Their own references: look at each one (open links, read
> images). Write a new spec in the work folder, `STYLE.md`, in the format of
> `$SKILL/references/style-format.md`. **Describe HOW each reference was made (masked text, slow
> camera, long holds)**, with hex colours and easing numbers. Show it in 5 lines.

> **Stage C, step 1 — brief in numbers.** Write `BRIEF.md` in the work folder: size (1920 x 1080 for
> 16:9, 1080 x 1920 for 9:16), length, 30 fps, loop or not, the exact words, palette in hex (brand
> colours replace the style's, **its rules stay**), fonts with their links, easing numbers, and the
> always-on ban list: no hue-shifting gradients, no glow, bloom or lens flares, no linear easing on
> anything the eye follows, no centred-everything, nothing scaling up from zero, at most 6 words on
> screen, one accent colour, no fake chrome. **Where the chosen style's own signature conflicts with
> this list (a sun's glow, a centred widget), the style wins: its gallery video is the proof.**
> Everything else on the list still applies.

> **Stage C, step 2 — storyboard.** Write `storyboard.json` … If any shot is `generate` (a
> Higgsfield AI clip), the table shows the credit estimate. **STOP and wait for a clear "yes" before
> spending anything.** Then make each clip … **and look at 3 frames of each before using it.**

> **Stage C, step 3 — build.** … One clock for all motion (hack 4): `STUDIO.play(t => render(t), { loop })`
> from `studio.js`, or one paused GSAP timeline. **Same time in, same picture out: no Math.random
> (use `STUDIO.rng`), no Date.now, no timers.** Fonts: only free ones, each loaded with a `<link>`;
> **if any font has a link, link every font.** Media goes inside the design (masks, cards, shapes),
> **never under unreadable text, never over a face.** Copy their photos and clips into
> `<work>/proj/media/`. **Never move or change their originals.**

> **Stage C, step 4 — 3 stills to approve (hack 5).** `stills.sh <work>/proj <t1>,<t2>,<t3>`, each
> about 0.6 s after a shot change, once things have landed. Read `stills/peek.jpg`. **Score
> composition, motion, type, colour and idea from 1 to 10 like a harsh senior motion designer. Fix
> the worst 3 things; repeat until all are 8 or more.** Then show the person the 3 stills and the
> scores and ask: "Good to render?" Wait for a yes.

> **Stage C, step 6 — checks.** `pacing_check.py <work>/out/draft.mp4` (add `--loop` if it loops;
> **no dead gap over 1 s, nothing on screen under 0.3 s**) and, if it loops, `loop_check.py` (**frame
> at 0 s must equal the frame at the end**). Read the contact sheet. **Fix and re-render until both
> pass.**

> **Stage C, step 8 — hand over.** Give the path, length, frame size, file size, check results, the
> scores, and one line on what you would try next. If the file is over 20 MB, also make a smaller
> copy for sharing.

> ## Hard rules
> - Never ask for, print, log or commit an API key. Keys live in the shell or a `.env` file only.
> - Never spend credits without showing the storyboard with its cost and getting a clear "yes".
> - **Never cover a face with text or graphics.**
> - **Never say a check passed without running it. Never say a look is right without looking at a frame.**
> - One style per piece. One accent colour. The ban list always applies.
> - Only free fonts. Never copy font files into a project.

> ## Before you start
> Run `bash $SKILL/scripts/doctor.sh` once. **If something is missing, give the person the one fix
> command and stop.** … Make a new work folder for each piece … **Never build inside `$SKILL`.**

## references/hacks.md — the derivation record, verbatim (each hack cites its source)

> ## 1. Pick one style first, and stick to it
> … A style is a short spec: colours in hex, fonts, easing numbers, motion rules and a ban list.
> Every shot in the piece uses that one spec. No mixing.
> **Why: when you leave the look open, every model falls back to the same default look (dark
> background, glow, purple gradient).** A fixed spec stops the drift between shots.
> Sources: Motion Lab says to ban the default AI look up front: https://github.com/NovusEdge/motion-lab
> · "Lock design tokens" in one shared place: https://aiartimind.com/…
>
> ## 2. Real references beat adjectives
> "Make it sleek and modern" means nothing. A real frame from a real film means a lot. Ask for 1 to 3
> references, then **describe HOW they were made (masked text, slow camera, long holds), not just
> how they feel.**
> Where to find good ones: https://savee.com · https://godly.website · https://mobbin.com · launch
> films from Linear, Apple and Cursor.
> Sources: a frame-by-frame look at the Linear Agent launch film (masked reveals, long holds,
> fast-start easing): https://impractical.ai/launch-videos/linear-agent
>
> ## 3. Write the brief in numbers
> … colours in hex · **easing as names and seconds (expo.out over 0.6 s), never "smooth"** · one hero
> move per shot, with its start time · a ban list …
> Sources: Motion Lab's numeric brief … · bans for shifting gradients, glow and linear easing …
>
> ## 4. Loops: one clock, last frame = first frame
> … **Prove it: `scripts/loop_check.py` renders the frame at 0 s and the frame at the end and diffs
> the pixels. It fails if they differ.** It also checks that the jump from the last frame back to
> the first is no bigger than a normal frame step.
> Sources: loop-phase helpers … **HyperFrames renders a paused timeline frame by frame, so the same
> time always gives the same picture.**
>
> ## 5. Check frames, not code
> Code that looks right can still look wrong. So look at pictures, at the moments that matter:
> 1. Before the full render, grab 3 or 4 stills, each about 0.6 s after a shot change …
> 2. After the render, run the pacing check: **no dead gap longer than 1 s, and nothing on screen for
>    less than 0.3 s.**
> 3. Look at the contact sheet it writes (one frame per second).

## references/style-format.md — the STYLE.md format, verbatim

> ```
> # <Display name>
> id: <id>
> Best for: <3 to 6 kinds of video>
> Mood: <one sentence>
> Reference piece: styles/<id>/index.html (<length> s, <loops or not>)   or   none, spec only
> Gallery video: gallery/videos/<id>.mp4
>
> ## Palette (hex)
> - ground: #......
> - ink: #......
> - ... (name every colour; mark the ONE accent)
>
> ## Type
> - Display: <font> <weight>, tracking <em>, line height <n>
> - Labels: <font> <weight>, <px>
> - Font links (paste into <head>; all fonts are free):
>   <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=…">
>
> ## Easing (numbers)
> - land: <ease>, <s>
> - leave: <ease>, <s>
> - move: <ease>, <s>
> - stagger: <s>
>
> ## Motion rules
> - <4 to 8 rules that make the motion feel like this look>
>
> ## Signature moves
> - <the 2 to 4 moves that make it recognisable>
>
> ## Composition
> - <grid, margins, where the hero sits, how full the frame is>
>
> ## With media
> - <how real photos or clips sit in this look: masks, grading, framing>
>
> ## Ban list
> - <things that break this look>
> ```
>
> Keep it under about 80 lines.
>
> ## Font rules (all styles)
> - Only free fonts: Google Fonts, or Fontshare loaded from its own link at render time.
> - **If you add any font `<link>`, link EVERY font the piece uses (HyperFrames then skips its own
>   font loading).** Never ship font files. Never use paid fonts.

## references/motion-craft.md — the numbers, verbatim

> ## Timing
> - A normal move: **8 to 18 frames (0.27 to 0.6 s). Median about 10 frames.**
> - A big camera move: 0.8 to 1.2 s.
> - Stagger between items: 2 to 3 frames (0.05 to 0.1 s). Keep a group within 6 frames so it reads
>   as one gesture.
> - **Hold a finished state at least 0.6 s.** Hold words long enough to read: about 0.3 s per word,
>   1.2 s minimum for a line.
> - **Something visible should change at least every 1 s. In the first 3 s, every 0.5 to 1 s.**
> - **Nothing should be on screen for less than 0.3 s.**
> - A flash … is ONE onset, then holds or fades over 0.3 s or more. **No flicker: two flashes 0.17 s
>   apart read as a glitch, and the pacing check fails them.**
> - Music-ready: put the big hits on a 0.5 s grid (120 BPM).
>
> ## Easing
> - Landing: power3.out or expo.out. · Leaving: power2.in or power3.in, **shorter than the landing.**
> - Moving from A to B: power3.inOut or expo.inOut. · Ambient drift: sine.inOut.
> - **Never `none`/linear on something the eye follows.** Linear is only for the master clock.
> - Overshoot: at most 4% (back.out(1.2)). No elastic unless the style asks for play.
>
> ## Composition
> - Fill the frame. Big confident shapes. No small card floating in empty space.
> - Safe margin: 80 to 120 px on a 1920 x 1080 frame. · 8 px spacing grid, 32 px or more between
>   separate things. · **At most 6 words on screen at once.** Words are labels, not sentences.
> - Hang text off a margin. Centre only when the style says so.
>
> ## Polish that separates good from great
> - Secondary motion: follow-through, a small settle, a shadow that tightens as a thing lands.
> - Parallax: near layers move faster than far ones.
> - **Masked reveals for text (slide up from behind an edge), not soft fades.**
> - Cut on motion: start the next shot while something is still moving.
> - One thing at a time gets the accent colour.
>
> ## Self-review (do it every round)
> **Score the stills 1 to 10 on: composition, motion, type, colour, idea. Fix the worst 3 things.
> Repeat until every score is 8 or more. Say the scores out loud to the person.**

## references/hyperframes.md — the engine contract, verbatim

> HyperFrames turns an HTML page with a paused GSAP timeline into an MP4, frame by frame. Docs:
> https://hyperframes.heygen.com , or in a terminal: `npx hyperframes docs gsap`. The scripts pin
> version 0.8.181 (`HF_VERSION` changes it). Needs Node 22+ and ffmpeg.
>
> ## The contract
> - One root element: `<div id="root" data-composition-id="main" data-start="0" data-duration="9"
>   data-width="1920" data-height="1080">`.
> - One paused timeline registered as `window.__timelines["main"] = tl`.
> - Timed media gets `class="clip"` plus `data-start` and `data-duration`. Video: `<video class="clip"
>   src="media/x.mp4" muted playsinline data-start="2" data-duration="4" data-media-start="0.5">`.
> - Render: `npx hyperframes render <dir> -o out.mp4 -q draft` (fast) or `-q high` (final).
> - Stills: `npx hyperframes snapshot <dir> --at 0,3,6 --no-end -o stills/`.
> - Lint: `npx hyperframes lint <dir>`.
>
> ## Must-follow rules (or renders flicker or differ)
> - **Same time in, same picture out.** No `Math.random` (use `STUDIO.rng(seed)`), no `Date.now`, no
>   timers, no `requestAnimationFrame` loops.
> - Text that changes over time changes inside the render function or a tween's onUpdate, never with
>   gsap.set on innerHTML.
> - Canvas: draw the whole frame from t in render(t).
> - Load media from the project folder (`media/`). Keep file names simple.
>
> ## Fonts
> - Use only free fonts … Never copy font files into a project. Never use paid fonts.
> - Load each font with a `<link>` in `<head>`. **If you add ANY font link, link EVERY font the page
>   uses: HyperFrames then skips its own font loading, and an unlinked font falls back to Times.**
> - **System font names are swapped by HyperFrames: "Helvetica Neue", "Avenir", "SF Pro" become
>   Inter and "Futura" becomes Montserrat.** Pick a Google Font on purpose instead.
> - Text measured in code (canvas, offsetWidth) must be measured again after fonts load
>   (`document.fonts.ready.then(redraw)`).
> - **Rendering needs internet for GSAP and fonts (both load from public CDNs).**
>
> ## Formats
> - 16:9 is 1920 x 1080. 9:16 is 1080 x 1920. **Lay the scene out again for each; never just crop.**
> - Transparent overlay for editors: `--format mov` with a transparent background.

## references/loops.md — verbatim

> 1. **No entrance.** **Frame 0 must be a finished, good-looking frame, not an empty page that things
>    fly into.** Move the build-in later, or start the clock in the middle of a hold.
> 2. **One clock.** Draw everything from `t` …
> 3. **Whole cycles.** Every wave, sway, bob, rotation or drift must finish a whole number of cycles
>    over the length … Find the offenders with `grep -n "t \*\|Math.sin(t\|Math.cos(t" index.html`.
>    A drift like `x = t * speed` becomes a sway or a wrap.
> 4. **Out and back.** Every one-off change … must return to its start value by the end.
> 5. **Wrap the noise.** Grain, flicker or particle frames that use a frame index must wrap:
>    `frame % totalFrames`.
> 6. **Reset while hidden.** If something must jump back, do it while it is off screen or covered.
> 7. **Keep moving at the seam.** Something big should be moving across the loop point, so it never
>    feels like a pause. Run `pacing_check.py --loop`, which measures the gap across the seam too.

## scripts/loop_check.py — verbatim

> ```python
> """loop_check.py: hack 4. Proves a loop is seamless.
> Test A (with --project): renders the frame at t = 0 and the frame at t = end with HyperFrames
>   and compares them pixel by pixel. They must be the same picture.
> Test B (always): compares the last frame of the video with the first. That jump must be no bigger
>   than a normal step between two frames, so the seam is invisible when the video repeats.
> Exit code 0 = pass, 1 = fail."""
> …
>     hf = ['npx', '--yes', 'hyperframes@' + os.environ.get('HF_VERSION', '0.8.181')]
>     subprocess.run(hf + ['snapshot', proj, '--at', f'0,{D}', '--no-end', '-o', tmp], …)
>     …
>     mean, moved = diff_stats(fa[0], fb[0], 8)
>     same = moved < 0.0005
>     ok &= same
>     print(f"A. frame at 0 s vs frame at {D} s: {moved * 100:.3f}% of pixels differ …")
> …
> steps = [diff_stats(frames[i - 1], frames[i])[0] for i in range(1, len(frames))]
> seam = diff_stats(frames[-1], frames[0])[0]
> typical = statistics.median(steps) if steps else 0
> p90 = sorted(steps)[int(0.9 * (len(steps) - 1))] if steps else 0
> limit = max(2.5 * typical, p90, 0.6)
> seam_ok = seam <= limit
> ```

## scripts/pacing_check.py — verbatim, the thresholds

> ```python
> """pacing_check.py: hack 5, part two. Measures pacing from the pixels of a rendered video.
> Checks:
>   1. No dead gaps: something visibly changes at least every --max-gap seconds.
>      A 'change' is a 0.1 s step where more than 0.4% of pixels move by more than 14 levels.
>   2. Nothing flashes: no shot shorter than --min-hold seconds between two hard cuts,
>      and no element that pops in and out again inside --min-hold seconds.
> Writes <video>-contact.jpg (one frame per second from 0 s, left to right) and <video>-pacing.json.
> What counts as a change: big things moving. A small label sliding, or a slow colour fade, may not
> count, so near a loop seam keep one big move going every second. Flicker faster than about 3 per
> second reads as a flash."""
> …
> f10, w, h = gray_frames(a.video, 10, 240)
>     _, moved = diff_stats(f10[i - 1], f10[i])
>     if moved > 0.004:            # a change
> …
> f30, _, _ = gray_frames(a.video, 30, 160)
>     _, moved = diff_stats(f30[i - 1], f30[i], 30)
>     if moved > 0.35:             # a hard cut
> …
>     _, ac = diff_stats(f30[i - k], f30[i], 30)
>     _, ab = diff_stats(f30[i - k], f30[i + k], 30)
>     if ac > 0.01 and ab < 0.25 * ac:     # popped in and out again
> …
> rep = {… 'longest_gap_s': …, 'dead_gaps': dead, 'hard_cuts': …, 'too_short_shots': short_shots,
>        'flashes': …, 'pass': not dead and not short_shots and not merged}
> ```

## styles/blueprint/STYLE.md — one style spec, verbatim

> ```
> # Blueprint
> id: blueprint
> Reference piece: styles/blueprint/index.html (12 s, loops)
> Gallery video: gallery/videos/blueprint.mp4
> Best for: how things work, product parts, engineering, step by step.
> Mood: a technical drawing that builds itself, line by line.
>
> ## Palette (hex)
> - ground: #0E3A66
> - lines: #CFE3F7 (2 px strokes)
> - faint grid: #1C4D80
> - text: #EAF3FC
> - accent: #FFD23F (one highlight: the part that matters)
>
> ## Type
> - Labels: IBM Plex Mono 500, 26 to 32 px, all caps, tracking 0.06em
> - Big numbers: IBM Plex Mono 700
> - Font links:
>   <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@500;700&display=swap">
>
> ## Easing (numbers)
> - draw a stroke: power1.inOut, 0.6 to 1.2 s (stroke-dashoffset)
> - dimension arrows: expo.out, 0.5 s
> - camera: power2.inOut, 1.0 s pans
>
> ## Motion rules
> - Everything is drawn as strokes first, then filled (if ever).
> - Labels arrive with a leader line that draws from the part to the label.
> - Only the accent part fills with yellow.
>
> ## Signature moves
> - Construction lines shoot across the sheet first, the real strokes trace over them, then the
>   construction lines fade.
> - Exploded view: parts slide apart along one axis with a small stagger and a damped settle. Inner
>   parts lag behind.
> - Hidden lines: a part inside another is dashed, and turns solid the moment it is uncovered.
> - Dimension lines grow out from their number, with the arrowheads riding the tips.
> - The key part fills yellow with a bottom up wipe, its label turns yellow, and the camera pushes in on it.
>
> ## Grid
> Fine grid at 24 px, heavy grid at 120 px. 96 px margins. …
> The grid moves at 30% of the camera for parallax.
>
> ## With media
> - Keep photos out of the drawing itself.
> - A photo can sit beside the drawing in a sharp edged frame drawn with the same 2 px line, graded to
>   a cool blue duotone.
> - Or trace the object from a photo, then cut from the photo to the drawing on a stroke.
>
> ## Ban list
> - photos inside the drawing
> - soft shadows, glow
> - rounded friendly type
> - more than one highlight at a time
> ```

## questions.md — verbatim

> 1. What is it for? (YouTube intro, Short, launch, explainer, ad) and how long? [12 s, 16:9]
> 2. What should it say? (the words on screen) [I'll write it from your topic]
> 3. Does it loop? [no]
> 4. Your brand: colours, fonts, logo? [use the style's]
> 5. Real media? Your photos or clips, AI clips from Higgsfield, or none (pure graphics) [none]
>
> Reply with just the numbers you want to change, like "1: Short, 8 s. 5: my photos". Or say "go" for
> all the defaults.

## The two cited origins

### NovusEdge/motion-lab — MIT, 16 files

> "A Claude Code skill for animation rendered from code … **Claude already renders such pieces
> correctly. It drives frames from time, captures them exactly and keeps loops seamless. What it
> lacks without direction is taste. Unspecified choices fall back to one look: a dark void, neon
> glow, purple-to-blue gradients, bloom, sparkles, a turntable spin.** The `directing-motion` skill
> makes Claude write a numeric brief before building, ban those defaults by name, keep one hero move
> per shot, and **review rendered frames by frame number**."

> `references/numbers.md` — "Easing curves, spring damping ratios, durations, stagger, beat grid,
> holds, animation principles **as frame counts**"
> `references/fixes.md` — "**Symptom → cause → fix** for flicker, seams, fonts, WebGL, silent
> parameter renames, audio"
> `scripts/contact-sheet.sh` — "One image of every Nth frame, **stamped with frame numbers**"
> `scripts/diff.sh` — "Changed pixels and their bounding box between two frames, **for seams and shot
> boundaries**"
> `scripts/hit-check.mjs` — "**Where each sound hit peaks in the decoded file**, against its target
> frame"
> `scripts/motion.mjs` — "Pure-time helpers: cubic-bezier, springs, beat frames, seeded RNG, loop phase"
> `scripts/freeze.html` — "`?t=2.5` snippet to open any single frame in a browser"

Its review loop, verbatim:

> Each review round, before changing code:
> 1. **Describe each still in one sentence as a viewer would.** Compare it with its line in SHOTS.
> 2. On the contact sheet, **count what moves in each frame. More than one hero at a time is the most
>    common failure**; demote the rest (later, shorter travel) or cut it.
> 3. **Check every BANNED item against the frames, including ones you added while fixing.**
> 4. Run `diff.sh` across the loop seam (frame N-1 against 0) and across every shot boundary:
>    **the changed box must contain only motion you planned. Small pops (a few dozen pixels) are
>    invisible on a contact sheet and obvious in a loop.**
> 5. **Write notes as `frame 84: <what is wrong>: <the number that changes>`. Apply one or two per
>    round. Re-render only the frames the notes name.**
>
> **Two to four rounds is normal. Frame numbers make notes precise; "it feels off" is not a note.**

Its common-mistakes table, verbatim:

> | Mistake | Instead |
> |---|---|
> | Building before writing the brief | Brief first; it costs minutes and saves review rounds |
> | A look from adjectives ("premium", "modern") | Hex values, roughness, light positions, fov |
> | Subtle principles (a 4% squash) that vanish at phone size | **Push key poses 10 to 30% past what feels right** |
> | Reviews that only catch bugs | Also check hierarchy, BANNED and SHOTS every round |
> | Sound on the picture frame, or measured in the source WAV | **1 frame after, measured on the decoded MP4** |

Its engine table names ours: *"GSAP (free incl. former bonus plugins), or **HyperFrames (Apache 2.0,
HTML + GSAP → MP4)**"*, and refuses Remotion on **licence** grounds (*"free for individuals and
companies up to 3 people; larger companies need a license"*) — where our `decisions.md` refuses it on
maintenance grounds.

### aiartimind.com — "How to Make Motion Graphics With Claude Opus 5.5 Without the AI-Slop Look"

> Meta description, verbatim: "A step-by-step Claude Opus 5.5 motion graphics workflow, shown with
> real renders: **pick a deterministic engine, add taste rules, brief with exclusions, lock tokens,
> plan beats in frames, build in four passes, inspect boundary frames and revise by frame range.**"

(The page body is client-rendered; only the meta description was machine-readable.)
