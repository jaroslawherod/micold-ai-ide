#!/usr/bin/env bash
# The shell a captured session switches to (feature 028, T069).
#
# The application runs `$SHELL` for a session's plain terminal, in the session's worktree. On a
# developer's machine or a CI runner that is an interactive shell with somebody's startup file
# behind it, and the first thing it draws is a prompt made of a user name, a host name and a home
# directory -- into a frame that is published, permanently, on a public site (FR-013). It is also a
# different string on every machine, so the same clip would be different pixels each time it was
# captured (FR-011b).
#
# This is that shell with those two properties removed and nothing else changed. It is a real
# interactive bash, spawned by the application through its own code path, in the working directory
# the application chose; `--norc --noprofile` is what keeps a startup file out of it, and bash takes
# `PS1` from the environment when it has no startup file to set one from.
#
# `scene_start` points `SHELL` here. Nothing else should: a shell whose prompt is a fixed string is
# exactly wrong outside a capture.

exec env \
  PS1='aurora-fleet $ ' \
  PS2='> ' \
  HISTFILE= \
  bash --norc --noprofile -i
