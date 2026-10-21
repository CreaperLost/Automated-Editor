"""Convert every TTS model's phrases into George's voice with the trained RVC model.

Pitch: RVC keeps the source melody, so each TTS voice is shifted by the semitone gap between
its median pitch and his. index_rate stays low: the feature index pulls in frames of his real
speech, which would bring some of the accent back."""
import glob
import json
import os
import sys
import time

import librosa
import numpy as np
import torch

APPLIO = r"C:\Users\George\Documents\AeroVoiceLab\src\Applio"
LAB = r"C:\Users\George\Documents\AeroVoiceLab"
INDEX_RATE = float(os.environ.get("RVC_INDEX_RATE", "0.25"))


_rmvpe = None


def median_f0(paths):
    """Median pitch with RMVPE (pyin made octave errors on the TTS voices: 61 Hz and 399 Hz)."""
    global _rmvpe
    if _rmvpe is None:
        from rvc.lib.predictors.f0 import RMVPE
        _rmvpe = RMVPE("cuda")
    f0s = []
    for p in paths:
        y, _ = librosa.load(p, sr=16000)
        f0 = np.asarray(_rmvpe.get_f0(y))
        f0s.append(f0[f0 > 0])
    return float(np.median(np.concatenate(f0s)))


def main():
    os.chdir(APPLIO)
    sys.path.insert(0, APPLIO)
    import core

    logs = os.path.join(APPLIO, "logs", "george")
    pth = max(glob.glob(os.path.join(logs, "george_*e_*s.pth")), key=os.path.getmtime)
    index = glob.glob(os.path.join(logs, "*.index"))
    index = index[0] if index else ""
    his = median_f0(sorted(glob.glob(os.path.join(LAB, "inputs", "rvc_dataset", "*.wav")))[:30])
    report = {"model": os.path.basename(pth), "index_rate": INDEX_RATE, "his_f0": round(his, 1)}
    torch.cuda.reset_peak_memory_stats()
    for tts in ("voxcpm", "chatterbox", "qwen"):
        src = sorted(glob.glob(os.path.join(LAB, "outputs", "tts", tts, "p*.wav")))
        if not src:
            continue
        theirs = median_f0(src)
        shift = int(round(12 * np.log2(his / theirs)))
        folder = os.path.join(LAB, "outputs", "vc", f"rvc_{tts}")
        os.makedirs(folder, exist_ok=True)
        t0 = time.time()
        for path in src:
            core.run_infer_script(pitch=shift, index_rate=INDEX_RATE, volume_envelope=1.0, protect=0.33,
                                  f0_method="rmvpe", input_path=path,
                                  output_path=os.path.join(folder, os.path.basename(path)), pth_path=pth,
                                  index_path=index, split_audio=False, f0_autotune=False,
                                  f0_autotune_strength=1.0, proposed_pitch=False, proposed_pitch_threshold=155.0,
                                  export_format="WAV", embedder_model="contentvec")
        report[tts] = {"tts_f0": round(theirs, 1), "shift_semitones": shift, "seconds": round(time.time() - t0, 1)}
    report["peak_vram_gb"] = round(torch.cuda.max_memory_allocated() / 2**30, 2)
    json.dump(report, open(os.path.join(LAB, "outputs", "vc", "rvc_run.json"), "w"), indent=1)
    print(report)


if __name__ == "__main__":
    main()
