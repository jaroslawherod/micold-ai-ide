# Cycle Log: Notify When a Session Needs Attention, and Track Unread Sessions

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and
failed before the implementation.

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace` -> 4004 passed, 1 failed, 7 ignored in the 315 binaries that ran; the run stopped at `micold-daemon --test mcp_create_session` (`a_pi_session_start_event_makes_pi_ready`, `left: ""` at line 600), so the binaries after it did not run. `scripts/build-lock.sh cargo test -p micold-daemon --test mcp_create_session` alone -> 19 passed, 0 failed. The branch changes only `specs/` at this point, so the failure predates the feature; it is listed in the ledger's follow-ups.
- commit: `a8521731` (test list planned against it)
- recorded: cycle 0, before any change. Baseline `unknown`: the first M1 cycle re-measures on its own base before the loop starts.

## Baseline, re-measured for M1

- suite: `scripts/build-lock.sh cargo test --workspace --no-fail-fast` on the M1 base (`6b4b6fa9`)
  -> 4352 passed, 2 failed, 9 ignored in 384 binaries. `--no-fail-fast` so that one failure could
  not hide the binaries after it. Every test that existed before M1 passed, also
  `mcp_create_session::a_pi_session_start_event_makes_pi_ready`. The 2 failures are cycle 1's new
  tests: the run was started on the clean tree, and cycle 1's test file and stub were written
  while it was building, so `micold-core`'s lib tests were compiled with them. The baseline is
  green.
- recorded: before any source change of M1 other than cycle 1's tests and stub.

## Batching note (M1)

As in `specs/038-issue-list-reporter-tooltip/tdd/cycle-log.md`: cycles are grouped per task pair
(the test task and its implementation task). A group's tests are written together and observed
failing together against stubs that only make the symbols resolve, then made green together. One
entry per group, every behavior id listed with its test. The inner loop runs the crate's tests; the
workspace suite runs in the milestone's gate. The build lock is shared with other worktrees and
was held by them for 10 minutes and more at a time during this milestone.

## Cycle 1 — U1, U2, U3, U4, U5 — T001, T007

- tests: `crates/micold-core/src/attention.rs::tests::{a_focused_window_showing_its_session_has_the_selected_session_in_view (U1),
  an_unfocused_window_has_no_session_in_view (U2), a_window_whose_main_area_is_taken_has_no_session_in_view (U3),
  a_window_with_no_selected_session_has_no_session_in_view (U4), the_shown_tab_is_not_one_of_the_facts (U5)}` (new)
- red: `scripts/build-lock.sh cargo test --workspace --no-fail-fast` (the baseline run above),
  against the stub `in_view` returning `None`
  ```
  thread 'attention::tests::a_focused_window_showing_its_session_has_the_selected_session_in_view' (105084) panicked at crates/micold-core/src/attention.rs:44:9:
  assertion `left == right` failed: a focused window whose main area shows the session has that session in view
    left: None
   right: Some(SessionId(15b7c06d-2eab-4445-a86f-647584c517a5))
  thread 'attention::tests::the_shown_tab_is_not_one_of_the_facts' (105088) panicked at crates/micold-core/src/attention.rs:97:9:
  assertion `left == right` failed: the three facts decide the answer; which tab is shown is not among them
  test result: FAILED. 264 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- U2, U3 and U4 passed on first run: the stub answers `None`, which is what they expect.
  Deliberate mutant, a second stub `facts.selected.or(Some(SessionId::from_uuid(Uuid::nil())))`:
  `scripts/build-lock.sh cargo test -p micold-core --lib attention::tests` ->
  `an_unfocused_window_has_no_session_in_view ... FAILED` (`left: Some(SessionId(d44985c6-…))`, `right: None`),
  `a_window_whose_main_area_is_taken_has_no_session_in_view ... FAILED`,
  `a_window_with_no_selected_session_has_no_session_in_view ... FAILED`
  (`left: Some(SessionId(00000000-0000-0000-0000-000000000000))`); `test result: FAILED. 2 passed; 3 failed`.
- green: `in_view` returns `selected` when `window_focused && !main_area_taken`, else `None`.
  `scripts/build-lock.sh cargo test -p micold-core --lib attention::tests` -> 5 passed, 0 failed.
  The crate's whole suite was not re-run after the green; it runs with cycle 2's red.
- refactor: none. One function of five lines.
- U5: `ViewFacts` has no field for the shown tab, so the rule holds by shape. The test destructures
  the struct without `..`, so a field added for the tab fails to compile there.
- notes: A10 stays PENDING; its client half is T006.

## Cycle 2 — U6, U7, U8 — T002, T008 (red observed; green written, not yet run)

- tests: `crates/micold-core/src/store.rs::attention_seq_tests::{a_session_stored_without_an_attention_sequence_reads_as_zero (U6),
  a_store_round_trip_keeps_the_attention_sequence (U7), the_attention_sequence_leaves_the_schema_version_as_it_was (U8)}` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --all-targets --no-fail-fast`, against the
  stub field `Session::attention_seq` that `StoredSession` does not carry
  ```
  thread 'store::attention_seq_tests::a_store_round_trip_keeps_the_attention_sequence' (390947) panicked at crates/micold-core/src/store.rs:1063:9:
  assertion `left == right` failed: the count of attention events is read back as it was written
    left: 0
   right: 3
  test result: FAILED. 268 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- U6 and U8 passed on first run: nothing is stored yet, so the value reads `0` and the schema
  version is untouched. They need a deliberate mutant once the green has run: drop
  `#[serde(default)]` from `StoredSession::attention_seq` (U6), bump `SCHEMA_VERSION` (U8).
- green (written, NOT YET RUN): `StoredSession::attention_seq` with `#[serde(default)]`, copied in
  `from_session` and `into_session`.

## Cycle 3 — U12 — T003, T009 (red observed; green written, not yet run)

- tests: `crates/micold-core/tests/schema_hash.rs::{the_wire_changes_for_this_feature_cost_exactly_one_version_bump (pin moved to 21),
  the_view_report_and_the_attention_sequence_are_in_the_hashed_source}` (new);
  `crates/micold-core/src/protocol/messages.rs::attention_wire_tests::{a_view_report_encodes_and_decodes,
  a_session_summary_carries_its_attention_sequence}` and the samples in `tests/protocol_roundtrip.rs`
  (written with the green: they cannot compile before the types exist)
- red: the same run as cycle 2
  ```
  thread 'the_wire_changes_for_this_feature_cost_exactly_one_version_bump' (392770) panicked at crates/micold-core/tests/schema_hash.rs:200:5:
  assertion `left == right` failed: the protocol version moved. [...]
    left: 20
   right: 21
  thread 'the_view_report_and_the_attention_sequence_are_in_the_hashed_source' (392769) panicked at crates/micold-core/tests/schema_hash.rs:266:9:
  `WindowView {` is not in messages.rs, so version 21's hash is not the hash of the message set that reports views and counts attention events
  ```
- green (written, NOT YET RUN): `ClientMsg::WindowView { focused, in_view }`, the `WindowView`
  struct, `SessionSummary::attention_seq`, `PROTOCOL_VERSION` 21 with its doc line.
- the round-trip tests pass on first run by construction; mutant to run after the green:
  `#[serde(skip)]` on `SessionSummary::attention_seq`.

## Notes and deviations

- Cycle 1's test file was written while the baseline run was building, so that run is both the
  baseline and cycle 1's red. No implementation existed then: `in_view` was the stub.
- Cycles 2 and 3: the red was observed (above). The green and the tests and stubs of cycles 4 and 5
  were written by the second M1 unit, which reached its context cap before any build ran. See the
  ledger's *Handover*.
