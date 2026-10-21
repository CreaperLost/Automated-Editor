"""Qwen3-TTS 1.7B VoiceDesign: the same description and seed for every phrase. Without
the Base model there is no design-then-clone step, so the speaker may drift a little
between phrases; the voice converter after it evens that out."""
import os
import sys

import soundfile as sf
import torch

sys.path.insert(0, os.path.dirname(__file__))
from common import OUT, VOICE, Meter, phrases  # noqa: E402

from qwen_tts import Qwen3TTSModel  # noqa: E402


def main():
    folder = os.path.join(OUT, "tts", "qwen")
    os.makedirs(folder, exist_ok=True)
    meter = Meter("Qwen3-TTS 1.7B VoiceDesign")
    model = Qwen3TTSModel.from_pretrained("Qwen/Qwen3-TTS-12Hz-1.7B-VoiceDesign", device_map="cuda:0",
                                          dtype=torch.bfloat16, attn_implementation="sdpa")
    total = 0.0
    for i, p in enumerate(phrases(), 1):
        torch.manual_seed(42)
        wavs, sr = model.generate_voice_design(text=p["text"], language="English", instruct=VOICE)
        sf.write(os.path.join(folder, f"p{i:02d}.wav"), wavs[0], sr)
        secs = len(wavs[0]) / sr
        total += secs
        meter.row(phrase=i, seconds=round(secs, 2), target=round(p["end"] - p["start"], 2))
    meter.save(folder, total)


if __name__ == "__main__":
    main()
