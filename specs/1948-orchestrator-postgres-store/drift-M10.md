# Plan drift, M10

Recorded at close against `scripts/autopilot/plan-drift.sh specs/1948-orchestrator-postgres-store M10 8005293fe~1 8005293fe`.

## UNPLANNED (changed, named by no M10 task)

- `crates/micold-core/src/orchestrator/mod.rs`: re-exports the new `OwnerId`.
- `crates/micold-daemon/tests/orchestrator/store_import.rs`: import keeps a file's owner and marks a missing one unowned (T056 behaviour reached through the import path).

## MISSING

- `registry.json`: a data file named by the tasks, not a repository path; path-matching artefact.

## Departures from tasks.md

- Owner on conflict: an `unowned` record never clears a stored owner, an owned one sets it (ledger *Decisions*, M10, review A-F1), refining plan.md's "insert only".
