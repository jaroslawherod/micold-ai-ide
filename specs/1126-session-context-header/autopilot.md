# Autopilot ledger — #1126 session-context-header

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start work on issue #1126 "Layout: worktree context header and session tabs above the terminal" (app redesign issues, started by dependency order); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1126
- **Worktree branch**: claude/project-thread-y28cqn
- **Started**: 2026-10-24
- **Phase**: clarify
- **Next step**: clarify unit (continued with the user's answer): apply the Open escalation's answer to spec.md US2 scenario 3, FR-008 to FR-010, Key Entities (Tab), SC-004; record it under Clarifications; run the next clarify round

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
| Spec | 1 | 0c554b9b6b3c47d8f964a5440a77a10f44444bf6:3c7db2eb08425cc9cab3f4001be9e64c03c8ccd2 | CHANGES: 4 MAJOR, 3 MINOR; all fixed |
| Spec | 2 | d6925d167b7f9c841828824458dcbad6a7efa800:3c7db2eb08425cc9cab3f4001be9e64c03c8ccd2 | CLEAN (sonnet); 1 MINOR fixed, prose only |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1, category 1 (product decision the repo does not settle).

**Question:** What do the tabs in the strip above the terminal hold, and what does its "New session" control do? (spec.md US2 scenario 3, FR-008 to FR-010)

Evidence: issue #1126 says "session tab strip (main/helper, New session)"; the mockup (`Main.dc.html`, board "Proposal: session view") draws tabs "claude · main", "helper · tests" and "New session", so the tabs are sessions. Today's bottom bar (feature 027, `docs/user-guide/worktrees-and-sessions.md#the-tab-strip`) holds the shown session's shell instances, a "+" and the pinned AI tab, and FR-009 says nothing it offered is lost. Neither the issue nor the mockup says where shell instances go. The sidebar's **Start another session...** (`worktrees-and-sessions.md#starting-switching-and-closing-sessions`, line ~937) starts a helper after a confirmation.

- **A (Recommended)**: tabs are the location's sessions (main first, then helpers); the shown session's tab stands for its AI conversation (today's AI tab), and the shown session's shell instances follow it as tabs with the "+" after them. "New session" runs the existing **Start another session...** action, confirmation included. Why: matches the issue and mockup and keeps every bottom-bar control (FR-009) in one row.
- **B**: tabs are the location's sessions only; shell instances and the "+" move into each pane's own header. Changes the pane headers, which the spec assumes unchanged.
- **C**: tabs stay today's (AI tab + shell instances + "+"); the control opens a shell instance and sessions stay switched from the sidebar. Contradicts the issue's "main/helper, New session".

## Follow-ups not done

None.
