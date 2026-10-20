#!/usr/bin/env bash
# Usage: lipsync.sh <variant> (a file stem in outputs/final, e.g. rvc_voxcpm_word)
# Runs MuseTalk 1.5 on the original 720p webcam minute with that variant's audio.
set -euo pipefail
LAB=/c/Users/George/Documents/AeroVoiceLab
export HF_HOME="$LAB/cache/hf" PYTHONUTF8=1 PYTHONIOENCODING=utf-8
MT="$LAB/src/MuseTalk"
py="$LAB/envs/musetalk/Scripts/python.exe"
v=$1
cd "$MT"

# weights: MuseTalk 1.5 only (no v1, no SyncNet; SyncNet is only for training)
if [ ! -f models/musetalkV15/unet.pth ]; then
  "$py" - <<'EOF'
from huggingface_hub import hf_hub_download, snapshot_download
snapshot_download("TMElyralab/MuseTalk", local_dir="models", allow_patterns=["musetalkV15/*"])
snapshot_download("stabilityai/sd-vae-ft-mse", local_dir="models/sd-vae",
                  allow_patterns=["config.json", "diffusion_pytorch_model.bin"])
snapshot_download("openai/whisper-tiny", local_dir="models/whisper",
                  allow_patterns=["config.json", "pytorch_model.bin", "preprocessor_config.json"])
hf_hub_download("yzd-v/DWPose", "dw-ll_ucoco_384.pth", local_dir="models/dwpose")
for f in ("79999_iter.pth", "resnet18-5c106cde.pth"):
    hf_hub_download("ManyOtherFunctions/face-parse-bisent", f, local_dir="models/face-parse-bisent")
EOF
fi

# MuseTalk holds every frame in RAM; the full 720p minute (~4.5 GB) ran the 16 GB PC out of
# memory. It only repaints the mouth, so run it on a 600x600 face crop and paste that back.
CROP_W=600 CROP_X=240 CROP_Y=0
out="$LAB/outputs/lipsync"
mkdir -p "$out"
crop="$LAB/inputs/test_webcam_face.mp4"
[ -f "$crop" ] || ffmpeg -v error -y -i "$LAB/inputs/test_webcam.mp4" \
  -vf "crop=$CROP_W:$CROP_W:$CROP_X:$CROP_Y" -c:v libx264 -crf 12 -preset fast -an "$crop"

cfg="$out/$v.yaml"
cat > "$cfg" <<EOF
task_0:
 video_path: "$(cygpath -m "$crop")"
 audio_path: "$(cygpath -m "$LAB/outputs/final/$v.wav")"
EOF
ff=$(cygpath -m "$(dirname "$(which ffmpeg)")")
start=$(date +%s)
"$py" -m scripts.inference --inference_config "$cfg" --result_dir "$(cygpath -m "$out")" \
  --unet_model_path models/musetalkV15/unet.pth --unet_config models/musetalkV15/musetalk.json \
  --version v15 --fps 30 --use_float16 --ffmpeg_path "$ff" --output_vid_name "${v}_face.mp4" \
  --saved_coord --use_saved_coord
face=$(find "$out" -name "${v}_face.mp4" | head -1)
ffmpeg -v error -y -i "$LAB/inputs/test_webcam.mp4" -i "$face" -i "$LAB/outputs/final/$v.wav" \
  -filter_complex "[0:v][1:v]overlay=$CROP_X:$CROP_Y:shortest=1[v]" -map "[v]" -map 2:a \
  -c:v libx264 -crf 16 -preset fast -c:a aac -b:a 192k "$out/${v}_lipsync.mp4"
echo "lipsync $v took $(( $(date +%s) - start ))s"
