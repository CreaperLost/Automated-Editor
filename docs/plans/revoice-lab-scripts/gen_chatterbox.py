"""Chatterbox Turbo, speaking with the VoxCPM2-designed anchor voice so both start
from the same native speaker."""
import os
import sys

import torchaudio as ta

sys.path.insert(0, os.path.dirname(__file__))
from common import OUT, Meter, phrases  # noqa: E402

from chatterbox.tts_turbo import ChatterboxTurboTTS  # noqa: E402


def main():
    folder = os.path.join(OUT, "tts", "chatterbox")
    os.makedirs(folder, exist_ok=True)
    meter = Meter("Chatterbox Turbo")
    model = ChatterboxTurboTTS.from_pretrained(device="cuda")
    anchor = os.path.join(OUT, "anchor_voice.wav")

    total = 0.0
    for i, p in enumerate(phrases(), 1):
        wav = model.generate(p["text"], audio_prompt_path=anchor)
        ta.save(os.path.join(folder, f"p{i:02d}.wav"), wav, model.sr)
        secs = wav.shape[-1] / model.sr
        total += secs
        meter.row(phrase=i, seconds=round(secs, 2), target=round(p["end"] - p["start"], 2))
    meter.save(folder, total)


if __name__ == "__main__":
    main()
