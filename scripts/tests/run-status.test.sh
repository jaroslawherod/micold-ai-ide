#!/usr/bin/env bash
# Drives scripts/autopilot/run-status.sh against fixture ledgers: one case per run state, in text and
# JSON, plus usage errors, precedence, the escalation shape and read-only behaviour. `gh` and
# pr-state.sh are stubs (AUTOPILOT_PR_STATE); every assertion on a result reads only the --json
# output (SC-002), except the text-form cases.
set -uo pipefail
# shellcheck source=lib/autopilot-fixtures.sh
. "$(dirname "$0")/lib/autopilot-fixtures.sh"
unset AUTOPILOT_SUPERVISED

export AUTOPILOT_NOW=2000000000
cat > "$tmp/prstate.sh" <<'STUB'
#!/usr/bin/env bash
# pr-state.sh stand-in: prints $GH_FIXTURES/prstate-<n>, exit 1 when it says STUCK.
f="$GH_FIXTURES/prstate-$1"
if [ -f "$f" ]; then cat "$f"; grep -qE '(^| )STUCK( |$)' "$f" && exit 1; exit 0; fi
echo "PR $1 STATE OPEN DWELL 10 LIMIT 3600 OK ACTION none"
STUB
chmod +x "$tmp/prstate.sh"
export AUTOPILOT_PR_STATE="$tmp/prstate.sh"
RS="$S/run-status.sh"
STATES="dead-no-ledger done blocked asking stuck waiting-ci running"

# --- helpers -----------------------------------------------------------------------------------
fresh() { d="$(new_repo)"; cd "$d/wt" || exit 1; export GH_FIXTURES="$d/fx"; ESC="None."; unset AUTOPILOT_SUPERVISED; }
# rs_ledger <phase> <pr...>: specs/042-x/autopilot.md for branch wt; $ESC is its Open escalation.
rs_ledger() {
  local ph="$1"; shift; local f="${LEDGER:-specs/042-x/autopilot.md}"
  mkdir -p "$(dirname "$f")"
  {
    echo "# Autopilot ledger — #42 x"
    echo "- **Kind**: feature"; echo "- **Issue**: #42"
    echo "- **Worktree branch**: ${BRANCH:-wt}"
    echo "- **Phase**: $ph"; echo "- **Next step**: open PR 2"
    echo; echo "## Pull requests"; echo "| PR | Purpose | Status |"
    for p in "$@"; do echo "| #$p | x | merged |"; done
    echo; echo "## Decisions"; echo "| D1 | see #999 |"
    echo; echo "## Open escalation"; echo; printf '%s\n' "$ESC"
    echo; echo "## Follow-ups not done"; echo; echo "None."
  } > "$f"
}
commit_ledger() { git add -A; GIT_COMMITTER_DATE="1700000000 +0000" git commit -qm "record the run"; }
# jc <name> <jq-filter> <regex>: the filter's output for the current worktree's JSON status.
jc() { check "$1" 0 "$3" bash -c '"$0" --json "$1" | jq -r "$2"' "$RS" "$PWD" "$2"; }
tc() { check "$1" 0 "$2" bash -c '"$0" "$1"' "$RS" "$PWD"; }
# state_case <state>: the current fixture reports <state> in JSON and text; records the coverage.
state_case() {
  local want="$1" f0=$failures
  jc "json: $want" '.state' "^$want\$"
  jc "json: $want is one of the seven" ".state | IN($(printf '"%s",' $STATES | sed 's/,$//'))" '^true$'
  [ "$failures" = "$f0" ] && : > "$tmp/cov-json-$want"
  f0=$failures
  check "text: $want" 0 "^STATE $want\$" bash -c '"$0" "$1" | head -1' "$RS" "$PWD"
  [ "$failures" = "$f0" ] && : > "$tmp/cov-text-$want"
}
SINGLE='- **Category**: 1 Product or scope decision
- **Question**: Which store?
- **Options**:
  1. SQLite (Recommended) — local and simple
  2. Postgres — remote
- **Recommended**: SQLite — fewer moving parts
- **Evidence**: specs/042-x/spec.md#storage
- **Banner**: sent 2026-11-10'

# --- usage -------------------------------------------------------------------------------------
fresh
check "a path outside any worktree is exit 2" 2 'not a git worktree' "$RS" "$tmp"
check "an unknown option is exit 2" 2 'unknown option' "$RS" --nope
rs_ledger 4-milestones; LEDGER=specs/043-y/bugs/BUG-1.autopilot.md rs_ledger 2-clarify
check "two open ledgers are exit 3 and name the feature ledger" 3 'specs/042-x/autopilot\.md' "$RS" --json .
check "two open ledgers are exit 3 and name the bug ledger" 3 'specs/043-y/bugs/BUG-1\.autopilot\.md' "$RS" --json .
check "two open ledgers list the first" 3 'specs/042-x/autopilot.md' "$RS" .
check "two open ledgers list the second" 3 'specs/043-y/bugs/BUG-1.autopilot.md' "$RS" .
fresh
rs_ledger done; LEDGER=specs/043-y/autopilot-77.md rs_ledger 4-milestones
jc "a done ledger and an open one resolve to the open one" '.ledger' '^specs/043-y/autopilot-77.md$'
check "the worktree argument is honoured from another directory" 0 '^specs/043-y/autopilot-77.md$' \
  bash -c 'cd "$2" && "$0" --json "$1" | jq -r .ledger' "$RS" "$d/wt" "$tmp"

# --- dead-no-ledger ----------------------------------------------------------------------------
fresh
state_case dead-no-ledger
jc "dead-no-ledger: no ledger, no PRs" '[.ledger, (.prs | length), .issue, .phase] | map(tostring) | join("|")' '^null\|0\|\|$'
fresh; ESC="$SINGLE"; rs_ledger 3-design; commit_ledger; git push -q origin HEAD:main; git reset -q --hard HEAD~1
state_case dead-no-ledger
jc "dead-no-ledger: a ledger only on origin/main says branch-start.sh is next" '.notes | join(" ")' 'branch-start\.sh is next'

# --- done --------------------------------------------------------------------------------------
fresh; rs_ledger done 10; commit_ledger; git push -q origin HEAD:main
pr_json "$d/fx" 10 MERGED > "$d/fx/pr-10.json"
state_case done
jc "done: the PR is listed as GitHub reports it" '.prs[0] | "\(.number) \(.state)"' '^10 MERGED$'
fresh; rs_ledger done 10; commit_ledger; git push -q origin HEAD:main
pr_json "$d/fx" 10 OPEN > "$d/fx/pr-10.json"
jc "done is refused when the handoff check fails" '.state' '^waiting-ci$'
jc "done is refused: the handoff lines are in notes" '.notes | join(" ")' 'PR #10 is OPEN'
fresh; rs_ledger done 10; commit_ledger; pr_json "$d/fx" 10 MERGED > "$d/fx/pr-10.json"
echo '[{"number":470}]' > "$d/fx/pr-list.json"
jc "done with an unmerged final PR that is open is waiting-ci" '.state' '^waiting-ci$'
jc "done with an unmerged final PR lists it" '[.prs[] | "\(.number) \(.state)"] | join(",")' '470 OPEN'
jc "done with an unmerged final PR notes the commits" '.notes | join(" ")' 'not on origin/main'
rm "$d/fx/pr-list.json"
jc "done with an unmerged final PR that is not open is running" '.state' '^running$'
fresh; rs_ledger done 10; commit_ledger; git push -q origin HEAD:main; pr_json "$d/fx" 10 MERGED > "$d/fx/pr-10.json"; echo x > stray.txt
jc "done with an uncommitted tree is running" '.state' '^running$'
fresh; rs_ledger done 10; commit_ledger; git push -q origin HEAD:main
jc "gh unavailable: the PR is UNKNOWN" '.prs[0].state' '^UNKNOWN$'
jc "gh unavailable: never done" '.state' '^running$'

# --- running -----------------------------------------------------------------------------------
fresh; rs_ledger 4-milestones
state_case running
jc "running: fields from the ledger" '[.issue, .flow, .phase, .next_step, .ledger] | join("|")' '^#42\|feature\|4-milestones\|open PR 2\|specs/042-x/autopilot.md$'
jc "running: no escalation" '.escalation' '^null$'
fresh; rs_ledger 4-milestones 10; pr_json "$d/fx" 10 MERGED > "$d/fx/pr-10.json"
echo "PR 10 STATE MERGED DWELL 9999 LIMIT 1 STUCK ACTION x" > "$d/fx/prstate-10"
jc "a run with no open PR is never stuck" '.state' '^running$'

# --- waiting-ci --------------------------------------------------------------------------------
fresh; rs_ledger 4-milestones 10; pr_json "$d/fx" 10 OPEN PENDING "" > "$d/fx/pr-10.json"
state_case waiting-ci
jc "waiting-ci: the open PR is listed with its GitHub state" '.prs[0] | "\(.number) \(.state)"' '^10 OPEN$'
jc "waiting-ci: dwell from pr-state" '[.dwell_source, .since, .seconds_in_state] | join(" ")' '^pr-state 1999999990 10$'
jc "waiting-ci: underlying equals state" '.underlying_state' '^waiting-ci$'
jc "GitHub wins over the ledger (ledger says merged)" '.prs[0].state' '^OPEN$'
check "(the fixture ledger says merged)" 0 '\| merged \|?$' grep '#10' specs/042-x/autopilot.md

# --- asking and blocked ------------------------------------------------------------------------
fresh; ESC="$SINGLE"; rs_ledger 3-design 10; pr_json "$d/fx" 10 OPEN > "$d/fx/pr-10.json"
state_case asking
jc "asking: the category" '.escalation.category' '^1 Product or scope decision$'
jc "asking: the banner" '.escalation.banner' '^sent 2026-11-10$'
jc "asking: the question" '.escalation.questions[0].question' '^Which store\?$'
jc "asking: the options are separate" '.escalation.questions[0].options | map("\(.label)=\(.description)") | join("|")' '^SQLite=local and simple\|Postgres=remote$'
jc "asking: the recommendation" '.escalation.questions[0].recommendation' '^SQLite$'
jc "asking: the evidence" '.escalation.questions[0].evidence' '^specs/042-x/spec.md#storage$'
jc "asking: the question number" '.escalation.questions[0].number' '^1$'
jc "asking beats stuck" '.state' '^asking$'
echo "PR 10 STATE OPEN DWELL 9000 LIMIT 3600 STUCK ACTION escalate" > "$d/fx/prstate-10"
jc "asking beats stuck even past the dwell limit" '.state' '^asking$'
tc "text: the question, options, recommendation and evidence" 'question 1: Which store\?'
tc "text: an option" 'option 1\.2: Postgres — remote'
tc "text: the recommendation" 'recommended 1: SQLite'
tc "text: the evidence" 'evidence 1: specs/042-x/spec.md#storage'
tc "text: the PR line" 'PR #10 OPEN'
fresh; ESC='- **Category**: 5 Blocked
- **Question**: blocked by work outside my flow: the CI runner is down
- **Evidence**: gh run list'; rs_ledger 4-milestones
state_case blocked
jc "blocked: the question" '.escalation.questions[0].question' 'blocked by work outside my flow'
jc "blocked: absent fields are empty, not dropped" '.escalation.questions[0] | [.options, .recommendation] | tojson' '^\[\[\],""\]$'
fresh; ESC='- **Question**: only a question'; rs_ledger 3-design
jc "a partial escalation keeps every field" '.escalation.questions[0] | keys | join(",")' '^evidence,number,options,question,recommendation$'
jc "a partial escalation has empty category and banner" '[.escalation.category, .escalation.banner] | join("|")' '^\|$'
fresh; ESC='Should the picker be modal?'; rs_ledger 3-design
jc "a free-text escalation gives its first line as the question" '.escalation.questions[0].question' '^Should the picker be modal\?$'
fresh; ESC='- **Category**: 1 Product or scope decision
- **Banner**: sent 2026-11-10

### Q1
- **Question**: First?
- **Options**:
  1. Yes (Recommended) — go
  2. No — stop
- **Recommended**: Yes — cheaper
- **Evidence**: a.md

### Q2
- **Question**: Second?'; rs_ledger 3-design
jc "a batch has one element per question" '.escalation.questions | map(.number) | join(",")' '^1,2$'
jc "a batch keeps each question's options apart" '.escalation.questions | map(.options | length) | join(",")' '^2,0$'
jc "a batch keeps each recommendation apart" '.escalation.questions | map(.recommendation) | join("|")' '^Yes\|$'
jc "a batch keeps category and banner" '[.escalation.category, .escalation.banner] | join("|")' '^1 Product or scope decision\|sent 2026-11-10$'
fresh; ESC="$SINGLE"; rs_ledger 3-design; ESC="None."; LEDGER=specs/050-other/autopilot.md BRANCH=other rs_ledger 4-milestones
jc "concurrent runs: asking about one does not touch the other" '.ledger' '^specs/042-x/autopilot.md$'
git switch -q -c other
jc "concurrent runs: the other branch sees its own ledger" '[.state, .ledger] | join(" ")' '^running specs/050-other/autopilot.md$'
git switch -q wt

fresh; ESC='```
'"$SINGLE"'
```'; rs_ledger 3-design
jc "a fenced escalation keeps its last field exact" '.escalation.banner' '^sent 2026-11-10$'
jc "a fenced escalation keeps its evidence exact" '.escalation.questions[0].evidence' '^specs/042-x/spec.md#storage$'
fresh; ESC='- **Category**: 1 Product or scope decision'; rs_ledger 3-design
jc "an escalation with no question still has one question element" '.escalation.questions | length' '^1$'

# --- stuck -------------------------------------------------------------------------------------
fresh; rs_ledger 4-milestones 10; pr_json "$d/fx" 10 OPEN > "$d/fx/pr-10.json"
echo "PR 10 STATE OPEN DWELL 5000 LIMIT 3600 STUCK ACTION escalate" > "$d/fx/prstate-10"
state_case stuck
jc "stuck: the underlying state is kept" '.underlying_state' '^waiting-ci$'
jc "stuck: since is now minus the helper's dwell" '[.since, .seconds_in_state, .dwell_source] | join(" ")' '^1999995000 5000 pr-state$'
fresh; ESC='- **Question**: blocked by work outside my flow'; rs_ledger 4-milestones 10; pr_json "$d/fx" 10 OPEN > "$d/fx/pr-10.json"
echo "PR 10 STATE OPEN DWELL 5000 LIMIT 3600 STUCK ACTION escalate" > "$d/fx/prstate-10"
jc "blocked beats stuck" '.state' '^blocked$'
fresh; rs_ledger 4-milestones 10; pr_json "$d/fx" 10 OPEN > "$d/fx/pr-10.json"; commit_ledger
AUTOPILOT_PR_STATE=/nonexistent jc "no pr-state.sh: waiting-ci, never stuck" '.state' '^waiting-ci$'
check "no pr-state.sh: dwell from the ledger commit" 0 '^ledger 1700000000 300000000$' \
  bash -c 'AUTOPILOT_PR_STATE=/nonexistent "$0" --json "$1" | jq -r "[.dwell_source, .since, .seconds_in_state] | join(\" \")"' "$RS" "$PWD"
echo more >> specs/042-x/autopilot.md
check "no pr-state.sh, ledger uncommitted: dwell from the file mtime" 0 '^ledger true$' \
  bash -c 'AUTOPILOT_PR_STATE=/nonexistent "$0" --json "$1" | jq -r "[.dwell_source, (.since > 1700000000)] | join(\" \")"' "$RS" "$PWD"
check "the helper that fails is no stuck" 0 '^waiting-ci$' \
  bash -c 'printf "#!/bin/sh\nexit 2\n" > "$2"; chmod +x "$2"; AUTOPILOT_PR_STATE="$2" "$0" --json "$1" | jq -r .state' "$RS" "$PWD" "$tmp/failing.sh"

# --- precedence --------------------------------------------------------------------------------
fresh; ESC="$SINGLE"; rs_ledger done 10; commit_ledger; git push -q origin HEAD:main; pr_json "$d/fx" 10 MERGED > "$d/fx/pr-10.json"
jc "done beats asking" '.state' '^done$'
fresh; ESC='- **Question**: blocked by work outside my flow'; rs_ledger 3-design 10; pr_json "$d/fx" 10 OPEN > "$d/fx/pr-10.json"
jc "blocked beats waiting-ci" '.state' '^blocked$'
fresh; rs_ledger 3-design 10; pr_json "$d/fx" 10 OPEN > "$d/fx/pr-10.json"
echo "PR 10 STATE OPEN DWELL 5000 LIMIT 3600 STUCK ACTION escalate" > "$d/fx/prstate-10"
pr_json "$d/fx" 11 OPEN > "$d/fx/pr-11.json"; rs_ledger 3-design 10 11
jc "several open PRs: any STUCK makes the run stuck" '.state' '^stuck$'

# --- a `quick` ledger is the chore flow (data-model flow) --------------------------------------
fresh; rs_ledger 4-milestones; sed -i 's/^- \*\*Kind\*\*: feature/- **Kind**: quick/' specs/042-x/autopilot.md
jc "a quick Kind reports as the chore flow" '.flow' '^chore$'
fresh; rs_ledger 4-milestones
jc "a feature Kind is reported unchanged" '.flow' '^feature$'

# --- shape and safety --------------------------------------------------------------------------
fresh; ESC="$SINGLE"; rs_ledger 3-design 10; pr_json "$d/fx" 10 OPEN > "$d/fx/pr-10.json"
check "--json is one line" 0 '^1$' bash -c '"$0" --json "$1" | wc -l | tr -d " "' "$RS" "$PWD"
check "--json parses" 0 '^true$' bash -c '"$0" --json "$1" | jq -e "type == \"object\"" ' "$RS" "$PWD"
jc "--json carries every contract key" 'keys | join(",")' '^dwell_source,escalation,flow,issue,ledger,next_step,notes,phase,prs,seconds_in_state,since,state,supervised,underlying_state$'
jc "supervised is false by default" '.supervised' '^false$'
check "supervised is true from the environment" 0 '^true$' bash -c 'AUTOPILOT_SUPERVISED=1 "$0" --json "$1" | jq -r .supervised' "$RS" "$PWD"
sed -i.bak 's/^- \*\*Kind\*\*: feature/&\n- **Supervised**: yes/' specs/042-x/autopilot.md; rm specs/042-x/autopilot.md.bak
jc "supervised is true from the ledger field" '.supervised' '^true$'
fresh; rs_ledger 4-milestones 10; pr_json "$d/fx" 10 OPEN > "$d/fx/pr-10.json"; commit_ledger; echo y >> specs/042-x/autopilot.md
snap() { cksum specs/042-x/autopilot.md; git rev-parse HEAD; git status --porcelain; git branch --list; }
before="$(snap)"; "$RS" --json . >/dev/null; "$RS" . >/dev/null; after="$(snap)"
check "a call changes no ledger, commit or file" 0 '^same$' bash -c '[ "$0" = "$1" ] && echo same' "$before" "$after"

# --- every state has a passing text case and a passing JSON case (SC-001, FR-014) --------------
for s in $STATES; do
  check "coverage: $s has a passing JSON case" 0 '^ok$' bash -c 'test -f "$0" && echo ok' "$tmp/cov-json-$s"
  check "coverage: $s has a passing text case" 0 '^ok$' bash -c 'test -f "$0" && echo ok' "$tmp/cov-text-$s"
done

echo "run-status: $cases case(s), $failures failure(s)"
[ "$failures" -eq 0 ]
