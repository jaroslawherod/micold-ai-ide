#!/usr/bin/env bash
# Asserts `site/capture/encode.sh` turns a frame list into a clip a reader can trust (feature 028,
# T065).
#
# A clip on this site is not a recording. The scene drives the application step by step and captures
# a still after each step, and this encoder turns that known list of frames into video at a fixed
# rate -- which is the only reason FR-011d ("two publications of the same version produce the same
# media") is achievable at all. A recorder samples on a wall clock and would produce a different
# file every run.
#
# So the assertion that matters most here is the last one: encode the same frames twice and the
# bytes are identical. Everything ffmpeg would otherwise stamp into the output -- an encoder version
# string, a muxing date, a random segment UID -- has to be off, and the only way to know it is off
# is to compare two runs. The rest of the assertions are the contract the pages depend on: an MP4
# and a WebM so every browser has one, a poster that is the clip's own first frame (FR-015a), no
# audio track (FR-015), and nothing longer than fifteen seconds (FR-015).

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

ENCODE=site/capture/encode.sh
failures=0

pass() { printf 'ok    %s\n' "$1"; }
fail() {
  printf 'FAIL  %s\n' "$1"
  [ $# -gt 1 ] && printf '      %s\n' "$2"
  failures=$((failures + 1))
}

for tool in ffmpeg ffprobe magick; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    printf 'clip-encode: %s is not installed -- the encoder cannot be tested without it\n' "$tool" >&2
    exit 1
  fi
done

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# --- the fixture frames ----------------------------------------------------------------------
#
# Shaped like a scene's output: `<id>.frames/` holding `frame-0001.png` upwards, one per step. The
# content only has to differ from frame to frame so that an encoder that silently dropped or
# reordered them would show up in the duration.

frames() {
  # $1 = directory, $2 = how many frames
  local dir="$1" count="$2" i
  mkdir -p "$dir"
  for i in $(seq 1 "$count"); do
    magick -size 320x200 "xc:rgb($((i * 12 % 256)),40,90)" \
      -fill white -pointsize 48 -annotate +20+120 "step $i" \
      "$(printf '%s/frame-%04d.png' "$dir" "$i")"
  done
}

frames "$work/in/open-project.frames" 6

run() {
  "$ENCODE" "$@" >"$work/out" 2>&1
}

expect_fail() {
  local what="$1" needle="$2"
  shift 2
  if run "$@"; then
    fail "$what" "the encoder accepted input it must refuse"
  elif grep -qiF -- "$needle" "$work/out"; then
    pass "$what"
  else
    fail "$what" "failed, but did not say \"$needle\": $(tail -5 "$work/out")"
  fi
}

# --- one good encode -------------------------------------------------------------------------

printf '== a frame list becomes a clip (FR-010, FR-015) ==\n'

mkdir -p "$work/media"
if run --frames "$work/in/open-project.frames" --id open-project --out "$work/media" --fps 2; then
  pass "a frame list encodes"
else
  fail "a frame list encodes" "$(tail -10 "$work/out")"
fi

for ext in mp4 webm png; do
  if [ -s "$work/media/open-project.$ext" ]; then
    pass "it wrote open-project.$ext"
  else
    fail "it wrote open-project.$ext" "$(tail -5 "$work/out")"
  fi
done

first="$work/in/open-project.frames/frame-0001.png"
if [ -f "$work/media/open-project.png" ] &&
  [ "$(sha256sum <"$first" | cut -d' ' -f1)" = "$(sha256sum <"$work/media/open-project.png" | cut -d' ' -f1)" ]; then
  pass "the poster is the clip's own first frame, byte for byte (FR-015a, FR-015b)"
else
  fail "the poster is the clip's own first frame" "the poster differs from frame-0001.png"
fi

# --- no audio, and not too long --------------------------------------------------------------
#
# Both are FR-015, and both are properties of the file rather than of the command line: a `-an` that
# was dropped in an edit is invisible until somebody's page starts making noise.

printf '== no audio, nothing over fifteen seconds (FR-015) ==\n'

for ext in mp4 webm; do
  streams="$(ffprobe -v error -select_streams a -show_entries stream=index -of csv=p=0 \
    "$work/media/open-project.$ext" 2>&1)"
  if [ -z "$streams" ]; then
    pass "the $ext carries no audio track"
  else
    fail "the $ext carries no audio track" "ffprobe found audio stream(s): $streams"
  fi

  duration="$(ffprobe -v error -show_entries format=duration -of csv=p=0 \
    "$work/media/open-project.$ext" 2>&1)"
  if awk -v d="$duration" 'BEGIN { exit !(d > 0 && d < 15) }' 2>/dev/null; then
    pass "the $ext is under fifteen seconds (${duration}s)"
  else
    fail "the $ext is under fifteen seconds" "ffprobe reported a duration of \"$duration\""
  fi
done

# Six frames at two a second is three seconds of clip. An encoder that dropped or duplicated frames
# would still satisfy "under fifteen seconds", so the length is checked against the frame list too.
duration="$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$work/media/open-project.mp4")"
if awk -v d="$duration" 'BEGIN { exit !(d > 2.5 && d < 3.5) }'; then
  pass "six frames at two a second is three seconds of clip"
else
  fail "six frames at two a second is three seconds of clip" "got ${duration}s"
fi

# --- the same frames twice ---------------------------------------------------------------------
#
# The whole reason clips are assembled from captured stills rather than recorded (research §7).

printf '== two runs, one result (FR-011d) ==\n'

mkdir -p "$work/media-again"
if run --frames "$work/in/open-project.frames" --id open-project --out "$work/media-again" --fps 2; then
  same=1
  for ext in mp4 webm png; do
    a="$(sha256sum <"$work/media/open-project.$ext" | cut -d' ' -f1)"
    b="$(sha256sum <"$work/media-again/open-project.$ext" | cut -d' ' -f1)"
    if [ "$a" != "$b" ]; then
      same=0
      fail "encoding the same frames twice is bit-identical" "$ext differs: $a vs $b"
    fi
  done
  [ "$same" -eq 1 ] && pass "encoding the same frames twice is bit-identical"
else
  fail "encoding the same frames twice is bit-identical" "$(tail -10 "$work/out")"
fi

# --- what it refuses ---------------------------------------------------------------------------

printf '== what it refuses ==\n'

frames "$work/in/too-long.frames" 40
expect_fail "a frame list that would run past fifteen seconds fails" "15" \
  --frames "$work/in/too-long.frames" --id too-long --out "$work/media" --fps 2

mkdir -p "$work/in/empty.frames"
expect_fail "an empty frame directory fails" "no frames" \
  --frames "$work/in/empty.frames" --id empty --out "$work/media" --fps 2

expect_fail "a frame directory that is not there fails" "nothing.frames" \
  --frames "$work/in/nothing.frames" --id nothing --out "$work/media" --fps 2

expect_fail "no arguments fails with usage" "usage"

printf '\n'
if [ "$failures" -eq 0 ]; then
  printf 'the clip encoder: all assertions hold\n'
else
  printf '%d assertion(s) failed\n' "$failures"
fi
exit "$([ "$failures" -eq 0 ] && echo 0 || echo 1)"
