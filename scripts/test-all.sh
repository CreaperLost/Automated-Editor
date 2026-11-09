#!/usr/bin/env bash
# Every headless check in one command. Run it before finishing any change.
#
#   bash scripts/test-all.sh          # everything
#   bash scripts/test-all.sh --quick  # skip the front-end build and the GPU-adapter test set
#
# Steps: cargo fmt --check, cargo test (all unit tests, incl. the smoke test), the GPU
# compositor/export tests on a software adapter (if one exists), front-end typecheck + tests +
# build, then the smoke test again with its summary line. clippy is reported, not gated: the
# tree is not clippy-clean yet (see docs/HANDOFF.md).
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
[[ -d "$HOME/.cargo/bin" ]] && export PATH="$HOME/.cargo/bin:$PATH"
quick=0; [[ "${1:-}" == "--quick" ]] && quick=1

export CARGO_TERM_COLOR="${CARGO_TERM_COLOR:-always}"
# On CI a missing FFmpeg must fail, not skip.
if [[ -n "${CI:-}" ]]; then export AEROEDITS_REQUIRE_FFMPEG=1; fi

results=()
failed=0
log="$(mktemp)"
step() {  # step <name> <command...>
  local name="$1"; shift
  printf '\n\033[1m==> %s\033[0m\n' "$name"
  local start=$SECONDS
  if "$@" 2>&1 | tee "$log"; [[ ${PIPESTATUS[0]} -eq 0 ]]; then
    results+=("PASS  $name ($((SECONDS - start))s)")
    local smoke
    smoke="$(grep -hE '^(smoke:|SKIPPED)' "$log" || true)"
    if [[ -n "$smoke" && -n "${GITHUB_ACTIONS:-}" ]]; then
      while IFS= read -r line; do echo "::notice title=smoke test::$line"; done <<<"$smoke"
    fi
  else
    results+=("FAIL  $name ($((SECONDS - start))s)"); failed=1
    # On GitHub Actions, the failure's last lines as an annotation: annotations are readable
    # without signing in, unlike job logs (handy from a phone or an unauthenticated session).
    if [[ -n "${GITHUB_ACTIONS:-}" ]]; then
      local tail_text
      tail_text="$(grep -vE '^\s*$' "$log" | sed 's/\x1b\[[0-9;]*m//g' | tail -n 25 | sed ':a;N;$!ba;s/%/%25/g;s/\r//g;s/\n/%0A/g')"
      echo "::error title=test-all: $name failed::$tail_text"
    fi
  fi
}
skip() { results+=("SKIP  $1: $2"); printf '\n==> %s: skipped (%s)\n' "$1" "$2"; }

have_vulkan() { ls /usr/share/vulkan/icd.d/*.json >/dev/null 2>&1; }

step "rustfmt"            bash -c 'cd src-tauri && cargo fmt --check'
step "cargo test"         bash -c 'cd src-tauri && cargo test -- --skip smoke'
if ((quick)); then
  skip "GPU tests" "--quick"
elif have_vulkan || [[ "$(uname -s)" != "Linux" ]]; then
  step "GPU tests (software adapter)" bash -c 'cd src-tauri && cargo test --lib gpu -- --include-ignored'
else
  skip "GPU tests" "no Vulkan driver (apt install mesa-vulkan-drivers)"
fi
step "smoke: silence cut, mouth noise, loudness, portrait captions -> export -> ffprobe" \
  bash -c 'cd src-tauri && cargo test --lib smoke -- --nocapture'
step "front-end typecheck" bash -c 'cd front-end && npx tsc --noEmit'
step "front-end tests"     bash -c 'cd front-end && npm run --silent test:cleanup'
if ((quick)); then
  skip "front-end build" "--quick"
else
  step "front-end build"   bash -c 'cd front-end && npm run --silent build'
fi
if command -v python3 >/dev/null 2>&1; then
  step "doc links"         python3 scripts/check_doc_links.py --all
else
  skip "doc links" "no python3"
fi
# Reported only.
if (cd src-tauri && cargo clippy --version >/dev/null 2>&1) && ((!quick)); then
  printf '\n==> clippy (not gating)\n'
  warnings=$(cd src-tauri && cargo clippy --lib --color never --message-format short 2>&1 | grep -c ': warning:' || true)
  results+=("INFO  clippy: $warnings warnings (not gating)")
fi

printf '\n\033[1mSummary\033[0m\n'
printf '  %s\n' "${results[@]}"
if [[ -n "${GITHUB_ACTIONS:-}" ]]; then
  summary="$(printf '%s%%0A' "${results[@]}")"
  echo "::notice title=test-all summary::$summary"
fi
rm -f "$log"
exit $failed
