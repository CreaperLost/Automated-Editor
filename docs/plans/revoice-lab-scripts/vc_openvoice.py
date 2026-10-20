"""OpenVoice V2 tone-colour converter: zero-shot, no training. Target timbre comes from
his own clips; each TTS model's source timbre comes from its own phrases."""
import glob
import os
import sys
import time

import torch
from huggingface_hub import snapshot_download

sys.path.insert(0, os.path.dirname(__file__))
from common import INPUTS, OUT  # noqa: E402

from openvoice.api import OpenVoiceBaseClass, ToneColorConverter  # noqa: E402

# Upstream bug: ToneColorConverter forwards enable_watermark to a base class that rejects it.
_base_init = OpenVoiceBaseClass.__init__
OpenVoiceBaseClass.__init__ = lambda self, *a, enable_watermark=None, **k: _base_init(self, *a, **k)


def main():
    ckpt = snapshot_download("myshell-ai/OpenVoiceV2", allow_patterns=["converter/*"])
    conv = ToneColorConverter(os.path.join(ckpt, "converter", "config.json"), device="cuda",
                              enable_watermark=False)
    conv.load_ckpt(os.path.join(ckpt, "converter", "checkpoint.pth"))
    torch.cuda.reset_peak_memory_stats()

    his = sorted(glob.glob(os.path.join(INPUTS, "rvc_dataset", "*.wav")))[:40]
    tgt_se = conv.extract_se(his)
    for tts in ("voxcpm", "chatterbox", "qwen"):
        src = sorted(glob.glob(os.path.join(OUT, "tts", tts, "p*.wav")))
        if not src:
            continue
        t0 = time.time()
        folder = os.path.join(OUT, "vc", f"openvoice_{tts}")
        os.makedirs(folder, exist_ok=True)
        src_se = conv.extract_se(src)
        for path in src:
            conv.convert(path, src_se, tgt_se, output_path=os.path.join(folder, os.path.basename(path)), tau=0.3)
        print(f"{tts}: {time.time() - t0:.1f}s, peak VRAM {torch.cuda.max_memory_allocated() / 2**30:.2f} GB")


if __name__ == "__main__":
    main()
