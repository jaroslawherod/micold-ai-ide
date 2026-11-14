#!/usr/bin/env bash
# Verifies one run and prints the next action for the supervisor, which makes the Micold call: the
# script has no session transport. Follow-ups are counted per run in the supervisor's ledger, capped
# at 3; at the cap one category 5 escalation is recorded and nothing more is sent.
#
# Usage: follow-up.sh <worktree-dir> --supervisor-ledger <file> --session live|stopped
#                     [--session-id <id>] [--check <id>]
# Line 1 starts with the action: DONE | SEND <id> | RESUME <id> | SKIP | ESCALATE | HOLD.
# Exit: 0 every action · 2 usage · 3 several ledgers. Contract: specs/1816-supervisor-verify-report/contracts/follow-up.md
# Env: AUTOPILOT_VERIFY_RUN replaces verify-run.sh.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
. "$here/supervisor-ledger.sh"
CAP=3
dir="" sled="" session="" sid="" check=""
usage() { echo "follow-up.sh: $1" >&2; exit 2; }
while [ $# -gt 0 ]; do
  case "$1" in
    -h|--help) sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    --supervisor-ledger|--session|--session-id|--check)
      [ $# -ge 2 ] || usage "$1 needs a value"
      case "$1" in
        --supervisor-ledger) sled="$2" ;; --session) session="$2" ;;
        --session-id) sid="$2" ;; --check) check="$2" ;;
      esac
      shift 2 ;;
    -*) usage "unknown option $1" ;;
    *) [ -z "$dir" ] || usage "one worktree only"; dir="$1"; shift ;;
  esac
done
[ -n "$dir" ] || usage "a worktree directory is required"
[ -n "$sled" ] || usage "--supervisor-ledger is required"
case "$session" in live|stopped) ;; *) usage "--session must be live or stopped" ;; esac
git -C "$dir" rev-parse --show-toplevel >/dev/null 2>&1 || usage "$dir is not a git worktree"
branch="$(git -C "$dir" branch --show-current 2>/dev/null)"
[ -n "$branch" ] || usage "$dir is on no branch"

out="$("${AUTOPILOT_VERIFY_RUN:-$here/verify-run.sh}" "$dir")"; rc=$?
case "$rc" in
  0) echo DONE; exit 0 ;;
  1) ;;
  *) exit "$rc" ;;
esac
remaining="$(sed 1d <<<"$out")"
hash="$(printf '%s\n' "$remaining" | cksum | cut -d' ' -f1)"

read -r count rowhash rowcheck <<<"$(sl_followup_row "$sled" "$branch")"
if [ "$count" -ge "$CAP" ]; then
  if sl_escalation_exists "$sled" "$branch"; then echo HOLD; exit 0; fi
  banner="- **Category**: 5 Non-convergence
- **Question**: Run $branch is still NOT DONE after $CAP follow-ups; how should it proceed?
- **Options**:
  1. Take over the run by hand (Recommended) — the supervisor stops sending follow-ups
  2. Reset the follow-up count and let the supervisor try $CAP more
- **Recommended**: Take over the run by hand — $CAP follow-ups did not converge it
- **Evidence**: follow-ups sent: $count; remaining:
$(sed 's/^/  /' <<<"$remaining")"
  sl_escalation_write "$sled" "$branch" <<<"$banner"
  echo ESCALATE
  printf '%s\n' "$banner"
  exit 0
fi
if [ -n "$check" ] && [ "$rowhash" = "$hash" ] && [ "$rowcheck" = "$check" ]; then echo SKIP; exit 0; fi

sl_followup_set "$sled" "$branch" $((count + 1)) "$hash" "${check:--}"
if [ "$session" = live ]; then
  echo "SEND${sid:+ $sid}"
  echo "Run not done. Remaining:"
else
  echo "RESUME${sid:+ $sid}"
fi
printf '%s\n' "$remaining"
exit 0
