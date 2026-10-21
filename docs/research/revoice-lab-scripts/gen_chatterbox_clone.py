"""Chatterbox Turbo cloning George directly from the 11 s reference, whole sentences.
(Round one cloned a synthetic VoxCPM voice, never him.)
Writes outputs/tts/chatterbox_clone/sNN.wav; split.py turns them into phrases."""
import json
import os
import sys

import torchaudio as ta

sys.path.insert(0, os.path.dirname(__file__))
from common import OUT, Meter, his_ref10, sentences  # noqa: E402

from chatterbox.tts_turbo import ChatterboxTurboTTS  # noqa: E402


def main():
    folder = os.path.join(OUT, "tts", "chatterbox_clone")
    os.makedirs(folder, exist_ok=True)
    model = ChatterboxTurboTTS.from_pretrained(device="cuda")
    ref, _ = his_ref10()
    meter = Meter("Chatterbox Turbo clone")
    sents = sentences()
    total = 0.0
    for i, s in enumerate(sents, 1):
        wav = model.generate(s["text"], audio_prompt_path=ref)
        ta.save(os.path.join(folder, f"s{i:02d}.wav"), wav, model.sr)
        secs = wav.shape[-1] / model.sr
        total += secs
        meter.row(sentence=i, seconds=round(secs, 2))
    json.dump(sents, open(os.path.join(folder, "sentences.json"), "w"), indent=1)
    meter.save(folder, total)


if __name__ == "__main__":
    main()
