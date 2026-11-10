#!/usr/bin/env bash
# Compare a milestone's diff to its committed plan (the file paths its tasks in tasks.md name).
#
#   plan-drift.sh <feature-dir> M<K> [<base> [<end>]]   base origin/main, end HEAD
#
# Prints `UNPLANNED <file>` for a changed file no task of the milestone names, and `MISSING <path>`
# for a planned path the diff does not touch. Paths under specs/ and the ledger are not compared.
# A drift is accepted once <feature-dir>/drift-M<K>.md mentions the path with a reason (or the
# task's line in tasks.md is updated to name it). Exit 1 while any drift is unrecorded, else 0.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
feat="${1:?usage: plan-drift.sh <feature-dir> M<K> [base]}" mid="${2:?usage: plan-drift.sh <feature-dir> M<K> [base]}"
base="${3:-origin/main}" end="${4:-HEAD}"
here="$(cd "$(dirname "$0")" && pwd)"

brief="$("$here/brief.py" milestone "$feat" "$mid")" || { echo "plan-drift: no plan for $mid" >&2; exit 2; }
plan="$(sed -n "/^## Tasks of $mid /,/^## /p" <<<"$brief" \
  | grep -oE '[A-Za-z0-9_.-]+(/[A-Za-z0-9_.-]+)*\.(rs|toml|md|sh|py|yml|yaml|json|css|svg|png|lock|txt|html|ron|ttf)' \
  | grep -vE '^specs/|^(tasks|plan|spec)\.md$' | sort -u || true)"
[ -n "$plan" ] || { echo "plan-drift: $mid's tasks name no file" >&2; exit 2; }
changed="$(git diff --name-only "$base...$end" | grep -vE '^specs/|\.autopilot\.md$' | sort -u || true)"
notes="$feat/drift-$mid.md"
[ -f "$notes" ] || notes=/dev/null

planned_for() { # does planned path $2 cover changed file $1?
  [ "$1" = "$2" ] || [[ "$1" == */"$2" ]]
}
recorded() { grep -qE -- "(^|[^A-Za-z0-9_./-])$(sed 's/[].[*^$]/\\&/g' <<<"$1")([^A-Za-z0-9_./-]|$)" "$notes"; }

drift=0
while IFS= read -r f; do
  [ -n "$f" ] || continue
  hit=0
  while IFS= read -r p; do [ -n "$p" ] && planned_for "$f" "$p" && hit=1 && break; done <<<"$plan"
  if [ "$hit" = 0 ] && ! recorded "$f"; then echo "UNPLANNED $f"; drift=1; fi
done <<<"$changed"
while IFS= read -r p; do
  [ -n "$p" ] || continue
  hit=0
  while IFS= read -r f; do [ -n "$f" ] && planned_for "$f" "$p" && hit=1 && break; done <<<"$changed"
  if [ "$hit" = 0 ] && ! recorded "$p"; then echo "MISSING $p"; drift=1; fi
done <<<"$plan"
[ "$drift" = 0 ] && echo "no plan drift for $mid"
exit "$drift"
