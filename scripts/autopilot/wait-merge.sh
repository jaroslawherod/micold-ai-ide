#!/usr/bin/env bash
# Wait for a PR's `ci complete` check on its current head, then rebase-merge it on green.
#
# Usage: wait-merge.sh <pr> [--no-merge]
# Runs for as long as CI does, so start it detached and wait on its last line:
#   log="$SCRATCHPAD/pr-<n>.log"
#   setsid nohup scripts/autopilot/wait-merge.sh <n> >"$log" 2>&1 &
#   then a Monitor or background `until grep -qE '^(MERGED|GREEN|RED|CHECKLESS|MERGE-FAILED)' "$log"`
#
# Last line, and exit code:
#   MERGED <pr> <merge-sha>          0  merged (or already merged)
#   GREEN <pr>                       0  green, --no-merge given
#   RED <pr> <run-id> <log-file>     1  failed checks listed above it; failing log tail in <log-file>
#   CHECKLESS <pr> <reason>          3  no `ci complete` after AUTOPILOT_CHECKLESS_AFTER seconds;
#                                       reason: conflicting | action_required <run-ids> | no-run
#   MERGE-FAILED <pr> <message>      4  green, but `gh pr merge` refused
#
# Never approves runs, closes, reopens or pushes: those stay with the caller.
set -uo pipefail

pr="${1:?usage: wait-merge.sh <pr> [--no-merge]}"
merge=1; [ "${2:-}" = --no-merge ] && merge=0
poll="${AUTOPILOT_POLL:-20}"
checkless_after="${AUTOPILOT_CHECKLESS_AFTER:-300}"
logdir="${AUTOPILOT_LOG_DIR:-${TMPDIR:-/tmp}}"

view() { gh pr view "$pr" --json "$1" -q "$2"; }
merged_line() { echo "MERGED $pr $(view mergeCommit '.mergeCommit.oid' 2>/dev/null)"; }
ci_state() {  # "<status> <conclusion>" of `ci complete` on the current head, or nothing yet
  view statusCheckRollup '.statusCheckRollup[] | select(.name=="ci complete") | "\(.status) \(.conclusion)"' \
    2>/dev/null | head -1
}

[ "$(view state .state)" = MERGED ] && { merged_line; exit 0; }

waited=0
while [ -z "$(ci_state)" ]; do
  if [ "$waited" -ge "$checkless_after" ]; then
    if [ "$(view mergeable .mergeable)" = CONFLICTING ]; then
      echo "CHECKLESS $pr conflicting"; exit 3
    fi
    branch="$(view headRefName .headRefName)"
    pending="$(gh run list --branch "$branch" --limit 5 --json databaseId,conclusion \
      -q '.[] | select(.conclusion=="action_required") | .databaseId' | tr '\n' ' ')"
    if [ -n "$pending" ]; then echo "CHECKLESS $pr action_required $pending"; exit 3; fi
    echo "CHECKLESS $pr no-run"; exit 3
  fi
  sleep "$poll"; waited=$((waited + poll))
done

while :; do
  read -r status conclusion <<<"$(ci_state)"
  [ "$status" = COMPLETED ] && break
  sleep "$poll"
done

if [ "$conclusion" != SUCCESS ]; then
  echo "Failed checks:"
  view statusCheckRollup '.statusCheckRollup[] | select(.conclusion=="FAILURE") | "- \(.name) \(.detailsUrl)"'
  run="$(view statusCheckRollup '.statusCheckRollup[] | select(.conclusion=="FAILURE" and .name!="ci complete") | .detailsUrl' \
    | grep -oE 'runs/[0-9]+' | head -1 | cut -d/ -f2)"
  out="$logdir/pr-$pr-failed.log"
  if [ -n "$run" ]; then gh run view "$run" --log-failed 2>&1 | tail -200 >"$out"; else : >"$out"; fi
  echo "RED $pr ${run:-none} $out"
  exit 1
fi

[ "$merge" -eq 0 ] && { echo "GREEN $pr"; exit 0; }
if ! err="$(gh pr merge "$pr" --rebase 2>&1)"; then
  [ "$(view state .state)" = MERGED ] && { merged_line; exit 0; }
  echo "MERGE-FAILED $pr $(echo "$err" | grep -v '^\s*$' | head -1)"
  exit 4
fi
[ "$(view state .state)" = MERGED ] && { merged_line; exit 0; }
echo "MERGE-FAILED $pr state is $(view state .state) after merge"
exit 4
