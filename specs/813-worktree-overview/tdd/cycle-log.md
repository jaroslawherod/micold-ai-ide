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

## M2 / T025, T026 → T028, T029 (client: counts carried, card footer)

- **Tests**: `tests/features_pr_status.rs` (`StatusesRead` stores `counts`; `Unavailable`, `Released`, disabling clear it with `statuses`) and `tests/overview.rs` (1 open / 2 merged → `Counts`; each unknown reason; `Hidden` for Default, detached, orphan; open PR with `conflict` → *merge conflict*).
- **Red**: both files failed to compile before the change (`no field counts on State`, `no field prs on CardModel`, `PrDisplay` unresolved).
- **Green**: `State::counts`, `Outcome::Ok.counts`, `CardModel::prs`, `pr_display` in `ui/overview.rs`, footer line in `overview_card.rs`; results in the M2 commit message.

## M3 / T031, T032, T035, T036 → keymap (chord, pane capture, unfocused subscription)

- **Tests**: keymap tests for `KeyRouting::ToggleOverview`, `is_overview_chord` and `overview_chord_label`; a `terminal_pane.rs` test that the focused chord is published as `Overview(Toggled)` and captured; the unfocused chord handled by the `ui::subscription` listener.
- **Red**: observed by the implementing workers as compile errors (`ToggleOverview` variant, `is_overview_chord`, `overview_chord_label` not found). The T032 pane test's red was **not observed separately**: it was written alongside the `ToggleOverview` routing and went green with it.
- **Green**: `--test keymap` 27 passed; `--lib` 663 passed.

## M3 / T033, T037 → Button::selected / mark, UnreadMark::icon

- **Tests**: showcase and component tests for a selected `Button`, a `Button` carrying a mark, and an `UnreadMark` with an icon.
- **Red**: compile errors observed by the implementing workers (`Button::selected`, `Button::mark`, `UnreadMark::icon` missing).
- **Green**: `--test showcase_completeness` 38 passed; `--lib` 663 passed.

## M3 / T034, T038 → overview::badge

- **Tests**: `tests/overview.rs` cases for the badge: `Error` for a card needing action, `Warning` for attention, none when no card needs the user.
- **Red**: compile errors observed by the implementing workers (`badge`, `Badge`, `Tone` missing in `features::overview`).
- **Green**: `--test overview` 27 passed.

## M4 / T040, T042, T043 → T044, T047 (activation rule, focused bounds, pressable card)

- **Tests**: `micold-core/src/overview.rs` (`activation_opens_the_main_session_when_there_is_one`, `activation_offers_to_start_where_there_is_no_session`, `activation_only_reveals_an_orphan_or_invalid_location`, `an_idle_card_activates_as_any_other`); `ui/focus.rs` (`the_focused_controls_bounds_are_reported_after_the_scrollables_translation`, `nothing_is_reported_when_nothing_is_focused`); `ui/material/overview_card.rs` (`a_card_takes_the_keyboard_only_with_a_press`, `enter_and_space_on_the_focused_card_publish_its_message`, `a_click_on_the_card_publishes_its_message`, `the_focused_card_draws_the_focus_ring`).
- **Red**: compile red, observed: `cargo test -p micold-core --lib overview` → `cannot find function activation` / `use of undeclared type Activation`; client lib → `cannot find function focused_bounds`, `no method named on_press found for struct OverviewCard`, `missing field activation in CardModel`. The ring test's first version (pixel within 8 of `secondary`) also failed green-side on `no ring on the focused card`: the antialiased stroke blends with the fill (`[164,158,171]` vs `[98,91,113]`), so it was changed to "nearer the ring colour than at rest by 60" before it was kept.
- **Green**: `micold-core overview` 19 passed; client `--lib` 670 passed (`overview_card` and `focus` filters 74); `--test overview` 27, `--test context_menu_anchor_call_sites` 2, `--test showcase_completeness` 38; clippy `-D warnings` clean. `Mounted::drive` / `focus_next` / `drawn` moved into `material/test_support.rs` so the focus tests share them.

## M4 / T041 → T045, T046, T048, T049 (activation follow-up, Escape, arrows, reveal)

- **Tests**: `tests/overview.rs` (`activating_a_card_with_a_session_closes_and_selects_it`, `activating_a_card_without_a_session_reveals_the_row_and_offers_the_start_menu_only`, `activating_an_orphan_card_only_reveals_its_row`, `the_revealed_row_clears_on_the_next_sidebar_selection`, `escape_closes_the_overview_after_the_overlay_registry_and_the_settings_draft`, `arrow_keys_move_the_focus_and_other_keys_do_not`).
- **Red**: with `SidebarState::revealed`, `arrow_focus` and `follow_up` stubbed (always `None`), `--test overview` → 27 passed, 6 failed on the assertions (no `revealed` set, `follow_up` is `None`, Escape gives `None`, arrows give `None`).
- **Green**: `overview::arrow_focus` / `follow_up`, `revealed` set in the `Message::Overview` arm and cleared in `session::selected`, `on_escape` last arm, `main.rs` `overview_message` (OfferStart: scroll, focus the start control by `start_control_id`, `scroll_focused_into_view`, `focused_bounds_task`, `StartMenuAnchored`, `StartMenuOpened`), arrow listener in `ui::subscription`, first card focused on open (`FIRST_CARD_ID`), revealed row drawn selected. `--test overview` 33 passed, `context_menu_anchor_call_sites` 2, client `--lib` 670, `micold-core overview` 19; fmt and clippy `-D warnings` clean. The `main.rs` chains and the on-open focus are not unit-testable (runtime tasks); quickstart B8, B9 cover them.

## M4 review fixes (collapsed group, repo-wide invariants)

- **Tests**: `tests/overview.rs::a_card_whose_worktree_sits_in_a_collapsed_group_has_its_row_drawn_once_activated`; the harness invariants `feature_registration_cost`, `feature_write_isolation`, `features_are_render_free`, `material_boundary`, `root_is_routing_only`.
- **Red**: the four invariant tests failed on the M4 diff (overview named `session`; `session::selected` wrote `sidebar.revealed`; `arrow_focus` named iced; `ui/focus.rs` named `scrollable(`); the collapsed-group row was not drawn after activation (`location_row` is `None`).
- **Green**: `follow_up`/`arrow_focus` moved to `ui::overview`; `Outcome::RevealMarkCleared` / `LocationRevealed` interpreted by the root (the latter also opens the row's run group); the `main.rs` OfferStart chain dismisses the start menu first and opens nothing without bounds; `ui/focus.rs` test uses `Scrollable::new`.

## M5 / T050–T060 (task progress, ticket, watch targets, titles, summary, progress bar, project-switch drop)

- **Tests**: `micold-core` `task_progress` and `naming::ticket` tests; `shell/overview_watch.rs::the_watch_targets_are_specs_and_specify_recursively`; `tests/overview.rs` (T052 summary and progress-reading tests, T053 title tests, `a_read_for_a_project_no_longer_active_is_dropped`); `shell/issues.rs::title_tests::the_failure_line_names_the_number_and_never_a_title`; `ui/material` ProgressBar geometry tests; `shell/changes_watch.rs::a_missing_target_is_retried_until_it_exists`.
- **Red**: `task_progress` and `naming::ticket` (core): failed on the `todo!()` panics, and `ticket` returned `None`. T052/T053 (client): compile red only, against the missing API (`Msg::ProgressRead`, `TitleRead`, `TitlesRequested`, `titles_to_fetch`, `summary`), no assertion-level red. ProgressBar geometry: red as for T052/T053, no assertion-level red recorded. `overview_watch` targets test and the `issues.rs` log-line test: no recorded red. Project-switch drop: compile red, observed: `Msg::ProgressRead` / `Msg::TitleRead` have no field named `project`. The retry test went in with its fix: no recorded red.
- **Green**: `task_progress`, `naming::ticket`, the overview module (`summary`, `titles_to_fetch`, `numeric_ticket`), `shell/overview_watch.rs`, `shell/issues.rs` title reads. Review fixes: `ProgressRead` and `TitleRead` carry the project they started in and `app.rs` drops one whose project is no longer `workspace.active`; `watch_loop` retries targets that did not exist (`watch_targets`) after each burst, and the Overview also watches the worktree root non-recursively so a later `specs/` is noticed. `TitlesRequested` stays before `send_op`: `wanted` is empty unless the route and outbox exist, and a lost daemon settles the titles as unavailable.
