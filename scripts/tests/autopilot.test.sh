#!/usr/bin/env bash
# Drives the autopilot helper scripts in scripts/autopilot/ against throwaway repositories.
#
# Each case builds a bare `origin` and a clone in a temp dir. `gh` is a stub on PATH that answers
# from JSON fixtures, so no case touches GitHub:
#   pr-<n>.json          what `gh pr view <n>` sees; pr-<n>.json.<k> replaces it from the k-th view on
#   pr-<n>.merged.json   becomes pr-<n>.json when `gh pr merge <n>` succeeds
#   merge-<n>.fail       `gh pr merge <n>` prints it and fails
#   runs.json            what `gh run list` sees

set -uo pipefail

ROOT="$(git rev-parse --show-toplevel)"
S="$ROOT/scripts/autopilot"
failures=0
cases=0
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

mkdir -p "$tmp/bin"
cat > "$tmp/bin/gh" <<'STUB'
#!/usr/bin/env bash
F="$GH_FIXTURES"
q() { local next=0; while [ $# -gt 0 ]; do [ "$1" = -q ] && { shift; echo "$1"; return; }; shift; done; echo .; }
case "$1 $2" in
  "pr view")
    n="$3"; c=$(( $(cat "$F/count-$n" 2>/dev/null || echo 0) + 1 )); echo "$c" > "$F/count-$n"
    f="$F/pr-$n.json"
    for k in $(seq 1 "$c"); do [ -f "$F/pr-$n.json.$k" ] && f="$F/pr-$n.json.$k"; done
    [ -f "$f" ] || { echo "no pull requests found" >&2; exit 1; }
    jq -r "$(q "$@")" "$f" ;;
  "pr merge")
    n="$3"
    if [ -f "$F/merge-$n.fail" ]; then cat "$F/merge-$n.fail" >&2; exit 1; fi
    for k in "$F/pr-$n.json".*; do [ -f "$k" ] && rm -f "$k"; done
    cp "$F/pr-$n.merged.json" "$F/pr-$n.json" ;;
  "run list") jq -r "$(q "$@")" "$F/runs.json" ;;
  "run view") echo "error[E0308]: mismatched types" ;;
  *) echo "gh stub: unhandled: $*" >&2; exit 2 ;;
esac
STUB
chmod +x "$tmp/bin/gh"
export PATH="$tmp/bin:$PATH"

# Fresh origin + clone on branch `wt`; echoes the clone path. GH_FIXTURES is per case.
new_repo() {
  local d; d="$(mktemp -d "$tmp/case.XXXX")"
  git init -q --bare -b main "$d/origin.git"
  git clone -q "$d/origin.git" "$d/wt" 2>/dev/null
  (
    cd "$d/wt"
    git config user.email t@example.com; git config user.name t; git config commit.gpgsign false
    git switch -q -c main 2>/dev/null || true
    echo seed > seed.txt; git add -A; git commit -qm seed; git push -q origin main
    git switch -q -c wt
  )
  mkdir -p "$d/fx"
  echo "$d"
}

ledger() {  # ledger <file> <branch> <phase> <pr...>
  local f="$1" b="$2" ph="$3"; shift 3
  mkdir -p "$(dirname "$f")"
  {
    echo "# Autopilot ledger"
    echo "- **Worktree branch**: $b"
    echo "- **Phase**: $ph"
    echo "- **Next step**: open PR 2"
    echo; echo "## Pull requests"; echo "| PR | Purpose |"
    for p in "$@"; do echo "| #$p | x |"; done
    echo; echo "## Decisions"; echo "| D1 | see #999 |"
    echo; echo "## Open escalation"; echo; echo "None."
  } > "$f"
}

pr_json() { # pr_json <fx> <n> <state> [ci-status ci-conclusion] [mergeable]
  local fx="$1" n="$2" st="$3" s="${4:-}" c="${5:-}" m="${6:-MERGEABLE}" roll='[]'
  [ -n "$s" ] && roll="[{\"name\":\"ci complete\",\"status\":\"$s\",\"conclusion\":\"$c\",\"detailsUrl\":\"https://x/actions/runs/77/job/1\"}]"
  [ "$c" = FAILURE ] && roll="[{\"name\":\"build + test\",\"status\":\"COMPLETED\",\"conclusion\":\"FAILURE\",\"detailsUrl\":\"https://x/actions/runs/77/job/2\"},${roll#[}"
  printf '{"state":"%s","mergeable":"%s","headRefName":"wt","headRefOid":"h1","mergeCommit":{"oid":"abc123"},"statusCheckRollup":%s}\n' \
    "$st" "$m" "$roll"
}

# check <name> <want-exit> <want-output-regex> <cmd...>   (runs in $PWD)
check() {
  local name="$1" want="$2" re="$3"; shift 3
  cases=$((cases + 1))
  local out code
  out="$("$@" 2>&1)"; code=$?
  if [ "$code" != "$want" ] || ! grep -qE -- "$re" <<<"$out"; then
    failures=$((failures + 1))
    echo "FAIL $name: exit $code (want $want), output:"; sed 's/^/    /' <<<"$out"
  fi
}

# --- branch-start.sh ---------------------------------------------------------------------------
d="$(new_repo)"; cd "$d/wt"
check "branch-start resets when nothing is unmerged" 0 '^RESET wt' "$S/branch-start.sh"

d="$(new_repo)"; cd "$d/wt"
echo a > a.txt; git add -A; git commit -qm "clarify round"
(cd "$d" && git clone -q origin.git other && cd other && git config user.email t@e && git config user.name t \
  && echo b > b.txt && git add -A && git commit -qm other && git push -q origin main)
check "branch-start rebases unmerged work" 0 '^REBASED 1 ' "$S/branch-start.sh"
check "branch-start kept the unmerged commit" 0 'clarify round' git log --oneline -1
check "branch-start is on top of origin/main" 0 '^yes$' bash -c 'git merge-base --is-ancestor origin/main HEAD && echo yes'

d="$(new_repo)"; cd "$d/wt"
echo x > dirty.txt
check "branch-start refuses a dirty tree" 1 '^DIRTY' "$S/branch-start.sh"

d="$(new_repo)"; cd "$d/wt"
echo mine > seed.txt; git commit -qam mine
(cd "$d" && git clone -q origin.git other && cd other && git config user.email t@e && git config user.name t \
  && echo theirs > seed.txt && git commit -qam theirs && git push -q origin main)
check "branch-start reports a conflict" 1 '^CONFLICT' "$S/branch-start.sh"
git rebase --abort 2>/dev/null

d="$(new_repo)"; cd "$d/wt"; git switch -q --detach
check "branch-start refuses a detached HEAD" 1 '^DETACHED' "$S/branch-start.sh"

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"
pr_json "$d/fx" 9 OPEN > "$d/fx/pr-9.json"
check "branch-start refuses while the previous PR is open" 1 '^PREVIOUS-PR-OPEN: #9 is OPEN' "$S/branch-start.sh" 9
pr_json "$d/fx" 9 MERGED > "$d/fx/pr-9.json"
check "branch-start proceeds once the previous PR merged" 0 '^RESET' "$S/branch-start.sh" 9

# --- resume.sh ---------------------------------------------------------------------------------
d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"
ledger specs/042-x/autopilot.md wt 3-design 10 11
pr_json "$d/fx" 10 MERGED > "$d/fx/pr-10.json"; pr_json "$d/fx" 11 OPEN > "$d/fx/pr-11.json"
check "resume finds the one ledger" 0 '^LEDGER specs/042-x/autopilot.md' "$S/resume.sh"
check "resume reports GitHub's PR state" 0 '^PR #11 OPEN' "$S/resume.sh"
check "resume ignores PR refs outside the PR tables" 0 '^PHASE 3-design' \
  bash -c "out=\$('$S/resume.sh'); ! grep -q '#999' <<<\"\$out\" && echo \"\$out\""

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"
ledger specs/042-x/autopilot.md other-branch 3-design
check "resume finds nothing for another branch" 2 '^NONE' "$S/resume.sh"

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"
ledger specs/042-x/autopilot.md wt 2-clarify; ledger specs/043-y/bugs/BUG-1.autopilot.md wt 4-milestones
check "resume reports several ledgers" 3 'BUG-1.autopilot.md' "$S/resume.sh"

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"
ledger specs/042-x/autopilot.md wt done; git add -A; git commit -qm "record the run"
check "resume spots an unmerged record PR" 4 '^RECORD-PR-PENDING' "$S/resume.sh"

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"
git switch -q main; ledger specs/042-x/autopilot.md wt 4-milestones; git add -A; git commit -qm ledger
git push -q origin main; git switch -q wt
check "resume falls back to a ledger only on origin/main" 0 '^LEDGER-ON-MAIN specs/042-x/autopilot.md \(phase 4-milestones\)' "$S/resume.sh"

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"
git switch -q main; ledger specs/042-x/autopilot.md wt-longer 4-milestones; git add -A; git commit -qm ledger
git push -q origin main; git switch -q wt
check "resume does not match another branch by prefix" 2 '^NONE' "$S/resume.sh"

# --- handoff-check.sh --------------------------------------------------------------------------
d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"
ledger "$d/ledger.md" wt done 10
pr_json "$d/fx" 10 MERGED > "$d/fx/pr-10.json"
check "handoff passes when all is merged" 0 '^OK$' "$S/handoff-check.sh" "$d/ledger.md"
pr_json "$d/fx" 12 OPEN > "$d/fx/pr-12.json"
check "handoff lists an open extra PR" 1 'PR #12 is OPEN' "$S/handoff-check.sh" "$d/ledger.md" 12
echo z > z.txt; git add -A; git commit -qm unpushed
check "handoff fails on a missing ledger" 1 'ledger nope.md not found' "$S/handoff-check.sh" nope.md
check "handoff lists an unmerged commit" 1 '1 commit\(s\) not on origin/main' "$S/handoff-check.sh" "$d/ledger.md"

# --- wait-merge.sh -----------------------------------------------------------------------------
export AUTOPILOT_POLL=0 AUTOPILOT_CHECKLESS_AFTER=0
wm() { d="$(new_repo)"; export GH_FIXTURES="$d/fx" AUTOPILOT_LOG_DIR="$d"; }

wm; pr_json "$d/fx" 5 MERGED > "$d/fx/pr-5.json"
check "wait-merge: already merged" 0 '^MERGED 5 abc123' "$S/wait-merge.sh" 5

wm; pr_json "$d/fx" 5 OPEN IN_PROGRESS "" > "$d/fx/pr-5.json"
pr_json "$d/fx" 5 OPEN COMPLETED SUCCESS > "$d/fx/pr-5.json.8"
pr_json "$d/fx" 5 MERGED COMPLETED SUCCESS > "$d/fx/pr-5.merged.json"
check "wait-merge: waits, then merges on green" 0 '^MERGED 5 abc123' "$S/wait-merge.sh" 5

wm; pr_json "$d/fx" 5 OPEN > "$d/fx/pr-5.json"
jq '.statusCheckRollup = [{"name":"build + test","status":"IN_PROGRESS","conclusion":""}]' "$d/fx/pr-5.json" > "$d/fx/x" \
  && mv "$d/fx/x" "$d/fx/pr-5.json"
pr_json "$d/fx" 5 OPEN COMPLETED SUCCESS > "$d/fx/pr-5.json.9"
check "wait-merge: other checks running is not checkless" 0 '^GREEN 5' "$S/wait-merge.sh" 5 --no-merge

wm; pr_json "$d/fx" 5 OPEN COMPLETED SUCCESS > "$d/fx/pr-5.json"
check "wait-merge: --no-merge stops at green" 0 '^GREEN 5' "$S/wait-merge.sh" 5 --no-merge

wm; pr_json "$d/fx" 5 OPEN COMPLETED FAILURE > "$d/fx/pr-5.json"
check "wait-merge: red names the failing check" 1 'build \+ test' "$S/wait-merge.sh" 5
check "wait-merge: red saves the failing log" 0 'E0308' cat "$d/pr-5-failed.log"

wm; pr_json "$d/fx" 5 OPEN "" "" CONFLICTING > "$d/fx/pr-5.json"
check "wait-merge: checkless from a conflict" 3 '^CHECKLESS 5 conflicting' "$S/wait-merge.sh" 5

wm; pr_json "$d/fx" 5 OPEN > "$d/fx/pr-5.json"
echo '[{"databaseId":91,"conclusion":"action_required","headSha":"h1"}]' > "$d/fx/runs.json"
check "wait-merge: checkless awaiting approval" 3 '^CHECKLESS 5 action_required 91' "$S/wait-merge.sh" 5

wm; pr_json "$d/fx" 5 OPEN > "$d/fx/pr-5.json"; echo '[]' > "$d/fx/runs.json"
check "wait-merge: checkless with no run" 3 '^CHECKLESS 5 no-run' "$S/wait-merge.sh" 5

wm; pr_json "$d/fx" 5 OPEN COMPLETED SUCCESS > "$d/fx/pr-5.json"
echo "This branch can't be rebased" > "$d/fx/merge-5.fail"
check "wait-merge: reports a refused merge" 4 "^MERGE-FAILED 5 This branch can't be rebased" "$S/wait-merge.sh" 5

wm; pr_json "$d/fx" 5 OPEN COMPLETED SUCCESS > "$d/fx/pr-5.json"
jq '.statusCheckRollup = [
  {"name":"ci complete","status":"COMPLETED","conclusion":"FAILURE","startedAt":"2026-01-01T10:00:00Z","detailsUrl":"https://x/actions/runs/70/job/1"},
  {"name":"ci complete","status":"COMPLETED","conclusion":"SUCCESS","startedAt":"2026-01-01T10:05:00Z","detailsUrl":"https://x/actions/runs/71/job/1"},
  {"context":"some/status","state":"SUCCESS"}]' "$d/fx/pr-5.json" > "$d/fx/x" && mv "$d/fx/x" "$d/fx/pr-5.json"
check "wait-merge: the newest ci complete wins over a superseded one" 0 '^GREEN 5' "$S/wait-merge.sh" 5 --no-merge

wm; pr_json "$d/fx" 5 OPEN COMPLETED SUCCESS > "$d/fx/pr-5.json"
jq '.statusCheckRollup = [
  {"name":"ci complete","status":"COMPLETED","conclusion":"SUCCESS","startedAt":"2026-01-01T10:00:00Z","detailsUrl":"https://x/actions/runs/70/job/1"},
  {"name":"ci complete","status":"QUEUED","conclusion":"","startedAt":"0001-01-01T00:00:00Z","detailsUrl":"https://x/actions/runs/71/job/1"}]' \
  "$d/fx/pr-5.json" > "$d/fx/x" && mv "$d/fx/x" "$d/fx/pr-5.json"
check "wait-merge: a queued re-run is not overtaken by an older green" 6 '^TIMEOUT 5 ' \
  env AUTOPILOT_MAX_WAIT=0 "$S/wait-merge.sh" 5 --no-merge

wm; pr_json "$d/fx" 5 OPEN COMPLETED FAILURE > "$d/fx/pr-5.json"
jq '.statusCheckRollup += [{"context":"ext/lint","state":"ERROR","targetUrl":"https://ext/1"}]' "$d/fx/pr-5.json" > "$d/fx/x" && mv "$d/fx/x" "$d/fx/pr-5.json"
check "wait-merge: red lists a failed commit status" 1 '^- ext/lint ERROR https://ext/1' "$S/wait-merge.sh" 5

wm; pr_json "$d/fx" 5 OPEN COMPLETED FAILURE > "$d/fx/pr-5.json"
jq '.statusCheckRollup[0].conclusion = "CANCELLED"' "$d/fx/pr-5.json" > "$d/fx/x" && mv "$d/fx/x" "$d/fx/pr-5.json"
check "wait-merge: red names a cancelled check" 1 'build \+ test CANCELLED' "$S/wait-merge.sh" 5
check "wait-merge: red finds the cancelled check's run" 1 '^RED 5 77 ' "$S/wait-merge.sh" 5

wm; pr_json "$d/fx" 5 OPEN IN_PROGRESS "" > "$d/fx/pr-5.json"; pr_json "$d/fx" 5 CLOSED > "$d/fx/pr-5.json.6"
check "wait-merge: a PR closed while waiting" 5 '^CLOSED 5' "$S/wait-merge.sh" 5

wm; pr_json "$d/fx" 5 OPEN IN_PROGRESS "" > "$d/fx/pr-5.json"
check "wait-merge: gives up after the max wait" 6 '^TIMEOUT 5 ' \
  env AUTOPILOT_MAX_WAIT=0 "$S/wait-merge.sh" 5

wm; pr_json "$d/fx" 5 OPEN > "$d/fx/pr-5.json"
echo '[{"databaseId":91,"conclusion":"action_required","headSha":"old"}]' > "$d/fx/runs.json"
check "wait-merge: ignores approvals pending on an older head" 3 '^CHECKLESS 5 no-run' "$S/wait-merge.sh" 5

# --- brief.py ---------------------------------------------------------------------------------
d="$(mktemp -d "$tmp/brief.XXXX")"; mkdir -p "$d/f"
cat > "$d/f/spec.md" <<'MD'
# Spec
## User Scenarios
### User Story 1 - Open a link (Priority: P1)
Scenario: click opens it.
### User Story 2 - Copy a link (Priority: P2)
Scenario: menu copies it.
## Requirements
- **FR-001**: MUST open links,
  across wrapped rows.
- **FR-002**: MUST copy links.
- **FR-003**: MUST show a hover.
- **FR-003a**: MUST underline on hover.
- **FR-004**: MUST fade.
- **FR-005**: MUST restore.
- **SC-001**: One gesture.
### User Story 3 - Hover a link (Priority: P3)
Scenario: hover underlines it.
MD
cat > "$d/f/tasks.md" <<'MD'
# Tasks
## Phase 3: US1
- [X] T001 [US1] Test opening
  - sub-bullet of T001
- [ ] T002 [US1] Implement opening
- [ ] T003 [US2] Copy
- [ ] T004 Hover test
- [ ] T005 Hover
- [ ] T006 Fade
- [ ] T007 Restore
- [ ] T007 Restore (duplicate id)
## Milestones
### M1 — Open links
- **Tasks**: T001–T002
- **Satisfies**: FR-001, SC-001
### M2 — Copy
- **Tasks**: T003
- **Satisfies**: FR-002
### M3 — Hover
- **Tasks**: T004,
  T005
- **Satisfies**: FR-001–FR-003,
  FR-003a, US3
### M4: Fade
- **Tasks**: T006-T007
- **Satisfies**: FR-003a–FR-005
## Dependencies
- T004 and T005 together
MD
B="$S/brief.py"
check "brief: milestone block" 0 '^### M1 — Open links' "$B" milestone "$d/f" M1
check "brief: milestone tasks with sub-bullets" 0 'sub-bullet of T001' "$B" milestone "$d/f" M1
check "brief: milestone counts its tasks" 0 '^## Tasks of M1 \(2 of 2\)' "$B" milestone "$d/f" M1
check "brief: requirement with its wrapped line" 0 'across wrapped rows' "$B" milestone "$d/f" M1
check "brief: the story the tasks tag" 0 'click opens it' "$B" milestone "$d/f" M1
check "brief: leaves out other milestones' tasks" 0 '^ok$' bash -c "! '$B' milestone '$d/f' M1 | grep -qE '[]] T003|[*]FR-002|menu copies' && echo ok"
check "brief: the leave-out check can see a task line" 0 '^ok$' bash -c "'$B' milestone '$d/f' M2 | grep -qE '[]] T003' && echo ok"
check "brief: wrapped Tasks field" 0 '^## Tasks of M3 \(2 of 2\)' "$B" milestone "$d/f" M3
check "brief: requirement range and suffixed ID" 0 '^ok$' bash -c "out=\$('$B' milestone '$d/f' M3); grep -q '^- [*][*]FR-002[*][*]' <<<\"\$out\" && grep -q '^- [*][*]FR-003a[*][*]' <<<\"\$out\" && ! grep -q 'Not found' <<<\"\$out\" && echo ok"
check "brief: hyphen range, colon heading, suffixed range start" 0 '^ok$' bash -c "out=\$('$B' milestone '$d/f' M4); grep -q '^## Tasks of M4 (2 of 2)' <<<\"\$out\" && grep -q '^- [*][*]FR-004[*][*]' <<<\"\$out\" && grep -q '^- [*][*]FR-005[*][*]' <<<\"\$out\" && echo ok"
check "brief: flags a task ID found twice" 0 'more than once.*T007' "$B" milestone "$d/f" M4
check "brief: story named only in Satisfies" 0 'hover underlines it' "$B" milestone "$d/f" M3
check "brief: non-checkbox bullets are not tasks" 0 '^ok$' bash -c "! '$B' milestone '$d/f' M3 | grep -q 'together' && echo ok"
check "brief: lists other milestones' task ranges" 0 '^- M1 — Open links: T001–T002' "$B" milestone "$d/f" M3
check "brief: unknown milestone fails" 1 'no milestone M9' "$B" milestone "$d/f" M9
check "brief: section stops at a sibling heading" 0 '^ok$' bash -c "out=\$('$B' section '$d/f/spec.md' 'User Story 1'); grep -q 'click opens' <<<\"\$out\" && ! grep -q 'Story 2' <<<\"\$out\" && echo ok"
check "brief: items by ID" 0 '^- \*\*SC-001' "$B" items "$d/f/spec.md" FR-002 SC-001
check "brief: missing item fails" 1 'not found.*FR-009' "$B" items "$d/f/spec.md" FR-009

echo "autopilot: $cases case(s), $failures failure(s)"
[ "$failures" -eq 0 ]
