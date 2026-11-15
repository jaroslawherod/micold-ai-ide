# Autopilot ledger — #1948 orchestrator-postgres-store

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1948 "feat: PostgreSQL store for the orchestrator (sqlx, migrations, store traits; SQLite for local mode)" (part of #1722; FR-042, FR-046). labels: effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Supervised**: no
- **Issue**: #1948
- **Worktree branch**: claude/project-thread-ns6vcg
- **Started**: 2026-11-15
- **Phase**: plan (written, awaiting review)
- **Next step**: plan round-2 re-review (sonnet, fix diff since round-1 snapshot), then tasks unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Spec | 1 | spec | d3937c8969e34c08a0ce2f78559cde430b5c65f9 | 09cb2cfc0a611043c58abc651afcc426257803e6:d65a0b90b4bab89ddf4da88fcd33c56490018866 | CHANGES: 5 MAJOR 3 MINOR, all fixed |
| Spec | 2 | spec | 69983c1058338b7230c9e70066dc73eca578a606 | 1faf1b6e8adebd019dd1d2e6f8675ad1d5a15b12:d3937c8969e34c08a0ce2f78559cde430b5c65f9 | CLEAN (2 MINOR fixed, prose only) |
| Plan | 1 | plan | (pending commit) | 039972fca59028890bb8b22565c8ee7ab6bca81a:b1cd1965e0f01444b33b7d262dbda5e068ff3b06 | CHANGES: 1 BLOCKER 1 MAJOR 3 MINOR, all fixed |
| Plan | 2 | plan fix diff | (this commit) | SNAP2 | pending (orchestrator dispatches sonnet re-review) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Cloud thread: preflight failed only on `mise` (missing) and `gh` auth (known cloud setup); the run proceeded. Use `scripts/gate.sh` and the GitHub MCP tools.
- #1949 (constitution 2.0.0, PostgreSQL for network deployments) is not merged: nothing here depends on it; it must merge before a network deployment ships. Constitution Principle IV currently says local-first storage.
- Constitution conflict: it still says local-only storage and three OSes; jaro's Linux-only decision (#1897, 2026-11-14) and #1949 supersede. The spec keeps Linux-only and cites #1897.
- kube_real tests cannot run in cloud (no kind).
