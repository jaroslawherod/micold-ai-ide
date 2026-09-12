#!/usr/bin/env bash
#
# Every success criterion names its observable  (feature 010, BUG-008).
#
# A criterion states a property; it does not say how anyone would know. Six of feature 010's
# verification tasks were written as "a person operates the GUI and confirms", and five of the six
# were still unrun when everything around them was ticked -- while CI stayed green, because a
# criterion nobody can check is also a criterion nothing can fail. The sixth closed only when
# someone rewrote it around an observable a machine could read.
#
# So a spec that carries the verification table must keep it complete:
#
#   * every SC-xxx the spec defines has a row, and
#   * every row names an SC-xxx the spec defines, and
#   * a row that says `human-only` says why.
#
# The first rule is the one that matters. Writing a criterion is cheap and forgetting to say how it
# will be observed is the default, so the check exists to make the omission loud at the moment it is
# made rather than at the end of the feature, when the answer is a walkthrough nobody has time for.
#
# OPT-IN, DELIBERATELY. A spec with no `### How each criterion is observed` heading is not failed.
# Fourteen features predate this rule; failing them all today would mean either fourteen tables
# written in one sitting by someone who did not write the criteria, or -- far more likely -- the
# check being deleted. Adoption is per spec, and the table's presence is the opt-in.
#
# Usage:
#   scripts/check-criteria-observables.sh [spec.md ...]     # default: specs/*/spec.md
#
# Exit codes:
#   0  every opted-in spec's table is complete
#   1  a criterion with no row, a row with no criterion, a duplicate row, or an unexplained
#      `human-only`
#   2  bad usage / a named file does not exist

set -uo pipefail

HEADING='### How each criterion is observed'

# A criterion is defined by a list item that leads with its bolded id: `- **SC-004a**: ...`. The
# trailing text varies a lot (`(bugfix BUG-012)`, `*(added -- BUG-008)*`), so nothing after the id
# is matched.
DEF_RE='^[[:space:]]*-[[:space:]]+\*\*(SC-[0-9]+[a-z]?)\*\*'
# A row is a table line whose first cell is exactly an id. Requiring the whole cell keeps prose
# that merely mentions `SC-011a` from counting as coverage of it.
ROW_RE='^\|[[:space:]]*(SC-[0-9]+[a-z]?)[[:space:]]*\|'

fail=0
checked=0
skipped=0

report() { printf '  %s\n' "$*"; }

check_one() {
  local spec="$1"
  if ! grep -qF -- "$HEADING" "$spec"; then
    skipped=$((skipped + 1))
    return 0
  fi
  checked=$((checked + 1))

  local defined rows bad=0
  defined="$(sed -nE "s/$DEF_RE.*/\1/p" "$spec" | sort -u)"
  # Rows are read from the table's own section only -- from the heading to the next heading of the
  # same or higher level -- so a table in a later section cannot satisfy this one.
  rows="$(awk -v h="$HEADING" '
      index($0, h) == 1 { inside = 1; next }
      inside && /^#{1,3} / { inside = 0 }
      inside { print }
    ' "$spec" | sed -nE "s/$ROW_RE.*/\1/p")"

  if [ -z "$defined" ]; then
    printf 'FAIL  %s\n' "$spec"
    report "the verification table is present but the spec defines no SC-xxx criteria."
    report "Either the criteria moved and the table is now describing nothing, or the ids are"
    report "written in a shape this check does not recognise (expected: '- **SC-004a**: ...')."
    fail=1
    return 0
  fi

  local dupes
  dupes="$(printf '%s\n' "$rows" | sort | uniq -d)"
  local uniq_rows
  uniq_rows="$(printf '%s\n' "$rows" | sort -u | sed '/^$/d')"

  local missing extra
  missing="$(comm -23 <(printf '%s\n' "$defined") <(printf '%s\n' "$uniq_rows"))"
  extra="$(comm -13 <(printf '%s\n' "$defined") <(printf '%s\n' "$uniq_rows"))"

  if [ -n "$missing" ]; then
    bad=1
    printf 'FAIL  %s\n' "$spec"
    report "criteria with no row in the verification table:"
    printf '%s\n' "$missing" | sed 's/^/        /'
    report "Say what would be observed and where it is asserted, or mark it human-only and say why."
  fi
  if [ -n "$extra" ]; then
    [ "$bad" = 1 ] || printf 'FAIL  %s\n' "$spec"
    bad=1
    report "rows naming a criterion this spec does not define:"
    printf '%s\n' "$extra" | sed 's/^/        /'
    report "Ids are feature-scoped -- SC-001 names a different criterion in most other features -- so"
    report "a row like this is usually a row copied from elsewhere, not a criterion gone missing."
  fi
  if [ -n "$dupes" ]; then
    [ "$bad" = 1 ] || printf 'FAIL  %s\n' "$spec"
    bad=1
    report "more than one row for:"
    printf '%s\n' "$dupes" | sed 's/^/        /'
    report "Two dispositions for one criterion means the reader picks; the writer should have."
  fi

  # `human-only` is the escape hatch, so it is the one that has to cost something. The marker may
  # not end the sentence: whatever follows it is the reason, and a reason is more than a few
  # characters. Nothing here judges the reason -- a check cannot -- but an unexplained waiver is
  # exactly the "written as a walkthrough and forgotten" shape this whole gate is about.
  local unexplained
  unexplained="$(awk -v h="$HEADING" '
      index($0, h) == 1 { inside = 1; next }
      inside && /^#{1,3} / { inside = 0 }
      inside && /^\|[[:space:]]*SC-/ {
        line = tolower($0)
        p = index(line, "human-only")
        if (p == 0) next
        rest = substr(line, p + 10)
        gsub(/[^a-z0-9]/, "", rest)
        if (length(rest) < 20) {
          match($0, /SC-[0-9]+[a-z]?/)
          print substr($0, RSTART, RLENGTH)
        }
      }
    ' "$spec")"
  if [ -n "$unexplained" ]; then
    [ "$bad" = 1 ] || printf 'FAIL  %s\n' "$spec"
    bad=1
    report "marked human-only with no reason after the marker:"
    printf '%s\n' "$unexplained" | sed 's/^/        /'
    report "'Needs a GUI' is not a reason -- the visual-pass skill drives the real binary headlessly."
    report "A reason names something no instrument here can read."
  fi

  if [ "$bad" = 1 ]; then
    fail=1
  else
    printf 'ok    %-58s %s criteria, all disposed\n' "$spec" "$(printf '%s\n' "$defined" | wc -l | tr -d ' ')"
  fi
}

specs=("$@")
if [ "${#specs[@]}" -eq 0 ]; then
  mapfile -t specs < <(ls -1 specs/*/spec.md 2>/dev/null)
fi
if [ "${#specs[@]}" -eq 0 ]; then
  echo "check-criteria-observables: no specs to check (run me from the repository root)" >&2
  exit 2
fi
for spec in "${specs[@]}"; do
  if [ ! -f "$spec" ]; then
    echo "check-criteria-observables: no such file: $spec" >&2
    exit 2
  fi
done

for spec in "${specs[@]}"; do
  check_one "$spec"
done

echo
if [ "$fail" -ne 0 ]; then
  echo "check-criteria-observables: incomplete verification table ($checked spec(s) opted in, $skipped without one)"
  exit 1
fi
echo "check-criteria-observables: $checked spec(s) opted in and complete; $skipped without a table"
