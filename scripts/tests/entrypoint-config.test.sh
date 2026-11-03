#!/usr/bin/env bash
# Drives `packaging/basic/entrypoint.sh`'s read of the daemon's configuration file (#1527, FR-005).
#
# The entrypoint takes the directory of sshd's host key and authorized keys from `[ssh] dir` of
# `/run/micold/config/daemon.toml`, by default `/run/micold/ssh` (the image's sshd_config). Runs against fixture files, with a
# fake sshd that records the arguments it is started with.

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel)"
ENTRY="$ROOT/packaging/basic/entrypoint.sh"
failures=0

pass() { printf 'ok    %s\n' "$1"; }
fail() { printf 'FAIL  %s\n      %s\n' "$1" "$2"; failures=$((failures + 1)); }

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
printf '#!/bin/sh\nprintf "%%s\\n" "$@" >"%s/sshd-args"\n' "$work" >"$work/sshd"
chmod 0755 "$work/sshd"
printf 'daemon:x:99999:99999:d:/nonexistent:/bin/false\n' >"$work/passwd"
: >"$work/group"

# run_entry [config text]: runs the entrypoint; echoes the HostKey and AuthorizedKeysFile options.
run_entry() {
  rm -f "$work/sshd-args" "$work/daemon.toml"
  if [ "$#" -gt 0 ]; then
    printf '%s\n' "$1" >"$work/daemon.toml"
  fi
  env -u MISE_DATA_DIR -u XDG_DATA_HOME MICOLD_PASSWD_FILE="$work/passwd" \
    MICOLD_GROUP_FILE="$work/group" MICOLD_HOME="$work/home" MICOLD_SSHD="$work/sshd" \
    MICOLD_CONFIG_FILE="$work/daemon.toml" sh "$ENTRY" >/dev/null 2>&1 </dev/null
  grep -E '^(HostKey|AuthorizedKeysFile)=' "$work/sshd-args" 2>/dev/null
}

expect() { # <name> <dir> <output>; the default directory is the image's sshd_config, no options
  local want="HostKey=$2/host_ed25519_key
AuthorizedKeysFile=$2/authorized_keys"
  [ "$2" = /run/micold/ssh ] && want=""
  if [ "$3" = "$want" ]; then pass "$1"; else fail "$1" "$3"; fi
}

expect "no file: the default directory" /run/micold/ssh "$(run_entry)"
expect "a file without [ssh]: the default directory" /run/micold/ssh \
  "$(run_entry $'schema = 1\n[daemon]\nname = "x"\n')"
expect "[ssh] dir in a file: that directory" /srv/keys \
  "$(run_entry $'schema = 1\n[daemon]\ndir = "/wrong"\n[ssh]\ndir = "/srv/keys"  # keys\n[listen]\ndir = "/also-wrong"\n')"
expect "a literal string is read too" /srv/lit \
  "$(run_entry $'[ssh]\ndir = \'/srv/lit\'\n')"
expect "an [ssh] table without dir: the default directory" /run/micold/ssh \
  "$(run_entry $'[ssh]\n[storage]\ndir = "/no"\n')"

exit $((failures > 0))
