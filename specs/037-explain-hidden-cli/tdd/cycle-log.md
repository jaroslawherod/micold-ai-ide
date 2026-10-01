# Cycle Log: Explain Why an AI CLI Is Not Offered

Append only. Newest last. Every entry's `red` block is the evidence that the test
existed and failed before the implementation.

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace` -> 4098 passed, 0 failed, 9 ignored
  (374 binaries), run as `mise run test`
- commit: `f490b395`
- recorded: cycle 0, before any change. The suite was running elsewhere when the test list was
  written, so the counts are filled in from that run. The spec, plan and tasks edits for feature
  037 touch no code.

## Notes and deviations

- The outer loop runs at two entry points, not an end-to-end GUI runner: the client's `App` in
  `crates/micold-client/src/main_tests.rs`, and the session service's integration tests for the two
  surfaces the service writes. See `test-list.md`, "Entry point".
- Behaviour ids `U<n>` are this list's. The contract's surfaces are written "surface U1" to
  "surface U6" and are a separate series.
- A7, A16, A17 (its note half) and A18 assert an absence and may pass once they compile. T039 and
  T044 record a deliberate mutant as their red evidence.
- A12 is a characterization test (US2-AS4, "unchanged from today"). It is written before T023 and
  must be green against the untouched launch gate. Its entry records that green, not a red.
- Twelve behaviours were `DONE` at planning, each held by an existing test: U37, U46, U49, U51, U52,
  U67, U68, U70, U76, U77, U86, U91.

## Cycle 1: U1–U8 `SpawnEnv::classify`, U9–U25 `name_list` and `explain` (T001–T003, T007, T008)

- test: `crates/micold-core/tests/cli_reason.rs` (new, 26 tests), against T001's declarations with
  `classify`, `name_list` and `explain` stubbed to `None` and `script_applied` to `false`
- red: `scripts/build-lock.sh cargo test -p micold-core --test cli_reason`
  -> `test result: FAILED. 2 passed; 24 failed`. Decisive lines:
  `a_missing_script_is_script_not_found ... left: None right: Some(ScriptNotFound)`;
  `only_applied_counts_as_the_script_applied ... (Applied) left: false right: true`;
  `a_name_list_reads_as_prose ... left: None right: Some("Pi Coding Agent")`;
  every `explain` test: `panicked at crates/micold-core/tests/cli_reason.rs:55:39: a missing CLI has a reason`
- mutants for the two tests the stub passed (`nothing_missing_has_no_explanation`,
  `settings_that_call_for_an_attempt_with_none_known_have_no_state`): with
  `EnvIncludeOutcome::Disabled => Some(SpawnEnv::Applied)` and `name_list(missing).unwrap_or_default()`
  -> `test result: FAILED. 24 passed; 2 failed`, those two. Restored, 26 passed.
- green: `crates/micold-core/src/cli_reason.rs` implements `classify` (enabled, blank path, attempt),
  `script_applied`, `name_list` and `explain` per W1 and W2. `cargo test -p micold-core --test cli_reason`
  -> 26 passed. Core subset (`cargo test -p micold-core --all-targets`) -> 1475 passed, 0 failed
- refactor: `not_applied` closure holds the sentence the three failed-attempt states share
- commit: `58590956`

## Cycle 2: U26, U27 the wire (T004, T009)

- test: `crates/micold-core/tests/schema_hash.rs::the_wire_changes_for_this_feature_cost_exactly_one_version_bump`
  (pin moved to 18), `::the_availability_answer_carries_the_environment_state_in_the_hashed_source` (new);
  `crates/micold-core/tests/protocol_roundtrip.rs::an_availability_answer_round_trips_with_and_without_the_environment_state` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test schema_hash`
  -> `assertion left == right failed ... left: 17 right: 18` and
  `DaemonMsg::AiCliAvailability does not carry env: Option<SpawnEnv> in messages.rs` (8 passed; 2 failed).
  `cargo test -p micold-core --test protocol_roundtrip`
  -> `error[E0559]: variant DaemonMsg::AiCliAvailability has no field named env` (a compile failure:
  the field cannot be stubbed without being the change)
- green: `env: Option<SpawnEnv>` on `DaemonMsg::AiCliAvailability`, `PROTOCOL_VERSION` 18. The service
  sends `env: None` and the client ignores the field until T010 and T011. Core subset -> 1475 passed,
  0 failed. `cargo check --workspace --all-targets` clean
- refactor: none
- commit: `2e477df7`

## Session 1 notes (M1, first unit)

- **Granularity.** Cycles follow the task pairs of tasks.md (T002/T007, T003/T008, T004/T009), one test
  function per behaviour, not one cycle per behaviour id. Each test's own red is in the run quoted.
- **Baseline.** The local full-suite baseline run was lost when the session ended, and was not
  repeated. The branch started at `976b0220` (docs only since `f490b395`), and CI on `main` was green.
  The full suite has not run on this branch yet: only the core subset and a workspace check.
- T015 (labels from the constants) and T016 (user guide) carry no behaviour marker: commit `da35f0df`.
- T008 is not ticked: `name_list` still has its private copy in
  `crates/micold-client/src/features/settings.rs`. U9–U25 are `DONE` against the core functions.

## Cycle 3: U28–U36, U38, U39 what the service answers (T005, T010; T008 remainder)

- test: `crates/micold-daemon/tests/ai_cli_availability.rs`: ten new tests for contract A2 S1–S7 and A3
  (`with_env_include_off_the_answer_says_it_is_off`, `with_a_blank_script_path_…`,
  `a_script_path_that_names_no_file_…`, `a_script_that_exits_with_an_error_…`,
  `a_script_path_that_names_a_directory_…` (unix), `a_script_that_runs_past_the_timeout_…`,
  `a_script_that_adds_the_cli_…`, `a_script_that_succeeds_and_adds_nothing_…`,
  `with_no_directory_the_state_is_known_only_from_the_settings`,
  `turning_env_include_on_changes_the_next_answer_for_the_same_directory`), and
  `a_second_answer_for_a_directory_does_not_run_the_script_again` now reads `availability_in` (U38)
- red: against stubs (`availability_in` returning `NoScriptPath`, `availability_for` returning `None`, the
  server sending `env: None`): `scripts/build-lock.sh cargo test -p micold-daemon --test ai_cli_availability`
  -> 9 passed; 11 failed, each on the state alone, for example `left: ([Pi], None)` /
  `right: ([Pi], Some(ScriptNotFound))` and `left: ([], NoScriptPath)` / `right: ([], IncludeOff)`. `available`
  was already right in every row, which is contract A2's last line
- green: `ResolvedEnv { vars, env }` in the cell, `spawn_env_for`, `availability_in`, `availability_for`;
  the server arm sends `env`. `--test ai_cli_availability --test env_include_cache_coherence`
  -> 20 passed and 3 passed, 0 failed
- refactor: `session_path` holds the PATH lookup `spawn_path_for` and `availability_in` share
- note: an attempt's state comes from a total match (`attempted`), not from `SpawnEnv::classify`, which
  returns an `Option`. `classify` decides the two settings states and S7
- T008 remainder: the client's private `name_list` is deleted; `missing_cli_notice` calls the core one

