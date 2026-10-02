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

## Notes and deviations

- Cycle 1's test file was written while the baseline run was building, so that run is both the
  baseline and cycle 1's red. No implementation existed then: `in_view` was the stub.
- Cycles 2 (U6–U8, T002/T008) and 3 (U12, T003/T009) were started and not finished: their tests
  and the stub field `Session::attention_seq` were written, and no red was observed before the
  unit handed over. They are in the stash entry named in the ledger's *Handover*, not in a commit.
