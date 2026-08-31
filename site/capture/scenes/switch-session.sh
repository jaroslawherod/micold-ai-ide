#!/usr/bin/env bash
# Moving between a session's AI CLI and a plain shell in the same worktree (feature 028, T069).
#
#   site/capture/scenes/switch-session.sh --out DIR --scheme light|dark
#
# The claim in `docs/user-guide/worktrees-and-sessions.md` is that a session's terminal can run a
# plain shell *scoped to the session's worktree*, that both processes keep running while you move
# between them, and that coming back finds the conversation as you left it. None of that is visible
# in a still: a still of a shell is a shell, and a still of the AI tab is where the reader already
# was. So the clip runs `git status` in the shell -- which names the worktree's own branch, from the
# shell's own working directory -- and then goes back to the conversation, which is still there.
#
# The shell is a real interactive bash, spawned by the application through `$SHELL`; `scene_start`
# points that at `site/capture/stub-shell.sh` so the prompt is a fixed string rather than a
# developer's own (FR-013, FR-011b).

set -euo pipefail

root="$(git rev-parse --show-toplevel)"
out=""
scheme="light"

while [ $# -gt 0 ]; do
  case "$1" in
    --out) out="$2"; shift 2 ;;
    --scheme) scheme="$2"; shift 2 ;;
    *) printf 'switch-session: unknown argument: %s\n' "$1" >&2; exit 1 ;;
  esac
done
[ -n "$out" ] || { printf 'switch-session: --out is required\n' >&2; exit 1; }

# shellcheck source=site/capture/scenes/lib.sh
. "$root/site/capture/scenes/lib.sh"

scene_pin_binaries
scene_start --scheme "$scheme"

# The route-planner worktree, because the shell below prints its branch and the sidebar names the
# same worktree beside it.
scene_open_session "$scene_row_feat"

# The conversation has to have something in it, or the last frame -- the one that says "it is still
# there" -- is a blank pane, which says nothing. The stub provider advances on Enter and never on a
# timer, so these two steps are this scene's decision rather than the machine's speed.
scene_session_step
scene_session_step

scene_frames_begin "$out/switch-session-$scheme.frames"

scene_point_away
scene_frame
scene_frame_hold 2

# The "+" at the end of the tab strip opens a Regular Terminal instance. It appears as a numbered
# tab beside the AI tab, and the pane switches to it.
scene_click "$scene_tab_new_x" "$scene_tab_y"
scene_point_away
scene_frame 1.5
scene_frame_hold 2

scene_type "git status"
scene_point_away
scene_frame 0.8
scene_key Return
scene_point_away
scene_frame 1.5
scene_frame_hold 4

# Back to the conversation, which was never stopped.
scene_click "$scene_tab_ai_x" "$scene_tab_y"
scene_point_away
scene_frame 1.2
scene_frame_hold 4

scene_stop
