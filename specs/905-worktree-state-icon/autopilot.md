# Autopilot ledger — #905 worktree-state-icon

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #905 — "Worktree should show a state icon: idle/waiting, blocked or running". Each worktree entry should show an icon for its current state: Idle / waiting (no work in progress, or waiting for user input); Blocked (cannot proceed, e.g. needs approval, failed, waiting on a lock); Running (the session in the worktree is actively working). Expected: the icon is visible on the worktree in the list without opening it; state is derived from the worktree's session and updates live; icons are distinguishable by shape as well as colour. labels: enhancement (orchestrator added in-progress, flow:feature)
- **Kind**: feature
- **Effort**: default
- **Issue**: #905
- **Worktree branch**: claude/project-thread-h057j5
- **Started**: 2026-10-17
- **Phase**: spec
- **Next step**: review spec.md (rubric spec), then commit; next unit: clarify

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

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 2148471a6892a9acfe8441d5914d9cacd029f1e4:bfdc456e3ecbab4afa9c30045923409081433f23 | CHANGES: 1 MAJOR, 2 MINOR (all fixed) |
| Spec review | 2 | 047d8dca47a8242fd7e54bccd72425e87b1e87dc:bfdc456e3ecbab4afa9c30045923409081433f23 | pending |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
