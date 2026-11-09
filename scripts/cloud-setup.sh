#!/usr/bin/env bash
# One-time setup of a fresh Ubuntu VM (Claude Code cloud sessions, CI, a new Linux box) so the
# repo builds and its headless tests run. Idempotent: rerunning skips what is already there.
#
#   bash scripts/cloud-setup.sh
#
# Env:
#   AEROEDITS_SKIP_TAURI_DEPS=1   skip the WebKitGTK/GTK libraries (only `--features tauri-app`
#                                 needs them; the core library, its tests and the smoke test don't)
#   AEROEDITS_SKIP_FETCH=1        skip `npm ci` and `cargo fetch`
#
# Needs network access to: the Ubuntu apt mirrors, static.rust-lang.org (rustup installs the
# toolchain pinned in rust-toolchain.toml), crates.io / static.crates.io / index.crates.io,
# registry.npmjs.org, github.com (+ objects.githubusercontent.com / codeload.github.com).
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

say() { printf '\n==> %s\n' "$*"; }
die() { printf '\nERROR: %s\n' "$*" >&2; exit 1; }

[[ "$(uname -s)" == "Linux" ]] || die "this script is for Linux (Ubuntu); on Windows/macOS see README.md"

SUDO=""
if [[ $EUID -ne 0 ]]; then
  command -v sudo >/dev/null 2>&1 || die "not root and no sudo: cannot install apt packages"
  SUDO="sudo"
fi

# --- System packages -------------------------------------------------------------------------
# Core: a C toolchain for build scripts, pkg-config, FFmpeg (the app's media backend; Ubuntu's
# build has libx264), and Mesa's lavapipe so wgpu has a software Vulkan adapter for the
# compositor/export tests and the smoke test (no real GPU in a VM).
packages=(build-essential pkg-config curl ca-certificates git xz-utils file unzip
          ffmpeg mesa-vulkan-drivers libvulkan1 libssl-dev)
# The Tauri desktop shell (`cargo check --features tauri-app`), same list as .github/workflows/ci.yml.
if [[ "${AEROEDITS_SKIP_TAURI_DEPS:-0}" != "1" ]]; then
  packages+=(libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libappindicator3-dev patchelf libxdo-dev)
fi

missing=()
for p in "${packages[@]}"; do
  dpkg-query -W -f='${Status}' "$p" 2>/dev/null | grep -q "install ok installed" || missing+=("$p")
done
if ((${#missing[@]})); then
  say "Installing apt packages: ${missing[*]}"
  export DEBIAN_FRONTEND=noninteractive
  $SUDO apt-get update -q
  $SUDO apt-get install -y -q --no-install-recommends "${missing[@]}"
else
  say "apt packages already installed"
fi

command -v ffmpeg >/dev/null || die "ffmpeg missing after apt install"
command -v ffprobe >/dev/null || die "ffprobe missing after apt install"
grep -q ' libx264 ' <<<"$(ffmpeg -hide_banner -encoders 2>/dev/null)" || die "this ffmpeg has no libx264 encoder"

# --- Rust -------------------------------------------------------------------------------------
# rust-toolchain.toml pins the version; rustup installs it on first use.
if ! command -v rustup >/dev/null 2>&1; then
  if [[ -x "$HOME/.cargo/bin/rustup" ]]; then
    export PATH="$HOME/.cargo/bin:$PATH"
  else
    say "Installing rustup"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain none
    export PATH="$HOME/.cargo/bin:$PATH"
  fi
fi
toolchain="$(sed -n 's/^channel *= *"\(.*\)"/\1/p' rust-toolchain.toml)"
[[ -n "$toolchain" ]] || die "could not read the channel from rust-toolchain.toml"
if ! rustup toolchain list | grep -q "^$toolchain"; then
  say "Installing Rust $toolchain (rustfmt, clippy)"
  rustup toolchain install "$toolchain" --profile minimal -c rustfmt -c clippy
else
  rustup component add --toolchain "$toolchain" rustfmt clippy >/dev/null 2>&1 || true
fi
(cd src-tauri && rustc --version) || die "rustc $toolchain not usable"

# --- Node -------------------------------------------------------------------------------------
want_node="$(tr -d ' v\r\n' < .nvmrc)"
node_major() { node -p 'process.versions.node.split(".")[0]' 2>/dev/null || echo 0; }
if [[ "$(node_major)" != "$want_node" ]]; then
  if [[ -s "${NVM_DIR:-$HOME/.nvm}/nvm.sh" ]]; then
    say "Installing Node $want_node with nvm"
    # shellcheck disable=SC1091
    set +u; . "${NVM_DIR:-$HOME/.nvm}/nvm.sh"; nvm install "$want_node" >/dev/null; nvm use "$want_node" >/dev/null; set -u
  elif command -v n >/dev/null 2>&1; then
    $SUDO n "$want_node"
  fi
fi
command -v node >/dev/null || die "Node.js not found; install Node $want_node (see .nvmrc)"
if [[ "$(node_major)" != "$want_node" ]]; then
  # Vite 8 / React 19 work on Node >= 20.19; warn rather than fail on a close version.
  [[ "$(node_major)" -ge 20 ]] || die "Node $(node -v) is too old; need $want_node (see .nvmrc)"
  echo "WARNING: Node $(node -v), .nvmrc wants $want_node. Continuing."
fi

# --- Dependencies -----------------------------------------------------------------------------
if [[ "${AEROEDITS_SKIP_FETCH:-0}" != "1" ]]; then
  say "npm ci (front-end)"
  (cd front-end && npm ci --no-audit --no-fund)
  say "cargo fetch (src-tauri)"
  (cd src-tauri && cargo fetch --locked)
fi

say "Setup done. Next: bash scripts/check-env.sh   then   bash scripts/test-all.sh"
