#!/usr/bin/env bash
# Self-test of scripts/client/bundle-check.mjs (#1735, SC-006): an export carrying the mock sentinel
# fails, a clean one passes, and a missing directory is a usage error, so the check cannot pass
# vacuously.

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

check() { node scripts/client/bundle-check.mjs "$@" >"$tmp/out" 2>&1; }

if check "$tmp/clean"; then pass "a clean export passes"; else fail "a clean export was refused: $(cat "$tmp/out")"; fi

status=0
check "$tmp/dirty" || status=$?
if [ "$status" = 1 ] && grep -q 'mock-chunk.js' "$tmp/out"; then
  pass "an export with the mock sentinel fails and names the file"
else
  fail "an export with the mock sentinel: exit $status, $(cat "$tmp/out")"
fi

status=0
check "$tmp/absent" || status=$?
if [ "$status" = 2 ]; then pass "a missing directory is a usage error"; else fail "a missing directory: exit $status"; fi

[ "$failures" = 0 ] || { echo "$failures failure(s)"; exit 1; }
echo "all passed"
