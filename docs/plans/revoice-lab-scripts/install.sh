#!/usr/bin/env bash
# Usage: install.sh <env-name>. Everything (envs, uv cache, Python builds) stays inside AeroVoiceLab.
# --torch-backend makes uv take torch/torchaudio/torchvision from the CUDA index; PyPI's
# Windows wheels are CPU-only.
set -euo pipefail
LAB=/c/Users/George/Documents/AeroVoiceLab
export UV_CACHE_DIR="$LAB/cache/uv" UV_LINK_MODE=hardlink UV_PYTHON_INSTALL_DIR="$LAB/cache/python"
cd "$LAB"
name=$1
py=envs/$name/Scripts/python.exe
tools=(soundfile numpy onnxruntime huggingface_hub)
# After torch is in: pin it, so the remaining packages can come from PyPI (some, like torchcodec,
# have no Windows wheel on the CUDA index) without swapping torch for a CPU build.
pin() {
  "$py" -c "import torch, torchaudio; print(f'torch=={torch.__version__}'); print(f'torchaudio=={torchaudio.__version__}')"     > "envs/$name-constraints.txt"
  rest=(-c "envs/$name-constraints.txt" --extra-index-url https://download.pytorch.org/whl/cu128 --index-strategy unsafe-best-match)
}

case $name in
  voxcpm)
    uv venv -q --allow-existing envs/voxcpm --python 3.11
    uv pip install --python $py --torch-backend cu128 torch torchaudio
    uv pip install --python $py --torch-backend cu128 -e src/VoxCPM "${tools[@]}" ;;
  chatterbox)
    uv venv -q --allow-existing envs/chatterbox --python 3.11
    uv pip install --python $py --torch-backend cu124 torch==2.6.0 torchaudio==2.6.0
    uv pip install --python $py --torch-backend cu124 -e src/chatterbox "${tools[@]}" ;;
  qwen)
    uv venv -q --allow-existing envs/qwen --python 3.11
    uv pip install --python $py --torch-backend cu128 torch torchaudio
    pin; uv pip install --python $py "${rest[@]}" -e src/Qwen3-TTS "${tools[@]}" ;;
  applio)
    uv venv -q --allow-existing envs/applio --python 3.12
    uv pip install --python $py --torch-backend cu128 torch==2.11.0 torchaudio==2.11.0
    pin; uv pip install --python $py "${rest[@]}" -r src/Applio/requirements.txt huggingface_hub ;;
  openvoice)
    uv venv -q --allow-existing envs/openvoice --python 3.11
    uv pip install --python $py --torch-backend cu128 torch torchaudio librosa pydub unidecode inflect \
      eng_to_ipa "${tools[@]}"
    uv pip install --python $py --no-deps -e src/OpenVoice wavmark ;;
  musetalk)
    uv python install 3.10
    uv venv -q --allow-existing envs/musetalk --python 3.10
    uv pip install --python $py --torch-backend cu118 torch==2.0.1 torchvision==0.15.2 torchaudio==2.0.2
    grep -v -E '^(tensorflow|tensorboard)' src/MuseTalk/requirements.txt > envs/musetalk-req.txt
    uv pip install --python $py --torch-backend cu118 -r envs/musetalk-req.txt "${tools[@]}" "numpy==1.23.5" \
      setuptools wheel mmengine
    uv pip install --python $py --torch-backend cu118 "mmcv==2.0.1" \
      -f https://download.openmmlab.com/mmcv/dist/cu118/torch2.0/index.html
    uv pip install --python $py --torch-backend cu118 "mmdet==3.1.0" "mmpose==1.1.0" ;;
  *) echo "unknown env $name"; exit 2 ;;
esac
"$py" -c "import torch; print('$name torch', torch.__version__, 'cuda', torch.cuda.is_available())"
