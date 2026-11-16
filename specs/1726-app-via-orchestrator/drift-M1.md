# Plan drift, M1

Paths M1 changed that its tasks do not name, each with the reason.

- `.gitattributes`: marks the contract's example fixtures so line endings do not change the schema-hash test input.
- `crates/micold-core/src/protocol/codec.rs`: the raw frame codec the routed client frames need on the client side (shared by the hub and the tests).
- `crates/micold-daemon/src/orchestrator/hub.rs`: the route table's shared state, per-route queues and pumps (T007 names client.rs; the state they share lives here).
- `crates/micold-daemon/src/orchestrator/mod.rs`: declares the `hub` module and shares it through `Shared`.
- `crates/micold-daemon/Cargo.toml`: dev and runtime dependencies for the new modules and the route tests.
- `crates/micold-daemon/tests/orchestrator/not_hosted.rs`: the orchestrator-mode test that asserted clients are refused now asserts they are served.
- `crates/micold-core/tests/main.rs`: registers the new core test modules.
- `crates/micold-core/tests/protocol_roundtrip.rs`: round-trips for the new `OutcomeUnknown` and `StreamEnded` messages.
- `crates/micold-core/tests/schema_hash.rs`: the schema hash changes with the new wire messages.
- `crates/micold-core/tests/client_routing_contract.rs`: reads the contract's examples as test input (T001 names the contract; the test is how it stays true).
- `crates/micold-daemon/tests/orchestrator_route/link_drop.rs`: SC-006 and re-attach cases (T010 names one binary; one module per behaviour).
- `crates/micold-daemon/tests/orchestrator_route/outage.rs`: orchestrator-restart case, one module per behaviour.
- `crates/micold-daemon/tests/orchestrator_route/refusals.rs`: FR-005 refusal cases, one module per behaviour.
- `crates/micold-daemon/tests/orchestrator_route/requests.rs`: routed request cases, one module per behaviour.
- `crates/micold-daemon/tests/orchestrator_route/slow_client.rs`: FR-004 slow-client case, one module per behaviour.
- `crates/micold-daemon/tests/orchestrator_route/streams.rs`: ordered-stream and takeover cases, one module per behaviour.
- `crates/micold-daemon/tests/orchestrator_route/hub_queues.rs`: review round 1 regressions (queued answers kept, dropped answer `unknown`, numbering without a gap).
- `crates/micold-daemon/tests/orchestrator_route/link_full.rs`: review round 1 regression (a daemon that stops reading holds no other route).
- `docs/daemon.md`: the daemon now serves clients in orchestrator mode.
- `docs/user-guide/orchestrator-mode.md`: user-facing description of routed clients.
