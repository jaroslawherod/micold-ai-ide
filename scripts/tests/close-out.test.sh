#!/usr/bin/env bash
# Drives scripts/autopilot/verify-run.sh against fixture runs: the verdict, the remaining lines, the
# exit codes and read-only behaviour. `gh` is the fixture stub (pr-<n>.json, issue-<n>.json) and
# pr-state.sh is stubbed through AUTOPILOT_PR_STATE. Spec: specs/1816-supervisor-verify-report.
set -uo pipefail
# shellcheck source=lib/autopilot-fixtures.sh
. "$(dirname "$0")/lib/autopilot-fixtures.sh"
unset AUTOPILOT_SUPERVISED
export AUTOPILOT_NOW=2000000000
cat > "$tmp/prstate.sh" <<'STUB'
#!/usr/bin/env bash
echo "PR $1 STATE OPEN DWELL 10 LIMIT 3600 OK ACTION none"
STUB
chmod +x "$tmp/prstate.sh"
export AUTOPILOT_PR_STATE="$tmp/prstate.sh"
VR="$S/verify-run.sh"

# fresh: a clean repo on branch wt with a done ledger (issue 42, kind feature) naming PR 7, merged,
# committed and pushed to origin/wt's main line, and its closed, labelled issue.
fresh() {
  d="$(new_repo)"; cd "$d/wt" || exit 1; export GH_FIXTURES="$d/fx"
  mkdir -p specs/042-x
  {
    echo "# Autopilot ledger — #42 x"; echo "- **Kind**: feature"; echo "- **Issue**: #42"
    echo "- **Worktree branch**: wt"; echo "- **Phase**: ${PHASE:-done}"; echo "- **Next step**: handoff"
    echo; echo "## Pull requests"; echo "| PR | Purpose | Status |"
    for p in ${PRS-7}; do echo "| #$p | x | merged |"; done
    echo; echo "## Decisions"; echo "| D1 | x |"
    echo; echo "## Open escalation"; echo; echo "None."
  } > specs/042-x/autopilot.md
  git add -A; git commit -qm "record the run"; git push -q origin HEAD:main 2>/dev/null
  for p in ${PRS-7}; do pr_json "$d/fx" "$p" MERGED > "$d/fx/pr-$p.json"; done
  issue_json CLOSED in-progress= flow:feature > "$d/fx/issue-42.json"
}
# issue_json <state> <label...>: a label spelled `x=` is omitted (a readable way to say "without x").
issue_json() {
  local st="$1" l ls=""; shift
  for l in "$@"; do case "$l" in *=) ;; *) ls="$ls${ls:+,}{\"name\":\"$l\"}" ;; esac; done
  printf '{"state":"%s","labels":[%s]}\n' "$st" "$ls"
}
# verdict <name> <want-exit> <regex>: the verdict text for the current worktree, run from $tmp.
verdict() { check "$1" "$2" "$3" bash -c 'cd "$2" && "$0" "$1"' "$VR" "$PWD" "$tmp"; }
# one <name> <regex>: NOT DONE with exactly this line among the remaining ones, exit 1.
notdone() { verdict "$1" 1 "^- $2\$"; }

# --- US1.1: all merged, clean, issue closed with its flow label -----------------------------------
fresh
verdict "all merged, clean, issue closed: VERIFIED" 0 '^VERIFIED$'
check "VERIFIED prints no remaining line" 0 '^VERIFIED$' bash -c '[ "$("$0" "$1" | wc -l)" = 1 ] && "$0" "$1"' "$VR" "$PWD"

# --- US1.2, US1.3: what remains in the run ----------------------------------------------------------
fresh; pr_json "$d/fx" 7 OPEN > "$d/fx/pr-7.json"
notdone "an open PR is listed" 'PR #7 is OPEN'
verdict "an open PR starts with NOT DONE" 1 '^NOT DONE$'
fresh; rm "$d/fx/pr-7.json"
notdone "a PR gh cannot see is UNKNOWN, never done" 'PR #7 is UNKNOWN'
fresh; echo x > new.txt
notdone "a dirty tree is listed" 'uncommitted work: 1 path\(s\)'
fresh; echo x > new.txt; git add -A; git commit -qm local
notdone "an unmerged commit is listed" '1 commit\(s\) not on origin/main'
PHASE=4-milestones fresh
notdone "a phase other than done is listed" 'ledger phase is 4-milestones'
PHASE=4-milestones fresh; pr_json "$d/fx" 7 OPEN > "$d/fx/pr-7.json"
notdone "an open PR is listed while the phase is open" 'PR #7 is OPEN'
fresh; git rm -q specs/042-x/autopilot.md; git commit -qm drop; git push -q origin HEAD:main 2>/dev/null
notdone "no ledger is listed" 'no ledger'
PRS="" fresh
verdict "a done ledger with no PRs is verified on the other checks" 0 '^VERIFIED$'
PHASE=4-milestones fresh; cp specs/042-x/autopilot.md specs/042-x/autopilot-2.md
 git add -A; git commit -qm two; git push -q origin HEAD:main 2>/dev/null
verdict "several ledgers are exit 3 and are listed" 3 'autopilot-2\.md'
check "not a worktree is exit 2" 2 'not a git worktree' "$VR" "$tmp"
check "an unknown option is exit 2" 2 'unknown option' "$VR" --nope

# --- review A: a PR only GitHub knows, several done ledgers, a failing run-status -------------------
fresh; echo '[{"number":9}]' > "$d/fx/pr-list.json"; pr_json "$d/fx" 9 OPEN > "$d/fx/pr-9.json"
notdone "the branch's open PR missing from the ledger is listed" 'PR #9 is OPEN'
fresh; cp specs/042-x/autopilot.md specs/042-x/autopilot-2.md; git add -A; git commit -qm two; git push -q origin HEAD:main 2>/dev/null
verdict "several done ledgers are exit 3 too" 3 'autopilot-2\.md'
fresh; printf '#!/usr/bin/env bash\nexit 5\n' > "$tmp/rs-fail.sh"; chmod +x "$tmp/rs-fail.sh"
check "a failing run-status.sh is NOT DONE, never a bare exit" 1 'run-status.sh failed \(exit 5\)' \
  env AUTOPILOT_RUN_STATUS="$tmp/rs-fail.sh" "$VR" "$d/wt"

# --- US1.4: the issue -------------------------------------------------------------------------------
fresh; issue_json OPEN in-progress flow:feature > "$d/fx/issue-42.json"
notdone "an open issue is listed" 'issue #42 is open'
fresh; issue_json OPEN in-progress flow:feature > "$d/fx/issue-42.json"
notdone "in-progress left is listed" 'issue #42 still labelled in-progress'
fresh; issue_json CLOSED flow:feature in-progress > "$d/fx/issue-42.json"
notdone "in-progress left on a closed issue is listed" 'issue #42 still labelled in-progress'
fresh; issue_json CLOSED bug > "$d/fx/issue-42.json"
notdone "a missing flow label is listed" 'issue #42 lacks flow:feature'
fresh; rm "$d/fx/issue-42.json"
notdone "an issue gh cannot read is UNKNOWN" 'issue #42 state UNKNOWN'

# --- US1.5, FR-005: the target is verified, the caller untouched --------------------------------------
fresh; target="$d/wt"; echo x > "$target/new.txt"
caller="$(new_repo)/wt"; ( cd "$caller" && echo dirty > c.txt )
check "verifying from a clean second clone reflects the dirty target" 1 'uncommitted work: 1 path' \
  bash -c 'cd "$2" && "$0" "$1"' "$VR" "$target" "$caller"
fresh; target="$d/wt"; caller="$(new_repo)/wt"; ( cd "$caller" && echo dirty > c.txt )
check "verifying from a dirty second clone reflects the clean target" 0 '^VERIFIED$' \
  bash -c 'cd "$2" && "$0" "$1"' "$VR" "$target" "$caller"
fresh; echo x > new.txt
snap() { ( cd "$d/wt" && sha256sum specs/042-x/autopilot.md; git rev-parse HEAD; git status --porcelain ); }
before="$(snap)"; "$VR" "$d/wt" >/dev/null 2>&1; after="$(snap)"
cases=$((cases + 1)); [ "$before" = "$after" ] || { failures=$((failures + 1)); echo "FAIL verifying changed the ledger, HEAD or tree"; }

# --- US2, FR-006, FR-007, SC-004: follow-up.sh ----------------------------------------------------
FU="$S/follow-up.sh"
# fu <args...>: follow-up.sh on the current worktree against $SL.
fu() { "$FU" "$d/wt" --supervisor-ledger "$SL" "$@"; }
# count: the Follow-ups count of branch wt in $SL.
count() { awk -F'|' '$2 ~ /^ wt $/ { gsub(/ /, "", $3); print $3 }' "$SL"; }
fresh; pr_json "$d/fx" 7 OPEN > "$d/fx/pr-7.json"; SL="$d/sup.md"
check "a live session gets SEND with the remaining lines" 0 '^SEND s1$' fu --session live --session-id s1
check "SEND carries the message head" 0 '^Run not done\. Remaining:$' fu --session live --session-id s1
check "SEND carries the remaining item" 0 '^- PR #7 is OPEN$' fu --session live --session-id s1
cases=$((cases + 1)); [ "$(count)" = 3 ] || { failures=$((failures + 1)); echo "FAIL count after three follow-ups is $(count)"; }
fresh; pr_json "$d/fx" 7 OPEN > "$d/fx/pr-7.json"; SL="$d/sup.md"
check "a stopped session gets RESUME with the remaining lines" 0 '^RESUME s2$' fu --session stopped --session-id s2
cases=$((cases + 1)); [ "$(count)" = 1 ] || { failures=$((failures + 1)); echo "FAIL RESUME did not count (got $(count))"; }
check "RESUME has no message head" 0 '^NONE$' bash -c '! "$@" | grep -q "Run not done" && echo NONE' _ "$FU" "$d/wt" --supervisor-ledger "$d/r.md" --session stopped
fresh; pr_json "$d/fx" 7 OPEN > "$d/fx/pr-7.json"; SL="$d/sup.md"
check "an absent session id prints the action alone" 0 '^SEND$' fu --session live

# cap: three counted follow-ups, then one ESCALATE, then HOLD
fresh; pr_json "$d/fx" 7 OPEN > "$d/fx/pr-7.json"; SL="$d/sup.md"
for i in 1 2 3; do fu --session live --session-id s1 >/dev/null; done
check "the fourth call is ESCALATE with a category 5 banner" 0 '^- \*\*Category\*\*: 5 ' fu --session live
fresh; pr_json "$d/fx" 7 OPEN > "$d/fx/pr-7.json"; SL="$d/sup.md"
for i in 1 2 3; do fu --session live --session-id s1 >/dev/null; done
check "ESCALATE is the first line at the cap" 0 '^ESCALATE$' bash -c '"$@" | head -1' _ "$FU" "$d/wt" --supervisor-ledger "$SL" --session live
cases=$((cases + 1)); [ "$(grep -c '^### wt$' "$SL")" = 1 ] || { failures=$((failures + 1)); echo "FAIL not exactly one escalation block"; }
check "the block names what remains" 0 'PR #7 is OPEN' grep -F 'PR #7 is OPEN' "$SL"
check "after ESCALATE the next call is HOLD" 0 '^HOLD$' fu --session live --session-id s1
check "HOLD sends nothing" 0 '^NONE$' bash -c '! "$@" | grep -q "^SEND" && echo NONE' _ "$FU" "$d/wt" --supervisor-ledger "$SL" --session live
cases=$((cases + 1)); [ "$(grep -c '^### wt$' "$SL")" = 1 ] || { failures=$((failures + 1)); echo "FAIL HOLD wrote a second block"; }
cases=$((cases + 1)); [ "$(count)" = 3 ] || { failures=$((failures + 1)); echo "FAIL count moved past the cap: $(count)"; }

# SKIP: same remaining set and check id
fresh; pr_json "$d/fx" 7 OPEN > "$d/fx/pr-7.json"; SL="$d/sup.md"
fu --session live --session-id s1 --check c1 >/dev/null
check "the same check id and remaining set is SKIP" 0 '^SKIP$' fu --session live --session-id s1 --check c1
cases=$((cases + 1)); [ "$(count)" = 1 ] || { failures=$((failures + 1)); echo "FAIL SKIP counted: $(count)"; }
check "a new check id sends again" 0 '^SEND s1$' fu --session live --session-id s1 --check c2
fresh; pr_json "$d/fx" 7 OPEN > "$d/fx/pr-7.json"; SL="$d/sup.md"
fu --session live --check c1 >/dev/null; echo x > new.txt
check "a changed remaining set is not SKIP" 0 '^SEND$' fu --session live --check c1

# DONE: VERIFIED after a follow-up, count unchanged
fresh; pr_json "$d/fx" 7 OPEN > "$d/fx/pr-7.json"; SL="$d/sup.md"
fu --session live --session-id s1 >/dev/null; pr_json "$d/fx" 7 MERGED > "$d/fx/pr-7.json"
check "a run turned VERIFIED is DONE" 0 '^DONE$' fu --session live --session-id s1
cases=$((cases + 1)); [ "$(count)" = 1 ] || { failures=$((failures + 1)); echo "FAIL DONE changed the count: $(count)"; }
check "an unknown option is exit 2" 2 'unknown option' "$FU" --nope
check "a missing ledger option is exit 2" 2 'supervisor-ledger' "$FU" "$d/wt" --session live
check "two runs never share a count" 0 '^SEND$' bash -c 'cd "$2" && git switch -q -c other && echo "| wt | 3 | 1 | - |" >/dev/null && "$0" "$2" --supervisor-ledger "$3" --session live' "$FU" "$d" "$d/wt" "$SL"

echo "close-out: $cases cases, $failures failed"
[ "$failures" -eq 0 ]
