#!/usr/bin/env bash
# Drives scripts/autopilot/pr-state.sh with a `gh` stub that prints a JSON fixture.
set -uo pipefail

ROOT="$(git rev-parse --show-toplevel)"
S="$ROOT/scripts/autopilot/pr-state.sh"
failures=0
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$tmp/bin"
printf '#!/usr/bin/env bash\n[ -f "$GH_FIXTURE" ] && cat "$GH_FIXTURE" || exit 1\n' > "$tmp/bin/gh"
chmod +x "$tmp/bin/gh"
export PATH="$tmp/bin:$PATH" GH_FIXTURE="$tmp/pr.json"
export AUTOPILOT_NOW=1000000000   # 2001-09-09T01:46:40Z
unset AUTOPILOT_LIMIT_DRAFT AUTOPILOT_LIMIT_CI_RED

PUSH="2001-09-09T01:36:40Z"   # 600 s before now
LATE="2001-09-09T01:26:40Z"   # 1200 s before now

# fixture <state> <draft> <mergeable> <auto> <commit-date> <checks-json>
fixture() {
  jq -n --arg st "$1" --argjson d "$2" --arg m "$3" --argjson a "$4" --arg c "$5" --argjson k "$6" \
    '{state:$st,isDraft:$d,mergeable:$m,autoMergeRequest:$a,commits:[{committedDate:$c}],statusCheckRollup:$k}' > "$GH_FIXTURE"
}
# expect <name> <exit> <expected-line-prefix>
expect() {
  local out code
  out="$("$S" 7 2>&1)"; code=$?
  if [ "$code" = "$2" ] && [[ "$out" == "$3"* ]]; then echo "ok   $1"; else
    echo "FAIL $1: exit $code, got: $out (want $2, $3...)"; failures=$((failures + 1)); fi
}

OK_CHECK='[{"name":"ci complete","status":"COMPLETED","conclusion":"SUCCESS","completedAt":"'$PUSH'"}]'
RED_CHECK='[{"name":"test","status":"COMPLETED","conclusion":"FAILURE","completedAt":"'$PUSH'"}]'
RUN_CHECK='[{"name":"test","status":"IN_PROGRESS","conclusion":"","completedAt":"0001-01-01T00:00:00Z"}]'
CTX_PENDING='[{"__typename":"StatusContext","state":"PENDING"}]'
CTX_ERROR='[{"__typename":"StatusContext","state":"ERROR"}]'

fixture OPEN true MERGEABLE null "$PUSH" '[]'
expect "draft within limit" 0 "PR 7 STATE DRAFT DWELL 600 LIMIT 1800 OK"
fixture OPEN true MERGEABLE null "$LATE" '[]'
AUTOPILOT_LIMIT_DRAFT=900 expect "draft past an overridden limit is stuck" 1 "PR 7 STATE DRAFT DWELL 1200 LIMIT 900 STUCK ACTION ready"
fixture OPEN false MERGEABLE null "$PUSH" '[]'
expect "no checks for 10 min is stuck" 1 "PR 7 STATE CHECKLESS DWELL 600 LIMIT 300 STUCK"
fixture OPEN false MERGEABLE null "$PUSH" "$RUN_CHECK"
expect "ci running" 0 "PR 7 STATE CI-RUNNING DWELL 600 LIMIT 3600 OK"
fixture OPEN false MERGEABLE null "$PUSH" "$CTX_PENDING"
expect "status context pending" 0 "PR 7 STATE CI-RUNNING"
fixture OPEN false MERGEABLE null "$PUSH" "$CTX_ERROR"
expect "status context error" 0 "PR 7 STATE CI-RED"
fixture OPEN false MERGEABLE null "2001-09-09T01:36:40.123Z" "$RUN_CHECK"
expect "fractional seconds parse" 0 "PR 7 STATE CI-RUNNING DWELL 600"
echo '{"state":' > "$GH_FIXTURE"
expect "garbage json" 2 "PR 7 STATE UNKNOWN"
fixture OPEN false MERGEABLE null "$PUSH" "$RED_CHECK"
expect "ci red within limit" 0 "PR 7 STATE CI-RED DWELL 600 LIMIT 1800 OK"
fixture OPEN false MERGEABLE null "$LATE" '[{"name":"test","status":"COMPLETED","conclusion":"FAILURE","completedAt":"'$LATE'"}]'
AUTOPILOT_LIMIT_CI_RED=1000 expect "ci red past limit" 1 "PR 7 STATE CI-RED DWELL 1200 LIMIT 1000 STUCK ACTION fix"
fixture OPEN false CONFLICTING null "$LATE" "$OK_CHECK"
expect "conflicting beats green checks; dwell from the later of push and check" 0 "PR 7 STATE CONFLICTING DWELL 600 LIMIT 900 OK"
fixture OPEN false CONFLICTING null "$LATE" '[]'
expect "conflicting past 15 min" 1 "PR 7 STATE CONFLICTING DWELL 1200 LIMIT 900 STUCK ACTION rebase"
fixture OPEN false MERGEABLE '{"mergeMethod":"REBASE"}' "$PUSH" "$OK_CHECK"
expect "armed" 0 "PR 7 STATE ARMED DWELL 600 LIMIT 1800 OK"
fixture OPEN false MERGEABLE null "$LATE" '[{"name":"ci complete","status":"COMPLETED","conclusion":"SUCCESS","completedAt":"'$LATE'"}]'
expect "green unmerged past 10 min" 1 "PR 7 STATE GREEN DWELL 1200 LIMIT 600 STUCK ACTION merge"
fixture MERGED false UNKNOWN null "$LATE" "$OK_CHECK"
expect "merged is terminal" 0 "PR 7 STATE MERGED DWELL 0 LIMIT 0 OK ACTION none"
rm -f "$GH_FIXTURE"
expect "gh failure" 2 "PR 7 STATE UNKNOWN"

[ "$failures" = 0 ] && echo "pr-state: all passed" || { echo "pr-state: $failures failed"; exit 1; }
