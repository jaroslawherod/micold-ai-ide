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

## Departures from plan.md

- The `block_on` guard is not `Handle::try_current` (plan.md, *Trait shape*): that is `Ok` on a
  `spawn_blocking` thread too, the supported path. `run_on` catches tokio's refusal to block inside
  an async task and answers `Unavailable` instead.
- `run_forward`'s retry fires on any `MigrateError::Execute`, not only busy or "already exists";
  a genuine failure fails the same way twice, so the retry costs one repeated statement.
- `migrate::check_not_newer` is M5 (T035), not here.
