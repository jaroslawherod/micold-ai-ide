# Cycle Log: Run a session on the Pi coding agent — BUG-001 increment

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and
failed before the implementation.

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace` -> 3276 passed, 0 failed, 6 ignored
  (326 test binaries)
- commit: `e54d6cd3`
- recorded: cycle 0, before any change. The branch was then rebased onto `origin/main`
  (`8cb88167`); the first cycle re-runs the suite on the rebased base before its red.

## Baseline, re-run on the rebased base

- suite: `scripts/build-lock.sh cargo test --workspace` -> 3284 passed, 0 failed, 6 ignored
  (326 test binaries)
- commit: `3f26a024` (on `origin/main` `8cb88167`)
- per-cycle runs: the full suite takes ~6 minutes behind a build lock shared with other worktrees,
  so each cycle runs the touched crate's suite (`cargo test -p <crate> --all-targets`, the
  profile's fast-subset shape) and the full workspace suite runs before the milestone's gate.

## Cycle 1: U1 `available_in(path)` lists a CLI present only in the given `PATH` value

- test: `crates/micold-core/tests/available_in.rs::a_cli_present_only_in_the_given_path_is_available` (new)
- red: `scripts/build-lock.sh cargo test --test available_in a_cli_present_only_in_the_given_path_is_available -- --exact`
  -> first `error[E0432]: unresolved import micold_core::provider::available_in`; with a stub
  returning `Vec::new()`: `left: []` / `right: [Pi]` (1 failed)
- green: `crates/micold-core/src/provider.rs` `available_in` filters `AiCli::ALL` by
  `resolves_on_path(command, path)`; `resolves_on_path` takes the `PATH` value instead of reading
  the process environment, and the three providers pass the process `PATH`. Core suite
  `scripts/build-lock.sh cargo test -p micold-core --all-targets` -> 1140 passed, 0 failed
- refactor: none in this cycle; threading the value through the trait's `is_available` is its own
  structural step after U2

## Cycle 2: U2 `available_in(path)` omits a CLI present only on the process's own `PATH`

- test: `crates/micold-core/tests/available_in.rs::a_cli_present_only_on_the_process_path_is_not_available_in_another_path` (new)
- red: passed on first run — cycle 1's green already reads only the given value. Deliberate
  mutant: `available_in` also accepting `which.provider().is_available()` (the process `PATH`).
  `scripts/build-lock.sh cargo test --test available_in` -> `got [Pi]` (1 passed, 1 failed).
  Mutant reverted byte for byte; file -> 2 passed
- green: no production change (covered by cycle 1)
- refactor: none in this cycle
- notes: the test moves the process `PATH` (the task text asks not to). It is the only way to
  place a CLI on the process `PATH` and not in the given value; it is the one test in its binary
  that touches the environment, and it restores `PATH` before asserting

## Structural step after cycle 2: the `PATH` value goes through the provider seam (T064)

- change: `AiCliProvider::is_available` takes the `PATH` value to walk (`&OsStr`), every provider
  forwards it to `resolves_on_path`, `available_in` asks the trait, `available_here` is
  `available_in(&process_path())`, and `process_path()` is public for the callers that still mean
  the process's own. No default method (the trait's FR-021 rule). Call sites that meant the
  process `PATH` pass `process_path()` explicitly: `state.rs`'s launch gate (changed in U8) and six
  test files.
- test changes, stated: the signature change touches the test files' call sites mechanically, and
  `micold-client/tests/cli_availability_comes_from_the_service.rs`'s probe list moved from
  `provider().is_available()` to `provider().is_available(` and gained `available_in(`, because
  the old spelling can no longer match any call and the guard would have gone silently blind
- suite: the first full run caught three providers still passing the process `PATH` (a rustfmt
  re-wrap hid them from the edit) — `available_in.rs::a_cli_present_only_on_the_process_path_is_not_available_in_another_path`
  -> `got [Pi]`, 3285 passed, 1 failed. Fixed; core suite -> 1141 passed, 0 failed; the other
  3144 were green in that run and untouched since

## Structural step: the request carries its directory, protocol 13 -> 14 (T065)

- change: `ClientMsg::AiCliAvailabilityRequest` gains `cwd: Option<PathBuf>`; `PROTOCOL_VERSION`
  13 -> 14 (`SCHEMA_HASH` regenerates in `build.rs`); the two literal pins
  (`protocol_auth.rs::the_protocol_version_is_fourteen`, `schema_hash.rs`'s
  `FEATURE_026_PROTOCOL_VERSION`) follow the bump the way their comments ask; the daemon ignores
  the field and the client sends `None` until U9 and U11/U12 drive them
- why here: A1's test cannot compile without the field (`error[E0559]: variant
  ClientMsg::AiCliAvailabilityRequest has no field named cwd`), so this is the minimal declaration
  its red needs
- suite: core -> 1141 passed, 0 failed; `ai_cli_availability` old tests and
  `cli_availability_comes_from_the_service` green
- commit: `f7b3e5a8`

## Outer loop opened: A1 (RED)

- test: `crates/micold-daemon/tests/ai_cli_availability.rs::a_cli_on_the_path_env_include_adds_for_a_directory_is_offered_for_it` (new)
- red: `scripts/build-lock.sh cargo test --test ai_cli_availability a_cli_on_the_path_env_include_adds_for_a_directory_is_offered_for_it -- --exact`
  -> `left: []` / `right: [Pi]` (1 failed). Kept out of commits until it is green.

## Cycle 3: U4 the answer for a directory uses that directory's env-include `PATH`

- test: `crates/micold-daemon/tests/ai_cli_availability.rs::the_answer_for_a_directory_walks_the_path_env_include_resolves_there` (new)
- red: `scripts/build-lock.sh cargo test --test ai_cli_availability the_answer_for_a_directory_walks_the_path_env_include_resolves_there -- --exact`
  -> first `error[E0599]: no method named ai_clis_available_in`; with a stub returning
  `available_here()`: `left: []` / `right: [Pi]` (1 failed)
- green: `crates/micold-daemon/src/state.rs` `DaemonState::ai_clis_available_in(cwd)` takes the
  `PATH` entry (case-insensitive, for Windows' `Path`) from `env_include_vars_for(cwd)` and asks
  `available_in`. No fallback yet: U5/U6 drive it. File -> 3 passed, A1 still red (the handler
  does not call it yet)
- refactor: none

## Cycle 4: U5 with env-include off, the answer uses the process's own `PATH`

- test: `crates/micold-daemon/tests/ai_cli_availability.rs::with_env_include_off_the_answer_walks_the_services_own_path` (new)
- red: `scripts/build-lock.sh cargo test --test ai_cli_availability with_env_include_off_the_answer_walks_the_services_own_path -- --exact`
  -> `left: []` / `right: [Pi]` (1 failed)
- green: `ai_clis_available_in` falls back to `process_path()` when the resolved environment
  carries no `PATH`. File -> 4 passed, A1 still red
- refactor: none

## Cycle 5: U6 env-include on, a result with no `PATH` falls back to the process's own

- test: `crates/micold-daemon/tests/ai_cli_availability.rs::a_script_that_leaves_path_alone_answers_from_the_services_own_path` (new)
- red: passed on first run — cycle 4's fallback covers it. Deliberate mutant: the fallback
  replaced by `.unwrap_or_default()`. `scripts/build-lock.sh cargo test --test ai_cli_availability a_script_that_leaves_path_alone_answers_from_the_services_own_path -- --exact`
  -> `left: []` / `right: [Pi]` (1 failed). Mutant reverted (`git diff` on `state.rs` empty);
  file -> 5 passed, A1 still red
- green: no production change
- refactor: none

## Cycle 6: U7 a second answer for the same directory is served from the shared cache

- test: `crates/micold-daemon/tests/ai_cli_availability.rs::a_second_answer_for_a_directory_does_not_run_the_script_again` (new)
- red: passed on first run — `env_include_vars_for` already caches per directory, and U4's green
  goes through it. Deliberate mutant: `self.invalidate_env_include(cwd)` at the top of
  `ai_clis_available_in`. `scripts/build-lock.sh cargo test --test ai_cli_availability a_second_answer_for_a_directory_does_not_run_the_script_again -- --exact`
  -> `left: 2` / `right: 1` (1 failed). Mutant removed (`git diff` on `state.rs` empty); file ->
  6 passed, A1 still red
- green: no production change
- refactor: none

## Cycle 7: U9 a request with no `cwd` resolves the environment in the home directory

- test: `crates/micold-daemon/tests/ai_cli_availability.rs::a_request_with_no_directory_is_answered_for_the_home_directory` (new)
- red: `scripts/build-lock.sh cargo test --test ai_cli_availability a_request_with_no_directory_is_answered_for_the_home_directory -- --exact`
  -> `the script ran in ""` / `left: None` / `right: Some("/home/jaro")` (1 failed): the handler
  still answered from `available_here()` and never sourced anything
- green: `crates/micold-daemon/src/server.rs` answers `AiCliAvailabilityRequest { req, cwd }` from
  `state.ai_clis_available_in(cwd or the home directory)`, and from `available_here()` only when
  there is no home directory at all. Still inline on the connection loop: U10 drives it off.
  File -> 8 passed; daemon crate `scripts/build-lock.sh cargo test -p micold-daemon --all-targets`
  -> 351 passed, 0 failed
- refactor: none

## Outer loop: A1 GREEN

- `a_cli_on_the_path_env_include_adds_for_a_directory_is_offered_for_it` passes with cycle 7's
  handler change (same run: file -> 8 passed). Committed with cycle 7.

## Cycle 8: U10 a slow env-include resolution does not hold up the next request

- test: `crates/micold-daemon/tests/ai_cli_availability.rs::a_slow_environment_does_not_hold_up_the_next_request_on_the_connection` (new)
- red: `scripts/build-lock.sh cargo test --test ai_cli_availability a_slow_environment_does_not_hold_up_the_next_request_on_the_connection -- --exact`
  -> `the availability answer arrived before the request sent after it: resolving the environment
  held up the connection loop for the script's 3s` (1 failed, 3.08s)
- green: the handler clones the state into a `tokio::spawn`ed task that resolves on
  `spawn_blocking` and sends `AiCliAvailability` when done — the BUG-009 pattern the worktree-create
  arm already uses. The state lock is never held across the resolution (`env_include_vars_for`
  drops it before sourcing). File -> 9 passed; daemon crate -> 352 passed, 0 failed
- refactor: the handler's comment rewritten for FR-003b (which environment, why spawned); no code
  moved

## Cycle 9: A3 with env-include off and `pi` absent from the service's `PATH`, it is not offered

- test: `crates/micold-daemon/tests/ai_cli_availability.rs::with_env_include_off_a_cli_only_the_script_would_add_is_not_offered` (new; helper `service_with_script` split out of `service_with` so a script can be configured while the feature is off)
- red: passed on first run — `env_include_vars_for` already short-circuits when disabled. Deliberate
  mutant: its `if !enabled || …` guard changed to `if (false && !enabled) || …` (env-include
  always on). `scripts/build-lock.sh cargo test --test ai_cli_availability with_env_include_off_a_cli_only_the_script_would_add_is_not_offered -- --exact`
  -> `got [Pi]` (1 failed). Mutant reverted (`git diff` on `state.rs` empty); file -> 10 passed
- green: no production change
- refactor: none
- notes: T063 was ticked one commit early (at cycle 8, before A3 ran); it is correct from this
  commit on

## Cycle 10: A2 / U8 the launch gate uses the spawn-environment `PATH`

- test: `crates/micold-daemon/tests/session_start.rs::a_cli_only_on_the_env_include_path_starts_rather_than_being_reported_missing` (new; one test for both ids — A2 is the observable start, U8 the gate that decides it; the stub's argv log also shows the spawn resolved `pi` on the session's `PATH`)
- red: `scripts/build-lock.sh cargo test --test session_start a_cli_only_on_the_env_include_path_starts_rather_than_being_reported_missing -- --exact`
  -> `Err(Custom { kind: NotFound, error: "Pi Coding Agent isn't installed. Install it, or start
  this session on another AI CLI." })` (1 failed)
- green: `start_session`'s gate asks `provider.is_available(&self.spawn_path_for(&plan.cwd))`.
  `spawn_path_for` is extracted from `ai_clis_available_in`, so the offer and the gate share one
  resolution. The portable-pty spawn needed no change: `CommandBuilder` resolves the program with
  its own `PATH` entry, which the env-include result sets.
- regressions, fixed in this cycle: `session_start.rs::starting_a_session_whose_cli_is_absent_reports_it_and_spends_no_restart_budget`,
  `session_start.rs::a_missing_cli_is_advised_on_where_sessions_run_and_on_what_is_being_started`,
  `catalog_join.rs::the_reason_a_start_failed_reaches_the_client_as_something_to_read`,
  `resume_failure_reported.rs::a_resume_that_fails_reaches_the_client`. Their catalogs loaded
  default settings — env-include on, sourcing the developer's own `~/.bashrc`, which on this
  machine puts `pi` and `copilot` on `PATH` — so with the gate now walking that `PATH` the CLI they
  hide was found. Not weakened: each file's `catalog_with_ai_cli_session` now writes
  env-include-off settings (session_start's only when the test wrote none), which restores the
  premise their `NoCliOnPath` guards state. Assertions untouched.
- suite: `session_start` -> 19 passed; daemon crate -> 354 passed, 0 failed
- refactor: `spawn_path_for` extraction (above), done while green

## Structural step: the two 027 availability tests state env-include off

- change: `ai_cli_availability.rs::the_service_reports_the_clis_on_its_own_path` and
  `::an_environment_with_no_cli_reports_an_empty_set_rather_than_failing` build their service with
  `service_with(store, None)` instead of `Catalog::ephemeral()`. They still passed, but only
  because their scratch `PATH` hides `bash` and the default `~/.bashrc` resolution failed; the
  premise ("the service's own `PATH` is what a session gets") is now explicit. Assertions untouched.
- suite: file -> 10 passed

## Cycle 11: U11 the start menu asks about its location's directory

- test: `crates/micold-client/src/main_tests.rs::tests::opening_the_start_menu_asks_about_its_locations_directory` (new, with helpers `connected_with_outbox` and `availability_asked_for`)
- red: `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide opening_the_start_menu_asks_about_its_locations_directory`
  -> `left: [None]` / `right: [Some("/repo/demo/.claude/worktrees/feature-x")]` (1 failed)
- green: `ask_cli_availability(app, cwd)` sends the directory it is given; the `StartMenuOpened`
  arm in `main.rs` passes `location.cwd(active project)`; Settings and the reconnect pass `None`.
  Client crate `scripts/build-lock.sh cargo test -p micold-client --all-targets` -> 1800 passed,
  0 failed (a first run died on `No space left on device`; another session's `cargo sweep`
  reclaimed the disk and the re-run is the one recorded)
- refactor: none

## Cycle 12: U12 opening Settings asks about no directory

- test: `crates/micold-client/src/main_tests.rs::tests::opening_settings_asks_about_no_directory` (new)
- red: passed on first run — cycle 11's green left Settings on `None`. Deliberate mutant:
  `persist.rs` passing `app.core.workspace.active.clone()`.
  `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide opening_settings_asks_about_no_directory`
  -> `left: [Some("/repo/demo")]` / `right: [None]` (1 failed). Mutant reverted (`git diff` on
  `persist.rs` empty); binary -> 168 passed
- green: no production change
- refactor: none

## Cycle 13: U13 (added mid-loop) an earlier request's answer does not replace a later one's

- why added: cycle 8 moved the answer off the service's connection loop, so two answers can now
  arrive in the reverse order of their requests — a first env-include resolution for a worktree
  takes up to the script's timeout while the home directory's is cached. Since cycle 11 each answer
  is about a different directory, so a late one would show another directory's set.
- test: `crates/micold-client/src/main_tests.rs::tests::an_answer_to_an_earlier_question_does_not_replace_a_later_one` (new)
- red: `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide an_answer_to_an_earlier_question_does_not_replace_a_later_one`
  -> `left: Some([ClaudeCode])` / `right: Some([Pi])` (1 failed)
- green: `App::cli_availability_asked` records the latest request's `req`; the reply arm drops an
  `AiCliAvailability` whose `req` is older. Client crate -> 1802 passed, 0 failed
- refactor: none
