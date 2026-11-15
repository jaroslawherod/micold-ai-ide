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
- **Phase**: spec
- **Next step**: dispatch spec review round 1 (autopilot-reviewer, opus), fix findings, then clarify unit

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
|---|---|---|---|---|---|
| Spec | 1 | spec.md | d65a0b90b4bab89ddf4da88fcd33c56490018866 | 09cb2cfc0a611043c58abc651afcc426257803e6:d65a0b90b4bab89ddf4da88fcd33c56490018866 | NOT RUN: unit has no Agent tool |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Spec written and committed. The round-1 spec review (opus, rubric spec.md) was NOT dispatched: this unit had no `Agent` tool. Next: dispatch it from the orchestrator, act on findings, then the clarify unit.

## Open escalation

None.

## Follow-ups not done

- Cloud thread: preflight failed only on `mise` (missing) and `gh` auth (known cloud setup); the run proceeded. Use `scripts/gate.sh` and the GitHub MCP tools.
- #1949 (constitution 2.0.0, PostgreSQL for network deployments) is not merged: nothing here depends on it; it must merge before a network deployment ships. Constitution Principle IV currently says local-first storage.
- kube_real tests cannot run in cloud (no kind).
