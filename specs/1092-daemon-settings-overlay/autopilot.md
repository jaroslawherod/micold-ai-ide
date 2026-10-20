# Autopilot ledger — #1092 daemon-settings-overlay

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Daemon settings should open in an overlay with its own Save, not rely on the Settings save button (issue #1092); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1092
- **Worktree branch**: feat/1092_daemon-settings-should-open-in-an-overlay-with-its
- **Started**: 2026-10-19
- **Phase**: milestone M2 (PR open)
- **Next step**: wait for CI, merge, then M3

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1137 | Design | merged | 0db3cf9ccbebc1b98662a5becfc4f2c4c3b96504 |
| #1160 | M1 | merged | 08cb6ea11959288eb6832df9cce396a0589238f1 |
| #1170 | M2 | open | - |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T014, T016, T017, T019, T020 | full | Add/Edit daemon in a modal dialog, own Save/Cancel, in-dialog confirmation, no inline form | #1160 | merged |
| M2 | T015, T018, T021–T023 | full | Add-path tests, focus return | #1170 | PR open |
| M3 | T024–T029 | light | Details in a dialog | - | pending |

## Decisions

- A fourth M1 part (unit) was allowed: decided by user. It fixed visual step 8 (root cause: `expand` clipped to its own bounds instead of the viewport; fix + pixel test `expand_clipping`), re-ran the visual pass step 8 (pass), and opened the PR. Review A/B not re-run: the fix is 6 lines in one widget plus its test (round counts stay A 1, B 1).

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | e6163bef65f6d91e605c7395cc01c429cae5abb0:0b267cf632915083624acd3d8b44e7c4cfe3f66f | CHANGES: 3 MAJOR (false premise, details mis-described, confirmations) - fixed |
| Spec | 2 | 264ca5b1ee1be4763aa21ea336b6d3cead721c30:0b267cf632915083624acd3d8b44e7c4cfe3f66f | CLEAN (1 MINOR fixed, prose only) |
| Tasks | 1 | 1182d8f167058d85b22495f71b0e31c065e4d5a4:14c1b91abf503dd9792cfcdeee7e7091799e87f8 | CLEAN (3 MINOR fixed, prose only) |
| Plan | 1 | 148e0e9fc90d89309565b866272acec9556384c0:e22746bdb8932f04732f387ba677726415c65416 | CLEAN (4 MINOR fixed, prose only) |
| M2 Review A | 1 | c2e04ee6d352affb88b858d6a55c99397353009f:7587e4590cd561429a0f28e2b229fe889f811818 | 8 MINOR-level, no BLOCKER/MAJOR; none fixed |
| M2 Review B | 1 | same | CLEAN (2 MINOR: no shell focus test, no tdd dir) |
| M2 Visual | 1 | same | Add->Cancel focus ring PASS; Esc/Save/Edit/Tab not judged (headless sway key input unreliable) |

## Declined review findings

- M2 A: dead `RowAction::Details` (used by M3 T024-T029); EditOpened overwrite (modal blocks re-entrant open); Debug-based id and duplicate `daemons-add` id (only one is ever in the tree); re-export chain (existing pattern). Declined, none BLOCKER/MAJOR.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- `navigation_drawer.rs` lines ~349 and ~447 call `with_layer(bounds)` without intersecting the viewport, the same class of bug as the `expand` one fixed in M1; not hit by this feature.

None.
