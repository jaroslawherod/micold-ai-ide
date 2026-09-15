#!/usr/bin/env bash
# Drives `scripts/windows-install-smoke.sh` with `uname` stubbed (feature 030, FR-018).
#
# The smoke itself installs and launches a real setup .exe, so it only runs on a Windows runner (the
# `ci.yml` packaging legs and the release `windows` job). What it decides before touching Windows —
# refusing another host, refusing a bad argument — is pinned here, on any host, so a mistake there
# costs a local run instead of a CI round trip. Nothing is installed.

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel)"
SCRIPT="$ROOT/scripts/windows-install-smoke.sh"

failures=0
cases=0

# new_stubs <uname -s output>: a temp dir whose bin/ holds a `uname` stub. Echoes the dir.
new_stubs() {
  local dir
  dir="$(mktemp -d)"
  mkdir -p "$dir/bin"
  printf '#!/usr/bin/env bash\necho %q\n' "$1" >"$dir/bin/uname"
  chmod +x "$dir/bin/uname"
  echo "$dir"
}

# run_script <dir> [script args...]: runs the script with the stubs first on PATH. Sets `status` and
# `output` (stdout and stderr together).
run_script() {
  local dir="$1"
  shift
  output="$(cd "$ROOT" && env PATH="$dir/bin:$PATH" "$SCRIPT" "$@" 2>&1)"
  status=$?
}

pass() { printf 'ok    %s\n' "$1"; }
fail() {
  printf 'FAIL  %s\n      %s\n' "$1" "$2"
  failures=$((failures + 1))
}

# expect_refusal <name> <want exit status> <want output substring>: the last run_script exited with
# that status and said so.
expect_refusal() {
  cases=$((cases + 1))
  if [ "$status" -ne "$2" ]; then
    fail "$1" "want exit $2, got $status; output: $output"
  elif ! printf '%s' "$output" | grep -qF -- "$3"; then
    fail "$1" "want output containing \`$3\`, got: $output"
  else
    pass "$1"
  fi
}

# --- host --------------------------------------------------------------------------------------

d="$(new_stubs Linux)"
touch "$d/micold-ai-ide-0.0.0-x64-setup.exe"
run_script "$d" "$d/micold-ai-ide-0.0.0-x64-setup.exe"
expect_refusal "refuses to run off Windows" 1 'the Windows installer is smoke-tested on Windows'
rm -rf "$d"

# --- arguments ---------------------------------------------------------------------------------

# A Windows host from here on.
WINDOWS_UNAME=MINGW64_NT-10.0-26100

d="$(new_stubs "$WINDOWS_UNAME")"
run_script "$d"
expect_refusal "refuses to run without a setup executable" 2 'usage: windows-install-smoke.sh <setup.exe>'
touch "$d/a-setup.exe" "$d/b-setup.exe"
run_script "$d" "$d/a-setup.exe" "$d/b-setup.exe"
expect_refusal "refuses more than one setup executable" 2 'usage: windows-install-smoke.sh <setup.exe>'
# An unmatched `dist/*-setup.exe` glob reaches the script literally; say so rather than run it.
run_script "$d" "$d/dist/micold-ai-ide-*-x64-setup.exe"
expect_refusal "names a setup executable that does not exist" 2 "no setup executable at '$d/dist/micold-ai-ide-*-x64-setup.exe'"
rm -rf "$d"

# -----------------------------------------------------------------------------------------------

printf '\n%d case(s), %d failure(s)\n' "$cases" "$failures"
[ "$failures" -eq 0 ]
