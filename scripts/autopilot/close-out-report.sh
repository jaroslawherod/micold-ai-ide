#!/usr/bin/env bash
# One markdown report over the supervisor's finished runs: completed, not done, escalated, and the
# worktrees safe to remove. A run is Completed only when verify-run.sh says VERIFIED.
#
# Usage: close-out-report.sh <worktree-dir>... [--supervisor-ledger <file>] [--parent <ref> --rows <file>]
#                            [--live <path>]... [--no-live-sessions]
# --parent <ref> --rows <file>: keep only the runs runs-of-parent.sh calls own (FR-016); another
# parent, none, or a removed parent leaves a run out, and each entry names its parent.
# Exit: 0 (also for zero runs and errored runs) · 2 usage or unreadable rows.
# Contract: specs/1816-supervisor-verify-report/contracts/close-out-report.md
# Env: AUTOPILOT_VERIFY_RUN replaces verify-run.sh, AUTOPILOT_RUN_STATUS replaces run-status.sh.
set -uo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
. "$here/ledger-of.sh"
. "$here/supervisor-ledger.sh"
dirs=() sled="" parent="" rows=""
usage() { echo "close-out-report.sh: $1" >&2; exit 2; }
while [ $# -gt 0 ]; do
  case "$1" in
    -h|--help) sed -n '2,10p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    --no-live-sessions) shift ;;
    --supervisor-ledger|--parent|--rows|--live)
      [ $# -ge 2 ] || usage "$1 needs a value"
      case "$1" in
        --supervisor-ledger) sled="$2" ;; --parent) parent="$2" ;; --rows) rows="$2" ;; --live) ;; # --live belongs to Safe to remove (M4)
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

# count_decisions <ledger>: "<by user> <by other>" of the Decisions table.
count_decisions() {
  [ -f "$1" ] || { echo "0 0"; return; }
  awk -F'|' '
    /^## / { on = ($0 == "## Decisions"); next }
    on && /^\|/ {
      c = $2; gsub(/^ +| +$/, "", c); if (c == "#" || c ~ /^-+$/) next
      b = $6; gsub(/^ +| +$/, "", b)
      if (b == "user") u++; else a++
    }
    END { print u + 0, a + 0 }' "$1"
}

completed="" notdone="" escalated=""
for dir in ${dirs[@]+"${dirs[@]}"}; do
  rparent=""
  if [ -n "$parent" ]; then
    abs="$(cd "$dir" 2>/dev/null && pwd)" || abs="$dir"
    "$here/runs-of-parent.sh" "$parent" --check "$abs" --rows "$rows" >/dev/null 2>&1 || continue
    rparent="$parent"
  fi
  branch="$(git -C "$dir" branch --show-current 2>/dev/null)"; branch="${branch:-$(basename "$dir")}"
  json="$("${AUTOPILOT_RUN_STATUS:-$here/run-status.sh}" --json "$dir" 2>/dev/null)" || json=""
  issue="$(jq -r '.issue // empty' <<<"$json" 2>/dev/null)"; issue="${issue#\#}"
  case "$issue" in ''|*[!0-9]*) title="${issue:-unknown} $branch" ;; *) title="#$issue $branch" ;; esac
  if [ -n "$json" ] && jq -e '.prs | length > 0' >/dev/null 2>&1 <<<"$json"; then
    prs="$(jq -r '[.prs[] | "#\(.number) \(.state)"] | join(", ")' <<<"$json")"
  elif [ -n "$json" ]; then prs=none; else prs=unknown; fi

  vout="$("${AUTOPILOT_VERIFY_RUN:-$here/verify-run.sh}" "$dir" 2>/dev/null)"; rc=$?
  head="### $title"$'\n'"${rparent:+- Parent: $rparent$'\n'}- PRs: $prs"
  if [ "$rc" -eq 0 ]; then
    ledger="$(ledger_of "$dir")"
    read -r nu na <<<"$(count_decisions "$ledger")"
    tokens="$([ -f "$ledger" ] && sed -n 's/^- \*\*Tokens\*\*: *\([0-9][0-9]*\) *$/\1/p' "$ledger" | head -1)"
    iv="$(sl_interventions "$sled" "$branch" | awk -F'|' '{ for (i = 3; i <= 4; i++) gsub(/^ +| +$/, "", $i); print "  - " $3 ": " $4 }')"
    completed="$completed$head"$'\n'"- Decisions: $nu by user, $na by agent"$'\n'
    if [ -n "$iv" ]; then completed="$completed- Interventions:"$'\n'"$iv"$'\n'; else completed="$completed- Interventions: none"$'\n'; fi
    completed="$completed- Tokens: ${tokens:-unknown}"$'\n'
  elif [ -f "$sled" ] && sl_escalation_exists "$sled" "$branch"; then
    q="$(sl_escalations "$sled" "$branch" | sed -n 's/^- \*\*Question\*\*: *//p' | head -1)"
    escalated="$escalated$head"$'\n'"- ${q:-escalated, no question recorded}"$'\n'
  else
    case "$rc" in
      1) rem="$(sed 1d <<<"$vout" | sed -n 's/^- //p')" ;;
      3) rem="several ledgers name the branch" ;;
      *) rem="verify-run.sh could not verify the run (exit $rc)" ;;
    esac
    notdone="$notdone$head"$'\n'"$(while IFS= read -r l; do printf -- '- Remaining: %s\n' "$l"; done <<<"$rem")"$'\n'
  fi
done

printf '## Completed\n%s## Not done\n%s## Escalated\n%s## Safe to remove\n- none\n' "$completed" "$notdone" "$escalated"
exit 0
