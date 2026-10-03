# M8 prep — red/green cycles (T101–T112, T121)

Written by the M8 prep unit on branch `feat/notify-session-needs-attention-m8-prep`, from
`origin/main` at `9bab6e4a` (the M6 merge), while M7 ran elsewhere. The M8 unit copies these
cycles into `cycle-log.md` with its own cycle numbers. Nothing here was pushed.

How the reds were made: a new field or parameter does not compile until it exists, so each red
ran against a **stub** that compiled and did the wrong thing; the stub is named in each cycle. All
reds are from the working tree, not from a commit: tests and code are in one commit per layer.
Commands ran through `scripts/build-lock.sh`.

## Cycle P1 — U42, U43, U44, U45 (A43, A48) — T101, T106

- tests: `crates/micold-core/src/settings.rs::desktop_notifications_tests`
  - `desktop_notifications_are_on_by_default` (U42)
  - `a_settings_file_without_the_field_reads_as_on` (U43)
  - `desktop_notifications_off_survives_a_save_and_load` (U44)
  - `the_stored_settings_hold_one_notification_key_and_none_per_ai_cli` (U45)
- stub: `Settings::desktop_notifications` with `#[serde(default)]`, `false` in `Default`, and no
  field in `StoredSettings` (not written, read back as `false`).
- red, `cargo test -p micold-core --lib desktop_notifications_tests`: `0 passed; 4 failed`
  - U42 `a fresh installation, and a settings file that cannot be read, notify`
  - U43 `the user of an older file never turned notifications off`
  - U44 `assertion left == right failed: the file holds the user's choice` (`left: None`, `right: Some(Bool(false))`)
  - U45 `assertion left == right failed: one switch, for every AI CLI` (`left: []`, `right: ["desktop_notifications"]`)
- green, `cargo test -p micold-core --all-targets`: exit 0. `default_desktop_notifications()`
  returns `true`; the field is in `StoredSettings`, `from_settings` and `into_settings`.
- Also changed: the key list in `tests/settings_issue_mapping.rs` gains `desktop_notifications`
  (that test says a new field is added there on purpose).

## Cycle P2 — U17 (the two fields) — T102, T107

- tests:
  - `crates/micold-core/tests/schema_hash.rs::the_desktop_notifications_setting_is_in_the_hashed_source`
  - `crates/micold-core/src/protocol/messages.rs::desktop_notifications_wire_tests::the_service_settings_carry_desktop_notifications`
  - `…::a_settings_change_carries_desktop_notifications_or_leaves_it`
- red, `cargo test -p micold-core --test schema_hash the_desktop`, before the fields existed:
  `0 passed; 1 failed`: ``` `pub desktop_notifications: bool,` is not in messages.rs, so the hash is
  not the hash of the message set that carries the Desktop notifications setting ```
- **No assertion red for the two round-trip tests**: they do not compile without the fields, and
  a derived `Serialize`/`Deserialize` field cannot be stubbed wrong. They were written with the
  fields.
- green, `cargo test -p micold-core --all-targets`: exit 0.
- The version number is cycle P7.

## Cycle P3 — U66, U67, U68 — T103, T108

- tests: `crates/micold-daemon/src/attention.rs::tests`
  - `with_the_setting_off_a_claim_is_refused_and_records_nothing` (U66)
  - `an_event_noted_with_the_setting_off_is_not_granted_later` (U67)
  - `noting_an_earlier_event_does_not_lower_what_was_granted` (U67, the `max`)
  - `an_event_noted_with_the_setting_on_is_still_granted` (U68)
- stub: `grant(.., enabled)` ignored `enabled`; `note_event` did nothing. T020's calls pass `true`.
- red, `cargo test -p micold-daemon --lib attention::`: `20 passed; 2 failed`
  - U66 `nothing is granted while desktop notifications are off`
  - U67 `the event happened while desktop notifications were off`
  - U68 and the `max` test passed against the stub: a `note_event` that does nothing is right for
    `enabled: true`, and lowers nothing. They pin the green code, not the stub.
- green, the same command: `22 passed`.

## Cycle P4 — U103–U110 (A23, A44–A48) — T104, T109

- tests: `crates/micold-daemon/tests/settings_desktop_notifications.rs` (new, the harness of
  `unread_state.rs`: real connections, idle processes, `note_activity` as `hooks.rs` calls it)
  - `on_a_fresh_store_the_service_reports_desktop_notifications_on` (A43, the service's half)
  - `turning_the_switch_off_reaches_every_connection` (U103, A45)
  - `the_switch_turned_off_survives_a_restart_of_the_service` (U104, A46)
  - `a_settings_change_that_does_not_name_the_switch_leaves_it_off` (U105)
  - `while_off_no_claim_is_granted_and_the_event_still_sets_unread` (U106, U107, A23, A44)
  - `turned_off_while_sessions_run_the_next_claim_is_refused` (A45)
  - `turned_on_again_the_next_event_is_granted_and_the_ones_made_while_off_are_not` (U108, U109, A47)
  - `the_switch_applies_alike_to_a_session_of_each_ai_cli` (U110, A48)
- stub: `settings_wire` reported the field; `server.rs` ignored it in `SettingsSet`;
  `claim_attention` passed `true`; `note_activity` did not call `note_event`.
- red, `cargo test -p micold-daemon --test settings_desktop_notifications`: `1 passed; 7 failed`
  - U103 `the window that turned it off is told: [OperationOk { req: 1, result: Ack }]` (`left: None`, `right: Some(false)`)
  - U104 `the settings file holds the switch as off`
  - U105 `a change that does not name the switch does not turn it on`
  - U106 `no notification for the session not in view` (`left: [(…0b, 1)]`, `right: []`)
  - A45 `the first window is refused: the switch holds for every window at once` (`left: [(…0a, 2)]`)
  - U109 `the event made while the switch was off raises nothing after the fact` (`left: [(…0a, 1)]`)
  - U110 `with the switch off a ClaudeCode session raises nothing` (`left: [(…c0, 1)]`)
  - the fresh-store test passed: the default was green since cycle P1.
- green, the same command: `8 passed`. `Catalog::desktop_notifications`,
  `set_desktop_notifications` and the line in `persist_service_settings`;
  `DaemonState::set_desktop_notifications`; `claim_attention` passes the setting; `note_activity`
  calls `note_event` with the new sequence after `mark_attention`; `server.rs` applies the field.

## Cycle P5 — U142, U143, U144 (A43) — T105, T110

- tests:
  - `crates/micold-client/tests/features_settings.rs::the_desktop_notifications_switch_is_seeded_from_the_stored_setting` (U142, A43)
  - `…::the_desktop_notifications_switch_reaches_what_save_writes` (U143, the draft half)
  - `crates/micold-client/src/main_tests.rs::the_desktop_notifications_message_changes_the_draft` (U142)
  - `…::turning_desktop_notifications_off_and_saving_tells_the_service` (U143)
  - `…::saving_with_desktop_notifications_on_tells_the_service_they_are_on` (U143)
  - `…::the_desktop_notifications_switch_opens_with_the_value_the_service_reported` (U144)
- stub: the draft field, `ValidSettings` field, session-state field and `Msg` variant existed;
  `from_settings` set `true` whatever was stored; the message's handler did nothing; the save sent
  `desktop_notifications: None`; `SettingsChanged` was not mirrored.
- red, `cargo test -p micold-client --test features_settings desktop_notifications` and
  `-p micold-client --bin micold-ai-ide desktop_notifications`: `1 passed; 1 failed` and `0 passed; 4 failed`
  - U142 `a user who turned desktop notifications off must see the switch off`
  - U142 `the switch was turned off` (the message)
  - U143 `the service must be told desktop notifications are off` (`left: Some(None)`, `right: Some(Some(false))`)
  - U143 `the service must be told the switch's position` (`left: Some(None)`, `right: Some(Some(true))`)
  - U144 `the page must show the switch as the service reported it` (`left: true`, `right: false`)
  - the draft half of U143 passed against the stub: `validate` and `into_settings` were written real.
- green, the same two commands: `2 passed` and `4 passed`.
- The tests sit beside those of `tool_server_enabled`, which are in `tests/features_settings.rs`
  and `src/main_tests.rs`, not in `src/features/settings.rs` as T105 words it.

## Cycle P6 — the checkbox — T111

- test: the existing gate `crates/micold-client/tests/settings_sections.rs::every_persisted_setting_is_claimed_or_recorded_as_deferred`.
- red, `cargo test -p micold-client --test settings_sections`, with the field in `Settings` and no
  control: `13 passed; 1 failed`: `these persisted settings are rendered by no section and
  recorded as deferred by nothing: ["desktop_notifications"]`
- green, the same command: `14 passed`. `SETTINGS` in `ui/settings/environment.rs` claims
  `("desktop_notifications", "DesktopNotificationsToggled")`; the `Checkbox` with its `field_note`
  sits after the tool-server switch; `FieldId::SettingsDesktopNotifications` is new.
- **No test**: how the row looks. Quickstart §B12 and the visual pass are the M8 unit's.

## Cycle P7 — U17 (the number) — T102, T107

- test: `crates/micold-core/tests/schema_hash.rs::the_wire_changes_for_this_feature_cost_exactly_one_version_bump`
  (the constant it pins moved 25 → 26).
- red, `cargo test -p micold-core --test schema_hash`, with `PROTOCOL_VERSION` still 25:
  `15 passed; 1 failed`: `assertion left == right failed: the protocol version moved…` (`left: 25`, `right: 26`)
- green, the same command: `16 passed`. `PROTOCOL_VERSION` is 26, with its line in `version.rs` and
  in `docs/daemon.md`.
- **The number is a placeholder for the M8 unit.** `origin/main` was at 25. When M7 takes 26, this
  becomes 27, as `tasks.md` plans: the constant in `schema_hash.rs`, `version.rs` and
  `docs/daemon.md` move together. The change is the last commit of the series, alone.

## T112 — user guide

No test. `docs/user-guide/settings.md` gains *Desktop notifications* under Environment;
`docs/user-guide/worktrees-and-sessions.md` no longer says the application has no switch, and its
unread paragraph names the switch.

## T121 — story 4's outer tests

Green on the working tree that became the series (before the version commit):
`cargo test -p micold-daemon --lib attention::` `22 passed`;
`--test settings_desktop_notifications` `8 passed`;
`-p micold-client --test features_settings desktop_notifications` `2 passed`;
`-p micold-client --bin micold-ai-ide desktop_notifications` `4 passed`;
`-p micold-client --test settings_sections` `14 passed`; `-p micold-core --all-targets` exit 0.
The full gate on the series' HEAD is in the prep unit's return, not here: this file is committed
before the version commit, which stays last.
