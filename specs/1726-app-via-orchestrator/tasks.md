# Tasks: The desktop app talks only to the orchestrator

**Input**: `specs/1726-app-via-orchestrator/` (plan.md, research.md, data-model.md, quickstart.md, contracts/)
**Tests**: mandatory (Constitution I); each test task is written failing before its implementation task.
**Format**: `[ID] [P?] [Story] Description` — `[P]` = different files, no dependency on an unfinished task.

## Phase 1: Setup

- [ ] T001 Write the shared contract `specs/orchestrator/contracts/client-routing.md` (client `Hello`, routed envelope opaque form, `OrchList`/`OrchListed`, `MissingReason` wire text, `StreamEnded`, `OutcomeUnknown`, refusals) with one JSON example per frame, and point `specs/1726-app-via-orchestrator/contracts/README.md` at it (FR-011)

## Phase 2: Foundational (core, render-free)

- [ ] T002 [P] Failing tests in `crates/micold-core/src/protocol/routing.rs`: opaque-form round trip, header-only decode, `req` mismatch is `Malformed`, contract examples from T001 decode (FR-011)
- [ ] T003 Implement `RoutedHeader` and the opaque body form of `RoutedEnvelope` in `crates/micold-core/src/protocol/routing.rs` (R3) so T002 passes
- [ ] T004 [P] Add `ClientMsg::OrchList`, `DaemonMsg::OrchListed`, `StreamEnded`, `OutcomeUnknown`, `ListKind`, `MissingDaemon`/`MissingReason`, `StreamEndReason`, `RouteRefusal` in `crates/micold-core/src/protocol/messages.rs` and `routing.rs`, with serde round-trip tests; bump `PROTOCOL_VERSION` 82 -> 83 in `crates/micold-core/src/protocol/version.rs`
- [ ] T005 [P] Failing unit tests for `RouteTable` (bijection, link conns never reused while open, `drop_link` returns every route and pending req) in a new `crates/micold-core/src/orchestrator/route.rs` (data-model)
- [ ] T006 Implement `RouteTable` in `crates/micold-core/src/orchestrator/route.rs` so T005 passes
- [ ] T007 [P] Failing unit tests for `merge_lists` (union, same path on two daemons stays two rows, a daemon in rows or missing never both, `no_daemons`) in a new `crates/micold-core/src/orchestrator/list_merge.rs` (FR-001)
- [ ] T008 Implement `merge_lists` in `crates/micold-core/src/orchestrator/list_merge.rs` so T007 passes

## Phase 3: User Story 2 — every request and stream reaches the owning daemon (P1)

- [ ] T009 [US2] Failing integration suite `crates/micold-daemon/tests/orchestrator_route/main.rs` with a harness spawning a real orchestrator and two real linked daemons; cases `requests` (FR-003, US2.1), `streams` incl. two-client takeover (FR-004, US2.2, US2.4), `refusals` (FR-005)
- [ ] T010 [US2] Daemon side: `crates/micold-daemon/src/orchestrator_link/virtual_conn.rs` (duplex per link conn into `server::serve_connection`, R1); `dial.rs` `steady(framed, Arc<DaemonState>)` demuxes routed frames and stamps `req` on answers; `mod.rs` `spawn_for(Arc<DaemonState>)`
- [ ] T011 [US2] Orchestrator side: new `crates/micold-daemon/src/orchestrator/client.rs` (client session on plain `Hello`, opaque relay via `RouteTable`, refusals `not_found`/`needs-update`/offline); `link.rs` `steady()` demux inbound frames to conns with one bounded queue per `(link, conn)`; replace `query.rs` `not_hosted()` with routing
- [ ] T012 [US2] Failing then passing cases `link_drop` (`OutcomeUnknown` 100/100, stream ends `daemon-offline`, re-attach after relink; FR-006, FR-012, SC-006) and `slow_client` (`slow-consumer`, other client keeps its rate; FR-004) in `crates/micold-daemon/tests/orchestrator_route/`; implement the link-drop and full-queue paths in `client.rs`/`link.rs`

## Phase 4: User Story 4 (orchestrator half) — an outage loses no session (P1)

- [ ] T013 [US4] Case `outage` in `crates/micold-daemon/tests/orchestrator_route/`: kill the orchestrator, the session keeps running on the daemon and the link redials (FR-010)

## Phase 5: User Story 1 — lists show every daemon's rows (P1)

- [ ] T014 [US1] Failing case `lists` in `crates/micold-daemon/tests/orchestrator_route/`: two daemons 100 % rows; one stopped named `offline`; one hung named `timeout` within 2.5 s (FR-001, FR-002, SC-001)
- [ ] T015 [US1] Implement `OrchList` fan-out in `crates/micold-daemon/src/orchestrator/client.rs`: every linked daemon in parallel with a 2 s deadline, `merge_lists`, needs-update/offline links named without asking (R4)

## Phase 6: User Story 5 — routing is not noticeably slower (P2)

- [ ] T016 [US5] `#[ignore]` benchmark case `bench` in `crates/micold-daemon/tests/orchestrator_route/`: 1,000 requests and 1,000 4 KiB chunks at 100/s, direct vs routed, asserts p95 deltas within 5 ms / 16 ms; run it in the milestone and record the numbers in the PR (SC-003)

## Phase 7: User Story 3 + US4 (app half) — one connection, started when absent (P1)

- [ ] T017 [US3] Failing headless test `crates/micold-client/tests/orchestrator_single_connection.rs`: orchestrator spawned, exactly one client connection and none to a daemon socket; two concurrent starts leave one orchestrator; a missing binary gives a named error (FR-007, FR-008, SC-004)
- [ ] T018 [US3] Generalise `connect::connect_or_spawn` to take the spawn command and endpoint; new `crates/micold-client/src/orchestrator_conn.rs` (one connection, channel per `(target, conn)`, `reconnect_loop` backoff); `crates/micold-client/src/daemon.rs` actors use a channel instead of a socket; start error in the existing connection banner (R6)
- [ ] T019 [US4] Failing then passing reconnect test in `crates/micold-client/tests/orchestrator_single_connection.rs`: orchestrator killed, client reconnects, actors re-handshake and re-read; `OutcomeUnknown` makes the actor re-request the target's snapshot (FR-009, SC-005)
- [ ] T020 [P] [US3] Failing then passing test in `crates/micold-client/src/daemon_config_render.rs`: rendered config has `[orchestrator_link] enabled = true` and `[daemon] id = <wire id>`; add `wire_id` (`slugify(name)-<8 hex>`, `local` for the host) to the provisioned registry entry, generated on next connect for existing entries (FR-013, R6)
- [ ] T021 [US3] Run the socket-opening client suites (`daemon_routing`, `catalog_two_daemons`, takeover/notification) with the client through the orchestrator and fix what fails (SC-002)
- [ ] T022 [US3] User guide in `docs/user-guide/`: the app connects through the orchestrator and starts it; troubleshooting an orchestrator that cannot start (US3 scenario 4, Constitution VII)

## Dependencies

Setup -> Foundational -> US2 -> US4 (orchestrator) -> US1 -> US5 -> US3/US4 (app). US1 needs US2's client session; the app phase needs all orchestrator work. Quickstart Part B is done by the close unit.

## Implementation Strategy

Ship the contract and core first, then the orchestrator relay with its integration suite, then lists and the benchmark, and only then switch the desktop transport. Direct daemon connections keep working until M4, so each milestone merges green.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Contract and core routing types 🎯 MVP

- **Tasks**: T001–T008
- **Deliverable**: `client-routing.md` is on main and `micold-core` encodes, decodes, routes and merges per that contract at protocol 83, proven by its unit tests.
- **Satisfies**: FR-011; FR-001 and FR-006 shapes (data-model)
- **Verify**: `scripts/build-lock.sh cargo test -p micold-core --all-targets routing route list_merge`
- **Depends on**: —
- **Tier**: full
- **Note**: rule 1 deviation: no user story is observable in core alone; the plan puts the shared contract first so M2 and the web client (#1732) build on one wire. Recorded in the ledger.

### M2 — Requests and streams routed through the orchestrator

- **Tasks**: T009–T013
- **Deliverable**: a client connected to a real orchestrator creates sessions, attaches to terminal streams and sees takeovers on either of two linked daemons; refusals, `unknown` on link drop, `daemon-offline` stream end and slow-consumer isolation hold.
- **Satisfies**: US2 scenarios 1–5; US4 (orchestrator half); FR-003–FR-006, FR-010, FR-012 (orchestrator half), SC-006
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator_route`
- **Depends on**: M1
- **Tier**: full

### M3 — Aggregated lists and the latency budget

- **Tasks**: T014–T016
- **Deliverable**: one list request through the orchestrator returns both daemons' rows and names an offline or hung daemon within 2.5 s; the benchmark shows routing within SC-003.
- **Satisfies**: US1 scenarios; US5; FR-001, FR-002, SC-001, SC-003
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator_route lists`; `... -- --ignored bench`
- **Depends on**: M2
- **Tier**: full

### M4 — The desktop app uses one orchestrator connection

- **Tasks**: T017–T022
- **Deliverable**: the app starts the orchestrator when absent, opens only that connection, reconnects after an outage and re-reads, and writes `[orchestrator_link]` into daemons it provisions; the user guide says so.
- **Satisfies**: US3 scenarios 1–4; US4 (app half); FR-007–FR-009, FR-012 (app half), FR-013, SC-002, SC-004, SC-005
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --test orchestrator_single_connection`; quickstart Part B
- **Depends on**: M1–M3
- **Tier**: full
