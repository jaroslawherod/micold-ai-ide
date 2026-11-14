# Plan drift, M2

| Path | Reason |
|---|---|
| `crates/micold-client/src/shell/daemon_sync.rs` | T022 names `crates/micold-client/src/`: the one client match on `DaemonMsg` is here. It gets an explicit `LinkPing` arm and the test `a_link_ping_is_ignored`. The existing `_ => {}` wildcard stays, so the explicit arm documents intent but does not make a future variant a compile error. `server.rs` is the same. |
| `crates/micold-core/src/orchestrator/mod.rs` | Module wiring for `liveness.rs` (T023). |
| `crates/micold-core/tests/protocol_roundtrip.rs` | The protocol sample lists and the version test for `LinkPing`/`LinkPong` and 81 (T018, T021). |
| `crates/micold-daemon/src/orchestrator/mod.rs` | `Shared::now`, the injected clock the link loop's timer reads (T024). |

## T022 red-first cannot hold

Both matches keep their `_ => {}` wildcard (changing them to exhaustive would break every future
variant addition outside this feature), so no test can fail for a missing explicit arm: the tests
`a_link_ping_is_ignored` and `a_stray_link_pong_is_ignored_and_the_connection_stays_up` are
regression guards, not red-first tests. The "Red first" clause of T022 is waived for that reason.

## Heartbeat cadence

`steady` checks on a real-time `tokio::time::interval(CHECK_INTERVAL)`; only the deadline reads the
injected clock (`Shared::now`). The heartbeat tests wait about 2 s of real time.
