"""Qwen3-TTS 1.7B Base cloning George, whole sentences, no conversion stage.

  xvec - speaker embedding only (x_vector_only_mode): copies timbre, not his way of speaking,
         so it should carry over the least accent.
  icl  - full cloning: continues from the 11 s reference and its transcript (closer timbre,
         more accent risk).
Writes outputs/tts/qwen_clone_<mode>/sNN.wav; split.py turns them into phrases."""
import json
import os
import sys

import soundfile as sf
import torch

sys.path.insert(0, os.path.dirname(__file__))
from common import OUT, Meter, his_ref10, sentences  # noqa: E402

from qwen_tts import Qwen3TTSModel  # noqa: E402


def main(mode):
    folder = os.path.join(OUT, "tts", f"qwen_clone_{mode}")
    os.makedirs(folder, exist_ok=True)
    model = Qwen3TTSModel.from_pretrained("Qwen/Qwen3-TTS-12Hz-1.7B-Base", device_map="cuda:0",
                                          dtype=torch.bfloat16, attn_implementation="sdpa")
    wav, text = his_ref10()
    ref = sf.read(wav, dtype="float32")
    prompt = model.create_voice_clone_prompt(ref_audio=ref, ref_text=None if mode == "xvec" else text,
                                             x_vector_only_mode=(mode == "xvec"))
    meter = Meter(f"Qwen3-TTS 1.7B Base ({mode})")
    sents = sentences()
    total = 0.0
    for i, s in enumerate(sents, 1):
        torch.manual_seed(42)
        wavs, sr = model.generate_voice_clone(text=s["text"], language="English", voice_clone_prompt=prompt)
        sf.write(os.path.join(folder, f"s{i:02d}.wav"), wavs[0], sr)
        total += len(wavs[0]) / sr
        meter.row(sentence=i, seconds=round(len(wavs[0]) / sr, 2))
    json.dump(sents, open(os.path.join(folder, "sentences.json"), "w"), indent=1)
    meter.save(folder, total)


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "xvec")
