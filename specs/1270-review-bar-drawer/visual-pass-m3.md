# Visual pass — M3 wiring (#1270)

**Date**: 2026-10-25 · **Route**: the window (`ui::view_with`) of the covered state
`session-area-review-shown`, rendered in process by iced's CPU rasteriser (tiny-skia,
`micold_material::test_support::Mounted::drawn`) at 1280×800, light and dark; a scratch test, not
committed. M3 changes only the old panel's glue (the bar and the drawer are drawn in M4), so this
checks that glue.

| Check | Result | Evidence |
|---|---|---|
| A new Review tab shows the panel closed (FR-011, D19): the header carries **Review (0)**, Clear sent, Discard drafts… and Send review (0); the diff takes the panel's room | pass | `visual-pass/m3-closed-light.png` |
| **Review (n)** opens the panel; the counts line, then the chips **Drafts n** / **Replies n** (selected one marked), then the selected tab's rows | pass | `visual-pass/m3-full-drafts-dark.png`, `visual-pass/m3-full-replies-light.png` |
| Drafts rows carry Edit/Delete; Replies rows the status and the reply; counts "2 drafts · 1 sent · 1 answered" | pass | same |
| With no comment, "No review comments yet" said once (the counts line is left out) | pass after fix | first render said it twice (counts line and empty text); `ui/changes.rs` now drops the counts line when the scope is empty |

Seen and not M3's: a draft row's **Delete** is clipped at the panel's right edge and the list pane's
**Files** chip is cut at this width; both come from code M3 leaves as it is (the panel's column widths and
`ReviewPanelRow`). M4 replaces the panel by the drawer (`ReviewPanelRow` in a 360 px `SideSheet`,
checked in M2's pass). Button colours are pale in the in-process render (first frame), as M2 noted.
