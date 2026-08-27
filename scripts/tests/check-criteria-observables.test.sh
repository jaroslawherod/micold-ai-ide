#!/usr/bin/env bash
# Drives `scripts/check-criteria-observables.sh` over crafted spec files (feature 010, BUG-008).
#
# A gate's own cases are exactly where this repository has been bitten before: the assertion-freeze
# check shipped with cases that "existed and nothing ran them", which is BUG-008 one level down --
# a green column standing in for coverage that was not there. So these run in CI, before the gate
# they test, and each case writes its own throwaway spec in a temp dir.
#
# What is pinned here, in the order the check will get it wrong:
#
#   1. **Opt-in.** A spec with no table must PASS. Fourteen features predate the rule; a check that
#      fails them all is a check that gets deleted, and then nothing is enforced anywhere.
#   2. **Completeness both ways.** A criterion with no row fails; a row naming a criterion the spec
#      does not define fails too. The second is not pedantry -- ids are feature-scoped, so a copied
#      row is the likeliest way a table comes to look complete while covering the wrong feature.
#   3. **The escape hatch costs something.** `human-only` with nothing after it fails. If it did
#      not, the cheapest way to satisfy this check would be to mark everything human-only, which is
#      precisely the state BUG-008 reported.

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel)"
CHECK="$ROOT/scripts/check-criteria-observables.sh"

failures=0
cases=0

HEADING='### How each criterion is observed'

# A spec with two criteria and a complete table. Cases mutate this and re-run.
new_spec() {
  local dir
  dir="$(mktemp -d)"
  cat > "$dir/spec.md" <<'MD'
## Success Criteria

- **SC-001**: Sessions survive the interface being closed.
- **SC-002**: A version mismatch is refused with both sides named.

### How each criterion is observed

| # | Observable | Where |
|---|---|---|
| SC-001 | the process and its output continue with no client reading | `tests/session_survival.rs` |
| SC-002 | the handshake's decision on a mismatched schema hash | `tests/handshake_flow.rs` |

## Notes
MD
  echo "$dir"
}

# run <name> <want_exit> <want_output_substring|-> <spec-file>
run() {
  local name="$1" want_exit="$2" want_out="$3" spec="$4"
  cases=$((cases + 1))

  local out got_exit
  out="$(cd "$ROOT" && "$CHECK" "$spec" 2>&1)"
  got_exit=$?

  if [ "$got_exit" != "$want_exit" ]; then
    printf 'FAIL  %-56s want exit=%s got=%s\n' "$name" "$want_exit" "$got_exit"
    printf '%s\n' "$out" | sed 's/^/        | /' | head -8
    failures=$((failures + 1))
    return
  fi
  if [ "$want_out" != "-" ] && ! printf '%s' "$out" | grep -qF -- "$want_out"; then
    printf 'FAIL  %-56s want output ~ %s\n' "$name" "$want_out"
    printf '%s\n' "$out" | sed 's/^/        | /' | head -8
    failures=$((failures + 1))
    return
  fi
  printf 'ok    %-56s exit=%s\n' "$name" "$got_exit"
}

# --- opt-in: the rule spreads by adoption, not by breaking every existing feature ----------------

d="$(new_spec)"
run "a complete table passes" 0 "all disposed" "$d/spec.md"
rm -rf "$d"

d="$(new_spec)"
# Everything but the table -- what the other thirty-six specs look like today.
sed -i "/$HEADING/,/^## Notes/{/^## Notes/!d}" "$d/spec.md"
run "a spec with no table is not failed" 0 "without a table" "$d/spec.md"
rm -rf "$d"

# --- completeness, both directions ---------------------------------------------------------------

d="$(new_spec)"
sed -i '/^| SC-002 |/d' "$d/spec.md"
run "a criterion with no row fails" 1 "criteria with no row" "$d/spec.md"
rm -rf "$d"

# The shape a new criterion arrives in: appended to the list, table untouched. This is the case the
# whole gate exists for -- it is what happened twenty-five times over.
d="$(new_spec)"
sed -i 's/^- \*\*SC-002\*\*.*/&\n- **SC-003**: A cold start reaches an attached state quickly./' "$d/spec.md"
run "a criterion added without a row fails" 1 "SC-003" "$d/spec.md"
rm -rf "$d"

# Ids are feature-scoped, so a row lifted from another feature's table looks perfectly plausible.
d="$(new_spec)"
sed -i '/^| SC-002 |/a | SC-019 | one leading indicator per row | `ui/sidebar.rs` |' "$d/spec.md"
run "a row for a criterion this spec lacks fails" 1 "does not define" "$d/spec.md"
rm -rf "$d"

d="$(new_spec)"
sed -i '/^| SC-001 |/a | SC-001 | something else entirely | `tests/other.rs` |' "$d/spec.md"
run "two rows for one criterion fails" 1 "more than one row" "$d/spec.md"
rm -rf "$d"

# A table in a later section must not satisfy this one: rows are read from the heading to the next
# heading, so an out-of-section row is invisible.
d="$(new_spec)"
sed -i '/^| SC-002 |/d' "$d/spec.md"
printf '\n| SC-002 | in some other section | `tests/handshake_flow.rs` |\n' >> "$d/spec.md"
run "a row outside the section does not count" 1 "SC-002" "$d/spec.md"
rm -rf "$d"

# --- the escape hatch has to cost something ------------------------------------------------------

d="$(new_spec)"
python3 - "$d/spec.md" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
s = s.replace("| SC-002 | the handshake's decision on a mismatched schema hash | `tests/handshake_flow.rs` |",
              "| SC-002 | human-only | — |")
open(p, 'w').write(s)
PY
run "human-only with no reason fails" 1 "no reason after the marker" "$d/spec.md"
rm -rf "$d"

d="$(new_spec)"
python3 - "$d/spec.md" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
s = s.replace("| SC-002 | the handshake's decision on a mismatched schema hash | `tests/handshake_flow.rs` |",
              "| SC-002 | human-only: it measures a person's reading speed, which no instrument here reads | — |")
open(p, 'w').write(s)
PY
run "human-only with a reason passes" 0 "all disposed" "$d/spec.md"
rm -rf "$d"

# "It needs a GUI" is the reason the check cannot judge, and deliberately does not try to. Recorded
# as a case so the next reader knows it is a gap and not an oversight: the rule is written in
# plan.md, and the reviewer enforces it.
d="$(new_spec)"
python3 - "$d/spec.md" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
s = s.replace("| SC-002 | the handshake's decision on a mismatched schema hash | `tests/handshake_flow.rs` |",
              "| SC-002 | human-only, because it needs a graphical user interface to look at | — |")
open(p, 'w').write(s)
PY
run "a bad reason still passes (a reviewer's job, not a grep's)" 0 "all disposed" "$d/spec.md"
rm -rf "$d"

# --- refusing to pass vacuously ------------------------------------------------------------------

# A table over a spec with no criteria is not "complete", it is describing nothing.
d="$(new_spec)"
sed -i '/^- \*\*SC-00/d' "$d/spec.md"
run "a table with no criteria at all is an error" 1 "defines no SC-xxx" "$d/spec.md"
rm -rf "$d"

run "a file that does not exist" 2 "no such file" "/nonexistent/spec.md"

# --- the real thing ------------------------------------------------------------------------------

run "feature 010's own table" 0 "all disposed" "specs/010-daemon-session-persistence/spec.md"

echo
if [ "$failures" -ne 0 ]; then
  echo "check-criteria-observables: $failures of $cases case(s) failed"
  exit 1
fi
echo "check-criteria-observables: all $cases cases passed"
