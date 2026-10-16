# Autopilot ledger — #837 issue-relationship-metadata

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #837 "Show all GitHub issue metadata (blocked by, blocking, parent, sub-issues…) when a worktree is created"; labels: enhancement, effort:high, flow:feature (orchestrator added in-progress)
- **Kind**: feature
- **Effort**: high
- **Issue**: #837
- **Worktree branch**: claude/project-thread-tw6sph
- **Started**: 2026-10-16
- **Phase**: plan
- **Next step**: plan unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

- Clarify: US2 scenario 6, form only, the sidebar row is unchanged (option A). Default chosen by the orchestrator, pending user confirmation. If the user picks B or C, the orchestrator continues the run with that change.
- Clarify: FR-001, the form reads the details when the issue is picked; FR-011, a github_issue tool call waits up to 10 s for them (agent-resolved).
- Clarify round 2: no critical ambiguities.
| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 939e5694d83a882c00de3c9ff98235ef44a95c5d:8a04f4206330fa7552531e4694e2244b186d345f | CLEAN: 3 MINOR, all fixed (F1, F3 prose only; F2 added FR-011/SC-005 tool-call bound, no new round needed after CLEAN) |
| Plan review | 1 | a8e72b879aa633e947404ec746d63f19b023f49e:43b70bc145326616180860e5edd7b9dc6af331d9 | pending |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None. (Sidebar question: option A applied as orchestrator default, pending user confirmation; see Decisions.)

## Follow-ups not done

None.
