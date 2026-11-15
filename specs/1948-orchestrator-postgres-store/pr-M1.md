## Milestone M1 of 10 — PostgreSQL store for the orchestrator (specs/1948-orchestrator-postgres-store)

**Deliverable**: `RegistryStore` has per-record `upsert` and `remove`, `StoreError` has `Unavailable`, `Refused` and `Schema`, `AuthStore` and `AuditStore` exist in `micold-core` with in-memory implementations, and one shared suite passes on the in-memory and JSON stores, tenant isolation included; the sqlx spike is recorded in research.md R2.
**Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_suite`, then `cargo test -p micold-core orchestrator::store`, `orchestrator::auth`, `orchestrator::audit`.
**Tasks**: T001–T013 · **Satisfies**: US1 scenario 3, US6 scenario 2 (auth and audit); FR-001, FR-002, FR-006 (types), FR-007, FR-015

Design notes: records that exist before any user get owner `unowned` and the first owner user takes them over (M10); the registry store covers all tenants, the auth and audit stores serve one each; the network-bind check refusing SQLite is deferred to #1732; a network PostgreSQL deployment waits on #1949. No user-guide change (label `docs-not-needed`).

Spike (T001): `sqlx` 0.8.6 runs `$n`, `ON CONFLICT`, `RETURNING` and `sqlx::migrate!` over `Any` on SQLite and PostgreSQL 16; `sqlx::migrate!` needs the `macros` feature (plan corrected); two concurrent `Migrator::run` on one SQLite file can fail, so the plan's retry rule stands.

## Agent review
- Review A (code-review, high; run by the unit, which had no subagent tool): 0 BLOCKER or MAJOR, 1 MINOR recorded as a follow-up for M2 (a SQL `rotate_refresh` must commit the family revocation before returning `Refused`).
- Review B (conformance, sonnet): pending, dispatched by the orchestrator.

## Local gate
`scripts/gate.sh` passed at 6d4d88cad (13123 tests, script tests). It was red twice before on script tests that passed alone (`check-assertions-frozen`, `autopilot.test.sh` facts cases) and green on the third run of the same tree. No macOS, sandbox or visual check: nothing in this diff touches them.

Refs #1948

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01NCdYvM45YiL86eGn3fUWzr
