#!/usr/bin/env bash
# A session mid-conversation, in the application's own terminal (feature 028, T046).
#
#   site/capture/scenes/session-terminal.sh --out DIR --scheme light|dark
#
# The subject is the terminal pane: a real session, in a real worktree, with the provider's output
# in it -- questions, an answer, and the diff the answer is about, in the red and green any reader
# recognises. Nothing here is a mock-up of a terminal; it is the application's terminal drawing what
# a provider wrote to it.
#
# The provider is `stub-cli.sh`, which replays a fixed transcript and advances only when this scene
# presses Enter (FR-011c, FR-011d). The session is opened in the `fix/AF-121-telemetry-drift`
# worktree because that is the branch the replayed conversation is about: a frame whose sidebar says
# one branch and whose terminal discusses another is a frame that quietly does not add up.

set -euo pipefail

root="$(git rev-parse --show-toplevel)"
out=""
scheme="light"

while [ $# -gt 0 ]; do
  case "$1" in
    --out) out="$2"; shift 2 ;;
    --scheme) scheme="$2"; shift 2 ;;
    *) printf 'session-terminal: unknown argument: %s\n' "$1" >&2; exit 1 ;;
  esac
done
[ -n "$out" ] || { printf 'session-terminal: --out is required\n' >&2; exit 1; }

# shellcheck source=site/capture/scenes/lib.sh
. "$root/site/capture/scenes/lib.sh"

scene_pin_binaries
scene_start --scheme "$scheme"

scene_open_session "$scene_row_fix"

# Two steps: the question about the ordering, then the one that carries the diff. A third step would
# push the diff off the top of the pane, and the diff is the reason this frame exists.
scene_session_step
scene_session_step

scene_point_away
scene_shot "$out/session-terminal-$scheme.png"

scene_stop
