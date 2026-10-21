# Autopilot ledger — #1092 daemon-settings-overlay

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Daemon settings should open in an overlay with its own Save, not rely on the Settings save button (issue #1092); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1092
- **Worktree branch**: feat/1092_daemon-settings-should-open-in-an-overlay-with-its
- **Started**: 2026-10-19
- **Phase**: done
- **Next step**: handoff

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1137 | Design | merged | 0db3cf9ccbebc1b98662a5becfc4f2c4c3b96504 |
| #1160 | M1 | merged | 08cb6ea11959288eb6832df9cce396a0589238f1 |
| #1170 | M2 | merged | e8055c90fa21a43bcfcc6fa33187f3ca3d39c5db |
| #1174 | M3 | merged | b56bf28d79e4eac5de6ee35510e7a0731e66e864 |
| #1182 | M4 | merged | 79cd9920ac36905bc8de980cda05bc2018e65667 |
| #1196 | Close | open | - |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T014, T016, T017, T019, T020 | full | Add/Edit daemon in a modal dialog, own Save/Cancel, in-dialog confirmation, no inline form | #1160 | merged |
| M2 | T015, T018, T021–T023 | full | Add-path tests, focus return | #1170 | merged |
| M3 | T024–T029 | light | Details in a dialog | #1174 | merged |
| M4 | T031–T033, T035–T039 | full | Test strength (tdd-verify F2-F8) and drift prompts wait while Details is open (converge) | #1182 | merged |

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
| M3 Review A | 1 | 2d73eeb79e83dc1283335b0e56fada23ba12f8f9:56fa4f9e024877f97adbf83b93318d93e9a288a1 | CLEAN (1 MINOR: opener not cleared on removal; declined, harmless) |
| M3 Review B | 1 | same | CLEAN (MINOR: Close bounds not asserted; test checks painted + no overflow) |
| M4 Review A | 1 | (not recorded by the interrupted unit) | findings fixed in the review-A fix commit (focus after queued drift prompt; stale Details) |
| M4 Review B | 1 | cd7553d2f4e78fcff4fbaae32cf5041d7515417c:d0e2912ca6fbf90ee8defc2a0f8ac4b48df04b6c | CLEAN (1 MINOR: Remove/Start/Stop wiring not pinned, outside T031) |
| Close Review | 1 | 5b858d29a95782be5e29784b579d637f7d6d6905:79cd9920ac36905bc8de980cda05bc2018e65667 | CLEAN (2 MINOR: verification.md stale on finding 2, fixed prose only; test counts task units only, left) |

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

## Close unit findings (1st close)

- Close unit 1: tdd-verify FAIL (tdd/verification.md); converge found one partial behaviour (drift prompt can open over Details). Both cut into M4. T034 accepted: no TDD extension was run, so red-phase evidence is missing for M1-M3.
- Visual pass steps 6-7 (headless sway, wtype): Details shows errors/log, Close/Escape/outside click close it, no Hide details: Pass. Live log line, Tab cycle: not judged (daemon unreachable; screenshots cannot show focus). Focus ring on the opener after Close/outside click: seen; after Escape on Details: unclear. Possible stale "This field is required" on a reopened Add dialog after a blur + Escape: unconfirmed, look at it in M4 visual or the next close pass.
- Stale-daemon wording differs: daemons.rs:989 "That daemon no longer exists." vs :1213 "This daemon no longer exists" (cosmetic, not fixed).

## Close unit findings (2nd close)

- converge: no unbuilt behaviour. tdd-verify re-run: FAIL only on HIGH finding 1 (no red phase recorded for T001-T029, T031-T033, T035-T038; cannot be earned retroactively; accepted in T034, needs a maintainer waiver to clear). Finding 2 (MED, focus push unpinned) fixed: test `shell::daemons::tests::the_shell_pushes_the_focus_task_when_a_dialog_closes`, red with the push deleted. Findings 4-6 LOW, left.
- Quickstart Part B finished (results in quickstart.md): all remaining items Pass; stale required error not reproduced. Open nit: no visible focus ring on the checkbox, Save daemon and one step after Cancel in the Add dialog; Edit dialog Tab not tested.
