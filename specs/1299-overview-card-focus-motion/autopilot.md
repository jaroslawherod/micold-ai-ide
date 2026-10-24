# Autopilot ledger — #1299 overview-card-focus-motion

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Start work on issue #1299 (Overview cards: animate keyboard focus and expansion), in dependency order of the app redesign issues; labels: enhancement, flow:feature (in-progress added at start)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1299
- **Worktree branch**: claude/project-thread-vugbvf
- **Started**: 2026-10-24
- **Phase**: implement M1 (reviews pending)
- **Next step**: full gate (scripts/gate.sh, CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 to fit the disk) green at b0b8c38f; reviews A and B of M1 (orchestrator runs them; unit has no Agent tool), then full gate and the M1 PR. Visual pass deferred to the close unit (no Agent/visual-pass in this session)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1319 | design (spec, plan, tasks) | merged | 1f66380fd3092861e818531f9c3a308d08abd5c5 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T011 | full | Tab and arrows fade an Overview card's focus in and out over 150 ms; Reduce motion makes it instant | — | implemented, in review |
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
| tasks | 2 | 6b49c1b05d37c7c32799ad09201c21d974d12f3f:426765d05b781532d0d71423dd122cba5db2be39 | CLEAN (1 MINOR applied: M1 Verify §B.8 limited to its focus half) |
| A M1 (code-review high) | 1 | 5ccdd3607cb7cb51889bd26cda53d6820e01f56e:b0b8c38f3d52bb1dee3ecaf67845ca0937dab3e8 | pending (orchestrator) |
| B M1 (conformance) | 1 | 5ccdd3607cb7cb51889bd26cda53d6820e01f56e:b0b8c38f3d52bb1dee3ecaf67845ca0937dab3e8 | pending (orchestrator) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- M1 visual pass (quickstart §B.4 arrow half, §B.6, §B.7, §B.8 focus half; client Tab onto/away fade): deferred to the close unit, no visual-pass skill reachable in this session.
- M1