# Plan drift, M3

Paths M3 changed that its tasks do not name, each with the reason.

- `crates/micold-core/src/connect.rs`: `Transport::Routed` (the actor's pipe) and `connect_or_spawn_with`, the loop `connect_or_spawn` already ran, with the connect and spawn steps as parameters (T018 "generalise").
- `crates/micold-core/src/spawn.rs`: `orchestrator_command` and `spawn_detached`, whose error names the program (US3.4).
- `crates/micold-core/src/daemons.rs`: `FileSettings::wire_id`, `new_wire_id`, `DaemonRegistry::ensure_wire_ids`; `set_file` keeps the wire id (T020 "registry entry").
- `crates/micold-client/src/features/settings.rs`: loaded registries get missing wire ids (T020 "generated on next connect").
- `crates/micold-client/src/lib.rs`, `crates/micold-client/Cargo.toml`: the new module; `bytes`, `tokio-util`, `futures-util` for its framing; `micold-daemon` as a dev-dependency for the linked daemon in T017's test.
- `Cargo.lock`: the dependency additions above.
- `docs/user-guide/orchestrator-mode.md`: T022's user guide page (the task names only `docs/user-guide/`).
- `crates/micold-client/tests/main.rs`: registers `orchestrator_single_connection`.
- `crates/micold-client/tests/golden/daemon_config/*.toml`, `tests/fixtures/daemon_config_delivery_host.golden.toml`, `tests/daemon_config_render.rs`, `tests/daemon_config_validate.rs`: every rendered file now ends with `[orchestrator_link] enabled = true`; provisioned kinds carry `[daemon] id`.

## T021: no change needed

`daemon_routing`, `catalog_two_daemons`, `notifications` and `attention_notify` (modules of `tests/main.rs`) open no socket: they drive `State`/`DaemonLinks` with an in-memory `Outbox` and fold snapshots, so they sit above the transport and are the same whether the actor's bytes go through a daemon socket or an orchestrator route. Run on the M3 head plus the round-1 fixes: `cargo test -p micold-client --test main -- daemon_routing:: catalog_two_daemons:: notifications:: attention_notify::` gave 40 passed, 0 failed. The routed path itself is exercised by `orchestrator_single_connection` (4 passed: one connection to a real linked daemon, two concurrent starts, a missing binary named, kill and re-handshake) and the `orchestrator_conn` unit tests (3 passed).

## Scope narrowed (not in the plan)

- Correction (M4 plan): container and pod daemons do have a route, the same #1725 `-R`; see research R7.
- Only the **host** daemon's actor is routed through the orchestrator. SSH, container and Kubernetes actors still hand over the `Connected` their provisioning step returns (a direct connection), though their rendered config now links them under their wire id. Routing them needs each provisioned daemon to reach the orchestrator socket (the SSH reverse forward of #1725 exists; container and pod placements have none), which is larger than the minimal desktop change this sunsetting app gets. Follow-up: route provisioned daemons by wire id once they are listed online.
- `OutcomeUnknown` and `StreamEnded` end the actor's route; the actor's existing reconnect then re-handshakes, and the fresh `Welcome` is the re-read of the target (T019). No per-request snapshot message was added.
