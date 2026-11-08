#!/usr/bin/env bash
# Prints the version of every tool the repo needs and proves the core works here: builds the
# library and runs the smoke test (fixture recording -> silence cut -> MP4 export -> ffprobe).
#
#   bash scripts/check-env.sh          # versions + build + smoke test
#   bash scripts/check-env.sh --quick  # versions only
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
[[ -d "$HOME/.cargo/bin" ]] && export PATH="$HOME/.cargo/bin:$PATH"

fail=0
row() {  # row <name> <command...>: print the first line of its version, or MISSING
  local name="$1"; shift
  local out
  if out="$("$@" 2>&1 | head -n 1)" && [[ -n "$out" ]]; then
    printf '  %-12s %s\n' "$name" "$out"
  else
    printf '  %-12s MISSING\n' "$name"; fail=1
  fi
}

echo "Tools:"
row rustc   bash -c 'cd src-tauri && rustc --version'
row cargo   bash -c 'cd src-tauri && cargo --version'
row rustfmt bash -c 'cd src-tauri && cargo fmt --version'
row clippy  bash -c 'cd src-tauri && cargo clippy --version'
row node    node --version
row npm     npm --version
row ffmpeg  bash -c 'ffmpeg -hide_banner -version'
row ffprobe bash -c 'ffprobe -hide_banner -version'
if ffmpeg -hide_banner -encoders 2>/dev/null | grep -q ' libx264 '; then
  printf '  %-12s %s\n' libx264 "present"
else
  printf '  %-12s MISSING (exports need it)\n' libx264; fail=1
fi
if ls /usr/share/vulkan/icd.d/*.json >/dev/null 2>&1; then
  printf '  %-12s %s\n' vulkan "$(ls /usr/share/vulkan/icd.d/ | tr '\n' ' ')"
else
  printf '  %-12s none (export/compositor tests will skip; apt install mesa-vulkan-drivers)\n' vulkan
fi
printf '  %-12s %s\n' front-deps "$([[ -d front-end/node_modules ]] && echo installed || echo 'missing (cd front-end && npm ci)')"

echo
echo "Optional (only on George's Windows machine / with keys; nothing here needs them):"
for v in OPENROUTER_API_KEY ELEVENLABS_API_KEY ZERNIO_API_KEY TREG_API_KEY; do
  printf '  %-20s %s\n' "$v" "$([[ -n "${!v:-}" ]] && echo set || echo 'not set')"
done

if [[ "${1:-}" == "--quick" ]]; then
  exit $fail
fi
((fail)) && { echo; echo "Missing tools above: run bash scripts/cloud-setup.sh"; exit 1; }

echo
echo "Building the core and running the smoke test..."
cd src-tauri
AEROEDITS_REQUIRE_FFMPEG=1 cargo test --lib smoke -- --nocapture 2>&1 | grep -E '^(smoke:|SKIPPED|test |test result)|panicked|error(\[|:)'
status=${PIPESTATUS[0]}
if [[ $status -eq 0 ]]; then echo "OK: core builds and renders here."; else echo "FAILED (exit $status)"; fi
exit $status
