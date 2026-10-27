# Cycle Log: #1465 — Back/forward navigation

Append only. Newest last. Every entry's `red` block is the evidence that the test failed without
the implementation it covers.

## Baseline

- M1 (T001–T016) landed in `81c5982a1`, `dac1d694d`, `02623b875` (+ two test-only follow-ups) with
  the tests written alongside the code, not run red first (review B M1 F1). The full gate was
  green at `4573c0ab8`. Cycles 1–8 below are the red evidence recovered after the fact: each
  behaviour's implementation was stubbed, the tests run red, the stub removed, the tests run green.

## Cycles 1–8 — M1 behaviours, red by stub (recovered evidence), grouped

- Method: one temporary build at `47db7b665` with eight stubs, each switched on by the env var
  `TDD_STUB=<name>` and inert otherwise, so one build served every red run (each build waits on
  the shared `target-shared` lock). The stubs were never committed; `git checkout` restored the
  three client files and the core stubs were deleted by hand, then the green run below.
- runs, per stub (`TDD_STUB=<name>`), each:
  `cargo test -p micold-core --lib history::`;
  `cargo test -p micold-client --no-fail-fast --test history --test features_history`;
  `cargo test -p micold-client --bin micold-ai-ide -- history side_button`.
- `TDD_STUB=none` (control): every M1 test passes. The only failures are the two new review-A
  tests of cycle 9, which have no implementation yet.

### Cycle 1 — record (T001, T002: FR-002–FR-005)

- stub: `History::record` returns at once.
- red: core 12 failed (`recording_the_current_entry_adds_nothing`,
  `recording_after_back_drops_the_forward_entries`, `sixty_records_keep_the_newest_fifty`,
  `stepping_past_either_end_changes_nothing`, `a_gone_entry_is_skipped_and_removed`,
  `every_entry_behind_gone_does_nothing_and_removes_them`,
  `removal_merges_neighbouring_equal_entries`, `an_entry_equal_to_shown_is_passed_over_and_kept`,
  `step_never_records`, `places_are_equal_when_what_is_shown_is`,
  `every_step_lands_on_an_existing_other_entry_or_does_nothing`, + cycle 9's); client
  `features_history` 1 failed, `tests/history.rs` 15 failed (every U1 recording test, every walk);
  binary `history_tests` 3 failed.
- green: stub removed → all pass.

### Cycle 2 — step (T001, T002: FR-006, FR-007, US4 skip-and-remove)

- stub: `History::step` returns `None`.
- red: core 9 failed (every stepping test, the SC-002 sweep included); `tests/history.rs` 7 failed
  (`back_and_forward_walk_the_sessions`, `an_archived_session_is_skipped_and_removed`,
  `a_removed_worktrees_places_are_skipped_and_removed`,
  `another_projects_place_is_skipped_and_removed_until_cross_project_moves`,
  `a_button_press_acts_while_a_field_holds_the_keyboard`,
  `finishing_a_move_records_only_a_place_other_than_the_target`, + cycle 9's); binary
  `history_tests` 3 failed. Recording tests stay green (step never records).
- green: stub removed → all pass.

### Cycle 3 — observe after every message (T003–T006: U1)

- stub: `features::history::observe` is a no-op.
- red: `features_history::observing_records_a_shown_place_once_and_nothing_while_moving`;
  `tests/history.rs` 14 failed (`a_sidebar_selection_records_one_entry`,
  `a_quick_switcher_choice_records_one_entry`, `a_navigation_chord_step_records_one_entry`,
  `the_overview_compare_and_a_review_tab_each_record_one_entry`,
  `the_shown_sessions_removal_records_where_the_app_moved`,
  `re_selecting_the_shown_session_records_nothing`, `two_windows_keep_independent_histories`,
  `nothing_is_observed_while_a_move_is_under_way`, and every walk); binary `history_tests` 3
  failed. The gate test stays green.
- green: stub removed → all pass.

### Cycle 4 — exists (T010, T012: research R5, active project only in M1)

- stub: `features::history::exists` returns `true`.
- red: `an_archived_session_is_skipped_and_removed`,
  `a_removed_worktrees_places_are_skipped_and_removed`,
  `another_projects_place_is_skipped_and_removed_until_cross_project_moves` (+ cycle 9's).
- green: stub removed → all pass.

### Cycle 5 — the gate (T007, T012: U3 blocked)

- stub: `resolve` ignores `navigation::blocked`.
- red: `a_press_does_nothing_over_settings_a_dialog_the_switcher_or_the_review_sheet`
  (+ cycle 9's).
- green: stub removed → all pass.

### Cycle 6 — apply (T007, T013: U4, research R6)

- stub: `State::apply_history` returns no messages for a `Step::Show`.
- red: `tests/history.rs` `apply_history_sends_each_views_messages`,
  `back_and_forward_walk_the_sessions`, `a_button_press_acts_while_a_field_holds_the_keyboard`,
  `finishing_a_move_records_only_a_place_other_than_the_target` (+ cycle 9's); binary
  `back_twice_then_a_new_selection_drops_the_forward_entries`,
  `back_from_the_overview_shows_the_previous_session_with_the_overview_closed`,
  `back_from_compare_shows_the_review_tab`.
- green: stub removed → all pass.

### Cycle 7 — finish a move (T006, T013: research R2)

- stub: `features::history::finish` clears `moving` but does not observe.
- red: `features_history::observing_records_a_shown_place_once_and_nothing_while_moving`,
  `tests/history.rs` `finishing_a_move_records_only_a_place_other_than_the_target`
  (+ cycle 9's).
- green: stub removed → all pass.

### Cycle 8 — side-button decode (T008, T009: H4, FR-008)

- stub: `shell::subscriptions::side_button` returns `None`.
- red: binary `shell::subscriptions::tests::side_button_maps_back_and_forward_presses_whatever_their_status`.
- green: stub removed → all pass.
- guard, passed on arrival: `terminal_pane` `a_side_button_press_in_a_focused_pane_never_reaches_the_process`
  (T011) — the pane already reported only Left/Middle/Right presses, so FR-008's "the program never
  receives them" needed no pane change; its live-harness assertion (a Left press *is* reported in
  mouse-reporting mode) shows the test can fail.

## Cycle 9 — Back with nothing shown returns to the current entry (review A M1 F1)

- tests: `micold-core` `history::tests::back_with_nothing_shown_returns_to_the_current_entry`
  (current entry first; a gone current entry removed and Back goes on; Forward unchanged; sweep of
  every gone subset and position with the invariants); `tests/history.rs`
  `back_with_nothing_shown_returns_to_the_place_just_left` (D → A → B, nothing shown, Back shows B
  then A).
- red: run before the fix (the `TDD_STUB=none` control above, and again with the fix reverted):
  core `assertion left == right failed: back to the current entry — left: Some(1)` (it skipped to
  the entry before); client `back_with_nothing_shown_returns_to_the_place_just_left` failed (Back
  showed A).
- green: `History::step` starts Back one past the position when `shown` is `None`, so the current
  entry is the first candidate; the position is clamped after a removed last entry. Core 21
  passed; `tests/history.rs` 19 passed, `features_history` 2 passed, binary `history`/`side_button`
  5 passed. Research R3 records the rule.
- refactor: none needed.
