# Cycle Log: Report a Missing Environment-Include Script

Append only. Newest last. Every entry's `red` block is the evidence that the test
existed and failed before the implementation.

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace` -> 3662 passed, 0 failed, 8 ignored
  (341 binaries)
- commit: `0f7e0c8f`
- recorded: cycle 0, before any change. The spec, plan and tasks edits for feature 035 were
  uncommitted in the working tree at the time, and they touch no code.

## Notes and deviations

- The outer loop runs at the binary's `App` in `crates/micold-client/src/main_tests.rs`, not an
  end-to-end GUI runner. See `test-list.md`, "Entry point".
- U63 is M1's interim behaviour and is meant to be superseded in M3 (T024). Its removal there is
  planned, not a regression.
- A2–A4 and U57 assert an absence (no lines, no probe call) and may pass once they compile. T033
  records a deliberate mutant as their red evidence.
- A9 and A10 (US3) exercise behaviour that M1 and M2 already ship (FR-009's check after every save). If
  they pass on arrival, T036 records a deliberate mutant as their red evidence.
- M1 (autopilot): the baseline was re-run at `f954674d` before the first cycle:
  `scripts/build-lock.sh cargo test --workspace` -> 3711 passed, 0 failed, 8 ignored (351 binaries).
- M1 batching: the tests of one contract group (C1–C6, P1–P8, C7–C8, S1–S4/S8, N rows) are written
  together against a stub, and each test's own failure is read from that one run before any of the
  group's implementation exists. The inner loop runs the test's own target; the full suite runs at
  the end of the milestone (`mise run gate`, T033), because a 6-minute workspace run per behaviour
  is the profile's "suite too slow" case.

## Cycle 1: U1–U10 `classify` (contract C1–C6)

- test: `crates/micold-core/tests/script_path_check.rs`, ten tests (names in test-list.md) (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test script_path_check` against a stub
  `classify` that returned `Some(Unchecked)` without probing -> 0 passed, 10 failed:
  - U1 `an_empty_path_has_no_check_and_examines_nothing`: `left: Some(Unchecked) right: None`
  - U2 `a_whitespace_only_path_...`: `left: Some(Unchecked) right: None`
  - U3 `a_tilde_path_is_not_found_without_a_probe`: `left: Some(Unchecked) right: Some(NotFound { tilde: true })`
  - U4 `a_backslash_tilde_path_is_not_found_on_every_os`: `left: Some(Unchecked) right: Some(NotFound { tilde: true })`
  - U5 `a_name_that_merely_starts_with_a_tilde_is_relative`: `left: Some(Unchecked) right: Some(Relative)`
  - U6 `a_relative_path_is_not_checked`: `left: Some(Unchecked) right: Some(Relative)`
  - U7 `an_absolute_path_that_is_a_file_is_present_and_probed_once_as_stored`: `left: Some(Unchecked) right: Some(Present)`
  - U8 `an_absolute_path_with_nothing_there_is_not_found`: `left: Some(Unchecked) right: Some(NotFound { tilde: false })`
  - U9 `an_absolute_path_that_is_not_a_file_is_not_readable`: `left: Some(Unchecked) right: Some(NotReadable)`
  - U10 `an_absolute_path_that_cannot_be_opened_is_not_readable`: `left: Some(Unchecked) right: Some(NotReadable)`
- green: `classify` in `crates/micold-core/src/script_path_check.rs` (blank, `~`, relative, probe, in
  R2's order) -> 10 passed
- refactor: none needed
- notes: `FakeScriptPathProbe::answering` and `calls()` (T007's double) were written with the tests,
  since the tests need them to compile; they record calls behind a `Mutex` so the fake is
  `Send + Sync`.

## Cycle 2: U11–U18 `StdScriptPathProbe` (contract P1–P8)

- test: `crates/micold-core/tests/script_path_check.rs`, eight tests on `tempfile` dirs (new)
- red: same command, against `StdScriptPathProbe::probe` stubbed as `todo!("T008")` -> 10 passed,
  8 failed, each with `not yet implemented: T008` (the deliberate not-implemented signal: any
  constant answer would have let one of P1–P8 pass on a stub)
- green: `metadata` then `File::open` (research R1) -> 18 passed
- refactor: the P5 test restores the locked directory's mode through a `Drop` guard, so a failing
  run no longer leaves a mode-000 directory in the temp dir (one was left by the red run and removed
  by hand)

## Cycle 3: U19–U22 `check_bounded` (contract C7–C8)

- test: `crates/micold-core/tests/script_path_check.rs`, four tests (new)
- red: same command, `check_bounded` stubbed as `todo!("T009")` -> 19 passed, 3 failed
  (`not yet implemented: T009` for U19, U20, U21). U22 `the_bound_is_two_seconds` passed on arrival
  (the constant is T001's declaration); deliberate mutant `from_secs(3)` ->
  `left: 3s right: 2s` (1 failed), then restored.
- green: detached `std::thread` + `mpsc::recv_timeout` (research R3) -> 22 passed. Core fast subset
  `scripts/build-lock.sh cargo test -p micold-core --all-targets` -> 1333 passed, 0 failed.
- refactor: none needed

## Cycle 4: U23 `ScriptPathProbe` is a registered port

- test: existing guards. T005 added `"ScriptPathProbe"` to `PORTS` in
  `crates/micold-client/tests/inventory/mod.rs` and `"FakeScriptPathProbe"` to the known fakes in
  `crates/micold-client/tests/no_concrete_implementations.rs`.
- red: `scripts/build-lock.sh cargo test -p micold-client --test no_concrete_implementations` ->
  `each_implementation_is_chosen_in_exactly_one_place` FAILED: ``- `StdScriptPathProbe` is chosen
  in 0 places: []`` (13 passed, 1 failed). `service_capability_fakes` passed on arrival (11 passed):
  the core fake already exists and the core tests exercise it, which is what that guard asks.
- green: `Capabilities::real()` constructs `StdScriptPathProbe`; accessor `script_path_probe()` and
  `#[cfg(test)] with_script_path_probe`; `base_app()` hands every test
  `FakeScriptPathProbe::answering(File)` -> 14 passed, 11 passed
- refactor: none needed

## Cycle 5: U24–U28, U37 the reducer holds check results (contract S1–S4, S8)

- test: `crates/micold-client/tests/features_settings.rs`, module `script_path_check`, six tests (new)
- red: `scripts/build-lock.sh cargo test -p micold-client --test features_settings script_path_check`
  against `Msg` variants and state fields whose reducer arms did nothing -> 2 passed, 4 failed:
  - U24 `starting_a_check_raises_the_sequence_...`: `each start takes a new number left: 4 right: 5`
  - U25 `a_save_marks_its_check_as_one_to_report_...`: `left: None right: Some(0)`
  - U26 `the_current_checks_answer_is_shown`: `left: Idle right: Done(CheckedScriptPath { .. NotFound { tilde: false } })`
  - U27 `a_blank_paths_answer_leaves_nothing_to_show`: `left: Done(..) right: Idle`
  - U28 and U37 assert an absence and passed on the stub. Deliberate mutants, together: drop the
    `seq` guard in `script_path_checked` and clear `settings_draft` there -> U28 `check 1 was
    superseded by 2: ...` and U37 `a check reports on the path and changes nothing else ...`
    (4 passed, 2 failed), then restored.
- green: `script_path_check_started` / `script_path_checked` in
  `crates/micold-client/src/features/settings.rs` -> 21 passed (the whole file)
- refactor: none needed
- notes: `script_check_save_seq` is set by S1 as U25 asks; nothing reads it until M2's S5–S7.

## Cycle 6 (outer loop opened): A1–A4 written, red

- test: `crates/micold-client/src/main_tests.rs`, module `tests::script_path_report`, four tests (new),
  at the entry point of test-list.md: `shell::persist::open_settings` (the real handler, split from
  `on_settings_opened` so a test can take the job it prepared), `ScriptPathCheckJob::run`,
  `app.core.update`, then `script_path_notice`.
- red: `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide script_path_report`
  against declared stubs -> 1 passed, 3 failed: A1, A2 and A3 each `not yet implemented: T018`
  (`open_settings` was a stub). A4
  `off_with_a_missing_stored_path_a_session_launch_examines_and_sources_nothing` passed on arrival:
  it asserts an absence, and T033 records its mutant.
- notes: A4 needs a recording resolver behind the client's `Arc<dyn EnvIncludeResolver + Send +
  Sync>`, so `FakeEnvIncludeResolver` now keeps its state behind a `Mutex` instead of a `RefCell`
  (same API), and `Capabilities` gains a `#[cfg(test)] with_env_include` narrowing. M3's U61 needs
  the same seam.

## Cycle 7: U38–U45, U63 `script_path_notice`, feature off and the interim (N1, N2, N5, N6, N8, N9, N11)

- test: `crates/micold-client/tests/features_settings.rs`, module `script_path_notice_off`, ten tests
  (new)
- red: `scripts/build-lock.sh cargo test -p micold-client --test features_settings script_path_notice_off`
  against a stub returning no lines -> 1 passed, 9 failed, for example:
  - U40 `off_and_not_found_says_so_by_path_...`: `issue #435 (N2) left: [] right: [Caution("Script not found: /tmp/does-not-exist.sh"), Note("Environment include is off, ...")]`
  - U41 `off_and_a_tilde_path_...`: `N5, feature off left: [] right: [Caution(..), Note("~ is not expanded. Use a full path."), Note(OFF)]`
  - U42 `off_and_not_readable_...`: `N6 left: [] right: [Caution("Not a readable file: /tmp/does-not-exist.sh"), Note(OFF)]`
  - U43 `a_relative_path_...`: `N8 with Disabled ... left: [] right: [Note("Relative path: ...")]`
  - U44 `a_check_with_no_answer_...`: `N9 with Disabled left: [] right: [Caution("Couldn't check the script path: ..."), Note("No answer within 2 seconds. ...")]`
  - U38 `with_no_check_yet_...`: `left: [] right: [Caution("Exited with an error"), Note("env.sh: line 3: nvm: command not found")]`
  - U39 `a_check_in_flight_...`: `with no previous answer, the page is 011's (N1) left: [] right: [Caution("Exited with an error"), ..]`
  - U63 `until_m3_the_on_state_page_is_exactly_011s`: `interim (U63): ... Present with MissingScript ... left: [] right: [Caution("Script not found")]`
  - U45 `off_and_present_says_nothing` passed on the stub; deliberate mutant (Present returns a
    not-found caution) -> `left: [Caution("Script not found: /tmp/does-not-exist.sh")] right: []`,
    then restored.
- green: `NoticeLine`, `script_path_notice` and `lines_011` (011's `failure()` logic, moved) in
  `crates/micold-client/src/features/settings.rs` -> 31 passed (the whole file)
- refactor: none needed; `ui/settings/environment.rs`'s `failure()` is deleted in T019 when the page
  renders these lines.

## Cycle 8: U55, U57 the triggers (T1; no check on a terminal restart); the check job (T014, T017, T018)

- test: `crates/micold-client/src/main_tests.rs`, module `tests::script_path_report`, two tests (new):
  `opening_settings_seeds_the_draft_at_once_and_checks_the_stored_path_as_it_is_stored` (U55, T1) and
  `a_terminal_restart_does_not_check_the_path` (U57)
- red: `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide script_path_report`
  against the `open_settings` / `ScriptPathCheckJob` stubs -> 2 passed, 4 failed: U55, A1, A2 and A3
  each `not yet implemented: T018`. U57 and A4 assert an absence and passed on arrival (mutants
  below, T033).
- green: `ScriptPathCheckJob`, `prepare_script_path_check` and `run_script_path_check` in
  `crates/micold-client/src/shell/env_include.rs`; `open_settings` seeds the draft and prepares the
  job, and `on_settings_opened` runs it on `spawn_blocking` -> 6 passed
- refactor: T017's `start_script_path_check(app, origin)` is split into `prepare_script_path_check`
  (returns the job) and `run_script_path_check(job) -> Task` so `open_settings` can hand its job to
  a test; `start` is their composition and is not needed until M2's save trigger.

## Cycle 9: T019 the page renders the notice

- No new test: GUI glue (Principle VIII exception, validated by T021's visual pass). `failure()` is
  deleted from `ui/settings/environment.rs`; the page pushes `script_path_notice(check, outcome)`
  after the timeout field, `Caution` through `caution`, `Note` through `note`. `script_check` is
  threaded from `ui::view` through `settings_view::view`.

## T033: A1–A4 and U57 at the full suite

- A1 and U55 went red for the right reason (the stub) and are green. A2, A3, A4 and U57 assert an
  absence. Deliberate mutants, applied together, then reverted with `git checkout`:
  - render a caution for `Present` (`script_path_notice`) -> A2
    `a readable file is not a problem to report (US1 scenario 2, SC-002) left: [Caution("Script not found: /tmp/does-not-exist.sh")] right: []`
  - probe before the blank short-circuit in `classify` -> A3 panicked at `main_tests.rs:4138`
    (`a blank path must not be probed`)
  - run a check from `view_and_start` -> A4 panicked at `main_tests.rs:4157` (the probe was called)
  - run a check from the `TerminalRestartRequested` handler -> U57 panicked at `main_tests.rs:4243`
    (`a restart must not check the path`)
  - (U55 also failed, under the `classify` mutant: `left: [path, path] right: [path]`.)
- full suite: `mise run gate` at 58b43cd8, and again at 7e7e3885 after the review fixes -> GATE_EXIT=0 both times; 3755 passed, 0 failed across the
  workspace; all six `tests::script_path_report` tests (A1–A4, U55, U57) ok.

## Cycle 10 (M2): U29–U36 the save-time notice in the reducer (T013, T016)

- test: `crates/micold-client/tests/features_settings.rs`, module `script_path_check`, eight tests
  (new): U29 `a_save_leaving_a_missing_path_posts_one_notice_naming_it`, U30
  `a_missing_path_starting_with_a_tilde_says_the_tilde_is_not_expanded`, U31
  `a_path_that_is_not_a_readable_file_says_so`, U32 `a_save_with_nothing_wrong_to_report_posts_nothing`,
  U33 `opening_settings_never_posts_a_notice`, U34
  `a_saves_notice_is_posted_even_when_a_newer_open_took_over_the_page`, U35
  `an_older_saves_answer_is_not_reported_once_a_newer_save_started`, U36
  `a_saves_answer_delivered_twice_is_reported_once`
- red: `scripts/build-lock.sh cargo test -p micold-client --test features_settings script_path_check`
  -> 9 passed; 5 failed. U29, U30, U31, U34, U36 each `left: [] right: [NotificationRaised(Notification
  { level: Info, message: "The environment-include script was not found: /tmp/does-not-exist.sh" })]`
  (U31: `... is not a readable file: ...`; U30: `... (~ is not expanded; use a full path)`). U32, U33
  and U35 assert an absence and passed on arrival (mutants below).
- green: `save_notice(&CheckedScriptPath) -> Option<String>` and S5–S7 in `script_path_checked`,
  which now returns `notifications::info(..)` as an `Outcome` from `update` -> features_settings 39
  passed. `save_notice` returns `Option` rather than the contract's `String`, so the three states
  with nothing to report are answered by the same match instead of an `unreachable!`.
- mutants (each applied alone after committing green at 44738fa7, then reverted with `git checkout`):
  - `let reports = true;` -> U33, U35, U36 failed (`features_settings.rs:597`, `:634`, `:656`)
  - `let reports = origin == CheckOrigin::Saved;` (no sequence gate) -> U35, U36 failed
  - `ScriptPathState::Present => Some(path.clone())` in `save_notice` -> U32 failed (`:578`)
  - drop `script_check_save_seq = None` -> U36 failed (`:656`)
- refactor: none needed.
- commit: 44738fa7

## Cycle 11 (M2): A5, U56, U58 every save checks and reports (T037, T038, T039)

- test: `crates/micold-client/src/main_tests.rs`, module `tests::script_path_report`, three tests
  (new): A5 `a_save_with_a_missing_path_saves_and_posts_one_notice_naming_it_with_the_feature_off_or_on`,
  U56 `a_save_checks_the_saved_path_even_when_the_path_did_not_change`, U58
  `a_save_with_a_missing_path_writes_only_the_three_environment_include_settings`. Seam:
  `Capabilities::with_settings` (test-only) for a `FakeSettingsStore`, and `apply_save`'s body moved
  into `save_and_prepare_check(app, valid) -> (Task, ScriptPathCheckJob)`, with the job stubbed
  `todo!("T038")`.
- red: `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide script_path_report`
  -> 6 passed; 3 failed, each `panicked at crates/micold-client/src/shell/persist.rs:412:64` (the
  `todo!("T038")` stub).
- green: `save_and_prepare_check` prepares `prepare_script_path_check(app, CheckOrigin::Saved)` after
  the write, 011's refresh and `Msg::Saved`; `apply_save` batches the survival task with
  `run_script_path_check(job)` -> 9 passed.
- mutant for U58 (it went red only through the stub): the write adds 1 to `env_include_timeout_secs`
  -> U58 failed at `main_tests.rs:4378`; reverted.
- refactor: none needed.
- commit: 2f94f598

## Cycle 12 (M2): U64 a failed write posts no path notice (review A); mutants for A5 and U56 (review B)

- test: `crates/micold-client/src/main_tests.rs` `tests::script_path_report::a_save_whose_write_failed_posts_no_notice_about_the_path` (new, U64, added to the list after review A found the
  save's check ran after a failed write).
- red: `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide script_path_report` -> 9 passed; 1 failed:
  `got [Notification { level: Error, message: "Couldn't save your settings: fake: save refused" }, Notification { level: Info, message: "The environment-include script was not found: /tmp/does-not-exist.sh" }]`
- green: `save_and_prepare_check` returns `Option<ScriptPathCheckJob>`, `None` when the write
  failed; `apply_save` runs the job only when there is one -> 10 passed.
- refactor (same commit): the save tests share `saving_app`, which also swaps in
  `FakeEnvIncludeResolver` so the feature-on save never runs a real shell over the temp dir;
  `opened()` removed for the existing `open_and_check`; U58's test renamed to what it asserts.
  `save_notice` made private; the contract names its `Option<String>` return.
- mutants for the tests that went red only through Cycle 11's `todo!` stub (review B F2), each
  applied alone and reverted with `git checkout`:
  - the save prepares its check with `CheckOrigin::Opened` -> A5 failed at `main_tests.rs:4341`
    (no notice), U56 at `:4368` (origin)
  - the save checks only when the path changed -> U56 failed at `:4358` (no job), A5 at `:4341`
- commit: 811b8faa

## T040: A5 at the full suite

- `mise run gate` at 07d0a75a -> GATE_EXIT=0; 3767 passed, 0 failed across the workspace;
  `tests::script_path_report::a_save_with_a_missing_path_saves_and_posts_one_notice_naming_it_with_the_feature_off_or_on`
  (A5) ok, with U56, U58 and U64. (The first gate, at 1b667643, stopped at `cargo fmt --check`;
  fixed in the `style(035)` commit.)

## M3 baseline

- suite: `scripts/build-lock.sh cargo test --workspace` at d044d567 (origin/main fae88e6c plus the
  ledger commit) -> 3804 passed, 0 failed.

## Cycle 13 (M3): A6–A8 and U46–U54, the on-state rows (T034, T022, T024)

- tests: `crates/micold-client/src/main_tests.rs` `tests::script_path_report::` A6
  `on_with_a_missing_stored_path_the_page_says_it_was_not_found_once_and_that_the_feature_is_on`,
  A7 `switching_the_feature_off_and_saving_keeps_the_same_not_found_report`, A8
  `creating_the_missing_file_clears_the_report_and_with_the_feature_on_says_how_to_source_it`;
  `crates/micold-client/tests/features_settings.rs` new module `script_path_notice_on` (U46–U54,
  plus the FR-002 on-state invariant and the no-path 011 line).
- test change before the implementation (T022, contracts §2 "011(o)"): the `script_path_notice_off`
  helper `lines_011` takes the checked path, so N8/N9 expect 011's `MissingScript` line as
  `Script not found: P`. U63's test `until_m3_the_on_state_page_is_exactly_011s` deleted as T024
  plans; U63 marked DROPPED.
- red (outer): `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide script_path_report`
  -> 11 passed; 2 failed: A6 `left: [Caution("Script not found")]` vs
  `right: [Caution("Script not found: /tmp/does-not-exist.sh"), Note("Environment include is on, …")]`;
  A8 `left: [Caution("Script not found")]` vs `right: [Caution("The last attempt could not find the script"), …]`.
  A7 passed at once: M1 already shipped the off rows (mutant below).
- red (inner): `scripts/build-lock.sh cargo test -p micold-client --test features_settings script_path_notice`
  -> 10 passed; 10 failed, e.g. U46 `left: [Caution("Script not found")]`, U47
  `left: [Caution("Exited with an error"), Note("env.sh: line 3: nvm: command not found")]`, N8
  `left: [Note("Relative path: …"), Caution("Script not found")]`, U53 `left: None` vs
  `right: Some(Caution("Script not found: /tmp/does-not-exist.sh"))`. U54 passed at once (the
  interim gate guaranteed it; mutant below).
- green: `script_path_notice` drops the interim gate, adds NOTICE_ON and rows N3, N4, N5 (on),
  N7 and N10; `lines_011` takes the checked path -> features_settings 49 passed; script_path_report
  13 passed.
- mutants (each applied alone from a backup copy and restored):
  - NotFound with the feature off returns 011(last) -> A7 failed at `main_tests.rs:4492` (and A1).
  - 011's line after the path's caution also when last is `MissingScript` -> U54 failed at
    `features_settings.rs:1050`, U46 at `:916`, U48 at `:939`.
- refactor: none beyond `cargo fmt`.
- commit: 30b37e4a

## Cycle 14 (M3): U59–U61, another window's save and no re-sourcing (T023, T025)

- seam (stub, before the tests): the `DaemonMsg::SettingsChanged` arm's body moved unchanged into
  `shell::daemon_sync::on_settings_changed(app, settings) -> Option<ScriptPathCheckJob>`, returning
  `None`; the arm runs the job when there is one (contract §3 T3; the same split as D21).
- tests: `crates/micold-client/src/main_tests.rs` `tests::script_path_report::`
  `another_windows_save_rechecks_the_new_path_while_settings_is_open` (U59),
  `another_windows_save_checks_nothing_while_settings_is_closed` (U60),
  `showing_settings_sources_nothing` (U61, through `on_settings_opened`).
- red: `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide script_path_report`
  -> 15 passed; 1 failed: U59 `panicked at crates/micold-client/src/main_tests.rs:4493:10:
  an open Settings page is re-checked (T3)`. U60 and U61 passed at once (absences; mutants below).
- green: `on_settings_changed` ends with `settings_draft.as_ref()?` and
  `prepare_script_path_check(app, CheckOrigin::Opened)` -> 16 passed.
- mutants (applied alone from a backup copy, restored):
  - the Settings-open guard dropped -> U60 failed at `main_tests.rs:4533`.
  - `open_settings` re-sources the home directory (`refresh_env_include`) -> U61 failed at
    `main_tests.rs:4551` (run while U61 still called `open_settings` through `open_and_check`;
    `on_settings_opened` runs the same `open_settings`).
- refactor: none beyond `cargo fmt`.
- commit: 3de463df

## T035: A6–A8 at the full suite

- `mise run gate` at 36015322 -> GATE_EXIT=0; 3820 passed, 0 failed across the workspace; A6
  `on_with_a_missing_stored_path_the_page_says_it_was_not_found_once_and_that_the_feature_is_on`,
  A7 `switching_the_feature_off_and_saving_keeps_the_same_not_found_report` and A8
  `creating_the_missing_file_clears_the_report_and_with_the_feature_on_says_how_to_source_it` ok;
  scripts suite 15 cases, 0 failures.

## Cycle 15 (M3): U65, U66 from review A

- tests: `crates/micold-client/tests/features_settings.rs`
  `script_path_notice_on::on_and_not_readable_after_a_missing_script_attempt_says_it_once_by_path`
  (U65) and `script_path_check::a_check_of_another_path_or_state_does_not_keep_showing_the_previous_answer`
  (U66). Stub so they compile: `Msg::ScriptPathCheckStarted` gains `path` and `enabled`, ignored by
  the reducer; the shell fills them from the stored values.
- red: `scripts/build-lock.sh cargo test -p micold-client --test features_settings` -> 49 passed;
  2 failed: U66 `left: Pending { seq: 1, last: Some(CheckedScriptPath { path: "/tmp/does-not-exist.sh", enabled: false, state: Present }) }`
  vs `right: Pending { seq: 1, last: None }`; U65 `left: [Caution("Not a readable file: …"), Note(ON), Caution("Script not found: /tmp/does-not-exist.sh")]`.
- green: S1 keeps `last` only when its path and enabled flag match the message's; the NotReadable
  row uses the same `after_on` as NotFound -> features_settings 51 passed; client bin 280 passed.
- refactor (same commit): `adopt_daemon_settings` shared by `SettingsChanged` and `Welcome`
  (review A F5); the moved comment no longer points at "`Welcome` below" (F8). Contract §1 S1 and
  §2 N7 updated.
- commit: 179c1bb9
