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
| Close the focused pane | Ctrl+Shift+W | Cmd+Shift+W |
| Focus the pane to the left / right / above / below | Ctrl+Shift+Arrow | Cmd+Shift+Arrow |

A new pane takes focus. Moving focus towards an edge with no pane there leaves it where it is. A
split that is refused (6 panes, or a pane too small) shows its reason next to the panes. The
existing Ctrl+Shift+E (release focus), T (new terminal), C and V (copy, paste) are unchanged.

## Resize, close and swap

**Resize.** Drag the line between two panes; both resize as you drag, and neither goes below a
minimum usable size. Double-press the line to give the panes on its two sides equal sizes again.
The terminals are told their new size when you let go.

**Close.** The × in a pane's header closes that pane, and so does Ctrl/Cmd+Shift+W for the focused
one. The pane next to it takes the freed space and focus moves to it. Closing only removes the
pane: the terminal it showed keeps running and stays in the tab strip and the sidebar, and you can
put it in a pane again. The last pane cannot be closed; a message next to the panes says so.

**Swap.** Drag a pane's header onto another pane. The two panes swap their terminals, and the
outlined pane shows where it will land. Focus stays with the terminal you moved.

## Each pane has its own size

A terminal is sized to the pane it is in, so splitting does not resize the terminals beside it.
A terminal that is no longer shown keeps its last size until it is shown again.

## Your layout is remembered per project

Each project keeps its own panes: the splits, where the dividers are, which terminal each pane
shows and which pane has the focus. Quit and reopen the app and every project comes back as you
left it, and switching projects switches layouts. A pane whose terminal no longer exists (its
session was closed) comes back empty so you can pick another; the rest of the layout is kept. If a
saved layout cannot be read, for example because it was written by a newer version of the app, the
project opens with one pane instead. Forgetting a project forgets its layout.

## Drop files and paste screenshots

Drag one or more files or folders from your file manager onto a terminal and their full paths
appear at that terminal's prompt, quoted for its shell, in the order you dropped them and separated
by single spaces. Nothing is sent: the paths wait at the prompt, after anything you had typed,
until you press Enter.

- The paths go to the pane you drop on, whether or not it has the focus.
- Names with spaces, quotes, `$` or other awkward characters are quoted so the shell reads each
  path as one argument. If a shell cannot take a name at all (for example a `%` in a name under
  `cmd`), the file is left out and a notice names it; the other files are still inserted.
- Dropping on a pane with no terminal, or on a terminal whose process has exited, inserts nothing
  and shows a notice saying why.

### Paste a screenshot into an AI session

Copy an image (a screenshot, say) and paste into an AI session: the image is saved as a PNG file
and its quoted path appears at the prompt, with nothing sent until you press Enter.

- The file goes in the session's worktree under `.micold-pasted/`, which stays out of `git status`.
  A session with no worktree, or one whose worktree can't be written to, keeps it in the app's
  data folder instead.
- Each paste gets its own file; two pastes never overwrite each other.
- If the clipboard holds text, even together with an image (a copied spreadsheet range, say), the
  text is pasted as always and no file is made.
- In a regular terminal, paste works as before.
- In a session that runs in the sandbox, paths are inserted as the container sees them; see
  [Dropping files into a sandboxed session](sandboxed-daemon.md#dropping-files-into-a-sandboxed-session).
- If the image can't be read or saved, nothing is inserted and a notice says why.
