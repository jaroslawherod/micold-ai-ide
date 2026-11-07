# Autopilot ledger — #1718 sidebar-stable-order

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1718 "Sidebar: stop grouping worktrees/sessions by state — rows jump between groups and are hard to track"; labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1718
- **Worktree branch**: feat/1718_sidebar-stop-grouping-worktrees-sessions-by-state
- **Started**: 2026-11-06
- **Phase**: design amendment (highlight + cleanup state) blocked on escalation
- **Next step**: design PR opened; then milestone M1 (finding: 905 StateIcon has 3 glyphs, so StateIcon is re-keyed on CardState, research R1/R6)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1750 | Design PR | merged | 01bc227bc6684dbbcc4dc964b53d0290eb704a03 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T014 | full | Flat stable sidebar list, no group residue (US1+US3 are one change) | #1758 | merged 8b1039d2c123f8e414806ecc75ab0cd6ae5a7c3f |
| M2 | T015–T022 | full | Five-state row indicator (StateIcon on CardState) | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

- M1 A and B round 1 snapshot f41ea9a..:4bc70c9.. both CLEAN (A: 2 MINOR, left: run_group_state kept by T007, reveal_ready &mut; B: Verify 64 passed). Gate 1 red on one stale test (key count), fixed

- plan R1 snapshot 9c7a789..:5d02cca.. CHANGES (2 MAJOR fixed); R2 snapshot cdf4399..:a38b989.. CHANGES (2 MAJOR: contradictory Complexity Tracking, incomplete file list; fixed, prose only)

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | c5e140ccc4b8e1b8c17dddadfae8ef5d5f94de53:c695061be43a32a00e41c482221be91d470a34bb | CHANGES: 3 MAJOR (persistence story, open markers, bad FR-017 cite); all fixed; no round 2, fixes remove claims |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| #1750 | Design PR | merged | 01bc227bc6684dbbcc4dc964b53d0290eb704a03 |

## Handover

None.

## Open escalation

Amend-design unit (Material highlight + cleanup-candidate state), phase plan. Finding: the signal exists and a state for it already exists. `CardState::Done` ("the work is merged and the worktree can go", `micold_core::overview::card_state` row 9) = PR merged AND branch containment-verified removable (`pr_status.removable`, feature 813 FR-017: only a merged PR's branch is ever removable) AND no running session (Working outranks it). It already has its own glyph (CardDone e86c), so a sixth state for "merged, no session" would duplicate Done. The only extra case in the user's wording is a CLOSED (unmerged) PR: `PrState::Closed` is read but deliberately never removable (unmerged work may be the only copy). A sixth `CardState` would also touch feature 813 (Overview order, counts, filters, `Ord`). No new data source is needed either way (Principle IV holds). Highlight request (1) is unambiguous and needs no decision. Nothing else edited yet.

Question: how to realise the cleanup candidate?
- (Recommended) No sixth state: Done is the cleanup candidate. Row highlight: Needs action and Needs attention get a louder container tint, Done a quieter tertiary/neutral container tint plus its existing glyph and tooltip "merged"; closed-unmerged PRs stay Idle, recorded as a follow-up. Evidence: card_state row 9, CurrentPr.merged, removable invariant.
- Add a sixth CardState `CleanupCandidate` = (merged-removable OR PR closed) with no running session, own glyph and tint. Costs: edits feature 813's state table/ordering/Overview, widens "can be deleted" to unmerged closed work (data-loss risk), six states in tests/docs.
- Keep five states and ship the highlight only; drop the cleanup request.

## Follow-ups not done

None.

- tasks R1 snapshot 09483c9..:83987a3.. (speckit-analyze: 0 CRITICAL/HIGH; I1,I2,C1,F1 fixed; no plan inconsistency found) -> CLEAN, 3 MINOR path fixes applied. Plan.md line 81 cites `tests/layout_snapshot.rs` without the crate prefix (tasks use the full path).
