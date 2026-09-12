#!/usr/bin/env bash
# The worktree list, with a session open in one of them (feature 028, T045).
#
#   site/capture/scenes/worktree-sidebar.sh --out DIR --scheme light|dark
#
# The subject is the left-hand column: the project's own checkout, the three worktrees below it with
# the branch type each one came from, and -- under the second of them -- the session that is open
# there. That last part is what makes it a picture of the feature rather than of a list. Selection
# in this application is of a *session*, not of a worktree: clicking a worktree's label leaves the
# sidebar looking exactly as it did, so a scene that clicked one and took a frame would publish the
# resting state and call it a selection.
#
# The terminal alongside says which checkout and which branch the session is working in, and says it
# because the provider was started there -- see `stub-cli.sh`. That is the point of the frame: the
# sidebar and the terminal agree.

set -euo pipefail

root="$(git rev-parse --show-toplevel)"
out=""
scheme="light"

while [ $# -gt 0 ]; do
  case "$1" in
    --out) out="$2"; shift 2 ;;
    --scheme) scheme="$2"; shift 2 ;;
    *) printf 'worktree-sidebar: unknown argument: %s\n' "$1" >&2; exit 1 ;;
  esac
done
[ -n "$out" ] || { printf 'worktree-sidebar: --out is required\n' >&2; exit 1; }

# shellcheck source=site/capture/scenes/lib.sh
. "$root/site/capture/scenes/lib.sh"

scene_pin_binaries
scene_start --scheme "$scheme"

# The second of the three worktrees. Its row expands to hold the session, and the session below it
# is the selected one -- the state the sidebar is in whenever anybody is working.
scene_open_session "$scene_row_feat"
scene_point_away

scene_shot "$out/worktree-sidebar-$scheme.png"

scene_stop
