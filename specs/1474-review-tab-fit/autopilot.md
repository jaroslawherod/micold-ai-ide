# Autopilot ledger — #1474 review-tab-fit

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1474, remaining items (diff header row, diff placeholder, comments drawer at 640x480); user decided: auto-collapse the sidebar at the smallest window. labels: bug, flow:bug (switched to flow:feature)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1474
- **Worktree branch**: fix/1474_review-tab-parts-are-squeezed-out-of-their-boxes
- **Started**: 2026-11-01
- **Phase**: clarify
- **Next step**: Clarify unit: spec has no markers; confirm threshold 720, then plan

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1549 | progress slot (earlier bug flow) | merged | n/a |
| #1592 | sort chips (earlier bug flow) | merged | 3190828cbe433f90b5458ad23837e6678d1c8910 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | How to give the review panes width at 640 | Auto-collapse the sidebar at the smallest window | user | AskUserQuestion 2026-11-01 |
| D2 | spec | Trigger, restore, manual toggle, full screen | Width < 720 logical px; restores on widening; manual show at compact holds until the window leaves compact; full screen unchanged | agent-resolved | spec.md FR-001..FR-005 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | d479a4fb980aed7b1d23fbfa608a58ec714398df:8d298a88dbce086d88ae937266f294e07cee7172 | pending |


## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.
