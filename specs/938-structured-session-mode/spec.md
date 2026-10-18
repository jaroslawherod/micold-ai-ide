# Feature Specification: Structured Session Mode — Claude over stream-json beside the Terminal Mode

**Feature Branch**: `claude/project-thread-cl0lw3`

**Created**: 2026-10-18

**Status**: Draft

**Input**: GitHub issue #938 "Structured session mode: run claude with stream-json next to the PTY
mode". "Part of the structured-session (stream-json) UI epic. Keeps the PTY/TUI session mode as the
fallback throughout. Add a session kind in the daemon that spawns `claude -p --input-format
stream-json --output-format stream-json --verbose --include-partial-messages` over pipes instead of
a PTY, and forwards parsed events to clients. Selectable per session; PTY stays the default.
Acceptance: typed event model (system/init, assistant, user, tool_use, tool_result, result, stream
deltas) with a tolerant parser (unknown event types are kept, not fatal); daemon session
abstraction supports both kinds; protocol carries events to the client; session resume works
(`--resume`); tests with recorded event fixtures."

## In short (for non-technical readers)

Today every AI session in the app shows the AI tool's own text screen, as if it ran in a terminal
window. This feature adds a second way to run a Claude Code session: the app talks to Claude in
a machine-readable form instead, receiving each step of Claude's work (its replies, the tools it
uses and their results, the end of each answer) as a separate, labelled item. On its own it shows
those items as a plain list; later features of the same epic turn them into a proper, readable
conversation view with input, permission prompts and usage figures. The terminal way stays the
default and keeps working as before. A user chooses the new way per session from the menu beside
the start-session button; it is offered for Claude Code only. Sessions run this way survive
restarts and continue the same conversation, several windows can watch the same session, and
nothing Claude writes, even something unexpected, can break the session.

## Context: where this sits in epic #945

Epic #945 builds a structured view of Claude sessions next to today's terminal view, in order:

| Issue | Delivers |
|---|---|
| **#938 (this)** | The session kind, the event model, the daemon side, the wire carrying events, resume |
| #939 | Read-only transcript view (assistant text, thinking, tool calls) |
| #940 | Markdown, highlighted code, Edit/Write diffs in that view |
| #941 | Prompt input, interrupt and queued messages |
| #942 | Permission prompts |
| #943 | Slash commands, model switching, falling back to the terminal mode |
| #944 | Usage, context meter, subagents, background tasks |

This feature is the foundation the rest stand on. It ships no transcript rendering of its own
beyond what the clarifications below settle; every richer view belongs to a later issue.

## Terms

- **Session kind**: how a session's AI CLI process is run and shown. **Terminal** (today's only
  kind, the default): the CLI runs in a pseudo-terminal and the user sees and types into its
  terminal screen. **Structured**: the CLI runs with its machine-readable streaming mode, exchanging
  one event per line on its standard input and output, with no terminal.
- **Event**: one line the CLI writes in structured mode, as the application understands it: a
  session-start (init) event, an assistant message, a user message, a turn result, a partial-message
  stream delta, an **unrecognised event** or an **unreadable line**. One line is one event.
- **Tool call / tool result**: a part of a message, not an event of its own: the CLI reports a tool
  call as a block inside an assistant message and its result as a block inside the following user
  message. The model types these blocks so a client can find every tool call and pair it with its
  result by the call's identity.
- **Unrecognised event**: a line that is valid structured data but of a type or shape the
  application does not model. It is kept whole, in order, never dropped and never fatal.
- **Unreadable line**: a line that is not valid structured data at all. It is kept as raw text in
  order, never fatal.
- **Discarded**: what the parser never does: every line read becomes exactly one event. Removing
  the oldest events from a bounded **event history** is eviction, not discarding, and is always
  flagged to the client.
- **User message**: what the user (or the application on the user's behalf, such as a session's
  first prompt) sends into a structured session; one per turn.
- **Event history**: the ordered events of one structured session that the daemon holds so that a
  client attaching later sees what it missed.
- **Recorded fixture**: a file holding the exact lines a real CLI wrote during a real structured
  session, kept in the repository and replayed by tests.

## Clarifications

### Session 2026-10-18

- Q: With which permission behaviour does a structured session run until #942 brings permission
  prompts? → A: The same permission configuration a terminal session of that location uses (FR-006
  already requires the same launch rules, and the daemon launches Claude with the user's own
  configuration plus only the tool-server binding, no permission override); a tool use that would
  need the user's approval is refused by the CLI and that refusal reaches clients as events.
  _(agent-resolved: crates/micold-core/src/provider.rs#ToolServerSupport, spec.md#FR-006)_
- Q: How does the user choose the structured kind for a session, and what does a window show for a
  structured session before #939 delivers the transcript view? → A: A per-session choice in the
  new-session flow, beside the existing AI CLI chevron, offered for Claude Code sessions only; the
  session's pane shows a plain, read-only, line-per-event log until #939 replaces it.
  _(decided by user, provisional)_
- Q: Does the event history survive a daemon restart? → A: No. The daemon keeps it for its own
  lifetime only; after a restart a resumed session continues the conversation and shows events from
  the resume on. Earlier history is #939's concern (it can read the CLI's own transcript).
  _(decided by user, provisional)_


## User Scenarios & Testing *(mandatory)*

### User Story 1 - Run a Claude session in structured mode (Priority: P1)

A user starts a Claude Code session in structured mode in a worktree. The daemon starts the CLI in
its streaming mode instead of in a terminal, sends it the session's first prompt as a user message,
reads every line it writes, turns each into an event, and forwards the events, in order, to every
client attached to the session. Terminal sessions are unaffected and remain the default.

**Why this priority**: Every other issue of the epic consumes these events. Without them nothing
else in the epic can be built or tested.

**Independent Test**: Start a structured session against a stand-in CLI that replays a recorded
fixture; an attached test client receives the fixture's events in order, each with the right
type, and the session's lifecycle reports running and then idle after the turn result.

**Acceptance Scenarios**:

1. **Given** a worktree and a Claude Code session created with the structured kind, **When** the
   session starts, **Then** the CLI runs without a terminal in its streaming mode with input and
   output both structured, verbose output and partial-message deltas enabled, in the session's
   working directory and with the same environment a terminal session of that location gets.
2. **Given** a running structured session, **When** the CLI writes an init event, assistant
   messages, tool calls, tool results, stream deltas and a turn result, **Then** each attached
   client receives each one as a typed event, in the order written.
3. **Given** a session created without naming a kind, **When** it starts, **Then** it is a terminal
   session exactly as today.
4. **Given** a structured session with a first prompt, **When** the CLI is ready, **Then** the
   prompt is delivered as the session's first user message, once.
5. **Given** a running structured session, **When** a client sends a user message for it, **Then**
   the daemon delivers it to the CLI as one structured user message, and the CLI's response events
   reach every attached client.

---

### User Story 2 - Nothing the CLI writes breaks the session (Priority: P1)

The CLI's event vocabulary grows with its releases. A new event type, a new field, or a malformed
line must not end the session, lose later events, or stop the stream reaching clients.

**Why this priority**: The epic names "parity drift with new claude releases" as a known gap; a
strict parser would turn every CLI release into an outage.

**Independent Test**: Replay a fixture that interleaves known events with an event of an unknown
type, a known type carrying unknown fields, and a line that is not structured data; every line
reaches the client in order, the known ones typed, the others kept whole.

**Acceptance Scenarios**:

1. **Given** the CLI writes an event of a type the application does not model, **When** it is
   read, **Then** it is forwarded as an unrecognised event carrying its full content, in its place
   in the order, and the session continues.
2. **Given** a known event type carrying extra fields, **When** it is read, **Then** it is
   recognised as its type and the extra fields do not cause a failure.
3. **Given** a known event type missing a field the model expects, **When** it is read, **Then** it
   is forwarded as an unrecognised event with its full content rather than failing the session.
4. **Given** a line that is not structured data, **When** it is read, **Then** it is forwarded as an
   unreadable line with its text, and the session continues.
5. **Given** the CLI writes to its error output, **When** it does, **Then** that text is recorded in
   the daemon log and does not interleave with or corrupt the event stream.

---

### User Story 3 - Resume a structured session (Priority: P2)

A structured session survives what a terminal session survives today: the CLI exiting, a restart,
the application or the daemon restarting. When it is started again it resumes the same Claude
conversation rather than starting a new one.

**Why this priority**: Sessions must be restorable (constitution Principle II); a structured
session that forgets its conversation on every restart is not usable.

**Independent Test**: Start a structured session against the stand-in CLI, stop it, restart it;
the stand-in records that it was launched with the resume handle of the same conversation, and the
session keeps its identity and kind.

**Acceptance Scenarios**:

1. **Given** a structured session whose conversation has started, **When** it is restarted or the
   daemon restores it, **Then** the CLI is launched in structured mode resuming that conversation.
2. **Given** a brand-new structured session, **When** it is first started, **Then** it starts a new
   conversation with the session's own identity, never a resume.
3. **Given** a structured session, **When** the application or the daemon restarts, **Then** the
   session is listed with its kind unchanged.
4. **Given** the resumed conversation no longer exists on disk, **When** the session starts, **Then**
   the failure is reported the same way a terminal session's failed resume is reported today.

---

### User Story 4 - Late and multiple clients see the same stream (Priority: P2)

Two windows attached to the same structured session, or a window attaching after events were
written, see the same ordered events.

**Why this priority**: Multi-window and reconnect are existing guarantees for terminal sessions;
structured sessions must not regress them.

**Independent Test**: Write events, then attach a second test client; it first receives the event
history, then live events, with no gap and no duplicate.

**Acceptance Scenarios**:

1. **Given** a structured session that has written events, **When** a client attaches, **Then** it
   receives the event history in order, followed by live events, with nothing missing or repeated
   at the seam.
2. **Given** two attached clients, **When** the CLI writes an event, **Then** both receive it.
3. **Given** a session whose event history has reached its bound, **When** the CLI writes another
   event, **Then** the oldest event is evicted, and a client attaching afterwards receives a marker
   saying earlier events were omitted before the remaining history.

---

### User Story 5 - A structured session behaves like a session, and refuses what it cannot do (Priority: P2)

A structured session is listed, started, stopped, killed and restarted like any session, and is
protected from crash loops the same way. Requests that cannot apply to it are refused with a reason
instead of being silently ignored.

**Why this priority**: The session kind must not open a second, weaker lifecycle beside the one
users and the rest of the application rely on.

**Independent Test**: With the stand-in CLI, create, stop, kill, restart and crash-loop a
structured session and compare each lifecycle report with a terminal session's; send a resize, a
keystroke input and a scrollback request to it and a structured request for a non-Claude CLI, and
check each refusal.

**Acceptance Scenarios**:

1. **Given** a structured session, **When** it is stopped, killed or restarted, **Then** its
   lifecycle reports the same states, in the same order, as a terminal session's.
2. **Given** a stand-in CLI that exits at once on every start, **When** a structured session
   starts, **Then** crash-loop protection stops restarting it as it does for a terminal session.
3. **Given** a request to create a structured Copilot, Pi, Codex or OpenCode session, **When** the
   daemon handles it, **Then** it is refused with a reason and no session is created.
4. **Given** a structured session, **When** a client sends it a resize, keystroke input or a
   scrollback request for its AI process, **Then** the request is refused with a reason.
5. **Given** a structured and a terminal session, **When** a client receives the session list,
   **Then** each entry names its session's kind.
6. **Given** the CLI writes to its error output, **When** it does, **Then** that text appears in
   the daemon log and in no event.

---

### User Story 6 - Choose the structured kind and see its events (Priority: P3)

A user picks "Claude Code (structured)" from the start-session chevron of a worktree; the session
starts in structured mode, its row says so, and its pane shows a plain read-only log, one line per
event, until #939 brings the transcript view. The user guide explains the kind.

**Why this priority**: The daemon side (US1–US5) is what the rest of the epic builds on; making the
kind selectable is what lets a user try it, and it must not ship before the daemon side is whole.

**Independent Test**: In the client's reducer tests, the start menu offers the structured entry
only with Claude Code and the chevron shows with Claude Code alone; picking the entry creates a
structured session; the log model turns event messages into one line each, with the omitted
marker first when events were omitted. The visual pass (quickstart Part B) confirms the row
marker and the pane.

**Acceptance Scenarios**:

1. **Given** a worktree where Claude Code is available, **When** the user opens the start-session
   chevron, **Then** its list offers "Claude Code (structured)" right after "Claude Code", even
   when Claude Code is the only CLI; **Given** Claude Code is not supported there, **Then** the
   entry is absent.
2. **Given** the user picks "Claude Code (structured)", **When** the session starts, **Then** it is
   a structured session and is marked as structured wherever sessions are listed; the plain
   start-session press and every other way of starting a session create a terminal session.
3. **Given** a structured session's pane, **When** events arrive, **Then** it shows one line per
   event in order, including unrecognised events, unreadable lines and the omitted-events marker,
   read-only, with no input.
4. **Given** the change that makes the kind selectable, **When** a user reads the user guide,
   **Then** it describes the structured kind, how to choose it, that it is Claude Code only, and
   what it does not yet do, naming each later issue of the epic.

---

### Edge Cases

- The CLI is not installed or cannot start: the session reports the same failure a terminal session
  reports today.
- The CLI exits mid-turn or crashes: the session's lifecycle and restart policy behave as for a
  terminal session (crash-loop detection included), and the events written before the exit are
  kept.
- A session of another AI CLI (Copilot, Pi, Codex, OpenCode) cannot be structured: the request is
  refused with a reason, and the session is not created half-way.
- A very long line (a large tool result or file content) is read whole; it is neither split into
  two events nor truncated.
- A partial line at the moment the CLI exits is kept as an unreadable line, not dropped.
- A user message sent while the session is not running, or while a turn is still in progress
  (before its turn result), is refused with a reason, not queued silently (queuing is #941).
- An empty or whitespace-only user message is refused with a reason and never reaches the CLI.
- Two clients send a user message at the same moment: the first to reach the daemon is delivered,
  the second is refused because a turn is now in progress; neither is merged into the other.
- The CLI exits or is restarted before writing a turn's result: the new process starts with no
  turn in progress, so a user message is accepted again once it runs, and clients are told the
  unfinished turn ended without a result.
- A restart or a resume never re-delivers the first prompt: it is delivered once in the session's
  life, not once per start.
- An older client that does not understand structured events attaches to a daemon that has
  structured sessions: the existing version handshake decides, as for any wire change.
- Operations that only make sense for a terminal (resize, scrollback, typed keystrokes) sent to a
  structured session's AI process are refused with a reason, not applied to nothing.
- The session's additional shell terminals (Regular mode) still work in a structured session; only
  the AI CLI process changes kind.
- Activity and attention signals that come from the CLI's hooks keep working in a structured
  session.
- Several structured sessions run at once, in the same or different worktrees: each session's
  events, history and user messages stay with that session only (Principle II).
- Linux, macOS and Windows: a structured session behaves the same on each; line endings the CLI
  writes on Windows do not produce unreadable lines or stray characters in event text, and stopping
  or killing the session ends the CLI process on every platform (Principle VI).
- A user message with multi-line text, quotes or non-ASCII characters reaches the CLI unchanged.

## Requirements *(mandatory)*

### Functional Requirements

**Session kind**

- **FR-001**: A session MUST have a kind, terminal or structured; terminal MUST be the default for
  every new session and for every session recorded before this feature.
- **FR-002**: The kind MUST be chosen when the session is created and MUST NOT change for the
  session's life (switching between kinds is #943).
- **FR-003**: Only Claude Code sessions MAY be structured; a request for any other AI CLI MUST be
  refused with a reason.
- **FR-004**: The kind MUST be persisted with the session and restored with it.

**Running a structured session**

- **FR-005**: A structured session MUST run the CLI without a terminal, in its print mode with
  structured input and output, verbose output, and partial-message deltas.
- **FR-006**: A structured session MUST start in the same working directory, with the same
  environment, identity handle and launch rules (new conversation or resume) as a terminal session
  of the same location, differing only in how input and output are exchanged.
- **FR-006a**: A structured session MUST run with the same permission configuration as a terminal
  session of the same location and MUST NOT add any permission override; a tool use that would need
  the user's approval is refused by the CLI, and that refusal reaches clients as ordinary events
  (asking the user is #942).
- **FR-007**: The daemon MUST deliver the session's first prompt, when it has one, as the first
  user message, exactly once in the session's life (never again on a restart or resume).
- **FR-008**: The daemon MUST accept a user message for a structured session from a client and
  deliver it to the CLI as one structured user message, with its text unchanged. It MUST refuse,
  with a reason, a message that is empty or blank, or that arrives while the session is not running
  or a turn is in progress.
- **FR-009**: A structured session MUST have the same lifecycle states, restart, stop and kill
  behaviour and crash-loop protection as a terminal session.
- **FR-010**: The CLI's error output MUST be recorded in the daemon log and MUST NOT enter the event
  stream.

**Event model and parsing**

- **FR-011**: The application MUST model these event types: session init, assistant message, user
  message, turn result, and partial-message stream delta; and, inside messages, these blocks: text,
  thinking, tool call and tool result. Together they MUST expose what the later views of the epic
  need (at least: message text and thinking, tool name, tool input and tool-call identity, tool
  result content, its error flag and the identity of the call it answers, the result's outcome,
  duration, cost and usage, and the session identity reported at init). A block of a kind the model
  does not know is kept with its full content inside its message.
- **FR-012**: An event of an unknown type, or of a known type that does not match the model, MUST
  be kept as an unrecognised event with its full original content.
- **FR-013**: A line that is not structured data MUST be kept as an unreadable line with its text.
- **FR-014**: No line the CLI writes MAY end the session or stop the stream, and the parser MUST
  NOT discard any line: each becomes exactly one event. Events MUST keep the order in which the CLI
  wrote them. (Eviction from the bounded history, FR-018, is the only way an event leaves it.)
  Besides the CLI's lines, the daemon adds its own notes as events: a turn that ended without a
  result (edge cases), and, on the wire only, a stand-in for an event too large for one message.
- **FR-015**: Extra fields on a known event type MUST NOT prevent it from being recognised.

**Carrying events to clients**

- **FR-016**: The protocol MUST carry each event of a structured session to every attached client,
  typed, in order.
- **FR-017**: A client attaching to a structured session MUST receive the event history and then
  live events, with no gap and no duplicate.
- **FR-018**: The event history held by the daemon MUST be bounded so that a long session cannot
  exhaust memory; when the bound drops old events, a client attaching later MUST be told that
  earlier events were omitted.
- **FR-019**: Terminal-only operations addressed to a structured session's AI process MUST be
  refused with a reason.
- **FR-020**: Session summaries sent to clients MUST carry the session's kind.

**Resume**

- **FR-021**: Restarting or restoring a structured session whose conversation has started MUST
  launch the CLI resuming that conversation in structured mode; a new session MUST start a new
  conversation.
- **FR-021a**: The event history MUST live in daemon memory only and is not written to disk; after
  a daemon restart a resumed session's history starts at the resume (earlier history is #939's,
  read from the CLI's own transcript). Stopping and starting a session within one daemon lifetime
  keeps its history.

**Selection and docs**

- **FR-022**: A user MUST be able to choose the structured kind per session from the start-session
  chevron's list, as a "Claude Code (structured)" entry offered only where Claude Code is available;
  the chevron MUST then be shown even when Claude Code is the only CLI. Every other way of starting
  a session, and the plain start-session press, creates a terminal session. The session MUST be marked as structured wherever sessions are listed.
- **FR-022a**: Until #939 replaces it, a structured session's pane MUST show a plain, read-only
  log with one line per event, in order, including unrecognised events and unreadable lines and
  the omitted-events marker; it offers no input (prompt input is #941).
- **FR-023**: The user guide MUST describe the structured kind, how to choose it, that it is
  Claude-only, and what it does not yet do (each later issue of the epic), in the same change that
  makes it user-selectable.

**Testing**

- **FR-024**: The event model and the parser MUST be tested against recorded fixtures captured from
  a real CLI, covering at least: a plain text turn, a turn with a tool call and its result, a turn
  with partial-message deltas, an error result, and a resumed session's init.
- **FR-025**: Structured session spawn, event forwarding, user message delivery and resume MUST be
  covered by integration tests that run the daemon against a stand-in CLI replaying fixtures, so no
  real CLI, network or account is needed in CI.

### Key Entities

- **Session kind**: terminal or structured; part of a session's persisted record.
- **Structured event**: one of the modelled types, an unrecognised event, or an unreadable line;
  carries its position in the session's stream.
- **Event history**: a session's bounded, ordered list of structured events in the daemon, with a
  marker when older events were dropped.
- **User message**: text sent into a structured session, one per turn.
- **Recorded fixture**: a captured line-per-event stream from a real CLI run, versioned with the
  tests, with the CLI version it was captured from.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of the lines in every recorded fixture reach an attached test client as one
  event each, in the fixture's order, each known event type recognised as that type and every tool
  call paired with its result.
- **SC-002**: A fixture with injected unknown types, unknown fields and non-structured lines
  produces zero session failures and zero discarded lines.
- **SC-003**: After a restart, a structured session resumes the same conversation in 100% of test
  runs; a new session never resumes.
- **SC-004**: Every existing terminal-session test passes unchanged: creating a session without a
  kind behaves exactly as before.
- **SC-005**: A client attaching after N events receives exactly N history events followed by live
  events, with no gap or duplicate, for N from 0 up to the history bound.
- **SC-006**: Events reach an attached client as the CLI writes them, not batched per turn: on the
  test stand-in, an event written while the turn is still in progress reaches the client before the
  turn's result is written.

## Assumptions

- The CLI's structured mode is the one the issue names; the application relies on the line
  vocabulary as observed in the recorded fixtures and treats anything else as unrecognised.
- The session's identity handle (the one used for new-conversation and resume today) works the same
  way in structured mode.
- Interrupting a turn, queuing messages, permission prompts, slash commands, model switching,
  switching a session between kinds, and any rich rendering are out of scope (#939–#944).
- Login and account setup stay the terminal mode's job; a structured session whose CLI is not
  logged in reports the CLI's error like any other failed turn.
- Hooks configured for the CLI fire in structured mode as they do in terminal mode, so existing
  activity and attention signals need no new source.
- The stand-in CLI used by tests is a test-only program and ships in no release.
