# Contract: header and strip layout (amends 1126 H9, H10, S7)

## H9'. Header structure
`row![ title, chips, changes_button ]`, spacing `MD`, padding `LG` horizontally and `XS` vertically, `align_y(Center)`. Title `Length::Fill`, `Ellipsized` at `TypeRole::Title`, in a `Tooltip`. Chips: state chip always, then branch, then pull request chip when present; each `Tag` capped at `CHIP_MAX_WIDTH`. Changes button: compact, `Shrink`, never narrower than it asks. Child lists never vary. The line is no taller than the chips plus padding; the header is about 40 px or less at default scale.

## H10'. Fit
As H10 (640x480 with 300 px sidebar, and 1280x800; light and dark; nothing overlaps or leaves the session area). Shortening order: title first, chips capped. The Changes label is whole for three-digit counts at 640x480; four-digit line counts are cut with an ellipsis there (tooltip whole), after the title is at its minimum. At 1280x800, five-digit file and six-digit line counts are whole.

## S7'. Strip cluster
Left to right: scrolling tabs (Fill), status words, switch slot, restart (when stopped), add control, split buttons. The switch slot is always one child: caption (`Ellipsized`, capped, full text in tooltip) then the switch button, or an empty container. Present only for a single pane showing a Claude terminal session's primary terminal, with a switch entry, while the Review tab is not shown. Tiled panes keep the per-pane switch row. Restart is the last control of the status cluster.

## Rows above the terminal
Header, strip: 2 rows, for every session, single pane.
