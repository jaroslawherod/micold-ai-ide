#!/usr/bin/env bash
# Sourced by the autopilot hooks: which ledger covers a directory.
#
#   ledger_of <dir>    the ledger that names <dir>'s branch as its Worktree branch, or nothing
#   run_ledger <dir>   that, or else the ledger of the session's own directory
#                      ($CLAUDE_PROJECT_DIR): a unit that prepares a later milestone works in
#                      another worktree, on a branch no ledger names, and is still part of the run.
ledger_of() {
  local top b
  top=$(git -C "$1" rev-parse --show-toplevel 2>/dev/null) || return 0
  b=$(git -C "$top" branch --show-current 2>/dev/null)
  [ -n "$b" ] || return 0
  (cd "$top" && grep -lxF -- "- **Worktree branch**: $b" specs/*/autopilot.md specs/*/bugs/*.autopilot.md \
    specs/quick/*.autopilot.md 2>/dev/null | head -1 | sed "s|^|$top/|")
}
run_ledger() {
  local l; l=$(ledger_of "$1")
  [ -z "$l" ] && [ -n "${CLAUDE_PROJECT_DIR:-}" ] && l=$(ledger_of "$CLAUDE_PROJECT_DIR")
  echo "$l"
}
