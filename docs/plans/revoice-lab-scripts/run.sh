#!/usr/bin/env bash
# Usage: run.sh <env> <script.py> [args...]  - runs a lab script in an env, caches inside the lab.
set -euo pipefail
LAB=/c/Users/George/Documents/AeroVoiceLab
export HF_HOME="$LAB/cache/hf" MODELSCOPE_CACHE="$LAB/cache/modelscope" TORCH_HOME="$LAB/cache/torch"
export PYTHONUTF8=1 PYTHONIOENCODING=utf-8
env=$1; shift
cd "$LAB"
exec "envs/$env/Scripts/python.exe" "scripts/$1" "${@:2}"
