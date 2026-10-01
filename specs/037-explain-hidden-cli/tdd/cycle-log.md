# Cycle Log: Explain Why an AI CLI Is Not Offered

Append only. Newest last. Every entry's `red` block is the evidence that the test
existed and failed before the implementation.

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace` -> 4098 passed, 0 failed, 9 ignored
  (374 binaries), run as `mise run test`
- commit: `f490b395`
- recorded: cycle 0, before any change. The suite was running elsewhere when the test list was
  written, so the counts are filled in from that run. The spec, plan and tasks edits for feature
  037 touch no code.

## Notes and deviations

- The outer loop runs at two entry points, not an end-to-end GUI runner: the client's `App` in
  `crates/micold-client/src/main_tests.rs`, and the session service's integration tests for the two
  surfaces the service writes. See `test-list.md`, "Entry point".
- Behaviour ids `U<n>` are this list's. The contract's surfaces are written "surface U1" to
  "surface U6" and are a separate series.
- A7, A16, A17 (its note half) and A18 assert an absence and may pass once they compile. T039 and
  T044 record a deliberate mutant as their red evidence.
- A12 is a characterization test (US2-AS4, "unchanged from today"). It is written before T023 and
  must be green against the untouched launch gate. Its entry records that green, not a red.
- Twelve behaviours were `DONE` at planning, each held by an existing test: U37, U46, U49, U51, U52,
  U67, U68, U70, U76, U77, U86, U91.
