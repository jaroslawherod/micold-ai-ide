# Autopilot ledger — #1726 app-via-orchestrator

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1726 "feat: the desktop app talks only to the orchestrator (aggregated lists, routed ops and streams, starts it when absent)" (part of #1722; covers FR-005, FR-006, FR-007, FR-010; blocked by #1725, closed). Cloud overrides: no mise, gh unauthenticated, gate scripts/gate.sh, GitHub MCP for PRs; desktop change minimal (iced app sunsetting), prefer orchestrator/protocol-level tests; aim 3-4 milestones. labels: effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Supervised**: no
- **Issue**: #1726
- **Worktree branch**: claude/project-thread-v4iabl
- **Started**: 2026-11-16
- **Phase**: plan
- **Next step**: plan review round 1 (opus, rubric rubrics/plan.md) on snapshot 8487a8d58e89bb17100164e9cf76b5dfba6a0c1d:e458a16fca33bf98126ebd10ded77ccc18266133; then tasks unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | FR-002 per-daemon list deadline | 2 s, daemons asked in parallel | agent | SC-004 budget |
| 2 | clarify | Who writes `[orchestrator_link]` into provisioned daemons | this feature, the app on provision (FR-013) | agent | specs/1725-daemons-dial-orchestrator/spec.md#Assumptions |

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| spec | 1 | design | 7c25a28d9e4bec1417923f0dd050b8627f79bebd | f3b201b3e2dc11ea9d3cb4d365efcd62e7a71cae:b3f47d077d50c0a5339d29f608b5f351591a7057 | CLEAN (3 MINOR applied in clarify) |
| plan | 1 | design | e458a16fca33bf98126ebd10ded77ccc18266133 | 8487a8d58e89bb17100164e9cf76b5dfba6a0c1d:e458a16fca33bf98126ebd10ded77ccc18266133 | pending |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done
