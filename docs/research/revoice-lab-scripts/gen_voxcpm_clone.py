"""VoxCPM2 single-step: clone George's own voice from ~20 s of his speech and ask for a neutral
American accent, one whole sentence at a time. No voice-conversion stage follows.
Writes outputs/tts/voxcpm_clone/sNN.wav; split.py turns them into phrase files."""
import json
import os
import sys

import soundfile as sf

sys.path.insert(0, os.path.dirname(__file__))
from common import OUT, Meter, his_reference, sentences  # noqa: E402

import torch  # noqa: E402
import torchaudio  # noqa: E402


def _load_with_soundfile(path, *args, **kwargs):
    # torchaudio 2.9+ reads files through torchcodec, which needs FFmpeg's shared DLLs on Windows
    # and crashes without them; soundfile reads the reference wav fine.
    data, sr = sf.read(str(path), dtype="float32", always_2d=True)
    return torch.from_numpy(data.T.copy()), sr


torchaudio.load = _load_with_soundfile

from voxcpm import VoxCPM  # noqa: E402

STYLE = "(Neutral General American English accent, clear natural pronunciation, relaxed conversational tutorial tone)"


def main():
    folder = os.path.join(OUT, "tts", "voxcpm_clone")
    os.makedirs(folder, exist_ok=True)
    ref = his_reference()
    model = VoxCPM.from_pretrained("openbmb/VoxCPM2", load_denoiser=False)
    meter = Meter("VoxCPM2 direct clone")  # after loading, so wall time is generation only
    sr = model.tts_model.sample_rate
    sents = sentences()
    total = 0.0
    for i, s in enumerate(sents, 1):
        wav = model.generate(text=STYLE + s["text"], reference_wav_path=ref,
                             cfg_value=2.0, inference_timesteps=10, seed=42)
        sf.write(os.path.join(folder, f"s{i:02d}.wav"), wav, sr)
        total += len(wav) / sr
        meter.row(sentence=i, seconds=round(len(wav) / sr, 2), text=s["text"])
    json.dump(sents, open(os.path.join(folder, "sentences.json"), "w"), indent=1)
    meter.save(folder, total)


if __name__ == "__main__":
    main()
