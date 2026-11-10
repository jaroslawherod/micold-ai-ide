#!/usr/bin/env bash
# The three checks before the WORK COMPLETE handoff.
#
# Usage: handoff-check.sh <ledger> [extra-pr...]
# Prints OK, or NOT DONE and one line per thing that remains. Exit 0 or 1.
set -uo pipefail
. "$(dirname "$0")/ledger-of.sh"
ledger="${1:?usage: handoff-check.sh <ledger> [extra-pr...]}"; shift
left=()
[ -f "$ledger" ] || { echo "NOT DONE"; echo "- ledger $ledger not found"; exit 1; }

dirty="$(git status --porcelain)"
[ -n "$dirty" ] && left+=("uncommitted work: $(echo "$dirty" | wc -l) path(s)")

if ! git fetch -q origin; then
  left+=("git fetch origin failed")
elif ! cherry="$(git cherry origin/main HEAD)"; then
  left+=("git cherry origin/main HEAD failed")
else
  unmerged="$(grep -c '^+' <<<"$cherry" || true)"
  [ "$unmerged" -gt 0 ] && left+=("$unmerged commit(s) not on origin/main")
fi

prs="$(ledger_prs "$ledger")"
for pr in $prs "$@"; do
  st="$(pr_state_of "$pr")"
  [ "$st" = MERGED ] || left+=("PR #$pr is $st")
done

if [ "${#left[@]}" -eq 0 ]; then echo OK; exit 0; fi
echo "NOT DONE"
printf -- '- %s\n' "${left[@]}"
exit 1
