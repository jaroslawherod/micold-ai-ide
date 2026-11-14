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
- **Phase**: plan
- **Next step**: plan review round 2 (reviewer pending), then tasks unit

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
| --- | --- | --- | --- | --- | --- |
| Review spec | 1 | spec | 861681bf64f50b5ce0c1aa81af07c06480367738 | 861681bf64f50b5ce0c1aa81af07c06480367738:fa0e41a737c21651652a6954264cefd2d07b1065 | CHANGES: 4 MAJOR, 2 MINOR, all fixed (round 2 pending) |
| Review spec | 2 | spec | b4f7aad4441e4a553d5eb93fdca3e56b77f1bf20 | 798b505d3b1d77b4744a9e98db61a4b244bfbe7f:1a3b3ce1940b26d815cc6cdc18e61163e7f2b1af | CHANGES: 1 MAJOR, 1 MINOR; fixed, prose only (out-of-scope wording), no round 3 |
| Review plan | 1 | plan | 2fb8d78017e17ab256129910edceaa365b11491e | 1ddc4daf07dab41e2627303773d39f3db679b3b8:2fb8d78017e17ab256129910edceaa365b11491e | CHANGES: 3 MAJOR, 3 MINOR, all fixed (round 2 pending) |
| Review plan | 2 | plan | f776b812d17e464ba995cf3257722a28600bc394 | 6611638df9e715ff6514de81ce5b274a5a25e502:f776b812d17e464ba995cf3257722a28600bc394 | CLEAN |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
