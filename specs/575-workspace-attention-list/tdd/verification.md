---
feature: 575-workspace-attention-list
verdict: PASS_WITH_GAPS # after remediation; the audit as found was FAIL (T010 test-after, see finding 1)
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against (no override or preset found)
verified_at: 1bbe8f4b # plus the test-only remediation of T023/T024 in the close commit
range: 7ad42e71..7303f78e # the tasks commit to M2's commit (M1 and M2 of PR #602)
behaviors: 7 # the test-first task pairs of tasks.md (no tdd/test-list.md exists) plus M2's closed-session rule
proven: 5
likely: 1 # M2
test_after: 1 # T010, red recovered after the fact by mutant E2 (T022)
no_test: 0
high_smells: 0 # S1 HIGH declined: mutant E shows the test catches a real colour bug
criteria_total: 10 # US1 1-5, US2 1-5
criteria_covered: 8 # US2.4 and US2.5 rest on 039 (R7) and quickstart B7/B8, not on a 575 test
mutation_score: unmeasured # no mutation tool in the profile; 8 deliberate mutants sampled: 8 caught
mutants_survived: 0
suite: cargo test --workspace green apart from the six root-only baseline failures (ledger, Environment notes M1)
---

# TDD Verification: Sidebar attention indicator (#575)

**Verdict: PASS_WITH_GAPS.** As found, the audit was **FAIL**: the count-contrast test (T010) was written
after the code it guards and has no recorded red. Mutant E2 has since given that test a red after the fact
(T022), and the smell findings have been cleared (T023, T024). Two gaps remain. M2's red is recorded only in
the ledger, and its test and source landed in one commit. Mutation is sampled by hand, not measured by a tool.

Independence: phases 0 to 4 were run by the close unit. The smell pass came from a fresh reviewer that did
not write the tests (`claude -p --agent autopilot-reviewer`). No `tdd/test-list.md` exists. As in 582, the
test-first task pairs of `tasks.md` stand in as the behaviour list.

## Test-first evidence

| Behavior (tasks) | Class | Evidence |
| --- | --- | --- |
| A session counts only when unread, not closed, not in view (T001 → T002) | PROVEN | red commit `41634afe` (stubs) before green `4f6fb12c`; cycle-log row 1 |
| A location row's count over its sessions (T003 → T006) | PROVEN | `41634afe` → `4f6fb12c`; 12 failed on the stub |
| Tooltip line in words (T004 → T007) | PROVEN | `41634afe` → `4f6fb12c` |
| Row geometry, slot, label role, count colour (T005 → T008) | PROVEN | `41634afe` → `4f6fb12c`; 5 of 11 failed on the stub |
| Live updates and FR-009 (T014 over T006) | PROVEN | stub re-run recorded in cycle-log row 5 |
| The switcher's counts skip closed sessions (T015-T017 → T018) | LIKELY | test and source both in `7303f78e`. The red is recorded in the ledger (*Environment notes (M2)*) and now in the cycle log |
| Count legible in every row state (T010) | TEST_AFTER | passed at first run; red recovered after the fact by mutant E2 (T022) |

No existing test was weakened: the branch's diff removes no assertion.

## Findings

1. **[HIGH → remediated] Test-after** `crates/micold-client/src/ui/material/composition_contrast.rs:177`. T010 has
   no red from before its code existed. Remediated by T022: mutant E2 (`count_tint` returns `outline_variant`)
   fails `a_location_rows_unread_count_is_legible_in_every_row_state`. The test can fail for the right reason,
   but its red came after the code, so the class stays TEST_AFTER.
2. **[MED → fixed] Foreign style** `crates/micold-client/tests/sidebar_attention.rs` (S3). About 20 assertions
   had no rule message. Each now has one (T023).
3. **[MED → fixed] Foreign style** `crates/micold-core/src/workspace.rs:581` (S4). A bare `assert_eq!` had no
   rule message. It now has one (T023, commit `1bbe8f4b`).
4. **[MED → fixed] Eager tests** `sidebar_attention.rs` (S5).
   `expanding_collapsing_and_hovering_a_row_read_nothing` is now three tests: unread flags, view report and
   count. `a_hidden_worktree_still_counts_on_the_switcher` is now two: the agent setting and the tag filter
   (T024).
5. **[LOW → fixed] Unclear names** in the tooltip section (S9). The tests are renamed for the function and
   rule (`unread_tooltip_line_is_singular_for_one`, `with_unread_line_appends_after_*`), and each now cites
   FR-005 or contract A7.
6. **[LOW, open] Redundant loop** `composition_contrast.rs:~182` (S6). The `row_tint` loop runs twice over a
   function that ignores that argument. It is kept as a guard in case `count_tint` ever reads the row tint.
7. **[LOW, open] Magic values** `sidebar_attention.rs` catalog fixture (S7): `schema_version: 1` and
   `attention_seq: 1`. The switcher totals are now explained in the assertion messages.
8. **Declined.** S1 (HIGH, "tautological `count_tint` test"): mutant E makes `count_tint` return the row
   tint, and the test catches it. S2 (MED, bypasses `mod support`): the peer files `features_sidebar.rs` and
   `switcher_unread.rs` build state the same way. S8 (LOW): the fixture check guards against a test that
   compares nothing, and is kept on purpose.

## Mutation results

No mutation tool is in the profile. 8 deliberate mutants were run by hand. All were caught, and the tree was
restored with `git checkout`.

| Mutant | Change | Caught by |
| --- | --- | --- |
| A | drop `!archived` in `counts_as_unread` | core `attention` tests |
| A2 | the same, seen from the sidebar | `the_location_rows_add_up_to_the_switchers_count` |
| B, B2 | drop the in-view check | `the_session_in_view_is_not_counted`, core `attention` |
| C | the sidebar `unread_count` uses `s.unread` | `a_closed_unread_session_adds_nothing_to_its_row`, in-view tests |
| D | `Count(n)` renders as `Unread` | 3 `tree_view` tests |
| E | `count_tint` returns the row tint | `the_count_is_drawn_in_the_text_colour_on_an_error_tinted_row` |
| E2 | `count_tint` returns `outline_variant` | `a_location_rows_unread_count_is_legible_in_every_row_state` |

## Traceability

| Criterion | Tests |
| --- | --- |
| US1.1, FR-001, FR-002 | `a_worktree_row_counts_its_unread_sessions_and_a_read_only_one_shows_none` |
| US1.2 | `the_default_row_counts_its_unread_sessions`, `a_project_without_worktrees_counts_on_its_default_row` |
| US1.3, FR-003 | `expanding_a_row_does_not_change_its_count` |
| US1.4 | `a_viewed_session_awaiting_input_does_not_count` |
| US1.5, FR-010 | `a_closed_unread_session_adds_nothing_to_its_row`, `switcher_unread::a_closed_unread_session_leaves_the_switcher_counts`, core `the_unread_count_leaves_out_a_closed_session`, `the_other_projects_total_leaves_out_closed_sessions`, `the_location_rows_add_up_to_the_switchers_count`, the two `*_still_counts_on_the_switcher` tests |
| US2.1, US2.3, FR-007 | `a_catalog_update_raises_and_lowers_the_count`, `a_closed_or_removed_session_lowers_the_count`, `a_burst_of_updates_settles_on_the_right_count` |
| US2.2, edge "session in view" | `selecting_a_session_drops_the_count_before_any_catalog_update`, `the_session_in_view_is_not_counted` |
| US2.4, US2.5, FR-008 | no 575 test: 039's one service and its persisted `unread` (R7). Quickstart B8 passed. B7 was not run |
| FR-004 | `tree_view`: `a_counted_location_row_has_the_height_of_a_plain_one`, `a_long_name_is_cut_short_before_the_count_is`, `the_count_comes_before_the_row_actions_and_does_not_move_on_hover`, `a_counted_row_keeps_its_label_role`, `a_count_of_zero_draws_nothing` |
| FR-005 | the six tooltip tests in `sidebar_attention.rs` |
| FR-009 | the three `expanding_collapsing_and_hovering_a_row_*` tests |
| FR-013 | `a_location_rows_unread_count_is_legible_in_every_row_state` |
| Edge "hidden worktree", "missing or invalid" | `a_hidden_worktree_has_no_row_to_carry_a_count`, `the_row_shown_for_the_current_session_carries_its_count`, `a_missing_or_invalid_worktree_carries_its_count` |

Every claimed test exists and runs: `cargo test -p micold-client --test sidebar_attention` gives 26 passed,
and `cargo test -p micold-core --lib workspace` gives 8 passed. The state tests go through the real entry
points: `State::update`, `reconcile_catalog`, `sidebar_entries` and `switcher_entries`.

## What was not audited

- Mutation was not measured by a tool. 8 hand-picked mutants are a sample, not a score.
- Multi-window agreement and restart (US2.4, US2.5, FR-008) have no 575 test. They rest on 039's tests and on
  quickstart B7 and B8, and B7 was not run.
- FR-011, FR-012 and FR-014 are checked by 039's regression tests, the showcase visual pass and the user-guide
  review, not by this audit.
- Performance and the macOS and Windows hosts (quickstart §C) were not assessed.
