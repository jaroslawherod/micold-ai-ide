## Milestone M3 of 10 — PostgreSQL store for the orchestrator (specs/1948-orchestrator-postgres-store)

**Deliverable**: users, invites, tokens, serials, enrollments and audit entries survive a reopen on SQLite, and the audit log rejects update and delete.
**Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_suite && scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_scan`
**Tasks**: T021–T025 · **Satisfies**: US1 acceptance scenario 2; FR-002, FR-011

`SqlAuthStore` and `SqlAuditStore` over the M2 handle, run by the shared suite on SQLite. Every statement binds `tenant_id`; redeeming an invite or enrollment is one compare-and-set `UPDATE`; rotate, remove and consume are one transaction each. `rotate_refresh` on a revoked token commits the family revocation, then refuses. `redeem_invite` refuses an unknown redeeming user on SQL and in memory (contract updated).

## Agent review
- Review A (code-review, high, run inline by the unit): 1 MINOR (`put_refresh` dropped `revoked_at`), fixed.
- Review B (conformance): run by the orchestrator.

## Local gate
`scripts/gate.sh` passed at the pushed head (see the PR's CI). No visual change.

Refs #1948

🤖 Generated with [Claude Code](https://claude.ai/code/session_01NCdYvLiNsbPECB3AwZe75w)

https://claude.ai/code/session_01NCdYvLiNsbPECB3AwZe75w
