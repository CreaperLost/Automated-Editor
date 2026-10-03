#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd -- "$repo_root"
npm --prefix front-end ci --include=dev
cd -- "$repo_root/src-tauri"
# macOS uses the native Swift media bridges, without FFmpeg sidecars.
cargo tauri build --ci --features tauri-app --bundles app,dmg
printf 'Installer output: %s/src-tauri/target/release/bundle/dmg\n' "$repo_root"
