---
feature: 1815-supervisor-question-routing
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: e826f648b # short SHA audited
behaviors: 15 # no test-list.md; derived from tasks.md test tasks T002-T005, T008-T009, T011-T012, T015, T017, T019-T020, T022 and the route subcommands
proven: 0
likely: 1 # M4 route target (red claimed in ledger, uncorroborated)
test_after: 14 # light tier: no cycle log; recorded, not the verdict reason
no_test: 0
high_smells: 0
criteria_total: 4 # user stories; 15 FRs and 4 SCs traced below
criteria_covered: 4
mutation_score: 84 # scope: deliberate mutants, 25 sampled, supervisor-route.sh (23) and answer.sh (2); no tool
mutants_survived: 4 # 2 real gaps (A1, K1), 1 near-equivalent (T3), 1 equivalent (T8)
suite: supervisor.test.sh 288 passed, 0 failed, 14s; autopilot.test.sh 354 passed, 0 failed, 41s
---

> Remediated after the audit: findings 1, 2, 3 and 5 (T023–T026) closed in caa363637; each new case fails under its named mutation. The suite now has 291 cases, 0 failures. The FAIL verdict above is the audit record at e826f648b.


# TDD Verification: Autopilot supervisor M4, question routing (#1815)

**Verdict: FAIL.** Two deliberate mutants inside DONE behaviors survive: free-text `route add` idempotency (`supervisor-route.sh:39`) and numeric ascending order of `route next` (`supervisor-route.sh:59`).

The audit is independent: the auditor did not write these tests. The tdd/ directory did not exist (light tier), so test-first evidence is recorded as absent and is not the reason for the verdict.

## Test-first evidence

No `tdd/test-list.md` or `tdd/cycle-log.md`. History is four milestone squashes on main (`afe9a3fe` M1, `b3ec7f88` M2, `9721aedb` M3, `d996a8d2` M4); the pre-squash branch commits are still reachable and were read.

| Behavior group | Class | Evidence |
|---|---|---|
| M1 route add/next/state/answer/compose/sent/drop/table, SKILL Route questions, ask.md (T002-T010) | TEST_AFTER | No red recorded. `autopilot.md:76` says tests and code were written in one commit (`376190a4b`: tests, `supervisor-route.sh`, SKILL together). Not splittable. |
| M2 route gate, answer.sh --by, resume.md (T011-T016) | TEST_AFTER | One commit `2628945b9` adds tests and source together; no red recorded. |
| M3 route next --reask, route overdue (T017-T019) | TEST_AFTER | One commit `cb3858481` adds tests and source together; no red recorded. |
| M4 route target (T020-T022) | LIKELY | `autopilot.md:79` claims 13 of 282 cases failed before the implementation, "in the PR body" of #1916. Not corroborated: `1d0b88284` is one commit with tests and source together, and the red output is not in the repo. |

Light tier: recorded, not failed on.

Existing tests changed in the range (all spec-driven, none weakened):

- `scripts/tests/autopilot.test.sh` (`afe9a3fe`): "supervised sends banner and notification, no AskUserQuestion" replaced by a version without "notification" plus two new cases (no notification; unsupervised unchanged). FR-006a mandates the change.
- `scripts/tests/supervisor.test.sh` (`afe9a3fe`): "skill reports asking, stuck, blocked and lost runs" narrowed to stuck/blocked/lost (line 175) and the `asking` half moved to line 368; "Never answer" replaced by line 369 ("Never unblock"). FR-015 and T009 mandate this. Net coverage is kept.
- `supervisor.test.sh` (`b3ec7f88`): the `ord "AskUserQuestion" "route answer"` check tightened to `route answer <key> user`. Stricter.
- No skips, no exclusions, no threshold changes.

`tasks.md`: every task is `[x]` and the work exists. There is no test list to cross-check the ticks against.

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| 1 | HIGH | Mutant survives: free-text `route add` made non-idempotent (drop the `rhas "R$1.Q1" ||` guard). Spec requires a restart never ask twice (FR-011, T004 "re-add is a no-op"). The only re-add test uses the `questions[]` path. | `scripts/autopilot/supervisor-route.sh:39`; test `scripts/tests/supervisor.test.sh:299` uses `R12.Q1` from `esc`; the free-text fixture (`R16`, line 311) is added once. |
| 2 | HIGH | Mutant survives: `sort -k1,1n -k2,2n` changed to lexical `-k1,1 -k2,2`. FR-006 requires ascending issue order and `route next` is the batcher. Fixtures use issues 12-23 and questions 1-5, so lexical and numeric order agree. | `scripts/autopilot/supervisor-route.sh:59`; tests at `supervisor.test.sh:301-307`, `389`. Needs an issue below 10 against one above 99, or `Q10` against `Q2`. |
| 3 | MED | Mutant survives: dropping the `sid != ($r \| sid)` self-exclusion in `route_target`. Reachable only if a row's parent worktree is its own worktree. Near-equivalent, but the guard is undocumented and unpinned. | `supervisor-route.sh:178` |
| 4 | MED | Skill-text cases pin exact prose (`grep -F "never retry"`, `"only the missing"`, step-number strings such as `"run steps 9 to 11"`, `"back to step 3"`). They verify the text exists, not that an agent following it behaves correctly, and break on any rewording. Accepted as the project's method for skill text (tasks.md, Tests header), but the real routing flow (collect, route, ask, send) has no executable end-to-end case. | `supervisor.test.sh:354-380, 402-410, 438`; `autopilot.test.sh` ask.md and resume.md cases |
| 5 | LOW | Dead stub `ord() { :; }` inside `bash -c` strings that never call it. | `supervisor.test.sh:378, 402` |
| 6 | LOW | The case name "an empty Sent is never overdue" asserts `^31$`, which proves issue 32 is absent only by omission. | `supervisor.test.sh:397` |
| 7 | MED | Test-first discipline is unverifiable for 14 of 15 behaviors (see Test-first evidence). Light tier, recorded. | `autopilot.md:76,79` |

No HIGH smells of the assertion-free, tautological, vacuous or conditional kind found. Each case asserts exit code and an anchored output regex. Doubles are fixture files, not mocks. Helpers `esc`, `sessions`, `mk` and `rt` are shared; `esc` and `sessions` are `supervisor.test.sh`'s own, and the `helpers` list in the profile is Rust-only, so Bypassed test utility and Foreign style cannot be graded for shell.

Properties: suites are deterministic (temp dirs, no clock; `route overdue` takes `--now`), isolated per case and fast (14s and 41s). One flake outside the feature: `autopilot.test.sh` prints `push negotiation failed` from git in this container, and it passes. `sessions()` (`supervisor.test.sh:276`) is defined and never visibly used; not confirmed as dead.

## Mutation results

No mutation tool in the profile (shell). Deliberate mutants: 25 sampled, one at a time, each restored with `cmp` and re-run green (final `supervisor: 288 case(s), 0 failure(s)`, `autopilot: 354 case(s), 0 failure(s)`, `git status` clean). Sampled behaviors: route target, gate, next, overdue, compose, drop, add, answer, sent, rcell, `answer.sh --by`. Not exhaustive.

| Mutant | Behavior | Survived | Judgment |
|---|---|---|---|
| target: drop `parent_state` check | US4.2/FR-012 | No | caught, 1 case |
| target: drop live-over-failed sort | FR-012 | No | caught |
| target: drop self-exclusion (`:178`) | FR-012 | Yes | finding 3, near-equivalent |
| target: always `user` | US4.1 | No | caught, 7 cases |
| target: no `is_caller` fallback | FR-012 | No | caught |
| target: `default` parent not matched | US4 | No | caught |
| target: `{sessions:[]}` shape unsupported | FR-012 | No | caught |
| target: non-string parent accepted | FR-012 | Yes | equivalent: jq then fails the worktree match and returns `user` |
| gate: category 4 not user-only | FR-003 | No | caught |
| next: limit 4 to 5 | FR-006 | No | caught |
| next --reask limited to 4 / uses pending | FR-010 | No | caught |
| overdue: `>` to `>=` | FR-008 | No | caught, boundary pinned |
| overdue: not unique | FR-008 | No | caught |
| compose: allsup never cleared | FR-005 | No | caught, 5 cases |
| compose: `resolved` not accepted | FR-005 | No | caught |
| compose: single not bare | FR-008a | No | caught |
| compose: missing never reported | FR-008a | No | caught |
| drop: drops every issue | SC-004 | No | caught |
| rcell: pipe kept | T004 | No | caught |
| add: free-text re-add not idempotent (`:39`) | FR-011 | Yes | finding 1 |
| answer: supervisor stays `answered` | FR-005 | No | caught |
| sent: writes the Answer cell | FR-008 | No | caught |
| keys: lexical sort (`:59`) | FR-006 | Yes | finding 2 |
| answer.sh: default `by` to supervisor | FR-005a | No | caught, 5 cases |
| answer.sh: By cell fixed to `user` | FR-005a | No | caught, 3 cases |

Score 21 of 25 caught (84%). Both real survivors sit in behaviors marked done, so each is HIGH.

## Traceability

| Criterion | Tests | End to end |
|---|---|---|
| US1 batch (FR-001, 006, 007, 008, 008a, 011, 014; SC-001, SC-004) | `supervisor.test.sh:298-348` (route CLI), `354-369` (SKILL order) | Partly: CLI is exercised for real; the skill's steps are checked by text only |
| US2 resolve routine (FR-002, 003, 004, 005, 005a; SC-002) | `333-342`, `373-381`; `autopilot.test.sh` `answer --by`, `resume.md` | Partly: same limit |
| US3 context cap (FR-009, 010, 011; SC-003) | `383-410` | Partly: same limit |
| US4 upward relay (FR-012, 013) | `412-438` | Partly: `route target` is real; the skill send and `not_found` handling are text |
| FR-006a | `autopilot.test.sh` ask.md and supervised.md cases | Text only |
| FR-015 | `supervisor.test.sh:368-369` | Text only |

Untested requirement: none by presence. Behaviorally unverified, since an agent acts on the skill text and no fixture drives it: FR-007 (the notification), FR-008 (confirm after `CHANGED`), FR-009 (drop on a session that left `asking`), FR-013 (`not_found` fallback). Tests tracing to nothing: none found.

## What was not audited

- `tdd/test-list.md` and `tdd/cycle-log.md` do not exist; behaviors were derived from tasks.md, and the count of 15 is the auditor's grouping.
- The M4 red claim lives in the PR body of #1916 and was not fetched; no network or `gh` call was made.
- Mutants were sampled (25), not exhaustive; `supervisor.sh` (the ledger half) and the other `answer.sh` branches were not mutated. No mutation tool and no coverage tool exist for shell.
- SKILL.md, ask.md, resume.md and supervised.md prose were not mutated; whether an agent following the text behaves correctly was not tested, since there is no live session harness.
- No shellcheck run. Performance under large ledgers, and Windows/macOS bash 3.2 behavior, were not assessed.
