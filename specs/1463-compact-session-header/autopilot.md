# Autopilot ledger — #1463 compact-session-header

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1463 "Compact session header: title, chips and Changes on one line; switch button in the tab strip" (body as in the issue); labels: none (in-progress, flow:feature)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1463
- **Worktree branch**: feat/1463_compact-session-header-title-chips-and-changes-on
- **Started**: 2026-10-26
- **Phase**: tasks
- **Next step**: merge design PR #1469, then M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1469 | Design | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001, T003–T008 | full | One-line header | | pending |
| M2 | T002, T009–T018 | full | Switch control in strip | | pending |

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

blocked by work outside my flow (category 5). M1 is done and reviewed (A and B addressed, partial visual pass) but `mise run gate` is red on `ui::changes::tests::a_24_character_name_keeps_its_room_at_every_depth` ("the tree indents at the default width: cap 0"), and the push hook refuses code no green gate saw. It fails identically on `origin/main` source (session_area.rs and button.rs checked out from origin/main 308ab390a, same result), so it is not this diff; it measures text width, likely this machine's fonts. Everything else in the gate (fmt, clippy, all other tests) passed in the earlier runs. Question: how to push? Option A (Recommended): run the full gate where the test passes (CI or a machine with fonts), or let the orchestrator push the branch (6 commits, rebased on 308ab390a) and let CI decide. Option B: a separate fix of that test on main first. Nothing is pushed; no PR yet. PR body notes to write: review A and B findings in Declined section, Q1/Q2 amendments.

## Follow-ups not done

- M1 visual pass was partial (dark only, 1600x1200 and 640x480 at the default sidebar; Xvfb/xdotool missing). Light scheme, 1280x800 and long-chip states unchecked; the label is cut at 640x480 as the user's Q1 answer allows. Screenshots in `visual-pass/`.

- With three long chips at the smallest window the Changes button can get no room (flat row, chips before button). A priority layout would fix it.
- `ui::changes::tests::a_24_character_name_keeps_its_room_at_every_depth` failed in the scoped gate (cap 0); unrelated to the header, check on main.

None yet. Pre-existing failure on main: ui::changes::tests::a_24_character_name_keeps_its_room_at_every_depth.

## Tasks unit

- Review round 1 (tasks, snapshot b95ec35da:421a1eab3): F1 Verify targets fixed (verified: gates are a module of `layout_snapshot`), F2 checklist ticked, F3 note added; fixed, prose only. speckit-analyze HIGH findings I1-I3, C1 fixed.
