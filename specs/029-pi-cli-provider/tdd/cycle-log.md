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
