# Cycle Log: Terminal History That Survives a Session Service Restart

Append only. Newest last. Every entry's `red` block is the evidence that the test
existed and failed before the implementation.

An entry is committed together with its test and code, so it cannot hold its own SHA: it names the
commit by subject, and the *Commit index* sections give the SHAs once the commits exist.

## Baseline

- suite: recorded by the autopilot unit that launched the M1 cycles (a workspace run of this
  worktree, in progress while cycle 1 ran); see the unit's entry in `autopilot.md`.
- commit: `8a220fb8` (branch head before any change), then `ee2780f0` (T001 scaffold: empty
  `terminal_history` and `history` modules, `chrono` for the daemon; `Cargo.lock` gained one line,
  `chrono` in `micold-daemon`'s dependency list, and no new crate)
- recorded: cycle 0, before any behaviour

## Cycle 1: U1 `validate` accepts a line whose runs' `chars` sum to its character count

- test: `crates/micold-core/tests/terminal_history_snapshot.rs::validate_accepts_a_line_whose_runs_sum_to_its_character_count` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_snapshot validate_accepts_a_line_whose_runs_sum_to_its_character_count -- --exact`
  -> `panicked at crates/micold-core/src/terminal_history/mod.rs:64:9: not implemented: HistorySnapshot::validate`
  (`test result: FAILED. 0 passed; 1 failed`). The data types of DM §1 and a `validate` stub were
  declared first, because the test cannot compile without the symbols; the stub's deliberate
  "not implemented" is the red.
- green: `validate` returns `Ok(())` (fake it; U2 forces the comparison).
  `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_snapshot` -> 1 passed, 0 failed
- refactor: none needed
- commit: `feat(041): a snapshot whose runs cover its text is valid (U1)`
