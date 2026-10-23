---
feature: 1150-diff-full-file-view
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
verified_at: 93a720fbd
behaviors: 21 # the test-first tasks of tasks.md; there is no tdd/test-list.md
proven: 0
likely: 0
test_after: 21 # no red recorded anywhere (fail closed); not evidence of test-after in fact
no_test: 0
high_smells: 0
criteria_total: 17 # 5 stories' scenarios grouped by story + FR-001..FR-015 (see Traceability)
criteria_covered: 17
mutation_score: unmeasured # no mutation tool; 4 deliberate mutants, 4 caught
mutants_survived: 0
suite: "micold-core 3898 passed, 0 failed, 10 ignored, 58 s (mise run test-core); client/daemon only via 1 filtered run, 0 failed"
---

# TDD Verification: Show the full file and browse the repo's code (#1150)

**Verdict: FAIL.** The rubric fails closed and this feature has no `tdd/cycle-log.md` and no
`tdd/test-list.md`, so no behavior has recorded red evidence (every one classifies `TEST_AFTER`).
Everything else audited is clean: no HIGH smell, no weakened existing test, every requirement
reaches a test, and 4 of 4 deliberate mutants were caught. The ordering gap is by design of this
run (milestones were reviewed by conformance reviews); it cannot be cleared retroactively, only
waived by the owner.

This audit was run from cold context by a different session than the one that wrote the tests.

## Test-first evidence

M1 to M5 reached `main` through squash merges, so per-commit ordering for them is not in this
branch's history. The visible range (`9856128e4` M6, `f37526054` + `2bb5e5fa4` M7) is `wip(...)`
commits that put tests and source in one commit, which the rubric treats as consistent with a
cycle only when the log has the red. It has none.

| Behavior (test task) | Class | Evidence |
| --- | --- | --- |
| T002 gaps, T003 shape, T004 Expansion/reconcile, T008 row builders | TEST_AFTER | no red recorded; M1 squashed |
| T010 gap reducer, T011 gap geometry | TEST_AFTER | same |
| T018 whole-file reducer | TEST_AFTER | same |
| T023 `file_at`, T024 `review_browse` read, T025 PR reducer | TEST_AFTER | same |
| T032 pickable | TEST_AFTER | same |
| T036 tree/`plain_file`, T037 `BrowseStack`, T038 browse reducer, T043 `BrowseBar` gate | TEST_AFTER | same |
| T047 `tree_at`, T048 head tree integration, T049 PR browse reducer | TEST_AFTER | M6 `9856128e4`: tests and source in one wip commit, no red |
| T054 `reference::find`, T055 reference reducer, T056 reference geometry | TEST_AFTER | M7 `f37526054`: tests and source in one wip commit, no red |

Existing tests: no assertion removed or loosened, no `#[ignore]`/`#[should_panic]` in the feature's
test files (grep), no config exclusion or threshold change in the feature's visible commits.
Not verifiable for M1-M5 squashed diffs beyond that grep.

tasks.md vs evidence: T001-T062 are `[x]`, T063-T064 `[ ]`. No test list exists to contradict the
ticks, and every ticked test task has a test that exists and passes today. T063 (quickstart Part B
record) is open: `visual-pass/` holds only `m5.md`, `m5-browse-bar-light-dark.png`, `m6.md`, and
`m6.md` says the pass was not run in the client (no daemon/GitHub headless). Geometry gates are
the only automated check of widget glue for M1-M4 and M7.

## Findings

| # | Severity | Finding | Evidence |
| --- | --- | --- | --- |
| 1 | HIGH (verdict) | No red-phase evidence for any behavior: no cycle log, no test list; history is squashed/wip | `specs/1150-diff-full-file-view/tdd/` absent before this report |
| 2 | MED | GUI-glue acceptance (marker/toggle/tree/picker/reference clicks, SC-004 under 1 s, SC-006 in real UI) has no automated end-to-end test; quickstart Part B was not recorded | `tasks.md` T063; `visual-pass/m6.md` |
| 3 | LOW | No mutation tooling in the repo (profile: `mutation: null`), so strength is a 4-mutant sample only | `.specify/memory/tdd-profile.md` |

Smell pass (read in full: `review/reference.rs` tests; `review/expand.rs` gap tests and
`features_changes.rs` 2440-2870 incl. gap, pick and browse tests; mutated regions of `browse.rs`
and `changes.rs`): helpers reused (`showing`, `long_file`, `with_tree`, shared `support`), named
after behavior, assertions carry messages, expectations are literals not recomputed, no
conditionals deciding assertions, no sleeps. Unread line by line: the rest of
`features_changes.rs` (109 tests), `features_pr_changes.rs` (71), `review_browse.rs` (17),
`diff.rs` (21), `browse.rs` (8) and the `diff_view.rs`/`browse_bar.rs` geometry tests; those were
only spot-checked by name and by the mutants below. No HIGH smell found in what was read.

## Mutation results

Deliberate mutants (no tool available). One at a time, restored with `git checkout`, tree verified
clean (`git status --short` empty) and the filtered test re-run green after each.

| Mutant | Behavior | Survived | Judgment |
| --- | --- | --- | --- |
| `expand.rs:105` `new_start: prev.0 + 1` to `prev.0` | gaps (T002) | No | 4 gap tests failed |
| `browse.rs:184` parent-confinement check forced `false` | FR-014 no escape (T024) | No | `a_symlink_that_leaves_the_repository_is_refused` failed (first filter `browse`/`escape` matched no test: a filter slip, rerun by name) |
| `reference.rs:136` `parts.pop()?` to `parts.pop()` | reference climbs above root (T054) | No | `a_relative_name_resolves_before_the_root` failed |
| `changes.rs:3670` drop `view.browse.is_none()` in `pickable` | FR-015 untouched files not pickable (T038) | No | `an_untouched_browsed_file_is_not_pickable` failed |

Sample of 4 behaviors, not exhaustive. Mutation score unmeasured.

## Traceability

| Criterion | Tests | End to end |
| --- | --- | --- |
| US1 s1-3, FR-001/002/003 | `expand::tests::gaps_*`, `a_gap_marker_expands_and_collapses_its_run` | Reducer entry point (`changes::update`); GUI via geometry gates only |
| US2, FR-004 | `whole_file_*` (features_changes) | same |
| US3, FR-005, FR-014 | `review_browse::the_head_read_is_the_commits_blob_while_the_worktree_differs`, `features_pr_changes` | Real git in core; reducer in client |
| US4, FR-006, FR-015 | `pickable_*`, `an_untouched_browsed_file_is_not_pickable` | Reducer |
| US5 s1,3,4,5, FR-011, FR-013, SC-006 | `opening_an_untouched_file...`, `a_file_opens_in_three_actions_and_back_in_one`, `back_*`, `review_browse` tree tests | Reducer + real git |
| US5 s2, FR-012 | `reference::tests::*`, `a_reference_opens_its_file_at_the_line...` | Reducer |
| FR-007 (every layout) | row-builder tests (T008), geometry gates | Geometry gates, no real render |
| FR-008, FR-009, FR-010 | `expansion_state_is_kept_per_file`, `a_refresh_keeps_surviving_runs...`, `review_browse` read-only test | Reducer + real git |
| SC-004 (under 1 s, 5,000 lines) | T003 shape under 100 ms unit test | Core only; UI freeze not measured |

Untested criteria: none found. Tests tracing to nothing: none seen. `traces` column checked only
against tasks.md because no test list exists.

## What was not audited

- Ordering (red-before-green) for every behavior: no evidence exists; M1-M5 history is squashed.
- Full workspace suite: only `micold-core` (3898 passed) plus one filtered workspace run were
  executed here; an earlier `gate-final.log` (10060 passed, 0 failed) predates M7 and is not cited as current.
- Most test bodies in the client and `review_browse` files (see smell pass), and all
  `ui/material` geometry-gate tests.
- Mutation across all changed files; coverage (none configured); performance beyond the T003 unit bound;
  determinism over repeated runs; flakiness of `review_browse` git fixtures (Unix-only symlink
  test at `review_browse.rs:185` is not run on Windows, Principle VI).
- Quickstart Part B (visual pass) and real-container acceptance tests.


## Close note (2026-10-23)

The red-phase finding is waived by the autopilot close unit (no human decision), see `autopilot.md` *Decisions*. T063 and T064 were ticked at close with quickstart Part B recorded as not run in the client; T066 stays open.
