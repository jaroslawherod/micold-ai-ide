#!/usr/bin/env bash
# The application at rest, with the demonstration project open (feature 028, T028).
#
#   site/capture/scenes/main-window.sh --out DIR --scheme light|dark
#
# The first image a visitor sees, so it is the plainest one: the window as it opens on a project,
# with nothing selected, nothing typed and no dialog. It owns one id per scheme --
# `main-window-light` and `main-window-dark` -- and `capture.sh` runs it once for each scheme any
# page asks for.

set -euo pipefail

root="$(git rev-parse --show-toplevel)"
out=""
scheme="light"

while [ $# -gt 0 ]; do
  case "$1" in
    --out) out="$2"; shift 2 ;;
    --scheme) scheme="$2"; shift 2 ;;
    *) printf 'main-window: unknown argument: %s\n' "$1" >&2; exit 1 ;;
  esac
done
[ -n "$out" ] || { printf 'main-window: --out is required\n' >&2; exit 1; }

# shellcheck source=site/capture/scenes/lib.sh
. "$root/site/capture/scenes/lib.sh"

scene_pin_binaries
scene_start --scheme "$scheme"

# The project opens into the worktree list; a fresh launch has nothing selected, which is the state
# this image is of. `scene_shot` settles first, so the window is drawn rather than drawing.
scene_shot "$out/main-window-$scheme.png"

scene_stop
