#!/usr/bin/env bash
# Capture twice and compare, byte for byte (feature 028, T076).
#
#   site/capture/verify-determinism.sh [--only ID] [--out DIR] [--keep]
#
# FR-011d asks that two publications of the same commit produce the same pictures. That is not a
# property anything downstream can check: a screenshot with a clock in it, a host path in a header,
# or a tooltip that arrived on a slow machine and not on a fast one is a *plausible* image, and it
# passes every check the site has. The only way to find out is to do it twice.
#
# So this runs the whole capture twice into two directories and compares every file both runs wrote.
# A difference is a bug in a scene, not a tolerance to widen -- the causes worth suspecting first
# are printed with the failure.
#
# It is slow by construction (each run launches the application once per scene and scheme), so it is
# not part of the merge gate; `--only` narrows it to one id while working on that id's scene.

set -euo pipefail

root="$(git rev-parse --show-toplevel)"
out="${TMPDIR:-/tmp}/micold-determinism-$$"
only=""
keep=0

die() {
  printf 'verify-determinism: %s\n' "$1" >&2
  exit 1
}

while [ $# -gt 0 ]; do
  case "$1" in
    --only) only="$2"; shift 2 ;;
    --out) out="$2"; shift 2 ;;
    --keep) keep=1; shift ;;
    -h | --help) sed -n '2,17p' "$0"; exit 0 ;;
    *) die "unknown argument: $1" ;;
  esac
done

args=()
[ -n "$only" ] && args=(--only "$only")

mkdir -p "$out/a" "$out/b"
[ "$keep" -eq 1 ] || trap 'rm -rf "$out"' EXIT

for run in a b; do
  printf 'verify-determinism: capture %s of 2 into %s\n' "$run" "$out/$run" >&2
  "$root/site/capture/capture.sh" --out "$out/$run" ${args[@]+"${args[@]}"} ||
    die "the capture itself failed on run $run -- fix that before asking whether it repeats"
done

# Hashes of every file each run wrote, keyed by the path *within* the run's directory, so the two
# lists are comparable even though the directories are not the same.
hashes() {
  (cd "$1" && find . -type f | sort | xargs -r sha256sum)
}

if diff -u <(hashes "$out/a") <(hashes "$out/b") >"$out/diff"; then
  printf 'verify-determinism: %d file(s), identical across two captures (FR-011d)\n' \
    "$(cd "$out/a" && find . -type f | wc -l)" >&2
  exit 0
fi

# Name the files rather than the hashes: the hash of a PNG says nothing to the person who has to fix
# the scene, and the file name says which scene wrote it.
printf '\nverify-determinism: the same commit captured twice and did not produce the same files.\n\n' >&2
awk '/^[-+][^-+]/ { print "  " $NF }' "$out/diff" | sort -u >&2
cat >&2 <<'WHY'

That is a bug in a scene, not a variance to accept. What usually causes it:

  * a timer instead of a settle -- a frame taken while something is still animating comes out
    differently on a loaded machine (`scene_settle`, `scene_frame`);
  * the pointer left on a control -- the row under it draws its actions, and a tooltip follows on a
    timer that may or may not have fired (`scene_point_away`);
  * a host path, a user name or a clock in frame -- the demonstration project pins its git identity
    and dates, `$SHELL` is the capture's own stub shell, and the project lives at a fixed path;
  * a clip encoded by an ffmpeg that was told to size its thread pool from the CPU count -- the
    encode is bit-exact and single-threaded on purpose (`encode.sh`).

The two captures are kept with --keep, which is the fastest way to see what moved.
WHY
exit 1
