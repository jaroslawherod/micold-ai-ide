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

## Cycle 11 — U46, U47, U48, U49 — T037, T042

- tests: `crates/micold-core/tests/notification_registers_nothing.rs` (new):
  `the_start_menu_shortcut_carries_the_id_the_client_uses` (U46),
  `the_packaging_registers_no_activator_and_no_protocol_handler` (U47),
  `the_client_reads_no_command_line_argument` (U48),
  `the_daemon_depends_on_no_notification_crate` (U49), `the_scan_reads_what_it_claims_to` (guard)
- red (U46), before the installer's line was added:
  `packaging/windows/micold-ai-ide.iss:67: the Start-menu shortcut does not carry
  'AppUserModelID: "MicoldAiIde.Client"'`
- green: `AppUserModelID: "MicoldAiIde.Client"` on the `{autoprograms}` shortcut (T042), not on the
  desktop one. `notification_registers_nothing`: 5 passed.
- U47 to U49 hold against today's sources and were green when written. Each rule was broken once on
  purpose and the break edited back out:
  - U47: `<key>CFBundleURLTypes</key>` added to `packaging/macos/Info.plist.in` —
    `packaging/macos/Info.plist.in:29: names 'CFBundleURLTypes'`
  - U48: `let _ = std::env::args();` added to `crates/micold-client/src/main.rs` —
    `crates/micold-client/src/main.rs:382: names 'env::args'`
  - U49: `zbus = "5"` added to `crates/micold-daemon/Cargo.toml` —
    `crates/micold-daemon/Cargo.toml:27: names 'zbus'`
- refactor: none beyond `cargo fmt`.

## Cycle 12 — U167, U168, U172, U173 — T122, T035, T036, T038, T039, T040, T041

- tests: `crates/micold-client/src/shell/desktop_notify/macos.rs::tests` (10, U167 and U168) on the
  pure `banner`, `notify_error`, `authorised` and `outcome`; `windows.rs::tests` (5, U172, U173, and the
  client's half of U46) on the pure `toast_text` and `notify_error`. Neither needs a bundle or shows
  a toast.
- The two modules compile on their own system only, so the red phase cannot be seen on the
  development host. T122 (the CI step `cargo test -p micold-client --bin micold-ai-ide
  desktop_notify` on the macOS and Windows legs) and T038 (the two crates) were pushed with the
  tests and with `todo!()` bodies, before T039 and T040. From Linux, `cargo check -p micold-client
  --bin micold-ai-ide --tests` passed for `aarch64-apple-darwin` and `x86_64-pc-windows-msvc` on that
  tree, so the red is the tests' and not the compiler's.
- red, on CI, pull request #557 at `022a6b60` (run
  <https://github.com/jaroslawherod/micold-ai-ide/actions/runs/37135332627>), step
  "Test (desktop notification backends)":
  - macOS (job 111238622228): `test result: FAILED. 0 passed; 10 failed`, each with
    `not yet implemented: T039` — all ten tests of `shell::desktop_notify::macos::tests`.
  - Windows (job 111238622255): `test result: FAILED. 1 passed; 4 failed`, each failure with
    `not yet implemented: T040` — `the_toast_carries_the_title_and_the_body_as_its_first_line`,
    `names_are_passed_as_written_because_the_crate_sets_them_as_inner_text`,
    `a_failure_of_the_system_is_a_refusal_with_its_reason`,
    `a_failure_to_read_a_file_is_a_refusal_with_its_reason`. The one that passed is
    `the_application_identity_is_the_one_the_installer_puts_on_its_shortcut`: the constant was
    written with the tests, because the installer's scan test (U46, cycle 11) reads it.
  - Every other step of both legs passed, `attention_notify` in the enumerated list among them.
- green: T039 — `banner`, `notify_error` (`NoBundleIdentifier` is `NoService`, anything else
  `Refused`), `authorised` (`Ok(false)` is `Refused`), `outcome`; `Notifier::show` asks for
  authorisation and sends on a thread of its own and waits 2 s for the answer. T040 —
  `toast_text`, `notify_error` (`Refused`), `Notifier::show` through
  `Toast::new(APP_USER_MODEL_ID)`. T041 — `system()` has one arm per system and the `Unsupported`
  notifier is gone. The green run is recorded under T118 below.
- T043: from Linux, `cargo clippy -p micold-client --bin micold-ai-ide --tests -- -D warnings`
  passed for `--target aarch64-apple-darwin` and for `--target x86_64-pc-windows-msvc`.
- **No test**: the calls into the system (`deliver` and `Notifier::show` on macOS, `Notifier::show`
  on Windows). They need a signed bundle and an installed build; quickstart §C1 and §C2 cover them.
- Decisions made here:
  - macOS sends with the crate's `blocking::send`, not `Notification::send_blocking`: the second
    waits on the main run loop for a response handle, which this slice does not use, and fails with
    `MainThreadNotRunning` when the loop is busy.
  - `show` does not wait for a person. The first request for authorisation returns when the user
    answers the system's prompt; `show` runs on the runtime's blocking pool, which the runtime
    waits for at exit. So the request and the send run on a thread of their own, `show` waits 2 s,
    and an unanswered prompt is reported as a refusal (`outcome(None)`). The notification is shown
    once the user allows it.
  - Authorisation is requested on every `show`: after the first answer the system returns the
    stored one at once, and a permission changed in System Settings takes effect without a restart.
- refactor: none beyond `cargo fmt`.
- green on CI (T118), pull request #557 at `cc0d815e` (run
  <https://github.com/jaroslawherod/micold-ai-ide/actions/runs/37137101271>), all three
  `build + test` legs and `ci complete` passed:
  - macOS (job 111243764629): step "Test (desktop notification backends)" `test result: ok. 10
    passed; 0 failed`; `attention_events` and `attention_claims` in "Test (daemon)";
    `attention_view_report` and `attention_notify` in the enumerated client list.
  - Windows (job 111243764632): the same step `test result: ok. 5 passed; 0 failed`;
    `attention_events` and `attention_claims` in "Test (daemon, Windows)"; the two client tests in
    the enumerated list.
  - Linux (job 111243764658): "Test (full workspace)" ran `attention_events`, `attention_claims`,
    `attention_view_report` and `attention_notify`; the `linux.rs` backend tests run there too.
  Story 1's outer tests (A1 to A13) are green on the three systems.

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

These M4 cycles were run on the prep branch `feat/notify-session-needs-attention-m4-prep` at base d4ea296f and cherry-picked after M3 merged; their M4-n tags are kept for the cross-references inside them.

## Cycle 13 — U9, U10, U11, U175 — T045, T052 (M4-1)

- Tests (`crates/micold-core/src/store.rs`, `mod unread_tests`):
  `a_session_stored_without_unread_reads_as_read`, `a_store_round_trip_keeps_unread`,
  `unread_is_written_to_the_sessions_state_file_and_to_no_other_file`; and
  `crates/micold-core/tests/store_fault_isolation.rs`:
  `a_state_file_that_cannot_be_parsed_leaves_no_session_unread_and_reports_nothing_new` (U175,
  characterization: it needs the field to compile and was green as soon as it did).
- Red (`mise run test-core`): `error[E0609]: no field 'unread' on type 'session::Session'`
  (`store.rs`, three places).
- Green: `Session::unread`, `StoredSession::unread` with `#[serde(default)]`. `mise run test-core`
  exit 0, the four tests `ok`.
- **Deviation from T045's wording**: "written to the catalog file and to no other file". Sessions
  are not in the catalog file (`projects.json`) since the per-project split; `StoredSession` is
  written to the project's state file, where `attention_seq` is. The test asserts that the only
  file under the store directory naming `"unread"` is `project_state_path(project)`.

## Cycle 14 — U14 — T046, T053 (field only; the version number is the last commit) (M4-2)

- Tests: `protocol::messages::attention_wire_tests::a_session_summary_carries_unread`;
  `crates/micold-core/tests/schema_hash.rs`: `unread_is_in_the_hashed_source`.
- Red: `error[E0560]: struct 'messages::SessionSummary' has no field named 'unread'`.
- Green: `SessionSummary::unread`; every `SessionSummary` literal of the workspace gains
  `unread: false` (the service's `session_summary` too, until cycle M4-4 makes it carry the
  session's value). `mise run test-core` exit 0.
- `PROTOCOL_VERSION` is **not** bumped in this cycle. `SCHEMA_HASH` is computed by `build.rs`, so
  it has changed with the field. The bump, its doc comment and the pin in `schema_hash.rs` are one
  commit at the end of the branch, because the number is "next free" after M3 merges.

## Cycle 15 — U59, U60 — T047, T055 (the `set_view` part) (M4-3)

- Tests (`crates/micold-daemon/src/attention.rs`): `a_report_returns_the_session_that_came_into_view`,
  `a_report_naming_the_same_session_or_none_returns_nothing`.
- Red (`cargo test -p micold-daemon --lib attention`): `error[E0308]: mismatched types`, five
  places (`set_view` returned `()`).
- Green: `Views::set_view` returns `Option<SessionId>`. `--lib attention` 12 passed. T004's tests
  keep their assertions and ignore the return value.

## Cycle 16 — U84 to U96, A14, A15, A19 to A22, A24 to A26, A31 to A36 — T048, T054, T055 (M4-4)

- Tests: `crates/micold-daemon/tests/unread_state.rs` (new), 14 tests, harness copied from
  `attention_events.rs`.
- Red: `test result: FAILED. 2 passed; 12 failed`. The two that passed assert that a session is
  *not* unread (`a_change_while_a_window_has_the_session_in_view_does_not_set_unread`,
  `a_restart_of_the_service_keeps_a_read_session_read`); they hold trivially before the feature
  and constrain it afterwards.
- Green: `Catalog::mark_attention` sets `unread`; `Catalog::mark_read`; `session_summary` carries
  it; `DaemonState::set_window_view` reads the session and broadcasts.
  `unread_state` 14 passed, `attention_events` 13 passed, `attention_claims` 5 passed.
- **Deviation from T054's wording** ("`mark_read(session)` clears it and persists"): `mark_read`
  changes memory only and `set_window_view` sets `attention_unsaved`, so the supervisor tick's
  `persist_attention` writes it, as M1 did for `mark_attention` (the state lock is held on the
  async runtime, and a store write is blocking I/O). The restart tests call `persist_attention`
  as U77 does.
- **No test** observes the tick's write of a read (the M1 gap for the attention event, unchanged).

## Cycle 17 — U125, U126, U127, A14, A36 — T051, T056 (M4-5)

- Tests: `crates/micold-client/tests/unread_rows.rs` (new), 5 tests.
- Red: the client crate did not compile (cycle M4-6's errors came first in the same run);
  `row_unread` and `in_view` did not exist in `features::attention`.
- Green: `features::attention::row_unread(&Session, Option<SessionId>)`,
  `features::attention::in_view(&State)` (the session of the last view report sent), `unread`
  copied in both arms of `reconcile_catalog`. `unread_rows` 5 passed.
- `--test unread_rows` in `.github/workflows/ci.yml` (T051's last sentence) was left out on the
  prep branch, because M3 owned the workflow files then. The M4 unit added the line after the
  cherry-pick, beside `--test attention_view_report`.

## Cycle 18 — U145 to U151 — T049, T050, T057, T058 (M4-6)

- Tests: `ui/material/unread_mark.rs` (3), `ui/material/tree_view.rs` (4, the file's first test
  module), `composition_contrast.rs::the_unread_mark_is_legible_on_every_fill_a_host_draws_it_on`,
  `anatomy_size.rs::an_unread_mark_is_8dp_in_both_axes` and
  `an_unread_tree_row_stands_at_its_densitys_height`.
- Red (`cargo test -p micold-client --lib --test unread_rows`): 19 compile errors, among them
  `cannot find 'UnreadMark' in 'super'`, `cannot find function 'fill' in module
  'super::unread_mark'`, `no method named 'unread' found for struct 'tree_view::TreeItem'`.
- Green: `UnreadMark::new(roles)` with `.count(n)` and `.worded(bool)`; `TreeItem::unread(bool)`.
  `cargo test -p micold-client` exit 0, 152 result lines `ok`, none failed.
- Design choices the M4 unit and M5 should know:
  - The mark is a `container` 8dp square with the `full` corner, filled by
    `unread_mark::fill(roles)` (`primary`), not a glyph: a glyph's dot is smaller than its em box.
  - The count and the word are drawn in `TypeRole::Label` in the theme's default text colour.
    Contract U1 says "the host's label role and colour"; no M4 host shows text, so no setter for
    the role exists yet. M5 adds it with its hosts.
  - The emphasised weight of an unread row is the view's `selected_label_role` (the sidebar's
    `SidebarSessionCurrent`); a view that sets none draws the label as the others. No new setter.
  - The contrast test covers four fills: the sidebar, a selected row (`secondary_container`), a
    menu panel (`surface_container`) and the app bar (`surface`).

## Cycle 19 — T059, T060, T061 (no behavior id)

- T059: `ui/sidebar.rs::session_tree_item` takes the session in view and calls
  `.unread(row_unread(session, in_view))`.
- T060: catalogue entry `material/unread_mark.rs` / `UnreadMark`, rendered by
  `sections::atoms::unread_mark`: a session row with the mark above one without. The showcase
  gates (`showcase_completeness`, `showcase_captions`, `material_builder_api`) passed in the run
  above. **No visual pass was run** on this branch.
- T061: `docs/user-guide/worktrees-and-sessions.md`, new section *Unread sessions*, placed before
  *Being told when a session needs you* so that it does not touch the lines M3 edits.

## Cycle 20 — Protocol version (T046's pin, T053's bump) — last commit

- `PROTOCOL_VERSION` 23 → 24 in `version.rs`, `FEATURE_026_PROTOCOL_VERSION` 24 in
  `schema_hash.rs`, `docs/daemon.md` "version 24 today". One commit, the last code commit of the
  branch. If M3 or another feature takes 24 first, this commit is the only one that moves.

## Cycle 21 — M4 unit, after the cherry-pick: a read reaches the store (review A F1, review B F1)

- No M3 commit conflicted with the 13 prep commits; `PROTOCOL_VERSION` 24 was still free.
- Test `a_read_is_written_by_persist_attention` in `crates/micold-daemon/tests/unread_state.rs`: the
  store holds `unread: true` after an attention event and `persist_attention`, and `false` after
  the view report and a second `persist_attention`. It loads the store with `DaemonState::new`,
  not `Service::restarted()`, which writes by itself; the helper service runs no supervisor tick.
- The behaviour was there (cycle 16), so the red is a mutation: with `inner.attention_unsaved |= read;`
  in `set_window_view` replaced by `let _ = read;` the test fails at its last assertion ("the store
  holds the session as read after the write the tick makes"), and so does
  `a_restart_of_the_service_keeps_a_read_session_read`. Restored: `unread_state` 15 passed.
- Without a test: `unwind` in `server.rs` calls `persist_attention` before the stop (review A F1),
  and `protocol_roundtrip.rs` round-trips `unread: true` (review A F2).

## Cycle 22 — U36 to U41 — T062, T066 (M5-1)

- Tests: `crates/micold-core/src/workspace.rs`, the file's first test module, 6 tests.
- Red, first run: `scripts/build-lock.sh cargo test -p micold-core --lib workspace::tests` did not
  compile: `no method named 'unread_session_count' found for struct 'workspace::Workspace'`, and the
  same for `other_projects_unread`.
- Red, with both functions stubbed to return `0`: 4 failed, 2 passed.
  `the_unread_count_covers_the_default_entry_and_every_worktree`: `left: 0 right: 3`;
  `the_unread_count_leaves_out_the_session_in_view`: `left: 0 right: 1`;
  `the_other_projects_total_sums_every_project_but_the_active_one`: `left: 0 right: 3`;
  `after_a_switch_the_other_projects_total_counts_the_remaining_projects_only`: `left: 0 right: 1`.
- U38 and U40 assert a zero and passed against the stub, so they were checked with a mutant of the
  real code: with the `s.unread &&` filter and the `active` filter both removed,
  `the_unread_count_is_zero_for_a_project_with_no_unread_session` and
  `the_other_projects_total_leaves_out_the_active_project` fail (4 failed, 2 passed). Restored.
- Green: `Workspace::unread_session_count(&Path, Option<SessionId>)` and
  `Workspace::other_projects_unread(Option<&Path>)` beside `running_session_count`.
  `cargo test -p micold-core --all-targets` exit 0, 145 result lines `ok`.
- `other_projects_unread` takes `Option<&Path>`, not `&Path`: with no project active every project
  is another one. It sums over `Workspace::projects`, the rows the switcher lists.
- Refactor: none needed.

## Cycle 23 — U128 to U130, U152 to U158 — T063 to T065, T067 to T070 (M5-2)

- Tests: `crates/micold-client/tests/switcher_unread.rs` (new, 5 tests; `--test switcher_unread`
  added to `.github/workflows/ci.yml`); `ui/material/menu.rs` (3, the file's first test module);
  `menu_anatomy.rs::a_switcher_row_with_an_unread_count_keeps_its_height`; `ui/material/button.rs`
  (3, the file's first test module);
  `button_anatomy.rs::a_button_with_an_unread_count_keeps_its_height`; `unread_mark.rs` (2, the
  host's role and colour of contract U1).
- All of these were written before any implementation and run in one build. Red:
  `scripts/build-lock.sh cargo test -p micold-client --lib --test switcher_unread --no-run` did
  not compile: `no field 'unread_count' on type '&SwitcherEntry'`, `no method named
  'other_projects_unread' found for struct 'micold_client::app::State'`, `struct
  'material::menu::MenuItem<_>' has no field named 'trailing_mark'`, `no method named
  'trailing_mark' found for struct 'material::button::Button<'a, M>'`, `cannot find function
  'unread_total_tooltip'`, `no method named 'role'` / `'muted'` / `'tint'` `found for struct
  'unread_mark::UnreadMark<'a, M>'`. One error was the tests' own (`'node' does not live long
  enough` in a layout helper) and was fixed in the tests.
- **Deviation**: one red for ten behaviors, and a compile error, not an assertion. The same form
  as cycles 17 and 18. To show the assertions hold something, the green tree was committed and
  then mutated in one run (restored with `git checkout`): the menu's mark without `.worded(true)`,
  `Button::trailing_mark` keeping a count of zero, the tooltip's plural without its `s`,
  `switcher_entries` passing `None` as the session in view, and `State::other_projects_unread`
  passing `None` as the active project. 8 tests failed: `a_trailing_mark_renders_the_worded_count_after_the_trailing_text`,
  `with_no_running_count_the_mark_alone_trails`, `a_trailing_mark_of_zero_renders_the_button_without_it`,
  `the_unread_totals_tooltip_names_the_other_projects`, and four of `switcher_unread`'s five (all
  but `every_switcher_entry_carries_its_projects_unread_count`). **Not mutated**: the two height
  tests (U155, U157) and `a_trailing_mark_renders_the_count_after_the_label_inside_the_button`.
- Green: `scripts/build-lock.sh cargo test -p micold-client --lib --test switcher_unread`: lib 507
  passed, `switcher_unread` 5 passed.
  - `UnreadMark::role(TypeRole)`, `.muted()`, `.tint(Rgb)`: the host's role and colour for the
    number and the word (decision D23, note 2). The mark itself stays `primary`.
  - `MenuItem::trailing_mark: Option<usize>`: `● n unread`, muted like the running count, pushed
    after `trailing_text`; `None` and `Some(0)` push nothing, so no gap is taken.
  - `Button::trailing_mark(n, tooltip)`: `● n` in `TypeRole::Action` and the variant's content
    colour, 8dp after the label; the whole button is wrapped in `Tooltip` (no layout node). A
    count of zero keeps neither the mark nor the tooltip. `unread_total_tooltip(n)` is exported
    from `ui::material`.
  - `SwitcherEntry::unread_count`, from `Workspace::unread_session_count(path, in_view)` with
    `features::attention::in_view`; `State::other_projects_unread()`.
  - T070: `ui/mod.rs` passes `trailing_mark` when the count is one or more; `ui/toolbar.rs`
    calls `.trailing_mark(total, unread_total_tooltip(total))` when the total is one or more.

## Cycle 24 — T071, T072 (no behavior id)

- T071: `samples::PROJECTS` gained an unread count per project, so the showcase's project
  switcher (`MenuOverlay`'s second opener, `sections/floating.rs`) shows a row with a running and
  an unread count and a row with the unread count alone. **Deviation from the task's file list**:
  the row is not in `sections/atoms.rs`, because a menu's rows are built only inside its panel
  (`menu::item_column` is `pub(super)`). `sections::atoms::unread_mark` gained the switcher's
  button with a count of 3 and with none; the catalogue's `posed` lists name both.
  `showcase_completeness` 10, `showcase_captions` 7 and `material_builder_api` 35 passed.
- T072: `docs/user-guide/project-selection.md`, under *Switching projects from the top bar*: the
  unread count on each row and the total on the button, with a link to *Unread sessions*.

## Cycle 25 — M5 verify: both counts on one row (review A F1), and two red gates

- **Red gates, no behavior id.** The scoped gate, run for the first time in M5, failed three
  times, each for one cause: `tests/features_project.rs` built a `SwitcherEntry` without
  `unread_count` (E0063); `composite_call_sites` no longer found `row![icon(..), ..]` in
  `button.rs`, whose content row had become `row![]` with pushes (the leading slot is a literal
  row again); `ui_glyph_literals` found U+25CF in two assertion messages of `menu.rs` (reworded).
- **Red** (review A F1): `a_row_with_both_counts_keeps_its_label_on_one_line` in `menu.rs`:
  `the label is 40dp high in 58.32422dp of width; alone on its row it is 20dp high`. The running
  count (50.6dp) and the worded mark (59.1dp) stood side by side.
- **Green**: `item_column` stacks the two counts in one trailing column, ending at the same edge.
  U152's test now asserts the mark under the running count, both ending at the trailing inset,
  and the pair inside the item's height. `menu_anatomy`'s height gate is unchanged and green.
- Scoped gate green on this tree.
- **T119**: `unread_state` 15, `unread_rows` 5 and `switcher_unread` 5 passed; `mise run gate` green on
  `d98f1a71`. A14 to A22 and A24 to A36 are `DONE` (A23 is T121's).

## Cycle 26 — U32, U33, U34, U35, U15 (messages only) — T073, T074, T081, T082 (M6-1)

Cycles 26 to 30 were run by a prep unit on the branch `feat/notify-session-needs-attention-m6-prep`, in parallel with M5, and cherry-picked onto `origin/main` at `57f14c72` with no conflict.

- tests: `crates/micold-core/src/attention.rs::tests::{u32_a_known_available_project_holding_the_session_resolves_to_show,
  u33_a_session_that_was_removed_resolves_to_unavailable,
  u33_a_session_held_by_another_project_resolves_to_unavailable,
  u34_a_forgotten_project_resolves_to_unavailable,
  u35_a_project_whose_folder_is_unavailable_resolves_to_unavailable}`;
  `crates/micold-core/src/protocol/messages.rs::attention_tests::a_reveal_and_its_forward_encode_and_decode`;
  `crates/micold-core/tests/schema_hash.rs::the_reveal_and_its_forward_are_in_the_hashed_source` (all new)
- red: `cargo test -p micold-core --all-targets`, exit 101. No stub: the red is the compiler
  naming the missing items, so no test ran.
  ```
  error[E0425]: cannot find function `resolve_reveal` in this scope
     --> crates/micold-core/src/attention.rs:496:13
  error[E0433]: cannot find type `Reveal` in this scope
     --> crates/micold-core/src/attention.rs:509:13
  error: could not compile `micold-core` (lib test) due to 12 previous errors
  ```
- green: the same command, exit 0, every binary `ok` (40 result lines). `Reveal`, `resolve_reveal`,
  `ClientMsg::SessionReveal`, `DaemonMsg::RevealSession`.
- not in this cycle: the version number. `PROTOCOL_VERSION` stays 24 until the last commit of the
  branch (cycle 30), so the M6 unit can move the bump.
- commit: `c1353804` [c627c807]

## Cycle 27 — U61, U62, U63, U64, U65, U97, U98, U99, U100, U101 (A41 service half, A42) — T075, T076, T083 (M6-2)

- tests: `crates/micold-daemon/src/attention.rs::tests::{focus_order_puts_the_connection_that_last_reported_focus_last (U61),
  a_report_without_focus_does_not_move_a_connection_in_focus_order (U61),
  remove_takes_a_connection_out_of_focus_order (U62),
  reveal_target_is_the_connection_that_holds_the_project (U63),
  with_no_holder_reveal_target_is_the_last_of_focus_order (U64),
  with_no_holder_and_an_empty_focus_order_reveal_target_is_the_sender (U65)}`;
  `crates/micold-daemon/tests/session_reveal.rs` (new):
  `a_reveal_is_forwarded_to_the_window_attached_to_the_project_and_to_no_other` (U97, A42),
  `with_the_project_attached_nowhere_a_reveal_goes_to_the_window_that_last_reported_focus` (U98),
  `with_no_holder_and_no_focused_window_a_reveal_goes_back_to_the_sender` (U99),
  `a_reveal_is_forwarded_for_a_session_and_a_project_that_do_not_exist` (U100),
  `a_reveal_changes_no_session_no_attachment_and_nothing_stored` (U101, A41)
- red: `cargo test -p micold-daemon --lib attention; cargo test -p micold-daemon --test session_reveal`,
  against the stubs `focus_order` returning `&[]` and `reveal_target` returning the sender, and no
  handler for `SessionReveal` (the connection loop's catch-all took it).
  ```
  test attention::tests::focus_order_puts_the_connection_that_last_reported_focus_last ... FAILED
  test attention::tests::a_report_without_focus_does_not_move_a_connection_in_focus_order ... FAILED
  test attention::tests::remove_takes_a_connection_out_of_focus_order ... FAILED
  test attention::tests::reveal_target_is_the_connection_that_holds_the_project ... FAILED
  test attention::tests::with_no_holder_reveal_target_is_the_last_of_focus_order ... FAILED
  test result: FAILED. 13 passed; 5 failed; 0 ignored; 0 measured; 78 filtered out
  test result: FAILED. 0 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out   (session_reveal)
  ```
  U65's test passed against the stub, which returned the sender: it has no red of its own.
- green: `cargo test -p micold-daemon --lib attention` 18 passed; `--test session_reveal` 5 passed;
  `--test attention_events` 13 passed; `--test attention_claims` 5 passed.
- commit: `e09abf00` [427ff981]

## Cycle 28 — U131, U132, U133, U134, U135, U136, U137 (A37–A41, window half) — T077, T084 (M6-3)

- tests: `crates/micold-client/tests/attention_reveal.rs` (new):
  `an_activated_notification_yields_one_session_reveal` (U131),
  `a_reveal_for_a_background_project_reopens_it_then_selects_the_session` (U132, A38),
  `a_reveal_for_the_active_project_selects_the_session_alone` (U133, A37, A41),
  `a_reveal_for_a_session_that_is_gone_pushes_the_notice_and_changes_no_selection` (U134, A40),
  `a_reveal_for_a_forgotten_or_unavailable_project_pushes_the_notice` (U134),
  `the_session_shown_by_a_reveal_is_in_view_so_its_row_is_not_unread` (U135, A39),
  `off_wayland_the_window_is_unminimised_then_focused` (U136),
  `on_wayland_the_window_is_unminimised_then_asks_for_attention` (U137)
- red: `cargo test -p micold-client --test attention_reveal`, against the stubs `raise_plan`
  returning no step, `notifier_event` returning `ClientMsg::Goodbye` and `State::reveal_session`
  returning no message.
  ```
  test result: FAILED. 0 passed; 8 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- green: `cargo test -p micold-client --test attention_reveal --test attention_notify
  --test features_attention --test feature_registration_cost` -> 8, 11, 7 and 5 passed.
- as built: the feature has no `Msg`, so the reducer step is `reveal_steps(Reveal, active) ->
  Vec<RevealStep>` in `features/attention.rs`, and the root's `State::reveal_session` turns the
  steps into `Message::Project(Reopened)` and `Message::Session(Selected)` for the shell to
  dispatch, or pushes the notice (the shape D20 set for this feature). The function that turns
  a click into `SessionReveal` is `notifier_event`.
- commit: `431b5faa` [5d8970b5]

## Cycle 29 — U161, U162, U163, U164, U165 — T078, T085 (with the glue of T088, T089) (M6-4)

- tests: `crates/micold-client/src/shell/desktop_notify/linux.rs::tests::{
  the_request_offers_the_default_action_whatever_the_service_can_do (U161),
  notify_request_carries_the_app_name_the_text_and_the_desktop_entry_and_nothing_else (U159, now with the action),
  the_default_action_of_a_notification_in_the_table_is_an_activation_of_its_session (U162),
  a_click_is_reported_once_however_many_times_the_service_says_it (U162),
  an_id_the_table_does_not_hold_is_no_event (U163),
  another_action_key_is_no_event (U164),
  a_closed_notification_leaves_the_table (U165),
  the_two_signals_are_read_from_the_bus_messages_that_carry_them (U162, U165),
  any_other_signal_and_a_signal_with_another_body_is_not_read (U164)}`
- red: `cargo test -p micold-client --bin micold-ai-ide desktop_notify`, against the stubs
  `signal` and `Shown::on_signal` returning `None` and `notify_request` offering no action.
  ```
  test shell::desktop_notify::linux::tests::a_closed_notification_leaves_the_table ... FAILED
  test shell::desktop_notify::linux::tests::a_click_is_reported_once_however_many_times_the_service_says_it ... FAILED
  test shell::desktop_notify::linux::tests::notify_request_carries_the_app_name_the_text_and_the_desktop_entry_and_nothing_else ... FAILED
  test shell::desktop_notify::linux::tests::the_request_offers_the_default_action_whatever_the_service_can_do ... FAILED
  test shell::desktop_notify::linux::tests::the_default_action_of_a_notification_in_the_table_is_an_activation_of_its_session ... FAILED
  test shell::desktop_notify::linux::tests::the_two_signals_are_read_from_the_bus_messages_that_carry_them ... FAILED
  test result: FAILED. 9 passed; 6 failed; 0 ignored; 0 measured; 329 filtered out
  ```
  The tests for U163, U164 and the unread signal passed against the stubs, which answer `None`
  to everything: they have no red of their own.
- green: the same command -> 15 passed.
- glue, no test of its own (plan, *Constitution Check*): `Notifier::listen` (the thread that reads
  the service's signals), `desktop_notify::{events, clicks}` (the process's one channel and its
  subscription), `shell/window_raise.rs` (T088), and the wiring in `shell/subscriptions.rs`,
  `shell/connection.rs`, `shell/daemon_sync.rs` and `features/connection.rs`
  (`Msg::NotifierReported`) (T089). `main.rs` needed no edit.
- commit: `75d105b2` [9932d817]. User guide (T090): `e05df97a` [29f951ef].

## Cycle 30 — U15 (the number) — T074, T082 (M6-5)

- test: `crates/micold-core/tests/schema_hash.rs::the_wire_changes_for_this_feature_cost_exactly_one_version_bump`
  (the constant it compares with moved to 25)
- red: `cargo test -p micold-core --test schema_hash`
  ```
  test the_wire_changes_for_this_feature_cost_exactly_one_version_bump ... FAILED
    left: 24
   right: 25
  test result: FAILED. 14 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
  ```
- green: the same command -> 15 passed, with `PROTOCOL_VERSION` 25.
- commit: `b445020c` [0a7eba54], the last commit of the branch, alone: `version.rs`, `schema_hash.rs` (the constant and
  its comment) and the version sentence of `docs/daemon.md`. 25 is one more than `main` at
  `ea477587`; if M5 or another feature takes 25 first, this commit is the one to move.

- **Not done in the prep:**
  - T079, T086 (macOS) and T080, T087 (Windows): their tests run only on those systems, and the
    macOS click needs a decision that has to be tried on a Mac (see the return of the prep unit).
    `desktop_notify::system(events)` takes the channel on all three systems; the macOS and Windows
    arms drop it, so a click there does nothing yet.
  - T077's sentence about `.github/workflows/ci.yml`: `--test attention_reveal` is not in the
    enumerated list yet.
  - T120: `session_reveal` and `attention_reveal` are green here (cycles 27, 28); the task is ticked
    by the M6 unit.

## Cycle 31 — U169, U170, U171, U174 — T079, T080, T086, T087 (M6-6)

- tests: `crates/micold-client/src/shell/desktop_notify/macos.rs::tests` — six on the pure
  `Shown::on_response` (U169: the default action of a held id is `Activated`, reported once; U170: a
  dismissal, a timeout and another action are nothing; U171: an id the table does not hold is
  nothing) and one on `prompt_is_open` (the M3 review A follow-up, no behavior id);
  `windows.rs::tests` — four on `on_activated` (U174: `None` is `Activated` for the toast's own
  session; each toast reports its own; an argument is nothing; a closed channel is not an error).
- The modules compile on their own system only. The tests were pushed with `todo!()` bodies
  (`e18b5ebb`). From Linux, `cargo clippy -p micold-client --bin micold-ai-ide --tests -- -D warnings`
  passed for `aarch64-apple-darwin` and `x86_64-pc-windows-msvc` on that tree, so the red is the
  tests' and not the compiler's.
- red, on CI, pull request #560 at `e18b5ebb` (run
  <https://github.com/jaroslawherod/micold-ai-ide/actions/runs/37147017754>), step
  "Test (desktop notification backends)", the only failed step of both legs:
  - macOS (job 111272988044): `test result: FAILED. 10 passed; 7 failed`, the seven new tests, each
    with `not yet implemented: T086`.
  - Windows (job 111272988013): `test result: FAILED. 5 passed; 4 failed`, the four new tests, each
    with `not yet implemented: T087`.
  - Linux passed, `attention_reveal` in the workspace step.
- green: `47bb094f` (T086, T087; ledger D26, D27). The same clippy cross-check passed for both
  targets on it. **The green run on CI is not recorded yet**: `47bb094f` is not pushed (ledger
  *Handover*).
- **No test**: the calls into the system (`deliver` and `Notifier::show` on macOS, `Notifier::show`
  on Windows); quickstart §C1 and §C2 (M9).

## Cycle 32 — M6 verify: review A round 1, F1 to F4 (U33, U132, U134, U160; no new behavior id)

- tests, one per finding:
  - F1 `crates/micold-core/src/attention.rs::tests::u33_a_session_the_user_closed_resolves_to_unavailable`
  - F3 `crates/micold-client/tests/attention_reveal.rs::the_switch_a_reveal_asks_for_arrives_at_the_revealed_session`
  - F2 `crates/micold-client/src/shell/daemon_sync.rs::tests::a_reveal_for_a_project_whose_folder_has_gone_says_so_and_selects_nothing`
  - F4 `crates/micold-client/src/shell/desktop_notify/linux.rs::tests::a_call_that_timed_out_keeps_the_connection_and_a_broken_bus_loses_it`
    (written against a `connection_is_lost` that only called `is_connection_error`, so that it compiled)
- red, on the working tree before the fixes (`cargo test -p micold-core --lib attention::tests::u3`,
  `-p micold-client --test attention_reveal`, `-p micold-client --bin micold-ai-ide -- a_reveal_for desktop_notify`):
  - F1 `assertion left == right failed: a closed session has no row to select` (`7 passed; 1 failed`)
  - F3 `assertion left == right failed: the switch restores the revealed session, so it is the one session started` (`8 passed; 1 failed`)
  - F2 `daemon_sync.rs:2495: assertion left == right failed` (no notice was shown) and
    F4 `assertion failed: !connection_is_lost(&io(std::io::ErrorKind::TimedOut))` (`15 passed; 2 failed`)
- green, the same three commands: `8 passed`, `9 passed`, `17 passed`.
  - F1: `resolve_reveal` requires `!s.archived`.
  - F3: `State::reveal_session` makes the session the project's remembered foreground before `Reopened`.
  - F2: the `RevealSession` arm scans the folders (`refresh_availability`) before `reveal_session`.
  - F4: `Notifier::show` forgets the connection only for `connection_is_lost`, which a timeout is not.
- The tests and the fixes are in one commit: the red above is from the working tree, not from a commit.
- **No test**: that `Reopened` and `Selected` are dispatched in order by the `Task` chain; that the
  listening thread ends with a broken connection.
- **Mutants for the tests of cycles 27 and 29 that had no red** (run on `577992f0`, each restored
  with `git checkout`); all four are killed:
  - U65, `Views::reveal_target` with `.unwrap_or(sender + 1)`:
    `with_no_holder_and_an_empty_focus_order_reveal_target_is_the_sender ... FAILED`.
  - U163, `Shown::on_signal` taking any held entry for an id it does not hold:
    `an_id_the_table_does_not_hold_is_no_event ... FAILED`.
  - U164, `Shown::on_signal` without its `key == DEFAULT_ACTION` guard:
    `another_action_key_is_no_event ... FAILED`.
  - U164, `signal` reading `ActivationToken` as `ActionInvoked`:
    `any_other_signal_and_a_signal_with_another_body_is_not_read ... FAILED`.
  - Cycle 26's red is a compile error, recorded there as it is.
