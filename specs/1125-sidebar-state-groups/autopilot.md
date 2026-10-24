# Autopilot ledger — #1125 sidebar-state-groups

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1125 "Layout: sidebar grouped by state with Overview as its first row" (body verbatim in the issue and in spec.md *Input*); the user asked to start the app redesign issues in dependency order. labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1125
- **Worktree branch**: claude/project-thread-22fhnj
- **Started**: 2026-10-24
- **Phase**: design-pr
- **Next step**: orchestrator opens the design PR (title/body in the session scratchpad), waits on CI and merges; then milestone M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| (to open) | Design: spec, plan, tasks (branch pushed at the tasks unit's last commit; orchestrator opens via GitHub MCP) | pending | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T019 | full | Sidebar grouped by state (headers with counts, Done collapsed, toggles, reveals/chords open a closed group) | — | todo |
| M2 | T020–T031 | full | Two-line rows, no type/issue chips, primary-container selection | — | todo |
| M3 | T032–T041 | full | Overview as the sidebar's first row, top bar's badge and tooltip | — | todo |
| M4 | T042–T051 | full | Chip bar (All, tag chips, add, hide, ⋯ menu) replaces the filter panel | — | todo |
| M5 | T052–T058 | full | Keyboard and hover stay on a row that moves group; KeyedColumn in cdk | — | todo |

Polish T059–T060 changes no code: close unit. M1 kept whole at 19 tasks (grouping, collapse and reveal-into-collapsed only work together; tasks.md M1 *Size*). M2 kept whole at 12 (two lines and dropped chips are one row change). US3 before US2 (both P1): US2's selected look uses US3's `Selection::Primary`.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Overview button in the top bar: removed here? | No: it stays; #1127 removes it ("Blocked by: #1125 (Overview moves into the sidebar first)") | agent-resolved | issue #1127 body |
| D2 | spec | Order of Done vs Idle | Done last, after Idle | agent-resolved | issue #1125 body; mockup board sidebar |
| D3 | clarify 1 | Needs attention: own group or joins Needs action? | Joins Needs action; no separate group | agent-resolved | issue #1125 lists four groups; mockup puts fix/1117 "checks failing" (NeedsAttention, overview.rs card_state) under "Needs action · 2" |
| D4 | clarify 1 | Where do refresh, attach existing, run in parallel go? | A more (⋯) menu at the chip bar's end, beside add and hide | decided by user | jaro, 2026-10-24 |
| D5 | clarify 1 | Which groups are collapsible? | Every group; Done starts collapsed, the others expanded | decided by user | jaro, 2026-10-24 |
| D6 | clarify 2 | More menu's look and dismissal | Shared `Menu` primitive + feature 017 overlay rules | agent-resolved | ui/material/menu.rs, micold-core/src/overlay.rs module docs |
| D7 | plan | Overview row badge under a tag filter: all locations or the listed ones? | The listed ones, the same cards as the top bar badge; spec FR-008 and US2.6 amended | agent-resolved | ui/overview.rs `cards` walks `sidebar_entries()` (filtered); 813 FR-001 one card per listed location; SC-002 requires equality (research R10) |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | da2d807c9816acd2626754adf775bd75bfdf81bb:3c7db2eb08425cc9cab3f4001be9e64c03c8ccd2 | CLEAN (3 MINOR, fixed, prose only) |
| Plan | 1 | e7874cdbc745d323aa39bf2af5b3eb37e903f5d6:14cef2c07be917f0d69a3be91fbcea6f77f1b079 | CLEAN |
| Analyze | 1 | — | 0 CRITICAL/HIGH; 4 MEDIUM, 7 LOW: I1 (spec FR-007/US2.5 now say no Overview row with no project open, matching Edge Cases), C1 (T012 and the test migration split), C2 (Tab-order tests T035, T045), C3 (revealed row in T033), C4, C5, D1 (plan Docs list), D2, A2, U1 fixed; A1 left (G5 already cross-referenced by FR-005/FR-006) |
| Tasks | 1 | c3988cc2b123a2d2b824ce43111671dff99979ae:a53d7012cf8f7b5295432a203dac8857c4efd1dc | CLEAN (3 MINOR: test callers named in T014, M1 and M4 Verify widened, US5.1 moved to M4 Satisfies; fixed) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.
