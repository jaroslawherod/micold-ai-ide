# Autopilot ledger — #1120 overview-card-readability

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1120 "Overview cards: readable layout and content" (body verbatim in the issue). labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1120
- **Worktree branch**: claude/project-thread-7041o6
- **Started**: 2026-10-19
- **Phase**: plan
- **Next step**: plan review round 1 (plan unit, continuing the handover). If the user's FR-012 card picks B or C, reopen FR-012 and research R7 first.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | FR-012: one generic sentence for every slash command, or special wording / hiding for built-ins? | A: one generic sentence for every command, built-ins included | A, orchestrator default (recommended), user card open | spec.md Clarifications; specs/032-untitled-session-labels/contracts/first-turn-label.md C3.5a |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | fdfeae51b28753936f4e21eca650306af1098566:38a9ce97b751560df8c8feaddf1271cbceba0cc2 | CHANGES: 2 MAJOR, 3 MINOR (run by orchestrator); all fixed |
| Spec | 2 | e3d615be650081e55115b2439e1bb15d4948f794:c672c92ff959967fbae970486c6fd8c64a0fa07c | CLEAN (scoped re-review on sonnet, run by orchestrator) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Plan unit 1 (2026-10-19) handed over at the 150k context cap after writing all plan artifacts
(plan.md, research.md R1–R15, data-model.md, contracts/card-text.md, contracts/overview-card.md,
quickstart.md), committed. Branch rebased onto origin/main by branch-start.sh (no PR yet).
Next: run `scripts/autopilot/review-snapshot.sh`, record it under *Review rounds* as `Plan | 1`,
dispatch the plan reviewer (tasks/review.md, rubrics/plan.md, round 1 on the session model), act on
its verdict (tasks/review-rounds.md), then set **Next step** to `tasks unit` and return DONE, PR none.
No open findings.

## Open escalation

None.

## Follow-ups not done

None.
