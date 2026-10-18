# Feature Specification: Review Comments the Agent Answers — Review Panel, Stable Batch, Replies and Re-anchoring

**Feature Branch**: `claude/project-thread-jjniz9`

**Created**: 2026-10-17

**Status**: Closed 2026-10-18 — shipped in PRs #950, #969, #981, #986, #993, #997

**Input**: GitHub issue #946 "Diff view: inline review comments batched and sent to the agent".
"Let the user leave feedback on the diff (the Changes view, and later the structured-session
Edit/Write diffs) and send it to the agent in one batch, like a PR review. Behavior: select a line
or range in a diff and add a comment; comments are drafted locally and listed in a review panel
(edit, delete). A 'Send review' action formats all comments (file, line range, quoted code, comment
text) into one message for the session's agent. The agent is asked to either apply each change or
reply with an explanation of why not; the reply is shown next to the comment (resolved/answered
state). Works in PTY sessions (message typed into the prompt) and structured sessions (stream-json
user message). Acceptance: comments persist across restarts until sent or discarded; batch message
format is documented and stable (one block per comment); per-comment status: draft, sent, applied,
answered; tests for formatting and for anchoring comments when the diff changes after the agent
edits. Related: structured session epic #945 (diff rendering issue)."

## Relation to feature 482

Feature 482 (shipped in PR #624, `specs/482-worktree-changes-review/`) already gives every entry a
Changes view in which the user picks one line or a range, writes a comment, edits or deletes it
while pending, sees it marked outdated when its lines change, and sends all pending comments to the
entry's session as one prompt typed into its terminal. Comments are kept per entry on the local
filesystem and survive restarts. 482 listed "threaded discussions or replies from the agent
attached to comments" as out of scope.

This feature builds on 482 and changes or adds only what the issue asks beyond it:

1. a **review panel** listing every comment of the entry in one place, across files;
2. a **stable, documented batch format** with one self-contained block per comment, each carrying
   a reference the agent can answer;
3. the agent **answering each comment** (applied, or answered with an explanation), the answer
   shown next to the comment, and the per-comment statuses draft, sent, applied, answered;
4. comments **following their code** when the agent's edits move it, instead of only being marked
   outdated.

Everything 482 specifies and this spec does not change stays as 482 specifies it (picking lines,
size limits, binary files, base resolution, which session receives a send, starting a session when
none runs, sending from two windows, clearing, removal with the worktree).

## Terms

- **Review comment** (from 482): a note on one line or a contiguous range of lines of one side of
  one file's diff, with the code it quoted when written.
- **Status**: where a comment is in its round trip. **Draft** (482's *pending*): written, not yet
  sent; editable and deletable. **Sent**: delivered to a session in a batch, no answer yet.
  **Applied**: the agent reported it made the requested change. **Answered**: the agent reported it
  did not make the change and gave an explanation.
- **Agent reply**: the text the agent attaches to a comment when it reports applied or answered.
- **Batch** (482's *review prompt*): the one message a **Send review** builds from all draft
  comments of an entry and delivers to one session.
- **Comment reference**: a short identifier of one comment, printed in its block of the batch, that
  the agent quotes back when it reports on that comment.
- **Review panel**: the list of all of an entry's comments shown beside the Changes view's diff.
- **Anchor**: the place in the current diff where a comment is shown.
- **Entry** (from 482): a worktree, or the project's "Default" entry (the project root).

## Clarifications

### Session 2026-10-17

- Q: Structured (stream-json) sessions do not exist yet (epic #945, #938–#944 open). Is delivering
  the batch to a structured session, and commenting on its Edit/Write diffs, part of this feature?
  → A: No. Ship for terminal (PTY) sessions only and leave both to epic #945; the batch stays
  independent of delivery (FR-026) so #945 can send the same text as a user message. _(orchestrator
  default under bias to action; the user may override)_
- Q: Must an agent report wait for the user's approval, like other agent-tool requests? → A: No;
  only tools that stop or remove something wait for the user, and a report only changes a
  comment's status and reply (FR-003). _(agent-resolved:
  docs/user-guide/agent-tools.md#requests-that-wait-for-you)_
- Q: Can a session in the project folder (the Default entry) report on comments? → A: Yes, on the
  Default entry's own comments only; the tool server knows each session's worktree (`whoami`), and
  a report on another entry's comment is refused (FR-004). _(agent-resolved:
  docs/user-guide/agent-tools.md#what-the-assistant-can-do; specs/482-worktree-changes-review/spec.md)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The agent answers each comment, and the answer shows next to it (Priority: P1)

The user sends a review. The agent works through it and, for each comment, either makes the change
and reports it as applied, or explains why it did not. Each comment's card and its row in the
review panel then show the new status and the agent's reply, so the user sees at a glance which
points were taken up and reads the reasons for the rest.

**Why this priority**: This is what the issue adds over 482: a review that comes back answered,
like a PR review, instead of a one-way prompt the user must reconcile by reading the terminal.

**Independent Test**: With a running session in a worktree, write two comments, send them, then
have the session report the first as applied with a short note and the second as answered with an
explanation; check that each comment shows its status and reply, in the diff and in the panel, and
still does so after the app restarts.

**Acceptance Scenarios**:

1. **Given** two sent comments of an entry, **When** the entry's session reports the first comment
   (by its reference) as applied with the reply "Renamed as asked", **Then** that comment shows the
   status Applied and the reply, in its diff card and its panel row, in every window open on the
   project.
2. **Given** a sent comment, **When** the session reports it as answered with an explanation,
   **Then** the comment shows the status Answered and the explanation.
3. **Given** a sent comment, **When** the session reports it as answered with an empty
   explanation, **Then** the report is refused, the session is told that an explanation is
   required, and the comment stays Sent.
4. **Given** a comment that is a draft, or that belongs to another entry, or a reference that names
   no comment, **When** a session reports on it, **Then** the report is refused with a reason that
   says which of these it was, and no comment changes.
5. **Given** a comment already Applied or Answered, **When** the session reports on it again,
   **Then** the latest report replaces the status and reply, and the comment keeps when it was
   first answered and when it was last answered.
6. **Given** a sent comment the session never reports on, **When** the user looks at it, **Then**
   it stays Sent; the user can mark it Applied or Answered by hand, or leave it.
7. **Given** comments with statuses and replies, **When** the app is quit and started again,
   **Then** every comment shows the same status and reply as before.
8. **Given** the batch, **When** the session reads it, **Then** it says, once, that for each
   comment the agent is to either apply the change or explain why not, and how to report each
   outcome by the comment's reference.

---

### User Story 2 - A stable batch with one block per comment (Priority: P1)

**Send review** delivers all draft comments of the entry as one message. Each comment is one
self-contained block holding its reference, its file, its line range and side, the quoted code and
the comment's text, so the agent can act on and answer each block on its own. The format is written
down in the user guide and does not change between sends or platforms.

**Why this priority**: The agent's answers (Story 1) depend on it finding each comment's reference
and context in a predictable place; the issue makes the documented, stable format an acceptance
criterion.

**Independent Test**: Build the batch for a fixed set of comments (two files, a single line, a
range, a removed line, an outdated one, one over 50 lines) with no window and no session, and check
it byte for byte against the documented format; build it again and check the bytes are equal.

**Acceptance Scenarios**:

1. **Given** draft comments in two files, **When** the user chooses Send review, **Then** the
   session receives exactly one message: a fixed preamble, then one block per draft comment in the
   documented order, and nothing for comments that are not drafts.
2. **Given** a comment block, **When** it is read on its own, **Then** it names the comment's
   reference, its file path relative to the entry root with `/` separators, its side (current or
   removed) and line range, the quoted code in a fence that the code cannot close, and the comment
   text.
3. **Given** the same comments, **When** the batch is built twice, or on Linux, macOS and Windows,
   **Then** the bytes are identical.
4. **Given** a comment on more than 50 lines, **When** the batch is built, **Then** its block quotes
   the first and last lines with an elision line, keeping the full line range (as 482).
5. **Given** a comment whose code has changed since it was written and could not be followed
   (Story 4), **When** the batch is built, **Then** its block says the file has changed since and
   quotes the code as written (as 482).
6. **Given** the batch is delivered, **When** delivery succeeds, **Then** every comment in it
   becomes Sent; **When** it fails, **Then** every comment stays a draft and the user sees why
   (as 482).

---

### User Story 3 - One review panel for all of an entry's comments (Priority: P2)

Beside the diff, a review panel lists every comment of the entry, across all files, grouped by
file, each row with its lines, the start of its text, its status and, once answered, the start of
the agent's reply. From a row the user jumps to the comment in the diff, and edits or deletes a
draft. Send review, Clear sent and Discard drafts sit with the panel.

**Why this priority**: 482 shows comments only under their lines, one file at a time; with replies
coming back across many files, the user needs one place to see the whole review and its state. The
round trip (Stories 1 and 2) works without it.

**Independent Test**: With comments in three files in all four statuses, open the panel and check
each row's file, lines, status and texts; jump from a row to its comment; edit and delete a draft
from the panel; check that sent, applied and answered rows offer neither edit nor delete.

**Acceptance Scenarios**:

1. **Given** comments in three files, **When** the user opens the Changes view, **Then** the panel
   lists every comment of the entry grouped by file in path order and by line within a file, each
   with its status, and a count of comments per status.
2. **Given** a row in the panel, **When** the user selects it, **Then** the diff shows that file
   with the comment's lines in view and the comment's card highlighted; a comment the current diff
   cannot show (its kind of change is toggled off, or its file left the list) says so instead of
   jumping.
3. **Given** a draft row, **When** the user edits it from the panel, **Then** the same editor as in
   the diff opens with its text, and saving changes the text in the diff card and the panel alike;
   **When** the user deletes it, **Then** it disappears from both.
4. **Given** a sent, applied or answered row, **When** the user looks at it, **Then** it offers no
   edit or delete, and its full reply can be read from it.
5. **Given** an entry with no comments, **When** the user opens the panel, **Then** it says there
   are no comments and how to add one.
6. **Given** the panel open in two windows on the same entry, **When** a comment is added, edited,
   deleted, sent or answered in either, **Then** both panels show the change.

---

### User Story 4 - Comments follow their code after the agent edits (Priority: P2)

After the agent edits a file, lines above a comment are added or removed and the code the comment
quoted moves. The comment moves with its code and stays on the lines it was about. Only when the
quoted code itself is gone or changed, or can no longer be found in one place, is the comment
marked outdated, shown at the closest place it can be, and still sent with the code as written.

**Why this priority**: The issue asks for anchoring when the diff changes after the agent edits;
482 marks a comment outdated as soon as its line numbers no longer hold its code, which strands
correct comments whenever earlier lines shift.

**Independent Test**: With no window and no session, take a file and a comment on lines 10–12;
insert 5 lines above them, delete 3 lines above them, change line 11, and duplicate the quoted
lines elsewhere; after each change compute the comment's anchor and check it against the expected
place and outdated flag.

**Acceptance Scenarios**:

1. **Given** a comment on lines 10–12 of the current version, **When** 5 lines are inserted above
   line 10, **Then** the comment is shown on lines 15–17, not outdated, and the next batch names
   lines 15–17.
2. **Given** the same comment, **When** 3 lines above it are deleted, **Then** it is shown on lines
   7–9, not outdated.
3. **Given** the same comment, **When** one of its own lines is changed, **Then** it is marked
   outdated and shown at its last known place, kept within the file's length.
4. **Given** the quoted lines occur at more than one place after the edit, **When** the view
   refreshes, **Then** the comment goes to the occurrence nearest its last known place; if two are
   equally near it is marked outdated rather than guessed.
5. **Given** a comment on removed lines (base side), **When** the current version changes, **Then**
   the comment stays where it is, because the base version did not change.
6. **Given** a comment that was outdated, **When** a later edit restores its quoted code in one
   place, **Then** it is shown there and is no longer outdated.
7. **Given** a comment that is Applied or Answered, **When** its code changes (as an applied change
   usually does), **Then** it keeps its status and reply and is shown at its last known place; it
   is not shown as a problem.
8. **Given** the user writing a draft in the editor, **When** the diff refreshes under it and its
   lines move, **Then** the editor follows the lines and keeps the text (as 482).

---

### User Story 5 - Send to a structured session (deferred to epic #945)

Structured (stream-json) sessions do not exist yet: epic #945 and its sub-issues #938–#944 are all
open. This feature ships for terminal (PTY) sessions only. Delivering the batch to a structured
session as one user message, and commenting on a structured session's Edit/Write diffs, are left
to epic #945 (see Clarifications and Out of Scope). This feature keeps building the batch
independent of how it is delivered (FR-026), so a structured session can later send the same bytes
as a user message, and a structured session's agent can report through the same tool (FR-003).

---

### Edge Cases

- **No comments to send**: Send review is unavailable while the entry has no drafts (as 482).
- **Report while a send is in flight**: a report on a comment that is still part of a send that has
  not finished delivering is refused as "not sent yet"; the agent cannot have seen it.
- **Report for a cleared comment**: a report naming a comment the user has cleared or discarded is
  refused as "no such comment" and changes nothing.
- **Very long reply**: a reply over 4,000 characters is kept in full and shown shortened in the
  panel and the card, with the whole text readable on demand. A report whose reply exceeds 20,000
  characters is refused with the limit named.
- **Control characters in a reply**: a reply is shown as plain text; line breaks are kept, other
  control characters are not rendered as control codes.
- **Many comments**: an entry with 200 comments lists them all in the panel and builds the batch
  without the window becoming unresponsive.
- **Agent without the app's tool server**: a session whose AI CLI is not connected to the app's
  agent tool server cannot report; its comments stay Sent and the user can mark them by hand.
  The batch's instructions still ask for a per-comment answer in its own reply text.
- **Concurrency (Principle II)**: two sessions of the same entry may report on the same comment;
  the last report to arrive wins, and every window shows the same result. A session can report
  only on comments of its own entry, never another worktree's. Two windows editing the panel of
  the same entry see each other's changes (as 482).
- **Removal**: removing the worktree removes its comments, statuses and replies (as 482).
- **File gone**: a comment whose file has left the diff stays in the panel with its status, shown
  as not in the current diff; it is still sent if it is a draft.
- **Cross-platform (Principle VI)**: the batch and the comment references are byte-identical on
  Linux, macOS and Windows; line endings in quotes, comment texts and replies are normalised to
  `\n`; re-anchoring treats a line that differs only by its CRLF ending as the same line, and
  treats a line-ending change on its own as no change to the anchor.
- **Older stored comments**: comments kept by the 482 version (pending or sent, with no reference
  or reply) load as Draft or Sent; a sent one gets a reference on load, but since no batch carried
  that reference to the agent, it can only be marked by hand (FR-006).

## Requirements *(mandatory)*

### Functional Requirements

#### Statuses and replies

- **FR-001**: Every comment MUST have exactly one status among Draft, Sent, Applied and Answered.
  A new comment is a Draft; a delivered batch makes its comments Sent; a report makes a Sent,
  Applied or Answered comment Applied or Answered. No path leads back to Draft.
- **FR-002**: Only Draft comments MUST be editable or deletable by the user (as 482's pending).
- **FR-003**: The entry's sessions MUST be able to report on a comment by its reference, through
  the app's agent tool server, with an outcome (applied or answered) and a reply text. Answered
  MUST require a non-empty reply; applied MAY have an empty one. A report changes only the
  comment's status and reply, so it MUST NOT wait for the user's approval.
- **FR-004**: A report MUST be refused, with a reason naming the case, and change nothing, when the
  reference names no comment, a comment of another entry, a Draft, or a comment whose send has not
  finished delivering, or when the reply exceeds 20,000 characters.
- **FR-005**: A repeated report on the same comment MUST replace its outcome and reply; the comment
  MUST keep the time of its first and of its latest report.
- **FR-006**: The user MUST be able to mark a Sent comment Applied, or Answered with a note, by hand,
  and to read the full reply of any answered comment.
- **FR-007**: Each comment's status and reply MUST be shown on its card in the diff and on its row
  in the review panel, and MUST reach every window open on the project within 2 seconds of the
  report.
- **FR-008**: Statuses, replies and their times MUST be kept on the local filesystem with the
  entry's comments and survive restarts; comments kept by the 482 version MUST load as Draft
  (pending) or Sent (sent).

#### Batch

- **FR-009**: **Send review** (482's Send to session, renamed) MUST build one batch from every
  Draft comment of the entry and nothing else, and deliver it to the session 482 chooses.
- **FR-010**: The batch MUST consist of a fixed preamble followed by one block per comment, ordered
  by file path, then line, then side, then creation time. Each block MUST hold, in a fixed layout:
  the comment reference, the file path relative to the entry root with `/` separators, the side
  (current or removed) and line range, the outdated note when applicable, the quoted code (elided
  over 50 lines, as 482) in a fence the code cannot close, and the comment text.
- **FR-011**: The preamble MUST ask the agent to handle each comment by either applying the change
  or explaining why not, and to report each outcome by the comment's reference through the app's
  agent tool server, naming the tool; it MUST also ask the agent, when it cannot use that tool, to
  answer each comment by its reference in its reply.
- **FR-012**: The batch MUST be byte-identical for the same comments on every platform and on
  every build of the same set; it MUST contain no `\r`.
- **FR-013**: The batch format MUST be documented in the user guide with an example and a field by
  field description, and its preamble and block layout MUST change only together with that
  documentation.
- **FR-014**: The comment reference MUST be unique within the entry, stable for the comment's life,
  and short enough to quote (at most 12 characters).
- **FR-015**: Delivery success MUST make the batch's comments Sent; delivery failure MUST leave
  them Draft and show the user why (as 482).

#### Review panel

- **FR-016**: The Changes view MUST offer a review panel listing every comment of the entry across
  all files, grouped by file in path order and by line within a file, each row with its line
  range and side, the start of its text, its status, and the start of its reply when it has one,
  plus a count per status.
- **FR-017**: Selecting a row MUST show that comment's file and lines in the diff and highlight its
  card, or say why the current diff cannot show it.
- **FR-018**: A Draft row MUST offer edit and delete with the same effect as on the diff card;
  other rows MUST NOT.
- **FR-019**: Send review, Clear sent and Discard drafts MUST be reachable from the panel, with
  482's availability rules. Clear sent MUST remove comments that are Sent, Applied or Answered.

#### Anchoring

- **FR-020**: When the current version of a file changes, a comment on the current side MUST be
  re-anchored to the lines that hold its quoted code: unchanged in place if they still do,
  otherwise the single occurrence nearest its last known place.
- **FR-021**: A Draft or Sent comment MUST be marked outdated, and shown at its last known place within the
  file's length, when its quoted code is no longer present, or is present at two or more places
  equally near its last known place. An outdated comment whose quoted code later reappears in one
  place MUST be re-anchored there and lose the outdated mark.
- **FR-022**: A comment on the removed side MUST keep its anchor while the base version is
  unchanged.
- **FR-023**: Re-anchoring MUST update the line range the next batch names for a Draft comment,
  and MUST NOT change any comment's quoted code, text, status or reply.
- **FR-024**: An Applied or Answered comment whose code is gone MUST be shown at its last known
  place with no outdated mark.
- **FR-025**: Re-anchoring MUST treat lines that differ only in a trailing `\r` as equal.

#### Delivery and scope

- **FR-026**: Building the batch MUST NOT depend on how it is delivered; the same text goes to a
  terminal session typed as one pasted submission (as 482). This feature delivers only to terminal
  (PTY) sessions; delivery to a structured session is left to epic #945 (User Story 5).
- **FR-027**: The batch format, the comment reference, the status transitions, report validation
  and re-anchoring MUST be verifiable by automated tests that run with no window open and no
  session running.
- **FR-028**: The user guide MUST describe the review panel, statuses and replies, marking by
  hand, how comments follow their code, and the batch format; the agent tools page MUST describe
  the report tool.

### Key Entities

- **Review comment** (extends 482): entry, reference, file path, side, line range (the current
  anchor), quoted code, text, status, outdated flag, creation time, and, once reported, the
  outcome's reply and the times of the first and latest report.
- **Comment reference**: short identifier unique within the entry, printed in the batch.
- **Agent report**: reference, outcome (applied or answered), reply text, reporting session.
- **Batch**: preamble plus ordered comment blocks, and the set of comments it covers.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In the Story 1 independent test, the user learns the outcome of every comment from
  the app without reading the session's terminal: 2 of 2 comments show their status and reply.
- **SC-002**: For a set of 20 comments across 5 files, the batch holds 20 blocks, each with a
  distinct reference, and 0 comments are missing or duplicated.
- **SC-003**: The same comments give a byte-identical batch on Linux, macOS and Windows in 100% of
  test runs.
- **SC-004**: In the anchoring tests, a comment whose quoted code moved without changing is placed
  on the code's new lines in 100% of cases; no comment is placed on lines that do not hold its
  quoted code without being marked outdated (or Applied or Answered).
- **SC-005**: A report reaches every open window as a changed status within 2 seconds in 95% of
  trials.
- **SC-006**: With 200 comments in an entry, opening the panel and sending a review never leave the
  window unresponsive for more than 100 ms at a time.
  **Bugfix**: 2026-10-18 — BUG-001 This is a release-build number; the locate measurement is asserted in the release test run only.
- **SC-007**: After a restart, 100% of comments show the same status and reply as before it.

## Out of Scope

- Commenting on a structured session's Edit/Write diffs (the issue's "later"; see Story 5).
- Delivering the batch to a structured (stream-json) session as a user message; it waits for
  epic #945 (see Story 5).
- Posting the review to GitHub, or importing GitHub review comments.
- Back-and-forth threads: the user replying to the agent's reply on a comment. The user writes a new
  comment instead.
- Verifying that an "applied" comment's change was really made; the status is the agent's report.
- Reopening a comment (returning it to Draft) after it is sent.
- Comments on binary, non-text or not-shown large files (as 482).

## Assumptions

- The report reaches the app through the same agent tool server the app already gives each
  session (`docs/user-guide/agent-tools.md`), so a report carries which session and entry it came
  from; a session not connected to it cannot report (Edge Cases).
- Comments, statuses and replies are stored where 482 keeps comments, per entry, on the local
  filesystem; nothing leaves the device except the batch sent to a local AI CLI session
  (Principle IV).
- The 2-second propagation target and the 50-line quote limit reuse 482's values; the 4,000 and
  20,000 character reply limits are defaults to keep the panel readable and the store small, not
  settings.
- "Nearest occurrence" is measured in lines from the comment's last known start line.
- The panel sits in the Changes view of the entry; there is no project-wide review list.
