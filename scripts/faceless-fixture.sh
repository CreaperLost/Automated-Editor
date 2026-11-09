#!/usr/bin/env bash
# Saved faceless productions for CI renders (test/productions/<name>/).
#
#   scripts/faceless-fixture.sh save <production folder> <name>
#       Copies a finished production into test/productions/<name>/: its project, current
#       revision, narration, graphics and research. Left out: exports, rendered graphics (CI
#       renders them again), the voice's raw takes and the fetched pages (other publishers'
#       articles; the quoted facts stay). WAVs are kept as Opus (about 20x smaller). Projects
#       keep absolute media paths, so the folder it was made in is kept in origin.txt.
#   scripts/faceless-fixture.sh restore <name>
#       Copies it back to that folder (so its paths resolve) and prints the folder.
#
# Make productions under a short fixed HOME (e.g. HOME=/tmp/aeroedits-home) so the same path
# exists on a CI runner.
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
store="$here/test/productions"

case "${1:-}" in
save)
    src="$(cd "$2" && pwd)"
    dest="$store/$3"
    rm -rf "$dest"
    mkdir -p "$dest"
    # Exports, render caches, the edit lock and the voice's raw takes are not the production.
    cp -a "$src/." "$dest/project/"
    (
        cd "$dest/project"
        rm -f .edit.lock ./*.mp4 faceless/voice/raw*.mp3
        # Fetched pages are other publishers' articles: the quotes in dossier.json stay.
        rm -rf faceless/sources
        # Rendered graphics: CI renders them again with the code under test.
        rm -f assets/graphics/*/*.webm assets/graphics/*/*.mov assets/graphics/*/*.png
        rm -rf ./*" frames" renders cache checkpoints
    )
    find "$dest/project" -name '*.wav' -print0 | while IFS= read -r -d '' wav; do
        ffmpeg -nostdin -v error -y -i "$wav" -c:a libopus -b:a 96k "$wav.opus"
        rm "$wav"
    done
    printf '%s\n' "$src" >"$dest/origin.txt"
    du -sh "$dest"
    ;;
restore)
    dest="$store/$2"
    origin="$(cat "$dest/origin.txt")"
    mkdir -p "$origin"
    cp -a "$dest/project/." "$origin/"
    find "$origin" -name '*.wav.opus' -print0 | while IFS= read -r -d '' opus; do
        ffmpeg -nostdin -v error -y -i "$opus" -ar 48000 "${opus%.opus}"
        rm "$opus"
    done
    printf '%s\n' "$origin"
    ;;
*)
    echo "Usage: $0 save <production folder> <name> | restore <name>" >&2
    exit 2
    ;;
esac
