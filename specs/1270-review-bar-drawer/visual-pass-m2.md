# Visual pass — M2 components (#1270)

**Date**: 2026-10-24 · **Tree**: branch `m2-1270` · **Route**: the showcase entries rendered in
process by iced's CPU rasteriser (tiny-skia, `micold_material::test_support::Mounted::drawn`), each
entry in a 1280 px wide frame (16 px padding), light and dark roles. Not a real display: this cloud
container has no lavapipe Vulkan driver and no xdotool, so the Xvfb route of the `visual-pass` skill
could not launch the showcase binary.

| Check | Result | Evidence |
|---|---|---|
| `ReviewBar`: Send review (n), ⋮ and Comments at the same place in every pose of one width (no comments, drafts, drawer open, sending, error shown) | pass | `visual-pass/m2-review-bar-{light,dark}.png` |
| `ReviewBar`: counts shorten — Full at 960, "3 drafts" at 520, hidden at 360; the error after the counts with its ×, nothing clipped or overlapping | pass | same |
| `SideSheet`: 360 px at the right of a 960 area, half of a 520 area; title, ×, rows inside; outline edge against the area | pass | `visual-pass/m2-side-sheet-{light,dark}.png` |
| Card: a short reply whole; a reply over 4 lines cut at 4 with "…" and Show all; a reply shortened past 4,000 characters with Show all | pass | `visual-pass/m2-card-replies-light.png` |
| Row whole-reply mode: reply wrapped whole; Show all under a shortened reply; Mark applied and Answer… on their own line inside a 240–340 px row | pass after fix | `visual-pass/m2-row-whole-reply-light.png`; the first render showed Answer… past a 340 px row's edge, so the actions moved under the reply (test `mark_applied_and_answer_only_on_a_sent_row_with_handlers` checks 240 px) |

Not judged here: colours of buttons and default text. The in-process render paints with the light
theme's default text colour and draws buttons at their first frame (pale), in both schemes and for
existing components alike, so contrast and the Comments toggle's selected layer need the M4 pass on a
real display (FR-024, quickstart §B).
