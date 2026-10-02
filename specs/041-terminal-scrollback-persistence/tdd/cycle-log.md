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

## Cycle 2: U2 `validate` rejects a line whose run sum is one more, and one less, than its character count

- test: `crates/micold-core/tests/terminal_history_snapshot.rs::validate_rejects_a_line_whose_run_sum_is_one_more_or_one_less_than_its_character_count` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_snapshot validate_rejects_a_line_whose_run_sum_is_one_more_or_one_less_than_its_character_count -- --exact`
  -> `assertion \`left == right\` failed: runs summing to 6 do not cover a line of 5 characters` /
  `left: Ok(())` / `right: Err(RunsDoNotCoverText { line: 0 })` (1 failed). The
  `SnapshotError::RunsDoNotCoverText` variant was declared first so the test compiles.
- green: `validate` sums each line's run `chars` (as `u64`, so no overflow) and compares with
  `text.chars().count()`. File run -> 2 passed, 0 failed
- refactor: none needed
- commit: `feat(041): a snapshot whose runs miss or overrun its text is invalid (U2)`

## Cycle 3: U3 `validate` rejects a `text` holding a C0 character, a C1 character and `ESC`

- test: `crates/micold-core/tests/terminal_history_snapshot.rs::validate_rejects_a_line_holding_a_c0_a_c1_or_an_escape_character` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_snapshot validate_rejects_a_line_holding_a_c0_a_c1_or_an_escape_character -- --exact`
  -> ``assertion `left == right` failed: '\u{7}' is a control character, which a line's text never holds`` /
  `left: Ok(())` / `right: Err(ControlCharacter { line: 0 })` (1 failed). The
  `SnapshotError::ControlCharacter` variant was declared first so the test compiles.
- green: `validate` rejects a line whose text has any `char::is_control` character (C0, `DEL`
  and C1). File run -> 3 passed, 0 failed
- refactor: none needed
- commit: `feat(041): a snapshot whose text holds a control character is invalid (U3)`

## Cycle 4: U4 `HistoryColor::Basic` accepts 0 and 15 and rejects 16

- test: `crates/micold-core/tests/terminal_history_snapshot.rs::basic_color_accepts_0_and_15_and_rejects_16` (new; foreground and background)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_snapshot basic_color_accepts_0_and_15_and_rejects_16 -- --exact`
  with the palette check disabled -> ``assertion `left == right` failed: Basic(16) is past the 16 basic colours`` /
  `left: Ok(())` / `right: Err(ColorOutOfRange { line: 0 })` (1 failed). The
  `SnapshotError::ColorOutOfRange` variant was declared first so the test compiles. (Written by part 2's
  worker, which stopped before committing; part 3 re-proved red and green on the same diff.)
- green: `validate` checks each run's `fg` and `bg` with `HistoryColor::is_in_palette` (`Basic` below
  `BASIC_COLORS` = 16). File run -> 4 passed, 0 failed
- refactor: none needed
- commit: `feat(041): a basic colour past 15 is invalid (U4)`
