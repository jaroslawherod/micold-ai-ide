# Autopilot ledger — #1463 compact-session-header

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1463 "Compact session header: title, chips and Changes on one line; switch button in the tab strip" (body as in the issue); labels: none (in-progress, flow:feature)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1463
- **Worktree branch**: feat/1463_compact-session-header-title-chips-and-changes-on
- **Started**: 2026-10-26
- **Phase**: milestone M3
- **Next step**: M3 PR open; then a new close unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1469 | Design | merged | |
| #1487 | M1 | merged | 304bfb4ffa97076810692dcbe28c9a135eb63cbf |
| #1505 | M2 | merged | e727ac22955731c40e6ea5a719038cb054309e0e |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001, T003–T008 | full | One-line header | #1487 | merged |
| M2 | T002, T009–T018 | full | Switch control in strip | #1505 | merged |
| M3 | T021–T028 | full | Gates pinned against the surviving mutants of tdd/verification.md | | open |

## Decisions

- M1 Q1 (user): at the smallest window, after the title the Changes label is cut with an ellipsis; chips keep capped width. FR-002, US1 scenario 3 and H10 amended.
- M1 Q2 (user): `Button::density(DENSE)` for Changes (button 28 dp, no taller than the title line, header at most 40 px); R2, FR-003, T003, T005 amended.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 3d25ab22c1115b85f3840db235757bfe769dee67:15d62e891bb1292c94c58b585a0a975869c291dd | CHANGES: 3 MAJOR (fixed) |
| Plan | 1 | 451151bbdbb1400f41c955e69f57147b6becfb8f:134dd94f7895067bdc00f383b68a5f86cb23fac3 | CHANGES: 1 MAJOR, 3 MINOR (fixed) |
| Plan | 2 | 40d70c4e315358c4e6e7ec4cbaa89d4fd3767ab7:134dd94f7895067bdc00f383b68a5f86cb23fac3 | CLEAN (1 MINOR fixed, prose only) |
| Spec | 2 | 525d2bab4d47e0f8aa2a29f66b10650bbafa6b9b:15d62e891bb1292c94c58b585a0a975869c291dd | CLEAN (3 MINOR; ordering fixed, others left) |
| M2 A (code-review high) | 1 | 1867cfd07858d4a7c36be3d0522c69194ce1ee37:d7e4a2ae4bdc3e2f1d70308c399b237583c386dd | CHANGES: 1 MAJOR (switch slot child list varied; fixed, always two children), 2 MINOR |
| M2 B (conformance, sonnet) | 1 | same | CHANGES: BLOCKER was a mid-edit tree (fixed: cap +SM padding); MAJOR tiled gate weak (strengthened: position check); showcase_completeness red found by scoped gate (exemption for GiveWayRow) |
| M3 A (code-review high) | 1 | 9f4ce8128edb3d7d01c34c7ed9cf53444ece8d51:2079d3bb224be245bbfe3dca4bb95f26f30a9193 | CLEAN (2 MINOR: message wording fixed; recorded-value provenance left) |
| M3 B (conformance, sonnet) | 1 | 992ed2d4a5b293fad342ed7d0714ac784d401d17:2079d3bb224be245bbfe3dca4bb95f26f30a9193 | CLEAN; Verify 163 + 414 passed; mutants T023, T026 fail their tests (1 MINOR left) |

## Declined review findings

- Review A F1 (chips row can starve the Changes button with three long chips at 640 px): declined for this milestone. Option A (user) puts the chips before the label's cut; a priority layout needs a give-way widget the client cannot use (Lane is crate-private to micold-material). The sidebar's Review changes reaches the same action. Recorded as a follow-up.
- Review A F2/F3 (gates weakened for a zero-width title and a cut button): these are the amended H10' the user chose.
- Review A F4 (28 dp button): user accepted Q2. Button rustdoc now says the 40 dp is the default.
- Review A F7 (density range): `density` takes the scale's steps; callers pass the named constants.
- Review A F8 (spec weakened): the amendment is the user's answer to Q1.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None. (The M2 push block was stale: the remote branch was deleted after #1487 merged; M2 rebased cleanly onto origin/main and the full gate is green.)

## Follow-ups not done

- M2 visual pass (m2-notes.md): at 640 the header title shrinks to ~3 px and Changes is cut; tiled panes' switch label wraps onto two lines at 640 and the second pane header clips; held-queue caption and PR chip not shown (no fixture); Review-tab close x clipped at 640. None blocks the criteria.

- M1 visual pass was partial (dark only, 1600x1200 and 640x480 at the default sidebar; Xvfb/xdotool missing). Light scheme, 1280x800 and long-chip states unchecked; the label is cut at 640x480 as the user's Q1 answer allows. Screenshots in `visual-pass/`.

- With three long chips at the smallest window the Changes button can get no room (flat row, chips before button). A priority layout would fix it.
- `ui::changes::tests::a_24_character_name_keeps_its_room_at_every_depth` failed once in the M2 scoped gate; checked at close: passes on main.

`ui::changes::tests::a_24_character_name_keeps_its_room_at_every_depth` does NOT fail on main: it passes at e727ac229 (main, M2 merged) in a scoped run, and the full workspace run in the close unit (11541 passed, 0 failed) included it. The earlier scoped-gate failure was not reproduced (likely an artifact of the mid-edit tree).

## Tasks unit

- Review round 1 (tasks, snapshot b95ec35da:421a1eab3): F1 Verify targets fixed (verified: gates are a module of `layout_snapshot`), F2 checklist ticked, F3 note added; fixed, prose only. speckit-analyze HIGH findings I1-I3, C1 fixed.

## Close unit

- `speckit-tdd-verify`: FAIL on test strength (mutation 7/11; F2 HIGH, F4/F5/F10/F11/F12 survivors). The missing red phase (F1) is a recorded limitation. Remediation tasks T021–T028 cut into M3. Full suite 11541 passed; one unrelated flake `micold-core streaming_child` passed on rerun.
