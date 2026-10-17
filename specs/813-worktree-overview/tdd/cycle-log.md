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
- **Green**: roles + palette tones added → `test result: ok. 11 passed; 0 failed`. Client: `cargo test -p micold-client --lib composition_contrast` → `test result: ok. 9 passed; 0 failed`
  (9, not 8, now the card-state pairs are in).

## M1 / T010 + T017 (Overview state and cards)

- **Tests**: `crates/micold-client/tests/overview.rs` (15 tests: toggle/close, R2, project switch,
  one card per sidebar entry and order, counts, R5 agent request, R6 PR states, Default never
  PR-derived, summary, R14 after `Released`, keyboard/main area, 30×3 build time).
- **Red**: with `State`/`Msg`/`update` wired and `cards` a stub returning `[]`:
  `test result: FAILED. 4 passed; 11 failed` (each on an empty card list). The keyboard/main-area
  test was already green: its wiring went in with the stub.
- **Green**: `cards` built from `sidebar_entries()`, sessions, agent prompts and `pr_status` →
  `test result: ok. 15 passed; 0 failed`.

## M1 / T014 (+ T022 routing) (selecting a session closes the Overview)

- **Test**: `main_tests.rs` `selecting_a_session_closes_the_overview_and_shows_it`.
- **Red**: `assertion failed: !app.core.overview.open`.
- **Green**: the `SessionMsg::Selected` arm sends `overview::Msg::Closed` →
  `test result: ok. 1 passed`.

## M1 / T011–T013, T018–T020 (OverviewCard, CardGrid, contrast, showcase)

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

## M1 / Visual-pass finding (FR-002: the type's word is visible text on the card)

- **Test**: `a_card_shows_its_type_and_state_with_their_words` now expects the footer
  `"Feature · 3 sessions, 2 running"`.
- **Red**: `assertion left == right failed: FR-002 …` (`left: "3 sessions, 2 running"`,
  `right: "Feature · 3 sessions, 2 running"`); `test result: FAILED. 4 passed; 1 failed`.
- **Green**: `words().footer` leads with `kind.label()` (tooltip kept) →
  `--lib overview_card`: `test result: ok. 5 passed; 0 failed` (height ×5 and 280 px tests unchanged).

## M1 / Assertion-red evidence, after the fact (red by mutation, after the fact)

Each line of implementation was broken once, the test run, the code restored byte-for-byte and the
test rerun green.

1. `card_state` (core): deleted the `agent_request` row. `cargo test -p micold-core overview`:
   `card_state_follows_the_table_… panicked … left: (Idle, Idle) right: (NeedsAction, AgentRequest)`.
   Green: `test result: ok. 15 passed`.
2. `OverviewCard`: `Fixed(HEIGHT + 1.0)` → `the_card_has_one_height_in_all_five_states` panicked with
   `[137.0, 137.0, 137.0, 137.0, 137.0]`; `state_label: model.kind.label()` →
   `a_card_shows_its_type_and_state_with_their_words … left: "Feature" right: "Needs action"`.
   Green: `--lib overview_card`: `ok. 5 passed`.
3. `palette(Done)` text role = fill role → `an_overview_cards_text_and_icon_are_legible_on_its_state_fill`:
   `Light / Done text: 1.00:1 (needs 4.5)`. Green: `--lib composition_contrast`: `ok. 9 passed`.
4. `main_content_key` returning `u64::MAX - 2` → `the_overview_key_sits_between_settings_and_changes`:
   `left: 18446744073709551613 right: 18446744073709551612`. Green: `--lib`: `ok. 657 passed`.
5. `CardGrid` without its own `Scrollable` (the grid unbounded, as under an outer scrollable):
   `thirty_cards_scroll_vertically` and `the_laid_out_grid_has_the_columns_its_width_gives` went red
   (`left: 0 right: 6`), but **`thirty_cards_lay_out_bounded_in_a_short_window` stayed green**: it
   builds its own `container(...Length::Fill)`, which clamps the height whatever the grid does, and
   it never calls `view`, so it cannot see an outer scrollable. No assertion-red for it; a stronger
   test needs `view` with a `State` (left open).
6. `overview_press` without the `workspace.active` check →
   `the_overview_button_needs_an_active_project_and_no_settings` panicked `no project`.
   Green: `--lib`: `ok. 657 passed`.
7. `TypeOrphan` mapped to `TypeIncluded`'s `\u{e157}`: `--test icons`:
   `TypeOrphan must map to its pinned codepoint`, `TypeIncluded must not share a glyph with
   TypeOrphan`; `test result: FAILED. 2 passed; 3 failed`. Green: `ok. 5 passed`.
8. `Prompt.session` = `SessionId(Default::default())` in `daemon_sync.rs`: bin test
   `a_queued_prompt_names_the_calling_session`: `the prompt carries the wire's caller`
   (`left: Some(SessionId(0000…)) right: Some(SessionId(dcd777dd-…))`). Green: `ok. 1 passed`.
   `--test features_agent_confirm` stayed green under the mutation (`ok. 12 passed`): it builds
   its own `Prompt`, so it does not pin the caller; the bin test is the pin.

## M2 / T024 → T027 (core: counts and conflict in the status reading)

- **Tests**: `crates/micold-core/tests/pull_request_counts.rs` (6 tests: query text, counts from the recorded
  answer, forks left out, 100 → capped, missing `oc`/`mc` → `Passing`, `conflict` mapping).
- **Red**: `cargo test -p micold-core --test pull_request_counts`: `unresolved import
  micold_core::overview::PrCounts`, `unresolved import micold_core::pull_request::StatusReading`
  (compile red: the API did not exist).
- **Green**: `StatusReading`, `PrCounts`, `oc`/`mc` aliases, `mergeable`, `PullRequestStatus::conflict`;
  `--test pull_request_{counts,parse,query,source,select,list,diff,failure,is_never_stored}` all `ok`
  (counts 6 passed). Recorded `pr_*.txt` answers gained derived `oc{i}`/`mc{i}` aliases; the pinned
  query texts (`pull_request_query.rs`, `status_query_50.graphql`) were updated.
