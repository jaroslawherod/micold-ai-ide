# Data Model: Container name minimum length

No stored field or schema changes. `ContainerSettings.container_name` stays a `String`.

| Item | Where | Rule |
|---|---|---|
| `CONTAINER_NAME_MIN: usize = 2` | `micold_core::daemons` (new) | fixed minimum |
| `container_name_long_enough(&str) -> bool` | `micold_core::daemons` (new) | `trim().chars().count() >= CONTAINER_NAME_MIN` |

Validation at Save (Container runtime): resolved name (typed trimmed, else default) is `None` gives "Enter a container name."; `Some` below the minimum gives the minimum message; otherwise the 1116 flow. Stored names are never revalidated on load. No state transitions.
