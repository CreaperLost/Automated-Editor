# Word alignment (plan, 2026-10-17)

Give every transcript word an accurate start and end, measured against the audio, so cuts,
the silence/gap passes and the check before export stop trusting loose timings.

## What is wrong today (measured on George's projects)

| Transcript | Provider | Words | Words touching the next | Longest "word" |
|---|---|---|---|---|
| Untitled 16 Oct 2026 | Parakeet | 2495 | 1797 | 2.37 s |
| Untitled 13 Oct 2026 | Parakeet | 939 | 619 | 1.52 s |
| Tutorial 16 Oct 2026 (same recording) | ElevenLabs | 2533 | 10 | 3.38 s |
| Agent test | ElevenLabs | 2895 | 19 | 2.48 s |

- Parakeet times words on an 80 ms grid and stretches them over the pauses next to them.
  `tighten_loose_words` patches this by trimming quiet at the edges.
- ElevenLabs words don't touch, but some still swallow seconds of pause or noise.
- All the recordings are in English.

## Approach

Forced alignment with a CTC character model, the method WhisperX uses for word timestamps.
The words are already known, so the model only has to find where each letter is.

- **Model:** `facebook/wav2vec2-base-960h` (Apache-2.0), the ONNX export at
  `onnx-community/wav2vec2-base-960h-ONNX`: `onnx/model.onnx` (fp32, 378 MB), plus
  `vocab.json`. It has 32 tokens: blank `<pad>`=0, `|`=4 for the gap between words,
  A–Z and `'`. Input is 16 kHz mono, zero-mean and unit-variance per window; output
  is one frame per **20 ms** (four times finer than Parakeet's 80 ms grid).
  It runs on the ONNX Runtime the app already ships for Parakeet, on the same
  CUDA/DirectML/CPU provider. It downloads from the transcription settings like Parakeet.
- **English only.** A multilingual aligner (MMS, about 1.2 GB) can be added later behind
  the same code if a non-English recording ever needs it.

## Pipeline: one pass, right after transcription

1. `transcript/align.rs`: read the track at 16 kHz mono with `audio::for_each_chunk`
   (the reader transcription already uses, so its time base is the same).
2. Split the words into utterances at pauses over 0.5 s, at most 20 s each. Each window is the
   utterance ±300 ms.
3. Run wav2vec2 on the window to get log-probabilities per 20 ms frame.
4. Build the target from the words: upper-case, apostrophes kept, `|` between words.
   Words with no letters the model knows ("3.5", "GPT-4", `[typing]`) are aligned around:
   they get the time between their aligned neighbours, inside their old span.
5. Viterbi CTC alignment (a trellis, O(frames × letters), plain Rust, no new crate).
   A word starts at its first letter's first frame and ends after its last letter's last frame.
6. **Guardrails, so it can only make things better:**
   - a word may shrink anywhere inside its old span, but grow past it by 120 ms at most;
   - a word whose letters score badly (low mean probability) keeps its old times;
   - an utterance that fails as a whole (score, length, or a model error) keeps its old times;
   - words stay in order and never overlap.
7. Save the transcript as usual, with `alignment: { model, version, alignedWords, keptWords }`
   on it. The words keep their ids and text; only `sourceStartUs` and `sourceEndUs` change.

## Why editing stays simple

- Nothing new to manage while editing. Cuts, undo, captions, suggestions and agent
  proposals already read word times by word id. They get better times and nothing else.
- It runs before any edit exists, at the end of a transcription, so no cut ever depends on the
  old times.
- `tighten_loose_words` stays for transcripts that aren't aligned (a build without the model,
  or a failed window). On aligned words it finds nothing to trim.
- If the model isn't downloaded, or the pass fails, the transcript saves unaligned with a
  note, exactly as today.

## Where it shows up

- Transcription settings: **Align words to the audio** (on by default once the model is
  downloaded), with the download button and size beside Parakeet's.
- An existing transcript: **Align words** in the transcript panel menu, which also runs on old
  projects. Before building this, check how `word_stamp` and saved word cuts react to
  changed times, and refuse or warn when cuts already depend on the transcript.
- Agents: the `transcribe` action reports "aligned N of M words". A new `align` action is
  for existing transcripts. `status` says whether a transcript is aligned.

## How we know it works

There is no hand-labelled ground truth, so three checks:
1. **Loose words:** the share of words with ≥150 ms of quiet inside their span, before and after.
   Expect it to fall sharply for both providers.
2. **Two transcribers agree:** align the Parakeet and the ElevenLabs transcript of the 16 Oct
   recording. For words both found, compare boundaries: the gap between them should shrink
   from tens of ms to the 20 ms grid.
3. **Listening:** a probe writes 20 cut points as short before/after WAV pairs (the
   cleanup probe's word guard, Balanced gaps), so George can judge by ear.

Also unit tests: the Viterbi aligner on synthetic log-probs, text normalisation, guardrails,
and a window that fails keeping its times.

## Cost

- Download: 378 MB once (fp16 is 189 MB but slow on CPU; int8 at 95 MB risks blurring
  letter timing).
- Run time (estimate): a few seconds per 30 minutes of audio on CUDA, about 1–2 minutes on CPU.

## Out of scope

- Non-English recordings.
- Changing what the words say (alignment never edits text).
- Re-running transcription.

## Status (2026-10-17): built, uncommitted

Measured with the probe on the 16 Oct recording (26.5 min of audio, on the CPU, about 70 s per transcript):
- Parakeet words holding >=150 ms of quiet: 234 (53.7 s) -> 6 (2.0 s). ElevenLabs: 2 -> 0.
- Parakeet vs ElevenLabs on 2408 shared words: start gap median/p90 70/220 ms -> 0/30 ms,
  end gap 70/210 ms -> 10/70 ms.
- Changed from the plan: words that can't be spelled keep their old times (the "any sound" span
  was too short: "$20" became 110 ms) and only block their neighbours.
- Not done: the WAV before/after listening clips, and a run inside the app. cuDNN 9 is not
  installed on this PC, so ONNX Runtime runs on the CPU (Parakeet too).
