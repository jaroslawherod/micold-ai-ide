#!/usr/bin/env bash
# Lists which finished runs' worktrees are safe to remove. It removes nothing: removal stays the
# user's click, or the supervisor's call to the app's `delete_worktree` when the supervisor ledger
# has `- **Auto-remove**: yes`, in which case each SAFE line is followed by a `REMOVE <path>` request.
#
# Usage: safe-to-remove.sh <worktree-dir>... [--live <path>]... [--no-live-sessions]
#                          [--supervisor-ledger <file>] [--parent <ref> --rows <file>]
# One line per worktree: `SAFE <path>` or `KEEP <path> <reason>`. A worktree is SAFE when verify-run.sh
# says VERIFIED (merged, clean, nothing unpushed, issue closed) and no session is live in it.
# --live <path>: a worktree a live session works in (repeatable). --no-live-sessions: the caller
# checked and none is live. Neither: every worktree is KEEP `no session list given`.
# --parent <ref> --rows <file>: list only the runs runs-of-parent.sh calls own (FR-016); others are skipped.
# Exit: 0 · 2 usage or unreadable rows. Contract: specs/1816-supervisor-verify-report/contracts/close-out-report.md
# Env: AUTOPILOT_VERIFY_RUN replaces verify-run.sh.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
. "$here/supervisor-ledger.sh"
dirs=() live=() sled="" parent="" rows="" nolive=no
usage() { echo "safe-to-remove.sh: $1" >&2; exit 2; }
while [ $# -gt 0 ]; do
  case "$1" in
    -h|--help) sed -n '2,13p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    --no-live-sessions) nolive=yes; shift ;;
    --supervisor-ledger|--parent|--rows|--live)
      [ $# -ge 2 ] || usage "$1 needs a value"
      case "$1" in
        --supervisor-ledger) sled="$2" ;; --parent) parent="$2" ;; --rows) rows="$2" ;; --live) live+=("$2") ;;
      esac
      shift 2 ;;
    -*) usage "unknown option $1" ;;
    *) dirs+=("$1"); shift ;;
  esac
done
{ [ -z "$parent" ] && [ -z "$rows" ]; } || { [ -n "$parent" ] && [ -n "$rows" ]; } || usage "--parent and --rows go together"
if [ -n "$parent" ]; then
  "$here/runs-of-parent.sh" "$parent" --rows "$rows" >/dev/null || usage "unreadable rows $rows"
fi

abs() { (cd "$1" 2>/dev/null && pwd -P) || printf '%s' "$1"; }
is_live() {
  local a p; a="$(abs "$1")"
  for p in ${live[@]+"${live[@]}"}; do [ "$(abs "$p")" = "$a" ] && return 0; done
  return 1
}
auto="$(sl_auto_remove "$sled")"

for dir in ${dirs[@]+"${dirs[@]}"}; do
  if [ -n "$parent" ]; then
    "$here/runs-of-parent.sh" "$parent" --check "$(abs "$dir")" --rows "$rows" >/dev/null 2>&1 || continue
  fi
  if ! git -C "$dir" rev-parse --show-toplevel >/dev/null 2>&1; then echo "KEEP $dir not a git worktree"; continue; fi
  vout="$("${AUTOPILOT_VERIFY_RUN:-$here/verify-run.sh}" "$dir" 2>/dev/null)"; rc=$?
  case "$rc" in
    0) ;;
    1) echo "KEEP $dir not verified: $(sed -n 's/^- //p' <<<"$vout" | head -1)"; continue ;;
    2) echo "KEEP $dir not a git worktree"; continue ;;
    3) echo "KEEP $dir several ledgers"; continue ;;
    *) echo "KEEP $dir not verified: verify-run.sh failed (exit $rc)"; continue ;;
  esac
  if is_live "$dir"; then echo "KEEP $dir live session"; continue; fi
  if [ "$nolive" = no ] && [ "${#live[@]}" -eq 0 ]; then echo "KEEP $dir no session list given"; continue; fi
  echo "SAFE $dir"
  [ "$auto" = yes ] && echo "REMOVE $dir"
done
exit 0
