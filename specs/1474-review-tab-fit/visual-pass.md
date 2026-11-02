# Visual pass: quickstart Part B (1474)

Date 2026-11-02. Branch fix/1474 at cb3910fbb. Real `micold-ai-ide` + `micold-daemon` debug build
(client and daemon built together, pinned copies, daemon handshake confirmed), private headless
sway (pixman) + lavapipe, private data home, pointer through a private `vptr.py` copy (plus a
right-click command). Not a real display or GPU. Fixture: a throwaway git repo as the project, its
**Default** entry (project root) reviewed with 10 changed files (+42 -48) and one draft comment.
No linked worktree was used (the session hook forbids `git worktree add`); the Default entry
shares the Review tab code. Window sizes are content sizes (640x480 etc., borders excluded).
Screenshots: `specs/1474-review-tab-fit/visual-pass/`.

## Verdicts

| Step | Dark | Light | Notes |
|---|---|---|---|
| 1 | pass | n/a | 1000 expanded, live resize to 640: sidebar became the strip, main widened. |
| 2 | pass | pass | See below. Loading placeholder not captured. |
| 3 | pass | n/a | See below. Layout concern (O1). |
| 4 | pass | n/a | Hidden at 1000, at 640 (strip) and back at 1000 (no sidebar). |
| 5 | pass | n/a | Overview full screen at 640, exit: strip. At 1000 expanded, exit: expanded. |
| 6 | looks sane | looks sane | No baseline build compared ("as before" is a judgement, not a diff). |

Step 2 at 640x480: the diff header's chips (Unified, Side by side, Whole file, collapse chevrons,
Mark reviewed) wrap onto rows and all sit inside the header, none clipped. The comments drawer:
title, close button, Drafts tab and the row with its Edit and Delete buttons are inside it; the
buttons are reached by scrolling the drawer and Edit responds to a click (jumps to the comment).
The tree pane: filter field, chips and file rows stay inside the pane and scroll within it. Same in
light and dark. Not exercised: the "diff loading" placeholder (the diff loads too fast to catch).

Step 3 sequence at 640 -> press show (expanded) -> 660 stays -> 720 stays -> 650 collapses to the
strip ("Show sidebar" tooltip).

## Observations (not defects against the quickstart wording)

- O1 (step 3 state, 640 wide with the sidebar expanded, main area about 420 px): the diff
  chips are clipped or squeezed: "Unified" is cut at the right edge and "Side by side" / "Whole
  file" wrap to 2-3 lines inside pills that are too short for them; the diff itself is not visible.
  This is the user-chosen expanded-over-narrow state, which the quickstart does not list under step
  2, but it is the same kind of "part squeezed out of its box" the issue is about. See
  `step3-sidebar-640-660-720-650.png` (top left and top right).
- O2 (640x480 with the strip): the fixed header (title row, tab bar, "Project root" heading row)
  takes about 320 of 480 px, leaving the file tree about 35 px and the diff about 50 px of visible
  height. Everything is inside its box, but the working area is tiny. "Project root" and its
  subtitle are ellipsised to "Projec..." / "Uncommitt...".

## Not covered

Mid-animation frames (the sidebar slide), perceived smoothness, a baseline comparison for step 6,
the diff-loading placeholder, and a linked worktree entry (Default entry used). Keyboard shortcuts
did not reach the app through wtype in this setup (Ctrl+Shift+O, Escape), so the Overview was
opened from the sidebar instead.
