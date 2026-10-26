# Reviewing a worktree's changes

The **Changes view** lists every file a worktree has changed, so you can see what a session did
before you merge it or tell the session what to change next.

## Opening the Changes view

While a session is shown, the header above the terminal ends with a **Changes** button for the
worktree (or the project root) the session runs in. Press it to open the Changes view of that
location. Its label counts the changes the way the view's list does, with both **Committed** and
**Uncommitted** on:

- **Changes · 3 files · +42 −7**: three changed files, 42 lines added and 7 removed;
- **Changes · no changes**: nothing to review;
- **Changes** alone: the counts are being read, or cannot be read here (for example, the session
  service runs on another computer). The button still opens the view.

The counts follow your edits: when files change on disk, they are read again, and while that read
runs the button shows **Changes** alone rather than an old figure.

You can also right-click a worktree in the sidebar and choose **Review changes**. The **Default**
entry (the project root, see [Worktrees & Sessions](worktrees-and-sessions.md#the-default-entry-sessions-without-a-worktree))
has a right-click menu too, with **Review changes** as its one item.

**Review changes** on a worktree other than the one shown switches to that worktree: the session
you last looked at there becomes the current one (else its main session), and its review is shown
under that worktree's own header. On a worktree with no session, its review is shown under its own
header, with no session current.

The review opens as a **Review** tab in the location's tab strip, after its session and shell
tabs and just before **New session**, and is the shown tab, underlined. The view takes the place
of the terminal, with the header and the strip still above it and the **Changes** button drawn
selected while it is shown (hover it: *The review is shown*). Its own header names the entry. A
review is always shown under its own worktree's header and strip, never another's. A worktree with
no session shows a strip of the **Review** tab alone beside **New session**. While the Review tab
is shown, the status words and the split buttons keep their place in the strip, greyed out: they
act on the terminal, which is not shown. When you close the last session of a worktree whose review
you kept, its review is shown in the session's place.

The view's header names the entry over its base line, and beside them holds the **source
control**: a button that reads what the view shows, **Working tree** when it opens. Press it (or
focus it and press `Enter` or `Space`) to open its list: **Working tree**, with its **Committed**
and **Uncommitted** chips (see [Committed and uncommitted](#committed-and-uncommitted)), then the
worktree's [pull requests](#pull-requests), then **Files** (see
[Browsing the repository](#browsing-the-repository)). What is shown now is marked as chosen. Pick an
option to show it; the list closes. The arrow keys move through the options and chips, `Enter` picks,
and `Esc`, `Tab` or a click outside the list closes it without changing anything. Hover the control
to read its whole label and the base line. Below the header, the file list (or the Files tree) has
a fixed-width column to itself, and the diff takes the rest of the width: a wider window widens
only the diff. In a narrow window the control keeps its width: the entry's column narrows and its base line ends in `…`.

The Review tab carries two marks:

- a **badge** with the number of draft comments waiting to be sent (`1`…`99`, then `99+`); with
  no drafts there is no badge. It follows drafts added, deleted or sent in any window;
- a **dot** while the tab is not shown and the worktree's files have changed since you last looked
  at the review. Hover the tab to read both: *Review · 2 drafts / Changed since you looked*.
  Showing the tab reads the changes again and the dot goes.

The review is kept while you work elsewhere. Selecting a session (in the sidebar, with a session
chord or the quick switcher), switching to another worktree or project, or opening the Overview or
Settings puts the review aside without losing anything: the selected file, both scroll positions,
revealed lines, the whole-file toggle and the **Committed**/**Uncommitted** chips, the comment you are
writing, picked lines, the comments drawer, the chosen pull request and a browsed file all stay as
you left them. Each location keeps its own review. Press the **Review** tab, **Changes** or
**Review changes** to show it again exactly as left; while it is shown, pressing **Changes** or
the tab changes nothing.

Only the Review tab's **×** drops the review (it takes the place of the view's former **Close**
button), returning to the session shown before it; the next open starts afresh, with your
comments and the chosen pull request still there. The review is also dropped
when its worktree is removed (for example, deleted from another window) or its project is removed
from the app.

## What the list shows

The changed files are shown as a tree. Each folder that holds a changed file has a row of its own,
with its files and sub-folders indented below it; at each level folders come before files, and both
are in name order. A folder whose only content is one folder shares its row with it, so
`crates/core/src/review` is one row rather than four. Every folder is open when the view opens.

A folder row shows a chevron, the folder's name and, under it, `+a −r`: the lines added and removed
in all the files below it, so a closed folder still says how much changed inside. Select a folder row
to close it and hide everything below it; select it again to open it, with any sub-folder you closed
still closed. The folders you close stay closed while the view is kept, even when the list is read
again or you switch the source. Selecting a file in a closed folder some other way (from the diff's
**Open diff**, or by following a reference) opens the folders above it.

A file row starts with its review state (○ not reviewed yet, ◉ not reviewed and with draft
comments, ✓ reviewed; see [Marking files reviewed](#marking-files-reviewed)) and the file's own name,
in bold; the folders above it say where it is. Under the name:

- what happened to it: **Added**, **Deleted**, **Renamed** or **Mode** (only its permissions
  changed); a modified file has no tag. A file git does not track yet counts as **Added**;
- **Binary** or **Not text** when the file's contents are not readable text;
- a small bar and `+a −r`: the green part of the bar is the lines added and the red part the lines
  removed, in proportion to each other, and the whole bar is longer the more lines the file changed
  compared with the other files listed, so the large changes stand out. A binary or non-text file, a
  **Mode** change and a rename with no edits have no bar and no counts;
- how many comments you have written on the file and not sent yet (see
  [Commenting on lines](#commenting-on-lines)), when there are any.

Hover a file row to see its full path, and for a rename the path it had before. A name too long for
the list ends in `…`; deep folders stop indenting further before a file's name loses its room.

Above the tree, the **Filter files…** box narrows the list: type part of a path and only the files
whose path matches stay listed, inside the folders that hold them, with the matched characters
highlighted. It matches as **Open file by path** does (see
[Browsing the repository](#browsing-the-repository)), so `tests.r` finds `daemon/tests.rs`. While the
box holds text every folder holding a match is shown open; empty it — press `Esc` in the box, or its
**×** — and the tree comes back with the folders you had closed still closed. When nothing matches,
the list says **No changed file matches the filter** with a **Clear filter** button; the diff keeps
showing the file you had open, and that file is selected again once the filter is cleared.

Under the filter box, the sort chips order each level of the tree; folders always stay before files
and the tree is never flattened:

- **Path** (the default): by name.
- **Size**: the largest change (lines added plus removed) first; a folder counts the files below it.
- **Unreviewed first**: files you have not marked reviewed first, and folders holding any such file
  before folders holding none. It is offered once the files' reviewed marks are known (see
  [Marking files reviewed](#marking-files-reviewed)).

The filter text and the chosen sort stay as they are while the view is kept, when the list is read
again and when you switch the source. Each window keeps its own.

A file changed both in a commit and again since is listed once, with the combined change. Long
lists scroll smoothly: only the rows on screen are drawn.

While the list is read, the view says **Reading changes…**. With nothing to list it says why:
**No changes against** `<branch>`, **No uncommitted changes**, or that both kinds are hidden.

## Pull requests

With **Show pull request status on worktrees** checked in [Settings →
GitHub](settings.md#show-pull-request-status-on-worktrees), the source control's list holds, under
a **Pull requests** heading, every pull request this worktree produced, open, draft, merged and closed,
newest first across all of them. Each row has the pull request's state mark, its check mark, its
`#number` and its title. The marks are the ones the
sidebar uses (see [Pull request status](worktrees-and-sessions.md#pull-request-status)): open,
draft, merged and closed each have their own shape, so a merged pull request is never mistaken for
a closed one. The check mark shows the result of the pull request's last commit: passing, failing
or still running. A pull request with no checks has no check mark. Merged and closed pull requests
show theirs too.

A worktree's pull requests are those of its current branch and of every branch created in it,
including branches that were merged and deleted since, and a branch name that had several pull
requests over time. The worktree's own git history says which branches were created there, so the
list survives restarting the app. Not listed: the repository's default branch, and a branch that
was only checked out in this worktree but created elsewhere (in another worktree, in the main
checkout, or on GitHub). Each worktree lists only its own branches, never another worktree's. When
the session service runs on another computer, only the current branch is listed.

The list is the same for a worktree with no session as for one with sessions, and it shows
only pull requests of this project's own repository. A pull request opened from a fork is not
listed.

The working-tree changes never wait for the pull requests: they appear first, and the pull
requests join the control's list when GitHub has answered. Until the first read finishes the list
says **Reading pull requests…** under the heading. A worktree none of whose branches has a pull request says **No pull requests from this
worktree yet**. When GitHub cannot be reached and no list has been read (no `gh`, not signed in, no
GitHub remote, offline), it shows one muted line, **Pull requests can't be shown for this
worktree**, and raises no error. The list has no pull-request part at all, neither heading nor line,
when the switch is off and when the entry is the project root. While GitHub cannot be reached after
a list was read, the last list and your selection stay; once that list is older than the sidebar's
stale threshold, **Possibly stale** shows under the heading. A long list scrolls inside the
control's list, which never grows past the room below the header.

### Reading a pull request's diff

Pick a pull request in the source control to read its diff: two actions from the working tree,
open the control and pick. The control then reads `#N · <title>`; a long title ends in `…`, while
the number stays whole and the full title is in the tooltip. The file list, the diff and the base
line show the pull request: **Pull request #N: base ... head**, as GitHub computes it, where *head* is that
pull request's own branch. Select a file as
usual; comments are kept per file and line as for any other diff. While the pull request is
chosen, the working-tree changes are not read, and nothing reads them behind your back.

Above the diff, a header names the pull request you are reading: its state and checks, its
number and title, and the base line. The **previous** and **next** buttons beside it step through
the list in the order it is shown, and the control's label follows; each is greyed out at its end
of the list. Picking a pull request or pressing a button while a diff is still loading shows the last one you chose, never an earlier one's diff.

Pick **Working tree** to go back, to the file and scroll position you left. Each entry remembers its own choice while the window is
open, so switching to another worktree and back returns to the pull request you were reading.

A pull request whose branch was merged and deleted still shows its diff: GitHub keeps it. If the
pull request is no longer listed after a refresh, the view returns to the working tree.

GitHub will not diff a pull request above 300 files or 20 000 changed lines. The pane then says
the pull request is too large to show here and offers **Open on GitHub**. A read that fails for
another reason says so in the pane, with no notice; pick it again to retry.

A pull request's files have their own marks and their own *n of m reviewed* (see [Marking files
reviewed](#marking-files-reviewed)), apart from the working tree's: marking `src/a.rs` in a pull
request leaves the working tree's `src/a.rs` as it was, and the other way round. A mark is made on
the file's diff in that pull request. When a new head commit changes a file's diff, the file is no
longer reviewed: it drops back to ○ or ◉ with a dot at the end of its row, as after an edit in the
working tree. A new head commit that leaves the file's diff the same keeps it reviewed. The marks
stay when the pull request is merged, closed or no longer listed, and show again whenever its diff
is shown; they go when you unmark them or the worktree is removed.

## Reading a diff

Select a file to see its diff beside the list. Each changed region starts with a header row,
`@@ -a,b +c,d @@`, followed by the name of the function or section it is in when git can tell. Under
it, every line has two numbers — its line in the old version, then in the new one — and then its
text:

- an **added** line has only a new number, a `+`, and a green-tinted background;
- a **removed** line has only an old number, a `−`, and a red-tinted background;
- an unchanged line around the change has both numbers and no tint.

A change of line ending alone (for example a file converted from `LF` to `CRLF`) shows as the same
line removed and added again; line endings themselves are never drawn. (With git's
`core.autocrlf` set to `true`, as is common on Windows, git itself ignores such a change, so the
file is not listed at all.) Long diffs scroll smoothly: only the rows on screen are drawn. While a
file's diff is read, the diff area says **Loading diff…**; the window keeps responding meanwhile.

### Unified or side by side

Two chips above the diff choose its layout:

- **Unified** — one column, the removed and added lines one under the other, as described above;
- **Side by side** — the old version on the left and the new one on the right, each line with its
  own number. Removed lines sit beside the added lines that replaced them; where one side has more
  lines than the other, the shorter side is left blank.

Both layouts show the same changed lines. The choice is kept: the next file you select opens in the
same layout, and so does the Changes view after you restart the app. Every open window follows
the latest choice.

### Syntax colouring

Files in a language the app recognises from their extension (Rust, Python, JavaScript, Markdown and
many more) are shown with syntax colouring, in both the light and the dark theme; the colours are
adjusted so that every token stays readable on the added and removed tints. Other files are shown
as plain text. Only the start of a very long line is coloured (its first 2,000 bytes, which is 2,000
characters of plain ASCII text); the rest of it is shown in the plain text colour.

### Reading hidden lines

A diff shows each change with a few lines around it. Where unchanged lines are left out (before
the first change, between two changes, after the last one), a row reads **N hidden lines**. Press it
and exactly those lines appear in place: numbered in step with the lines around them, coloured like
them, and in both the unified and the side-by-side layout. Press the row that opens the revealed
lines to hide them again and get the usual hunk back.

- Revealed lines are picked like the lines of the diff: click the line number, shift-click to
  extend, together with diff lines or alone. A pick on them names the same file and line numbers as
  a diff line would, so you comment on them in the same steps. Where picking is not allowed for the
  shown diff, revealed lines cannot be picked either.
- The view remembers what you opened for each file while it stays open. A refresh keeps the
  revealed lines that are still there and drops the ones that are gone.
- The lines come from the file as it is on disk, so they are the latest saved version, committed or
  not. Nothing is written: revealing lines changes no file and no review state, and it works
  offline.
- Added, untracked and deleted files, and files whose contents are not shown as text, have no
  hidden lines and no row.
- A file over the size limits (see *Large files*) shows its lines only after **Show diff**; its
  hidden lines are then available like any other file's.
- Reading a pull request, the lines are the pull request's head version of the file, not the
  worktree's, even when the worktree has moved on. See *Pull request files* below.

### Showing the whole file

Instead of opening hidden lines one row at a time, press **Whole file** beside the layout choice to
see every line of the selected file. Added and removed lines stay marked and in their place, the
numbers run on without a break, and the lines are coloured like the rest of the diff. Press it
again to go back to the hunks.

- The choice is kept for each file while the view stays open, and a refresh keeps it.
- An added or untracked file shows all its lines already, so it has no **Whole file** button. For a
  file with no text version to show (deleted, binary, or not readable as text) the button is
  greyed out, and hovering it says why.
- The lines come from the file as it is on disk, like the hidden lines above. A very long file may
  be left uncoloured rather than make the view wait.

### Pull request files

With a pull request selected, hidden lines and **Whole file** show the pull request's head version
of the file. The view reads it from this computer's copy of the repository when the head commit is
there, and from GitHub otherwise, so a head that was never fetched needs the GitHub command-line
tool signed in and a connection. While it is read the pane says **Reading file…**; if it cannot be
read (no connection, no sign-in, the file is missing at the head, or it is larger than 2 MB) the
pane says why above the diff, and the diff itself stays usable. Nothing is written.

### Binary and non-text files

A file whose contents are not readable text is listed, but its contents are never shown. The diff
area says what it is instead:

- **Binary file — not shown**;
- **Not UTF-8 text — not shown**, for text in another encoding;
- **Only the file mode changed**, when only its permissions changed.

A renamed file whose contents did not change says **The content did not change**.

### Large files

A file with more than 5,000 changed lines, or a version larger than 2 MB, is not drawn straight
away. The diff area shows how many lines were added and removed and a **Show diff** button; press it
to read and show the whole diff. The view remembers the choice for that file until it closes.

### Browsing the repository

Pick **Files** in the source control to list every file of the worktree instead of the changed
ones; the control then reads `Files · Working tree`, and picking **Files** again returns to the
changed files. Files lists tracked and untracked files, but not ignored ones. Open a folder to see
its files, then press a file to read it in the right pane, with line numbers and syntax colouring.
Type in **Open file by path** to narrow the list and pick a match; a path you type that is ignored
by git but exists in the worktree opens too.

Press **Back** above the file to return to what you were reading, with the same file, scroll
position and expanded lines. When the change modifies the browsed file, **Open diff** switches to
its diff. Picking lines for a comment is not yet available in a file you browse.

### Following a reference

A path or import string in a line of a diff, or of a file you browse, that names a file of the
repository is underlined. Press it to open that file in the right pane, at the line the text gives
when it gives one: `src/main.rs:42`, `src/main.rs#L42` and `src/main.rs(42)` open at line 42, and
an import such as `./util` opens `util.ts` or the folder's `index` file. Text that names no file of
the repository is never underlined, and **Back** returns to where you pressed. The links need the
repository's file list, which the view reads when it shows a diff; with a pull request shown they
name the files of its head commit. Following a name to a definition inside a file is not available.

With a pull request shown, **Files** stays available and lists the files of the pull request's head
commit (the control reads `Files · #N · <title>`); picking a pull request while Files is on keeps it
on, including the ones it does not change; untracked and ignored files of the worktree are never
listed. A file you open is the pull request's version, read from your repository when it has the
commit and from GitHub otherwise. When GitHub lists only part of a very large tree, a note says so;
a path you type still opens. Choosing another pull request, or the working tree, reads the file you
were browsing again at that version, says so if it does not exist there, and forgets where **Back**
led.

### Keeping up with changes

The view keeps watching the entry while it is shown. When a file in the entry is created, edited,
deleted or renamed, or a change is staged or committed — by you in a terminal or editor, or by
the agent — the changes are read again within about two seconds. A burst of edits (a build, an
agent rewriting several files) is read once when it settles, not once per file. Files git ignores
do not trigger a read, and the Default entry does not react to changes inside its worktrees.

What you are reading does not move while the agent works. Once the list has files, a read that
finds something new is held: the file list, its counts and the diff on screen stay exactly as they
were, scroll position included. Instead the view's header says **3 files changed since you looked**
(**1 file changed since you looked** for one), and the row of each of those files carries a dot
at its end (tooltip: "Changed since you looked"; the same dot a reviewed file gets when it changes,
see *Marking files reviewed*).
The count includes files the list does not show yet (a new file) and files whose changes were
undone; each file counts once however often it changes, and a file whose content goes back to what
you are looking at leaves the count. Press the notice to show the latest list and diff in one step:
the file you had selected stays selected while it is still listed, and the dots and the notice go.

You do not have to apply everything to look at one file. Selecting a file without a dot shows its
diff as before. Selecting a file with a dot reads and shows its latest diff, its row takes the
latest counts and loses its dot, and the other changes stay held. If a dotted file has no changes
any more (they were undone, or the file was removed), the diff area says so and the row keeps its
dot until you press the notice.

What you change yourself shows at once and leaves nothing held: switching the source between the
working tree and a pull request, or turning **Committed** or **Uncommitted** on or off. A list that
is empty, not read yet or failed is replaced at once too, since nothing on screen would move. A
read that fails while changes are held keeps the list and the notice, with no error.

The **Pull requests** list follows the same pull request status refresh as the sidebar: a pull
request opened, merged or closed while the view is open shows its new state within that refresh
interval (5 minutes), and a list refresh in the sidebar updates it too. If a refresh cannot reach
GitHub, the last list stays on screen with no error, and once it is older than ten minutes it is
captioned **Possibly stale** until a refresh succeeds.

A pull request opened from a new branch of the worktree appears at the next refresh, without
reopening the view. The pull request you are reading stays shown and keeps its place in the list
while the list changes; its header follows its state and checks. It is replaced by the working
tree only when it leaves the list.

The text you are writing in a comment box is kept while changes are held and when you apply them.
Picked lines are kept while they are still in the diff; when applying removes them, the pick is
dropped. While changes are held, a comment you save quotes the lines on screen.

A review put aside is not read while it is hidden. When you show it again it is read once more if
its files may have changed meanwhile (they changed while its location was shown, its location was
not shown, or the Overview or Settings covered it). That read is held like any other: you come
back to the list and diff you left, with **n files changed since you looked** in the header when
something changed meanwhile.

## Marking files reviewed

Once you have read a file's diff, press **Mark reviewed** in the diff header, beside **Whole
file**. The file's row turns to ✓ and the control reads **Reviewed**, shown pressed; press it
again to take the mark back. A file with draft comments shows ◉ until you mark it; marking it keeps
its drafts and their count on the row.

A mark is made on the file's change as it is now. When the file's change differs from the marked one
(an edit, a new commit that alters it), it is no longer reviewed: it drops back to ○ or ◉, with a
dot at the end of its row (tooltip: "Changed since you marked it reviewed"), and no longer counts in
*n*. Mark it again to review the new change. If you have not marked it again, undo the change, so
the file's change is the one you marked, and the mark comes back without pressing anything. Other
files changing, or new files appearing, leave a mark alone. Committing or staging a file unaltered
keeps it reviewed in an entry that has a base (see [The base line](#the-base-line)); in an entry
without one only uncommitted changes are listed, so a committed file leaves the list. A renamed file
starts unreviewed: its change is the one under its new name.

While changes are held (see [Keeping up with changes](#keeping-up-with-changes)), a reviewed file
that changed since you looked is not reviewed either: its row drops back to ○ or ◉ with the dot
(tooltip: "Changed since you looked, and since you marked it reviewed") and it does not count in
*n*, even when another window has already marked its new change. Holding changes writes nothing to
the session service, so other windows keep the mark. Select the file or press the notice to see
its latest change: a mark on that change shows the file reviewed again, and a mark on an older one
leaves it to be marked again, as above.

The view's header reads *n of m reviewed* with a bar filled to match: *m* is the number of files
the list shows, so it follows the **Committed** and **Uncommitted** chips, and *n* how many of
them are reviewed. With no files listed, or while marks are unavailable, the header shows neither,
and nothing else in it moves.

A mark covers the file's whole change, committed and uncommitted together, whatever the chips
show. **Mark reviewed** is unavailable, with a tooltip saying why, when:

- **Committed** or **Uncommitted** is off and the file has changes of the hidden kind: turn both
  on to see its whole change first. **Reviewed** stays pressable, so you can always take a mark
  back;
- the file's change has not been read yet, or could not be read (it can be marked once a later
  read succeeds);
- the session service is not connected. Rows then show ○ and ◉ only and the header shows no
  progress; comments keep working.

Marks are kept by the session service, beside the entry's comments: every window on the project
shows the same marks, and they are still there after a restart. They are removed with the
worktree.

## Commenting on lines

You can leave comments on the diff, the way you would in a code review, for the session to act on
later.

### Picking lines

Click a line's number to pick that line; its numbers fill in. To pick several lines in a row,
**Shift**-click another line's number on the same side: every line between the two is picked. A
pick is either removed lines or added and unchanged lines, never both; clicking a number on the
other side starts a new pick. An unchanged line counts as part of the new version. In the side by
side layout, click the number on the side you mean. Binary, non-text and large files that are not
shown cannot be commented on.

To pick a range in one gesture, press on a line's number and drag over the numbers of the lines you
want, up or down; the pick follows the pointer line by line. Rows without a number on the side you
pressed — lines of the other side, hunk headers, hidden-lines rows, comment cards — do not change
the pick, which ends at the last line of that side you passed. Letting go, even outside the diff or
the window, picks the range and opens the comment box under the line you let go on (see below).
Press **Esc** while dragging to cancel: the pick and the comment box go back to how they were
before you pressed, and the comments drawer, if open, stays open. Switching to another window or
leaving the Review tab while dragging cancels the same way. A press and release on one number
without moving to another line is a click, as above, however quickly you click. Dragging works with
a mouse and with a touchpad's press-and-drag.

### Writing a comment

With lines picked, press **Add comment** under them. A text box opens under the last picked line;
write the comment and press **Save** (or **Ctrl+Enter**, **Cmd+Enter** on macOS). **Cancel** closes
it without saving. **Save** stays unavailable while the box is empty.

Dragging over line numbers opens the box by itself: when you let go, it is open under the line you
let go on with the cursor in it, so you can type at once. If a new comment's box is already open,
the drag moves it to the new lines and keeps what you wrote. While you are editing a comment or
writing a note, a drag changes only the pick; that box and its text stay.

With [**Suggestion chips**](settings.md#suggestion-chips) on (Settings → Appearance; off by
default), three chips sit under every comment box, between the text and **Cancel**/**Save**:

| Chip | Puts in the box |
|---|---|
| *Rename* | "Rename this to " — finish it with the new name |
| *Add test* | "Add a test that covers this." |
| *Why?* | "Why is this needed?" |

In an empty box the chip's text becomes the comment; in a box with text it goes on a new line after
what you wrote. The cursor stays in the box, at the end, and the text counts as typed: **Save**
becomes available and you can edit it before saving. **Tab** from the text box reaches each chip in
turn; **Enter** or **Space** presses the focused one.

The comment then shows as a card under its last line, marked **Draft** until it is sent. The text you are
writing is kept while the list or the diff is read again.
If a change removes the lines you picked, the box moves above the diff with the note "The lines
this comment was on are gone. Pick lines to place it." and keeps your text; pick lines again and
**Save** adds the comment there. If the file itself leaves the list, the unsaved comment is closed.

### Editing and deleting

A draft comment's card has **Edit** and **Delete**. **Edit** opens the text box in its place with
the comment's text; **Save** keeps the new text. **Delete** removes the comment straight away.

A comment whose lines the diff on screen does not show — for example because you turned off the
kind of change it is on — is listed under **Not in the current diff** above that file's diff, with
the lines it is on.

### How comments follow their code

A comment stays with the code it quoted. When lines above it are added or removed — usually
because the agent edited the file — the card moves to the lines that now hold its code, and the next
review you send names those new lines.

- **Nearest occurrence.** If the quoted code now appears in more than one place, the comment goes to
  the one nearest where it was. If two are equally near, it is not guessed: the comment is marked
  **Outdated**.
- **Outdated.** A draft or sent comment is marked **Outdated** when its quoted code is gone — a line
  of it changed, or the file is gone. Its card stays at the closest place it can be, kept within the
  file's length, and it is still sent with the code as written. If an edit puts the code back in one
  place, the comment moves there and the mark goes.
- **Removed lines.** A comment on removed lines stays where it is, as long as the base version of
  the file has not changed.
- **Applied and answered comments** are never marked outdated. When their code changes — as an
  applied change usually does — they keep their status and reply and stay at their last place.
- **Line endings.** A difference in a line's ending (`\r\n` against `\n`) does not move or outdate a
  comment.

A comment on a pull request's diff is moved and stored as soon as the pull request is shown. A
comment on the committed changes alone is shown at the new place but is only stored there once you
open the entry's working tree. A comment you are writing follows its lines the same way, and keeps
its text.

### Where comments are kept

Comments belong to the worktree (or the Default entry) they were written in, and the session
service keeps them: they are still there after you restart the app, and every window open on the
project shows the same comments as they are added, edited and deleted.

### Commenting on a pull request

While a pull request's diff is shown, you comment on it the same way as on the working-tree diff:
click a line number (shift-click to extend the range), write the comment and save. Binary,
non-text and too-large files take no comments, as before.

- A comment belongs to the pull request it was written on and shows on that pull request's diff
  only, never on another pull request's or the working-tree diff.
- The comments drawer lists them under the pull request's number and title, after the
  working-tree comments, ordered by pull request number.
- Comments are kept per pull request on your computer, survive a restart, and go away when the
  worktree is removed.
- If a pull request leaves the list, its comments are kept and shown flagged "not in the list".
- **Clear sent** and **Discard drafts…** act only on the comments of what is shown: the pull
  request, or the working tree.
- Nothing is posted to GitHub. To have the agent act on a pull request's comments, see [Sending a
  pull request's comments](#sending-a-pull-requests-comments).

### Sending a review

A bar runs along the bottom of the Review tab, just above the window's status bar, whenever the
Review tab is shown. On its left it counts the comments of what is shown (the working tree, or the
pull request) in each status, leaving out a status with none: *3 drafts · 2 sent · 1 applied ·
1 answered*, or "No review comments yet". On its right are **Comments** (which opens the [comments
drawer](#the-comments-drawer)), **⋮** (the [overflow menu](#clearing-and-discarding)) and
**Send review (n)**. In a narrow window the counts shorten to the drafts alone (*3 drafts*), then go;
the three controls stay. They never move: opening the drawer, the number of drafts, **Sending…** or
an error leave **Send review (n)** exactly where it is.

**Send review (n)** sends the draft comments of what is shown — *n* of them — to the session
working in that worktree (or in the project root, for the Default entry) as one message. It is
filled while there are drafts to send; with none it reads **Send review (0)**, unfilled and
unavailable. It reads **Sending…** and is unavailable while a send of this entry is under way in any
window. Only draft comments are sent: a comment that was already sent is not sent again, so a later
send carries only the comments added since.

The message goes to the session running in the same entry; with several running there, to the one
you used most recently (typed into, or whose activity last changed). It never goes to a session of
another worktree. It is typed as one pasted submission, exactly as if you had pasted it and pressed
Enter. What it contains is described under [The review message](#the-review-message).

When it is delivered, a message reads "Sent a review of *n* comments to *session*", the comments
become **sent** (they stay on their lines, but can no longer be edited or deleted) and the bar's
counts move them from draft to sent. When it is not delivered — for example the session's terminal
cannot take a pasted text — an error reads "Couldn't send the review: *why*", the comments stay
drafts, and the bar keeps showing the error after the counts until you dismiss it with its **×** or
start the next send.

#### When no session is running

If no session is running in the entry, Send review starts one there with the project's default AI
CLI (Settings), waits until it is ready for input, and types the message as its first input. A
session that ended earlier is never resumed for this: the new session starts fresh. The message
then reads "Started a session and sent a review of *n* comments", and the new session appears in
the sidebar like any other.

If the session cannot start — the default AI CLI is not installed or not available where the entry
runs, its process fails to start, or it is not ready for input within a minute — the error reads
"Couldn't send the review: *why*", nothing is typed, and every comment stays a draft. A session
that was started but not ready in time stays in the sidebar; you can use it, or close it, and send
again.

#### Sending from two windows

While a send of an entry is under way, Send review reads **Sending…** and is unavailable for that
entry in every window, including while a new session is starting for it. Each comment is delivered
once: a second send started meanwhile is refused, and the comments of the first are not sent twice.

### Sending a pull request's comments

While a pull request's diff is shown, **Send review (n)** sends that pull request's draft comments
only — *n* of them. The drafts of other pull requests and of the working tree stay drafts, and the
working-tree diff's **Send review** sends the working-tree drafts only. It is unavailable while the
pull request is not in the list.

The message goes to the session of the worktree, and a session is started when none runs, exactly
as in [Sending a review](#sending-a-review); the comments become **sent** once it is delivered, and
stay drafts, with an error, when it is not. The agent's reports show on the comments as for the
working tree's.

The message names the pull request (number, title, head branch and head commit) and says it is merged or closed
when it is. It asks the agent to make each change or explain why not, to commit the changes and to
push them to the pull request's head branch. It asks for nothing to be posted: **nothing is posted
to GitHub** by Micold or by the message, and the agent's push is what updates the pull request.

When the worktree does not have the pull request's head branch checked out, the message is still
delivered, with a paragraph telling the agent to switch to that branch before changing anything.
Other paragraphs appear only when they apply: when the pull request's head has moved since the
comments were written, one names the commits they were written against and the head now (each
comment then also says which commit its code is from, if they differ); and when a commit the
comments were written against is not in the worktree, one says so.

### When the pull request moves

A pull request's branch can gain commits (the agent's push, a colleague's) or be force-pushed while
your comments are drafts. When the pull request list refreshes and shows a new head for the
pull request you are looking at, its diff is read again. When it differs from the diff on screen it
is held, as for the working tree (see *Keeping up with changes*): the header says how many files
changed since you looked and their rows carry a dot. Selecting a dotted file shows its latest patch;
pressing the notice shows the new diff, and the file you had selected stays selected. Comments you
write before that record the commit of the diff on screen.

Each comment keeps the commit it was written against. It follows its code: where the quoted lines
are still in the new diff the comment stays or moves with them, and where they are gone the comment
is marked **outdated** and stays at its last place in the file. The same mark shows when the file is
no longer part of the pull request. Because a pull request's diff shows only the changed lines and
a few around them, quoted code that is no longer among those lines reads as outdated too, even when
the code itself is unchanged.

An outdated comment is still sent. The message names the commit the comments were written against
and the head now, and each comment carries its quoted code and line range, so the agent can find the
code. A merged or closed pull request keeps its comments visible and editable, and **Send review**
stays available; the message says the pull request is merged or closed.

### The review message

The message asks the session to address the comments. This is one for a worktree with three
drafts:

````text
Please work through these review comments on the changes in this worktree. Each block below is one comment: its reference, the file (relative to the worktree root), the lines it is about, the code on those lines when the comment was written, and the comment itself.

For each comment, either make the change it asks for, or explain why you are not making it. Then report each outcome by the comment's reference with the report_review_comment tool of the micold tool server: outcome "applied" when you made the change (a reply is optional), or "answered" with your explanation as the reply. If you cannot use that tool, answer each comment by its reference in your reply instead.

### Comment C-00000002: src/a.rs, line 3 (removed, base version)
```
use std::io;
```
Keep this import.

### Comment C-00000001: src/a.rs, lines 12-14 (current)
```
fn a() {
    b.clone()
}
```
Why clone here?

### Comment C-00000003: src/b.rs, line 7 (current)
Note: the file has changed since this comment was written; the code is quoted as written.
```
let x = 1;
```
Name this.
````

A pull request's message has the same blocks. Its first paragraph reads "Please work through these
review comments on pull request #12 "Add retry" (head branch feat/retry, head commit 3f2a9c1…). Each block below is one
comment: …" (", which is merged" or ", which is closed" follows the branch for those states, and the
title is cut to one line of at most 200 characters), and its second reads "For each comment, either
make the change it asks for, or explain why you are not making it. Commit the changes you make and
push them to the branch feat/retry. Then report each outcome …". The notes of [Sending a pull
request's comments](#sending-a-pull-requests-comments) follow, each its own paragraph, then the
blocks.

From the top:

- **Preamble.** Two paragraphs, each on one line. The first says where the comments are and what a
  block holds; for the Default entry it says "the project root" in place of this worktree and the
  worktree root. The second asks the session to make each change or explain why not, and to report
  each outcome by the comment's reference with the `report_review_comment` tool; if it cannot use
  that tool, it answers each comment by its reference in its reply.
- **Reference.** `C-` and 8 lowercase hex digits, such as `C-00000001`. It is unique in the entry
  and does not change.
- **Path.** Relative to the entry's root, with `/` separators.
- **Lines.** "line *n*" for one line, "lines *a*-*b*" for a range, as the comment is placed now.
- **Side.** "(current)" for added and unchanged lines, "(removed, base version)" for removed lines.
- **Outdated note.** An outdated comment has a line saying the file has changed since the comment
  was written and the code is quoted as written.
- **Fence.** The quoted code sits in a fence one backtick longer than the longest run of backticks
  in it, and at least three.
- **Elision.** A quote of more than 50 lines shows its first line, "... *k* lines not shown ...",
  and its last line. The heading keeps the full range.
- **Comment text.** As you wrote it.
- **Order.** By path, then start line, then current before removed, then creation time.

### Statuses and replies

Each comment has one of four statuses, shown on its card:

| Status | Meaning |
|---|---|
| **Draft** | Written, not sent. Only a draft can be edited or deleted |
| **Sent** | Typed into the session in a review, no answer yet |
| **Applied** | The session made the change |
| **Answered** | The session did not make the change; its reply says why |

The session reports Applied or Answered by the comment's reference with the
`report_review_comment` tool ([Reporting on review comments](agent-tools.md#reporting-on-review-comments)).
Its reply is drawn under the comment's text. A card shows at most the reply's first 4 lines, as
drawn at the card's width (a long line that wraps counts as the lines it takes); a longer reply
ends there with **…** and offers **Show all**. A later report replaces the status and the reply. Statuses and replies reach
every window open on the project within two seconds and are kept with the comments across
restarts.

A session started without the app's tool server cannot report: its comments stay Sent, and the
review message asks it to answer each comment by its reference in its reply instead.

### Marking a comment by hand

A Sent comment the session never reports on stays Sent. Its card offers two actions:

- **Mark applied** records the comment as Applied at once.
- **Answer…** opens a note under the card. Write why the comment was not applied, or what you did
  instead, and **Save**: the comment becomes Answered with your note as its reply. **Save** stays
  off while the note is blank, and a note is at most 20,000 characters.

A comment you mark is shown like one the session reported. If the session reports on it later, its
report replaces your mark, as a later report replaces an earlier one. A Draft, Applied or Answered
comment has neither action. Cancel closes the note without marking anything.

### Reading a whole reply

A card shows at most the first 4 lines of a reply, then **Show all** under it; a reply that fits
in 4 lines has no **Show all**. A reply longer than 4,000 characters also offers it, whatever the
card shows of its start. Press **Show all** to read the whole reply in a dialog, scrolling if it is
long; **Close** (or Esc) returns to the diff. The dialog closes by itself when its comment is cleared or discarded.

### The comments drawer

**Comments** on the bar opens the comments drawer: a panel over the right side of the diff, from
under the view's header down to the bar, 360 pixels wide (at most half the diff's area in a narrow
window). It covers the diff rather than squeezing it: the diff keeps its width and its lines do not
move, and while the drawer is closed the diff takes the whole width beside the file list. The
drawer is closed when you open the review. **Comments** again, the drawer's **×**, or **Esc** closes
it; **Esc** closes a menu or dialog open above it first. Whether it is open, its tab and where each
tab was scrolled are kept with the entry's Review tab, so they are as you left them when you come
back to it.

The drawer opens by itself, on **Replies**, when the agent first reports on a review you sent from
this window: once per send, only while that entry's Review tab is shown, and not if you closed the
drawer since sending. An open drawer stays on its tab. A report on a review sent from another window
does not open it.

- **Tabs.** **Drafts** lists the comments not sent yet; **Replies** lists sent, applied and
  answered comments. Each lists every comment of the entry, the working tree's first, then each pull
  request's by number. Each tab's label carries its count (*Drafts 3*, *Replies 4*); a tab with no
  rows says "No drafts" or "No replies yet", and with no comment at all, how to start one.
- **What a row shows.** The file and the lines (*Line 3*, *Lines 3–5*, or *Old lines 3–5* for the
  base version), its status and the start of the comment. Rows are ordered by file, then by line. A
  row whose file is no longer among the changes is marked *Not in the current diff* and keeps its
  status.
- **Drafts.** A draft row has **Edit** and **Delete**. **Edit** jumps to the comment, as
  pressing the row does, and opens its text in the row.
  Closing the drawer (or switching its tab) keeps what you wrote: meanwhile it is shown under the
  comment's card in the diff, and it is back in the row when you reopen the drawer. No row offers
  them while a send is under way.
- **Replies.** A row shows the whole reply, wrapped; a reply longer than 4,000 characters shows its
  start and **Show all**. A Sent row offers **Mark applied** and **Answer…**, as its card does
  ([Marking a comment by hand](#marking-a-comment-by-hand)); **Answer…** also jumps to the comment and opens the note in the row.
- **Jumping.** Press a row to select its file and scroll the diff to the comment's first line, left
  of the drawer; the comment's card is outlined. A jump can be refused: when the file is hidden
  because the **Committed** or **Uncommitted** button is off, the drawer says to turn it back on;
  when the file is not among the changes at all, it says so. The diff stays where it was.

### Clearing and discarding

The bar's **⋮** opens a menu that tidies the comments of what is shown (the working tree, or the
shown pull request) once a round of feedback is done:

- **Clear sent** removes every sent, applied and answered comment of what is shown at once and
  keeps the drafts. It is unavailable while there is none.
- **Discard drafts…** asks "Discard *n* draft comments? This cannot be undone.", *n* counting the
  drafts of what is shown. **Discard** removes them; **Cancel** keeps them. It is unavailable while
  there are no drafts, and while a send of the entry is under way.

**Esc** or a click outside closes the menu without changing anything. Both actions change the
comments in every window open on the project.

### When a worktree is removed

Removing a worktree removes its comments with it: deleting it from the sidebar closes its Changes
view and deletes its comments, and a worktree removed outside the app (for example with
`git worktree remove`) loses its comments the next time the project's worktree list is refreshed.
Its comments do not reappear in any other entry. Removing the project from the app deletes the
comments of all its entries.

## The base line

Under the header, **Compared with** `<branch>` **at** `<short commit>` names what the worktree is
compared against: the point where its branch left the project's default branch (`origin/HEAD`,
otherwise `main`, otherwise `master`). Nothing is stored per worktree; the base is worked out each
time the view opens.

When there is no such point, the line says why instead, and only uncommitted changes are listed:

- no default branch was found (no `origin/HEAD`, `main` or `master`);
- the worktree's branch has no history in common with the default branch.

## Committed and uncommitted

Two chips inside the source control's **Working tree** option choose what is listed, and both are
on when the view opens:

- **Committed** — the commits on the worktree's branch since the base;
- **Uncommitted** — what is in the worktree's files but not committed yet, including files git
  does not track.

Turn one off to see only the other kind of change: the list stays open, so you can turn the other
back on, and the closed control reads **Working tree · uncommitted** (or **· committed**, or **·
nothing shown** with both off). Pressing a chip while a pull request is shown returns to the working
tree with the new kinds. They reset each time the view opens.

## The Default entry

The project root has no branch of its own to compare with the default branch, so for the
**Default** entry the **Committed** chip in the source control's **Working tree** option is
unavailable, and a note under the chips says so: only the uncommitted changes in the project root are
listed. The control reads **Working tree**, and its list has no pull requests.
