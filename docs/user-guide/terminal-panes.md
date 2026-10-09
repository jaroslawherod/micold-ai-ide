# Terminal panes

Split the terminal area to watch several terminals at once.

## Split and pick a terminal

Every pane has a header with two buttons: **split side by side** and **split stacked**. The same
two buttons sit in the bar under the terminal and act on the focused pane. A new pane opens on a
terminal no other pane shows. When every terminal is already shown, the pane is empty and lists the
terminals you can put in it: one choice fills it.

Choosing a terminal from the tab strip or the sidebar that another pane already shows focuses that
pane; a terminal is never shown twice.

An area holds at most 6 panes. A seventh split is refused with a message next to the panes, which
goes away on your next pane action. A terminal whose process exited shows its state (for example
"exited") in its own pane header; the other panes keep running.

## Focus

Exactly one pane is focused. Its header is filled with the accent container colour and starts with
an accent strip, and its terminal has the focus ring. Keys go to the focused pane's terminal only.
Pressing in another pane focuses it; that press only moves focus and is not sent to the program in
the pane. Selecting and scrolling belong to the focused pane.

## Keyboard shortcuts

With the terminal focused, these act on the focused pane. The app takes them: the terminal never
receives them, so they shadow the same keys with Shift in a terminal program.

| Action | Linux / Windows | macOS |
|---|---|---|
| Split side by side | Ctrl+Shift+D | Cmd+Shift+D |
| Split stacked | Ctrl+Shift+H | Cmd+Shift+H |
| Focus the pane to the left / right / above / below | Ctrl+Shift+Arrow | Cmd+Shift+Arrow |

A new pane takes focus. Moving focus towards an edge with no pane there leaves it where it is. A
split that is refused (6 panes, or a pane too small) shows its reason next to the panes. The
existing Ctrl+Shift+E (release focus), T (new terminal), C and V (copy, paste) are unchanged. A
close shortcut arrives with closing panes.

## Each pane has its own size

A terminal is sized to the pane it is in, so splitting does not resize the terminals beside it.
A terminal that is no longer shown keeps its last size until it is shown again.
