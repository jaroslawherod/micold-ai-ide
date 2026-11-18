# Drift M2 (10f82e095..a1c9514b3)

- `crates/micold-core/src/client_api/mod.rs`, `crates/micold-core/src/client_api/content.rs`: module root and the content projection the messages of T005 need.
- `crates/micold-daemon/tests/orchestrator/ws_admission.rs`, `crates/micold-daemon/tests/orchestrator/ws_liveness.rs`, `crates/micold-daemon/tests/orchestrator/store_scan.rs`, `crates/micold-daemon/tests/orchestrator/support.rs`, `crates/micold-daemon/tests/orchestrator_route/main.rs`, `crates/micold-daemon/tests/support/history.rs`, `crates/micold-daemon/tests/support/linked_daemon.rs`: the integration tests went into the existing `tests/orchestrator` binary (no new top-level test executables); shared fixtures in support.
- MISSING `ws_heartbeat.rs`: T014's heartbeat and malformed-frame tests are in `ws_liveness.rs`.
- MISSING `http.rs`: named in T018 as one possible wiring site; the listener was wired in `orchestrator/mod.rs`.
- MISSING `data-model.md`: named as an input, not an output; no change was needed.
- `docs/user-guide/orchestrator-mode.md`: documents `--api-addr`.
