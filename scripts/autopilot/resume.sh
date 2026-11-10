#!/usr/bin/env bash
# Find this worktree's autopilot ledger and the state of every PR it records.
#
# Prints the ledger, its Phase and Next step, then one line per recorded PR with GitHub's state.
# GitHub wins over the ledger: a PR the ledger calls merged that GitHub reports OPEN is a step to redo.
#
# Exit: 0 one unfinished ledger · 2 none · 3 several (each printed; ask the user) ·
#       4 a ledger that reads done while the run's last PR (which sets done) never merged: prints
#         its open PR or `none` (merge the PR, or dispatch a unit to open it, then hand off).
set -uo pipefail

b="$(git branch --show-current)"
line="- **Worktree branch**: $b"
. "$(dirname "$0")/ledger-of.sh"
ledgers() { ledgers_of .; }

report() {
  local f="$1"
  echo "LEDGER $f"
  echo "PHASE $(phase_of "$f")"
  echo "NEXT $(sed -n 's/^- \*\*Next step\*\*: //p' "$f" | head -1)"
  sed -n '/^## Open escalation/,/^## /p' "$f" | grep -v '^## ' | grep -vE '^\s*$|^None\.' \
    | sed 's/^/ESCALATION /' | head -5
  sed -n '/^## Handover/,/^## /p' "$f" | grep -v '^## ' | grep -vE '^\s*$|^None\.' \
    | sed 's/^/HANDOVER /' | head -5
  for pr in $(ledger_prs "$f"); do
    echo "PR #$pr $(pr_state_of "$pr")"
  done
}

open=()  # bash 3.2 (macOS) has no mapfile
while read -r f; do [ -n "$f" ] && [ "$(phase_of "$f")" != done ] && open+=("$f"); done < <(ledgers)
case "${#open[@]}" in
  1) report "${open[0]}"; exit 0 ;;
  0) ;;
  *) for f in "${open[@]}"; do report "$f"; echo; done; exit 3 ;;
esac

git fetch -q origin
if git cherry origin/main HEAD 2>/dev/null | grep -q '^+'; then
  done_ledger="$(ledgers | grep -v '^specs/quick/' | head -1)"  # a quick run commits no ledger
  if [ -n "$done_ledger" ]; then
    pr="$(final_pr_open "$b")"
    echo "FINAL-PR-PENDING $done_ledger ${pr:-none}"
    exit 4
  fi
fi

found="$(ledger_on_main "$b")"
if [ -n "$found" ]; then
  echo "LEDGER-ON-MAIN $found: run scripts/autopilot/branch-start.sh, then resume.sh again"
  exit 0
fi
echo "NONE: no autopilot run to resume in this worktree"
exit 2
