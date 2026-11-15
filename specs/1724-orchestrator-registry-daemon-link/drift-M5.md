# Plan drift, M5

| Path | Reason |
|---|---|
| `main.rs` (`crates/micold-daemon/src/main.rs`) | Untouched: its `orchestrator` arm already prints `micold-daemon: orchestrator: <error>` and exits 1, so the corrupt-file error from `serve` reaches stderr and a non-zero exit with no change (proven by `restart::a_corrupt_registry_fails_the_start_naming_the_file_and_leaves_it`, which runs the binary). |
| `orchestrator/registry.json` | A runtime file under the data directory, not a source path: written by `JsonRegistryStore`. |
| `crates/micold-daemon/src/orchestrator/link.rs` | Not named by T039: the snapshots each registry call returns are saved where the calls are made, through `Shared::update`/`Shared::save` defined in `mod.rs`. |
| `crates/micold-daemon/tests/orchestrator/support.rs` | Harness: `Orchestrator::start_in`/`stop`/`restart`/`external` for a restart on one directory and the spawned binary; `child_count` moved here from `not_hosted.rs` to be shared with `restart.rs`; a `CHILDREN` lock (`children_quiet`) held by the tests that spawn a process or count children (Review A round 1 F1). |
| `crates/micold-daemon/src/orchestrator/mod.rs`, `save.rs` | Review A round 1 F2/F3: plan.md says the caller saves "after unlocking through one saver"; the write now runs on one dedicated thread (`save.rs`, `SaveWorker`) that a link hands its snapshot to, and snapshots handed over during a write are coalesced to the newest. So no fsync runs on a runtime worker, and a ping answer that moves only `last_seen` is still persisted (FR-004 keeps last seen in the store) without blocking accept or a link's timers. `serve` keeps link tasks in a `JoinSet` and owns the worker, dropped after it: stopping the orchestrator aborts the links, writes what is waiting and joins the thread, so nothing writes the registry file afterwards. `run_with` takes the store. |
| `crates/micold-daemon/tests/orchestrator/not_hosted.rs` | `child_count` moved to `support.rs`; `no_child_process_is_spawned` holds the `CHILDREN` lock. |
| `crates/micold-core/src/orchestrator/mod.rs` | `pub mod store` and its re-exports. |

Full path for the `save.rs` row above (close check): `crates/micold-daemon/src/orchestrator/save.rs` is the dedicated saving thread (Review A round 1 F2/F3).
