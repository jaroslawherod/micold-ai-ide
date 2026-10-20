# Autopilot ledger — #1149 pr-diff-review-comments

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1149 "Review comments on a pull request's diff, sent to the agent session to apply and push to the PR" (verbatim in spec.md Input); labels: none
- **Kind**: feature
- **Effort**: default
- **Issue**: #1149
- **Worktree branch**: feat/1149_review-comments-on-a-pull-request-s-diff-sent-to
- **Started**: 2026-10-20
- **Phase**: spec
- **Next step**: clarify unit (escalated, awaiting answers): resolve the 2 [NEEDS CLARIFICATION] markers in spec.md (wrong-branch delivery, merged/closed PR)

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
| Spec | 1 | bfa4381fe14264233731c90e5798d8826424308b:cf7c64d7d7ee21a78fe03d60d64165dd811a1c92 | CLEAN (3 MINOR, fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1 (repo does not settle either; product decisions):
1. Wrong checked-out branch on Send review (Story 3 sc.2, FR-009): deliver with a switch-branch preamble (Recommended) or refuse.
2. Merged/closed PR on Send review (Story 4 sc.3): allow with a state line in the batch (Recommended) or block.
On answers: record `_(decided by user)_` in spec.md Clarifications, replace both markers, no new round unless a requirement changes.

## Follow-ups not done

None.
