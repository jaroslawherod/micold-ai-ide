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
