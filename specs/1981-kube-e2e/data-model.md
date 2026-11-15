# Data model (#1981)

No new persisted data. Configuration entities:

| Entity | Fields | Rules |
|---|---|---|
| `OrchestratorConfig` (new, `crates/micold-daemon/src/orchestrator/config.rs`) | `data_dir: PathBuf`, `socket: PathBuf`, `ops_addr: Option<SocketAddr>` | flag > env > default; `ops_addr` default `None`; invalid address is an error naming the source (flag or variable) |
| `Readiness` (new, `ops.rs`) | `registry_loaded: AtomicBool`, `link_bound: AtomicBool` | `/readyz` 200 iff both true; flags only go false to true |
| Chart values | see [contracts/chart-values.md](contracts/chart-values.md) | `state.mode` enum `file`; `database.existingSecret` reserved |
| Harness environment | see [contracts/harness-env.md](contracts/harness-env.md) | |

The registry file (`<data-dir>/orchestrator/registry.json`) is unchanged and owned by `JsonRegistryStore`.
