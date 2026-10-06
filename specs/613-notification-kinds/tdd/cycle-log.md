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
