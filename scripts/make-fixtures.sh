#!/usr/bin/env bash
# Regenerates the tiny media fixtures in test/fixtures/ with FFmpeg.
# The committed files are what the smoke tests use; rerun this only to change them.
#
#   scripts/make-fixtures.sh            # writes test/fixtures/*
#   FFMPEG=/path/to/ffmpeg scripts/make-fixtures.sh
#
# Fixtures (all a few seconds, under 2 MB together):
#   screen.mp4  6 s, 320x180, 30 fps H.264 test pattern, no sound (a recording's screen track)
#   voice.wav   6 s, 48 kHz mono: 2 s tone, 2 s digital silence, 2 s tone (a recording's mic)
#   clip.mp4    6 s, 320x180, 30 fps H.264 + AAC, same tone/silence/tone sound (a plain video)
set -euo pipefail

FFMPEG="${FFMPEG:-ffmpeg}"
command -v "$FFMPEG" >/dev/null 2>&1 || { echo "error: ffmpeg not found (set FFMPEG=...)" >&2; exit 1; }

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
out="$root/test/fixtures"
mkdir -p "$out"

# Tone (440 Hz, about -12 dBFS), silence, tone. aevalsrc keeps it exact and deterministic.
tone_gap_tone='0.25*sin(2*PI*440*t)*(lt(t\,2)+gte(t\,4))'

"$FFMPEG" -hide_banner -v error -y \
  -f lavfi -i "testsrc2=size=320x180:rate=30:duration=6" \
  -c:v libx264 -preset veryfast -crf 30 -pix_fmt yuv420p -g 30 -bf 0 \
  -video_track_timescale 30 -an \
  "$out/screen.mp4"

"$FFMPEG" -hide_banner -v error -y \
  -f lavfi -i "aevalsrc=${tone_gap_tone}:s=48000:d=6:c=mono" \
  -c:a pcm_s16le \
  "$out/voice.wav"

"$FFMPEG" -hide_banner -v error -y \
  -f lavfi -i "testsrc2=size=320x180:rate=30:duration=6" \
  -f lavfi -i "aevalsrc=${tone_gap_tone}:s=48000:d=6:c=mono" \
  -c:v libx264 -preset veryfast -crf 30 -pix_fmt yuv420p -g 30 -bf 0 \
  -c:a aac -b:a 64k -shortest \
  "$out/clip.mp4"

ls -l "$out"
