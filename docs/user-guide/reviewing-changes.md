# Reviewing a worktree's changes

The **Changes view** lists every file a worktree has changed, so you can see what a session did
before you merge it or tell the session what to change next.

## Opening the Changes view

Right-click a worktree in the sidebar and choose **Review changes**. The **Default** entry (the
project root, see [Worktrees & Sessions](worktrees-and-sessions.md#the-default-entry-sessions-without-a-worktree))
has a right-click menu too, with **Review changes** as its one item.

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

## Pull requests

With **Show pull request status on worktrees** checked in [Settings →
GitHub](settings.md#show-pull-request-status-on-worktrees), the list is preceded by a **Pull
requests** section: every pull request this worktree produced, open, draft, merged and closed,
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

The section is the same for a worktree with no session as for one with sessions, and it shows
only pull requests of this project's own repository. A pull request opened from a fork is not
listed.

The working-tree changes below it never wait for the section: they appear first, and the section
joins when GitHub has answered. Until the first read finishes the section says **Reading pull
requests…**. A worktree none of whose branches has a pull request says **No pull requests from this
worktree yet**. When GitHub cannot be reached and no list has been read (no `gh`, not signed in, no
GitHub remote, offline), it shows one muted line, **Pull requests can't be shown for this
worktree**, and raises no error. The section is absent, with no line at all, when the switch is off
and when the entry is the project root. While GitHub cannot be reached after a list was read, the
last list and your selection stay. A long list scrolls inside the section.

### Reading a pull request's diff

Click a pull request row to read its diff. The file list, the diff and the base line then show
the pull request: **Pull request #N: base ... head**, as GitHub computes it, where *head* is that
pull request's own branch. Select a file as
usual; comments are kept per file and line as for any other diff. While the pull request is
chosen, the working-tree changes are not read, and nothing reads them behind your back.

Above the diff, a header names the pull request you are reading: its state and checks, its
number and title, and the base line. The **previous** and **next** buttons beside it step through
the list in the order it is shown; each is greyed out at its end of the list. Pressing a row or
a button while a diff is still loading shows the last one you chose, never an earlier one's diff.

Click the **Working tree** row to go back. Each entry remembers its own choice while the window is
open, so switching to another worktree and back returns to the pull request you were reading.

A pull request whose branch was merged and deleted still shows its diff: GitHub keeps it. If the
pull request is no longer listed after a refresh, the view returns to the working tree.

GitHub will not diff a pull request above 300 files or 20 000 changed lines. The pane then says
the pull request is too large to show here and offers **Open on GitHub**. A read that fails for
another reason says so in the pane, with no notice; click the row again to retry.

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

### Keeping up with changes

The view keeps itself current while it is open. When a file in the entry is created, edited,
deleted or renamed, or a change is staged or committed — by you in a terminal or editor, or by
the agent — the list and the diff on screen are read again within about two seconds, with nothing
to press. A burst of edits (a build, an agent rewriting several files) is read once when it
settles, not once per file. Files git ignores do not trigger a refresh, and the Default entry does
not react to changes inside its worktrees.

The **Pull requests** list follows the same pull request status refresh as the sidebar: a pull
request opened, merged or closed while the view is open shows its new state within that refresh
interval (5 minutes), and a list refresh in the sidebar updates it too. If a refresh cannot reach
GitHub, the last list stays on screen with no error, and once it is older than ten minutes it is
captioned **Possibly stale** until a refresh succeeds.

A pull request opened from a new branch of the worktree appears at the next refresh, without
reopening the view. The pull request you are reading stays shown and keeps its place in the list
while the list changes; its header follows its state and checks. It is replaced by the working
tree only when it leaves the list.

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
- The review panel lists them under the pull request's number and title, after the working-tree
  comments, ordered by pull request number.
- Comments are kept per pull request on your computer, survive a restart, and go away when the
  worktree is removed.
- If a pull request leaves the list, its comments are kept and shown flagged "not in the list".
- **Clear sent** and **Discard drafts** act only on the comments of what is shown: the pull
  request, or the working tree.
- Nothing is posted to GitHub. To have the agent act on a pull request's comments, see [Sending a
  pull request's comments](#sending-a-pull-requests-comments).

### Sending a review

**Send review (n)** in the view's header sends the entry's draft comments — *n* of them — to the
session working in that worktree (or in the project root, for the Default entry) as one message. It
is unavailable while there are no drafts, and reads **Sending…** while a send of this entry is
under way in any window. Only draft comments are sent: a comment that was already sent is not sent
again, so a later send carries only the comments added since.

The message goes to the session running in the same entry; with several running there, to the one
you used most recently (typed into, or whose activity last changed). It never goes to a session of
another worktree. It is typed as one pasted submission, exactly as if you had pasted it and pressed
Enter. What it contains is described under [The review message](#the-review-message).

When it is delivered, a message reads "Sent a review of *n* comments to *session*", and the
comments become **sent**: they stay on their lines, but can no longer be edited or deleted. When it
is not delivered — for example the session's terminal cannot take a pasted text — an error reads
"Couldn't send the review: *why*" and the comments stay drafts.

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
pull request you are looking at, its diff is read again; the file you had selected stays selected.

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
Its reply is drawn under the comment's text; a reply longer than 4,000 characters shows its start
and **Show all**. A later report replaces the status and the reply. Statuses and replies reach
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

A reply longer than 4,000 characters shows its start with **Show all** under it. Press **Show all**
to read the whole reply in a dialog, scrolling if it is long; **Close** (or Esc) returns to the
diff. The dialog closes by itself when its comment is cleared or discarded.

### The review panel

The panel on the right of the diff lists every comment of the entry, across all its files. It is
open by default; the **Review (n)** button in the header, *n* being the number of comments, closes
and reopens it.

- **What a row shows.** The file and the lines (*Lines 3–5*, or *Old lines 3–5* for the base
  version), the start of the comment, its status (Draft, Sent, Applied or Answered) and, once the
  agent has reported, the start of its reply. Rows are ordered by file, then by line. A row whose
  file is no longer among the changes is marked *Not in the current diff* and keeps its status.
- **Counts.** Under the panel's title, how many comments are in each status.
- **Jumping.** Press a row to select its file and scroll the diff to the comment's first line; the
  comment's card is outlined. A jump can be refused: when the file is hidden because the
  **Committed** or **Uncommitted** button is off, the panel says to turn it back on; when the file
  is not among the changes at all, it says so. The diff stays where it was.
- **Editing and deleting.** A draft row has **Edit** and **Delete**, the same as its card. Sent,
  applied and answered rows have neither, and no row offers them while a send is under way.
- **Buttons.** **Send review (n)**, **Clear sent** and **Discard drafts…** are in the panel's header
  and are available under the same conditions as described here.
- **No comments.** The panel says "No review comments yet" and how to start one.

### Clearing and discarding

Two buttons in the review panel's header tidy the list once a round of feedback is done:

- **Clear sent** removes every sent comment of the entry at once and keeps the drafts. It is
  unavailable while the entry has no sent comments.
- **Discard drafts…** asks "Discard *n* draft comments? This cannot be undone." **Discard**
  removes every draft comment of the entry; **Cancel** keeps them. It is unavailable while the
  entry has no draft comments, and while a send of the entry is under way.

Both change the entry's comments in every window open on the project.

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

Two toggles choose what is listed, and both are on when the view opens:

- **Committed** — the commits on the worktree's branch since the base;
- **Uncommitted** — what is in the worktree's files but not committed yet, including files git
  does not track.

Turn one off to see only the other kind of change. They reset each time the view opens.

## The Default entry

The project root has no branch of its own to compare with the default branch, so for the
**Default** entry the **Committed** toggle is unavailable and a note beside it says so: only the
uncommitted changes in the project root are listed.
