# Implementation Plan: The desktop app talks only to the orchestrator

**Branch**: `claude/project-thread-v4iabl` | **Date**: 2026-11-16 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1726-app-via-orchestrator/spec.md` (issue #1726, epic #1722)

## Summary

The orchestrator (#1724) holds one link per daemon (#1725) but carries only pings on it and refuses
session work from clients. This feature makes the link carry client traffic and makes the
orchestrator a relay:

- **Daemon side**: each routed `conn` arriving on the link is a *virtual client connection*,
  served by the daemon's existing `server::serve_connection` over an in-memory duplex. Every
  existing behaviour (terminal and structured streams, takeover, notifications, progress) then
  works unchanged, because it is the same code path a socket client uses (research R1).
- **Orchestrator side**: a client connection that opens with a plain `Hello` is a *client
  session*. Its frames come wrapped in a routed envelope naming the target daemon; the orchestrator
  maps `(client, target)` to a link `conn` id and relays inner frames as opaque bytes, decoding only
  the envelope header (R2, R3). Lists are a new orchestrator-level request answered by fanning out
  to every linked daemon in parallel with a 2 s deadline, unioning the rows and naming every daemon
  missing with a reason (R4).
- **Desktop side (minimal)**: the per-daemon actors keep their state and views; only their
  transport changes, from one socket per daemon to one virtual channel over a single orchestrator
  connection, which the app starts with `connect_or_spawn` on the orchestrator endpoint (R5, R6).
  The app also writes `[orchestrator_link] enabled = true` (and the daemon id) when it provisions
  (FR-013).

Guardrails and tests sit at the protocol, orchestrator and daemon level (spawned real
orchestrator + real daemons in `crates/micold-daemon/tests/`), per the cloud override: the iced app
is being retired, and the web client (#1732, #1735) will use the same contract
(`specs/orchestrator/contracts/client-routing.md`, FR-011).

## Technical Context

**Language/Version**: Rust (workspace toolchain, edition 2021)

**Primary Dependencies**: tokio, tokio-util `Framed`/`LengthDelimitedCodec`, serde_json, postcard
(existing); no new crate.

**Storage**: none new. The registry (`orchestrator/registry.json`) is unchanged; routes and
subscriptions are in memory only and rebuilt on reconnect (FR-009).

**Testing**: `cargo test` via `scripts/gate.sh`; core unit tests (`micold-core`), daemon-crate
integration tests that spawn a real orchestrator and real daemons
(`crates/micold-daemon/tests/orchestrator_route/`), one headless client test for the single
connection and the start (`crates/micold-client/tests/`).

**Target Platform**: orchestrator and daemons on Linux (and macOS for the local pair); the app on
Linux, macOS, Windows (orchestrator endpoint is a named pipe on Windows via the existing
`endpoint::orchestrator_endpoint`).

**Project Type**: Rust workspace: core library, daemon/orchestrator binary, iced desktop client.

**Performance Goals**: SC-003: +5 ms p95 per request, +16 ms p95 per 4 KiB terminal chunk at
100 chunks/s, measured against a direct connection.

**Constraints**: FR-002 list deadline 2 s per daemon, in parallel. Per-stream back-pressure
(FR-004): one bounded queue per `(link, conn)`; a full queue ends that conn, never stalls the link.
SC-004: orchestrator running and connected within 5 s of app start.

**Scale/Scope**: tens of daemons, a few clients, hundreds of sessions per orchestrator.

## Constitution Check

*Re-checked after Phase 1: unchanged.*

- [x] **I. Test-First**: every milestone starts with failing tests listed in quickstart.md Part A
  (contract decode tests, two-daemon aggregation, routed stream order, link-drop `unknown`,
  single-connection client test, benchmark). PASS.
- [x] **II. Multi-Session**: routing keys by daemon-qualified ref and per-client `conn`; no
  session state moves into the orchestrator; each virtual connection is the daemon's own
  per-connection state. PASS.
- [x] **III. Worktree Integration**: worktree operations are relayed unchanged to the owning
  daemon. PASS.
- [x] **IV. Local-First**: the orchestrator is local, started by the app; no network service.
  PASS.
- [~] **V. TS + RN client**: the change touches the iced client, which the constitution marks for
  retirement. Justified in Complexity Tracking: kept to the transport seam; the contract is
  client-neutral for the RN client.
- [x] **VI. Linux daemons, web reach**: daemons/orchestrator Linux; endpoint abstraction covers
  macOS/Windows for the app; no browser code. PASS.
- [x] **VII. Docs**: `docs/` user guide gains "the app connects through the orchestrator" and the
  troubleshooting of an orchestrator that cannot start (US3 scenario 4); contract under
  `specs/orchestrator/contracts/`. PASS.
- [x] **VIII. UI components**: no new UI component; the reconnecting state reuses the existing
  connection banner. PASS.

## Project Structure

### Documentation (this feature)

```text
specs/1726-app-via-orchestrator/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/README.md          # points at the shared contract
└── tasks.md                     # tasks unit
specs/orchestrator/contracts/client-routing.md   # FR-011, shared with the web client
```

### Source Code

```text
crates/micold-core/src/protocol/
├── routing.rs        # RoutedEnvelope gains the opaque-frame body form (R3); ListKind, MissingDaemon
├── messages.rs       # ClientMsg::OrchList, DaemonMsg::OrchListed, StreamEnded, OutcomeUnknown
└── version.rs        # PROTOCOL_VERSION 82 -> 83
crates/micold-daemon/src/
├── orchestrator/
│   ├── link.rs       # steady(): demux inbound routed frames to conns; outbound queues per conn
│   ├── client.rs     # NEW: client session: Hello, routes, relay, list fan-out
│   ├── route.rs      # NEW: RouteTable (client conn <-> link conn), pending requests
│   └── query.rs      # not_hosted() replaced by routing; listing kept
└── orchestrator_link/
    ├── dial.rs       # steady(): routed frames -> virtual connections
    └── virtual_conn.rs  # NEW: duplex per conn into server::serve_connection
crates/micold-client/src/
├── daemon.rs         # actor transport: orchestrator channel instead of daemon socket
├── orchestrator_conn.rs  # NEW: one connection, connect_or_spawn, channel mux, reconnect
└── daemon_config_render.rs  # [orchestrator_link] enabled = true, [daemon] id (FR-013)
crates/micold-daemon/tests/orchestrator_route/   # NEW integration suite (M2, M3)
crates/micold-client/tests/orchestrator_single_connection.rs   # NEW (M4)
docs/user-guide/...   # M4
```

**Structure Decision**: routing logic lives in `micold-daemon/src/orchestrator/` beside the link
and registry; the reusable, render-free parts (route table, list merge, envelope) live in
`micold-core` so they test with `cargo test -p micold-core` and the web client's server reuses them.

## Milestones

| ID | Deliverable | Covers | Tier |
|---|---|---|---|
| M1 | Contract + core: `client-routing.md`; opaque routed frame form, list/outcome/stream-end messages, `RouteTable` and `merge_lists` in core with unit tests; protocol 83 | FR-011, FR-001/FR-005 shapes | core |
| M2 | Routing end to end: daemon virtual connections over the link; orchestrator client sessions relaying requests and streams; `not_found`/`needs-update`/offline refusals; `unknown` on link drop; `daemon offline` stream end; per-conn back-pressure. Integration suite with a real orchestrator and two real daemons | FR-003..FR-006, FR-010, FR-012 (orchestrator half) | daemon |
| M3 | Aggregated lists with the 2 s parallel deadline and reasons; SC-003 benchmark test (ignored by default, run in the milestone) | FR-001, FR-002, SC-001, SC-003 | daemon |
| M4 | Desktop: one orchestrator connection started with `connect_or_spawn`, actors over channels, reconnect + re-read + re-attach, `unknown` -> re-read, start failure message; FR-013 config writer; docs | FR-007..FR-009, FR-012 (app half), FR-013, SC-002, SC-004, SC-005 | client |

M1 to M3 leave the app as it is (direct connections still work: the daemon's own listener is
untouched), so each merges green on its own. M4 is the only desktop change.

## Risks

- **Grid frames through JSON**: avoided by the opaque form (R3); the benchmark in M3 guards it.
- **Daemon id mapping**: the app's actors must know each daemon's wire id. The host daemon is
  `local`; the app writes `[daemon] id` for daemons it provisions (FR-013 writer, R6). A daemon
  whose id the app cannot map is shown from the aggregated list only.
- **SC-002 suites**: most client acceptance suites are render-free and do not open sockets; the
  suites that do (`daemon_routing`, `catalog_two_daemons`, takeover/notification) are run against an
  orchestrator in M4. The daemon-level M2 suite carries the stream/takeover guarantees, so the
  desktop run is a confirmation, not the guardrail.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| Change to the iced client (Principle V says the RN client is the UI) | FR-007: the shipped app must stop opening daemon connections now | Waiting for the RN client leaves the orchestrator unused and FR-007 unmet; the change is confined to the transport seam (`orchestrator_conn.rs` + actor connect) |
