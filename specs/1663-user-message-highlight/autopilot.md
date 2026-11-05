# Autopilot ledger — #1663 user-message-highlight

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1663 "Structured view: messages sent by the user are not visually distinct from the agent's output" (tinted surface + left accent for user turns: typed, queued-delivered, slash commands; theme tokens; contrast; selection #1550; visual pass). labels: none (now flow:feature)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1663
- **Worktree branch**: feat/1663_structured-view-messages-sent-by-the-user-are-not
- **Started**: 2026-11-05
- **Phase**: plan
- **Next step**: tasks unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Tint+accent or right alignment | Tint + left accent, full width | agent-resolved | spec.md#Assumptions |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 6911cba97a4f081d2cad1cc6b9eeb0fef5bc3891:93bc2a7f9a0a8723aa5ec8061cfe36c235c9f6f3 | CLEAN (3 MINOR, fixed prose only) |
| Plan | 1 | 376151e778b78200cd75a9b4d2f7044881ea765f:82db0e58b277c30ba4d849a4598ba04e62761bc1 | CHANGES (1 MAJOR test-observable named in plan; 3 MINOR fixed); fixes are plan-text clarifications, checked by me, no round 2 |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
