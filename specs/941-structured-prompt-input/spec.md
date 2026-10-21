# Feature Specification: Structured Session Prompt Input — composer, interrupt and queued messages

**Feature Branch**: `claude/project-thread-ik47pq`

**Created**: 2026-10-21

**Status**: Draft

**Input**: GitHub issue #941 "Structured session view: prompt input, interrupt and queued messages".
"Part of the structured-session (stream-json) UI epic. Keeps the PTY/TUI session mode as the
fallback throughout. Composer for sending user messages over stream-json input, with interrupt
(cancel current turn), image/file attachments via existing drop handling, and message queueing
while busy. Acceptance: Send, interrupt and queue work; state (idle/busy) reflected in the UI and
sidebar. Multi-line input and history." labels: enhancement, flow:feature, in-progress

## In short (for non-technical readers)

A structured Claude Code session (#938) can be read as a conversation since #939, but the user
cannot yet talk to it from the window. This feature adds a message box under the transcript. The
user writes a message (several lines if needed) and sends it; Claude answers in the transcript.
While Claude is answering, the session shows it is busy, in the pane and in the sidebar, and a
**Stop** control cancels the answer in progress without ending the session. Messages written while
Claude is busy are not lost or refused: they wait in a visible queue and go out, in order, when
Claude is done. Files dragged onto the pane are attached to the message: pictures are sent along
with it, other files are named by their path. Earlier messages can be recalled with the arrow keys,
as in a shell. Terminal sessions are unchanged and stay the default.

## Context: where this sits in epic #945

| Issue | Delivers |
|---|---|
| #938 (merged) | The structured session kind, the event model, the daemon side, the wire, and sending one user message while the session is idle |
| #939 (merged) | The read-only transcript view this feature puts the composer under |
| #940 | Markdown, highlighted code and Edit/Write diffs inside the transcript |
| **#941 (this)** | The composer: sending, multi-line input, input history, attachments, interrupt, the queue, and the idle/busy state in the pane and the sidebar |
| #942 | Permission prompts; its "waiting for you" state and deny-with-a-message sit beside this composer |
| #943 | Slash commands, model switching, falling back to the terminal mode |
| #944 | Usage, context meter, sub-agents, background tasks |

## Terms

- **Composer**: the message box at the bottom of a structured session's pane: a multi-line text
  area, the draft's attachments, a send control and, while the session is busy, a stop control.
- **Draft**: what the composer holds and has not sent yet: text and attachments.
- **Idle / busy**: a structured session is **busy** from the moment a user message is delivered to
  its CLI until that turn ends (its result arrives, it is interrupted, or the CLI process ends); it
  is **idle** while it runs and no turn is in progress. A session that is not running is neither:
  it is **not running** (stopped, starting, failed, ended).
- **Interrupt**: cancelling the turn in progress; the CLI stops answering, the turn ends, and the
  session stays running and becomes idle.
- **Queue**: the ordered list of messages the user sent while the session was busy, each waiting to
  be delivered when the session is next idle.
- **Queued message**: one entry of the queue: its text and attachments, editable or removable until
  it is delivered.
- **Attachment**: a file added to a draft by dropping it on the pane: an **image attachment** is
  sent with the message as a picture; a **file reference** is any other file, inserted into the
  text as its path.
- **Input history**: the session's earlier user messages, which the user can step through from the
  composer to recall one into the draft.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Send a message and see the session busy until it is answered (Priority: P1)

A user writes a message in the composer of an idle structured session and sends it. The message
appears in the transcript, the session shows busy in the pane and in the sidebar while Claude
answers, and shows idle again once the turn ends.

**Why this priority**: Without sending there is no conversation; it is the core of the issue and,
alone, already makes a structured session usable from the window.

**Independent Test**: With a stand-in CLI that answers each user message with a turn, send a
message from the composer and check: the CLI received it once with its text unchanged, the
transcript shows it, the session is busy until the stand-in writes the turn's result and idle
after, and the sidebar row's activity follows the same changes.

**Acceptance Scenarios**:

1. **Given** an idle running structured session, **When** the user writes text in the composer and
   sends it (the send key or the send control), **Then** the message is delivered to the CLI once
   with its text unchanged, the composer is emptied, and the message appears in the transcript.
2. **Given** a message was just delivered, **When** the turn is in progress, **Then** the pane shows
   the session is busy (a label and an indicator, not colour alone), the stop control is offered,
   and the session's row wherever sessions are listed shows it working.
3. **Given** a busy session, **When** the turn's result arrives, **Then** the pane and the sidebar
   show the session idle within 1 second, and the stop control is withdrawn.
4. **Given** an empty or whitespace-only draft with no attachment, **When** the user tries to send,
   **Then** nothing is sent and the send control is unavailable.
5. **Given** a session that is not running, **When** its pane is shown, **Then** the composer says
   the session is not running and does not send, and the user can still write and keep a draft.
6. **Given** a send that the service refuses (for a reason of #938's FR-008), **When** the refusal
   comes back, **Then** the composer shows the reason and the draft is put back unchanged, nothing
   lost.
7. **Given** a terminal session, **When** its pane is shown, **Then** no composer appears and the
   terminal behaves exactly as before.

---

### User Story 2 - Write multi-line messages and recall earlier ones (Priority: P1)

The user writes a message over several lines, pastes a block of text or code, and sends it as one
message. With the caret at the start of an empty or untouched draft, the up and down keys step
through the session's earlier messages, as in a shell, and a recalled message can be edited before
sending.

**Why this priority**: It is half of the issue's acceptance; prompts to Claude are routinely
several lines long, and re-sending or adjusting an earlier prompt is a frequent need.

**Independent Test**: Drive the composer state with key input: type two lines with the
new-line key, send, and check one message with both lines reached the CLI; then step back
through history and check the earlier messages are recalled newest first, that stepping forward
past the newest restores the draft the user had, and that editing a recalled message does not
change the history.

**Acceptance Scenarios**:

1. **Given** the composer has focus, **When** the user presses the new-line key combination,
   **Then** a line break is inserted and nothing is sent; **When** the user presses the send key,
   **Then** the whole text, line breaks included, is sent as one message.
2. **Given** text pasted into the composer with line breaks, tabs, quotes or non-ASCII characters,
   **When** it is sent, **Then** the CLI receives it unchanged.
3. **Given** a draft longer than the composer's height, **When** the user keeps typing, **Then**
   the composer grows up to a limit and then scrolls inside itself; the transcript stays usable.
4. **Given** a session with earlier user messages, **When** the user presses the history-back key
   with the caret on the draft's first line, **Then** the most recent earlier message replaces the
   draft in the composer; pressing it again recalls the one before, down to the oldest.
5. **Given** a recalled message, **When** the user presses the history-forward key with the caret on
   the last line, **Then** the next newer message is recalled; past the newest, the draft the user
   had before stepping into the history comes back unchanged.
6. **Given** the caret is inside a multi-line draft away from its first (or last) line, **When** the
   user presses up (or down), **Then** the caret moves between lines and the history is not
   stepped.
7. **Given** a draft in a session's composer, **When** the user switches to another pane or session
   and back in the same window, **Then** the draft, its attachments and its caret position are
   still there.

---

### User Story 3 - Interrupt a turn (Priority: P1)

Claude is going the wrong way. The user activates **Stop** (or presses the interrupt key while the
composer or transcript has focus); the turn ends at once, the transcript says it was interrupted,
and the session stays running and idle, ready for the next message.

**Why this priority**: It is in the acceptance, and without it a wrong turn can only be ended by
stopping the whole session; a terminal session offers it with one key today.

**Independent Test**: With a stand-in CLI that keeps a turn open until it is told to stop, send a
message, interrupt, and check the CLI was asked to interrupt the turn (not killed), that the turn
ends with an interrupted marking in the transcript, that the session is idle again, and that the
CLI process is still the same running process.

**Acceptance Scenarios**:

1. **Given** a busy session, **When** the user activates the stop control or the interrupt key,
   **Then** the CLI is asked to cancel the turn in progress, and the session's process keeps
   running.
2. **Given** the CLI ended the turn in answer to the interrupt, **When** it is shown, **Then** the
   turn closes with a notice that the user interrupted it, the session is idle in the pane and the
   sidebar of every window, and a new message can be sent.
3. **Given** an idle or not-running session, **When** its pane is shown, **Then** no stop control is
   offered and the interrupt key does nothing to the session.
4. **Given** an interrupt was asked for, **When** the CLI has not ended the turn within 10 seconds,
   **Then** the pane says the interrupt has not taken effect yet and offers stopping the session
   instead; the session is not stopped without the user asking.
5. **Given** two windows viewing a busy session, **When** both ask to interrupt it, **Then** the
   turn is interrupted once and both windows show it idle.
6. **Given** a pending permission request (#942) in the turn, **When** the turn is interrupted,
   **Then** the turn ends as above and #942's rules withdraw the request.

---

### User Story 4 - Queue messages while Claude is busy (Priority: P2)

While Claude answers, the user thinks of the next thing to ask and sends it anyway. Instead of
being refused, the message waits in a visible queue above the composer. When the turn ends, the
first queued message goes out on its own, and so on in order. Until it goes out, a queued message
can be edited, moved back into the composer or removed.

**Why this priority**: It is in the acceptance and removes the "wait until done" friction of #938's
refusal, but sending and interrupting are useful without it.

**Independent Test**: With a stand-in CLI, send one message and, while its turn is open, send two
more; check both are shown queued in order and not delivered; end the turn and check the first
queued message is delivered, alone; end that turn and check the second is delivered; remove a
queued message before its turn and check it is never delivered.

**Acceptance Scenarios**:

1. **Given** a busy session, **When** the user sends a draft, **Then** it is added to the end of the
   session's queue, shown above the composer with its position, the composer is emptied, and
   nothing reaches the CLI.
2. **Given** a non-empty queue, **When** the turn in progress ends with a result, **Then** the first
   queued message is delivered to the CLI as the next user message, alone, it leaves the queue and
   appears in the transcript, and the session is busy again.
3. **Given** a queued message not yet delivered, **When** the user removes it, **Then** it leaves the
   queue and is never delivered; **When** the user edits it, **Then** its text and attachments
   change in place in the queue, keeping its position; **When** the composer's draft is empty and
   the user moves it back into the composer, **Then** it leaves the queue and becomes the draft
   (moving it back is not offered while the draft holds anything, so no draft is displaced).
4. **Given** a queue, **When** the session's process ends or the session is stopped, **Then** the
   queue is kept, nothing in it is delivered while the session is not running, and the pane says
   the queue is on hold; **When** the session runs again and is idle, **Then** delivery resumes
   [NEEDS CLARIFICATION: resume delivery automatically after a restart, or wait for the user to
   release the queue?].
5. **Given** a queue, **When** the user interrupts the turn in progress, **Then**
   [NEEDS CLARIFICATION: is the queue kept and delivery continues with the next queued message,
   kept but put on hold until the user releases it, or emptied back into the composer as Claude
   Code's terminal does?].
6. **Given** two windows viewing the same session, **When** one queues a message, **Then**
   [NEEDS CLARIFICATION: is the queue the session's — held by the service, shown, editable and
   delivered the same in every window and kept when the window closes — or each window's own,
   held only while that window is open?].

---

### User Story 5 - Attach images and files by dropping them (Priority: P2)

The user drags a screenshot onto a structured session's pane: it appears as an attachment of the
draft, with its name and a small preview, and is sent with the message as a picture Claude can see.
A dropped source file or folder is inserted into the draft as its path, as a drop on a terminal
inserts it today.

**Why this priority**: In the issue's scope and how users show Claude a screenshot today, but text
messages are the main need.

**Independent Test**: Drop a PNG and a text file on a structured session's pane (driven through
the same drop events the terminal pane receives); check the draft holds one image attachment and
the text file's path in its text; send, and check the CLI received one user message carrying the
picture and the text with the path.

**Acceptance Scenarios**:

1. **Given** a structured session's pane, **When** the user drops an image file (PNG, JPEG, GIF or
   WebP) on it, **Then** the image is added to the draft as an attachment showing its file name and a
   preview, and the composer takes focus.
2. **Given** a draft with image attachments, **When** it is sent, **Then** the CLI receives one user
   message carrying the text and each image, in the order attached, and the transcript's user
   message shows that it had attachments and their names.
3. **Given** the user drops a file that is not one of those image kinds, or a folder, **When** the
   drop settles, **Then** its path is inserted into the draft's text at the caret, quoted where the
   path needs it, and nothing is attached.
4. **Given** an image attachment, **When** the user removes it from the draft before sending,
   **Then** it is not sent.
5. **Given** a dropped image larger than the size the CLI accepts, or a file that cannot be read,
   **When** it is dropped, **Then** it is not attached and the composer says why, naming the file.
6. **Given** a draft holding only attachments and no text, **When** it is sent, **Then** it is
   delivered as a message with the pictures alone.
7. **Given** files dropped on a terminal pane, **When** the drop settles, **Then** the terminal
   behaves exactly as before (feature 487).

---

### Edge Cases

- **Empty input**: an empty or whitespace-only draft with no attachments cannot be sent, and cannot
  be queued; whitespace around a non-empty message is sent as typed.
- **Sending at the turn boundary**: a message sent just as a turn ends is either delivered as the
  next message or queued and then delivered; it is never refused for a turn in progress, never
  delivered twice and never lost.
- **Two windows send at once to an idle session** (Principle II): one message is delivered; the
  other is neither refused for the turn in progress, merged nor lost: it waits in a queue behind
  the first (which queue, and whether both windows show it, is clarification 3 of User Story 4).
- **Two structured sessions** side by side in one window: each has its own composer, draft, queue,
  history and busy state; a send, an interrupt or a drop on one never reaches the other.
- **Session ends mid-turn** (crash, kill, restart): the turn closes as #939 shows it (ended without a
  result), the session is not running, the stop control is withdrawn and the queue is on hold.
- **Interrupt racing the turn's end**: the turn ends on its own just as the user interrupts; the
  session becomes idle once, no error is shown, and the next queued message is handled by the rules
  of User Story 4.
- **Refusal or delivery failure**: if the service refuses or cannot deliver a message (the session
  stopped between send and delivery), the message comes back to the composer (or stays at the head
  of the queue) with the reason; it is never silently dropped.
- **Lost connection to the service** while a draft or queue is held: the draft stays in the
  composer; sending is unavailable until the connection is back, and says so.
- **Very long message** (a pasted file of thousands of lines): it can be written and sent; the
  composer stays responsive and the message reaches the CLI unchanged.
- **Input method composition** (Chinese, Japanese, Korean, dead keys): the send key while text is
  still being composed completes the composition and does not send.
- **History**: a session with no earlier messages leaves the history keys as caret movement only;
  history holds what the session's user messages were, including ones sent from other windows and
  ones restored with the session's saved events; a message that was refused is not in the history.
- **Dropped images while the session is busy**: they attach to the draft as usual and travel with it
  into the queue.
- **Many files in one drop**: images and other files of one drop are split as above, in drop order;
  a drop of more images than the CLI accepts in one message attaches as many as it accepts and says
  that the rest were not attached.
- **Session deleted or forgotten** while it has a draft or queue: the draft and the queue are
  discarded with it.
- **Cross-platform (Principle VI)**: the send, new-line, history and interrupt keys are the same on
  Linux, macOS and Windows except where the platform's primary modifier differs (Command on macOS,
  Control elsewhere); dropped paths use the platform's form (drive letters and backslashes on
  Windows) and are quoted for the platform's shell rules as feature 487 does; images dropped from
  any platform's file manager attach the same way.
- **Light and dark theme**: the busy and idle states, the queue, attachments and refusal messages
  are readable in both themes and never conveyed by colour alone.

## Requirements *(mandatory)*

### Functional Requirements

**Composer and sending**

- **FR-001**: A structured session's pane MUST show a composer below the transcript in every
  state of the session; a terminal session's pane MUST NOT show one and MUST be unchanged.
- **FR-002**: The composer MUST accept multi-line text: a send key (Enter) sends the draft, and a
  new-line key combination (Shift+Enter) inserts a line break without sending; pasted text MUST
  keep its line breaks, tabs and characters.
- **FR-003**: Sending a draft to an idle running session MUST deliver it to the CLI as exactly one
  user message whose text is the draft's text unchanged, MUST empty the composer, and the message
  MUST appear in the transcript of every window viewing the session.
- **FR-004**: A draft with no text other than whitespace and no attachment MUST NOT be sendable.
- **FR-005**: When the service refuses or fails to deliver a sent message, the composer MUST show
  the reason, and the message MUST return to the composer (or stay queued) with its text and
  attachments unchanged.
- **FR-006**: While the session is not running, or the window has no connection to the service,
  the composer MUST say so and MUST NOT send, and MUST still let the user write and keep a draft.
- **FR-007**: Each session's draft (text, attachments, caret) MUST be kept per window while the
  window stays open, across switching panes and sessions.
- **FR-008**: The composer MUST grow with its content up to a height limit and then scroll inside
  itself, and MUST stay responsive with a draft of 10,000 lines.

**Idle / busy state**

- **FR-009**: Whether a structured session is idle, busy or not running MUST be the same in every
  window viewing it, MUST change in every such window when it changes, and a window that starts
  viewing a session MUST show its current state at once, including a turn that started before the
  window viewed the session.
- **FR-010**: A busy session's pane MUST show it is busy by a text label and an indicator, not by
  colour alone, and an idle session's pane MUST show it is ready for a message.
- **FR-011**: Wherever sessions are listed (the sidebar, the overview), a busy structured session
  MUST show the working activity a terminal session shows while its CLI works, and an idle one
  MUST stop showing it, within 1 second of the change; #942's "waiting for you" state takes
  precedence while a permission request is pending.

**Interrupt**

- **FR-012**: While a session is busy, its pane MUST offer a stop control and an interrupt key
  (Escape while the composer or the transcript has focus); while it is idle or not running, MUST
  NOT offer them.
- **FR-013**: An interrupt MUST make the CLI cancel the turn in progress and end it; the session's
  CLI process MUST be the same running process before and after (it is not ended, killed or
  restarted).
- **FR-014**: A turn ended by an interrupt MUST close in the transcript with a notice that the user
  interrupted it, in place of an ordinary footer, and the session MUST become idle.
- **FR-015**: An interrupt the CLI has not acted on within 10 seconds MUST be reported in the pane,
  with stopping the session offered as the next step; the session MUST NOT be stopped unless the
  user chooses it. The user MAY interrupt again while the notice is shown; when the turn then ends,
  by the interrupt or on its own, the notice MUST clear and the session become idle.
- **FR-016**: Interrupts from several clients for the same turn MUST interrupt it once; an interrupt
  for a session that is no longer busy MUST change nothing and MUST NOT be shown as an error.
- **FR-017**: The existing interrupt request a terminal session takes (Ctrl-C to its terminal) MUST
  keep its behaviour for terminal sessions.

**Queue**

- **FR-018**: Sending a draft while the session is busy MUST add it to the end of the session's
  queue instead of delivering it or refusing it; the queue MUST be shown above the composer, in
  order, with each message's text (cut to one line) and its attachments.
- **FR-019**: When a turn ends with a result and the queue is not empty, the first queued message
  MUST be delivered as the next user message, alone and unchanged, and leave the queue; one turn
  takes one queued message.
- **FR-020**: Until it is delivered, a queued message MUST be removable (never delivered) and
  editable in place (its position kept); while the composer's draft is empty it MUST also be
  movable back into the composer, leaving the queue.
- **FR-021**: No message MAY be delivered twice, delivered out of queue order, or lost while it is
  queued and the queue is held, across a turn ending, an interrupt and a session restart; whether
  the queue is held across a window closing is set by clarification 3 of User Story 4.
- **FR-022**: While the session is not running, the queue MUST be kept and shown on hold and
  nothing in it delivered; what happens to it on restart, on interrupt and across windows is set
  by the clarifications in User Story 4.

**Input history**

- **FR-023**: The history-back key (Up) with the caret on the draft's first line, and the
  history-forward key (Down) with the caret on its last line, MUST step through the session's
  earlier user messages, newest first; elsewhere in the draft they MUST move the caret.
- **FR-024**: Stepping forward past the newest message MUST restore the draft the user had before
  stepping into the history; editing a recalled message MUST NOT change the history.
- **FR-025**: The history MUST hold the user messages of the session's transcript (including those
  sent from other windows and those restored from saved events), oldest to newest, without refused
  messages.

**Attachments**

- **FR-026**: Files dropped on a structured session's pane MUST go to that session's composer, and
  MUST be grouped, ordered and quoted exactly as a drop of the same files on a terminal pane is
  (feature 487).
- **FR-027**: A dropped PNG, JPEG, GIF or WebP file within the CLI's accepted image size MUST be
  added to the draft as an image attachment showing its name and a preview, removable before
  sending; on send, the CLI MUST receive the images in one user message with the text, in the order
  attached.
- **FR-028**: Any other dropped file or folder MUST be inserted into the draft's text at the caret
  as its path, quoted as feature 487 quotes it, and MUST NOT be attached.
- **FR-029**: An image that cannot be read, is larger than the CLI accepts, or exceeds the number
  of images one message may carry MUST NOT be attached, and the composer MUST name the file and
  why.
- **FR-030**: A user message sent with image attachments MUST be shown in the transcript with an
  indication of its attachments and their names.

**Isolation**

- **FR-031**: Each structured session's composer, draft, queue, history and busy state MUST be
  separate from every other session's (Principle II); a send, an interrupt or a drop on one session
  MUST never reach another.
- **FR-032**: Two windows sending to an idle session at the same moment MUST result in one
  delivered message and the other queued behind it, neither refused for a turn in progress, merged
  nor lost; if the queue is the session's (clarification 3 of User Story 4), both windows MUST
  show it in the same order.

**Documentation**

- **FR-033**: The user guide's structured-sessions section MUST describe the composer (send and
  new-line keys, history keys, attachments), the busy and idle states, interrupting a turn and the
  queue, and MUST keep listing what the later issues of the epic add, in the same change that ships
  the composer.

### Constraints

- **Terminal mode untouched**: terminal sessions, their drop handling and their interrupt keep
  their behaviour; the composer exists only in structured sessions' panes.
- **Builds on #938 and #939**: the session kind, events, history, the user-message delivery and the
  transcript are reused; this feature replaces #938's refusal of a message during a turn by
  queueing, and adds the interrupt and an idle/busy state every window shares.
- **Shared UI components (Principle VIII)**: the composer, attachment chips, queue rows and busy
  indicator are built from, or added to, the shared component library with its builder-style
  interface, not as one-off widgets of this view.
- **Seam for #942 and #943**: the composer exposes a place for #942's "waiting for you" state and
  deny-with-a-message, and for #943's slash-command completion, without either being built here.

### Out of Scope

- Permission prompts and answering them (#942); slash commands, command completion and model
  switching (#943); usage, context meter and sub-agents (#944); markdown and diffs (#940).
- Pasting images from the clipboard, and attaching files through a file picker; only dropping
  attaches (the issue's "via existing drop handling").
- Attaching non-image file contents; such files are referenced by path, which Claude can read.
- Searching the input history, and keeping drafts across application restarts.
- Editing or re-running a message that was already delivered.

### Key Entities

- **Draft**: per window and session: text, caret, and ordered attachments.
- **Attachment**: a dropped image's name, kind, size and content, held until the message is sent or
  the attachment removed.
- **Queued message**: text and attachments, its position in the session's queue, and who queued it.
- **Session turn state**: per structured session: not running, idle, busy, or busy with an
  interrupt asked for, and since when.
- **Input history**: the session's delivered user messages in order, and per window the position
  stepped to and the draft saved when stepping in (restored on stepping out, never added to the
  history).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 100% of the send test cases (single-line, multi-line, pasted non-ASCII, with and
  without images), the CLI receives exactly one user message per send with the text unchanged, and
  the transcript shows it once.
- **SC-002**: After a turn starts or ends, every window viewing the session, and its sidebar row,
  shows the new idle/busy state within 1 second.
- **SC-003**: An interrupt ends the turn in progress, with the session still running and idle,
  within 3 seconds in 95% of runs against a stand-in CLI that honours interrupts, and the session's
  process is the same process before and after in 100% of runs.
- **SC-004**: Across the queue test cases (several queued messages, removals, edits, interrupts,
  restarts, and two windows sending at once), 0 messages are delivered twice, out of order or
  lost while their queue is held.
- **SC-005**: In 100% of the refusal and lost-connection test cases, the user's text and
  attachments are back in the composer (or still queued) unchanged.
- **SC-006**: A user can recall and resend their previous message with two key presses (history
  back, send).
- **SC-007**: Typing into a 10,000-line draft never stops the window responding to input for more
  than 100 ms in a release build on a CI runner.
- **SC-008**: Every existing terminal-session test, including the drop-on-terminal tests of feature
  487, passes unchanged.

## Assumptions

- Enter sends and Shift+Enter inserts a line break, as in Claude Code's terminal interface; Up and
  Down step the history only at the draft's first and last line, as in Claude Code.
- The CLI's structured input carries an interrupt control request that cancels the turn in
  progress and makes the CLI end it, and carries user messages whose content is a list of text and
  image blocks; the image kinds and size limit are the CLI's (PNG, JPEG, GIF, WebP).
- Escape interrupts only while focus is in the session's composer or transcript, so it does not
  steal Escape from dialogs, menus or the quick switcher.
- The idle/busy state is kept by the background service that runs the session and sent to every
  window, since several windows must agree on it; the interrupt is a request on the CLI's
  structured input, not a signal to its process.
- The busy state starts when the service delivers a message to the CLI, not when the user presses
  send, so a queued message does not make the session busy by itself.
- The working activity shown for a busy structured session is the same signal and presentation a
  terminal session gets from its CLI's hooks; when hooks also report activity, the turn state
  decides between working and idle for a structured session.
- One queued message is delivered per turn, unmerged, so each shows as its own turn in the
  transcript; merging queued messages into one is not done.
- Drafts and the per-window history position are in memory only; the history's content comes from
  the session's events, which #938 already saves.
- A turn ended by an interrupt is recognised from what the CLI reports for it; when the CLI reports
  nothing distinguishable, the pane's knowledge that an interrupt was asked for marks it.
