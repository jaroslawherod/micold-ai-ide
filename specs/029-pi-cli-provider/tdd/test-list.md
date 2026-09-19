---
feature: 029-pi-cli-provider
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 3 # in scope: US1 scenario 5b, SC-001a, SC-006a (amended) — BUG-001 only
planned_at: e54d6cd3
updated_at: e54d6cd3
suite_baseline: green # 3276 passed, 0 failed at e54d6cd3; see cycle-log.md
---

# Test List: Run a session on the Pi coding agent — BUG-001 increment

**Scope**: BUG-001 only (FR-003b, SC-001a, SC-006a as amended, US1 scenario 5b; tasks T062–T070 and
the tasks this plan adds). Phases 1–7 shipped in PR #300 without a test list and are out of scope.

## Outer loop: acceptance behaviors

The profile's acceptance runner (`sandbox_real_*`) drives the real container runtime and does not
reach this defect, which is host placement. These are therefore **daemon integration tests over the
real protocol** (`serve_connection` over a duplex stream, a real env-include script run by a real
`bash`). That is the highest real entry point the repository can test without a GUI. The GUI half
is covered by T070's visual pass.

| id  | behavior                                                                                                                                                       | traces                      | kind    | state   | test |
| --- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------- | ------- | ------- | ---- |
| A1  | With `pi` only on the `PATH` that env-include adds for a directory, an `AiCliAvailabilityRequest` for that directory lists Pi                                     | US1-5b, SC-001a, FR-003b    | example | DONE    | `a_cli_on_the_path_env_include_adds_for_a_directory_is_offered_for_it` (crates/micold-daemon/tests/ai_cli_availability.rs) |
| A2  | With `pi` only on the env-include `PATH` for a directory, starting a Pi session there is not refused as a missing CLI                                            | US1-5b, SC-001a, FR-003b    | example | PENDING |      |
| A3  | With env-include off and `pi` absent from the service's own `PATH`, the same request does not list Pi                                                            | US1-5, FR-003, FR-003b      | example | DONE    | `with_env_include_off_a_cli_only_the_script_would_add_is_not_offered` (crates/micold-daemon/tests/ai_cli_availability.rs) |

## Inner loop: unit behaviors

### `crates/micold-core/src/provider.rs`

| id  | behavior                                                                                                   | traces            | kind    | state   | test |
| --- | ---------------------------------------------------------------------------------------------------------- | ----------------- | ------- | ------- | ---- |
| U1  | `available_in(path)` lists a CLI that is present only in a directory of the given `PATH` value             | FR-003b           | example | DONE    | `a_cli_present_only_in_the_given_path_is_available` (crates/micold-core/tests/available_in.rs) |
| U2  | `available_in(path)` omits a CLI that is present only on the process's own `PATH`, not in the given value  | FR-003b           | example | DONE    | `a_cli_present_only_on_the_process_path_is_not_available_in_another_path` (crates/micold-core/tests/available_in.rs) |
| U3  | `available_here()` still answers from the process's own `PATH`                                              | FR-003, FR-023c(027) | example | DONE | `available_here` (crates/micold-core/tests/available_here.rs) |

### `crates/micold-daemon/src/state.rs`

| id  | behavior                                                                                                                    | traces            | kind    | state   | test |
| --- | --------------------------------------------------------------------------------------------------------------------------- | ----------------- | ------- | ------- | ---- |
| U4  | The availability answer for a directory uses the `PATH` from that directory's env-include result when env-include is on      | FR-003b           | example | DONE    | `the_answer_for_a_directory_walks_the_path_env_include_resolves_there` (crates/micold-daemon/tests/ai_cli_availability.rs) |
| U5  | With env-include off, the availability answer uses the process's own `PATH`                                                  | FR-003b           | example | DONE    | `with_env_include_off_the_answer_walks_the_services_own_path` (crates/micold-daemon/tests/ai_cli_availability.rs) |
| U6  | With env-include on but a result that carries no `PATH` (the script left it unchanged), the process's own `PATH` is used      | FR-003b           | example | DONE    | `a_script_that_leaves_path_alone_answers_from_the_services_own_path` (crates/micold-daemon/tests/ai_cli_availability.rs) |
| U7  | A second availability answer for the same directory is served from the shared cache: the script runs once, not twice         | SC-006a, FR-003b  | example | DONE    | `a_second_answer_for_a_directory_does_not_run_the_script_again` (crates/micold-daemon/tests/ai_cli_availability.rs) |
| U8  | The start-time availability gate in `start_session` uses the same resolution as U4, so it does not refuse what was offered    | FR-003b, SC-001a  | example | PENDING |      |

### `crates/micold-daemon/src/server.rs`

| id  | behavior                                                                                                                    | traces   | kind    | state   | test |
| --- | --------------------------------------------------------------------------------------------------------------------------- | -------- | ------- | ------- | ---- |
| U9  | A request with no `cwd` resolves the environment in the user's home directory                                                | FR-003b  | example | DONE    | `a_request_with_no_directory_is_answered_for_the_home_directory` (crates/micold-daemon/tests/ai_cli_availability.rs) |
| U10 | While a slow env-include resolution runs for an availability request, another request on the same connection is answered     | FR-003b  | example | DONE    | `a_slow_environment_does_not_hold_up_the_next_request_on_the_connection` (crates/micold-daemon/tests/ai_cli_availability.rs) |

### `crates/micold-client/src/main.rs`, `shell/persist.rs`, `shell/daemon_sync.rs`

| id  | behavior                                                                                                   | traces  | kind    | state   | test |
| --- | ---------------------------------------------------------------------------------------------------------- | ------- | ------- | ------- | ---- |
| U11 | Opening the per-session start menu for a location sends an availability request carrying that location's directory | FR-003b | example | PENDING |      |
| U12 | Opening Settings sends an availability request with no directory                                            | FR-003b | example | PENDING |      |

## Invariants and edge cases still to place

- Windows `PATHEXT` handling under `available_in`: unchanged code path, exercised only on the
  `windows-latest` CI leg. Existing coverage in `crates/micold-core/tests/copilot_provider.rs` stays
  the check; no new behavior.

## Out of scope

- Phases 1–7 of feature 029: shipped in PR #300 without a TDD list; not re-planned here.
- Sandboxed placement: FR-003b needs no change there (the daemon already runs inside the sandbox);
  existing `sandbox_real_*` tests cover it.
- Explaining in the UI why a CLI is hidden: not a requirement; documentation only (T068).
- The fixture path in the user's `settings.json` (T069): an investigation, not a behavior.
- Protocol bump 13 → 14 and schema hash (T065): structural; the existing schema-hash gate fails
  until it is regenerated.

## Verification commands

Copied verbatim from `.specify/memory/tdd-profile.md` at planning time:

- Single test: `scripts/build-lock.sh cargo test --test {file} {name} -- --exact`
- Unit test in `src/`: `scripts/build-lock.sh cargo test -p {crate} --lib {module}::tests::{name} -- --exact`
- File: `scripts/build-lock.sh cargo test --test {file}`
- Full suite: `scripts/build-lock.sh cargo test --workspace`
- Fast subset: `scripts/build-lock.sh cargo test -p micold-core --all-targets`
- Coverage: none (no `cargo-llvm-cov`)
- Mutation: none (no `cargo-mutants`); deliberate-mutant spot checks by hand
- Assert on the observed `N passed` count, never on the exit code alone: a filter matching nothing
  exits 0.
