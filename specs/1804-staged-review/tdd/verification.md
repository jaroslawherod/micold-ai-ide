---
feature: 1804-staged-review
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against
verified_at: d14e1692c
behaviors: 0 # no tdd/test-list.md; audited against spec.md and tasks.md instead
proven: 0
likely: 0
test_after: 2 # milestone-level units M1, M2 (no red recorded)
no_test: 0
high_smells: 0
criteria_total: 9 # US1 sc.1-4, US2 sc.1-3, US3 sc.1-2 (FR/SC mapped in Traceability)
criteria_covered: 7
mutation_score: unmeasured (no mutation tool); 5 deliberate mutants, 3 caught, 2 survived
mutants_survived: 2
suite: 303 cases passed, 0 failed, ~35s (scripts/tests/autopilot.test.sh only; cargo suite not run)
---

# TDD Verification: Staged review for speckit-autopilot

**Verdict: FAIL.** No red is recorded for any behavior (no `tdd/test-list.md`, no `tdd/cycle-log.md`), so
test-first is `TEST_AFTER` by the rubric, and a deliberate mutant on the callers line budget survived.

Independence: the audit was run from cold context by a separate agent; every
file cited was re-read. Fail-closed: the missing test list is a gap, not an assumption of compliance.
Per the rubric a missing test list is `BLOCKED`; the invoking instruction asked for an audit against
`spec.md`, so the audit ran and the verdict is `FAIL` on the evidence it could gather.

## Test-first evidence

The feature has no test list, so there are no behavior ids. Units are the two merged milestones.

| Unit | Class | Evidence |
| ---- | ----- | -------- |
| M1 (PR #1820, T002-T010) | TEST_AFTER | No red recorded anywhere. `cb8731d73` adds `review-facts.py` (+288) and 105 test lines in `scripts/tests/autopilot.test.sh` in one commit, so ordering is not visible. `tasks.md` says "seen failing" but the ledger (`autopilot.md:43`) holds no red output. |
| M1 review fixes | TEST_AFTER | `dbee88ecd` fixes char literals, deletions, git-config prefixes in the script; the tests arrive afterwards in `2f17a76c4` ("tests added" per `autopilot.md:43`). Source before test, shown by history. |
| M2 (PR #1824, T011-T015) | TEST_AFTER | `badf7c6e9` adds the three skill files' text and the 15 grep assertions together. No red recorded. |

Existing tests: diffed each feature commit on `scripts/tests/autopilot.test.sh` (`cb8731d73`, `2f17a76c4`, `badf7c6e9`): additions only, no removed or loosened assertion, no skip, no threshold change. No weakened test.

`tasks.md`: T001-T015 ticked, T016-T017 unticked and explicitly post-merge (US3). No test list to cross-check the ticks against, so no tick is backed by evidence beyond the green suite.

## Findings

Remediation (close PR): the two surviving mutants (callers line budget, `UNANSWERED: <reason>` wording) are closed by a new callers-budget case and `grep -F 'UNANSWERED: <reason>'` in `scripts/tests/autopilot.test.sh`; the suite is now 304 cases. Finding 6's drift files are committed in the same PR. The test-first gap is not retroactively fixable (no test list, no cycle log) and is accepted.

| # | Severity | Finding | Evidence |
| - | -------- | ------- | -------- |
| 1 | HIGH | No test-first evidence: no test list, no cycle log, no red output. M1 and M2 test and source landed in the same commit; M1 review fixes landed source-first. | `specs/1804-staged-review/` has no `tdd/`; `git show --stat cb8731d73 dbee88ecd 2f17a76c4 badf7c6e9` |
| 2 | MED | The 300-line callers budget and the "N more symbols: callers not looked up" path are untested. Raising `CALLERS_LINES` to 3000 leaves the suite green. The only truncation test (`autopilot.test.sh:359`) exercises the per-symbol cap `CALLERS_SHOWN = 8`. The spec's "under 400 lines total" bound is therefore unpinned. | `scripts/autopilot/review-facts.py:18,252`; `scripts/tests/autopilot.test.sh:359-362` |
| 3 | MED | The M2 markdown assertions are presence greps, so they prove a phrase exists, not that the flow says the right thing. Replacing `UNANSWERED: <reason>` with `MISSING: <reason>` in the worker-output line survives, because `UNANSWERED` appears elsewhere in `review.md`. Each assertion matches a phrase anywhere in the file. This is the nature of testing prose, but nothing ties the phrase to the stage it constrains. | `scripts/tests/autopilot.test.sh:857-871`; `.claude/skills/speckit-autopilot/tasks/review.md:53` |
| 4 | LOW | Every case is a `check` with a regex over combined output; failures say "exit 0 (want 0), output:" with an empty output, so a failing case names the case but not what differed (observed on mutants M1, M2, M4). | `scripts/tests/autopilot.test.sh:352-436` |
| 5 | LOW | `autopilot.test.sh:364` ("without data the word covered does not appear") is a double-negated pipeline that is hard to read and passes if `covered` is absent for any reason. The positive check at 363 already pins the output. | `scripts/tests/autopilot.test.sh:364` |
| 6 | INFO | Two untracked files, `specs/1804-staged-review/drift-M1.md` and `drift-M2.md`, appeared in the tree during the audit (created 08:42:10, before the mutants ran, not by this audit's commands). Left as found. | `git status --short` |

Smell catalogue: no tautological, doubled-subject or re-implemented-expectation smells found. The facts tests build a real throwaway git repo (`new_repo`) and run the real script, no doubles; the staged-verdict case (`:413-436`) runs the real `brief.py` and `checkpoint.sh`. The repository's profile conventions (Rust cargo tests, `Fake*` helpers) do not apply to this shell suite; style matches the surrounding `autopilot.test.sh` cases. Isolation: each case uses a temp repo. Determinism: the 50-file case is time-bounded (`[ SECONDS - t0 -lt 60 ]`), mildly load-sensitive. Speed: ~35 s for the whole file.

## Mutation results

No mutation tool in the profile. Deliberate mutants, one at a time, each restored with `git checkout --` and the suite re-run green (303 cases, 0 failures) at the end. Five sampled from the high-risk behaviors (fail-closed coverage, failure exits, size bound, plan limit, unanswered-probe rule); not exhaustive.

| Mutant | Behavior | Survived | Judgment |
| ------ | -------- | -------- | -------- |
| `review-facts.py:274` "unknown (no coverage data)" to "covered (no coverage data)" | FR-003 | No | Caught by `autopilot.test.sh:363` |
| `review-facts.py:216` `raise Fail("detached HEAD")` to `pass` | FR-011 | No | Caught by `:394` |
| `review-facts.py:18` `CALLERS_LINES` 300 to 3000 | US1 sc.2 total size bound | Yes | Real gap, finding 2 |
| `review.md` "at most 6 probes" to "at most 60 probes" | FR-004 | No | Caught by `:857` |
| `review.md:53` `UNANSWERED: <reason>` to `MISSING: <reason>` | FR-005, FR-011 | Yes | Weak assertion, finding 3 |

## Traceability

No `traces` column exists; mapped by reading the tests.

| Criterion | Tests | End to end |
| --------- | ----- | ---------- |
| US1 sc.1 / FR-001 (files, ranges, symbols) | `:352-355` | Yes (real script on real repo) |
| US1 sc.2 (callers) | `:356-362` | Yes; total budget unpinned (finding 2) |
| US1 sc.3 / FR-003 (uncovered) | `:372-374` | Yes |
| US1 sc.4 / FR-003 (unknown) | `:363-364`, `:386` | Yes |
| FR-002 (per-snapshot file, untracked) | `:377-381` | Yes |
| FR-011 script side | `:390-394` | Yes |
| SC-003 (50 files, 60 s, no model) | `:409-412` | Yes |
| SC-004 (ledger readers) | `:413-436` | Yes |
| US2 sc.1-3 / FR-004, FR-005, FR-006 | `:857-860` | Prose only; no staged run in the suite. T015 fixture run is recorded in the PR body, not reproducible here. |
| FR-007, FR-010, FR-011 (flow side), FR-013 wording | `:861-871` | Prose only (presence greps) |
| FR-008, FR-009, US3 sc.1-2, SC-001, SC-002, SC-005 | `:866` (wording only) | Not testable yet: needs T016-T017 after three shadow reviews |
| FR-012 | none | Untested; part of post-merge US3 |

Untested criteria: FR-012 and the US3 outcomes (explicitly deferred to follow-up #1808). Tests tracing to nothing: none.

## What was not audited

- The model-driven flow (plan, worker, verdict stages): not executable without model runs; the suite checks only that the skill text says it. T015's fixture run was not re-performed.
- `cargo test --workspace` and `mise run gate`: not run; the change touches no Rust.
- Git history of the earlier rebase: `badf7c6e9` and later commits were rebased onto `main`, so original commit order may differ from what is visible.
- Mutation: five hand-picked mutants, no tool; no coverage tool exists for shell/Python here.
- Performance beyond the 50-file case; behavior on macOS/BSD (SC cross-platform claim) was not run.
- Spec/plan/task artifact quality, and the `docs/` surface: out of scope.
