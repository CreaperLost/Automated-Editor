"""Score every candidate the same way and write outputs/review/comparison.md.

Usage (chatterbox env, which has the speaker encoder):
  evaluate.py name=folder [name=folder ...]
where folder holds the candidate's phrase files (pNN.wav) and run.json, and the fitted track is
outputs/final/<name>_word_nostretch.wav.

Columns
  voice match  cosine similarity of Chatterbox's speaker encoder (an independent model; it isn't
               one of the cloning models compared, except Chatterbox itself) between the candidate
               and 2 minutes of George's real speech from other parts of the recording.
               His own test minute against that sets the ceiling.
  clarity      mean CTC confidence of an English wav2vec2 model reading the known words; his
               accented original scores lower than native speech.
  pace         seconds of speech vs his 45.4 s of slots.
  timing       median / 90th-percentile word-onset error after the no-stretch word fit.
  hard stops   words that end abruptly before a pause (his original: 0).
"""
import glob
import json
import os
import sys

import numpy as np
import soundfile as sf
import torch

sys.path.insert(0, os.path.dirname(__file__))
import fit  # noqa: E402
from align import align_audio  # noqa: E402
from common import INPUTS, OUT, phrases  # noqa: E402

from chatterbox.models.voice_encoder import VoiceEncoder  # noqa: E402
from huggingface_hub import hf_hub_download  # noqa: E402
from safetensors.torch import load_file  # noqa: E402

FINAL = os.path.join(OUT, "final")


def encoder():
    ve = VoiceEncoder()
    ve.load_state_dict(load_file(hf_hub_download("ResembleAI/chatterbox-turbo", "ve.safetensors")))
    return ve.to("cuda").eval()


def to16(x):
    return np.interp(np.arange(0, len(x), 3), np.arange(len(x)), x).astype(np.float32)


def embed(ve, wavs16):
    e = ve.embeds_from_wavs(wavs16, sample_rate=16000, as_spk=True)
    e = np.asarray(e).reshape(-1)
    return e / np.linalg.norm(e)


def clarity(folder):
    confs = []
    for i, p in enumerate(phrases(), 1):
        x = fit.trim(fit.load(os.path.join(folder, f"p{i:02d}.wav")))
        r = align_audio(to16(x), [w["text"] for w in p["words"]])
        sc = [a[2] for a in r if a]
        confs.append(np.mean(sc) if sc else 0.0)
    return float(np.mean(confs))


def hard_stops(path):
    x, sr = sf.read(path)
    h = sr // 200
    n = len(x) // h
    e = 10 * np.log10((x[:n * h].reshape(n, h) ** 2).mean(1) + 1e-12)
    # Independent of the noise floor (pauses are digital silence without room tone): a word
    # ending before >=150 ms of quiet is "hard" if its last 5 ms is still within 30 dB of the peak,
    # i.e. it never decayed.
    quiet = e < e.max() - 50
    tot = cnt = 0
    for i in range(4, n - 30):
        if not quiet[i] and quiet[i + 1:i + 31].all():
            tot += 1
            cnt += e[i] > e.max() - 30
    return cnt, tot


def main(cands):
    ve = encoder()
    ph = phrases()
    spans = [(t[0][0], t[-1][1]) for t in json.load(open(os.path.join(OUT, "original_aligned.json")))]
    his = [to16(fit.load(c)) for c in sorted(glob.glob(os.path.join(INPUTS, "rvc_dataset", "*.wav")))[60:100]]
    his_e = embed(ve, his)
    metrics = json.load(open(os.path.join(FINAL, "metrics.json")))

    def speech_of(path):
        x = fit.load(path)
        return [to16(x[int(a * fit.SR):int(b * fit.SR)]) for a, b in spans]

    rows = []
    orig = os.path.join(INPUTS, "test_mic.wav")
    ceiling = float(embed(ve, speech_of(orig)) @ his_e)
    o_conf = []
    a16 = fit.load(orig)
    for p, (a, b) in zip(ph, spans):
        r = align_audio(to16(a16[int((a - 0.3) * fit.SR):int((b + 0.3) * fit.SR)]), [w["text"] for w in p["words"]])
        o_conf.append(np.mean([q[2] for q in r if q]))
    rows.append({"name": "original (your voice, accented)", "sim": ceiling, "clarity": float(np.mean(o_conf)),
                 "pace": sum(b - a for a, b in spans), "timing": "-", "stops": "%d/%d" % hard_stops(orig),
                 "vram": "-", "rtf": "-"})
    for name, folder in cands:
        wav = os.path.join(FINAL, f"{name}_word_nostretch.wav")
        m = metrics.get(f"{name}_word_nostretch", {})
        run = json.load(open(os.path.join(folder, "run.json"))) if os.path.exists(os.path.join(folder, "run.json")) else {}
        pace = sum(len(fit.trim(fit.load(os.path.join(folder, f"p{i:02d}.wav")))) for i in range(1, len(ph) + 1)) / fit.SR
        c, t = hard_stops(wav)
        rows.append({"name": name, "sim": float(embed(ve, speech_of(wav)) @ his_e), "clarity": clarity(folder),
                     "pace": pace, "timing": f"{m.get('onset_err_ms_median', '?')} / {m.get('onset_err_ms_p90', '?')} ms",
                     "stops": f"{c}/{t}", "vram": run.get("peak_vram_gb", "-"), "rtf": run.get("rtf", "-")})
        print(rows[-1])
    lines = ["| Candidate | Voice match ↑ | Clarity ↑ | Speech s (slots 45.4) | Timing median / p90 | Hard stops | Peak VRAM GB | RTF |",
             "|---|---|---|---|---|---|---|---|"]
    for r in rows:
        lines.append(f"| {r['name']} | {r['sim']:.3f} | {r['clarity']:.3f} | {r['pace']:.1f} | {r['timing']} | "
                     f"{r['stops']} | {r['vram']} | {r['rtf']} |")
    out = os.path.join(OUT, "review", "comparison.md")
    open(out, "w", encoding="utf-8").write(__doc__.split("Columns")[1].join(["Columns", ""]) + "\n" + "\n".join(lines) + "\n")
    print("\n".join(lines))


if __name__ == "__main__":
    torch.set_grad_enabled(False)
    main([a.split("=", 1) for a in sys.argv[1:]])
