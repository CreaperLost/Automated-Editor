# 26 — The style is a program: tokens, seeds and a frame grid

- **Source:** the same artefact as [finding 25](25-the-kit-behind-the-video.md) — *NextPhase —
  Editor V1 Skill*, 79 files, read as code rather than as documentation:
  `assets/tokens.css` · `components/ev1_devices.py` (541 lines) · `components/ev1_more.py` (572) ·
  `scripts/ev1_treat.py` (188) · `scripts/ev1_clip.py` (55) · `scripts/snap-onsets.py` (59) ·
  `scripts/bed-mix.py` (93) · `scripts/preview.py` (130) · `scripts/clipsheet.py` (18) ·
  `template/build.py` (124)
- **Raw capture:** [evidence/2026-10-25/editor-v1-skill-kit.md](evidence/2026-10-25/editor-v1-skill-kit.md)
- **Evidence grade:** **source read, not run.** Every quote below is upstream text. Nothing was
  executed, no render was made, and no number was reproduced. Where this file compares the kit to
  AeroEdits, the AeroEdits side was verified in this repository; the kit side was not.

Finding 25 read the kit as a document. This one reads it as **an implementation**, because that is
where the transferable engineering is. The short version: **the kit turns a derived style into a
program — a token file, a library of functions that each write their own HTML, their own GSAP and
their own sound cues, and a build that is one block per spoken line.** It is the same shape as
AeroEdits, and the differences are the useful part.

## 1. The architecture

| stage | the kit | AeroEdits |
|---|---|---|
| the input | `PLAN.md` — a beat table (time · said · device · real material) plus a sources table with licences and FLAGs | the transcript, plus the edit document |
| the plan is a **file** | yes, human-readable, shown to the person, edited, approved | the edit document (structured) |
| the look | `assets/tokens.css` — ~30 tokens, each annotated MEASURED or DERIVED | `style/look.rs` — five global controls + ease/duration/gsap-ease |
| the work | `build.py`: one block per plan line, keyed to word onsets, each calling a device | the passes |
| the device | a function that writes HTML + GSAP + SFX cues | a pass |
| the check | `hf lint` → `hf snapshot --at <every beat>` → *look at every frame* → `hf render` | `hyperframes check --json` (lint/runtime/contrast/layout) + the pixel gate |
| the mix | `bed-mix.py` (voice + bed, cut not ducked) + `mix-sfx.py` (device-written cues) | `media/audio.rs` mixer + `sound_effects.rs` |
| the last check | **re-transcribe the final and diff it against the take** | — |

**The device is the unit of reuse, and it writes three things at once.** From `ev1_devices.py`:

> A device writes its HTML, its GSAP lines and its own SFX cues, so the sound lands on the frame
> the picture changes and **the cue sheet can never drift from the cut**.

That third part is the one to steal. It is the same idea as our `sound_effects.rs` — *"The events
are worked out from the edit document alone"* — but stated as an invariant of the *device* rather
than of a mix pass. A device cannot emit a picture without emitting its sound, because the sound
is part of its return value.

## 2. The tokens file is a `Look`, and it is annotated

`assets/tokens.css` opens with a header that says what every token is:

> ```
> MEASURED = read off the frames. DERIVED = converted to 1920x1080 @30.
> Import this into every Editor V1 composition; never mix with another style.
> ```

and each token carries its provenance inline:

```css
--ev1-cap-size:    62px;    /* REF-1 (16:9) caption ~62px at 1080; REF-2's 4:3 frame read 48 */
--ev1-teal:    #0E5560;      /* shadow tone of the teal split (REF-1 22% of held runtime) */
--ev1-focus-frames: 12;       /* rack focus in: 10 frames @24 */
--ev1-vignette-edge: 0.70;    /* edge/centre luma: 0.86 REF-1, 0.59 REF-2 */
--ev1-push:         1.06;     /* slow push on a held still across its beat */
```

**Our `look.rs` has five controls. This has about thirty, and every one is traceable to a
measurement or a stated decision.** The difference is not size — it is that each token says *where
it came from*, which is what makes the style arguable and reproducible.

The motion tokens are the interesting part, because they are **measured in frames at the source fps
and derived to 30 fps**:

```css
--ev1-tile-frames:  10;   /* tile-flash reveal: 9 frames @24 -> 11 @30, 1 click per pane */
--ev1-pop-step:     0.08s;/* collage pop-in: one photo every 2 frames @25 */
--ev1-invert-frames: 2;   /* negative flash out of a title card (REF-2 f503-504) */
--ev1-focus-frames: 12;   /* rack focus in: 10 frames @24 */
--ev1-strobe-step:  0.08s;/* strobe montage: 2-frame images on one word */
```

**A duration expressed in frames, not seconds, with the source frame rate noted.** Our look has
`duration` and `gsap-ease` tokens in seconds. A frame count is the unit that survives a change of
frame rate, and it is the unit that decides whether two things land on the same frame.

And the instruction at the end — *"never mix with another style"* — is finding
[17](17-what-the-seven-agree-on.md)'s rule, stated as a file-level constraint.

## 3. The renderer rules, learned the hard way

`ev1_devices.py` carries a list of invariants that is worth reading twice, because every line says
*"this broke an earlier build"*:

> - every time is snapped to a whole frame; **clip durations are FLOORED to 4 decimals**
> - every clip gets an explicit z-index (**track index is not paint order**)
> - **videos are top-level clips, never inside a timed wrapper**
> - initial transforms go in a `gsap.set` preamble **OUTSIDE** the timeline; `fromTo` always
>   carries `immediateRender:false`
> - nothing tweens visibility/autoAlpha on a `.clip`; **inner nodes carry opacity**

Those are five concrete facts about HyperFrames and GSAP that we also had to learn — our runtime
pins GSAP 3.15.0 by SHA-256 precisely because of that kind of fragility. The kit simply wrote them
down.

**One of them may be a bug in our code.** The kit's helper is:

```python
def dstr(t0, t1):     # clip duration floored so the renderer's ceil gives exactly the frame count
    n = round(t1 * FPS) - round(t0 * FPS)
    return f'{math.floor(n / FPS * 1e4) / 1e4:.4f}'
```

**The renderer's ceil is the reason for the floor.** Our `graphics/mod.rs::size_composition` writes
the root's duration as `format!("{seconds:.3}")` — **rounded to three decimals, not floored**. A
duration that rounds *up* (say 1.0166 s → `"1.017"`) ceils at 30 fps to 31 frames where the
intended count was 30.

**This is a hypothesis, not a confirmed bug** — it depends on whether our pinned renderer ceils
the same way, and I did not run it. But it is a cheap thing to check: the kit gives the rule, the
reason and a one-line fix, and the failure mode is one extra frame at the end of a graphic.

## 4. The treatment engine: model-free, seeded, deterministic

`scripts/ev1_treat.py` is the file that matters most for AeroEdits, and it is 188 lines of numpy
and OpenCV. Its docstring:

> The references never show a photo as it came: 58-68% of held shots carry a treatment. These are
> those treatments, **measured, as functions and a CLI**.
> …
> **Everything is seeded: the same call gives the same pixels (renders must be deterministic).**

Six looks, each a pure function with its measurement in the comment:

```python
elif name == 'mono':      # T2: 18% of held runtime, neutral grey, hard contrast
    g = scurve(L, 0.45); out = np.dstack([g, g, g])
elif name == 'teal':      # T1: 22% — hue mass piles in 180-220deg; shadows teal, highlights warm
    g = scurve(L, 0.25)[..., None]
    base = scurve(f, 0.2) * 0.6 + g * 0.4                      # 40% toward grey
    teal = np.array([0.50, 0.40, 0.08])                        # BGR, pulled into the shadows
```

plus `recolor` (rotate the hues inside a degree band to a target, feathered 12°), `halftone` (a
45° dot screen at a **measured pitch of 0.75 % of the height**), `vignette` (an **edge/centre luma
ratio of 0.86 / 0.59**, measured), `glitch_frames` (a seeded datamosh) and `led` (a dot-matrix
rebuild).

**Three things this says that we do not:**

1. **We check pixels; they transform them.** Our `graphics/gate.rs` measures a graphic to decide
   whether it is acceptable. **We never treat anything** — no grade, no vignette, no halftone, no
   recolour, on a still or on the footage. Their hard rule is *"Every found image is treated."*
   A treatment engine is ~200 lines, deterministic, and it is the difference between a source
   image and a shot.
2. **Seeded, not model-generated.** `np.random.default_rng(seed)` for everything that looks
   random, and the docstring says why: *"renders must be deterministic."* This is the same
   requirement our `decisions.md` states when it refuses generative video (*"No editability or
   determinism"*) — **and it is answered here without a model at all.** Procedural, seeded,
   reproducible.
3. **The bugs are documented next to the fix.** `recolor` carries the note *"Rotating the hue BY
   the weight walks every feathered pixel through the colour wheel and paints rainbow fringes"* —
   the wrong version is described so the right one is not re-derived.

## 5. A renderer limitation that constrains us too

`scripts/ev1_clip.py` grades moving footage with ffmpeg, and says why:

> The grade is baked in with ffmpeg **because CSS filters on `<video>` do not reach the renderer.**

It also fixes the output shape for the renderer:

> Every clip leaves here as 1920x1080, 30fps CFR, `-g 30`, yuv420p, no audio — **the shape the
> renderer seeks reliably (long-GOP and VFR sources render black or frozen)**.

Both are constraints on our runtime, discovered the hard way by someone else.

**The second one is worth checking against us.** Our repository normalises `yuv420p` in several
places, but a search for an import-time normalisation to **30 fps CFR with `-g 30`** found only
`fps=30/1` in a performance probe (`perf.rs`) — not in the import path. If the renderer really
does render long-GOP or VFR sources black or frozen, **the fix is one ffmpeg pass at import**,
and the kit already contains the exact filter chain.

The six ffmpeg looks are also a ready-made palette: `natural` (slight contrast), `mono`, `teal`,
`warm`, `cold`, `archive` — each a real filter chain with measured settings, e.g. teal is
`eq=contrast=1.14:saturation=0.78,colorbalance=rs=-0.14:bs=0.12:rh=0.06:bh=-0.07`.

## 6. The word-onset correction — 60–220 ms

`scripts/snap-onsets.py` is 59 lines and fixes something we have both ingredients for and never
combine:

> The speech engine reports the first word after a pause **60-220 ms EARLY**, so a caption or a cut
> keyed to it lands before the sound. This finds the real pauses with ffmpeg `silencedetect`
> (noise −38 dB, at least 0.12 s) and moves **only the FIRST word after each pause** to the
> pause's end. **Shorter dips are plosive closures inside a word** ("O|pus", "Clau|de"): snapping
> to those moves words onto the wrong syllable, so they are left alone.

That is a precise, correct, and *narrow* rule: the error is not uniform, it only affects the word
after a real pause, and it must be detected from the **audio**, not inferred from the transcript.

**Our side:** we have `dsp/silence.rs` (silence detection) and per-word `source_start_us` from
Parakeet (`transcript/parakeet.rs`). **Nothing combines them.** Our word starts are whatever the
ASR reported. Finding [20](20-the-runtimes-author-on-the-record.md) already noted that timestamps
decide whether *"cut the pauses between sentences to a quarter of a second"* works at all; this
is the correction that makes the timestamps true.

It also reframes finding [24](24-astra-the-vendors-own-numbers.md)'s 49 s → 37 s: **if 12 seconds
of that commercial were micropauses, then a caption or a cut keyed to the transcript may have
been landing before the sound — and trimming them by hand is what the editor had to do.**

## 7. The bed cuts; we duck

`scripts/bed-mix.py` is the sound grammar, and it is a different operation from ours:

```
--under LU    the bed sits this many LU under the voice (refs 0.8-3 dB; 5.5 keeps every word clear
              on a dense bed. Re-transcribe the final to check nothing got masked.)
--ramp a,b,dB[,fade]   a gain change between video times a and b ... -70 = the bed CUTS OUT (the
              punchline plays dry, then the bed slams back on the next line). Repeatable; ramps multiply.
--song-at S   the song time that plays at video time 0 (line the song's drop up with a word:
              S = drop_time_in_song - word_onset)
--voice-end T measure the voice's loudness up to T only (skip a silent tail).
```

**Our side:** the mixer ducks. `sound_effects.rs` sets `DUCK_DB = 8.0` for the effects lane, and
the project-level `audio.duck_db` is 12.0 for background lanes. We never cut a lane.

The difference is not the number, it is the **kind** of operation. Their reasoning: a duck leaves
the consonant audible over a moving bed, and that is exactly where *"Claude did."* became *"Flaw
dead."* Their answer is to **remove the bed entirely at the structural points** and let it slam
back on the next line.

Two details we have no equivalent of: **the song's drop is keyed to a word**
(`S = drop_time_in_song − word_onset`), and **the voice's loudness is measured up to the end of
the speech, not over a silent tail** — a real detail, because a trailing silence drags an
integrated measurement down.

## 8. The devices write their own sound

The role table in `ev1_devices.py`:

```python
SND = {
    # role:        (file in <skill>/sfx/,            gain,  pre-roll s)
    # Gains reproduce the refs' level vs a -20 LUFS voice: the clicks sit only 2-7 dB under the
    # voice. Pre-roll = file start -> transient, minus the 4-14 ms the refs' clicks LAG the picture
    'train_tile':  ('ev1kit/clicktrain_1', 0.39, 0.017),   # tile reveal: 1 click/frame
    'click':       ('ev1kit/click_3', 0.42, -0.008),       # a single pop / cut in a run
    'swish':       ('ev1kit/swish_1', 0.20, 0.270),        # into a title: transient = the cut
```

**A negative pre-roll.** The click *lags* the picture by 4–14 ms in the references, so the file
starts 8 ms *before* the picture change. That is a measured relationship between two events, and
it is the sort of thing that makes a cut feel sharp.

**And the sounds are synthesised to the measurement** — `scripts/synth-sfx.py` (507 lines) builds
22 sounds from noise, sine waves and envelopes, *"shaped to the measured length, timing and level
of the sounds in the reference edits"*, so the `SND` gains and pre-rolls hold. We synthesise ours
too, from the same principle.

## 9. The install proves itself

`scripts/preview.py` is the smoke test, and it is worth copying whatever the rest of the kit is
not:

> Draws its own test pictures in code (nothing downloaded, nobody's media), treats them with
> `ev1_treat.py`, builds a comp from the real devices (tile-flash reveal, pop + shatter, a
> shape-matched strobe, the red shock word, the 3D wall, a datamosh, the chrome title), renders it
> with HyperFrames, lays the device-written clicks with `mix-sfx.py` and opens `preview.mp4`. Also
> writes **`preview-sheet.jpg` (one frame per beat) — look at it.**

**One frame per beat**, and the instruction to look at it.

We have the analogue of the sheet — `mcp/storyboard.rs` renders one PNG of the finished edit, a
tile per scene — but not the analogue of the smoke test: **an end-to-end proof that exercises
treat → build → render → mix → sheet without any user media at all.**

`scripts/clipsheet.py` is the other half: a contact sheet of **8 frames per source file**,
labelled with the timestamp, so a human can look at a clip before it enters a build. That is the
reference log's *"contact sheets at 2fps over the whole runtime"* applied to incoming material.
We do not source material, so we have no equivalent — but the *idea* of a sheet you look at
before using a thing is the same idea as our storyboard.

## 10. What we would take

1. **A treatment engine.** ~200 seeded lines of numpy/OpenCV that grade every still we put on
   screen, with the parameters annotated by where they came from. We check pixels and never change
   them. Their rule: *"Every found image is treated."* This is the single largest visible gap.
2. **A `Look` with provenance in the token.** ~30 tokens instead of five, each carrying the
   measurement or the decision behind it, and expressed in frames where it is a duration.
3. **Snap word onsets to real pauses.** We have silence detection and word timings; the ASR
   reports the first word after a pause 60–220 ms early, and a caption or cut keyed to it lands
   before the sound.
4. **Cut the bed at structure points instead of ducking it.** A duck leaves the consonant
   audible over a moving bed, which is how "Claude did" became "Flaw dead".
5. **Align the music drop to a word**, and measure the voice's loudness only up to the end of the
   speech.
6. **The duration-floor rule** — check whether our renderer ceils, and if it does, floor the
   duration we write so the frame count comes out as intended.
7. **Normalise imported footage to 30 fps CFR with `-g 30`** at import, if the renderer does
   render long-GOP or VFR sources black or frozen.
8. **An end-to-end self-test** that renders a real composition from synthetic material and writes
   a sheet — one frame per beat — to look at.
9. **A device library that writes its own sound**, so a picture and its cue cannot drift.
10. **Pre-roll on the click**, compensating for the measured 4–14 ms lag.

## What this implies outside this folder — **not applied**

This folder is the only place anything was changed. For the record, here is what a maintainer would
want to change elsewhere — **none of it has been applied**:

- **`gaps.md` → §7 (Style)** would gain a row: *nothing is treated — no grade, vignette, halftone
  or recolour on any still or on the footage*; and a row: *the look's tokens carry no provenance*.
- **`gaps.md` → §12 (Performance and render)** would gain a row: *imported footage is not
  normalised to 30 fps CFR with `-g 30`*, with the long-GOP/VFR risk attached.
- **`gaps.md` → §11 (Transcription)** would gain a row: *word onsets are not corrected against
  detected silence; the ASR reports the first word after a pause 60–220 ms early.*
- **`gaps.md` → §9 (Audio)** would gain a row: *background lanes are ducked and never cut; a duck
  leaves the consonant audible over a moving bed.*
- **`gaps.md` → §3 (Timing)** would gain a row: *durations are rounded to three decimals; if the
  renderer ceils, a duration that rounds up yields one frame too many.*
- **`roadmap.md` → GFX-5** would gain its third answer, and the most concrete: *a derived style is
  a token file plus a treatment engine plus a device library — all of it reimplementable, and all
  of it small.*
- **`research/creator-workflows.md` → "Tricks worth stealing"** would gain five rows: *treat every
  found image; seed everything that looks random; snap word onsets to real pauses; cut the bed at
  structure points; align the music drop to a word.*

## Caveats

- **The kit was read, not run.** No device was rendered, no treatment was applied, no file was
  written. Every comparison with our code is from reading ours and theirs.
- **The duration-floor issue is a hypothesis.** It depends on how our pinned renderer rounds a
  duration, and I did not render anything to find out. Our `data-duration` is on the *root*
  composition; the kit's floor is applied to *per-clip* durations. Our clip durations may be
  written elsewhere in the codebase.
- **The import-normalisation gap is also a hypothesis.** The search was for `fps=30`/`fps_mode`/
  `-g 30` patterns; an import path that normalises through a different filter chain would not
  have matched.
- **The ASR onset error (60–220 ms) is the kit author's measurement, of his engine, on his take.**
  It is a plausible figure for a word-level aligner, but it was not reproduced here and it may
  differ per model.
- **The six ffmpeg looks and the treatment parameters are his measurements of three specific
  reels.** They are not a general palette; the numbers are only meaningful against those
  references.
- **We are not adopting the code.** The licence permits it (finding 25 §1, and the user's note
  that AeroEdits is not sold), but what is worth taking is the *shape*: measured tokens, seeded
  transforms, devices that write their own sound, and a check that reads the output.
