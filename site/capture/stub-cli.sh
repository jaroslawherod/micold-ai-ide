#!/usr/bin/env bash
# The provider the capture run talks to (feature 028, T020).
#
# The screenshots and clips show an AI session, so something has to answer. The real provider is the
# wrong thing to ask on three counts: it needs a credential the publication does not have, it needs
# a network the publication should not depend on, and -- decisively -- it answers differently every
# time. Clips assembled from a different conversation on every publication are clips whose changes
# mean nothing, and screenshots that cannot be reproduced cannot be reviewed (FR-011c).
#
# So this replays `transcript/claude-session.txt`, one block per step, and it advances *only* when
# the scene presses Enter -- never on a timer (FR-011d). A timer would make the capture a race
# between the recorder and the clock, which is how a clip ends up cut mid-sentence on a loaded
# machine and fine on the machine it was written on.
#
# It is installed on `PATH` under the provider's own name (`claude`) by the scene helpers, so the
# application launches it exactly as it launches the real one, through the same code path.
#
#   site/capture/stub-cli.sh            # interactive: a block, then wait for Enter
#   site/capture/stub-cli.sh --replay   # every block at once, for the harness test
#
# Any other arguments are ignored on purpose: the application passes the provider its own flags, and
# a stub that rejected them would be testing the flags rather than replaying the session.

set -euo pipefail

# `$0` is not this file. The application spawns the session's AI CLI by name, so the scene helpers
# put a symlink called `claude` on `PATH` and that is what runs -- from a directory that contains
# nothing but links. The transcript sits beside the *script*, so the link is followed back to it
# first; without this the stub prints "no transcript" into the terminal pane and the scene captures
# the error message as though it were a session.
here="$(cd "$(dirname "$(readlink -f "$0")")" && pwd -P)"
transcript="${MICOLD_CAPTURE_TRANSCRIPT:-$here/transcript/claude-session.txt}"

[ -f "$transcript" ] || {
  printf 'stub-cli.sh: no transcript at %s\n' "$transcript" >&2
  exit 1
}

# The two things the session knows about the world it was started in. The application launches the
# provider in the checkout the session belongs to, so this is how the terminal pane can say which
# worktree and which branch the reader is looking at without the transcript naming either -- a
# transcript that said `main` outright would contradict the sidebar in every screenshot taken in a
# side branch's worktree. Both are read once, before anything is printed, so the substitution costs
# nothing per line and cannot change part-way through a session.
checkout="$(basename "$PWD")"
branch="$(git symbolic-ref --quiet --short HEAD 2>/dev/null || echo "a detached HEAD")"

# --- colour ---------------------------------------------------------------------------------------
#
# The transcript is plain text on purpose: it is read, reviewed and diffed by people, and a file with
# escape bytes in it is none of those things. The colour a reader sees in the published terminal
# screenshots is applied here instead, by rule -- a removed line red, an added line green, a passing
# test green, the reader's own question bold -- which is what the real provider does and what makes
# the diff in the middle of the session read as a diff (FR-011c).
#
# The codes are the eight-colour ones on purpose. The application's terminal maps them through its
# own palette, so the same session comes out in the light scheme's colours and in the dark scheme's
# colours without the stub knowing which scheme it was started in.
esc=$(printf '\033')
colour="${MICOLD_CAPTURE_COLOUR:-1}"

paint() {
  if [ "$colour" = "1" ]; then
    printf '%s[%sm%s%s[0m\n' "$esc" "$1" "$2" "$esc"
  else
    printf '%s\n' "$2"
  fi
}

replay=0
case "${1:-}" in
  --replay) replay=1 ;;
  -h | --help) sed -n '2,22p' "$0"; exit 0 ;;
esac

# One pass over the file: comments dropped, `### step` treated as "stop here until the reader asks
# for more". `printf '%s\n'` rather than `echo` so a line of the transcript that begins with `-` or
# contains a backslash is printed as written.
# The transcript is read on fd 3, not on stdin: stdin is where the scene's Enter arrives, and a loop
# reading the file on stdin would consume its own transcript instead of waiting for the reader.
while IFS= read -r line <&3 || [ -n "$line" ]; do
  case "$line" in
    # Before the comment arm, because the step marker starts with `#` too and the first matching
    # arm wins: the other order silently turns every boundary into a comment and replays the whole
    # session in one go, which looks like a working stub right up until the clips are wrong.
    '### step')
      if [ "$replay" -eq 0 ]; then
        # The step boundary. Waiting on stdin is what makes the pace the scene's decision: the scene
        # types its prompt, presses Enter, takes its frame, and only then does the next block exist.
        IFS= read -r _ || break
      fi
      continue
      ;;
    '#'*) continue ;;
  esac
  line="${line//\{checkout\}/$checkout}"
  line="${line//\{branch\}/$branch}"
  # Whole lines rather than spans: a terminal screenshot shows lines, and a rule that painted part
  # of a line would need the transcript to carry markup, which is what keeping it plain text avoids.
  case "$line" in
    '> '*) paint 1 "$line" ;;
    '  - '*) paint 31 "$line" ;;
    '  + '*) paint 32 "$line" ;;
    *' ... ok' | '  test result: ok'*) paint 32 "$line" ;;
    '  running '*' tests') paint 2 "$line" ;;
    *) printf '%s\n' "$line" ;;
  esac
done 3<"$transcript"
