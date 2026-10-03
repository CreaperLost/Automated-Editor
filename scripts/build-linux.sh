#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd -- "$repo_root"
npm --prefix front-end ci --include=dev
cd -- "$repo_root/src-tauri"
node scripts/fetch-ffmpeg.mjs
cargo tauri build --ci --features tauri-app --bundles deb,appimage --config tauri.ffmpeg.conf.json
printf 'Installer output: %s/src-tauri/target/release/bundle/{deb,appimage}\n' "$repo_root"
