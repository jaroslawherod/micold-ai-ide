#!/usr/bin/env bash
# Asserts the capture harness is reproducible and self-contained (feature 028, T010).
#
# Every screenshot and clip on the published site is taken from the running application, so the
# harness is the part of this feature with the most ways to be quietly wrong. Three of them are
# checked here, because each produces a *plausible* result that only fails later, in public:
#
#   1. **The display.** A capture that lands on the developer's own X display picks up their
#      wallpaper, their other windows and their theme. It looks fine to whoever ran it and is
#      unreproducible for everyone else. `display.sh` must own a display nobody else is on, with its
#      own runtime directory (FR-011b).
#   2. **The project.** A capture of a real checkout ships someone's home directory, branch names and
#      client names to a public site. The demonstration project must be fabricated, and it must be
#      fabricated the same way twice, or two publications differ for no reason anyone can see
#      (FR-011b, FR-013).
#   3. **The provider.** A real `claude` session answers differently every time and needs a network
#      and a credential. The stub replays a transcript, and it must replay it *byte-identically* —
#      a stub that is merely similar makes every publication's clips differ (FR-011c, FR-011d).
#
# Skips rather than fails when Xvfb and xdotool are absent: this runs in CI's `docs` job, which is
# deliberately a plain runner. The publication workflow installs them, and `build.sh` fails there if
# they are missing.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

failures=0
pass() { printf 'ok    %s\n' "$1"; }
skip() { printf 'skip  %s (%s)\n' "$1" "$2"; }
fail() {
  printf 'FAIL  %s\n' "$1"
  [ $# -gt 1 ] && printf '      %s\n' "$2"
  failures=$((failures + 1))
}

work="$(mktemp -d)"
cleanup() {
  [ -n "${MICOLD_CAPTURE_DISPLAY:-}" ] && site/capture/display.sh stop >/dev/null 2>&1 || true
  rm -rf "$work"
}
trap cleanup EXIT

echo "== the private display =="
if ! command -v Xvfb >/dev/null || ! command -v xdotool >/dev/null; then
  skip "display.sh starts a private X display" "Xvfb or xdotool is not installed"
else
  if env_file="$(site/capture/display.sh start 2>"$work/display.err")" && [ -n "$env_file" ]; then
    # shellcheck disable=SC1090
    . "$env_file"
    if [ -n "${DISPLAY:-}" ] && DISPLAY="$DISPLAY" xdotool getdisplaygeometry >/dev/null 2>&1; then
      pass "display.sh starts a private X display"
    else
      fail "display.sh starts a private X display" "DISPLAY=${DISPLAY:-unset} does not answer"
    fi
    # A shared runtime directory is how one capture run ends up talking to another run's session
    # bus, and how a capture on a developer's machine reaches their real session.
    case "${XDG_RUNTIME_DIR:-}" in
      "$work"* | /tmp/*) pass "the display owns a private XDG_RUNTIME_DIR" ;;
      *) fail "the display owns a private XDG_RUNTIME_DIR" "XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-unset}" ;;
    esac
    case "${WAYLAND_DISPLAY:-}" in
      "") pass "WAYLAND_DISPLAY is cleared, so the toolkit cannot reach the host session" ;;
      *) fail "WAYLAND_DISPLAY is cleared" "WAYLAND_DISPLAY=$WAYLAND_DISPLAY" ;;
    esac
    stopped_display="$DISPLAY"
    site/capture/display.sh stop >/dev/null 2>&1 || true
    unset MICOLD_CAPTURE_DISPLAY
    if DISPLAY="$stopped_display" xdotool getdisplaygeometry >/dev/null 2>&1; then
      fail "display.sh stops the display it started" "$stopped_display still answers"
    else
      pass "display.sh stops the display it started"
    fi
  else
    fail "display.sh starts a private X display" "$(tail -3 "$work/display.err")"
  fi
fi

echo
echo "== the demonstration project =="
if site/capture/demo-project.sh "$work/demo-a" > "$work/demo.log" 2>&1 \
  && site/capture/demo-project.sh "$work/demo-b" >> "$work/demo.log" 2>&1; then
  pass "demo-project.sh builds a project"

  if git -C "$work/demo-a" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    pass "the project is a git repository"
  else
    fail "the project is a git repository"
  fi

  a="$(git -C "$work/demo-a" log --format='%an|%ae|%ad|%s' --date=iso)"
  b="$(git -C "$work/demo-b" log --format='%an|%ae|%ad|%s' --date=iso)"
  if [ "$a" = "$b" ] && [ -n "$a" ]; then
    pass "commit metadata is fixed, not taken from the clock or the committer"
  else
    fail "commit metadata is fixed" "two runs differ"
  fi

  # `$HOME` is the giveaway that matters: a screenshot of a window whose title bar or sidebar shows
  # /home/<someone> ships that name to everyone who reads the page.
  if grep -rIlF "$HOME" "$work/demo-a" --exclude-dir=.git 2>/dev/null | head -1 | grep -q .; then
    fail "no host path appears in the project" "$(grep -rIlF "$HOME" "$work/demo-a" --exclude-dir=.git | head -3)"
  else
    pass "no host path appears in the project"
  fi
else
  fail "demo-project.sh builds a project" "$(tail -3 "$work/demo.log")"
fi

echo
echo "== the stub provider =="
if site/capture/stub-cli.sh --replay > "$work/run-a.txt" 2>&1 \
  && site/capture/stub-cli.sh --replay > "$work/run-b.txt" 2>&1; then
  if cmp -s "$work/run-a.txt" "$work/run-b.txt" && [ -s "$work/run-a.txt" ]; then
    pass "the stub replays its transcript byte-identically"
  else
    fail "the stub replays its transcript byte-identically" "$(diff "$work/run-a.txt" "$work/run-b.txt" | head -5)"
  fi
  # A transcript that carries a timestamp, a duration or a token count is a transcript that differs
  # between publications even when nothing changed (FR-011d).
  if grep -nEi '[0-9]{2}:[0-9]{2}:[0-9]{2}|[0-9]+ *(ms|tokens)' "$work/run-a.txt" >/dev/null; then
    fail "the transcript carries no clock or counter" "$(grep -nEi '[0-9]{2}:[0-9]{2}:[0-9]{2}|[0-9]+ *(ms|tokens)' "$work/run-a.txt" | head -3)"
  else
    pass "the transcript carries no clock or counter"
  fi
else
  fail "the stub replays its transcript" "$(tail -3 "$work/run-a.txt" 2>/dev/null)"
fi

# The application never runs the stub by its own name. It spawns the session's AI CLI -- `claude` --
# and the scene helpers put a symlink of that name on `PATH`, so the stub is always reached through a
# link, from a directory that holds nothing else. A stub that finds its transcript beside `$0`
# therefore finds nothing, and the failure arrives as a screenshot of an error message rather than as
# a failed check.
mkdir -p "$work/path"
ln -sf "$PWD/site/capture/stub-cli.sh" "$work/path/claude"
if "$work/path/claude" --replay > "$work/run-linked.txt" 2>&1 \
  && cmp -s "$work/run-a.txt" "$work/run-linked.txt"; then
  pass "the stub replays the same transcript when reached through a symlink under the provider's name"
else
  fail "the stub replays the same transcript when reached through a symlink under the provider's name" \
    "$(tail -3 "$work/run-linked.txt")"
fi

# A terminal screenshot of a session that is all one colour is a screenshot of a session nobody
# would recognise: the diff in the middle of it is the part a reader's eye goes to, and it is the
# part that is red and green in every terminal they have ever used. The colour is applied by the
# stub rather than written into the transcript, so these two assertions are a pair -- the escapes
# reach the terminal, and the file a person reads and reviews stays plain text.
if grep -q "$(printf '\033')\[32m" "$work/run-a.txt" && grep -q "$(printf '\033')\[31m" "$work/run-a.txt"; then
  pass "the replay is coloured, so the diff reads as a diff"
else
  fail "the replay is coloured, so the diff reads as a diff" "no red and green in the replayed session"
fi
if grep -q "$(printf '\033')" site/capture/transcript/claude-session.txt; then
  fail "the transcript itself is plain text" "it carries escape bytes"
else
  pass "the transcript itself is plain text"
fi

# The session the screenshots show is not always the project's default worktree -- the sidebar
# screenshot has one open in a side branch's worktree -- and the terminal pane says out loud which
# checkout and which branch the session is working in. A transcript that names one of them literally
# would say `main` in a screenshot whose sidebar clearly shows a `feat/` worktree selected, which is
# the sort of quiet contradiction a reader notices and the author never does. So the stub takes both
# from the directory it was actually started in.
if [ -d "$work/demo-a" ]; then
  wt="$work/demo-a/.claude/worktrees/feat-AF-114-route-planner"
  branch="$(git -C "$wt" branch --show-current)"
  if (cd "$wt" && "$work/path/claude" --replay) > "$work/run-worktree.txt" 2>&1; then
    if head -1 "$work/run-worktree.txt" | grep -qF "$branch" \
      && head -1 "$work/run-worktree.txt" | grep -qF "feat-AF-114-route-planner"; then
      pass "the stub names the checkout and the branch it was started in"
    else
      fail "the stub names the checkout and the branch it was started in" \
        "$(head -1 "$work/run-worktree.txt")"
    fi
  else
    fail "the stub names the checkout and the branch it was started in" \
      "$(tail -3 "$work/run-worktree.txt")"
  fi
  # A placeholder that survives into the terminal pane is published as-is.
  if grep -n '{[a-z]*}' "$work/run-worktree.txt" >/dev/null; then
    fail "no placeholder survives the replay" "$(grep -n '{[a-z]*}' "$work/run-worktree.txt" | head -3)"
  else
    pass "no placeholder survives the replay"
  fi
fi

echo
if [ "$failures" -eq 0 ]; then
  echo "the capture harness: all assertions hold"
else
  echo "the capture harness: $failures assertion(s) failed"
  exit 1
fi
