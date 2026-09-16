# Cycle Log: Real Terminal Emulator — BUG-007

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and failed
before the implementation.

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace` -> 3085 passed, 0 failed, 6 ignored (311 test binaries)
- commit: `3e513a1f` (BUG-007 record and spec patch; no code for the fix yet)
- recorded: cycle 0, before any change, 2026-09-15
- per-cycle runs use the changed crate's tests (`-p micold-core` / `-p micold-daemon` /
  `-p micold-client`); the whole workspace is re-run before each commit that touches more than one
  crate and by `mise run gate` before the PR. The full suite takes ~6 minutes behind a shared build
  lock, so it is not re-run after every refactor move.

## Cycle 1: U8 a dark terminal defaults to on_surface over surface

- test: `crates/micold-core/tests/tokens.rs::a_dark_terminal_defaults_to_on_surface_over_surface` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test tokens a_dark_terminal_defaults_to_on_surface_over_surface -- --exact`
  -> `panicked at crates/micold-core/src/tokens/mod.rs:356:5: not yet implemented: BUG-007 U8` (1 failed).
  The symbol was declared first as a `todo!()` stub, so the red is the not-implemented signal, not a
  compile error.
- green: fake it — `terminal_defaults` returns `DARK.on_surface` / `DARK.surface` for any scheme; U9
  forces the generalisation. Suite `scripts/build-lock.sh cargo test -p micold-core --all-targets`
  -> 1059 passed, 0 failed
- refactor: none — the fake is the cycle's intended state
- notes: the first attempt at this cycle hit `ENOSPC` (disk at 2 MB free) before compiling; space was
  reclaimed (shared `target-shared/debug/incremental`) and the cycle re-run from the start.

## Cycle 2: U9 a light terminal defaults to on_surface over surface

- test: `crates/micold-core/tests/tokens.rs::a_light_terminal_defaults_to_on_surface_over_surface` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --test tokens a_light_terminal_defaults_to_on_surface_over_surface -- --exact`
  -> `assertion left == right failed … left: TerminalDefaults { foreground: Rgb { r: 230, g: 225, b: 230 }, background: Rgb { r: 20, g: 19, b: 22 } }`
  (1 failed — the U8 fake answers dark)
- green: `crates/micold-core/src/tokens/mod.rs` `terminal_defaults` reads `roles(scheme)`. Suite
  `scripts/build-lock.sh cargo test -p micold-core --all-targets` -> 1060 passed, 0 failed
- refactor: separate structural commit — `TermPalette::from_scheme` (client) takes its `fg`/`bg` from
  `terminal_defaults` instead of naming the roles itself (T075)
  — done: `crates/micold-client/src/ui/terminal.rs` `TermPalette::from_scheme`; suite
  `scripts/build-lock.sh cargo test -p micold-client` -> 1696 passed, 0 failed (`style_snapshot`
  unchanged, no regeneration). T075 ticked (U8, U9 DONE).

## Cycle 3: U1 a dark pane answers a background query with the dark surface

- test: `crates/micold-daemon/tests/vt_color_queries.rs::a_dark_pane_answers_a_background_query_with_the_dark_surface` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test vt_color_queries a_dark_pane_answers_a_background_query_with_the_dark_surface -- --exact`
  -> `left: "\u{1b}]11;rgb:e5e5/e5e5/e5e5\u{7}"  right: "\u{1b}]11;rgb:1414/1313/1616\u{7}"` (1 failed —
  the reported reason: xterm entry 7 for a dark pane). `TerminalColors` and the new
  `DaemonListener::new` parameter were added first as a stub the listener ignored, with its two
  callers (`supervisor.rs`, `tests/support/mod.rs`) passing `TerminalColors::default()`, so the red
  is the assertion and not a compile error (T074).
- green: `crates/micold-daemon/src/terminal.rs` — `TerminalColors` holds the scheme in an
  `Arc<AtomicBool>`; the `ColorRequest` arm answers `NamedColor::Background` from
  `tokens::terminal_defaults(scheme).background`, read at reply time. Suite
  `scripts/build-lock.sh cargo test -p micold-daemon` -> 331 passed, 0 failed
- refactor: none yet — the `tokens::Rgb` → `vte::Rgb` conversion is written once; extract it when a
  second role needs it

## Cycle 4: U2 a dark pane answers a foreground query with the dark on_surface

- test: `crates/micold-daemon/tests/vt_color_queries.rs::a_dark_pane_answers_a_foreground_query_with_the_dark_on_surface` (new; queries with an `ST` terminator)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test vt_color_queries a_dark_pane_answers_a_foreground_query_with_the_dark_on_surface -- --exact`
  -> `left: "\u{1b}]10;rgb:e5e5/e5e5/e5e5\u{1b}\\"  right: "\u{1b}]10;rgb:e6e6/e1e1/e6e6\u{1b}\\"` (1 failed)
- green: the `ColorRequest` arm answers `NamedColor::Foreground` from `terminal_defaults(scheme).foreground`.
  Suite `scripts/build-lock.sh cargo test -p micold-daemon` -> 332 passed, 0 failed
- refactor: separate structural commit — the `tokens::Rgb` → `vte::Rgb` conversion, now written twice,
  becomes one function and the `if` chain a `match` on the dynamic index

## Cycle 5: U3 a dark pane answers a cursor query with the dark on_surface

- test: `crates/micold-daemon/tests/vt_color_queries.rs::a_dark_pane_answers_a_cursor_query_with_the_dark_on_surface` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test vt_color_queries a_dark_pane_answers_a_cursor_query_with_the_dark_on_surface -- --exact`
  -> `left: "\u{1b}]12;rgb:e5e5/e5e5/e5e5\u{7}"  right: "\u{1b}]12;rgb:e6e6/e1e1/e6e6\u{7}"` (1 failed)
- green: `TerminalColors::dynamic` answers `NamedColor::Cursor` with the foreground. Suite
  `scripts/build-lock.sh cargo test -p micold-daemon` -> 333 passed, 0 failed
- refactor: none needed, one match guard widened

## Cycles 6–9: U4–U7 the answer follows the scheme; palette queries are unchanged

These four pin behaviour cycles 3–5 already built (the listener reads `TerminalColors` at reply
time, and `dynamic` only sees indices past the palette), so each passed on first run. A passing
first run is not red evidence, so each was proved by mutating the production code instead.

- tests (new, `crates/micold-daemon/tests/vt_color_queries.rs`):
  U4 `a_light_pane_answers_a_background_query_with_the_light_surface`,
  U5 `a_scheme_changed_after_the_listener_was_built_answers_the_next_query`,
  U6 `before_any_scheme_is_reported_the_answer_is_light`,
  U7 `a_palette_query_is_still_answered_from_the_xterm_table`
- red (mutation, U4–U6): `TerminalColors::scheme` forced to `Dark` →
  `scripts/build-lock.sh cargo test -p micold-daemon --test vt_color_queries` -> 3 failed, each
  `left: "\u{1b}]11;rgb:1414/1313/1616\u{7}"` (the dark surface where light was expected); U1–U3, U7 passed
- red (mutation, U7): `TerminalColors::dynamic` answers the foreground for any index instead of
  `None` → 1 failed, `left: "\u{1b}]4;1;rgb:1c1c/1b1b/1e1e\u{7}"  right: "\u{1b}]4;1;rgb:cdcd/0000/0000\u{7}"`
- green: mutations reverted. Suite `scripts/build-lock.sh cargo test -p micold-daemon` -> 337 passed, 0 failed
- refactor: none

## Cycle 10: U10 `TerminalColorScheme` round-trips through the wire codec

- test: `crates/micold-core/tests/protocol_roundtrip.rs` `sample_client_msgs` gains
  `TerminalColorScheme { Dark }` and `{ Light }`; the version pins move with it —
  `protocol_auth.rs::the_protocol_version_is_thirteen` (renamed) and `schema_hash.rs`'s
  `FEATURE_026_PROTOCOL_VERSION` = 13
- red: the variant and `ColorScheme`'s serde were added first as a stub with the version left at 12, so
  the red is the wire-change gate, not a compile error.
  `scripts/build-lock.sh cargo test -p micold-core --test protocol_auth --test schema_hash --test protocol_roundtrip`
  -> `the_protocol_version_is_thirteen … left: 12 right: 13` (1 failed)
- green: `PROTOCOL_VERSION` 12 → 13 with its doc line; `server.rs` gains an arm that ignores the
  message until T078 wires it. Suite `scripts/build-lock.sh cargo test -p micold-core --all-targets`
  -> 1088 passed, 0 failed; `cargo check -p micold-daemon -p micold-client --all-targets` clean
- refactor: none (`cargo fmt` also rewrapped cycle 4's `ColorRequest` arm)
