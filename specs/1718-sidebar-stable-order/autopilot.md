# Autopilot ledger — #1718 sidebar-stable-order

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1718 "Sidebar: stop grouping worktrees/sessions by state — rows jump between groups and are hard to track"; labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1718
- **Worktree branch**: feat/1718_sidebar-stop-grouping-worktrees-sessions-by-state
- **Started**: 2026-11-06
- **Phase**: M2 implemented (T015-T029), tests first; gate, reviews and visual pass next
- **Next step**: scoped gate + review A, then review B + visual pass, full gate, PR for M2

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1750 | Design PR | merged | 01bc227bc6684dbbcc4dc964b53d0290eb704a03 |
| #1782 | Amend design (tint, cleanup candidate) | merged | 9d65146d1f8bb4dcb76f62c50ce241d65b91b04e |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T014 | full | Flat stable sidebar list, no group residue (US1+US3 are one change) | #1758 | merged 8b1039d2c123f8e414806ecc75ab0cd6ae5a7c3f |
| M2 | T015–T030 | full | Five-state row indicator (StateIcon on CardState) plus row tints | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | plan | Realise the cleanup candidate how? | No sixth state: Done is the candidate; louder tint for Needs action/attention, quieter for Done; closed-unmerged stays Idle (follow-up) | user | research R7 |

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

M2 code is done and committed (T015-T029 ticked, tests-first commit then prod commit, red evidence in
tdd/cycle-log.md; layout_snapshot fixture accepted deliberately: the tint container adds a tree level).
Review A: CLEAN (3 MINORs; F1 group tooltip fixed; F2/F3 accepted: glyph keeps tint ink on a selected row,
selected row drops its tint by design (R7, T026)). Review snapshot for A: cc6798975ec5:e085a674f061.
Scoped gate: only failure was layout_snapshot, now fixed; re-run scoped gate not yet done.
Next: (1) scoped gate; (2) review B (autopilot-reviewer, sonnet, rubrics/conformance.md; Verify:
`cargo test -p micold-client --lib state_icon`) together with the visual pass (visual-pass via
autopilot-worker: quickstart Part B steps 1 and 3, drive the real flat sidebar if possible, else the showcase
StateIcon entry, both themes; save evidence in specs/1718-sidebar-stable-order); (3) tick T030; (4) full
`mise run gate`; (5) add one line to user guide that a selected row shows the selection instead of its tint
(optional); (6) PR "feat(1718): show five-state row indicator and row tints (#1718)" per tasks/pr.md.
No PR open yet for M2. #1782 merged (9d65146d1).

## Open escalation

None.

## Follow-ups not done

Closed-unmerged PR as a cleanup candidate: own issue, needs a data-safety design (spec Out of Scope, research R7).

- tasks R1 snapshot 09483c9..:83987a3.. (speckit-analyze: 0 CRITICAL/HIGH; I1,I2,C1,F1 fixed; no plan inconsistency found) -> CLEAN, 3 MINOR path fixes applied. Plan.md line 81 cites `tests/layout_snapshot.rs` without the crate prefix (tasks use the full path).
- amend-design part 2 round 1 snapshot 56bba68..:eaa6b00.. CHANGES (3 MAJOR: tooltip wording source, [P] file overlap, checklist/tdd; fixed, spec-artifact edits only, no behaviour claim) -> no round 2, prose/task-format fixes
