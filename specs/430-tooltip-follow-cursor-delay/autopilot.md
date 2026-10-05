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

M1 T001-T012 committed locally (97e89cf8), not pushed; reviews A and B CLEAN. Full gate `mise run gate` started detached on the final tree, log `/tmp/claude-1000/-home-jaro-workspaces-micold-ai-ide--claude-worktrees-fix-issue-430/03cd0d33-2f02-4edc-8c7d-c7eaa2e1b3e2/scratchpad/gate-final.log` (ends with `GATE_EXIT=<n>`); it is queued on the build lock and will run when the lock frees. Do not start a second one.
Next: when `GATE_EXIT=0`, push (`git push --force-with-lease -u origin HEAD`), open the PR with `/tmp/claude-1000/-home-jaro-workspaces-micold-ai-ide--claude-worktrees-fix-issue-430/03cd0d33-2f02-4edc-8c7d-c7eaa2e1b3e2/scratchpad/pr-body.md` (title `feat(430): follow-cursor tooltip placement and ShowTimer (#430)`, `Refs #430`, label `docs-not-needed`), record the PR number here. If the log is gone, rerun the gate.
T019: NOT RUN. No Xvfb/xdotool/xwininfo on this machine and installing them needs root (not done); recorded in visual-pass.md and the PR body.
Caveats (both recorded in the PR body): tests for placement/glue were written after the code, only T011 was seen red; a `subject()` change on a zero-delay follow tooltip reopens at once on the next observation.
Blocker: see Open escalation.

## Open escalation

Blocked by work outside my flow (category 5): the build lock `/home/jaro/workspaces/micold-ai-ide/.git/micold-build.lock` is held by pid 228600, a test binary `target-shared/debug/deps/attach_apply-7ab230cce8e2ae42` (worktree feat-582-attach-provider), running 12h46m at 0% CPU, in futex wait: a hung test. Nine other builds queue behind it, including this gate. Only killing pid 228600 frees the lock; it belongs to another flow and I was told not to kill other builds.
Question: kill pid 228600 (Recommended; it is hung, evidence above), or wait? Once the lock frees, the queued gate needs no action from me.

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
