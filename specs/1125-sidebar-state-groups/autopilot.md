# Autopilot ledger — #1125 sidebar-state-groups

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1125 "Layout: sidebar grouped by state with Overview as its first row" (body verbatim in the issue and in spec.md *Input*); the user asked to start the app redesign issues in dependency order. labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1125
- **Worktree branch**: claude/project-thread-22fhnj
- **Started**: 2026-10-24
- **Phase**: clarify
- **Next step**: clarify unit: resolve the three [NEEDS CLARIFICATION] markers in spec.md (US1 sc.2, US4 sc.6, US5 sc.2)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Overview button in the top bar: removed here? | No: it stays; #1127 removes it ("Blocked by: #1125 (Overview moves into the sidebar first)") | agent-resolved | issue #1127 body |
| D2 | spec | Order of Done vs Idle | Done last, after Idle | agent-resolved | issue #1125 body; mockup board sidebar |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | da2d807c9816acd2626754adf775bd75bfdf81bb:3c7db2eb08425cc9cab3f4001be9e64c03c8ccd2 | CLEAN (3 MINOR, fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.
