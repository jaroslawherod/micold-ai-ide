# Autopilot ledger — #430 tooltip-follow-cursor-delay

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: #430; labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #430
- **Worktree branch**: fix/issue-430
- **Started**: 2026-10-05
- **Phase**: milestone M1
- **Next step**: CI and merge of the M1 PR, then M2

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #594 | Spec | merged | 5ae41ab8c09440cc64a73f0fb24317185bce43af |
| #596 | Design (clarify, plan, tasks) | merged | 47ae6297d5e472d802897310d574e25d53bea3d3 |
| #600 | M1 pointer-following placement, ShowTimer | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T012, T019 | full | Pointer-following placement (FollowCursor), ShowTimer in core | #600 | in review |
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
| Review A M1 430 | 1 | 269bde8a:81c36f8b | CLEAN (1 MINOR: glue harness cannot see invalidate_layout; not applied) |
| Review B M1 430 | 1 | 269bde8a:81c36f8b | CLEAN (1 MINOR: no tdd cycle-log) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None. (Part 2: gate green with MICOLD_SKIP_GH_LAUNCH_TEST=1, the test's own opt-out, because this machine has no gh where a desktop launch finds it; CI runs it. T019 visual pass NOT RUN: no Xvfb/xdotool, installing needs root. Caveats recorded in the PR body. The lock-holding hung test pid 228600 was killed on the user's answer.)

## Open escalation

None. (Answered: kill pid 228600; done.)

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
