# Autopilot ledger — #1271 review-reviewed-marks

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start work on issue #1271 "Review UI: reviewed marks and progress" (blocked by #1269, not merged; spec builds on its design in docs/development/review-redesign.md); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1271
- **Worktree branch**: claude/project-thread-umhor9
- **Started**: 2026-10-24
- **Phase**: spec
- **Next step**: spec review round 2 (scoped, sonnet) on the round-1 fixes, then clarify unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Reset marks on every new head commit, or per file's change? | Per file's change (a mark holds while the file's whole change is the same) | agent-resolved | spec.md#Assumptions |
| D2 | spec | Row symbol for a marked file with drafts | Reviewed wins; draft count stays on the row | agent-resolved | spec.md#Assumptions |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 5cce47b02a0a2a5998d722bd75a1292810c331b6:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | CHANGES: 1 MAJOR, 3 MINOR (run by the orchestrator). F1 fixed: FR-016 keeps a mark until unmarked/re-marked/entry removed, edge case and US2 scenario 6 added; F2 fixed (prose): US2 s5 limited to entries with a base; F3 fixed: older-window edge case; F4 fixed: US4 s2, PR marks kept like PR comments |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.
