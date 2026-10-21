# AeroVoiceLab: local re-voicing test (17 Oct 2026)

A test bench, separate from the Automated-Editor app, for replacing George's accented English
with a synced, locally generated copy of his own voice. Deleting this folder removes everything
(models, Python environments, caches); nothing here touches the app or the original recording.

Test material: chunk 21 (55 s) of `Documents\AeroShootRec\Untitled 16 Oct 2026.aero`.

## Where to look first

`outputs\review\`
| File | What it is | How it was made |
|---|---|---|
| `0_original_your_voice.mp4` | Your webcam minute with your real mic audio | Untouched |
| `1_CURRENT_BEST_voxcpm2_direct_clone.mp4` | Previous best | VoxCPM2 (Apache) clones you from a 25 s reference with a "neutral American accent" instruction |
| `3_chatterbox_turbo_direct_clone.mp4` | Chatterbox Turbo (MIT) cloning you | 11 s reference (`inputs\his_ref10.wav`) |
| `4_qwen3_base_voiceprint_clone.mp4` | Qwen3-TTS 1.7B Base (Apache), voiceprint only | Copies your timbre from a speaker embedding, not your way of speaking |
| `5_qwen3_base_full_clone.mp4` | Qwen3-TTS 1.7B Base, full cloning | Continues from the 11 s reference and its transcript |
| `6_longcat_audiodit_3.5b_clone.mp4` | LongCat-AudioDiT 3.5B (MIT) | Continues from the 11 s reference; each sentence made exactly as long as you took |
| `comparison.md` | The score table for all of the above | `scripts\evaluate.py` |
| `archive\` | Rounds one and two (native voices, OpenVoice, RVC, stretching, MuseTalk, LatentSync) | Kept for reference |

Every clip 1-6 is generated one whole sentence at a time, cut into your phrases at the quiet
points between words, and placed so each word starts where yours did. No time-stretching (it
caused the metallic sound), no voice conversion, no added room noise, no lip sync.

## Folders

| Folder | Contents |
|---|---|
| `inputs\` | The test minute (`test_mic.wav`, `test_webcam.mp4`), its words (`test_words.json`), references of your voice (`his_ref10.wav` + `.txt`, `his_reference.wav`), and 15 min of your clips (`rvc_dataset\`) |
| `outputs\tts\<model>\` | Raw generated sentences (`sNN.wav`), the phrase files cut from them (`pNN.wav`), and `run.json` (speed, VRAM) |
| `outputs\final\` | Each candidate fitted to your timing: `<name>_word_nostretch.wav` and `.mp4`, plus `metrics.json` (timing errors) |
| `outputs\original_aligned.json` | Word-by-word times of your original take (wav2vec2 aligner) |
| `outputs\vc\` | Round two voice-conversion outputs (RVC, OpenVoice) |
| `models\rvc_george\` | Your trained RVC voice at pass 50, plus the checkpoints to resume training |
| `envs\` | One Python environment per model: `voxcpm`, `chatterbox`, `qwen`, `longcat` |
| `src\` | Model code: VoxCPM, chatterbox, Qwen3-TTS, LongCat-AudioDiT |
| `cache\` | Downloaded weights (`hf\`), packages (`uv\`), Python builds |
| `logs\` | One log per install or run |

## Scripts (`scripts\`)

| Script | What it does |
|---|---|
| `prep.py` | Cuts the test minute and the 15 min of training clips out of the recording (maps transcript times past the recording pauses) |
| `make_ref.py` | Cuts the 11 s reference clip + exact transcript from chunk 22 |
| `common.py` | Shared bits: phrases, sentences, original sentence durations, reference paths, speed/VRAM meter |
| `gen_voxcpm_clone.py` | VoxCPM2 direct clone |
| `gen_chatterbox_clone.py` | Chatterbox Turbo direct clone |
| `gen_qwen_clone.py xvec / icl` | Qwen3-TTS Base, voiceprint-only or full cloning |
| `gen_longcat.py` | LongCat-AudioDiT 3.5B, each sentence at your duration |
| `split.py <folder>` | Cuts whole-sentence takes into your phrases at the quiet points |
| `fit.py name=folder` | Places every word at your original onset and writes the .wav/.mp4. `FIT_STRETCH=0` (no stretching), `FIT_ROOMTONE=0` (no added room noise) |
| `align.py` | wav2vec2 forced aligner (the app's own model, on CPU) |
| `check_tts.py <folder>` | Quick clarity check of one folder |
| `evaluate.py name=folder ...` | Writes `outputs\review\comparison.md` |
| `run.sh <env> <script>` | Runs a script in an environment with caches kept inside the lab |
| `install.sh <env>` | How the environments were built |
| Older: `gen_voxcpm.py`, `gen_chatterbox.py`, `gen_qwen.py`, `vc_openvoice.py`, `vc_rvc.py`, `rvc_train.py`, `lipsync*.sh`, `gen_zonos.py` | Round one and two; their environments are deleted |

To regenerate one candidate, for example Qwen voiceprint:

    bash scripts/run.sh qwen gen_qwen_clone.py xvec
    envs/voxcpm/Scripts/python.exe scripts/split.py outputs/tts/qwen_clone_xvec
    FIT_ROOMTONE=0 FIT_STRETCH=0 FIT_TAG=_nostretch envs/voxcpm/Scripts/python.exe scripts/fit.py qwen_clone_xvec=outputs/tts/qwen_clone_xvec
