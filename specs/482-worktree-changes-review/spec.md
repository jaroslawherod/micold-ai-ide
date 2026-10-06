# Feature Specification: Review a Worktree's Changes and Send Comments to Its Session

**Feature Branch**: `claude/project-thread-v1va8z`

**Created**: 2026-10-06

**Status**: Draft

**Input**: User description: "Implement GitHub issue #482: Review a worktree's changes with inline
comments and send them to the session. Problem: reviewing what an agent changed in a worktree means
leaving the app for a terminal or an editor, reading `git diff`, and then typing feedback back into
the session by hand. Proposal: add a Changes view per worktree. List the changed files against the
worktree's base branch (committed and uncommitted, with a toggle for each), and show a unified or
side-by-side diff for the selected file with syntax colouring. Let the user add comments to single
lines or line ranges. A Send to session action collects all pending comments into one prompt (file,
line range, quoted code, comment text) and sends it to the worktree's AI session, starting one if
none is running. Comments that were sent are marked as sent; the list can be cleared. Acceptance
criteria: the view refreshes when files in the worktree change; large diffs and binary files do not
freeze the UI (binary files are listed but not rendered); a sent prompt contains every pending
comment with enough context for the agent to locate it; diff rendering and comment-to-prompt
formatting live in the render-free core and are unit-tested."

## Terms

- **Changes view**: the per-worktree view this feature adds, listing changed files and showing the
  diff of the selected one.
- **Base branch**: the branch the worktree's changes are compared against. Committed changes are
  the commits on the worktree's branch since it diverged from the base branch.
- **Committed changes**: differences between the base branch (at the point the worktree's branch
  diverged from it) and the worktree's last commit.
- **Uncommitted changes**: differences between the worktree's last commit and the files on disk,
  staged or not, including new files git does not yet track (ignored files excluded).
- **Review comment**: a note the user attaches to one line or a contiguous range of lines of one
  file's diff. It is **pending** until sent, then **sent**.
- **Review prompt**: the single message built from all pending review comments and delivered to an
  AI session as user input.
- **Entry**: a worktree or the project's "Default" entry (the project root), as listed in the
  sidebar.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See what changed in a worktree (Priority: P1)

A user whose agent has been working in a worktree opens that worktree's Changes view from inside
the app. They see every file the worktree changed against its base branch, each with its kind of
change (added, modified, deleted, renamed, binary), and the diff of the file they select, with
syntax colouring. They switch between a unified and a side-by-side layout, and they choose whether
committed changes, uncommitted changes, or both are listed.

**Why this priority**: Reading the changes is the precondition for every comment; on its own it
already removes the trip to a terminal or editor.

**Independent Test**: In a worktree with one committed edit, one uncommitted edit, one new
untracked file and one binary file, open the Changes view and check the file list, each toggle,
both layouts, and that the binary file is listed with no diff body.

**Acceptance Scenarios**:

1. **Given** a worktree whose branch has one commit changing `a.rs` and an uncommitted edit to
   `b.rs`, **When** the user opens its Changes view with both toggles on, **Then** both files are
   listed with their change kind and line counts added and removed.
2. **Given** the same worktree, **When** the user turns the uncommitted toggle off, **Then** only
   `a.rs` is listed; **When** they turn it back on and the committed toggle off, **Then** only
   `b.rs` is listed.
3. **Given** a file that is changed both in a commit and on disk, **When** both toggles are on,
   **Then** it is listed once and its diff shows the combined change from the base to the file on
   disk.
4. **Given** a selected text file, **When** the user switches between unified and side-by-side,
   **Then** the same changed lines are shown in the chosen layout with line numbers for both sides
   and syntax colouring for a recognised language, and the choice is kept for the next file and
   after the app restarts.
5. **Given** a changed binary file, **When** the user selects it, **Then** it is listed and the
   diff area says it is binary and shows no content.
6. **Given** a worktree with no changes against its base under the current toggles, **When** the
   user opens the Changes view, **Then** it says there are no changes and shows no diff.
7. **Given** a changed file whose diff exceeds the size limits in Edge Cases, **When** the user
   selects it, **Then** the diff area shows its line counts and a show-diff action instead of the
   diff, and the window keeps responding to input; **When** the user takes that action, **Then**
   the diff is shown.
8. **Given** the app in a light or a dark theme, **When** a diff is shown, **Then** added, removed
   and context lines and the syntax colours are distinguishable in that theme.
9. **Given** the "Default" entry (the project root), **When** the user opens its Changes view,
   **Then** it lists the root's uncommitted changes only, and the committed toggle is unavailable
   with a note saying why.

---

### User Story 2 - Comment on lines and send the comments to the session (Priority: P1)

While reading a diff, the user adds comments to a single line or a range of lines, across one or
several files. When done, they choose **Send to session**. Every pending comment is sent to the
worktree's AI session as one prompt naming the file, the line range, the quoted code and the
comment text, and each comment is then marked as sent.

**Why this priority**: This is the feedback loop the issue asks for; without it the view is only
a diff viewer.

**Independent Test**: With a running session in the worktree, add a single-line comment in one
file and a three-line range comment in another, send, and check the session received one prompt
holding both comments with their file, lines, quoted code and text, and that both comments now
show as sent.

**Acceptance Scenarios**:

1. **Given** a diff on screen, **When** the user picks one line and writes a comment, **Then** a
   pending comment appears anchored to that line.
2. **Given** a diff on screen, **When** the user picks a contiguous range of lines in one file and
   writes a comment, **Then** a pending comment appears anchored to that range.
3. **Given** pending comments in two files and a running AI session in the worktree, **When** the
   user chooses Send to session, **Then** the session receives exactly one prompt that holds every
   pending comment, each with its file path relative to the worktree, its line range, the quoted
   code of those lines and its text, and every one of those comments becomes sent.
4. **Given** a comment on a removed line (a line that exists only in the base version), **When** it
   is sent, **Then** the prompt says the line was removed and quotes the removed code, with its
   line number in the base version.
5. **Given** no pending comments, **When** the user looks at Send to session, **Then** the action
   is unavailable.
6. **Given** some sent and some pending comments, **When** the user sends again, **Then** only
   the pending ones are in the prompt.
7. **Given** a pending comment, **When** the user edits or deletes it before sending, **Then** the
   prompt carries the edited text, or no trace of the deleted comment.

---

### User Story 3 - Send when no session is running (Priority: P2)

The user reviews a worktree whose AI session is not running and chooses Send to session. The app
starts a session for the worktree and delivers the prompt to it as its first input.

**Why this priority**: Reviews often happen after the agent has stopped; requiring a manual start
first is friction, but the core loop (Stories 1 and 2) works without it.

**Independent Test**: In a worktree with no running session, add a comment, send, and check that
a session is now running in that worktree and that its first input is the review prompt.

**Acceptance Scenarios**:

1. **Given** a worktree with no running session and pending comments, **When** the user chooses
   Send to session, **Then** a session runs in that worktree and receives the review prompt as its
   first input, and the comments become sent.
2. **Given** the session fails to start, **When** the user chooses Send to session, **Then** an
   error says so and every comment stays pending.
3. **Given** a send in progress for a worktree, **When** the user chooses Send to session again in
   the same or a second window, **Then** the action is unavailable and each comment reaches the
   session exactly once.
4. **Given** a worktree with more than one running session, **When** the user chooses Send to
   session, **Then** [NEEDS CLARIFICATION: which session receives the prompt when several run, and
   whether a stopped session is resumed or a new one started when none runs: ask each time, the
   most recently active one, or always a new session?].

---

### User Story 4 - Keep the view current and the comment list tidy (Priority: P3)

While the agent keeps editing files, the Changes view keeps up without the user refreshing it.
When a round of feedback is done, the user clears the sent comments.

**Why this priority**: A stale view misleads, and a long list of old comments hides the new ones;
both matter once the loop runs more than once.

**Independent Test**: With the Changes view open, change a file on disk from outside the app and
check the list and the diff update; then clear the sent comments and check only pending ones stay.

**Acceptance Scenarios**:

1. **Given** the Changes view is open, **When** a file in the worktree is created, edited, deleted
   or committed, **Then** the file list and the open diff reflect it within 2 seconds, without any
   user action.
2. **Given** a comment anchored to lines whose content has since changed, **When** the view
   refreshes, **Then** the comment stays, keeps the code it quoted when it was written, and is
   marked outdated; it is still sent with that quoted code.
3. **Given** sent and pending comments, **When** the user clears the list, **Then** the sent
   comments are removed and the pending ones stay. A separate action discards pending comments
   after the user confirms.
4. **Given** pending and sent comments in a worktree, **When** the app is quit and started again,
   **Then** the worktree's Changes view shows the same comments with the same states.
5. **Given** a worktree with comments, **When** the worktree is removed, **Then** its comments are
   gone and appear nowhere else.

---

### Edge Cases

- **No base found**: the base branch no longer exists or the worktree's branch shares no history
  with it. The view lists uncommitted changes only and says why committed changes are missing.
- **Both toggles off**: the list is empty and says that both kinds are hidden.
- **Large diff**: a file whose diff exceeds 5,000 changed lines, or whose either version exceeds
  2 MB, is listed with its counts but its diff is shown only after the user asks for it; the app
  stays responsive while it loads. A worktree with thousands of changed files lists them without
  blocking input.
- **Binary and undisplayable files**: binary files, and text that is not valid UTF-8, are listed
  and marked; no content is rendered and no line comment can be added to them.
- **Renames, deletions, new files, mode changes**: a rename shows old and new paths; a deleted
  file shows all lines removed; a new or untracked file shows all lines added; a mode-only change
  is listed with no line diff.
- **Very long comment ranges**: a comment on more than 50 lines quotes the first and last lines
  with an elision marker in the prompt, keeping the full line range, so one comment cannot flood
  the prompt.
- **File changes while composing**: the diff refreshes under an open comment editor; the text
  being typed is kept.
- **Session ends during sending**: if delivery fails, an error is shown and every comment stays
  pending; no comment is marked sent unless the prompt was delivered.
- **Isolation (Principle II)**: comments belong to one entry. A prompt goes only to a session of
  the entry it was written in, never to another worktree's session. Two windows showing the same
  entry show the same comments and states.
- **Default entry**: the project root has a Changes view of its uncommitted changes only, because
  the root usually has the base branch checked out and has no branch of its own to compare; it
  never creates or changes a worktree.
- **Repeated or concurrent send**: while a send is in progress the action is unavailable, and when
  two windows send the same entry's comments at once each pending comment is delivered at most
  once.
- **Cross-platform (Principle VI)**: paths in the list and in the prompt use `/` on every platform;
  files with CRLF line endings show their line content without the line-ending characters, and a
  change that only switches line endings is shown as a change; refresh on change works on Linux,
  macOS and Windows.
- **Removed worktree**: if the worktree is removed while its Changes view is open, the view closes
  and its comments are discarded.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Each entry MUST offer a Changes view that the user can open from the app.
- **FR-002**: The Changes view MUST list every file that differs between the entry's base and its
  current state, with the change kind (added, modified, deleted, renamed, mode change, binary) and
  the number of lines added and removed.
- **FR-003**: For a worktree, the base MUST be the point where the entry's branch diverged from its base branch.
  The base branch is the branch the worktree was created from when the app knows it, otherwise
  [NEEDS CLARIFICATION: fallback base branch when the app does not know where the worktree came
  from: the repository's default branch (remote HEAD), the project root's checked-out branch, or a
  user choice?]. The "Default" entry MUST list uncommitted changes only.
- **FR-004**: The view MUST have two independent toggles, committed and uncommitted, both on by
  default (the "Default" entry has only uncommitted, per FR-003); the list and diffs MUST show only the kinds switched on, and a file changed in both
  appears once with the combined change.
- **FR-005**: Uncommitted changes MUST include staged changes, unstaged changes and untracked files
  that are not ignored.
- **FR-006**: Selecting a text file MUST show its diff with line numbers for each side, in unified
  or side-by-side layout as the user chooses; the layout choice MUST persist across files and
  restarts.
- **FR-007**: Diffs MUST be syntax-coloured for recognised languages and shown as plain text
  otherwise, in both light and dark themes.
- **FR-008**: Binary files and non-UTF-8 text MUST be listed and marked, and MUST NOT have content
  rendered or accept line comments.
- **FR-009**: Diffs over the size limits in Edge Cases MUST NOT be rendered until the user asks;
  computing any diff MUST NOT block user input or window redraws.
- **FR-010**: The view MUST refresh its list and the open diff within 2 seconds of a file in the
  entry being created, edited, deleted, renamed, staged or committed, with no user action.
- **FR-011**: Users MUST be able to add a comment to one line or to a contiguous range of lines on
  one side of one file's diff, and to edit or delete a pending comment.
- **FR-012**: A comment MUST record the file path, the side (new or removed lines), the line range
  in that side's version, the quoted code of those lines at the time of writing, and its text.
- **FR-013**: When the lines a comment points at no longer hold the quoted code, the comment MUST
  be marked outdated and MUST still be sent with its recorded quote and line range.
- **FR-014**: A Send to session action MUST be available whenever at least one pending comment
  exists, and MUST build one review prompt holding every pending comment, and nothing else from the
  comment list.
- **FR-015**: For each comment the review prompt MUST state the file path relative to the entry
  root with `/` separators, the line range, whether the lines are current or removed, the quoted
  code (elided per Edge Cases when over 50 lines) and the comment text; comments MUST appear
  grouped by file and in line order. The prompt MUST be identical for the same comments on every
  platform.
- **FR-016**: The review prompt MUST be delivered to a session of the same entry as user input;
  when no session of that entry runs, the app MUST start one and deliver the prompt as its first
  input.
- **FR-017**: Comments MUST become sent only once their prompt is delivered; on any failure they
  MUST stay pending and the user MUST see an error.
- **FR-018**: A send in progress MUST make Send to session unavailable for that entry in every
  window, and each pending comment MUST be delivered at most once.
- **FR-019**: Users MUST be able to clear sent comments in one action, and to discard all pending
  comments after a confirmation.
- **FR-020**: Pending and sent comments MUST be kept on the local filesystem per entry and MUST
  survive an app restart; they MUST be discarded when the worktree is removed.
- **FR-021**: Comments and prompts MUST be scoped to their entry; no comment may be shown in, or
  sent to a session of, a different entry.
- **FR-022**: The diff results (file list, hunks, line pairing for both layouts, binary and size
  classification) and the review prompt text MUST be verifiable by automated tests that run with no
  window open and no session running.
- **FR-023**: The user guide MUST describe the Changes view, commenting, sending and clearing.

### Key Entities

- **Changed file**: path (and old path for a rename), change kind, lines added and removed,
  whether it is binary or over the size limit, and whether its change is committed, uncommitted or
  both.
- **File diff**: the hunks of one changed file, each a run of context, added and removed lines
  with their line numbers in the base and current versions.
- **Review comment**: entry, file path, side, line range, quoted code, text, state (pending or
  sent), outdated flag, creation time.
- **Review prompt**: the ordered text built from a set of pending comments, plus the comments it
  covers.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In the Story 2 independent test, every step from opening the Changes view to the
  prompt being delivered is an in-app action, and the user types 0 file paths and 0 line numbers.
- **SC-002**: In a test with 20 comments across 5 files, the delivered prompt holds all 20, each
  with its file, line range, quoted code and text; 0 are missing or duplicated.
- **SC-003**: Opening the Changes view on a worktree with 2,000 changed files, or selecting a file
  with a 50,000-line diff, never leaves the window unresponsive for more than 100 ms at a time.
- **SC-004**: After a file in the worktree changes, the view shows the change within 2 seconds in
  95% of trials.
- **SC-005**: Binary files appear in the list in 100% of cases and their content is never rendered.
- **SC-006**: The same set of comments produces a byte-identical prompt on Linux, macOS and
  Windows.

## Out of Scope

- Editing files, staging, committing, reverting hunks or resolving conflicts from the Changes view.
- Comparing arbitrary refs or commits picked by the user; the view compares only against the base.
- Posting comments to GitHub pull requests or importing review comments from them.
- Threaded discussions or replies from the agent attached to comments.
- Rendering images or other binary previews.
- Word-level (intra-line) highlighting beyond line-level added and removed marking.

## Assumptions

- Delivering the review prompt reuses the same path the app already uses to send user input to a
  session; the session's AI CLI receives it as if the user had typed and submitted it.
- Syntax colouring recognises languages by file extension; an unknown extension falls back to
  plain text.
- File-change detection watches the entry's directory and its git metadata, debounced, and skips
  ignored paths; it runs only while a Changes view of that entry is open.
- The size limits (5,000 changed lines, 2 MB per version, 50 quoted lines) are defaults chosen to
  keep the UI responsive; they are not user settings in this feature.
- Comments are stored with the app's other per-project local state; none of them leaves the device
  except as the prompt the user explicitly sends to a local AI CLI session.
