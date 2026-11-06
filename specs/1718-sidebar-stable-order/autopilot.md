# Autopilot ledger — #1718 sidebar-stable-order

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1718 "Sidebar: stop grouping worktrees/sessions by state — rows jump between groups and are hard to track"; labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1718
- **Worktree branch**: feat/1718_sidebar-stop-grouping-worktrees-sessions-by-state
- **Started**: 2026-11-06
- **Phase**: tasks (design PR open)
- **Next step**: design PR opened; then milestone M1 (finding: 905 StateIcon has 3 glyphs, so StateIcon is re-keyed on CardState, research R1/R6)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1750 | Design PR | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T014 | full | Flat stable sidebar list, no group residue (US1+US3 are one change) | | pending |
| M2 | T015–T022 | full | Five-state row indicator (StateIcon on CardState) | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

- plan R1 snapshot 9c7a789..:5d02cca.. CHANGES (2 MAJOR fixed); R2 snapshot cdf4399..:a38b989.. CHANGES (2 MAJOR: contradictory Complexity Tracking, incomplete file list; fixed, prose only)

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| #1750 | Design PR | open | |
| Spec | 1 | c5e140ccc4b8e1b8c17dddadfae8ef5d5f94de53:c695061be43a32a00e41c482221be91d470a34bb | CHANGES: 3 MAJOR (persistence story, open markers, bad FR-017 cite); all fixed; no round 2, fixes remove claims |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| #1750 | Design PR | open | |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.

- tasks R1 snapshot 09483c9..:83987a3.. (speckit-analyze: 0 CRITICAL/HIGH; I1,I2,C1,F1 fixed; no plan inconsistency found) -> CLEAN, 3 MINOR path fixes applied. Plan.md line 81 cites `tests/layout_snapshot.rs` without the crate prefix (tasks use the full path).
