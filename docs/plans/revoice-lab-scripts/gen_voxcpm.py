"""VoxCPM2: design a native voice once (the anchor), then clone that anchor per phrase
so every phrase has the same speaker."""
import os
import sys

import soundfile as sf

sys.path.insert(0, os.path.dirname(__file__))
from common import ANCHOR_TEXT, OUT, VOICE, Meter, phrases  # noqa: E402

from voxcpm import VoxCPM  # noqa: E402


def main():
    folder = os.path.join(OUT, "tts", "voxcpm")
    os.makedirs(folder, exist_ok=True)
    meter = Meter("VoxCPM2")
    model = VoxCPM.from_pretrained("openbmb/VoxCPM2", load_denoiser=False)
    sr = model.tts_model.sample_rate

    anchor = os.path.join(OUT, "anchor_voice.wav")
    wav = model.generate(text=f"({VOICE}){ANCHOR_TEXT}", cfg_value=2.0, inference_timesteps=10, seed=42)
    sf.write(anchor, wav, sr)

    total = 0.0
    for i, p in enumerate(phrases(), 1):
        wav = model.generate(text=p["text"], reference_wav_path=anchor,
                             cfg_value=2.0, inference_timesteps=10, seed=42)
        sf.write(os.path.join(folder, f"p{i:02d}.wav"), wav, sr)
        total += len(wav) / sr
        meter.row(phrase=i, seconds=round(len(wav) / sr, 2), target=round(p["end"] - p["start"], 2))
    meter.save(folder, total)


if __name__ == "__main__":
    main()
