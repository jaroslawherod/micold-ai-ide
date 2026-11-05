#!/bin/sh
# The basic image's entrypoint (feature 806, research R3).
#
# 1. The container runs as the host uid, which has no passwd entry; sshd refuses a user it cannot
#    resolve. Add one, home `$HOME` (the container home the client passes with `-e HOME`; `MICOLD_HOME`
#    is not a start-up variable any more, #1537), when absent.
# 2. Create the home's `~/.local/bin` and mise's shims folder, which #1289 puts on every session's
#    `PATH`, so a session that lists them sees what exists (#1439, FR-011).
# 3. When provisioning has started a daemon build here before, start it again in the background,
#    so a restart-policy restart runs the daemon unattended (feature 027's survive-logout). The
#    daemon runs the container's start script itself; a refusal is named on stderr (#1533).
# 4. Take the directory of sshd's host key and authorized keys from `[ssh] dir` of the daemon's
#    configuration file, `/run/micold/config/daemon.toml` (#1527), by default `/run/micold/ssh`.
# 5. Run sshd as this container's main process and, when the runtime stops the container, stop every
#    daemon first: the daemon is not sshd's child (the client starts it over SSH), so the runtime's
#    SIGTERM, which reaches only this process, would end it unsaved when the container goes. The
#    daemon's orderly stop saves terminal history (feature 041, FR-002).
set -eu

# Overridable so scripts/tests/basic-entrypoint.test.sh can drive this against fixture files.
passwd_file="${MICOLD_PASSWD_FILE:-/etc/passwd}"
group_file="${MICOLD_GROUP_FILE:-/etc/group}"
config_file="${MICOLD_CONFIG_FILE:-/run/micold/config/daemon.toml}"
sshd_bin="${MICOLD_SSHD:-/usr/sbin/sshd}"

uid="$(id -u)"
gid="$(id -g)"
# A runtime sets HOME=/ for a uid with no passwd entry; `/` is not writable, so fall back to /tmp.
home="${HOME:-/}"
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

# The shims folder by #1289's rule (contract toolchain-path P2): `$MISE_DATA_DIR/shims`, else
# `$XDG_DATA_HOME/mise/shims`, else `~/.local/share/mise/shims`; an empty value counts as unset.
# `mkdir -p` leaves existing folders and their files alone. A home it cannot write costs one line on
# stderr, not the container (#1439, FR-011). The folder is picked from the container's environment:
# when a project's environment-include sets MISE_DATA_DIR or XDG_DATA_HOME, the session PATH names
# that folder instead, which this does not create (it is on PATH all the same; mise creates it).
if [ -n "${MISE_DATA_DIR:-}" ]; then
  shims="$MISE_DATA_DIR/shims"
elif [ -n "${XDG_DATA_HOME:-}" ]; then
  shims="$XDG_DATA_HOME/mise/shims"
else
  shims="$HOME/.local/share/mise/shims"
fi
if ! mkdir -p "$HOME/.local/bin" "$shims" 2>/dev/null; then
  echo "micold-entrypoint: cannot create $HOME/.local/bin and $shims; sessions start without them" >&2
fi

last="$HOME/.micold/remote/last"
if [ -f "$last" ]; then
  daemon="$HOME/.micold/remote/$(cat "$last")/micold-daemon"
  if [ -x "$daemon" ]; then
    # The daemon runs the start script of `/run/micold/config/daemon.toml` itself, as this user,
    # before it listens (#1533). One that refuses exits non-zero: say so in the container's log;
    # the application shows the failure when it next connects.
    (
      status=0
      "$daemon" || status=$?
      [ "$status" -eq 0 ] || echo "micold-entrypoint: the daemon exited with status ${status}; if its start script refused, the output is in the daemon log and startup-result.json" >&2
    ) &
  fi
fi

# A daemon, not a bridge (whose command line ends in `remote bridge`), as the lifecycle probes tell
# them apart. A zombie has no command line, so it never matches.
daemon_pattern='/remote/.*/micold-daemon$'

# `[ssh] dir` of the configuration file: the value of the first `dir` key under `[ssh]`, quoted
# either way TOML allows. Absent file, table or key: the directory the client has always mounted.
ssh_dir=/run/micold/ssh
if [ -r "$config_file" ]; then
  configured="$(awk '
    /^[ \t]*\[/ { in_ssh = ($0 ~ /^[ \t]*\[ssh\][ \t]*(#.*)?$/); next }
    in_ssh && /^[ \t]*dir[ \t]*=/ {
      value = $0
      sub(/^[ \t]*dir[ \t]*=[ \t]*/, "", value)
      quote = substr(value, 1, 1)
      if (quote == "\"" || quote == "\047") {
        value = substr(value, 2)
        end = index(value, quote)
        if (end > 0) { print substr(value, 1, end - 1); exit }
      }
    }
  ' "$config_file")"
  [ -n "$configured" ] && ssh_dir="$configured"
fi

# The image's sshd_config names the default directory; a configured one is added to it on the command
# line (an option there wins over the file's AuthorizedKeysFile, and a second HostKey is loaded
# beside the first, whose file may then be absent).
if [ "$ssh_dir" = /run/micold/ssh ]; then
  "$sshd_bin" -D -e -f /etc/micold/sshd_config &
else
  "$sshd_bin" -D -e -f /etc/micold/sshd_config \
    -o "HostKey=$ssh_dir/host_ed25519_key" -o "AuthorizedKeysFile=$ssh_dir/authorized_keys" &
fi
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
