# Autopilot ledger — #1718 sidebar-stable-order

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1718 "Sidebar: stop grouping worktrees/sessions by state — rows jump between groups and are hard to track"; labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1718
- **Worktree branch**: feat/1718_sidebar-stop-grouping-worktrees-sessions-by-state
- **Started**: 2026-11-06
- **Phase**: implement M1 (handover)
- **Next step**: design PR opened; then milestone M1 (finding: 905 StateIcon has 3 glyphs, so StateIcon is re-keyed on CardState, research R1/R6)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1750 | Design PR | merged | 01bc227bc6684dbbcc4dc964b53d0290eb704a03 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T014 | full | Flat stable sidebar list, no group residue (US1+US3 are one change) | | pending |
| M2 | T015–T022 | full | Five-state row indicator (StateIcon on CardState) | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

- M1 A snapshot 5614aef..:aac273a.. (dispatched); M1 B same snapshot (dispatched)

- plan R1 snapshot 9c7a789..:5d02cca.. CHANGES (2 MAJOR fixed); R2 snapshot cdf4399..:a38b989.. CHANGES (2 MAJOR: contradictory Complexity Tracking, incomplete file list; fixed, prose only)

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | c5e140ccc4b8e1b8c17dddadfae8ef5d5f94de53:c695061be43a32a00e41c482221be91d470a34bb | CHANGES: 3 MAJOR (persistence story, open markers, bad FR-017 cite); all fixed; no round 2, fixes remove claims |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| #1750 | Design PR | merged | 01bc227bc6684dbbcc4dc964b53d0290eb704a03 |

## Handover

M1 (T001-T014) in progress, unit 2, no M1 PR open yet. The design PR #1750 merged (SHA 01bc227bc6684dbbcc4dc964b53d0290eb704a03).
Done and committed in the WIP commit: all production edits (core sidebar_groups.rs, client features/sidebar.rs, navigation.rs, main.rs, ui/sidebar.rs, tree_view.rs, material text.rs/type_role_mapping.rs, showcase catalogue/surfaces, composition_contrast.rs); new tests (sidebar_state.rs `flat_list` mod, navigation_apply.rs `over_the_flat_list` mod, features_sidebar.rs `rows` helper removed); docs (worktrees-and-sessions.md, layout-snapshot.md); covered_states.rs and context_menu_anchor.rs row paths re-pointed (no header row); layout snapshot fixture regenerated (UPDATE_LAYOUT_SNAPSHOT=1, one run passed 177). `cargo check --workspace --all-targets` is green. Targeted `--test main -- sidebar_state:: navigation_apply:: features_sidebar::` had 2 failures, both fixed (index offsets, chord yields Step::Session); re-run not yet seen green.
TDD note: the previous unit edited production before tests, so there is no red-first evidence for T002-T006; the unit rewrote the tests after. Say so in the PR body, or write red evidence by running the new tests against origin/main (they cannot compile there: they only use the new API shape, so the compile failure is the red).
Build hazard: target-shared build-script dirs collide across worktrees (relative .d paths); if `micold-core` build.rs fails with "read routing.rs", `touch crates/micold-core/build.rs` and rerun.
Next: (1) confirm the targeted tests and layout_snapshot compare green; check `tests/showcase_completeness.rs` and `tests/gates/*` rows; (2) tick T001-T014 in tasks.md and write tdd/cycle-log if the TDD hook requires it; (3) `mise run gate` detached (queue behind other worktrees, hold.sh); (4) verify.md, review.md, review-rounds.md, gate.md, pr.md; (5) ledger table: M1 row PR, #1750 merged.

## Open escalation

None.

## Follow-ups not done

None.

- tasks R1 snapshot 09483c9..:83987a3.. (speckit-analyze: 0 CRITICAL/HIGH; I1,I2,C1,F1 fixed; no plan inconsistency found) -> CLEAN, 3 MINOR path fixes applied. Plan.md line 81 cites `tests/layout_snapshot.rs` without the crate prefix (tasks use the full path).
