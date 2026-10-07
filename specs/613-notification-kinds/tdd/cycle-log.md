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

## Cycle 11 — M3: T059/T061 (threshold-free note; SC-008), T056/T061 (core settings), T032+T057/T037+T062 (wire, core part)

- **Tests**: `crates/micold-core/src/attention.rs` — `notification_kind_tests::the_long_task_description_names_the_threshold_not_a_duration`
  (replaces T001's "a minute" check), `turn_clock_tests::the_threshold_passed_decides_long_from_short_for_each_allowed_value`,
  `turn_clock_tests::a_turn_is_classified_by_the_threshold_passed_at_its_finish`;
  `crates/micold-core/tests/settings_long_task_threshold.rs` (NEW, 7); `protocol_roundtrip.rs` fields filled with
  `Some`/`None`; `schema_hash.rs::the_notification_kinds_and_the_threshold_are_in_the_hashed_source`, version constant 32;
  `settings_issue_mapping.rs` key list gains `long_task_threshold_secs` (a new field on purpose, as Cycle 2).
- **Red**: `scripts/build-lock.sh cargo test -p micold-core --lib attention` → `left: "A session finished a turn that took a
  minute or more." right: "A session finished a turn at least as long as the long-task threshold."` (65 passed; 1 failed).
  The two SC-008/FR-013 clock tests passed at once (`TurnClock::change` already takes the threshold); deliberate mutant
  `>= LONG_TASK_THRESHOLD` in `change` → both FAILED, restored. `cargo test -p micold-core --test settings_long_task_threshold`
  → `error[E0432]: unresolved imports micold_core::settings::clamp_long_task_threshold …`. `cargo test -p micold-core --test
  protocol_roundtrip --test schema_hash` → `error[E0559]: variant ClientMsg::SettingsSet has no field named notification_kinds`.
- **Green**: description text; `MIN/MAX_LONG_TASK_THRESHOLD_SECS`, `clamp_long_task_threshold`,
  `default_long_task_threshold_secs()` (= `LONG_TASK_THRESHOLD`), the field on `Settings`/`StoredSettings`, clamped in
  `into_settings`; `DaemonSettings.{notification_kinds, long_task_threshold_secs}`, `SettingsSet.{…: Option<…>}`,
  `PROTOCOL_VERSION` 32 → `cargo test -p micold-core --no-fail-fast`: all green but the known root-only permission tests.
- **Refactor**: none needed.

## Cycle 12 — M3: T038/T063 service, T033/T058 integration, client T035/T042/T060/T064, UI T034/T036/T039–T041/T043/T044/T065

- **Red**: `catalog.rs` `notify_tests::{set_notification_kinds_stores_and_persists_them,
  set_long_task_threshold_clamps_and_stores_the_value, the_service_write_keeps_the_long_task_threshold}` written by unit 1
  before the setters existed — red as E0599 (no method `set_notification_kinds` / `set_long_task_threshold` /
  `long_task_threshold` on `Catalog`). The client and UI tests (features_settings, main_tests, icons*, settings_sections,
  notification_icon, checkbox) were written by delegated workers in the same pass as their code: no separate red run
  recorded for them (honest gap; closed retroactively by mutation below).
- **Green**: `Catalog::{set_notification_kinds, long_task_threshold, set_long_task_threshold}` and the wire/persist
  fields; `DaemonState::{set_notification_kinds (C15), set_long_task_threshold_secs, effective_long_task_threshold}` with
  the test override; `server.rs` `SettingsSet` arms; client draft, save, mirror; Settings rows, threshold field, icons,
  showcase. `settings_notification_kinds` 9/9, `settings_long_task_threshold` 7/7, `features_settings` 70/70, icon and
  section tests green.
- **Red (retroactive, mutation, 2026-10-06)**: one mutant per behaviour against the finished code,
  covering targets `--lib`, `features_settings`, `settings_sections`, `notification_icon`, `icons`,
  `icon_roles`; each file restored with `git checkout` after the run.
  - Save writes the kinds — `settings.rs` save → `notification_kinds: Default::default()` →
    `save_writes_the_whole_kinds_value_and_the_parsed_threshold` and
    `the_master_switch_never_changes_the_kinds_or_the_threshold` failed (`left != right` on the kinds).
  - A kind toggle edits the draft — `notification_kind_toggled` made a no-op → the same two tests failed.
  - Threshold range refusal — `long_task_threshold` accepts any `t >= 1` →
    `a_bad_threshold_refuses_the_save_with_the_s6_message` failed (`"9"` validated).
  - Kind → icon mapping — `SessionError => Icon::NeedsPermission` → `notification_icon::{each_kind_maps_to_its_icon,
    the_mapping_is_one_to_one}` failed.
  - `Checkbox::icon` — the setter drops the icon → `checkbox::tests::icon_is_chainable_and_keeps_label_state_and_toggle` failed.
  - **Survived** (no unit test can see them; covered by the visual pass only, visual-pass.md §B5): the
    component's label `disabled_tint` → `tint`, and the kind rows' `if master_on` gate → `if true`. The
    style path's disabled label colour is unit-tested (`a_disabled_checkbox_label_takes_the_disabled_colour`).
  - Disabled checked box keeps its mark (visual-pass finding) — test-first: `a_disabled_checked_checkbox_keeps_a_visible_mark`
    red (`assertion left != right failed: Light`) before the `style::checkbox` change, green after.
- **Refactor**: none.

## Cycle 13 — M4: T046/T048/T049 (rasteriser), T047/T050/T051 (backends, icon files), T052 (guide)

- **Red**: `tests/notification_icon.rs` rasteriser tests (size, tile luminance in [0.10, 0.30], tile
  and white glyph pixels, ink centred within 1 px at 64, pairwise mask distinctness at 16) written
  before `render`/`tile_colour`/`glyph_mask` existed — E0432 unresolved imports. Then the I5/I6
  tests: `linux.rs` (`image-data` per kind, `(iiibiiay)` signature), `mod.rs`
  (`icon_files_in`: writable, unwritable, none), `tests/notification_icon.rs` (`write_files`: four
  256 px PNGs, overwrite, refusal), `windows.rs` (`toast_icon`, `ICON_CROP`), `macos.rs`
  (`banner_image`) — E0432/E0422/E0425 (no `write_files`, `Hint`, `ImageData`, `image_data`,
  `hint_value`, `icon_files_in`).
- **Green**: `notification_icon::{Rgba, tile_colour, file_stem, render, glyph_mask, png,
  IconFiles, write_files}`; Linux request hint; `mod.rs` once-per-run files under the data dir
  with one log line on failure; Windows `Toast::icon(path, IconCrop::Square, kind.name())`; macOS
  `Notification::image_path`. First green run failed the distinctness gate honestly:
  `SessionError and LongTaskFinished differ in only 13 of 256 pixels` with the glyph at the tile's
  62.5 % span; the gate's masks are now the glyph drawn *at* 16×16 (ink box spanning the square,
  FR-017's wording), which passes; the tile render keeps 62.5 %. Linux: 30 `desktop_notify` bin
  tests and 11 `notification_icon` tests green. Windows and macOS tests compile only:
  `cargo clippy -p micold-client --all-targets -D warnings` for `aarch64-apple-darwin` and
  `x86_64-pc-windows-msvc` clean, `cargo check --workspace --target aarch64-apple-darwin` clean;
  this container cannot run them.
- **Refactor**: none.

## Cycle 14 — close: Phase 8 TDD remediation (T067–T076)

- **T068 (Finding 2)**: decided to delete the C15 snapshot in `DaemonState::set_notification_kinds`: `note_event(.., notify=false)` already spends an event made while its kind was off, and the snapshot's one observable effect was dropping a pending event of a kind that was already on. Test first: `settings_notification_kinds::turning_a_kind_on_keeps_a_pending_event_of_a_kind_that_was_already_on`. **Red**: `scripts/build-lock.sh cargo test -p micold-daemon --test settings_notification_kinds` → `turning_a_kind_on_keeps_a_pending_event_of_a_kind_that_was_already_on ... FAILED`: `the permission asked while its kind was on is still notified  left: []  right: [(…0a, 1, NeedsPermission)]` (9 passed; 1 failed). **Green**: snapshot block removed, contract C15 reworded → 10/10, `settings_desktop_notifications` 10/10 (the master switch's snapshot is kept).
- **T059 red (retroactive, Finding 3)**: `TurnClock::change` compares with `LONG_TASK_THRESHOLD` instead of the `threshold` passed (`crates/micold-core/src/attention.rs:311`) → `scripts/build-lock.sh cargo test -p micold-core --lib attention` → `a_turn_is_classified_by_the_threshold_passed_at_its_finish` FAILED: `30 s is long under the 20 s threshold in force at the finish (FR-013)  left: Some(TurnFinished)  right: Some(LongTaskFinished)`, and `the_threshold_passed_decides_long_from_short_for_each_allowed_value` FAILED: `a turn of exactly 10 s under a 10 s threshold is long (SC-008)` (64 passed; 2 failed); restored.
- **T033 red (retroactive, Finding 3)**: `Catalog::notify` ignores the kind switch (`crates/micold-daemon/src/catalog.rs:423`, `desktop_notifications && true`) → `scripts/build-lock.sh cargo test -p micold-daemon --test settings_notification_kinds --no-fail-fast` → 4 FAILED: `with_only_one_kind_on_only_events_of_that_kind_are_granted`, `turn_finished_on_and_long_task_off_grants_the_short_turn_and_refuses_the_long` (`the long turn is a long task, and that kind is off`), `an_event_made_while_its_kind_is_off_is_not_granted_after_the_kind_is_turned_on` (`the long task made while its kind was off raises nothing after the fact`), `a_give_up_notice_follows_the_session_error_switch_alone` (`with only NeedsPermission on: [SessionErrorNotice { … }]`); restored.
- **T058 red (retroactive, Finding 3)**: the `SettingsSet` arm drops `long_task_threshold_secs` (`crates/micold-daemon/src/server.rs:1125`, `Some(_secs) => Ok(())`) → `scripts/build-lock.sh cargo test -p micold-daemon --test settings_long_task_threshold --no-fail-fast` → 5 FAILED (2 passed), e.g. `the_threshold_is_stored_persisted_and_pushed_to_both_windows`: `the window that saved is told: [OperationOk { req: 1, result: Ack }]`, `the_effective_threshold_follows_the_setting_and_a_running_turn_sees_the_change`: `the turn begun before the change is judged by the new value`; restored.
- **T071 (Finding 6)**: injectable turn clock: `Uptime::saturating_add` (core `clock.rs`) and the test seam `DaemonState::advance_turn_clock(by)` (`turn_now()` = `clock::now()` + offset, read at both `TurnClock::change` sites). The 613 tests now set a 30 s threshold and make a turn long with `advance_turn_clock(31 s)` instead of a 300 ms sleep (`attention_claims`, `settings_notification_kinds`, `settings_long_task_threshold`, `copilot_activity`, `pi_activity`), so a slow runner can never turn a short turn long. Proof: 20 consecutive green runs under `taskset -c 0` of `cargo test -p micold-daemon --test attention_claims --test settings_notification_kinds --test settings_long_task_threshold` (`GREEN_RUNS=20`); `copilot_activity` 13/13, `pi_activity` 10/10, `settings_desktop_notifications` 10/10, `unread_state` 16/16.
- **T034, T035, T036, T060 reds (retroactive, Finding 3)**: one run with three source breaks, restored with `git checkout` after: `Icon::NeedsPermission => '\u{e2e6}'` (`icons.rs:214`, the `task_alt` codepoint); the draft opened from `DaemonSettings` takes `notification_kinds: Default::default()` and `long_task_threshold_secs: String::new()` (`features/settings.rs:832-833`); the kind rows iterate `NotificationKind::ALL.into_iter().rev()` (`ui/settings/environment.rs:177`). `scripts/build-lock.sh cargo test -p micold-client --test features_settings --test settings_sections --test icons --test icons_font --test notification_icon --no-fail-fast` → T034: `icons` 3 FAILED (`the_four_notification_kind_icons_are_in_the_vocabulary_and_distinct`: `NeedsPermission codepoint left: '\u{e2e6}' right: '\u{e925}'`, `all_covers_every_variant_without_duplicates`, `glyph_maps_every_variant_to_its_pinned_codepoint`), `icons_font` 1 FAILED (`the_notification_kind_icons_have_glyphs_in_the_bundled_font`), `notification_icon` 2 FAILED (`the_mapping_is_one_to_one`, the 16 px distinctness gate); T035/T060: `features_settings` 3 FAILED (`the_draft_is_seeded_with_the_stored_kinds_and_threshold`: `left: "" right: "60"`, `the_mapping_survives_other_saves`, `issue_mapping::an_invalid_mapping_refuses_the_save`). T036: `settings_sections` stayed 17/17 under the reversed order — the source scan cannot see order at run time; closed by T075's geometry gate below.
- **T067 (Finding 1)**: `Capabilities::with_notifier` (test-only) and three `daemon_sync` bin tests that feed `DaemonMsg::AttentionGranted` / `SessionErrorNotice` through `on_daemon_event` and run the returned task to its end: `a_needs_permission_grant_reaches_the_notifier_with_its_kind_and_title`, `each_granted_kind_reaches_the_notifier_as_that_kind`, `a_session_error_notice_reaches_the_notifier_as_a_session_error`. **Red (M15 again)**: the grant arm passes `NotificationKind::TurnFinished` → `scripts/build-lock.sh cargo test -p micold-client --bins -- daemon_sync::tests::` → 2 FAILED (`left: TurnFinished right: NeedsPermission`; `left: [TurnFinished] right: [NeedsPermission]`), 40 passed; restored → green.
- **T072 (Finding 7)**: FR-017's reading recorded in `research.md` R7: the 16 px gate is over the glyph drawn at 16×16; no gate over `render(kind, 16)`.
- **T073 (Finding 9)**: `the_sc_001_sequence_gives_eleven_notifications_of_their_kinds` adds a focused view report and an error ending (`Ended { error: true }`) after the 30 turns: 5 + 5 grants, 1 `SessionErrorNotice`, 0 Turn finished, 11 in all. Green.
- **T074 (Finding 10)**: `endings_without_an_error_send_nothing` split into `an_ending_that_is_no_error_sends_nothing`, `a_user_stop_sends_nothing` (ends through `stop_session` and a supervision tick alone, no `Ended` event), `a_close_sends_nothing`. Green.
- **T075 (Finding 8)**: gate `tests/gates/notification_kind_rows_sit_under_the_switch.rs` (in the `layout_snapshot` binary): one run of five rows indented by `spacing::MD` directly under the master switch's row, stacked without overlap inside the page, the fourth (after Long task finished) the only one of another height (the threshold field). Green (59/59); its red (reversed order) not yet run.
- **T076 (Findings 11-13)**: `attention_support` gains `pub idle_process`, `pub next_frame`, `connect_with_settings`; `attention_error_notice.rs` and `settings_notification_kinds.rs` use them, `attention_claims.rs` imports `kinds`; the hook receiver binds in a `tempfile` directory; `linux.rs` fixtures titled "needs permission". `attention_claims` 19/19, `attention_error_notice` 12/12, `settings_notification_kinds`, `copilot_activity`, `pi_activity` green; clippy `-p micold-daemon -p micold-client --all-targets -D warnings` clean.
