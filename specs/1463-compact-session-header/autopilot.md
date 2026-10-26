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

M2 (T002, T009-T018) implemented and green locally; branch is 3 local commits ahead of origin/main 304bfb4ff (M1 merged as #1487, merge SHA 304bfb4ffa97076810692dcbe28c9a135eb63cbf), nothing pushed, no PR. Escalation answered by the user: Q1 = Option A (strip gives way by rank: caption, then switch button label, then status words; restart, New session, splits whole; tab region may be 0; FR-012 and H10 amended), Q2 = "about 100 px" (TWO_ROWS_MAX 100; FR-009/SC-001 reworded; say so in the PR).
Done and uncommitted at handover (commit first): `micold-material` `GiveWayRow` (pub, give_way.rs); `ui/terminal.rs` (`SwitchOffer`, `switch_offer_of`, `strip_switch_offer`, `switch_button_shrinking`, unit test T009); `ui/session_area.rs` (strip row is `row![tabs, GiveWayRow[cluster, new, splits]]`, cluster is `GiveWayRow[words, switch slot, restart]`, slot `GiveWayRow[caption, button]`); gates in `tests/gates/session_area_fits.rs` (new strip paths, `claude_terminal`, two rows, empty slot, long caption, filled strip, review shown/closed restart, tiled row (weak: height heuristic)); `covered_states.rs` paths (`SESSION_STRIP_NEW_SESSION`, `SESSION_STRIP_SPLIT`, `with_review_of` pub); `session_area_stability.rs` (pane marker, `switch_slot` builder, restart path `[1,0,2]`); fixture `layout_snapshot.txt` re-recorded; docs (terminal-panes.md, worktrees-and-sessions.md), 1126 contract H10 note, spec FR-009/FR-012/SC-001; tasks.md T002, T009-T018 ticked. `cargo test -p micold-client --test layout_snapshot --test session_area_stability` green (158 + 16).
Next: (1) commit; (2) one compiler warning "call to `.clone()` on a reference" seen in the test build log (find with cargo clippy, fix); (3) tdd/cycle-log.md for M2 (honest note: gates written after the implementation, red runs observed: tab region 0 and restart squeezed at 640+300 with a plain Row; New session clipped; stability restart path); (4) strengthen the tiled-row gate if cheap; (5) run scoped gate + review A (code-review high) in its shadow, review B (conformance, sonnet), visual pass via autopilot-worker; (6) full gate (`mise run gate`; macOS cross-check not needed, no cfg change); (7) PR: push, body notes Q1/Q2 amendments and FR-009 wording; if the push hook refuses only for `ui::changes::tests::a_24_character_name_keeps_its_room_at_every_depth`, return ESCALATE asking the user to push (CI decides). PR title `feat(1463): switch control in the tab strip (#1463)`, body ends `Refs #1463`.

## Open escalation

None.

## Follow-ups not done

- M1 visual pass was partial (dark only, 1600x1200 and 640x480 at the default sidebar; Xvfb/xdotool missing). Light scheme, 1280x800 and long-chip states unchecked; the label is cut at 640x480 as the user's Q1 answer allows. Screenshots in `visual-pass/`.

- With three long chips at the smallest window the Changes button can get no room (flat row, chips before button). A priority layout would fix it.
- `ui::changes::tests::a_24_character_name_keeps_its_room_at_every_depth` failed in the scoped gate (cap 0); unrelated to the header, check on main.

None yet. Pre-existing failure on main: ui::changes::tests::a_24_character_name_keeps_its_room_at_every_depth.

## Tasks unit

- Review round 1 (tasks, snapshot b95ec35da:421a1eab3): F1 Verify targets fixed (verified: gates are a module of `layout_snapshot`), F2 checklist ticked, F3 note added; fixed, prose only. speckit-analyze HIGH findings I1-I3, C1 fixed.
