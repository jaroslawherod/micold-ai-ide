#!/usr/bin/env bash
# The theme toggle, one press at a time (feature 028, T070).
#
#   site/capture/scenes/theme-follow.sh --out DIR --scheme light|dark
#
# The application's theme control is a cycle of three -- follow the system, then light, then dark --
# on a single item in the overflow menu, and the item says which one it is on. A still can show one
# of those three states and a reader has to be told about the other two; the clip presses the item
# and lets them watch the window change.
#
# It starts on `auto`, which is where a reader starts: it is the application's default, and it is
# not the same statement as "light" even though a display that reports no preference of its own
# renders it that way (`micold-core`'s `resolve`). That is why `--preference auto` exists on
# `scene_start` -- the scheme the frames are taken in is still decided here, not guessed.

set -euo pipefail

root="$(git rev-parse --show-toplevel)"
out=""
scheme="light"

while [ $# -gt 0 ]; do
  case "$1" in
    --out) out="$2"; shift 2 ;;
    --scheme) scheme="$2"; shift 2 ;;
    *) printf 'theme-follow: unknown argument: %s\n' "$1" >&2; exit 1 ;;
  esac
done
[ -n "$out" ] || { printf 'theme-follow: --out is required\n' >&2; exit 1; }

# shellcheck source=site/capture/scenes/lib.sh
. "$root/site/capture/scenes/lib.sh"

scene_pin_binaries
scene_start --scheme "$scheme" --preference auto

scene_frames_begin "$out/theme-follow-$scheme.frames"

scene_point_away
scene_frame
scene_frame_hold 1

# The menu, on "Theme: Auto".
scene_click "$scene_menu_x" "$scene_menu_y"
scene_point_away
scene_frame 1.0
scene_frame_hold 3

# Each press advances the cycle and leaves the menu open, so the item's own label is the caption:
# Auto, then Light, then Dark -- and the third press is the one the window answers.
scene_click "$scene_menu_theme_x" "$scene_menu_theme_y"
scene_point_away
scene_frame 1.0
scene_frame_hold 3

scene_click "$scene_menu_theme_x" "$scene_menu_theme_y"
scene_point_away
scene_frame 1.0
scene_frame_hold 4

# Out of the menu, on the theme that was chosen.
scene_key Escape
scene_point_away
scene_frame 1.0
scene_frame_hold 4

scene_stop
