# 25 — The kit behind the video: a derived style, written down as a spec

- **Source:** [Google Drive folder](https://drive.google.com/drive/u/3/folders/19QyjPktZgEzqNoGO2hfyHma_9h27B65G) —
  *"NextPhase — Editor V1 Skill"*, public, three files shared 21 Oct 2026:
  `NextPhase-EditorV1.zip` (2,410,673 bytes) · `READ ME FIRST.txt` · `SETUP-PROMPT (copy-paste).txt`
- **What it is:** the artefact the video in
  [finding 23](23-the-style-is-derived-measured-and-named.md) distributes — a Claude Code skill named
  `editor-v1`, 79 files, plus an install prompt.
- **Raw captures:** [the kit, read in full](evidence/2026-10-25/editor-v1-skill-kit.md) ·
  [the folder page](evidence/2026-10-25/nextphase-editor-v1-skill-google-drive.md) ·
  [the video](evidence/2026-10-25/the-only-ai-video-editing-you-will-ever-need-turned-it-into.md)
- **Evidence grade:** **a published artefact, read in full, with its licence notes and its own
  measurement record.** The kit was **not installed and not run.** Every number below is the
  author's, produced on his own references by the method he documents.

Finding 23 read the *video*. This is the *thing behind it*, and it is the most complete artefact in
this folder: it contains the derivation record, the spec, the plan format, the device library, the
sound grammar, three separate checks — and a licence.

## 1. The licence, read as text

> "The skill (SKILL.md, reference/, components/, scripts/, template/, the design tokens and the
> synthesised sounds in sfx/) **is shared free by NextPhase. Use it to make your own videos,
> personal or commercial. Please don't resell it or repackage it as a paid product.**"

That is a bespoke permission, not an OSI licence. Under this repo's rule it is **not NOASSERTION**
— there is explicit licence text — and it is *more* usable than either of the two refs we refused:
`video-talkcraft` (PolyForm Noncommercial) and `kitcut` (no licence file at all).

**Verdict: usable outright — run it, adopt it, reimplement it.**

> **Update (25 Oct):** this file first recorded the restriction as binding on us, on the assumption
> that AeroEdits is a paid product. **It is not — we are selling nothing.** The clause *"don't
> resell it or repackage it as a paid product"* restricts what the kit may be *sold as*, and it
> does not bite on a project that is not sold at all. The licence permits running it, adopting its
> code, and shipping whatever we build with it. **The only thing it would stop is reselling the
> kit itself**, which was never the plan.

The clause would matter again the day AeroEdits has a price. At that point the question is not
"may we ship it" but "may we ship the *method*", and the answer to that is yes — a method is not
the artefact.

What the licence *does* allow, and what we want:

- run it as-is, to make videos, for money;
- read it, learn the method, and **reimplement** the ideas in our own code — a method is not the
  artefact;
- **adopt its code and its data into AeroEdits**, which the licence permits as long as AeroEdits
  is not sold as a repackaging of the kit.

The third-party table in `LICENSE-NOTES.md` is the same discipline the rest of this folder uses:

| what | licence | in the zip? |
|---|---|---|
| Inter, Playfair Display, Bodoni Moda, Arimo, JetBrains Mono | SIL OFL 1.1, licence file beside each | yes |
| GSAP 3.14.2 | GSAP Standard "No Charge" License | no — downloaded by `doctor.sh` |
| HyperFrames | **Apache-2.0** | no — installed from npm |
| Headless Chrome | Chrome / Chromium terms | no |
| Speech model (Parakeet or whisper) | the model's own licence | no |
| **Robust Video Matting** (the cut-out) | **GPL-3.0** | no — downloaded on first use |
| PyTorch, numpy, OpenCV, Pillow, soundfile | their own open licences | no — private venv |

**Robust Video Matting is GPL-3.0 and is a separate runtime component** — the one shape our own
rule allows. It is also, note, the only part of the kit we have **no equivalent of at all**: we
detect faces (YuNet) but we cannot cut a person out of a frame.

## 2. The kit and the video are the same edit

The zip ships `template/example/PLAN.md` — *"the 45 s cold open of an 'AI Editor Showdown' video
(1920×1080 @30, 44.97 s, a straight teleprompter read)"*. Its beat table is the video in finding
23, line for line:

| the kit's plan | the video's captions |
|---|---|
| `0.00 Right now,` → tile reveal from black onto a race start | `[00:00] Right now, three AI companies are in a` |
| `2.32 Claude, ChatGPT, Gemini.` → each company's own film, the cut on each name | `[00:03] …Claude, ChatGPT, Gemini` |
| `4.64 Billions` → money strobe, 2 frames per note, a click per note | `[00:05] …Billions of dollars` |
| `15.84 Which one can edit a video?` → HERO: the speaker cut out, the world behind swaps | `[00:15] Which one can edit a video?` |
| `20.56 They cut your silence,` → rebuilt waveform of THIS take; its real silences flag red | `[00:20] …They cut your silence` |
| `29.68 landing right on the word.` → the rebuilt timeline of THIS edit | `[00:28] …landing right on the word` |
| `40.72 Claude did.` → the bed CUTS OUT, the line plays dry | `[00:37] …Claude did.` |

**So the kit is the video's own build, published with its plan.** Two consequences: the style was
measured off three *other* reels (REF-1..3) and applied to this one; and the kit's `As built`
section is a real edit's real numbers — **46.4 s, 1392 frames, −18.9 LUFS integrated, true peak
−3.3 dBTP**.

## 3. The derivation record — this is what GFX-5 asked for

`reference/reference-log.md` is the artefact the roadmap's GFX-5 wants: *"AI-built styles: later,
research first."* It is not a description of a look. It is a **method, the raw numbers, and a
statement of what was not measured.**

**The method** (verbatim, condensed):

> 1. Per frame: 96×54 grey diff to the previous frame, HSV-histogram Bhattacharyya distance,
>    luma/saturation, colourfulness, hue histogram of saturated px.
> 2. A cut where the diff > max(14, 4× the local median) or hist distance > 0.5.
> 3. Contact sheets at 2fps over the whole runtime, full-frame-rate strips across every
>    distinct transition, full-res stills for type, grid and texture.
> 4. `demucs --two-stems=vocals` → levels, spectral-flux onsets, onset↔cut sync, periodicity,
>    and log-frequency spectrograms with cut markers.
> 5. Caption sync: count near-white px in the centre band per frame; jumps vs ASR word onsets.

**The numbers** (REF-1 a founders story, 78.4 s · REF-2 a game-theory explainer, 93.6 s ·
REF-3 a company-origin story, 70.5 s):

| | REF-1 | REF-2 | REF-3 |
|---|---|---|---|
| detected changes / min | 81 | 164 | **175** |
| held shots (≥0.25 s) median | 0.79 s | 0.56 s | — |
| held shots p90 | 1.77 s | 1.10 s | — |
| strobe/flash (<0.17 s) share of runtime | 3.7 % | 13.2 % | high |
| natural colour / teal / paper / warm / B&W / black | 42 / 22 / 13 / 11 / 6 / 6 % | 32 / 7 / 11 / 8 / 18 / 25 % | |
| bed under voice during speech | 3.0 dB | 13.5 dB | 0.8 dB |
| SFX/music onsets / min | 84 | 78 | 113 |
| **onsets within 2 frames of a cut** | **24 %** | **46 %** | **69 %** |
| caption onset vs spoken onset | — | — | **±1 frame** |

**And then the section that makes the whole thing trustworthy:**

> ## What is not measured (decisions)
> - The speaker on camera ~25–35% of runtime (the refs have no narrator on screen).
> - The speaker's camera RAW — no grade, no B&W, no LUT, no vignette; the matte RGB from the
>   raw plate too.
> - Bed ~5.5 LU under the voice, cut (not ducked) on structure — between the references'
>   0.8–3 dB and a quieter talking-head mix.
> - 30fps output; frame counts converted.

**A derived style must record which numbers were measured and which were chosen.** Nothing else in
this folder does that. Every other reference describes a look; this one says *"here is the frame
that 4-second ceiling came from, and here is the four places where we decided for ourselves."*
That single section is the most transferable idea in the artefact — it is what turns "taste" into
something a second person can argue with.

## 4. The eight things a first draft gets wrong

> A first draft built from stills, screenshots and synthetic ticks looked nothing like the
> references. **The frame-by-frame breakdowns are the spec**; where `devices.md`'s first pass and
> the `edl/` files disagree, the `edl/` files win.

| # | the rule | the number |
|---|---|---|
| 1 | **Moving footage, not stills** | 75 % (REF-1), 62 % (REF-2), 50 % (REF-3) of runtime |
| 2 | **Metaphors and puns on most lines** | 15 % (REF-1) / 59 % by time (REF-2) / 31 % (REF-3) |
| 3 | **Everything moves** | a steady 4–7.5 %/s zoom on 62/79 shots; new cuts open defocused and sharpen over ~9 frames (`focus_in=9`); hard punch-ins on the stressed word |
| 4 | **Pace** | ~60 (REF-1) to ~180 (REF-3) images a minute. **A 4-second hold is a mistake.** |
| 5 | **Strobes are shape-matched** | every image in a burst shares one shape, 2–4 frames each, one click each |
| 6 | **Captions change character** | 62 px base · italic serif for a stressed phrase · 2.3× bold for the key word · ink on light pages |
| 7 | **Sound on every change** | clicks only 2–7 dB under the voice, landing 4–14 ms after the picture |
| 8 | **The bed carries the cuts** | runs from frame 0 ~5.5 LU under the voice, **is not ducked**, cuts out under the punchline and slams back on the next line |

**This is finding [03](03-shot-recipe-card-format.md)'s recipe card at the scale of a whole style** —
and the "eight things a first draft gets wrong" framing is the part worth stealing: **write the
failure list, not the description.** A new draft's known failure modes are more actionable than
any adjective.

Rule 4 is the one finding 23 predicted: **a 4-second hold is a mistake**, now with its source
(the held-shot median is 0.56–0.79 s and the p90 is 1.10–1.77 s — 4 s is not a fast take, it is an
outlier), and with the numbers a check would need.

## 5. The spec has a precedence rule, and it records its own errors

`reference/devices.md` opens with a warning that reads like a changelog of being wrong:

> ⚠ **This is the FIRST pass, written before the frame-by-frame breakdowns.**
> `edl/REF-1.md`, `edl/REF-2.md`, `edl/REF-3.md` and `edl/SOUND.md` list every device with
> frame recipes and **mark what this file got wrong** (pop collage pulls OUT 11 %; chapter entry
> is a DOUBLE negative, 2+2 frames; the invert is 1 negative + 1 overexposed frame; REF-1 titles
> sit in FRONT of a cut-out; captions 62px; the strobe is shape-matched; the wall sweep is silent;
> the zoom-through jumps to 2.4x over 2 blurred frames). **Where they disagree, they win.**

**Two mechanisms we do not have.** Our `rules.md`, our method files and our `Look` are all
prose, and prose has no precedence rule against a measurement. Here the *spec* is the data, the
data wins, and the prose is labelled as a first pass with its known errors listed. Finding
[14](14-ffmpeg-skill-contract-and-evals.md) found the same idea in a different form — a schema
derived from the code, and CI that fails on drift. **This is that idea applied to taste.**

And it has a **"Not yet built"** section — devices seen in the references, documented, waiting for
a video that needs them. An honest TODO inside the spec.

## 6. The three checks, and the one that matters most

The workflow (§5) is:

> `hf lint .` (0 errors) → `hf snapshot . --at <every beat> --no-end -o out/snaps` → **look at
> every frame**: legible captions on light pages, nothing covering the eyes, no blank captures, no
> placeholder → `FINAL=1 build.py` → `hf render .`

and then (§7):

> **re-transcribe `out/final.mp4` and diff it against the take: a word the bed or a hit masked shows
> up here.**

**The last one is gate 3, done with a transcript instead of pixels.** It is the third independent
implementation in this folder — Sirio's Eyeline re-transcribes the assembled cut (finding
[22](22-the-open-source-mirror.md)), video-use self-evaluates at cut boundaries (finding
[13](13-the-llm-reads-the-video.md)), and this one re-transcribes the **rendered file**.

And it caught a real defect in a real build, twice:

> A drop or impact ON "Claude" masked the consonant (a re-transcription heard **"Flaw dead"**).

> at −4 LU a dense trap section turned **"a leak" into "illegal"**; −5.5 LU plus a −3 dB settle
> under the key phrase fixed it. Never put a drop or impact ON a short punchline word — **it masks
> the consonant.**

**"Claude did." → "Flaw dead."** That is the defect class finding
[24](24-astra-the-vendors-own-numbers.md) recorded as *"Hear that pop? Terrible"* — an audio defect
at a boundary that no pixel gate can see — and here it is **caught by machine, with the fix
written down** (cut the bed under the punchline, slam back on the next line).

### The kit's checks against our gate

| the kit's check | what we have |
|---|---|
| `hf lint .` → 0 errors | **we have it** — `hyperframes lint`, plus `hyperframes check <stage> --json` with lint/runtime/**contrast**/**layout** sections |
| look at every frame | **we compute it; they let the agent look.** Our `snapshot` deliberately passes `--describe false` — *"Never send frames to a vision model, whatever is in the environment"* |
| legible captions on light pages | partly — our `check` has a contrast section |
| **nothing covering the eyes** | **we have the ingredients and no check** — `FaceBox` carries `eye_y`; nothing reads it |
| no blank captures / no placeholder | n/a — we do not source pages |
| **re-transcribe the final, diff vs the take** | **we do not do this.** We have the transcript stack and the export is a file |
| plan + rights flags, wait for a go | we have `propose_cuts` and the Agent panel; **no plan file, no rights table** |

Two of those rows are cheap and large.

**We already call the exact command the kit uses.** Our `graphics/gate.rs::snapshot()` runs
`hyperframes snapshot <stage> --output <dir> --at <seconds, comma-separated> --no-end
--describe false`. The kit runs `hf snapshot . --at <every beat> --no-end -o out/snaps` — **the same
command, the same comma-separated `--at`, the same `--no-end`.** Ours snapshots *a graphic* at its
own times; theirs snapshots *the whole edit* at every beat. **The frame set gate 3 needs is one
argument away.**

## 7. The devices are a library of structural sections — with a routing rule

Thirty-odd devices, each with a measured spec, an explicit z-index, frame-snapped timing, and its
own sound cues. `tile_reveal` (9 panes, 1/frame, white 1f then picture, a click each) ·
`cutout_world` (the hero) · `pop_collage` / `pop_shatter` · `strobe` · `wall3d` · `page_card` ·
`calendar` · `terminal` / `code_scroll` / `typewriter` · `slap_caps` · `neg_flash` · `keyword` ·
`edit_timeline` · `filmstrip` · `player_window` · and eight more.

**The routing rule is the most transferable sentence in the kit:**

> **Pick the device from what the speaker SAYS**, not from what is free in the timeline — the table
> in SKILL.md §2 is the routing.

*the speaker says "the API" → a terminal; says "every few weeks" → a calendar of real dates; says
"leaked" → a datamosh.* **The semantics pick the treatment; the timeline only says when.**

Finding [20](20-the-runtimes-author-on-the-record.md) recorded that HyperFrames' desktop app has
`moments` — *"a library of reusable structural sections"*. This is that idea as a **skill**, on the
runtime we ship, with measured specs and a semantic router. Our `Look` holds skin; our presets
hold rules and steps. **We have no library of named structural sections and no router.**

The treatments are a closed set with measured shares — `teal` split (REF-1 22 %), `mono` hard B&W
(REF-2 18 %), `warm` amber (8–11 %), `invert` negative, `natural` + push + vignette (32–42 %),
`recolor FROM-TO:TARGET`, `halftone` — and one hard exception:

> **Your face: NOT graded.** Your camera stays RAW on every face beat: no grade, no B&W, no LUT,
> no vignette — **the found material carries the colour.**

That is a *treatment model with a default and a named exception*, which is exactly what our `Look`
lacks and what finding [01](01-treatment-model-and-visual-lead.md) argued for: the face is one
treatment among others, not the default.

## 8. The sound: cues are written by the devices

> The devices cue the clicks, trains, hits and glitches into `sfx-cues.txt`; **never hand-edit it,
> or the sound and the picture drift apart on the first re-time.**

**We already do exactly this.** Our `sound_effects.rs` says: *"The events are worked out from the
edit document alone and mixed as a function of timeline time, so playback and export make the same
samples however they read them."* Two independent implementations of *plan-derived audio*.

Their version adds three things we do not have:

- **A measured justification.** *"The references are dense: 78–113 SFX/music onsets a minute, and
  24–69 % of them land within two frames of a cut. Sound is half of what makes the stream of
  pictures feel cut rather than slid."* Our `sound_effects.rs` says *"Never overdone"* — a taste
  rule. This is a taste rule with a number.
- **A role table with gain and pre-roll.** 12 roles, each with its file, gain, pre-roll and the
  device that uses it — pre-roll compensates for *"the 4–14 ms the reference clicks LAG the
  picture."* A click that lands 10 ms late reads as sloppy.
- **22 sounds synthesised in code** to the measured length, transient and level of the reference
  sound of the same class — so *"the `SND` gains and pre-rolls hold"* and no sample is copied. We
  synthesise our sounds too, from the same principle.

And the rule that matters: **cut the bed, do not duck it.** Our `sound_effects.rs` has `DUCK_DB =
8.0` — we duck the effects lane under speech. Their bed *stops* under the punchline and slams back,
because a duck leaves the consonant audible over a moving bed. **A duck and a cut are different
operations and the difference is exactly where the masking defect lives.**

## 9. The two-pass build, and the video that shows itself

> `edit_timeline`, `filmstrip`, `player_window`, the "every picture" strobe and the render-log
> `code_scroll` need the video to exist first. Build in two passes.

`edit_timeline` is the device to look at:

> the real timeline of THIS edit: **V1 shot thumbs, A1 words at their onsets (+ waveform), FX =
> every cue**; the playhead sweeps and **each cut it crosses flashes a guide to the word it lands
> on**, a click each

**That is the artefact our `storyboard` tool lacks.** Ours renders the finished edit as one PNG —
a tile per scene, with what is said — and **does not mark where a cut starts, has no waveform, and
maps nothing to a word.** The kit's version marks the cuts, carries the waveform, lists every
sound cue, and maps each cut to the word it lands on.

**And it is the device we would want for gate 3**, because a cut→word map is a checkable artefact:
every cut should land on a word, and the kit's own measurement says the reference captions land
within **±1 frame** of the spoken onset.

Two passes is also a mechanism we do not have: *PASS=1 renders, PASS=2 adds the beats that show
the edit itself.* Our renderer has no equivalent, and no need for one yet — but it is how a video
can display its own timeline, which is a way of making the edit's structure legible without a
human describing it.

## 10. What we would take

1. **Re-transcribe the final and diff it against the take.** The cheapest gate 3 in this folder.
   We already have the transcript stack, the export is a file, and the defect it catches
   (*"Claude did." → "Flaw dead"*) is invisible to every check we have. It also catches the
   pause-trimming class from finding 24 — a word the edit destroyed shows up in the diff.
2. **Snapshot the whole edit at every beat.** We already call `hyperframes snapshot --at a,b,c`.
   One argument away from the frame set a visual check needs. (And decide whether `--describe
   false` should stay: the kit's model *looks*; ours computes. Both are defensible — but ours
   currently does neither for the edit.)
3. **A check that nothing covers the eyes.** We have `FaceBox` with `eye_y` and no check reads it.
4. **Separate measurement from decision in anything we derive.** The *"What is not measured
   (decisions)"* section is the template, and it should be the shape of `GFX-5`'s output and of
   any `Look` we derive from a reference.
5. **A device library with a semantic router.** *"Pick the device from what the speaker SAYS."*
   We have presets and a `Look`; we have no named structural sections with measured specs and no
   router that reads the words to choose one.
6. **The max-hold rule with its number.** 4 s is a mistake; the median is 0.56–0.79 s and the p90
   is 1.10–1.77 s. This is gate 3's first pixel-free rule and it now has a source.
7. **A sources table with licence and FLAG, in a plan file, shown before the build.** We record no
   source and no licence for anything we put on screen.
8. **A cut→word map in the storyboard.** Our storyboard is one PNG; theirs is a timeline with a
   playhead, a waveform, every cue, and each cut mapped to its word.
9. **Cut the bed, don't duck it** — and never put a transient on a word.

## What this implies outside this folder — **not applied**

This folder is the only place anything was changed. For the record, here is what a maintainer would
want to change elsewhere — **none of it has been applied**:

- **`roadmap.md` → GFX-5** would gain its second, concrete answer: *the artefact is a derivation
  record — a method, the raw numbers, and an explicit "what was not measured" section — and it
  exists, free, for another runtime we already ship.* The licence permits study and
  reimplementation; it does not permit shipping it inside AeroEdits.
- **`roadmap.md` → PLAN-1** would gain the plan-as-file shape: *a plan file with a beat table, a
  sources table with licences and FLAGs, shown before the build, and a wait for a go.*
- **`gaps.md` → §7 (Style)** would gain rows: *no derived-style record*; *no device library*; *no
  semantic router*; *no precedence rule between a method doc and a measurement*.
- **`gaps.md` → §11 (Transcription)** would gain a row: *the final render is never re-transcribed
  and diffed against the take.*
- **`gaps.md` → assets** would gain a row: *no source, licence or rights flag for any asset* — and
  a row for *no person cut-out*.
- **`gaps.md` → §5 (Intent)** would gain a row: *the face is the default treatment; the reference
  makes found material the default and the face the exception.*
- **`research/competitors.md` → "At a glance"** would gain a row for **NextPhase Editor V1** —
  a named style derived by measuring references, shipped as a skill on our own runtime, with a
  derivation record and three checks.
- **`research/creator-workflows.md` → artefact index** would gain **Editor V1** (Google Drive,
  79 files, bespoke licence: free including commercial, no resale/repackaging as a paid product;
  third-party: SIL OFL fonts, GSAP no-charge, HyperFrames Apache-2.0, RVM GPL-3.0 runtime
  component). **Adoptable outright as long as AeroEdits is not sold as a repackaging of it.**
- **`research/creator-workflows.md` → "Tricks worth stealing"** would gain six rows: derive a
  style by measuring references and write down what was *not* measured; keep a spec that records
  its own errors and lets the data win; route a device by what the speaker says; re-transcribe the
  final and diff it; cut the bed rather than ducking it; never put a transient on a word.
- **`research/creator-workflows.md` → "Corrections"** would gain one note: **finding 23's
  inference that the kit's ~1/3 face share independently confirms iart-ai's three-treatments model
  is wrong as stated** — the kit marks that share as *"a decision, not a measurement"*. The *rule*
  converges; the number does not.

## Caveats

- **The kit was read, not installed, and not run.** Nothing here was reproduced: no frame was
  analysed, no device was rendered, no measurement was checked.
- **The measurements are the author's**, produced on three reels he names only by description. The
  kit says the raw analysis is not shipped — *"it is frames and audio of other people's videos"* —
  so REF-1..3 cannot be independently identified from the kit alone.
- **The licence is bespoke, not OSI.** *"Please don't resell it or repackage it as a paid
  product"* is the author's request rather than a legal opinion, but it is the reason the verdict
  is "use it, don't ship it". If we ever wanted to ship it, that clause is the thing to take to
  the author first — not something to read around.
- **The kit pins HyperFrames 0.8.181; we pin 0.8.162.** Its `hf snapshot --at <every beat>` usage
  is good evidence that the command takes comma-separated timestamps in a version close to ours —
  but 0.8.162 specifically should be checked before relying on it.
- **Robust Video Matting is GPL-3.0 and is downloaded at runtime**, not distributed in the zip. It
  is also a component we have no equivalent of: we detect faces, we cannot matte a person.
- **Our own loudness default differs.** The kit's worked example is −18.9 LUFS integrated; our
  export normalises to `AEROEDITS_LOUDNESS_TARGET` (default −14), and we already implement
  K-weighted `integrated_lufs`. Whether our target is *too loud* for the bed-under-voice grammar
  is a question we can answer with the code we have — it was not answered here.
- **The kit's own advice on our default model**: its worked example's timing comes from
  **parakeet**, which is the local model we ship but do not default to (finding 23 §5).
