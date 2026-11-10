#!/usr/bin/env bash
# Sourced by the autopilot hooks and scripts: which ledger covers a directory, and what a ledger says.
#
#   ledgers_of <dir>      every ledger that names <dir>'s branch as its Worktree branch, one per line,
#                         relative to the worktree's toplevel
#   ledger_of <dir>       the first of them, absolute, or nothing
#   run_ledger <dir>      that, or else the ledger of the session's own directory
#                         ($CLAUDE_PROJECT_DIR): a unit that prepares a later milestone works in
#                         another worktree, on a branch no ledger names, and is still part of the run.
#   ledger_prs <ledger>   the PR numbers of its "Pull requests" and "Milestones" tables
#   phase_of <ledger>     its Phase
#   pr_state_of <n>       GitHub's state of PR <n>, or UNKNOWN when gh fails
#   final_pr_open <br>    the number of <br>'s open PR, or nothing (run in the worktree)
#   ledger_on_main <br>   `<path> (phase <p>)` of an unfinished ledger of <br> on origin/main, else
#                         exit 1 (run in the worktree, after `git fetch`)
ledgers_of() {
  local top b
  top=$(git -C "$1" rev-parse --show-toplevel 2>/dev/null) || return 0
  b=$(git -C "$top" branch --show-current 2>/dev/null)
  [ -n "$b" ] || return 0
  (cd "$top" && grep -lxF -- "- **Worktree branch**: $b" specs/*/autopilot.md specs/*/autopilot-*.md specs/*/bugs/*.autopilot.md \
    specs/quick/*.autopilot.md 2>/dev/null)
  return 0
}
ledger_of() {
  local top l
  top=$(git -C "$1" rev-parse --show-toplevel 2>/dev/null) || return 0
  l=$(ledgers_of "$1" | head -1)
  [ -n "$l" ] && echo "$top/$l"
  return 0
}
run_ledger() {
  local l; l=$(ledger_of "$1")
  [ -z "$l" ] && [ -n "${CLAUDE_PROJECT_DIR:-}" ] && l=$(ledger_of "$CLAUDE_PROJECT_DIR")
  echo "$l"
}
ledger_prs() {
  sed -n '/^## Pull requests/,/^## Decisions/p' "$1" | grep -oE '#[0-9]+' | tr -d '#' | sort -un
}
phase_of() { sed -n 's/^- \*\*Phase\*\*: //p' "$1" | head -1; }
pr_state_of() { gh pr view "$1" --json state -q .state 2>/dev/null || echo UNKNOWN; }
final_pr_open() { gh pr list --head "$1" --state open --json number -q '.[0].number // empty' 2>/dev/null; }
ledger_on_main() {
  local line="- **Worktree branch**: $1" f ph
  # git grep has no -x: take substring hits, then keep only files with the exact line.
  git grep -lF -- "$line" origin/main -- 'specs/' 2>/dev/null | sed 's/^origin\/main://' \
    | while read -r f; do
        git show "origin/main:$f" | grep -qxF -- "$line" || continue
        ph="$(git show "origin/main:$f" | sed -n 's/^- \*\*Phase\*\*: //p' | head -1)"
        if [ "$ph" != done ]; then echo "$f (phase $ph)"; fi
      done | sed -n 1p | grep .
}
