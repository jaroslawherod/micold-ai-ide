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
# A removal that has been adjudicated -- read, judged not to be a relaxation, and written down with
# its reason -- is subtracted first. See ADJUDICATIONS below.
#
# Usage:
#   scripts/check-assertions-frozen.sh [base-ref]      # default: origin/main
#
# Environment:
#   ASSERTION_FREEZE   auto (default) | enforce | report
#   FREEZE_BRANCH      branch name to judge scope by, when HEAD is detached (CI)
#
# Exit codes:
#   0  no assertion lost, or every loss adjudicated, or losses reported out of scope
#   1  one or more assertions removed without reappearing and without an adjudication, in scope;
#      or an adjudication that no longer names a missing assertion
#   2  bad usage / not a git repository / nothing to compare against
#
# ---------------------------------------------------------------------------------------------
# ADJUDICATIONS: THE THIRD OPTION THIS CHECK WAS MISSING  (task T074)
#
# FR-027 forbids expectations being relaxed, rewritten or deleted. It permits relocation, and it
# cannot forbid the *spelling* of an assertion changing when the thing it names changes shape: a
# function that answered `bool` and now answers `Option<T>` is asserted differently and asserts the
# same thing.
#
# spec.md Q3 settled that this must not be automated away -- a `norm()` waiver for `x.field` ->
# `x.field()` would auto-pass a faithful rename and a deliberate reversal alike. The reader is the
# adjudicator, which is why the report prints the closest surviving assertion beside each loss.
#
# What was missing was anywhere to write the verdict down. The check had two whole-run settings and
# no third option, so a branch that had adjudicated its report honestly still exited 1, and the only
# route to a green blocking job was to relax the check for everyone. Feature 021's own branch is the
# case: 34 losses, every one of them a return-type change, a message nesting or a strengthening, and
# not one a relaxation (T073).
#
# So losses named in the adjudication file are subtracted before the verdict. It is the discipline
# the guard tests in this repository already use -- `ALLOWED` in tests/feature_write_isolation.rs,
# `CORE_MEDIATED` beside it -- and it carries the same two rules:
#
#   * nothing goes in without a reason, under a heading naming the task that removed it; and
#   * nothing stays in after it stops being true. An adjudication whose assertion is no longer
#     missing FAILS the check, as loudly as an unadjudicated removal, so the file cannot outlive
#     what it permits.
#
# The second rule is the one that makes this safe. Without it the file is a place to bury anything;
# with it, an entry is a claim about the current tree that the check re-verifies on every run.
#
# ---------------------------------------------------------------------------------------------
# SCOPE: FR-027 BINDS FEATURE 021, NOT THE REPOSITORY  (spec.md Q3, task T074)
#
# FR-027 freezes the suite "for the duration of feature 021", so that a red suite unambiguously
# means the restructuring broke something. It says nothing about the other features shipping
# alongside it, each of which owns its own expectations and is entitled to change them.
#
# This job ran on every branch regardless, which is how feature 023 -- a deliberate behavior change
# with its own specification -- came to be reported for twelve assertions it had every right to
# change. Advisory, that is noise. Blocking, it stops every concurrent feature dead, which is why
# T074's promotion was held until this existed.
#
# So the check now decides scope from the change itself:
#
#   in scope  <=>  it touches specs/021-mvu-slice-architecture/, or its branch names 021
#
# Two independent signals, either sufficient, because the failure that matters is the silent one:
# an in-scope change judged out of scope is unenforced and says nothing, while the reverse is loud
# and fixed by whoever hits it. The spec-directory signal is the load-bearing one -- executing a
# task ticks tasks.md in the same commit, so a 021 change touches that directory as a matter of
# workflow rather than of naming discipline. Verified against all sixteen 021-era pull requests
# open or merged as of 2026-08-11: 8/8 in-scope touch it, 8/8 out-of-scope do not, and every
# in-scope branch also names 021.
#
# Out of scope the report still prints, in full, and exits 0. A change that rewrites twelve
# expectations should be told so even when it is allowed to -- the point is that it be deliberate.
# What out-of-scope no longer does is fail the build.
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

FEATURE_DIR="specs/021-mvu-slice-architecture/"

# On a pull request CI checks out a detached merge commit, so there is no branch name to read from
# the repository -- the workflow passes it in. Locally the symbolic ref is the answer, and
# `--abbrev-ref` yields "HEAD" when detached, which matches no feature and simply abstains.
BRANCH="${FREEZE_BRANCH:-$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo HEAD)}"

scope_reason() {
  if git diff --name-only "$MERGE_BASE" HEAD -- "$FEATURE_DIR" | grep -q .; then
    echo "it touches ${FEATURE_DIR}"
    return 0
  fi
  case "$BRANCH" in
    *021*) echo "its branch '${BRANCH}' names feature 021"; return 0 ;;
  esac
  echo "it touches neither ${FEATURE_DIR} nor a branch naming feature 021"
  return 1
}

MODE="${ASSERTION_FREEZE:-auto}"
case "$MODE" in
  enforce) SCOPE_REASON="ASSERTION_FREEZE=enforce" ;;
  report) SCOPE_REASON="ASSERTION_FREEZE=report" ;;
  auto)
    if SCOPE_REASON=$(scope_reason); then MODE=enforce; else MODE=report; fi
    ;;
  *)
    echo "error: ASSERTION_FREEZE must be auto, enforce or report (got '$MODE')" >&2
    exit 2
    ;;
esac
ADJUDICATIONS="${FREEZE_ADJUDICATIONS:-${FEATURE_DIR}assertion-adjudications.md}"
export FREEZE_MODE="$MODE" FREEZE_REASON="$SCOPE_REASON" FREEZE_ADJUDICATIONS="$ADJUDICATIONS"

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
import os, re, sys
from collections import Counter, defaultdict

# Set by the shell above from ASSERTION_FREEZE and, on 'auto', from whether the change is one made
# for feature 021. 'report' prints the identical report and exits 0 -- see the SCOPE section.
MODE = os.environ.get("FREEZE_MODE", "enforce")
REASON = os.environ.get("FREEZE_REASON", "")
ADJUDICATIONS = os.environ.get("FREEZE_ADJUDICATIONS", "")


def adjudications(path):
    """Assertions this feature has read, judged and written down, as {normalised: heading}.

    Line-oriented on purpose: a `was:` line is the key, everything else is prose for the reader, and
    the nearest `## ` heading above it is the task that removed it. An entry with no heading above
    it is refused rather than accepted quietly -- an adjudication with nobody's name on it is the
    thing this file exists to prevent.
    """
    try:
        with open(path, encoding="utf-8") as f:
            lines = f.read().splitlines()
    except FileNotFoundError:
        return {}, []
    found, heading, bad = {}, None, []
    for line in lines:
        if line.startswith("## "):
            heading = line[3:].strip()
        elif line.startswith("was: "):
            key = line[5:].strip()
            if not heading:
                bad.append(key)
            else:
                found[key] = heading
    return found, bad

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

# No fast path for `before == after`: a stale adjudication has to be caught even on a change that
# touched no assertion at all, or it would survive every run that happened not to move one.

lost = before - after
gained = after - before

# Adjudications are checked even when nothing is lost, because a stale one is exactly as wrong as
# an unadjudicated removal and would otherwise sit here indefinitely once the burn-down finished.
judged, unheaded = adjudications(ADJUDICATIONS)

# Stale means the assertion is BACK IN THE TREE, not that it is absent from this diff. The two read
# alike and are not: `a not in lost` was true of every entry the moment the branch merged, because a
# removal behind the base is missing from both sides and can never appear in a diff again. That took
# this job red on main within the hour and kept it red on every branch cut from main afterwards --
# in scope or out, since the verdict below precedes the scope gate -- which made the file unlandable
# by construction: correct on the branch, wrong in the commit after, with no commit in between to
# fix it in. `a in after` is what the report has always said and what the design paragraph above
# promised: an entry is a claim about the current tree, re-verified every run. It goes stale when
# someone reinstates the assertion, which is the burial the rule exists to prevent, and it survives
# the merge that made it true.
#
# The cost is that an entry naming an assertion that never existed -- a typo, a hand-written key --
# is no longer caught, because after the merge nothing distinguishes it from a removal that stuck.
# Nothing available at a single revision can: nobody's tree contains it either way. The heading rule
# is what covers that entry, by requiring a task and a reason a reader can check against the diff.
stale = [a for a in judged if a in after]
adjudicated = Counter({a: lost[a] for a in judged if a in lost})
lost = lost - adjudicated

if unheaded:
    print(
        f"assertion freeze: FAILED — {len(unheaded)} adjudication(s) in {ADJUDICATIONS} sit under\n"
        "no `## ` heading, so nothing names the task that removed them or why:",
        file=sys.stderr,
    )
    for a in sorted(unheaded):
        print(f"  - {a[:160]}", file=sys.stderr)
    sys.exit(1)

if stale:
    print(
        f"assertion freeze: FAILED — {len(stale)} adjudication(s) in {ADJUDICATIONS} name\n"
        "assertions that are not missing from the suite:",
        file=sys.stderr,
    )
    for a in sorted(stale):
        print(f"  - [{judged[a]}] {a[:160]}", file=sys.stderr)
    print(file=sys.stderr)
    print(
        "Delete each line. An adjudication that outlives the removal it explained is a place to\n"
        "bury the next one, which is the whole reason this file re-verifies itself every run.",
        file=sys.stderr,
    )
    sys.exit(1)

if not lost:
    total = sum(before.values())
    added = sum(gained.values())
    if adjudicated:
        print(
            f"assertion freeze: OK — {total} assertion(s) intact, {added} added, "
            f"{sum(adjudicated.values())} removal(s) adjudicated in {ADJUDICATIONS}"
        )
        for a, count in sorted(adjudicated.items()):
            suffix = f"  (x{count})" if count > 1 else ""
            print(f"  - [{judged[a]}] {a[:160]}{suffix}")
    else:
        print(f"assertion freeze: OK — {total} assertion(s) intact, {added} added")
    sys.exit(0)

n = sum(lost.values())
if MODE == "enforce":
    print(f"assertion freeze: FAILED — {n} assertion(s) removed and not reinstated", file=sys.stderr)
else:
    print(
        f"assertion freeze: {n} assertion(s) changed — reported, not enforced",
        file=sys.stderr,
    )
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

if MODE != "enforce":
    print(f"FR-027 freezes the suite for changes made FOR FEATURE 021. This is not one:", file=sys.stderr)
    print(f"{REASON}.", file=sys.stderr)
    print(file=sys.stderr)
    print("The expectations above belong to this change's own feature, which is entitled to change", file=sys.stderr)
    print("them. They are listed so that doing so is deliberate rather than incidental -- read them", file=sys.stderr)
    print("and confirm each says what you now mean. Nothing here fails the build.", file=sys.stderr)
    sys.exit(0)

print("FR-027 freezes the existing suite for the duration of feature 021.", file=sys.stderr)
print(f"This change is in scope: {REASON}.", file=sys.stderr)
print("Tests may be ADDED or RELOCATED; expectations may not be relaxed, rewritten or deleted.", file=sys.stderr)
print(file=sys.stderr)
print("If a test turns out to assert a latent bug, the bug and its assertion both stay:", file=sys.stderr)
print("file it separately and fix it in its own change (spec.md, Edge Cases).", file=sys.stderr)
print(file=sys.stderr)
print("A high-percentage 'closest surviving' line usually means the assertion was rewritten rather", file=sys.stderr)
print("than removed -- which FR-027 also forbids. Reinstate it, or adjudicate it: read it, judge", file=sys.stderr)
print(f"whether it says less than it did, and if it does not, record it in {ADJUDICATIONS}", file=sys.stderr)
print("under a heading naming the task, with the reason. A mechanism rename is NOT admitted on its", file=sys.stderr)
print("own -- spec.md Q3 refuses that waiver and says why; an adjudication is a reader's verdict, not", file=sys.stderr)
print("a pattern the check applies for you.", file=sys.stderr)
sys.exit(1)
PYCHECK
)

{
  dump_at "$MERGE_BASE"
  printf '\0\0\0SPLIT\0\0\0'
  dump_at HEAD
} | python3 -c "$CHECK"
