# Autopilot ledger — #1210 worktree-session-guard

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1210 (issue #1210: Guard: stop sessions from working inside a worktree they created); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1210
- **Worktree branch**: feat/1210_guard-stop-sessions-from-working-inside-a-worktree
- **Started**: 2026-10-21
- **Phase**: milestone M1 (implemented; gate and reviews)
- **Next step**: M1 scoped gate, review A and B, full gate, PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1214 | Design | merged | 46582900eb258d140dd971d45c5d5e39e35131d3 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T010, T028 | full | Edit/write guard + allowlist, deny-body channel proven | - | pending |
| M2 | T011–T014 | full | Bash commands judged | - | pending |
| M3 | T015–T017 | full | EnterWorktree and git worktree add denied | - | pending |
| M4 | T018–T020 | full | create_worktree hand-off text | - | pending |
| M5 | T021–T027 | full | Opt-out setting and toggle | - | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | tasks | Prefetch worktree name (plan flag) | No repo convention (grep of scripts, .claude, docs finds none); keep `autopilot-prefetch` prefix constant; spec assumption corrected | agent | research.md R5, T001 |
| 2 | tasks | JSON deny from HTTP hook unverified (plan flag) | T002 is the first task of M1; negative result escalates | agent | tasks.md T002 |
| 3 | tasks | US4 allowlist placement | Folded into M1 so the live guard cannot block target-shared writes or prefetch worktrees on main | agent | milestones.md rule 6 |

## Review rounds

- plan r1 (d3ad9a32...:d703bce5...): CHANGES, 2 MAJOR fixed. plan r2 (7fbf2284...:fe467967...): CLEAN (MINOR: version.rs, daemon_sync.rs not listed; covered by "grep it").

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | 04953a8e0fcf81bad3b44bee76d980bf3a569b97:65cefbbe38d05d3054bce51ba4a2c4b3ab37a9e9 | CHANGES: 2 MAJOR (fixed) |
| Review spec | 2 | 41605b740bcbaa3d9d2d22ecdcae8ba555dd538f:65cefbbe38d05d3054bce51ba4a2c4b3ab37a9e9 | CLEAN |

- tasks r1 (c3782371...:847ac3b5...): analyze fixed D1 (T028), A1, B1, E1, T2; reviewer CLEAN (3 MINOR fixed, prose/labels only).

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
