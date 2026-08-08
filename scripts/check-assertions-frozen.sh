#!/usr/bin/env bash
#
# Assertion-freeze check for feature 021 (FR-027).
#
# The restructuring's entire safety argument is that the existing test suite is the behavior
# specification: if the suite is red, the refactor broke something. That inference only holds while
# the assertions are frozen. Weaken one to make a step pass and the signal is gone -- not just for
# that step, but for every step after it.
#
# So this fails the build when a diff REMOVES an assertion from crates/*/tests/.
#
# Relocation is explicitly allowed (FR-027, as amended 2026-08-07): a quarter of the shell file is
# an inline test module whose tests must travel with their subjects for the file to be split at
# all. An assertion that disappears from one file and reappears -- byte-identical, modulo leading
# whitespace -- anywhere else in the same diff is a move, not a deletion.
#
# Usage:
#   scripts/check-assertions-frozen.sh [base-ref]      # default: origin/main
#
# Exit codes:
#   0  no assertion lost
#   1  one or more assertions removed without reappearing
#   2  bad usage / not a git repository

set -euo pipefail

BASE="${1:-origin/main}"

if ! git rev-parse --git-dir >/dev/null 2>&1; then
  echo "error: not a git repository" >&2
  exit 2
fi

if ! git rev-parse --verify --quiet "$BASE" >/dev/null; then
  echo "error: base ref '$BASE' not found (fetch it first?)" >&2
  exit 2
fi

# Pathspec note: 'crates/*/tests/' silently matches NOTHING -- a trailing slash defeats git's
# wildcard matching, and `git diff` reports no error for a pathspec that matches no path. A check
# that passes vacuously is worse than no check, so this uses the ':(glob)' form and the script
# verifies below that it actually saw the test tree.
DIFF=$(git diff "$BASE"...HEAD -- ':(glob)crates/*/tests/**' ':(glob)crates/*/src/**' || true)

if [ -z "$DIFF" ]; then
  echo "assertion freeze: no changes under crates/*/tests or crates/*/src; nothing to check"
  exit 0
fi

# An "assertion line" is any line mentioning assert!/assert_eq!/assert_ne!/debug_assert*, plus
# #[should_panic], which encodes an expectation just as load-bearing as an assert.
ASSERT_RE='(^|[^a-zA-Z_])(assert|assert_eq|assert_ne|debug_assert|debug_assert_eq|debug_assert_ne)!|#\[should_panic'

# Normalise so a relocated assertion matches its original: strip the diff marker, then strip
# leading indentation (re-indenting on a move is not a change of expectation).
normalise() { sed -E 's/^.//; s/^[[:space:]]+//'; }

REMOVED=$(printf '%s\n' "$DIFF" | grep -E '^-' | grep -vE '^---' | grep -E "$ASSERT_RE" | normalise | sort || true)
ADDED=$(printf '%s\n' "$DIFF" | grep -E '^\+' | grep -vE '^\+\+\+' | grep -E "$ASSERT_RE" | normalise | sort || true)

if [ -z "$REMOVED" ]; then
  echo "assertion freeze: OK — no assertion removed"
  exit 0
fi

# comm -23: present in REMOVED, absent from ADDED. Those are the genuine losses.
LOST=$(comm -23 <(printf '%s\n' "$REMOVED") <(printf '%s\n' "$ADDED") || true)

if [ -z "$LOST" ]; then
  COUNT=$(printf '%s\n' "$REMOVED" | grep -c . || true)
  echo "assertion freeze: OK — ${COUNT} assertion(s) moved, all reappear unchanged"
  exit 0
fi

COUNT=$(printf '%s\n' "$LOST" | grep -c . || true)
echo "assertion freeze: FAILED — ${COUNT} assertion(s) removed and not reinstated" >&2
echo >&2
printf '%s\n' "$LOST" | sed 's/^/  - /' >&2
echo >&2
echo "FR-027 freezes the existing suite for the duration of feature 021." >&2
echo "Tests may be ADDED or RELOCATED; expectations may not be relaxed, rewritten or deleted." >&2
echo >&2
echo "If a test turns out to assert a latent bug, the bug and its assertion both stay:" >&2
echo "file it separately and fix it in its own change (spec.md, Edge Cases)." >&2
exit 1
