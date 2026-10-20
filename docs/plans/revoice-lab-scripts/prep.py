"""Cut the test minute and the RVC training clips out of the 16 Oct recording.

Transcript times are wall-clock (they include the recording pauses); the mic and
webcam chunks only hold active time, so every word time is mapped through the
pause list in manifest.json first.
"""
import json
import os
import shutil
import subprocess

REC = r"C:\Users\George\Documents\AeroShootRec\Untitled 16 Oct 2026.aero"
TRANSCRIPT = r"C:\Users\George\Documents\AeroEdits\Untitled 16 Oct 2026\transcripts\rec-57e0702f2a26.mic.json"
LAB = r"C:\Users\George\Documents\AeroVoiceLab"
TEST_CHUNK = 21
PHRASE_GAP = 0.30  # seconds of quiet that ends a phrase in the test minute
TRAIN_GAP = 0.60   # looser grouping for training clips
PAD = 0.12


def probe_duration(path):
    out = subprocess.check_output(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", path])
    return float(out.decode())


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

    mic_dir = os.path.join(REC, "media", "mic")
    chunks, t = [], 0.0
    for name in sorted(os.listdir(mic_dir)):
        dur = probe_duration(os.path.join(mic_dir, name))
        chunks.append({"index": len(chunks) + 1, "file": name, "start": t, "dur": dur})
        t += dur

    transcript = json.load(open(TRANSCRIPT, encoding="utf-8"))
    words = []
    for w in transcript["words"]:
        a, b = active(w["sourceStartUs"]), active(w["sourceEndUs"])
        if a is None or b is None:
            continue
        words.append({"id": w["id"], "text": w["text"], "start": a, "end": b})

    def chunk_words(c):
        return [dict(w, start=w["start"] - c["start"], end=w["end"] - c["start"])
                for w in words if c["start"] <= w["start"] < c["start"] + c["dur"]]

    def group(ws, gap):
        phrases = []
        for w in ws:
            if phrases and w["start"] - phrases[-1][-1]["end"] < gap:
                phrases[-1].append(w)
            else:
                phrases.append([w])
        return phrases

    inputs = os.path.join(LAB, "inputs")
    test = chunks[TEST_CHUNK - 1]
    name = f"{TEST_CHUNK:06d}"
    shutil.copy(os.path.join(mic_dir, test["file"]), os.path.join(inputs, "test_mic.wav"))
    shutil.copy(os.path.join(REC, "media", "webcam", name + ".mp4"), os.path.join(inputs, "test_webcam.mp4"))
    tw = chunk_words(test)
    phrases = [{"start": p[0]["start"], "end": p[-1]["end"], "text": " ".join(w["text"] for w in p),
                "words": p} for p in group(tw, PHRASE_GAP)]
    json.dump({"chunk": TEST_CHUNK, "duration": test["dur"], "words": tw, "phrases": phrases},
              open(os.path.join(inputs, "test_words.json"), "w", encoding="utf-8"), indent=1)
    print(f"test chunk {TEST_CHUNK}: {len(tw)} words, {len(phrases)} phrases")

    train_dir = os.path.join(inputs, "rvc_dataset")
    shutil.rmtree(train_dir, ignore_errors=True)
    os.makedirs(train_dir)
    total, n = 0.0, 0
    for c in chunks:
        if c["index"] == TEST_CHUNK:
            continue
        for p in group(chunk_words(c), TRAIN_GAP):
            a = max(0.0, p[0]["start"] - PAD)
            b = min(c["dur"], p[-1]["end"] + PAD)
            if b - a < 1.0:
                continue
            n += 1
            subprocess.run(["ffmpeg", "-v", "error", "-y", "-ss", f"{a:.3f}", "-to", f"{b:.3f}",
                            "-i", os.path.join(mic_dir, c["file"]),
                            os.path.join(train_dir, f"clip_{n:04d}.wav")], check=True)
            total += b - a
    print(f"rvc dataset: {n} clips, {total / 60:.1f} min")


if __name__ == "__main__":
    main()
