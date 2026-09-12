#!/usr/bin/env bash
# Turn a scene's captured frames into a clip (feature 028, T067).
#
#   site/capture/encode.sh --frames DIR --id ID --out DIR [--fps N]
#
# Produces, from `<frames>/frame-0001.png` upwards:
#
#   <out>/<id>.png     the poster -- the clip's own first frame, copied
#   <out>/<id>.mp4     H.264, muted
#   <out>/<id>.webm    VP9, muted
#
# Two decisions are worth the space.
#
# **A known frame list at a fixed rate, not a recording.** `ffmpeg -f x11grab` samples on a wall
# clock: the same interaction recorded twice produces different frames, and FR-011d asks for the
# opposite. The scenes therefore drive the application step by step and capture a still after each
# step, and this reads that list. The cost is real and recorded in research §7 -- the clip shows the
# states of an interaction, not the transitions between them.
#
# **Bit-exact, or the determinism is only half true.** Deterministic frames in do not give identical
# files out. ffmpeg writes an encoder version string, a muxing date and -- in Matroska/WebM -- a
# random segment UID unless told not to, and libx264 writes its build and settings into SEI. So both
# encodes run with `-fflags +bitexact -flags +bitexact -fflags2 +bitexact` and `-map_metadata -1`,
# and with a fixed thread count: an encoder that sizes its thread pool from the CPU count partitions
# the picture differently on a different machine, and the same frames then encode to different bytes
# on a laptop and on a runner. `scripts/tests/clip-encode.test.sh` encodes twice and compares.

set -uo pipefail

frames=""
id=""
out=""
fps=2

# FR-015. Enforced here rather than in a check downstream, because the frame list is where the
# length actually comes from and the scene author is the person who can shorten it.
max_seconds=15

usage() {
  cat >&2 <<'USAGE'
usage: encode.sh --frames DIR --id ID --out DIR [--fps N]

  --frames DIR   the scene's frames, named frame-0001.png upwards
  --id ID        the manifest id -- names the three files written
  --out DIR      where to write <id>.png, <id>.mp4 and <id>.webm
  --fps N        frames per second (default 2 -- half a second a step)
USAGE
  exit 2
}

die() {
  printf 'encode.sh: %s\n' "$1" >&2
  exit 1
}

[ $# -gt 0 ] || usage

while [ $# -gt 0 ]; do
  case "$1" in
    --frames) frames="${2:-}"; shift 2 ;;
    --id) id="${2:-}"; shift 2 ;;
    --out) out="${2:-}"; shift 2 ;;
    --fps) fps="${2:-}"; shift 2 ;;
    -h | --help) usage ;;
    *) printf 'encode.sh: unknown argument: %s\n' "$1" >&2; usage ;;
  esac
done

[ -n "$frames" ] && [ -n "$id" ] && [ -n "$out" ] || usage
# ffmpeg is declared in `mise.toml`, and mise installs a tool without putting it on the PATH of a
# shell that did not go through `mise run` or `mise x`. So a `site/build.sh` run straight from a
# terminal would die on a tool the repository has already installed for it, saying only that it is
# not there. The same fallback `site/build.sh` uses for Node, for the same reason.
if ! command -v ffmpeg >/dev/null 2>&1 && command -v mise >/dev/null 2>&1; then
  # Asked twice, because the capture harness points `XDG_DATA_HOME` at a throwaway state directory
  # so the application it drives has no profile of the machine's -- and mise keeps its installs
  # under `$XDG_DATA_HOME/mise`, so inside a capture it looks in that empty directory and reports a
  # tool it installed as not installed. The second attempt asks with the variable out of the way.
  # The first is still tried first: a machine that really does keep its data home elsewhere is
  # answered correctly there, and only a run whose data home is not its own falls through.
  ffmpeg_bin="$(mise where ffmpeg 2>/dev/null || env -u XDG_DATA_HOME mise where ffmpeg 2>/dev/null || true)/bin"
  [ -x "$ffmpeg_bin/ffmpeg" ] && PATH="$ffmpeg_bin:$PATH" && export PATH
fi
command -v ffmpeg >/dev/null 2>&1 || die "ffmpeg is not installed -- see site/README.md"

[ -d "$frames" ] || die "no such frame directory: $frames"

# Sorted by name, which is why the frames are numbered rather than timestamped: the order is a
# property of the file names and not of when the capture happened.
mapfile -t list < <(find "$frames" -maxdepth 1 -name 'frame-*.png' -type f | sort)
[ "${#list[@]}" -gt 0 ] || die "no frames in $frames -- the scene captured nothing"

# `awk` rather than shell arithmetic: the rate may be fractional and the length usually is.
seconds="$(awk -v n="${#list[@]}" -v r="$fps" 'BEGIN { printf "%.2f", n / r }')"
if awk -v s="$seconds" -v m="$max_seconds" 'BEGIN { exit !(s > m) }'; then
  die "${#list[@]} frames at $fps a second is ${seconds}s, and a clip must be under ${max_seconds}s (FR-015) -- shorten the scene or raise --fps"
fi

mkdir -p "$out"

# The poster is the first frame itself, not a re-encode of it: FR-015b asks the still to be legible
# on its own, and the frame already is. Copying also makes it checkable -- the test compares hashes.
cp "${list[0]}" "$out/$id.png"

# The frame list, as a file ffmpeg reads. `-i frame-%04d.png` would work only while the numbering is
# gapless from one; a scene that captures conditionally leaves gaps, and concat does not care.
listfile="$(mktemp)"
trap 'rm -f "$listfile"' EXIT
for frame in "${list[@]}"; do
  printf "file '%s'\nduration %s\n" "$frame" "$(awk -v r="$fps" 'BEGIN { printf "%.6f", 1 / r }')" >>"$listfile"
done
# concat holds the last entry for no time unless it is repeated, so the final frame would flash past.
printf "file '%s'\n" "${list[${#list[@]} - 1]}" >>"$listfile"

# Shared by both encodes.
#
#   -an              no audio track at all, not a silent one (FR-015)
#   -pix_fmt yuv420p the only chroma subsampling every browser decodes
#   -r 30            a constant output rate; the input rate is the step rate above
#   -threads 1       see the header -- a thread count that varies is a result that varies
common=(
  -nostdin -hide_banner -loglevel error -y
  -fflags +bitexact -f concat -safe 0 -i "$listfile"
  -an -map_metadata -1 -pix_fmt yuv420p -r 30 -threads 1
  -flags +bitexact -fflags +bitexact
)

# H.264 for Safari and for anything that only ever learned one codec. `-preset veryslow` costs
# seconds on a clip this short and buys back the bytes FR-015c is counting.
ffmpeg "${common[@]}" \
  -c:v libx264 -preset veryslow -crf 26 -profile:v high -level 4.0 \
  -x264-params "log-level=none" -movflags +faststart \
  "$out/$id.mp4" || die "ffmpeg could not encode $out/$id.mp4"

# VP9 for everyone else, and smaller than the H.264 at the same quality on flat UI frames.
ffmpeg "${common[@]}" \
  -c:v libvpx-vp9 -b:v 0 -crf 34 -row-mt 0 -deadline good -cpu-used 2 \
  "$out/$id.webm" || die "ffmpeg could not encode $out/$id.webm"

printf 'encode.sh: %s -- %d frame(s), %ss (%s, %s)\n' "$id" "${#list[@]}" "$seconds" \
  "$(du -h "$out/$id.mp4" | cut -f1)" "$(du -h "$out/$id.webm" | cut -f1)" >&2
