"""ZONOS2 Q8 (zonos2.cpp, Vulkan on the RTX 3060), one whole sentence at a time.

  clone  - his own voice from ~20 s of his speech (speaker embedding, so less of the accent
           should carry over than with audio-continuation cloning). No conversion after it.
  native - Zyphra's bundled AmericanMale voice, for the native-speech -> RVC path.
Writes outputs/tts/zonos_<mode>/sNN.wav + sentences.json; split.py turns them into phrases."""
import json
import os
import subprocess
import sys
import time

sys.path.insert(0, os.path.dirname(__file__))
from common import OUT, his_reference, sentences  # noqa: E402

Z = r"C:\Users\George\Documents\AeroVoiceLab\models\zonos2"
QUANT = os.environ.get("ZONOS_QUANT", "q8_0")


def main(mode):
    folder = os.path.join(OUT, "tts", f"zonos_{mode}")
    os.makedirs(folder, exist_ok=True)
    if mode == "clone":
        # The raw 48 kHz wav reference made ZONOS2 babble (a 19-word sentence came out 25 s long);
        # a 12 s loudness-normalised 44.1 kHz mp3, like Zyphra's bundled voices, reads properly.
        ref = os.path.join(OUT, "zonos_debug", "ref12_norm.mp3")
        if not os.path.exists(ref):
            os.makedirs(os.path.dirname(ref), exist_ok=True)
            subprocess.run(["ffmpeg", "-v", "error", "-y", "-i", his_reference(), "-t", "12", "-af",
                            "highpass=f=70,loudnorm=I=-18:TP=-2", "-ar", "44100", "-ac", "1", "-b:a", "192k", ref],
                           check=True)
    else:
        ref = os.path.join(Z, "app", "default_voices", "AmericanMale.mp3")
    sents = sentences()
    rows, t0 = [], time.time()
    for i, s in enumerate(sents, 1):
        out = os.path.join(folder, f"s{i:02d}.wav")
        t = time.time()
        subprocess.run([os.path.join(Z, "app", "zonos2-cli.exe"), os.path.join(Z, f"zonos2-{QUANT}.gguf"),
                        "--tts", s["text"], out, "--dac", os.path.join(Z, "dac.gguf"),
                        "--clone", ref, "--spk-encoder", os.path.join(Z, "spk-encoder.gguf"),
                        # --max: the CLI stops at 400 frames (4.64 s) by default, which cut long sentences
                        "--gpu", "--seed", "1", "--max", "2400"], check=True)
        dur = float(subprocess.check_output(["ffprobe", "-v", "error", "-show_entries", "format=duration",
                                             "-of", "csv=p=0", out]).decode())
        rows.append({"sentence": i, "seconds": round(dur, 2), "wall_s": round(time.time() - t, 1)})
        print(rows[-1])
    json.dump(sents, open(os.path.join(folder, "sentences.json"), "w"), indent=1)
    audio = sum(r["seconds"] for r in rows)
    wall = time.time() - t0
    json.dump({"model": f"ZONOS2 {QUANT} ({mode})", "wall_s": round(wall, 1), "audio_s": round(audio, 1),
               "rtf": round(wall / audio, 2), "rows": rows}, open(os.path.join(folder, "run.json"), "w"), indent=1)
    print(f"ZONOS2 {mode}: {audio:.1f}s audio in {wall:.0f}s (includes model load per sentence)")


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "clone")
