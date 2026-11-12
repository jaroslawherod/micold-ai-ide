# Plan drift, M1

| Path | Reason |
|---|---|
| `crates/micold-daemon/src/catalog.rs` | Extracts the free `wire_settings(&Settings)` from `Catalog::settings_wire`: the orchestrator's Welcome needs the default settings without a `Catalog`. Behaviour of the daemon path unchanged (SC-006, the existing daemon suites pass). |
| `crates/micold-core/src/daemon_config/tests.rs` | The `Kind` serialization round-trip test of T003. |
| `crates/micold-daemon/tests/orchestrator/{heartbeat,netns,no_outbound,one_link_per_id,restart,version_mismatch}.rs` | One-line stubs T010 creates so `[[test]] orchestrator` (T002) and `every_test_file_is_wired` compile; later milestones fill them. |

`MISSING contracts/link.md`, `MISSING data-model.md`, `MISSING every_test_file_is_wired.rs` are read-only references in task text, not planned edits.
