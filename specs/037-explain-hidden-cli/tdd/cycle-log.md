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
- After two rebases onto `origin/main`, where feature 034 took protocol 18 (commit 746cfa6c) and
  then 19 (commit 31c8b01b), 037's bump is 19 -> 20 and the version pin in `tests/schema_hash.rs`
  was re-taken at 20. The dated entries below record the 17 -> 18 they ran against and are left as
  written.

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

## Cycle 4: U40–U45, U47, U48, U50, U53–U55, A1–A8 the client files the state and Settings says it (T006, T011–T014, T038)

- test: `tests/directory_availability.rs` (U40–U44), `src/main_tests.rs` module
  `the_settings_note_explains_a_missing_cli` (U45, A1–A8),
  `tests/missing_cli_is_reported_where_it_is_chosen.rs` (U47, U48, U53–U55),
  `tests/a_field_note_shares_its_fields_column.rs` (U50), commit `1c419173`
- compile: the first run stopped at `error[E0425]: cannot find type SpawnEnv` in
  `features/session.rs`; the field was committed without its import. Import added, nothing else
- red: `scripts/build-lock.sh cargo test -p micold-client --no-fail-fast` -> 4 targets failed, 17 tests:
  bin 309 passed; 7 failed (U45, A1, A3–A6, A8 with A2); `a_field_note_shares_its_fields_column`
  2 passed; 2 failed; `directory_availability` 18 passed; 3 failed (U40–U42);
  `missing_cli_is_reported_where_it_is_chosen` 8 passed; 5 failed (U47, U48, U53, U54 and the reworded
  027 host test). Every other target ok
- passed on arrival: U43 and U44 (`answered` already replaces the whole answer and drops an older one, so
  `env` travels with `available`), U55 (a property of the signature) and A7 (an absence; mutant in cycle 5)
- green: `answered` stamps `asked_for` with the request's key; the shell arm files the answer's `env`;
  `missing_cli_notice` returns `explain(missing, env?, place, AttemptDir::Home)` as `{reason} {action}`
- one test outside the list went red at green: `provider_choice_surfaces` pressed the centre of the
  `field_note` column, which is inside the note once the note wraps, so the select never opened. Its
  fixture now carries `env: Some(Applied)` and `SETTINGS_SELECT` addresses the select itself. The
  product is unchanged: a person presses the select, not the column's centre
- result: `cargo test -p micold-client --no-fail-fast` -> every target ok but that one; then
  `--test provider_choice_surfaces` -> 6 passed. The full suite runs in cycle 5 (`mise run gate`)

## Cycle 5: A1–A8 with the full suite (T039)

- A7 mutant: `missing_cli_notice` returning `Some("mutant")` when nothing is missing ->
  `cargo test -p micold-client --bin micold-ai-ide the_settings_note_explains` -> 6 passed; 2 failed:
  `with_every_cli_found_there_is_no_note_in_any_state` (A7) and
  `the_note_appears_with_the_answer_and_goes_when_the_cli_is_found` (A2). Reverted with `git checkout`
- gate: the first run stopped at `cargo fmt --check` (import order in `features/settings.rs`), the second
  at clippy `match_like_matches_macro` in `crates/micold-core/tests/cli_reason.rs`; neither reached a
  test. Third run, on `b205f81d`: `mise run gate` -> `GATE_EXIT=0`, 374 `test result: ok`, 0 failed.
  A1–A8 (`the_settings_note_explains_a_missing_cli`) -> 8 passed
- U43, U44 and U55 are characterization entries: they hold by how `answered` already files an answer and by
  the function's signature, passed on arrival, and have no red and no mutant (Review B, F1)
- T017: quickstart §B B1–B8 in the light and dark themes pass; B14 is recorded as covered by Part A
  (`evidence/README.md`)

## Cycle 6: U56–U63 `start_refusal` and `start_refusal_unknown` (T018, T022)

- test: `crates/micold-core/tests/cli_reason.rs`, the eight tests under "W3: the sentence said when a
  start is refused"
- stub: both functions declared returning `String::new()` so the test file compiles
- red: `scripts/build-lock.sh cargo test -p micold-core --test cli_reason` ->
  `test result: FAILED. 26 passed; 8 failed`; the eight are the new ones
  (`a_fresh_refusal_is_the_explanation_and_the_offer_of_another_cli`, …, `w3e_…`)
- green: `start_refusal` returns 027's two sentences for `Applied` in an image and otherwise
  `explain(&[cli], ..)`'s `{reason} {action}` plus the launch's ending; `start_refusal_unknown` is W5's
  sentence. `mise run test-core` -> exit 0, 134 `test result: ok`, 0 failed
- refactor: none needed

## Cycle 7: U64–U66, A10–A14 the service's refusals (T019, T041, T020, T023, T024)

- test: `crates/micold-daemon/tests/session_start.rs`:
  `with_environment_include_off_a_refused_start_says_sessions_get_only_the_login_path` (U64),
  `a_refused_start_after_the_script_failed_says_the_script_failed_for_the_directory` (U65),
  `a_refused_start_after_the_script_ran_says_the_cli_is_not_on_the_session_path` (A11),
  `a_resume_after_the_script_timed_out_says_to_fix_it_and_restart_this_session` (A10),
  `a_refused_start_and_an_availability_answer_name_the_same_state` (U66),
  `in_an_image_whose_script_ran_a_missing_cli_keeps_the_sentences_it_had` (A12),
  `in_an_image_with_environment_include_off_the_refusal_does_not_blame_the_image` (A13);
  `crates/micold-daemon/tests/mcp_create_session.rs`:
  `a_cli_the_directorys_environment_lacks_is_refused_with_the_reason_and_no_record` (A14)
- red, before any change under `src/`: `scripts/build-lock.sh cargo test -p micold-daemon --test
  session_start` -> `test result: FAILED. 19 passed; 6 failed`: U64, U65, A11, A10, U66 and A13, each on
  the old sentence (left `"Claude Code isn't installed. Install it, …"`).
  `… --test mcp_create_session` -> `test result: FAILED. 17 passed; 1 failed`: A14 (left
  `"Pi Coding Agent is not installed where this session would run: …"`)
- A12 is a characterization test written against literal strings: it was green in that red run, before
  T023 changed the gate, and is green after it
- green: the launch gate resolves the directory once (`spawn_env_for`) and fails with
  `start_refusal(cli, env, place, Dir(cwd), launch)`; `missing_cli_reason` is removed; `create_session`
  replies with `explain`'s `{reason} {action}` from `State::availability_in(cwd)`.
  `session_start` -> `test result: ok. 25 passed; 0 failed`; `mcp_create_session` ->
  `test result: ok. 18 passed; 0 failed`; `cargo test -p micold-daemon` -> exit 0, 97 `test result: ok`
- replaced: `a_missing_cli_is_advised_on_where_sessions_run_and_on_what_is_being_started` and the file's
  `missing_cli_reason` helper; its properties (a resume never offers another CLI, an image never says
  "install") are asserted in U64, A12 and A13. The T020 test is renamed: "not installed" is no longer
  what it checks
- written by a subagent from the task text; the red and green lines are its runs

## Cycle 8: U69, U71–U75, A9 the missing-default message (T040, T021, T025)

- test: `crates/micold-client/tests/unavailable_default_says_so.rs`:
  `in_each_state_the_press_says_that_states_refusal_once` (U69),
  `in_each_state_the_press_opens_the_list_starts_nothing_and_keeps_the_stored_default` (U71),
  `an_answer_without_a_state_says_only_that_the_cli_would_not_be_found` (U72),
  `a_row_on_the_home_answer_names_the_home_directory_and_its_own_answer_names_its_own` (U73),
  `an_answer_settled_in_an_image_says_the_image_form` (U74);
  `tests/directory_availability.rs`:
  `a_newer_answer_after_the_missing_default_message_says_nothing_and_leaves_it_as_said` (U75);
  `src/main_tests.rs`: `a_missing_default_says_why_the_list_opened::
  pressing_start_says_include_is_off_opens_the_list_and_starts_nothing` (A9)
- red, before any change under `src/`: `scripts/build-lock.sh cargo test -p micold-client --test
  unavailable_default_says_so --test directory_availability --test start_failure_notice
  --no-fail-fast` -> `unavailable_default_says_so`: `test result: FAILED. 6 passed; 7 failed` (the five
  above and the two 026/029 tests that quoted the old sentence); `directory_availability`:
  `test result: FAILED. 21 passed; 1 failed` (U75); left `"Pi Coding Agent isn't installed. Install
  it, or start this session on another AI CLI."`. `… --bin micold-ai-ide
  a_missing_default_says_why_the_list_opened` -> `test result: FAILED. 0 passed; 1 failed` (A9)
- green: `start_menu_toggled` posts `start_refusal(cli, env, place, dir, Fresh)` from the answer in use
  for the row (`State::answer_in_use`, which `known_clis` now reads too), and `start_refusal_unknown`
  when the answer has no state. The same commands -> 13 passed, 22 passed, 4 passed, 1 passed;
  `cargo test -p micold-client --no-fail-fast` -> exit 0, 146 `test result: ok`, 0 failed
- U71 and U75 went red only through the message equality they assert together with the list, the start
  and the stored default; those parts have no red and no mutant of their own
- `start_failure_notice.rs`: fixture only (`REASON` is now `start_refusal(..)`), assertions unchanged,
  green before and after
- after green, not test-first: the branch for a press with no answer in use kept 026's "isn't
  installed" sentence. The view never sends that press (W5); it now says `start_refusal_unknown`
  (FR-002). No test pins the branch
- written by a subagent from the task text; the red and green lines are its runs

## Cycle 9: review A round 1, F1 and F2 (T023)

- test: `crates/micold-daemon/tests/session_start.rs`:
  `a_restart_after_the_script_is_fixed_sources_it_again_and_starts` (F1),
  `an_ai_session_whose_folder_is_gone_is_told_that_and_not_that_the_script_timed_out` (F2)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test session_start` ->
  `test result: FAILED. 25 passed; 2 failed`. F1: the restart returned `"… the startup script exited
  with an error for /tmp/.tmpV0VM4A … Then restart this session …"`. F2: left `"… the startup script
  timed out for /tmp/.tmp1MiAMd/gone …"`, right `"This session's folder no longer exists: …"`
- green: a start refused for a missing CLI drops the directory's cached resolution
  (`refuse_and_forget_env`); in that branch a folder that is gone gets the folder sentence
  (`folder_gone`). Same command -> `test result: ok. 27 passed; 0 failed`;
  `cargo test -p micold-daemon` -> exit 0, 97 `test result: ok`
- refactor (F4, F5): `cli_reason::explain_one` is total, so neither `start_refusal` nor
  `mcp/tools.rs` holds an `unreachable!`; `explain_one_says_what_explain_says_for_a_list_of_one` pins
  it. `ImageReference` borrows the `NoCliOnPath` that holds `ENV_LOCK`.
  `cargo test -p micold-core --test cli_reason` -> `test result: ok. 35 passed`
- written by a subagent from the findings; the red and green lines are its runs

## Cycle 10: review B round 1, F1 (T024); A9–A14 with the full suite (T042); B9 and B10 (T027)

- test: `crates/micold-daemon/tests/mcp_create_session.rs`:
  `create_session_after_the_script_is_fixed_sources_it_again_and_starts`
- red: in `mise run gate` on the tree with the test and without the fix ->
  `test result: FAILED. 18 passed; 1 failed`: the second `create_session`, made after the script was
  fixed and with no setting saved, returned `"… the startup script exited with an error for
  /tmp/.tmpvRwgYB, so its PATH additions are not applied. Fix the script named in "Script path"."`
- green: the refusal in `mcp/tools.rs` drops the directory's resolution
  (`state.invalidate_env_include(&cwd)`), as the launch gate's refusals do (D18). In the gate below:
  `mcp_create_session` -> `test result: ok. 19 passed; 0 failed`
- gate (T042), on `1e58735e`: `mise run gate` -> `GATE_EXIT=0`, 380 `test result: ok`, 0 failed;
  `cargo check --workspace --target aarch64-apple-darwin` -> exit 0. A9
  (`a_missing_default_says_why_the_list_opened`), A10–A13 (`session_start`, 27 passed) and A14
  (`mcp_create_session`) are green with the full suite
- A12 is a characterization test (Story 2 scenario 4): it was green against the untouched gate before
  T023 changed it (cycle 7) and is `BASELINE` in the test list, with no red
- T027: quickstart §B B9 passes. B10: the banner passes word for word; after a restart in place the
  pane keeps its terminal and shows no sentence (`evidence/README.md`, ledger D20)

## Cycle 11: U78–U85, U87–U90, A15–A20 the row list's note (T043, T028, T029, T030–T033)

- test: `crates/micold-client/tests/directory_availability.rs` (U78–U85),
  `src/ui/material/menu_anatomy.rs` (U87–U90), `src/main_tests.rs`
  `a_rows_cli_list_names_what_is_not_offered` (A15–A20); the names are in `test-list.md`
- red, against stubs (`start_menu_note` returned `None`; `MenuOverlay::note` and
  `menu_panel_size_with_note` ignored the note):
  - `cargo test -p micold-client --test directory_availability` -> `test result: FAILED. 27 passed; 3
    failed`: U78 `left: None, right: Some("A session would not find Pi Coding Agent: the startup
    script exited with an error for /repo, …")`, U84 `the home answer is in use for the row`, U85
    `left: None`
  - `cargo test -p micold-client --lib -- menu_anatomy` -> `test result: FAILED. 12 passed; 3 failed`:
    U87 and U89 `no child 2 at depth 1 of [0, 2]`, U88 `the panel lays out no note under its items`
  - `cargo test -p micold-client --bin micold-ai-ide -- a_rows_cli_list_names_what_is_not_offered` ->
    `test result: FAILED. 3 passed; 3 failed`: A15 `a list of two that lacks a third says so`, A19
    `feat-b lacks Copilot`, A20 `the list says what it does not offer`
- green: `State::start_menu_note` (T030), `MenuOverlay::note`, `menu::body` and
  `menu_panel_size_with_note` (T031), the start list in `ui/mod.rs` (T032), the showcase's
  "Open a start list with a note" (T033). `scripts/build-lock.sh cargo test -p micold-client
  --no-fail-fast` -> exit 0, 146 `test result: ok`, 0 failed; `directory_availability` 30 passed,
  `menu_anatomy` 15 passed, `a_rows_cli_list_names_what_is_not_offered` 6 passed
- U79–U83, U90 and A16–A18 assert an absence or an equality the stubs already met, and passed
  before the implementation. Each has a mutant that turns it red, applied to the green tree and
  reverted (`git checkout`), one at a time:
  - drop `if answer.available.len() < 2 { return None; }` -> `directory_availability`: `28 passed; 2
    failed` (U80 `a_list_of_one_has_no_note`, U81 `a_list_of_none_has_no_note`); the bin group: `4
    passed; 2 failed` (A16, A17)
  - `answer.env?` -> `answer.env.unwrap_or(SpawnEnv::ScriptFailed)` -> `29 passed; 1 failed` (U82)
  - `&answer.missing()` -> `&[AiCli::Pi]` -> `28 passed; 2 failed` (U79, and U85); the bin group: `4
    passed; 2 failed` (A18, and A19)
  - no answer in use returns `Some("made up")` instead of `None` -> `29 passed; 1 failed` (U83)
  - `menu_panel_size_with_note` returns the height without the note's block -> `menu_anatomy`: `14
    passed; 1 failed` (U90)
- T032 and T033 are GUI glue with no test of their own (tasks.md); T035 looks at them
- the tests and the implementation were written in one sitting and the implementation first compiled
  after the reds above were recorded; it was green on its first build

## Cycle 12: review A round 1, F2 (T029); A15–A20 with the full suite (T044)

- test: `src/ui/material/menu_anatomy.rs`: `PATH_NOTE`, a sentence naming a directory wider than the
  panel, added to `the_clamping_estimate_matches_a_panel_with_a_note` (U90) and to
  `a_note_wraps_at_the_panels_width_less_the_item_padding_at_both_sides` (U88: it takes more than
  the five lines its words alone need)
- no red: a characterization of the green code, written after review A said the estimate was held
  only for sentences without a path. It passed on its first run. The cycle 11 mutant of the
  estimate turns U90 red for every fixture
- the showcase's sample directory (`samples::PROJECT_DIR`) is now a worktree path wider than the
  panel, so its pose shows the break the first visual pass could not
- gate (T044), on the code tree of `5662df9b` (tree `41ede9e7414b`, before the squash and the rebase onto 038's docs): `mise run gate` -> `GATE_EXIT=0`, 380 `test result: ok`, 0 failed. A15–A20
  (`a_rows_cli_list_names_what_is_not_offered`, 6 passed), U78–U85 (`directory_availability`, 30
  passed) and U87–U90 (`menu_anatomy`, 15 passed) are green with the full suite. The mutants for
  A16, A17 and A18 are in cycle 11. No `cfg(target_os)` arm is touched, so no macOS cross-check
- T035: quickstart §B B11, B12 and B13 pass (`evidence/README.md`). B11 and B12 at the real client
  (B11 light and dark; opened with too little room under the row, the panel moves up and stays in
  the window); B11 and B13 at the showcase in both themes. B13 was not run at the real client: a
  row with every CLI has the list it had, and `a_row_with_every_cli_has_no_note` holds the absence

## Close: tdd-verify remediation (T045–T050)

- T045: the image placement asserts `starts_with("Pi Coding Agent isn't in {IMAGE}.")`, the host
  `starts_with("Pi Coding Agent was not found on the PATH")`; the disjunction is gone.
- T047: `a_press_with_no_answer_in_use_says_only_that_the_cli_would_not_be_found`
  (`unavailable_default_says_so.rs`) asserts equality with `start_refusal_unknown(Pi)` and the
  literals "Pi Coding Agent would not be found by a session here" and "Start this session on
  another AI CLI."; U72 gained the first literal.
- T048: U69 asserts a per-state fragment, whether "your home directory" is named, and the Fresh
  ending; the `main_tests.rs` IncludeOff press asserts "A session would not find Pi Coding Agent:",
  "sessions get only the login PATH" and the Fresh ending. The IncludeOff sentence names no
  directory, so the directory argument there is held by U73 and the `AttemptDir` mutants below.
- T049: 35 assertions in `crates/micold-core/tests/cli_reason.rs` gained a message (35 tests before
  and after, `35 passed`).
- T046, mutants (each applied alone to c331d0f8 plus the test edits, restored by reversing the
  edit; `cargo test -p micold-client --no-fail-fast`):
  - `session.rs:2180` `None => String::new()` -> KILLED (U72 and 3 more; 10 passed, 4 failed)
  - `session.rs:2185` `None => String::new()` -> KILLED (T047's test; 13 passed, 1 failed)
  - U43: `answered` keeps the old home `env` (`session.rs:405`) -> KILLED
    (`a_newer_answer_replaces_the_state_together_with_the_set`; 29 passed, 1 failed)
  - U44: drop the stale-request guard (`session.rs:402`) -> KILLED (28 passed, 2 failed)
  - `session.rs:335` `Home` -> `Dir("/mutant")` -> KILLED (U73 and 3 more)
  - `session.rs:336` `Dir(dir)` -> `Home` -> KILLED (U73 and 3 more)
  - `settings.rs:1680` `Home` -> `Dir("/mutant")` -> KILLED (U47, U48 and the host test)
  - `session.rs:2073` `attempt_dir()` -> `Home` -> KILLED (3 tests)
  - `session.rs:2176` `attempt_dir()` -> `Home` -> KILLED (U75)
  - `showcase/sections/floating.rs:160` `Dir(PROJECT_DIR)` -> `Home` -> SURVIVED: the showcase
    sample's directory only sets the pose's text; held by the visual pass (B11 showcase), not a test
  - U55: no compilable mutant. `missing_cli_notice` takes `Option<&CliAvailability>` and the type
    has no interior mutability, so the property is held by the compiler; U55 is a characterization
- not done (MED/LOW, no behaviour): finding 4's daemon half, 5, 7, 9, 10 (ledger D24)

