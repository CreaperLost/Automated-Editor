# Jad M.H: NextPhase Editor V1 ("The Documentary Edit") In-Depth Research Report

- **Video Source:** [youtube.com/watch?v=ge1yzQ5MXmM](https://www.youtube.com/watch?v=ge1yzQ5MXmM)
- **Code & Skill Source:** Google Drive Archive (`NextPhase-EditorV1.zip` / `skill/editor-v1/`)
- **Creator:** Jad M.H (NextPhase)
- **Title:** *The Only AI Video Editing You Will Ever Need (Turned it Into A Free Skill)*
- **Skill Name:** `editor-v1` ("the documentary edit")
- **Category:** Open-Source Documentary NLE Agent Skill, Frame-by-Frame Reverse-Engineered Grammar & Procedural Audio Engine
- **Analyzed for:** AeroEdits AI Agent & Motion Graphics Architecture

---

## 1. Executive Summary

Inspection of the actual distribution archive (`NextPhase-EditorV1.zip`) reveals that Jad M.H’s system is not merely a generic prompt pack, but **one of the most meticulously engineered, empirical AI video production skills created to date**.

Dubbed **NextPhase Editor V1 (`editor-v1`)**, the skill transforms raw talking-head recordings into fast-paced, high-retention documentary edits (the visual grammar popularized by Vox, Johnny Harris, MagnatesMedia, and tech founder reels). The skill was **reverse-engineered frame-by-frame off three high-performing documentary reels** (`REF-1` founder story, `REF-2` game theory explainer, `REF-3` company origin story).

It operates locally on **HyperFrames + FFmpeg + Python**, executing an end-to-end pipeline:
1. Automated sourcing of real evidence online (company launch films, archive footage, headless web captures).
2. Systematic visual treatment (teal split, hard B&W, amber, halftone shadows), while **keeping the speaker's face plate strictly raw**.
3. Background matting (`matte.py`) allowing world swaps and a **560px red "Mega Word"** to sit *behind* the speaker’s head.
4. An empirical library of **18 named animation devices** running at a blistering velocity of **a new picture every 0.6–1.5 seconds**.
5. Procedurally synthesized, transient-calibrated sound design (`synth-sfx.py`) with click rotation and structural music bed cuts.

---

## 2. The Empirical Foundation: Frame-by-Frame Reel Measurements

Unlike typical prompt templates based on subjective taste, `editor-v1` is derived from mathematical measurements of 3 top documentary reels:

| Measured Metric | Reference 1 (Founder Story, 78.4s) | Reference 2 (Game Theory, 93.6s) | Reference 3 (Origin Story, 70.5s) | NextPhase Specification |
| :--- | :--- | :--- | :--- | :--- |
| **Pacing (Hold Times)** | Median 0.79s per shot | Median 0.56s per shot | Median 0.68s per shot | **0.6s to 1.5s per picture** (60–180 images/min). |
| **Motion vs. Stills** | 75% moving footage | 62% moving footage | 50% moving footage | **$\ge$ 60% moving clips**; static holds are banned. |
| **Metaphors & Puns** | 15% of lines | 59% of runtime | 31% of lines | Metaphors on every conceptual line (*"leaked"* $\to$ dripping tap; *"nothing"* $\to$ tumbleweed). |
| **Internal Drift** | Steady 4–7.5%/s zoom on 62/79 shots | Constant camera push | Stepped window growth | **`broll(rate=0.05)`**: 5%/s continuous push; cuts open defocused (`focus_in=9`). |
| **Acoustic Density** | 78 SFX onsets/min | 113 SFX onsets/min | 84 SFX onsets/min | Clicks on every change, landing 4–14ms after picture. |
| **Music Relationship**| Bed 0.8–3 dB under voice | Bed 13.5 dB under voice | Bed 2.5 dB under voice | **Bed ~5.5 LU under voice; CUTS OUT under punchlines.** |

---

## 3. The 18 Named Animation Devices (`components/ev1_devices.py`)

Every device in the NextPhase catalog is mapped to semantic speech cues (`SKILL.md §2` routing):

```
┌────────────────────────────────────────────────────────────────────────┐
│                        NextPhase Device Taxonomy                       │
├───────────────────┬────────────────────────────────────────────────────┤
│ Category          │ Named Devices & Frame Mechanics                    │
├───────────────────┼────────────────────────────────────────────────────┤
│ World Swaps       │ • `tile_reveal`: 9 panes on thirds grid, 1f white  │
│                   │   flash, 1 click per pane (9f end-to-end)          │
│                   │ • `cutout_world`: Speaker cut-out persists while   │
│                   │   world swaps; 560px red "Mega Word" behind head   │
├───────────────────┼────────────────────────────────────────────────────┤
│ Rapid Multi-Item  │ • `pop_collage`: 1 card pops per word every 2f,    │
│                   │   preceded by 1f white box, with click per pop     │
│                   │ • `pop_shatter`: Stack pulls out in 3D & shatters  │
│                   │ • `wall3d`: Perspective plane 0.8-3.5s camera      │
│                   │   sweep across endless screenshots; motion blurred │
├───────────────────┼────────────────────────────────────────────────────┤
│ Punctuation       │ • `strobe`: 2-3f shape-matched images with hopping │
│                   │   square & rapid click train on abstract nouns     │
│                   │ • `neg_flash` / `double_neg`: 1-2f negative pulses │
│                   │ • `datamosh`: 3-4f RGB glitch slices on hard turns │
├───────────────────┼────────────────────────────────────────────────────┤
│ Evidence & UI     │ • `page_card` / `focus_card`: Rack-focus in with   │
│                   │   highlighter swept over OCR-measured text boxes   │
│                   │ • `terminal` / `code_scroll`: Real code with       │
│                   │   blinking caret and active line markers           │
│                   │ • `calendar`: Real dates with cards landing on days│
│                   │ • `black_card`: Minimal word on black; bed drops   │
├───────────────────┼────────────────────────────────────────────────────┤
│ Meta Editing      │ • `silence_cut` / `edit_timeline`: Displays real   │
│                   │   audio waveforms and cut markers on screen        │
└───────────────────┴────────────────────────────────────────────────────┘
```

### Layer Stacking Order
To prevent visual clipping, NextPhase enforces a strict z-index hierarchy:
$$\text{Ground} \longrightarrow \text{Tile Panes} \longrightarrow \text{Display Word} \longrightarrow \mathbf{\text{Speaker's Matte}} \longrightarrow \text{Captions} \longrightarrow \text{Grid/Vignette} \longrightarrow \text{Flashes}$$

---

## 4. Sourcing & Treatment Engine (`scripts/`)

### 4.1 Real Evidence Over Generative Hallucinations (`find-footage.py` & `capture.mjs`)
NextPhase explicitly bans AI video diffusion (Sora, Runway) for software and documentary topics:
- **`find-footage.py`**: Queries public-domain and open archives (Archive.org Prelinger Archive, Wikimedia Commons, NASA, Mixkit) via API and scraping.
- **`capture.mjs`**: Headless Chrome script that visits first-party corporate websites, automatically dismisses cookie banners, triggers scroll-reveals, strips emoji clutter, and captures crisp 4K viewport assets.

### 4.2 The Color Treatment Rule (`ev1_treat.py`)
- **Found Footage & Screenshots:** 58% to 68% of shots receive a stylized color grade (teal split with halftone shadows, hard monochrome black & white, or warm amber).
- **The Speaker Exception:** The speaker's webcam plate is kept **100% RAW**. No LUT, no vignette, no B&W. The contrast between the natural human face and the hyper-stylized background creates the documentary aesthetic.

### 4.3 Background Matting (`matte.py`)
Runs robust video matting (RVM / BiRefNet) on the raw camera plate, downsampling to $0.3\times$ for speed, outputting a VP9 WebM with alpha. The matte allows placing the world and large typography behind the speaker without a green screen.

---

## 5. Acoustic Engineering & Procedural SFX (`synth-sfx.py` & `bed-mix.py`)

### 5.1 Pure Procedural Synthesis (Zero External Copyright Audio)
Instead of shipping copyrighted sound effect libraries, `synth-sfx.py` **synthesizes 22 distinct sound effects procedurally** using Python math:
- Clicks, click trains (1 click per frame for tile reveals), swishes, glitches, sub-booms, and risers.
- Each sound is tuned to the exact transient offset, frequency peak, and decay length measured from the reference reels.
- **Click Rotation (`rotate_clicks`):** Rotates across 4 synthesized click variations (`click_1..4`) to prevent the auditory "metronome effect".

### 5.2 Music Bed Architecture: Cut, Don't Duck (`bed-mix.py`)
Standard editors duck music gradually under speech. NextPhase rejects continuous ducking:
- The music bed runs at a fixed level (~5.5 LUFS below dialogue).
- **The Punchline Cut:** Under punchlines or shocking revelations, the music bed is abruptly cut to $-\infty$ dB (`--ramp a,b,-70`). The speaker's punchline plays **completely dry**.
- The music slams back in with an impact hit on the very next sentence.
- **Intelligibility Verification:** The pipeline re-transcribes the final rendered mix via Whisper. If a sound effect or music drop caused a word to be misrecognized, the mix is automatically flagged and attenuated.

---

## 6. AeroEdits vs. NextPhase Comparison

| Capability | NextPhase Editor V1 | AeroEdits |
| :--- | :--- | :--- |
| **Aesthetic Grammar** | Fast documentary found-footage edit (0.6–1.5s per picture). | Technical tutorial, screencast, and short-form layout editor. |
| **Motion Technology** | Standalone **HyperFrames** (Node.js CLI via Claude Code). | Pinned, embedded **HyperFrames + GSAP 3.15** runtime in Tauri desktop app. |
| **Speaker Presence** | ~25–35% on camera; matting for `cutout_world`. | Full-time webcam bubble or split-screen layouts; YuNet face auto-centering. |
| **Telemetry Support**| None (relies purely on speech audio and web scraping). | **Deep mouse dwell, click clusters, active window bounds, typing carets.** |
| **Sound Design** | 22 procedurally synthesized SFX + structural bed cuts. | Roadmap item `SND-2` (SFX cues at graphic events); -14 LUFS normalization. |
| **Workflow Safety** | Single terminal batch render. | Non-destructive multi-gigabyte undo/redo (`history/`) + named checkpoints. |

---

## 7. Actionable Takeaways for AeroEdits

### 🟢 What We Should Adopt

1. **Incorporate the Top NextPhase Devices into AeroEdits HyperFrames Library:**
   - **`tile_reveal`:** 9-pane thirds grid with white 1-frame flash and synchronized clicks for opening chapters.
   - **`pop_collage`:** Staggered card reveals popping on individual words.
   - **`wall3d`:** 3D perspective plane with camera sweep for "everyone / everywhere" claims.
   - **`page_card` with OCR Highlight:** Real document cards with highlighter pens sweeping across text.
2. **Procedural SFX Engine (`synth-sfx.py`):**
   - Port NextPhase's procedural audio synthesizer into AeroEdits. By generating sound effects mathematically in code, AeroEdits can ship a rich library of clicks, whooshes, and sub-booms without licensing costs or external audio asset bloat.
3. **The "Punchline Cut" Music Bed Automation (`SND-3`):**
   - In AeroEdits' audio engine, support structural music cuts: allowing the agent to drop background music to zero under designated punchline sentences, slamming it back on the next phrase.
4. **Click Rotation to Prevent Auditory Fatigue:**
   - When graphics trigger sequential clicks (e.g. counters or bullet cards), automatically cycle through 4 audio micro-variations.
5. **Intelligibility Re-Transcription Check:**
   - Run a quick Whisper re-transcription over the final export mix. If dialogue recognition confidence drops near an SFX hit, automatically attenuate the effect by 3dB.

### 🔴 What We Refuse / Avoid

1. **Do NOT abandon tutorial screencast telemetry for 100% b-roll saturation:**
   - NextPhase replaces the screen every 0.8 seconds with found footage. This is brilliant for storytelling reels, but fatal for software tutorials where viewers need to follow code syntax and mouse cursor navigation. In AeroEdits, high-velocity documentary pacing belongs in cold opens and concept breaks, while the main body stays anchored to screen telemetry.
2. **Do NOT require complex Python PyTorch matting dependencies on end-user machines:**
   - NextPhase requires a 1 GB PyTorch install for background matting. AeroEdits must keep its lightweight footprint, prioritizing lightweight WebGL or ONNX-based background removal if matting is introduced.
