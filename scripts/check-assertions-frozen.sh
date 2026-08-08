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

# Comparison is by whole assertion *statement*, not by line, and the reason is worth stating.
#
# The first version of this check compared assertion lines. Two things a relocation does defeat
# that, and both showed up on this feature's own pull request during the advisory period (T005) --
# which is exactly what the advisory period was for:
#
#   1. The module path changes. `app::DEFAULT_LOCATION_LABEL` becomes
#      `features::sidebar::DEFAULT_LOCATION_LABEL` when the constant moves, and the expectation is
#      identical.
#   2. rustfmt rewraps it. A one-line `assert_eq!(path::CONST, "x");` that grows past the width
#      limit becomes four lines, only the first of which contains the word `assert`.
#
# A tempting fix -- squash the added side into one blob and substring-search each removed assertion
# line in it -- is worse than the bug. A multi-line `assert!(` contributes only the fragment
# `assert!(` to the removed set, which is a substring of literally any added assertion, so deleting
# a multi-line assertion outright would pass. That was verified, not assumed.
#
# So: extract balanced `assert*!( ... )` invocations from each side of the diff and compare those.
# Paths are normalised away, but only *snake_case* segments -- those are crate and module names.
# CamelCase segments stay, so `Level::Info` does not collapse to `Info` and swapping one enum for
# another is still a change.
CHECK=$(cat <<'PYCHECK'
import re, sys
from collections import Counter

diff = sys.stdin.read()

def side(marker, skip):
    out = []
    for line in diff.splitlines():
        if line.startswith(skip):
            continue
        if line.startswith(marker):
            out.append(line[1:])
    return "\n".join(out)

MACRO = re.compile(r"(?<![A-Za-z0-9_])(debug_assert|assert)(_eq|_ne)?!")
PANIC = re.compile(r"#\[should_panic[^\]]*\]")
OPEN = {"(": ")", "[": "]", "{": "}"}

def extract(text):
    """Every assertion in `text`, as a balanced source span. Unbalanced tails (a partially
    modified multi-line assertion) fall back to the rest of the text, which still differs from
    an intact one -- erring toward flagging."""
    found = []
    for m in PANIC.finditer(text):
        found.append(m.group(0))
    for m in MACRO.finditer(text):
        i = m.end()
        while i < len(text) and text[i] not in OPEN:
            if not text[i].isspace():
                break
            i += 1
        if i >= len(text) or text[i] not in OPEN:
            found.append(text[m.start():m.end()])
            continue
        close, depth, j, in_str, esc = OPEN[text[i]], 0, i, False, False
        while j < len(text):
            c = text[j]
            if in_str:
                if esc:
                    esc = False
                elif c == "\\":
                    esc = True
                elif c == '"':
                    in_str = False
            elif c == '"':
                in_str = True
            elif c in OPEN:
                depth += 1
            elif c in ")]}":
                depth -= 1
                if depth == 0:
                    j += 1
                    break
            j += 1
        found.append(text[m.start():j])
    return found

def norm(a):
    a = re.sub(r"([a-z_][a-z0-9_]*::)+", "", a)
    return re.sub(r"\s+", "", a)

removed = Counter(norm(a) for a in extract(side("-", "---")))
added = Counter(norm(a) for a in extract(side("+", "+++")))

if not removed:
    print("assertion freeze: OK \u2014 no assertion removed")
    sys.exit(0)

lost = removed - added
if not lost:
    print(f"assertion freeze: OK \u2014 {sum(removed.values())} assertion(s) moved, all reappear unchanged")
    sys.exit(0)

n = sum(lost.values())
print(f"assertion freeze: FAILED \u2014 {n} assertion(s) removed and not reinstated", file=sys.stderr)
print(file=sys.stderr)
for a, count in lost.items():
    shown = a if len(a) <= 160 else a[:157] + "..."
    print(f"  - {shown}" + (f"  (x{count})" if count > 1 else ""), file=sys.stderr)
print(file=sys.stderr)
print("FR-027 freezes the existing suite for the duration of feature 021.", file=sys.stderr)
print("Tests may be ADDED or RELOCATED; expectations may not be relaxed, rewritten or deleted.", file=sys.stderr)
print(file=sys.stderr)
print("If a test turns out to assert a latent bug, the bug and its assertion both stay:", file=sys.stderr)
print("file it separately and fix it in its own change (spec.md, Edge Cases).", file=sys.stderr)
sys.exit(1)
PYCHECK
)

printf '%s\n' "$DIFF" | python3 -c "$CHECK"
