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

## Cycle 5: U5 `HistoryColor::Dim` accepts 0 and 7 and rejects 8

- test: `crates/micold-core/tests/terminal_history_snapshot.rs::dim_color_accepts_0_and_7_and_rejects_8` (new; foreground and background)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_snapshot` (one shared file run with U5, U6 and U7 written together: `test result: FAILED. 4 passed; 3 failed`)
  -> ``assertion `left == right` failed: Dim(8) is past the 8 colours that have a dim variant`` /
  `left: Ok(())` / `right: Err(ColorOutOfRange { line: 0 })`. No stub needed: `is_in_palette` accepted
  every `Dim` index.
- green: `is_in_palette` also checks `Dim` below `DIM_COLORS` = 8. File run -> 7 passed, 0 failed
- refactor: none needed
- commit: `feat(041): dim colours, emptiness and style flags of a snapshot (U5-U7)`

## Cycle 6: U6 an empty snapshot is `is_empty()`; one with a line is not

- test: `crates/micold-core/tests/terminal_history_snapshot.rs::an_empty_snapshot_is_empty_and_one_with_a_line_is_not` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_snapshot` (one shared file run with U5, U6 and U7 written together: `test result: FAILED. 4 passed; 3 failed`)
  -> `panicked at crates/micold-core/src/terminal_history/mod.rs:124:9:` / `not implemented: HistorySnapshot::is_empty`.
  The `is_empty` stub was declared first so the test compiles; its deliberate "not implemented" is the red.
- green: `is_empty` returns `self.lines.is_empty()` (a blank line still counts as something to
  show). File run -> 7 passed, 0 failed
- refactor: none needed
- commit: `feat(041): dim colours, emptiness and style flags of a snapshot (U5-U7)`

## Cycle 7: U7 `StyleFlags` round-trips each of bold, dim, italic, underline, inverse, strikethrough, hidden

- test: `crates/micold-core/tests/terminal_history_snapshot.rs::style_flags_round_trip_each_attribute` (new; each flag set is read back and sets no other)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_snapshot` (one shared file run with U5, U6 and U7 written together: `test result: FAILED. 4 passed; 3 failed`)
  -> `panicked at crates/micold-core/src/terminal_history/mod.rs:82:9:` / `not implemented: StyleFlags::with`.
  The seven flag constants (distinct bits) and `with`/`contains` stubs were declared first so the test
  compiles.
- green: `with` ORs the bits, `contains` checks every bit of the argument is set. File run -> 7 passed,
  0 failed
- refactor: none needed
- notes: "round-trips" is read as set-then-read-back through `with`/`contains`; no `bits`/`from_bits`
  API was added, since no listed behaviour needs one yet.
- commit: `feat(041): dim colours, emptiness and style flags of a snapshot (U5-U7)`

## Cycle 8: U8 at 80 columns `separator_line` is `── session restarted at 2026-10-02 14:31 +02:00 ──`

- test: `crates/micold-core/tests/terminal_history_text.rs::at_80_columns_the_separator_is_the_full_text` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_text` (one shared file run with U8 to U11 written together against an `unimplemented!` stub of `separator_line`, declared with `pub mod text;` so the file compiles: `test result: FAILED. 0 passed; 4 failed`)
  -> `panicked at crates/micold-core/src/terminal_history/text.rs:8:5:` / `not implemented: separator_line("2026-10-02 14:31 +02:00", 80)`
- green: `separator_line` formats `session restarted at {at}` and passes it to `fit`, which returns it between `── ` and ` ──` when that fits in `columns`, else the bare text cut to `columns` characters. File run -> 4 passed, 0 failed
- refactor: none beyond the shared `fit` helper written in the green step (U12/U13's `notice_line` follows the same rule)
- notes: the stub's deliberate "not implemented" is the red, as in cycle 1; this test was not run against a partial implementation.
- commit: `feat(041): the session restarted separator line (U8-U11)`

## Cycle 9: U9 at a width narrower than the full text the rules are dropped (49 and 44 columns)

- test: `crates/micold-core/tests/terminal_history_text.rs::narrower_than_the_full_text_the_rules_are_dropped` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_text` (one shared file run with U8 to U11 written together against an `unimplemented!` stub of `separator_line`, declared with `pub mod text;` so the file compiles: `test result: FAILED. 0 passed; 4 failed`)
  -> `panicked at crates/micold-core/src/terminal_history/text.rs:8:5:` / `not implemented: separator_line("2026-10-02 14:31 +02:00", 49)`
- green: `separator_line` formats `session restarted at {at}` and passes it to `fit`, which returns it between `── ` and ` ──` when that fits in `columns`, else the bare text cut to `columns` characters. File run -> 4 passed, 0 failed
- refactor: none beyond the shared `fit` helper written in the green step (U12/U13's `notice_line` follows the same rule)
- notes: the stub's deliberate "not implemented" is the red, as in cycle 1; this test was not run against a partial implementation.
- commit: `feat(041): the session restarted separator line (U8-U11)`

## Cycle 10: U10 at a width narrower than the text without rules the text is cut to the width (43 columns)

- test: `crates/micold-core/tests/terminal_history_text.rs::narrower_than_the_text_without_rules_the_text_is_cut_to_the_width` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_text` (one shared file run with U8 to U11 written together against an `unimplemented!` stub of `separator_line`, declared with `pub mod text;` so the file compiles: `test result: FAILED. 0 passed; 4 failed`)
  -> `panicked at crates/micold-core/src/terminal_history/text.rs:8:5:` / `not implemented: separator_line("2026-10-02 14:31 +02:00", 43)`
- green: `separator_line` formats `session restarted at {at}` and passes it to `fit`, which returns it between `── ` and ` ──` when that fits in `columns`, else the bare text cut to `columns` characters. File run -> 4 passed, 0 failed
- refactor: none beyond the shared `fit` helper written in the green step (U12/U13's `notice_line` follows the same rule)
- notes: the stub's deliberate "not implemented" is the red, as in cycle 1; this test was not run against a partial implementation.
- commit: `feat(041): the session restarted separator line (U8-U11)`

## Cycle 11: U11 the separator is never wider than `columns` and never holds a line break, at widths 1, 2, 50 and 49

- test: `crates/micold-core/tests/terminal_history_text.rs::the_separator_is_never_wider_than_the_columns_and_never_breaks_the_line` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_text` (one shared file run with U8 to U11 written together against an `unimplemented!` stub of `separator_line`, declared with `pub mod text;` so the file compiles: `test result: FAILED. 0 passed; 4 failed`)
  -> `panicked at crates/micold-core/src/terminal_history/text.rs:8:5:` / `not implemented: separator_line("2026-10-02 14:31 +02:00", 1)`
- green: `separator_line` formats `session restarted at {at}` and passes it to `fit`, which returns it between `── ` and ` ──` when that fits in `columns`, else the bare text cut to `columns` characters. File run -> 4 passed, 0 failed
- refactor: none beyond the shared `fit` helper written in the green step (U12/U13's `notice_line` follows the same rule)
- notes: the stub's deliberate "not implemented" is the red, as in cycle 1; this test was not run against a partial implementation.
- commit: `feat(041): the session restarted separator line (U8-U11)`

## Cycle 12: U14 `capture` returns the text and order of history rows then screen rows

- test: `crates/micold-daemon/src/history.rs::tests::capture_returns_the_history_rows_then_the_screen_rows_in_order` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --lib history::` (one shared run with U14 to U22 written together against a fake `capture` that returns every screen row as an empty line, nothing trimmed: `test result: FAILED. 0 passed; 9 failed`)
  -> `assertion `left == right` failed` / `left: ["", "", ""]` / `right: ["one", "two", "three", "four", "five"]`
- green: `capture` walks the buffer from the oldest history row to the last screen row that shows anything (the `plain_tail` rule), joins rows whose last cell has `WRAPLINE`, skips spacer cells, appends zero-width marks to their base, ends a non-wrapping row at its last non-empty cell, and maps colours and flags through the explicit `BASIC_COLORS`/`DIM_COLORS`/`STYLE_FLAGS` tables (unknown named colour -> `Default`). Shared run -> 9 passed, 0 failed, no warnings
- refactor: none needed
- notes: assertion-level red from the shared fake; this test was not run against a partial implementation.
- commit: `feat(041): capture a terminal's history with colours and styles (U14-U22)`

## Cycle 13: U15 each of the 16 basic colours is captured as foreground and as background

- test: `crates/micold-daemon/src/history.rs::tests::each_of_the_16_basic_colours_is_captured_as_foreground_and_as_background` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --lib history::` (one shared run with U14 to U22 written together against a fake `capture` that returns every screen row as an empty line, nothing trimmed: `test result: FAILED. 0 passed; 9 failed`)
  -> `assertion `left == right` failed` / `left: 20` / `right: 16` (the line count, before the per-colour assertions)
- green: `capture` walks the buffer from the oldest history row to the last screen row that shows anything (the `plain_tail` rule), joins rows whose last cell has `WRAPLINE`, skips spacer cells, appends zero-width marks to their base, ends a non-wrapping row at its last non-empty cell, and maps colours and flags through the explicit `BASIC_COLORS`/`DIM_COLORS`/`STYLE_FLAGS` tables (unknown named colour -> `Default`). Shared run -> 9 passed, 0 failed, no warnings
- refactor: none needed
- notes: assertion-level red from the shared fake; this test was not run against a partial implementation.
- commit: `feat(041): capture a terminal's history with colours and styles (U14-U22)`

## Cycle 14: U16 an indexed colour and an RGB colour are captured as foreground and as background

- test: `crates/micold-daemon/src/history.rs::tests::an_indexed_and_an_rgb_colour_are_captured_as_foreground_and_as_background` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --lib history::` (one shared run with U14 to U22 written together against a fake `capture` that returns every screen row as an empty line, nothing trimmed: `test result: FAILED. 0 passed; 9 failed`)
  -> `assertion `left == right` failed` / `left: ["", "", ""]` / `right: ["ABCD"]`
- green: `capture` walks the buffer from the oldest history row to the last screen row that shows anything (the `plain_tail` rule), joins rows whose last cell has `WRAPLINE`, skips spacer cells, appends zero-width marks to their base, ends a non-wrapping row at its last non-empty cell, and maps colours and flags through the explicit `BASIC_COLORS`/`DIM_COLORS`/`STYLE_FLAGS` tables (unknown named colour -> `Default`). Shared run -> 9 passed, 0 failed, no warnings
- refactor: none needed
- notes: assertion-level red from the shared fake; this test was not run against a partial implementation.
- commit: `feat(041): capture a terminal's history with colours and styles (U14-U22)`

## Cycle 15: U17 each of bold, dim, italic, underline, inverse, strikethrough is captured

- test: `crates/micold-daemon/src/history.rs::tests::each_style_flag_is_captured_and_every_underline_kind_is_underline` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --lib history::` (one shared run with U14 to U22 written together against a fake `capture` that returns every screen row as an empty line, nothing trimmed: `test result: FAILED. 0 passed; 9 failed`)
  -> `assertion `left == right` failed` / `left: ["", "", ""]` / `right: ["x-x-x-x-x-x-x-x-"]`
- green: `capture` walks the buffer from the oldest history row to the last screen row that shows anything (the `plain_tail` rule), joins rows whose last cell has `WRAPLINE`, skips spacer cells, appends zero-width marks to their base, ends a non-wrapping row at its last non-empty cell, and maps colours and flags through the explicit `BASIC_COLORS`/`DIM_COLORS`/`STYLE_FLAGS` tables (unknown named colour -> `Default`). Shared run -> 9 passed, 0 failed, no warnings
- refactor: none needed
- notes: assertion-level red from the shared fake. Hidden (SGR 8) and a curly underline (SGR 4:3 -> underline) are covered too, since T005 asks for each flag of `StyleFlags`.
- commit: `feat(041): capture a terminal's history with colours and styles (U14-U22)`

## Cycle 16: U18 two rows joined by the wrap flag are one `LogicalLine`

- test: `crates/micold-daemon/src/history.rs::tests::two_rows_joined_by_the_wrap_flag_are_one_logical_line` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --lib history::` (one shared run with U14 to U22 written together against a fake `capture` that returns every screen row as an empty line, nothing trimmed: `test result: FAILED. 0 passed; 9 failed`)
  -> `assertion `left == right` failed` / `left: ["", "", "", ""]` / `right: ["abcdefgh", "next"]`
- green: `capture` walks the buffer from the oldest history row to the last screen row that shows anything (the `plain_tail` rule), joins rows whose last cell has `WRAPLINE`, skips spacer cells, appends zero-width marks to their base, ends a non-wrapping row at its last non-empty cell, and maps colours and flags through the explicit `BASIC_COLORS`/`DIM_COLORS`/`STYLE_FLAGS` tables (unknown named colour -> `Default`). Shared run -> 9 passed, 0 failed, no warnings
- refactor: none needed
- notes: assertion-level red from the shared fake; this test was not run against a partial implementation.
- commit: `feat(041): capture a terminal's history with colours and styles (U14-U22)`

## Cycle 17: U19 a wide character counts as one character and its spacer is skipped

- test: `crates/micold-daemon/src/history.rs::tests::a_wide_character_counts_as_one_character_and_its_spacer_is_skipped` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --lib history::` (one shared run with U14 to U22 written together against a fake `capture` that returns every screen row as an empty line, nothing trimmed: `test result: FAILED. 0 passed; 9 failed`)
  -> `assertion `left == right` failed` / `left: ["", "", "", ""]` / `right: ["a世b", "abc世"]`
- green: `capture` walks the buffer from the oldest history row to the last screen row that shows anything (the `plain_tail` rule), joins rows whose last cell has `WRAPLINE`, skips spacer cells, appends zero-width marks to their base, ends a non-wrapping row at its last non-empty cell, and maps colours and flags through the explicit `BASIC_COLORS`/`DIM_COLORS`/`STYLE_FLAGS` tables (unknown named colour -> `Default`). Shared run -> 9 passed, 0 failed, no warnings
- refactor: none needed
- notes: assertion-level red from the shared fake. The second line puts the wide character at a wrap, so the leading spacer is skipped as well as the trailing one.
- commit: `feat(041): capture a terminal's history with colours and styles (U14-U22)`

## Cycle 18: U20 a zero-width character follows its base character

- test: `crates/micold-daemon/src/history.rs::tests::a_zero_width_character_follows_its_base_character` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --lib history::` (one shared run with U14 to U22 written together against a fake `capture` that returns every screen row as an empty line, nothing trimmed: `test result: FAILED. 0 passed; 9 failed`)
  -> `assertion `left == right` failed` / `left: ["", "", ""]` / `right: ["e\u{301}x"]`
- green: `capture` walks the buffer from the oldest history row to the last screen row that shows anything (the `plain_tail` rule), joins rows whose last cell has `WRAPLINE`, skips spacer cells, appends zero-width marks to their base, ends a non-wrapping row at its last non-empty cell, and maps colours and flags through the explicit `BASIC_COLORS`/`DIM_COLORS`/`STYLE_FLAGS` tables (unknown named colour -> `Default`). Shared run -> 9 passed, 0 failed, no warnings
- refactor: none needed
- notes: assertion-level red from the shared fake; this test was not run against a partial implementation.
- commit: `feat(041): capture a terminal's history with colours and styles (U14-U22)`

## Cycle 19: U21 trailing empty screen rows are not captured

- test: `crates/micold-daemon/src/history.rs::tests::trailing_empty_screen_rows_are_not_captured` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --lib history::` (one shared run with U14 to U22 written together against a fake `capture` that returns every screen row as an empty line, nothing trimmed: `test result: FAILED. 0 passed; 9 failed`)
  -> `assertion `left == right` failed` / `left: ["", "", "", "", "", ""]` / `right: ["a", "", "b"]`
- green: `capture` walks the buffer from the oldest history row to the last screen row that shows anything (the `plain_tail` rule), joins rows whose last cell has `WRAPLINE`, skips spacer cells, appends zero-width marks to their base, ends a non-wrapping row at its last non-empty cell, and maps colours and flags through the explicit `BASIC_COLORS`/`DIM_COLORS`/`STYLE_FLAGS` tables (unknown named colour -> `Default`). Shared run -> 9 passed, 0 failed, no warnings
- refactor: none needed
- notes: assertion-level red from the shared fake; this test was not run against a partial implementation.
- commit: `feat(041): capture a terminal's history with colours and styles (U14-U22)`

## Cycle 20: U22 a `Term` that printed nothing gives an empty snapshot

- test: `crates/micold-daemon/src/history.rs::tests::a_term_that_printed_nothing_gives_an_empty_snapshot` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --lib history::` (one shared run with U14 to U22 written together against a fake `capture` that returns every screen row as an empty line, nothing trimmed: `test result: FAILED. 0 passed; 9 failed`)
  -> `assertion `left == right` failed` / `left: HistorySnapshot { lines: [LogicalLine { text: "", runs: [] }, LogicalLine { text: "", runs: [] }, LogicalLine { text: "", runs: [] }] }` / `right: HistorySnapshot { lines: [] }`
- green: `capture` walks the buffer from the oldest history row to the last screen row that shows anything (the `plain_tail` rule), joins rows whose last cell has `WRAPLINE`, skips spacer cells, appends zero-width marks to their base, ends a non-wrapping row at its last non-empty cell, and maps colours and flags through the explicit `BASIC_COLORS`/`DIM_COLORS`/`STYLE_FLAGS` tables (unknown named colour -> `Default`). Shared run -> 9 passed, 0 failed, no warnings
- refactor: none needed
- notes: the fake was chosen as "no trimming" rather than "empty snapshot" so this test, which an empty fake would pass, has its own assertion-level red.
- commit: `feat(041): capture a terminal's history with colours and styles (U14-U22)`
