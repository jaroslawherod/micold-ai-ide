---
feature: 1812-supervisor-run-status
verdict: FAIL (F2-F4 fixed at close; F1, F5, F6 accepted)
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 8753adb2d
behaviors: 0 # no tdd/test-list.md exists
proven: 0
likely: 0
test_after: unknown # fail closed: no red recorded for any behavior
no_test: 4 # FR-010, FR-011b, FR-013, US2 AC1/AC2 (prose behaviors, see finding 2)
high_smells: 0
criteria_total: 13 # US1 AC1-8, US2 AC1-4, US3 AC1
criteria_covered: 10 # US2 AC3 partial (answer.sh only); US2 AC1, AC2 untested
mutation_score: 92 # 11 of 12 deliberate mutants caught; scope: sample of changed scripts, no mutation tool (shell)
mutants_survived: 1 # not equivalent
suite: run-status.test.sh 110 passed, 0 failed, 9s; autopilot.test.sh 329 passed, 0 failed, 38s
---

# TDD Verification: Autopilot supervisor M1/M2 (run status, supervised escalation)

**Verdict: FAIL.** The feature has no test list and no cycle log, so no behavior has a recorded red;
fail closed, that is test-after. A second blocker: the supervised escalation behavior (US2 AC1, AC2,
FR-010, FR-011b, FR-013) has no automated test. The tests that exist are strong: 0 HIGH smells, no
existing test weakened, 11 of 12 deliberate mutants caught. The rubric table also lists "no test list"
as BLOCKED; the audit ran against `spec.md` and `tasks.md` instead, so FAIL is the conservative reading.

Audit independence: the auditor did not write these tests. Phase 3 was done directly, not by a subagent.

## Test-first evidence

| Unit | Class | Evidence |
| ---- | ----- | -------- |
| M1 (T001-T014, T019): run-status.sh, ledger-of.sh helpers, resume/handoff refactor | TEST_AFTER (fail closed) | No cycle log, no red output. `d430eb2ce` adds `run-status.sh` and `run-status.test.sh` in one commit, so order cannot be shown. tasks.md claims "write first, watch them fail" and ticks all of T001-T014. |
| M2 (T015-T018): supervised.sh, answer.sh | TEST_AFTER (fail closed) | `8753adb2d` adds scripts and cases together; no red recorded. |
| M2 prose (T019-T024, FR-010, FR-011b, FR-013) | NO_TEST | Skill prose and a manual pressure scenario only. |

Existing tests: `d430eb2ce` moved the `gh` stub, `new_repo`, `ledger`, `pr_json` and `check` out of
`autopilot.test.sh` into `lib/autopilot-fixtures.sh`. The moved bodies match the removed lines; no
assertion was removed, loosened, skipped or renamed. `resume.sh`/`handoff-check.sh` were refactored onto
`ledger-of.sh` with the existing cases unchanged and green. No weakened test.

tasks.md: every task is `[x]`; there is no test list to confirm the ticks against (completion claims
without behavior-level evidence).

## Findings

| # | Severity | Finding | Evidence |
| - | -------- | ------- | -------- |
| 1 | HIGH | No `tdd/test-list.md` or `cycle-log.md`: no red recorded for any behavior, M1 and M2 each land test and source in one commit. | `specs/1812-supervisor-run-status/tdd/` (absent); `d430eb2ce`, `8753adb2d` |
| 2 | HIGH | Supervised escalation behavior has no automated test: ends the turn with no interactive question (US2 AC1, FR-010, SC-003), unsupervised unchanged (US2 AC2, FR-013), `resume` with no answer re-prints the banner (FR-011b), `resume <answer>` calls `answer.sh` then continues (US2 AC3). Only `answer.sh` and `supervised.sh` mechanics are tested; the prose in `tasks/ask.md`, `tasks/resume.md`, `rules/escalation.md` is covered by a manual scenario in `tests/pressure-scenarios.md`. The suite already greps skill prose elsewhere (`autopilot.test.sh:635`), so a prose-pin test fits repo style. | `.claude/skills/speckit-autopilot/tasks/ask.md`, `resume.md`; `scripts/tests/autopilot.test.sh` (no case names them) |
| 3 | MED | Mutant M6 survived: removing `[ "$flow" = quick ] && flow=chore` leaves all 110 cases green, but data-model.md says quick = chore. No fixture uses a `quick` Kind. | `scripts/autopilot/run-status.sh:98`; `specs/1812-supervisor-run-status/data-model.md:12` |
| 4 | LOW | Vacuous alternation: `'autopilot.md.*|BUG-1.autopilot.md'` matches any output containing `autopilot.md`; the two following cases pin each ledger, so no gap, but the case itself proves little. | `scripts/tests/run-status.test.sh:70` |
| 5 | LOW | Ledger-unchanged assertions interpolate the ledger text into a single-quoted `bash -c` string via nested `sed`; fragile quoting where a `cksum` or file compare would do. | `scripts/tests/autopilot.test.sh` (answer cases "ignored call leaves the ledger alone", "refused batch leaves the ledger alone") |
| 6 | LOW | SC-005 (under 5 s) is never gated, by design (tasks.md); `run-status.test.sh` runs in 9 s for 110 cases, so no regression signal exists. | `tasks.md` Dependencies |

Smell pass, other items: no assertion-free, tautological, doubled-subject, conditional-logic, skipped or
snapshot tests. The two stubs (`gh`, `pr-state.sh`) sit at true external boundaries; the code under test is
real. Clock is pinned with `AUTOPILOT_NOW`; `date +%F` in `answer.sh` is real-clock but not asserted.
Tests are isolated (temp repos), deterministic across two runs, and name what broke.

## Mutation results

No mutation tool for shell; adapted to deliberate mutants. Sample of 12 on the highest-risk behaviors
(state precedence, GitHub-wins, done gating, stuck, dwell, decision numbering, data loss in the ledger
rewrite, answer validation, supervised marker). Each: one change, run the owning suite, restore from a copy,
`git diff -- scripts` clean afterwards, both suites re-run green (110/0 and 329/0).

| Mutant | Behavior | Survived | Judgment |
| ------ | -------- | -------- | -------- |
| M1 `run-status.sh` blocked/asking branches swapped | FR-001a | No | 12 cases fail |
| M2 `[ "$st" = OPEN ]` inverted | FR-004 | No | 20 fail |
| M3 `stuck` assignment dropped | FR-006 | No | 5 fail |
| M4 handoff check made always-pass | FR-005 | No | 7 fail |
| M5 `since = now + dwell` | FR-006a | No | 2 fail |
| M6 `quick` to `chore` mapping removed | data-model flow | Yes | Real gap, finding 3 |
| M7 `answer.sh` `D` number restarts at 0 | FR-012 | No | 3 fail |
| M8 `answer.sh` pipe escape removed | FR-012 | No | 1 fail (single case pins it) |
| M9 `answer.sh` section not reset to `None.` | FR-012, SC-004 | No | 3 fail |
| M10 `answer.sh` missing-question check disabled | FR-011a | No | 5 fail |
| M11 `answer.sh` By `user` to `agent` | FR-012 | No | 4 fail |
| M12 `supervised.sh` `0` treated as set | FR-009 | No | 1 fail |

Sampled 12 of the feature's behaviors; not exhaustive. Not sampled: `escalation_json` awk parser internals,
`ledger_on_main`, `resume.sh` mapping.

## Traceability

| Criterion | Tests | End to end |
| --------- | ----- | ---------- |
| US1 AC1-8 (waiting-ci, asking, done, dead-no-ledger, GitHub wins, stuck, blocked, running) | `run-status.test.sh` `state_case` plus per-state `jc` cases | Yes (`run-status.sh --json` and text) |
| US3 AC1 / FR-014 | `run-status.test.sh` coverage loop (one JSON and one text case per state) | Yes |
| US2 AC4 (asking shows question, options, recommendation) | `run-status.test.sh` asking cases | Yes |
| US2 AC3 (answer recorded, escalation cleared, continues) | `autopilot.test.sh` `answer:` cases | Partial: `answer.sh` only, "continues" is prose |
| US2 AC1, AC2 (turn ends, unsupervised unchanged) | none | No |
| FR-001a, 001b, 002-008 | run-status and ledger-of helper cases | Yes |
| FR-009 | `supervised:` cases, `supervised is true from ...` run-status cases | Yes |
| FR-010, FR-011, FR-011b, FR-013 | none (prose, manual scenario) | No |
| FR-011a, FR-012 | `answer:` batch and single cases | Yes (script level) |
| SC-001, SC-002 | state coverage loop, JSON-only assertions | Yes |
| SC-003 | none | No |
| SC-005 | not gated | No |

Untested criteria: US2 AC1, US2 AC2, FR-010, FR-011b, FR-013, SC-003. Tests tracing to nothing: none
(usage, precedence and read-only cases trace to FR-001, FR-001a, FR-008).

## What was not audited

- No test list existed, so per-behavior ids, `traces` columns and cycle-log red outputs were not checked.
- Git history cannot show order: each milestone is one commit, so test-first is unprovable either way.
- Mutation: 12 deliberate mutants by hand, no tool, no score comparable to a tool run.
- Skill prose quality (`tasks/ask.md`, `resume.md`, `rules/*.md`) and the pressure scenarios themselves were not executed.
- Performance beyond wall time; SC-005 timing against live GitHub.
- Real `pr-state.sh` (#1806): tests use a stub, never the real helper.
- `mise run gate` and the other 36 `scripts/tests/*.test.sh` files.
- Working tree at audit time had uncommitted edits to `spec.md` and `quickstart.md` and an untracked
  `drift-M1.md`; none were made by the auditor and none were assessed.
- Remediation tasks were not appended to `tasks.md` (the caller said to edit nothing else).

## Close resolution (2026-11-12)

- F1: not fixable after the fact. M1 and M2 each landed tests and source in one commit; no `tdd/test-list.md` or cycle log exists. Accepted for this shell-only internal tooling feature.
- F2: fixed. Nine prose-pin cases for `tasks/ask.md` and `tasks/resume.md` in `scripts/tests/autopilot.test.sh`; `supervised.sh` marker logic and the `asking` state were already covered.
- F3: fixed. A `quick` Kind case asserting flow `chore` in `scripts/tests/run-status.test.sh`.
- F4: fixed. The alternation case is split into two path-pinning cases.
- F5, F6: LOW, accepted (fragile expectation text; SC-005 timing recorded in quickstart, not gated).
