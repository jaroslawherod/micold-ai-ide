# Drift: M2 (T014–T016)

Files M2 changed that tasks.md T014–T016 do not name (they name `client.rs` and the `orchestrator_route/` directory only):

| Path | Why |
|---|---|
| `crates/micold-daemon/src/orchestrator/list.rs` | the `OrchList` fan-out (deadline, merge) lives in its own module beside `client.rs` |
| `crates/micold-daemon/src/orchestrator/mod.rs` | declares the `list` module |
| `crates/micold-daemon/src/orchestrator/link.rs` | exposes what the fan-out needs from a link (ask without a route, needs-update/offline naming) |
| `crates/micold-daemon/tests/orchestrator_route/main.rs` | registers the `lists` and `bench` modules |
| `crates/micold-daemon/tests/orchestrator_route/lists.rs` | T014's case `lists`, one module per behaviour |
| `crates/micold-daemon/tests/orchestrator_route/bench.rs` | T016's case `bench` |
