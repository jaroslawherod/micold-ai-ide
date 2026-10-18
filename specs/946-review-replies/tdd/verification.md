---
feature: 946-review-replies
verdict: FAIL
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against (no override or preset found)
verified_at: 7953641d # short SHA audited; diff base 1a621e02 (design PR)
behaviors: 26 # cycle-log rows (22) + 4 M5 test/impl pairs with no row; there is no tdd/test-list.md
proven: 16
likely: 0
test_after: 8
no_test: 0
not_applicable: 2
high_smells: 3 # F2, F3 and F4's vacuous/removed-assertion class; F1 and F4 are test-first failures
criteria_total: 28 # US1-US4 acceptance scenarios (US5 deferred to #945)
criteria_covered: 28 # by task claims and the green runs below; per-scenario test bodies opened only for the sample named in Traceability
mutation_score: unmeasured # no mutation tool in the profile; 10 deliberate mutants run on core review code, 7 caught, 3 not (see Mutation results)
mutants_survived: 3 # M3a (vacuous test, siblings catch it), M7 (near-equivalent), M9 (real gap)
suite: targeted 372 passed, 0 failed (core lib review:: 131; core tests 129; daemon review_* 33; client features_changes 79); full workspace NOT run (disk)
---

# TDD Verification: Review comments the agent answers (946)

**Verdict: FAIL.** The whole of M5 (US4, re-anchoring: T057-T066) has no recorded red and landed as one commit
with tests and implementation together, and a removed existing test (bytewise path order of the batch) left
a surviving mutant.

The audit is independent: this session did not write these tests. All file contents were re-read cold; the
smell pass was done directly (no subagent), on the M5 and mutation-targeted files only.

## Test-first evidence

There is no `tdd/test-list.md` (the cycle log says tasks.md's test tasks are the list), so behaviours are the
cycle-log rows plus the M5 pairs. Git order corroborates every row: each feature commit carries its tests with
or before its source (`e149c1b2` tests first, then `b4dc9913`, `41c395ce`, `350ad2c2`, `07228abe`, `00001324`,
`aa3ef422`, `8ea49efe`).

| Behaviour (tasks) | Class | Evidence |
|---|---|---|
| T001 module declaration | NOT_APPLICABLE | structural, compiles |
| T003 (workspace) Pending to Draft rename | NOT_APPLICABLE | compile-driven rename |
| T002/T003 core: statuses, reference, fresh_id, Move | PROVEN | 6 failed against `todo!()` stubs; three sub-tests passed at declaration (disclosed in the log) |
| T004/T005 store v2 | PROVEN | 4 failed; v1 load and v3 quarantine passed at declaration (disclosed) |
| T006/T007 wire delta | PROVEN | `protocol_roundtrip` did not compile |
| T008/T009 Mark/Reanchor refused | PROVEN | 1 failed (`expect_err` on `Ack`) |
| T010/T014 batch layout | PROVEN | 6 failed; 4 properties passed at declaration (disclosed) |
| T011/T015 RefTaken | PROVEN | 1 failed |
| T012/T016 send only Drafts | TEST_AFTER | log: "passed at declaration"; red shown afterwards by mutation |
| T013/T017 Send review wording | PROVEN | 4 failed |
| T019/T023 report | PROVEN | 5 failed |
| T020/T024 reply display | PROVEN | 2 failed |
| T021/T025 tool catalog/parse/policy | PROVEN | 4 failed; policy test passed at declaration (disclosed) |
| T022/T026 daemon report | PROVEN | 4 failed |
| T027/T029 Applied/Answered card | TEST_AFTER | log: "not red-first", tests+impl in one commit (`804ce616` per log; `ab1c5a94` is titled "untested"); red only by later mutation |
| T028/T030/T031 card model | PROVEN | E0432 unresolved `card_model` |
| T035/T039 mark core | PROVEN | E0599 no `mark` |
| T036/T040 mark daemon | PROVEN | 3 failed |
| T037/T041 client mark/answer/reply | PROVEN | unresolved names |
| T038/T042 card actions | PROVEN | E0599 |
| T047/T050 (+T048/T051) panel row, highlighted | TEST_AFTER | log: "not red-first", written together (`40a532e1`) |
| T049/T052 panel state | TEST_AFTER | log: "not red-first"; "red by mutation is not recorded" |
| T057/T061 `locate`, `is_outdated` | TEST_AFTER | no cycle-log row at all; `ccdd1dc3` adds `anchor.rs` and its tests together |
| T058/T062 `reanchor` | TEST_AFTER | same; no row |
| T059/T063 daemon Reanchor + W16 | TEST_AFTER | same; no row |
| T060/T064 client anchors | TEST_AFTER | same; no row |

Changes to tests that already existed (diff `1a621e02..HEAD`, 946 files only):

- Spec-driven and recorded, not weakening: `review_edit.rs` `Pending` to `Draft`; 482's JSON-shape test
  renamed with `{"draft":null}`; `review_send.rs` refusal message "nothing is marked sent, FR-017" to
  "both stay Draft, FR-015" (same `assert_pushed_exactly`, same `false`); `features_changes.rs` outdated
  `sent` list from `vec![changed.id]` to empty (W16 moves current-side judgement to the daemon, which
  `review_send.rs` `w16_the_daemon_judges_...` now pins); `not_in_diff` fixture `gone` to `outside`
  (diff and fixture changed together; assertion kept).
- Not recorded as intended: 482's `files_are_in_bytewise_path_order` (`prompt.rs` at `1a621e02:445-457`,
  asserted `B` < `a/z` < `b`) was deleted in T010's rewrite and no P1-P9 test pins cross-file order beyond
  `a.rs`/`b.rs` (F2).
- No `#[ignore]`, skip, config exclusion, or lowered threshold was added.

tasks.md vs evidence: no behavioural task is ticked without a passing test. T034, T046, T067 (visual passes) are
honestly unticked; T056 is ticked with evidence only for M4 (`visual-pass/m4.md`); T068-T070 are open by design.
The cycle log title says "(M1, M2)" yet holds M3 and M4 rows and has no M5 rows.

## Findings

| # | Severity | Finding | Evidence |
|---|---|---|---|
| F1 | HIGH | M5 (T057-T066, 4 behaviours, about 800 lines of tests and code) has no recorded red, no cycle-log rows, and went in as one commit. Test-after by the rubric's definition | `git show --stat ccdd1dc3` (anchor.rs +305 with tests, comment.rs, review.rs, state.rs, features_changes.rs, review_edit.rs, review_send.rs); `tdd/cycle-log.md` ends at T049/T052 |
| F2 | HIGH | Removed coverage: the bytewise cross-file path ordering of the batch (FR-010, SC-003) is no longer asserted. Mutant M9 (sort key lowercased path) survives all 11 `review::prompt` tests. Should assert order of paths `B`, `a/z`, `b` (as the deleted test did) | deleted `prompt.rs:445-457` at `1a621e02`; sort at `crates/micold-core/src/review/prompt.rs:47-55` |
| F3 | HIGH | Conditional logic with an empty arm: the SC-004 property test asserts only when `locate` returns `Same`/`Moved`; `Gone | Ambiguous => {}`. With `locate` forced to `Gone`, this test still passes (M3a). It should also assert a `Moved`/`Same` count is non-zero, or assert the exact expected variant per edit | `crates/micold-core/src/review/anchor.rs:271-279` (test at 257-283) |
| F4 | HIGH | Four earlier behaviours are TEST_AFTER (T012/T016, T027/T029, T047+T048/T050+T051, T049/T052). The first two later got red by mutation, T047/T050 and T049/T052 have none ("not recorded"); the 80 px jump offset and order assertions are claimed to pin the panel but were never shown to fail | cycle-log rows named; `features_changes.rs` panel tests (79 pass) |
| F5 | MED | `reanchor_refuses_a_move_to_a_range_of_another_size_than_the_quote` never reaches the size guard: its `wrong` move is rejected because `from` is not the comment's range, as its own message says. M7 (guard removed) survives. Judged near-equivalent (the guard only differs for a stored range whose length differs from its quote) but the test name over-claims | `crates/micold-core/src/review/comment.rs:1451-1461`; guard at `:733` |
| F6 | MED | Wall-clock budgets (`< 100 ms`) in two new tests; flaky on a loaded CI box. Matches the repo's budget-test convention, so MED not HIGH | `anchor.rs:285-308`, `features_changes.rs:2226` |
| F7 | MED | Fixed `sleep(QUIET)` to prove an absence (no extra push, nothing typed); same convention as 482 | `review_send.rs:126,509,545,589,907`; `review_reports.rs:332` |
| F8 | MED | US1/US3/US4 GUI behaviour is verified only through `changes::update` state tests and geometry tests; visual passes for M2, M3, M5 not run (T034, T046, T067), T068-T070 open. Not a test smell, a verification gap | tasks.md |
| F9 | LOW | W16 daemon tests build the expected batch with `prompt::build` of the moved comments (partly re-implemented expectation); mitigated by the P1 golden bytes and the explicit `contains("lines 8")` | `review_send.rs` `w16_a_draft_whose_code_moved_...` |
| F10 | LOW | Cycle-log title "(M1, M2)" is stale; "Red shown afterwards by mutation" rows are evidence of strength, not of order | `tdd/cycle-log.md:1` |

Smell catalogue otherwise: no assertion-free, tautological, doubled-subject, over-mocked, self-approving
snapshot, empty or skipped tests in the files read (`anchor.rs` tests, M5 additions in `comment.rs`,
`review_edit.rs`, `review_send.rs`, `features_changes.rs`, `review_reports.rs` structure). Style matches the
exemplars (inline `#[cfg(test)]` for pure logic, one integration file per concern, stock libtest asserts with
rule messages). Hand-rolled helpers in `review_send.rs` (`place_quote`, `entry_dir`, `send_with`) are local to
that file's existing helper set; no `helpers` fake was bypassed that I found.

## Mutation results

No mutation tool (profile: `mutation: null`). Deliberate mutants, one at a time, each restored with
`git checkout` and the `review::` suite re-run green (131 passed) afterwards. Scope: 3 files
(`anchor.rs`, `comment.rs`, `reply.rs`) plus `prompt.rs`; core only. Not sampled: daemon and client code.

| Mutant | Behaviour | Survived | Judgment |
|---|---|---|---|
| M1 `anchor.rs` tie `tied = true` to `false` | T057 | No | `two_equally_near_occurrences_are_ambiguous` |
| M2 `anchor.rs` distance from `last.start()` to file top | T057 | No | 2 tests fail |
| M3a `locate` always `Gone`, SC-004 test alone | T057 | Yes | F3: the test is vacuous in isolation |
| M3b same, whole `review::anchor` | T057 | No | 8 of 12 fail |
| M4 `report` re-report sets `first_at = at` | T023 | No | caught |
| M5 reply limit `>` to `>=` | T023/T039 | No | 2 tests fail |
| M6 `shortened` `nth(max)` to `nth(max+1)` | T024 | No | caught |
| M7 `reanchor` size guard removed | T062 | Yes | F5, near-equivalent, test mislabelled |
| M8 blank-answer check switched to `Applied` | T023/T039 | No | 6 tests fail |
| M9 `prompt.rs` path sort case-insensitive | T014 | Yes | F2, real gap |

Earlier mutation recorded in the log (begin_send filter, `Parts::tag`) was not re-run.

## Traceability

Mapped by scenario from tasks.md claims (no `traces` column exists). Tests opened and sampled: `anchor.rs`
(all), M5 additions in `features_changes.rs`, `review_edit.rs`, `review_send.rs`, and by mutation the
report/mark/reply paths. The rest is by task text plus the green run counts.

| Criterion | Tests | End to end |
|---|---|---|
| US1 s1-s5, s7 | `review_reports.rs` (4: report, refusals, D5, conflict; restart in s1), `mcp_tools_catalog`, `mcp_policy`, `comment.rs` report | Yes (tool server to disk to push) |
| US1 s6, FR-006 | `review_edit.rs` Mark tests, `features_changes.rs` mark/answer | Daemon yes; card actions state-level |
| US1 s8, US2 s1-s6 | `prompt.rs` P1-P9, `review_send.rs`, `features_changes.rs` | Yes (typed into a session) |
| US3 s1-s6 | `features_changes.rs` panel tests, `review_panel_row` geometry | State/geometry only; no window |
| US4 s1-s4, s6 | `anchor.rs`, `review_send.rs` W16, `review_edit.rs` W15 | Yes at the daemon; s1 also through send |
| US4 s5, s7, s8 | `features_changes.rs` anchors tests | State-level only |

Untested criteria: none found. Tests tracing to nothing: none found. Weak spot: SC-003 (identical bytes on three
platforms) rests on P1 plus F2's gap; only Linux was run here.

## What was not audited

- The full workspace suite did not complete: the cold build exhausted the cloud disk. About 17 GB of
  test binaries in `target-shared/debug/deps` were deleted to continue (rebuildable). Only the targeted
  runs above were executed; `fmt`, clippy and `scripts/tests/*.test.sh` were not run.
- Mutation was a hand-picked sample of 10 mutants on core review code; no daemon or client mutants, no
  tool score. M4 panel behaviour and daemon `Reanchor`/W16 logic have no measured strength.
- Coverage: no tool.
- macOS and Windows (SC-003, quickstart section C), sandbox/acceptance tests, performance beyond the two
  `< 100 ms` asserts, and visual output were not assessed.
- Remediation tasks were not appended to tasks.md, per the request not to edit other files. The follow-up
  order is: F2, F3, F1/F4 (record or reproduce red for M5 by mutation), F5.
- Docs (`reviewing-changes.md`, `agent-tools.md`) were not checked against FR-028 or the batch bytes (T070).
