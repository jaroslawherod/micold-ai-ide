# TDD cycle log — 575 sidebar attention indicator (M1)

No `tdd/test-list.md` was derived at design time; as in 582, the test-first task pairs of
tasks.md (T001/T002, T003-T004/T006-T007, T005/T008, T014/T006) are the test list. Each red was
run against stubs that compile and return the "nothing" answer, and failed on its assertion.

| Behaviour | Tasks | Red (stub) | Green |
|---|---|---|---|
| A session counts only when unread, not closed, not in view | T001 → T002 | `attention::counted_session_tests::an_unread_session_out_of_view_counts ... FAILED` (stub `counts_as_unread` = false; 2 of 5 fail, the 3 "does not count" cases pass by construction) | `cargo test -p micold-core --lib attention` ok |
| A location row's count over its sessions, expanded or not, hidden rows excluded | T003 → T006 | `sidebar_attention`: 12 failed, 4 passed (stub `unread_count` = 0), e.g. `a_worktree_row_counts_its_unread_sessions_and_a_read_only_one_shows_none` left `Some(0)` right `Some(2)`; the passing 4 are the zero/hidden cases and the "unchanged at 0" tooltip | 16 passed |
| Tooltip line in words | T004 → T007 | `one_unread_session_is_singular`, `several_unread_sessions_are_plural`, `the_line_follows_*` FAILED (stub `None` / unchanged) | passed |
| Row geometry, slot, label role, last builder wins, count colour | T005 → T008 | `tree_view`: 5 failed, 6 passed; `a_long_name_is_cut_short_before_the_count_is` "the row has no 8dp mark", `the_count_is_drawn_in_the_text_colour_on_an_error_tinted_row` left error `Rgb{186,26,26}` right `on_surface`, `the_last_unread_builder_call_wins` | 11 passed |
| Live updates through `reconcile_catalog` and view; FR-009 | T014 (over T006) | with `unread_count` stubbed to 0 again: all 5 new tests FAILED at their count assertion (e.g. `a_catalog_update_raises_and_lowers_the_count` left `Some(0)` right `Some(1)`); `expanding_collapsing_and_hovering_a_row_read_nothing` passed its unread and view-report assertions and failed only at the count | 21 passed |
| Count legible in every row state (contrast) | T010 | test-after (the count colour was fixed by T008's red); passes at first run | composition_contrast ok |
| Closed sessions leave the switcher's counts (M2) | T015-T017 → T018 | core `the_unread_count_leaves_out_a_closed_session`, `the_other_projects_total_leaves_out_closed_sessions` and `sidebar_attention::the_location_rows_add_up_to_the_switchers_count` FAILED before T018 (recorded in the ledger, *Environment notes (M2)*). The test and the source landed in one commit (`7303f78e`), so history cannot show the order | passed |
| Count legible in every row state, red after the fact (T022) | T010 | mutant E2 (`count_tint` returns `outline_variant`): `a_location_rows_unread_count_is_legible_in_every_row_state` FAILED; restored with `git checkout` | composition_contrast ok |
