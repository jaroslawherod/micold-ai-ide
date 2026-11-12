# Plan drift, M2

| Path | Reason |
|---|---|
| `crates/micold-client/src/shell/daemon_sync.rs` | T022 names `crates/micold-client/src/`: the one client match on `DaemonMsg` is here. It gets an explicit `LinkPing` arm and the test `a_link_ping_is_ignored`. The existing `_ => {}` wildcard stays, so the explicit arm documents intent but does not make a future variant a compile error. `server.rs` is the same. |
| `crates/micold-core/src/orchestrator/mod.rs` | Module wiring for `liveness.rs` (T023). |
| `crates/micold-core/tests/protocol_roundtrip.rs` | The protocol sample lists and the version test for `LinkPing`/`LinkPong` and 81 (T018, T021). |
| `crates/micold-daemon/src/orchestrator/mod.rs` | `Shared::now`, the injected clock the link loop's timer reads (T024). |
