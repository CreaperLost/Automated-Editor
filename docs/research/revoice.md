# Re-voice and dubbing: learnings (paused 18 Oct 2026)

**Goal.** Replace George's accented English with his own voice speaking clear, native-sounding
English, in sync with the video (webcam face visible: small overlay in tutorials, half the
frame in Shorts). Run locally on his PC at no per-video cost. Every model must be licensed
for users who make monetized videos and ads.

**Outcome.** Paused. Two days of testing (17-18 Oct) found no local, commercially usable
combination good enough to ship:

- **Voice:** single-step clones sound like him but keep his accent; native voices lose the
  accent but sound robotic or processed once converted to his timbre.
- **Lip sync:** both usable models (MuseTalk 1.5, LatentSync 1.5) visibly degrade his mouth at
  256 px. The sharper versions need 18 GB+ of VRAM, or have non-commercial licences.

Revisit when open models improve (see "When to revisit").

## Setup that was tested

- **Hardware:** RTX 3060 **12 GB** (not 8 GB as first assumed), 16 GB RAM, Windows 11.
- **Test material:** 55 s webcam minute, chunk 21 of the 16 Oct recording: 124 words in 11
  phrases. 15 min of his other speech for training, and an 11 s reference clip with its
  transcript from chunk 22.
- **Lab:** a separate folder (`Documents\AeroVoiceLab`) with one Python environment per model.
  The lab's scripts were removed from the repo on 24 Oct 2026; read them with
  `git show bee77ae:docs/research/revoice-lab-scripts/<file>` (README.md lists them).

## Licence rule (his decision)

- **Allowed:** MIT, Apache-2.0, GPL-3.0 (keep it a separate component, or put the app under
  GPL), OpenRAIL (pass its use restrictions on to users).
- **Not allowed:** non-commercial, research-only, or revenue-capped licences. The app is free
  and open source, but its users publish monetized videos and ads, which counts as commercial
  use.
- **Ruled out by licence:**
  - F5-TTS: weights CC-BY-NC.
  - Voxtral TTS: CC-BY-NC.
  - OmniVoice: weights CC-BY-NC. The code is Apache, which misleads many blogs.
  - Breeze TTS 2: research only.
  - Fish Audio S2 / Drama 3: research licence, or community licence capped at $500k revenue.
  - XTTS v2: CPML, and Coqui no longer exists to sell a licence.
  - Wav2Lip: non-commercial.
  - InsightFace's pretrained face models: non-commercial. LatentSync uses them; swap in YuNet.
- **Check licences on the Hugging Face card and the LICENSE file:** code and weights often
  differ.

## What we learned, by stage

### Timing / sync (our own code, the most reusable part)

- **Word timings:** the app's wav2vec2 aligner works well on generated speech too (forced CTC
  alignment), both to find word onsets and as a rough "clarity" score.
- **Time-stretching is what caused the "metallic" sound.** Rubber Band at up to 1.35x was the
  culprit; he confirmed it by listening stage by stage. Avoid stretching speech more than about
  ±8%.
- **Pinning every word to his onsets** (cut at word boundaries, place each word) made fluent
  speech choppy. He said the raw sentences sounded clearly better than the synced clips.
- **Phrase placement** (each phrase between his pauses starts where his did, untouched inside)
  sounds natural, but words drift 0.2-0.4 s at the median and up to 2 s in long run-on
  phrases. A phrase must never start before the previous one ends.
- **Generate whole sentences, never pause-split fragments.** Fragments like "You can" or
  "So the first skill" are read flat and robotic.
- **Clipped word endings:**
  - Cut at the quietest point before a word, not at its onset.
  - Fade out over about 60 ms before an inserted pause.
  - Remember the aligner marks a word's end where its last letter *starts*.
- **Duration as an input** (LongCat-AudioDiT) gives his sentence lengths exactly, but the
  model fills his pauses with slower speech instead of leaving gaps.
- **Room tone:** adding a looped bed of his room noise under clean generated speech sounded
  like "noise correction". Leave it out.

### Voice

- **Cloning carries the accent.** Every single-step clone (VoxCPM2, Qwen3-TTS, Chatterbox,
  LongCat) sounded like him *with* his accent. That held even for Qwen's voiceprint-only mode
  and VoxCPM2's "neutral American accent" instruction. Benchmark "SIM" scores reward copying
  the speaker, accent included.
- **Two-stage (native TTS, then voice conversion)** is the only route that removes the accent.
  - Round one: robotic, because I copied a synthetic copy of a voice and generated fragments.
  - RVC at pass 50 and OpenVoice both judged unfairly, because the stretching masked them.
  - Never tested properly: a curated native speaker (Qwen3-TTS CustomVoice) converted with
    Seed-VC v2 or Chatterbox VC.
- **ElevenLabs** produces a near-perfect clone from 30 s; local open models are not there yet.

### Lip sync

- **MuseTalk 1.5** (MIT): repaints the lower face at 256 px. To him it looked like "a blur
  filter over my mouth".
- **LatentSync 1.5** (Apache code, OpenRAIL++ weights): keeps beard texture and is better than
  MuseTalk, but still "not good enough".
  - Also 256 px, takes 17 min per minute of video, and needs 25 fps input.
  - Version 1.6 (512 px) needs 18 GB of VRAM.
- **Not tested, too heavy for 12 GB / 16 GB RAM:**
  - InfiniteTalk (Apache, Wan 14B).
  - LTX LipDub (22B; licence unclear).
- **Practical tricks that worked:**
  - Run lip sync on a 600x600 face crop of the webcam stream and paste it back. The full 720p
    minute ran 16 GB of RAM out of memory.
  - Do it only on re-voiced segments.

## Measured results (single-step clones, 18 Oct)

- **Voice match:** a speaker-encoder cosine (Chatterbox's encoder) against 2 min of his real
  speech. The ceiling is his own minute, 0.978.
- **Clarity:** wav2vec2 confidence reading the known words. His accented original scores
  0.877; native TTS 0.95-0.97.
- **RTF:** generation time per second of audio, model already loaded.
- **Timing:** word-onset error after the word fit, median / 90th percentile.

| Model (licence) | Voice match | Clarity | Peak VRAM | RTF | Timing |
|---|---|---|---|---|---|
| VoxCPM2 2B, clone + accent instruction (Apache) | 0.967 | 0.922 | 6.5 GB | 3.6 | 40 / 254 ms |
| Qwen3-TTS 1.7B Base, voiceprint only (Apache) | 0.967 | **0.956** | 4.4 GB | 5.2 | 70 / 294 ms |
| Qwen3-TTS 1.7B Base, full clone (Apache) | 0.960 | 0.923 | 4.6 GB | 4.8 | 40 / 240 ms |
| LongCat-AudioDiT 3.5B, bf16 (MIT) | 0.966 | 0.938 | 8.1 GB | **0.63** | 140 / 774 ms |
| Chatterbox Turbo, clone (MIT) | 0.941 | 0.934 | 3.3 GB | 0.99 | 100 / 340 ms |

**His verdict:** all of them were "trash" once synced. The raw sentences were better, but the
accent was still there.

**Published Seed-TTS-eval test-EN (WER / SIM):**
- VoxCPM2: 1.84 / 75.3
- Qwen3-TTS: 1.23 / 71.7
- LongCat-AudioDiT 3.5B: 1.50 / 78.6
- MOSS-TTS: 1.85 / 73.4
- Fish S2 (licence out): 0.99 / –

## Dead ends and gotchas (Windows)

- **PyTorch from PyPI on Windows is CPU-only.** Install torch with `uv pip --torch-backend cuXXX`,
  then pin it with a constraints file before installing anything else.
- **torchaudio 2.9+ can't load audio** without FFmpeg shared DLLs (it uses torchcodec), and
  crashed silently in VoxCPM. Patch `torchaudio.load` to use soundfile.
- **ZONOS2 Q8** (Apache weights; quantised GGUF, Windows Vulkan build ran at RTF 0.6):
  - The CLI stops at 400 frames (4.6 s) unless `--max` is set.
  - Cloning his voice babbled (3x-long nonsense) with a raw wav reference, and was still poor
    with a normalised mp3.
  - The zonos2.cpp runtime has no licence file, and its binaries are unsigned.
- **Applio/RVC:**
  - Training ran at 30 s per pass over 15 min of audio.
  - Training resumes from its G_/D_ checkpoints.
  - pyin pitch estimates had octave errors; use RMVPE.
- **MuseTalk on Windows:**
  - Needs `setuptools<70` (mmengine imports pkg_resources).
  - Needs `chumpy` built without build isolation.
  - Needs UTF-8 console output.
- **OpenVoice** passes `enable_watermark` to a base class that rejects it (upstream bug).
- **Downloads ran at a few MB/s.** Two uv installs at once fought over the cache ("Access is
  denied" on rename).
- **VoiceStudio** (the popular "local ElevenLabs", AGPL) is a wrapper. Its default engine is
  OmniVoice (non-commercial), and AGPL would bind the app.

## If we pick this up again

1. **Sync:** use phrase placement, whole-sentence generation, no stretching and no room tone.
   Reuse `fit.py` (`FIT_MODES=phrase FIT_STRETCH=0 FIT_ROOMTONE=0`), `split.py` and `align.py`.
2. **Voice:** test the untested route first: a curated native speaker (Qwen3-TTS CustomVoice)
   converted with Seed-VC v2 (GPL) or Chatterbox VC (MIT).
3. **Watch for:**
   - an open, commercially licensed model that clones timbre without accent (e.g. the
     accent-steering TTS paper, arXiv 2603.05977, if code ships);
   - ZONOS2 cloning that works on Windows;
   - a 512 px lip-sync model that fits 12 GB under MIT, Apache or OpenRAIL;
   - LatentSync 1.6 memory optimisations.
4. **YouTube:** auto-dubbing into 27 languages is free for all creators. Clean English in, then
   let YouTube dub; our own dubs aren't worth building.

## When to revisit

When any of these appears on Hugging Face or the Artificial Analysis open-weights arena with
an allowed licence:
- an English TTS whose clone keeps timbre but not accent;
- a voice converter that beats Seed-VC;
- a lip-sync model at 512 px under 12 GB.
