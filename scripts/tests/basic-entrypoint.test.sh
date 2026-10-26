#!/usr/bin/env bash
# Drives `packaging/basic/entrypoint.sh`'s passwd handling (issue #1287).
#
# sshd sets a login's HOME from passwd. The base image already has `node` at uid 1000 with home
# /home/node, and uid 1000 is the usual host uid, so the entrypoint must point that existing entry at
# the provisioned home ($MICOLD_HOME) instead of leaving /home/node. Runs against fixture files.
#
# And its home folders (#1439, FR-011, contract B5): before sshd starts, `~/.local/bin` and the mise
# shims folder that #1289 puts on `PATH` exist in the home, without touching what is in them; a home
# where they cannot be made costs one line on stderr, not the container.

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel)"
ENTRY="$ROOT/packaging/basic/entrypoint.sh"
uid="$(id -u)"
gid="$(id -g)"
failures=0
# The provisioned home of the passwd cases. Under a temp folder: the entrypoint now creates folders
# in the home (#1439), and a fixed path such as /home/jaro would be created on the test host.
jaro="$(mktemp -d)/jaro"

pass() { printf 'ok    %s\n' "$1"; }
fail() { printf 'FAIL  %s\n      %s\n' "$1" "$2"; failures=$((failures + 1)); }

# run_entry <passwd content>: run the entrypoint on a fixture; echoes the resulting passwd.
run_entry() {
  local d
  d="$(mktemp -d)"
  printf '%s\n' "$1" >"$d/passwd"
  : >"$d/group"
  MICOLD_PASSWD_FILE="$d/passwd" MICOLD_GROUP_FILE="$d/group" MICOLD_HOME="$jaro" \
    sh "$ENTRY" >/dev/null 2>&1 </dev/null
  cat "$d/passwd"
  rm -rf "$d"
}

out="$(run_entry "daemon:x:99999:99999:d:/nonexistent:/bin/false
node:x:${uid}:${gid}:node:/home/node:/bin/bash")"
if printf '%s\n' "$out" | grep -qx "node:x:${uid}:${gid}:node:${jaro}:/bin/bash"; then
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
if printf '%s\n' "$out" | grep -qx "micold:x:${uid}:${gid}:micold:${jaro}:/bin/sh"; then
  pass "a uid with no entry still gets a micold entry"
else
  fail "a uid with no entry still gets a micold entry" "$out"
fi

# --- home folders (1439, contract B5) ------------------------------------------------------------

# run_home <home> [VAR=value...]: run the entrypoint with <home> as MICOLD_HOME, MISE_DATA_DIR and
# XDG_DATA_HOME unset unless given. Leaves its stderr in "$err".
err="$(mktemp)"
run_home() {
  local home="$1" d
  shift
  d="$(mktemp -d)"
  printf 'daemon:x:99999:99999:d:/nonexistent:/bin/false\n' >"$d/passwd"
  : >"$d/group"
  env -u MISE_DATA_DIR -u XDG_DATA_HOME MICOLD_PASSWD_FILE="$d/passwd" MICOLD_GROUP_FILE="$d/group" \
    MICOLD_HOME="$home" "$@" sh "$ENTRY" >/dev/null 2>"$err" </dev/null
  rm -rf "$d"
}

home="$(mktemp -d)"
run_home "$home"
if [ -d "$home/.local/bin" ] && [ -d "$home/.local/share/mise/shims" ]; then
  pass "a new home gets .local/bin and .local/share/mise/shims"
else
  fail "a new home gets .local/bin and .local/share/mise/shims" "$(cd "$home" && find . | sort)"
fi
rm -rf "$home"

home="$(mktemp -d)"
run_home "$home" MISE_DATA_DIR="$home/mise-data" XDG_DATA_HOME="$home/xdg"
if [ -d "$home/mise-data/shims" ] && [ ! -e "$home/xdg" ] && [ ! -e "$home/.local/share/mise" ]; then
  pass "the shims folder follows MISE_DATA_DIR first"
else
  fail "the shims folder follows MISE_DATA_DIR first" "$(cd "$home" && find . | sort)"
fi
rm -rf "$home"

home="$(mktemp -d)"
run_home "$home" MISE_DATA_DIR= XDG_DATA_HOME="$home/xdg"
if [ -d "$home/xdg/mise/shims" ] && [ ! -e "$home/.local/share/mise" ]; then
  pass "the shims folder follows XDG_DATA_HOME when MISE_DATA_DIR is empty"
else
  fail "the shims folder follows XDG_DATA_HOME when MISE_DATA_DIR is empty" "$(cd "$home" && find . | sort)"
fi
rm -rf "$home"

home="$(mktemp -d)"
run_home "$home" MISE_DATA_DIR= XDG_DATA_HOME=
if [ -d "$home/.local/share/mise/shims" ]; then
  pass "an empty MISE_DATA_DIR and XDG_DATA_HOME count as unset"
else
  fail "an empty MISE_DATA_DIR and XDG_DATA_HOME count as unset" "$(cd "$home" && find . | sort)"
fi
rm -rf "$home"

home="$(mktemp -d)"
mkdir -p "$home/.local/bin" "$home/.local/share/mise/shims"
printf 'tool\n' >"$home/.local/bin/tool"
chmod 0750 "$home/.local/bin/tool"
printf 'shim\n' >"$home/.local/share/mise/shims/cargo"
before="$(cd "$home" && find . -exec ls -ld {} + | awk '{print $1, $5, $NF}' | sort)"
run_home "$home"
after="$(cd "$home" && find . -exec ls -ld {} + | awk '{print $1, $5, $NF}' | sort)"
if [ "$before" = "$after" ]; then
  pass "files already in both folders are unchanged"
else
  fail "files already in both folders are unchanged" "before: $before / after: $after"
fi
rm -rf "$home"

# A home where the folders cannot be made: `.local` is a file, which stops `mkdir -p` even as root.
# The entrypoint's next step (starting the last provisioned daemon) still runs.
home="$(mktemp -d)"
marker="$(mktemp -d)/started"
: >"$home/.local"
mkdir -p "$home/.micold/remote/v1"
printf 'v1\n' >"$home/.micold/remote/last"
printf '#!/bin/sh\n: >"%s"\n' "$marker" >"$home/.micold/remote/v1/micold-daemon"
chmod 0755 "$home/.micold/remote/v1/micold-daemon"
run_home "$home"
for _ in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do
  [ -e "$marker" ] && break
  sleep 0.1
done
lines="$(grep -c '^micold-entrypoint:' "$err")"
if [ "$lines" -eq 1 ] && grep -q '\.local/bin' "$err" && [ -e "$marker" ]; then
  pass "a home where the folders cannot be made: one line on stderr, and the entrypoint carries on"
else
  fail "a home where the folders cannot be made: one line on stderr, and the entrypoint carries on" \
    "entrypoint lines: $lines, daemon started: $([ -e "$marker" ] && echo yes || echo no), stderr: $(cat "$err")"
fi
rm -rf "$home" "$(dirname "$marker")" "$err"

rm -rf "$(dirname "$jaro")"

[ "$failures" -eq 0 ]
