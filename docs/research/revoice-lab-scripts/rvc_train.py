"""Train an RVC voice model of George on ~15 min of his mic clips with Applio.

Only the files this run needs are fetched (40 kHz HiFi-GAN pretrained G/D, RMVPE, ContentVec),
not Applio's full prerequisite bundle."""
import os
import shutil
import sys
import time

from huggingface_hub import hf_hub_download

APPLIO = r"C:\Users\George\Documents\AeroVoiceLab\src\Applio"
DATASET = r"C:\Users\George\Documents\AeroVoiceLab\inputs\rvc_dataset"
NAME = "george"
EPOCHS = int(os.environ.get("RVC_EPOCHS", "200"))

NEEDED = {
    "pretrained_v2/f0G40k.pth": "rvc/models/pretraineds/hifi-gan/f0G40k.pth",
    "pretrained_v2/f0D40k.pth": "rvc/models/pretraineds/hifi-gan/f0D40k.pth",
    "predictors/rmvpe.pt": "rvc/models/predictors/rmvpe.pt",
    "embedders/contentvec/pytorch_model.bin": "rvc/models/embedders/contentvec/pytorch_model.bin",
    "embedders/contentvec/config.json": "rvc/models/embedders/contentvec/config.json",
}


def fetch():
    for remote, local in NEEDED.items():
        dest = os.path.join(APPLIO, local)
        if os.path.isfile(dest) and os.path.getsize(dest) > 0:
            continue
        os.makedirs(os.path.dirname(dest), exist_ok=True)
        shutil.copy(hf_hub_download("IAHispano/Applio", f"Resources/{remote}"), dest)


def main():
    os.chdir(APPLIO)
    sys.path.insert(0, APPLIO)
    fetch()
    import core

    t0 = time.time()
    # Resume: slices and features from an earlier run are reused, and Applio's train.py picks
    # up the latest G_/D_ checkpoint by itself, so a stopped run continues where it left off.
    # RVC_EPOCHS is the new total, e.g. RVC_EPOCHS=100 after stopping at 50.
    if not os.path.isfile(os.path.join(APPLIO, "logs", NAME, "filelist.txt")):
        print(core.run_preprocess_script(NAME, DATASET, 40000, 6, "Automatic", False, 3.0, 0.3, "none"))
        print(core.run_extract_script(NAME, "rmvpe", 6, 0, 40000, "contentvec", None, 2))
    t1 = time.time()
    print(core.run_train_script(NAME, save_every_epoch=50, save_only_latest=True, save_every_weights=True,
                                total_epoch=EPOCHS, sample_rate=40000, batch_size=8, gpu=0, pretrained=True,
                                cleanup=False, index_algorithm="Auto"))
    print(f"prep+extract {t1 - t0:.0f}s, train {time.time() - t1:.0f}s")


if __name__ == "__main__":
    main()
