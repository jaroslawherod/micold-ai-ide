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

## Cycle 21: U23 capture after `seed(Seed::History)` equals the input lines followed by the separator in the dim style

- test: `crates/micold-daemon/src/history.rs::tests::capture_after_a_seed_is_the_input_lines_then_the_separator_in_the_dim_style` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --lib history::` (one shared run with U23 to U28 written together against a fake `seed` that writes `unseeded` at the cursor whatever it is given: `test result: FAILED. 9 passed; 6 failed` (the 9 passing are U14 to U22))
  -> `assertion `left == right` failed` / `left: [LogicalLine { text: "unseeded", runs: [StyleRun { chars: 8, ... }] }]` / `right: [LogicalLine { text: "plain", ...`
- green: `seed` returns on `Seed::None`; for `Seed::History` it keeps the last `limit + screen rows` lines, writes each run through `Handler::terminal_attribute` (reset, colours, flags via `FLAG_ATTRIBUTES`) and `input`, then reset, `carriage_return`, `linefeed`; then the `separator_line` for `at` formatted `%Y-%m-%d %H:%M %:z` in the dim style the same way; then `clear_screen(ClearMode::All)` and `goto(0, 0)`. Shared run -> 15 passed, 0 failed, no warnings
- refactor: none needed
- notes: assertion-level red from the shared fake; this test was not run against a partial implementation.
- commit: `feat(041): seed a terminal with its earlier history and the separator (U23-U28)`

## Cycle 22: U24 after seeding the screen is blank, the cursor is at home, attributes are reset and the seeded lines are all in the history

- test: `crates/micold-daemon/src/history.rs::tests::after_a_seed_the_screen_is_blank_at_home_with_attributes_reset_and_the_lines_in_history` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --lib history::` (one shared run with U23 to U28 written together against a fake `seed` that writes `unseeded` at the cursor whatever it is given: `test result: FAILED. 9 passed; 6 failed` (the 9 passing are U14 to U22))
  -> `assertion `left == right` failed: lines and separator in history` / `left: 0` / `right: 3`
- green: `seed` returns on `Seed::None`; for `Seed::History` it keeps the last `limit + screen rows` lines, writes each run through `Handler::terminal_attribute` (reset, colours, flags via `FLAG_ATTRIBUTES`) and `input`, then reset, `carriage_return`, `linefeed`; then the `separator_line` for `at` formatted `%Y-%m-%d %H:%M %:z` in the dim style the same way; then `clear_screen(ClearMode::All)` and `goto(0, 0)`. Shared run -> 15 passed, 0 failed, no warnings
- refactor: none needed
- notes: assertion-level red from the shared fake. Attributes reset is observed by feeding `x` afterwards and capturing it in the default style, since a fresh term's blank screen and home cursor alone would pass on a no-op.
- commit: `feat(041): seed a terminal with its earlier history and the separator (U23-U28)`

## Cycle 23: U25 a snapshot longer than the `Term`'s history limit leaves the most recent lines, and one exactly at the limit leaves all

- test: `crates/micold-daemon/src/history.rs::tests::a_snapshot_longer_than_the_history_limit_leaves_the_most_recent_lines` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --lib history::` (one shared run with U23 to U28 written together against a fake `seed` that writes `unseeded` at the cursor whatever it is given: `test result: FAILED. 9 passed; 6 failed` (the 9 passing are U14 to U22))
  -> `assertion `left == right` failed: exactly at the limit` / `left: [LogicalLine { text: "unseeded", ... }]` / `right: [LogicalLine { text: "line0", ...`
- green: `seed` returns on `Seed::None`; for `Seed::History` it keeps the last `limit + screen rows` lines, writes each run through `Handler::terminal_attribute` (reset, colours, flags via `FLAG_ATTRIBUTES`) and `input`, then reset, `carriage_return`, `linefeed`; then the `separator_line` for `at` formatted `%Y-%m-%d %H:%M %:z` in the dim style the same way; then `clear_screen(ClearMode::All)` and `goto(0, 0)`. Shared run -> 15 passed, 0 failed, no warnings
- refactor: none needed
- notes: assertion-level red from the shared fake. Decided: "exactly at the limit" means the seeded rows (lines plus separator) fill the history exactly, so `limit - 1` lines all stay; the separator takes one history row.
- commit: `feat(041): seed a terminal with its earlier history and the separator (U23-U28)`

## Cycle 24: U26 seeding at a narrower width wraps, and a later capture gives the same logical lines

- test: `crates/micold-daemon/src/history.rs::tests::seeding_at_a_narrower_width_wraps_and_a_later_capture_gives_the_same_logical_lines` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --lib history::` (one shared run with U23 to U28 written together against a fake `seed` that writes `unseeded` at the cursor whatever it is given: `test result: FAILED. 9 passed; 6 failed` (the 9 passing are U14 to U22))
  -> `assertion `left == right` failed: the lines wrapped` / `left: 0` / `right: 6`
- green: `seed` returns on `Seed::None`; for `Seed::History` it keeps the last `limit + screen rows` lines, writes each run through `Handler::terminal_attribute` (reset, colours, flags via `FLAG_ATTRIBUTES`) and `input`, then reset, `carriage_return`, `linefeed`; then the `separator_line` for `at` formatted `%Y-%m-%d %H:%M %:z` in the dim style the same way; then `clear_screen(ClearMode::All)` and `goto(0, 0)`. Shared run -> 15 passed, 0 failed, no warnings
- refactor: none needed
- notes: assertion-level red from the shared fake; this test was not run against a partial implementation.
- commit: `feat(041): seed a terminal with its earlier history and the separator (U23-U28)`

## Cycle 25: U27 `Seed::None` leaves the `Term` untouched

- test: `crates/micold-daemon/src/history.rs::tests::seed_none_leaves_the_term_untouched` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --lib history::` (one shared run with U23 to U28 written together against a fake `seed` that writes `unseeded` at the cursor whatever it is given: `test result: FAILED. 9 passed; 6 failed` (the 9 passing are U14 to U22))
  -> `assertion `left == right` failed` / `left: HistorySnapshot { lines: [... LogicalLine { text: "lastunseeded", ...` / `right: HistorySnapshot { lines: [... LogicalLine { text: "last", ...`
- green: `seed` returns on `Seed::None`; for `Seed::History` it keeps the last `limit + screen rows` lines, writes each run through `Handler::terminal_attribute` (reset, colours, flags via `FLAG_ATTRIBUTES`) and `input`, then reset, `carriage_return`, `linefeed`; then the `separator_line` for `at` formatted `%Y-%m-%d %H:%M %:z` in the dim style the same way; then `clear_screen(ClearMode::All)` and `goto(0, 0)`. Shared run -> 15 passed, 0 failed, no warnings
- refactor: none needed
- notes: the fake writes whatever it is given, so this test, which a no-op fake would pass, has its own assertion-level red.
- commit: `feat(041): seed a terminal with its earlier history and the separator (U23-U28)`

## Cycle 26: U28 a second seed after more output keeps the first separator

- test: `crates/micold-daemon/src/history.rs::tests::a_second_seed_after_more_output_keeps_the_first_separator` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --lib history::` (one shared run with U23 to U28 written together against a fake `seed` that writes `unseeded` at the cursor whatever it is given: `test result: FAILED. 9 passed; 6 failed` (the 9 passing are U14 to U22))
  -> `assertion `left == right` failed` / `left: [LogicalLine { text: "unseeded", ... }]` / `right: [LogicalLine { text: "earlier", ... }, LogicalLine { text: "── session restarted at 2026-10-02 14:31 +02:00 ──", ...`
- green: `seed` returns on `Seed::None`; for `Seed::History` it keeps the last `limit + screen rows` lines, writes each run through `Handler::terminal_attribute` (reset, colours, flags via `FLAG_ATTRIBUTES`) and `input`, then reset, `carriage_return`, `linefeed`; then the `separator_line` for `at` formatted `%Y-%m-%d %H:%M %:z` in the dim style the same way; then `clear_screen(ClearMode::All)` and `goto(0, 0)`. Shared run -> 15 passed, 0 failed, no warnings
- refactor: none needed
- notes: assertion-level red from the shared fake; this test was not run against a partial implementation.
- commit: `feat(041): seed a terminal with its earlier history and the separator (U23-U28)`

## Cycle 27: A9 Stop then start in one service run shows the 200 lines, one separator, the new output, nothing missing

- test: `crates/micold-daemon/tests/history_restart_in_run.rs::a9_stop_then_start_shows_the_earlier_lines_one_separator_and_the_new_output` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_restart_in_run` (one shared run, all 13 tests written together against stubs: the `Seed` argument added to `spawn_answering`/`spawn_ai_cli` and ignored, no `carried`, no `teardown`: `test result: FAILED. 2 passed; 11 failed`)
  -> `no line reads "line 1" in [ "new output", ]`
- green: `spawn_answering` seeds the new `Term` before the reader thread starts; `PtySession::teardown(&self, wait)` (kill, close the master, wait up to `TEARDOWN_WAIT` = 2 s for `output_ended`, join; once only; `Drop` calls it; `resize` ignored after it); `Inner.carried`, filled by `carry_history` (teardown, then capture) in `stop_session`, the tick's clean exit and give-up, and `respawn_primary` before its spawn, for the primary of an `AiCli` session only; `start_session` and `respawn_primary` seed from it when it has lines and remove it after a successful spawn; `remove_live_by_ids` drops it. Same run -> 13 passed, 0 failed (three more runs of the binary: 13 passed each)
- refactor: none needed
- notes: assertion-level red from the stubs.
- commit: `feat(041): carry a session's history across a stop and start in one run (A9, A10, U30-U37, U133-U135)`

## Cycle 28: A10 A process that exits by itself and is restarted shows its last lines above one separator and the new output below

- test: `crates/micold-daemon/tests/history_restart_in_run.rs::a10_a_process_that_exits_by_itself_is_restarted_below_its_last_lines` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_restart_in_run` (one shared run, all 13 tests written together against stubs: the `Seed` argument added to `spawn_answering`/`spawn_ai_cli` and ignored, no `carried`, no `teardown`: `test result: FAILED. 2 passed; 11 failed`)
  -> `no line reads "before exit A" in [ "after restart", ]`
- green: `spawn_answering` seeds the new `Term` before the reader thread starts; `PtySession::teardown(&self, wait)` (kill, close the master, wait up to `TEARDOWN_WAIT` = 2 s for `output_ended`, join; once only; `Drop` calls it; `resize` ignored after it); `Inner.carried`, filled by `carry_history` (teardown, then capture) in `stop_session`, the tick's clean exit and give-up, and `respawn_primary` before its spawn, for the primary of an `AiCli` session only; `start_session` and `respawn_primary` seed from it when it has lines and remove it after a successful spawn; `remove_live_by_ids` drops it. Same run -> 13 passed, 0 failed (three more runs of the binary: 13 passed each)
- refactor: none needed
- notes: assertion-level red from the stubs.
- commit: `feat(041): carry a session's history across a stop and start in one run (A9, A10, U30-U37, U133-U135)`

## Cycle 29: U30 A session with no output shows no separator after stop and start

- test: `crates/micold-daemon/tests/history_restart_in_run.rs::u30_a_session_with_no_output_shows_no_separator_after_stop_and_start` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_restart_in_run` (one shared run, all 13 tests written together against stubs: the `Seed` argument added to `spawn_answering`/`spawn_ai_cli` and ignored, no `carried`, no `teardown`: `test result: FAILED. 2 passed; 11 failed`)
  -> passed at once; see notes
- green: `spawn_answering` seeds the new `Term` before the reader thread starts; `PtySession::teardown(&self, wait)` (kill, close the master, wait up to `TEARDOWN_WAIT` = 2 s for `output_ended`, join; once only; `Drop` calls it; `resize` ignored after it); `Inner.carried`, filled by `carry_history` (teardown, then capture) in `stop_session`, the tick's clean exit and give-up, and `respawn_primary` before its spawn, for the primary of an `AiCli` session only; `start_session` and `respawn_primary` seed from it when it has lines and remove it after a successful spawn; `remove_live_by_ids` drops it. Same run -> 13 passed, 0 failed (three more runs of the binary: 13 passed each)
- refactor: none needed
- notes: passed at once against the stubs (nothing is seeded). Shown able to fail by a temporary mutation, gated by an environment variable in one build so the four mutations ran as separate runs of the same test binary and none could cause another's failure; removed before the commit: `carried_seed` also seeds an empty snapshot (`MICOLD_MUT_U30`) -> `nothing to restore, so no separator: [ "── session restarted at 2026-10-03 09:35 +02:00 ──", ]`
- commit: `feat(041): carry a session's history across a stop and start in one run (A9, A10, U30-U37, U133-U135)`

## Cycle 30: U31 Two stops and starts show two separators in order

- test: `crates/micold-daemon/tests/history_restart_in_run.rs::u31_two_stops_and_starts_show_two_separators_in_order` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_restart_in_run` (one shared run, all 13 tests written together against stubs: the `Seed` argument added to `spawn_answering`/`spawn_ai_cli` and ignored, no `carried`, no `teardown`: `test result: FAILED. 2 passed; 11 failed`)
  -> `assertion left == right failed: [ "three", ]` / `left: 0` / `right: 2`
- green: `spawn_answering` seeds the new `Term` before the reader thread starts; `PtySession::teardown(&self, wait)` (kill, close the master, wait up to `TEARDOWN_WAIT` = 2 s for `output_ended`, join; once only; `Drop` calls it; `resize` ignored after it); `Inner.carried`, filled by `carry_history` (teardown, then capture) in `stop_session`, the tick's clean exit and give-up, and `respawn_primary` before its spawn, for the primary of an `AiCli` session only; `start_session` and `respawn_primary` seed from it when it has lines and remove it after a successful spawn; `remove_live_by_ids` drops it. Same run -> 13 passed, 0 failed (three more runs of the binary: 13 passed each)
- refactor: none needed
- notes: assertion-level red from the stubs.
- commit: `feat(041): carry a session's history across a stop and start in one run (A9, A10, U30-U37, U133-U135)`

## Cycle 31: U32 Two sessions each show only their own lines after stop and start

- test: `crates/micold-daemon/tests/history_restart_in_run.rs::u32_two_sessions_each_show_only_their_own_lines_after_stop_and_start` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_restart_in_run` (one shared run, all 13 tests written together against stubs: the `Seed` argument added to `spawn_answering`/`spawn_ai_cli` and ignored, no `carried`, no `teardown`: `test result: FAILED. 2 passed; 11 failed`)
  -> `assertion left == right failed` / `left: 0` / `right: 1` (no separator)
- green: `spawn_answering` seeds the new `Term` before the reader thread starts; `PtySession::teardown(&self, wait)` (kill, close the master, wait up to `TEARDOWN_WAIT` = 2 s for `output_ended`, join; once only; `Drop` calls it; `resize` ignored after it); `Inner.carried`, filled by `carry_history` (teardown, then capture) in `stop_session`, the tick's clean exit and give-up, and `respawn_primary` before its spawn, for the primary of an `AiCli` session only; `start_session` and `respawn_primary` seed from it when it has lines and remove it after a successful spawn; `remove_live_by_ids` drops it. Same run -> 13 passed, 0 failed (three more runs of the binary: 13 passed each)
- refactor: none needed
- notes: assertion-level red from the stubs.
- commit: `feat(041): carry a session's history across a stop and start in one run (A9, A10, U30-U37, U133-U135)`

## Cycle 32: U33 The fake CLI's recorded stdin is empty after a start

- test: `crates/micold-daemon/tests/history_restart_in_run.rs::u33_the_cli_receives_nothing_on_stdin_at_a_start` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_restart_in_run` (one shared run, all 13 tests written together against stubs: the `Seed` argument added to `spawn_answering`/`spawn_ai_cli` and ignored, no `carried`, no `teardown`: `test result: FAILED. 2 passed; 11 failed`)
  -> passed at once; see notes
- green: `spawn_answering` seeds the new `Term` before the reader thread starts; `PtySession::teardown(&self, wait)` (kill, close the master, wait up to `TEARDOWN_WAIT` = 2 s for `output_ended`, join; once only; `Drop` calls it; `resize` ignored after it); `Inner.carried`, filled by `carry_history` (teardown, then capture) in `stop_session`, the tick's clean exit and give-up, and `respawn_primary` before its spawn, for the primary of an `AiCli` session only; `start_session` and `respawn_primary` seed from it when it has lines and remove it after a successful spawn; `remove_live_by_ids` drops it. Same run -> 13 passed, 0 failed (three more runs of the binary: 13 passed each)
- refactor: none needed
- notes: passed at once against the stubs. Shown able to fail by a temporary mutation, gated by an environment variable in one build so the four mutations ran as separate runs of the same test binary and none could cause another's failure; removed before the commit: `spawn_answering` also writes a separator line to the PTY writer when it has a history seed (`MICOLD_MUT_U33`) -> `only what was typed reached the process` / `left: "── session restarted at mutation ──\ndone\n"` / `right: "done\n"`
- commit: `feat(041): carry a session's history across a stop and start in one run (A9, A10, U30-U37, U133-U135)`

## Cycle 33: U34 A Regular Terminal instance stopped and started has an empty history

- test: `crates/micold-daemon/tests/history_restart_in_run.rs::u34_a_regular_terminal_stopped_and_started_has_an_empty_history` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_restart_in_run` (one shared run, all 13 tests written together against stubs: the `Seed` argument added to `spawn_answering`/`spawn_ai_cli` and ignored, no `carried`, no `teardown`: `test result: FAILED. 2 passed; 11 failed`)
  -> passed at once; see notes
- green: `spawn_answering` seeds the new `Term` before the reader thread starts; `PtySession::teardown(&self, wait)` (kill, close the master, wait up to `TEARDOWN_WAIT` = 2 s for `output_ended`, join; once only; `Drop` calls it; `resize` ignored after it); `Inner.carried`, filled by `carry_history` (teardown, then capture) in `stop_session`, the tick's clean exit and give-up, and `respawn_primary` before its spawn, for the primary of an `AiCli` session only; `start_session` and `respawn_primary` seed from it when it has lines and remove it after a successful spawn; `remove_live_by_ids` drops it. Same run -> 13 passed, 0 failed (three more runs of the binary: 13 passed each)
- refactor: none needed
- notes: the stub run failed on a test bug (the shell printed `$ MARK-42`, the wait wanted a line reading `MARK-42`); the wait was fixed, after which it passes at once against the stubs. Shown able to fail by a temporary mutation, gated by an environment variable in one build so the four mutations ran as separate runs of the same test binary and none could cause another's failure; removed before the commit: `covered` returns true for every mode and the Regular arm of `start_session` seeds after the spawn (`MICOLD_MUT_U34`) -> `nothing of the earlier shell: [ "$ echo MARK-$((40+2))", "MARK-42", "$", "── session restarted at …", …`
- commit: `feat(041): carry a session's history across a stop and start in one run (A9, A10, U30-U37, U133-U135)`

## Cycle 34: U35 A fake CLI that prints `ESC[2J ESC[H` at start leaves the seeded lines and separator in the history

- test: `crates/micold-daemon/tests/history_restart_in_run.rs::u35_a_cli_that_erases_the_screen_at_start_leaves_the_restored_lines` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_restart_in_run` (one shared run, all 13 tests written together against stubs: the `Seed` argument added to `spawn_answering`/`spawn_ai_cli` and ignored, no `carried`, no `teardown`: `test result: FAILED. 2 passed; 11 failed`)
  -> `assertion left == right failed` / `left: 0` / `right: 1` (no separator)
- green: `spawn_answering` seeds the new `Term` before the reader thread starts; `PtySession::teardown(&self, wait)` (kill, close the master, wait up to `TEARDOWN_WAIT` = 2 s for `output_ended`, join; once only; `Drop` calls it; `resize` ignored after it); `Inner.carried`, filled by `carry_history` (teardown, then capture) in `stop_session`, the tick's clean exit and give-up, and `respawn_primary` before its spawn, for the primary of an `AiCli` session only; `start_session` and `respawn_primary` seed from it when it has lines and remove it after a successful spawn; `remove_live_by_ids` drops it. Same run -> 13 passed, 0 failed (three more runs of the binary: 13 passed each)
- refactor: none needed
- notes: assertion-level red from the stubs.
- commit: `feat(041): carry a session's history across a stop and start in one run (A9, A10, U30-U37, U133-U135)`

## Cycle 35: U36 A fake CLI that enters and leaves the alternate screen leaves them in the primary grid's history

- test: `crates/micold-daemon/tests/history_restart_in_run.rs::u36_a_cli_that_uses_the_alternate_screen_leaves_the_restored_lines_in_the_primary_history` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_restart_in_run` (one shared run, all 13 tests written together against stubs: the `Seed` argument added to `spawn_answering`/`spawn_ai_cli` and ignored, no `carried`, no `teardown`: `test result: FAILED. 2 passed; 11 failed`)
  -> `assertion left == right failed` / `left: 0` / `right: 1` (no separator)
- green: `spawn_answering` seeds the new `Term` before the reader thread starts; `PtySession::teardown(&self, wait)` (kill, close the master, wait up to `TEARDOWN_WAIT` = 2 s for `output_ended`, join; once only; `Drop` calls it; `resize` ignored after it); `Inner.carried`, filled by `carry_history` (teardown, then capture) in `stop_session`, the tick's clean exit and give-up, and `respawn_primary` before its spawn, for the primary of an `AiCli` session only; `start_session` and `respawn_primary` seed from it when it has lines and remove it after a successful spawn; `remove_live_by_ids` drops it. Same run -> 13 passed, 0 failed (three more runs of the binary: 13 passed each)
- refactor: none needed
- notes: assertion-level red from the stubs.
- commit: `feat(041): carry a session's history across a stop and start in one run (A9, A10, U30-U37, U133-U135)`

## Cycle 36: U37 A second attached client receives the same lines in its first `full` frame

- test: `crates/micold-daemon/tests/history_restart_in_run.rs::u37_a_second_window_gets_the_same_lines_in_its_first_full_frame` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_restart_in_run` (one shared run, all 13 tests written together against stubs: the `Seed` argument added to `spawn_answering`/`spawn_ai_cli` and ignored, no `carried`, no `teardown`: `test result: FAILED. 2 passed; 11 failed`)
  -> `there is history above the screen` (the two first full frames agreed, but held no history)
- green: `spawn_answering` seeds the new `Term` before the reader thread starts; `PtySession::teardown(&self, wait)` (kill, close the master, wait up to `TEARDOWN_WAIT` = 2 s for `output_ended`, join; once only; `Drop` calls it; `resize` ignored after it); `Inner.carried`, filled by `carry_history` (teardown, then capture) in `stop_session`, the tick's clean exit and give-up, and `respawn_primary` before its spawn, for the primary of an `AiCli` session only; `start_session` and `respawn_primary` seed from it when it has lines and remove it after a successful spawn; `remove_live_by_ids` drops it. Same run -> 13 passed, 0 failed (three more runs of the binary: 13 passed each)
- refactor: none needed
- notes: assertion-level red from the stubs.
- commit: `feat(041): carry a session's history across a stop and start in one run (A9, A10, U30-U37, U133-U135)`

## Cycle 37: U133 With a client attached and streaming, a stop keeps the last line the process printed

- test: `crates/micold-daemon/tests/history_restart_in_run.rs::u133_with_a_window_streaming_a_stop_keeps_the_last_line` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_restart_in_run` (one shared run, all 13 tests written together against stubs: the `Seed` argument added to `spawn_answering`/`spawn_ai_cli` and ignored, no `carried`, no `teardown`: `test result: FAILED. 2 passed; 11 failed`)
  -> `one separator: []` / `left: 0` / `right: 1`
- green: `spawn_answering` seeds the new `Term` before the reader thread starts; `PtySession::teardown(&self, wait)` (kill, close the master, wait up to `TEARDOWN_WAIT` = 2 s for `output_ended`, join; once only; `Drop` calls it; `resize` ignored after it); `Inner.carried`, filled by `carry_history` (teardown, then capture) in `stop_session`, the tick's clean exit and give-up, and `respawn_primary` before its spawn, for the primary of an `AiCli` session only; `start_session` and `respawn_primary` seed from it when it has lines and remove it after a successful spawn; `remove_live_by_ids` drops it. Same run -> 13 passed, 0 failed (three more runs of the binary: 13 passed each)
- refactor: none needed
- notes: also shown to depend on the teardown by a temporary mutation, gated by an environment variable in one build so the four mutations ran as separate runs of the same test binary and none could cause another's failure; removed before the commit: `carry_history` kills without `teardown` (`MICOLD_MUT_NOTEARDOWN`) -> `the last line the process printed is above the separator: [ "burst 2477", …` (3 of 3 runs red, the capture ending 500 to 1000 lines short)
- commit: `feat(041): carry a session's history across a stop and start in one run (A9, A10, U30-U37, U133-U135)`

## Cycle 38: U134 With a client attached and streaming, a self-exit and restart keeps the last line

- test: `crates/micold-daemon/tests/history_restart_in_run.rs::u134_with_a_window_streaming_a_self_exit_and_restart_keeps_the_last_line` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_restart_in_run` (one shared run, all 13 tests written together against stubs: the `Seed` argument added to `spawn_answering`/`spawn_ai_cli` and ignored, no `carried`, no `teardown`: `test result: FAILED. 2 passed; 11 failed`)
  -> `one separator: []` / `left: 0` / `right: 1`
- green: `spawn_answering` seeds the new `Term` before the reader thread starts; `PtySession::teardown(&self, wait)` (kill, close the master, wait up to `TEARDOWN_WAIT` = 2 s for `output_ended`, join; once only; `Drop` calls it; `resize` ignored after it); `Inner.carried`, filled by `carry_history` (teardown, then capture) in `stop_session`, the tick's clean exit and give-up, and `respawn_primary` before its spawn, for the primary of an `AiCli` session only; `start_session` and `respawn_primary` seed from it when it has lines and remove it after a successful spawn; `remove_live_by_ids` drops it. Same run -> 13 passed, 0 failed (three more runs of the binary: 13 passed each)
- refactor: none needed
- notes: also shown to depend on the teardown by the same `MICOLD_MUT_NOTEARDOWN` mutation -> `the last line the process printed is above the separator: [ "burst 2233", …` (3 of 3 runs red)
- commit: `feat(041): carry a session's history across a stop and start in one run (A9, A10, U30-U37, U133-U135)`

## Cycle 39: U135 A fake CLI that leaves a detached grandchild holding the terminal open is stopped with a reply within 3 s and its parsed output is carried (`cfg(unix)`)

- test: `crates/micold-daemon/tests/history_restart_in_run.rs::u135_a_detached_grandchild_does_not_hold_the_stop_and_the_output_is_carried` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_restart_in_run` (one shared run, all 13 tests written together against stubs: the `Seed` argument added to `spawn_answering`/`spawn_ai_cli` and ignored, no `carried`, no `teardown`: `test result: FAILED. 2 passed; 11 failed`)
  -> `the stop replied within 3 s (took 3.000150378s)` / `left: Err(Timeout)` / `right: Ok(true)` (the old `Drop` joined the reader, which the grandchild kept open)
- green: `spawn_answering` seeds the new `Term` before the reader thread starts; `PtySession::teardown(&self, wait)` (kill, close the master, wait up to `TEARDOWN_WAIT` = 2 s for `output_ended`, join; once only; `Drop` calls it; `resize` ignored after it); `Inner.carried`, filled by `carry_history` (teardown, then capture) in `stop_session`, the tick's clean exit and give-up, and `respawn_primary` before its spawn, for the primary of an `AiCli` session only; `start_session` and `respawn_primary` seed from it when it has lines and remove it after a successful spawn; `remove_live_by_ids` drops it. Same run -> 13 passed, 0 failed (three more runs of the binary: 13 passed each)
- refactor: none needed
- notes: assertion-level red from the stubs.
- commit: `feat(041): carry a session's history across a stop and start in one run (A9, A10, U30-U37, U133-U135)`

## Cycle 40: U38 the stop-start and self-exit-restart cases pass on a real pseudoconsole under `cfg(windows)`

- test: `crates/micold-daemon/tests/history_restart_in_run.rs`: A9, A10, U133 and U134 carry no `cfg` gate, so they run on the `windows-latest` CI leg against ConPTY with the compiled stand-in (new)
- red: not observable on this host (profile, *Additions from feature 030*). The Linux red of the same four tests is in cycles 27, 28, 37 and 38; the Windows red and green are to be recorded from the CI run of the pushed branch (run URL and log line).
- green: pending the CI Windows leg
- refactor: none needed
- notes: row stays PENDING until the CI evidence is recorded here, so T007 stays unticked.
- commit: `feat(041): carry a session's history across a stop and start in one run (A9, A10, U30-U37, U133-U135)`

## Review A fix (M1): a start during the stop's teardown waits for the history

- test: `crates/micold-daemon/tests/history_restart_in_run.rs::a_start_during_the_stops_teardown_still_shows_the_earlier_lines` (new regression test for review A's finding)
- red: with the fix in place but the start's wait mutated to `Duration::ZERO` (the pre-fix behaviour),
  `scripts/build-lock.sh cargo test -p micold-daemon --test history_restart_in_run a_start_during_the_stops_teardown_still_shows_the_earlier_lines -- --exact`
  -> ``assertion `left == right` failed: one separator`` / `left: 0` / `right: 1` (1 failed)
- green: `Inner.carrying` is marked under the lock that takes the process out (stop, tick, respawn)
  and cleared by `carry_history` (which keeps the capture only while the mark stands, so a removal
  meanwhile drops it) and by `remove_live_by_ids`; `carried_seed` waits on a `Condvar` while the mark
  stands (bounded at `2 × TEARDOWN_WAIT`); `carried` holds `Arc`s, cloned under the lock and copied
  off it, and a start or a respawn's swap takes its entry only on success and only if it is the one it
  seeded from; `ClientMsg::SessionStop` runs `stop_session` on `spawn_blocking`. Green in `mise run gate`.
- refactor: none
- review A round 2 follow-up (same behaviour, refactor of the fix): one carrier per ending process
  (`Carry::Own(token)` / `Carry::Join`), the entry taken out and the carry marked under one lock in
  stop, the supervision tick and the respawn; `stop_session` split into `begin_stop` (on the window's
  loop) and `finish_stop` (`spawn_blocking`); a timed-out wait cancels the mark; a drop guard clears it
  on any exit. The regression test above stays the pin; the respawn-and-stop overlap and the timeout
  have no deterministic test (timing-dependent) and are covered by the token check.

## Review A round 3 fix (M1): a session's stop, drop, respawn and start run under its gate

- test: `crates/micold-daemon/tests/attention_claims.rs::the_grants_of_a_session_dropped_by_supervision_are_forgotten` (new; feature 039's `forget_session` on the tick's drop path, lost in a rebase)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test attention_claims -- the_grants_of`
  -> ``assertion `left == right` failed: what was granted for the dropped session was forgotten`` / `left: []` / `right: [(SessionId(…0a), 1)]` (1 failed)
- test: `crates/micold-daemon/tests/history_restart_in_run.rs::a_tick_leaves_a_session_whose_gate_is_held_for_the_next_tick` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_restart_in_run -- a_tick_leaves`
  -> `no lifecycle moved while the gate was held` (1 failed: the tick respawned although a stop or start held the gate)
- test: `…::a_respawn_does_not_replace_the_process_of_a_start_that_came_meanwhile` (new; the finding's own scenario). No red: on the old code the outcome depended on timing and this run passed. It pins the order under the gate.
- green: the token and condvar scheme is removed (`carrying`, `next_carry`, `carry_done`, `Carry`, `claim_carry*`, `wait_carry`, the drop guard, `PendingStop`, `begin_stop`, `finish_stop`). `start_session` and `stop_session` wait for the session's gate and call their `_gated` forms; `ops::start_session` and the new `ops::stop_session` queue for the gate on the caller, await it on a task and run the `_gated` form on a blocking thread; a window's `SessionStop` goes through `ops::stop_session`. The tick tries each dead session's gate (`try_lock`, off the state lock), applies the policy only to those it holds, and holds each until its drop or respawn is done; a busy session waits for the next tick. A respawn does nothing unless the dead process it was planned for is still the primary, and swaps only over that process. The tick's drop forgets the session's grants again. Every process is killed before the primary's teardown and capture.
  `scripts/build-lock.sh cargo test -p micold-daemon` -> 702 passed, 0 failed.
- refactor: none
- removed tests: none (no test pinned the removed scheme; `a_start_during_the_stops_teardown_still_shows_the_earlier_lines` stays and passes because the start waits for the gate).

## Cycle 41: U39 Encode then decode returns the same snapshot

- test: `crates/micold-core/tests/terminal_history_format.rs::encode_then_decode_gives_the_same_snapshot` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_format` (one shared run, all 19 tests written together against a stub: `encode` returns no bytes, `decode` returns `Err(Unreadable(Other))`, `Display` writes nothing: `test result: FAILED. 1 passed; 18 failed`; commit `0821f1b2`)
  -> ``assertion `left == right` failed: empty`` / `left: Err(Unreadable(Other))` / `right: Ok(HistorySnapshot { lines: [] })`
- green: `terminal_history/format.rs`: private `SavedHistory`/`SavedLine`/`SavedRun` (styles deduplicated in order of first use), `encode` (magic, version, length, `postcard` payload, `protocol::hashing::sha256` of all before it), `decode` with checks 2 to 10 of HF §4 in order (check 1 is the reader's), `DamageReason` and its `Display`; `HistoryStyle`, `HistoryColor` and `StyleFlags` derive `Serialize`, `Deserialize` and `Hash`. Same command -> `test result: ok. 19 passed; 0 failed`; `mise run test-core` green (150 test binaries ok)
- refactor: none needed
- notes: assertion-level red from the stub.
- commit: `feat(041): encode and decode the saved-history file, with the ten checks of a read (U39-U43)` (`82c1bef7`)

## Cycle 42: U40 The encoded header bytes are those of HF §2

- test: `crates/micold-core/tests/terminal_history_format.rs::the_encoded_bytes_are_the_header_the_payload_and_the_checksum_of_both`, `crates/micold-core/tests/terminal_history_format.rs::a_style_used_twice_is_stored_once` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_format` (one shared run, all 19 tests written together against a stub: `encode` returns no bytes, `decode` returns `Err(Unreadable(Other))`, `Display` writes nothing: `test result: FAILED. 1 passed; 18 failed`; commit `0821f1b2`)
  -> ``assertion `left == right` failed`` / `left: 0` / `right: 63` (file length); `left: []` / `right: [77, 73, 67, 79, 76, 68, 84, 72, 1, 0, 0, 0, 16, …]`
- green: `terminal_history/format.rs`: private `SavedHistory`/`SavedLine`/`SavedRun` (styles deduplicated in order of first use), `encode` (magic, version, length, `postcard` payload, `protocol::hashing::sha256` of all before it), `decode` with checks 2 to 10 of HF §4 in order (check 1 is the reader's), `DamageReason` and its `Display`; `HistoryStyle`, `HistoryColor` and `StyleFlags` derive `Serialize`, `Deserialize` and `Hash`. Same command -> `test result: ok. 19 passed; 0 failed`; `mise run test-core` green (150 test binaries ok)
- refactor: none needed
- notes: assertion-level red from the stub. The tests also pin the payload bytes of two small snapshots, written by hand, so the helper that builds damaged files is tied to the encoder.
- commit: `feat(041): encode and decode the saved-history file, with the ten checks of a read (U39-U43)` (`82c1bef7`)

## Cycle 43: U41 Each row of HF §4's table gives its `DamageReason`

- test: `a_file_over_the_size_cap_is_too_large`, `another_magic_is_not_a_history`, `fewer_than_52_bytes_is_not_a_history`, `version_2_is_another_version`, `a_file_cut_short_or_grown_is_truncated`, `one_flipped_payload_bit_fails_the_checksum`, `bytes_after_the_payload_or_a_payload_cut_short_are_malformed`, `a_style_index_outside_the_styles_is_a_bad_style_index`, `runs_that_do_not_sum_to_the_text_are_a_bad_run_length`, `a_text_with_esc_is_a_control_character`, `the_first_failing_check_names_the_damage`, `each_reason_has_its_own_text_for_the_log` in `crates/micold-core/tests/terminal_history_format.rs` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_format` (one shared run, all 19 tests written together against a stub: `encode` returns no bytes, `decode` returns `Err(Unreadable(Other))`, `Display` writes nothing: `test result: FAILED. 1 passed; 18 failed`; commit `0821f1b2`)
  -> `left: Err(Unreadable(Other))` / `right: Err(TooLarge)`, and the same with `Malformed`, `BadStyleIndex`, `BadRunLength`, `ControlCharacter`; `left: ""` / `right: "written by another version"`. The magic, version, truncation and flipped-bit tests failed on indexing the stub's empty output (`index out of bounds`), not on their assertion.
- green: `terminal_history/format.rs`: private `SavedHistory`/`SavedLine`/`SavedRun` (styles deduplicated in order of first use), `encode` (magic, version, length, `postcard` payload, `protocol::hashing::sha256` of all before it), `decode` with checks 2 to 10 of HF §4 in order (check 1 is the reader's), `DamageReason` and its `Display`; `HistoryStyle`, `HistoryColor` and `StyleFlags` derive `Serialize`, `Deserialize` and `Hash`. Same command -> `test result: ok. 19 passed; 0 failed`; `mise run test-core` green (150 test binaries ok)
- refactor: none needed
- notes: one test corrected between red and green, with the reason: `fewer_than_52_bytes_is_not_a_history` took `encode` of an empty snapshot to be 52 bytes; it is 54 (the payload of two empty lists is 2 bytes), seen as `left: 54` / `right: 52` at the first green run. It now cuts a hand-built file with no payload to 51 bytes (`NotAHistory`) and also asserts that the 52-byte one passes the size check (`Malformed`). The assertion on the reason was not weakened.
- commit: `feat(041): encode and decode the saved-history file, with the ten checks of a read (U39-U43)` (`82c1bef7`)

## Cycle 44: U42 1,000 random byte strings and every prefix of a valid file decode to `Damaged` without a panic

- test: `crates/micold-core/tests/terminal_history_format.rs::random_byte_strings_are_damaged_without_a_panic`, `crates/micold-core/tests/terminal_history_format.rs::every_prefix_of_a_valid_file_is_damaged`, `crates/micold-core/tests/terminal_history_format.rs::random_payloads_with_a_matching_checksum_do_not_panic` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_format` (one shared run, all 19 tests written together against a stub: `encode` returns no bytes, `decode` returns `Err(Unreadable(Other))`, `Display` writes nothing: `test result: FAILED. 1 passed; 18 failed`; commit `0821f1b2`)
  -> `the whole file is valid` (prefixes); `random payloads reached the payload check`; `random_byte_strings_are_damaged_without_a_panic` passed at once; see notes
- green: `terminal_history/format.rs`: private `SavedHistory`/`SavedLine`/`SavedRun` (styles deduplicated in order of first use), `encode` (magic, version, length, `postcard` payload, `protocol::hashing::sha256` of all before it), `decode` with checks 2 to 10 of HF §4 in order (check 1 is the reader's), `DamageReason` and its `Display`; `HistoryStyle`, `HistoryColor` and `StyleFlags` derive `Serialize`, `Deserialize` and `Hash`. Same command -> `test result: ok. 19 passed; 0 failed`; `mise run test-core` green (150 test binaries ok)
- refactor: none needed
- notes: the random-strings test passed at once against the stub, which calls everything damaged. Shown able to fail by a temporary mutation after green, restored with `git checkout`: the minimum-size check of `decode` removed -> `random_byte_strings_are_damaged_without_a_panic` and `every_prefix_of_a_valid_file_is_damaged` FAILED with `attempt to subtract with overflow` at `format.rs:184` (3 failed, 16 passed). The random inputs come from a xorshift generator in the test, so no dependency was added.
- commit: `feat(041): encode and decode the saved-history file, with the ten checks of a read (U39-U43)` (`82c1bef7`)

## Cycle 45: U43 The bytes of `fixtures/terminal_history/v1.history` equal the encoding of a fixed snapshot built in the test

- test: `crates/micold-core/tests/terminal_history_format.rs::the_v1_fixture_is_the_encoding_of_its_snapshot` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_format` (one shared run, all 19 tests written together against a stub: `encode` returns no bytes, `decode` returns `Err(Unreadable(Other))`, `Display` writes nothing: `test result: FAILED. 1 passed; 18 failed`; commit `0821f1b2`)
  -> `…/tests/fixtures/terminal_history/v1.history: No such file or directory (os error 2)` (the fixture did not exist yet, so this red is not an assertion on bytes)
- green: `terminal_history/format.rs`: private `SavedHistory`/`SavedLine`/`SavedRun` (styles deduplicated in order of first use), `encode` (magic, version, length, `postcard` payload, `protocol::hashing::sha256` of all before it), `decode` with checks 2 to 10 of HF §4 in order (check 1 is the reader's), `DamageReason` and its `Display`; `HistoryStyle`, `HistoryColor` and `StyleFlags` derive `Serialize`, `Deserialize` and `Hash`. Same command -> `test result: ok. 19 passed; 0 failed`; `mise run test-core` green (150 test binaries ok)
- refactor: none needed
- notes: the fixture (307 bytes) was written once from `encode` by a temporary test that was removed before the commit; the committed test only reads it. Shown able to fail by a temporary mutation after green, restored with `git checkout`: the two fields of `SavedRun` swapped -> `the_v1_fixture_is_the_encoding_of_its_snapshot` FAILED at its byte comparison (6 failed, 13 passed).
- commit: `feat(041): encode and decode the saved-history file, with the ten checks of a read (U39-U43)` (`82c1bef7`)

## T007 closed: U38 on Windows (M2 unit)

- U38 (`crates/micold-daemon/tests/history_restart_in_run.rs`) ran on the Windows leg of PR #577: CI run 37214267941, job 111471529932 `build + test (windows-latest)`, `Running tests\history_restart_in_run.rs` -> `test result: ok. 5 passed; 0 failed; 0 ignored`. T007 ticked.

## Cycle 46: U55 `write` creates the directory `0700` and the file `0600`, and replaces an existing file through a rename

- test: `crates/micold-core/tests/owner_only.rs::unix::write_creates_the_directory_0700_and_the_file_0600`, `…::unix::write_replaces_an_existing_file_through_a_rename`, `…::unix::write_narrows_an_existing_wider_file_and_directory`, `…::write_with_stores_what_the_fill_wrote_and_returns_the_path` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test owner_only` (one shared run, all 10 Unix-side tests written together against a stub: `ensure_dir` and `write_with` return `Err(Unsupported)`: `test result: FAILED. 1 passed; 9 failed`; commit `b8ec8bb7`)
  -> ``called `Result::unwrap()` on an `Err` value: Kind(Unsupported)`` at the first `write` of each test
- green: `crates/micold-core/src/owner_only.rs`: `write(dir, file, bytes)` is `write_with(dir, file, fill)` with `write_all`; `write_with` held the body of the daemon's Unix half as it was (directory `0700`, temporary file `.<file>.tmp` created `0600` with `create_new`, `fill`, rename). Same command -> these four and U58's test pass, `test result: FAILED. 5 passed; 5 failed` (U56 and U59 still red)
- refactor: none
- notes: the red is the stub's error, not an assertion on a mode. The rename is asserted through the inode of the name and through a handle that was open on the old file and still reads the whole old content.
- commit: `feat(041): owner_only::write and ensure_dir in micold-core, on both platforms (U55-U59)` (`4a318367`)

## Cycle 47: U56 `ensure_dir` creates a missing directory `0700` and tightens an existing looser one

- test: `crates/micold-core/tests/owner_only.rs::unix::ensure_dir_creates_a_missing_directory_0700`, `…::unix::ensure_dir_tightens_an_existing_looser_directory` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test owner_only` (one shared run, all 10 Unix-side tests written together against a stub: `ensure_dir` and `write_with` return `Err(Unsupported)`: `test result: FAILED. 1 passed; 9 failed`; commit `b8ec8bb7`)
  -> ``called `Result::unwrap()` on an `Err` value: Kind(Unsupported)`` at `ensure_dir`; still so after cycle 46's green
- green: `ensure_dir` creates the directory and its missing parents `0700` and takes the group and other bits from an existing one (`mode & 0o700`, only when any of them is set); `write_with` calls it. The Windows arm (`create_dir_all`, protected DACL with one inheritable ACE) and the per-platform `create_owner_only` were written in the same step. Same command -> `test result: FAILED. 8 passed; 2 failed` (the two cleanup tests of U59 still red)
- refactor: none
- notes: `ensure_dir` leaves the owner's own bits alone. The daemon's helper set `0700` always, which made a directory the user had set `0500` writable again; with that body U59's read-only test failed (cycle 49).
- commit: `feat(041): owner_only::write and ensure_dir in micold-core, on both platforms (U55-U59)` (`4a318367`)

## Cycle 48: U57 On Windows the directory and file carry a protected DACL with exactly one entry, for the current user

- test: `crates/micold-core/tests/owner_only.rs::windows::the_written_file_and_its_directory_have_a_protected_dacl_for_the_current_user_only`, `…::windows::rewriting_replaces_the_bytes_and_keeps_the_owner_only_dacl`, `…::windows::ensure_dir_gives_a_missing_and_an_existing_directory_the_owner_only_dacl` (new, `cfg(windows)`)
- red: not observed. No Windows host here; the tests are compiled only (`scripts/build-lock.sh cargo check --workspace --all-targets --target x86_64-pc-windows-msvc` -> `Finished`). Test-after as far as this log can show; the Windows leg of CI is the first run.
- green: the Windows arm of `owner_only.rs`: `set_protected_dacl` moved unchanged from `crates/micold-daemon/src/platform/windows.rs`; the directory gets `D:P(A;OICI;GA;;;<sid>)`, the file `D:P(A;;GA;;;<sid>)`, the SID from `crate::endpoint::user_sid`.
- refactor: none
- notes: the DACL reader is the one of `crates/micold-daemon/tests/mcp_binding_file_mode.rs`, copied. Unlike that test, the directory's DACL is asserted too, and `ensure_dir` on a directory that inherited its entries.
- commit: `feat(041): owner_only::write and ensure_dir in micold-core, on both platforms (U55-U59)` (`4a318367`)

## Cycle 49: U58 The temporary file is owner-only before any content is written

- test: `crates/micold-core/tests/owner_only.rs::unix::the_temporary_file_is_0600_before_any_content_is_written`, `…::windows::the_temporary_file_has_the_owner_only_dacl_before_any_content_is_written` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test owner_only` (one shared run, all 10 Unix-side tests written together against a stub: `ensure_dir` and `write_with` return `Err(Unsupported)`: `test result: FAILED. 1 passed; 9 failed`; commit `b8ec8bb7`)
  -> ``called `Result::unwrap()` on an `Err` value: Kind(Unsupported)`` (the Unix test; the Windows one was not run, see cycle 48)
- green: with cycle 46's green on Unix (the file is created `0600` and `fill` runs after it). On Windows the order changed against the daemon's half, which wrote the bytes and then set the DACL: `create_owner_only` creates the empty file and sets its protected DACL, then `fill` writes.
- refactor: none
- notes: the test looks from inside `fill`: one entry in the directory, not the final name, mode `0600` (Windows: the owner-only DACL), length 0. `write_with` exists for this and for U59's failing fill.
- commit: `feat(041): owner_only::write and ensure_dir in micold-core, on both platforms (U55-U59)` (`4a318367`)

## Cycle 50: U59 `write` into a read-only directory returns the error and leaves no temporary file

- test: `crates/micold-core/tests/owner_only.rs::unix::write_into_a_read_only_directory_returns_the_error_and_leaves_no_temporary_file`, `…::a_fill_that_fails_returns_its_error_and_leaves_no_temporary_file`, `…::a_rename_that_fails_returns_the_error_and_leaves_no_temporary_file` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test owner_only` (one shared run, all 10 Unix-side tests written together against a stub: `ensure_dir` and `write_with` return `Err(Unsupported)`: `test result: FAILED. 1 passed; 9 failed`; commit `b8ec8bb7`)
  -> read-only directory: `left: Unsupported` / `right: PermissionDenied`; failing fill: ``called `Result::unwrap()` on an `Err` value: Kind(Unsupported)``; `a_rename_that_fails…` passed against the stub. Then, with cycle 46's body (directory set `0700` always, no cleanup), all three failed on their assertions: read-only directory at `unwrap_err` on an `Ok` (the write went through); the other two `left: [".../.a.history.tmp", ".../a.history"]` / `right: [".../a.history"]`
- green: cycle 47's `ensure_dir` made the read-only test pass (`PermissionDenied`, nothing in the directory, mode still `0500`); `write_with` removes the temporary file when the creation, the fill or the rename fails and returns that error. Same command -> `test result: ok. 10 passed; 0 failed`
- refactor: `refactor(041): sync the file and its directory in owner_only::write; the daemon's write_owner_only delegates to it` (`3f90f223`): `sync_all` on the temporary file before the rename and, on Unix, of the directory after it (HF §3, R5); `platform::write_owner_only` is one call to `micold_core::owner_only::write` and the halves in `platform/unix.rs` and `platform/windows.rs` are removed. `owner_only` -> `test result: ok. 10 passed; 0 failed`; `scripts/build-lock.sh cargo test -p micold-daemon --test mcp_binding_file_mode` (unchanged) -> `test result: ok. 4 passed; 0 failed`; `mise run test-core` green (151 test binaries ok); clippy `-D warnings` on both crates clean; `cargo check --workspace --all-targets --target x86_64-pc-windows-msvc` and `cargo check --workspace --target aarch64-apple-darwin` -> `Finished`
- notes: the read-only test returns early when run as root. No test pins the two syncs: nothing a test can see here tells a synced file from an unsynced one. A failed directory sync is ignored (the whole file is under its name by then); a failed file sync fails the write.
- commit: `feat(041): owner_only::write and ensure_dir in micold-core, on both platforms (U55-U59)` (`4a318367`)

## Cycle 51: (not on the list) a file that passes the ten checks and holds a colour index outside its palette is `Malformed`

- test: `crates/micold-core/tests/terminal_history_format.rs::a_colour_index_outside_its_palette_is_malformed` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_format` -> `test result: FAILED. 19 passed; 1 failed` (commit `cb956e7d`)
  -> `left: Ok(HistorySnapshot { lines: [LogicalLine { text: "ab", runs: [StyleRun { chars: 2, style: HistoryStyle { fg: Basic(16), bg: Default, flags: StyleFlags(0) } }] }] })` / `right: Err(Malformed)`
- green: `crates/micold-core/src/terminal_history/format.rs`: after `check()` and `into_snapshot()`, `decode` runs `HistorySnapshot::validate` and maps any failure to `DamageReason::Malformed`. Same command -> `test result: ok. 20 passed; 0 failed`
- refactor: none
- notes: decided by the unit, not a row of the test list: FR-016 skips a history that cannot be used for any reason, and `decode` returned a snapshot `validate` rejects from a file with a valid checksum. `validate` also repeats checks 9 and 10; those run first in `check()` and keep their own reasons, so the only new rejection is `Basic(16..)` and `Dim(8..)` as a foreground or a background of a style a run uses. A live capture cannot produce one: `micold-daemon/src/history.rs::history_color` takes the index from a position in its 16- and 8-entry tables. A style in the file's table that no run uses is not looked at. HF §4's table and DM §3 still describe `Malformed` as the postcard check only; they are not edited here.
- commit: `feat(041): decode returns only a snapshot that validates; anything else is Malformed (FR-016)` (`4c2dddd6`)

## Cycle 52: U44 `save` then `load` gives `History`, in the file `<dir>/<session uuid>.history`

- test: `crates/micold-core/tests/terminal_history_store.rs::a_saved_snapshot_loads_from_the_file_named_after_the_session`, `…::another_store_on_the_same_directory_loads_what_the_first_saved`, `…::an_empty_snapshot_loads_as_an_empty_history` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_store` (one shared run, all 18 tests written together against a stub: `save` returns `Err(Unsupported)`, `load` returns `None`, `history_dir()` returns `None`: `test result: FAILED. 0 passed; 18 failed`; commit `f02def10`)
  -> ``called `Result::unwrap()` on an `Err` value: Kind(Unsupported)`` at the first `save`
- green: `crates/micold-core/src/terminal_history/store.rs`: `save` encodes, takes the mutex and calls `owner_only::write(dir, "<uuid>.history", bytes)`; `load` reads the file under the mutex (`read_capped`) and decodes outside it. Same command -> `test result: FAILED. 14 passed; 4 failed` (U51's two, U52 and U54 still red)
- refactor: none
- notes: the file's bytes are asserted equal to `encode(&snapshot)`, and a second store on the same directory loads what the first saved (what a service start does). An empty snapshot loads as `History` with no lines (DM §3).
- commit: `feat(041): HistoryStore in micold-core: save through owner_only::write, load, Unchanged on an equal checksum, no directory creation in a container (U44-U54)` (`dc92df9a`)

## Cycle 53: U45 `load` of an absent file gives `None`

- test: `crates/micold-core/tests/terminal_history_store.rs::a_session_with_no_file_loads_as_none` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_store` (one shared run, all 18 tests written together against a stub: `save` returns `Err(Unsupported)`, `load` returns `None`, `history_dir()` returns `None`: `test result: FAILED. 0 passed; 18 failed`; commit `f02def10`)
  -> ``called `Result::unwrap()` on an `Err` value: Kind(Unsupported)`` at the first `save` of the other session; the first assertion (no directory at all) held against the stub, whose `load` is `None` always
- green: with cycle 52's green: `File::open` failing with `NotFound` is `None`, also when the directory is absent. Same run.
- refactor: none
- notes: the stub could not tell this from the real thing on its own, so the test also saves another session's file and loads it as `History`: a `load` that answers `None` always fails there.
- commit: `feat(041): HistoryStore in micold-core: save through owner_only::write, load, Unchanged on an equal checksum, no directory creation in a container (U44-U54)` (`dc92df9a`)

## Cycle 54: U46 `load` of a damaged file gives `Damaged`

- test: `crates/micold-core/tests/terminal_history_store.rs::a_damaged_file_loads_as_damaged_with_its_reason`, `…::a_file_over_the_size_cap_loads_as_too_large`, `…::a_save_replaces_a_damaged_file` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_store` (one shared run, all 18 tests written together against a stub: `save` returns `Err(Unsupported)`, `load` returns `None`, `history_dir()` returns `None`: `test result: FAILED. 0 passed; 18 failed`; commit `f02def10`)
  -> `a_damaged_file_loads_as_damaged_with_its_reason`: `left: None` / `right: Damaged(Truncated)`; `a_file_over_the_size_cap_loads_as_too_large`: `left: None` / `right: Damaged(TooLarge)`; `a_save_replaces_a_damaged_file`: ``called `Result::unwrap()` on an `Err` value: Kind(Unsupported)`` at the first `save`
- green: with cycle 52's green: `load` returns `decode`'s reason; the length is taken from the open file's metadata and one over `MAX_FILE_BYTES` is `TooLarge` before any content is read (HF §4 checks 1 then 2), and the read itself is capped at `MAX_FILE_BYTES + 1`. Same run.
- refactor: none
- notes: the too-large file is sparse (`set_len`), so the test costs no disk. An empty file is `Damaged(NotAHistory)`, not `None`.
- commit: `feat(041): HistoryStore in micold-core: save through owner_only::write, load, Unchanged on an equal checksum, no directory creation in a container (U44-U54)` (`dc92df9a`)

## Cycle 55: U47 `load` of a file with mode `000` gives `Damaged(Unreadable)`

- test: `crates/micold-core/tests/terminal_history_store.rs::unix::a_file_that_cannot_be_opened_loads_as_unreadable` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_store` (one shared run, all 18 tests written together against a stub: `save` returns `Err(Unsupported)`, `load` returns `None`, `history_dir()` returns `None`: `test result: FAILED. 0 passed; 18 failed`; commit `f02def10`)
  -> ``called `Result::unwrap()` on an `Err` value: Kind(Unsupported)`` at the first `save`
- green: with cycle 52's green: an open or read error other than `NotFound` is `Damaged(Unreadable(kind))`; here `PermissionDenied`. Same run.
- refactor: none
- notes: returns early when run as root, as `owner_only`'s read-only test does.
- commit: `feat(041): HistoryStore in micold-core: save through owner_only::write, load, Unchanged on an equal checksum, no directory creation in a container (U44-U54)` (`dc92df9a`)

## Cycle 56: U48 A save over an existing file leaves no temporary file

- test: `crates/micold-core/tests/terminal_history_store.rs::a_save_over_an_existing_file_leaves_no_temporary_file` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_store` (one shared run, all 18 tests written together against a stub: `save` returns `Err(Unsupported)`, `load` returns `None`, `history_dir()` returns `None`: `test result: FAILED. 0 passed; 18 failed`; commit `f02def10`)
  -> ``called `Result::unwrap()` on an `Err` value: Kind(Unsupported)`` at the first `save`
- green: with cycle 52's green (`owner_only::write` renames its temporary file). Same run.
- refactor: none
- notes: none
- commit: `feat(041): HistoryStore in micold-core: save through owner_only::write, load, Unchanged on an equal checksum, no directory creation in a container (U44-U54)` (`dc92df9a`)

## Cycle 57: U49 A temporary file left behind before the rename leaves the previous file loadable

- test: `crates/micold-core/tests/terminal_history_store.rs::a_temporary_file_left_behind_leaves_the_previous_file_loadable` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_store` (one shared run, all 18 tests written together against a stub: `save` returns `Err(Unsupported)`, `load` returns `None`, `history_dir()` returns `None`: `test result: FAILED. 0 passed; 18 failed`; commit `f02def10`)
  -> ``called `Result::unwrap()` on an `Err` value: Kind(Unsupported)`` at the first `save`
- green: with cycle 52's green: `load` reads only `<uuid>.history`; the next `save` goes through `owner_only::write_with`, which removes a leftover `.<uuid>.history.tmp` first. Same run.
- refactor: none
- notes: the leftover is half of another snapshot's encoding under the name `owner_only` uses for its temporary file. The test knows that name; a change of it in `owner_only.rs` needs this test changed with it.
- commit: `feat(041): HistoryStore in micold-core: save through owner_only::write, load, Unchanged on an equal checksum, no directory creation in a container (U44-U54)` (`dc92df9a`)

## Cycle 58: U50 Two ids make two files and never each other's content

- test: `crates/micold-core/tests/terminal_history_store.rs::two_sessions_have_two_files_and_never_each_others_content` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_store` (one shared run, all 18 tests written together against a stub: `save` returns `Err(Unsupported)`, `load` returns `None`, `history_dir()` returns `None`: `test result: FAILED. 0 passed; 18 failed`; commit `f02def10`)
  -> ``called `Result::unwrap()` on an `Err` value: Kind(Unsupported)`` at the first `save`
- green: with cycle 52's green. Same run.
- refactor: none
- notes: none
- commit: `feat(041): HistoryStore in micold-core: save through owner_only::write, load, Unchanged on an equal checksum, no directory creation in a container (U44-U54)` (`dc92df9a`)

## Cycle 59: U51 A second `save` of an equal snapshot returns `Unchanged`, and the file's modification time and inode stay

- test: `crates/micold-core/tests/terminal_history_store.rs::a_second_save_of_an_equal_snapshot_is_unchanged`, `…::unix::an_unchanged_save_leaves_the_file_as_it_is`, `…::the_first_save_of_a_store_writes_even_over_an_equal_file` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_store` (one shared run, all 18 tests written together against a stub: `save` returns `Err(Unsupported)`, `load` returns `None`, `history_dir()` returns `None`: `test result: FAILED. 0 passed; 18 failed`; commit `f02def10`)
  -> ``called `Result::unwrap()` on an `Err` value: Kind(Unsupported)`` at the first `save`; then, with cycle 52's body, on the assertion: `left: Saved` / `right: Unchanged` in both tests
- green: `State.last_written: HashMap<SessionId, [u8; 32]>` under the mutex; `save` compares the last 32 bytes of the encoding (its SHA-256) with the entry and returns `Unchanged` before any file operation, and records the checksum after a successful write. `format::CHECKSUM_BYTES` became `pub(super)`. Same command -> `test result: FAILED. 16 passed; 2 failed` (U52, U54)
- refactor: none
- notes: "last written" is what this store wrote, not what is on disk: a new store writes over an equal file (`the_first_save_of_a_store_writes_even_over_an_equal_file`). The inode and the modification time (seconds and nanoseconds, after a 20 ms sleep) are `cfg(unix)`; the answer itself is tested on every platform.
- commit: `feat(041): HistoryStore in micold-core: save through owner_only::write, load, Unchanged on an equal checksum, no directory creation in a container (U44-U54)` (`dc92df9a`)

## Cycle 60: U52 With `create_dir = false` and no directory `save` returns `Skipped` and creates nothing; with the directory present it saves

- test: `crates/micold-core/tests/terminal_history_store.rs::a_store_that_does_not_create_its_directory_skips_until_it_exists` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_store` (one shared run, all 18 tests written together against a stub: `save` returns `Err(Unsupported)`, `load` returns `None`, `history_dir()` returns `None`: `test result: FAILED. 0 passed; 18 failed`; commit `f02def10`)
  -> ``called `Result::unwrap()` on an `Err` value: Kind(Unsupported)`` at the first `save`; then, with cycle 59's body: `left: Saved` / `right: Skipped(NoDirectory)`
- green: `HistoryStore` keeps `create_dir`; `save` returns `Skipped(SkipReason::NoDirectory)` when it is false and `dir` is not a directory, before the write and without recording a checksum. Same command -> `test result: FAILED. 17 passed; 1 failed` (U54)
- refactor: none
- notes: the save after the directory appears is `Saved` although the snapshot is the one that was skipped: a skipped save is not "last written". `SkipReason` has this one variant; T044 and T058 add the setting and the forgotten session.
- commit: `feat(041): HistoryStore in micold-core: save through owner_only::write, load, Unchanged on an equal checksum, no directory creation in a container (U44-U54)` (`dc92df9a`)

## Cycle 61: U53 The directory is mode `0700` and the file `0600`

- test: `crates/micold-core/tests/terminal_history_store.rs::unix::the_directory_is_0700_and_the_file_0600` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_store` (one shared run, all 18 tests written together against a stub: `save` returns `Err(Unsupported)`, `load` returns `None`, `history_dir()` returns `None`: `test result: FAILED. 0 passed; 18 failed`; commit `f02def10`)
  -> ``called `Result::unwrap()` on an `Err` value: Kind(Unsupported)`` at the first `save`
- green: with cycle 52's green (`owner_only::write`). Same run.
- refactor: none
- notes: none
- commit: `feat(041): HistoryStore in micold-core: save through owner_only::write, load, Unchanged on an equal checksum, no directory creation in a container (U44-U54)` (`dc92df9a`)

## Cycle 62: U54 `history_dir()` ends in `terminal-history` under `data_local_dir()`, and on Windows is not under `data_dir()`

- test: `crates/micold-core/tests/terminal_history_store.rs::unix::the_history_directory_is_terminal_history_under_the_local_data_directory`, `…::on_windows_the_history_directory_is_local_and_not_in_the_roaming_profile` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test terminal_history_store` (one shared run, all 18 tests written together against a stub: `save` returns `Err(Unsupported)`, `load` returns `None`, `history_dir()` returns `None`: `test result: FAILED. 0 passed; 18 failed`; commit `f02def10`)
  -> `left: None` / `right: Some("/home/jaro/.local/share/micold-ai-ide/terminal-history")`
- green: `history_dir()` is `ProjectDirs::from("", "", "micold-ai-ide")`'s `data_local_dir()` joined with `terminal-history`. Same command -> `test result: ok. 18 passed; 0 failed`
- refactor: none
- notes: the test was changed after this green, for a reason of its own: as first written it named `ProjectDirs::from(` to compute the expected path, and `mise run test-core` failed in `tests_never_write_the_real_data_directory` (a test file may not resolve the developer's data directory unredirected). It now sets `XDG_DATA_HOME` and `HOME` to a temporary directory and expects `<tmp>/micold-ai-ide/terminal-history` (macOS: under `Library/Application Support`), in `mod unix`. Checked with a mutant (`"terminal-histor"`): `left: Some("/tmp/.tmpJOpWwI/micold-ai-ide/terminal-histor")` / `right: Some("/tmp/.tmpJOpWwI/micold-ai-ide/terminal-history")`, restored. The `cfg(windows)` test compares with `%LOCALAPPDATA%\micold-ai-ide\data\terminal-history` and asserts the path is not under `%APPDATA%`; red not observed, it is compiled only here (`cargo check -p micold-core --all-targets --target x86_64-pc-windows-msvc` -> `Finished`) and CI's Windows leg is its first run.
- commit: `feat(041): HistoryStore in micold-core: save through owner_only::write, load, Unchanged on an equal checksum, no directory creation in a container (U44-U54)` (`dc92df9a`)

End of U44-U54: `mise run test-core` green (152 test binaries ok, 1759 passed, 0 failed, 7 ignored); `terminal_history_store` -> `test result: ok. 18 passed; 0 failed`; `terminal_history_format` -> `test result: ok. 20 passed; 0 failed`; `scripts/build-lock.sh cargo clippy -p micold-core --all-targets -- -D warnings` clean; `scripts/build-lock.sh cargo check -p micold-core --all-targets --target x86_64-pc-windows-msvc` -> `Finished`. The eleven behaviours were driven as one red run against a stub and four green steps (cycles 52, 59, 60, 62), not eleven separate red-green pairs; `a_store_is_shared_between_threads` (`HistoryStore: Send + Sync`, a save from another thread) belongs to no row and passed with cycle 52's green.

# M2, daemon: T017, T018, T022, T023 (A2-A6, A19, U60-U64)

The 13 tests of `history_service_restart.rs` and the one of `history_timing.rs` were written together and run red once against a stub (`DaemonState::set_history_store` existed, nothing saved or loaded), then made green in two steps: the save at each capture point (T022), then the load at a start (T023). The entries below share that red run and name the step that turned each green.

## Cycle 63: T022 An AI CLI session has a saved-history file after a stop, a Regular Terminal has none (U62)

- test: `crates/micold-daemon/tests/history_service_restart.rs::u62_a_regular_terminal_has_no_file_after_a_stop` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_service_restart` (one shared run, all 13 tests written together against a `DaemonState` that holds a `HistoryStore` and uses it for nothing: `test result: FAILED. 2 passed; 11 failed`; commit `c27e5de5`)
  -> `assertion `left == right` failed: one file, the AI CLI session's` / `left: []` / `right: ["/tmp/.tmp8eP76s/199ee9a7-….history"]`
- green: step 1, the save: `carry_history` keeps a clone of the captured snapshot, releases the state lock and calls `save_history`, which calls `HistoryStore::save` and logs one `warn!` with `session` and `reason` on `Err`. `scripts/build-lock.sh cargo test -p micold-daemon --test history_service_restart` -> `test result: FAILED. 6 passed; 7 failed`
- refactor: none
- notes: the red was the missing file of the AI CLI session beside it; the Regular Terminal half is the negative and cannot fail before the save exists. `carry_history` is called only for a covered primary (M1), so no new condition was needed.
- commit: `feat(041): the service saves a history at each process end and loads it at a session's first start (T022, T023)` (`a76c7051`)

## Cycle 64: T022 A failed save is one warning naming the session and the reason; a skipped save is none (FR-007, R15)

- test: `crates/micold-daemon/tests/history_service_restart.rs::a_failed_save_is_one_warning_and_the_stop_and_the_next_start_go_on`, `…::a_skipped_save_is_not_a_warning` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_service_restart` (one shared run, all 13 tests written together against a `DaemonState` that holds a `HistoryStore` and uses it for nothing: `test result: FAILED. 2 passed; 11 failed`; commit `c27e5de5`)
  -> `assertion `left == right` failed: one warning naming the session: []` / `left: 0` / `right: 1`
- green: with cycle 63's green. Same run.
- refactor: none
- notes: `a_skipped_save_is_not_a_warning` passed at the red run (nothing was saved, so nothing was logged) and guards the green: its `Skipped(NoDirectory)` is `Ok`, so it is not logged. No mutant was run for it. The failure is made with a regular file where the directory should be.
- commit: `feat(041): the service saves a history at each process end and loads it at a session's first start (T022, T023)` (`a76c7051`)

## Cycle 65: U61 A process that exits by itself is saved at that exit and restored after a restart

- test: `crates/micold-daemon/tests/history_service_restart.rs::u61_a_process_that_exits_by_itself_is_saved_at_the_exit_and_restored` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_service_restart` (one shared run, all 13 tests written together against a `DaemonState` that holds a `HistoryStore` and uses it for nothing: `test result: FAILED. 2 passed; 11 failed`; commit `c27e5de5`)
  -> `saved by the tick that saw the exit, with no stop request` (the file is absent); after step 1: `assertion `left == right` failed: one separator: ["after restart", …]`
- green: step 2, the load: `carried_seed` became `start_seed`, which calls `saved_seed` (`HistoryStore::load`) only when `carried` has no entry for the session; `server::run` builds the store from `history_dir()`. `scripts/build-lock.sh cargo test -p micold-daemon --test history_service_restart` -> `test result: ok. 13 passed; 0 failed` (after the test correction noted under cycle 67)
- refactor: none
- notes: the save half went green with step 1 (the tick's clean-exit drop goes through `carry_history`), the restore half with step 2. Runs on every platform.
- commit: `feat(041): the service saves a history at each process end and loads it at a session's first start (T022, T023)` (`a76c7051`)

## Cycle 66: A1, A2 After a stop and a service restart a start shows the 200 styled lines, one separator with the time of that start, then the new output

- test: `crates/micold-daemon/tests/history_service_restart.rs::a1_a2_after_a_service_restart_a_start_shows_the_lines_one_separator_and_the_new_output` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_service_restart` (one shared run, all 13 tests written together against a `DaemonState` that holds a `HistoryStore` and uses it for nothing: `test result: FAILED. 2 passed; 11 failed`; commit `c27e5de5`)
  -> `no line reads "line 1" in ["new output"]`
- green: step 2, the load: `carried_seed` became `start_seed`, which calls `saved_seed` (`HistoryStore::load`) only when `carried` has no entry for the session; `server::run` builds the store from `history_dir()`. `scripts/build-lock.sh cargo test -p micold-daemon --test history_service_restart` -> `test result: ok. 13 passed; 0 failed` (after the test correction noted under cycle 67)
- refactor: none
- notes: the service restart is a second `DaemonState` with the same sessions and a new `HistoryStore` on the same temporary directory. The separator is compared with `session restarted at <local date and minute>` taken before and after the start. Colour (`Basic(1)`) and bold are read from line 200. A1 stays PENDING in the list: its orderly stop of the service is T031 and T061. Runs on every platform.
- commit: `feat(041): the service saves a history at each process end and loads it at a session's first start (T022, T023)` (`a76c7051`)

## Cycle 67: A5, U60 A history longer than the limit, and a smaller limit at the restore, restore the most recent lines up to the limit

- test: `crates/micold-daemon/tests/history_service_restart.rs::a5_a_history_longer_than_the_limit_restores_the_most_recent_lines`, `…::u60_a_smaller_limit_at_the_restore_shows_the_most_recent_lines_up_to_it` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_service_restart` (one shared run, all 13 tests written together against a `DaemonState` that holds a `HistoryStore` and uses it for nothing: `test result: FAILED. 2 passed; 11 failed`; commit `c27e5de5`)
  -> A5: `assertion `left == right` failed: one separator` / `left: 0` / `right: 1`; U60: `the stop saved no readable history`, and after step 1 the same as A5
- green: step 2, the load: `carried_seed` became `start_seed`, which calls `saved_seed` (`HistoryStore::load`) only when `carried` has no entry for the session; `server::run` builds the store from `history_dir()`. `scripts/build-lock.sh cargo test -p micold-daemon --test history_service_restart` -> `test result: ok. 13 passed; 0 failed` (after the test correction noted under cycle 67)
- refactor: none
- notes: the tests were corrected once, before green, for a reason of their own: as first written they required at least `limit` restored lines and failed with `99 lines restored with a limit of 100 and 30 rows`. `history::seed` writes the separator below the lines and then moves the screen into the history, so a history of `limit` rows holds the separator and the most recent `limit - 1` lines. The assertion is now exact: restored lines + 1 == limit, and they count back from `line 400` without a gap. U60 first loads the file with a store of its own and finds all 400 lines in it.
- commit: `feat(041): the service saves a history at each process end and loads it at a session's first start (T022, T023)` (`a76c7051`)

## Cycle 68: A4, A6 Two restarts show output, separator, output, separator; two sessions each show only their own history

- test: `crates/micold-daemon/tests/history_service_restart.rs::a4_a_second_restart_shows_output_separator_output_separator_in_order`, `…::a6_two_sessions_each_show_only_their_own_history_after_a_restart` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_service_restart` (one shared run, all 13 tests written together against a `DaemonState` that holds a `HistoryStore` and uses it for nothing: `test result: FAILED. 2 passed; 11 failed`; commit `c27e5de5`)
  -> `assertion `left == right` failed: ["three"]` / `left: 0` / `right: 2` (A4); `["restarted"]` / `left: 0` / `right: 1` (A6)
- green: step 2, the load: `carried_seed` became `start_seed`, which calls `saved_seed` (`HistoryStore::load`) only when `carried` has no entry for the session; `server::run` builds the store from `history_dir()`. `scripts/build-lock.sh cargo test -p micold-daemon --test history_service_restart` -> `test result: ok. 13 passed; 0 failed` (after the test correction noted under cycle 67)
- refactor: none
- notes: A4 uses three services on one directory. A6 uses the `args` line, which carries each session's id.
- commit: `feat(041): the service saves a history at each process end and loads it at a session's first start (T022, T023)` (`a76c7051`)

## Cycle 69: A19 A file of random bytes: the session starts with no history and one warning naming the session

- test: `crates/micold-daemon/tests/history_service_restart.rs::a19_a_file_of_random_bytes_starts_the_session_with_no_history_and_one_warning` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_service_restart` (one shared run, all 13 tests written together against a `DaemonState` that holds a `HistoryStore` and uses it for nothing: `test result: FAILED. 2 passed; 11 failed`; commit `c27e5de5`)
  -> `assertion `left == right` failed: one warning naming the session: []` / `left: 0` / `right: 1`
- green: step 2, the load: `carried_seed` became `start_seed`, which calls `saved_seed` (`HistoryStore::load`) only when `carried` has no entry for the session; `server::run` builds the store from `history_dir()`. `scripts/build-lock.sh cargo test -p micold-daemon --test history_service_restart` -> `test result: ok. 13 passed; 0 failed` (after the test correction noted under cycle 67)
- refactor: none
- notes: `LoadOutcome::Damaged(reason)` gives `Seed::None` and one `warn!` (`saved terminal history could not be read; starting without it`, `session`, `reason`). The notice line of data-model §6 arrives with T054. The file holds 4 KiB from a fixed xorshift and a readable marker; the only non-empty line of the terminal is the new output.
- commit: `feat(041): the service saves a history at each process end and loads it at a session's first start (T022, T023)` (`a76c7051`)

## Cycle 70: U63 A file is not read while a carried history exists

- test: `crates/micold-daemon/tests/history_service_restart.rs::u63_a_file_is_not_read_while_a_carried_history_exists` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_service_restart` (one shared run, all 13 tests written together against a `DaemonState` that holds a `HistoryStore` and uses it for nothing: `test result: FAILED. 2 passed; 11 failed`; commit `c27e5de5`)
  -> `the stop saved the history` (the file is absent)
- green: with cycle 63's green (step 1): the start in the same run was already seeded from `carried`. Still green after step 2, which is what the row is about: `start_seed` does not call `load` when `carried` has the id.
- refactor: none
- notes: the file is replaced by unreadable bytes between the stop and the start; the start shows the carried line above one separator and logs nothing for the session. No mutant (a `load` before the `carried` lookup) was run.
- commit: `feat(041): the service saves a history at each process end and loads it at a session's first start (T022, T023)` (`a76c7051`)

## Cycle 71: A3 A session that printed nothing shows no separator and no blank history after a restart

- test: `crates/micold-daemon/tests/history_service_restart.rs::a3_a_session_that_printed_nothing_shows_no_separator_after_a_restart` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_service_restart` (one shared run, all 13 tests written together against a `DaemonState` that holds a `HistoryStore` and uses it for nothing: `test result: FAILED. 2 passed; 11 failed`; commit `c27e5de5`)
  -> none: it passed at the red run, since nothing was restored at all
- green: passes with steps 1 and 2: the stop saves an empty snapshot, and `LoadOutcome::History` of an empty snapshot is `Seed::None`.
- refactor: none
- notes: red not observed for this row; it is the negative of cycle 66 and has no failing state before the load exists. No mutant (seeding an empty history) was run.
- commit: `feat(041): the service saves a history at each process end and loads it at a session's first start (T022, T023)` (`a76c7051`)

## Cycle 72: D17 A `SessionStop` followed at once by a `SessionStart` over a real connection saves and restores in that order

- test: `crates/micold-daemon/tests/history_service_restart.rs::a_stop_then_a_start_over_a_connection_saves_and_restores_in_that_order` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_service_restart` (one shared run, all 13 tests written together against a `DaemonState` that holds a `HistoryStore` and uses it for nothing: `test result: FAILED. 2 passed; 11 failed`; commit `c27e5de5`)
  -> `the stop saved no readable history`
- green: with cycle 63's green (step 1). Same run.
- refactor: none
- notes: belongs to no row of the list; it covers the path M1 left unproven. `current_thread` runtime, `serve_connection` on an in-memory stream, both messages sent back to back. `ops::start_session` and `ops::stop_session` were not changed: the save and the load run inside `stop_session_gated` and `start_session_gated`, which those already call on the blocking pool with the gate held.
- commit: `feat(041): the service saves a history at each process end and loads it at a session's first start (T022, T023)` (`a76c7051`)

## Cycle 73: U64 A saved history of 10,000 lines of 100 characters delays the start by no more than 1 s

- test: `crates/micold-daemon/tests/history_timing.rs::u64_a_saved_history_of_ten_thousand_lines_delays_the_start_by_no_more_than_a_second` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_timing -- --nocapture` (commit `c27e5de5`) -> `test result: FAILED. 0 passed; 1 failed`
  -> `start with no file: 2.480697ms; with 10,000 saved lines: 2.314284ms` and `no line reads "saved.1....…" in ["ready"]`
- green: step 2 (the load). Same command -> `start with no file: 2.479354ms; with 10,000 saved lines: 154.139304ms`, `test result: ok. 1 passed; 0 failed`
- refactor: none
- notes: debug build. Each start is timed from the request until the stand-in has created a file as its first step; a third session is started first and not measured. The test was corrected once before green: at the default limit of 10,000 the separator takes the place of the oldest line (cycle 67), so the service's limit is set to 20,000 and all 10,000 lines are asserted present above the separator.
- commit: `feat(041): the service saves a history at each process end and loads it at a session's first start (T022, T023)` (`a76c7051`)

End of T017, T018, T022, T023: `history_service_restart` -> `test result: ok. 13 passed; 0 failed` and `history_timing` -> `test result: ok. 1 passed; 0 failed`, three runs in a row (156.9 ms, 156.3 ms, 163.1 ms against 2.3 ms, 2.3 ms, 2.1 ms); `scripts/build-lock.sh cargo test -p micold-daemon` -> 109 test binaries ok, 716 passed, 0 failed; `cargo clippy -p micold-daemon --all-targets -- -D warnings` clean; `cargo check -p micold-daemon --all-targets --target x86_64-pc-windows-msvc` -> `Finished`; `micold-core`'s `tests_never_write_the_real_data_directory` -> `2 passed`. Refactor after green (`8e636d9e`): the stand-in CLI and the history helpers moved to `crates/micold-daemon/tests/support/history.rs`, used by `history_restart_in_run.rs`, `history_service_restart.rs` and `history_timing.rs`. The eleven behaviours were driven as one red run and two green steps, not eleven separate red-green pairs.

## Red CI on PR #578: the directory's DACL has two entries on Windows (U57)

- red (CI run 37219182985, `build + test (windows-latest)`, the first run of these tests): `crates/micold-core/tests/owner_only.rs::windows::the_written_file_and_its_directory_have_a_protected_dacl_for_the_current_user_only` and `…::ensure_dir_gives_a_missing_and_an_existing_directory_the_owner_only_dacl`
  -> ``assertion `left == right` failed: the directory`` / `left: Dacl { protected: true, ace_count: 2, first_ace_allows_current_user: true }` / `right: Dacl { protected: true, ace_count: 1, … }` (5 passed; 2 failed). The file's assertion in the same test passed.
- cause: the directory's entry was `(A;OICI;GA;;;<sid>)`. Windows stores an inheritable entry with a generic right on a directory as two: one effective with the right mapped to the file rights, one inherit-only with the generic right. The file's entry has no inheritance flags, so it stays one. The SDDL came unchanged from the daemon's helper, whose test never read the directory's DACL.
- green: `ensure_dir` writes `(A;OICI;FA;;;<sid>)`: the file rights by name, nothing to map, one entry. Not runnable here; the Windows leg of the next CI run is the proof. Locally: `cargo check --workspace --all-targets --target x86_64-pc-windows-msvc` and `mise run gate`.
- refactor: none

## Cycles 74-75 (M3): the save schedule of a running terminal (U65-U71)

- test: `crates/micold-core/tests/terminal_history_schedule.rs` (new), written first against a stub whose `due` is never true (commit `bafd637d`)
- red: the stub's run failed the cases that expect a save; the cases that expect none passed against it, as a stub that is never due would
- green: `SaveSchedule` in `schedule.rs` (commit `f3ad3d85`)
- refactor: none
- commit: `feat(041): SaveSchedule — a running terminal is due when its output moved and 30 s passed (T027)` (`f3ad3d85`)

## Cycle 76 (M3): the saver saves a running terminal when due, off the state lock and under its gate (U72-U78)

- test: `crates/micold-daemon/tests/history_periodic_save.rs` (new): U72, U73, U74, U75, U76, U77, U78 and one case for FR-005 (input and a resize during saves)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test history_periodic_save` against a `save_due_at` that returns at once (the test written after the saver, then the saver stubbed to prove it): `test result: FAILED. 2 passed; 6 failed`. U72, U73, U74, U75, U77 and U78 failed (`no saved history` or `the file is there`); U76 (a Regular Terminal, nothing saved) and the FR-005 case pass against a saver that does nothing, as they would
- green: `DaemonState::save_due_at` with `history::Saver` (a `SaveSchedule` per covered live terminal, keyed by session and by process, so a restarted process starts a new one; failures logged once per session and reason), and `spawn_history_saver` in `server.rs`. Same command: `test result: ok. 8 passed; 0 failed`
- refactor: none
- notes: the stub was put in after the code, so this red is shown by mutation, not by a red run before the code. Input needs a rising serial per session: the first version of the tests reused serial 0 and the stand-in saw only the first line. The saver holds the session's gate from the check that the process is still the live one until its file is written, so a stop's final save is never overwritten by an older one. The saver task is spawned in `server.rs` beside the supervision tick, not in `main.rs`, which only calls `run`.
- commit: `feat(041): the saver saves a running terminal when due, off the state lock and under its gate (T026, T028-T030)`
