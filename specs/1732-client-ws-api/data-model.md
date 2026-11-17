# Data model: client WebSocket API (#1732)

All wire types are Rust structs in `micold_core::client_api`, `serde` + `schemars::JsonSchema`, snake_case JSON.

## Messages
- **ClientRequest** `{ id: u64, type, ... }`, `type` one of `ping`, `list_daemons`, `list_sessions`,
  `read_session { daemon, project, session }`, `subscribe { since: Option<{epoch, seq}> }`.
  `classify(&ClientRequest) -> Access { Read | Change }`: exhaustive match, no wildcard.
- **ServerMessage** `{ id: Option<u64>, type, ... }`: `pong`, `daemons { rows }`, `sessions { rows }`,
  `session_content { session, entries }`, `subscribed { epoch, seq, snapshot: Option<Snapshot> }`,
  `changes { epoch, changes }` (unsolicited, `id` absent), `resync_needed { epoch, seq, reason }`,
  `error { code, message }`.
- **ErrorCode**: `malformed`, `unknown_type`, `terminal_not_supported`, `not_found`, `daemon_unreachable`,
  `timeout`, `internal`.

## Entities (match the #1735 web model; plain string ids on the wire)
- **Snapshot** `{ epoch, seq, daemons, projects, locations, sessions }`, arrays in the orchestrator's stable order.
- **Daemon** `{ id, name, status: online | offline | needs_update }`; **Project** `{ id, daemon_id, name }`;
  **Location** `{ id, project_id, name, branch, is_default }`;
  **Session** `{ id, location_id, name, kind: structured | terminal, state: working | waiting | idle | error | stopped,
  reachable: bool }` (`reachable` false while its daemon is offline, FR-014a).
- **Change** `{ seq, kind: session_state | daemon_status | upsert | remove, ... , after_key? }` (shape of the #1735 `Change`).
- **ContentEntry** `{ seq, role: user | assistant | system, kind: text | tool_call | tool_result | status, text }`,
  text truncated to 64 KiB per entry with a `truncated` flag; no terminal bytes.

## ChangeLog (core, in memory)
`{ epoch: u64, next_seq: u64, ring: VecDeque<Change> (cap 1024) }`.
- `append(changes) -> seq range`; seq strictly increasing from 1 per epoch.
- `since(epoch, n) -> Changes(Vec) | ResyncNeeded`: `ResyncNeeded` when `epoch` differs or `n + 1 < oldest`;
  `n == last` returns an empty batch; `n > last` is `ResyncNeeded` (client ahead of this orchestrator).
- Subscribe with `since: None` returns a fresh `Snapshot` at the current seq.

## Diff (core, pure)
`diff(old: &Snapshot, new: &Snapshot) -> Vec<ChangeKind>`: removed rows first, then upserts in new order with
`after_key`, state and status changes as the small kinds. Applying the output to `old` with the client reducer yields
`new` (property test, shared vectors).

## ClientConnection (server, per socket)
`{ last_message_at, subscribed: bool, cursor: Seq }`. Dropped at 40 s silence. Two tabs are two connections.

## OriginAllowList
`{ origins: Vec<String> }`, exact, case-insensitive scheme and host match against the `Origin` header; no wildcard.
Absent `Origin` is accepted only when the peer address is loopback.

## Web client state
`ConnectionState = connecting | connected | reconnecting{attempt} | offline` (`offline` shows the "unreachable"
copy); `lastSeq`, `epoch`, stored in memory only. On `resync_needed` the store replaces its snapshot.
