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
