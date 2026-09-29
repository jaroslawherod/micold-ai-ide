#!/usr/bin/env bash
# The three checks before the WORK COMPLETE handoff.
#
# Usage: handoff-check.sh <ledger> [extra-pr...]
# Prints OK, or NOT DONE and one line per thing that remains. Exit 0 or 1.
set -uo pipefail
ledger="${1:?usage: handoff-check.sh <ledger> [extra-pr...]}"; shift
left=()

dirty="$(git status --porcelain)"
[ -n "$dirty" ] && left+=("uncommitted work: $(echo "$dirty" | wc -l) path(s)")

git fetch -q origin
unmerged="$(git cherry origin/main HEAD | grep -c '^+' || true)"
[ "$unmerged" -gt 0 ] && left+=("$unmerged commit(s) not on origin/main")

prs="$(sed -n '/^## Pull requests/,/^## Decisions/p' "$ledger" | grep -oE '#[0-9]+' | tr -d '#' | sort -un)"
for pr in $prs "$@"; do
  st="$(gh pr view "$pr" --json state -q .state 2>/dev/null || echo UNKNOWN)"
  [ "$st" = MERGED ] || left+=("PR #$pr is $st")
done

if [ "${#left[@]}" -eq 0 ]; then echo OK; exit 0; fi
echo "NOT DONE"
printf -- '- %s\n' "${left[@]}"
exit 1
