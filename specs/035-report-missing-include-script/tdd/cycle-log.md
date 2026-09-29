# Cycle Log: Report a Missing Environment-Include Script

Append only. Newest last. Every entry's `red` block is the evidence that the test
existed and failed before the implementation.

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace` -> 3662 passed, 0 failed, 8 ignored
  (341 binaries)
- commit: `0f7e0c8f`
- recorded: cycle 0, before any change. The spec, plan and tasks edits for feature 035 were
  uncommitted in the working tree at the time, and they touch no code.

## Notes and deviations

- The outer loop runs at the binary's `App` in `crates/micold-client/src/main_tests.rs`, not an
  end-to-end GUI runner. See `test-list.md`, "Entry point".
- U63 is M1's interim behaviour and is meant to be superseded in M3 (T024). Its removal there is
  planned, not a regression.
- A2–A4 and U57 assert an absence (no lines, no probe call) and may pass once they compile. T033
  records a deliberate mutant as their red evidence.
- A9 and A10 (US3) exercise behaviour that M1 and M2 already ship (FR-009's check after every save). If
  they pass on arrival, T036 records a deliberate mutant as their red evidence.
