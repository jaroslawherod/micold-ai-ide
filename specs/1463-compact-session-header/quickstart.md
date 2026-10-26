# Quickstart: Compact session header

## Part A. Automated
1. `mise run test-core`
2. `cargo test -p micold-client --test session_area_stability --test layout_snapshot` (through `mise run gate`): header one line, strip order, restart last, stable child list, tiled geometry relative to the pane-area origin unchanged (absolute fixtures re-recorded).
3. `mise run gate` before pushing.

## Part B. Visual pass (needs a display; use the `visual-pass` skill)
For 640x480 with the sidebar at 300 px and 1280x800, light and dark:
1. Claude terminal session, single pane: two rows above the terminal (about 90 px); title, chips, Changes on one line; "Open in structured view" in the strip before restart.
2. Held queue present, long caption at 640x480: caption cut with ellipsis, switch and restart whole.
3. Press Changes: Review tab shows; the switch control and caption disappear; close it: they return, restart did not move.
4. Long title, long branch, pull request chip: title cut first.
5. Tiled panes: each pane keeps its switch row.
6. Stopped session: restart still last.
Record screenshots under `visual-pass/`.
