# Autopilot ledger — #430 tooltip-follow-cursor-delay

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: #430; labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #430
- **Worktree branch**: fix/issue-430
- **Started**: 2026-10-05
- **Phase**: design
- **Next step**: orchestrator: wait for the design PR to merge, then milestone M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #594 | Spec | merged | 5ae41ab8c09440cc64a73f0fb24317185bce43af |
| #596 | Design (clarify, plan, tasks) | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T012, T019 | full | Pointer-following placement (FollowCursor), ShowTimer in core | | pending |
| M2 | T013–T018 | full | show_delay, working alone and with FollowCursor | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | Show delay vs after_rest? | Separate delay from entering; after_rest unchanged | agent-resolved | spec.md#Clarifications |
| D2 | plan | Both after_rest and show_delay set? | One mode `Wait::{Hover,Delay,Rest}`; the setter called last wins | agent-resolved | research.md#R1 |
| D3 | tasks | User guide? | None: no user-visible change; PRs take `docs-not-needed` | agent-resolved | plan.md Constitution VII |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review A spec 430 | 1 | 7288a534:431b4987 | CHANGES: 4 MAJOR |
| Review A spec 430 | 2 | b7d68903:431b4987 | CLEAN (3 MINOR, applied) |
| Review A tasks 430 | 1 | bd1c9cf0 | CHANGES: 1 MAJOR (fixed) |
| Review A tasks 430 | 2 | bd1c9cf0 | CHANGES: 1 MAJOR (T019 order, fixed) |
| Review A tasks 430 | 3 | 037380d0:70d1227f | CLEAN |
| Review A plan 430 | 1 | a3eb3d60:29382409 | CLEAN (3 MINOR, applied) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None. <or, when a unit handed over: what is done, the next step, open findings with their review
snapshots. The next unit sets it back to None.>

## Open escalation

None. <or: the banner as sent, and when>

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
