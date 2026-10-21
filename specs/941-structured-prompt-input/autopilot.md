# Autopilot ledger — #941 structured-prompt-input

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 941 — issue #941 "Structured session view: prompt input, interrupt and queued messages"; labels: enhancement, flow:feature, in-progress
- **Kind**: feature
- **Effort**: default
- **Issue**: #941
- **Worktree branch**: claude/project-thread-ik47pq
- **Started**: 2026-10-21
- **Phase**: clarify
- **Next step**: clarify round 1: apply the user's answers to the two escalated questions (US4 scenarios 4 and 5, FR-022), then run round 2

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| 1 | clarify r1 | Is the queue the session's or each window's? | The session's, held by the service, shared by every window, kept when a window closes | agent | specs/938-structured-session-mode/spec.md#Edge Cases; spec.md#Constraints, #FR-009; constitution II |
| 2 | clarify r1 | Does the queue survive a service restart? | As the event history: saved under Save terminal history, memory only otherwise | agent | specs/938-structured-session-mode/spec.md#Clarifications Q3 |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | 124c8bf01142dc504b1089c95d75b80d67980571:65cefbbe38d05d3054bce51ba4a2c4b3ab37a9e9 | CHANGES: 3 MAJOR, 2 MINOR (all fixed) |
| Review spec | 2 | 7c8c2f791befe95ba7718b63d4cde6c4a169439f:65cefbbe38d05d3054bce51ba4a2c4b3ab37a9e9 | CLEAN (sonnet) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1 (category 1, product decision):

1. After a session restarts with messages still queued, does delivery resume on its own?
   - (Recommended) Hold until the user releases it: the queue stays on hold, the pane offers one
     "Send queued" control. A restart follows a stop, a crash or a service restart (#938 restores
     sessions unattended), so the user may not be watching or may no longer want those messages.
   - Resume automatically once the session is running and idle.
2. What does interrupting a turn do to the queue?
   - (Recommended) Keep it, put on hold until the user releases it (same control as Q1). An
     interrupt means Claude went the wrong way, so the next queued message may no longer fit; the
     queue is shared by every window (Decision 1), so emptying it into one window's composer
     (Claude Code terminal's behaviour) would pick a window arbitrarily.
   - Keep it and deliver the next queued message right away.
   - Empty it back into the interrupting window's composer, as Claude Code's terminal does.

## Follow-ups not done

- Cloud container: `gh` unauthenticated; no gh steps run in the spec unit (none needed: no PR). Reviewers ran via `claude -p --agent autopilot-reviewer` (no Agent tool).
