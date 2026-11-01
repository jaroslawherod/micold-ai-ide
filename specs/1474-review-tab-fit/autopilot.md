# Autopilot ledger — #1474 review-tab-fit

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1474, remaining items (diff header row, diff placeholder, comments drawer at 640x480); user decided: auto-collapse the sidebar at the smallest window. labels: bug, flow:bug (switched to flow:feature)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1474
- **Worktree branch**: fix/1474_review-tab-parts-are-squeezed-out-of-their-boxes
- **Started**: 2026-11-01
- **Phase**: tasks (done)
- **Next step**: design PR merge, then milestone M1. T019 (quickstart §B record) is left to the close unit. Spec "persisted today" wording corrected: nothing found persisting sidebar hidden; only "a resize never writes hidden/width" is promised.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1549 | progress slot (earlier bug flow) | merged | n/a |
| #1592 | sort chips (earlier bug flow) | merged | 3190828cbe433f90b5458ad23837e6678d1c8910 |
| #1626 | design (spec, plan, tasks) | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T010 | full | sidebar collapses to the strip below 720px | | pending |
| M2 | T011–T018 | full | review tab fits at 640x480 | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | How to give the review panes width at 640 | Auto-collapse the sidebar at the smallest window | user | AskUserQuestion 2026-11-01 |
| D2 | spec | Trigger, restore, manual toggle, full screen | Width < 720 logical px; restores on widening; manual show at compact holds until the window leaves compact; full screen unchanged | agent-resolved | spec.md FR-001..FR-005 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | d479a4fb980aed7b1d23fbfa608a58ec714398df:8d298a88dbce086d88ae937266f294e07cee7172 | CHANGES: 3 MAJOR (fixed) |
| Spec | 2 | dc353a16f282665981ce56c92fbed4adaa868667:2eff934fc3e57811dc995beaa9e06dbb27f41556 | CLEAN (1 MINOR fixed) |
| Plan | 1 | 04aabdf50db4f867c6c977f36de718b6cd34eaac:08b2c3758d99257ccec6a1fa67e96392653d8fb9 | CHANGES: 2 MAJOR (fixed), 3 MINOR (fixed) |
| Plan | 2 | fbff5a3d1173541778ab6ffce9a94740c03224fb:08b2c3758d99257ccec6a1fa67e96392653d8fb9 | CLEAN (2 MINOR fixed, prose only) |


## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.
