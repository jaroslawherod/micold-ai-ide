# Plan drift, M7

Recorded at close against `scripts/autopilot/plan-drift.sh specs/1948-orchestrator-postgres-store M7 3a74d3004~1 3a74d3004`.

## UNPLANNED (changed, named by no M7 task)

- `crates/micold-core/src/daemon_config/validate.rs`: the `[database]` checks (T042) hook into the validator; the `[orchestrator]` header finding was extracted to `at_table_header` so both tables share it.
- `crates/micold-daemon/src/config_check.rs`: `config check` reports the FR-009 backend-rule finding (T043); the task named the rule, not the CLI file.
- `crates/micold-daemon/tests/orchestrator/main.rs`: registers the new test module.
