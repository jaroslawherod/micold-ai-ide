# Reviewing a worktree's changes

The **Changes view** lists every file a worktree has changed, so you can see what a session did
before you merge it or tell the session what to change next.

## Opening the Changes view

Right-click a worktree in the sidebar and choose **Review changes**. The **Default** entry (the
project root, see [Worktrees & Sessions](worktrees-and-sessions.md#the-default-entry-sessions-without-a-worktree))
has the same right-click menu, with **Review changes** as its one item.

The view takes the place of the terminal. Its header names the entry; **Close** returns to the
terminal. Selecting a session in the sidebar also closes it, and so does the worktree disappearing
from the sidebar (for example, deleted from another window).

## What the list shows

One row per changed file:

- the file's path, relative to the worktree;
- what happened to it: **Added**, **Modified**, **Deleted**, **Renamed from** `<old path>` or
  **Mode** (only its permissions changed). A file git does not track yet counts as **Added**;
- **Binary** or **Not text** when the file's contents are not readable text;
- `+a −r`: how many lines were added and removed;
- how many comments you have written on the file and not sent yet (see
  [Commenting on lines](#commenting-on-lines)), when there are any.

A file changed both in a commit and again since is listed once, with the combined change. Long
lists scroll smoothly: only the rows on screen are drawn.

While the list is read, the view says **Reading changes…**. With nothing to list it says why:
**No changes against** `<branch>`, **No uncommitted changes**, or that both kinds are hidden.

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
as plain text. Only the first 2,000 characters of a very long line are coloured; the rest of it is
shown in the plain text colour.

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

### Keeping up with changes

The view keeps itself current while it is open. When a file in the entry is created, edited,
deleted or renamed, or a change is staged or committed — by you in a terminal or editor, or by
the agent — the list and the diff on screen are read again within about two seconds, with nothing
to press. A burst of edits (a build, an agent rewriting several files) is read once when it
settles, not once per file. Files git ignores do not trigger a refresh, and the Default entry does
not react to changes inside its worktrees.

The text you are writing in a comment box is kept across a refresh. Picked lines are kept while
they are still in the diff; when they are gone, the pick is dropped.

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

### Writing a comment

With lines picked, press **Add comment** under them. A text box opens under the last picked line;
write the comment and press **Save** (or **Ctrl+Enter**, **Cmd+Enter** on macOS). **Cancel** closes
it without saving. **Save** stays unavailable while the box is empty.

The comment then shows as a card under its last line, marked **Pending** until it is sent to a
session. The text you are writing is kept while the list or the diff is read again.
If a change removes the lines you picked, the box moves above the diff with the note "The lines
this comment was on are gone. Pick lines to place it." and keeps your text; pick lines again and
**Save** adds the comment there. If the file itself leaves the list, the unsaved comment is closed.

### Editing and deleting

A pending comment's card has **Edit** and **Delete**. **Edit** opens the text box in its place with
the comment's text; **Save** keeps the new text. **Delete** removes the comment straight away.

A comment whose lines the diff on screen does not show — for example because you turned off the
kind of change it is on — is listed under **Not in the current diff** above that file's diff, with
the lines it is on.

### Outdated comments

When the lines a comment points at no longer hold the code it quoted — the agent or you changed
them, or the file is gone — its card is marked **Outdated**. The comment stays where it is and
keeps the code it quoted when it was written. It is still sent with that quoted code and its line
range, so the session sees what you were looking at.

### Where comments are kept

Comments belong to the worktree (or the Default entry) they were written in, and the session
service keeps them: they are still there after you restart the app, and every window open on the
project shows the same comments as they are added, edited and deleted.

### Sending comments to the session

**Send to session (n)** in the view's header sends the entry's pending comments — *n* of them — to
the session working in that worktree (or in the project root, for the Default entry) as one
prompt. It is unavailable while there are no pending comments, and reads **Sending…** while a send
of this entry is under way in any window.

The prompt asks the session to address the comments. For each file, in path order, it lists every
comment with the lines it is about — marked as current lines, or as removed lines with their
numbers in the base version — the code on those lines as it was when the comment was written, and
the comment's text. A long range of lines is shortened to its first and last lines.

The prompt goes to the session running in the same entry; with several running there, to the one
you used most recently (typed into, or whose activity last changed). It never goes to a session of
another worktree. It is typed as one pasted submission, exactly as if you had pasted it and pressed
Enter.

When it is delivered, a message says how many comments went to which session, and the comments
become **sent**: they stay on their lines, but can no longer be edited or deleted, and a later
send carries only the comments added since. When it is not delivered — for example the session's
terminal cannot take a pasted text — an error says why and the comments stay pending.

#### When no session is running

If no session is running in the entry, Send to session starts one there with the project's
default AI CLI (Settings), waits until it is ready for input, and types the prompt as its first
input. A session that ended earlier is never resumed for this: the new session starts fresh. The
message then reads "Started a session and sent *n* comments", and the new session appears in the
sidebar like any other.

If the session cannot start — the default AI CLI is not installed or not available where the entry
runs, its process fails to start, or it is not ready for input within a minute — an error says
why, nothing is typed, and every comment stays pending. A session that was started but not ready
in time stays in the sidebar; you can use it, or close it, and send again.

#### Sending from two windows

While a send of an entry is under way, Send to session reads **Sending…** and is unavailable for
that entry in every window, including while a new session is starting for it. Each comment is
delivered once: a second send started meanwhile is refused, and the comments of the first are
not sent twice.

## The base line

Under the header, **Compared with** `<branch>` **at** `<short commit>` names what the worktree is
compared against: the point where its branch left the project's default branch (`origin/HEAD`,
otherwise `main`, otherwise `master`). Nothing is stored per worktree; the base is worked out each
time the view opens.

When there is no such point, the line says why instead, and only uncommitted changes are listed:

- no default branch was found (no `origin/HEAD`, `main` or `master`);
- the worktree's branch has no history in common with the default branch.

## Committed and uncommitted

Two toggles choose what is listed, and both are on when the view opens:

- **Committed** — the commits on the worktree's branch since the base;
- **Uncommitted** — what is in the worktree's files but not committed yet, including files git
  does not track.

Turn one off to see only the other kind of change. They reset each time the view opens.

## The Default entry

The project root has no branch of its own to compare with the default branch, so for the
**Default** entry the **Committed** toggle is unavailable and a note beside it says so: only the
uncommitted changes in the project root are listed.
