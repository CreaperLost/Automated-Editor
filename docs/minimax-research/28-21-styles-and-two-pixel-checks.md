# 28 — 21 named styles, two pixel checks, and notes written against frame numbers

- **Source:** [github.com/ItsssssJack/claude-video-2](https://github.com/ItsssssJack/claude-video-2) —
  *Claude Video 2.0: make studio-quality motion graphics with Claude Code. Free skill with a gallery
  of 21 looks* · 111 files · MIT · 44 stars · last push 21 Oct 2026 · **built on HyperFrames**
- **Video:** [youtu.be/KphrV76DJoI](https://youtu.be/KphrV76DJoI) — *Claude Video 2.0 Just Changed
  Everything (FREE Skill)* · **Jack Roberts** · 24 Oct 2026 · 13:00 · 35,518 views · 645 likes
- **Also published as:** a Notion page
  (`app.notion.com/p/Motion-Video-free-Claude-skill-400e8d6bd137810897c2ea505cbf29b8`) — **the repo
  carries that page's own source as `NOTION-PAGE.md`**, with an unfilled `[REPO LINK GOES HERE]`
  placeholder where the download link goes.
- **Raw captures:** [the kit, read in full](evidence/2026-10-25/claude-video-2-skill.md) ·
  [the repo](evidence/2026-10-25/itsssssjack-claude-video-2.md) ·
  [the video](evidence/2026-10-25/claude-video-2-0-just-changed-everything-free-skill.md) ·
  [the Notion page](evidence/2026-10-25/notion.md)
- **Evidence grade:** **a published artefact, read in full, with its licence text read.** **Not
  installed, not run** — no render was made and no check was executed.

**This is the most useful single artefact in the folder**, and it is not close. It is MIT — the
first fully permissive licence we have found in the field — it runs on **our runtime**, and it
ships **two pixel checks over a rendered video**, which is precisely the thing this whole research
folder has been circling and not finding.

## 1. The licence verdict: MIT, read as text

> "Permission is hereby granted, free of charge, to any person obtaining a copy of this software
> and associated documentation files (the "Software"), to deal in the Software without
> restriction, including without limitation the rights to use, copy, modify, merge, publish,
> distribute, sublicense, and/or sell copies of the Software …"

**Unrestricted, including commercial use and redistribution.** That places it in the same class as
HyperFrames itself (Apache-2.0) — **the two things in this folder we can adopt outright** — and
well above kitcut (no licence), `video-talkcraft` (PolyForm Noncommercial), Editor V1 (bespoke, no
resale) and Higgsfield's own skill (no licence at all).

## 2. What it does

**A gallery of 21 looks → five questions → build → 3 stills to approve → render → two pixel
checks.**

Twenty-one named styles in `styles/`, each a folder with a **`STYLE.md` spec** (palette in hex,
type, easing numbers, motion rules, signature moves, composition, media, a ban list) and, for most,
**a full working piece to start from**. A gallery server opens them as real videos; the person
picks one, or pastes their own references and the skill **writes a new spec from them**.

Then five questions with defaults, a brief in numbers, a storyboard with the AI-credit cost shown
before anything is spent, **3 stills with scores**, a render, and the checks.

The five questions are a genuinely good interface:

> 1. What is it for? (YouTube intro, Short, launch, explainer, ad) and how long? [12 s, 16:9]
> 2. What should it say? (the words on screen) [I'll write it from your topic]
> 3. Does it loop? [no]
> 4. Your brand: colours, fonts, logo? [use the style's]
> 5. Real media? Your photos or clips, AI clips from Higgsfield, or none (pure graphics) [none]

**Five questions, each with a default, answerable as "1: Short, 8 s. 5: my photos"** — or "go" for
all of them. That is a better shape than anything we have for our `preset` tool.

## 3. The two checks — gate 3, on our runtime

This is the part worth copying line by line.

### `pacing_check.py` — pacing measured from the pixels of the rendered video

| check | the rule, in code |
|---|---|
| **no dead gaps** | something visibly changes at least every `--max-gap` (default **1.0 s**); *"a change is a 0.1 s step where more than 0.4 % of pixels move by more than 14 levels"* |
| **no short shots** | no shot shorter than `--min-hold` (default **0.3 s**) between two hard cuts; a hard cut is **>35 %** of pixels moving by **>30 levels** between adjacent frames at 30 fps |
| **no flashes** | no element that pops in and out again inside 0.3 s — detected by a forward change >1 % and a backward change <25 % of it |
| **output** | a contact sheet (**one frame per second**) and a JSON report: `longest_gap_s`, `dead_gaps`, `hard_cuts`, `too_short_shots`, `flashes`, `pass` |

### `loop_check.py` — two tests, one of them a render-and-diff

> **Test A** (with `--project`): renders the frame at t = 0 and the frame at t = end with HyperFrames
> and compares them pixel by pixel. They must be the same picture. *(<0.05 % of pixels may differ.)*
>
> **Test B** (always): compares the last frame of the video with the first. **That jump must be no
> bigger than a normal step between two frames**, so the seam is invisible when the video repeats.
> *(limit = max(2.5 × the median frame step, the p90 step, 0.6 levels) — a **relative** threshold.)*

And the command it runs:

```python
hf = ['npx', '--yes', 'hyperframes@' + os.environ.get('HF_VERSION', '0.8.181')]
subprocess.run(hf + ['snapshot', proj, '--at', f'0,{D}', '--no-end', '-o', tmp], …)
```

**This is exactly the command our `gate.rs::snapshot()` already runs** — `hyperframes snapshot
<stage> --output <dir> --at <seconds, comma-separated> --no-end` — pointed at a *whole project*
instead of a *graphic stage*. Finding [26](26-the-style-is-a-program.md) called it "one argument
away". **Here is the working implementation.**

### What we have, against that

| | AeroEdits today | this skill |
|---|---|---|
| pixel checks | `gate.rs`: coverage, opacity, full-frame fill, overlay share, a graphic's own cut-in/out frames, hard-coded colours and fonts, fixed text, variables, look compliance | same *kind*, but **of the whole piece over time** |
| temporal checks | **none** | dead gap, short shot, flash, loop seam |
| frame-to-frame diff | **none** | the primitive every one of its checks is built on |
| a still to look at | `mcp/storyboard.rs` — one PNG, a tile per scene | `stills.sh` — 3 stills, 0.6 s after a shot change, + a contact sheet, one frame per second |
| scoring | **none** | 1–10 on composition, motion, type, colour, idea; fix the worst 3; repeat until all ≥ 8 |

**Our gate measures a graphic in isolation and stops at the edges of the frame. Every one of these
checks is about what happens *between* frames.** That is gate 3, and it is the first complete
implementation of it we have found — in any language, on any runtime.

## 4. The style spec: richer than ours

`references/style-format.md` gives the format, and `styles/blueprint/STYLE.md` is a worked example.
The sections:

| section | the blueprint example |
|---|---|
| Palette (hex) | `ground: #0E3A66` · `lines: #CFE3F7 (2 px strokes)` · `accent: #FFD23F (one highlight: the part that matters)` |
| Type | `Labels: IBM Plex Mono 500, 26 to 32 px, all caps, tracking 0.06em` + the font link |
| **Easing (numbers)** | `draw a stroke: power1.inOut, 0.6 to 1.2 s (stroke-dashoffset)` · `camera: power2.inOut, 1.0 s pans` |
| **Motion rules** | *"Everything is drawn as strokes first, then filled"* · *"Labels arrive with a leader line that draws from the part to the label"* |
| **Signature moves** | *"Construction lines shoot across the sheet first, the real strokes trace over them"* · *"Hidden lines: a part inside another is dashed, and turns solid the moment it is uncovered"* |
| Composition | grid at 24/120 px, 96 px margins, parallax at 30 % of the camera |
| With media | *"A photo can sit beside the drawing … graded to a cool blue duotone"* |
| **Ban list** | *"photos inside the drawing"* · *"soft shadows, glow"* · *"rounded friendly type"* · *"more than one highlight at a time"* |

**Our `Look` is five global controls — size, roundness, speed, energy, density — plus ease and
duration tokens.** This is a *named style as a document*: a palette, a type scale, easing numbers,
4–8 motion rules, 2–4 signature moves, a composition, and a ban list, in **under about 80 lines**.

Two things in it are new to us:

- **The ban list.** A style's *negative space* — what must not appear. Ours has no equivalent.
  Editor V1 had one too (*"no hue-shifting gradients, no glow, bloom or lens flares…"*), and Max
  had one in the brief. **Three artefacts, three independent ban lists.**
- **Signature moves.** Not "how the motion should feel" but **the two to four moves that make this
  look recognisable** — named, countable, and checkable ("a part inside another is dashed, and
  turns solid the moment it is uncovered").

And the precedence rule is explicit:

> "Where the chosen style's own signature conflicts with this list (a sun's glow, a centred
> widget), **the style wins: its gallery video is the proof.** Everything else on the list still
> applies."

The same shape as Editor V1's *"where they disagree, they win"* — **two artefacts, same rule.**

## 5. The five hacks, each with its source

`references/hacks.md` is the derivation record, and it cites where every habit came from — the
first artefact in this folder to do that at the level of a *habit* rather than a *measurement*.

> **1. Pick one style first, and stick to it.**
> "Why: when you leave the look open, **every model falls back to the same default look (dark
> background, glow, purple gradient).** A fixed spec stops the drift between shots."
> *Source: Motion Lab says to ban the default AI look up front.*
>
> **2. Real references beat adjectives.**
> *"Make it sleek and modern" means nothing. A real frame from a real film means a lot.* Ask for 1
> to 3 references, then **describe HOW they were made (masked text, slow camera, long holds), not
> just how they feel.**
> Where to find them: savee.com, godly.website, mobbin.com, launch films from Linear, Apple and
> Cursor. *Source: a frame-by-frame look at the Linear Agent launch film.*
>
> **3. Write the brief in numbers.** Length in seconds · colours in hex · **easing as names and
> seconds (expo.out over 0.6 s), never "smooth"** · one hero move per shot, with its start time ·
> a ban list.
>
> **4. Loops: one clock, last frame = first frame.**
> *"Prove it: `loop_check.py` renders the frame at 0 s and the frame at the end and diffs the
> pixels. It fails if they differ."* *Source: HyperFrames renders a paused timeline frame by
> frame, so the same time always gives the same picture.*
>
> **5. Check frames, not code.**
> *"Code that looks right can still look wrong."* 3–4 stills at the shot changes → the pacing check
> → the contact sheet.

**Hack 1 is the field's clearest statement of the problem we have not solved.** Not "the model is
bad at motion" — *the model renders perfectly; the problem is that every unspecified decision
collapses into the same look.* **That is the whole thesis of our `Look`, stated by a third party,
and it is a much sharper statement of it than ours.**

**Hack 3 is our `Look`'s missing half.** Our look holds *skin* (how a graphic looks); theirs holds
*direction* (how the piece moves: easing names and seconds, one hero move per shot with a start
time, a hold minimum). **We have one; they have the other.**

## 6. Motion Lab — the origin, and the best review loop in the folder

`references/hacks.md` cites [NovusEdge/motion-lab](https://github.com/NovusEdge/motion-lab) for
three of the five hacks, so I captured it too. **MIT**, 16 files, and its README says the thing
plainly:

> "Claude already renders such pieces correctly. It drives frames from time, captures them exactly
> and keeps loops seamless. **What it lacks without direction is taste. Unspecified choices fall
> back to one look: a dark void, neon glow, purple-to-blue gradients, bloom, sparkles, a turntable
> spin.** The `directing-motion` skill makes Claude write a numeric brief before building, ban those
> defaults by name, keep one hero move per shot, and **review rendered frames by frame number**."

Its review loop is the best thing in this folder for AGT-3:

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

**That is the learning format we do not have.** Our checkpoint-diff learns *that* a graphic
changed; this learns **where** (a frame number), **what is wrong** (in one sentence, as a viewer
would say it), and **which number to change** (a parameter, not an adjective) — and it bounds the
work: one or two notes per round, re-render only the frames named, two to four rounds is normal.

Its common-mistakes table is the severity-tiered checklist (finding
[07](07-motion-taste-checklist.md)) in its most explicit form:

> | Mistake | Instead |
> |---|---|
> | Building before writing the brief | Brief first; it costs minutes and saves review rounds |
> | A look from adjectives ("premium", "modern") | Hex values, roughness, light positions, fov |
> | Subtle principles (a 4 % squash) that vanish at phone size | **Push key poses 10 to 30 % past what feels right** |
> | Reviews that only catch bugs | Also check hierarchy, BANNED and SHOTS every round |
> | Sound on the picture frame, or measured in the source WAV | **1 frame after, measured on the decoded MP4** |

Two of those lines are about our problems specifically. **"Sound … measured on the source WAV" →
"1 frame after, measured on the decoded MP4"** is the sound-picture-offset rule, and it pairs with
Editor V1's measured *"clicks land 4–14 ms after the picture, 2–7 dB under the voice"*. **Two
independent derivations of the same relationship.**

And its engine table names ours — *"GSAP (free incl. former bonus plugins), or **HyperFrames
(Apache 2.0, HTML + GSAP → MP4)**"* — and refuses Remotion on **licence** grounds (*"free for
individuals and companies up to 3 people; larger companies need a license"*), where our
`decisions.md` refuses it on **maintenance** grounds. Two different reasons, same conclusion.

## 7. Three derivations of "a style"

This folder now holds three independent answers to the question GFX-5 asks, and they agree on the
shape and differ on the evidence:

| | **Editor V1** ([25](25-the-kit-behind-the-video.md)) | **Max's personal skill** ([27](27-the-personal-skill-higgsfield-publishes.md)) | **Claude Video 2.0** (this) |
|---|---|---|---|
| the style is derived by | **measuring three reference reels frame by frame** | **measuring one episode against the author's final cut** | **describing the references' production method** (masked text, slow camera, long holds) |
| it is written as | ~30 tokens, each annotated MEASURED or DERIVED | a personal checklist, with a "limits of generalisation" section | a **STYLE.md spec**: palette, type, easing numbers, motion rules, signature moves, composition, ban list |
| the numbers are | measured from pixels | measured from the edit | **prescribed** — hex, frames, seconds |
| it forbids | a mix of styles | a mechanical A–B–C rotation | **a ban list**, with the style winning its own signature |
| it checks by | re-transcribe the final and diff it | watch the whole programme in motion | **pixels**: pacing, loop, stills, scores |

**Three routes, one conclusion: the style is a written artefact with numbers in it, and the agent
must be bound by it.** That is what "the app holds the look" means in practice — and this artefact
shows the fullest version of it: **21 named styles, each with a spec and a working example, any of
which the user can pick from a gallery.**

## 8. What the skill documents about our runtime

`references/hyperframes.md` is **a third-party statement of the HyperFrames contract**, and it
matches what our repo assumes:

> - One root element: `<div id="root" data-composition-id="main" data-start="0" data-duration="9"
>   data-width="1920" data-height="1080">`.
> - One paused timeline registered as `window.__timelines["main"] = tl`.
> - Timed media gets `class="clip"` plus `data-start` and `data-duration`.
> - Render: `npx hyperframes render <dir> -o out.mp4 -q draft` … Stills: `npx hyperframes snapshot
>   <dir> --at 0,3,6 --no-end -o stills/`. Lint: `npx hyperframes lint <dir>`.
> - **Same time in, same picture out.** No `Math.random`, no `Date.now`, no timers, no
>   `requestAnimationFrame` loops.
> - **If you add ANY font link, link EVERY font the page uses: HyperFrames then skips its own font
>   loading, and an unlinked font falls back to Times.**
> - **System font names are swapped by HyperFrames: "Helvetica Neue", "Avenir", "SF Pro" become
>   Inter and "Futura" becomes Montserrat.**
> - **Rendering needs internet for GSAP and fonts (both load from public CDNs).**
> - 16:9 is 1920 × 1080, 9:16 is 1080 × 1920. **Lay the scene out again for each; never just crop.**

Three of those deserve a note:

1. **The font swap.** Our templates set
   `--look-font: var(--brand-font, -apple-system, BlinkMacSystemFont, "SF Pro Display", "Segoe UI
   Variable Display", "Segoe UI", system-ui, sans-serif)`. **If HyperFrames rewrites "SF Pro" to
   Inter, our intended system-font stack is not what the renderer draws** — and it would be
   different on a Mac than on Windows. **This is a hypothesis, not a confirmed bug**: I did not
   render anything, I do not know the swap's matching rules, and `-apple-system` may win before
   the named face is reached. **But it is checkable in one render**, and our gate already inspects
   `font-family:` in a composition.
2. **GSAP from a CDN vs. ours.** They load GSAP from a public CDN and warn that rendering needs
   the internet; **we vendor GSAP and pin it by SHA-256.** Their constraint does not bind us, and
   our pinning is the better answer to the same problem.
3. **`--format mov` with a transparent background** — "transparent overlay for editors". Our
   graphics already carry `data-overlay`, so we are on the same path.

They also pin **HyperFrames 0.8.181** — the same version as Editor V1. **We pin 0.8.162.** Two
independent artefacts have converged on a newer version than ours.

## 9. What we would take

1. **A pacing check.** Dead gaps, short shots and flashes, measured from the pixels of the
   rendered video, with the thresholds written down: **a change = >0.4 % of pixels moving >14
   levels in 0.1 s; nothing on screen under 0.3 s; nothing dead for over 1 s.** We have the pixel
   machinery and **no temporal check at all**. This is the single biggest gap in this finding.
2. **A loop check.** Render the frame at t = 0 and t = end with `hyperframes snapshot --at 0,D`,
   diff them, and fail if they differ; plus a seam check against the median frame step. Our
   `snapshot()` already takes a comma-separated `--at` — **we are one argument away**.
3. **A frame-to-frame diff primitive** as the base of both. Our gate compares a graphic against
   its background; this compares a frame against the frame before it. Different axis, same
   machine.
4. **The review note format**: `frame 84: <what is wrong>: <the number that changes>`, one or two
   per round, re-render only the frames named, two to four rounds is normal. **This is the AGT-3
   output format** — anchored, specific, bounded, and *not* "it feels off".
5. **Score the stills 1–10 on composition, motion, type, colour, idea; fix the worst 3; repeat
   until all ≥ 8; show the person the scores.** A taste check with a number and a stopping rule.
6. **A `STYLE.md` format for a `Look`**: palette in hex, type, easing as names and seconds, 4–8
   motion rules, 2–4 signature moves, composition, a ban list, under 80 lines. **Our `Look` holds
   skin; this holds direction.**
7. **A ban list** on every style, and **a rule that the style's own signature wins over it.**
8. **A gallery of named styles, each with a working example**, so the user picks rather than
   describes. Our `preset` tool returns rules; a gallery returns *looks you can watch*.
9. **Five questions with defaults**, answerable as "1: Short, 8 s. 5: my photos" or "go".
10. **"Never say a check passed without running it. Never say a look is right without looking at a
    frame."** As a hard rule in our skill file.
11. **Check whether HyperFrames rewrites our system font names.** One render answers it.
12. **A ban on text over a face**, which they enforce in the brief and hard rules — and which our
    face boxes could enforce mechanically.

## What this implies outside this folder — **not applied**

This folder is the only place anything was changed. For the record, here is what a maintainer would
want to change elsewhere — **none of it has been applied**:

- **`gaps.md` → §7 (Style)** would gain rows: *the look holds skin but not direction* (no easing
  numbers, no motion rules, no signature moves); *no ban list*; *no named styles to pick from*.
- **`gaps.md` → §12 (Render and export)** would gain a row: *no temporal check — no dead-gap, no
  short-shot, no flash, no loop-seam rule*; and a row: *the renderer may rewrite system font names,
  and our templates name "SF Pro Display"*.
- **`gaps.md` → §11 (Transcription and learning)** would gain a row: *learned notes are not anchored
  to a frame and do not name the number to change.*
- **`roadmap.md` → GFX-5** would gain its **fourth** answer: *a style is a spec document with
  numbers, a ban list and a working example, and the agent is bound by it — and 21 of them already
  exist, MIT, on our runtime.*
- **`research/competitors.md` → "At a glance"** would gain **Claude Video 2.0** — MIT, HyperFrames
  + GSAP, 21 styles, a gallery, two pixel checks, a scored stills loop — the closest published
  artefact to AeroEdits' thesis, and licence-clear.
- **`research/creator-workflows.md` → artefact index** would gain **Claude Video 2.0** (MIT) and
  **Motion Lab** (MIT), with the frame-numbered review loop and the symptom→cause→fix table.
- **`research/creator-workflows.md` → "Tricks worth stealing"** would gain five rows: *write notes
  as `frame N: what is wrong: the number that changes`*; *score the stills 1–10 and fix the worst
  three*; *check the dead gap, the short shot and the flash in the pixels*; *ban the default look
  by name*; *one style, one accent, one hero move per shot*.

## Caveats

- **The skill was read, not run.** No render was made, no check was executed, and no comparison
  was performed against a real video. Every threshold quoted here is the author's.
- **The genre is different.** These are 12-second motion graphics, not a 10-minute edit. **"Nothing
  dead for over 1 s"** and **"a 4-second hold is a mistake"** are the same family of rule at
  different scales — a long-form piece has a different number, and the field's 4 s figure (finding
  24 and 27) is the one that applies to us.
- **The four-second number still rests on two derivations**, not three: Editor V1 (measured from
  three reels) and Max (from his own episodes). This artefact's 1 s is a different genre's number
  and should not be merged with it.
- **The font-swap issue is a hypothesis.** I did not render anything and do not know the matching
  rules. It is one render to settle.
- **The Notion page's body is client-rendered**, so only its meta description was machine-readable;
  the full text is in the repo's own `NOTION-PAGE.md`, which the capture quotes. The page also
  carries an unfilled `[REPO LINK GOES HERE]` placeholder in the repo copy.
- **Motion Lab is a 1-star repository** with no forks, published 11 Oct — a week before this one.
  It is cited as a source by three of the five hacks, which makes it an origin rather than an
  independent corroboration.
- **AI-artimind's body is client-rendered too**; only its meta description was captured, and it
  names *"revise by frame range"* — the same learning idea, from the other side.
- **Nothing here is a measured result.** The numbers are the author's rules; no claim in this
  finding was independently verified.
