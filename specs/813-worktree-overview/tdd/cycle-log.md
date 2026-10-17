# TDD cycle log — 813 worktree overview

No `tdd/test-list.md` was planned for this feature (as for 905); the tasks' test-first order is the
list. Append only.

## M1 / T006–T008 → T015 (core card rules)

- **Tests**: `crates/micold-core/src/overview.rs` `tests` (15 tests: location type precedence and
  words, `card_state` exhaustive over 204 signal combinations against the table, invariants,
  `session_counts`, `order`, `attention`).
- **Red**: `scripts/build-lock.sh cargo test -p micold-core --lib overview::` with every function
  body stubbed `todo!()` → `test result: FAILED. 0 passed; 15 failed` (`panicked at
  crates/micold-core/src/overview.rs:43:21: not yet implemented`).
- **Green**: implementation restored → `test result: ok. 15 passed; 0 failed`.
- **Notes**: deviation — the implementation was drafted before the tests and stashed for the red
  run, so the red is a stub red, not an assertion red.

## M1 / T002 → T003 (warning and success roles)

- **Tests**: `crates/micold-core/src/tokens/css.rs` `COLOR_ROLES` + 8 names;
  `crates/micold-client/src/ui/material/composition_contrast.rs`
  `warning_and_success_roles_are_legible_on_their_fills`.
- **Red**: `scripts/build-lock.sh cargo test -p micold-core --lib tokens::` →
  `every_colour_role_is_emitted_as_a_hex_triple ... FAILED` (`light: --micold-warning is not emitted`).
- **Green**: roles + palette tones added → `test result: ok. 11 passed; 0 failed`. The client
  contrast test was not yet run (client build in progress at handover).

## M2 / T010 + T017 (Overview state and cards)

- **Tests**: `crates/micold-client/tests/overview.rs` (15 tests: toggle/close, R2, project switch,
  one card per sidebar entry and order, counts, R5 agent request, R6 PR states, Default never
  PR-derived, summary, R14 after `Released`, keyboard/main area, 30×3 build time).
- **Red**: with `State`/`Msg`/`update` wired and `cards` a stub returning `[]`:
  `test result: FAILED. 4 passed; 11 failed` (each on an empty card list). The keyboard/main-area
  test was already green: its wiring went in with the stub.
- **Green**: `cards` built from `sidebar_entries()`, sessions, agent prompts and `pr_status` →
  `test result: ok. 15 passed; 0 failed`.

## M2 / T014 (+ T022 routing) (selecting a session closes the Overview)

- **Test**: `main_tests.rs` `selecting_a_session_closes_the_overview_and_shows_it`.
- **Red**: `assertion failed: !app.core.overview.open`.
- **Green**: the `SessionMsg::Selected` arm sends `overview::Msg::Closed` →
  `test result: ok. 1 passed`.

## M2 / T011–T013, T018–T020 (OverviewCard, CardGrid, contrast, showcase)

- **Tests**: `overview_card.rs` (one height ×5 states, no overlap at 280 px with a long name, name
  tooltip, type/state icon + word), `card_grid.rs` (columns formula, columns when laid out, no card
  overlap, 30 cards scroll), `composition_contrast.rs` (card palette per state, both schemes),
  `showcase_completeness.rs` (both entries and their poses).
- **Red**: `card_grid` with `columns` a stub returning 0: `15 passed; 2 failed`
  (`columns_follow_the_width…` left 0 / right 1; `the_laid_out_grid…` 1 column, not 3). The card
  and contrast tests were written together with their implementation (`palette`, `words`), so they
  have no recorded red; the card height is a constant, so its test pins the constant.
- **Green**: `columns` = `max(1, floor((w+gap)/(min+gap)))` →
  `--lib overview_card card_grid composition_contrast`: `17 passed; 0 failed`;
  `--test showcase_completeness`: `38 passed; 0 failed`.

## T021–T023 — placement, button, docs, architecture fix

- **Architecture**: `feature_registration_cost` failed 3 (cross-feature names, no isolation test,
  `update` not shape A). `cards` moved to `ui::overview::cards`; `features::overview::update` now
  returns `Vec<Outcome>`; `tests/features_overview.rs` added. Green: `--test
  feature_registration_cost --test overview --test features_overview`: `7 + 15 + 2 passed`.
- **Key order**: `main_content_key` test (`overview_placement_tests`) was written with its
  implementation, so no recorded red. Green: `--lib overview_placement`: `1 passed`.
- **Button/docs**: toolbar button has no unit test (built from `Button`/`Tooltip`); docs checked by
  `documentation-set` and `check-user-guide-updated` scripts and `--test icons`: all pass.
- **Review fixes (F1-F4, G1)**: `starting_a_session_closes_the_overview` and
  `opening_changes_closes_the_overview` (tests/overview.rs) red first (`2 failed` at `assert!(!state.overview.open)`),
  green after the root closes the Overview on `Session::Started`, `ChangesRequested` and
  `ChangesClosedForCompare`: `--test overview`: `17 passed`.
- **Layout/toolbar**: `thirty_cards_lay_out_bounded_in_a_short_window` (ui/overview.rs) and
  `the_overview_button_needs_an_active_project_and_no_settings` (toolbar.rs) were written with the
  fix, no recorded red. The key-order test moved into `ui::overview::tests` (G1: `ui/mod.rs` keeps
  no inline test module). `leaves`/`overlap` now live in `test_support`. Green: `--lib`: `657 passed`.
