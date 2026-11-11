---
feature: 1802-verdict-head-sha
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: c6817744c # short SHA audited
behaviors: 0 # no tdd/test-list.md; 15 behavioral task groups used as the unit
proven: 0
likely: 0
test_after: 15 # fail closed: no red recorded anywhere
no_test: 0
high_smells: 0
criteria_total: 16 # US1 x4, US2 x4, US3 x2, SC-001..004, FR-004 hook, FR-008
criteria_covered: 14 # SC-004 gate not run; FR-007/FR-009 doc requirements untestable here
mutation_score: 12 of 16 killed # scope: 16 hand-made mutants on the feature's 5 scripts, no tool; 4 survived (1 equivalent, 3 untested detail)
mutants_survived: 4
suite: verdict-guard.test.sh 59 passed, 0 failed, 7s; autopilot.test.sh 288 passed, 0 failed, 34s
---

# TDD Verification: Review verdicts bound to the PR head SHA

**Verdict: FAIL.** The feature has no `tdd/test-list.md` and no `tdd/cycle-log.md`, so there is no red-phase
evidence for any behavior, git shows each test and its source landing in the same commit, and tasks T001-T019
are ticked `[X]` with nothing behind the ticks. The tests themselves are strong (12 of 16 mutants killed, the
survivors are detail-level), so the failure is missing evidence, not weak assertions. The rubric calls a
feature without a test list `BLOCKED`; the audit was run anyway (per the skill) and fails closed to `FAIL`.
The audit was independent: this session did not write the tests.

## Test-first evidence

No cycle log, no test list. History (`git log --stat 4b2a22c13..HEAD -- scripts`):

| Task group | Class | Evidence |
| --- | --- | --- |
| T004/T005 record cases + `record-verdict.sh` | TEST_AFTER | no red recorded; `07de67eaa` adds test and source together |
| T008-T010 guard cases + `verdict-guard.sh` | TEST_AFTER | same commit `07de67eaa` |
| T011/T012 wait-merge cases + `--squash`, `--match-head-commit` | TEST_AFTER | same commit `07de67eaa` |
| T013/T014 hook cases + `gate-hook.sh` | TEST_AFTER | `autopilot.test.sh` and `gate-hook.sh` both in `07de67eaa`; hardening in `c166d8a52` |
| T016-T018 carry cases + carry path | TEST_AFTER | `829d1f1cd` adds both together; fix `15b4d1e45` |

A same-commit test and source pair is `PROVEN` only with a recorded red; none exists. `tasks.md` says "red first"
for every test task; nothing corroborates it.

Existing tests changed by the feature (`scripts/tests/autopilot.test.sh`): the two `gate-hook` merge cases were
inverted as T013 planned (not a silent weakening); a `merge_ready` helper was added to the existing wait-merge
cases so they pass the guard (no assertion loosened). One re-pointed case is a finding (F4). No skipped tests,
no config exclusions, no thresholds changed.

`tasks.md` check: T001-T019 are `[X]` with no behavior marked DONE anywhere (F2). T020 is correctly `[ ]`.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| F1 | HIGH | No `tdd/test-list.md` or `tdd/cycle-log.md`: no red evidence for any behavior; every behavior is `TEST_AFTER` by the fail-closed rule | `git log` `07de67eaa`, `829d1f1cd`; `specs/1802-verdict-head-sha/` had no `tdd/` |
| F2 | HIGH | Tasks T001-T019 ticked `[X]` while no behavior is DONE on a list: completion claims with no evidence | `specs/1802-verdict-head-sha/tasks.md:9-48` |
| F3 | MED | Three mutants survived in FR-005's patch identity: dropping `-M` from `git diff --name-status`, dropping the `sed` that strips the rename score, and `--stable` to `--unstable` (all in `vl_identity`). No test has a rename or a whitespace/order case, so "rename = old and new path" and "stable" are unpinned. Probably near-equivalent today (patch-id is computed without `-M`), so MED, not HIGH | `scripts/autopilot/verdict-lib.sh` `vl_identity` (the `patch-id` line and the last two lines); `verdict-guard.test.sh` has no rename case |
| F4 | MED | Re-pointed test no longer exercises what its name says: `hook reads -d only from the wait-merge call's own arguments` runs `wait-merge.sh 11 test -d x`, which never matches the merge branch of the hook that parses `-d`. T013 required "a case that still reaches that code". The intent survives only incidentally in the line-566 case | `scripts/tests/autopilot.test.sh:562` (was `test -d x && ... --rebase` form) |
| F5 | MED | Bypassed test utility / duplicated setup: `verdict-guard.test.sh` redefines `check`, `new_repo`, `pr_json`, the `gh` stub and `q`, although `scripts/tests/lib/autopilot-fixtures.sh` (shared by `autopilot.test.sh` and `run-status.test.sh`) provides `q`, `new_repo`, `ledger`, `pr_json`, `check` | `scripts/tests/verdict-guard.test.sh:22-57,61-101` vs `scripts/tests/lib/autopilot-fixtures.sh:23,51,66,81,90` |
| F6 | LOW | Loose assertion: `amended message only` accepts `ALLOWED` or `CARRIED`, so it cannot tell whether the same-tree path or the carry path ran | `scripts/tests/verdict-guard.test.sh:207` |
| F7 | LOW | Untested branches: a non-SHA `headRefOid` (second "PR head unreadable" refuse), `base unreadable`, and wait-merge's `guard failed:` and `guard gave no head` fallbacks | `scripts/autopilot/verdict-guard.sh` (`vl_is_sha "$P"` line, `carry()` base fetch); `scripts/autopilot/wait-merge.sh` guard block |
| F8 | LOW | SC-004 and `quickstart.md` steps 1-4 (T020) not run or recorded; `mise run gate` not run by this audit | `specs/1802-verdict-head-sha/tasks.md:52` |

Smell pass (all new or changed shell test files): no assertion-free, tautological, vacuous, conditional-logic or
skipped tests found. The `gh` stub answers from JSON fixtures but the subjects (guard, wait-merge) run for real
against real throw-away git repos, so there is no doubled subject. No real clock, network or sleep
(`AUTOPILOT_POLL=0`). Suite time is 7 s and 34 s; two clean runs, no flakiness seen. Failure messages name the
case. The profile has no shell exemplar; the shell convention (`scripts/tests/*.test.sh`, pass/fail counters) is
followed.

## Mutation results

Scope: the feature's scripts only (`record-verdict.sh`, `verdict-guard.sh`, `verdict-lib.sh`, `wait-merge.sh`,
`gate-hook.sh`). No mutation tool in the profile; 16 deliberate mutants, one at a time, each restored with
`git checkout`. Behaviors sampled: approval vocabulary, merge-commit refusal, latest-round selection, head
pinning, patch-id equality, carry-comment dedupe, bugs-ledger exclusion, head validation, design-scope mapping,
hook deny, rename set, `--stable`, same-head shortcut.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| `vl_approved` also accepts CHANGES | US1-4 | No | 3 cases fail |
| drop `--match-head-commit` in wait-merge | FR-004 | No | 4 fail |
| drop merge-commit refusal | FR-005 | No | 1 fails |
| latest round `>=` to `<=` | Binding model | No | 2 fail |
| patch identity equality always true | FR-005 | No | 10 fail |
| carry comment dedupe off | FR-006 | No | 3 fail |
| remove bugs-ledger exclusion | FR-010 | No | 1 fails |
| remove head SHA check in recorder | FR-002 | No | 2 fail |
| remove approved check in guard loop | US1-4 | No | 4 fail |
| UNGUARDED head replaced by a constant | FR-004 | No | 1 fails |
| disable the hook's merge deny | FR-004 | No | 5 fail (`autopilot.test.sh`) |
| design scope matches any Purpose | Binding model | No | 1 fails |
| drop `sed` rename-score strip | FR-005 | Yes | near-equivalent, F3 |
| `patch-id --stable` to `--unstable` | FR-005 | Yes | untested, F3 |
| drop `-M` | FR-005 | Yes | near-equivalent, F3 |
| remove the `H = P` shortcut | US1-2 | Yes | equivalent: equal heads give an empty diff, same result |

Score: 12 of 16 killed (75%); 1 equivalent, 3 triaged into F3. The mutated files were restored each time; the
final `git status` showed no script modified. It did show `autopilot.md` modified and `drift-M1.md`,
`drift-M2.md` untracked, written by the orchestrator at 00:45, not by this audit.

## Traceability

No `traces` column exists; the mapping was built from the tests as they stand.

| Criterion | Tests (`verdict-guard.test.sh` line) | End to end |
| --- | --- | --- |
| US1-1 stale refused, names both SHAs | 138, 241 (via `wait-merge.sh`) | Yes |
| US1-2 current allowed | 135, 246-252 | Yes |
| US1-3 no verdict | 147, 180, 185 | Yes (guard) |
| US1-4 CHANGES/pending refused | 149-158 | Yes (guard) |
| US2-1 clean replay carried + comment | 195-199, 254-256 | Yes |
| US2-2 changed hunk / conflict refused | 212-218 | Yes (guard) |
| US2-3 CHANGES not carried | 220 | Yes (guard) |
| US2-4 chain from original H | 203 | Yes |
| US3-1 row has Scope and full Head | 109-110 | Yes |
| US3-2 bad SHA / verdict refused | 111-113 | Yes |
| FR-004 hook denies direct merge | `autopilot.test.sh:566-575` | Yes |
| FR-008 UNGUARDED, gh failure | 163, 166, 258 | Yes |
| SC-001, SC-002, SC-003 | 241-243 (0 merges), 195 (no dispatch implicit), 212-218 | Yes |
| FR-007 `tasks/merge.md` maps exit 7; FR-009 docs | none (doc edits) | n/a |
| SC-004 gate passes | not run by this audit | unverified (F8) |

Untested criteria: none among behavioral ones; SC-004 unverified; FR-007/FR-009 are doc edits. Tests tracing to
nothing: none.

## What was not audited

- `mise run gate`, `quickstart.md` steps and T020 were not run (F8).
- Mutation was 16 hand-made mutants, not a tool run. The `.claude/skills/speckit-autopilot` doc edits (FR-007,
  FR-009) were not graded. `autopilot.test.sh` cases for other features (1804, 1806, 1812) were out of scope.
- Real `gh` behavior: the stub is the only `gh`; payload shapes (`comments[].body`, `headRefOid`) are unverified
  against live GitHub.
- Cross-platform (bash 3.2, BSD tools): not run on macOS.
- Coverage and performance: no tool; speed observed only.
