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
