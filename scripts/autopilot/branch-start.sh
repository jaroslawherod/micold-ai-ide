#!/usr/bin/env bash
# Put this worktree's branch on the right base before an autopilot unit starts work.
#
# Work already merged (all local commits have a patch-equal twin on origin/main): reset the branch to
# origin/main. Unmerged work from an earlier unit (clarify rounds, a BUG record, close-phase
# milestones): rebase it onto origin/main, which drops commits already merged. Never drops unmerged
# commits.
#
# Prints one line: RESET, REBASED <n>, DIRTY or CONFLICT. Exit 0 on RESET/REBASED, 1 otherwise.
set -uo pipefail

if [ -n "$(git status --porcelain)" ]; then
  echo "DIRTY: commit or discard these first:"
  git status --porcelain | head -20
  exit 1
fi
git fetch -q origin || { echo "FETCH-FAILED"; exit 1; }
branch="$(git branch --show-current)"
unmerged="$(git cherry origin/main HEAD | grep -c '^+' || true)"
if [ "$unmerged" -eq 0 ]; then
  git switch -q -C "$branch" origin/main
  echo "RESET $branch to origin/main"
elif git rebase -q origin/main >/dev/null 2>&1; then
  echo "REBASED $unmerged unmerged commit(s) of $branch onto origin/main"
else
  echo "CONFLICT: rebase stopped. Resolve, run the gate, then git rebase --continue:"
  git diff --name-only --diff-filter=U
  exit 1
fi
