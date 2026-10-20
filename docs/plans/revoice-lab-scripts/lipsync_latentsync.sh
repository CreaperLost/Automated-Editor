#!/usr/bin/env bash
# Usage: lipsync_latentsync.sh <variant> (a file stem in outputs/final, e.g. openvoice_voxcpm_word)
# LatentSync 1.5 (256 px face, stage2.yaml) on a 600x600 face crop of the webcam minute.
# LatentSync only accepts 25 fps, so the crop is converted to 25 fps and the result back to 30.
set -euo pipefail
LAB=/c/Users/George/Documents/AeroVoiceLab
export HF_HOME="$LAB/cache/hf" PYTHONUTF8=1 PYTHONIOENCODING=utf-8
LS="$LAB/src/LatentSync"
py="$LAB/envs/latentsync/Scripts/python.exe"
v=$1
CROP_W=600 CROP_X=240 CROP_Y=0
out="$LAB/outputs/lipsync_latentsync"
mkdir -p "$out"
crop25="$LAB/inputs/test_webcam_face_25fps.mp4"
[ -f "$crop25" ] || ffmpeg -v error -y -i "$LAB/inputs/test_webcam.mp4" \
  -vf "crop=$CROP_W:$CROP_W:$CROP_X:$CROP_Y,fps=25" -c:v libx264 -crf 12 -preset fast -an "$crop25"

cd "$LS"
start=$(date +%s)
"$py" -m scripts.inference --unet_config_path configs/unet/stage2.yaml \
  --inference_ckpt_path checkpoints/latentsync_unet.pt --inference_steps 20 --guidance_scale 1.5 \
  --enable_deepcache --video_path "$(cygpath -m "$crop25")" \
  --audio_path "$(cygpath -m "$LAB/outputs/final/$v.wav")" \
  --video_out_path "$(cygpath -m "$out/${v}_face.mp4")" --temp_dir "$(cygpath -m "$out/temp")"
ffmpeg -v error -y -i "$LAB/inputs/test_webcam.mp4" -i "$out/${v}_face.mp4" -i "$LAB/outputs/final/$v.wav" \
  -filter_complex "[1:v]fps=30[f];[0:v][f]overlay=$CROP_X:$CROP_Y:shortest=1[v]" -map "[v]" -map 2:a \
  -c:v libx264 -crf 16 -preset fast -c:a aac -b:a 192k "$out/${v}_latentsync.mp4"
ffmpeg -v error -y -i "$LAB/inputs/test_webcam.mp4" -i "$out/${v}_latentsync.mp4" \
  -filter_complex "[0:v]crop=$CROP_W:$CROP_W:$CROP_X:$CROP_Y[a];[1:v]crop=$CROP_W:$CROP_W:$CROP_X:$CROP_Y[b];[a][b]hstack[v]" \
  -map "[v]" -map 1:a -c:v libx264 -crf 18 -preset fast -c:a aac "$out/${v}_side_by_side.mp4"
echo "latentsync $v took $(( $(date +%s) - start ))s"
