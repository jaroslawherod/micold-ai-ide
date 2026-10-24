# Autopilot ledger — #1126 session-context-header

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start work on issue #1126 "Layout: worktree context header and session tabs above the terminal" (app redesign issues, started by dependency order); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1126
- **Worktree branch**: claude/project-thread-y28cqn
- **Started**: 2026-10-24
- **Phase**: plan
- **Next step**: plan review round 1 (rubrics/plan.md) — not run: this plan unit had no Agent tool to dispatch a fresh reviewer; the orchestrator dispatches it on snapshot 51263958474f497c08284a0df18662a4cf866d16:ceb78663833a922d96dfbaefd6548051373a2ac8, then tasks unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

- Plan (2026-10-24): header facts from the Overview card's own `CardModel` (R2); counts from the Changes view's list reader in the background for the shown location (R5, R6); strip = session tabs + shown session's shells and "+", status/restart, New session, splits at the right; keyed row for presses (R7–R10); header stays above the open Changes view, strip does not (R12). Edits confined to `ui/mod.rs`'s active-session/Changes branch for #1124/#1125 rebases.
- Clarify round 1 (strip tab contents, US2 sc. 3): option A, session tabs (main, then helpers) with the shown session's tab as its AI conversation, its shell tabs and "+" after it; New session runs **Start another session...** with its confirmation. Orchestrator's provisional pick of the recommended option, user decision pending (decision card). Applied to spec.md Terms, US2 sc. 3/7, FR-008 to FR-010, Key Entities (Tab), SC-004.
- Clarify round 2: session tab order and press behaviour agent-resolved from worktrees-and-sessions.md; no new requirement, clarify ends.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 0c554b9b6b3c47d8f964a5440a77a10f44444bf6:3c7db2eb08425cc9cab3f4001be9e64c03c8ccd2 | CHANGES: 4 MAJOR, 3 MINOR; all fixed |
| Spec | 2 | d6925d167b7f9c841828824458dcbad6a7efa800:3c7db2eb08425cc9cab3f4001be9e64c03c8ccd2 | CLEAN (sonnet); 1 MINOR fixed, prose only |
| Plan | 1 | 51263958474f497c08284a0df18662a4cf866d16:ceb78663833a922d96dfbaefd6548051373a2ac8 | pending: no Agent tool in the plan unit |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Plan unit, 2026-10-24. Done: plan.md, research.md (R1–R14), data-model.md, contracts/{context-header,session-area}.md, quickstart.md, committed 05558f7f. Next: plan review round 1 with rubrics/plan.md on snapshot 51263958474f497c08284a0df18662a4cf866d16:ceb78663833a922d96dfbaefd6548051373a2ac8 (this unit had no Agent tool to dispatch it), fix findings, then set Next step to `tasks unit`. No open findings, no PR.

## Open escalation

None.

## Follow-ups not done

None.
