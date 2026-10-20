"""Fit generated phrases into the original timing and assemble one 55 s track per variant.

Two fits per variant:
  phrase - each phrase stretched uniformly so its first and last words land on the originals.
  word   - each phrase cut at word onsets and every piece stretched so each onset lands on the
           original onset (what lip movement follows).
Then loudness is matched to his speech and his own room tone is laid under the gaps.
Writes outputs/final/<variant>_<fit>.wav, a webcam .mp4 with that audio, and metrics.json.
"""
import json
import os
import subprocess
import sys

import numpy as np
import soundfile as sf

sys.path.insert(0, os.path.dirname(__file__))
from align import align_audio, load_16k  # noqa: E402
from common import INPUTS, OUT, phrases  # noqa: E402

SR = 48000
MIC = os.path.join(INPUTS, "test_mic.wav")
WEBCAM = os.path.join(INPUTS, "test_webcam.mp4")
FINAL = os.path.join(OUT, "final")
MIN_PIECE = 0.25  # seconds; shorter word pieces merge into the next one
CTX = 0.10        # context kept around a piece while stretching, then cropped off
# How far a word piece may be stretched. Round one allowed 0.75-1.35 with Rubber Band and he
# heard it as a metallic sound (confirmed on 17 Oct by listening stage by stage), so gaps are now
# made up mostly with pauses. FIT_STRETCH=0 never stretches; 0.08 allows +-8%.
STRETCH = float(os.environ.get("FIT_STRETCH", "0.08"))
MIN_STRETCH, MAX_STRETCH = 1 - STRETCH, 1 + STRETCH
ENGINE = os.environ.get("FIT_ENGINE", "atempo")  # atempo (WSOLA) or rubberband
TAG = os.environ.get("FIT_TAG", "")  # suffix on output names, to keep variants side by side
CUT_SEARCH = 0.12  # seconds before an onset searched for the quietest cut point
RELEASE = int(0.060 * SR)  # fade-out before an inserted pause
TRIM_TAIL = 0.12  # seconds of decay trim() keeps after the last word


def load(path, rate=SR):
    raw = subprocess.check_output(["ffmpeg", "-v", "error", "-i", path, "-ac", "1", "-ar", str(rate),
                                   "-f", "f32le", "-"])
    return np.frombuffer(raw, dtype=np.float32).copy()


def stretch(x, ratio):
    """Time-stretch to len(x) * ratio samples, pitch kept."""
    if abs(ratio - 1) < 1e-3:
        return x.copy()
    af = (f"atempo={1 / ratio:.6f}" if ENGINE == "atempo"
          else f"rubberband=tempo={1 / ratio:.6f}:transients=crisp:pitchq=quality")
    p = subprocess.run(["ffmpeg", "-v", "error", "-f", "f32le", "-ar", str(SR), "-ac", "1", "-i", "-",
                        "-af", af, "-f", "f32le", "-"], input=x.astype(np.float32).tobytes(),
                       capture_output=True, check=True)
    return np.frombuffer(p.stdout, dtype=np.float32).copy()


def stretch_exact(x, n_out, ctx, tail=0):
    """Stretch the middle of x (ctx samples of context each side) to n_out samples,
    plus `tail` extra samples of the right context for crossfading."""
    core = len(x) - 2 * ctx
    ratio = n_out / max(core, 1)
    y = stretch(x, ratio)
    a = int(round(ctx * len(y) / len(x)))
    y = y[a:a + n_out + tail]
    return np.pad(y, (0, n_out + tail - len(y))), ratio


def rms_db(x, win):
    n = len(x) // win
    f = x[:n * win].reshape(n, win)
    return 10 * np.log10((f ** 2).mean(axis=1) + 1e-12)


def trim(x):
    win = SR // 100
    db = rms_db(x, win)
    # generous threshold and padding: soft final consonants ("-s", "-t") sit well below the peak
    on = np.where(db > db.max() - 50)[0]
    a = max(0, on[0] * win - SR * 5 // 100)
    b = min(len(x), (on[-1] + 1) * win + int(TRIM_TAIL * SR))
    return x[a:b]


def original_alignment(ph):
    path = os.path.join(OUT, "original_aligned.json")
    if os.path.exists(path):
        return json.load(open(path))
    audio16 = load_16k(MIC)
    out = []
    for p in ph:
        a, b = max(0, p["start"] - 0.3), p["end"] + 0.3
        words = [w["text"] for w in p["words"]]
        res = align_audio(audio16[int(a * 16000):int(b * 16000)], words)
        times = []
        for w, r in zip(p["words"], res):
            times.append([a + r[0], a + r[1]] if r else [w["start"], w["end"]])
        out.append(times)
    json.dump(out, open(path, "w"), indent=1)
    return out


def word_onsets(x, words):
    res = align_audio(np.interp(np.arange(0, len(x), SR / 16000), np.arange(len(x)), x).astype(np.float32),
                      words)
    return [r[0] if r else None for r in res]


def fit_phrase(x, target_len):
    n = int(np.clip(target_len, len(x) * MIN_STRETCH, len(x) * MAX_STRETCH))
    y, ratio = stretch_exact(np.pad(x, (int(CTX * SR),) * 2), n, int(CTX * SR))
    return y, [ratio]


def quiet_cut(x, onset):
    """The quietest 5 ms in the CUT_SEARCH before a word onset, so a cut never clips the end
    of the previous word (round one cut right at the onset and clipped about half the words)."""
    hop = SR // 200
    a = max(0, int((onset - CUT_SEARCH) * SR))
    b = max(a + hop, int(onset * SR))
    seg = x[a:b]
    n = len(seg) // hop
    if n < 2:
        return onset
    e = (seg[:n * hop].reshape(n, hop) ** 2).mean(axis=1)
    return (a + int(np.argmin(e)) * hop + hop // 2) / SR


def fit_words(x, gen_on, orig_on, target_len):
    """Cut just before generated word onsets; stretch each piece so onsets hit the original onsets."""
    pairs = [(0.0, 0.0)]
    for g, o in zip(gen_on[1:], orig_on[1:]):
        if g is None or o is None:
            continue
        c = quiet_cut(x, g)
        # the cut sits `g - c` before the onset; keep that lead in the target so onsets still line up
        t = o - (g - c)
        if c - pairs[-1][0] >= MIN_PIECE and t - pairs[-1][1] >= MIN_PIECE:
            pairs.append((c, t))
    # The aligner's word end is where the last letter starts, not where the sound dies away, and
    # trim() keeps TRIM_TAIL of decay after the last word; give the target the same tail so the
    # final word isn't squeezed.
    pairs.append((len(x) / SR, target_len / SR + TRIM_TAIL))
    ctx = int(CTX * SR)
    padded = np.pad(x, (ctx, ctx))
    fade = int(0.010 * SR)
    ramp = np.sqrt(np.linspace(0, 1, fade, dtype=np.float32))
    out, ratios = np.zeros(target_len + 2 * SR, np.float32), []
    cursor = 0
    for k, ((g0, o0), (g1, o1)) in enumerate(zip(pairs, pairs[1:])):
        a, b = int(g0 * SR), int(g1 * SR)
        want = int(round(o1 * SR)) - int(round(o0 * SR))
        if want <= 0 or b <= a:
            continue
        # Stretch only within natural limits. A slot that is much longer gets the piece at
        # MAX_STRETCH followed by quiet (a pause, as he paused); a slot that is too short gets
        # MIN_STRETCH and the following words start a little late instead.
        n_out = int(np.clip(want, (b - a) * MIN_STRETCH, (b - a) * MAX_STRETCH))
        # each piece runs `fade` samples past its end; the next piece fades in over that overlap
        piece, r = stretch_exact(padded[a:b + 2 * ctx], n_out, ctx, tail=fade)
        ratios.append(r)
        if k > 0:
            piece[:fade] *= ramp
        piece[n_out:] *= ramp[::-1]
        if want - n_out > RELEASE:
            # a pause follows: let the word decay instead of stopping dead
            rel = (np.cos(np.linspace(0, np.pi / 2, RELEASE, dtype=np.float32)) ** 2)
            piece[n_out - RELEASE:n_out] *= rel
            piece[n_out:] = 0
        start = max(int(round(o0 * SR)), cursor)
        out[start:start + len(piece)] += piece[:len(out) - start]
        cursor = start + n_out
    return out[:max(target_len, cursor)], ratios


def room_tone(orig, spans, length):
    """Loop the quietest half-second between phrases under the whole track."""
    win = SR // 2
    best, best_db = None, 1e9
    for a, b in zip([s[1] for s in spans[:-1]], [s[0] for s in spans[1:]]):
        for t in np.arange(a + 0.1, b - 0.6, 0.1):
            seg = orig[int(t * SR):int(t * SR) + win]
            db = 10 * np.log10((seg ** 2).mean() + 1e-12)
            if db < best_db:
                best, best_db = seg, db
    if best is None:
        return np.zeros(length, np.float32)
    fade = np.hanning(win).astype(np.float32)
    tone = np.zeros(length + win, np.float32)
    for s in range(0, length, win // 2):
        tone[s:s + win] += best * fade
    return tone[:length]


def speech_rms(x, spans):
    parts = [x[int(a * SR):int(b * SR)] for a, b in spans]
    return float(np.sqrt(np.mean(np.concatenate(parts) ** 2)))


def main(variants):
    os.makedirs(FINAL, exist_ok=True)
    ph = phrases()
    orig = load(MIC)
    length = len(orig)
    orig_words = original_alignment(ph)
    spans = [(t[0][0], t[-1][1]) for t in orig_words]
    tone = room_tone(orig, spans, length)
    metrics = json.load(open(os.path.join(FINAL, "metrics.json"))) if os.path.exists(
        os.path.join(FINAL, "metrics.json")) else {}

    for name, folder in variants:
        clips = [trim(load(os.path.join(folder, f"p{i:02d}.wav"))) for i in range(1, len(ph) + 1)]
        for fit in os.environ.get("FIT_MODES", "word").split(","):
            track, ratios, errs = np.zeros(length, np.float32), [], []
            prev_end = 0
            for p, x, ow in zip(ph, clips, orig_words):
                o0, o1 = ow[0][0], ow[-1][1]
                target = int(round((o1 - o0) * SR))
                words = [w["text"] for w in p["words"]]
                if fit == "phrase":
                    y, r = fit_phrase(x, target)
                else:
                    gen_on = word_onsets(x, words)
                    y, r = fit_words(x, gen_on, [w[0] - o0 for w in ow], target)
                ratios += r
                # A phrase never starts before the previous one has finished (plus a short breath);
                # unstretched phrase placement can otherwise overlap and garble both.
                a = max(int(round(o0 * SR)), prev_end + int(0.05 * SR))
                track[a:a + len(y)] += y[:length - a]
                prev_end = a + len(y)
                # measure: where do word onsets land now?
                lead = min(a, int(0.2 * SR))
                got = word_onsets(track[a - lead:a + max(target, len(y)) + int(0.2 * SR)], words)
                for g, w in zip(got, ow):
                    if g is not None:
                        errs.append(abs((g - lead / SR + a / SR) - w[0]) * 1000)
            track *= speech_rms(orig, spans) / max(speech_rms(track, spans), 1e-9)
            if os.environ.get("FIT_ROOMTONE", "1") != "0":
                track += tone
            track /= max(1.0, np.abs(track).max() / 0.98)
            wav = os.path.join(FINAL, f"{name}_{fit}{TAG}.wav")
            sf.write(wav, track, SR, subtype="PCM_16")
            subprocess.run(["ffmpeg", "-v", "error", "-y", "-i", WEBCAM, "-i", wav, "-map", "0:v", "-map", "1:a",
                            "-c:v", "copy", "-c:a", "aac", "-b:a", "192k", "-shortest",
                            wav[:-4] + ".mp4"], check=True)
            e = np.array(errs) if errs else np.array([np.nan])
            metrics[f"{name}_{fit}{TAG}"] = {
                "onset_err_ms_median": round(float(np.median(e)), 1),
                "onset_err_ms_p90": round(float(np.percentile(e, 90)), 1),
                "words_found": f"{len(errs)}/{sum(len(p['words']) for p in ph)}",
                "stretch_min": round(min(ratios), 2), "stretch_max": round(max(ratios), 2),
            }
            print(name, fit, metrics[f"{name}_{fit}{TAG}"])
    json.dump(metrics, open(os.path.join(FINAL, "metrics.json"), "w"), indent=1)


if __name__ == "__main__":
    # args: name=folder pairs, e.g. rvc_voxcpm=outputs/vc/rvc_voxcpm
    main([a.split("=", 1) for a in sys.argv[1:]])
