# Plan drift, M2

Recorded against `scripts/autopilot/plan-drift.sh specs/1948-orchestrator-postgres-store M2`.

## UNPLANNED (changed, named by no M2 task)

- `crates/micold-daemon/tests/orchestrator/main.rs`: registers the `store_migrations` module (T014
  says "registered in `main.rs`").
- `crates/micold-daemon/tests/orchestrator/store_support.rs`: the `SqliteBackend` factory (T015).
- `crates/micold-daemon/tests/orchestrator/store_suite.rs`: the `sqlite` module running the registry
  cases (T015); no existing case changed.
- `crates/micold-daemon/src/orchestrator/mod.rs`: `pub mod db;` (T020).
- `crates/micold-daemon/tests/orchestrator/store_migrations.rs` (T014), `db/mod.rs` (T018),
  `db/migrate.rs` (T017), `db/registry.rs` (T019) are planned; the script's relative-path matching
  lists them as MISSING only because tasks.md names them in full.

- `crates/micold-daemon/migrations/sqlite/0001_orchestrator_schema.sql`: T016's file; the script's
  extension list lacks `.sql`. `contracts/schema.md` (read-only, under specs/) is the source it
  copies, unchanged.

## Departures from plan.md

- The `block_on` guard is not `Handle::try_current` (plan.md, *Trait shape*): that is `Ok` on a
  `spawn_blocking` thread too, the supported path. `run_on` catches tokio's refusal to block inside
  an async task and answers `Unavailable` instead.
- `run_forward`'s retry fires on any `MigrateError::Execute`, not only busy or "already exists";
  a genuine failure fails the same way twice, so the retry costs one repeated statement.
- `migrate::check_not_newer` is M5 (T035), not here.
- `run_forward` retries only an "already exists", "locked" or "busy" execute error (review A).
- `StoreHandle::open_sqlite` also tightens an existing database file to 0600 (review A), beyond T018.
- Review A findings declined: `Drop` skipping `pool.close()` on a `spawn_blocking` thread (a SQLite
  WAL left by a hard stop is recovered on the next open; the runtime-context probe cannot tell the
  thread kinds apart); one runtime per handle (the plan's decision, `Drop` and `run_on` are its
  cost); `Failed { version: 0 }` and `VersionMissing` becoming `DatabaseNewer` (M5, T035).
