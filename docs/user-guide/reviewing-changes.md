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
- `+a −r`: how many lines were added and removed.

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
line removed and added again; line endings themselves are never drawn. Long diffs scroll smoothly:
only the rows on screen are drawn. While a file's diff is read, the diff area says
**Loading diff…**; the window keeps responding meanwhile.

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
