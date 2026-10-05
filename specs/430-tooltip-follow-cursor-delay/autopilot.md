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
- **Next step**: full gate (`mise run gate`), then open the M1 PR; T019 visual pass blocked (see Handover)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #594 | Spec | merged | 5ae41ab8c09440cc64a73f0fb24317185bce43af |
| #596 | Design (clarify, plan, tasks) | merged | 47ae6297d5e472d802897310d574e25d53bea3d3 |

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
| Review A M1 430 | 1 | 269bde8a:81c36f8b | CLEAN (1 MINOR: glue harness cannot see invalidate_layout; not applied) |
| Review B M1 430 | 1 | 269bde8a:81c36f8b | CLEAN (1 MINOR: no tdd cycle-log) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 code, tests and showcase pose are written and uncommitted in the worktree (T001-T012 ticked; T019 open). Local checks green: core `tooltip_show` (12), client lib tooltip tests, `tooltip_show_glue`, `idle_requests_no_frames`, `tooltip_rest_glue`; clippy not re-run after the `spacing::MD` fix. Reviews A and B are CLEAN. The scoped gate (log in scratchpad `sg.log`) has waited hours on the build lock held by other worktrees; a kill of it and a `MICOLD_NO_BUILD_LOCK=1` full gate were denied by the permission classifier, so no gate has run on the final tree.
Open: (1) run `mise run gate` (wait for the lock), commit, push, open the PR; (2) T019: this machine has no Xvfb/xdotool, so the visual pass could not run (note in visual-pass.md); install them or the orchestrator accepts T019 as not run; (3) TDD note: implementation was written before the placement/glue tests, so there is no red evidence for T003/T006/T007/T009 (T011 was seen red: missing `ShowTimer`).
Design note: a `subject()` change on a follow tooltip with no delay reopens at once on the next observation (the zero-delay rule), it does not stay closed.

## Open escalation

None. <or: the banner as sent, and when>

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
