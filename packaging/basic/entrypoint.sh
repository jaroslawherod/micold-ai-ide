#!/bin/sh
# The basic image's entrypoint (feature 806, research R3).
#
# 1. The container runs as the host uid, which has no passwd entry; sshd refuses a user it cannot
#    resolve. Add one, home `$MICOLD_HOME` (the container home the client passes), when absent.
# 2. When provisioning has started a daemon build here before, start it again in the background,
#    so a restart-policy restart runs the daemon unattended (feature 027's survive-logout).
# 3. Run sshd in the foreground as this container's process.
set -eu

uid="$(id -u)"
gid="$(id -g)"
home="${MICOLD_HOME:-${HOME:-/tmp}}"

if ! grep -q "^[^:]*:[^:]*:${uid}:" /etc/passwd; then
  if [ -w /etc/passwd ]; then
    echo "micold:x:${uid}:${gid}:micold:${home}:/bin/sh" >>/etc/passwd
  else
    echo "micold-entrypoint: /etc/passwd is not writable; sshd cannot resolve uid ${uid}" >&2
  fi
fi
if ! grep -q "^[^:]*:[^:]*:${gid}:" /etc/group && [ -w /etc/group ]; then
  echo "micold:x:${gid}:" >>/etc/group
fi

export HOME="$home"
last="$HOME/.micold/remote/last"
if [ -f "$last" ]; then
  daemon="$HOME/.micold/remote/$(cat "$last")/micold-daemon"
  if [ -x "$daemon" ]; then
    "$daemon" &
  fi
fi

exec /usr/sbin/sshd -D -e -f /etc/micold/sshd_config
