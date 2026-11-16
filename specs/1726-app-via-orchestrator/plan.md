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

**Target Platform**: Linux only (#1897): orchestrator, daemons and the app.

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
- [x] **VI. Linux daemons, web reach**: daemons/orchestrator Linux; Linux only (#1897); no browser
  code. PASS.
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
│   ├── route.rs      # NEW: RouteTable (client conn <-> link conn), pending requests (unit tests: insert, clear, drop_link)
│   └── query.rs      # not_hosted() replaced by routing; listing kept
└── orchestrator_link/
    ├── mod.rs        # spawn_for(Arc<DaemonState>); invariant reworded (R1)
    ├── dial.rs       # steady(framed, Arc<DaemonState>): routed frames -> virtual connections; decodes outbound JSON for answer req
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

## Amendment: provisioned daemons through the orchestrator (tasks M4)

Numbering: the milestone table above predates the tasks cut. Plan M1+M2 = tasks M1, plan M3 =
tasks M2, plan M4 = tasks M3; this amendment is tasks M4 (quickstart: M4b).

Added 2026-11-16 after M3 (#2007) shipped the host daemon only (drift-M3.md, *Scope narrowed*);
jaro chose to finish FR-007 for SSH, container and Kubernetes placements in one more milestone.
Research R7 has the route per placement and the alternatives.

- **Route (all three placements, R7)**: the existing #1725 reverse forward. Containers and pods are
  already reached over SSH with a pinned reach (`container_options`, the pod's relay on
  `127.0.0.1:<port>`), and all three `ProvisionOptions` already carry `link: orchestrator_link()`,
  so every provisioned daemon already dials the orchestrator through `-R`. No new
  transport, no dial-out to a TCP address, no new kube port-forward.
- **What changes**: the bridge (`micold-daemon remote bridge`) is today both the holder of the
  `-R` and the actor's daemon client connection. Provisioning keeps it as is up to the handshake
  (version check, install on mismatch: that connection is part of provisioning, FR-007 keeps it).
  Then the app closes it and opens a **link holder**: the same `ssh` argv with the admitted link
  (`ExitOnForwardFailure=yes`) running a new remote subcommand `micold-daemon remote hold-link`,
  which opens no daemon connection, prints `ready` and waits for stdin EOF. The actor then opens a
  route to the daemon's wire id on the one orchestrator connection and handshakes on it, retrying
  while the orchestrator answers `not_found` (the daemon's link not up yet) until a 10 s deadline,
  the loop `host_routed` already runs for `local`.
- **Swap order**: (1) the provisioning handshake ends and the app closes the bridge's stdin; (2) it
  waits for the bridge's ssh child to exit, which releases the remote socket of the `-R`
  (`prepare-link` removes a stale socket file before each bind, #1725); (3) it opens the holder.
  `ready` is printed by the remote command, which ssh starts only after the forward is confirmed
  under `ExitOnForwardFailure=yes`, so `ready` means "forward bound", not "link up". The daemon's
  link drops at (1) and redials with its backoff; `routed_to`'s retry on `not_found` until the 10 s
  deadline covers it coming back.
- **Precondition**: the daemon's config carries `[daemon] id` and `[orchestrator_link]` (M3's
  FR-013 writer, re-rendered on every provision/connect, T020), so the wire id exists before
  `routed_to` is called.
- **Orchestrator restart** (FR-009): the holder and `-R` survive; the daemon redials the restarted
  orchestrator; the actor's route ends and it re-routes with `routed_to` over the same holder, with
  no re-provisioning. Only a dead holder re-provisions.
- **Lifetime**: the actor owns the holder (and, for a pod, the relay it rides on) for as long as its
  route lives. A dead holder drops the daemon's link, the orchestrator ends the route with
  `daemon offline`, and the actor's existing reconnect re-provisions, as a dead bridge does today.
- **Failure**: a refused link (`LinkRefusal`, e.g. a user's `ssh_config` forward) or a passed
  deadline fails the connect with the cause in the daemon's status cell; no fallback to the direct
  bridge (FR-007).
- **Code**: `crates/micold-core/src/ssh/argv.rs` (`hold_link` argv, shares `bridge_with_link`'s
  prefix), `crates/micold-core/src/provision/mod.rs` (`hold_link` step: admit the link, open the
  holder, read `ready` or the `remote port forwarding failed` line, reusing `admit_link` and
  `FORWARD_NOT_BOUND`), `crates/micold-daemon/src/remote.rs` (`hold-link`),
  `crates/micold-client/src/daemon.rs` (`orchestrator_link()` made unconditional: no `cfg!(unix)`,
  returns a `LinkRequest`, Windows wording dropped, #1897; one `routed_to(wire_id, deadline)` helper generalising
  `host_routed`; the SSH, container and pod connect paths call it after provisioning).
- **Tests**: core unit tests with the fake `SshHost` (holder argv carries the `-R` and
  `ExitOnForwardFailure`, never a bridge; not-bound refusal); a daemon test for `remote hold-link`
  (no connection to the daemon socket, exits on EOF); a client test with a real orchestrator and a
  linked daemon announcing a non-`local` wire id (route opened before the link is up succeeds once
  it is; deadline names the daemon). `ssh_real_link` gains a routed case (runs where sshd exists;
  `kube_real` cannot run in the cloud and is not extended).

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
