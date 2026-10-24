# Terminal panes

Split the terminal area to watch several terminals at once.

## The header above the terminal

While a session is shown, a header above its terminal names the worktree it runs in:

- **Title**: the worktree's issue title, else its branch, else its name; the same title its
  Overview card shows. The **Default** entry's title is *Default*.
- **State**: the worktree's Overview state (*Needs action*, *Needs attention*, *Working*, *Done* or
  *Idle*), with its icon and colour. Point at it to read why.
- **Branch**: the short form of the branch, such as `fix/1117` for `fix/1117_ci-red-on-main`. Point
  at it for the full branch. No branch, no chip.
- **Pull request**: the branch's current pull request, its state and its checks, such as
  `PR #1118 · Open · checks failing`, the same reading the sidebar shows. No chip when the branch has
  no pull request or pull request status is off in Settings; an old reading says so in its tooltip.

The header follows the session you show, and changes when the sidebar does. Text that does not fit
ends in `…`; point at it to read it in full.

## The tab strip above the terminal

Under the header, one row of tabs switches what the terminal shows. Press a numbered tab to switch
to a shell, or `Ctrl+Shift+T` (`Cmd+Shift+T` on macOS) while the terminal has focus to open a new one; press the session's tab
to go back to the AI CLI. Nothing is drawn under the terminal.

- **Session tabs**: one per session of the shown worktree, the main session first, then its
  helpers, each named `<command> · <session name>`, such as `claude · main`. Point at one for its
  full name, whether it is the main or a helper session, and its status. The shown session's tab is
  its AI conversation: press it to show the AI CLI. Press another session's tab to show that
  session, as its sidebar row does.
- **Shell tabs**: the shown session's shell instances, numbered, right after its tab, with a close
  control each. Press one to show that shell.
- **"+"**: opens another shell instance of the shown session (`Ctrl+Shift+T`, or `Cmd+Shift+T` on
  macOS, while the terminal has focus).
- **Status and restart**: the shown session's status (*running*, *starting…*, *idle*, *exited*,
  *failed after 3 attempts* and so on) sits after the tabs. While the shown process has stopped,
  its tab wears the stopped mark and a **restart** control appears beside the status; press it to
  run the process again. Right-click the shown session's tab or a shell tab for its menu
  (*Restart* while it is stopped, *Close* on a shell tab); another session's tab has no menu.
- **New session**: starts another session in the shown worktree, as the sidebar's *Start another
  session…* does, confirmation included. Cancel changes nothing; confirm adds the new helper's tab
  after the others and shows it.
- **Split buttons** at the right end: split the focused pane side by side or stacked.

When the tabs do not fit, the row scrolls with the mouse wheel rather than shrinking a tab. New
session and the split buttons stay at the right end. A faded edge says more tabs lie beyond it, and
it takes the accent colour when the marked tab is the one out of view.

## Split and pick a terminal

Every pane has a header with two buttons: **split side by side** and **split stacked**. The same
two buttons sit at the right end of the [tab strip](#the-tab-strip-above-the-terminal) and act on the focused pane. The header and the
strip are drawn once above all the panes. A new pane opens on a
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

With the terminal focused, the pane chords act on the focused pane. The app takes every chord
below: the terminal never receives them, so they shadow the same keys with Shift in a terminal
program. The Overview, quick-switcher and location chords work whether or not a terminal is focused.

| Action | Linux / Windows | macOS |
|---|---|---|
| Split side by side | Ctrl+Shift+D | Cmd+Shift+D |
| Split stacked | Ctrl+Shift+H | Cmd+Shift+H |
| Close the focused pane | Ctrl+Shift+W | Cmd+Shift+W |
| Focus the pane to the left / right / above / below | Ctrl+Shift+Arrow | Cmd+Shift+Arrow |
| Open or close the [worktree Overview](overview.md) | Ctrl+Shift+O | Cmd+Shift+O |
| Enter or leave the Overview's [full screen](overview.md#full-screen) (only while the Overview is open; not reserved from a focused terminal) | Ctrl+Shift+F | Cmd+Shift+F |
| Quick-switcher (jump to a location, session or project) | Ctrl+Shift+P | Cmd+Shift+P |
| Previous / next location ([Default and the worktrees](worktrees-and-sessions.md)) | Ctrl+Shift+, / Ctrl+Shift+. | Cmd+Shift+, / Cmd+Shift+. |
| Previous / next session of the current location | Ctrl+Shift+[ / Ctrl+Shift+] | Cmd+Shift+[ / Cmd+Shift+] |
| Previous / next project (unavailable projects are skipped) | Ctrl+Shift+PageUp / Ctrl+Shift+PageDown | Cmd+Shift+PageUp / Cmd+Shift+PageDown (laptop: Cmd+Shift+Fn+Up / Cmd+Shift+Fn+Down) |

A new pane takes focus. Moving focus towards an edge with no pane there leaves it where it is. A
split that is refused (6 panes, or a pane too small) shows its reason next to the panes. While a
terminal has focus, Ctrl+Shift+E releases focus, Ctrl+Shift+T opens a shell instance, and
Ctrl+Shift+C and Ctrl+Shift+V copy and paste; on macOS they are Cmd+Shift+E, Cmd+Shift+T, Cmd+C and
Cmd+V.

## Resize, close and swap

**Resize.** Drag the line between two panes; both resize as you drag, and neither goes below a
minimum usable size. Double-press the line to give the panes on its two sides equal sizes again.
The terminals are told their new size when you let go.

**Close.** The × in a pane's header closes that pane, and so does Ctrl/Cmd+Shift+W for the focused
one. The pane next to it takes the freed space and focus moves to it. Closing only removes the
pane: the terminal it showed keeps running and stays in the [tab strip](#the-tab-strip-above-the-terminal) and the sidebar, and you can
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
- Pasted images are temporary. Deleting the session removes its pasted images, and the next start of
  the app removes any left behind by a session that no longer exists. Files you dropped are never
  deleted.
