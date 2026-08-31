---

description: "Task list for feature 028 — client-managed session service lifecycle"
---

# Tasks: Client-Managed Session Service Lifecycle

**Input**: Design documents from `/specs/028-client-managed-daemon/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md

**Tests**: Per Constitution Principle I (Test-First Development, NON-NEGOTIABLE), test tasks are
MANDATORY. Every user story writes its failing tests before its implementation tasks.

**Documentation**: Per Constitution Principle VII, every user-facing story ships its documentation in
the same change.

**Cross-platform**: Per Constitution Principle VI, the rule is platform-agnostic; only the clock
reading is `cfg`-split, behind one function in `micold-core`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: US1–US5, mapping to the user stories in `spec.md`
- Every task names the exact file it touches

## Path Conventions

Three-crate Rust workspace: `crates/micold-core/`, `crates/micold-daemon/`, `crates/micold-client/`,
with `packaging/` and `docs/` at the repository root. Build and test through `mise run <task>`, never
bare `cargo` (see `CLAUDE.md`).

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Capture the before-state and put the two new module files in place, so every later task
touches exactly one file.

- [ ] T001 Record the pre-change baseline — `mise run test` green, and the current `systemctl --user list-unit-files | grep micold` output — in `specs/028-client-managed-daemon/evidence/baseline.md`
- [ ] T002 [P] Create `crates/micold-core/src/clock.rs` and declare `pub mod clock;` in `crates/micold-core/src/lib.rs`
- [ ] T003 [P] Create `crates/micold-daemon/src/idle.rs` and declare `pub mod idle;` in `crates/micold-daemon/src/lib.rs`

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The pure primitives every stopping-related story rests on — the clock, the presence
count, the rule, the stop reason — plus retiring the predicate they replace.

**⚠️ CRITICAL**: US3, US4 and US5 cannot start until this phase is done. **US1 does not depend on it**
and may be delivered first or in parallel (see Dependencies).

### Tests (MANDATORY — Constitution Principle I) ⚠️

- [ ] T004 [P] Write failing unit tests for `Uptime` in `crates/micold-core/src/clock.rs` (`#[cfg(test)]`): monotonic across many consecutive reads, and `saturating_sub` of a later reading from an earlier one yields zero rather than panicking (data-model G3)
- [ ] T005 [P] Write failing unit tests for `Presence` in `crates/micold-daemon/src/idle.rs`: `alone_since.is_some() ⟺ connected == 0`, a reconnect clears the armed deadline, and a daemon that has never had a client is idle from construction (data-model G1)
- [ ] T006 [P] Write failing unit tests for `IdleWindow::expired` and `IDLE_WINDOW` in `crates/micold-daemon/src/idle.rs`: zero connections plus the window ⇒ expired, one connection ⇒ never, and the constant is 30 minutes (data-model G2, FR-008, FR-017)
- [ ] T007 Rewrite `crates/micold-daemon/tests/daemon_lifecycle.rs` lines 20 and 73–104 to assert the clarified rule — a live session does **not** hold the daemon up — replacing the `may_exit`/`Lifecycle` assertions this feature retires (FR-006a); it must fail until T009–T011

### Implementation

- [ ] T008 Implement `Uptime` and `now()` in `crates/micold-core/src/clock.rs` — `CLOCK_BOOTTIME` on Linux, `mach_continuous_time()` on macOS, `GetTickCount64()` on Windows — behind one `cfg`-free public signature, using the `libc`/`windows-sys` deps already declared in `crates/micold-core/Cargo.toml:42-50` (research R3, Principle VI)
- [ ] T009 Implement `Presence` in `crates/micold-daemon/src/idle.rs` with `client_connected` / `client_disconnected` as the only transitions and `alone_since` initialised at construction (data-model G1)
- [ ] T010 Implement `IdleWindow`, the `IDLE_WINDOW` constant and `StopReason` in `crates/micold-daemon/src/idle.rs` (data-model G2, G4)
- [ ] T011 Replace the `Lifecycle` field and its `lifecycle()` accessor with `Presence` in `crates/micold-daemon/src/state.rs:34,45,242,254`, then delete `crates/micold-daemon/src/lifecycle.rs` and its `pub mod lifecycle;` declaration in `crates/micold-daemon/src/lib.rs`
- [ ] T012 Feed `Presence` from the real accept loop — `state.register` at `crates/micold-daemon/src/server.rs:369` and `state.deregister` at `:393` — closing the "no call sites" gap research R1 found

**Checkpoint**: the rule, the counter and the clock exist, are tested, and are wired to real
connections; nothing yet acts on them.

---

## Phase 3: User Story 1 - Nothing to install, nothing to register (Priority: P1) 🎯 MVP

**Goal**: The application becomes the only thing that starts the session service. No unit ships, no
socket activation exists, no logout-survival opt-in registers anything, and an upgrade cleans up what
a previous release enabled.

**Independent test**: On a clean machine, `systemctl --user list-unit-files | grep micold` is empty
after installing and after using the app; on a machine upgraded from the previous release with the
old opt-in enabled, it is empty after opening the app once. (quickstart B7, B8)

### Tests (MANDATORY — Constitution Principle I) ⚠️

- [ ] T013 [P] [US1] Write failing guard test `crates/micold-daemon/tests/no_socket_activation.rs` — no `listenfd` or `LISTEN_FDS` reference remains under `crates/micold-daemon/src/`, and `crates/micold-daemon/Cargo.toml` declares no `listenfd` dependency (lifecycle contract §1.3; source-guard style follows `crates/micold-core/tests/documentation_is_not_read.rs`)
- [ ] T014 [P] [US1] Write failing test `crates/micold-client/tests/deb_ships_no_service_units.rs` — parse `[package.metadata.deb].assets` in `crates/micold-client/Cargo.toml` and assert no destination under `usr/lib/systemd` and no source under `packaging/micold-daemon.` (packaging contract §1.1–1.2)
- [ ] T015 [P] [US1] Write failing unit tests in `crates/micold-client/src/shell/legacy_units.rs` (`#[cfg(test)]`): the un-enable is attempted when a unit is enabled, skipped when nothing is, every failure is swallowed, and it does not repeat once nothing is enabled (packaging contract §2.6–2.7)
- [ ] T016 [P] [US1] Write failing guard test `crates/micold-client/tests/no_host_logout_survival.rs` — no `logout_survival::enable`, `LogoutSurvivalRequested`, `LogoutSurvivalOutcome`, or "Keep sessions after logout" string remains in `crates/micold-client/src/` or `crates/micold-core/src/` (FR-005, packaging contract §4.11)

### Implementation

- [ ] T017 [US1] Delete `systemd_listener()` (`crates/micold-daemon/src/server.rs:251-265`), `serve_unix()` (`:283-296`) and the adoption branch in `run()` (`:182-188`), leaving `singleton::acquire` as the only bind path (lifecycle contract §1.3); `crates/micold-daemon/tests/daemon_singleton.rs` and `exclusivity.rs` MUST still pass unchanged — that is FR-004's regression evidence for the removed bind paths
- [ ] T018 [US1] Remove `listenfd` from `crates/micold-daemon/Cargo.toml:33` and from `[workspace.dependencies]` in the root `Cargo.toml` if no other crate uses it
- [ ] T019 [P] [US1] Delete `packaging/micold-daemon.service` and `packaging/micold-daemon.socket`
- [ ] T020 [US1] Remove the two `usr/lib/systemd/user/…` asset lines and their comment block from `[package.metadata.deb].assets` in `crates/micold-client/Cargo.toml:72-80` (packaging contract §2.5 — dpkg then removes them on upgrade with no maintainer script)
- [ ] T021 [P] [US1] Trim `crates/micold-core/src/logout_survival.rs` to what the sandbox still needs — research R8 keeps the module, so: delete both `cfg` arms of `enable()` (lines 72-106) and the `run()` helper they orphan (line 134, or T060's `clippy -D warnings` fails on dead code), and make `enable_for`'s `Placement::HostProcess` arm return `SurvivalOutcome::Unsupported`; the sandbox arm and `PendingSandboxRestart` stay
- [ ] T022 [US1] Delete `on_logout_survival_requested` and `on_logout_survival_outcome` and their test from `crates/micold-client/src/shell/service_control.rs:72-110,179`
- [ ] T023 [US1] Delete `Message::LogoutSurvivalRequested` and `Message::LogoutSurvivalOutcome` from `crates/micold-client/src/app.rs:612,1067` and their dispatch arms in `crates/micold-client/src/main.rs:662-665`
- [ ] T024 [US1] Remove the "Keep sessions after logout" overflow-menu item from `crates/micold-client/src/ui/toolbar.rs:41-46`
- [ ] T025 [US1] Implement the one-shot migration in `crates/micold-client/src/shell/legacy_units.rs` — `systemctl --user disable --now micold-daemon.socket micold-daemon.service`, decision logic render-free and tested, all failures swallowed (research R7, packaging contract §2.6)
- [ ] T026 [US1] Invoke the migration in `crates/micold-client/src/main.rs` **before** the client connects or auto-spawns a daemon — research R7's ordering hazard: run it after connecting and socket activation starts a daemon from a unit file that no longer exists
- [ ] T027 [P] [US1] Rewrite `docs/daemon.md` — the "Surviving logout" section (lines 286–330), the lifetime-table row at line 22, and the "logs to the systemd journal" note at line 276 — to state that a directly-hosted service does not survive logout and to name the sandboxed placement as the supported way to get that (FR-005c, packaging contract §4.12)
- [ ] T028 [P] [US1] Record quickstart B8 (clean install registers nothing) and B7 (upgrade migration) in `specs/028-client-managed-daemon/evidence/us1-packaging.md`

**Checkpoint**: US1 is independently shippable — the service is no longer a system service, whether
or not anything else in this feature lands.

---

## Phase 4: User Story 2 - Work outlives the window (Priority: P1)

**Goal**: Quitting, closing or crashing the application changes no session's fate, and the presence
count that the idle rule reads is the honest one.

**Independent test**: Start a session, kill the client process, reopen — the session is still running
and reattaches. Kill a client without a clean close and confirm the daemon counts it gone within 60
seconds.

**Note on documentation**: this story's promise is unchanged from feature 010 and is already
documented; the sentences it shares with the new behaviour are written by T027 and T058.

### Tests (MANDATORY — Constitution Principle I) ⚠️

- [ ] T029 [P] [US2] Write failing test `crates/micold-daemon/tests/presence_counting.rs` — a refused handshake never increments the count, a completed handshake increments it once, a clean close decrements it (lifecycle contract §2.5)
- [ ] T030 [P] [US2] Write failing test in `crates/micold-daemon/tests/presence_counting.rs` — a connection dropped without a clean close is counted as gone within 60 seconds (lifecycle contract §2.6, research R6)
- [ ] T031 [P] [US2] Extend `crates/micold-daemon/tests/session_survival.rs` with a regression asserting the daemon still outlives client exit now that the systemd path is gone (FR-006, lifecycle contract §2.4)

### Implementation

- [ ] T032 [US2] Make `Presence` the single count in `crates/micold-daemon/src/state.rs` — `register`/`deregister` its only mutators, no second counter anywhere (lifecycle contract §2.5)
- [ ] T033 [US2] Confirm and, where missing, add EOF- and error-path deregistration in `crates/micold-daemon/src/server.rs:376-393`, so a crashed client is counted gone without a keepalive (research R6)

**Checkpoint**: the count the rule will read is correct under clean exits, crashes and refusals.

---

## Phase 5: User Story 3 - Idle work is not left running forever (Priority: P1)

**Goal**: The service stops itself after 30 continuous minutes with no application connected, cleanly
and completely, and the next start is indistinguishable from a first start.

**Independent test**: quickstart B1 (a real thirty minutes), B2 (connections not activity), B3
(suspend counts), B4 (a live session does not save it).

### Tests (MANDATORY — Constitution Principle I) ⚠️

- [ ] T034 [P] [US3] Write failing integration test `crates/micold-daemon/tests/idle_stop.rs` — with a test-only short window, a daemon with zero connections exits by itself; with one connection it never does (FR-008, FR-009, lifecycle contract §3.7, §3.9)
- [ ] T035 [P] [US3] Write failing test in `crates/micold-daemon/tests/idle_stop.rs` for the shutdown order — sessions are marked `InterruptedResumable` and persisted **before** their process trees are killed, and the endpoint is released last (data-model G5, lifecycle contract §3.11)
- [ ] T036 [P] [US3] Write failing test `crates/micold-daemon/tests/idle_teardown.rs` — after the stop: no descendant process, no file at the endpoint path, the lock released, and a fresh daemon binds the same endpoint with no recovery step — looped over 20 consecutive stop-and-restart cycles, asserting zero residue on each (FR-013, FR-014, SC-007, lifecycle contract §3.12–3.13)
- [ ] T037 [P] [US3] Write failing test `crates/micold-daemon/tests/idle_race.rs` — a connect issued as the window expires ends attached to a working daemon with no `DaemonConnectFailed` reaching the client (FR-016, lifecycle contract §4.14, research R5)
- [ ] T038 [P] [US3] Write failing test in `crates/micold-daemon/tests/idle_stop.rs` — a live session does **not** extend the window, and afterwards the session is `InterruptedResumable` and did not auto-resume (FR-006a, FR-006b, FR-006c, lifecycle contract §3.10)
- [ ] T039 [P] [US3] Write failing unit test in `crates/micold-client/src/daemon.rs` (`#[cfg(test)]`) — a single transient connect failure is absorbed by the existing `RECONNECT_BACKOFF` at line 126 without raising the connection banner (lifecycle contract §4.15)

### Implementation

- [ ] T040 [US3] Add the 30-second idle tick beside `spawn_supervisor` in `crates/micold-daemon/src/server.rs:220-240`, evaluating `IdleWindow::expired` and signalling shutdown — a tick, not a single 30-minute sleep, so waking from suspend is prompt and the overshoot is bounded at 30 s (research R3, SC-004)
- [ ] T041 [US3] Add a test-only window override (a constructor parameter or env var read once at startup, not a `cfg(test)` fork) in `crates/micold-daemon/src/idle.rs`, so integration tests run in seconds while T006 asserts the real 30-minute constant
- [ ] T042 [US3] Implement the ordered unwind in `crates/micold-daemon/src/server.rs::run()` per data-model G5: diagnostics line, stop accepting, persist sessions as interrupted-resumable, drop the session table so `PtySession::Drop` terminates each process tree (`crates/micold-daemon/src/supervisor.rs:366-382`), drop `BoundListener` last, return `Ok(())`
- [ ] T043 [US3] Ensure `crates/micold-daemon/src/main.rs` returns from `run()` normally and never calls `process::exit`, so every `Drop` in T042's order actually runs (research R4)
- [ ] T044 [P] [US3] Document the idle window in `docs/daemon.md` — when the service stops, what happens to running sessions, and that reopening presents them as resumable (FR-025)

**Checkpoint**: the feature's core promise holds on the host placement, end to end.

---

## Phase 6: User Story 4 - The same rules inside the sandbox (Priority: P2)

**Goal**: Everything above holds in the sandboxed placement, with the one approved exception — while
the keep-it-running opt-in is on, the sandbox is not idle-stopped.

**Independent test**: quickstart B5 (opt-in off — container `exited`, `RestartCount` unchanged) and
B6 (opt-in on — still running, and the copy said so).

### Tests (MANDATORY — Constitution Principle I) ⚠️

- [ ] T045 [P] [US4] Write failing unit test in `crates/micold-core/src/sandbox/argv.rs` (`#[cfg(test)]`) — `create` emits `MICOLD_IDLE_STOP=off` exactly when `profile.survive_logout` is set, alongside the `--restart unless-stopped` that `restart_policy` (line 69) already selects (research R2a)
- [ ] T046 [P] [US4] Write failing unit test in `crates/micold-core/src/sandbox/lifecycle.rs` (`#[cfg(test)]`) — toggling `survive_logout` against a `SandboxState::Running` yields `Stale`, mirroring `mount_set_changed` at line 388 (FR-022a, research R2a)
- [ ] T047 [P] [US4] Write failing real-runtime test `crates/micold-daemon/tests/sandbox_idle.rs` behind `--features sandbox-real-runtime` — with the opt-in off, after the injected window the container reports status `exited` and `RestartCount` unchanged (FR-019, FR-020, lifecycle contract §5.17)
- [ ] T048 [P] [US4] Write failing real-runtime test in `crates/micold-daemon/tests/sandbox_idle.rs` — with the opt-in on, the container is still running after the window, deliberately (FR-022, lifecycle contract §5.18)
- [ ] T049 [P] [US4] Extend `crates/micold-core/tests/sandbox_parity.rs` so the presence count, the window, the clock and the shutdown order are asserted to be the same code on both placements (FR-018, lifecycle contract §5.16)

### Implementation

- [ ] T050 [US4] Pass `MICOLD_IDLE_STOP` in `crates/micold-core/src/sandbox/argv.rs::create`, immediately beside the `MICOLD_IMAGE_REFERENCE` at line 114 and for the same reason — a daemon inside a container cannot see how its container was created (research R2a); pass T041's window override through the same `-e` path, or T047 and T048 have no way to run in seconds
- [ ] T051 [US4] Read it once at daemon startup and suppress `IdleWindow` when its value is `off` — the value decides, not mere presence, so an unset variable or any other value leaves the idle rule in force — in `crates/micold-daemon/src/server.rs::run()` and `crates/micold-daemon/src/idle.rs`
- [ ] T052 [US4] Make `survive_logout` part of what makes a sandbox stale in `crates/micold-core/src/sandbox/lifecycle.rs`, beside `mount_set_changed` — without this FR-022a silently does not hold, because both the restart policy and the environment are fixed at creation (research R2a)
- [ ] T053 [P] [US4] Update `docs/user-guide/sandboxed-daemon.md` — the opt-in now means "keep the sandbox running: it survives logout and reboot, and is not stopped when idle" (FR-022, packaging contract §4.13)
- [ ] T054 [P] [US4] Update the toggle's copy in `crates/micold-client/src/ui/settings/daemon.rs:49,306` so it states that before the choice is made (spec FR-022, quickstart B6 step 3)

**Checkpoint**: both placements behave identically except where the user asked otherwise.

---

## Phase 7: User Story 5 - Knowing what the application is doing (Priority: P3)

**Goal**: An automatic stop is visible and distinguishable from a crash, and the documentation says
what the app leaves running.

**Independent test**: stop a daemon by the idle rule and by `kill -9`; the log distinguishes them
with no other evidence.

### Tests (MANDATORY — Constitution Principle I) ⚠️

- [ ] T055 [P] [US5] Write failing test in `crates/micold-daemon/tests/idle_stop.rs` — an idle stop writes exactly one diagnostics line naming inactivity, **before** teardown begins (FR-024, lifecycle contract §6.19)
- [ ] T056 [P] [US5] Write failing test in `crates/micold-daemon/tests/idle_stop.rs` — a killed daemon produces no such line, so the log alone separates the two (lifecycle contract §6.20, data-model G4)

### Implementation

- [ ] T057 [US5] Emit the `StopReason::Idle` line through `crates/micold-daemon/src/logging.rs` as the first step of T042's unwind in `crates/micold-daemon/src/server.rs`
- [ ] T058 [P] [US5] State in `docs/user-guide/worktrees-and-sessions.md` that closing the window leaves work running and that the service ends itself after a period with nothing connected (FR-025)
- [ ] T059 [P] [US5] Note in `docs/user-guide/settings.md` what the sandbox toggle now promises (FR-025, packaging contract §4.13)

---

## Phase 8: Polish & Cross-Cutting Concerns

- [ ] T060 [P] Run `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings` — the local gate omits `fmt`, and CI stops at `cargo fmt --check` before any other job
- [ ] T061 Run `mise run test` and confirm the whole workspace is green
- [ ] T062 Run `cargo test -p micold-daemon --features sandbox-real-runtime sandbox_idle` on a machine with a working Docker daemon (quickstart Part A, real-runtime section)
- [ ] T063 [P] Extend `crates/micold-core/tests/quickstart_a_runs_everywhere.rs` so quickstart Part A's commands stay covered by the existing gate
- [ ] T064 Sweep `crates/micold-daemon/tests/sandbox_real_staleness.rs`, `sandbox_real_limits.rs`, `sandbox_real_parity.rs`, `sandbox_real_boundary.rs`, `sandbox_real_fingerprint.rs` and `crates/micold-daemon/tests/sandbox_real_support/mod.rs` for `survive_logout` uses whose meaning the amendment changes
- [ ] T065 Verify the pinned client/daemon pair still connects after the change — a mixed pair from `target-shared` refuses even with matching version numbers printed
- [ ] T066 Execute quickstart Part B (B1–B8) and record each with date, machine and outcome in `specs/028-client-managed-daemon/evidence/quickstart-b.md`
- [ ] T067 Reconcile the spec artifacts with what was built — if any task forced a decision the design documents do not carry, amend `research.md`, `data-model.md` or `contracts/` rather than leaving the record wrong

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: no dependencies.
- **Phase 2 (Foundational)**: needs T002/T003. Blocks US3, US4, US5. **Does not block US1.**
- **US1 (Phase 3)**: needs only Phase 1. Deliverable on its own, and the MVP.
- **US2 (Phase 4)**: needs T012 (presence fed from the accept loop).
- **US3 (Phase 5)**: needs Phase 2 complete and US2's honest count.
- **US4 (Phase 6)**: needs US3 — it is the same unwind, observed from outside the container.
- **US5 (Phase 7)**: needs T042 (the unwind exists to write a line at the start of).
- **Phase 8**: everything.

### User Story Dependencies

```text
US1 ──────────────────────────────► (independent; MVP)
Phase 2 ──► US2 ──► US3 ──► US4
                     └────► US5
```

US1 is genuinely independent: it removes the system-service path and touches no code the idle rule
uses. US3 through US5 are a chain because each observes the previous one's shutdown.

### Within Each User Story

Tests before implementation, always. Within the tests and within the implementation, `[P]` tasks
touch different files and may run together.

### Parallel Opportunities

- Phase 1: T002, T003 together.
- Phase 2 tests: T004, T005, T006 together (three files); T007 after, since it depends on the shape T009–T011 land.
- US1 tests: T013, T014, T015, T016 — four different files, all together.
- US1 implementation: T019, T021, T027, T028 are independent of the daemon/client edits.
- US3 tests: T034–T039 — six independent test files.
- US4 tests: T045–T049 together; T047 and T048 share a file, so write them in one pass.
- Docs: T027, T044, T053, T058, T059 are five different files and never conflict.

---

## Parallel Example: User Story 1

```text
# All four failing tests at once — four separate files:
T013  crates/micold-daemon/tests/no_socket_activation.rs
T014  crates/micold-client/tests/deb_ships_no_service_units.rs
T015  crates/micold-client/src/shell/legacy_units.rs  (#[cfg(test)])
T016  crates/micold-client/tests/no_host_logout_survival.rs

# Then the independent deletions:
T019  packaging/micold-daemon.{service,socket}
T021  crates/micold-core/src/logout_survival.rs
T027  docs/daemon.md
```

---

## Implementation Strategy

### MVP First (User Story 1 only)

US1 alone delivers the headline: the session service stops being a system service. It ships without
the idle rule, without touching the sandbox, and without the clock. If the feature had to stop after
one phase, this is the phase.

### Incremental Delivery

1. **Phase 1 + US1** — nothing is registered any more. Shippable.
2. **Phase 2 + US2** — the count the rule reads is correct. No user-visible change yet.
3. **US3** — the idle stop works on the host. The feature's second half arrives.
4. **US4** — the sandbox matches, with the approved exception.
5. **US5** — the stop is visible and documented.
6. **Phase 8** — gates, real-runtime suite, and the hand-recorded Part B.

### Parallel Team Strategy

US1 and Phase 2 have no file in common and can proceed simultaneously from the start — one person
removing the system-service path while another builds the clock, the counter and the rule. They meet
at US3.

---

## Notes

- The 30-minute constant is asserted by a unit test (T006); every integration test uses T041's short
  window. A test that waited out the real window would be a test nobody runs.
- `may_exit` had no call sites before this feature (research R1), so T007 is rewriting an assertion
  about behaviour that never ran — not changing behaviour twice.
- T026's ordering is the one migration hazard research R7 names explicitly: the un-enable must happen
  before the client connects, or socket activation starts a daemon from a unit file dpkg just removed.
- T052 is the task FR-022a rests on. Without it the setting can be turned off and the container keeps
  its old restart policy and environment, and the amendment's promise quietly fails.
