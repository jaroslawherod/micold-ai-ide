# Cycle Log: Create a Worktree from a GitHub Issue

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and
failed before the implementation.

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace` -> not yet measured
- commit: `d6c2f33e` (test list planned against it)
- recorded: cycle 0. The design PR changes no code, so the full suite is not run for it. M1 starts
  from a fresh `origin/main` after this PR merges; its first entry below runs the suite on that
  base and records the counts before any red.

## Baseline (measured, M1)

- suite: `scripts/build-lock.sh cargo test --workspace` -> 3662 passed, 0 failed, 8 ignored, 341
  binaries (exit 0)
- commit: `13967080` (`origin/main` with design PR #459 merged)
- recorded: before any M1 change

## Notes and deviations (M1)

- **Batched reds, per test file.** As in feature 033's M1: every test of one file is written first,
  run against the code as it stands (or a stub that only makes the symbols resolve), and each is
  observed failing on its own assertion. A test that passed at that point is held to the
  deliberate-mutant check after green. The inner loop runs the one test target; the per-cycle suite
  is `mise run test-core` (the core crate, where every M1 behaviour lives); the full workspace suite
  runs in `mise run gate` before the PR.
- **Structural move first.** T010's move of `run_bounded` / `RunOutcome` / `kill_process_group` to
  `process.rs` is behaviour-preserving, so it went in as its own refactor commit on green
  (`refactor(034): move the bounded runner…`, `mise run test-core` 1264 passed, 0 failed) before
  cycle 1's red. That is also U5's baseline: the env-include suites passed unchanged across it.

## Cycle 1: U1–U5 — the bounded runner drains its pipes

- test: `crates/micold-core/tests/process_run_bounded.rs` (new), 4 tests plus `child_helper` (the
  child is the test binary re-run with `MICOLD_RUN_BOUNDED_CHILD`, so no shell on any OS)
- red: `scripts/build-lock.sh cargo test -p micold-core --test process_run_bounded`
  -> `test result: FAILED. 4 passed; 1 failed`. Decisive line,
  `large_output_is_drained_while_waiting`: `output larger than a pipe buffer must not stall the
  child into a timeout, got TimedOut { stderr: "" } after 5.001466872s`.
  Passed before the change (behaviour the moved runner already had): U1
  `a_child_past_the_bound_is_killed_and_reported`, U2 `a_child_inside_the_bound_exits_normally`,
  U4 `a_failing_child_reports_status_and_output`.
- green: `process::run_bounded` reads stdout and stderr on two `drain` threads started right after
  spawn and joins them after the group kill; stderr is decoded lossily. -> `5 passed; 0 failed`;
  `mise run test-core` 1269 passed, 0 failed (U5: `env_include*.rs` unchanged and green)
- mutant check (the three that passed first): group kill removed -> U1 FAILED; poll bound replaced
  by `Duration::ZERO` -> U2 and U4 FAILED; exit code forced to 0 -> U4 FAILED. Code restored each
  time, 5 passed again.
- refactor: none needed
- commit: the commit that adds this entry
