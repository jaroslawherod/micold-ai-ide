# M6 prep cycles: a click on the notification opens the session

Written by the M6 prep unit on branch `feat/notify-session-needs-attention-m6-prep`, ahead of M5's
merge. The M6 unit moves these entries into `cycle-log.md`. Grouped per task pair, as the batching
note of `cycle-log.md` says. Commands ran through `scripts/build-lock.sh`.

## Cycle P1 — U32, U33, U34, U35, U15 (messages only) — T073, T074, T081, T082

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
  branch (cycle P-last), so the M6 unit can move the bump.
- commit: `c1353804`

## Cycle P2 — U61, U62, U63, U64, U65, U97, U98, U99, U100, U101 (A41 service half, A42) — T075, T076, T083

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
- commit: `e09abf00`

## Cycle P3 — U131, U132, U133, U134, U135, U136, U137 (A37–A41, window half) — T077, T084

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
- commit: `431b5faa`

## Cycle P4 — U161, U162, U163, U164, U165 — T078, T085 (with the glue of T088, T089)

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
- commit: `75d105b2`. User guide (T090): `e05df97a`.

## Cycle P-last — U15 (the number) — T074, T082

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
- commit: the last commit of the branch, alone: `version.rs`, `schema_hash.rs` (the constant and
  its comment) and the version sentence of `docs/daemon.md`. 25 is one more than `main` at
  `ea477587`; if M5 or another feature takes 25 first, this commit is the one to move.

## Not done in the prep

- T079, T086 (macOS) and T080, T087 (Windows): their tests run only on those systems, and the
  macOS click needs a decision that has to be tried on a Mac (see the return of the prep unit).
  `desktop_notify::system(events)` takes the channel on all three systems; the macOS and Windows
  arms drop it, so a click there does nothing yet.
- T077's sentence about `.github/workflows/ci.yml`: `--test attention_reveal` is not in the
  enumerated list yet.
- T120: `session_reveal` and `attention_reveal` are green here (cycles P2, P3); the task is ticked
  by the M6 unit.
