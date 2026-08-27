#!/usr/bin/env bash
# The shared parts of every scene (feature 028, T021).
#
# A scene is a short script that opens the application on the demonstration project, puts it into
# one state, and takes one frame. Everything the scenes have in common lives here, because every one
# of these steps has a way of going wrong that produces a *plausible* image rather than an error:
#
#   * launching out of `target-shared/` captures whatever branch built last (CLAUDE.md);
#   * a client and a daemon from different builds refuse each other while printing matching version
#     numbers, so the window opens and shows nothing;
#   * there is no window manager on the capture display, so nothing takes input focus and every
#     keystroke goes nowhere -- silently;
#   * a window left at its default size makes each publication's images a different shape.
#
# Source it from a scene, after sourcing the display's env file:
#
#   . "$(site/capture/display.sh start)"
#   . site/capture/scenes/lib.sh
#   scene_pin_binaries
#   scene_start --scheme light
#   scene_shot "$out/overview.png"
#   scene_stop

set -euo pipefail

scene_root="$(git rev-parse --show-toplevel)"
scene_site="$scene_root/site"

# Everything this run writes lives under the display's own private directories, so two capture runs
# on one machine -- two agents, two worktrees -- cannot use each other's binaries or logs. The
# project is the one thing they do share, because its path is published; `scene_hold_project`
# serialises them over it.
scene_work="${MICOLD_CAPTURE_WORK:-${XDG_CACHE_HOME:-/tmp}/micold-capture}"
scene_pin="$scene_work/bin"
scene_stub="$scene_work/path"
scene_log="$scene_work/client.log"
scene_pid=""
scene_win=""

# The demonstration project, at a fixed and neutral path -- deliberately *not* under `$scene_work`.
#
# The application prints the active project's directory in its own header, so whatever this is set
# to is published in every frame that shows a project. `$scene_work` is private per user and per
# checkout by design, which makes it exactly the wrong thing to photograph: it carries a uid and a
# hash of one developer's worktree (FR-013), and it is a different string on every machine, so two
# publications of the same release would not produce the same pixels (FR-011b).
scene_project="${MICOLD_CAPTURE_PROJECT:-/tmp/micold-demo/aurora-fleet}"
# One capture at a time per project directory: the path above is shared by every run on the machine
# and `demo-project.sh` starts by deleting it (see `scene_hold_project`).
scene_lock="${scene_project%/*}/.capture-lock"

# The application's window, sized the same on every run: the published images are a set, and a set
# whose members are different shapes reads as a set of screenshots of different programs.
scene_width="${MICOLD_CAPTURE_WIDTH:-1440}"
scene_height="${MICOLD_CAPTURE_HEIGHT:-900}"

scene_die() {
  printf 'scene: %s\n' "$1" >&2
  [ -f "$scene_log" ] && tail -20 "$scene_log" >&2
  exit 1
}

scene_note() {
  printf 'scene: %s\n' "$1" >&2
}

# --- the binaries --------------------------------------------------------------------------------

# One build, both binaries, copied out before anything runs them.
#
# `--bin` filters the whole invocation, so both binaries have to be named or cargo silently builds
# one of them and leaves the other at whatever a different branch last put in the shared target
# directory. The client then meets a daemon it does not agree with, refuses it, and reports matching
# version numbers while doing so -- the mismatch is in the protocol schema hash, which is not
# printed. Copying both out of one invocation is what makes that impossible rather than unlikely.
scene_pin_binaries() {
  local profile="${MICOLD_CAPTURE_PROFILE:-release}"
  local flag=()
  [ "$profile" = "release" ] && flag=(--release)

  mkdir -p "$scene_pin"
  scene_note "building the client and the daemon ($profile)"
  "$scene_root/scripts/build-lock.sh" cargo build "${flag[@]+"${flag[@]}"}" \
    -p micold-client --bin micold-ai-ide \
    -p micold-daemon --bin micold-daemon >&2

  local target
  target="$("$scene_root/scripts/build-lock.sh" --print-target-dir)/$profile"
  [ -x "$target/micold-ai-ide" ] || scene_die "no client binary at $target/micold-ai-ide"
  [ -x "$target/micold-daemon" ] || scene_die "no daemon binary at $target/micold-daemon"

  # "Text file busy" if a previous run is still holding them -- which would abort the copy halfway
  # and leave a mismatched pair pinned.
  scene_stop
  cp "$target/micold-ai-ide" "$target/micold-daemon" "$scene_pin/"

  scene_note "pinned $(cd "$scene_pin" && sha256sum micold-ai-ide micold-daemon | tr '\n' ' ')"
}

# --- the run -------------------------------------------------------------------------------------

# Hold the demonstration project for the length of one scene.
#
# `$scene_project` is a fixed path (it is published, so it cannot be per-run) and `demo-project.sh`
# rebuilds it from scratch each time, deleting it first. Two capture runs on one machine would
# therefore have one rebuilding the project while the other photographs it, and the failure looks
# like a screenshot rather than an error. `flock` makes the second run wait.
#
# The descriptor is 8, and it is closed explicitly when the client is launched: a lock fd is
# inherited by every child, and the daemon outlives the scene by design, so an inherited descriptor
# would leave the lock held by a process nobody is looking at.
scene_hold_project() {
  local root="${scene_project%/*}"
  # A symlink or another user's directory at a well-known /tmp path is somebody else's decision
  # about where this run writes. Refuse it rather than follow it.
  if [ -L "$root" ] || { [ -e "$root" ] && [ ! -O "$root" ]; }; then
    scene_die "$root is not yours -- set MICOLD_CAPTURE_PROJECT to a path you own"
  fi
  mkdir -p "$root"
  exec 8>"$scene_lock"
  if ! flock -w 0 8; then
    scene_note "waiting for another capture run to release $scene_project"
    flock -w 600 8 || scene_die "another capture run has held $scene_project for 10 minutes"
  fi
}

scene_start() {
  local scheme="light"
  while [ $# -gt 0 ]; do
    case "$1" in
      --scheme) scheme="$2"; shift 2 ;;
      *) scene_die "scene_start: unknown argument: $1" ;;
    esac
  done
  case "$scheme" in light | dark) ;; *) scene_die "unknown scheme: $scheme" ;; esac

  [ -n "${DISPLAY:-}" ] || scene_die "no DISPLAY -- source the file display.sh start prints"
  [ -n "${XDG_DATA_HOME:-}" ] || scene_die "no XDG_DATA_HOME -- source the display's env file"
  [ -x "$scene_pin/micold-ai-ide" ] || scene_die "no pinned binary -- call scene_pin_binaries first"

  mkdir -p "$scene_work" "$scene_stub"

  # The provider. The application spawns whatever the session's AI CLI is by name, so the stub is
  # installed under that name and reached through the same code path as the real one.
  ln -sf "$scene_site/capture/stub-cli.sh" "$scene_stub/claude"
  PATH="$scene_stub:$PATH"
  export PATH

  scene_hold_project
  "$scene_site/capture/demo-project.sh" "$scene_project" >&2

  # The scheme is forced rather than followed: `FollowSystem` on a bare X display with no desktop
  # portal is whatever the toolkit guesses, which is not a decision the published images should be
  # left to. A page in one scheme shows a screenshot taken in that scheme (FR-032), so the scheme is
  # an input to the capture, not an observation of it.
  mkdir -p "$XDG_DATA_HOME/micold-ai-ide"
  printf '{\n  "settings_version": 4,\n  "theme": "%s"\n}\n' "$scheme" \
    >"$XDG_DATA_HOME/micold-ai-ide/settings.json"

  # `env -u WAYLAND_DISPLAY` is not belt-and-braces: winit prefers Wayland and ignores DISPLAY
  # entirely when it is set, so the window would open on the host's real session -- or not at all.
  scene_note "launching on $DISPLAY in the $scheme scheme"
  env -u WAYLAND_DISPLAY "$scene_pin/micold-ai-ide" >"$scene_log" 2>&1 8>&- &
  scene_pid=$!

  scene_wait_for_window
  scene_place_window

  # The one check that catches every pinning failure at once. It has to be read from the log,
  # because the window looks the same either way.
  if grep -qi 'contract or build mismatch\|refusing client' "$scene_log"; then
    scene_die "the client refused the daemon -- the pinned pair is not from one build"
  fi
}

scene_wait_for_window() {
  local waited=0
  while :; do
    scene_win="$(xdotool search --onlyvisible --class micold 2>/dev/null | head -1 || true)"
    [ -n "$scene_win" ] && break
    kill -0 "$scene_pid" 2>/dev/null || scene_die "the client exited before opening a window"
    waited=$((waited + 1))
    # Software rendering on a cold cache is slow; 60s is generous on purpose, and the loop exits
    # the moment the window is there, so being generous costs nothing when it is not needed.
    [ "$waited" -gt 600 ] && scene_die "no window after 60s"
    sleep 0.1
  done
  scene_note "window $scene_win is up"
}

scene_place_window() {
  xdotool windowsize "$scene_win" "$scene_width" "$scene_height"
  xdotool windowmove "$scene_win" 0 0
  # A resize is a relayout, and a frame taken during one is a frame of a half-drawn application.
  sleep 1
}

# There is no window manager on the capture display, so nothing assigns input focus and keyboard
# events are delivered nowhere at all. Every key goes through here, which is the only way that is
# not a bug waiting to be rediscovered.
scene_key() {
  xdotool windowfocus "$scene_win"
  xdotool key --clearmodifiers "$@"
}

scene_type() {
  xdotool windowfocus "$scene_win"
  xdotool type --clearmodifiers --delay 12 "$1"
}

scene_click() {
  xdotool mousemove "$1" "$2"
  xdotool click 1
}

# --- the window, by position ----------------------------------------------------------------------
#
# There is no accessibility bus on the capture display and the application exposes no scripting
# interface, so a scene reaches a control the only way anything can: by clicking where it is drawn.
# The numbers below are that "where", written down once, at the fixed window size above and with the
# demonstration project's fixed worktree list. They are measured, not guessed -- and because a click
# that lands on nothing produces a *plausible* frame rather than an error, the helpers that use them
# check afterwards that the thing they asked for actually happened.
#
# The sidebar's rows, top to bottom: the project's own checkout, then the three worktrees
# `demo-project.sh` creates, in the order the application lists them.
scene_row_default=137
scene_row_docs=183
scene_row_feat=247
scene_row_fix=311
# A row draws its actions right-aligned while it is the pointer's or the selection's row. A named
# worktree carries three (start a session, more, remove) and the default checkout carries two, so
# "start a session" sits further left on the named ones -- which is the only one a scene wants.
scene_action_start=234
# The overflow menu in the app bar, and the settings item inside it.
scene_menu_x=1400
scene_menu_y=32
scene_menu_settings_x=1262
scene_menu_settings_y=145
# The settings sections, down the left of the settings view.
scene_settings_x=144
scene_settings_appearance_y=93

# Take the pointer off every control before a frame.
#
# Hover is a state and it is the one state a capture cannot hold still: the row under the pointer
# draws its actions, and a moment later a tooltip arrives on a timer. Whether that timer has fired
# when the frame is taken depends on how loaded the machine is, so a scene that leaves the pointer
# on a control publishes one image on a busy machine and another on an idle one (FR-011d). The
# corner below is outside the application's window, so nothing in it is under the pointer at all.
scene_point_away() {
  xdotool mousemove "$((scene_width + 100))" "$((scene_height + 60))"
}

# Start a session in the worktree whose row is at $1, and prove one started.
#
# The click is on an action that is only drawn while the row is the pointer's, so the pointer goes
# there first and the row is given a moment to draw before the click. What makes this safe to build
# a published image on is the wait afterwards: the application spawns the session's provider, which
# on the capture display is `stub-cli.sh` under the provider's own name, so counting those processes
# answers "did the click land?" -- a question no screenshot can be trusted to answer about itself.
scene_open_session() {
  local row="$1" want="${2:-1}" waited=0 running
  xdotool mousemove 120 "$row"
  scene_settle 0.4
  scene_click "$scene_action_start" "$row"
  while :; do
    # `pgrep -c` prints its count and *exits non-zero* when the count is zero, so the fallback has
    # to keep the count it already printed rather than print another one.
    running="$(pgrep -fc "$scene_stub/claude" 2>/dev/null || true)"
    [ "${running:-0}" -ge "$want" ] && break
    waited=$((waited + 1))
    [ "$waited" -gt 60 ] && scene_die "no session started from the row at y=$row"
    scene_settle 0.25
  done
  # The provider has been spawned; the pane it draws into has not necessarily been painted yet.
  scene_settle 1.2
}

# Let the replayed session move on by one block. The stub advances on Enter and never on a timer, so
# the pace of every terminal frame is this scene's decision rather than the machine's load
# (FR-011d). Enter alone, and nothing typed: the terminal echoes what is typed and the transcript
# prints the question itself, so typing it here would publish it twice.
scene_session_step() {
  scene_key Return
  scene_settle "${1:-1.0}"
}

# A settle before every frame. The application animates state changes with the same motion tokens
# the site is dressed in, so a frame taken immediately after an interaction catches the transition
# rather than the state -- and a transition caught at a slightly different moment on every
# publication is a set of images that differ for no reason.
scene_settle() {
  sleep "${1:-0.6}"
}

scene_shot() {
  local out="$1"
  mkdir -p "$(dirname "$out")"
  scene_settle
  # The frame is the application's window, not the display. The root window is deliberately larger
  # than the window is placed at, so grabbing the root alone would publish a black margin down two
  # sides of every still. The rectangle is read back from the server rather than assumed from
  # `$scene_width`/`$scene_height`: a window that did not take the size it was asked for would
  # otherwise be cropped to a lie.
  #
  # The grab is still of the root. `import -window "$scene_win"` reads the window's own drawable,
  # which on a server with no compositor holds whatever was last painted into it -- the parts that
  # were never exposed are undefined, not blank, so a partly-obscured window yields a frame that
  # looks plausible and is wrong.
  # `--shell` emits WINDOW/X/Y/WIDTH/HEIGHT/SCREEN; all six are declared so none leaks out.
  local WINDOW X Y WIDTH HEIGHT SCREEN
  eval "$(xdotool getwindowgeometry --shell "$scene_win")"
  import -window root png:- | convert png:- -crop "${WIDTH}x${HEIGHT}+${X}+${Y}" +repage "$out"
  # A black or empty frame is a launch failure that looks like a screenshot. `identify` reports the
  # mean; an all-black frame has a mean of zero and is refused here rather than published.
  local mean
  mean="$(identify -format '%[fx:mean]' "$out" 2>/dev/null || echo 0)"
  case "$mean" in
    0 | 0.0 | 0.000000) scene_die "$out is a blank frame -- the application did not draw" ;;
  esac
  scene_note "captured $out"
}

scene_stop() {
  if [ -n "$scene_pid" ] && kill -0 "$scene_pid" 2>/dev/null; then
    kill "$scene_pid" 2>/dev/null || true
    wait "$scene_pid" 2>/dev/null || true
  fi
  # The daemon outlives the client by design (it is a session service), so it is stopped by name
  # *within this run's own pin directory* -- never by pattern across the machine, which would kill
  # a daemon a person on this machine is using.
  pkill -f "^$scene_pin/micold-daemon" 2>/dev/null || true
  scene_pid=""
  # Releases the project for the next run (and for the next scene of this one).
  exec 8>&-
}
