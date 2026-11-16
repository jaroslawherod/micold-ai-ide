# Plan drift, M4

Recorded against `scripts/autopilot/plan-drift.sh specs/1948-orchestrator-postgres-store M4`.

## UNPLANNED (changed, named by no M4 task)

- `crates/micold-daemon/tests/orchestrator/store_suite.rs`: a `postgres` module runs the registry,
  auth and audit cases on `PostgresBackend` (T026 "every shared-suite case runs on it"); the case
  macros take an `Option` of a backend so a skipped backend returns early.
- `crates/micold-daemon/tests/orchestrator/store_scan.rs`: `postgres_refuses_a_raw_update_or_delete_of_the_audit_log`
  pins T028's trigger on PostgreSQL.
- `crates/micold-daemon/tests/orchestrator/no_outbound.rs`: the `db/` directory is no longer
  skipped; only its `.connect_with(` call sites are blanked (M2 review B F2, handover note).
- `crates/micold-daemon/migrations/postgres/0001_orchestrator_schema.sql` also refuses `TRUNCATE`
  (a statement trigger) and gives text columns `COLLATE "C"` so `ORDER BY id` is byte order, as in
  SQLite and the memory store (M3 review A note).
- `store_migrations.rs` gains error-arm cases beyond T027 (unparseable URL, missing and present
  `ca_file`) for diff coverage.

## Departures from plan.md

- `StoreHandle::open_postgres(url, ca_file, pool_size)` takes the URL text; reading `url_file` and
  `[database]` is M7/M8. An unparseable URL is `Unavailable { target: "postgres" }` with a fixed
  reason (the parser's text could echo the URL).
- `PgConnectOptions` is converted to `sqlx::Any` through `to_url_lossy()`, which carries the
  password; every driver error is passed through `redact` (URL and password cut) before it is kept.
- `db/registry.rs`, `db/auth.rs`, `db/audit.rs` (named by T029, reported MISSING): unchanged on
  purpose; every statement already runs on both dialects, which the `postgres` suite proves.
