#!/usr/bin/env bash
# Self-test of scripts/client/bundle-check.mjs (#1735, SC-006, SC-007): an export carrying the mock
# sentinel or a terminal emulator fails, a lockfile listing a terminal emulator fails, a clean one
# passes, and a missing directory is a usage error, so the check cannot pass vacuously.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

failures=0
fail() { printf 'FAIL  %s\n' "$1"; failures=$((failures + 1)); }
pass() { printf 'ok    %s\n' "$1"; }

if ! command -v node >/dev/null 2>&1; then
  echo "skip: node is not installed"
  exit 0
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/clean/_expo/static/js/web" "$tmp/dirty/_expo/static/js/web"
echo 'console.log("app")' >"$tmp/clean/_expo/static/js/web/entry.js"
echo '<html></html>' >"$tmp/clean/index.html"
cp -r "$tmp/clean/." "$tmp/dirty/"
echo 'var s="__MICOLD_MOCK__";' >"$tmp/dirty/_expo/static/js/web/mock-chunk.js"

printf 'packages:\n\n  react@19.3.1:\n    resolution: {integrity: sha512-x}\n' >"$tmp/clean.yaml"
cp "$tmp/clean.yaml" "$tmp/xterm.yaml"
printf "\n  '@xterm/xterm@5.5.0':\n    resolution: {integrity: sha512-y}\n" >>"$tmp/xterm.yaml"
cp "$tmp/clean.yaml" "$tmp/pty.yaml"
printf '\n  node-pty@1.0.0:\n    resolution: {integrity: sha512-z}\n' >>"$tmp/pty.yaml"
mkdir -p "$tmp/term"
cp -r "$tmp/clean/." "$tmp/term/"
echo 'var t=document.querySelector(".xterm-helper-textarea");' >"$tmp/term/_expo/static/js/web/term-chunk.js"

mkdir -p "$tmp/near"
cp -r "$tmp/clean/." "$tmp/near/"
echo 'var a="shterm",b="xtermish",c="myhterm";' >"$tmp/near/_expo/static/js/web/near-chunk.js"
printf 'xterm' >"$tmp/near/_expo/static/js/web/font.ttf"

check() { node scripts/client/bundle-check.mjs "$@" >"$tmp/out" 2>&1; }
refused() { # <what> <file to name> <args...>
  local what="$1" name="$2" status=0
  shift 2
  check "$@" || status=$?
  if [ "$status" = 1 ] && grep -q "$name" "$tmp/out"; then pass "$what fails and names $name"; else fail "$what: exit $status, $(cat "$tmp/out")"; fi
}

if check "$tmp/clean" --lock "$tmp/clean.yaml"; then pass "a clean export passes"; else fail "a clean export was refused: $(cat "$tmp/out")"; fi

status=0
check "$tmp/dirty" || status=$?
if [ "$status" = 1 ] && grep -q 'mock-chunk.js' "$tmp/out"; then
  pass "an export with the mock sentinel fails and names the file"
else
  fail "an export with the mock sentinel: exit $status, $(cat "$tmp/out")"
fi

refused "a lockfile with @xterm/xterm" "@xterm/xterm" "$tmp/clean" --lock "$tmp/xterm.yaml"
refused "a lockfile with node-pty" "node-pty" "$tmp/clean" --lock "$tmp/pty.yaml"
refused "an export with a terminal emulator" "term-chunk.js" "$tmp/term" --lock "$tmp/clean.yaml"

if check "$tmp/near" --lock "$tmp/clean.yaml"; then pass "near-miss words and non-script files pass"; else fail "a near-miss export was refused: $(cat "$tmp/out")"; fi

status=0
check "$tmp/clean" --lock "$tmp/absent.yaml" || status=$?
if [ "$status" = 2 ]; then pass "a missing lockfile is a usage error"; else fail "a missing lockfile: exit $status"; fi

status=0
check "$tmp/absent" || status=$?
if [ "$status" = 2 ]; then pass "a missing directory is a usage error"; else fail "a missing directory: exit $status"; fi

[ "$failures" = 0 ] || { echo "$failures failure(s)"; exit 1; }
echo "all passed"
