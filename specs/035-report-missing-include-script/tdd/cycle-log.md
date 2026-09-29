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
