#!/bin/sh
# The basic image's entrypoint (feature 806, research R3).
#
# 1. The container runs as the host uid, which has no passwd entry; sshd refuses a user it cannot
#    resolve. Add one, home `$MICOLD_HOME` (the container home the client passes), when absent.
# 2. When provisioning has started a daemon build here before, start it again in the background,
#    so a restart-policy restart runs the daemon unattended (feature 027's survive-logout).
# 3. Run sshd as this container's main process and, when the runtime stops the container, stop every
#    daemon first: the daemon is not sshd's child (the client starts it over SSH), so the runtime's
#    SIGTERM, which reaches only this process, would end it unsaved when the container goes. The
#    daemon's orderly stop saves terminal history (feature 041, FR-002).
set -eu

# Overridable so scripts/tests/basic-entrypoint.test.sh can drive this against fixture files.
passwd_file="${MICOLD_PASSWD_FILE:-/etc/passwd}"
group_file="${MICOLD_GROUP_FILE:-/etc/group}"

uid="$(id -u)"
gid="$(id -g)"
# A runtime sets HOME=/ for a uid with no passwd entry; `/` is not writable, so fall back to /tmp.
home="${MICOLD_HOME:-${HOME:-/}}"
[ "$home" = "/" ] && home=/tmp

if ! grep -q "^[^:]*:[^:]*:${uid}:" "$passwd_file"; then
  if [ -w "$passwd_file" ]; then
    # The image's own `micold` entry is for the Kubernetes pod's uid 10001; the client logs in as
    # `micold`, so that name must resolve to this uid. Rewrite in place: /etc itself is not writable.
    rest="$(grep -v '^micold:' "$passwd_file" || true)"
    printf '%s\nmicold:x:%s:%s:micold:%s:/bin/sh\n' "$rest" "$uid" "$gid" "$home" >"$passwd_file"
  else
    echo "micold-entrypoint: $passwd_file is not writable; sshd cannot resolve uid ${uid}" >&2
  fi
elif [ -w "$passwd_file" ]; then
  # The base image's own user (`node`, uid 1000, home /home/node) already owns this uid, and sshd sets
  # a login's HOME from passwd: point its home at the provisioned one so every session sees it.
  fixed="$(awk -F: -v OFS=: -v uid="$uid" -v home="$home" '$3 == uid && !done { $6 = home; done = 1 } 1' "$passwd_file")"
  printf '%s\n' "$fixed" >"$passwd_file"
fi
if ! grep -q "^[^:]*:[^:]*:${gid}:" "$group_file" && [ -w "$group_file" ]; then
  echo "micold:x:${gid}:" >>"$group_file"
fi

export HOME="$home"
last="$HOME/.micold/remote/last"
if [ -f "$last" ]; then
  daemon="$HOME/.micold/remote/$(cat "$last")/micold-daemon"
  if [ -x "$daemon" ]; then
    "$daemon" &
  fi
fi

# A daemon, not a bridge (whose command line ends in `remote bridge`), as the lifecycle probes tell
# them apart. A zombie has no command line, so it never matches.
daemon_pattern='/remote/.*/micold-daemon$'

/usr/sbin/sshd -D -e -f /etc/micold/sshd_config &
sshd_pid=$!

stop_all() {
  trap '' TERM INT
  pkill -TERM -f "$daemon_pattern" 2>/dev/null || true
  waited=0
  while pgrep -f "$daemon_pattern" >/dev/null 2>&1 && [ "$waited" -lt 40 ]; do
    sleep 0.25
    waited=$((waited + 1))
  done
  kill -TERM "$sshd_pid" 2>/dev/null || true
  wait "$sshd_pid" 2>/dev/null || true
  exit 0
}
trap stop_all TERM INT

# `wait` also reaps the orphans this process, as the container's first, inherits.
status=0
wait "$sshd_pid" || status=$?
exit "$status"
