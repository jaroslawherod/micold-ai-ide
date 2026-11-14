# Plan drift, M5

| Path | Reason |
|---|---|
| `main.rs` (`crates/micold-daemon/src/main.rs`) | Untouched: its `orchestrator` arm already prints `micold-daemon: orchestrator: <error>` and exits 1, so the corrupt-file error from `serve` reaches stderr and a non-zero exit with no change (proven by `restart::a_corrupt_registry_fails_the_start_naming_the_file_and_leaves_it`, which runs the binary). |
| `orchestrator/registry.json` | A runtime file under the data directory, not a source path: written by `JsonRegistryStore`. |
| `crates/micold-daemon/src/orchestrator/link.rs` | Not named by T039: the snapshots each registry call returns are saved where the calls are made, through `Shared::update`/`Shared::save` defined in `mod.rs`. |
| `crates/micold-daemon/tests/orchestrator/support.rs` | Harness: `Orchestrator::start_in`/`stop`/`restart`/`external` for a restart on one directory and the spawned binary; `child_count` moved here from `not_hosted.rs` to be shared with `restart.rs`. |
| `crates/micold-daemon/src/orchestrator/mod.rs` | `serve` keeps link tasks in a `JoinSet`, so stopping the orchestrator stops its links with it; without it a stopped in-process orchestrator's links kept saving into the registry of the one restarted on the same directory. `run_with` takes the store. |
| `crates/micold-daemon/tests/orchestrator/not_hosted.rs` | `child_count` moved to `support.rs`, unchanged. |
| `crates/micold-core/src/orchestrator/mod.rs` | `pub mod store` and its re-exports. |
