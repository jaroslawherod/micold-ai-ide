# Tasks: The desktop app talks only to the orchestrator

**Input**: `specs/1726-app-via-orchestrator/` (plan.md, research.md, data-model.md, quickstart.md, contracts/)
**Tests**: mandatory (Constitution I); each test task is written failing before its implementation task.
**Format**: `[ID] [P?] [Story] Description` — `[P]` = different files, no dependency on an unfinished task.

## Phase 1: Setup

- [x] T001 Write the shared contract `specs/orchestrator/contracts/client-routing.md` (client `Hello`, routed envelope opaque form, `OrchList`/`OrchListed`, `MissingReason` wire text, `StreamEnded`, `OutcomeUnknown`, refusals) with one JSON example per frame, and point `specs/1726-app-via-orchestrator/contracts/README.md` at it (FR-011)

## Phase 2: Foundational (core, render-free)

- [x] T002 [P] Failing tests in `crates/micold-core/src/protocol/routing.rs`: opaque-form round trip, header-only decode, `req` mismatch is `Malformed`, contract examples from T001 decode (FR-011)
- [x] T003 Implement `RoutedHeader` and the opaque body form of `RoutedEnvelope` in `crates/micold-core/src/protocol/routing.rs` (R3) so T002 passes
- [x] T004a [P] Failing serde round-trip tests in `crates/micold-core/src/protocol/messages.rs` and `routing.rs` for `ClientMsg::OrchList`, `DaemonMsg::OrchListed`, `StreamEnded`, `OutcomeUnknown`, `ListKind`, `MissingDaemon`/`MissingReason`, `StreamEndReason`, `RouteRefusal`, each matching its T001 contract example
- [x] T004 Add those types in `crates/micold-core/src/protocol/messages.rs` and `routing.rs` so T004a passes; bump `PROTOCOL_VERSION` 82 -> 83 in `crates/micold-core/src/protocol/version.rs`
- [x] T005 [P] Failing unit tests for `RouteTable` (bijection, link conns never reused while open, `drop_link` returns every route and pending req) in a new `crates/micold-core/src/orchestrator/route.rs` (data-model)
- [x] T006 Implement `RouteTable` in `crates/micold-core/src/orchestrator/route.rs` so T005 passes
- [x] T007 [P] Failing unit tests for `merge_lists` (union, same path on two daemons stays two rows, a daemon in rows or missing never both, `no_daemons`) in a new `crates/micold-core/src/orchestrator/list_merge.rs` (FR-001)
- [x] T008 Implement `merge_lists` in `crates/micold-core/src/orchestrator/list_merge.rs` so T007 passes

## Phase 3: User Story 2 — every request and stream reaches the owning daemon (P1)

- [x] T009 [US2] Failing integration suite `crates/micold-daemon/tests/orchestrator_route/main.rs` with a harness spawning a real orchestrator and two real linked daemons; cases `requests` (FR-003, US2.1), `streams` incl. two-client takeover (FR-004, US2.2, US2.4), `refusals` (FR-005)
- [x] T010 [US2] Daemon side: `crates/micold-daemon/src/orchestrator_link/virtual_conn.rs` (duplex per link conn into `server::serve_connection`, R1); `dial.rs` `steady(framed, Arc<DaemonState>)` demuxes routed frames and stamps `req` on answers; `mod.rs` `spawn_for(Arc<DaemonState>)`
- [x] T011 [US2] Orchestrator side: new `crates/micold-daemon/src/orchestrator/client.rs` (client session on plain `Hello`, opaque relay via `RouteTable`, refusals `not_found`/`needs-update`/offline); `link.rs` `steady()` demux inbound frames to conns with one bounded queue per `(link, conn)`; replace `query.rs` `not_hosted()` with routing
- [x] T012a [US2] Failing cases `link_drop` (`OutcomeUnknown` 100/100, stream ends `daemon-offline`, re-attach after relink; FR-006, FR-012, SC-006) and `slow_client` (`slow-consumer`, other client keeps its rate; FR-004) in `crates/micold-daemon/tests/orchestrator_route/`
- [x] T012 [US2] Implement the link-drop and full-queue paths in `crates/micold-daemon/src/orchestrator/client.rs` and `link.rs` so T012a passes

## Phase 4: User Story 4 (orchestrator half) — an outage loses no session (P1)

- [x] T013 [US4] Case `outage` in `crates/micold-daemon/tests/orchestrator_route/`: kill the orchestrator, the session keeps running on the daemon and the link redials (FR-010)

## Phase 5: User Story 1 — lists show every daemon's rows (P1)

- [x] T014 [US1] Failing case `lists` in `crates/micold-daemon/tests/orchestrator_route/`: two daemons 100 % rows; one stopped named `offline`; one hung named `timeout` within 2.5 s (FR-001, FR-002, SC-001)
- [x] T015 [US1] Implement `OrchList` fan-out in `crates/micold-daemon/src/orchestrator/client.rs`: every linked daemon in parallel with a 2 s deadline, `merge_lists`, needs-update/offline links named without asking (R4)

## Phase 6: User Story 5 — routing is not noticeably slower (P2)

- [x] T016 [US5] `#[ignore]` benchmark case `bench` in `crates/micold-daemon/tests/orchestrator_route/`: 1,000 requests and 1,000 4 KiB chunks at 100/s, direct vs routed, asserts p95 deltas within 5 ms / 16 ms; run it in the milestone and record the numbers in the PR (SC-003)

## Phase 7: User Story 3 + US4 (app half) — one connection, started when absent (P1)

- [x] T017 [US3] Failing headless test `crates/micold-client/tests/orchestrator_single_connection.rs`: orchestrator spawned, exactly one client connection and none to a daemon socket; two concurrent starts leave one orchestrator; a missing binary gives a named error (FR-007, FR-008, SC-004)
- [x] T018 [US3] Generalise `connect::connect_or_spawn` to take the spawn command and endpoint; new `crates/micold-client/src/orchestrator_conn.rs` (one connection, channel per `(target, conn)`, `reconnect_loop` backoff); `crates/micold-client/src/daemon.rs` actors use a channel instead of a socket; start error in the existing connection banner (R6)
- [x] T019a [US4] Failing reconnect test in `crates/micold-client/tests/orchestrator_single_connection.rs`: orchestrator killed, client reconnects, actors re-handshake and re-read; `OutcomeUnknown` makes the actor re-request the target's snapshot (FR-009, SC-005)
- [x] T019 [US4] Implement re-handshake/re-read on reconnect and the `OutcomeUnknown` snapshot re-request in `crates/micold-client/src/orchestrator_conn.rs` and `crates/micold-client/src/daemon.rs` so T019a passes
- [x] T020a [P] [US3] Failing test in `crates/micold-client/src/daemon_config_render.rs`: rendered config has `[orchestrator_link] enabled = true` and `[daemon] id = <wire id>` (FR-013)
- [x] T020 [US3] Render those keys in `crates/micold-client/src/daemon_config_render.rs` and add `wire_id` (`slugify(name)-<8 hex>`, `local` for the host) to the provisioned registry entry, generated on next connect for existing entries, so T020a passes (R6)
- [x] T021 [US3] Run the socket-opening client suites `crates/micold-client/tests/daemon_routing.rs`, `crates/micold-client/tests/catalog_two_daemons.rs`, `crates/micold-client/tests/notifications.rs` and `crates/micold-client/tests/attention_notify.rs` with the client through the orchestrator and fix what fails (SC-002)
- [x] T022 [US3] User guide in `docs/user-guide/`: the app connects through the orchestrator and starts it; troubleshooting an orchestrator that cannot start (US3 scenario 4, Constitution VII)

## Phase 8: FR-007 in full — SSH, container and pod daemons through the orchestrator (P1)

Plan *Amendment*, research R7. Linux only (#1897).

- [x] T023 [P] [US3] Failing tests in `crates/micold-core/src/ssh/argv.rs`: `hold_link` argv carries the admitted `-R`, `ClearAllForwardings=no`, `ExitOnForwardFailure=yes` and the remote command `remote hold-link`, never `remote bridge`
- [x] T024 [US3] Add `argv::hold_link`, sharing `bridge_with_link`'s prefix, so T023 passes
- [x] T025 [US3] Failing tests in `crates/micold-core/src/provision/mod.rs` with the fake `SshHost`: a `hold_link` step admits the link (reusing `admit_link`), opens the holder and returns on its `ready` line; a `remote port forwarding failed` line gives `FailureCause::LinkForward(NotBound)`; a refused link opens nothing
- [x] T026 [US3] Implement the `hold_link` step in `crates/micold-core/src/provision/mod.rs` (holder handle owning the ssh child, reusing `FORWARD_NOT_BOUND`) so T025 passes
- [x] T027 [P] [US3] Failing test in `crates/micold-daemon/tests/remote_hold_link.rs` (new `[[test]]` in `crates/micold-daemon/Cargo.toml`): `micold-daemon remote hold-link` prints `ready`, opens no connection to the daemon socket and exits 0 on stdin EOF
- [x] T028 [US3] Add the `hold-link` subcommand to `crates/micold-daemon/src/remote.rs` (usage line too) so T027 passes
- [x] T029 [US3] Failing test in `crates/micold-client/tests/orchestrator_single_connection.rs`: with a real orchestrator, a route to a non-`local` wire id opened before that daemon links succeeds once it links and handshakes; with no daemon the deadline error names the wire id; the client still has one connection and none to the daemon socket (FR-007); a unit test in `crates/micold-client/src/daemon.rs` that `ssh_options`, `container_options` and the pod options carry the link from the unconditional `orchestrator_link()`
- [x] T030 [US3] In `crates/micold-client/src/daemon.rs`: make `orchestrator_link()` unconditional (no `cfg!(unix)`, #1897) returning `io::Result<LinkRequest>` (the `orchestrator_endpoint()` error is no longer dropped by `.ok()`; a connect path surfaces it in the daemon's status cell); add `routed_to(wire_id, deadline)` generalising `host_routed` (10 s), `host_routed` built on it, so T029 passes
- [x] T031 [US3] Failing test `crates/micold-client/tests/orchestrator_provisioned.rs` (registered in `crates/micold-client/tests/main.rs`), driving the provisioned connect path with the fake `SshHost` and a real orchestrator plus linked daemon: after provisioning the bridge is closed, the holder is open and the actor's transport is the route; a refused link (`LinkRefusal`) ends the connect with its cause in the daemon's status cell and opens no bridge for the actor; a passed `routed_to` deadline is shown in the status cell; after the swap the linked daemon counts no client connection on its socket (only the orchestrator link); the holder ending ends the route (`daemon offline`) and the actor's reconnect re-provisions; an orchestrator restart re-routes over the same holder without re-provisioning (FR-009); one case per placement (SSH host, container, pod with its relay dropped after the holder) (plan *Amendment*: Swap order, Lifetime, Failure, Orchestrator restart)
- [x] T032 [US3] Wire the SSH, container and pod connect paths in `crates/micold-client/src/daemon.rs`: after provisioning drop the bridge and await its ssh child's exit (releasing the remote socket, which `prepare_link_at` then removes), then start the holder (the pod's relay kept beside it), hand the actor `routed_to`'s `Connected`; the actor owns the holder for its route's life; no fallback to the bridge. T031 passes
- [x] T033 [US3] Extend `crates/micold-daemon/tests/ssh_real/ssh_real_link.rs` with a routed case (holder + route by wire id); runs where sshd exists, not in the cloud; `kube_real` not extended
- [x] T034 [US3] Update `docs/user-guide/orchestrator-mode.md`: remote, container and pod daemons also go through the orchestrator; a refused link forward is shown in the daemon's status cell (Constitution VII); add a line to `drift-M3.md`'s follow-up that M4 closed it

## Dependencies

Phase 8: T023→T024→T025→T026; T027→T028; T029→T030; T026, T028, T030 → T031 → T032; T033 after T032; T034 last. T023 and T027 may run in parallel.

Setup -> Foundational -> US2 -> US4 (orchestrator) -> US1 -> US5 -> US3/US4 (app) -> FR-007 in full (Phase 8, needs the app phase's orchestrator connection). US1 needs US2's client session; the app phase needs all orchestrator work. Quickstart Part B is done by the close unit.

## Implementation Strategy

Ship the contract and core first, then the orchestrator relay with its integration suite, then lists and the benchmark, and only then switch the desktop transport. Direct daemon connections keep working until M3, so each milestone merges green.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Contract, core and requests/streams routed through the orchestrator 🎯 MVP

- **Tasks**: T001–T013
- **Deliverable**: `client-routing.md` is on main at protocol 83, and a client connected to a real orchestrator creates sessions, attaches to terminal streams and sees takeovers on either of two linked daemons; refusals, `unknown` on link drop, `daemon-offline` stream end, slow-consumer isolation and session survival across an orchestrator outage hold.
- **Satisfies**: US2 scenarios 1–5; US4 (orchestrator half); FR-003–FR-006, FR-010, FR-011, FR-012 (orchestrator half), SC-006
- **Verify**: `scripts/build-lock.sh cargo test -p micold-core --all-targets routing route list_merge`; `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator_route`
- **Depends on**: —
- **Tier**: full
- **Note**: US2 and US4 (orchestrator half) share one milestone: the outage case reuses US2's harness and relay, and alone is one test with no code of its own.

### M2 — Aggregated lists and the latency budget

- **Tasks**: T014–T016
- **Deliverable**: one list request through the orchestrator returns both daemons' rows and names an offline or hung daemon within 2.5 s; the benchmark shows routing within SC-003.
- **Satisfies**: US1 scenarios; US5; FR-001, FR-002, SC-001, SC-003
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator_route lists`; `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator_route -- --ignored bench`
- **Depends on**: M1
- **Tier**: full
- **Note**: US1 and US5 share one milestone: US5 is one `#[ignore]` benchmark with no code of its own, measured over the routed path US1 completes.

### M3 — The desktop app uses one orchestrator connection

- **Tasks**: T017–T022
- **Deliverable**: the app starts the orchestrator when absent, opens only that connection, reconnects after an outage and re-reads, and writes `[orchestrator_link]` into daemons it provisions; the existing socket-opening suites pass through the orchestrator; the user guide says so.
- **Satisfies**: US3 scenarios 1–4; US4 (app half); FR-007–FR-009, FR-012 (app half), FR-013, SC-002, SC-004, SC-005
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --test orchestrator_single_connection --test daemon_routing --test catalog_two_daemons --test notifications --test attention_notify`; `scripts/build-lock.sh cargo test -p micold-client --lib daemon_config_render`; quickstart Part B
- **Depends on**: M1–M2
- **Tier**: full
- **Note**: US3 and US4 (app half) share one milestone: reconnect (US4) is the same `orchestrator_conn.rs` that US3 introduces, and the app cannot ship US3 without surviving an outage.

### M4 — Provisioned daemons through the orchestrator

- **Tasks**: T023–T034
- **Deliverable**: SSH-host, container and pod daemons are reached through the app's one orchestrator connection: after provisioning the bridge is replaced by a `remote hold-link` holder of the #1725 `-R`, and the actor routes by wire id; the app opens no client connection to any daemon (FR-007 in full).
- **Satisfies**: US3 scenario 1 for every placement; FR-007, SC-004
- **Verify**: `scripts/build-lock.sh cargo test -p micold-core --lib -- ssh::argv provision`; `scripts/build-lock.sh cargo test -p micold-daemon --test remote_hold_link`; `scripts/build-lock.sh cargo test -p micold-client --test main -- orchestrator_single_connection:: orchestrator_provisioned::`; `scripts/build-lock.sh cargo test -p micold-client --lib -- daemon::`; `scripts/build-lock.sh cargo test -p micold-daemon --features ssh-real-runtime --test ssh_real -- ssh_real_link` (needs sshd; not run in the cloud)
- **Depends on**: M3
- **Tier**: full
- **Note**: process boundary (a new remote subcommand, holder lifetime), so full.
