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

## M2 — Back/Forward keys (T017–T023)

Red is reproducible from commit c520924b8 (`test(1465): failing tests … stubs in keymap`): the
tests and the stubbed `history_key` / `history_key_labels` / `history_key_label` are committed
there.

### Cycle 10 — H1 decode and labels (T017, T020)

- tests: `tests/keymap.rs` `the_platforms_back_and_forward_keys_decode`,
  `the_back_and_forward_labels_name_both_platform_forms`.
- red (c520924b8): both FAILED (`history_key` returned `None`, labels empty); 56 passed, 3 failed
  with the docs test below.
- green: `keymap::history_key` (Alt alone on ←/→; macOS Cmd alone on physical `[`/`]` with the
  logical fallback) and the labels → keymap 58 passed (docs test still red until cycle 13).
- guards, passed on arrival: `near_misses_of_the_back_and_forward_keys_decode_to_nothing`,
  `no_back_or_forward_key_is_another_chord`, `no_other_chord_is_a_back_or_forward_key`,
  `the_terminal_encodes_the_back_and_forward_keys_as_before` (H2, SC-003: `encode` is untouched;
  a stub returning `None` cannot fail them, the implementation could).

### Cycle 11 — routing (T018, T021)

- tests: `ui::app_chord_tests::the_back_and_forward_keys_press_the_history_from_the_keyboard`
  (press and repeat), `ui::overview_arrow_tests::a_back_or_forward_key_is_not_a_card_step`.
- red (c520924b8): both FAILED (`app_chord` gave `None`; `overview_arrow` gave `CardStepped`);
  `near_misses_of_the_back_and_forward_keys_are_not_app_chords` passed on arrival (guard).
- green: `app_chord` checks `history_key` after the other chords; `overview_arrow` returns `None`
  for a press it decodes → lib filter `app_chord overview_arrow history` 15 passed.

### Cycle 12 — the key gate (T019, T022)

- tests: `tests/history.rs` `a_key_press_does_nothing_while_a_field_holds_the_keyboard`,
  `a_key_press_does_nothing_while_the_terminal_holds_the_keyboard`.
- red (c520924b8): both FAILED at `assert_key_ignored` (the key stepped back).
  `a_key_press_acts_over_a_closed_comment_boxs_stale_focus`, `a_key_press_does_nothing_over_settings`
  passed on arrival (the stale-field rule and `navigation::blocked` are M1's).
- green: `resolve` returns `Step::None` for `Source::Key` while `terminal_focused()` or
  `live_focused_field()` → history 23 passed, 1 failed (cycle 12b).

### Cycle 12b — holding the key walks on through sessions (US2 AS4, FR-012)

- test: `three_key_presses_step_back_three_entries` (Overview → B → A → D by key).
- red: passed before the gate (nothing stopped a key), then FAILED once the gate was in: "back to
  A" — landing on B focused B's terminal, so the next press was the terminal's.
- green: `State::finish_history_move(source)` releases the terminal a key move focused (research
  R8, contracts/ui.md U4, ledger decision 6) → history 25 passed.
- guard, passed on arrival: `a_button_move_lands_on_the_terminal_as_a_click_would`.

### Cycle 13 — the guide (T023, FR-013)

- test: `tests/keymap.rs` `the_user_guide_lists_back_and_forward_with_their_keys_and_buttons`.
- red: FAILED — `Back row lists "Alt+←": | Back | Mouse back button | Mouse back button |`.
- green: terminal-panes.md rows and intro → keymap 59 passed.

### Cycle 14 — the Review tab's own Back first (T024, T025, FR-011, SC-005)

- tests: `tests/history.rs` `back_in_a_browsing_review_tab_is_the_tabs_own_back` (Button and Key →
  `Step::ReviewBack`, list untouched, `moving` unset, `apply_history` → `[Changes(BrowseBack)]`),
  `main_tests` `history_tests::back_in_a_browsed_review_tab_returns_to_the_reading_then_leaves_the_tab`.
- red (146392a08): both FAILED (`Show(Session feat-a)` where `ReviewBack` was expected; the bin
  test: Back left the tab with the browse still open). The root declines `Message::Changes`, so
  the history tests drive `changes::update` directly (test fixed before green; red re-confirmed
  with the implementation stashed).
- guards, passed on arrival: `back_in_a_review_tab_without_a_browse_frame_is_the_global_back`,
  `forward_from_a_browsing_review_tab_is_the_global_forward`.
- green: `resolve` returns `ReviewBack` for Back on a shown Review tab where
  `changes::can_go_back`; `apply_history(ReviewBack)` → `[BrowseBack]`; allowlist entry
  `history` → `changes` → history 28 passed, `feature_registration_cost` 7 passed, bin
  `history_tests` 4 passed.
- T026 is prose (guide sentence); no test.
