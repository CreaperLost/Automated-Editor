"""CTC forced alignment with the app's wav2vec2-base-960h ONNX model (CPU).

align(path, words) -> [(start_s, end_s, score)] per word, times relative to the file.
"""
import json
import os
import re
import subprocess

import numpy as np
import onnxruntime as ort

MODEL_DIR = os.path.join(os.environ["APPDATA"], "AeroEdits", "models", "wav2vec2-base-960h")
FRAME_S = 0.02
_session = None
_vocab = json.load(open(os.path.join(MODEL_DIR, "vocab.json"), encoding="utf-8"))


def load_16k(path, start=None, end=None):
    cmd = ["ffmpeg", "-v", "error"]
    if start is not None:
        cmd += ["-ss", f"{start:.3f}"]
    if end is not None:
        cmd += ["-to", f"{end:.3f}"]
    cmd += ["-i", path, "-ac", "1", "-ar", "16000", "-f", "f32le", "-"]
    return np.frombuffer(subprocess.check_output(cmd), dtype=np.float32).copy()


def emissions(audio):
    global _session
    if _session is None:
        _session = ort.InferenceSession(os.path.join(MODEL_DIR, "model.onnx"),
                                        providers=["CPUExecutionProvider"])
    x = (audio - audio.mean()) / np.sqrt(audio.var() + 1e-7)
    name = _session.get_inputs()[0].name
    logits = _session.run(None, {name: x[None, :].astype(np.float32)})[0][0]
    logits = logits - logits.max(axis=1, keepdims=True)
    return logits - np.log(np.exp(logits).sum(axis=1, keepdims=True))


def spell(word):
    return re.sub(r"[^A-Z']", "", word.upper())


def align_audio(audio, words):
    em = emissions(audio)
    tokens, owner = [], []
    for i, w in enumerate(words):
        letters = spell(w)
        if not letters:
            continue
        if tokens:
            tokens.append(_vocab["|"])
            owner.append(-1)
        for ch in letters:
            tokens.append(_vocab[ch])
            owner.append(i)
    T, J = em.shape[0], len(tokens)
    if J == 0 or T < J:
        return [None] * len(words)
    trellis = np.full((T + 1, J + 1), -np.inf, dtype=np.float32)
    trellis[0, 0] = 0
    trellis[1:, 0] = np.cumsum(em[:, 0])
    tok = np.array(tokens)
    for t in range(T):
        stay = trellis[t, 1:] + em[t, 0]
        move = trellis[t, :-1] + em[t, tok]
        trellis[t + 1, 1:] = np.maximum(stay, move)
    # backtrack
    j, frames = J, {}
    for t in range(T, 0, -1):
        if j == 0:
            break
        stay = trellis[t - 1, j] + em[t - 1, 0]
        move = trellis[t - 1, j - 1] + em[t - 1, tok[j - 1]]
        if move >= stay:
            frames.setdefault(j - 1, []).append((t - 1, float(np.exp(em[t - 1, tok[j - 1]]))))
            j -= 1
        else:
            # blank frame after token j-1 keeps that token "open"
            pass
    out = []
    for i in range(len(words)):
        idx = [k for k, o in enumerate(owner) if o == i and k in frames]
        if not idx:
            out.append(None)
            continue
        fs = [f for k in idx for f, _ in frames[k]]
        sc = [s for k in idx for _, s in frames[k]]
        out.append((min(fs) * FRAME_S, (max(fs) + 1) * FRAME_S, float(np.mean(sc))))
    return out


def align(path, words, start=None, end=None):
    return align_audio(load_16k(path, start, end), words)
