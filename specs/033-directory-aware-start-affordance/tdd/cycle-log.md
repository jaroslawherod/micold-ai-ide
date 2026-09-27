# Cycle Log: The start affordance answers for its own directory

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and
failed before the implementation.

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace` -> not yet measured
- commit: `7cca563c` (test list planned against it)
- recorded: cycle 0. The design PR changes no code, and the full suite was queued behind other
  worktrees' builds on the shared lock, so the run was stopped. M1 starts from a fresh
  `origin/main` after this PR merges. Its first entry below runs the suite on that base and
  records the counts before any red.

## Baseline (measured, M1)

- suite: `scripts/build-lock.sh cargo test --workspace` -> 3564 passed, 0 failed, 8 ignored, 337
  binaries (exit 0)
- commit: `84d61968` (`origin/main` with PR #417 merged)
- recorded: before any M1 change

## Notes and deviations (M1)

- **Batched reds.** A client test build takes minutes on the shared target dir, so the M1 cycles
  group the behaviors of one test file into one red run: every test in the batch is written first,
  run against a stub that compiles (default values, no behavior), and each is observed failing on
  its own assertion. A test that passed against the stub was held to the deliberate-mutant check
  after green. The full suite runs at the end of each phase (and in `mise run gate`), not per
  behavior; the inner loop uses `cargo test -p micold-client --test <file>`.

## Cycle 1: U1–U10, U16–U21 — the per-directory store and which rows exist

- test: `crates/micold-client/tests/directory_availability.rs` (new), 16 tests
- stub: `AvailabilityAnswers` / `AvailabilityKey` / `EnvIncludeSettings` with default-valued
  bodies, `app::State::location_dir` returning `None`, `wanted_availability_dirs` returning an
  empty set
- red: `scripts/build-lock.sh cargo test -p micold-client --test directory_availability`
  -> `test result: FAILED. 3 passed; 13 failed`. Decisive lines, e.g.
  `a_directory_reads_its_own_answer_and_falls_back_to_home`: `assertion left == right failed: a row
  whose own answer is pending reads the home answer / left: None`;
  `unasked_lists_only_directories_neither_held_nor_asked`: `P is held and Q is in flight, so only
  R needs asking / left: []`; `the_wanted_set_is_the_rows_on_screen`: `left: {}`;
  `env_include_changed_reports_only_a_real_change`: `unset → set is a change`.
  Passed on the stub: U4 `an_answer_to_an_unknown_request_is_dropped`, U19
  `a_worktree_whose_directory_is_gone_is_not_asked_about`, U21 `no_project_no_wanted_directories`
  (all three assert an absence).
- green: `features/session.rs` `AvailabilityAnswers` operations per data-model.md;
  `app.rs` `location_dir` through `SessionLocation::cwd`; `wanted_availability_dirs` over
  `visible_worktrees()` filtered by `can_start_session()`. -> `16 passed; 0 failed`
- mutant check (the three stub-passers): unknown `req` filed under home and `true` returned;
  `can_start_session` filter dropped; no-project case returning `{""}` ->
  `an_answer_to_an_unknown_request_is_dropped`, `a_worktree_whose_directory_is_gone_is_not_asked_about`,
  `no_project_no_wanted_directories` all FAILED (with `clear_…` and `retain_…`); code restored,
  16 passed again
- refactor: none needed
- commit: the commit that adds this entry

## Cycle 2: U11–U15 — the readers take the row's directory (with T040's migration)

- test: `crates/micold-client/tests/features_session.rs` (extended): `the_chevron_follows_the_rows_own_answer`,
  `the_primary_press_reads_the_rows_answer`, `a_default_missing_in_the_row_opens_its_list_marked`,
  `an_empty_answer_is_an_answer_not_a_fallback`, `settings_reads_home_only`
- stub: T040's stage — every reader takes a directory and reads the home answer (today's
  behaviour); every consumer and the T007 tests migrated onto the new API
- red: `scripts/build-lock.sh cargo test -p micold-client --tests --no-fail-fast`
  -> `features_session`: `28 passed; 5 failed`, e.g. `the_primary_press_reads_the_rows_answer`:
  `left: OfferChoice { providers: [ClaudeCode], unavailable_default: Some(Pi) } right: Start(Pi)`;
  `an_empty_answer_is_an_answer_not_a_fallback`: `left: Start(ClaudeCode) right: NothingAvailable`.
  The same run failed `cli_availability_comes_from_the_service::no_client_source_probes_this_process_for_a_cli`
  on the reader's data-model name `available_in(`, which is the forbidden spelling of
  `micold_core::provider`'s PATH probe; the reader was renamed `known_clis` (not a test change).
- green: `session::State::known_clis` reads `for_dir(d)` for `Some(d)` and `home()` for `None`
  -> `features_session` 33 passed
- refactor: none
- commit: squashed with cycle 3 (a WIP commit made on resume, `420e9def`, already carried the green
  reader body; folded in so no `wip` commit reaches main)

## Cycle 3: A1–A7, U22–U27, U35–U36 — rows are asked for, and read, their own directory

- tests: `crates/micold-client/src/main_tests.rs` (new, each named `availability_*`), plus
  `provider_choice_surfaces.rs::{the_settings_select_lists_the_home_answer, the_start_list_lists_its_rows_answer}`
  and `unavailable_default_says_so.rs::a_default_the_lists_directory_provides_is_not_reported_missing`
  (placed beside `start_menu_toggled`'s other notice tests rather than in
  `missing_cli_is_reported_where_it_is_chosen.rs`, which covers the Settings sentence);
  `an_answer_to_an_earlier_question_does_not_replace_a_later_one` rewritten per T007 (FR-002/FR-009
  reverse its premise)
- red: `scripts/build-lock.sh cargo test -p micold-client --tests --no-fail-fast`
  -> `micold-ai-ide`: `212 passed; 13 failed`, e.g.
  `availability_connect_asks_home_and_each_row_directory_once`: `left: [None] right: [None,
  Some("/repo/demo"), Some("/repo/demo/.claude/worktrees/feat-a")]`;
  `availability_a_start_list_asks_for_its_own_directory`: `and not as the home answer / left:
  Some([ClaudeCode, Pi])`; `availability_the_primary_press_starts_a_default_only_the_row_provides`:
  `left: OfferChoice {..} right: Start(Pi)`; `availability_opening_a_project_asks_once_per_directory`:
  `left: [] right: [Some("/tmp/.tmp…")]`; `availability_a_new_worktree_is_asked_about_once`:
  `left: []`; `availability_opening_one_rows_list_does_not_change_another_row`: `B offers no choice`.
  `unavailable_default_says_so`: `left: Some("Pi Coding Agent isn't installed. …") right: None`.
  Passed on the stub: `availability_settings_asks_for_home_only` (U24 — Settings already asked
  `cwd: None` only; it pins that the new keyed ask kept it), `the_settings_select_lists_the_home_answer` (U35).
- test fix before green: U35/U36 matched the menu string `"Pi"`, but menus paint
  `display_name()` ("Pi Coding Agent"), so U35 was vacuous and U36's red was not the behaviour's.
  Both now compare with `AiCli::Pi.provider().display_name()`.
- green: T013 `ask_cli_availability(app, AvailabilityKey)` records the key it asks; T014
  `sync_cli_availability` + `on_connected` clear → record env-include → ask Home → sync; T015 sync
  after `CatalogChanged`'s `reconcile_catalog(.., true)`, `open_verified_project` and
  `on_known_project_reopened`; T016 `StartMenuOpened` asks `Dir(location_dir)`, Settings asks
  `Home`; T017 the start list and the missing-default notice read the list's directory (sidebar rows
  already pass `location_dir` since T040). -> all client targets green, 0 failed
- mutant check (U35, U36): Settings fed `for_dir(project root)` and the start list fed
  `offered_providers(None)` -> both FAILED; restored, 6 passed
- refactor: none needed
- commit: the commit that adds this entry
