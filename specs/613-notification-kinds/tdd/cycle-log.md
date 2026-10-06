# TDD cycle log — 613 notification kinds

Append only. Deviation for the whole milestone: `tdd/test-list.md` was never generated (the
`speckit.tdd.plan` hook is optional and was not run in the design phase); the loop is driven from
the test-first order of `tasks.md` instead, one task pair (test task → implementation task) per
cycle. Each red was observed with the command shown before the implementation existed. Where Rust
needed the symbol to exist, a `todo!()` stub was added first and the red is the stub's or the
assertion's failure; where only a field or variant was missing, the red is the compile error.

## Cycle 1 — T001/T003, T005/T012, T006/T013 (micold-core attention)

- **Tests**: `crates/micold-core/src/attention.rs` — `notification_kind_tests::*` (10),
  `turn_clock_tests::*` (14), `tests::the_title_names_the_session_and_states_the_kind`,
  `tests::u28…u31` (rewritten for the kind argument)
- **Red**: `scripts/build-lock.sh cargo test -p micold-core --lib attention` →
  `test result: FAILED. 40 passed; 24 failed` — e.g. `the title states NeedsPermission in words (T1)
  left: "Fix the parser is waiting for input" right: "Fix the parser needs permission"`; the kind and
  clock tests panicked at the `todo!()` stubs.
- **Green**: `NotificationKind`, `NotificationKinds`, `LONG_TASK_THRESHOLD`, `TurnClock::change` per
  the data-model table, `notification_text(kind, …)` → `test result: ok. 64 passed; 0 failed`.
- **Refactor**: none needed.

## Cycle 2 — T002/T004 (core part: settings)

- **Tests**: `crates/micold-core/src/settings.rs` — `notification_kinds_tests::*` (5); U45 now
  expects `["desktop_notifications", "notification_kinds"]` (FR-011 adds the per-kind key on purpose;
  still none per AI CLI); `tests/settings_issue_mapping.rs` key list gains `notification_kinds`.
- **Red**: `scripts/build-lock.sh cargo test -p micold-core --lib settings` →
  `error[E0609]: no field 'notification_kinds' on type 'settings::Settings'`.
- **Green**: the field on `Settings` and `StoredSettings` with `#[serde(default)]`, both conversions
  → `test result: ok. 12 passed; 0 failed`.
- **Refactor**: none needed.

## Cycle 3 — T007/T014 (wire W5.1)

- **Tests**: `tests/protocol_roundtrip.rs::an_attention_grant_carries_its_kind_as_a_snake_case_string`,
  `tests/schema_hash.rs::the_grants_kind_is_in_the_hashed_source`, pinned version 30.
- **Red**: `scripts/build-lock.sh cargo test -p micold-core --test schema_hash --test protocol_roundtrip`
  → `error[E0559]: variant 'DaemonMsg::AttentionGranted' has no field named 'kind'`.
- **Green**: `kind: NotificationKind` on the grant, `PROTOCOL_VERSION` 30 →
  `cargo test -p micold-core --all-targets`: 1441 passed, 1 failed
  (`settings_refuses_save_over_failed_read`, fails identically on the unchanged tree: the container
  runs as root, so the unreadable-file permission is ignored; passes in CI).
- **Refactor**: none needed.

## Cycle 4 — T002/T004 (daemon part: catalog)

- **Tests**: `crates/micold-daemon/src/catalog.rs` — `notify_tests::*`: `notify(kind)` is the master
  switch AND the kind's own switch; `persist_service_settings` keeps the stored `notification_kinds`.
- **Red** (recorded in unit 3 by stubbing `Catalog::notify` to the master switch alone; tests were
  written with the code in 6a9d2397): `cargo test -p micold-daemon --lib catalog` → `FAILED. 2 passed;
  1 failed`, `notify_follows_each_kind_while_the_master_switch_is_on` (catalog.rs:1358).
- **Green**: `cargo test -p micold-daemon --lib` attention/activity/hooks/catalog → 47 passed.
- **Refactor**: none needed.

## Cycle 5 — T008/T015, T008/T016 (SubagentStop ignored; `turn_change`)

- **Tests**: `activity.rs::turn_change_maps_each_event`,
  `a_spinner_is_work_only_when_it_lifted_the_signal`; `hooks.rs` SubagentStop → `Ignored`, not
  registered; `tests/hooks_receiver.rs`.
- **Red** (recorded in unit 3 by stubbing): `turn_change` → `None`: `--lib activity` `FAILED. 11 passed;
  2 failed` (`turn_change_maps_each_event`), `--test attention_claims` `FAILED. 7 passed; 12 failed`
  (`a_long_turn_is_granted_as_long_task_finished`). SubagentStop classified as `Stop` again: `--lib
  hooks` `FAILED. 6 passed; 1 failed` (`classifies_hook_event_names`), `--test attention_claims`
  `FAILED. 18 passed; 1 failed` (`a_helper_agent_finishing_changes_nothing`).
- **Green**: daemon lib 47 passed; `--test hooks_receiver` 5 passed.
- **Refactor**: none needed.

## Cycle 6 — T009/T017, T010/T018 (pending kinds, `note_activity`, claim grants kind)

- **Tests**: `attention.rs` `Views::grant`/`note_event` unit tests;
  `tests/attention_claims.rs` 613 block (US1.1–US1.3, US1.6–US1.8, FR-024, SC-001, FR-018,
  Principle II, refused permission, reconnection, restart), threshold 200 ms; 039 harnesses
  (`attention_claims`, `settings_desktop_notifications`, `unread_state`) moved to threshold zero.
- **Red**: in unit 3 before the last fixes, `attention_claims` had two failures (the supervision
  test expected a grant, now refused per C12; SC-001 claimed only at the end and lost the
  overwritten kinds). Recorded in unit 3 by stubbing `Views::grant` to ignore `notify`: `--lib
  attention` → `FAILED. 24 passed; 2 failed`
  (`with_the_kind_off_now_a_claim_is_refused_and_records_nothing`); `attention_claims` stays green
  under that stub, since `note_event` already uses up an event whose kind is off.
- **Green**: `cargo test -p micold-daemon --test attention_claims --test settings_desktop_notifications
  --test unread_state --test hooks_receiver --test attention_events --test copilot_activity
  --test pi_activity` → 18, 14, 11, 5, 8, 10, 16 passed.
- **Refactor**: none needed.

## Cycle 7 — T011/T019 (client: title by kind; save keeps stored kinds)

- **Tests**: `crates/micold-client/tests/attention_notify.rs` (grant's kind titles the
  notification); bin `desktop_notify`, `persist`.
- **Red** (recorded in unit 3 by stubbing the client to title every grant `TurnFinished`):
  `cargo test -p micold-client --test attention_notify` → `FAILED. 11 passed; 3 failed`
  (`a_needs_permission_grant_shows_the_needs_permission_title`).
- **Green**: `attention_notify` 14 passed; bin tests passed.
- **Refactor**: none needed.

## Cycle 8 — T010/T018 (review A F2: a spinner-lifted turn starts the clock)

- **Tests**: `tests/attention_claims.rs::a_turn_seen_only_by_its_spinner_is_timed_from_the_spinner`
  (unix); `tests/copilot_activity.rs`, `tests/pi_activity.rs` long/short turn grants (US1.9) through
  the shared `tests/attention_support/mod.rs`.
- **Red**: `cargo test -p micold-daemon --test attention_claims spinner` → `left: []`,
  `right: [(…0b, 1, LongTaskFinished)]`: `drain_signals` lifted the signal without the clock.
- **Green**: `drain_signals` feeds `turn_change(SpinnerObserved, true)` to the clock →
  `attention_claims` 19 passed, `activity_pipeline` 14 passed.
- **Refactor**: none needed.

## Cycle 9 — M2: T021/T026, T022/T027, T023/T028, T024/T029, T025/T030 (Session error)

- **Tests**: `activity.rs` (`copilot_marks_only_session_error_as_an_error_ending`,
  `pi_shutdown_is_not_an_error_ending`, `an_error_ending_ends_the_signal_as_any_ending_does`);
  `attention.rs` `error_notice_target` (3); `tests/attention_error_notice.rs` (NEW, 9: give-up to
  the focused window only, restart/clean exit/no-error/stop/close/in view/no window/kind off/master
  off send none, unfocused fallback, unread and `attention_seq` unchanged);
  `tests/copilot_activity.rs` (`session.error` → `error: true`); `micold-client` `attention_notify`
  (2); `micold-core` `protocol_roundtrip` (`a_session_error_notice_round_trips_without_a_req`),
  `schema_hash` (`the_session_error_notice_is_in_the_hashed_source`, version constant 31).
- **Red** (stubs: `error_notice_target` → `None`, `session_error_notification` → `None`,
  `session.error` → `error: false`, version constant 30): `attention_error_notice` →
  `FAILED. 6 passed; 3 failed`; client `attention_notify` → `FAILED. 15 passed; 1 failed`
  (`a_session_error_notice_shows_one_session_error_notification`); core `schema_hash` →
  `FAILED. 18 passed; 1 failed` (`left: 31, right: 30`).
- **Green**: daemon `--lib -- activity attention` 45 passed; `attention_error_notice` 9,
  `copilot_activity` 13, `pi_activity` 10, `supervision_giveup` 1, `attention_events` 14,
  `attention_claims` 19, `activity_ended` 2 passed; client `attention_notify` 16 passed;
  `micold-core --all-targets` green but for the root-only permission tests.
- **Refactor**: none needed.

## Cycle 10 — M2 review A F1: a give-up after a reported error sends no second notice (C7, FR-007)

- **Test**: `tests/attention_error_notice.rs`
  `a_give_up_after_a_reported_error_sends_no_second_notice`.
- **Red**: `attention_error_notice` → `FAILED. 9 passed; 1 failed` (`left: [SessionErrorNotice
  {..}]`, `right: []`): the give-up sent a second notice.
- **Green**: the GiveUp arm reads whether the session had already ended before `note_ended`, and
  sends no notice if it had; `attention_error_notice` 10 passed.
- **Refactor**: none needed.
