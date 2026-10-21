"""A ~10 s reference clip of George *with its transcript* (LongCat and Qwen's full cloning mode
continue from a prompt, so they need the exact words). Taken from chunk 22, the minute after the
test minute, so the test words never appear in the reference.
Writes inputs/his_ref10.wav and inputs/his_ref10.txt."""
import json
import os
import subprocess

REC = r"C:\Users\George\Documents\AeroShootRec\Untitled 16 Oct 2026.aero"
TRANSCRIPT = r"C:\Users\George\Documents\AeroEdits\Untitled 16 Oct 2026\transcripts\rec-57e0702f2a26.mic.json"
INPUTS = r"C:\Users\George\Documents\AeroVoiceLab\inputs"
CHUNK, TARGET, GAP = 22, 10.0, 0.30


def main():
    manifest = json.load(open(os.path.join(REC, "manifest.json"), encoding="utf-8"))
    pauses = [(p["startUs"] / 1e6, p["endUs"] / 1e6) for p in manifest["pauseIntervals"]]

    def active(us):
        s, off = us / 1e6, 0.0
        for a, b in pauses:
            if s >= b:
                off += b - a
            elif s > a:
                return None
        return s - off

    mic = os.path.join(REC, "media", "mic")
    names = sorted(os.listdir(mic))
    start = 0.0
    for n in names[:CHUNK - 1]:
        start += float(subprocess.check_output(["ffprobe", "-v", "error", "-show_entries", "format=duration",
                                                "-of", "csv=p=0", os.path.join(mic, n)]).decode())
    words = []
    for w in json.load(open(TRANSCRIPT, encoding="utf-8"))["words"]:
        a, b = active(w["sourceStartUs"]), active(w["sourceEndUs"])
        if a is not None and b is not None and start <= a < start + 55:
            words.append((a - start, b - start, w["text"]))
    # whole phrases (split at pauses) until ~TARGET seconds, starting at a sentence start
    first = next(i for i, w in enumerate(words) if i > 0 and words[i - 1][2].endswith((".", "?", "!")))
    sel = [words[first]]
    for w in words[first + 1:]:
        if sel[-1][1] - sel[0][0] >= 0.8 * TARGET and sel[-1][2].endswith((".", ",", "?", "!")):
            break
        sel.append(w)
    a, b = max(0.0, sel[0][0] - 0.10), sel[-1][1] + 0.15
    out = os.path.join(INPUTS, "his_ref10.wav")
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-ss", f"{a:.3f}", "-to", f"{b:.3f}",
                    "-i", os.path.join(mic, names[CHUNK - 1]), "-ac", "1", out], check=True)
    text = " ".join(w[2] for w in sel)
    # Parakeet's misspellings of names; the prompt text has to match what he actually said
    for a_, b_ in {"codecs": "Codex", "codec": "Codex", "cloud code": "Claude Code"}.items():
        text = text.replace(a_, b_)
    open(os.path.join(INPUTS, "his_ref10.txt"), "w", encoding="utf-8").write(text)
    print(f"{b - a:.1f}s: {text}")


if __name__ == "__main__":
    main()
