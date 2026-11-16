# Research: the desktop app talks only to the orchestrator

## R1: How the daemon serves routed traffic

**Decision**: every routed `conn` on the link is a virtual client connection: the daemon's link
task creates a `tokio::io::duplex` per new `conn` and runs the existing
`server::serve_connection(state, half)` on one half; the link task writes inbound inner frames into
the other half and wraps what comes out back into routed frames for that `conn`.

**Rationale**: `serve_connection<S: AsyncRead + AsyncWrite>` is already generic. Handshake, takeover,
attention, progress, structured and terminal streams all keep their one implementation, so SC-002
holds by construction and FR-010 (sessions survive an orchestrator outage) stays true: a dropped
link ends virtual connections exactly as a dropped socket ends a client.

**Alternatives**: a per-message dispatcher on the link (re-implements the connection state
machine; every stream feature would need porting); a new daemon listener the orchestrator dials
(spec Assumption: the link is the only path).

## R2: How a client addresses a daemon through the orchestrator

**Decision**: a client connection to the orchestrator opens with the usual `Hello` (plain control
frame, as the routed-envelope contract requires) and gets `Welcome`; then it switches to routed
frames. Each routed frame's `target` names the daemon; the client picks its own `conn` per target.
The orchestrator keeps a `RouteTable`: `(client id, client conn) <-> (daemon, link conn)`, link
conns allocated per daemon. A frame with no target and an `OrchList` body is for the orchestrator.

**Rationale**: matches `routed-envelope.md` as shipped by #1723 (target, conn, seq); the client
needs no knowledge of other clients' conns; a refusal for an unknown or unusable daemon needs only
the target, before anything is relayed.

**Alternatives**: the orchestrator reading the ref inside every `ClientMsg` (dozens of variants, many
without a ref, e.g. `Ping`, terminal input): rejected.

## R3: Opaque inner frames

**Decision**: add a second routed body form: header kind `2`, encoding byte `1` (opaque): a short
JSON header `{target, conn, seq}` length-prefixed, followed by the inner frame's own bytes (kind,
encoding, body). The orchestrator decodes only the header and copies the bytes. Encoding `0`
(JSON `msg`) stays valid for control messages from non-Rust peers.

**Rationale**: terminal output is postcard `Grid` frames; re-encoding them through JSON per hop
risks SC-003 and costs CPU per chunk. Copying bytes keeps the orchestrator out of message
semantics, so new message variants need no orchestrator change.

**Alternatives**: decode and re-encode every frame (slower, couples orchestrator to every variant).

## R4: Lists

**Decision**: new `ClientMsg::OrchList { req, kind: Projects | Worktrees | Sessions }` sent to the
orchestrator. The orchestrator opens (once per daemon, kept open) an orchestrator-owned virtual
connection on the link (`conn 0`) and asks each online daemon with its existing requests (the
project catalogue from `Welcome`, worktree snapshots, the session list `Attach` returns) under a
2 s `tokio::time::timeout`, all daemons via `join_all`. The answer
`DaemonMsg::OrchListed { req, rows, missing: [{daemon, reason}] }` uses the pure `merge_lists` in
core: rows kept apart by daemon id, never merged (two daemons announcing one path stay two rows).
Reasons: `offline` and `needs-update` from the registry without asking; `timeout`; `error: <msg>`.
No daemon linked: empty rows, empty missing, `no_daemons: true`.

**Rationale**: no new daemon request is needed; the merge and reason logic is a pure function,
tested in core. Parallel + per-daemon timeout gives the 2 s bound (FR-002).

**Alternatives**: a new daemon `ListRows` request (extra surface for the same data); caching rows
in the orchestrator (stale after an outage; FR-009 wants the daemons' truth).

## R5: Drops, outcomes, back-pressure

**Decision**:
- Link drop: for every route on that link, the orchestrator sends the client
  `DaemonMsg::StreamEnded { reason: DaemonOffline }` on that conn and, for each request id it saw
  forwarded and not yet answered, `DaemonMsg::OutcomeUnknown { req }`. To know pending ids
  without decoding opaque frames, the client's routed header carries `req` when the inner frame is
  a request (optional header field), and the daemon's answer header carries the `req` it answers.
  No retry (FR-006).
- Back-pressure: one bounded `mpsc` (64 frames) per `(link, conn)` towards the client and per
  client conn towards the link writer; a full queue ends that route with `StreamEnded { reason:
  SlowConsumer }`, the link and other routes go on (FR-004).
- Ordering: one queue per route, FIFO; a link is one ordered byte stream; no reordering point.

## R6: Desktop transport and start

**Decision**: `orchestrator_conn.rs` holds the one connection, opened with
`connect::connect_or_spawn` generalised to take the spawn command (`micold-daemon orchestrator`)
and the orchestrator endpoint; the orchestrator's existing singleton (`crate::singleton::acquire`)
makes concurrent starts converge on one (US3 scenario 3). Each daemon actor gets a channel
(`target`, its own conn) in place of its socket, keeping its pump, so views are unchanged. Wire id:
host daemon `local`; for provisioned daemons the app writes `[daemon] id = <actor's DaemonId
slug>` and `[orchestrator_link] enabled = true` into the rendered config (FR-013). Start failure:
the error from `connect_or_spawn` is shown in the existing connection banner; no fallback to a
direct daemon socket. Reconnect: the existing `reconnect_loop` backoff applies to the orchestrator
connection; on reconnect every actor re-handshakes its channel, which re-reads state as a fresh
connect does today; on `OutcomeUnknown` the actor re-requests the target's snapshot.

**Alternatives**: rewriting the client around the aggregated list (large UI change on an app being
retired).
