# Plan drift, M9

Recorded at close against `scripts/autopilot/plan-drift.sh specs/1948-orchestrator-postgres-store M9 83926c52f~1 83926c52f`.

## UNPLANNED (changed, named by no M9 task)

- `crates/micold-daemon/src/orchestrator/db/{mod,registry}.rs`: `import` module registered; `upsert_row` made `pub(super)` so the import writes rows the same way.
- `crates/micold-daemon/tests/orchestrator/store_config.rs`: M8's "unimported registry.json refuses" test now expects the import (M9 replaces that refusal).
- `crates/micold-daemon/tests/orchestrator/main.rs`: registers the new test module.

## MISSING

- `registry.json`: a data file named by the tasks, not a repository path; path-matching artefact.
