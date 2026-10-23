#!/usr/bin/env bash
# Drives `packaging/basic/entrypoint.sh`'s passwd handling (issue #1287).
#
# sshd sets a login's HOME from passwd. The base image already has `node` at uid 1000 with home
# /home/node, and uid 1000 is the usual host uid, so the entrypoint must point that existing entry at
# the provisioned home ($MICOLD_HOME) instead of leaving /home/node. Runs against fixture files.

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel)"
ENTRY="$ROOT/packaging/basic/entrypoint.sh"
uid="$(id -u)"
gid="$(id -g)"
failures=0

pass() { printf 'ok    %s\n' "$1"; }
fail() { printf 'FAIL  %s\n      %s\n' "$1" "$2"; failures=$((failures + 1)); }

# run_entry <passwd content>: run the entrypoint on a fixture; echoes the resulting passwd.
run_entry() {
  local d
  d="$(mktemp -d)"
  printf '%s\n' "$1" >"$d/passwd"
  : >"$d/group"
  MICOLD_PASSWD_FILE="$d/passwd" MICOLD_GROUP_FILE="$d/group" MICOLD_HOME=/home/jaro \
    sh "$ENTRY" >/dev/null 2>&1 </dev/null
  cat "$d/passwd"
  rm -rf "$d"
}

out="$(run_entry "daemon:x:99999:99999:d:/nonexistent:/bin/false
node:x:${uid}:${gid}:node:/home/node:/bin/bash")"
if printf '%s\n' "$out" | grep -qx "node:x:${uid}:${gid}:node:/home/jaro:/bin/bash"; then
  pass "an existing entry for the host uid gets the provisioned home"
else
  fail "an existing entry for the host uid gets the provisioned home" "$out"
fi
if printf '%s\n' "$out" | grep -qx 'daemon:x:99999:99999:d:/nonexistent:/bin/false'; then
  pass "other entries are untouched"
else
  fail "other entries are untouched" "$out"
fi

out="$(run_entry "daemon:x:99999:99999:d:/nonexistent:/bin/false")"
if printf '%s\n' "$out" | grep -qx "micold:x:${uid}:${gid}:micold:/home/jaro:/bin/sh"; then
  pass "a uid with no entry still gets a micold entry"
else
  fail "a uid with no entry still gets a micold entry" "$out"
fi

[ "$failures" -eq 0 ]
