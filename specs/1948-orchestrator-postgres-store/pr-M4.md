## Milestone M4 of 10 — PostgreSQL store for the orchestrator (specs/1948-orchestrator-postgres-store)

**Deliverable**: the shared suite passes on PostgreSQL and CI runs it against a Postgres service container that fails, not skips, when missing.
**Verify**: `MICOLD_TEST_POSTGRES_URL=postgres://… scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_`; CI `test` job green
**Tasks**: T026–T031 · **Satisfies**: US1 acceptance scenario 1; FR-012, FR-016, SC-004

`StoreHandle::open_postgres(url, ca_file, pool_size)` over the same `sqlx::Any` queries: `migrations/postgres/0001_orchestrator_schema.sql` (same versions and descriptions as SQLite, checked by a test; `BIGSERIAL` seq; text keys `COLLATE "C"` so `ORDER BY id` matches the memory order; a plpgsql trigger refuses `UPDATE`, `DELETE` and `TRUNCATE` on `audit_log`; no `GRANT`). With `ca_file` the connection is `verify-full` against it, without it `prefer`. An unreachable server is `StoreError::Unavailable` naming `host:port`, with the URL and password cut from the driver cause. No query needed a dialect change.

Tests: a `PostgresBackend` factory gives each case its own database (created and dropped); the `postgres` module runs every shared-suite case. Unset `MICOLD_TEST_POSTGRES_URL` prints `skipped: …`; `MICOLD_REQUIRE_POSTGRES=1` makes that a failure. CI's `test` job gains a `postgres:17` service container and both variables; `docs/development/ci-pipeline.md` describes it. The no_outbound scan no longer skips `db/`: only its `.connect_with(` call sites are blanked (M2 review B F2).

## Agent review
- Review A (code-review, high, run inline by the unit: no Agent tool): 1 MINOR (doc comments over 100 columns) fixed; noted: `redact` of a one-character password also masks that character elsewhere in the cause (safe, only less readable).
- Review B (conformance): run by the orchestrator.

## Local gate
`scripts/gate.sh` passed at the pushed head with `MICOLD_TEST_POSTGRES_URL` on a local PostgreSQL 16 and `MICOLD_REQUIRE_POSTGRES=1`. No visual change.

Refs #1948

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01NCdYvKZJu5VAvGJpoSi72Y
