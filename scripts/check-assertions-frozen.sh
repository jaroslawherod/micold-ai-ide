#!/usr/bin/env bash
#
# Assertion-freeze check for feature 021 (FR-027).
#
# The restructuring's entire safety argument is that the existing test suite is the behavior
# specification: if the suite is red, the refactor broke something. That inference only holds while
# the assertions are frozen. Weaken one to make a step pass and the signal is gone -- not just for
# that step, but for every step after it.
#
# So this fails the build when a change REMOVES an assertion from crates/*/tests/ or crates/*/src/.
#
# Relocation is explicitly allowed (FR-027, as amended 2026-08-07): a quarter of the shell file is
# an inline test module whose tests must travel with their subjects for the file to be split at
# all. An assertion that disappears from one file and reappears -- identical, modulo whitespace and
# module paths -- anywhere else is a move, not a deletion.
#
# Usage:
#   scripts/check-assertions-frozen.sh [base-ref]      # default: origin/main
#
# Exit codes:
#   0  no assertion lost
#   1  one or more assertions removed without reappearing
#   2  bad usage / not a git repository / nothing to compare against
#
# ---------------------------------------------------------------------------------------------
# WHOLE FILES, NOT DIFF LINES  (issue #146)
#
# This compared assertions reconstructed from the `-` and `+` sides of a diff until #146. That is
# the natural thing to write and it has a hole big enough to drive the whole feature through:
#
#     assert!(                       assert!(
#   -     !state.enabled,       ->   +     state.enabled,
#   -     "starts off"               +     "starts ON now"
#     );                             );
#
# The `assert!(` opener never changes, so it stays *context* and appears on neither side. The
# changed lines carry no `assert` token, so the macro scanner finds nothing in either direction:
# the assertion enters neither the removed set nor the added one, and a reversed expectation passes
# as "no assertion removed". rustfmt puts `assert!(` on its own line for every assertion that
# carries a message -- which is to say, for every expectation that bothered to explain itself, the
# ones most worth freezing.
#
# The earlier design already knew the shape of this: it recorded that deleting a multi-line
# assertion outright would defeat a substring-based check, and guarded against that. Deleting one
# is caught, because the opener goes to the `-` side along with the body. *Editing* one in place is
# not, because the opener goes nowhere.
#
# There is no patch to the line-based approach that fixes this, because the bug is the approach: an
# assertion's identity must not depend on which of its lines a diff happened to touch. So both
# sides are now extracted from **whole files** at each revision and compared as multisets. A
# relocation still cancels (same assertion, some file, both sides). A rewrite no longer can.
#
# The cost is reading the test and source trees twice instead of one diff. It is a few megabytes
# and well under a second; the check runs once per push.
# ---------------------------------------------------------------------------------------------

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

# Three-dot semantics, preserved from the diff-based version: compare against the merge base, so a
# change is judged on what it did, not on what main did underneath it.
MERGE_BASE=$(git merge-base "$BASE" HEAD 2>/dev/null || true)
if [ -z "$MERGE_BASE" ]; then
  echo "error: no merge base between '$BASE' and HEAD" >&2
  exit 2
fi

# Path selection is done here rather than by pathspec, because `git ls-tree` does not accept
# ':(glob)' magic -- it fails outright, and a check that dies on its own pathspec is a check that
# stops protecting anything. The old comment claimed the ':(glob)' form was the safe choice *and*
# that the script verified the pathspec matched something; neither was true. It is verified now, in
# the extractor: an empty base side is a hard error rather than a pass.
matches_path() {
  case "$1" in
    crates/*/tests/*.rs | crates/*/src/*.rs) return 0 ;;
    *) return 1 ;;
  esac
}

# `-z` throughout: paths may contain anything except NUL, and a quoted path from `ls-tree` would
# otherwise need unquoting.
files_at() {
  git ls-tree -r -z --name-only "$1"
}

# Every matching file's contents at a revision, each preceded by a NUL-terminated path, so the
# extractor can attribute an assertion to the file it came from.
dump_at() {
  local rev="$1" path
  while IFS= read -r -d '' path; do
    matches_path "$path" || continue
    printf '\0%s\0' "$path"
    git show "$rev:$path" 2>/dev/null || true
  done < <(files_at "$rev")
}

CHECK=$(cat <<'PYCHECK'
import re, sys
from collections import Counter, defaultdict

# A reader piping this through `head` closes the stream early. Without this, Python raises
# BrokenPipeError, prints a traceback over the report it was in the middle of, and exits 120 --
# which reads like the check itself crashed. With it, the process dies by SIGPIPE like any
# well-behaved filter. CI does not pipe, so it still sees the documented 0/1/2.
try:
    import signal
    signal.signal(signal.SIGPIPE, signal.SIG_DFL)
except (ImportError, AttributeError, ValueError):
    pass  # not POSIX; the failure mode this guards against does not arise there

MACRO = re.compile(r"(?<![A-Za-z0-9_])(debug_assert|assert)(_eq|_ne)?!")
PANIC = re.compile(r"#\[should_panic[^\]]*\]")
OPEN = {"(": ")", "[": "]", "{": "}"}


def extract(text):
    """Every assertion in `text`, as a balanced source span.

    Unbalanced tails fall back to the rest of the text, which still differs from an intact
    assertion -- erring toward flagging.
    """
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
    """Whitespace and snake_case module paths are mechanism, not expectation.

    Only *snake_case* segments are stripped -- those are crate and module names. CamelCase stays,
    so `Level::Info` does not collapse to `Info` and swapping one enum variant for another is still
    a change.

    Do NOT extend this to `x.field` -> `x.field()`. It looks like the same kind of rename and is
    not: a module path carries no truth value, so stripping it is safe by construction, whereas
    `()` swaps a stored fact for a computed one and the computation is arbitrary. `assert!(s.ready)`
    -> `assert!(s.ready())` reads identically whether `ready()` is a faithful predicate or `true`.
    Refused as spec.md Q3 (feature 021), which works the case through: of the twelve such renames
    feature 023 produced, ten were faithful and two were deliberate reversals, and nothing in the
    text distinguishes them. What the reader needs is the `closest surviving` line below, which
    settles the faithful ones at a glance while still showing the other two.
    """
    a = re.sub(r"([a-z_][a-z0-9_]*::)+", "", a)
    return re.sub(r"\s+", "", a)


def parse(blob):
    """A NUL-delimited `\\0path\\0contents` stream -> {normalised assertion: Counter, and origins}."""
    counts = Counter()
    origin = defaultdict(set)
    parts = blob.split("\0")
    # parts[0] is empty (the stream starts with NUL); then path, contents, path, contents, ...
    for k in range(1, len(parts) - 1, 2):
        path, contents = parts[k], parts[k + 1]
        for a in extract(contents):
            n = norm(a)
            counts[n] += 1
            origin[n].add(path)
    return counts, origin


before_blob, after_blob = sys.stdin.read().split("\0\0\0SPLIT\0\0\0", 1)
before, before_where = parse(before_blob)
after, after_where = parse(after_blob)

# The guard the old comment promised and never implemented: a pathspec that matches nothing, or a
# tree with no assertions, must not pass silently.
if not before:
    print(
        "error: found no assertions under crates/*/tests or crates/*/src at the base revision --\n"
        "       the pathspec matched nothing, so this check would pass vacuously",
        file=sys.stderr,
    )
    sys.exit(2)

if before == after:
    print(f"assertion freeze: OK — all {sum(before.values())} assertion(s) intact")
    sys.exit(0)

lost = before - after
gained = after - before

if not lost:
    print(
        f"assertion freeze: OK — {sum(before.values())} assertion(s) intact, "
        f"{sum(gained.values())} added"
    )
    sys.exit(0)

n = sum(lost.values())
print(f"assertion freeze: FAILED — {n} assertion(s) removed and not reinstated", file=sys.stderr)
print(file=sys.stderr)


def shorten(a, limit=160):
    return a if len(a) <= limit else a[:limit - 3] + "..."


# Show the closest surviving assertion alongside each loss. Without it a report says only "this
# text is gone", and the reader has to hunt for whether it was rewritten, renamed or genuinely
# dropped -- three very different verdicts. With it, the usual case (a mechanism rename) is
# adjudicable at a glance, and a real deletion is visible by having no near neighbour.
import difflib

candidates = list(gained)
for a, count in sorted(lost.items()):
    where = ", ".join(sorted(before_where.get(a, ()))[:2]) or "?"
    print(f"  - {shorten(a)}" + (f"  (x{count})" if count > 1 else ""), file=sys.stderr)
    print(f"      was in: {where}", file=sys.stderr)
    near = difflib.get_close_matches(a, candidates, n=1, cutoff=0.75)
    if near:
        ratio = difflib.SequenceMatcher(None, a, near[0]).ratio()
        print(f"      closest surviving ({ratio:.0%}): {shorten(near[0])}", file=sys.stderr)
    else:
        print("      no near match survives — this looks like an outright deletion", file=sys.stderr)
    print(file=sys.stderr)

print("FR-027 freezes the existing suite for the duration of feature 021.", file=sys.stderr)
print("Tests may be ADDED or RELOCATED; expectations may not be relaxed, rewritten or deleted.", file=sys.stderr)
print(file=sys.stderr)
print("If a test turns out to assert a latent bug, the bug and its assertion both stay:", file=sys.stderr)
print("file it separately and fix it in its own change (spec.md, Edge Cases).", file=sys.stderr)
print(file=sys.stderr)
print("A high-percentage 'closest surviving' line usually means the assertion was rewritten rather", file=sys.stderr)
print("than removed -- which FR-027 also forbids. Reinstate it, or get the exception recorded.", file=sys.stderr)
sys.exit(1)
PYCHECK
)

{
  dump_at "$MERGE_BASE"
  printf '\0\0\0SPLIT\0\0\0'
  dump_at HEAD
} | python3 -c "$CHECK"
