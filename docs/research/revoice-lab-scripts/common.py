import json
import os
import time

LAB = r"C:\Users\George\Documents\AeroVoiceLab"
INPUTS = os.path.join(LAB, "inputs")
OUT = os.path.join(LAB, "outputs")

# One description for every model that designs a voice, written to land near his
# timbre (young adult male, mid-low, warm) but with native General American speech.
VOICE = ("A man in his late twenties with a warm, mid-low, clear voice and a neutral "
         "General American English accent. Friendly, energetic tech tutorial presenter, "
         "natural conversational pace, close-mic studio recording.")
ANCHOR_TEXT = ("Hi everyone, welcome back to the channel. Today I'm going to show you something "
               "really useful, and by the end of this video you'll be able to set it up yourself.")

# Spelling fixes only where the transcript misspelled a name; wording stays as spoken.
FIXES = {"cloud code": "Claude Code"}


def phrases():
    data = json.load(open(os.path.join(INPUTS, "test_words.json"), encoding="utf-8"))
    out = []
    for p in data["phrases"]:
        text = p["text"]
        for a, b in FIXES.items():
            text = text.replace(a, b)
        out.append(dict(p, text=text))
    return out


def sentences():
    """Whole sentences of the test minute (round one generated pause-split fragments such as
    "You can", which TTS reads flat). Each has its text and, per word, the phrase it belongs to."""
    words = [(i, w["text"]) for i, p in enumerate(phrases()) for w in p["words"]]
    out, cur = [], []
    for k, (i, t) in enumerate(words):
        cur.append((i, t))
        if t.endswith((".", "?", "!")) or k == len(words) - 1:
            out.append(cur)
            cur = []
    if len(out) > 1 and len(out[-1]) < 6 and not out[-1][-1][1].endswith((".", "?", "!")):
        out[-2].extend(out.pop())  # a short trailing fragment joins the last sentence
    result = []
    for s in out:
        text = " ".join(t for _, t in s)
        for a, b in FIXES.items():
            text = text.replace(a, b)
        text = text.replace("You can Go ahead", "You can go ahead")
        result.append({"text": text, "phrase_of_word": [i for i, _ in s]})
    return result


def sentence_spans():
    """(start, end) seconds of each sentence in his original take, from the aligned word times
    (outputs/original_aligned.json, written by fit.py), plus a short tail for the last word to decay."""
    aligned = json.load(open(os.path.join(OUT, "original_aligned.json")))
    flat = [t for phrase in aligned for t in phrase]
    spans, k = [], 0
    for s in sentences():
        n = len(s["phrase_of_word"])
        spans.append((flat[k][0], flat[k + n - 1][1] + 0.15))
        k += n
    return spans


def his_ref10():
    """The 11 s reference from chunk 22 and its exact transcript (scripts/make_ref.py)."""
    wav = os.path.join(INPUTS, "his_ref10.wav")
    return wav, open(os.path.join(INPUTS, "his_ref10.txt"), encoding="utf-8").read().strip()


def his_reference(seconds=20.0):
    """About `seconds` of his cleanest consecutive speech, written once to inputs/his_reference.wav."""
    import glob
    import subprocess
    path = os.path.join(INPUTS, "his_reference.wav")
    if os.path.exists(path):
        return path
    clips, total = [], 0.0
    for c in sorted(glob.glob(os.path.join(INPUTS, "rvc_dataset", "*.wav")))[40:]:
        d = float(subprocess.check_output(["ffprobe", "-v", "error", "-show_entries", "format=duration",
                                           "-of", "csv=p=0", c]).decode())
        clips.append(c)
        total += d
        if total >= seconds:
            break
    lst = os.path.join(INPUTS, "his_reference.txt")
    open(lst, "w").write("".join(f"file '{c}'\n" for c in clips))
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-f", "concat", "-safe", "0", "-i", lst, "-ac", "1", path],
                   check=True)
    return path


class Meter:
    """Wall time and peak VRAM for one model run, written next to its outputs."""

    def __init__(self, name):
        import torch
        self.torch, self.name, self.t0 = torch, name, time.time()
        torch.cuda.reset_peak_memory_stats()
        self.rows = []

    def row(self, **kw):
        self.rows.append(kw)

    def save(self, folder, audio_seconds):
        wall = time.time() - self.t0
        info = {"model": self.name, "wall_s": round(wall, 1), "audio_s": round(audio_seconds, 1),
                "rtf": round(wall / max(audio_seconds, 1e-6), 2),
                "peak_vram_gb": round(self.torch.cuda.max_memory_allocated() / 2**30, 2),
                "rows": self.rows}
        json.dump(info, open(os.path.join(folder, "run.json"), "w"), indent=1)
        print(json.dumps({k: v for k, v in info.items() if k != "rows"}))
