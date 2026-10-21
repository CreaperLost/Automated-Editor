"""LongCat-AudioDiT 3.5B (MIT) cloning George from the 11 s reference + transcript.

LongCat takes the output length as an input, so each sentence is generated to last exactly as
long as he took to say it: his pacing without any time-stretching.
Writes outputs/tts/longcat/sNN.wav; split.py turns them into phrases."""
import json
import os
import sys

import soundfile as sf
import torch
import torch.nn.functional as F

sys.path.insert(0, os.path.dirname(__file__))
sys.path.insert(0, r"C:\Users\George\Documents\AeroVoiceLab\src\LongCat-AudioDiT")
from common import OUT, Meter, his_ref10, sentence_spans, sentences  # noqa: E402

import audiodit  # noqa: E402,F401  (registers the model with transformers)
from audiodit import AudioDiTModel  # noqa: E402
from transformers import AutoTokenizer  # noqa: E402
from utils import load_audio, normalize_text  # noqa: E402

MODEL = os.environ.get("LONGCAT_MODEL", "meituan-longcat/LongCat-AudioDiT-3.5B")


def main():
    folder = os.path.join(OUT, "tts", "longcat")
    os.makedirs(folder, exist_ok=True)
    torch.manual_seed(1024)
    # bf16 halves the 15 GB fp32 checkpoint so the 3.5B model fits the 12 GB card
    model = AudioDiTModel.from_pretrained(MODEL, torch_dtype=torch.bfloat16, low_cpu_mem_usage=True).to("cuda")
    model.vae.to_half()
    model.eval()
    tok = AutoTokenizer.from_pretrained(model.config.text_encoder_model)
    sr, hop = model.config.sampling_rate, model.config.latent_hop

    ref_wav, ref_text = his_ref10()
    prompt = load_audio(ref_wav, sr)
    pw = prompt
    if pw.shape[-1] % hop:
        pw = F.pad(pw, (0, hop - pw.shape[-1] % hop))
    with torch.no_grad():
        prompt_frames = model.vae.encode(F.pad(pw, (0, hop * 3)).unsqueeze(0).to("cuda").half()).shape[-1] - 3

    meter = Meter(f"LongCat-AudioDiT {MODEL.split('-')[-1]}")
    sents, spans = sentences(), sentence_spans()
    total = 0.0
    for i, (s, (a, b)) in enumerate(zip(sents, spans), 1):
        gen_frames = int(round((b - a) * sr / hop))  # his own duration for this sentence
        inputs = tok([f"{normalize_text(ref_text)} {normalize_text(s['text'])}"], padding="longest",
                     return_tensors="pt")
        with torch.no_grad():
            out = model(input_ids=inputs.input_ids, attention_mask=inputs.attention_mask,
                        prompt_audio=prompt.unsqueeze(0), duration=prompt_frames + gen_frames,
                        steps=16, cfg_strength=4.0, guidance_method="apg")
        wav = out.waveform.squeeze().float().cpu().numpy()
        sf.write(os.path.join(folder, f"s{i:02d}.wav"), wav, sr)
        total += len(wav) / sr
        meter.row(sentence=i, seconds=round(len(wav) / sr, 2), target=round(b - a, 2))
    json.dump(sents, open(os.path.join(folder, "sentences.json"), "w"), indent=1)
    meter.save(folder, total)


if __name__ == "__main__":
    main()
