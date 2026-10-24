# Autopilot ledger — #1272 review-source-dropdown

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1272 — issue #1272 "Review UI: source dropdown and two-column layout"; labels: enhancement, flow:feature (in-progress added by the orchestrator)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1272
- **Worktree branch**: claude/project-thread-43e62l
- **Started**: 2026-10-24
- **Phase**: tasks
- **Next step**: tasks unit: speckit-tasks, milestones, analyze, tasks review, checklists, design PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | How does the Files option combine with the shown source (Story 3 s2)? | As today: on/off mode over the shown source, stays on across source picks (tree and browsed file re-read, Back forgotten); label `Files · <source>`; Open diff leaves source and Files unchanged | agent | docs/user-guide/reviewing-changes.md#Browsing the repository; specs/1150-diff-full-file-view/spec.md#Edge Cases; features/changes.rs `reread_after_source_change`; spec FR-006/FR-007/SC-004 |
| 2 | plan | How to sequence against unmerged #1270/#1269? | M1 core model, M2 reducer, M3 shared `MenuSelect` ship first; M4 view wiring (header, left column, 260 px two columns, snapshot, docs, visual pass) waits on #1270 merged | agent | plan.md#Milestones and sequencing; research.md R8 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 0f203fb2717e1fa99d2ac2b68c94c24f64c916f3:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |
| Spec | 2 | 5cb0e7aeb46f21b103a07c493f87bb563ef37b13:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | CLEAN: 3 MINOR (fixed, prose only) |
| Clarify | 1 | 1 question, agent-resolved (Story 3 s2) | requirements changed: FR-001, FR-002, FR-007, Story 3 s1–s5 |
| Clarify | 2 | rescan of the clarified spec | no critical ambiguities |
| Plan | 1 | 6f35131b36e2a60c6aab6421cd8d93d2789984e8:1ce7434bb25151d52292a1c7868132c7a0ef2313 | CLEAN: 3 MINOR (fixed, prose only: F1 `source_menu_stops` without a clock, F2 shared `show_working_tree`, F3 *The Default entry* in the M4 docs); dispatched by the orchestrator |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Depends on unmerged #1270 (R1, bottom bar + drawer; removes the right column) and possibly #1269 (R0, Review tab). FR-008's two-column result is verifiable only after #1270 merges; plan must sequence or gate on it.
