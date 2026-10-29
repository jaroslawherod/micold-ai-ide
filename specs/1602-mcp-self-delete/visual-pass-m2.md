# Visual pass M2 (#1602): delete prompt loss list

- Result: PASS, with one cosmetic note. Quickstart Part B steps 1, 2 and 6 (loss list, not the live MCP flow).
- Route: no Xvfb, xdotool, or sway on the host. The full window was drawn headless through `support::layout::runtime_frame` (tiny-skia, reference fonts, light scheme, 1280x800) with a scratch test (since deleted). The prompt is a `Prompt` fixture: "planner" deletes "w1" with `stop_sessions` and `delete_branch`, `self_targeted` true. Real MCP flow not driven.
- `visual-pass-m2-small.png` (2 files, 1 commit): headline, muted 60-second text, "The session that asked ends after you allow this.", "Uncommitted changes (2)" with 2 rows, "Unpushed commits (1)" with 1 row, Allow and Deny. Nothing clipped or overlapping.
- `visual-pass-m2-60.png` (50 rows plus 10 more, heading says 60): the file list is a fixed-height scroll area (about 9 rows) with a scrollbar. The dialog grows 54 px over the small one, and the commit section and both buttons stay inside the window.
- `visual-pass-m2-60-scrolled.png` (wheel scrolled to the end): the last rows and "… and 10 more" are visible, with the same layout and buttons.
- Note: the "… and 10 more" row is below the fold until scrolled, so it is not visible at rest. The scrollbar thumb overlaps the last character of the longest rows (".rs" at the right edge). Both are cosmetic.
