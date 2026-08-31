#!/usr/bin/env bash
# Creating a worktree, and starting a session in it (feature 028, T068).
#
#   site/capture/scenes/create-worktree.sh --out DIR --scheme light|dark
#
# A clip rather than a still, because this is the one thing in the application that is a *sequence*:
# a form, three answers, and a row in the sidebar that was not there before, with a session running
# inside it. A still of the finished worktree shows none of that, and a still of the form shows a
# form. `docs/user-guide/worktrees-and-sessions.md` describes the same three fields in prose.
#
# The frames are taken at the points a reader would look: after each answer, and after the row
# arrives. Each is held for a moment (`scene_frame_hold`) so the clip can be followed at the pace
# somebody reads a form, not the pace a script fills one in.

set -euo pipefail

root="$(git rev-parse --show-toplevel)"
out=""
scheme="light"

while [ $# -gt 0 ]; do
  case "$1" in
    --out) out="$2"; shift 2 ;;
    --scheme) scheme="$2"; shift 2 ;;
    *) printf 'create-worktree: unknown argument: %s\n' "$1" >&2; exit 1 ;;
  esac
done
[ -n "$out" ] || { printf 'create-worktree: --out is required\n' >&2; exit 1; }

# shellcheck source=site/capture/scenes/lib.sh
. "$root/site/capture/scenes/lib.sh"

scene_pin_binaries
scene_start --scheme "$scheme"

scene_frames_begin "$out/create-worktree-$scheme.frames"

# The sidebar as it stands: three worktrees, and the control that adds a fourth.
scene_point_away
scene_frame
scene_frame_hold 2

scene_click "$scene_add_worktree_x" "$scene_add_worktree_y"
scene_point_away
scene_frame 1.2
scene_frame_hold 2

# The type. It is a list rather than a text field, so the list is opened and a row is pressed --
# both are frames, because the list is where a reader sees that the types are fixed.
scene_click "$scene_form_x" "$scene_form_type_y"
scene_point_away
scene_frame 1.0
scene_frame_hold 1
scene_click "$scene_form_x" "$scene_form_type_feat_y"
scene_point_away
scene_frame 0.8

# The ticket and the name. Typing is captured as it happens rather than pasted: the form derives the
# branch and directory from what is in it, and the derivation is the thing worth showing.
scene_click "$scene_form_x" "$scene_form_ticket_y"
scene_type "AF-126"
scene_point_away
scene_frame 0.8
scene_click "$scene_form_x" "$scene_form_name_y"
scene_type "route replay"
scene_point_away
scene_frame 0.8
scene_frame_hold 3

scene_click "$scene_form_create_x" "$scene_form_create_y"
scene_point_away
# Creating the worktree is a git operation on a real repository -- a branch, a checkout, and a
# rescan of the project. Two seconds is long enough for the demonstration project on any machine
# this runs on, and the frame after it is the answer to whether it worked.
scene_frame 2.0
scene_frame_hold 2

# The row is there; the session is what it is for. `scene_open_session` proves one started rather
# than trusting the frame, so a row that moved would fail this scene instead of publishing a clip
# that ends on a form.
scene_open_session "$scene_row_new"
scene_point_away
scene_frame
scene_frame_hold 4

scene_stop
