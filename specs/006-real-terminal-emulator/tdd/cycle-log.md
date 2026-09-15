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
