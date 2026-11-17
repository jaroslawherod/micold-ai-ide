# Research: client WebSocket API (#1732)

Each entry: decision, why, rejected alternatives.

## R1 WebSocket server: `tokio-tungstenite` with an upgrade callback
Already resolved in `Cargo.lock` (0.29, via `kube`), pure Rust, no C. `accept_hdr_async` hands the request to a
callback before the 101 response, so a foreign `Origin` is refused with 403 before any frame (FR-004) and `/api/ws`
is the only path (others get 404). The existing hand-rolled `http.rs` stays for the two loopback hooks.
Rejected: `axum` (a large new tree for one route); hand-rolled RFC 6455 (SHA-1 handshake, masking, fragmentation
and close handling are exactly where hostile input bites).

## R2 Own message types, not `ClientMsg`
`ClientMsg` carries `[u8;32]` hashes, `PathBuf`s and ~100 daemon-only variants, and its schema hash gates the daemon
link. A browser API wants a small versioned set. New `client_api` types (`{"id","type",...}` JSON), mapped by the
orchestrator onto its existing list and a one-shot content read. Rejected: exposing the routed envelope (#1723),
which would leak daemon framing into the TypeScript surface.

## R3 Schema and types: `schemars` then `json-schema-to-typescript`
FR-009 wants a schema artifact and TS derived from it. `schemars` derive gives the schema from the Rust types;
`json-schema-to-typescript` is the standard schema-to-TS tool. A 40-line `micold-api-schema` bin writes the schema
(pattern: `micold-tokens-ts`). `scripts/api-types.sh` runs both; `--check` regenerates to a temp dir and diffs, and
prints the regenerate command. The schema half also has a cargo test, so the plain Rust gate catches it without Node.
The wire uses plain string ids; the client brands them at one seam (`wire.ts`), keeping #1735's `DaemonId` etc.
Rejected: `ts-rs`/`specta` (no schema artifact, two sources); hand-written TS (the drift this feature removes);
`typeshare` (no schema).

## R4 Classification table
`client_api::classify(&ClientRequest) -> Access { Read, Change }` is an exhaustive `match` with no wildcard, so a
new variant fails to compile; a test enumerates one value per kind (via a `KINDS` const checked against the schema's
`oneOf`) and asserts each is classified. Today every kind (ping, list_daemons, list_sessions, read_session,
subscribe) is `Read`; `Change` exists for #1734 and no kind uses it yet.

## R5 Resume retention: 1024 changes plus an epoch
A `ChangeLog` ring keeps the last 1024 changes with strictly increasing `seq` (u64). Every snapshot and subscribe
reply carries `epoch` (random u64 per orchestrator process). Resume with another epoch, or `since < oldest_seq - 1`,
answers `resync_needed`; the client then reloads a snapshot (FR-003, US2.2). A restart therefore resyncs rather than
mis-resuming. Restart of the poller (last subscriber leaves, a new one arrives): the feed keeps its last snapshot and log across a stop and
diffs against that snapshot on restart, so the changes made while stopped are appended in order and a resume across the gap
stays exact; the epoch changes only with the process. Boundaries: `since(epoch, 0)` on an empty log is an empty batch if
`next_seq == 1`, else `ResyncNeeded`; `n + 1 == oldest` is servable (nothing missing), `n + 1 < oldest` is not.
Rejected: a time window (needs a clock in tests, unbounded under bursts); persisting the log in the store
(a write per change for a case the spec already answers with resync). 1024 covers minutes of normal activity: a poll
emits only differences.

## R6 Reconnect backoff: the existing `ConnectionManager`
Reuse its exponential backoff with full jitter: base 500 ms, ceiling 15 s, state `unreachable` (its `offline`) after
8 attempts with a retry action; a policy close (1008, origin refusal) goes straight to `unreachable` with no retry
loop; 1001/1006/1011 and a missed pong reconnect and resume. A new `connecting` state is added for the first open.
Origin matching compares scheme, host and port (default ports 80/443 normalised); `Origin: null` and any malformed value are
refused (R11). Visibility: on `visibilitychange` to visible, the manager checks pong age and reconnects at once if stale (Safari
throttling). Rejected: fixed interval (tight loop on a down server); unbounded attempts (never tells the user).

## R7 Heartbeat
Client sends `{"type":"ping"}` every 20 s, server answers `pong`. Both sides declare dead at 40 s of silence
(FR-005). The server timer is a tokio `interval`; the client uses the injected `Clock`, so tests are deterministic.
Rejected: WebSocket protocol pings (browsers cannot see or send them).

## R8 Change feed: one poller diffing `gather`
Daemons emit no change events, and adding them is a daemon wire change out of scope. One `feed` task runs while at
least one subscriber exists: every 1 s it gathers daemons, projects, worktrees and sessions (existing `list::gather`,
2 s per-daemon deadline), diffs against the last snapshot, appends changes to the log and broadcasts. A link
register or loss pokes it for an immediate pass. Worst case for SC-006 is the 1 s tick plus the gather; typical well
under 2 s. Rejected: per-socket polling (N times the daemon load); push events from daemons (wire change).

## R9 `LinkDirectory` and the content read (a daemon wire change)
A trait in `orchestrator/directory.rs`: `list(kind)`, `read_session(daemon, project, session)`. `InProcess` wraps
`Arc<Shared>` and reuses `list::gather` and the conversation helper of `list.rs`. `ws.rs` and `feed.rs` depend only on
the trait, so FR-046 later replaces one impl.
Content read: `Attach` is not usable. The daemon refuses it with `ProjectBusy` while a desktop holds the project
(`state.rs:3099-3116`) and, once granted, would block that desktop. Decision (option a): a new read-only daemon request
`ClientMsg::ReadStructuredEvents { project, session }`, answered with the session's non-live history
(`DaemonMsg::StructuredEvents`, `live: false`) or a refusal (`NotFound`, `TerminalSession`); it takes no attachment, no
lock and no view. This is a recorded wire change: `SCHEMA_HASH` changes, so `PROTOCOL_VERSION` is bumped and the
orchestrator and daemons upgrade together (the existing handshake refuses a mismatch and reports `NeedsUpdate`). The
orchestrator opens a conversation as `list::ask` does, sends `Hello` then the request, projects events to `ContentEntry`,
ends the conversation. A terminal session answers `terminal_not_supported`; a daemon refusal maps to `not_found`.
Rejected: (b) attach-then-detach (ProjectBusy, blocks the desktop); (c) reading the daemon's event files directly (the
orchestrator has no filesystem access to remote daemons).
**This is the first task of M1**: a failing daemon integration test (`tests/orchestrator/read_structured_events.rs` plus a daemon
test for the new request) lands before the API types freeze, so a refusal or an unexpected cost surfaces first.

## R10 FR-012: the orchestrator sends the order computed
The #1735 contract already says the client never sorts and `merge_lists` defines the stable order in Rust, so
re-implementing it in TypeScript would create the second copy this FR fears. The snapshot arrays are in that order and
a change carries `after_key`. Parity is proven by shared vectors: a Rust test produces
`packages/protocol/tests/vectors/sidebar-order.json` (snapshot plus changes plus expected key order, check mode fails
on drift) and a jest test applies the changes with the client's reducer and asserts the same order. The vectors cover grouping and nesting (daemon, project, location, session, with a project holding several worktrees and a
session under each), a daemon going offline (its rows keep their position, sessions marked unreachable), and remove then
upsert of the same key (it returns to its `after_key` place, not the old one). The client reducer under test is
`packages/protocol/src/order.ts` (`applyChanges`). Rejected: porting the sort (two implementations, vectors needed anyway).

## R11 Local defaults
API `127.0.0.1:8730`, web client `127.0.0.1:8731` (8091 is the existing Playwright mock server; neither number appears
in the repo). Overrides `MICOLD_LOCAL_API_PORT`, `MICOLD_LOCAL_WEB_PORT`. The stack passes
`--allow-origin http://127.0.0.1:8731 --allow-origin http://localhost:8731`. The client's address comes from
`EXPO_PUBLIC_ORCHESTRATOR_URL` at export time, default `ws://127.0.0.1:8730/api/ws` (FR-016). In production the listener
is off; an operator sets `--api-addr` and `--allow-origin` (env `MICOLD_ORCHESTRATOR_API_ADDR`,
`MICOLD_ORCHESTRATOR_ALLOW_ORIGINS`, comma separated). Allow-list entries are exact origins including the port
(`http://127.0.0.1:8731` does not allow `http://127.0.0.1:9999`); `Origin: null` is always refused. A non-loopback `--api-addr` is refused until TLS (#1733).

## R12 Local stack and its test data
`scripts/local-stack.sh` (mise task `local-stack`): checks both ports free first (bash `/dev/tcp`; reports the
holder and exits, no second copy), makes a private data directory, builds the web export with the URL, starts the
orchestrator, a daemon configured to dial it (`[orchestrator_link]`, #1725) on a seeded catalog (a project, a worktree,
two sessions) and `e2e/serve.mjs` for the export, then `wait`s; a `trap` stops all three on interrupt. The Playwright
stack spec starts it via the same script and a fixture seed, so "a real session" needs no AI CLI. Rejected: creating
sessions through a real AI CLI in CI (needs credentials); a mock daemon (not the real stack).

## R13 — Local kind stack pins (M5, 2026-11-17)

Checked from the M5 session; GitHub's API and release pages were not reachable through its proxy,
so versions come from the Go module proxy (tags) and Docker Hub.

| Tool / image | Latest seen | Pinned | Why |
|---|---|---|---|
| kind | v0.33.0 (proxy.golang.org, 2026-08-26) | v0.30.0 (unchanged) | the bump needs the release's SHA256, published only on GitHub; follow-up |
| kindest/node | v1.37.0 (Docker Hub, 2026-08-26) | v1.34.0@sha256:ede5bb69… | the node image kind v0.30.0 ships for; bump with kind |
| helm | v3.22.1 (v3 line), v4.3.2 | v3.19.0 (unchanged) | get.helm.sh unreachable for the checksum; v4 is a major change; follow-up |
| kubectl | v1.37.2 (dl.k8s.io stable) | v1.34.0 (unchanged) | matches the node image's minor |
| postgres | 18.7-alpine3.24 (Docker Hub) | 18.7-alpine3.24@sha256:fa1b61f5… | official image; Bitnami's chart images left the free catalogue in 2025 |
| alpine/git (seed) | v2.54.1 | v2.54.1@sha256:f6629095… | sh and git for the seed init container |
| node (web image) | 22-alpine | 22-alpine@sha256:12f54caf… | the repo's node major |

Deviations found while implementing: the daemon's `[orchestrator_link]` is a local socket only, so
seeded daemons are sidecars of the orchestrator pod (not a separate Deployment), sharing
`/run/micold`. The client API refuses non-loopback addresses until TLS (#1733), so it binds the
pod's loopback and is reached by `kubectl port-forward` (which connects inside the pod), not by a
Service. The kind helpers of `e2e-kind.sh` are inline one-liners, so nothing was extracted to a
shared library.
