#!/usr/bin/env bash
# Report which state a PR sits in, how long it has been there, and the action once it is stuck.
#
# Usage: pr-state.sh <pr>
# Prints one line:  PR <n> STATE <state> DWELL <s> LIMIT <s> <OK|STUCK> ACTION <action>
# Exit: 0 within its limit (or MERGED/CLOSED) · 1 stuck past its limit · 2 usage or gh failure.
#
# States, limits (seconds; override with AUTOPILOT_LIMIT_<STATE>, `-` as `_`) and actions are the table
# in rules/waiting.md. Dwell is measured from the later of the head commit's date and the latest
# finished check: an upper-bound proxy, as GitHub keeps no per-state timestamp.
# AUTOPILOT_NOW (epoch seconds) replaces the clock, for tests.
set -uo pipefail

pr="${1:?usage: pr-state.sh <pr>}"
now="${AUTOPILOT_NOW:-$(date +%s)}"

json="$(gh pr view "$pr" --json state,isDraft,mergeable,autoMergeRequest,statusCheckRollup,commits 2>/dev/null)" \
  || { echo "PR $pr STATE UNKNOWN gh failed" >&2; exit 2; }

parsed="$(jq -r '
  def ts: if . == null or . == "" or startswith("0001") then 0 else (sub("\\.[0-9]+Z$";"Z") | fromdateiso8601) end;
  (.statusCheckRollup // []) as $c
  | ([$c[] | select((.conclusion // "") | IN("FAILURE","TIMED_OUT","CANCELLED","STARTUP_FAILURE"))
          , select((.state // "") | IN("FAILURE","ERROR"))]) as $red
  | ([$c[] | select((.status // "COMPLETED") != "COMPLETED"), select((.state // "") == "PENDING")]) as $run
  | (([.commits[-1].committedDate // ""] + [$c[] | (.completedAt // "")]) | map(ts) | max) as $since
  | (if .state != "OPEN" then .state
     elif .isDraft then "DRAFT"
     elif .mergeable == "CONFLICTING" then "CONFLICTING"
     elif ($c | length) == 0 then "CHECKLESS"
     elif ($red | length) > 0 then "CI-RED"
     elif ($run | length) > 0 then "CI-RUNNING"
     elif .autoMergeRequest != null then "ARMED"
     else "GREEN" end) as $s
  | "\($s) \($since)"' <<<"$json")" || { echo "PR $pr STATE UNKNOWN unparsable gh output" >&2; exit 2; }
read -r state since <<<"$parsed"
[ -n "${state:-}" ] || { echo "PR $pr STATE UNKNOWN empty state" >&2; exit 2; }

case "$state" in
  DRAFT)       limit=1800; action="ready: gh pr ready, else escalate blocked" ;;
  CHECKLESS)   limit=300;  action="unblock: tasks/ci.md, no-checks section" ;;
  CI-RUNNING)  limit=3600; action="cancel and rerun: gh run cancel <run>, gh run rerun <run>, else escalate blocked" ;;
  CI-RED)      limit=1800; action="fix: tasks/red-ci.md, else escalate blocked" ;;
  CONFLICTING) limit=900;  action="rebase: git rebase origin/main, gate, push --force-with-lease" ;;
  ARMED)       limit=1800; action="merge: wait-merge.sh, else disarm and escalate blocked" ;;
  GREEN)       limit=600;  action="merge: wait-merge.sh" ;;
  *)           echo "PR $pr STATE $state DWELL 0 LIMIT 0 OK ACTION none"; exit 0 ;;
esac
var="AUTOPILOT_LIMIT_${state//-/_}"
limit="${!var:-$limit}"
dwell=$((now - since)); [ "$dwell" -lt 0 ] && dwell=0
verdict=OK; [ "$dwell" -gt "$limit" ] && verdict=STUCK
echo "PR $pr STATE $state DWELL $dwell LIMIT $limit $verdict ACTION $action"
[ "$verdict" = OK ]
