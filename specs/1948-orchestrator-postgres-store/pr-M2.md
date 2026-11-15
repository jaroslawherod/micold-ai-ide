## Milestone M2 of 10 — PostgreSQL store for the orchestrator (specs/1948-orchestrator-postgres-store)

**Deliverable**: a SQLite file (owner-only, WAL) migrates to the v1 schema and holds the registry across reopen, passing the shared registry suite.
**Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_migrations && scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_suite`
**Tasks**: T014–T020 · **Satisfies**: US1 scenario 2 (registry); FR-002, FR-003 (forward, transactional), FR-007

`StoreHandle` (private 2-worker runtime, `block_on` that answers `Unavailable` from an async task, `Drop` that never blocks), `migrations/sqlite/0001_orchestrator_schema.sql`, `MIGRATOR` and `run_forward`, and `SqlRegistryStore` (`load` ordered by `(tenant_id, id)`; `save` in one transaction; one-statement `upsert` and `remove`). The shared registry suite now runs on SQLite, and reopening drops the handle and opens the file again. Records are written with owner `unowned` (M10 carries the real owner). No user-guide change (label `docs-not-needed`).

## Agent review
- Review A (code-review, high; run by the unit, which had no subagent tool): 0 BLOCKER; fixed a non-ASCII or special-character data directory breaking the URL, `SQLITE_BUSY` reported as `Refused`, `save` upgrading a read snapshot, an existing loose file keeping its mode, `kind` encoded two ways; declined `Drop` skipping `pool.close()` on a blocking thread, one runtime per handle, and the version-0 schema error (M5).
- Review B (conformance, sonnet): pending, dispatched by the orchestrator.

## Local gate
GATE_LINE

Refs #1948

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01NCdYvPFAbHd4g2aXmV47ks
