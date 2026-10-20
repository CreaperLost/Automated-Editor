"""Aligner check of one TTS/VC folder: words found, mean CTC confidence, trimmed length vs slot."""
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(__file__))
import fit  # noqa: E402
from align import align_audio  # noqa: E402
from common import phrases  # noqa: E402

folder = sys.argv[1]
rows = []
for i, p in enumerate(phrases(), 1):
    x = fit.trim(fit.load(os.path.join(folder, f"p{i:02d}.wav")))
    x16 = np.interp(np.arange(0, len(x), 3), np.arange(len(x)), x).astype(np.float32)
    res = align_audio(x16, [w["text"] for w in p["words"]])
    sc = [r[2] for r in res if r]
    rows.append((len(sc), len(res), np.mean(sc) if sc else 0, len(x) / 48000, p["end"] - p["start"]))
    print(f"{i:2d} {len(x) / 48000:5.2f}s (slot {p['end'] - p['start']:5.2f}) words {len(sc)}/{len(res)} conf {rows[-1][2]:.2f}")
print("overall conf", round(float(np.mean([r[2] for r in rows])), 3),
      "speech s", round(sum(r[3] for r in rows), 1), "slots s", round(sum(r[4] for r in rows), 1))
