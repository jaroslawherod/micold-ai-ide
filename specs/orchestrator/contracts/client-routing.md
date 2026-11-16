# Contract: clients through the orchestrator (lists, routed requests and streams)

Feature 1726 (issue #1726, epic #1722). Covers orchestrator FR-005, FR-006, FR-007, FR-010.
Extends [routed-envelope.md](routed-envelope.md). Protocol version 83. Source of truth in code:
`crates/micold-core/src/protocol/routing.rs` and `messages.rs`.

## 1. Client session

1. The client connects to the orchestrator endpoint and sends a plain `Hello` (Control frame).
   The orchestrator answers `Welcome` (or `Refused` on a version mismatch, as a daemon does).
2. From then on every frame either way is `Routed` (kind `2`).
3. The orchestrator never starts sessions itself; every session request goes to a daemon.

## 2. Routed frame forms

| Encoding byte | Body |
|---|---|
| `0` JSON | `{target, conn, seq, req?, msg}` as in routed-envelope.md §3, plus `req` (see below) |
| `1` opaque | u32 BE header length, JSON header `{target, conn, seq, req?}`, then one whole inner frame (its own kind, encoding, reserved bytes and body) |

The orchestrator reads only the header. `target` absent: the frame is for the orchestrator and
must be JSON with an `OrchList` body; anything else is `Malformed`. The client MUST set header `req` on every
frame whose inner message is a request, and omit it otherwise; it lets the orchestrator refuse
(§3) and answer `OutcomeUnknown` (§5) without decoding opaque frames. For JSON frames only, the
orchestrator checks that header `req` equals the inner message's `req` (a mismatch is
`Malformed`); opaque frames are not decoded, so their header is trusted as sent. On answers from
a daemon, header `req` is set by the daemon link from the answer's `req`.

An opaque frame whose inner part is empty means "this conn ends": a client sends it to close a
conn, and a daemon sends it when its connection for that conn closed (the client then gets
`StreamEnded { reason: "closed" }`).

Examples (the test `client_routing_contract` decodes every fence labelled `client-example`):

```json client-example request
{"target":"build-box","conn":7,"seq":2,"msg":{"Ping":{"nonce":9}}}
```

```json client-example opaque-header
{"target":"build-box","conn":7,"seq":3,"req":12}
```

A conn is the client's choice, per target. The first frame on a new `(target, conn)` opens a route;
the inner stream on it starts with the client's `Hello` to the daemon, exactly as on a direct
socket.

## 3. Refusals (FR-005)

| Target | Answer on that conn |
|---|---|
| unknown daemon id | `OperationError { kind: not_found }` for the `req`, or `StreamEnded { reason: "not_found" }` when there is none |
| `needs-update` | `OperationError { kind: refused, message: "needs-update" }`, or `StreamEnded { reason: "needs-update" }` |
| `offline` | `OperationError { kind: refused, message: "offline" }`, or `StreamEnded { reason: "offline" }` |

```json client-example refusal
{"target":"nowhere","conn":1,"seq":2,"req":3,"msg":{"OperationError":{"req":3,"kind":"NotFound","message":"unknown daemon","detail":null}}}
```

## 4. Lists (FR-001, FR-002)

Request (no target): `OrchList { req, kind: "projects" | "worktrees" | "sessions" }`.
Answer: `OrchListed { req, rows, missing: [{daemon, reason}], no_daemons }`.

- Every linked daemon is asked in parallel, each with a 2 s deadline; the answer comes within
  about 2 s whatever the daemons do.
- `reason`: `offline`, `needs-update` (from the registry, not asked), `timeout`, `error: <message>`.
- Rows keep their daemon-qualified refs; rows of different daemons are never merged.
- No daemon linked: `rows: []`, `missing: []`, `no_daemons: true`. Not an error.

## 5. Streams, drops and outcomes (FR-004, FR-006, FR-012)

- Frames on one route are delivered in order, none lost or duplicated while the link is up.
- Each route has its own bounded queue toward the client and another toward the daemon; when
  either is full the route ends with `StreamEnded { reason: "slow-consumer" }`, an answer that did
  not fit counting as not answered (`OutcomeUnknown`); frames already queued are still delivered.
  A daemon stall that outlasts a route's 1024-frame queue toward it ends that route the same way.
  Other routes and the link continue.
- The daemon's side of a link is one shared pipe: a daemon reads the link in order, so a conn that
  stalls the daemon's own handling stalls the other conns on that link (back-pressure, not loss).
- Link drop: every route to that daemon gets `OutcomeUnknown { req }` for each forwarded request
  not yet answered, then `StreamEnded { reason: "daemon-offline" }`. Nothing is retried. The client
  re-reads and re-attaches when the daemon is listed online again.
- Client drop: the orchestrator closes that client's link conns; the daemon sees a client leave.
- `StreamEnded.reason` is one of `daemon-offline`, `slow-consumer`, `client-gone`, `closed`,
  `not_found`, `needs-update`, `offline`.

```json client-example stream-ended
{"target":"build-box","conn":7,"seq":9,"msg":{"StreamEnded":{"reason":"daemon-offline"}}}
```

```json client-example outcome-unknown
{"target":"build-box","conn":7,"seq":10,"req":11,"msg":{"OutcomeUnknown":{"req":11}}}
```

## 6. Worked example

Client asks for sessions; `build-box` is offline:

```json client-example orch-list
{"conn":0,"seq":1,"req":5,"msg":{"OrchList":{"req":5,"kind":"sessions"}}}
```

```json client-example orch-listed
{"conn":0,"seq":1,"req":5,"msg":{"OrchListed":{"req":5,"rows":{"sessions":[]},"missing":[{"daemon":"build-box","reason":"offline"}],"no_daemons":false}}}
```

(A row is a full `SessionSummary` with its `daemon` ref; the example has none.)
