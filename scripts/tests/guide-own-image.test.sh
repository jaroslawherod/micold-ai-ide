#!/usr/bin/env bash
# #1439 (FR-012): the sandbox page's own-image example starts `FROM` the basic image the
# application pins (`BASIC_IMAGE` in basic_image.rs, without its digest), so a pin PR that moves the
# tag moves the example with it; and `scripts/check-guide-own-image.sh` finds the example, and
# refuses a page without one. CI's container-daemon job builds the example (quickstart B3).

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

failures=0
fail() { printf 'FAIL  %s\n' "$1"; failures=$((failures + 1)); }
pass() { printf 'ok    %s\n' "$1"; }

pinned="$(grep -o 'pub const BASIC_IMAGE: &str = "[^"@]*' crates/micold-core/src/sandbox/basic_image.rs | sed 's/.*"//')"
from="$(scripts/check-guide-own-image.sh --print | sed -n 's/^FROM //p' | head -n1)"
if [ -n "$pinned" ] && [ "$from" = "$pinned" ]; then
  pass "the own-image example starts FROM the pinned basic image $pinned"
else
  fail "the own-image example starts FROM '$from', the pinned basic image is '$pinned'"
fi

if scripts/check-guide-own-image.sh --print | grep -q 'apt-get install.*libpq-dev'; then
  pass "the own-image example installs libpq-dev"
else
  fail "the own-image example does not install libpq-dev"
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
sed '/^```dockerfile$/,/^```$/d' docs/user-guide/sandboxed-daemon.md >"$tmp/page.md"
if MICOLD_GUIDE="$tmp/page.md" scripts/check-guide-own-image.sh --print >/dev/null 2>&1; then
  fail "a page without the example is not refused"
else
  pass "a page without the example is refused"
fi

# A Dockerfile comment inside the example is part of it, not the section's next heading.
awk '{ print } /^```dockerfile$/ && !done { print "# a comment inside the example"; done = 1 }' \
  docs/user-guide/sandboxed-daemon.md >"$tmp/commented.md"
if MICOLD_GUIDE="$tmp/commented.md" scripts/check-guide-own-image.sh --print 2>/dev/null | grep -q '^FROM '; then
  pass "a comment inside the example does not end it"
else
  fail "a comment inside the example ends it before its FROM line"
fi

[ "$failures" -eq 0 ]
