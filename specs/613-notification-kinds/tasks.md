---

description: "Task list for feature 613: notification kinds, per-kind settings and icons"
---

# Tasks: Notify Only When a Session Needs Attention, With Per-Kind Settings and Icons

**Input**: Design documents from `/specs/613-notification-kinds/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/ (wire.md,
classification.md, notification.md), quickstart.md

**Tests**: Per Constitution Principle I (Test-First Development, NON-NEGOTIABLE), test tasks are
MANDATORY. Every phase writes its failing tests before its implementation tasks; the number in
brackets after an implementation task names the test task it turns green.

**Documentation**: Per Constitution Principle VII, each story's user-guide task ships in that
story's milestone (FR-023).

**Cross-platform**: Per Constitution Principle VI, kinds, defaults and switches are platform-free
core and service logic; only icon delivery differs, inside the existing
`shell/desktop_notify/{linux,macos,windows}.rs` backends.

**Wire versions**: each milestone that changes the wire takes the next `PROTOCOL_VERSION` (contract
[wire.md](./contracts/wire.md), 039 R10): M1 29 → 30 (`AttentionGranted.kind`), M2 30 → 31
(`SessionErrorNotice`), M3 31 → 32 (`notification_kinds` and `long_task_threshold_secs` in
`DaemonSettings` and `SettingsSet`). If
an earlier milestone has not merged when a later one is cut, the later one still bumps once from
whatever `main` has. No `#[serde(default)]` on wire types.

**Test threshold**: the daemon's integration tests cannot wait 60 s for a long turn.
`DaemonState` holds `long_task_threshold: Duration`, initialised from
`micold_core::attention::LONG_TASK_THRESHOLD` and set by a `pub fn set_long_task_threshold(&self,
Duration)` that only tests call. `note_activity` passes that field to `TurnClock::change`. The
constant stays the only definition of 60 s (contract C6). From M3 (D4 = B) the threshold is the
setting `Settings::long_task_threshold_secs` (10–3600 s, default `LONG_TASK_THRESHOLD`), read live
from the catalog; the field becomes `long_task_threshold_override: Option<Duration>`, still set
only by `set_long_task_threshold`, and wins over the setting when set (T063, data-model
"Long-task threshold").

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3)

---

## Phase 1: Setup (Shared Infrastructure)

No setup tasks: the workspace, crates and test harnesses exist. The one dependency change (the
`png-format` feature of `tiny-skia`) belongs to User Story 3 (T048).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the kinds, their defaults and the stored switches that every story reads.

- [X] T001 [P] Unit tests in `crates/micold-core/src/attention.rs` (`#[cfg(test)]`): `NotificationKind::ALL` is `[NeedsPermission, SessionError, LongTaskFinished, TurnFinished]` (FR-009); `name()` is "Needs permission", "Session error", "Long task finished", "Turn finished"; `description()` is the data-model text, and **Long task finished**'s description is built from `LONG_TASK_THRESHOLD` (a test that formats the constant, not a second literal); `default_on()` is true for the first three and false for `TurnFinished` (FR-010); serde encodes the kinds as `"needs_permission"`, `"session_error"`, `"long_task_finished"`, `"turn_finished"`; `NotificationKinds::default()` equals the four defaults; `is_on(kind)` reads the kind's field and `set(kind, on)` writes only that field; `LONG_TASK_THRESHOLD == Duration::from_secs(60)`
- [X] T002 [P] Settings load tests in `crates/micold-core/src/settings.rs` (beside `desktop_notifications_tests`): a fresh `Settings::default()` has the default kinds; a settings file written before this feature, once with `"desktop_notifications": false` and once with `true`, keeps that value and gets the default kinds (FR-010, US2.8); a file whose `notification_kinds` object lacks one field gets that field's default and keeps the others; a round trip keeps all four values; an unreadable file falls back to `Settings::default()` as the existing load rules do (Edge Cases "Settings file unreadable"). In `crates/micold-daemon/src/catalog.rs` tests: `notify(kind)` is false for every kind while `desktop_notifications` is off and follows `is_on(kind)` while it is on; `persist_service_settings` writes `notification_kinds` into the file it writes
- [X] T003 `NotificationKind` (closed enum, `Copy`, `#[serde(rename_all = "snake_case")]`) with `ALL`, `name`, `description`, `default_on`; `NotificationKinds { needs_permission, session_error, long_task_finished, turn_finished }` with `Default`, `is_on`, `set`, and serde per field defaults (`needs_permission: true`, `session_error: true`, `long_task_finished: true`, `turn_finished: false`); `pub const LONG_TASK_THRESHOLD: Duration = Duration::from_secs(60)` — all in `crates/micold-core/src/attention.rs` (T001)
- [X] T004 `Settings::notification_kinds: NotificationKinds` with `#[serde(default)]` on the field in `crates/micold-core/src/settings.rs`, on both the in-memory `Settings` (line ~169) and the file form beside the second `desktop_notifications` field (line ~443) and both conversions; in `crates/micold-daemon/src/catalog.rs`, `persist_service_settings` carries `notification_kinds` over into the file it writes as it does `desktop_notifications`, and the catalog gains `notification_kinds()` and `notify(kind) -> bool` = `desktop_notifications && notification_kinds.is_on(kind)` (data-model "Effective switch"), read live on every call (C16) (T002)

**Checkpoint**: core and catalog know the kinds and the defaults; nothing uses them yet.

---

## Phase 3: User Story 1, slice A — awaiting-input kinds, notified by their defaults (Priority: P1) 🎯 MVP

**Goal**: Every change of a session into awaiting input gets a kind from the session's turn: a stop
mid-turn is **Needs permission**, a finished turn is **Long task finished** (≥ 60 s) or **Turn
finished**. The service grants a window's claim only when that kind is on, so with the defaults a
short turn raises no notification and still makes the session unread. The notification's title
names the kind. Claude Code's `SubagentStop` no longer counts as a turn end (FR-024).

**Independent Test**: `crates/micold-daemon/tests/attention_claims.rs` and `attention_events.rs`
drive hook sequences with a short test threshold and check which claims are granted, with which
kind, and that `attention_seq`/unread change exactly as before; `crates/micold-client/tests/attention_notify.rs`
checks the title per kind.

### Tests for User Story 1, slice A (MANDATORY — Constitution Principle I) ⚠️

- [X] T005 [P] [US1] `TurnClock` unit tests in `crates/micold-core/src/attention.rs`: every cell of the data-model transition table (`NotInTurn`, `Working{s}`, `Paused{s}` × `PromptSubmitted`, `Working`, `AskedUser`, `Finished`), each with its returned kind or `None`; a finish at exactly the threshold is `LongTaskFinished` and one millisecond less is `TurnFinished` (C4, Edge Cases "exactly at the threshold"); a turn paused by `AskedUser` and resumed by `Working` keeps its start, so the wait counts (C4, FR-003, US1.7); `NotInTurn` + `Finished` is `TurnFinished` and `NotInTurn` + `Working` starts the clock then (C5, unknown start); `Paused` + `AskedUser` returns `None`; `PromptSubmitted` while `Working` restarts at `now`. Times are built from `micold_core::clock::Uptime` values, not the real clock
- [X] T006 [US1] Unit tests for `notification_text(kind, project, worktree, session)` in `crates/micold-core/src/attention.rs`: the four titles of contract T1 (`<session> needs permission`, `<session> stopped with an error`, `<session> finished a long task`, `<session> finished its turn`) and the body `<project> — <worktree>` for every kind (FR-008, US3.3); no other text (T2)
- [X] T007 [P] [US1] Protocol tests in `crates/micold-core/tests/protocol_roundtrip.rs` and `crates/micold-core/tests/schema_hash.rs`: `DaemonMsg::AttentionGranted { session, seq, kind }` encodes `kind` as its snake-case string and round-trips for each awaiting-input kind; `PROTOCOL_VERSION` is 30; the schema hash is updated (wire W5.1)
- [X] T008 [P] [US1] Daemon unit tests in `crates/micold-daemon/src/hooks.rs`: `classifies_hook_event_names` asserts `"SubagentStop"` → `HookClass::Ignored` (C17); the `settings_json` test asserts `SubagentStop` is not registered (FR-024). In `crates/micold-daemon/src/activity.rs`: `turn_change(&ActivityEvent, lifted: bool) -> Option<TurnChange>` maps `Hook(UserPromptSubmit)` → `PromptSubmitted`, `Hook(PreToolUse)` → `Working`, `SpinnerObserved` → `Working` only when `lifted`, `Hook(Notification)` → `AskedUser`, `Hook(Stop)` → `Finished`, and `Hook(PostToolUse)`, `ReadyForInput`, `Ended` → `None` (data-model "Mapping")
- [X] T009 [P] [US1] `Views` unit tests in `crates/micold-daemon/src/attention.rs`: `note_event(session, seq, kind, true)` keeps `kind` pending and `note_event(…, false)` records `seq` as granted (C10); `grant(session, seq, current, notify)` returns `Some(kind)` only under 039's rule with a pending kind and `notify(kind)` true now, `None` when the sequence has no pending kind or `notify` is false now, and a refused claim records nothing (C11); pending kinds at or below a granted or recorded-as-granted sequence are dropped, and `forget_session` drops all of the session's (C12); two sessions' pending kinds never mix (Principle II)
- [X] T010 [P] [US1] Integration tests in `crates/micold-daemon/tests/attention_claims.rs`, one window not viewing session B, `set_long_task_threshold(200 ms)`, events through `DaemonState::note_activity`: `UserPromptSubmit`, `Stop` at once → `attention_seq` +1, the claim refused (Turn finished off, US1.1); `UserPromptSubmit`, wait past the threshold, `Stop` → claim granted with `kind: LongTaskFinished` (US1.2); `UserPromptSubmit`, `PreToolUse`, `Notification` → granted `NeedsPermission` (US1.3); after that, `PreToolUse` then `Stop` past the threshold counted from the prompt (the pause inside it) → granted `LongTaskFinished` (US1.7); a repeated `Notification`/`Stop` without work in between adds nothing (US1.8); a `SubagentStop` POST mid-turn and while paused changes no signal, no `attention_seq` and no unread, and the turn across it still ends as `LongTaskFinished` (FR-024); the SC-001 sequence (20 short, 5 long, 5 short with one permission each) — each permission turn being prompt, `PreToolUse`, `Notification`, `PreToolUse` (the user answered), `Stop` — yields 5 `LongTaskFinished` and 5 `NeedsPermission` grants, no other grant, and 35 `attention_seq` increments (20 + 5 turn ends, plus 5 permissions, plus 5 permission-turn ends); master switch off → no grant, `attention_seq` unchanged in behaviour (FR-018); the session in view → no event (US1.6); three sessions at once each get their own kind (Principle II); a permission refused so that `Stop` follows `Notification` directly adds nothing and grants nothing (Edge Cases "A permission refused"); a window that connects after an event was noted claims it and is granted the kind noted then (Edge Cases "Reconnection"); after a service restart on the same store, a claim for an event noted before it is refused. In `crates/micold-daemon/tests/copilot_activity.rs` and `pi_activity.rs`: a long turn of each CLI is granted `LongTaskFinished` (US1.9)
- [X] T011 [P] [US1] Client tests in `crates/micold-client/tests/attention_notify.rs`: `attention_notification(session, kind)` builds a `DesktopNotification` whose `kind` is the given kind and whose title and body are `notification_text(kind, …)`; `DaemonMsg::AttentionGranted { kind: NeedsPermission, .. }` handled by the app shows a notification with the **Needs permission** title; an unknown session shows nothing

### Implementation for User Story 1, slice A

- [X] T012 [US1] `TurnChange { PromptSubmitted, Working, AskedUser, Finished }` and `TurnClock` with `change(&mut self, change, now: Uptime, threshold: Duration) -> Option<NotificationKind>` per the data-model table, in `crates/micold-core/src/attention.rs` (T005)
- [X] T013 [US1] `notification_text` gains `kind: NotificationKind` as its first argument in `crates/micold-core/src/attention.rs`; update every caller (T006)
- [X] T014 [US1] `AttentionGranted { session, seq, kind: NotificationKind }` in `crates/micold-core/src/protocol/messages.rs`; `PROTOCOL_VERSION` 29 → 30 in `crates/micold-core/src/protocol/version.rs`; refresh the schema hash (T007)
- [X] T015 [US1] In `crates/micold-daemon/src/hooks.rs`: `classify_hook` returns `HookClass::Ignored` for `"SubagentStop"` (only `"Stop"` maps to `HookKind::Stop`); remove `HooksMap.subagent_stop`, its serde rename and the doc comment that groups it with `Stop`, so `settings_json` no longer registers it. Update `specs/010-daemon-session-persistence/contracts/hooks.md`: `SubagentStop` is no longer registered and is ignored if received (FR-024, research R3) (T008)
- [X] T016 [US1] `pub fn turn_change(event: &ActivityEvent, lifted: bool) -> Option<TurnChange>` in `crates/micold-daemon/src/activity.rs` (T008)
- [X] T017 [US1] In `crates/micold-daemon/src/attention.rs`: `Views` gains `pending: HashMap<SessionId, Vec<(u64, NotificationKind)>>`; `note_event(session, seq, kind, notify: bool)`; `grant(session, seq, current, notify: impl Fn(NotificationKind) -> bool) -> Option<NotificationKind>`; pruning on grant and recorded grant; `forget_session` clears pending (T009)
- [X] T018 [US1] In `crates/micold-daemon/src/state.rs`: `LiveSession.turn: TurnClock` created `NotInTurn` with the live entry (C1); `DaemonState.long_task_threshold` and `set_long_task_threshold` (see "Test threshold" above); `note_activity` calls `turn.change(turn_change(&event, lifted)?, clock::now(), threshold)` on every event before deciding `began_waiting` (C2), and on `began_waiting` passes the returned kind (`TurnFinished` if none, C3) and `catalog.notify(kind)` to `Views::note_event` (C3, C10); the claim path (line ~1243) calls `Views::grant` with `|k| catalog.notify(k)` and answers `AttentionGranted { kind }` (C11) (T010)
- [X] T019 [US1] Client: `DesktopNotification` gains `kind: NotificationKind` in `crates/micold-client/src/features/attention.rs`; `State::attention_notification(session, kind)` in `crates/micold-client/src/app.rs` builds title and body with `notification_text(kind, …)`; `crates/micold-client/src/shell/daemon_sync.rs` passes `AttentionGranted`'s `kind`; the three backends in `crates/micold-client/src/shell/desktop_notify/` and their test helpers carry the field (no icon yet) (T011)
- [X] T020 [US1] User guide `docs/user-guide/settings.md` §Desktop notifications: the app now notifies, out of the box, when a session not in view stops mid-turn for a permission or an answer (**Needs permission**) and when it finishes a turn of a minute or more (**Long task finished**); a shorter turn (**Turn finished**) raises none but still marks the session unread; the title says which; Claude Code's helper agents finishing inside a turn no longer mark a session unread (FR-023, FR-024)

**Checkpoint**: `mise run gate` green. With default settings a short turn raises no notification,
a long turn and a permission stop each raise one, titled by kind.

---

## Phase 4: User Story 1, slice B — Session error (Priority: P1)

**Goal**: A session that ends because of an error — the crash-loop give-up or a CLI-reported error
(Copilot `session.error`) — raises one **Session error** notification in one window, when the kind
and the master switch are on, the session is in view nowhere and a window is connected. Clean
exits, user stops and closes raise nothing; unread is untouched.

**Independent Test**: `crates/micold-daemon/tests/attention_error_notice.rs` (NEW) with two
windows; `crates/micold-client/tests/attention_notify.rs` for the notice's notification.

### Tests for User Story 1, slice B (MANDATORY — Constitution Principle I) ⚠️

- [ ] T021 [P] [US1] Daemon unit tests in `crates/micold-daemon/src/activity.rs`: `copilot_event` sets `error: true` on `session.error` and `error: false` on `session.shutdown` (C8); every other constructor of `ActivityEvent::Ended` sets `error: false`; the FSM's resulting `ActivitySignal::Ended { reason }` is the same whatever `error` is
- [ ] T022 [P] [US1] `Views::error_notice_target()` unit tests in `crates/micold-daemon/src/attention.rs`: the last entry of `focus_order` when there is one; else the lowest `ClientId` among connections with a stored `WindowView`; else `None` (C14)
- [ ] T023 [P] [US1] Protocol tests in `crates/micold-core/tests/protocol_roundtrip.rs` and `crates/micold-core/tests/schema_hash.rs`: `DaemonMsg::SessionErrorNotice { project, session }` round-trips; it has no `req`; `PROTOCOL_VERSION` is 31; the schema hash is updated (W5.2)
- [ ] T024 [P] [US1] Integration tests in `crates/micold-daemon/tests/attention_error_notice.rs` (NEW), modelled on `supervision_giveup.rs` and `attention_events.rs`: a crash loop that ends in give-up sends exactly one `SessionErrorNotice` to the focused window and none to the other (C7, C13, US1.4); `note_activity(Ended { error: true })` on a live Copilot session sends one (C8); a second error `Ended` on an already ended session sends none; a crash followed by `Restart` sends none (C9, Edge Cases "repeated crashes"); a clean exit (`SupervisionAction::Stop`), `Ended { error: false }`, a user stop and a close send none (US1.5); the session in view in a window → none (US1.6); no window connected → none, and none after a window connects later (FR-007, US1.10); `notification_kinds.session_error: false` in the settings file the catalog loads → none, and master off → none; `attention_seq` and unread are unchanged by every error ending (Edge Cases "Unread is unchanged")
- [ ] T025 [P] [US1] Client tests in `crates/micold-client/tests/attention_notify.rs`: `DaemonMsg::SessionErrorNotice` for a known session shows one notification of kind `SessionError` titled `<session> stopped with an error`, built through `attention_notification(session, SessionError)` (T3); for an unknown project or session it shows nothing

### Implementation for User Story 1, slice B

- [ ] T026 [US1] `ActivityEvent::Ended` gains `error: bool` in `crates/micold-daemon/src/activity.rs`; `copilot_event` sets it for `session.error` only; every other construction site in `crates/micold-daemon/src/` and its tests sets `false` (T021)
- [ ] T027 [US1] `Views::error_notice_target(&self) -> Option<ClientId>` in `crates/micold-daemon/src/attention.rs` (T022)
- [ ] T028 [US1] `DaemonMsg::SessionErrorNotice { project: PathBuf, session: SessionId }` in `crates/micold-core/src/protocol/messages.rs`; `PROTOCOL_VERSION` 30 → 31 in `crates/micold-core/src/protocol/version.rs`; refresh the schema hash (T023)
- [ ] T029 [US1] In `crates/micold-daemon/src/state.rs`: one `error_ending(project, session)` helper that sends `SessionErrorNotice` to `Views::error_notice_target()` through that client's outbound channel, as `RevealSession` reaches one window, only when `catalog.notify(SessionError)`, `!Views::is_in_view(session)` and a target exists, and otherwise does nothing and keeps nothing (C13); called from the `SupervisionAction::GiveUp` arm of the supervision tick (C7) and from `note_activity` when an `Ended { error: true }` reaches a session whose signal was not already `Ended` (C8). Neither path touches `attention_seq` or unread (T024)
- [ ] T030 [US1] Client: `crates/micold-client/src/shell/daemon_sync.rs` handles `DaemonMsg::SessionErrorNotice` by `app.core.attention_notification(session, NotificationKind::SessionError)` and the same `show_attention_notification` path as a grant; clicking it reveals the session as 039 does for any notification (FR-019) (T025)
- [ ] T031 [US1] User guide `docs/user-guide/settings.md` §Desktop notifications: a session that ends because of an error (it crashed until the app gave up restarting it, or its AI CLI reported an error) raises one **Session error** notification; closing or stopping a session, or a CLI that exits normally, raises none; an error does not mark the session unread (FR-023)

**Checkpoint**: `mise run gate` green. User Story 1 is complete with the defaults.

---

## Phase 5: User Story 2 — choose which kinds I get (Priority: P2)

**Goal**: Settings shows, under **Desktop notifications**, one row per kind in `NotificationKind::ALL`
order, each the shared `Checkbox` with the kind's icon, name and one-line note, and under **Long
task finished** the long-task threshold field (seconds, 10–3600, default 60, D4 = B); the rows and
the field are disabled while the master switch is off. Saving sends the four values and the
threshold to the service, which stores them, pushes them to every window and applies them to the
next event without a restart.

**Independent Test**: `crates/micold-daemon/tests/settings_notification_kinds.rs` (NEW);
`crates/micold-daemon/tests/settings_long_task_threshold.rs` (NEW);
`crates/micold-core/tests/settings_long_task_threshold.rs` (NEW);
`crates/micold-client/tests/features_settings.rs`; quickstart §B5–B6, §B8 and §B9.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T032 [P] [US2] Protocol tests in `crates/micold-core/tests/protocol_roundtrip.rs` and `crates/micold-core/tests/schema_hash.rs`: `DaemonSettings.notification_kinds` and `ClientMsg::SettingsSet.notification_kinds: Option<NotificationKinds>` round-trip, `None` included; `PROTOCOL_VERSION` is 32; the schema hash is updated (W5.3, W5.4). Update `crates/micold-core/tests/settings_contract_examples.rs` if it lists the settings file's fields
- [ ] T033 [P] [US2] Integration tests in `crates/micold-daemon/tests/settings_notification_kinds.rs` (NEW), modelled on `settings_desktop_notifications.rs`, two windows, test threshold 200 ms: `SettingsSet { notification_kinds: Some(k) }` stores all four values, persists them to `settings.json` and pushes `SettingsChanged` with them to both windows (W5.3, W5.4); `notification_kinds: None` leaves them unchanged; after a service restart on the same settings directory they are as set (SC-006, US2.7); with **Turn finished** on and **Long task finished** off, a short turn's claim is granted `TurnFinished` and a long turn's is refused (US2 Independent Test); for each kind with only that kind on, events of that kind are granted and events of the other kinds are not (SC-002; Session error through the give-up path); `attention_seq` and unread are the same with every switch off as with every switch on (SC-003, US2.3); an event while a kind is off is not granted after the kind is turned on, and an event while the master switch is off is not granted after it is turned on (C15, FR-013, US2.4, US2.6)
- [ ] T034 [P] [US2] Client tests in `crates/micold-client/tests/icons_font.rs` and `crates/micold-client/tests/icons.rs`: `Icon::{NeedsPermission, SessionError, LongTaskFinished, TurnFinished}` are in `Icon::ALL` with the codepoints of Material Symbols `pan_tool`, `error`, `task_alt`, `chat_bubble`, each present in the bundled font (I1); the four are distinct. In `crates/micold-client/tests/notification_icon.rs` (NEW): `notification_icon::icon(kind)` maps each kind to its variant, one-to-one (I2, FR-015). Add `--test notification_icon` to the enumerated `cargo test -p micold-client` list in `.github/workflows/ci.yml`
- [ ] T035 [P] [US2] Client tests in `crates/micold-client/tests/features_settings.rs`: the draft opened from `DaemonSettings` holds its `notification_kinds`; `SettingsMsg::NotificationKindToggled(kind, on)` changes only that kind in the draft; saving sends `SettingsSet { notification_kinds: Some(<whole draft>) }` (S3, W5.4); with the draft's `desktop_notifications` false the kind values are kept unchanged and the view model marks the rows not toggleable (S2, FR-012, US2.5); turning the master switch back on leaves the kind values as they were (US2.6); a `SettingsChanged` from another window updates the stored values
- [ ] T056 [P] [US2] Settings tests in `crates/micold-core/tests/settings_long_task_threshold.rs` (NEW), modelled on `settings_env_include.rs`: `Settings::default().long_task_threshold_secs == LONG_TASK_THRESHOLD.as_secs()` (60, FR-025); `MIN_LONG_TASK_THRESHOLD_SECS == 10`, `MAX_LONG_TASK_THRESHOLD_SECS == 3600`; `clamp_long_task_threshold` maps 0 → 10, 9 → 10, 10 → 10, 600 → 600, 3600 → 3600, 3601 → 3600, `u64::MAX` → 3600; a settings file written before this feature loads 60 (US2.13); a file with 5 loads 10 and one with 99999 loads 3600 (clamp on read, FR-026, US2.13); a round trip keeps 120; an unreadable file gives 60 (Edge Cases "Settings file unreadable"). In `crates/micold-daemon/src/catalog.rs` tests: `set_long_task_threshold(5)` stores 10, `(99999)` stores 3600, `(120)` stores 120, `long_task_threshold()` returns it as a `Duration`, and `persist_service_settings` writes the field into the file it writes
- [ ] T057 [P] [US2] Protocol tests in `crates/micold-core/tests/protocol_roundtrip.rs` and `crates/micold-core/tests/schema_hash.rs`, beside T032's: `DaemonSettings.long_task_threshold_secs` and `ClientMsg::SettingsSet.long_task_threshold_secs: Option<u64>` round-trip, `None` included, in the same version 32 schema hash (W5.6). Update `crates/micold-core/tests/settings_contract_examples.rs` if it lists the settings file's fields
- [ ] T058 [P] [US2] Integration tests in `crates/micold-daemon/tests/settings_long_task_threshold.rs` (NEW), modelled on `settings_desktop_notifications.rs`, two windows, no test override: `SettingsSet { long_task_threshold_secs: Some(20) }` stores 20, persists it to `settings.json` and pushes `SettingsChanged` with it to both windows (W5.6); `None` leaves it unchanged; `Some(5)` stores 10 and `Some(99999)` stores 3600 (FR-026); after a service restart on the same settings directory it is as set (SC-006, US2.7); `DaemonState`'s effective threshold follows the setting after each `SettingsSet` with no restart, and a turn begun before the change is classified with the value in force at its `Stop` (assert through `DaemonState::effective_long_task_threshold()`, the accessor the `Stop` path reads (T063), FR-013, US2.11, Edge Cases "Threshold changed while a turn is running"); with `set_long_task_threshold(200 ms)` the override wins over the setting; `attention_seq` and unread are unchanged by any threshold value (FR-018)
- [ ] T059 [P] [US2] `TurnClock` and kind tests in `crates/micold-core/src/attention.rs`: for a threshold T of 10 s, 60 s and 3600 s, a finish at `since + T` is `LongTaskFinished` and at `since + T − 1 ms` is `TurnFinished` (SC-008); a turn started under one threshold and finished with another passed to `change` is classified by the one passed at the finish (FR-013); `NotificationKind::LongTaskFinished.description()` is "A session finished a turn at least as long as the long-task threshold." and contains no duration (data-model; replaces T001's "a minute" check, which the implementation task updates)
- [ ] T060 [P] [US2] Client tests in `crates/micold-client/tests/features_settings.rs`: the draft opened from `DaemonSettings` shows `long_task_threshold_secs` as text ("60" by default, US2.10); `SettingsMsg::LongTaskThresholdChanged(text)` edits only that text; saving "20" sends `SettingsSet { long_task_threshold_secs: Some(20) }` (S6, US2.11); saving "9", "3601", "abc" or "" refuses the save with the S6 message on `FieldId::SettingsLongTaskThreshold` and sends nothing (US2.12, FR-026); with the draft's `desktop_notifications` false the field is not editable and keeps its value (FR-012); with **Long task finished** off it stays editable (FR-025); a `SettingsChanged` from another window updates the stored value. In `crates/micold-client/tests/settings_sections.rs`: the Desktop notifications section shows the threshold field directly after the **Long task finished** row, indented as the kind rows, labelled "Long-task threshold" with supporting text "Seconds, 10–3600" (S5, US2.10)
- [ ] T036 [P] [US2] Component tests in `crates/micold-client/src/ui/material/checkbox.rs` (`#[cfg(test)]`) and `crates/micold-client/tests/icon_roles.rs`: `Checkbox::new(…).icon(Icon)` is chainable and keeps label, checked state and toggle; the glyph is drawn in the label's colour role, so it holds ≥3:1 against the row's background in the light and the dark theme for an enabled row (S4, FR-017); a disabled row (no `on_toggle`) shows the glyph in the disabled role as the label is, exempt from the 3:1 gate (FR-017). In `crates/micold-client/tests/settings_sections.rs`: the Desktop notifications section lists, after the master switch, four kind rows in `NotificationKind::ALL` order, each with the kind's name and `description()` as its note, indented one spacing step (S1, FR-009, US2.1, US2.9: no per-CLI rows)

### Implementation for User Story 2

- [ ] T037 [US2] `DaemonSettings.notification_kinds: NotificationKinds` and `ClientMsg::SettingsSet.notification_kinds: Option<NotificationKinds>` in `crates/micold-core/src/protocol/messages.rs`; `PROTOCOL_VERSION` 31 → 32 in `crates/micold-core/src/protocol/version.rs`; refresh the schema hash; fill every `SettingsSet` and `DaemonSettings` construction site in the workspace (T032)
- [ ] T038 [US2] Service: `Catalog::set_notification_kinds` persisting through `persist_service_settings` and `DaemonSettings` carrying the value in `crates/micold-daemon/src/catalog.rs`; `DaemonState::set_notification_kinds(kinds)` in `crates/micold-daemon/src/state.rs` that, when any kind turns on, first records every session's current `attention_seq` as granted, as `set_desktop_notifications` does when the master switch turns on (C15); handle `SettingsSet.notification_kinds` in `crates/micold-daemon/src/server.rs` beside `desktop_notifications`, then push `SettingsChanged` (T033)
- [ ] T039 [P] [US2] `Icon::{NeedsPermission, SessionError, LongTaskFinished, TurnFinished}` with their codepoints, names and `Icon::ALL` entries in `crates/micold-client/src/icons.rs`; `docs/user-guide/icons.md` lists the four kind icons with what each means (FR-023, I1) (T034)
- [ ] T040 [P] [US2] `crates/micold-client/src/notification_icon.rs` (NEW, render-free) with `pub fn icon(kind: NotificationKind) -> Icon`, the only kind→icon mapping (I2); `pub mod notification_icon` in `crates/micold-client/src/lib.rs` (T034)
- [ ] T041 [P] [US2] `Checkbox::icon(self, Icon) -> Self` builder method in `crates/micold-client/src/ui/material/checkbox.rs`, the glyph before the label in the label's colour role (S4, FR-022) (T036)
- [ ] T042 [US2] Settings draft: `notification_kinds` in the draft, `SettingsMsg::NotificationKindToggled(NotificationKind, bool)`, the save path sending the whole value, and the `SettingsChanged` update, in `crates/micold-client/src/features/settings.rs` (T035)
- [ ] T043 [US2] Settings view in `crates/micold-client/src/ui/settings/environment.rs`: below **Desktop notifications**, for each `NotificationKind::ALL`, `field_note(Checkbox::new(kind.name(), on, roles).icon(notification_icon::icon(kind)), Some(kind.description()))` indented one spacing step, with `on_toggle` only while the draft's `desktop_notifications` is true (S1, S2) (T036)
- [ ] T044 [US2] Showcase: the Checkbox section in `crates/micold-client/src/showcase/sections/controls.rs` shows the four kind rows (icon, name, note), checked and unchecked, enabled and disabled, in the light and the dark theme (I8, FR-022)
- [ ] T061 [US2] In `crates/micold-core/src/settings.rs`: `MIN_LONG_TASK_THRESHOLD_SECS`, `MAX_LONG_TASK_THRESHOLD_SECS`, `clamp_long_task_threshold`, `default_long_task_threshold_secs()` returning `LONG_TASK_THRESHOLD.as_secs()` (C6), and `long_task_threshold_secs` on `Settings` and its file form with that serde default, clamped in `into_settings`, exactly as `env_include_timeout_secs` (lines ~50–90, ~126, ~410, ~501); in `crates/micold-core/src/attention.rs`, `LongTaskFinished.description()` becomes the threshold-free text and T001's description test is updated to it (T056, T059)
- [ ] T062 [US2] `DaemonSettings.long_task_threshold_secs: u64` and `ClientMsg::SettingsSet.long_task_threshold_secs: Option<u64>` in `crates/micold-core/src/protocol/messages.rs`, in T037's 31 → 32 bump and schema-hash refresh; fill every `SettingsSet` and `DaemonSettings` construction site in the workspace (T057)
- [ ] T063 [US2] Service: `Catalog::set_long_task_threshold(secs)` (clamped), `long_task_threshold() -> Duration`, `DaemonSettings` carrying the value and `persist_service_settings` carrying it over, in `crates/micold-daemon/src/catalog.rs`; in `crates/micold-daemon/src/state.rs`, `long_task_threshold` becomes `long_task_threshold_override: Option<Duration>` set by `set_long_task_threshold`, and `pub fn effective_long_task_threshold(&self) -> Duration` (the override, else `catalog.long_task_threshold()`) is what both readers (`note_activity`, line ~3351, and line ~3493) call, on every event (C2); handle `SettingsSet.long_task_threshold_secs` in `crates/micold-daemon/src/server.rs` beside `env_include_timeout_secs`, then push `SettingsChanged` (T058)
- [ ] T064 [US2] Client: in `crates/micold-client/src/features/settings.rs`, the draft's threshold text seeded from `DaemonSettings`, `SettingsMsg::LongTaskThresholdChanged(String)`, a `long_task_threshold()` check modelled on `timeout()` (S6) on the save path, the `SettingsChanged` update; `FieldId::SettingsLongTaskThreshold` in `crates/micold-client/src/features/window.rs`; in `crates/micold-client/src/ui/settings/environment.rs`, the `TextField` of S5 under the **Long task finished** row, with `on_input` only while the draft's `desktop_notifications` is true, and its message pair in the file's field/message table (T060)
- [ ] T065 [US2] Showcase: under a Long task finished kind row in `crates/micold-client/src/showcase/sections/controls.rs`, the threshold field valid (60), refused (5, with the S6 message) and disabled, in the light and the dark theme (S7, FR-022)
- [ ] T066 [US2] User guide `docs/user-guide/settings.md` §Desktop notifications: the long-task threshold next to **Long task finished**, in seconds, 10–3600, 60 by default; it decides between **Long task finished** and **Turn finished** even while the former is off; a change applies to the next turn end, one already running included; out-of-range input is refused, an out-of-range value in the file is clamped; the settings file key `long_task_threshold_secs`. Reword M1's fixed wording (line ~157, "a turn that took a minute or more", and any other fixed-60 s wording in that section) to "at least as long as the long-task threshold (60 seconds by default)" (FR-023, FR-025, FR-026)
- [ ] T045 [US2] User guide `docs/user-guide/settings.md` §Desktop notifications: the four kind switches, their order, defaults and notes; the master switch turns all off and greys the rows, which keep their positions; changes apply to the next event without a restart, and events while off are never notified later; the switches apply to every AI CLI; unread marks do not depend on them; the settings file key `notification_kinds` and its four fields (FR-023)

**Checkpoint**: `mise run gate` green; visual pass of Settings and the showcase (quickstart §B5,
§B8, §B9) recorded in `specs/613-notification-kinds/visual-pass/`.

---

## Phase 6: User Story 3 — tell kinds apart at a glance by their icon (Priority: P3)

**Goal**: Each desktop notification carries its kind's icon: a white Material Symbols glyph on a
coloured rounded tile, rasterised once per run by the client from the bundled font, handed to the
OS as `image-data` on Linux and as a PNG file on Windows and macOS. Where the OS shows no app
icon the notification still appears, titled by kind.

**Independent Test**: `crates/micold-client/tests/notification_icon.rs` (contrast and
distinctness gates); the backends' request-builder tests; quickstart §B2–B4 and §B7.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T046 [P] [US3] Rasteriser tests in `crates/micold-client/tests/notification_icon.rs`: `render(kind, px)` returns a `px`×`px` RGBA image for 16, 64 and 256; each kind's tile colour has relative luminance in [0.10, 0.30], so ≥3:1 against white and against black, and the white glyph holds ≥3:1 on the tile (I3, FR-017); the glyph is non-empty and centred (its ink bounding box's centre within 1 px of the image centre at 64); the four glyph masks drawn in one colour at 16×16 differ pairwise in at least 10% of their pixels (I4, FR-017)
- [ ] T047 [P] [US3] Backend request tests: in `crates/micold-client/src/shell/desktop_notify/linux.rs` the request for a notification of each kind carries the hint `image-data` `(iiibiiay)` = width, height, rowstride `4 × width`, has-alpha `true`, 8, 4 and the bytes of `render(kind, 64)` (I5); in `windows.rs` the toast for a kind with an icon path carries that path with `IconCrop::Square` and the kind's name, and without a path carries no image (I6, FR-016); in `macos.rs` the banner carries `image_path` when a path is given and none otherwise (I6, I7). In `crates/micold-client/src/shell/desktop_notify/mod.rs` tests: the icon files are written once per run as `<dir>/notification-icons/<kind>.png`, overwriting old ones, decode as 256×256 PNG; a directory that cannot be written yields no paths, the notification is still shown, and the failure is logged once per run (I6)

### Implementation for User Story 3

- [ ] T048 [US3] Dependencies in `Cargo.toml` (workspace) and `crates/micold-client/Cargo.toml`: `tiny-skia` 0.11 with the `png-format` feature as a normal dependency of `micold-client`; `ttf-parser` stays a normal dependency (research R7)
- [ ] T049 [US3] `render(kind, px) -> Rgba` (glyph outline from the bundled Material Symbols font via `ttf-parser`, filled white with `tiny-skia`, centred on a rounded-square tile) and the four tile colours in `crates/micold-client/src/notification_icon.rs` (I3, I4) (T046)
- [ ] T050 [US3] Linux: the `image-data` hint built from `render(kind, 64)` in the request builder of `crates/micold-client/src/shell/desktop_notify/linux.rs` (I5) (T047)
- [ ] T051 [US3] Icon files: once per run, `crates/micold-client/src/shell/desktop_notify/mod.rs` writes `render(kind, 256)` as PNG to `<ProjectDirs data dir>/notification-icons/<kind>.png` (data dir as `shell/startup.rs` resolves it) and hands the paths to the backend; `crates/micold-client/src/shell/desktop_notify/windows.rs` calls `Toast::icon(path, IconCrop::Square, kind.name())`; `crates/micold-client/src/shell/desktop_notify/macos.rs` calls `Notification::image_path(path)`; a missing path shows the notification without an icon (I6, I7, FR-016) (T047)
- [ ] T052 [US3] User guide `docs/user-guide/settings.md` §Desktop notifications: each notification shows its kind's icon, the same as beside its switch; the title names the kind where the system shows no app icon; which systems show it (FR-016, FR-023)

**Checkpoint**: `mise run gate` green; `scripts/build-lock.sh cargo check --workspace --target
aarch64-apple-darwin` (the macOS arm changed); visual pass of the four notifications in light and
dark (quickstart §B2–B4, §B7) recorded in `specs/613-notification-kinds/visual-pass/`.

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: no code; done by the close unit in the close PR.

- [ ] T053 [P] `docs/development/architecture.md` §Session attention: the kinds, `TurnClock`, the pending kinds in `Views`, `SessionErrorNotice`, the per-kind switches, the icon rasteriser, and `SubagentStop` ignored, with links to `specs/613-notification-kinds/research.md`
- [ ] T054 Run quickstart.md Part A and record the result; collect the Part B passes recorded in M3 and M4, and run any step not yet covered, in `specs/613-notification-kinds/visual-pass/`. Record that SC-004 (icon recognition) and SC-005 (finding a switch in 30 s) are human trials, covered here only by proxies: the distinctness gate (T046) and the Settings layout (T036)
- [ ] T055 Confirm CI's Linux, macOS and Windows legs ran the feature's tests (Principle VI) and SC-007 through CI's sandbox job

---

## Dependencies & Execution Order

### Phase Dependencies

- **Foundational (Phase 2)**: blocks every story.
- **US1 slice A (Phase 3)**: after Phase 2.
- **US1 slice B (Phase 4)**: after Phase 3 (`attention_notification(session, kind)`, `catalog.notify`).
- **US2 (Phase 5)**: after Phase 4 (its SC-002 test drives Session error).
- **US3 (Phase 6)**: after Phase 5 (`notification_icon::icon` and the `Icon` variants).
- **Polish (Phase 7)**: after all stories.

### Within Each Phase

- Test tasks first; they fail before their implementation task (Principle I).
- Core (`micold-core`) before daemon, daemon before client.
- The phase's user-guide task ships in the same milestone (Principle VII).

### Parallel Opportunities

- T001 and T002; within each phase every test task marked [P].
- In Phase 5, T039, T040 and T041 touch different files and can run together.
- In Phase 5, the threshold tasks T056–T066 (D4 = B) sit beside the switch tasks: T056–T060 are
  tests, written before T061–T066; T062 shares T037's protocol bump, and T064 and T043 both edit
  `environment.rs` (not [P] with each other).

---

## Parallel Example: User Story 1, slice A

```bash
Task: "TurnClock unit tests in crates/micold-core/src/attention.rs"      # T005
Task: "Protocol tests in crates/micold-core/tests/protocol_roundtrip.rs"  # T007
Task: "Views unit tests in crates/micold-daemon/src/attention.rs"         # T009
Task: "Client tests in crates/micold-client/tests/attention_notify.rs"    # T011
```

---

## Implementation Strategy

### MVP First

Phase 2 + Phase 3: with the defaults, short turns stop notifying and permission stops and long
turns are titled by kind. That alone answers the issue's main complaint.

### Incremental Delivery

1. M1: awaiting-input kinds with defaults (MVP).
2. M2: Session error notifications.
3. M3: per-kind switches in Settings, with their icons.
4. M4: the icons in the desktop notifications.
5. Close: architecture doc and recorded passes.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Awaiting-input kinds, notified by their defaults 🎯 MVP

- **Tasks**: T001–T020
- **Deliverable**: On `main`, a session not in view raises a desktop notification only when it
  stops mid-turn for a permission or finishes a turn of 60 s or more, titled by its kind; a short
  turn raises none but still marks the session unread; Claude Code's `SubagentStop` no longer ends
  a turn or marks a session unread. The user guide says so.
- **Satisfies**: US1 acceptance scenarios 1–3, 6–11; FR-001–FR-003, FR-005, FR-006, FR-008,
  FR-010 (defaults), FR-018, FR-024; SC-001 (except its error ending); wire W5.1
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-daemon --lib attention activity hooks`; `scripts/build-lock.sh cargo test -p micold-daemon --test attention_claims --test attention_events --test copilot_activity --test pi_activity`; `scripts/build-lock.sh cargo test -p micold-client --test attention_notify`
- **Depends on**: —
- **Tier**: full

### M2 — Session error notifications

- **Tasks**: T021–T031
- **Deliverable**: On `main`, a session not in view that crashes until the service gives up, or
  whose Copilot CLI reports an error, raises one **Session error** notification in one window;
  clean exits, stops and closes raise none, and unread marks are untouched. The user guide says so.
- **Satisfies**: US1 acceptance scenarios 4, 5, 10 (errors); FR-004, FR-007; SC-001 (its error
  ending); wire W5.2
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test attention_error_notice`; `scripts/build-lock.sh cargo test -p micold-daemon --lib activity attention`; `scripts/build-lock.sh cargo test -p micold-client --test attention_notify`; `mise run test-core`
- **Depends on**: M1
- **Tier**: full

### M3 — Per-kind switches in Settings

- **Tasks**: T032–T045, T056–T066 (T056–T066: the adjustable long-task threshold, D4 = B)
- **Deliverable**: On `main`, Settings shows four kind switches with their icons and notes under
  **Desktop notifications**, and next to **Long task finished** a long-task threshold in seconds
  (10–3600, default 60), all greyed while it is off; changes apply to the next event without a
  restart and survive a restart; the showcase and the user guide (settings and icons) show them.
- **Satisfies**: US2 acceptance scenarios 1–13; FR-009, FR-011–FR-014, FR-015 (Settings side),
  FR-017 (Settings contrast), FR-022, FR-023, FR-025, FR-026; SC-002, SC-003, SC-005, SC-006,
  SC-008; wire W5.3, W5.4, W5.6
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-client --lib checkbox`; `scripts/build-lock.sh cargo test -p micold-daemon --lib catalog`; `scripts/build-lock.sh cargo test -p micold-daemon --test settings_notification_kinds --test settings_long_task_threshold --test settings_desktop_notifications`; `scripts/build-lock.sh cargo test -p micold-client --test features_settings --test icons_font --test icons --test icon_roles --test settings_sections --test notification_icon`; quickstart §B5, §B6, §B8, §B9 via the `visual-pass` skill
- **Depends on**: M2
- **Tier**: full

### M4 — Kind icons in desktop notifications

- **Tasks**: T046–T052
- **Deliverable**: On `main`, every desktop notification carries its kind's icon (Linux
  `image-data`, Windows and macOS PNG files), the same icon as in Settings, legible on light and
  dark desktops; where the system shows no app icon the title still names the kind.
- **Satisfies**: US3 acceptance scenarios 1–5; FR-015, FR-016, FR-017, FR-021; SC-004 (shape
  distinctness gate)
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --test notification_icon`; `scripts/build-lock.sh cargo test -p micold-client --lib desktop_notify`; `scripts/build-lock.sh cargo check --workspace --target aarch64-apple-darwin`; quickstart §B2–B4, §B7 via the `visual-pass` skill
- **Depends on**: M3
- **Tier**: full

Phase 7 (T053–T055) changes no code: the close unit does it in the close PR.
