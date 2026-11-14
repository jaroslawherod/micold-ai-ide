# Autopilot ledger — #1815 supervisor-question-routing

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: autopilot supervisor M4: route session questions to the user in one batch (issue #1815); labels: enhancement, in-progress, flow:feature
- **Kind**: feature
- **Effort**: default
- **Supervised**: no
- **Issue**: #1815
- **Worktree branch**: claude/project-thread-0lx36q
- **Runs as**: the orchestrator dispatches units with the `autopilot-unit` type in a cloud thread (no `mise`, no `gh`; no PR for the spec unit, pushed to the harness branch only)
- **Started**: 2026-11-14
- **Phase**: spec
- **Next step**: spec review round 1 (the orchestrator dispatches an autopilot-reviewer with rubrics/spec.md; the spec unit had no Agent tool), then clarify unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | #1858 (parent field, sibling visibility) lives only in open PR #1873, not on main: depend on it? | No. Spec batching, repo-evidence resolution, answer routing and context cap in full against main. Spec the upward relay as conditional: applies when the daemon reports the supervisor's parent; no parent reported means top tier, ask the user. | orchestrator | spec.md Background, US4, FR-012, FR-013 |

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
|---|---|---|---|---|---|

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Spec review round 1 not run: the spec unit had no `Agent` tool. Snapshot before review: `861681bf64f50b5ce0c1aa81af07c06480367738:fa0e41a737c21651652a6954264cefd2d07b1065`.
