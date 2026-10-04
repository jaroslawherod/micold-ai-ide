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
  "pr list") [ -f "$F/pr-list.json" ] && jq -r "$(q "$@")" "$F/pr-list.json" || echo "" ;;
  "run list") jq -r "$(q "$@")" "$F/runs.json" ;;
  "run view") echo "error[E0308]: mismatched types" ;;
  "api user") echo me ;;
  "issue view") jq -r "$(q "$@")" "$F/issue-$3.json" ;;
  "issue create") echo "$*" >> "$F/gh.log"; echo "https://github.com/o/r/issues/42" ;;
  "issue "*|"label create") echo "$*" >> "$F/gh.log" ;;
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
printf '\n## Handover\n\nNone.\n' >> specs/042-x/autopilot.md
check "resume prints no handover while it reads None" 0 '^ok$' \
  bash -c "out=\$('$S/resume.sh') && ! grep -q '^HANDOVER' <<<\"\$out\" && echo ok"
python3 -c "import sys; p=sys.argv[1]; s=open(p).read(); open(p,'w').write(s.replace('## Handover\n\nNone.', '## Handover\n\nM2: T010 done, next T011.'))" specs/042-x/autopilot.md
check "resume prints an open handover" 0 '^HANDOVER M2: T010 done, next T011\.$' "$S/resume.sh"

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"
ledger specs/042-x/autopilot.md other-branch 3-design
check "resume finds nothing for another branch" 2 '^NONE' "$S/resume.sh"

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"
ledger specs/042-x/autopilot.md wt 2-clarify; ledger specs/043-y/bugs/BUG-1.autopilot.md wt 4-milestones
check "resume reports several ledgers" 3 'BUG-1.autopilot.md' "$S/resume.sh"

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"
ledger specs/042-x/autopilot.md wt done; git add -A; git commit -qm "record the run"
check "resume spots an unmerged final PR" 4 '^FINAL-PR-PENDING specs/042-x/autopilot.md none$' "$S/resume.sh"

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"
ledger specs/042-x/autopilot.md wt done; git add -A; git commit -qm "record the run"
echo '[{"number": 470}]' > "$d/fx/pr-list.json"
check "resume names the open final PR" 4 '^FINAL-PR-PENDING specs/042-x/autopilot.md 470$' "$S/resume.sh"

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"
git switch -q main; ledger specs/042-x/autopilot.md wt 4-milestones; git add -A; git commit -qm ledger
git push -q origin main; git switch -q wt
check "resume falls back to a ledger only on origin/main" 0 '^LEDGER-ON-MAIN specs/042-x/autopilot.md \(phase 4-milestones\)' "$S/resume.sh"

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"
git switch -q main; ledger specs/042-x/autopilot.md wt-longer 4-milestones; git add -A; git commit -qm ledger
git push -q origin main; git switch -q wt
check "resume does not match another branch by prefix" 2 '^NONE' "$S/resume.sh"

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"
ledger specs/quick/2026-09-30-flaky.autopilot.md wt 4-milestones
check "resume finds a quick run's local ledger" 0 '^LEDGER specs/quick/2026-09-30-flaky\.autopilot\.md$' "$S/resume.sh"

d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"
ledger specs/quick/2026-09-30-flaky.autopilot.md wt done; echo fix > fix.txt; git add -A; git commit -qm fix
check "resume expects no final PR for a quick run" 2 '^NONE' "$S/resume.sh"

# --- scoped-gate.sh: picks the crates the branch changed, against the merge base -----------------
d="$(new_repo)"; cd "$d/wt"; G="$S/scoped-gate.sh"
check "scoped gate with nothing changed runs fmt only" 0 '^SCOPE none' "$G" --dry-run
check "scoped gate with nothing changed builds nothing" 0 '^RUN cargo fmt --all -- --check$' "$G" --dry-run
mkdir -p crates/micold-client/src crates/micold-daemon/src; echo x > crates/micold-client/src/a.rs; git add -A; git commit -qm client
echo y > crates/micold-daemon/src/b.rs  # untracked counts too
check "scoped gate names the changed crates" 0 '^SCOPE micold-client micold-daemon$' "$G" --dry-run
check "scoped gate tests only the changed crates" 0 'cargo test -p micold-client -p micold-daemon' "$G" --dry-run
check "scoped gate leaves the shell suites out" 0 '^ok$' bash -c "! '$G' --dry-run | grep -q test-scripts && echo ok"
mkdir -p scripts; echo z > scripts/c.sh
check "scoped gate runs the shell suites for scripts/" 0 '^RUN mise run test-scripts$' "$G" --dry-run
mkdir -p crates/micold-core/src; echo w > crates/micold-core/src/c.rs
check "scoped gate widens to the workspace for micold-core" 0 '^SCOPE workspace$' "$G" --dry-run
check "scoped gate runs CI's clippy order for the workspace" 0 'clippy -p micold-core --all-targets.*' "$G" --dry-run
check "scoped gate records no green gate" 0 '^ok$' bash -c "'$G' --dry-run >/dev/null; [ ! -e \"\$(git rev-parse --git-path autopilot-gate-ok)\" ] && echo ok"
cd "$ROOT"

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

# review-snapshot.sh: a snapshot takes uncommitted and untracked files, and leaves the index,
# HEAD and stash alone; it refuses a snapshot a rebase made stale. Run inside a real git worktree.
R="$S/review-snapshot.sh"
d="$(new_repo)"; cd "$d/wt"; git worktree add -q "$d/lwt" -b lwt; cd "$d/lwt"
echo one > seed.txt; echo new > added.txt; echo gone > doomed.txt; git add doomed.txt; git commit -qm doomed
head_before="$(git rev-parse HEAD)"
snap="$("$R")"
echo two > seed.txt; echo later > later.txt; rm doomed.txt
check "review-snapshot prints tree:head" 0 "^[0-9a-f]{40}:$head_before\$" echo "$snap"
check "review-snapshot diffs a changed file" 0 '^\+two$' "$R" diff "$snap"
check "review-snapshot diffs an untracked file added after" 0 '^\+later$' "$R" diff "$snap"
check "review-snapshot diffs a deleted file" 0 '^-gone$' "$R" diff "$snap"
check "review-snapshot leaves out what the snapshot already held" 0 '^ok$' \
  bash -c "out=\$('$R' diff '$snap') && ! grep -q '^+new\$' <<<\"\$out\" && echo ok"
check "review-snapshot leaves index, HEAD and stash alone" 0 '^ok$' bash -c \
  "[ -z \"\$(git diff --cached --name-only)\" ] && [ \"\$(git rev-parse HEAD)\" = $head_before ] \
   && [ -z \"\$(git stash list)\" ] && echo ok"
check "review-snapshot refuses an unknown snapshot" 2 'UNKNOWN-SNAPSHOT' bash -c "'$R' diff 0123:4567 2>&1"
git add -A; git commit -qm wip; git reset -q --hard HEAD~2; echo rebased > seed.txt; git commit -qam rebased
check "review-snapshot refuses a snapshot a rebase made stale" 2 'STALE-SNAPSHOT' bash -c "'$R' diff '$snap' 2>&1"
cd "$d/wt"

# autopilot-tokens.py: a request that re-writes most of a large context counts as a rebuild; the
# first request, a cached one and a small one do not.
d="$(new_repo)"
msg() { printf '{"type":"assistant","message":{"id":"%s","model":"claude-x","usage":{"input_tokens":1,"cache_creation_input_tokens":%s,"cache_read_input_tokens":%s,"output_tokens":1}}}\n' "$@"; }
{ msg m1 40000 0; msg m2 500 40000; msg m3 41000 0; msg m3 41000 0; msg m4 9000 1000; } > "$d/s.jsonl"
check "autopilot-tokens counts one cache rebuild" 0 '^\| orchestrator \(main session\) \| x \| 4 \| [^|]+\| [^|]+\| [^|]+\| 1 \|' \
  "$(dirname "$S")/autopilot-tokens.py" "$d/s.jsonl"
check "autopilot-tokens prices the rebuild's cache write" 0 '^\| orchestrator \(main session\) .*\| 51k \| 100% \|$' \
  "$(dirname "$S")/autopilot-tokens.py" "$d/s.jsonl"
# --rebuilds lists each one with the idle time before it and what the request before it called.
tmsg() { printf '{"type":"assistant","timestamp":"2026-01-01T00:%s:00Z","message":{"id":"%s","model":"claude-x","content":[%s],"usage":{"input_tokens":1,"cache_creation_input_tokens":%s,"cache_read_input_tokens":%s,"output_tokens":1}}}\n' "$@"; }
gate='{"type":"tool_use","name":"Bash","input":{"command":"cd /w && X=1 scripts/build-lock.sh mise run gate"}}'
{ tmsg 00 m1 '' 40000 0; tmsg 01 m2 "$gate" 500 40000; tmsg 01 m2 '{"type":"tool_use","name":"Edit","input":{}}' 500 40000
  tmsg 08 m3 '' 41000 0; tmsg 09 m4 '' 500 41000; tmsg 30 m5 '' 42000 0; } > "$d/r.jsonl"
check "autopilot-tokens --rebuilds names what a rebuild waited on" 0 '^\| orchestrator \(main session\) \| 41k \| 51k \| 7 \| Bash:build-lock.sh, Edit \|$' \
  "$(dirname "$S")/autopilot-tokens.py" --rebuilds "$d/r.jsonl"
check "autopilot-tokens --rebuilds marks a rebuild after a turn ended" 0 '^\| orchestrator \(main session\) \| 42k \| 52k \| 21 \| \(turn ended\) \|$' \
  "$(dirname "$S")/autopilot-tokens.py" --rebuilds "$d/r.jsonl"
check "autopilot-tokens --rebuilds totals them" 0 '2 rebuild\(s\), 104k cost_eq' \
  "$(dirname "$S")/autopilot-tokens.py" --rebuilds "$d/r.jsonl"

# context.py: finds a unit's transcript by its description (the newest one) under a fake HOME,
# and reports the last request's context against the cap.
d="$(new_repo)"; cd "$d/wt"
proj="$d/home/.claude/projects/$(pwd -P | sed 's/[^A-Za-z0-9]/-/g')"
mkdir -p "$proj/s1/subagents"
cmsg() { printf '{"type":"assistant","message":{"id":"%s","model":"claude-x","usage":{"input_tokens":%s,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":1}}}\n' "$@"; }
{ cmsg a 200000; cmsg b 90000; } > "$proj/s1/subagents/agent-old.jsonl"
echo '{"description":"Milestone M2 042"}' > "$proj/s1/subagents/agent-old.meta.json"
{ cmsg c 100000; cmsg d 160000; } > "$proj/s1/subagents/agent-new.jsonl"
echo '{"description":"Milestone M2 042"}' > "$proj/s1/subagents/agent-new.meta.json"
touch -d '1 hour ago' "$proj/s1/subagents/agent-old.jsonl"
cmsg e 42000 > "$proj/s1.jsonl"
C="$S/context.py"
check "context reports the newest unit's last request" 3 '^CONTEXT 160000 OVER 150000$' env HOME="$d/home" "$C" Milestone M2 042
check "context honours the cap setting" 0 '^CONTEXT 160000 OK 200000$' env HOME="$d/home" AUTOPILOT_CONTEXT_CAP=200000 "$C" "Milestone M2 042"
check "context reports the main session" 0 '^CONTEXT 42000 OK' env HOME="$d/home" "$C"
check "context fails for an unknown unit" 2 'no transcript' bash -c "HOME='$d/home' '$C' 'Milestone M9 042' 2>&1"
cd "$ROOT"

# autopilot-tokens.py: a lone read-only call right after another counts as unbatched; the first of
# a run, a batched message and a write do not.
d="$(new_repo)"
tmsg() {  # tmsg <id> <tool json>...
  local id="$1"; shift; local blocks; blocks="$(IFS=,; echo "$*")"
  printf '{"type":"assistant","message":{"id":"%s","model":"claude-x","content":[%s],"usage":{"input_tokens":1,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":1}}}\n' "$id" "$blocks"
}
rd='{"type":"tool_use","name":"Read","input":{"file_path":"a"}}'
gr='{"type":"tool_use","name":"Bash","input":{"command":"grep -n x y"}}'
wr='{"type":"tool_use","name":"Bash","input":{"command":"cargo test"}}'
{ tmsg u1 "$rd"; tmsg u2 "$gr"; tmsg u3 "$rd"; tmsg u4 "$rd" "$gr"; tmsg u5 "$rd"; tmsg u6 "$wr"; tmsg u7 "$rd"; } > "$d/b.jsonl"
check "autopilot-tokens counts unbatched reads" 0 '^\| orchestrator \(main session\) \| x \| 7 \| [^|]+\| [^|]+\| [^|]+\| 0 \| 2 \|' \
  "$(dirname "$S")/autopilot-tokens.py" "$d/b.jsonl"

check "autopilot-tokens classifies shell reads and writes" 0 '^ok$' python3 - "$(dirname "$S")/autopilot-tokens.py" <<'PY'
import importlib.util, sys
spec = importlib.util.spec_from_file_location("t", sys.argv[1]); t = importlib.util.module_from_spec(spec)
spec.loader.exec_module(t)
cases = {"cd /a/b && git status -sb": True, "git -C d log --oneline": True, "env X=1 grep -n a b": True,
         "grep x f 2>/dev/null | head": True, "cat > f <<EOF": False, "find . -delete": False,
         "git branch -D x": False, "ls; cargo build": False, "sed -i s/a/b/ f": False, "mise run gate": False,
         "grep -n 'a|b -> c' f": True, 'grep -rn "a\\|b; c" .': True, 'echo "x" > f': False}
bad = [c for c, want in cases.items() if t.read_only_bash(c) != want]
print("ok" if not bad else "misclassified: %s" % bad)
PY

# checkpoint.sh: one call reports branch, changes, unmerged commits, the ledger and the context.
d="$(new_repo)"; cd "$d/wt"
proj="$d/home/.claude/projects/$(pwd -P | sed 's/[^A-Za-z0-9]/-/g')"
mkdir -p "$proj/s1/subagents"
printf '{"type":"assistant","message":{"id":"a","model":"claude-x","usage":{"input_tokens":170000,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":1}}}\n' > "$proj/s1/subagents/agent-u.jsonl"
echo '{"description":"Milestone M1 042"}' > "$proj/s1/subagents/agent-u.meta.json"
ledger specs/042-x/autopilot.md wt 4-milestones; git add -A; git commit -qm "M1 work"; echo x > dirty.txt
K="$S/checkpoint.sh"
check "checkpoint exits OVER past the cap" 3 '^CONTEXT 170000 OVER' env HOME="$d/home" "$K" "Milestone M1 042" specs/042-x/autopilot.md
check "checkpoint reports unmerged commits" 3 '^UNMERGED 1 commit' env HOME="$d/home" "$K" "Milestone M1 042" specs/042-x/autopilot.md
check "checkpoint reports changed files" 3 '^  \?\? dirty\.txt' env HOME="$d/home" "$K" "Milestone M1 042" specs/042-x/autopilot.md
check "checkpoint reports the ledger phase" 3 '^PHASE 4-milestones$' env HOME="$d/home" "$K" "Milestone M1 042" specs/042-x/autopilot.md
check "checkpoint prints no escalation while it reads None" 0 '^ok$' bash -c \
  "out=\$(HOME='$d/home' AUTOPILOT_CONTEXT_CAP=900000 '$K' 'Milestone M1 042' specs/042-x/autopilot.md) && ! grep -q '^OPEN_ESCALATION' <<<\"\$out\" && echo ok"
python3 - specs/042-x/autopilot.md <<'PY'
import sys
p = sys.argv[1]; s = open(p).read()
s = s.replace("## Open escalation\n\nNone.", "## Handover\n\nM1: gate green, next review B.\n\n## Open escalation\n\nWhich locale wins?")
open(p, "w").write(s)
PY
check "checkpoint reports an open handover" 3 '^HANDOVER M1: gate green, next review B\.$' env HOME="$d/home" "$K" "Milestone M1 042" specs/042-x/autopilot.md
check "checkpoint reports an open escalation" 3 '^OPEN_ESCALATION Which locale wins\?$' env HOME="$d/home" "$K" "Milestone M1 042" specs/042-x/autopilot.md
check "checkpoint reports the next step" 3 '^NEXT open PR 2$' env HOME="$d/home" "$K" "Milestone M1 042" specs/042-x/autopilot.md
cd "$ROOT"

# measure-skill.sh: tokens per role, and the delta against a ref (also one from before the split).
d="$(new_repo)"; cd "$d/wt"; M="$S/measure-skill.sh"; k=.claude/skills/speckit-autopilot
mkdir -p $k/phases $k/references $k/tests
printf -- '---\ndescription: %s\n---\n%s\n' "$(printf 'd%.0s' $(seq 1 39))" "$(printf 'o%.0s' $(seq 1 391))" > $k/SKILL.md
printf '%0400d' 0 > $k/unit.md; printf '%0800d' 0 > $k/phases/1-spec.md
printf '%0400d' 0 > $k/references/r.md; printf '%09000d' 0 > $k/README.md; printf '%09000d' 0 > $k/tests/t.md
git add -A; git commit -qm skill
check "measure-skill counts the description" 0 '^description +10$' "$M"
check "measure-skill counts SKILL.md for the orchestrator" 0 '^orchestrator +113$' "$M"
check "measure-skill adds unit.md to each phase file of an old layout" 0 '^unit:1-spec +300$' "$M"
check "measure-skill leaves README and tests out of on-demand" 0 '^on-demand +100$' "$M"
mkdir -p $k/rules $k/flows $k/tasks $k/rubrics
printf '%0400d' 0 > $k/rules/unit.md; printf '%0400d' 0 > $k/rules/context.md; printf '%0200d' 0 > $k/rules/waiting.md
printf '%0400d' 0 > $k/flows/choose.md; printf '%0400d' 0 > $k/flows/bug.md; printf '%0200d' 0 > $k/flows/switch.md
printf '%0800d' 0 > $k/tasks/bug.md; printf '%0400d' 0 > $k/rubrics/spec.md
check "measure-skill counts the two rule files every unit reads" 0 '^unit +200$' "$M"
check "measure-skill adds choose.md to each flow file" 0 '^flow:bug +200$' "$M"
check "measure-skill counts each task file alone" 0 '^task:bug +200$' "$M"
check "measure-skill puts other rules, the switch file and rubrics on demand" 0 '^on-demand +200$' "$M"
check "measure-skill shows the delta against a ref" 0 '^task:bug +0 +200 +\+200$' "$M" HEAD
check "measure-skill shows an old ref's rows as gone" 0 '^unit:1-spec +300 +0 +-300$' "$M" HEAD
check "measure-skill refuses an unknown ref" 2 'unknown ref' "$M" no-such-ref
cd "$ROOT"

# gate-hook.sh: the PreToolUse hook that enforces the hard rules inside an autopilot worktree.
hook() { jq -n --arg c "$1" --arg d "${2:-$PWD}" '{tool_input:{command:$c},cwd:$d}' | "$S/gate-hook.sh"; }
d="$(new_repo)"; cd "$d/wt"; export GH_FIXTURES="$d/fx"
check "hook ignores a branch without a ledger" 0 '' hook "gh pr merge 5 --admin"
ledger specs/042-x/autopilot.md wt 4-milestones 11; git add -A; git commit -qm ledger
echo '{"headRefName":"wt"}' > "$d/fx/pr-12.json"; echo '{"headRefName":"other"}' > "$d/fx/pr-13.json"
check "hook blocks --delete-branch" 2 'never pass --delete-branch' hook "gh pr merge 11 --rebase --delete-branch"
check "hook blocks -d" 2 'never pass --delete-branch' hook "gh pr merge 11 -d"
check "hook reads -d only from the merge's own arguments" 0 '' hook "test -d x && gh pr merge 11 --rebase"
check "hook blocks --admin" 2 'never merge with --admin' hook "gh pr merge 11 --admin"
check "hook blocks removing the worktree" 2 'never remove a worktree' hook "git worktree remove ."
check "hook allows a PR in the ledger" 0 '' hook "scripts/autopilot/wait-merge.sh 11"
check "hook allows a PR from this branch" 0 '' hook "gh pr merge 12 --rebase"
check "hook blocks another flow's PR" 2 '#13 is not this flow' hook "gh pr comment 13 --body hi"
check "hook blocks waiting on another flow's PR" 2 '#13 is not this flow' hook "scripts/autopilot/wait-merge.sh 13"
check "hook blocks a merge through gh api" 2 'do not merge through gh api' hook "gh api -X PUT repos/o/r/pulls/11/merge"
check "hook allows pushing docs without a gate" 0 '' hook "git push -u origin wt"
echo 'fn main() {}' > main.rs; git add -A; git commit -qm code
check "hook blocks pushing code no gate saw" 2 'has not passed .mise run gate' hook "git push -u origin wt"
check "hook judges the repo git -C names" 0 '' hook "git -C $d push" "$d/wt"
git rev-parse 'HEAD^{tree}' >> "$(git rev-parse --git-path autopilot-gate-ok)"
check "hook allows pushing code the gate saw" 0 '' hook "git push -u origin wt"
echo more >> specs/042-x/autopilot.md; git commit -qam "ledger after gate"
check "hook allows docs committed after the gate" 0 '' hook "git push"
echo 'fn x() {}' >> main.rs; git commit -qam "code after gate"
check "hook blocks code committed after the gate" 2 'has not passed' hook "cd . && git push"
cd "$ROOT"

# context-hook.py: tells the caller its context passed the cap, on a ledger branch, without nagging.
d="$(new_repo)"; cd "$d/wt"; C="$S/context-hook.py"; export TMPDIR="$d/tmp"; mkdir -p "$TMPDIR" "$d/s1/subagents"
usage() { printf '{"type":"assistant","message":{"id":"%s","model":"claude-x","usage":{"input_tokens":%s,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":1}}}\n' "$1" "$2"; }
usage a 170000 > "$d/s1/subagents/agent-u1.jsonl"; usage a 90000 > "$d/s1/subagents/agent-u2.jsonl"; usage a 160000 > "$d/s1.jsonl"
chook() { jq -n --arg t "$d/s1.jsonl" --arg c "$PWD" --arg a "${1:-}" '{transcript_path:$t,cwd:$c,session_id:"s1"} + (if $a == "" then {} else {agent_id:$a} end)' | "$C"; }
check "context hook is silent without a ledger" 0 '^$' chook u1
ledger specs/042-x/autopilot.md wt 4-milestones
check "context hook tells a unit over the cap to hand over" 0 'additionalContext.*170000 tokens.*STATUS: HANDOVER' chook u1
check "context hook does not repeat at the same size" 0 '^$' chook u1
usage b 185000 >> "$d/s1/subagents/agent-u1.jsonl"
check "context hook waits for a full step of growth" 0 '^$' chook u1
usage c 195000 >> "$d/s1/subagents/agent-u1.jsonl"
check "context hook repeats after the context grew a step" 0 '195000 tokens' chook u1
check "context hook is silent under the cap" 0 '^$' chook u2
check "context hook tells the orchestrator to suggest /clear" 0 '160000 tokens.*/speckit-autopilot resume' chook
check "context hook honours the cap override" 0 '90000 tokens, over the 50000 cap' env AUTOPILOT_CONTEXT_CAP=50000 bash -c "$(declare -f chook); d='$d' C='$C' chook u2"
check "context hook ignores a missing transcript" 0 '^$' chook nope

# The same hook: after three requests in a row that each made one read-only call, it says to batch.
tuse() {  # tuse <id> <context> <tool json>...
  local id="$1" ctx="$2"; shift 2; local blocks; blocks="$(IFS=,; echo "$*")"
  printf '{"type":"assistant","message":{"id":"%s","model":"claude-x","content":[%s],"usage":{"input_tokens":%s,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":1}}}\n' "$id" "$blocks" "$ctx"
}
gr='{"type":"tool_use","name":"Bash","input":{"command":"grep -n x y"}}'
rd='{"type":"tool_use","name":"Read","input":{"file_path":"a"}}'
wr='{"type":"tool_use","name":"Bash","input":{"command":"cargo test"}}'
sub="$d/s1/subagents"
{ tuse a 90000 "$gr"; tuse b 90000 "$rd"; } > "$sub/agent-b1.jsonl"
check "batching: two lone reads are not yet a run" 0 '^$' chook b1
tuse c 91000 "$gr" >> "$sub/agent-b1.jsonl"
check "batching: three lone reads in a row" 0 'autopilot batching: your last 3 calls.*91000 tokens.*Explore' chook b1
tuse e 92000 "$gr" >> "$sub/agent-b1.jsonl"
check "batching: quiet right after it spoke" 0 '^$' chook b1
for i in 1 2 3 4 5 6 7; do tuse "q$i" 93000 "$gr"; done >> "$sub/agent-b1.jsonl"
check "batching: speaks again after eight more requests" 0 'autopilot batching: your last 11 calls' chook b1
{ tuse a 90000 "$gr"; tuse b 90000 "$gr"; tuse c 90000 "$rd" "$gr"; tuse e 90000 "$gr"; tuse f 90000 "$rd"; } > "$sub/agent-b2.jsonl"
check "batching: a batched message ends the run" 0 '^$' chook b2
{ tuse a 90000 "$gr"; tuse b 90000 "$wr"; tuse c 90000 "$gr"; tuse e 90000 "$rd"; } > "$sub/agent-b3.jsonl"
check "batching: a build is not a read" 0 '^$' chook b3
{ tuse a 30000 "$gr"; tuse b 30000 "$gr"; tuse c 30000 "$gr"; } > "$sub/agent-b4.jsonl"
check "batching: silent while the context is small" 0 '^$' chook b4
check "batching: honours the context setting" 0 'autopilot batching' env AUTOPILOT_BATCH_CTX=20000 bash -c "$(declare -f chook); d='$d' C='$C' chook b4"
{ tuse a 90000 "$gr"; tuse b 90000 "$gr"; } > "$sub/agent-b5.jsonl"
check "batching: honours the run setting" 0 'your last 2 calls' env AUTOPILOT_BATCH_RUN=2 bash -c "$(declare -f chook); d='$d' C='$C' chook b5"
{ tuse a 170000 "$gr"; tuse b 170000 "$gr"; tuse c 170000 "$gr"; } > "$sub/agent-b6.jsonl"
check "batching and the cap in one message" 0 'autopilot batching.*autopilot context: 170000' chook b6
git mv -q specs/042-x/autopilot.md specs/042-x/elsewhere.md 2>/dev/null || mv specs/042-x/autopilot.md specs/042-x/elsewhere.md
{ tuse a 90000 "$gr"; tuse b 90000 "$gr"; tuse c 90000 "$gr"; } > "$sub/agent-b7.jsonl"
check "batching: silent without a ledger" 0 '^$' chook b7
unset TMPDIR; cd "$ROOT"

# hold.sh: comes back when the log has its result line or the hold time passed, counts the holds
# per file, and says STOP once more holds would cost more than a rebuild.
d="$(new_repo)"; H="$S/hold.sh"
hold() { env TMPDIR="$d" AUTOPILOT_HOLD_SECS=1 "$H" "$@"; }
printf 'building\nGATE_EXIT=0\n' > "$d/done.log"; echo building > "$d/run.log"
check "hold reports a finished log at once" 0 '^DONE GATE_EXIT=0$' hold "$d/done.log"
check "hold comes back while the log has no result" 0 '^HOLD 1/10$' hold "$d/run.log"
check "hold counts the holds on one file" 0 '^HOLD 2/10$' hold "$d/run.log"
check "hold says stop at the limit" 3 '^STOP 2/2$' env AUTOPILOT_HOLD_MAX=2 TMPDIR="$d" AUTOPILOT_HOLD_SECS=1 "$H" "$d/run.log"
check "hold holds on a file nothing writes" 0 '^HOLD 1/10$' hold "$d/no-such-file"
check "hold --long has its own limit" 0 '^HOLD 1/12$' hold --long "$d/long.log"
check "hold takes a pattern" 0 '^DONE building$' hold "$d/run.log" '^build'
(sleep 1; echo "WAIT_EXIT=7" >> "$d/late.log") &
check "hold sees a result that arrives during the hold" 0 '^DONE WAIT_EXIT=7$' env TMPDIR="$d" AUTOPILOT_HOLD_SECS=20 "$H" "$d/late.log"
wait
echo "GATE_EXIT=1" >> "$d/run.log"
check "hold reports a red gate's exit code" 0 '^DONE GATE_EXIT=1$' hold "$d/run.log"
echo building > "$d/again.log"; hold "$d/again.log" >/dev/null; echo "GATE_EXIT=0" >> "$d/again.log"
hold "$d/again.log" >/dev/null; echo building > "$d/again.log"
check "hold starts counting again after DONE" 0 '^HOLD 1/10$' hold "$d/again.log"
echo building > "$d/run.log"; hold "$d/run.log" >/dev/null
sleep 4
check "hold forgets a count older than three hold times" 0 '^HOLD 1/10$' hold "$d/run.log"

# brief.py ledger: the state in full, of the history only the rows that name the milestone.
d="$(mktemp -d "$tmp/ledger.XXXX")"
cat > "$d/autopilot.md" <<'MD'
# Autopilot ledger — 042-links

- **Worktree branch**: wt
- **Phase**: 4-milestone M2

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #11 | Spec | merged | abc |

## Decisions

| # | Phase | Question | Answer |
|---|---|---|---|
| D1 | clarify | old question | old answer |
| D2 | 4-milestone M2 | mine | yes |
| D3 | 4-milestone M21 | not mine | no |

## Handover

T031 done; next T032.

## Open escalation

None.

Resolved 2026-01-02: disk was full.

## Follow-ups not done

- a defect elsewhere (M1 review A)
- another one
MD
B="$S/brief.py"
check "brief ledger: header and state sections in full" 0 '^ok$' bash -c "out=\$('$B' ledger '$d/autopilot.md' M2); grep -q 'Phase\*\*: 4-milestone M2' <<<\"\$out\" && grep -q '^| #11 | Spec' <<<\"\$out\" && grep -q '^T031 done; next T032' <<<\"\$out\" && echo ok"
check "brief ledger: the milestone's history rows, under the table head" 0 '^ok$' bash -c "out=\$('$B' ledger '$d/autopilot.md' M2); grep -q '^| D2 ' <<<\"\$out\" && grep -q '^| # | Phase' <<<\"\$out\" && echo ok"
check "brief ledger: leaves out other milestones' rows" 0 '^ok$' bash -c "out=\$('$B' ledger '$d/autopilot.md' M2); ! grep -qE '^\| D1 |^\| D3 |another one|a defect' <<<\"\$out\" && echo ok"
check "brief ledger: counts what it left out and says how to get it" 0 '^\(2 other line\(s\) not shown: brief.py section .*autopilot.md "Decisions"\)$' "$B" ledger "$d/autopilot.md" M2
check "brief ledger: no milestone, no history rows" 0 '^\(3 line\(s\) not shown: .*"Decisions"\)$' "$B" ledger "$d/autopilot.md"
check "brief ledger: a resolved escalation is not open" 0 '^ok$' bash -c "out=\$('$B' ledger '$d/autopilot.md'); grep -q '^(1 resolved, not shown)' <<<\"\$out\" && ! grep -q 'disk was full' <<<\"\$out\" && echo ok"

# read-hook.sh: on a ledger branch, blocks a Read without limit of a long file or a grown ledger.
d="$(new_repo)"; cd "$d/wt"; RH="$S/read-hook.sh"
seq 1 500 > long.rs; seq 1 50 > short.rs; seq 1 500 > shot.png
rhook() {  # rhook <file> [<limit>]
  jq -cn --arg f "$1" --arg l "${2:-}" --arg cwd "$PWD" \
    '{cwd: $cwd, tool_name: "Read", tool_input: ({file_path: $f} + (if $l == "" then {} else {limit: ($l | tonumber)} end))}' | "$RH"
}
check "read hook is silent off a ledger branch" 0 '^$' rhook "$PWD/long.rs"
ledger specs/042-x/autopilot.md wt 4-milestone 11
check "read hook blocks a whole Read of a long file" 2 'long.rs has 500 lines.*limit: 500' rhook "$PWD/long.rs"
check "read hook passes a Read with a limit" 0 '^$' rhook "$PWD/long.rs" 500
check "read hook passes a short file" 0 '^$' rhook "$PWD/short.rs"
check "read hook passes an image" 0 '^$' rhook "$PWD/shot.png"
check "read hook passes a missing file" 0 '^$' rhook "$PWD/nope.rs"
check "read hook honours the line limit setting" 2 'short.rs has 50 lines' env AUTOPILOT_READ_LINES=40 bash -c "$(declare -f rhook); RH='$RH' rhook '$PWD/short.rs'"
check "read hook passes a young ledger" 0 '^$' rhook "$PWD/specs/042-x/autopilot.md"
seq 1 100 >> specs/042-x/autopilot.md
check "read hook sends a grown ledger to brief.py" 2 'ledger has 1[0-9][0-9] lines.*brief.py ledger' rhook "$PWD/specs/042-x/autopilot.md"
mkdir -p .claude/skills/x; seq 1 500 > .claude/skills/x/unit.md
check "read hook passes the skill's own files" 0 '^$' rhook "$PWD/.claude/skills/x/unit.md"
cd "$ROOT"

# issue.sh: claims the issue a run starts from, refuses another flow's, closes it at handoff.
d="$(new_repo)"; export GH_FIXTURES="$d/fx"; I="$S/issue.sh"
echo '{"labels":[{"name":"bug"}],"assignees":[{"login":"me"}]}' > "$d/fx/issue-7.json"
echo '{"labels":[{"name":"in-progress"}],"assignees":[]}' > "$d/fx/issue-8.json"
echo '{"labels":[],"assignees":[{"login":"someone"}]}' > "$d/fx/issue-9.json"
check "issue start claims a free issue" 0 '^ISSUE_STARTED #7$' "$I" start 7 feat/x
check "issue start labels, assigns and comments the branch" 0 'add-label in-progress --add-assignee @me' cat "$d/fx/gh.log"
check "issue start names the branch" 0 'issue comment 7 --body .*`feat/x`' cat "$d/fx/gh.log"
check "issue start refuses an issue in progress" 1 '^ISSUE_TAKEN #8$' "$I" start 8 feat/x
check "issue start refuses an issue assigned to someone else" 1 '^ISSUE_TAKEN #9$' "$I" start 9 feat/x
check "issue start leaves a taken issue untouched" 0 '^ok$' bash -c "! grep -qE ' (8|9) ' '$d/fx/gh.log' && echo ok"
check "issue done closes it citing every PR" 0 '^ISSUE_CLOSED #7$' "$I" done 7 11 12
check "issue done lists the PRs" 0 'issue close 7 --reason completed --comment .*#11, #12\.' cat "$d/fx/gh.log"
check "issue done needs a PR" 2 'Usage: issue.sh' "$I" done 7
# Flow and effort labels: written on start, on a new issue and on a switch; older ones removed.
echo '{"labels":[{"name":"bug"},{"name":"flow:chore"},{"name":"effort:high"}],"assignees":[]}' > "$d/fx/issue-10.json"
: > "$d/fx/gh.log"
check "issue labels prints the issue's labels" 0 '^LABELS #10: bug, flow:chore, effort:high$' "$I" labels 10
check "issue labels says none on a bare issue" 0 '^LABELS #9: none$' "$I" labels 9
check "issue start writes the chosen flow and level" 0 '^ISSUE_STARTED #10$' "$I" start 10 feat/x bugfix low
check "issue start replaces the older flow and effort labels" 0 'issue edit 10 --add-label in-progress,flow:bugfix,effort:low --remove-label flow:chore,effort:high' cat "$d/fx/gh.log"
check "issue start creates the labels the repo lacks" 0 'label create flow:bugfix' cat "$d/fx/gh.log"
: > "$d/fx/gh.log"
check "issue flow moves the flow label" 0 '^ISSUE_FLOW #10 feature$' "$I" flow 10 feature
check "issue flow without a level keeps the effort label" 0 'issue edit 10 --add-label flow:feature --remove-label flow:chore$' cat "$d/fx/gh.log"
check "issue start refuses an unknown flow" 2 "unknown flow 'quick'" "$I" start 7 feat/x quick
check "issue start refuses an unknown effort" 2 "unknown effort 'medium'" "$I" start 7 feat/x bug medium
echo body > "$d/body.md"; : > "$d/fx/gh.log"
check "issue new opens the run's issue" 0 '^ISSUE_NEW #42$' "$I" new feat/x "Fix the flaky test" "$d/body.md" chore low
check "issue new labels and claims it" 0 'issue create --title Fix the flaky test .*--assignee @me --label in-progress,flow:chore,effort:low' cat "$d/fx/gh.log"
check "issue new marks a bugfix as a bug" 0 '^ISSUE_NEW #42$' "$I" new feat/x "Scrollback lost" "$d/body.md" bugfix
check "issue new adds the bug label beside the flow" 0 '--label in-progress,flow:bugfix,bug$' cat "$d/fx/gh.log"
check "issue new marks a feature as an enhancement" 0 '^ISSUE_NEW #42$' "$I" new feat/x "Folders" "$d/body.md" feature high
check "issue new adds the enhancement label" 0 '--label in-progress,flow:feature,effort:high,enhancement$' cat "$d/fx/gh.log"
check "issue new names the branch" 0 'issue comment 42 --body .*`feat/x`' cat "$d/fx/gh.log"
check "issue flow stops when the issue cannot be read" 2 '^$' bash -c "'$I' flow 99 bug 2>/dev/null"
check "issue new needs the body file" 2 'issue.sh new' "$I" new feat/x "t" "$d/none.md" chore

# The skill's own shape: SKILL.md routes, every step is a small file, each flow lists what it reads.
cd "$ROOT"; k=.claude/skills/speckit-autopilot; MAX_LINES=60
modules() { ls $k/SKILL.md $k/flows/*.md $k/tasks/*.md $k/rules/*.md $k/rubrics/*.md; }
too_long() { for f in $(modules); do n=$(wc -l < "$f"); [ "$n" -le "$MAX_LINES" ] || echo "$f $n"; done; }
none() { local out; out="$("$@")"; [ -z "$out" ] && echo none || echo "$out"; }
check "no skill module is over the line ceiling" 0 '^none$' none too_long
broken_links() {
  for f in $(modules) $k/phases/*.md $k/references/*.md $k/unit.md $k/README.md; do
    grep -o '](\([^)#]*\)' "$f" | sed 's/^](//' | grep -v '^http' | sort -u | while read -r l; do
      [ -e "$(dirname "$f")/$l" ] || echo "$f -> $l"
    done
  done
}
check "every link between skill files resolves" 0 '^none$' none broken_links
unlisted() {
  for f in $k/tasks/*.md $k/rules/*.md $k/rubrics/*.md $k/flows/*.md; do
    grep -qs "$(basename "$(dirname "$f")")/$(basename "$f")\|($(basename "$f"))" $(modules | grep -vx "$f") || echo "$f"
  done
}
check "every module is reachable from another" 0 '^none$' none unlisted
tasks_of() { grep -o '(\.\./tasks/[a-z-]*\.md)' "$k/flows/$1.md" | sed 's|.*/||; s|\.md)||' | sort -u | paste -sd' ' -; }
check "the bug flow reads only its own tasks" 0 '^bug gate pr red-ci$' tasks_of bug
check "the chore flow reads only its own tasks" 0 '^chore gate pr red-ci$' tasks_of chore
check "the bugfix flow skips spec, plan, tasks and close" 0 '^bugfix gate implement pr red-ci review review-rounds verify$' tasks_of bugfix
check "the feature flow reads every Spec Kit task" 0 '^clarify close gate implement milestone-format milestones plan pr red-ci review review-rounds spec tasks verify$' tasks_of feature
check "SKILL.md routes to every flow" 0 '^4$' bash -c "grep -o 'flows/\(bug\|bugfix\|feature\|chore\)\.md' $k/SKILL.md | sort -u | wc -l"
check "SKILL.md holds no rubric and no gate command" 0 '^ok$' bash -c "! grep -q 'mise run gate\|rubrics/\|BLOCKER' $k/SKILL.md && echo ok"
check "the labels table names every flow and effort label" 0 '^6$' bash -c "grep -o 'flow:bug\b\|flow:bugfix\|flow:feature\|flow:chore\|effort:high\|effort:low' $k/flows/choose.md | sort -u | wc -l"

echo "autopilot: $cases case(s), $failures failure(s)"
[ "$failures" -eq 0 ]
