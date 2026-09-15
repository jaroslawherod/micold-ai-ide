#!/usr/bin/env bash
# Drives `scripts/build-lock.sh` on a PATH without `flock` (feature 030, FR-016).
#
# Git Bash on a Windows runner has no `flock`, and `scripts/windows-installer.sh` builds through this
# wrapper. The repo-wide lock is a concession to many worktrees on one Linux machine; cargo's own
# target-directory lock still applies without it. Runs in a throwaway git repository, so the real
# lock file is never touched.

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel)"
SCRIPT="$ROOT/scripts/build-lock.sh"

failures=0
cases=0

pass() { printf 'ok    %s\n' "$1"; }
fail() {
  printf 'FAIL  %s\n      %s\n' "$1" "$2"
  failures=$((failures + 1))
}

# new_path_without_flock: a temp dir whose bin/ links every tool the wrapper needs except `flock`.
# Echoes the dir, which also holds an initialised git repository in repo/.
new_path_without_flock() {
  local dir tool
  dir="$(mktemp -d)"
  mkdir -p "$dir/bin" "$dir/repo"
  for tool in bash git dirname cat rm; do
    ln -s "$(command -v "$tool")" "$dir/bin/$tool"
  done
  git -C "$dir/repo" init --quiet
  echo "$dir"
}

# --- no flock ----------------------------------------------------------------------------------

d="$(new_path_without_flock)"
cases=$((cases + 1))
output="$(cd "$d/repo" && env PATH="$d/bin" "$SCRIPT" bash -c 'echo built; exit 3' 2>&1)"
status=$?
if [ "$status" -ne 3 ]; then
  fail "runs the command unlocked when flock is missing" "want exit 3 (the command's), got $status; output: $output"
elif ! printf '%s' "$output" | grep -qx 'built'; then
  fail "runs the command unlocked when flock is missing" "want the command's output \`built\`, got: $output"
else
  pass "runs the command unlocked when flock is missing"
fi
rm -rf "$d"

# -----------------------------------------------------------------------------------------------

printf '\n%d case(s), %d failure(s)\n' "$cases" "$failures"
[ "$failures" -eq 0 ]
