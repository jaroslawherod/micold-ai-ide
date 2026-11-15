---
description: "Task list for feature 1725: daemons dial the orchestrator"
---

# Tasks: Daemons dial the orchestrator

**Input**: `specs/1725-daemons-dial-orchestrator/` (spec.md, plan.md, research.md, data-model.md, contracts/dial.md, quickstart.md)

**Tests**: mandatory (Constitution I). Every implementation task is preceded by the failing test task that covers it; run the test and watch it fail first.

**Documentation**: each milestone updates `docs/user-guide/daemon-configuration.md` and/or `docs/daemon.md` in the same change (Constitution VII; CI's user-guide gate).

**Cross-platform**: the dialer connects through `micold_core::connect` helpers (no `cfg(unix)` outside `platform/`); the forward is Unix-remote only. Tests run on Linux.

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Setup

- [ ] T001 Scaffold the new test binary `crates/micold-daemon/tests/orchestrator_dial/main.rs` with a helper that starts the real `orchestrator::run_with` on a temp socket and a daemon with a given `[orchestrator_link]` config; first `grep -n` `crates/micold-daemon/tests/orchestrator/main.rs` and reuse or extract its helpers instead of copying them; add `[[test]] name = "orchestrator_dial" path = "tests/orchestrator_dial/main.rs"` to `crates/micold-daemon/Cargo.toml` (`autotests = false`); each later test task adds its own `mod` to that `main.rs`

## Phase 2: Foundational (blocks every story)

- [ ] T002 [P] Write failing unit tests for the `[orchestrator_link]` table in `crates/micold-core/src/daemon_config/tests.rs`: defaults (`enabled = false`, no `socket`), unknown key rejected, wrong type rejected, rule L1 (`orchestrator_link.socket` is empty), independence from O1-O5 (FR-013)
- [ ] T003 [P] Write a failing round-trip test (`encode_announcement` then `decode_first_frame`) in `crates/micold-core/src/orchestrator/announce.rs`
- [ ] T004 [P] Write failing unit tests in `crates/micold-core/src/endpoint.rs` for `orchestrator_endpoint()` and the effective dial endpoint: `socket` if set, else `orchestrator_endpoint()` for kind `host`, else `orchestrator.sock` beside `resolve_pinned()`'s daemon socket for `ssh`, `container`, `kubernetes` (data-model.md)
- [ ] T005 [P] Write failing unit tests in `crates/micold-core/src/orchestrator/dial.rs` (no clock): `RetryPolicy` delays 1, 2, 4, 8, 15, 15 s; `after_refusal` is always `MAX_DELAY` and does not grow the connect backoff; `on_linked` resets; `after_loss` gives 1 s then 2 s; `jittered(delay, unit)` stays in `1.0..=1.2` and never applies to the capped connect delay; `LinkStatus` variants (FR-006, FR-007)
- [ ] T006 [P] Write failing `config check` tests in `crates/micold-daemon/tests/config_check.rs`: an empty `socket` is reported as L1, an unknown key under `[orchestrator_link]` is reported, a valid table passes (FR-013)
- [ ] T007 Add `OrchestratorLink` (`#[serde(deny_unknown_fields)]`, `enabled: bool` default false, `socket: Option<PathBuf>`) and `DaemonConfig.orchestrator_link` in `crates/micold-core/src/daemon_config/mod.rs`, and rule L1 `orchestrator_link()` in `crates/micold-core/src/daemon_config/validate.rs`
- [ ] T008 Implement `encode_announcement` in `crates/micold-core/src/orchestrator/announce.rs`
- [ ] T009 Move `orchestrator_endpoint()` from `crates/micold-daemon/src/orchestrator/mod.rs` into `crates/micold-core/src/endpoint.rs` (the daemon's `endpoint()` delegates) and add the effective-endpoint function for the four kinds
- [ ] T010 Implement `LinkStatus`, `RetryPolicy` (wrapping `ssh::reconnect::Backoff`) and `jittered` in the new `crates/micold-core/src/orchestrator/dial.rs`; export it from `crates/micold-core/src/orchestrator/mod.rs`

**Checkpoint**: core rules green; no daemon behaviour yet.

## Phase 3: User Story 1 - The host daemon links to the local orchestrator (P1) MVP

**Goal**: a host daemon with `[orchestrator_link] enabled = true` dials the local orchestrator and shows `online`.

**Independent Test**: orchestrator on a temp socket plus a configured daemon; the registry lists the daemon `online` with every announced field, and a client is served throughout.

- [ ] T011 [US1] Write failing integration tests in `crates/micold-daemon/tests/orchestrator_dial/host.rs`: daemon `online` with id, kind `host`, name, protocol version, capabilities, tenant (SC-001, scenario 1); daemon started before the orchestrator links without restart (scenario 2); `enabled = false` spawns no task, opens no socket and logs nothing at start (scenario 3, FR-010, SC-005); a client is accepted as fast as before while the orchestrator is absent or slow (scenario 4, FR-001); on a paused clock, advance past `LIVENESS_DEADLINE` plus one check interval and assert the daemon is still `online` with one pong per ping and exactly one link (FR-002); a fake orchestrator that closes after half a verdict frame is a dropped link that is retried, no panic (edge case); a daemon started with `socket = ""` still starts and does not dial (edge case)
- [ ] T012 [US1] Implement one dial attempt in `crates/micold-daemon/src/orchestrator_link/dial.rs`, generic over `AsyncRead + AsyncWrite + Unpin + Send`: write the `LinkAnnouncement` and read the `LinkVerdict` within `LIVENESS_DEADLINE`, switch with `map_codec` to `ClientCodec`, send `Hello` as `connect::handshake_with` does (no token), expect `Welcome`; every error is an `io::Error`, never a panic
- [ ] T013 [US1] Implement the link task in `crates/micold-daemon/src/orchestrator_link/mod.rs` (`spawn(config, state) -> handle`): connect through `connect::dial_address(DialAddress::Local)`, treat an absent or closed endpoint as `NotLinked`, steady loop answering `LinkPing` with `LinkPong` under a `LIVENESS_DEADLINE` read timeout that any frame resets, re-dial with `RetryPolicy`; hold the link status in a watch cell and log only on a status change (including `orchestrator link lost` once per loss); return before spawning anything when `enabled` is false or the table is absent
- [ ] T014 [US1] Call `orchestrator_link::spawn` in `crates/micold-daemon/src/server.rs` after the daemon listener is bound, without awaiting it; declare the module in `crates/micold-daemon/src/lib.rs` (or `main.rs`, wherever `orchestrator` is declared)
- [ ] T015 [US1] Document `[orchestrator_link]` (`enabled`, `socket`, rule L1) in `docs/user-guide/daemon-configuration.md` and "Linking to an orchestrator" (what is dialled, host case) in `docs/daemon.md`

**Checkpoint**: Quickstart A steps 1-3 and 5 pass.

## Phase 4: User Story 4 - Sessions survive an orchestrator outage and the link returns (P1)

**Goal**: an outage of the orchestrator changes nothing for sessions or clients; the link returns by itself; a stalled dialer cannot hold an id reservation.

**Independent Test**: a daemon with a producing session and a client; stop the orchestrator, start it, the daemon is `online` again within 16 s.

- [ ] T016 [US4] Write failing integration tests in `crates/micold-daemon/tests/orchestrator_dial/outage.rs`: with a session producing output and a connected client, stop the orchestrator for 30 s (paused clock) and assert session and client are unchanged (scenario 1, SC-003); `online` within 16 s of its return (scenario 2); an orchestrator restart while linked makes the daemon re-dial and be accepted as the restored record, with no `AlreadyLinked` refusal and no duplicate (scenario 3)
- [ ] T017 [US4] Write a failing integration test in `crates/micold-daemon/tests/orchestrator_dial/stalled.rs` for FR-011: a dialer that stalls (never reads) at each of the four bounded sends (`refuse`, `Accepted`, `Welcome`, `Refused`), one case each; the orchestrator link ends within `LIVENESS_DEADLINE` and the id reservation is released (`claim_failed`)
- [ ] T018 [US4] Add `send_bounded(framed, item)` (`timeout(LIVENESS_DEADLINE, framed.send(item))` mapped to `io::ErrorKind::TimedOut`) in `crates/micold-daemon/src/orchestrator/link.rs` and use it for `refuse`, `Accepted`, `Welcome` and `Refused`
- [ ] T019 [US4] Make T016 pass in `crates/micold-daemon/src/orchestrator_link/mod.rs`: the task reads only `DaemonConfig` and its own status cell, takes no `DaemonState` lock a session waits on, and keeps the log-on-change of T013 (FR-008)
- [ ] T020 [US4] Document the outage behaviour (sessions keep running, the link returns by itself, the 16 s bound) in `docs/user-guide/daemon-configuration.md` and `docs/daemon.md`

## Phase 5: User Story 5 - A refusal or a crowd does not turn the daemon into a hot loop (P2)

**Goal**: refused links are retried at the capped delay and the reason is observable.

**Independent Test**: an orchestrator that refuses every attempt; at most one attempt per capped delay over 60 s, and `micold-daemon remote status` names the reason.

- [ ] T021 [P] [US5] Write failing unit tests in `crates/micold-core/src/ssh/status.rs`: `RemoteStatus` with `link: Option<LinkStatusReport>` round-trips as `{ "state", "since", "detail" }`; a `status.json` without the field still parses; `STATUS_VERSION` is unchanged and a `None` link serializes to no field (FR-012)
- [ ] T022 [US5] Write failing integration tests in `crates/micold-daemon/tests/orchestrator_dial/refused.rs` with a fake orchestrator refusing `VersionMismatch`, `AlreadyLinked` and `Hello` in turn over a paused clock: at most one attempt per capped delay once the backoff has grown, 60 s run (scenarios 1-2, SC-004); against the real `orchestrator::run_with`, a dialer announcing a different protocol version is listed `needs-update` in the registry while it retries at the capped delay (scenario 1); the first re-dial after a lost link comes after 1 s (scenario 3); one log line per status change, none per retry
- [ ] T023 [US5] Write a failing integration test in `crates/micold-daemon/tests/orchestrator_dial/status.rs`: `status.json` carries the `link` object and `micold-daemon remote status` prints the state and the refusal reason (FR-012)
- [ ] T024 [US5] Add `LinkStatusReport` and `RemoteStatus.link` (`#[serde(default, skip_serializing_if = "Option::is_none")]`) in `crates/micold-core/src/ssh/status.rs`
- [ ] T025 [US5] Handle refusals in `crates/micold-daemon/src/orchestrator_link/mod.rs`: `Refused(reason)` and `HelloRefused` use `RetryPolicy::after_refusal` with jitter, the connect backoff does not grow, the refusal reason and message enter the T013 status cell
- [ ] T026 [US5] Write the link status into `status.json` in `crates/micold-daemon/src/runtime_status.rs` and print it from `micold-daemon remote status` in `crates/micold-daemon/src/remote.rs`
- [ ] T027 [US5] Document the retry delays, the refusal reasons and `remote status` in `docs/user-guide/daemon-configuration.md` and `docs/daemon.md`

## Phase 6: User Story 2 - Container, SSH-host and pod daemons link through a reverse forward (P1)

**Goal**: the holder of the SSH connection opens one reverse forward carrying the orchestrator link only; the remote daemon dials it.

**Independent Test**: four daemons (host, container, ssh, pod) behind rootless `sshd`; each `online` once with its own kind; a probe from the remote side to any other client address fails.

- [ ] T028 [US2] Write failing unit tests in `crates/micold-core/src/ssh/argv.rs`: `LinkForward` rejects non-absolute paths, `:`, whitespace and control characters; `bridge_with_link` renders exactly one `-R <remote>:<local>` plus `-o ExitOnForwardFailure=yes` and no other forward; `Forward` is an enum (install relay, link) with no free-form `-R`; the path is quoted; `prepare_link` runs `remote prepare-link`; `ClearAllForwardings` is off only on the pinned reach and, for a user-config reach, only after `ssh -G` shows no configured forward (research R2); when `ssh -G` itself fails or reports a configured forward on a user-config reach, the open fails with a message naming the cause (FR-005), never silently without the forward
- [ ] T029 [P] [US2] Write a failing integration test in `crates/micold-daemon/tests/remote_prepare_link.rs` (add `mod remote_prepare_link;` to `crates/micold-daemon/tests/main.rs`, or `every_test_file_is_wired` fails) for `remote prepare-link`: creates and verifies the 0700 runtime directory, removes an existing `orchestrator.sock`, prints `<dir>/orchestrator.sock` (contracts/dial.md)
- [ ] T030 [P] [US2] Write a failing unit test in `crates/micold-core/src/ssh/provision.rs` (and `crates/micold-core/src/provision/mod.rs` if its tests live there): with `ProvisionOptions.link` set, `start_step` runs `prepare_link`, reads the printed path and builds the bridge with the forward; unset, the argv is unchanged
- [ ] T031 [US2] Write failing `ssh_real` tests in `crates/micold-daemon/tests/ssh_real/ssh_real_link.rs` (rootless `sshd`, off by default; add the `#[cfg(all(feature = "ssh-real-runtime", unix))] mod ssh_real_link;` line to `crates/micold-daemon/tests/ssh_real/main.rs`): four kinds each `online` once under their own id (scenarios 1 and 4, SC-001); the remote shows `orchestrator.sock` only and a probe to any other client address fails (scenario 2, SC-006); a taken remote endpoint fails the open with a message (scenario 3); `AllowStreamLocalForwarding no` fails with ssh's text (FR-005)
- [ ] T032 [US2] Implement `LinkForward`, the `Forward` enum, `bridge_with_link` and `prepare_link` in `crates/micold-core/src/ssh/argv.rs`, including the `ssh -G` check of research R2 and its fail-the-open outcome
- [ ] T033 [US2] Implement `remote prepare-link` in `crates/micold-daemon/src/remote.rs` (resolve `endpoint::resolve_pinned()`, create and verify the 0700 directory, unlink `orchestrator.sock`, print the path)
- [ ] T034 [US2] Add `ProvisionOptions.link: Option<LinkRequest>` in `crates/micold-core/src/ssh/provision.rs` and the prepare-then-bridge step in `crates/micold-core/src/provision/mod.rs` `start_step`
- [ ] T035 [US2] Set `link` at the three `ProvisionOptions` construction sites in `crates/micold-client/src/daemon.rs` (app side, no other change)
- [ ] T036 [P] [US2] Tighten `packaging/basic/sshd_config` (`AllowStreamLocalForwarding remote`, `StreamLocalBindUnlink yes`) and note the two lines in `packaging/basic/README.md`; check the image's `ListenAddress` against the pod's loopback sshd listener and record the result in research.md R1 (`mise run image` where a runtime exists, else the config read)
- [ ] T037 [US2] Document the forward, its sshd settings and the Unix-remote-only limit (feature 489) in `docs/user-guide/daemon-configuration.md` and `docs/daemon.md`

## Phase 7: User Story 3 - Dropping the SSH connection shows offline; reconnecting restores it (P1)

**Goal**: killing the app's SSH connection shows the daemon `offline` within 10 s; reopening shows `online` within 16 s with the same daemon pid.

**Independent Test**: `ssh_real` link test kills the bridge `ssh` and reopens it.

- [ ] T038 [US3] Write failing `ssh_real` tests in `crates/micold-daemon/tests/ssh_real/ssh_real_link.rs`: kill one bridge `ssh`, that daemon `offline` within 10 s and the others unchanged (SC-002, scenario 1); reopen, `online` within 16 s with the daemon pid unchanged (scenario 2); `kill -9` the app's `ssh` and reopen at once while the remote sshd session lives, `online` (stale socket, spec edge case); a dial while the forward is down (stale or absent `orchestrator.sock`) is a connect failure, not a refusal, so the delay is capped at 15 s, and it backs off quietly (scenario 3)
- [ ] T039 [US3] Make T038 pass in `crates/micold-daemon/src/remote.rs` and `crates/micold-daemon/src/orchestrator_link/mod.rs` (stale-socket removal in `prepare-link`, a refused connect is `NotLinked` with no log line per retry); change nothing T038 does not need
- [ ] T040 [US3] Document the offline and reconnect timings and the stale-forward troubleshooting in `docs/user-guide/daemon-configuration.md` and `docs/daemon.md`

## Dependencies & Execution Order

- Phase 1 then Phase 2 then Phase 3 (M1). Test tasks first within each phase, then the implementation they cover.
- M2 (US4) and M3 (US5) depend on M1 only. M4 (US2) depends on M1. M5 (US3) depends on M4.
- Parallel: T002-T006, T021, T029, T030, T036 touch disjoint files.

## Implementation Strategy

MVP first: M1 is the host link (the smallest end-to-end link). Then the outage and refusal hardening on the same task (M2, M3), then the forward (M4) and its drop and return behaviour (M5). The quickstart results and ticks are left to the close unit.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Host daemon links to the local orchestrator 🎯 MVP

- **Tasks**: T001–T015
- **Deliverable**: a host daemon with `[orchestrator_link] enabled = true` dials the local orchestrator and the registry lists it `online` with its announced fields; with the table absent nothing changes.
- **Satisfies**: US1 acceptance scenarios 1–4; FR-001, FR-002, FR-003, FR-006, FR-010, FR-013; SC-001 (host), SC-005
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator_dial host`; quickstart §A steps 1–3, 5
- **Depends on**: —
- **Tier**: full

### M2 — Sessions survive an orchestrator outage; handshake sends are bounded

- **Tasks**: T016–T020
- **Deliverable**: stopping the orchestrator changes nothing for sessions or clients, the daemon is `online` again within 16 s of its return, and a stalled dialer no longer holds a link task or id reservation.
- **Satisfies**: US4 acceptance scenarios 1–3; FR-008, FR-011; SC-003
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator_dial -- outage stalled`; quickstart §A step 4
- **Depends on**: M1
- **Tier**: full

### M3 — Refusals back off and the link status is observable

- **Tasks**: T021–T027
- **Deliverable**: an orchestrator that refuses every attempt gets at most one attempt per capped delay, and `micold-daemon remote status` shows the link state and refusal reason.
- **Satisfies**: US5 acceptance scenarios 1–3; FR-007, FR-012; SC-004
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator_dial -- refused status`; `scripts/build-lock.sh cargo test -p micold-core status`
- **Depends on**: M1
- **Tier**: full

### M4 — Container, SSH-host and pod daemons link through a reverse forward

- **Tasks**: T028–T037
- **Deliverable**: opening the app's SSH connection with the link request opens one reverse forward, and the container, SSH-host and pod daemons behind it are listed `online` with their own kind; the forward reaches nothing else.
- **Satisfies**: US2 acceptance scenarios 1–4; FR-004, FR-005; SC-001 (four kinds), SC-006
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --features ssh-real-runtime --test ssh_real ssh_real_link -- --test-threads=1` (rootless `sshd`; quickstart §B steps 1, 3, 5); unit tests `scripts/build-lock.sh cargo test -p micold-core argv`
- **Depends on**: M1
- **Tier**: full

### M5 — Dropping the SSH connection shows offline; reopening restores it

- **Tasks**: T038–T040
- **Deliverable**: killing the app's SSH connection shows the daemon `offline` within 10 s, and reopening it shows `online` within 16 s with the daemon pid unchanged, including after a `kill -9` and an immediate reopen.
- **Satisfies**: US3 acceptance scenarios 1–3; FR-009; SC-002
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --features ssh-real-runtime --test ssh_real ssh_real_link -- --test-threads=1` (quickstart §B steps 2, 4)
- **Depends on**: M4
- **Tier**: full
