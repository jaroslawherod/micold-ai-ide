---
description: "Task list for running the session daemon on a remote host over SSH"
---

# Tasks: Run the session daemon on a remote host over SSH

**Input**: `specs/489-ssh-remote-daemon/` (spec.md, plan.md, research.md, data-model.md, contracts/, quickstart.md)

**Tests**: mandatory (Constitution I). Within each phase the test tasks come first and must fail before the
implementation tasks they cover. Pure rules are render-free tests in `micold-core`; `ssh_real_*` run against a
rootless `sshd` and are off by default (`mise run test-ssh`).

**Documentation**: each story's user-guide task sits in that story's milestone (Constitution VII).

**Format**: `[ID] [P?] [Story] Description`. Paths are repo-relative. `core` = `crates/micold-core/src`,
`client` = `crates/micold-client/src`, `daemon` = `crates/micold-daemon`.

## Phase 1: Setup

- [ ] T001 Enable the `process` feature of the workspace `tokio` dependency in `Cargo.toml` (and the crates that use it)
- [ ] T002 Add the off-by-default `ssh-real-runtime` feature to `daemon/Cargo.toml`, forward it to `core/Cargo.toml` (read by `crates/micold-core/build.rs`, T068), and a `test-ssh` task to `mise.toml` (builds the daemon, runs `ssh_real_*` with `--test-threads=1`)
- [ ] T003 Create the `ssh` module skeleton `core/ssh/mod.rs` and export it from `core/lib.rs`
- [ ] T004 [P] Add the rootless-`sshd` harness `daemon/tests/ssh_real_support/{mod,sshd}.rs`: host and client keys, `authorized_keys`, loopback port, a `Host` alias in a private ssh config, skip with a printed reason when `sshd` is missing

## Phase 2: Foundational

**Tests first**

- [ ] T005 [P] Test `SshTarget` validation (host not blank, no whitespace, not starting `-`, user without `@`, port 1..=65535, `InvalidField{field}`) in `core/ssh/target.rs`
- [ ] T006 [P] Test the argv builders in `core/ssh/argv.rs`: `BatchMode=yes`, `StrictHostKeyChecking=yes`, `ClearAllForwardings=yes`, `-T`, `--` before host, `LC_ALL=C`, no `-L/-R/-D`, no `-F`, no `accept-new`/`no`, no secret, per client platform
- [ ] T007 [P] Test `SshFailure` classification fixtures and `is_permanent` for every class and reason string of contracts/ssh-transport.md in `core/ssh/classify.rs`
- [ ] T008 [P] Test `DaemonRuntime::Ssh` serde round trip, `label()`, `is_startable()`, additive load of an old `settings.json` with no `settings_version` bump in `core/daemons.rs` and `core/settings.rs`
- [ ] T009 [P] Test the provision sequence (probe, copy only when absent, start, reuse a running daemon, permanent failures stop) over a stand-in `SshRunner`, and the artefact-selection table over (client, remote) pairs in `core/ssh/provision.rs`
- [ ] T010 [P] Test `remote bridge` (relays bytes to the daemon's Unix socket, starts the daemon when none runs, reuses a running one) and `remote prune` against a temp endpoint in `daemon/src/remote.rs`
- [ ] T011 Test the client actor with a stand-in transport: `Dial::Ssh` goes `Starting` to `Connected` with version and endpoint, a permanent failure goes `Unreachable{reason}` with no retry, in `client/daemon.rs`

**Implementation**

- [ ] T012 [P] Implement `SshTarget` and `SshSettings` in `core/ssh/target.rs`
- [ ] T013 [P] Implement the pure argv builders in `core/ssh/argv.rs`
- [ ] T014 [P] Implement `SshFailure` classification in `core/ssh/classify.rs`
- [ ] T015 Add `DaemonRuntime::Ssh(SshSettings)` to `core/daemons.rs` and its persistence in `core/settings.rs`
- [ ] T016 Implement `SshRunner`, the probe, copy and start sequence and artefact selection in `core/ssh/provision.rs`
- [ ] T017 Implement `micold-daemon remote bridge | prune` in `daemon/src/remote.rs` and dispatch it from `daemon/src/main.rs`
- [ ] T018 Add `Transport::Ssh(SshStdio)` in `core/connect.rs`, `DialAddress::Ssh` in `core/endpoint.rs`, and `RemotePlacement` carrying the target in `core/sandbox/placement.rs`
- [ ] T019 Add `Dial::Ssh` to the actor in `client/daemon.rs` and the one real `SystemSsh` runner (Windows `ssh.exe` lookup and hint, `CREATE_NO_WINDOW`) chosen only in `client/shell/daemons.rs`, with a 15 s watchdog deadline so no connect hangs (the `ConnectTimeout` value replaces it in T063); keep `no_concrete_implementations.rs` green

## Phase 3: User Story 1 - Work on a remote project end to end (P1) MVP

**Goal**: add a remote daemon, start it, see it connected, then work on a remote repository.
**Independent test**: against a reachable SSH host with a git repository, add, start, create a worktree, type in a terminal.

### Scenario 1: add and start (M1)

- [ ] T020 [P] [US1] Test the Daemons form reducers for an SSH runtime (draft, add, start, `Starting` with copy progress, `Connected` with version and `user@host:port`; `auto_start` applies to an `Ssh` entry, FR-020) in `client/features/daemons.rs`
- [ ] T021 [P] [US1] Add the geometry-gate case for the SSH fields and state badges of the Daemons form in `client/ui/settings/daemons.rs`
- [ ] T022 [US1] Write `daemon/tests/ssh_real_endtoend.rs` part 1: add the stand-in host, start, copy build, `Connected` with version and endpoint, no new listener
- [ ] T023 [US1] Add SSH runtime fields (host, user, port, key file) and messages to `Draft` and `Msg` in `client/features/daemons.rs`
- [ ] T024 [US1] Render the "Remote over SSH" runtime option and its host, user, port and key file fields in `client/ui/settings/daemons.rs`, reusing the shared text-input and badge builders (Principle VIII)
- [ ] T025 [US1] Run provisioning from the client and publish copy progress as daemon state in `client/shell/daemons.rs`
- [ ] T026 [US1] Add CI job `ssh-remote` running `mise run test-ssh` in `.github/workflows/ci.yml`, and document it in `docs/development/ci-pipeline.md`
- [ ] T027 [US1] Write `docs/user-guide/remote-daemons.md` (add, start, states), link it from `docs/SUMMARY.md`, `docs/user-guide/settings.md` and `docs/user-guide/sandboxed-daemon.md`, and describe the helper in `docs/daemon.md`

### Scenarios 2-4: remote projects, worktrees, sessions (M2)

- [ ] T028 [P] [US1] Test `RemoteLocation` and the namespaced identity path round trip, including Windows separators, in `core/ssh/remote_path.rs`
- [ ] T029 [P] [US1] Test that `canonicalize_best_effort` returns a `micold-remote://` identity verbatim in `core/project.rs`
- [ ] T030 [P] [US1] Test add-repository binding to its one daemon, never guessed, and boundary path translation in `client/features/daemons.rs` and `client/shell/daemon_sync.rs`
- [ ] T031 [P] [US1] Test that a remote worktree shows the host path and the daemon name in row labels in `client/links.rs` and `client/features/sidebar.rs`
- [ ] T032 [US1] Extend `daemon/tests/ssh_real_endtoend.rs`: add repository, list and create a worktree, session with the fake AI CLI, input and output, terminal command
- [ ] T033 [US1] Implement `RemoteLocation` and the identity mapping in `core/ssh/remote_path.rs`
- [ ] T034 [US1] Keep a remote identity verbatim in `canonicalize_best_effort` in `core/project.rs`
- [ ] T035 [US1] Translate paths at the daemon boundary in `client/shell/daemon_sync.rs` and pass the open-project gate over the wire in `client/shell/workspace.rs`
- [ ] T036 [US1] Add the repository path step to `client/features/daemons.rs` and show host name and remote path in `client/links.rs` and `client/features/sidebar.rs`
- [ ] T037 [US1] Add the repository field to `client/ui/settings/daemons.rs`
- [ ] T038 [US1] Document remote repositories and worktrees in `docs/user-guide/remote-daemons.md` and `docs/user-guide/worktrees-and-sessions.md`

## Phase 4: User Story 2 - A network drop loses nothing (P1)

**Goal**: sessions survive a drop and the client reconnects by itself.
**Independent test**: break the SSH connection with a running session, restore it, see the session and its output.

- [ ] T039 [P] [US2] Test `Backoff` (1, 2, 4, 8, 15, 15 s; reset) in `core/ssh/reconnect.rs`
- [ ] T040 [P] [US2] Test the state-machine delta (`Reconnecting`, `Attempts`, `GiveUp`, `Reconnect`, `Unreachable` meaning, local runtimes unchanged) in `core/daemons.rs`
- [ ] T041 [P] [US2] Test the actor loop with a stand-in transport: drop, backoff, explicit reconnect bumps the epoch, no second connect, in `client/daemon.rs`
- [ ] T042 [P] [US2] Test `session_lost_label`: `disconnected` while reconnecting, `lost to daemon <name>` after a restart (strings per contracts/remote-daemon.md; spec FR-019 term `lost to its daemon`), in `client/links.rs`
- [ ] T043 [US2] Add the fault proxy `daemon/tests/ssh_real_support/proxy.rs` (cut and restore the connection)
- [ ] T044 [US2] Write `daemon/tests/ssh_real_reconnect.rs`: drop, sessions keep running, reconnect within 30 s with output intact, relaunch finds sessions, stop trying then Reconnect, no second daemon
- [ ] T045 [US2] Implement `Backoff` in `core/ssh/reconnect.rs`
- [ ] T046 [US2] Add `DaemonState::Reconnecting`, `Attempts`, `GiveUp` and `Reconnect` to `core/daemons.rs`
- [ ] T047 [US2] Implement the reconnect loop, per-actor backoff, epoch and give-up in `client/daemon.rs`
- [ ] T048 [US2] Implement the session labels in `client/links.rs`
- [ ] T049 [US2] Show the `Reconnecting` badge with reason and the stop-trying and Reconnect actions in `client/ui/settings/daemons.rs`
- [ ] T050 [US2] Document reconnection in `docs/user-guide/remote-daemons.md`

## Phase 5: User Story 4 - Local work is unaffected (P1)

**Goal**: no remote state or failure touches a local daemon or its sessions.
**Independent test**: with a connected local daemon, make the remote unreachable and keep typing locally.

- [ ] T051 [P] [US4] Test a remote daemon in every non-connected state beside a connected local one: local `DaemonView` and outbox untouched, in `client/daemon.rs`
- [ ] T052 [P] [US4] Test that app start never awaits a remote dial (`DaemonStates::at_launch`) in `core/daemons.rs` and `client/shell/daemons.rs`
- [ ] T053 [US4] Write `daemon/tests/ssh_real_concurrent.rs`: two bridges on one daemon see the same sessions, input reaches one session, one client leaving leaves the other; local session unaffected while the remote is cut
- [ ] T054 [US4] Fix any cross-daemon coupling the tests find and spawn every remote dial asynchronously at launch in `client/daemon.rs` and `client/shell/daemons.rs`
- [ ] T055 [US4] Document the isolation guarantee in `docs/user-guide/remote-daemons.md`

## Phase 6: User Story 3 - See and manage the connection (P2)

**Goal**: choose a host, see failure reasons, manage the daemon.
**Independent test**: add a daemon, cause each failure, read the named reason within the timeout.

- [ ] T056 [P] [US3] Test the host list from `~/.ssh/config` and the `ssh -G` output parser in `core/ssh/config_hosts.rs`
- [ ] T057 [P] [US3] Test `SshDuplicate{other}` over (resolved host, user, port) and `InvalidField` naming the field in `core/daemons.rs`
- [ ] T058 [P] [US3] Test picker, entered fields, recent errors, edit, remove with confirmation in `client/features/daemons.rs`
- [ ] T059 [P] [US3] Add geometry-gate cases for the picker and error rows in `client/ui/settings/daemons.rs`
- [ ] T060 [US3] Write `daemon/tests/ssh_real_hostkey_auth.rs`: unknown and changed host key, auth refused, missing key, unreachable host within the timeout, no hang, no key recorded
- [ ] T061 [US3] Implement `config_hosts.rs` in `core/ssh/config_hosts.rs`
- [ ] T062 [US3] Add the duplicate and field rules to `DaemonRegistry::add` and `edit` in `core/daemons.rs`
- [ ] T063 [US3] Resolve with `ssh -G` and run the `ConnectTimeout` watchdog in `client/shell/daemons.rs`
- [ ] T064 [US3] Build the host picker as a shared widget in `client/ui/material/` (builder with `.into()`, Principle VIII), then add it, the errors list and the edit and remove flows to `client/features/daemons.rs` and `client/ui/settings/daemons.rs`
- [ ] T065 [US3] Document hosts, failure reasons and management in `docs/user-guide/remote-daemons.md` and `docs/user-guide/settings.md`

## Phase 7: User Story 5 - Version mismatch is explained (P2)

**Goal**: both versions and the way out, never ending sessions.

### Scenarios 1 and 3: explain, refuse sessions (M6)

- [ ] T066 [P] [US5] Test `MismatchExplanation` for both directions and the `WayOut` function in `core/ssh/mismatch.rs`
- [ ] T067 [P] [US5] Test that no session starts on a mismatched daemon and the state text names both versions, in `client/features/daemons.rs`
- [ ] T068 [US5] Add the stale-daemon fixture `daemon/tests/ssh_real_support/stale_daemon.rs` and the `MICOLD_FIXTURE_VERSION_SUFFIX` read in `crates/micold-core/build.rs` (honoured only under `ssh-real-runtime`, `rerun-if-env-changed`), with a test that a default build ignores the variable
- [ ] T069 [US5] Write `daemon/tests/ssh_real_mismatch.rs` part 1: both directions, explanation text, session start refused, running sessions untouched
- [ ] T070 [US5] Implement `MismatchExplanation`, `Side`, `WayOut` and `ReplaceOffer` (M6 always `Unknown`) in `core/ssh/mismatch.rs`
- [ ] T071 [US5] Show the explanation and refuse session start in `client/features/daemons.rs` and `client/ui/settings/daemons.rs`
- [ ] T072 [US5] Document version mismatch in `docs/user-guide/remote-daemons.md`

### Scenario 2: follow the way out, replace (M7)

- [ ] T073 [P] [US5] Test `RemoteStatus` parsing (unknown fields ignored, unknown `status_version` is `Unknown`) in `core/ssh/status.rs`
- [ ] T074 [P] [US5] Test the status writer and `remote status | stop` (stop refuses unless zero sessions proven) in `daemon/src/runtime_status.rs` and `daemon/src/remote.rs`
- [ ] T075 [P] [US5] Test `ReplaceOffer` states driving the Replace action in `client/features/daemons.rs`
- [ ] T076 [US5] Extend `daemon/tests/ssh_real_mismatch.rs`: replace offered with no sessions and then connects, withheld with sessions
- [ ] T077 [US5] Implement `RemoteStatus` in `core/ssh/status.rs`
- [ ] T078 [US5] Write `status.json` from the daemon in `daemon/src/runtime_status.rs`
- [ ] T079 [US5] Implement `remote status | stop` in `daemon/src/remote.rs`
- [ ] T080 [US5] Feed `ReplaceOffer::{Available, Blocked}` from `RemoteStatus` and wire the Replace action in `client/shell/daemons.rs`, `client/features/daemons.rs` and `client/ui/settings/daemons.rs`
- [ ] T081 [US5] Document replacing a daemon in `docs/user-guide/remote-daemons.md`

## Phase 8: Polish and cross-cutting

- [ ] T082 Test that the macOS bundle (`scripts/macos-bundle.sh`) and Windows installer (`scripts/windows-installer.sh`) ship the Linux remote-daemon artefact in `scripts/tests/remote-daemon-bundle.test.sh`
- [ ] T083 Build the Linux (musl) remote-daemon artefact with `scripts/remote-daemon-bundle.sh` and include it from `scripts/macos-bundle.sh`, `scripts/windows-installer.sh`, `.github/workflows/ci.yml` and `.github/workflows/release.yml`
- [ ] T084 Document per-platform requirements (`ssh.exe`, agent service, supported remote OS) in `docs/user-guide/remote-daemons.md`

## Dependencies and order

Setup, then Foundational, then stories in listed order (US1, US2, US4, US3, US5), then Polish. Tests precede
the implementation they cover within each group. `[P]` tasks touch different files.

## Implementation Strategy

MVP is Setup, Foundational and US1 scenario 1: a user adds a remote daemon and sees it connected, proven by
a real `sshd`. Each later milestone adds one observable behaviour and merges on its own. Quickstart Part B
(visual pass, which also checks SC-001's 3 minutes) and task ticks are left to the close unit.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Remote over SSH daemon connects 🎯 MVP

- **Tasks**: T001–T027
- **Deliverable**: In Settings > Daemons a user adds a "Remote over SSH" daemon, starts it (the client copies its build and starts the remote daemon over `ssh`), and sees it `Connected` with version and `user@host:port`.
- **Satisfies**: US1 acceptance scenario 1; FR-001, FR-002 (typed host), FR-003, FR-004, FR-005, FR-009 (default 15 s deadline), FR-010 (argv rule), FR-017 (connect), FR-018 (rules)
- **Verify**: `mise run test-core`, `mise run test-ssh` (`ssh_real_endtoend` part 1); quickstart A.1–A.3
- **Depends on**: —
- **Tier**: full

### M2 — Remote projects, worktrees and sessions

- **Tasks**: T028–T038
- **Deliverable**: A repository on the remote host can be added, its worktrees listed and created, and a session and terminal run in them, shown with the remote path and the daemon's name.
- **Satisfies**: US1 acceptance scenarios 2–4; FR-012, FR-015
- **Verify**: `mise run test-ssh` (`ssh_real_endtoend`); `mise run test-core` (identity round trip)
- **Depends on**: M1
- **Tier**: full

### M3 — Reconnect after a network drop

- **Tasks**: T039–T050
- **Deliverable**: A dropped remote daemon shows `Reconnecting`, its sessions show `disconnected`, and the client reconnects automatically with sessions and output intact; the user can stop trying and reconnect.
- **Satisfies**: US2 acceptance scenarios 1–4; FR-006, FR-007, FR-008, FR-019
- **Verify**: `mise run test-ssh` (`ssh_real_reconnect`); `mise run test-core` (backoff, state machine)
- **Depends on**: M2
- **Tier**: full

### M4 — Local work unaffected, concurrent clients

- **Tasks**: T051–T055
- **Deliverable**: A remote daemon in any failure state, or a cut connection, never changes a local daemon or its sessions, app start never waits on a remote, and several clients share one remote daemon.
- **Satisfies**: US4 acceptance scenarios 1–2; FR-013, FR-020, FR-021
- **Verify**: `mise run test-ssh` (`ssh_real_concurrent`); `mise run test` (isolation tests)
- **Depends on**: M3
- **Tier**: full

### M5 — Host picker, failure reasons, daemon management

- **Tasks**: T056–T065
- **Deliverable**: A user picks a host from their SSH config or enters host, user, port and key; every SSH failure shows a named reason within the timeout; duplicates and invalid fields are refused; edit, remove and recent errors work.
- **Satisfies**: US3 acceptance scenarios 1–4; FR-002, FR-009, FR-010, FR-014, FR-016, SC-003
- **Verify**: `mise run test-ssh` (`ssh_real_hostkey_auth`); `mise run test-core` (registry, config hosts)
- **Depends on**: M3
- **Tier**: full

### M6 — Version mismatch is explained

- **Tasks**: T066–T072
- **Deliverable**: A client and remote daemon of different versions, either side older, show `version mismatch` naming both versions and the way out, and no session starts on it.
- **Satisfies**: US5 acceptance scenarios 1 and 3; FR-011 (explanation), SC-005
- **Verify**: `mise run test-ssh` (`ssh_real_mismatch` part 1); `mise run test-core` (mismatch rules)
- **Depends on**: M3
- **Tier**: full

### M7 — Replace a mismatched daemon with no running sessions

- **Tasks**: T073–T081
- **Deliverable**: When the mismatched remote daemon has no running sessions the user can replace it from the app; with sessions the app withholds replacement and gives the instructions.
- **Satisfies**: US5 acceptance scenario 2; FR-011 (no end, offer)
- **Verify**: `mise run test-ssh` (`ssh_real_mismatch` replace cases)
- **Depends on**: M6
- **Tier**: full

### M8 — Remote daemon artefact for macOS and Windows clients

- **Tasks**: T082–T084
- **Deliverable**: macOS and Windows client packages ship the Linux remote-daemon artefact, so those clients can provision a Linux remote host; per-platform requirements are documented.
- **Satisfies**: FR-018, SC-001 on all three clients
- **Verify**: `scripts/tests/remote-daemon-bundle.test.sh` (via `mise run test-scripts`)
- **Depends on**: M1
- **Tier**: full
