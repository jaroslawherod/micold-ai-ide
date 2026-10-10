---

description: "Task list for feature 490: Show Claude plan usage and the next limit reset"
---

# Tasks: Show Claude Plan Usage and the Next Limit Reset

**Input**: Design documents from `/specs/490-claude-plan-usage/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/ (status-line-relay.md,
plan-usage-wire.md, usage-indicator.md), quickstart.md

**Tests**: Per Constitution Principle I, test tasks are MANDATORY and come before the
implementation they cover. Each test task names the file and the behaviours it pins; write it,
run it, see it fail for the right reason, then implement.

**Documentation**: Per Principle VII, each user-visible milestone carries its user-guide task.

**Cross-platform**: Per Principle VI, the only platform branch is `status_relay::user_shell`
(research R4); every test below runs on Linux, macOS and Windows in CI.

**Note on stories**: US1 and US3 are both P1 and share one data path: US3's "nothing shown,
nothing logged" scenarios are properties of the same route, relay and indicator that deliver US1.
Their tests sit with the task that implements the behaviour, labelled with the story they prove.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: US1–US4 from spec.md

---

## Phase 1: Setup

- [ ] T001 Add `chrono = { workspace = true }` (workspace entry: `default-features = false, features = ["clock"]`, research R11) to `crates/micold-core/Cargo.toml` and `crates/micold-client/Cargo.toml`
- [ ] T002 Create the `plan_usage` module: `crates/micold-core/src/plan_usage.rs` with `pub mod status_line;` and an empty `crates/micold-core/src/plan_usage/status_line.rs`, registered as `pub mod plan_usage;` in `crates/micold-core/src/lib.rs`

---

## Phase 2: Foundational (core model, settings, wire)

**Purpose**: The source-neutral reading model, the two settings and the wire messages every later
phase uses. No I/O.

### Tests (write first, see them fail)

- [ ] T003 [P] Core model tests in `crates/micold-core/tests/plan_usage.rs` (data-model *UsageReading*, *LimitWindow*, *CurrentUsage*; research R7, R11, R12): `LimitWindow::name` maps `five_hour` → "5-hour", `seven_day` → "Weekly", `seven_day_opus` → "Seven day opus"; `percent_label` rounds tenths half up (`425` → "43%", `424` → "42%"); `at_or_above` is `used_tenths >= threshold * 10` and a window above 1000 counts as above any threshold; `UsageReading::current(now)` drops windows with `resets_at <= now`, keeps a window with no `resets_at`, returns `None` when none remain; headline = highest `used_tenths`, tie → earliest `resets_at` (absent last), tie → first; `same_windows` ignores `obtained_at`; `CurrentUsage::label` gives `"42% · 15:30"`, or `"42%"` when the headline has no reset; `details` gives one `"5-hour: 42%, resets 15:30"` line per window, then `"At or above 80%: 5-hour"` when any window is at or above the threshold, then `"Updated 14:02"`; `warning(threshold)` lists the windows at or above it; `format_reset(resets_at, now, offset)` gives `HH:MM` within 24 h and `Mon HH:MM` beyond, for UTC and a non-zero `FixedOffset` (FR-008)
- [ ] T004 [P] Settings tests in `crates/micold-core/tests/settings_plan_usage.rs` (data-model *WarnPercent*, *Settings*; FR-001, FR-005): `WarnPercent::clamped` maps 49 → 50, 101 → 100, keeps 50..=100; `WarnPercent::parse` refuses "49", "101", "", "8o" with the message "Enter a whole number from 50 to 100" and accepts "50", "80", "100"; `Settings::default()` has `plan_usage_enabled == true` (`DEFAULT_PLAN_USAGE_ENABLED`) and `plan_usage_warn_percent == 80`; a stored `settings.json` without either field loads `true`, `80`; a stored `30` loads as `50`; `settings_version` is unchanged (mirror `crates/micold-core/tests/settings_long_task_threshold.rs`)
- [ ] T005 [P] Status-line parse tests in `crates/micold-core/tests/plan_usage_status_line.rs` (data-model *Status-line extraction*; spec Edge Cases): `extract` returns exactly the `rate_limits` object of a full status-line input and `None` for non-JSON or input without `rate_limits`; `parse` turns `five_hour`/`seven_day` into windows with `used_percentage` 42.04 → `used_tenths` 420; drops a window with negative `used_percentage`, a non-numeric or missing `used_percentage`, or `resets_at <= obtained_at`; keeps a window with no `resets_at`; keeps a percentage above 100 as reported (105 → 1050); excludes `spend_limit`; keeps an unknown key holding `used_percentage`; returns `None` when no window survives
- [ ] T006 [P] Wire tests: extend `crates/micold-core/tests/settings_contract_examples.rs` with a stored-settings example carrying both fields and one without them (defaults `true`, `80`; contract W4); extend `crates/micold-core/tests/protocol_roundtrip.rs` with `DaemonMsg::Welcome { plan_usage: Some(..) }`, the W4 `PlanUsageChanged` example verbatim, `PlanUsageChanged { reading: None }`, and `ClientMsg::SettingsSet` with `plan_usage_enabled` / `plan_usage_warn_percent` set and unset

### Implementation

- [ ] T007 Implement `UsageReading` (`windows: Vec<LimitWindow>` non-empty, `obtained_at: i64`; derives `Debug, Clone, PartialEq, Eq, Serialize, Deserialize`), `LimitWindow` (`key: String` non-empty, `used_tenths: u32`, `resets_at: Option<i64>`), `CurrentUsage` and `format_reset` in `crates/micold-core/src/plan_usage.rs` until T003 passes
- [ ] T008 Implement `WarnPercent` (`u8` newtype, 50..=100, default 80) in `crates/micold-core/src/plan_usage.rs`, and `plan_usage_enabled: bool` (serde default `DEFAULT_PLAN_USAGE_ENABLED = true`) and `plan_usage_warn_percent: u8` (serde default 80, clamped on read by `clamp_plan_usage_warn_percent`) in `crates/micold-core/src/settings.rs`, following `long_task_threshold_secs`, until T004 passes
- [ ] T009 Implement `status_line::extract(stdin: &[u8]) -> Option<serde_json::Value>` and `status_line::parse(body: &serde_json::Value, obtained_at: i64) -> Option<UsageReading>` in `crates/micold-core/src/plan_usage/status_line.rs` until T005 passes
- [ ] T010 Extend `crates/micold-core/src/protocol/messages.rs` (contract W1–W3): `DaemonSettings` gains `plan_usage_enabled: bool`, `plan_usage_warn_percent: u8`; `ClientMsg::SettingsSet` gains `plan_usage_enabled: Option<bool>`, `plan_usage_warn_percent: Option<u8>`; `DaemonMsg::Welcome` gains `plan_usage: Option<UsageReading>`; new `DaemonMsg::PlanUsageChanged { reading: Option<UsageReading> }`; T006 passes (`SCHEMA_HASH` is rebaked by `build.rs`)

**Checkpoint**: `mise run test-core` green.

---

## Phase 3: User Story 1 + 3 — readings reach the daemon (Priority: P1) 🎯 MVP

**Goal**: The daemon accepts a reading on `POST /status/<uuid>`, keeps one for the account
(newest wins), puts it in `Welcome` and broadcasts it; anything it cannot use is answered and
dropped silently.

**Independent Test**: `cargo test -p micold-daemon --test plan_usage_route --test plan_usage_setting`.

### Tests (write first, see them fail)

- [ ] T011 [P] [US1] Route and state tests in `crates/micold-daemon/tests/plan_usage_route.rs` (contract S3; research R5, R6, R7): not POST → 405; path neither `/status/<uuid>` nor `/hook/<uuid>` → 404; missing or wrong token → 403, decided before the body is read; body over 16 KiB → 413; body not a JSON object → 400; object with no usable window → 200 and nothing stored; usable reading → 200 and every connected client gets `PlanUsageChanged` with `obtained_at` stamped by the daemon; a second session's different reading replaces the first (FR-019); ten sessions posting the same windows leave one stored reading and cause one broadcast (Principle II); an identical reading within 60 s is not broadcast again; `Welcome` carries the stored reading, and `None` once every window's reset has passed; with `plan_usage_enabled` off a valid reading is answered 200 and dropped (S3.2)
- [ ] T012 [US3] Silence tests in `crates/micold-daemon/tests/plan_usage_route.rs` (same file as T011, after it) (S3.1; FR-015, FR-016, SC-003): with a captured tracing subscriber, a rejected request is logged at `debug` once per (session, answer) until that session's next accepted reading, never at `warn` or above, and the request body never appears in any log line
- [ ] T013 [P] [US3] Setting tests in `crates/micold-daemon/tests/plan_usage_setting.rs` (contract W3; FR-002, FR-018; mirror `crates/micold-daemon/tests/settings_long_task_threshold.rs`): `SettingsSet { plan_usage_warn_percent: Some(30) }` stores 50, persists and broadcasts `SettingsChanged`; `DaemonSettings` carries both fields with defaults `true`, `80`; `plan_usage_enabled: Some(false)` clears the stored reading and broadcasts `PlanUsageChanged { reading: None }`

### Implementation

- [ ] T014 [US1] In `crates/micold-daemon/src/state.rs` add `plan_usage: Option<UsageReading>` to `Inner` (memory only, FR-014), `note_plan_usage(&self, session, reading) -> bool` (false while off or for an unknown session; replaces; true unless only `obtained_at` moved within 60 s, R6), the reading part of `set_plan_usage_enabled(bool)` (off clears and broadcasts `None`), and the `Welcome` value with all-passed readings dropped (R7)
- [ ] T015 [US1] Plumb both settings through `crates/micold-daemon/src/catalog.rs` and `crates/micold-daemon/src/server.rs` exactly as `long_task_threshold_secs` (clamp the percent before storing), fill `Welcome.plan_usage`, and broadcast `PlanUsageChanged` when `note_plan_usage` returns true
- [ ] T016 [US1] Add the `POST /status/<session-uuid>` route to the hook receiver in `crates/micold-daemon/src/hooks.rs` (S3 table; token checked before the body; 16 KiB bound; `status_line::parse` with the daemon's wall clock; `note_plan_usage`; debug-once rejection log keyed by (session, answer), reset on accept) until T011–T013 pass
- [ ] T017 [US1] Keep the client building on the new wire without showing anything: in `crates/micold-client/src/shell/daemon_sync.rs` accept `Welcome.plan_usage` and `PlanUsageChanged` and drop them, and in `crates/micold-client/src/features/settings.rs` send `None` for the two new `SettingsSet` fields (T034 and T044 replace them)

**Checkpoint (M1)**: a POST to `/status/<uuid>` with the session's hook token is broadcast to every client; `mise run gate` green.

---

## Phase 4: User Story 1 + 3 — Claude Code sessions feed the daemon, the user's status line kept (Priority: P1)

**Goal**: Each Claude session the app starts carries a `statusLine` block that runs the relay; the
relay posts `rate_limits` and runs the user's own status line unchanged. Off: nothing is given.

**Independent Test**: quickstart Part A relay smoke test (output and `exit=0`);
`cargo test -p micold-daemon --test status_relay --test hooks_receiver --test plan_usage_setting`.

### Tests (write first, see them fail)

- [ ] T018 [P] [US3] User status line tests in `crates/micold-core/tests/plan_usage_user_status_line.rs` (data-model *UserStatusLine*, *RelayConfig*; research R4; FR-013, FR-020): precedence `cwd/.claude/settings.local.json` → `cwd/.claude/settings.json` → `<claude_config_dir or home/.claude>/settings.json` with `CLAUDE_CONFIG_DIR` honoured through `ConfigLocations`; only a `statusLine` with `type: "command"` and a non-empty `command` counts; `padding` and `refreshInterval` carried; absent, unreadable or malformed files name nothing and fall through; a file whose other keys have unexpected types (an `env` map of numbers) still yields its `statusLine` (only `statusLine` is deserialised); `RelayConfig { url, token, user_command }` round-trips through serde; `ConfigLocations::claude_settings_dir()` returns `CLAUDE_CONFIG_DIR` when set, else `home/.claude`
- [ ] T019 [P] [US1] Relay tests in `crates/micold-daemon/tests/status_relay.rs` with injected stdin, stdout, shell and a fake loopback receiver (contract S2; FR-012, FR-014, FR-017, FR-020): the posted body is exactly the `rate_limits` object with `Authorization: Bearer <token>` (S2.3); the user command gets the complete stdin and `MICOLD_STATUS_RELAY=1`, its stdout is copied unchanged and as it arrives (S2.4); a receiver that never answers does not delay the user's output and the POST gives up at 500 ms; exit status is the user command's, else 0 (S2.5); the relay writes nothing to stdout or stderr in any failure (missing, unreadable or malformed relay file, refused POST, failing user command); stdin over 1 MiB posts nothing and still runs the user command (S2.1); a missing relay file runs no user command and exits 0 (S2.2); started with `MICOLD_STATUS_RELAY=1` it posts nothing and runs no user command (S2.6); and, at process level, the built `micold-daemon` binary run as `status-line <missing-file>` with JSON on stdin exits 0 with empty stdout and stderr and starts no daemon (the `main.rs` dispatch, T024)
- [ ] T020 [P] [US3] Settings-file tests: extend `crates/micold-daemon/tests/hooks_receiver.rs` (contract S1; FR-002): with the switch on, `settings_json` has a `statusLine` block of `type: "command"` whose `command` is the double-quoted daemon path, `status-line`, and the double-quoted relay path with `/` separators (S1.2); `padding` and `refreshInterval` appear only when the user's status line sets them (S1.1); with it off there is no `statusLine` key and the `hooks` map is byte-identical (S1.3); `prepare_settings` writes `<uuid>.status.json` owner-only when on and none when off; `forget` deletes it; sessions of other AI CLIs get no `statusLine` block and no relay file in either state
- [ ] T021 [P] [US3] Switch tests: extend `crates/micold-daemon/tests/plan_usage_setting.rs` (research R9; FR-002, FR-007, SC-007): turning the switch off rewrites every live session's settings file without `statusLine` and deletes its relay file; turning it on rewrites them with the block and writes the relay files

### Implementation

- [ ] T022 [US3] Add `ConfigLocations::claude_settings_dir()` beside `claude_json()` in `crates/micold-core/src/mcp/binding.rs`; declare `pub mod user_status_line;` in `crates/micold-core/src/plan_usage.rs` and implement `UserStatusLine::resolve(locations, cwd)` and `RelayConfig` in `crates/micold-core/src/plan_usage/user_status_line.rs` until T018 passes
- [ ] T023 [US1] Implement the relay in `crates/micold-daemon/src/status_relay.rs` (S2.1–S2.6): POST and user command concurrently, the POST bounded at 500 ms, `user_shell` choosing `sh -c` on Unix and on Windows Git Bash (`CLAUDE_CODE_GIT_BASH_PATH`, else `bash.exe` beside `git.exe` on `PATH`), else `powershell -NoProfile -Command` (R4), until T019 passes
- [ ] T024 [US1] Dispatch `micold-daemon status-line <relay-file>` in `crates/micold-daemon/src/main.rs` before the runtime, logging and singleton start (S2)
- [ ] T025 [US1] In `crates/micold-daemon/src/hooks.rs`: `settings_json` gains an optional typed `statusLine` struct (S1.4, no `json!`); `prepare_settings` resolves `UserStatusLine` against the session's cwd and writes the relay file with `micold_core::owner_only::write` while the switch is on; `forget` deletes the relay file with the token; until T020 passes
- [ ] T026 [US1] Complete `set_plan_usage_enabled` in `crates/micold-daemon/src/state.rs`: rewrite every registered session's settings file and write or delete its relay file (R9) until T021 passes
- [ ] T027 [US3] Add *Claude plan usage* to `docs/user-guide/worktrees-and-sessions.md` (FR-003, FR-020; Principle VII): the values come from Claude Code's own status-line data in the sessions the app runs; the app reads only the `statusLine` key of `.claude/settings.local.json`, `.claude/settings.json` and `~/.claude/settings.json` (or `CLAUDE_CONFIG_DIR`) to keep the user's own status line; no credential is read; nothing is sent anywhere; the user's own status line keeps working

**Checkpoint (M2)**: quickstart Part A relay smoke test prints the user's status line and `exit=0`; `mise run gate` green.

---

## Phase 5: User Story 1 + 3 — the usage indicator (Priority: P1)

**Goal**: The app bar shows the highest current window and its reset, with every window in the
tooltip; hidden whenever there is no current reading.

**Independent Test**: `cargo test -p micold-client --test features_plan_usage --test idle_subscriptions --test toolbar`; quickstart §B1, §B5.

### Tests (write first, see them fail)

- [ ] T028 [P] [US1] Feature tests in `crates/micold-client/tests/features_plan_usage.rs` (data-model *Client state*; US1 s1–s4, US3 s1–s4, s6, s7): the reading is set from `Welcome.plan_usage` and from `PlanUsageChanged`, replaced by a newer one, and reset to `None` on disconnect; the view model is `None` when the switch is off, when there is no reading, and once every window's reset has passed; otherwise it yields the label (`"42% · 15:30"`) and the details lines of `CurrentUsage` for the given offset
- [ ] T029 [P] [US1] Extend `crates/micold-client/tests/idle_subscriptions.rs` (R7): no plan-usage tick without a reading; a 30 s tick while one exists
- [ ] T030 [P] [US1] Extend `crates/micold-client/tests/icons.rs` and `crates/micold-client/tests/icon_roles.rs`: `Icon::PlanUsage` is U+E1AF in the `on_surface_variant` role (U2)
- [ ] T031 [P] [US1] Extend `crates/micold-client/tests/toolbar.rs` and `crates/micold-client/tests/gates/bar_controls_hold_their_size.rs` (U2.2, U3, U4): the indicator is the first trailing action, before the project switcher, only when the switch is on and a current reading exists; without it the bar is unchanged; with it, it is 40 px high like the project switcher and the bar's other actions keep their positions

### Implementation

- [ ] T032 [US1] Create the render-free `crates/micold-client/src/features/plan_usage.rs` (reading state, view model from `UsageReading::current(now)` and the settings' threshold, tick decision) and register it in `crates/micold-client/src/features/mod.rs` until T028 passes
- [ ] T033 [US1] Add `Icon::PlanUsage` (U+E1AF, `data_usage`) to `crates/micold-client/src/icons.rs` until T030 passes
- [ ] T034 [US1] Route `Welcome.plan_usage` and `PlanUsageChanged` into the feature in `crates/micold-client/src/shell/daemon_sync.rs` (replacing T017's drop, clearing on disconnect), and add the 30 s `iced::time::every` tick while a reading exists in `crates/micold-client/src/shell/subscriptions.rs` until T029 passes
- [ ] T035 [US1] Create the shared `UsageIndicator` (normal look; `new(label, roles)`, `.details(lines)`, `.into()`; 40 px, 8 px gap, 12 px ends; shared `Tooltip`; not pressable) in `crates/micold-client/src/ui/material/usage_indicator.rs` and export it from `crates/micold-client/src/ui/material/mod.rs` (contract U1, U2)
- [ ] T036 [US1] Place the indicator as the first `Toolbar::action` in `crates/micold-client/src/ui/toolbar.rs`, mapping the feature's view model with the `chrono::Local` offset (U3), until T031 passes
- [ ] T037 [P] [US1] Add a `UsageIndicator` showcase entry in `crates/micold-client/src/showcase/sections/atoms.rs` with a normal and a long-label sample in `crates/micold-client/src/showcase/samples.rs` (U4)
- [ ] T038 [US1] Extend *Claude plan usage* in `docs/user-guide/worktrees-and-sessions.md` with the app-bar indicator: what the indicator shows, that it appears once a Claude session has had a response, its tooltip, and that it hides when there is no current reading (Principle VII)

**Checkpoint (M3)**: quickstart §B1 and §B5 pass; `mise run gate` green.

---

## Phase 6: User Story 2 — the warning look (Priority: P2)

**Goal**: At or above the threshold in any window, the indicator switches to its warning look.

**Independent Test**: `cargo test -p micold-client --test features_plan_usage --test icons --test icon_roles`; quickstart §B2.

### Tests (write first, see them fail)

- [ ] T039 [P] [US2] Extend `crates/micold-client/tests/features_plan_usage.rs` (US2 s1–s4; FR-010): with threshold 80 the view model warns at 80.0% and not at 79.9%; a window above 100% warns; a newer reading below the threshold returns to normal; changing the threshold in `DaemonSettings` changes the look of the stored reading without a new reading; reaching the threshold produces no notification or dialog command, only the view-model change (FR-011)
- [ ] T040 [P] [US2] Extend `crates/micold-client/tests/icons.rs`, `crates/micold-client/tests/icon_roles.rs` and `crates/micold-client/tests/gates/bar_controls_hold_their_size.rs`: `Icon::UsageWarning` is U+E002 in the `error` role; the warning look keeps the 40 px height

### Implementation

- [ ] T041 [US2] Add `Icon::UsageWarning` (U+E002) to `crates/micold-client/src/icons.rs`, `.warning(bool)` to `UsageIndicator` in `crates/micold-client/src/ui/material/usage_indicator.rs` (glyph and label in `error`, U2), and a warning sample to the showcase in `crates/micold-client/src/showcase/samples.rs`
- [ ] T042 [US2] Pass `!current.warning(threshold).is_empty()` from the view model in `crates/micold-client/src/features/plan_usage.rs` through `crates/micold-client/src/ui/toolbar.rs` until T039 and T040 pass

---

## Phase 7: User Story 4 — the settings (Priority: P3)

**Goal**: Settings → Environment has the switch, the threshold and the statement of what is read.

**Independent Test**: `cargo test -p micold-client --test features_settings --test settings_sections`; quickstart §B2, §B3.

### Tests (write first, see them fail)

- [ ] T043 [P] [US4] Extend `crates/micold-client/tests/features_settings.rs` and `crates/micold-client/tests/settings_sections.rs` (US4 s1–s4; FR-001, FR-003, FR-005, FR-018): the drafts start from `DaemonSettings` (switch on, "80"); "49" and "101" are refused with "Enter a whole number from 50 to 100" and nothing is sent; a valid save sends both fields in `SettingsSet`; a `SettingsChanged` from another window updates the drafts; the Environment section has **Claude plan usage** after **Desktop notifications**, and its field note says the values come from Claude Code's status-line data in the sessions the app runs, that no credential is read and that nothing is sent anywhere

### Implementation

- [ ] T044 [US4] Add the switch and threshold drafts with `WarnPercent::parse` validation to `crates/micold-client/src/features/settings.rs`, replacing T017's `None`s, following `long_task_threshold_secs`
- [ ] T045 [US4] Add the **Claude plan usage** subsection (switch **Show Claude plan usage**, threshold field, FR-003 statement) to `crates/micold-client/src/ui/settings/environment.rs` until T043 passes
- [ ] T046 [US4] Add *Environment → Claude plan usage* to `docs/user-guide/settings.md`: the switch (on by default and why), the threshold (80 by default, 50–100), the warning look, and the FR-003 statement (source, only `statusLine` read from Claude Code's settings files, no credential, nothing sent), linking the T027 section

---

## Phase 8: Polish

Doc-only; done by the close unit, not a milestone.

- [ ] T047 Run quickstart Part B §B1–§B6 (the `visual-pass` skill for §B2 and §B6) and record the results in `specs/490-claude-plan-usage/quickstart.md`; §B4's Windows run and the real-account steps are marked for the user when no such machine or account is available
- [ ] T048 Confirm CI's Linux, macOS and Windows jobs ran the feature's tests on the last milestone PR and note it in `specs/490-claude-plan-usage/quickstart.md` (Principle VI)

---

## Dependencies & Execution Order

- Phase 1 → Phase 2 → Phase 3 (M1). Phase 4 (M2) needs Phase 3's route. Phase 5 (M3) needs M1's
  wire and, to show anything, M2's relay. Phases 6–7 (M4) need M3's indicator.
- Within a phase: tests first, then implementation in the listed order; [P] tests touch different
  files.
- T017 is a placeholder that T034 and T044 replace; no other task depends on it.

### Parallel examples

```text
Phase 2 tests: T003, T004, T005, T006 together.
Phase 4 tests: T018, T019, T020, T021 together.
Phase 5 tests: T028, T029, T030, T031 together; T037 beside T036.
```

## Implementation Strategy

MVP is M1–M3: readings reach the daemon, Claude sessions feed them, the bar shows them. M4 adds
the warning look and the settings. The source is isolated in M2 (plan Summary): if the open
escalation picks another source, only M2's tasks and `DEFAULT_PLAN_USAGE_ENABLED` change.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Readings reach the daemon 🎯 MVP

- **Tasks**: T001–T017
- **Deliverable**: a `POST /status/<uuid>` carrying a `rate_limits` body and the session's hook token makes the daemon keep the reading (newest wins), put it in `Welcome` and broadcast `PlanUsageChanged` to every client; anything else is answered 4xx/200-dropped and logged only at debug, once
- **Satisfies**: US1 s3 (daemon half: route to broadcast); US3 s1, s7 (no reading → nothing); FR-005 (clamp), FR-008, FR-009 (model), FR-014, FR-015, FR-016, FR-018 (settings broadcast), FR-019
- **Verify**: `cargo test -p micold-core --test plan_usage --test plan_usage_status_line --test settings_plan_usage --test settings_contract_examples --test protocol_roundtrip` and `cargo test -p micold-daemon --test plan_usage_route --test plan_usage_setting`
- **Depends on**: —
- **Tier**: full

### M2 — Claude Code sessions feed the daemon, the user's status line kept

- **Tasks**: T018–T027
- **Deliverable**: every Claude session the app starts with the switch on posts its plan-usage values to the daemon through `micold-daemon status-line`, while showing the user's own status line unchanged; with the switch off, sessions get no `statusLine` and no relay file
- **Satisfies**: US1 s3 (session half: Claude Code to route); US3 s5, s8; FR-002, FR-003 (guide), FR-007, FR-012, FR-013, FR-014, FR-017, FR-020; SC-004, SC-006, SC-007
- **Verify**: quickstart Part A relay smoke test, up to the user's status line output and `exit=0` (the app-bar line is M3's); `cargo test -p micold-core --test plan_usage_user_status_line` and `cargo test -p micold-daemon --test status_relay --test hooks_receiver --test plan_usage_setting`
- **Depends on**: M1
- **Tier**: full

### M3 — The usage indicator in the app bar

- **Tasks**: T028–T038
- **Deliverable**: the app bar shows `NN% · HH:MM` for the highest current window with every window and the reading time in its tooltip, and shows nothing when there is no current reading
- **Satisfies**: US1 s1–s4; US3 s1–s4, s6, s7; FR-004, FR-006, FR-007 (client side), FR-008, FR-009, FR-015; SC-001, SC-002 (manual, §B1), SC-003
- **Verify**: `cargo test -p micold-client --test features_plan_usage --test idle_subscriptions --test toolbar --test icons --test icon_roles` and the bar gate `bar_controls_hold_their_size`; quickstart §B1, §B5
- **Depends on**: M1, M2
- **Tier**: full

### M4 — Warning look and plan-usage settings

- **Tasks**: T039–T046
- **Deliverable**: at or above the threshold the indicator shows the warning glyph in the error colour, and Settings → Environment has the switch, the 50–100 threshold and the statement of what is read
- **Satisfies**: US2 s1–s4; US4 s1–s4; FR-001, FR-003, FR-005, FR-010, FR-011, FR-018; SC-005
- **Verify**: `cargo test -p micold-client --test features_plan_usage --test features_settings --test settings_sections --test icons --test icon_roles`; quickstart §B2, §B3
- **Depends on**: M3
- **Tier**: light
