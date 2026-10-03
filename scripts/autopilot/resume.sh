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
ledgers() { grep -lxF -- "$line" specs/*/autopilot.md specs/*/bugs/*.autopilot.md specs/quick/*.autopilot.md 2>/dev/null; }
# PR numbers from the ledger's "Pull requests" and "Milestones" tables only.
ledger_prs() {
  sed -n '/^## Pull requests/,/^## Decisions/p' "$1" | grep -oE '#[0-9]+' | tr -d '#' | sort -un
}
phase_of() { sed -n 's/^- \*\*Phase\*\*: //p' "$1" | head -1; }

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
    echo "PR #$pr $(gh pr view "$pr" --json state -q .state 2>/dev/null || echo UNKNOWN)"
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
    pr="$(gh pr list --head "$b" --state open --json number -q '.[0].number // empty' 2>/dev/null)"
    echo "FINAL-PR-PENDING $done_ledger ${pr:-none}"
    exit 4
  fi
fi

# git grep has no -x: take substring hits, then keep only files with the exact line.
found="$(git grep -lF -- "$line" origin/main -- 'specs/' 2>/dev/null | sed 's/^origin\/main://' \
  | while read -r f; do
      git show "origin/main:$f" | grep -qxF -- "$line" || continue
      ph="$(git show "origin/main:$f" | sed -n 's/^- \*\*Phase\*\*: //p' | head -1)"
      [ "$ph" != done ] && echo "$f (phase $ph)"
    done | head -1)"
if [ -n "$found" ]; then
  echo "LEDGER-ON-MAIN $found: run scripts/autopilot/branch-start.sh, then resume.sh again"
  exit 0
fi
echo "NONE: no autopilot run to resume in this worktree"
exit 2
