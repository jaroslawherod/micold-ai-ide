# Autopilot ledger — #1299 overview-card-focus-motion

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Start work on issue #1299 (Overview cards: animate keyboard focus and expansion), in dependency order of the app redesign issues; labels: enhancement, flow:feature (in-progress added at start)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1299
- **Worktree branch**: claude/project-thread-vugbvf
- **Started**: 2026-10-24
- **Phase**: tasks
- **Next step**: tasks review round 2 (orchestrator, scoped, sonnet) on the snapshot below; if CLEAN: close checklists (requirements.md: 0 unchecked), specs-only gate, design PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T011 | full | Tab and arrows fade an Overview card's focus in and out over 150 ms; Reduce motion makes it instant | — | pending |
| M2 | T012–T023 | full | expanded card's shadow eases in and out, never cut while settling; showcase live pose; Reduce motion instant (12 tasks, kept whole: lift + shadow quad + draw order are one observable change) | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | Window blur/refocus: does the focus indication change? | No; only the collapsing shadow eases | agent | card_grid.rs on_window_blur; 1121 Edge Cases |
| 2 | clarify | Does the hover state layer fade? | No, out of scope | agent | 1299 spec Input (issue text) |
| 3 | clarify | Focus fade when the card's context menu opens/closes? | Yes, out on open (FR-002), in on return (FR-001) | agent | 972 US3-1, FR-011 |
| 4 | plan | FR-006: is the expanded card's growth animated? | No: height changes in one step; focus indication and shadow ease. Eased layout height reflows the grid every frame and the inner scrollable overflows (scrollbar shows, K4 Page Up/Down retarget mid-growth); a clipped reveal cuts corners and the ring's bottom edge; no collapse counterpart (card rebuilt collapsed). Post research R1's comment text on #1299 | agent | research.md R1; card_grid.rs layout, overview_card.rs expanded_body |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| spec | 1 | 6b313c8abe79644b3f5e295d48bb1ca579e97efa:3c7db2eb08425cc9cab3f4001be9e64c03c8ccd2 | CLEAN (3 MINOR, all applied: US1-4 link focus keeps card expanded; window-blur edge case; FR-001 bezier moved to Assumptions) |
| plan | 1 | 577b43efa9a360325533cf64df7477eec5f512fc:4acec2deae057fa05e7e2201ced9f3ffd17255f2 | CLEAN (3 MINOR, all applied: FR-004 reversal is `on_frame` retargeting, not private `retarget`; SC-001 16 ms tick assertions added to the lift test both directions; R8/D1 built-focused start limited to wrappers with `on_focus_change`, cards fade in after reopening) |
| tasks | 1 | 2bed35d91ef460077b32335b674b42ff21c4b977:7a4c2143fcab1448dc202c93ae8352b219a54d1b | CHANGES (2 MAJOR, 2 MINOR, all fixed: F1 M1 Verify used showcase live pose built in M2 → client Tab onto/away check; F2 `[P]` dropped from T003/T015, Parallel Opportunities updated; F3 T014 gains US2-4 ring+lift together case; F4 T001 names the closures by test name. speckit-analyze not loadable; FR/SC coverage checked by hand) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Issue #1298 (card design match) runs in a parallel session and edits `overview_card.rs`; expect a rebase conflict there.
