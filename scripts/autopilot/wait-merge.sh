#!/usr/bin/env bash
# Wait for a PR's `ci complete` check on its current head, then rebase-merge it on green.
#
# Usage: wait-merge.sh <pr> [--no-merge]
# Runs for as long as CI does, so start it detached and wait on its last line (see SKILL.md).
#
# Last line, and exit code:
#   MERGED <pr> <merge-sha>          0  merged (or already merged)
#   GREEN <pr>                       0  green, --no-merge given
#   RED <pr> <run-id> <log-file>     1  failed checks listed above it; failing log tail in <log-file>
#   CHECKLESS <pr> <reason>          3  no check at all after AUTOPILOT_CHECKLESS_AFTER seconds;
#                                       reason: conflicting | action_required <run-ids> | no-run
#   MERGE-FAILED <pr> <message>      4  green, but `gh pr merge` refused
#   CLOSED <pr>                      5  the PR was closed while waiting
#   TIMEOUT <pr> <what>              6  no result within AUTOPILOT_MAX_WAIT seconds (gh failing, CI stuck)
#
# Never approves runs, closes, reopens or pushes: those stay with the caller.
set -uo pipefail

pr="${1:?usage: wait-merge.sh <pr> [--no-merge]}"
merge=1; [ "${2:-}" = --no-merge ] && merge=0
poll="${AUTOPILOT_POLL:-20}"
checkless_after="${AUTOPILOT_CHECKLESS_AFTER:-300}"
max_wait="${AUTOPILOT_MAX_WAIT:-7200}"
logdir="${AUTOPILOT_LOG_DIR:-${TMPDIR:-/tmp}}"
waited=0

view() { gh pr view "$pr" --json "$1" -q "$2"; }
merged_line() { echo "MERGED $pr $(view mergeCommit '.mergeCommit.oid' 2>/dev/null)"; }
# The current `ci complete` run on the head: a relabel re-run leaves a superseded one. A run not yet
# completed wins (a queued run may carry a zero startedAt); otherwise the latest started.
ci_state() {
  view statusCheckRollup \
    '[.statusCheckRollup[] | select(.name=="ci complete")] | sort_by([(.status != "COMPLETED"), (.startedAt // "")]) | last
     | select(. != null) | "\(.status) \(.conclusion // "")"' 2>/dev/null
}
# Exits when the PR is no longer open; otherwise sleeps one poll, bounded by max_wait.
tick() {
  case "$(view state .state 2>/dev/null)" in
    MERGED) merged_line; exit 0 ;;
    CLOSED) echo "CLOSED $pr"; exit 5 ;;
  esac
  if [ "$waited" -ge "$max_wait" ]; then echo "TIMEOUT $pr $1"; exit 6; fi
  sleep "$poll"; waited=$((waited + poll))
}

any_checks() { [ "$(view statusCheckRollup '.statusCheckRollup | length' 2>/dev/null)" -gt 0 ] 2>/dev/null; }

tick "before checks"
# `ci complete` needs every other job, so it appears only when they finish: while other checks run,
# keep waiting. Only a PR with no check at all is checkless.
while [ -z "$(ci_state)" ]; do
  if [ "$waited" -ge "$checkless_after" ] && ! any_checks; then
    if [ "$(view mergeable .mergeable)" = CONFLICTING ]; then
      echo "CHECKLESS $pr conflicting"; exit 3
    fi
    branch="$(view headRefName .headRefName)"; head="$(view headRefOid .headRefOid)"
    if [ -z "$branch" ] || [ -z "$head" ]; then echo "TIMEOUT $pr gh pr view failing"; exit 6; fi
    pending="$(gh run list --branch "$branch" --limit 10 --json databaseId,conclusion,headSha \
      -q ".[] | select(.conclusion==\"action_required\" and .headSha==\"$head\") | .databaseId" | tr '\n' ' ')"
    if [ -n "$pending" ]; then echo "CHECKLESS $pr action_required ${pending% }"; exit 3; fi
    echo "CHECKLESS $pr no-run"; exit 3
  fi
  tick "waiting for ci complete to appear"
done

# A failed `ci complete` is not final while another run on the same head is still going: a second
# event (a recreated branch, a relabel) cancels the first run, and the newer run's `ci complete`
# appears only when its other jobs finish.
active_run() {
  local branch head
  branch="$(view headRefName .headRefName 2>/dev/null)"; head="$(view headRefOid .headRefOid 2>/dev/null)"
  [ -n "$branch" ] && [ -n "$head" ] || return 0
  gh run list --branch "$branch" --limit 10 --json databaseId,status,headSha \
    -q ".[] | select(.headSha==\"$head\" and (.status==\"in_progress\" or .status==\"queued\")) | .databaseId" 2>/dev/null
}
while :; do
  while :; do
    read -r status conclusion <<<"$(ci_state)"
    [ "${status:-}" = COMPLETED ] && break
    tick "waiting for ci complete to finish"
  done
  [ "$conclusion" = SUCCESS ] && break
  [ -n "$(active_run)" ] || break
  tick "waiting for a newer run on the same head"
done

if [ "$conclusion" != SUCCESS ]; then
  bad='select(.name != null and .name != "ci complete" and (.conclusion // "") != ""
              and ([.conclusion] | inside(["SUCCESS","SKIPPED","NEUTRAL"]) | not))'
  echo "Failed checks:"
  view statusCheckRollup ".statusCheckRollup[] | $bad | \"- \(.name) \(.conclusion) \(.detailsUrl)\""
  view statusCheckRollup '.statusCheckRollup[] | select(.context != null and (.state=="FAILURE" or .state=="ERROR"))
    | "- \(.context) \(.state) \(.targetUrl // "")"'
  run="$( { view statusCheckRollup ".statusCheckRollup[] | $bad | .detailsUrl"
            view statusCheckRollup '.statusCheckRollup[] | select(.name=="ci complete") | .detailsUrl'; } \
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
