# Plan drift, M8

Recorded at close against `scripts/autopilot/plan-drift.sh specs/1948-orchestrator-postgres-store M8 2243de02d~1 2243de02d`.

## UNPLANNED (changed, named by no M8 task)

- `crates/micold-daemon/src/orchestrator/save.rs`: the save loop prints a database store's error without double-prefixing the JSON store's own text (store-agnostic saver, T047).
- `crates/micold-daemon/tests/orchestrator/{restart,support}.rs`: restart tests move from `registry.json` to the SQLite file of local mode; `support` gains `start_with(store)` so tests start on a chosen store.
- `crates/micold-daemon/tests/orchestrator/main.rs`: registers the new test module.
- `docs/SUMMARY.md`: links the M8 guide page (guide task).
