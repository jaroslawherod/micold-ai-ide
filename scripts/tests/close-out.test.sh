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

# fresh: a clean repo on branch $BR (wt) with a done ledger (issue $ISSUE (42), kind feature) naming PR 7
# (or $PRS), merged, with $DECS (agent) as the Decisions' By cells and $TOKENS as its Tokens field,
# committed and pushed to origin/wt's main line, and its closed, labelled issue.
fresh() {
  local n="${ISSUE:-42}" br="${BR:-wt}" dec
  d="$(new_repo)"; cd "$d/wt" || exit 1; export GH_FIXTURES="$d/fx"
  [ "$br" = wt ] || git switch -q -c "$br"
  mkdir -p specs/0$n-x
  {
    echo "# Autopilot ledger — #$n x"; echo "- **Kind**: feature"; echo "- **Issue**: #$n"
    echo "- **Worktree branch**: $br"; echo "- **Phase**: ${PHASE:-done}"; echo "- **Next step**: handoff"
    [ -z "${TOKENS:-}" ] || echo "- **Tokens**: $TOKENS"
    echo; echo "## Pull requests"; echo "| PR | Purpose | Status |"
    for p in ${PRS-7}; do echo "| #$p | x | merged |"; done
    echo; echo "## Decisions"; echo "| # | Phase | Question | Answer | By | Evidence |"; echo "|---|---|---|---|---|---|"
    for dec in ${DECS:-agent}; do echo "| 1 | spec | q | a | $dec | e |"; done
    echo; echo "## Open escalation"; echo; echo "None."
  } > specs/0$n-x/autopilot.md
  git add -A; git commit -qm "record the run"; git push -q origin HEAD:main 2>/dev/null
  for p in ${PRS-7}; do pr_json "$d/fx" "$p" MERGED > "$d/fx/pr-$p.json"; done
  issue_json CLOSED in-progress= flow:feature > "$d/fx/issue-$n.json"
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
check "RESUME has no message head" 0 '^OK$' bash -c 'o="$("$@")" || exit 1; [ "$(head -1 <<<"$o")" = "RESUME" ] && ! grep -q "Run not done" <<<"$o" && echo OK' _ "$FU" "$d/wt" --supervisor-ledger "$d/r.md" --session stopped
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
check "HOLD sends nothing" 0 '^OK$' bash -c 'o="$("$@")" || exit 1; [ "$o" = "HOLD" ] && echo OK' _ "$FU" "$d/wt" --supervisor-ledger "$SL" --session live
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
printf '## Follow-ups\n\n| Run | Count | Remaining hash | Check |\n|---|---|---|---|\n| wt | 3 | 1 | - |\n' > "$d/seed.md"
check "two runs never share a count" 0 '^SEND$' bash -c 'cd "$2" && git switch -q -c other && "$0" "$2" --supervisor-ledger "$3" --session live' "$FU" "$d/wt" "$d/wt" "$d/seed.md"

# reset: back to 0, block removed, three more rounds, ESCALATE again
fresh; pr_json "$d/fx" 7 OPEN > "$d/fx/pr-7.json"; SL="$d/sup.md"
for i in 1 2 3; do fu --session live >/dev/null; done; fu --session live >/dev/null
"$S/supervisor-ledger.sh" followup-reset "$SL" wt
cases=$((cases + 1)); [ "$(count)" = 0 ] && ! grep -q '^### wt$' "$SL" || { failures=$((failures + 1)); echo "FAIL reset left count $(count) or the block"; }
for i in 1 2 3; do fu --session live >/dev/null; done
check "after a reset and three more rounds the run escalates again" 0 '^ESCALATE$' bash -c '"$@" | head -1' _ "$FU" "$d/wt" --supervisor-ledger "$SL" --session live
cases=$((cases + 1)); [ "$(grep -c '^### wt$' "$SL")" = 1 ] || { failures=$((failures + 1)); echo "FAIL not one block after re-escalation"; }

# a padded row is still the run's row; a non-numeric count reads as 0
fresh; pr_json "$d/fx" 7 OPEN > "$d/fx/pr-7.json"; SL="$d/sup.md"
printf '## Follow-ups\n\n| Run | Count | Remaining hash | Check |\n|---|---|---|---|\n|  wt  | 1 | 5 | - |\n' > "$SL"
fu --session live >/dev/null; fu --session live >/dev/null
cases=$((cases + 1)); [ "$(grep -c 'wt' "$SL")" = 1 ] || { failures=$((failures + 1)); echo "FAIL a padded row was not replaced"; }
check "a padded row's count reaches the cap" 0 '^ESCALATE$' fu --session live
printf '## Follow-ups\n\n| Run | Count | Remaining hash | Check |\n|---|---|---|---|\n| wt |  |  |  |\n' > "$SL"
check "an empty count reads as 0" 0 '^SEND$' fu --session live

# --- US3, FR-008..FR-010, FR-016, SC-002: close-out-report.sh -------------------------------------------
CO="$S/close-out-report.sh"
# newrun <issue> <branch>: a fresh run whose worktree is $RUN; its fixtures join the shared $FX.
newrun() {
  ISSUE="$1" BR="$2" fresh; RUN="$d/wt"
  if [ -z "${FX:-}" ]; then FX="$d/fx"; else cp "$d/fx/"* "$FX/"; fi
  export GH_FIXTURES="$FX"
}
# section <name>: the lines of `## <name>` of the report on stdin.
section() { awk -v h="## $1" '$0 == h { on = 1; next } /^## / { on = 0 } on'; }
# report <args...>: the report, run from $tmp.
report() { ( cd "$tmp" && "$CO" "$@" ); }
SL="$tmp/sup-report.md"
printf '%s\n' '## Interventions' '' '| Run | When | What |' '|---|---|---|' \
  '| second | 2026-11-14 | sent a follow-up about PR #8 |' '| second | 2026-11-15 | resumed the session |' \
  '| other | 2026-11-15 | not this run |' > "$SL"

# US3.1: two verified runs, one entry each; decisions counted by user and by agent
FX=""; DECS="user agent agent" TOKENS=1200 newrun 42 first; r1="$RUN"; PRS=8 newrun 43 second; r2="$RUN"; unset FX
out="$(report "$r1" "$r2" --supervisor-ledger "$SL" --no-live-sessions)"; rc=$?
cases=$((cases + 1)); [ "$rc" = 0 ] || { failures=$((failures + 1)); echo "FAIL report exit $rc"; }
want="$(printf '%s\n' '### #42 first' '- PRs: #7 MERGED' '- Decisions: 1 by user, 2 by agent' '- Interventions: none' '- Tokens: 1200' \
  '### #43 second' '- PRs: #8 MERGED' '- Decisions: 0 by user, 1 by agent' '- Interventions:' \
  '  - 2026-11-14: sent a follow-up about PR #8' '  - 2026-11-15: resumed the session' '- Tokens: unknown')"
cases=$((cases + 1)); [ "$(section Completed <<<"$out")" = "$want" ] || { failures=$((failures + 1)); echo "FAIL Completed section:"; section Completed <<<"$out" | sed 's/^/    /'; }
cases=$((cases + 1)); [ "$(grep '^## ' <<<"$out" | tr '\n' '|')" = '## Completed|## Not done|## Escalated|## Safe to remove|' ] \
  || { failures=$((failures + 1)); echo "FAIL sections not all present, in order"; }
cases=$((cases + 1)); [ -z "$(section 'Not done' <<<"$out")" ] && [ -z "$(section Escalated <<<"$out")" ] \
  || { failures=$((failures + 1)); echo "FAIL a verified run is also listed apart"; }

# US3.2, SC-002: an unmerged PR is under Not done, never Completed
FX=""; newrun 42 first; r1="$RUN"; pr_json "$d/fx" 7 OPEN > "$FX/pr-7.json"; unset FX
out="$(report "$r1" --supervisor-ledger "$SL")"
cases=$((cases + 1)); [ -z "$(section Completed <<<"$out")" ] || { failures=$((failures + 1)); echo "FAIL an open-PR run is under Completed"; }
cases=$((cases + 1)); [ "$(section 'Not done' <<<"$out")" = "$(printf '%s\n' '### #42 first' '- PRs: #7 OPEN' '- Remaining: PR #7 is OPEN')" ] \
  || { failures=$((failures + 1)); echo "FAIL Not done section:"; section 'Not done' <<<"$out" | sed 's/^/    /'; }

# a run that regressed after it was reported done moves to Not done
FX=""; newrun 42 first; r1="$RUN"; unset FX
check "a verified run is Completed" 0 '^### #42 first$' bash -c '"$0" "$@" | awk "/^## Completed/{on=1;next} /^## /{on=0} on"' "$CO" "$r1"
pr_json "$d/fx" 7 OPEN > "$d/fx/pr-7.json"
cases=$((cases + 1)); [ -z "$(section Completed <<<"$(report "$r1")")" ] || { failures=$((failures + 1)); echo "FAIL a regressed run stays Completed"; }
check "the regressed run is under Not done" 0 'Remaining: PR #7 is OPEN' report "$r1"

# a run with an escalation block is under Escalated with its Question, not under Not done
FX=""; newrun 42 first; r1="$RUN"; pr_json "$d/fx" 7 OPEN > "$d/fx/pr-7.json"; unset FX
printf '%s\n' '## Escalations' '' '### first' '- **Category**: 5 Non-convergence' \
  '- **Question**: Run first is still NOT DONE after 3 follow-ups; how should it proceed?' '- **Recommended**: x' > "$tmp/sup-esc.md"
out="$(report "$r1" --supervisor-ledger "$tmp/sup-esc.md")"
cases=$((cases + 1)); [ "$(section Escalated <<<"$out")" = "$(printf '%s\n' '### #42 first' '- PRs: #7 OPEN' '- Run first is still NOT DONE after 3 follow-ups; how should it proceed?')" ] \
  || { failures=$((failures + 1)); echo "FAIL Escalated section:"; section Escalated <<<"$out" | sed 's/^/    /'; }
cases=$((cases + 1)); [ -z "$(section 'Not done' <<<"$out")" ] || { failures=$((failures + 1)); echo "FAIL an escalated run is also under Not done"; }
# a verified run with a stale block stays Completed (precedence)
pr_json "$d/fx" 7 MERGED > "$d/fx/pr-7.json"
cases=$((cases + 1)); [ -z "$(section Escalated <<<"$(report "$r1" --supervisor-ledger "$tmp/sup-esc.md")")" ] \
  || { failures=$((failures + 1)); echo "FAIL a verified run with an old block is Escalated"; }

# errors never abort the report: several ledgers (exit 3) and a non-worktree are one line under Not done
FX=""; newrun 42 first; r1="$RUN"; unset FX
cp specs/042-x/autopilot.md specs/042-x/autopilot-2.md; git add -A; git commit -qm two; git push -q origin HEAD:main 2>/dev/null
out="$(report "$r1" "$tmp/nowhere")"; rc=$?
cases=$((cases + 1)); [ "$rc" = 0 ] && [ "$(section 'Not done' <<<"$out" | grep -c '^- Remaining: ')" = 2 ] \
  || { failures=$((failures + 1)); echo "FAIL errored runs: exit $rc"; printf '%s\n' "$out" | sed 's/^/    /'; }

# US3.3, edge: zero runs and an absent supervisor ledger give empty sections, exit 0
check "no runs is exit 0 with every section" 0 '^## Safe to remove$' report
check "no runs lists nothing" 0 '^OK$' bash -c '! "$0" | grep -q "^###" && echo OK' "$CO"
check "an absent supervisor ledger is exit 0" 0 '^## Completed$' report "$r1" --supervisor-ledger "$tmp/absent.md"
check "an unknown option is exit 2" 2 'unknown option' "$CO" --nope

# FR-016, T020: runs come from runs-of-parent.sh; another parent or none is left out; the parent is named
FX=""; newrun 42 own; r1="$RUN"; newrun 43 stray; r2="$RUN"; newrun 44 loose; r3="$RUN"; unset FX
rows="$tmp/rows.json"
jq -n --arg a "$r1" --arg b "$r2" --arg c "$r3" '{worktrees: [
  {ref: "wt", path: $a, parent: "sup-1", parent_state: "known"},
  {ref: "wt", path: $b, parent: "sup-2", parent_state: "known"},
  {ref: "wt", path: $c}]}' > "$rows"
out="$(report "$r1" "$r2" "$r3" --parent sup-1 --rows "$rows")"
cases=$((cases + 1)); [ "$(section Completed <<<"$out")" = "$(printf '%s\n' '### #42 own' '- Parent: sup-1' '- PRs: #7 MERGED' '- Decisions: 0 by user, 1 by agent' '- Interventions: none' '- Tokens: unknown')" ] \
  || { failures=$((failures + 1)); echo "FAIL own run only, with its parent:"; section Completed <<<"$out" | sed 's/^/    /'; }
cases=$((cases + 1)); ! grep -qE '#43|#44' <<<"$out" || { failures=$((failures + 1)); echo "FAIL a run of another parent or none is reported"; }
jq -n --arg a "$r1" '{worktrees: [{ref: "wt", path: $a, parent: "sup-1", parent_state: "removed"}]}' > "$rows"
cases=$((cases + 1)); ! grep -q '^###' <<<"$(report "$r1" --parent sup-1 --rows "$rows")" || { failures=$((failures + 1)); echo "FAIL a removed-parent run is reported"; }
check "a parent without rows is exit 2" 2 'rows' "$CO" "$r1" --parent sup-1
check "unreadable rows are exit 2" 2 'unreadable|rows' "$CO" "$r1" --parent sup-1 --rows "$tmp/absent.json"

# --- US4, FR-011..FR-013, FR-016, SC-005: safe-to-remove.sh ---------------------------------------------
SR="$S/safe-to-remove.sh"
# safe <args...>: the list, run from $tmp.
safe() { ( cd "$tmp" && "$SR" "$@" ); }
# want_lines <name> <want> <got>: the two multi-line texts are equal.
want_lines() { cases=$((cases + 1)); [ "$2" = "$3" ] || { failures=$((failures + 1)); echo "FAIL $1:"; printf '%s\n' "$3" | sed 's/^/    /'; }; }
AR="$tmp/sup-auto.md"; printf '%s\n' '# Supervisor' '- **Auto-remove**: yes' > "$AR"
NOAR="$tmp/sup-noauto.md"; printf '%s\n' '# Supervisor' '- **Auto-remove**: maybe' > "$NOAR"

# US4.1: a clean verified run with no live session is SAFE; a session elsewhere does not matter
FX=""; newrun 42 clean; r1="$RUN"; unset FX
want_lines "a clean verified run is SAFE" "SAFE $r1" "$(safe "$r1" --live "$tmp/elsewhere")"
want_lines "--no-live-sessions says nobody is live" "SAFE $r1" "$(safe "$r1" --no-live-sessions)"

# US4.2: every reason to keep a worktree is named
want_lines "a live session keeps it" "KEEP $r1 live session" "$(safe "$r1" --live "$r1")"
want_lines "a live session under a relative or odd path keeps it" "KEEP $r1 live session" "$(safe "$r1" --live "$r1/../wt")"
want_lines "no session list keeps it" "KEEP $r1 no session list given" "$(safe "$r1")"
(cd "$r1" && echo x > new.txt)
want_lines "a dirty tree keeps it" "KEEP $r1 not verified: uncommitted work: 1 path(s)" "$(safe "$r1" --no-live-sessions)"
(cd "$r1" && rm new.txt && echo x > new.txt && git add -A && git commit -qm local)
want_lines "an unmerged commit keeps it" "KEEP $r1 not verified: 1 commit(s) not on origin/main" "$(safe "$r1" --no-live-sessions)"
FX=""; newrun 44 twin; r3="$RUN"; unset FX
(cd "$r3" && cp specs/044-x/autopilot.md specs/044-x/autopilot-2.md && git add -A && git commit -qm two -q && git push -q origin HEAD:main 2>/dev/null)
want_lines "several ledgers keep it" "KEEP $r3 several ledgers" "$(safe "$r3" --no-live-sessions 2>/dev/null)"
FX=""; PRS=8 newrun 43 open; r2="$RUN"; pr_json "$d/fx" 8 OPEN > "$d/fx/pr-8.json"; newrun 45 ready; r4="$RUN"; unset FX
want_lines "a run that is not verified keeps it, with the first remaining line" "KEEP $r2 not verified: PR #8 is OPEN" "$(safe "$r2" --no-live-sessions)"
want_lines "a non-worktree keeps it" "KEEP $tmp/nowhere not a git worktree" "$(safe "$tmp/nowhere" --no-live-sessions)"
check "no directory is exit 0 with no lines" 0 '^OK$' bash -c '[ -z "$("$0")" ] && echo OK' "$SR"
check "an unknown option is exit 2" 2 'unknown option' "$SR" --nope

# US4.3, SC-005: no opt-in, no REMOVE line, nothing removed; US4.4: opt-in, REMOVE only after SAFE
for led in "" "--supervisor-ledger $tmp/absent.md" "--supervisor-ledger $NOAR"; do
  # shellcheck disable=SC2086
  out="$(safe "$r4" "$r2" --no-live-sessions $led)"
  cases=$((cases + 1)); ! grep -q '^REMOVE' <<<"$out" && [ -d "$r4" ] || { failures=$((failures + 1)); echo "FAIL REMOVE without opt-in (${led:-no ledger})"; }
done
want_lines "opt-in: a REMOVE request follows each SAFE line only" "$(printf '%s\n' "SAFE $r4" "REMOVE $r4" "KEEP $r2 not verified: PR #8 is OPEN")" \
  "$(safe "$r4" "$r2" --no-live-sessions --supervisor-ledger "$AR")"
want_lines "opt-in: a live worktree gets no REMOVE" "KEEP $r4 live session" "$(safe "$r4" --live "$r4" --supervisor-ledger "$AR")"
cases=$((cases + 1)); [ -d "$r4" ] && [ -d "$r2" ] || { failures=$((failures + 1)); echo "FAIL a worktree disappeared"; }
cases=$((cases + 1)); ! grep -nE 'worktree[[:space:]]+(remove|prune)|delete_worktree|(^|[^[:alnum:]_.-])rm[[:space:]]|git[[:space:]]+branch[[:space:]]+-[dD]' <(grep -v '^[[:space:]]*#' "$SR") >/dev/null \
  || { failures=$((failures + 1)); echo "FAIL safe-to-remove.sh has a removal command"; }

# FR-016, T021: only the supervisor's own parent's runs are listed; another parent's is never SAFE or REMOVE
rows="$tmp/rows-safe.json"
jq -n --arg a "$r4" --arg b "$r1" '{worktrees: [
  {ref: "wt", path: $a, parent: "sup-1", parent_state: "known"},
  {ref: "wt", path: $b, parent: "sup-2", parent_state: "known"}]}' > "$rows"
want_lines "only the own parent's runs are listed" "$(printf '%s\n' "SAFE $r4" "REMOVE $r4")" \
  "$(safe "$r4" "$r1" --no-live-sessions --supervisor-ledger "$AR" --parent sup-1 --rows "$rows")"
check "a parent without rows is exit 2" 2 'rows' "$SR" "$r4" --parent sup-1
check "unreadable rows are exit 2" 2 'unreadable|rows' "$SR" "$r4" --parent sup-1 --rows "$tmp/absent.json"

# the report's Safe to remove section lists the SAFE worktrees, and "none" otherwise; never a REMOVE
out="$(report "$r4" "$r2" --no-live-sessions --supervisor-ledger "$AR")"
want_lines "the report lists the SAFE worktree" "- $r4" "$(section 'Safe to remove' <<<"$out")"
out="$(report "$r4" "$r2")"
want_lines "no session list: the report says none" "- none" "$(section 'Safe to remove' <<<"$out")"
out="$(report "$r4" --live "$r4")"
want_lines "a live worktree is not in the report's list" "- none" "$(section 'Safe to remove' <<<"$out")"
out="$(report "$r4" "$r1" --no-live-sessions --parent sup-1 --rows "$rows")"
want_lines "the report's list keeps to the own parent" "- $r4" "$(section 'Safe to remove' <<<"$out")"

# FR-015: the rules file names the four scripts and the supervisor-ledger opt-in
RULES="$ROOT/.claude/skills/speckit-autopilot/rules/supervisor-close-out.md"
for w in verify-run.sh follow-up.sh close-out-report.sh safe-to-remove.sh 'Auto-remove' delete_worktree; do
  cases=$((cases + 1)); grep -qF -- "$w" "$RULES" 2>/dev/null || { failures=$((failures + 1)); echo "FAIL the rules file does not name $w"; }
done

echo "close-out: $cases cases, $failures failed"
[ "$failures" -eq 0 ]
