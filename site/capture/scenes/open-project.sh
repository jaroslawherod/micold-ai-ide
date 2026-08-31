#!/usr/bin/env bash
# The empty state, and a project opening into it (feature 028, T071).
#
#   site/capture/scenes/open-project.sh --out DIR --scheme light|dark
#
# The first window a reader ever sees is this one: no project, no sidebar, and a list of the
# projects the application already knows. `docs/user-guide/project-selection.md` describes what
# happens next; this is what it looks like.
#
# The application does not return to the empty state on its own -- it reopens whatever was last
# open -- so `scene_start --no-project` seeds the catalogue with no active project, which is the
# state a first run is in.
#
# Opening is from the known-projects list rather than from "Open a project": that button opens the
# in-app folder browser, which lists the capture machine's own filesystem -- a home directory with
# somebody's name in it (FR-013), and a different set of folders on every machine that publishes
# (FR-011b). The list route reaches the same place and photographs nothing but the demonstration
# project.

set -euo pipefail

root="$(git rev-parse --show-toplevel)"
out=""
scheme="light"

while [ $# -gt 0 ]; do
  case "$1" in
    --out) out="$2"; shift 2 ;;
    --scheme) scheme="$2"; shift 2 ;;
    *) printf 'open-project: unknown argument: %s\n' "$1" >&2; exit 1 ;;
  esac
done
[ -n "$out" ] || { printf 'open-project: --out is required\n' >&2; exit 1; }

# shellcheck source=site/capture/scenes/lib.sh
. "$root/site/capture/scenes/lib.sh"

scene_pin_binaries
scene_start --scheme "$scheme" --no-project

scene_frames_begin "$out/open-project-$scheme.frames"

scene_point_away
scene_frame
scene_frame_hold 4

scene_click "$scene_project_open_x" "$scene_project_open_y"
scene_point_away
# Opening a project scans it for worktrees, so the sidebar arrives a moment after the window
# changes; the frame is taken after both.
scene_frame 2.0
scene_frame_hold 5

scene_stop
