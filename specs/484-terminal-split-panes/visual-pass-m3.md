# Visual pass, M3 (quickstart Part B step 6)

Real client and daemon (debug build of this branch) on a private Xvfb, lavapipe, 1500x900, with a
seeded git project and two live sessions. Driven with xdotool.

| Check | Result |
|---|---|
| Header: split, split, close (×) buttons sit in a row, no collision | pass |
| Divider drag: panes resize live, the other pane keeps its minimum | pass |
| Double press on the divider: equal sizes | pass |
| Header drag onto another pane: target pane outlined while dragging; terminals swap, focus follows the moved terminal | pass |
| Header ×: sibling fills the area, both sessions stay in the sidebar | pass |
| Ctrl+Shift+D then Ctrl+Shift+W (chords) split and close | pass |
| Closing the last pane (chord): "The last pane stays open." above the terminal | pass |

Found and fixed during the pass:

- The header took half of its pane: the accent strip had `Length::Fill` height, which made the whole
  header row fill. The strip now has a fixed height (`ui/panes.rs`, `STRIP_HEIGHT`).
- A refused close of the last pane showed nothing, because the reason was drawn only by the
  multi-pane view. The single-pane view now shows it above the terminal (`ui/terminal.rs`).

Not checked: light/dark of the drop outline (it is the `primary` role, resolved per theme).
