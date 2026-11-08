#!/usr/bin/env bash
# Pulls N evenly spaced frames from a video as PNGs, plus one contact sheet, so an agent that
# cannot watch video can look at the result (Read the PNGs) and describe it.
#
#   bash scripts/snapshot-frames.sh <video> [N=6] [out-dir]
#
# out-dir defaults to ${TMPDIR:-/tmp}/aeroedits-snapshots/<video name>/ (outside the repo).
# Prints each PNG's path with its timestamp, then the contact sheet's path.
set -euo pipefail

video="${1:?usage: snapshot-frames.sh <video> [N] [out-dir]}"
count="${2:-6}"
name="$(basename "${video%.*}")"
out="${3:-${TMPDIR:-/tmp}/aeroedits-snapshots/$name}"
[[ -f "$video" ]] || { echo "no such file: $video" >&2; exit 1; }
command -v ffmpeg >/dev/null && command -v ffprobe >/dev/null || { echo "need ffmpeg and ffprobe" >&2; exit 1; }
mkdir -p "$out"

duration="$(ffprobe -v error -show_entries format=duration -of default=nw=1:nk=1 "$video")"
echo "$video: ${duration}s, $(ffprobe -v error -select_streams v:0 -show_entries stream=width,height,r_frame_rate -of csv=p=0 "$video")"

for ((i = 0; i < count; i++)); do
  # Centre of each of N equal slices, so the first and last frames are never black edges.
  t="$(awk -v d="$duration" -v i="$i" -v n="$count" 'BEGIN { printf "%.3f", d * (i + 0.5) / n }')"
  file="$(printf '%s/frame-%02d.png' "$out" "$((i + 1))")"
  ffmpeg -hide_banner -v error -y -ss "$t" -i "$video" -frames:v 1 "$file"
  echo "  $t s -> $file"
done

cols=$(( count < 3 ? count : 3 ))
rows=$(( (count + cols - 1) / cols ))
sheet="$out/contact-sheet.png"
ffmpeg -hide_banner -v error -y -start_number 1 -i "$out/frame-%02d.png" \
  -vf "scale=480:-2,tile=${cols}x${rows}:padding=4:color=white" -frames:v 1 "$sheet"
echo "contact sheet: $sheet"
