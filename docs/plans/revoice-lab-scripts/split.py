"""Cut whole-sentence takes (sNN.wav + sentences.json) into the phrase files fit.py expects (pNN.wav).

Each sentence is aligned word by word; it is cut at the quiet point just before the first word of
each new phrase, and a phrase that spans two sentences gets both parts joined."""
import json
import os
import sys

import numpy as np
import soundfile as sf

sys.path.insert(0, os.path.dirname(__file__))
import fit  # noqa: E402
from common import phrases  # noqa: E402

SR = fit.SR


def main(folder):
    sents = json.load(open(os.path.join(folder, "sentences.json")))
    n_phrases = len(phrases())
    parts = {i: [] for i in range(n_phrases)}
    for k, s in enumerate(sents, 1):
        x = fit.trim(fit.load(os.path.join(folder, f"s{k:02d}.wav")))
        words = s["text"].split()
        owners = s["phrase_of_word"]
        if len(words) != len(owners):
            raise SystemExit(f"sentence {k}: {len(words)} words vs {len(owners)} owners")
        onsets = fit.word_onsets(x, words)
        cuts = [0]
        for j in range(1, len(words)):
            if owners[j] != owners[j - 1]:
                on = onsets[j]
                if on is None:
                    raise SystemExit(f"sentence {k}: word '{words[j]}' not found")
                cuts.append(int(fit.quiet_cut(x, on) * SR))
        cuts.append(len(x))
        starts = [0] + [j for j in range(1, len(words)) if owners[j] != owners[j - 1]]
        for (a, b), j in zip(zip(cuts, cuts[1:]), starts):
            parts[owners[j]].append(x[a:b])
    for i in range(n_phrases):
        gap = np.zeros(int(0.15 * SR), np.float32)
        audio = parts[i][0] if len(parts[i]) == 1 else np.concatenate(
            [y for p in parts[i] for y in (p, gap)][:-1])
        sf.write(os.path.join(folder, f"p{i + 1:02d}.wav"), audio, SR)
    print(f"{folder}: {len(sents)} sentences -> {n_phrases} phrases")


if __name__ == "__main__":
    main(sys.argv[1])
