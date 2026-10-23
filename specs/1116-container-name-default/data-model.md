# Data model: Container name default

No stored shape changes. `ContainerSettings.container_name: String` (`crates/micold-core/src/daemons.rs`) keeps holding the resolved name; old files load unchanged (FR-009).

| Item | Where | Rule |
|---|---|---|
| `DEFAULT_CONTAINER_NAME_MAX = 63` (new) | `micold-core/src/daemons.rs` | length cap of a defaulted name |
| `default_container_name(&str) -> Option<String>` (new) | same | FR-002; `Some` is non-empty, starts `[A-Za-z0-9]`, only `[A-Za-z0-9_.-]`, at most 63, no trailing `-_.` |
| `Draft::resolved_container_name(&self) -> Option<String>` (new) | `micold-client/src/features/daemons.rs` | trimmed `container_name` if non-empty, else `default_container_name(&self.name)` |
| `Draft::required_fields` | same | Container arm no longer lists `DaemonContainerName` |
| `Draft.container_name` | same | still the field as typed; empty means "default" |

State: none new. The resolved name is computed on demand (save, error, placeholder), never held.
