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
