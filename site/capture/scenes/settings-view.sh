#!/usr/bin/env bash
# The settings view (feature 028, T047).
#
#   site/capture/scenes/settings-view.sh --out DIR --scheme light|dark
#
# The settings live in four sections -- appearance, terminal, environment, session service -- and
# the application shows one of them at a time beside a list of all four. So there is no frame that
# shows the theme setting, the scrollback setting and where the session service runs at once, and a
# still that appeared to would be a still of a view this application does not have. This one is of
# the section a reader lands on, with the list beside it naming the other three, which is what the
# view actually looks like; `docs/user-guide/settings.md` covers the sections in prose.

set -euo pipefail

root="$(git rev-parse --show-toplevel)"
out=""
scheme="light"

while [ $# -gt 0 ]; do
  case "$1" in
    --out) out="$2"; shift 2 ;;
    --scheme) scheme="$2"; shift 2 ;;
    *) printf 'settings-view: unknown argument: %s\n' "$1" >&2; exit 1 ;;
  esac
done
[ -n "$out" ] || { printf 'settings-view: --out is required\n' >&2; exit 1; }

# shellcheck source=site/capture/scenes/lib.sh
. "$root/site/capture/scenes/lib.sh"

scene_pin_binaries
scene_start --scheme "$scheme"

# The app bar's overflow menu, then the settings item in it.
scene_click "$scene_menu_x" "$scene_menu_y"
scene_settle 0.7
scene_click "$scene_menu_settings_x" "$scene_menu_settings_y"
scene_settle 1.0

# The view opens on appearance and this scene leaves it there. Clicking the section anyway looked
# like cheap insurance and is not: a clicked control keeps the focus ring, so the frame would
# publish a section that appears to have just been chosen rather than the one the view opens on.
scene_point_away

scene_shot "$out/settings-view-$scheme.png"

scene_stop
