#!/usr/bin/env bash
# Wait for a PR's `ci complete` check on its current head, then rebase-merge it on green.
#
# Usage: wait-merge.sh <pr> [--no-merge|--squash]
#   --squash   squash-merge instead of rebase-merge
#   --no-merge stop at green and merge nothing (the verdict guard is skipped)
# Before any merge it runs verdict-guard.sh: a head whose approved review covers another head is refused.
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
#   VERDICT-REFUSED <pr> ...         7  green, but the review verdict does not cover the head (see verdict-guard.sh)
#   TIMEOUT <pr> <what>              6  no result within AUTOPILOT_MAX_WAIT seconds (default 6 h; gh failing, CI stuck).
#                                       Each wake after a timeout re-writes the orchestrator's cache (about 170k cost_eq at 100k of
#                                       context): a 2 h cap woke it four times in one run while CI sat queued for 2 h 44 min.
#
# Never approves runs, closes, reopens or pushes: those stay with the caller.
set -uo pipefail

pr="${1:?usage: wait-merge.sh <pr> [--no-merge|--squash]}"
merge=1; method=--rebase
case "${2:-}" in --no-merge) merge=0 ;; --squash) method=--squash ;; esac
poll="${AUTOPILOT_POLL:-20}"
checkless_after="${AUTOPILOT_CHECKLESS_AFTER:-300}"
max_wait="${AUTOPILOT_MAX_WAIT:-21600}"
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
# The guard reads the head fresh; the merge is pinned to that head, so a push in between is refused.
guard="$("$(dirname "${BASH_SOURCE[0]}")/verdict-guard.sh" "$pr" 2>&1)"; gcode=$?
glast="$(tail -n 1 <<<"$guard")"
if [ "$gcode" -ne 0 ]; then
  case "$glast" in VERDICT-REFUSED*) echo "$glast" ;; *) echo "VERDICT-REFUSED $pr reviewed none current unknown re-review: guard failed: $glast" ;; esac
  exit 7
fi
case "$glast" in
  ALLOWED*) head="${glast##* }" ;;
  UNGUARDED*|CARRIED*) head="$(view headRefOid .headRefOid)" ;;
esac
if [ -z "${head:-}" ]; then
  echo "VERDICT-REFUSED $pr reviewed none current unknown re-review: guard gave no head ($glast)"; exit 7
fi
if ! err="$(gh pr merge "$pr" "$method" --match-head-commit "$head" 2>&1)"; then
  [ "$(view state .state)" = MERGED ] && { merged_line; exit 0; }
  echo "MERGE-FAILED $pr $(echo "$err" | grep -v '^\s*$' | head -1)"
  exit 4
fi
[ "$(view state .state)" = MERGED ] && { merged_line; exit 0; }
echo "MERGE-FAILED $pr state is $(view state .state) after merge"
exit 4
