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

A conn is the client's choice, per target. The first frame on a new `(target, conn)` opens a route;
the inner stream on it starts with the client's `Hello` to the daemon, exactly as on a direct
socket.

## 3. Refusals (FR-005)

| Target | Answer on that conn |
|---|---|
| unknown daemon id | `OperationError { kind: not_found }` for the `req`, or `StreamEnded { reason: not_found }` when there is none |
| `needs-update` | `OperationError { kind: refused, message: "needs-update" }` |
| `offline` | `OperationError { kind: refused, message: "offline" }` |

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
- Each route has its own bounded queue; when it is full the route ends with
  `StreamEnded { reason: "slow-consumer" }`; other routes and the link continue.
- Link drop: every route to that daemon gets `OutcomeUnknown { req }` for each forwarded request
  not yet answered, then `StreamEnded { reason: "daemon-offline" }`. Nothing is retried. The client
  re-reads and re-attaches when the daemon is listed online again.
- Client drop: the orchestrator closes that client's link conns; the daemon sees a client leave.

## 6. Worked example

Client asks for sessions; `build-box` is offline:

```json
{"conn":0,"seq":1,"msg":{"OrchList":{"req":5,"kind":"sessions"}}}
{"conn":0,"seq":1,"msg":{"OrchListed":{"req":5,"rows":{"sessions":[{"daemon":"local","id":"s1"}]},"missing":[{"daemon":"build-box","reason":"offline"}],"no_daemons":false}}}
```
