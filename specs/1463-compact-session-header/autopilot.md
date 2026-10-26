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

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

M1 (category 6, plan false). At the smallest window (640 wide, sidebar 300, so the area is 334 px, 302 inside the header padding) the one-line header cannot hold what FR-002 and US1 scenario 3 promise: `Changes · 12 files · +345 −67` asks 251.5 px, one chip is about 113 px, MD spacing 12 each, so even with the title at 0 the line overflows by about 90 px (more with branch and PR chips). Measured on the work tree: with the Changes button laid out first (outer `row![lead, changes]`, `lead = row![title Fill, chips]`) the title goes to 0 and the chips go to 0 wide. Also research R2 is false: `material::Button::padding` cannot shrink the button, its height is `Fixed(density::BUTTON_BASE)` 40 dp; I added `Button::density(step)` (default `STANDARD`) and use `DENSE` (28 dp) for Changes. Chips are 15.6 px tall, so "Changes no taller than the chips" (T003) is unreachable; gate asserts it is no taller than the title line (28 dp) and the header at most 40 px. Question for the user: what gives way at the smallest window after the title? Option A (Recommended): the Changes label, cut with an ellipsis (tooltip whole), after the title; chips keep their capped width; amends FR-002, scenario 3 and H10 so the three-digit width guarantee holds only where the line has room. Option B: the chips collapse (state chip invisible) so the button keeps its width. Option C: below a width the header wraps to two rows (varies the child list; breaks FR-001, FR-011). WIP uncommitted work is in the tree.

## Follow-ups not done

None yet. Pre-existing failure on main: ui::changes::tests::a_24_character_name_keeps_its_room_at_every_depth.

## Tasks unit

- Review round 1 (tasks, snapshot b95ec35da:421a1eab3): F1 Verify targets fixed (verified: gates are a module of `layout_snapshot`), F2 checklist ticked, F3 note added; fixed, prose only. speckit-analyze HIGH findings I1-I3, C1 fixed.
