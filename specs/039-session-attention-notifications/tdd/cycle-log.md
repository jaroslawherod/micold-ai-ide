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

## Cycle 2 — U6, U7, U8 — T002, T008

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
  version is untouched. Both were then mutated against the green (below).
- green: `StoredSession::attention_seq` with `#[serde(default)]`, copied in `from_session` and
  `into_session`. `scripts/build-lock.sh cargo test -p micold-core --all-targets` (run1.log, `CORE_RC=0`)
  ```
  test store::attention_seq_tests::a_session_stored_without_an_attention_sequence_reads_as_zero ... ok
  test store::attention_seq_tests::the_attention_sequence_leaves_the_schema_version_as_it_was ... ok
  test store::attention_seq_tests::a_store_round_trip_keeps_the_attention_sequence ... ok
  ```
- mutants (run3.log, `== MUTANTS`): U6 `#[serde(default)]` dropped from `StoredSession::attention_seq`
  -> `MUTANT U6 KILLED`; U8 `SCHEMA_VERSION` 1 -> 2 -> `MUTANT U8 KILLED`.

## Cycle 3 — U12 — T003, T009

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
- green: `ClientMsg::WindowView { focused, in_view }`, the `WindowView` struct,
  `SessionSummary::attention_seq`, `PROTOCOL_VERSION` 21 with its doc line. Same core run as cycle 2
  (run1.log, `CORE_RC=0`)
  ```
  test protocol::messages::attention_wire_tests::a_session_summary_carries_its_attention_sequence ... ok
  test protocol::messages::attention_wire_tests::a_view_report_encodes_and_decodes ... ok
  test the_view_report_and_the_attention_sequence_are_in_the_hashed_source ... ok
  ```
- the round-trip tests pass on first run by construction. Mutant (run3.log): `#[serde(skip)]` on
  `SessionSummary::attention_seq` -> `MUTANT U12 KILLED`.

## Cycle 4 — U50, U51, U52, U53 — T004, T010

- tests: `crates/micold-daemon/src/attention.rs::tests::{a_second_report_from_a_connection_replaces_its_first (U50),
  an_unfocused_report_is_stored_with_nothing_in_view (U51), a_session_is_in_view_while_any_report_names_it (U52),
  a_removed_connection_s_report_is_forgotten (U53)}` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --lib attention::` (run1.log, `DLIB_RC=101`),
  against the stub `Views` that stores nothing
  ```
  thread 'attention::tests::a_second_report_from_a_connection_replaces_its_first' panicked at crates/micold-daemon/src/attention.rs:67:9:
  the window's last report names the session it has in view
  thread 'attention::tests::a_session_is_in_view_while_any_report_names_it' panicked at crates/micold-daemon/src/attention.rs:104:9:
  the first window has it in view
  test result: FAILED. 1 passed; 3 failed; 0 ignored; 0 measured; 78 filtered out
  ```
- U51 passed on the stub (it stores nothing, so it reads `None`); killed by a mutant (below).
- green: `Views { views: HashMap<ClientId, WindowView> }` with `set_view` (clears `in_view` when
  `focused` is false), `is_in_view`, `remove`. `scripts/build-lock.sh cargo test -p micold-daemon --lib attention::`
  (run2.log, `DLIB_RC=0`)
  ```
  test attention::tests::a_removed_connection_s_report_is_forgotten ... ok
  test attention::tests::a_second_report_from_a_connection_replaces_its_first ... ok
  test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 78 filtered out
  ```
- mutants (run3.log): U51 `let in_view = if view.focused { view.in_view } else { None };` ->
  `let in_view = view.in_view;` -> `MUTANT U51 KILLED`.
- refactor: none.

## Cycle 5 — U69 to U79, A3 — T005, T011, T012, T013

- tests: `crates/micold-daemon/tests/attention_events.rs::{a_change_into_awaiting_input_in_view_nowhere_adds_one_for_every_window (U69),
  a_change_while_one_window_has_the_session_in_view_adds_nothing (U70),
  a_repeated_waiting_signal_adds_nothing (U71), working_and_awaiting_input_again_adds_one_more (U72),
  three_sessions_changing_at_once_each_add_one_to_their_own_sequence (U73),
  a_change_with_no_connection_still_adds_one (U74),
  after_the_viewing_connection_closes_the_next_change_adds_one (U75),
  a_view_report_from_a_connection_attached_to_no_project_is_accepted_without_a_reply (U76),
  the_sequence_survives_a_restart_of_the_service_on_the_same_store (U77),
  a_removed_session_is_in_no_later_catalog (U78),
  a_session_that_ends_adds_nothing_to_its_sequence (U79),
  a_change_while_the_only_viewer_lost_focus_adds_one (A3)}` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test attention_events` (run1.log, `DEV_RC=101`),
  against the stub that counts nothing
  ```
  thread 'a_change_with_no_connection_still_adds_one' panicked at crates/micold-daemon/tests/attention_events.rs:450:5:
    left: 0
   right: 1
  thread 'a_repeated_waiting_signal_adds_nothing' panicked at crates/micold-daemon/tests/attention_events.rs:366:5:
    left: 0
   right: 1
  thread 'a_removed_session_is_in_no_later_catalog' panicked at crates/micold-daemon/tests/attention_events.rs:538:5:
  assertion `left == right` failed: precondition: the session has an attention event
  test result: FAILED. 3 passed; 9 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- the 3 that passed on the stub expect nothing to be added or no reply: U70 (adds nothing), U76, U79.
  U71 and U78 failed on their precondition asserts (`the first wait counted`, `the session has an
  attention event`), not on the behaviour they name, since the stub counts nothing; both were
  mutated against the green. U77 is killed by its red: the stub persisted nothing.
- U78 is a characterization: removal is existing behaviour (`sessions_for` filters archived sessions);
  it passed with the green and no change was made for it.
- green: `Catalog::mark_attention`, the began-waiting check and `Views` held in `state.rs`,
  `ClientMsg::WindowView` handled in `server.rs` with no reply. `scripts/build-lock.sh cargo test -p micold-daemon --test attention_events`
  (run2.log, `DEV_RC=0`)
  ```
  test a_repeated_waiting_signal_adds_nothing ... ok
  test a_removed_session_is_in_no_later_catalog ... ok
  test the_sequence_survives_a_restart_of_the_service_on_the_same_store ... ok
  test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- mutants (run3.log, all `KILLED`): U70 and U76 `state.set_window_view(id, WindowView { focused, in_view })`
  replaced by a no-op in server.rs (`MUTANT U70_U76 KILLED`); U70 `if began_waiting && !inner.views.is_in_view(session)`
  -> `if began_waiting` (`MUTANT U70b KILLED`); U71 `began_waiting` no longer requires `before != AwaitingInput` (`MUTANT U71 KILLED`);
  U75 `inner.views.remove(id)` -> `let _ = id;` (`MUTANT U75 KILLED`); U79 `began_waiting` replaced by `changed`
  (`MUTANT U79 KILLED`).

## Cycle 6 — U111 to U116, U176 to U178, A9, A10 — T006, T014, T015, T123

- tests: `crates/micold-client/tests/features_attention.rs::{the_first_report_of_a_connection_is_sent_also_when_nothing_is_in_view (U111),
  a_report_is_sent_again_only_when_the_derived_value_differs_from_the_last_one_sent (U112),
  losing_focus_reports_an_unfocused_window_with_nothing_in_view (U113),
  opening_settings_reports_nothing_in_view_and_leaving_it_reports_the_session_again (U114),
  a_reconnect_forgets_what_was_sent_so_the_next_report_goes_out_whatever_its_value (U115)}`;
  `crates/micold-client/tests/attention_view_report.rs::{view_facts_names_the_active_projects_selected_session_and_passes_focus_through (U176),
  view_facts_has_the_main_area_taken_while_settings_is_open (U177),
  showing_another_tab_of_the_selected_session_leaves_view_facts_equal (U178),
  with_settings_filling_the_main_area_the_window_reports_nothing_in_view (A9),
  a_selected_session_stays_reported_in_view_whichever_of_its_tabs_is_shown (A10)}`;
  `crates/micold-client/src/catalog_sync.rs::tests::a_catalog_snapshots_attention_seq_reaches_the_clients_session (U116)` (new)
- red: run3.log `== RED` (`RED_RC=101`), against the stubs `view_report` returning `None` and `view_facts`
  returning an empty `ViewFacts`
  ```
  thread 'the_first_report_of_a_connection_is_sent_also_when_nothing_is_in_view' panicked at crates/micold-client/tests/features_attention.rs:36:5:
    left: None
   right: Some(WindowView { focused: true, in_view: None })
  thread 'view_facts_has_the_main_area_taken_while_settings_is_open' panicked at crates/micold-client/tests/attention_view_report.rs:88:5:
    left: ViewFacts { window_focused: true, main_area_taken: false, selected: None }
  test result: FAILED. 0 passed; 5 failed (attention_view_report); 0 passed; 5 failed (features_attention)
  ```
- green: `features::attention::{State, view_report}`, `State::view_facts`, `attention_seq` copied in
  `reconcile_catalog`, and `main.rs` sending the report after each update. run3.log `== GREEN`
  (`GREEN_RC=0`, `GREENLIB_RC=0`)
  ```
  test with_settings_filling_the_main_area_the_window_reports_nothing_in_view ... ok
  test a_reconnect_forgets_what_was_sent_so_the_next_report_goes_out_whatever_its_value ... ok
  test catalog_sync::tests::a_catalog_snapshots_attention_seq_reaches_the_clients_session ... ok
  test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out  (x2)
  ```
- mutants (all KILLED): `view_report` sends without comparing (`features_attention`), `connection_started` keeps what was sent (`features_attention`), `view_facts` with `main_area_taken: false` and with `selected: None` (`attention_view_report`), `reconcile_catalog` without the copy on the existing and on the restored branch (`catalog_sync`).
- refactor: none.

## Cycle 7 — U13, U18–U31 — T017, T018, T019, T024, T025, T026

- tests: `crates/micold-core/src/attention.rs::tests::{u18_a_session_not_seen_before_is_adopted_with_no_claim_even_when_awaiting (U18),
  u19_a_higher_sequence_in_live_phase_is_claimed (U19), u20_the_same_sequence_again_is_not_claimed (U20),
  u21_reconnected_claims_a_change_into_awaiting_that_is_not_in_view (U21),
  u22_reconnected_adopts_when_the_last_seen_activity_was_awaiting (U22),
  u23_reconnected_does_not_claim_a_session_in_view (U23),
  u24_reconnected_does_not_claim_a_session_no_longer_awaiting (U24),
  u25_a_lower_sequence_is_adopted_with_no_claim (U25),
  u26_ten_sessions_with_higher_sequences_give_ten_claims (U26),
  u27_a_session_absent_from_the_snapshot_is_dropped (U27), u28_… to u31_… (U28–U31)}`,
  `protocol::messages::attention_wire_tests::a_claim_and_its_grant_encode_and_decode (U13)`,
  `tests/schema_hash.rs::{the_claim_and_the_grant_are_in_the_hashed_source,
  the_wire_changes_for_this_feature_cost_exactly_one_version_bump (23)}` (new / constant moved)
- red: `mise run test-core > red.log`, against stubs (`observe` returns no claims, `notification_text`
  returns empty strings; the two message variants existed so the tests compile)
  ```
  thread 'attention::tests::u19_a_higher_sequence_in_live_phase_is_claimed' panicked at crates/micold-core/src/attention.rs:263:9:
  test result: FAILED. 274 passed; 12 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- the 12 failures are U18–U22, U25–U31. U23 and U24 pass on the stub (they expect no claim), so each
  was checked against the green by reading the rule: dropping `in_view != Some(id)` or `now.awaiting`
  from the Reconnected arm fails them. U13 passed on the stub and `PROTOCOL_VERSION` was still 21:
  the lib failures stopped cargo before the `schema_hash` binary ran, so the version-23 and hashed-source
  assertions were not seen red.
- green: `mise run test-core` — 286 lib tests pass, all other binaries pass; clippy `-D warnings` and
  `cargo check --workspace --all-targets` clean (no exhaustive match needed an arm).
- refactor: `observe` rebuilds `seen` from the snapshot, which is what drops absent sessions (U27);
  `docs/daemon.md` names version 23.

## Cycle 8 — U54–U58, U80–U83, A1, A8, A12 — T020, T021, T027, T028

- tests: `crates/micold-daemon/src/attention.rs::tests::{the_first_claim_of_a_sequence_is_granted (U54),
  the_same_sequence_is_not_granted_again (U55), a_sequence_above_the_current_one_is_not_granted (U56),
  a_later_sequence_of_the_same_session_is_granted (U57),
  a_grant_for_one_session_does_not_use_up_another_s (U58)}`;
  `crates/micold-daemon/tests/attention_claims.rs::{two_windows_claim_one_sequence_and_only_the_first_is_granted (U80, A1, A8),
  a_claim_for_an_unknown_session_is_not_answered (U81),
  a_claim_above_the_current_sequence_is_not_answered (W1.4),
  ten_sessions_with_one_event_each_give_ten_grants (U82, A12),
  a_claim_is_no_operation_and_needs_no_attachment (U83)}` (new)
- red: `cargo test -p micold-daemon --lib attention` and `--test attention_claims` against stubs
  (`grant` returns false; no `AttentionClaim` arm in `server.rs`)
  ```
  test result: FAILED. 4 passed; 5 failed (lib: U54–U58)
  test result: FAILED. 2 passed; 3 failed (claims: U80, U82, U83)
  ```
- the two integration tests that pass on the stub expect silence (unknown session, sequence too high);
  they would fail if the handler granted unconditionally, which the grant tests U56 and U81's
  unknown-session lookup guard in the green.
- green: lib `attention` 9 pass, `attention_claims` 5 pass, M1's `attention_events` 13 pass; clippy
  `-D warnings` and `cargo fmt` clean.
- refactor: `Catalog::attention_seq` reads the session's current sequence for `SharedState::claim_attention`
  (state.rs), which `server.rs` calls for `ClientMsg::AttentionClaim`; the lock is dropped before the send.

## Cycle 9 — U117 to U124, A1, A4, A7, A11, A13 — T022, T030, T033

- tests: `crates/micold-client/tests/attention_notify.rs::{a_snapshot_with_a_higher_sequence_yields_one_claim (U117),
  the_first_snapshot_yields_no_claim_and_nothing_is_shown (U118, A13),
  without_a_grant_nothing_is_shown (U120),
  a_grant_shows_one_notification_named_as_the_sidebar_names_the_session (U119, A1),
  a_worktree_with_no_rename_is_named_by_its_derived_name (U119),
  a_session_of_the_default_entry_is_named_by_the_default_entrys_name (A7),
  a_session_of_a_project_that_is_not_the_active_one_is_named_by_its_own_project (U124, A4),
  a_failure_to_show_is_logged_once_per_run_and_pushes_no_notice (U121, U122),
  the_first_snapshot_after_a_reconnect_is_observed_as_a_reconnect (U123, A11)}` (new), driven through
  the root (`State::attention_on_welcome`, `attention_on_catalog_changed`, `attention_granted`) with a
  recording `DesktopNotifier`
- red: `scripts/build-lock.sh cargo test -p micold-client --test attention_notify > c9-red.log`, against
  stubs (`snapshot_claims` returns no messages, `show_granted` calls nothing and returns `None`; the seam
  types and the root methods existed so the tests compile)
  ```
  thread 'a_snapshot_with_a_higher_sequence_yields_one_claim' panicked at crates/micold-client/tests/attention_notify.rs:130:5:
  test result: FAILED. 1 passed; 8 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- the one pass is U118/A13, which expects no claim and no `show`; it holds on the stub by construction,
  and is kept because the green must keep it (the tracker adopts on the first snapshot).
- green: the same command with `--test features_attention --test feature_registration_cost
  --test no_concrete_implementations`: 9 + 5 + 7 + 14 pass.
- wiring (T033), not covered by a test of its own: `daemon_sync::claim_attention_events` sends the
  claims of the `Welcome` catalog (after the first view report) and of every `CatalogChanged`; the
  `AttentionGranted` arm calls `attention_granted` with `Capabilities::notifier()` and writes the
  returned line with `log_line`.
- refactor: none beyond `cargo fmt`.

## Cycle 10 — U159, U160 — T023, T029, T031, T032

- tests: `crates/micold-client/src/shell/desktop_notify/linux.rs::tests::{notify_request_carries_the_app_name_the_text_and_the_desktop_entry_and_nothing_else (U159),
  no_session_bus_is_no_notification_service (U160),
  nobody_serving_the_interface_is_no_notification_service (U160),
  any_other_bus_failure_is_a_refusal_with_the_buses_reason (U160)}` (new), on the pure `notify_request`
  and `notify_error`; no bus is needed
- red: **not recorded.** The worker that ran this cycle was cut off by an API error before it wrote
  this entry, and its logs went with its scratch directory. The tests and the implementation were
  both in the tree, uncommitted, when the unit resumed; whether the tests were seen failing against a
  stub is not known.
- green: `mise run gate` on the committed tree (the four tests run in the `micold-ai-ide` binary's
  unit tests).
- T029: `zbus` 5.19 with `async-io` and `blocking-api` (the call in `Notifier::show` is the blocking
  one), Linux-only in `crates/micold-client/Cargo.toml`; `Cargo.lock` gains the one dependency edge and
  no crate.
- T031: `desktop_notify::system()` has two `cfg` arms, Linux and not-Linux (`Unsupported`), which
  covers macOS and Windows until T041.
- the `Notify` call itself (`Notifier::show`) is not covered by a test: it needs a session bus.
  Quickstart §B covers it.
- refactor: none beyond `cargo fmt`.

## Notes and deviations

- Cycle 1's test file was written while the baseline run was building, so that run is both the
  baseline and cycle 1's red. No implementation existed then: `in_view` was the stub.
- Cycles 2 and 3: the red was observed (above). The green and the tests and stubs of cycles 4 and 5
  were written by the second M1 unit, which reached its context cap before any build ran. See the
  ledger's *Handover*.
- Cycle 6: U111 to U115 live in `tests/features_attention.rs` (the feature isolation gate requires that
  file), not in `tests/attention_view_report.rs` as T006 words it. Cycle 6's red and green ran in one
  build-lock run, the green applied by a script after the red.
- Cycle 5: U78 is a characterization of existing behaviour, not a new one; U77 is killed by its red.
- Cycle 9: the notifier is held in `Capabilities` (`shell/capabilities.rs`, the single assembly point
  `no_concrete_implementations` guards), which `App` holds, rather than as a field of `App` itself.
  The failure line is logged with the client's `log_line`, not `tracing::warn!`: the client has no
  `tracing` dependency or subscriber. `AttentionTracker` gained `Clone, PartialEq, Eq` so that
  `app::State` keeps its derives. `shell/desktop_notify/mod.rs` arrived here with only the
  `Unsupported` notifier, so the wiring compiles; cycle 10 adds the Linux arm.

## Review A round 1 fixes (M1)

- F1 (MAJOR): `Catalog::mark_attention` no longer writes the store; `note_activity` counts in
  memory and sets `attention_unsaved` under the lock, and the supervisor tick writes it in a
  `spawn_blocking` hop (`DaemonState::persist_attention`), as names are written (feature 029).
  U77 now calls `persist_attention` before the restart, standing in for the tick.
- F2 (MINOR): `release_attachments` forgets the connection's view. Test
  `after_the_viewing_connection_is_released_the_next_change_adds_one`, written after the fix (a
  review fix); its mutant (drop the `views.remove`) is KILLED.
- Mutant `server.rs` tick does not call `persist_attention`: SURVIVED. The tick is glue with no
  test harness in this crate (the names write beside it has none either). A known gap: no test
  observes the tick's write; U77 observes `persist_attention` itself.

## Review A round 1 fixes (M2)

- F1 (MAJOR): the notification is shown off the update thread. The `AttentionGranted` arm builds
  the notification (`State::attention_notification`) and returns a `Task` that calls
  `notifier.show` in `spawn_blocking` (`daemon_sync::show_attention_notification`); the result
  comes back as `ConnectionMsg::AttentionShown`, whose handler (`daemon_sync::on_attention_shown`)
  calls `State::attention_shown` and logs the line. The log-once rule moved into
  `features::attention::show_result`; `show_granted` is `show_result` over a call on this thread.
  The variant is in the connection's vocabulary and not at the root, which
  `root_vocabulary_is_cross_cutting` holds at 12 wrappers and 5 variants; `connection` naming
  `attention::NotifyError` is one new entry in `ALLOWED_CROSS_FEATURE_NAMES`
  (`tests/feature_registration_cost.rs`), without which that gate fails.
  - Tests (`tests/attention_notify.rs`): `a_reported_failure_to_show_is_logged_once_per_run`,
    `a_reported_success_logs_nothing_and_leaves_the_first_failure_to_be_logged`.
  - Red: `error[E0599]: no method named 'attention_shown' found for struct
    'micold_client::app::State'`. Green: `attention_notify` 11 passed.
  - The bus connection is built with `method_timeout` of 2 s. `zbus` reports a call past it as
    `Error::InputOutput` of kind `TimedOut`, which `notify_error` already mapped to `NoService`;
    `a_call_that_timed_out_is_no_notification_service` characterizes that and was green when
    written.
  - **No test**: the off-thread `Task` wiring (`show_attention_notification`, the `AttentionShown`
    arm) and the timeout itself (that a hung service ends the call after 2 s). Both need a running
    iced runtime or a session bus with a silent service.
- F2 (MAJOR): the body is escaped as markup (`&`, `<`, `>`); the summary is left as plain text.
  - Test: `the_body_is_escaped_as_markup_and_the_summary_is_left_as_plain_text`.
  - Red: `left: "R&D — <x>"`, `right: "R&amp;D — &lt;x&gt;"`. Green: `desktop_notify` 7 passed.
- F3 (MINOR): a call that fails with a connection error (`is_connection_error`: `Address`,
  `InputOutput`, `Handshake`, `Connection`) drops the kept connection, so the next `show` opens a
  new one. A timeout is one of them. Test
  `a_broken_bus_is_a_connection_error_and_a_services_answer_is_not`, written with the fix, covers
  the predicate. **No test** observes the drop and reopen: it needs a session bus.
- F4 (MINOR): `Views::forget_session`, called by `DaemonState::remove_session` under the lock.
  - Test: `after_a_session_is_forgotten_the_same_sequence_is_granted_again`.
  - Red: `error[E0599]: no method named 'forget_session' found for struct 'attention::Views'`.
    Green: `micold-daemon --lib attention` 10 passed.
  - **No test** observes that `remove_session` makes the call.
