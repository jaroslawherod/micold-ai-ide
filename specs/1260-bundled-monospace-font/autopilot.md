# Autopilot ledger — #1260 bundled-monospace-font

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1260 — "Bundle a monospace font so code blocks and diffs look the same on every host" (follow-up from #940; #1276 depends on it). labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1260
- **Worktree branch**: claude/project-thread-c0bgc0
- **Started**: 2026-10-24
- **Phase**: plan
- **Next step**: plan unit (clarify done: CLEAN after 2 rounds)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | FR-007: does the terminal pane adopt the bundled monospace face? | Yes (option A): terminal cells use the bundled face (Bold for bold cells), keeping its own grid metrics | agent | 018 FR-012 exempts the terminal from Roboto/type scale only, not from a bundled mono; `cell_font` uses `Font::MONOSPACE`; `CellMetrics::new` hard-codes 0.6 em = Fira Mono's advance; Principle VI |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Clarify | 1 | FR-007 | resolved by agent (requirement changed: FR-007, FR-010, edge case) |
| Clarify | 2 | after round 1 | no critical ambiguities; unit ends CLEAN |
| Spec review | 1 | 2a0f5efa62e0546cb87daef04623b4c3b3b572ad:3c7db2eb08425cc9cab3f4001be9e64c03c8ccd2 | CLEAN (3 MINOR, all fixed: Regular+Bold decided as FR-001/FR-011; SC-002 reworded; checklist tick corrected, FR-001/FR-006 reworded as outcomes). Reviewer ran as headless `claude -p` (no Agent tool in this unit) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.
