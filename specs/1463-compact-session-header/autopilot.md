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

M2 in progress, branch reset to origin/main 304bfb4ff (M1 merged as PR #1487, merge SHA 304bfb4ffa97076810692dcbe28c9a135eb63cbf; the old local M1 commits were dropped). Done, in one WIP commit (not pushed): `terminal.rs` (`SwitchOffer`, `switch_offer_of`, `strip_switch_offer`; `pane` no longer stacks the switch row; tiled `terminal_switch_strip` kept), `session_area.rs` (`switch_slot`, `status_cluster(model, switch, r)`, `SWITCH_CAPTION_MAX_WIDTH`), five new gates at the end of `tests/gates/session_area_fits.rs` (two-rows, empty slot, long caption pass; `the_filled_strip_and_the_header_fit_at_the_smallest_window` is RED, see escalation). Not done: T009 unit test, T011 tiled-pane test, T015 (stability marker `fn status_cluster<'a>(` is still the old one and the `the_pane_area_draws_no_bar_under_the_terminal` end marker was set to a guess `"let body"` — fix), T016 fixtures (`UPDATE_LAYOUT_SNAPSHOT=1`), T017 docs, T018 (is the red gate), tick tasks, cycle-log, reviews A and B, visual pass, gate, PR. Known gate red unrelated: `ui::changes::tests::a_24_character_name_keeps_its_room_at_every_depth`.
Decision made: two rows above the terminal measure 36 (header) + 64 (strip: 48 dp icon-button target + padding, unchanged by this feature) = 100 px, not 90; gate bound `TWO_ROWS_MAX` = 100 (spec says "about 90"; amend FR-009/SC-001 wording to "about 100" in the PR).

## Open escalation

M2 (category 6, the plan proved false): at the smallest window beside a 300 px sidebar the session area is 334 px wide. The strip's fixed controls (New session 48, split buttons 48, restart 51, status words 19 to 40, spacings about 40) leave about 112 px for the switch slot before the tab region reaches 0, yet the outlined switch button "Open in structured view" alone is 196 px. Measured with the slot in place: tab region 0 wide, New session squeezed to 11 or 0 wide, split buttons clipped (0 to 1.5 wide), restart squeezed to 22 of 51 wide (stopped session with 3 queued messages). So FR-012 ("switch control, restart or any fixed control is not clipped") and US3 scenario 2 (nothing outside the area) cannot both hold with the full-label button; plan R5 assumed only the caption and the tabs give way. Questions: how should the strip give way at that width? Option A (Recommended): the switch button's label is cut with an ellipsis (full label in its tooltip) after the caption and the tabs have given way, as the user chose for the Changes label in M1; restart, New session and splits stay whole; amend FR-012 and H10 accordingly. Option B: the switch button becomes icon-only (switch glyph, label in tooltip) below a strip width. Option C: leave the strip clipping at 640 with sidebar open (drop the T018 inside check at that window).

## Follow-ups not done

- M1 visual pass was partial (dark only, 1600x1200 and 640x480 at the default sidebar; Xvfb/xdotool missing). Light scheme, 1280x800 and long-chip states unchecked; the label is cut at 640x480 as the user's Q1 answer allows. Screenshots in `visual-pass/`.

- With three long chips at the smallest window the Changes button can get no room (flat row, chips before button). A priority layout would fix it.
- `ui::changes::tests::a_24_character_name_keeps_its_room_at_every_depth` failed in the scoped gate (cap 0); unrelated to the header, check on main.

None yet. Pre-existing failure on main: ui::changes::tests::a_24_character_name_keeps_its_room_at_every_depth.

## Tasks unit

- Review round 1 (tasks, snapshot b95ec35da:421a1eab3): F1 Verify targets fixed (verified: gates are a module of `layout_snapshot`), F2 checklist ticked, F3 note added; fixed, prose only. speckit-analyze HIGH findings I1-I3, C1 fixed.
