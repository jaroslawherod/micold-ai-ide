# Data model

All in memory; nothing persisted. Types in `micold-core` unless noted.

| Entity | Fields | Rules |
|---|---|---|
| `RoutedHeader` | `target: Option<WireDaemonId>`, `conn: u64`, `seq: u64`, `req: Option<u64>` | `target` absent only for orchestrator-addressed frames (`OrchList`); `req` set on request frames and their answers (R5) |
| `ListKind` | `Projects \| Worktrees \| Sessions` | |
| `MissingDaemon` | `daemon: WireDaemonId`, `reason: MissingReason` | |
| `MissingReason` | `Offline \| NeedsUpdate \| Timeout \| Error(String)` | wire text `offline`, `needs-update`, `timeout`, `error: <message>` |
| `ListAnswer` | `req`, `rows: ListRows`, `missing: Vec<MissingDaemon>`, `no_daemons: bool` | every row carries its daemon ref; rows of two daemons never merged; a daemon is in rows or in missing, never both |
| `RouteTable` | `by_client: (ClientId, conn) -> (daemon, link_conn)`, `by_link: (daemon, link_conn) -> (ClientId, conn)`, `pending: (daemon, link_conn) -> set<req>` | bijective; link conns allocated per daemon, never reused while open; `drop_link(daemon)` returns every route and pending req to notify |
| `StreamEndReason` | `DaemonOffline \| SlowConsumer \| ClientGone` | |
| `RouteRefusal` | `NotFound \| NeedsUpdate \| Offline` | answered as `OperationError` kind `not_found` / refused with the reason (FR-005) |
| `VirtualConn` (daemon crate) | `conn: u64`, duplex half, outbound queue | one per link conn; ends when the link ends |
| `OrchestratorConn` (client crate) | the one connection, channel per actor `(target, conn)` | reconnect re-opens all channels |

State transitions, route: `opening` (first frame to a target) -> `open` -> `ended(reason)`. A
route ends on client drop (`ClientGone`, link conn closed towards the daemon), link drop
(`DaemonOffline`, client told), or a full queue (`SlowConsumer`).
