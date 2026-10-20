# Visual pass M1 (quickstart Part B steps 1-5, 8, 9)

Run on private headless sway with lavapipe (no Xvfb/xdotool); client and daemon built from this worktree.

| Step | Result | Evidence |
|---|---|---|
| 1 | Pass | No inline form; Add daemon opens dialog with scrim; outside click does not reach the page. |
| 2 | Pass | Fields prefilled; Save daemon closes dialog, row shows new name. Page Save independence not judgeable from pixels. |
| 3 | Pass | Cancel, Escape and outside click each leave stored name unchanged on reopen. |
| 4 | Pass | Empty name keeps dialog open; "This field is required" beside Name and Container name. |
| 5 | Pass (caveat) | Confirmation shows inside dialog; No and Escape keep edits; Yes closes. Daemon was Unreachable (podman unavailable), not truly running. |
| 8 | Pass (after fix) | First run failed: the form body overdrew the action row below 700x493. Cause: `expand` (animation.rs) clipped to its own bounds, which replaces the enclosing scroll clip; now cut to the viewport (regression test `expand_clipping`). Re-run at 700x493 (client 696x464): Save daemon and Cancel visible and the body scrolls above them, at scroll top and scrolled down; also fine at ~750x566. |
| 9 | Pass | Scrim, surface and action row look right in light and dark. |

Observations: edit dialog title follows the Name field while typing; Escape on Settings with no dialog leaves Settings (pre-existing status unchecked).

Screenshots (not copied, in scratchpad): b1-add-dialog, b4-empty-name, b5-confirm, b8-min-window-FAIL, b8-700x493-FAIL, b9-light.
