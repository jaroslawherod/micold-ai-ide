---
description: "Task list for feature 039 — notify when a session needs attention, and track unread sessions"
---

# Tasks: Notify When a Session Needs Attention, and Track Unread Sessions

**Input**: Design documents from `/specs/039-session-attention-notifications/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: Per Constitution Principle I (Test-First Development, NON-NEGOTIABLE), test tasks are
MANDATORY and must be observed failing before their implementation. Every phase lists its failing
tests first. The glue exception is claimed where plan.md *Constitution Check* records it: the system call of
each backend in `crates/micold-client/src/shell/desktop_notify/` and the `iced::window` task per
step in `shell/window_raise.rs` (one call per step, no branch); the wiring lines in `src/main.rs`,
`shell/daemon_sync.rs`, `shell/startup.rs` and `shell/persist.rs` (T016, T033, T089, T110), which
decide nothing; and `src/ui/` composition. All are verified by the recorded quickstart §B and §C.

**Documentation**: Per Constitution Principle VII, each slice that changes what the user sees
carries its own user-guide task in the milestone that ships it (CI's user-guide gate).

**Cross-platform**: Per Constitution Principle VI, the only `cfg` arms are in
`crates/micold-client/src/shell/desktop_notify/` and the Linux arm of `shell/window_raise.rs`.
Core and daemon code never name an operating system. Before pushing a `cfg` arm, cross-check with
`cargo check -p micold-client --target aarch64-apple-darwin`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story the task belongs to (US1–US4)
- Every description carries an exact file path

## Path Conventions

Three-crate workspace: `crates/micold-core/` (render-free rules, wire, stores),
`crates/micold-daemon/` (the session service), `crates/micold-client/` (features, shell, UI,
showcase). Build and test through `mise run <task>` (CLAUDE.md). "NEW" marks a file that does not
exist yet. Contract ids: `W…` [contracts/wire.md](./contracts/wire.md), `N…`
[contracts/desktop-notification.md](./contracts/desktop-notification.md), `U…`
[contracts/unread-mark.md](./contracts/unread-mark.md), `A…` [data-model.md](./data-model.md).
Contract ids are written in round brackets, `(U3)`, `(A5)`.

Ids in square brackets, `[A1]` and `[U69]`, are the behavior ids of
[tdd/test-list.md](./tdd/test-list.md). `speckit-tdd-run` ticks a task when the behaviors it
carries are done; a task without one (dependencies, `shell/` and `ui/` glue, docs, recorded
passes) is left to `speckit-implement`. T118 to T121 are each story's final run of its outer
tests. T118 carries no behavior id: story 1's behaviors are all done by the end of M2, and T118 is
ticked in M3, by `speckit-implement`, when CI's three legs have run them. T118 to T123 were added
after T117 and keep their numbers; each stands in the phase it belongs to.

Each wire change bumps `PROTOCOL_VERSION` in `crates/micold-core/src/protocol/version.rs` by one,
in one edit, and updates `crates/micold-core/tests/schema_hash.rs`. The numbers below (21 to 26)
are the plan's. Each bump takes the next free number: one more than `PROTOCOL_VERSION` on `main`
when the milestone is implemented. When another feature has taken a number by then, or M7 closes
without its wire change (T091), every later number of this feature moves with it, and the protocol
test of that milestone asserts the number actually taken.

---

## Phase 1: Setup (Shared Infrastructure)

No setup tasks: no new crate, binary or configuration. Each dependency is added by the slice that
first uses it (T029, T037, T096).

---

## Phase 2: Foundational (Blocking Prerequisites)

No foundational tasks. What every story builds on — the view report and the attention sequence —
is the first slice of story 1 and ships with a deliverable of its own.

---

## Phase 3: User Story 1, slice A — the service knows what is in view and counts attention events (Priority: P1) 🎯 MVP

**Goal**: Each window tells the session service which session it has in view. The service adds one
to a session's `attention_seq` on every change into awaiting input that happens while the session
is in view in no window, stores it, and sends it to every window. Nothing is shown yet.

**Independent Test**: `crates/micold-daemon/tests/attention_events.rs` with two connections;
`crates/micold-client/tests/attention_view_report.rs`.

### Tests for User Story 1, slice A (MANDATORY — Constitution Principle I) ⚠️

- [X] T001 [P] [US1] [A10] [U1] [U2] [U3] [U4] [U5] Unit tests for `in_view(ViewFacts) -> Option<SessionId>` in `crates/micold-core/src/attention.rs` (NEW, `#[cfg(test)]`): the selected session when `window_focused` and not `main_area_taken`; `None` when the window is unfocused (US1.3); `None` when the main area is taken (US1.9); `None` with no selected session; the value does not depend on which tab of the session is shown (US1.10) — `ViewFacts` has no field for it (FR-002, FR-016, spec Terms)
- [X] T002 [P] [US1] [U6] [U7] [U8] Store tests in `crates/micold-core/src/store.rs`: a `StoredSession` written without `attention_seq` reads as `0`; a round trip keeps the value; `schema_version` is unchanged (FR-008a, research R1)
- [X] T003 [P] [US1] [U12] Protocol tests in `crates/micold-core/src/protocol/messages.rs` and `crates/micold-core/tests/schema_hash.rs`: `ClientMsg::WindowView { focused, in_view }` and `SessionSummary::attention_seq` encode and decode; `PROTOCOL_VERSION` is 21 (W1)
- [X] T004 [P] [US1] [U50] [U51] [U52] [U53] Unit tests for `Views` in `crates/micold-daemon/src/attention.rs` (NEW): `set_view` stores one report per connection; a report with `focused: false` is stored with `in_view: None` whatever it carried; `is_in_view(session)` is true while any stored report names it; `remove(client)` forgets the report (W1.1, W1.2)
- [X] T005 [P] [US1] [A2] [A3] [A5] [A6] [U69] [U70] [U71] [U72] [U73] [U74] [U75] [U76] [U77] [U78] [U79] Integration tests in `crates/micold-daemon/tests/attention_events.rs` (NEW), two connections: a change into `AwaitingInput` with the session in view nowhere adds one to `attention_seq` in the `CatalogChanged` both receive (W1.3, A2); with one connection reporting it in view nothing changes (A3, FR-002); a repeated waiting signal changes nothing (FR-003, US1.5); working and awaiting input again adds one more (US1.6); three sessions changing at once each add one to their own (FR-009, US1.12); with no connection the change still counts (A6); after the connection that had it in view closes, the next change counts; a `WindowView` from a connection attached to no project is accepted (W1.6) and gets no `OperationOk` (W1.5); the value survives a restart of the service on the same store directory; a removed session leaves the catalog (A5); a session that ends adds nothing to its sequence (FR-005)
- [X] T006 [P] [US1] [A3] [A9] [A10] [U111] [U112] [U113] [U114] [U115] [U116] [U176] [U177] [U178] Client tests in `crates/micold-client/tests/attention_view_report.rs` (NEW) for `features::attention::view_report(&mut State, ViewFacts) -> Option<WindowView>`: one report after each `Welcome`, also when nothing is in view; afterwards a report only when the derived value differs from the last one sent; losing focus reports `focused: false, in_view: None`; opening Settings reports `in_view: None` and leaving it reports the session again; a reconnect resets `sent_view` (W1.1). In the same file, for `State::view_facts(window_focused) -> ViewFacts` of `crates/micold-client/src/app.rs`: `selected` is the active project's selected session and `window_focused` is passed through; with Settings open `main_area_taken` is true (US1.9); showing another tab of the selected session leaves the facts equal (US1.10). Add `--test attention_view_report` to the enumerated `cargo test -p micold-client` list in `.github/workflows/ci.yml`, so the macOS and Windows legs run it (T122 adds the macOS and Windows backend step later; this line is the list's). In `crates/micold-client/src/catalog_sync.rs` tests: `reconcile_catalog` copies `attention_seq` into `Workspace::sessions`

### Implementation for User Story 1, slice A

- [X] T007 [US1] [U1] [U2] [U3] [U4] [U5] `ViewFacts { window_focused, main_area_taken, selected }` and `in_view` in `crates/micold-core/src/attention.rs` (NEW); `pub mod attention` in `crates/micold-core/src/lib.rs` (T001)
- [X] T008 [US1] [U6] [U7] [U8] `attention_seq: u64` on `Session` in `crates/micold-core/src/session.rs` and, with `#[serde(default)]`, on `StoredSession` in `crates/micold-core/src/store.rs`, with both conversions (T002)
- [X] T009 [US1] [U12] `SessionSummary::attention_seq`, `WindowView` and `ClientMsg::WindowView` in `crates/micold-core/src/protocol/messages.rs`; `PROTOCOL_VERSION` 20 → 21 in `crates/micold-core/src/protocol/version.rs` (T003)
- [X] T010 [US1] [U50] [U51] [U52] [U53] `Views` with `views: HashMap<ClientId, WindowView>`, `set_view`, `remove`, `is_in_view` in `crates/micold-daemon/src/attention.rs` (NEW, pure, no I/O); declare the module in `crates/micold-daemon/src/lib.rs` (T004)
- [X] T011 [US1] [U69] [U72] [U73] [U74] [U77] `Catalog::mark_attention(session)` in `crates/micold-daemon/src/catalog.rs`: adds one to `attention_seq` and persists; `session_summary` carries the field (T005)
- [X] T012 [US1] [U69] [U70] [U71] [U72] [U73] [U74] [U75] [U78] [U79] In `crates/micold-daemon/src/state.rs`: hold a `Views`; `set_window_view(client, view)`; `deregister` calls `Views::remove`; in `note_activity`, when the signal before is not `AwaitingInput`, the signal after is, and `is_in_view` is false, call `mark_attention` (T005)
- [X] T013 [US1] [U70] [U76] Handle `ClientMsg::WindowView` in `crates/micold-daemon/src/server.rs`: no `req`, no reply, no attachment needed (W1.5, W1.6) (T005)
- [X] T014 [US1] [U116] Copy `attention_seq` in `reconcile_catalog` in `crates/micold-client/src/catalog_sync.rs` (T006)
- [X] T015 [US1] [U111] [U112] [U113] [U114] [U115] `features::attention::State { sent_view: Option<WindowView> }` and `view_report` in `crates/micold-client/src/features/attention.rs` (NEW); register the module in `crates/micold-client/src/features/mod.rs` and the state in `crates/micold-client/src/app.rs` (T006)
- [X] T123 [US1] [U176] [U177] [U178] `State::view_facts(&self, window_focused: bool) -> ViewFacts` in `crates/micold-client/src/app.rs`, beside `terminal_focused`: `main_area_taken` is `settings.settings_draft.is_some()`, `selected` is the active project's selected session; render-free, so that no rule is left in `main.rs` (T006)
- [X] T016 [US1] In `crates/micold-client/src/main.rs`: after each update pass `state.view_facts(app.window_focused)` to `view_report` and send `ClientMsg::WindowView` when it returns one — one call and the send, no rule of its own; clear `sent_view` where `crates/micold-client/src/shell/daemon_sync.rs` handles a new connection, so the first report follows `Welcome` (T015, T123)

**Checkpoint**: `mise run gate` green. The service counts attention events; no window shows
anything new. The PR carries the `docs-not-needed` label: nothing the user sees has changed.

---

## Phase 4: User Story 1, slice B — one desktop notification on Linux (Priority: P1)

**Goal**: A window that sees a higher `attention_seq` claims it; the service grants each sequence
to one window; that window shows a desktop notification naming the project, the worktree and the
session. On macOS and Windows the backend reports that it cannot show one, which is logged once
(FR-010) until slice C.

**Independent Test**: quickstart §B1 to B5, B13 on Linux; `attention_claims.rs`,
`attention_notify.rs`.

### Tests for User Story 1, slice B (MANDATORY — Constitution Principle I) ⚠️

- [X] T017 [US1] [U18] [U19] [U20] [U21] [U22] [U23] [U24] [U25] [U26] [U27] Unit tests for `AttentionTracker::observe(sessions, phase, in_view) -> Vec<Claim>` in `crates/micold-core/src/attention.rs`, one per row of research R3's table: a session not seen before is adopted with no claim, also when it is `AwaitingInput` (FR-005, US1.13); a higher `attention_seq` with `Phase::Live` is claimed; the same sequence again is not; `Phase::Reconnected` claims only when the activity last seen was not `AwaitingInput`, it is now, the sequence is higher and the session is not `in_view` (FR-006, US1.11), and otherwise adopts; a lower sequence is adopted; ten sessions with higher sequences give ten claims (FR-009); a session absent from the snapshot is dropped from `seen`
- [X] T018 [US1] [U28] [U29] [U30] [U31] Unit tests for `notification_text(project, worktree, session) -> NotificationText` in `crates/micold-core/src/attention.rs`: title `"{session} is waiting for input"`, body `"{project} — {worktree}"`; the Default entry's name and a placeholder session label are passed through unchanged; nothing else is in either string (FR-004, US1.1, US1.7)
- [X] T019 [P] [US1] [U13] Protocol tests in `crates/micold-core/src/protocol/messages.rs` and `crates/micold-core/tests/schema_hash.rs`: `ClientMsg::AttentionClaim { session, seq }` and `DaemonMsg::AttentionGranted { session, seq }` encode and decode; `PROTOCOL_VERSION` is 23 (W1)
- [X] T020 [P] [US1] [U54] [U55] [U56] [U57] [U58] Unit tests for `Views::grant(session, seq, current_seq)` in `crates/micold-daemon/src/attention.rs`: true once per sequence; false for the same sequence again; false when `seq > current_seq`; true for a later sequence; sessions are independent (FR-006a, FR-009, W1.4)
- [X] T021 [P] [US1] [A1] [A8] [A12] [U80] [U81] [U82] [U83] Integration tests in `crates/micold-daemon/tests/attention_claims.rs` (NEW): two connections claim the same sequence and exactly one `AttentionGranted` is sent, to that claimer only (FR-006a); a claim for an unknown session is not answered; ten sessions give ten grants (SC-005); a claim gets no `OperationOk` (W1.5); a connection attached to no project may claim (W1.6)
- [X] T022 [P] [US1] [A1] [A4] [A7] [A11] [A13] [U117] [U118] [U119] [U120] [U121] [U122] [U123] [U124] Client tests in `crates/micold-client/tests/attention_notify.rs` (NEW) with a recording `DesktopNotifier`: a snapshot with a higher sequence yields one `AttentionClaim`; the first snapshot yields none (FR-005); `AttentionGranted` calls `show` once with the title and body built from the labels the sidebar shows (project name, `worktree_display_name` or the Default entry's name, `Session::label.display()`), and with the project path and session id (N1, N2); a granted session of a project that is not the active one is named by its own project (US1.4); no grant, no `show`; an `Err` from `show` is logged once per run and pushes no in-app notice, and a second `Err` is not logged (FR-010, N4); the first snapshot after a reconnect is observed with `Phase::Reconnected`
- [X] T023 [P] [US1] [U159] [U160] Unit tests in `crates/micold-client/src/shell/desktop_notify/linux.rs` (NEW, `#[cfg(test)]`) for the pure `notify_request(&DesktopNotification) -> NotifyRequest`: application name, summary, body and the `desktop-entry` hint `micold-ai-ide`; no other text; and for the error mapping from a bus failure to `NotifyError`

### Implementation for User Story 1, slice B

- [X] T024 [US1] [U18] [U19] [U20] [U21] [U22] [U23] [U24] [U25] [U26] [U27] `AttentionTracker`, `Seen { seq, awaiting }`, `Claim { session, seq }`, `Phase { Live, Reconnected }` and `observe` in `crates/micold-core/src/attention.rs` (T017)
- [X] T025 [US1] [U28] [U29] [U30] [U31] `NotificationText { title, body }` and `notification_text` in `crates/micold-core/src/attention.rs` (T018)
- [X] T026 [US1] [U13] `ClientMsg::AttentionClaim` and `DaemonMsg::AttentionGranted` in `crates/micold-core/src/protocol/messages.rs`; `PROTOCOL_VERSION` 22 → 23 in `crates/micold-core/src/protocol/version.rs` (T019)
- [X] T027 [US1] [U54] [U55] [U56] [U57] [U58] `granted: HashMap<SessionId, u64>` and `grant` in `crates/micold-daemon/src/attention.rs` (T020)
- [X] T028 [US1] [U80] [U81] [U82] [U83] Handle `ClientMsg::AttentionClaim` in `crates/micold-daemon/src/server.rs` through a `SharedState` method in `crates/micold-daemon/src/state.rs` that reads the session's current sequence and sends `AttentionGranted` to the claimer only (T021)
- [X] T029 [US1] Add `zbus` 5.19 as a Linux-only dependency of `micold-client` in `Cargo.toml` (workspace) and `crates/micold-client/Cargo.toml`, with the `async-io` feature `Cargo.lock` already has
- [X] T030 [US1] [U117] [U118] [U119] [U120] [U121] [U122] [U123] [U124] The seam in `crates/micold-client/src/features/attention.rs`, where `tests/` can reach it: `DesktopNotification`, `trait DesktopNotifier`, `NotifyError`; `tracker` and `failure_logged` in `State`; the reducer steps "snapshot → claims" and "grant → show, log once" (T022)
- [X] T031 [US1] `crates/micold-client/src/shell/desktop_notify/mod.rs` (NEW): `system() -> Box<dyn DesktopNotifier>` with three `cfg` arms — Linux to `linux.rs`; macOS and Windows to a notifier whose `show` returns `NotifyError::Unsupported` until T041; declare the module in `crates/micold-client/src/shell/mod.rs`
- [X] T032 [US1] [U159] [U160] `crates/micold-client/src/shell/desktop_notify/linux.rs`: `notify_request` and the `org.freedesktop.Notifications.Notify` call over one `zbus` connection; no action is offered yet (T023)
- [X] T033 [US1] In `crates/micold-client/src/shell/daemon_sync.rs`: call the reducer on every catalog snapshot (`Welcome`, `CatalogChanged`) and send its claims; on `DaemonMsg::AttentionGranted` call it with the notifier from `desktop_notify::system()` held by `App` in `crates/micold-client/src/main.rs`
- [X] T034 [US1] User guide, `docs/user-guide/worktrees-and-sessions.md`: the desktop notification — when it appears, what it names, never for the session in view, one however many windows are open, none while no window is open; on Linux it needs a notification service; that this release shows it on Linux (FR-031)

**Checkpoint**: `mise run gate` green; quickstart §B1 to B5 and B13 pass on Linux.

---

## Phase 5: User Story 1, slice C — the same notification on macOS and Windows (Priority: P1)

**Goal**: The notification appears through `UNUserNotificationCenter` on macOS and as a toast on
Windows. Story 1 is complete on the three systems (FR-029).

**Independent Test**: CI's macOS and Windows jobs run T035 and T036, by the step T122 adds; quickstart §C1 and §C2 (B1,
B2) on a bundle and an installed build.

### Tests for User Story 1, slice C (MANDATORY — Constitution Principle I) ⚠️

- [X] T122 [US1] In `.github/workflows/ci.yml`, first in this phase: add `--test attention_notify` to the enumerated `cargo test -p micold-client` list, and beside it a step for the macOS and Windows legs, `cargo test -p micold-client --bin micold-ai-ide desktop_notify`. The backends' tests are `#[cfg(test)]` modules of the binary behind an operating-system `cfg`: the list reaches no unit test of the binary, and `cargo test --workspace` runs on the Linux leg only, so without this step T035, T036, T079 and T080 run nowhere. Their red phase cannot be seen on the development host: push this task with T035 and T036 before T039 and T040, and record the failing macOS and Windows runs (the run's URL and the failing test names) in `specs/039-session-attention-notifications/tdd/cycle-log.md`
- [X] T035 [P] [US1] [U167] [U168] Unit tests in `crates/micold-client/src/shell/desktop_notify/macos.rs` (NEW, `#[cfg(test)]`): the pure mapping from `DesktopNotification` to the title and message passed to `mac-usernotifications`; each `mac_usernotifications::Error` (no bundle, authorisation refused) maps to a `NotifyError`
- [X] T036 [P] [US1] [U172] [U173] Unit tests in `crates/micold-client/src/shell/desktop_notify/windows.rs` (NEW, `#[cfg(test)]`): the pure mapping to the toast's title and first text line; `APP_USER_MODEL_ID` is `"MicoldAiIde.Client"`; an error from `show` maps to a `NotifyError`
- [X] T037 [P] [US1] [U46] [U47] [U48] [U49] Source-scan test `crates/micold-core/tests/notification_registers_nothing.rs` (NEW, in the style of `crates/micold-core/tests/macos_registers_nothing.rs`): `packaging/windows/micold-ai-ide.iss` puts `AppUserModelID: "MicoldAiIde.Client"` on the Start-menu shortcut, the same string as `APP_USER_MODEL_ID` in `windows.rs`, and registers no toast activator and no protocol handler; the macOS bundle's `Info.plist` template registers no URL scheme; `crates/micold-client/src/main.rs` reads no command-line argument (FR-015a, N8); `crates/micold-daemon/Cargo.toml` names none of `zbus`, `mac-usernotifications` and `tauri-winrt-notification` — the session service shows nothing itself (FR-007). The three cases that hold against today's sources (`[U47]` to `[U49]`) are green when written: for each, break the rule once on purpose, record the failing output in `specs/039-session-attention-notifications/tdd/cycle-log.md`, and revert

### Implementation for User Story 1, slice C

- [X] T038 [US1] Add `mac-usernotifications` 0.3.1 (macOS only) and `tauri-winrt-notification` 0.8.1 (Windows only) in `Cargo.toml` (workspace) and `crates/micold-client/Cargo.toml`; both are `MIT OR Apache-2.0`
- [X] T039 [US1] [U167] [U168] `crates/micold-client/src/shell/desktop_notify/macos.rs`: `show` through `mac-usernotifications`, asking for authorisation on first use; an unbundled binary returns the error of `check_bundle` (T035)
- [X] T040 [US1] [U46] [U172] [U173] `crates/micold-client/src/shell/desktop_notify/windows.rs`: `show` through `Toast::new(APP_USER_MODEL_ID)` (T036)
- [X] T041 [US1] The macOS and Windows arms of `system()` in `crates/micold-client/src/shell/desktop_notify/mod.rs` return the two backends; the `Unsupported` notifier is removed
- [X] T042 [US1] [U46] `AppUserModelID: "MicoldAiIde.Client"` on the Start-menu shortcut in `packaging/windows/micold-ai-ide.iss` (T037)
- [X] T043 [US1] Cross-check the arms from Linux: `cargo check -p micold-client --target aarch64-apple-darwin` for `crates/micold-client/src/shell/desktop_notify/macos.rs`, and the Windows target when its toolchain is installed; otherwise CI's Windows job is the check
- [X] T044 [US1] User guide: `docs/user-guide/worktrees-and-sessions.md` names the three systems and says the system may ask for, or withhold, permission; `docs/user-guide/install-macos.md` and `docs/user-guide/install-windows.md` say where notifications are allowed or turned off in the system's settings, and that Windows shows them only for an installed build (FR-031)
- [X] T118 [US1] Story 1's outer tests (behaviors A1 to A13 of the test list) are green on CI's Linux, macOS and Windows legs: locally `scripts/build-lock.sh cargo test --test attention_events`, `--test attention_claims`, `--test attention_view_report`, `--test attention_notify`, and `mise run test-core`; on the pull request, the macOS and Windows jobs with the list and the step of T122. Record the three jobs' result in `specs/039-session-attention-notifications/tdd/cycle-log.md`. Story 1 is not complete until they pass

**Checkpoint**: `mise run gate` green; CI green on Linux, macOS and Windows.

---

## Phase 6: User Story 2, slice A — the unread mark on a session's row, kept across restarts (Priority: P2)

**Goal**: A session that changes to awaiting input while not in view is unread; its sidebar row
carries the mark; the mark goes when the session comes into view; unread state is the same in
every window and outlives the last one.

**Independent Test**: quickstart §B2 to B4, B6 (row), B7, B8; `unread_state.rs`,
`unread_rows.rs`.

### Tests for User Story 2, slice A (MANDATORY — Constitution Principle I) ⚠️

- [x] T045 [P] [US2] [U9] [U10] [U11] [U175] Store tests in `crates/micold-core/src/store.rs`: `unread` missing reads as `false` (first start with the feature, FR-008a); a round trip keeps `true`; the field is written to the catalog file and to no other file of the store (FR-025); after a load that recovered from a store file it could not parse, as `crates/micold-core/tests/store_fault_isolation.rs` describes it today, no session is unread and the load reports nothing it did not report before (FR-008a, spec Edge Cases: stored unread state unreadable)
- [x] T046 [P] [US2] [U14] Protocol tests in `crates/micold-core/src/protocol/messages.rs` and `crates/micold-core/tests/schema_hash.rs`: `SessionSummary::unread` encodes and decodes; `PROTOCOL_VERSION` is 23 (W2)
- [x] T047 [P] [US2] [U59] [U60] Unit test in `crates/micold-daemon/src/attention.rs`: `set_view` returns the session that came into view, and `None` when the report names the same session as before or none
- [x] T048 [P] [US2] [A14] [A15] [A19] [A20] [A21] [A22] [A24] [A25] [A26] [A31] [A32] [A33] [A34] [A35] [A36] [U84] [U85] [U86] [U87] [U88] [U89] [U90] [U91] [U92] [U93] [U94] [U95] [U96] Integration tests in `crates/micold-daemon/tests/unread_state.rs` (NEW), two connections: an attention event sets `unread` (W2.1, A1, US2.1); a change while in view does not (US2.2, US2.12); a `WindowView` naming an unread session clears it and every connection receives the `CatalogChanged` (W2.2, FR-024, US2.3); working again does not clear it (FR-020, US2.8); a claim and a grant change nothing (W2.3, FR-017); with no connection the event sets it (US2.18) and it stays set when the session works again (US2.22); a restart of the service keeps `true` (US2.19) and keeps `false` for a session that did not change (US2.20); read at closing, then working and awaiting input again with no connection, is unread (US2.21); a removed session is gone from the snapshot (US2.9, A5); a session of a connection that dropped is not in view, so its change sets `unread` (US2.13); a session created with no connection that reaches `AwaitingInput` is unread (FR-008)
- [x] T049 [P] [US2] [U145] [U146] [U147] [U148] Component tests in `crates/micold-client/src/ui/material/unread_mark.rs` (NEW, `#[cfg(test)]`) and the gates `crates/micold-client/src/ui/material/composition_contrast.rs` and `anatomy_size.rs`: the mark is an 8dp filled circle in the `primary` role (U1); it meets 3:1 against its surface in the light and the dark scheme (U3); `count(0)` renders nothing (U2); `.worded(true)` adds the word `unread`
- [x] T050 [P] [US2] [U149] [U150] [U151] Tests in `crates/micold-client/src/ui/material/tree_view.rs`: a `TreeItem` with `.unread(true)` has the height of one without; its label truncates before the mark is pushed out (U8); the badge slot and its `ActivityBadge` are the same node as before (FR-018, FR-032)
- [x] T051 [P] [US2] [A14] [A36] [U125] [U126] [U127] Client tests in `crates/micold-client/tests/unread_rows.rs` (NEW): `features::attention::row_unread(&Session, in_view) -> bool` is true when `unread` and the session is not the one in view, and false at once for the session in view, before the service's answer arrives (U5, FR-019); `reconcile_catalog` copies `unread`. Add `--test unread_rows` to the enumerated `cargo test -p micold-client` list in `.github/workflows/ci.yml`, so the macOS and Windows legs run it

### Implementation for User Story 2, slice A

- [x] T052 [US2] [U9] [U10] [U11] `unread: bool` on `Session` in `crates/micold-core/src/session.rs` and, with `#[serde(default)]`, on `StoredSession` in `crates/micold-core/src/store.rs` (T045)
- [x] T053 [US2] [U14] `SessionSummary::unread` in `crates/micold-core/src/protocol/messages.rs`; `PROTOCOL_VERSION` 23 → 24 in `crates/micold-core/src/protocol/version.rs` (T046)
- [x] T054 [US2] [U84] [U86] [U87] [U88] [U89] [U90] [U91] [U92] [U93] [U96] In `crates/micold-daemon/src/catalog.rs`: `mark_attention` also sets `unread`; `mark_read(session)` clears it and persists; `session_summary` carries it (T048)
- [x] T055 [US2] [U59] [U60] [U85] [U86] [U94] [U95] In `crates/micold-daemon/src/attention.rs` `set_view` returns the session that came into view; in `crates/micold-daemon/src/state.rs` `set_window_view` calls `mark_read` and `broadcast_catalog` when that session is unread; T004's tests keep their assertions and ignore the new return value (T047, T048)
- [x] T056 [US2] [U125] [U126] [U127] Copy `unread` in `crates/micold-client/src/catalog_sync.rs`; `row_unread` in `crates/micold-client/src/features/attention.rs` (T051)
- [x] T057 [US2] [U145] [U146] [U147] [U148] `UnreadMark::new(roles)` with `.count(n)` and `.worded(bool)`, builder form ending in `.into()` (U4), in `crates/micold-client/src/ui/material/unread_mark.rs` (NEW); export it from `crates/micold-client/src/ui/material/mod.rs` (T049)
- [x] T058 [US2] [U149] [U150] [U151] `TreeItem::unread(bool)` in `crates/micold-client/src/ui/material/tree_view.rs`: the mark in the trailing slot before any trailing action, the label in the emphasised weight (T050)
- [x] T059 [US2] The session row in `crates/micold-client/src/ui/sidebar.rs` passes `.unread(row_unread(..))`
- [x] T060 [P] [US2] Showcase entry in `crates/micold-client/src/showcase/catalogue.rs` and `crates/micold-client/src/showcase/sections/atoms.rs`: a session row with the unread mark beside one without (FR-030)
- [x] T061 [US2] User guide, `docs/user-guide/worktrees-and-sessions.md`: the unread mark, how it differs from the activity indicator, what clears it, that it does not depend on notifications, and that sessions which finished a turn while the application was closed are unread when it is opened (FR-031)

**Checkpoint**: `mise run gate` green; quickstart §B6 (row), B7 and B8 pass.

---

## Phase 7: User Story 2, slice B — unread counts on the switcher's rows and button (Priority: P2)

**Goal**: Each project's row in the switcher shows its number of unread sessions with the running
count, and the switcher's button shows the total for the projects other than the active one, with
its panel closed or open.

**Independent Test**: quickstart §B5, B6 (panel and button), B7; `switcher_unread.rs`.

### Tests for User Story 2, slice B (MANDATORY — Constitution Principle I) ⚠️

- [x] T062 [P] [US2] [A27] [A30] [U36] [U37] [U38] [U39] [U40] [U41] Unit tests in `crates/micold-core/src/workspace.rs`: `unread_session_count(&project, in_view)` counts sessions of the Default entry and of every worktree, whatever the sidebar's filter hides (FR-022, US2.14), less `in_view`, and is zero for a project with none; `other_projects_unread(&active)` sums the other projects and never the active one (US2.15, US2.16), and after a switch to Q counts R only (US2.17)
- [x] T063 [P] [US2] [A16] [A17] [A18] [A28] [A29] [A30] [U128] [U129] [U130] Client tests in `crates/micold-client/tests/switcher_unread.rs` (NEW): every `SwitcherEntry` from `State::switcher_entries` carries `unread_count`, the active project's too (US2.4, US2.5); the count falls at once for the session in view (FR-019); the button's total equals `other_projects_unread`. Add `--test switcher_unread` to the enumerated `cargo test -p micold-client` list in `.github/workflows/ci.yml`, so the macOS and Windows legs run it
- [x] T064 [P] [US2] [A17] [U152] [U153] [U154] [U155] Tests in `crates/micold-client/src/ui/material/menu.rs` and the gate `crates/micold-client/src/ui/material/menu_anatomy.rs`: `trailing_mark: Some(n)` renders `● n unread` after `trailing_text`; `None` renders what the row renders today (FR-032); with no running count the mark alone trails; the row's height is unchanged (FR-021)
- [x] T065 [P] [US2] [A28] [U156] [U157] [U158] Tests in `crates/micold-client/src/ui/material/button.rs` and the gate `crates/micold-client/src/ui/material/button_anatomy.rs`: `.trailing_mark(n, tooltip)` renders `● n` after the label inside the button; the button's height is unchanged; the pure `unread_total_tooltip(n)` gives `1 unread session in other projects` and `{n} unread sessions in other projects` (FR-023)

### Implementation for User Story 2, slice B

- [x] T066 [US2] [U36] [U37] [U38] [U39] [U40] [U41] `unread_session_count` and `other_projects_unread` beside `running_session_count` in `crates/micold-core/src/workspace.rs` (T062)
- [x] T067 [US2] [U128] [U129] [U130] `SwitcherEntry::unread_count` in `crates/micold-client/src/features/project.rs`; `switcher_entries` and the other-projects total in `crates/micold-client/src/app.rs` (T063)
- [x] T068 [US2] [U152] [U153] [U154] [U155] `MenuItem::trailing_mark: Option<usize>` in `crates/micold-client/src/ui/material/menu.rs`, composing `UnreadMark` (T064)
- [x] T069 [US2] [U156] [U157] [U158] `Button::trailing_mark(n, tooltip)` and `unread_total_tooltip` in `crates/micold-client/src/ui/material/button.rs`, composing `UnreadMark` (T065)
- [x] T070 [US2] The switcher's rows in `crates/micold-client/src/ui/mod.rs` pass `trailing_mark` when the count is one or more (U6); the switcher's button in `crates/micold-client/src/ui/toolbar.rs` passes `.trailing_mark` when the total is one or more (U7)
- [x] T071 [P] [US2] Showcase entries in `crates/micold-client/src/showcase/catalogue.rs` and `crates/micold-client/src/showcase/sections/atoms.rs`: a switcher row with a running count and an unread count, and the switcher's button with an unread count (FR-030)
- [x] T072 [US2] User guide, `docs/user-guide/project-selection.md`: the unread count on each project's row and the total on the switcher's button (FR-031)
- [x] T119 [US2] [A14] [A15] [A16] [A17] [A18] [A19] [A20] [A21] [A22] [A24] [A25] [A26] [A27] [A28] [A29] [A30] [A31] [A32] [A33] [A34] [A35] [A36] Story 2's outer tests are green: `scripts/build-lock.sh cargo test --test unread_state`, `--test unread_rows`, `--test switcher_unread`, and `mise run gate` for the component gates of `crates/micold-client/src/ui/material/`. Story 2 is not complete until they pass (US2.10 is closed by T121)

**Checkpoint**: `mise run gate` green; quickstart §B5 to B7 pass. Story 2 is complete.

---

## Phase 8: User Story 3, slice A — a click on the notification opens the session (Priority: P3)

**Goal**: Clicking a notification brings the window that holds the session's project to the front
and shows the session; a session that is gone yields an in-app notice. On Wayland the window asks
for the user's attention until slice B.

**Independent Test**: quickstart §B9 to B11a; `session_reveal.rs`, `attention_reveal.rs`.

### Tests for User Story 3, slice A (MANDATORY — Constitution Principle I) ⚠️

- [x] T073 [US3] [U32] [U33] [U34] [U35] Unit tests for `resolve_reveal(&Workspace, project, session) -> Reveal` in `crates/micold-core/src/attention.rs`: `Show` when the project is known and available and the session exists; `Unavailable` when the session was removed, the project forgotten, or its folder unavailable (FR-011, FR-013)
- [x] T074 [P] [US3] [U15] Protocol tests in `crates/micold-core/src/protocol/messages.rs` and `crates/micold-core/tests/schema_hash.rs`: `ClientMsg::SessionReveal { project, session }` and `DaemonMsg::RevealSession { project, session }`; `PROTOCOL_VERSION` is 24 (W3)
- [x] T075 [P] [US3] [U61] [U62] [U63] [U64] [U65] Unit tests in `crates/micold-daemon/src/attention.rs`: `focus_order` puts the connection that last reported `focused: true` last; `remove` takes a connection out of it; `reveal_target(holder, sender)` is the holder, else the last of `focus_order`, else the sender (FR-012)
- [x] T076 [P] [US3] [A41] [A42] [U97] [U98] [U99] [U100] [U101] Integration tests in `crates/micold-daemon/tests/session_reveal.rs` (NEW), two connections: `SessionReveal` is forwarded as `RevealSession` to exactly one connection — the one attached to the project (US3.6), else the one that last reported focus, else the sender (W3.1); it is forwarded for a session that does not exist (W3.2); no session, attachment or stored state changes (W3.3, FR-014)
- [x] T077 [P] [US3] [A37] [A38] [A39] [A40] [A41] [U131] [U132] [U133] [U134] [U135] [U136] [U137] Client tests in `crates/micold-client/tests/attention_reveal.rs` (NEW): `NotifierEvent::Activated` yields one `SessionReveal`; `RevealSession` with `Reveal::Show` yields `ProjectMsg::Reopened` then `SessionMsg::Selected` when the project is not active, `SessionMsg::Selected` alone when it is, and no other message (N5, FR-014, US3.1, US3.2, US3.5); `Reveal::Unavailable` pushes `That session is no longer available.` at `Level::Info` and changes no selection (FR-013, US3.4); the session shown is then in view, so `row_unread` is false (US3.3); `raise_plan(false)` is `[Unminimize, Focus]` and `raise_plan(true)` is `[Unminimize, RequestAttention]` (N6). Add `--test attention_reveal` to the enumerated `cargo test -p micold-client` list in `.github/workflows/ci.yml`, so the macOS and Windows legs run it; T079 and T080 run there by the step T122 added
- [x] T078 [P] [US3] [U161] [U162] [U163] [U164] [U165] Unit tests in `crates/micold-client/src/shell/desktop_notify/linux.rs`: `notify_request` now offers the `default` action, the same whether or not the service lists the `actions` capability (FR-015, N7); `ActionInvoked(id, "default")` for an id in the table maps to `NotifierEvent::Activated { project, session }`; an id the table does not hold maps to nothing (N9); another action key maps to nothing; `NotificationClosed` removes the id
- [x] T079 [P] [US3] [U169] [U170] [U171] Unit tests in `crates/micold-client/src/shell/desktop_notify/macos.rs`: a response with the default action maps to `Activated`; a dismissal or a timeout maps to nothing; an unknown notification id maps to nothing (N9)
- [x] T080 [P] [US3] [U174] Unit tests in `crates/micold-client/src/shell/desktop_notify/windows.rs`: `on_activated` called with `None` maps to `Activated` for the toast's session; the id table is keyed per toast

### Implementation for User Story 3, slice A

- [x] T081 [US3] [U32] [U33] [U34] [U35] `Reveal { Show { project, session }, Unavailable }` and `resolve_reveal` in `crates/micold-core/src/attention.rs` (T073)
- [x] T082 [US3] [U15] `ClientMsg::SessionReveal` and `DaemonMsg::RevealSession` in `crates/micold-core/src/protocol/messages.rs`; `PROTOCOL_VERSION` 24 → 25 in `crates/micold-core/src/protocol/version.rs` (T074)
- [x] T083 [US3] [U61] [U62] [U63] [U64] [U65] [U97] [U98] [U99] [U100] [U101] `focus_order` and `reveal_target` in `crates/micold-daemon/src/attention.rs`; in `crates/micold-daemon/src/state.rs` and `crates/micold-daemon/src/server.rs` handle `SessionReveal`: find the connection attached to the project, pick the target, send it `RevealSession` (T075, T076)
- [x] T084 [US3] [U131] [U132] [U133] [U134] [U135] [U136] [U137] In `crates/micold-client/src/features/attention.rs`: `NotifierEvent::Activated { project, session }`, `RaiseStep { Unminimize, Focus, RequestAttention }`, `raise_plan(wayland: bool)`, and the reducer steps "activated → `SessionReveal`" and "`RevealSession` → raise, then messages or notice" (T077)
- [x] T085 [US3] [U161] [U162] [U163] [U164] [U165] `system(events)` takes the channel a backend reports on, in `crates/micold-client/src/shell/desktop_notify/mod.rs`; `linux.rs` offers the `default` action, keeps the id table, and maps the `ActionInvoked` and `NotificationClosed` signals of its one connection (T078)
- [x] T086 [P] [US3] [U169] [U170] [U171] `crates/micold-client/src/shell/desktop_notify/macos.rs`: report the delegate's response through the channel (T079)
- [x] T087 [P] [US3] [U174] `crates/micold-client/src/shell/desktop_notify/windows.rs`: `on_activated` only sends the event over the channel — it is called on a thread of the system's (T080)
- [x] T088 [US3] `crates/micold-client/src/shell/window_raise.rs` (NEW): one `iced::window` task per `RaiseStep` for the window from `iced::window::latest()` — `minimize(id, false)`, `gain_focus(id)`, `request_user_attention(id, ..)`; it decides nothing (N6)
- [x] T089 [US3] Wire it in `crates/micold-client/src/shell/daemon_sync.rs` and `crates/micold-client/src/main.rs`: a subscription delivers `NotifierEvent`s; `DaemonMsg::RevealSession` runs the raise steps and dispatches the reducer's messages
- [x] T090 [US3] User guide, `docs/user-guide/worktrees-and-sessions.md`: what clicking does; which window comes forward with several open; the notice for a session that is gone; that a notification raised by a window since closed, or shown by a text-only notification service, does not open the session; that on Wayland the window asks for attention instead of taking focus (FR-031). In `docs/user-guide/install-windows.md`: a click on a toast that has moved to the notification centre may not open the session (research R4, not yet verified)
- [x] T120 [US3] [A37] [A38] [A39] [A40] [A41] [A42] Story 3's outer tests are green: `scripts/build-lock.sh cargo test --test session_reveal` and `--test attention_reveal`. Story 3's six scenarios are not complete until they pass

**Checkpoint**: `mise run gate` green; quickstart §B9 to B11a pass on X11.

---

## Phase 9: User Story 3, slice B — keyboard focus from a click on Wayland (Priority: P3)

**Goal**: On Wayland the click's activation token travels to the window that is raised, which
activates its surface with `xdg_activation_v1`; where that is not possible it asks for attention
as before.

**Independent Test**: quickstart §C4 on the development host's Wayland session; the
`raise_plan` rows in `attention_reveal.rs`.

- [x] T091 [US3] **Probe first** (research R7, *Unverified*). On a Wayland session, outside the worktree: bind `xdg_activation_v1` on a second connection made with `Backend::from_foreign_display` from the handles `iced::window::run` gives, and activate the window with the token of a notification's `ActivationToken` signal. Record the compositor, the notification service and the outcome in `specs/039-session-attention-notifications/research.md` R7 in place of the **Unverified** paragraph. If no compositor at hand gives focus, or the binding cannot be made to work: do not start T092 to T100; write the limit into `docs/user-guide/worktrees-and-sessions.md` (it is already stated by T090), add a follow-up to `specs/039-session-attention-notifications/autopilot.md`, and close this milestone with the record alone: tick T092 to T100 without doing them, each with the suffix `— DROPPED (probe, R7)`, so that the milestone's range is closed, and note in `specs/039-session-attention-notifications/autopilot.md` that M7 shipped T091 only and took no wire number; in that case mark `[U16]`, `[U102]`, `[U138]` to `[U141]` and `[U166]` as `DROPPED` in `specs/039-session-attention-notifications/tdd/test-list.md`, with the probe's result as the reason

### Tests for User Story 3, slice B (MANDATORY — Constitution Principle I) ⚠️

- [x] T092 [P] [US3] [U16] Protocol tests in `crates/micold-core/src/protocol/messages.rs` and `crates/micold-core/tests/schema_hash.rs`: `activation: Option<String>` on `SessionReveal` and `RevealSession`; `PROTOCOL_VERSION` is 25 (W3.4)
- [x] T093 [P] [US3] [U102] In `crates/micold-daemon/tests/session_reveal.rs`: `activation` reaches the target connection unchanged, also when the target is not the sender (W3.4)
- [x] T094 [P] [US3] [U138] [U139] [U140] [U141] In `crates/micold-client/tests/attention_reveal.rs`: `raise_plan(true, Some(token))` is `[Unminimize, Activate(token)]`; `raise_plan(true, None)` is `[Unminimize, RequestAttention]`; `raise_plan(false, _)` is `[Unminimize, Focus]`; `after_activation(false)` is `Some(RequestAttention)` and `after_activation(true)` is `None`; the token of `Activated` is put into `SessionReveal`, and the token of `RevealSession` is the one passed to `raise_plan` (N6)
- [x] T095 [P] [US3] [U166] In `crates/micold-client/src/shell/desktop_notify/linux.rs` tests: an `ActivationToken(id, token)` signal that precedes `ActionInvoked` for the same id is carried in `Activated { activation: Some(token) }`; without it `activation` is `None`

### Implementation for User Story 3, slice B

- [x] T096 [US3] Add `wayland-client` 0.31, `wayland-backend` 0.3 with `client_system` and `wayland-protocols` 0.32 with `client` and `staging` as Linux-only dependencies in `Cargo.toml` (workspace) and `crates/micold-client/Cargo.toml`; all three are in `Cargo.lock` already
- [x] T097 [US3] [U16] [U102] `activation: Option<String>` on both reveal messages in `crates/micold-core/src/protocol/messages.rs`, forwarded unread in `crates/micold-daemon/src/state.rs`; `PROTOCOL_VERSION` 25 → 26 in `crates/micold-core/src/protocol/version.rs` (T092, T093)
- [x] T098 [US3] [U138] [U139] [U140] [U141] In `crates/micold-client/src/features/attention.rs`: `NotifierEvent::Activated::activation`, `RaiseStep::Activate(String)`, `raise_plan(wayland, activation)`, `after_activation(done)` (T094)
- [x] T099 [US3] [U166] The `ActivationToken` signal in `crates/micold-client/src/shell/desktop_notify/linux.rs` (T095)
- [x] T100 [US3] The Linux arm of `crates/micold-client/src/shell/window_raise.rs`: carry out `Activate` through `iced::window::run` and the binding T091 proved, report whether it was done, and issue the step `after_activation` returns; the `unsafe` block is confined to the foreign-display call; update the Wayland sentence in `docs/user-guide/worktrees-and-sessions.md` to what T091 recorded (FR-031)

**Checkpoint**: `mise run gate` green; quickstart §C4 recorded. Story 3 is complete (its six scenarios were closed by T120; this slice adds keyboard focus on Wayland, or records why not).

---

## Phase 10: User Story 4 — turn desktop notifications off (Priority: P4)

**Goal**: One **Desktop notifications** switch in Settings, on by default, kept across restarts,
the same in every window. While it is off nothing is notified; unread marks are unaffected.

**Independent Test**: quickstart §B12; `settings_desktop_notifications.rs`.

### Tests for User Story 4 (MANDATORY — Constitution Principle I) ⚠️

- [x] T101 [P] [US4] [A43] [A48] [U42] [U43] [U44] [U45] Unit tests in `crates/micold-core/src/settings.rs`: `desktop_notifications` is `true` by default and when the field is missing from the file (FR-026, spec Edge Cases); `false` survives a round trip; the serialised settings hold exactly one key that names notifications, and none per AI CLI (FR-028)
- [x] T102 [P] [US4] [U17] Protocol tests in `crates/micold-core/src/protocol/messages.rs` and `crates/micold-core/tests/schema_hash.rs`: `DaemonSettings::desktop_notifications: bool` and `ClientMsg::SettingsSet::desktop_notifications: Option<bool>`; `PROTOCOL_VERSION` is one more than before this milestone — 26 as planned, 25 when M7 took no number (W4)
- [x] T103 [P] [US4] [U66] [U67] [U68] Unit tests in `crates/micold-daemon/src/attention.rs`: `grant(.., enabled: false)` is false and records nothing; `note_event(session, seq, enabled: false)` records `seq` as granted, so a later `grant(session, seq, seq, true)` is false (FR-027); `note_event(.., true)` records nothing
- [x] T104 [P] [US4] [A23] [A44] [A45] [A46] [A47] [A48] [U103] [U104] [U105] [U106] [U107] [U108] [U109] [U110] Integration tests in `crates/micold-daemon/tests/settings_desktop_notifications.rs` (NEW, in the style of `crates/micold-daemon/tests/settings_default_ai_cli.rs`): `SettingsSet { desktop_notifications: Some(false) }` reaches every connection as `SettingsChanged` and survives a restart of the service (US4.4); `None` leaves it unchanged; while off a claim is not granted (US4.2, US4.3) and the event still sets `unread` (FR-017, US2.10, SC-003); after it is turned on the next event is granted and the events made while off are not, also for a connection that reconnects (US4.5, W4.2); the rule is the same for a session of each AI CLI, as none is named anywhere on the path (FR-028, US4.6)
- [x] T105 [P] [US4] [A43] [U142] [U143] [U144] Client tests beside those of `tool_server_enabled` in `crates/micold-client/src/features/settings.rs`: the draft holds `desktop_notifications`; its message changes the draft; saving sends `SettingsSet` with `Some(value)`; `SettingsChanged` updates the draft's source (US4.1)

### Implementation for User Story 4

- [x] T106 [US4] [U42] [U43] [U44] [U45] `Settings::desktop_notifications` with `#[serde(default = "default_desktop_notifications")]` returning `true`, in `crates/micold-core/src/settings.rs` (T101)
- [x] T107 [US4] [U17] The two wire fields in `crates/micold-core/src/protocol/messages.rs`; `PROTOCOL_VERSION` plus one (26 → 27 as planned) in `crates/micold-core/src/protocol/version.rs` (T102)
- [x] T108 [US4] [U66] [U67] [U68] `note_event` and the `enabled` parameter of `grant` in `crates/micold-daemon/src/attention.rs`; the calls in T020's tests pass `true` and assert what they asserted (T103)
- [x] T109 [US4] [U103] [U104] [U105] [U106] [U107] [U108] [U109] [U110] In `crates/micold-daemon/src/catalog.rs` the field joins `persist_service_settings` and `DaemonSettings`, as `tool_server_enabled` does; in `crates/micold-daemon/src/state.rs` the claim path passes the setting to `grant` and `note_activity` calls `note_event`; `crates/micold-daemon/src/server.rs` applies `SettingsSet` (T104)
- [x] T110 [US4] [U142] [U143] [U144] Mirror the field wherever the client mirrors `tool_server_enabled`: `crates/micold-client/src/features/settings.rs`, `crates/micold-client/src/shell/startup.rs`, `crates/micold-client/src/shell/persist.rs`, `crates/micold-client/src/shell/daemon_sync.rs` (T105)
- [x] T111 [US4] The **Desktop notifications** `Checkbox` with its `field_note`, beside the tool-server switch on the Environment page in `crates/micold-client/src/ui/settings/environment.rs` (research R8)
- [x] T112 [US4] User guide, `docs/user-guide/settings.md`: the switch, its default, that it applies to every AI CLI and every window at once, that unread marks do not depend on it, and that the operating system's own permission is separate (FR-031)
- [x] T121 [US4] [A23] [A43] [A44] [A45] [A46] [A47] [A48] Story 4's outer tests are green: `scripts/build-lock.sh cargo test --test settings_desktop_notifications`, and the settings tests of `crates/micold-core/src/settings.rs` and `crates/micold-client/src/features/settings.rs`. Story 4, and US2.10, are not complete until they pass

**Checkpoint**: `mise run gate` green; quickstart §B12 passes. All four stories are complete.

---

## Phase 11: Polish & Cross-Cutting Concerns

- [x] T113 [P] `docs/development/architecture.md`: the attention flow — view report, attention sequence, claim and grant, reveal routing — and why the session service is the arbiter (research R1 to R3, R6)
- [x] T114 [P] `docs/development/component-library.md`: `UnreadMark` and the three host APIs (`TreeItem::unread`, `MenuItem::trailing_mark`, `Button::trailing_mark`)
- [x] T115 Run quickstart §B (B1 to B15) with the `visual-pass` skill, in the light and the dark scheme, and record each step's result — for B7 and B9 the time measured (SC-006, SC-004) — under a `## Record` heading in `specs/039-session-attention-notifications/quickstart.md`
- [x] T116 Run quickstart §C and record it in the same section of `specs/039-session-attention-notifications/quickstart.md`: C3 (the session service in a container: `mise run image`, then B1, B6, B9) and C4 on the development host; C1 on a macOS bundle and C2 on an installed Windows build, including whether a click on a toast in the notification centre is reported (research R4, **Unverified**) — write the answer into R4; only if it contradicts the sentence T090 put in `docs/user-guide/install-windows.md`, correct that sentence. A pass that needs a machine that is not at hand is listed under *Follow-ups not done* in `specs/039-session-attention-notifications/autopilot.md`, with the steps to run
- [x] T117 `mise run gate` green on the branch, and CI green on Linux, macOS and Windows for `crates/micold-client/src/shell/desktop_notify/` (Principle VI)

---

## Phase 12: Bugfix BUG-566 — a click counts only from the service that showed the notification (GitHub #566)

**Goal**: On Linux, a notification signal opens a session only when it comes from the unique bus
name that answered the `Notify` call for its id. A signal forged by another peer, broadcast or sent
to the window's own name, and a signal of a restarted service that reuses an id, open nothing
(FR-015b, N9a). Local to `linux.rs`: the match rule, `Shown`, `signal` and the listening thread;
the connection and `notify_error` stay as they are (issue #569 changes them).

### Tests for BUG-566 (MANDATORY — Constitution Principle I) ⚠️

- [x] T124 [BUG-566] [U179] [U180] [U181] *(test)* `crates/micold-client/src/shell/desktop_notify/linux.rs`
      tests, against stubs that compile so each fails on its assertion. U179: an entry recorded
      for service `:1.5`, then `ActionInvoked(id, "default")` from `:1.9` is no event and the entry
      stays (a later one from `:1.5` is `Activated`); an `ActivationToken` or `NotificationClosed`
      from `:1.9` changes nothing. U180: after `NameOwnerChanged(":1.5" → ":1.6")`, the click for
      the old id from `:1.6` is nothing. U181: `NameOwnerChanged` with old owner `:1.5` drops `:1.5`'s
      entries and keeps one recorded for `:1.6`; one with a sender other than
      `org.freedesktop.DBus`, or naming another bus name, drops nothing. The message parsing
      (sender, and `NameOwnerChanged`'s arguments) is tested on `zbus::Message`s built as the
      existing `message` helper builds them.

### Implementation for BUG-566

- [x] T125 [BUG-566] `crates/micold-client/src/shell/desktop_notify/linux.rs`: `Entry` keeps the
      service's unique name; `Shown::record` takes it from the `Notify` reply's header sender (no
      sender: nothing recorded, the notification is still shown, N7); `Shown::on_signal` takes the
      signal's sender and acts only on a match; a `NameOwnerChanged` for
      `org.freedesktop.Notifications` from `org.freedesktop.DBus` drops the old owner's entries.
      `SIGNALS` names `sender='org.freedesktop.Notifications'`; the listening thread also reads
      `type='signal',sender='org.freedesktop.DBus',interface='org.freedesktop.DBus',member='NameOwnerChanged',arg0='org.freedesktop.Notifications'`
      (one iterator per rule, or one thread reading both). Module and type docs say why the
      sender rule is not the check (BUG-566 *Mechanism*). Turns T124 green.
- [x] T126 [BUG-566] Verify on a private `dbus-daemon --session` with a stand-in service, as the
      M6 visual pass did (`visual-pass/M6/`): a click from the service still opens the session; a
      forged `ActionInvoked` from another peer, broadcast and unicast to the client's unique name,
      opens nothing; after the service is restarted, its id 1 does not open the old notification's
      session. Record the result in `bugs/BUG-566.md`. Then the gate's commands (`mise run gate`).

**Bugfix**: 2026-10-06 — BUG-566. Phase 12 (T124–T126) added; no task reopened. See `bugs/BUG-566.md`.

---

## Dependencies & Execution Order

### Phase Dependencies

- **US1 slice A (Phase 3)** has no prerequisite.
- **US1 slice B (Phase 4)** depends on slice A (the view report and `attention_seq`).
- **US1 slice C (Phase 5)** depends on slice B (the seam and `system()`).
- **US2 slice A (Phase 6)** needs the rules of US1 slice A only (the attention event and `set_view`), but T048 asserts that a claim and a grant leave `unread` alone and T053 takes the wire number after slice B's, so it depends on US1 slice B.
- **US2 slice B (Phase 7)** depends on US2 slice A (`unread`, `UnreadMark`).
- **US3 slice A (Phase 8)** depends on US1 slices B and C (the backends that raise the notification) and on US2 slice A (`row_unread` in T077).
- **US3 slice B (Phase 9)** depends on US3 slice A.
- **US4 (Phase 10)** depends on US1 slice B (`grant`). Its `unread` assertion in T104 depends on US2 slice A, and its wire number follows US3 slice B's, taken or not.
- **Polish (Phase 11)** depends on all stories.
- **Bugfix BUG-566 (Phase 12)** depends on US3 slice A (T078, T085) and M7 (T095, T099); T124 before T125, T126 last.

### Within Each Phase

- Tests first, observed failing; then implementation in the listed order.
- Core (`micold-core`) before the daemon, the daemon before the client's features, features before `shell/` and `ui/`.
- The user-guide task lands in the same phase as the behaviour it describes.
- A story's final run task (T118 to T121) comes last in the phase that completes the story.
- T122 comes first in US1 slice C: it is what lets the macOS and Windows tests of that slice run.

### Parallel Opportunities

- US1 slice A: T001 to T006 touch different files.
- US1 slice B: T019 to T023; T017 and T018 share `attention.rs`.
- US1 slice C: T035 to T037, after T122; T039 and T040 are different files.
- US2 slice A: T045 to T051; T060 beside the code tasks.
- US2 slice B: T062 to T065; T071 beside the code tasks.
- US3 slice A: T074 to T080; T086 and T087.
- US4: T101 to T105.

## Parallel Example: User Story 1, slice A

```text
T001 core attention.rs   T002 store.rs   T003 messages.rs + schema_hash.rs
T004 daemon attention.rs   T005 attention_events.rs   T006 attention_view_report.rs
```

## Implementation Strategy

The session service is the arbiter, so its part comes first and alone: slice A of story 1 ships
the view report and the attention sequence, proved by an integration test, with nothing new on
screen. Slice B puts the notification on the Linux desktop and slice C on macOS and Windows; the
three together are the MVP. Story 2 then adds the mark and the counts in two steps, the row before
the switcher. Story 3 adds the click, with the Wayland focus path — the one part that starts with a
probe — as its own step, so a failed probe costs one small milestone. Story 4 adds the switch.
Each step merges on its own with `mise run gate` green.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — The service knows what is in view and counts attention events 🎯 MVP

- **Tasks**: T001–T016, T123
- **Deliverable**: On `main`, every window reports the session it has in view, and the session
  service counts, stores and sends to every window one attention event for each change into
  awaiting input that happens while the session is in view nowhere. A developer observes it in the
  integration test; the application looks as before.
- **Satisfies**: the rules under US1 scenarios 2, 3, 5, 6, 9, 10, 12 (in view, repeated signal,
  several sessions); FR-002, FR-003, FR-009 (events); wire W1.1–W1.3, W1.6
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test attention_events`; `scripts/build-lock.sh cargo test -p micold-client --test attention_view_report`; `mise run test-core`
- **Depends on**: —
- **Tier**: full

### M2 — One desktop notification on Linux

- **Tasks**: T017–T034
- **Deliverable**: On `main`, on Linux, a session that changes to awaiting input while not in view
  raises exactly one desktop notification naming its project, worktree and session, however many
  windows are open; none for the session in view, none at start, none with no window open; a
  system that cannot show it changes nothing else. The user guide describes it.
- **Satisfies**: US1 acceptance scenarios 1–13 on Linux; FR-001–FR-010; SC-001, SC-002, SC-005,
  SC-007 on Linux
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test attention_claims`; `scripts/build-lock.sh cargo test -p micold-client --test attention_notify`; the notification part of quickstart §B1–B5 and B13 (their marks and counts ship in M4 and M5)
- **Depends on**: M1
- **Tier**: full

### M3 — The notification on macOS and Windows

- **Tasks**: T035–T044, T118, T122
- **Deliverable**: On `main`, the same notification is shown through the system's own facility on
  macOS (a signed bundle) and Windows (an installed build); the installer's shortcut carries the
  application's identity; the user guide covers the three systems and the system's permission.
- **Satisfies**: US1 acceptance scenarios 1–13 on macOS and Windows; FR-029 (story 1), FR-015a
  (nothing registered); SC-001
- **Verify**: `mise run test-core` (`notification_registers_nothing`); CI's macOS and Windows jobs green, running T035 and T036 by the step T122 adds; `cargo check -p micold-client --target aarch64-apple-darwin`; quickstart §C1, §C2 (B1, B2) where the machine is at hand
- **Depends on**: M2
- **Tier**: full

### M4 — The unread mark on a session's row

- **Tasks**: T045–T061
- **Deliverable**: On `main`, a session that finished a turn while not in view carries an unread
  mark on its sidebar row, in every window; the mark goes within a second of the session coming
  into view; sessions that finished a turn while the application was closed are marked when it is
  opened; the showcase shows the marked row; the user guide describes the mark.
- **Satisfies**: US2 acceptance scenarios 1–3 (mark), 6–9, 11–13, 18–23; FR-008, FR-008a, FR-016–FR-020,
  FR-024, FR-025, FR-030 (row), FR-032; SC-002, SC-006, SC-009
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test unread_state`; `scripts/build-lock.sh cargo test -p micold-client --test unread_rows`; quickstart §B6 (row), B7, B8; `mise run showcase` for the row
- **Depends on**: M2
- **Tier**: full

### M5 — Unread counts on the switcher

- **Tasks**: T062–T072, T119
- **Deliverable**: On `main`, each project's row in the switcher shows `● n unread` with its
  running count, and the switcher's button shows `● n` for the unread sessions of the other
  projects with its panel closed or open; the showcase shows both; the user guide describes them.
- **Satisfies**: US2 acceptance scenarios 3 (count), 4, 5, 14–17; FR-021–FR-023, FR-030; SC-008,
  SC-010
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --test switcher_unread`; `mise run test-core`; quickstart §B5, B6 (panel and button), B7
- **Depends on**: M4
- **Tier**: full

### M6 — A click on the notification opens the session

- **Tasks**: T073–T090, T120
- **Deliverable**: On `main`, clicking a notification brings the window that holds the session's
  project to the front and shows the session, on Linux (X11), macOS and Windows; a session that is
  gone yields the notice `That session is no longer available.`; on Wayland the window asks for
  attention. The user guide describes clicking and its limits.
- **Satisfies**: US3 acceptance scenarios 1–6; FR-011–FR-015; SC-004 on X11 (quickstart B9; macOS and Windows as recorded in M9)
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test session_reveal`; `scripts/build-lock.sh cargo test -p micold-client --test attention_reveal`; quickstart §B9–B11a
- **Depends on**: M3, M4
- **Tier**: full

### M7 — Keyboard focus from a click on Wayland

- **Tasks**: T091–T100
- **Deliverable**: On `main`, on a Wayland session whose compositor honours the notification's
  activation token, a click gives the window keyboard focus, also when the window raised is not
  the one that showed the notification; research R7 records the probe. If the probe fails, the
  deliverable is that record and the limit in the user guide; T092–T100 are then ticked as
  `DROPPED` (T091) and the milestone takes no wire number.
- **Satisfies**: US3 acceptance scenario 1 on Wayland; FR-011 (keyboard focus), FR-015
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --test attention_reveal`; quickstart §C4 on the development host; `specs/039-session-attention-notifications/research.md` R7 holds the probe's result
- **Depends on**: M6
- **Tier**: full

### M8 — The Desktop notifications switch

- **Tasks**: T101–T112, T121
- **Deliverable**: On `main`, Settings → Environment has a **Desktop notifications** switch, on by
  default; turning it off stops notifications for every AI CLI and every window at once, without a
  restart, and leaves unread marks as they are; the choice survives a restart; the user guide
  describes it.
- **Satisfies**: US4 acceptance scenarios 1–6; US2 scenario 10; FR-017, FR-026–FR-028; SC-003
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test settings_desktop_notifications`; quickstart §B12
- **Depends on**: M4, M7
- **Tier**: full

### M9 — Developer docs and the recorded passes

- **Tasks**: T113–T117
- **Deliverable**: On `main`, the architecture document describes the attention flow, the
  component-library document describes `UnreadMark`, and `quickstart.md` holds the recorded §B
  pass and the §C passes that could be run, with the rest listed as follow-ups in the ledger.
- **Satisfies**: FR-029 (recorded), FR-031 (developer docs); SC-001–SC-010 as recorded, with each pass that could not be run listed as a follow-up
- **Verify**: `specs/039-session-attention-notifications/quickstart.md` § Record lists B1–B15 and C1–C4 with a result or a follow-up each; `mise run gate`
- **Depends on**: M1–M8
- **Tier**: light
