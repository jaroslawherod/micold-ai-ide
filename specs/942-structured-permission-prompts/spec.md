# Feature Specification: Structured Session Permission Prompts — approve, deny, plan approval and questions

**Feature Branch**: `claude/project-thread-tt60gq`

**Created**: 2026-10-21

**Status**: Draft

**Input**: GitHub issue #942 "Structured session: permission prompts via permission-prompt-tool".
"Part of the structured-session (stream-json) UI epic. Keeps the PTY/TUI session mode as the
fallback throughout. Serve an MCP permission-prompt tool (`--permission-prompt-tool`) from the
daemon and show an approve/deny/always-allow UI for tool requests. Also surface plan mode
(ExitPlanMode) and AskUserQuestion as dedicated UI. Acceptance: Tool request blocks the turn until
answered; decision returned to claude. Permission mode (default/acceptEdits/plan/bypass) selectable
per session. AskUserQuestion renders options and returns the answer. Reuse the existing hooks
receiver where possible." Labels: enhancement, flow:feature.

## In short (for non-technical readers)

A structured Claude session (feature 938) cannot ask the user anything today: when Claude wants to
run a tool that needs permission, such as editing a file or running a command, the request is
refused on the spot and Claude carries on without it. This feature lets the request reach the user
instead. The session pauses, the app shows what Claude wants to do with **Allow**, **Deny** and
**Always allow** buttons, and Claude continues with the user's answer. Two special requests get
their own views: when Claude has drawn up a plan and asks to start working on it, the user reads
the plan and approves or rejects it; when Claude asks the user a multiple-choice question, the user
picks an answer from the options shown. Each session also gets a permission mode, chosen by the
user: ask as usual, accept file edits without asking, plan only, or never ask. The terminal way of
running a session stays the default and is unchanged.

## Context: where this sits in epic #945

| Issue | Delivers | State |
|---|---|---|
| #938 | The structured session kind, event model, daemon side, wire, resume | merged (`specs/938-structured-session-mode`) |
| #939 | Read-only transcript view (assistant text, thinking, tool calls) | in progress, not on main |
| #940 | Markdown, highlighted code, Edit/Write diffs in that view | planned |
| #941 | Prompt input (composer), interrupt, queued messages, idle/busy state | in progress, not on main |
| **#942 (this)** | Permission prompts, permission mode, plan approval, questions | |
| #943 | Slash commands, model switching, falling back to the terminal mode | depends on this |
| #944 | Usage, context meter, subagents, background tasks | |

**Dependencies.** The daemon and protocol side of this feature (serving the prompt tool, holding a
pending request, returning the decision, the permission mode) builds only on what #938 merged and
is specifiable and testable without any later issue. The window side depends on two features not
yet on main: the prompts, plan approval and questions are shown inside **#939's transcript view**,
at the tool call they belong to, and the permission-mode control, the "waiting for you" state and
denying with a message to Claude sit in or beside **#941's composer** and use its idle/busy state.
Until both are merged, the window parts of this feature cannot be built; the plan orders the
milestones so that the daemon side lands first. **#943** needs this feature: a skill that asks for
permission or asks the user a question must run unchanged in a structured session.

## Terms

- **Permission request**: Claude's request, during a turn, to use one tool with a given input, put to
  the application because the session's permission rules neither allow nor forbid it outright. It
  names the tool, its input and the tool call it belongs to.
- **Pending request**: a permission request the user has not yet answered. While one is pending the
  turn waits.
- **Decision**: the answer returned to Claude for one request: **allow** (optionally with a changed
  input, as for a question's answers) or **deny** (with a message Claude reads).
- **Always allow**: an allow that also adds a rule so that the same kind of request is allowed
  without asking from then on, within the rule's scope.
- **Permission mode**: how a structured session's Claude treats tool requests, one of **Default**
  (ask when the rules require it), **Accept edits** (file edits in the working directory are allowed
  without asking), **Plan** (Claude only reads and plans, and asks to leave plan mode with a plan),
  **Bypass** (nothing is asked; every tool runs). These are the CLI's own modes.
- **Plan approval**: the request Claude makes, in plan mode, to stop planning and start working
  (the CLI's `ExitPlanMode` tool). It carries the plan.
- **Question**: a request in which Claude asks the user one or more multiple-choice questions (the
  CLI's `AskUserQuestion` tool). Each question has a text, a short header, two or more options each
  with a label and a description, and whether several options may be chosen.
- **Prompt tool**: the tool the CLI calls to put a permission request to the application
  (`--permission-prompt-tool`), served by the daemon.
- **Structured session**, **event**, **event history**, **turn**: as feature 938 defines them.

## Clarifications

_None yet. Open questions are marked in the requirements._

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Approve or deny a tool request (Priority: P1)

A user works in a structured Claude session with the Default permission mode. Claude decides to run
a shell command the user's rules do not allow outright. Instead of the request being refused, the
turn pauses and the session's view shows the request at its tool call: the tool's name, what it
would do (the command, the file and the change, the address), and **Allow**, **Deny** and **Always
allow**. The user presses Allow; Claude runs the command and the turn continues. Later the user
denies another request and types why; Claude reads the reason and adjusts.

**Why this priority**: Without it a structured session can only do what the rules already allow,
which makes it unusable for real work; it is the core of the issue.

**Independent Test**: With the daemon running a stand-in CLI that issues a permission request,
attach a test client, see the pending request arrive, answer it, and check the stand-in receives
the decision and the turn ends with the stand-in's result. In the window, the same with a request
shown in the transcript view.

**Acceptance Scenarios**:

1. **Given** a structured session in a turn, **When** Claude asks permission for a tool use, **Then**
   every attached client is shown the pending request with the tool's name and input, and the turn
   does not continue until a decision is returned.
2. **Given** a pending request, **When** the user allows it, **Then** Claude receives an allow
   decision with the original input, runs the tool, and the request is shown as allowed.
3. **Given** a pending request, **When** the user denies it, optionally with a message, **Then**
   Claude receives a deny decision carrying that message (or a standard one when none is given),
   does not run the tool, and the request is shown as denied.
4. **Given** a pending request, **When** the user chooses Always allow, **Then** Claude receives an
   allow decision together with a rule that allows that kind of request, and a later identical
   request in the same session is not asked again.
5. **Given** two windows attached to the session, **When** one answers, **Then** the request closes
   in both and the other window's answer, if any, has no effect.
6. **Given** a pending request and no window open, **When** a window attaches later, **Then** it is
   shown the pending request and can answer it.

---

### User Story 2 - Choose a session's permission mode (Priority: P1)

A user starting a structured session chooses its permission mode: Default for normal work, Accept
edits to let Claude change files without asking, Plan to have Claude only investigate and propose,
or Bypass to never be asked. The session's view always shows the current mode.

**Why this priority**: The acceptance makes the mode selectable per session; it decides how often
the prompts of story 1 appear and is the way into plan mode for story 3.

**Independent Test**: Create structured sessions with each mode against the stand-in CLI and check
the CLI is launched in that mode, the mode is shown to clients and survives a restart.

**Acceptance Scenarios**:

1. **Given** a new structured session, **When** the user chooses no mode, **Then** it runs with the
   permission configuration a terminal session of that location uses (feature 938 FR-006a), shown
   as Default.
2. **Given** the user chooses Accept edits, Plan or Bypass, **When** the session starts, **Then**
   the CLI runs in that mode and clients show it.
3. **Given** a structured session with a chosen mode, **When** it is restarted or the daemon
   restarts and resumes it, **Then** it runs in the same mode.
4. **Given** the user chooses Bypass, **When** they confirm the choice, **Then** the session runs
   without asking for any tool use, and its view says so plainly.
5. **Given** a running session, **When** the user changes its mode, **Then**
   [NEEDS CLARIFICATION: can the mode be changed while a session runs (taking effect at once, for
   the next tool request), or only chosen when the session is created?].

---

### User Story 3 - Approve or reject a plan (Priority: P2)

A user runs a structured session in Plan mode. Claude investigates, then asks to start working with
a plan. The view shows the plan in full with **Approve and accept edits**, **Approve** (ask before
edits) and **Keep planning**. The user approves; the session leaves plan mode, in the mode chosen,
and Claude begins. Rejecting with a note keeps Claude planning and gives it the note.

**Why this priority**: Plan mode is one of the four modes the acceptance names; without a plan view
it can only be left by ending the session.

**Independent Test**: The stand-in CLI issues a plan approval request; a test client approves or
rejects it and the stand-in receives the decision; on approve, the session's mode as shown changes.

**Acceptance Scenarios**:

1. **Given** a session in Plan mode, **When** Claude asks to leave plan mode, **Then** clients show
   a plan approval with the plan's full text, not a generic tool request.
2. **Given** a plan approval, **When** the user approves with accept edits (or without), **Then**
   Claude receives an allow decision, the session's mode becomes Accept edits (or Default, whatever
   mode the session had before Plan), and the shown mode changes.
3. **Given** a plan approval, **When** the user keeps planning, optionally with a note, **Then**
   Claude receives a deny decision with the note, and the session stays in Plan mode.

---

### User Story 4 - Answer Claude's question (Priority: P2)

Claude, or a skill it runs, asks the user to choose between options ("Which database should the
migration target?"). The view shows each question with its options as choices (one, or several
when the question allows it) and an **Other** field for a free answer. The user picks and submits;
Claude continues with the answer.

**Why this priority**: A required acceptance item, and what lets skills that ask questions run in a
structured session (#943).

**Independent Test**: The stand-in CLI issues a question request with two questions, one
multi-select; a test client answers both and the stand-in receives the answers in the decision.

**Acceptance Scenarios**:

1. **Given** Claude asks one or more questions, **When** the request arrives, **Then** clients show
   every question with its header, text and every option's label and description, as single or
   multiple choice as the question says.
2. **Given** a question request, **When** the user answers every question and submits, **Then**
   Claude receives an allow decision whose input carries each question's chosen option labels (or
   the free answer), and continues.
3. **Given** a question request, **When** the user declines to answer, **Then** Claude receives a
   deny decision saying the user declined, and continues without an answer.
4. **Given** a question request, **When** not every question has an answer, **Then** it cannot be
   submitted.

---

### User Story 5 - Know when a session waits for you (Priority: P2)

A user has several sessions open. One structured session's Claude asks for permission. The session
is marked as waiting for the user wherever sessions are listed, and, if the user turned on
needs-permission notifications, a desktop notification is shown, exactly as for a terminal session
that asks for permission. When the request is answered, the mark clears.

**Why this priority**: A paused turn the user does not notice is a session that silently stops
working; the application already has this signal for terminal sessions and must give it here too.

**Independent Test**: With a stand-in CLI issuing a request, check the session's activity becomes
"needs permission" while it is pending and returns to working when answered.

**Acceptance Scenarios**:

1. **Given** a structured session, **When** a request becomes pending, **Then** the session's
   activity is "needs permission", as a terminal session's is when its CLI asks for permission, and
   the needs-permission notification fires under the same setting.
2. **Given** a pending request, **When** it is answered or withdrawn, **Then** the session's
   activity returns to busy (the turn continues) or idle (the turn ended).

---

### User Story 6 - Requests never outlive their turn (Priority: P3)

The user interrupts a turn (#941), stops the session, or the CLI exits while a request is pending.
The request disappears from every window; nothing answered later reaches a process that no longer
asks.

**Why this priority**: Correctness of the pending state; rare in practice.

**Independent Test**: With a request pending, stop the session or kill the stand-in; check every
client is told the request is withdrawn and an answer sent afterwards is refused with a reason.

**Acceptance Scenarios**:

1. **Given** a pending request, **When** the session stops, its process ends, or the turn is
   interrupted, **Then** the request is withdrawn from every client and, if the CLI still waits,
   it receives a deny decision.
2. **Given** a withdrawn or already answered request, **When** a client sends a decision for it,
   **Then** the daemon refuses it with a reason and changes nothing.

### Edge Cases

- **Several requests at once**: Claude may ask for more than one tool use in one turn (parallel tool
  calls). Each is its own pending request, shown at its own tool call and answered independently,
  in any order.
- **Bypass mode**: requests are not expected; should the CLI still send one, it is shown and
  answered normally, never auto-answered by the application.
- **A plan approval with an empty or missing plan**: shown as a generic tool request (allow or
  deny), never dropped.
- **A request for a tool the application has no dedicated view for** (an MCP server's tool, an
  unknown tool): shown as a generic request with the tool's name and its input as structured text.
- **A very large input** (a Write of a big file): the view shows a bounded preview with a way to see
  the whole input; the decision still returns the original input unchanged.
- **The request names a tool call the transcript does not show** (it was evicted from the bounded
  history, or the request arrived first): the request is still shown, at the end of the transcript.
- **A malformed request** from the CLI (missing tool name or input): the daemon answers deny with a
  reason, logs it, and the turn continues; it never blocks the turn forever.
- **A question with no options or an unknown shape**: shown as a generic request (allow or deny),
  never dropped.
- **Daemon restart with a request pending**: the CLI process ends with the daemon; the turn ended
  without a result (feature 938 edge cases), and no request is restored as pending.
- **The window closes mid-answer**: nothing is sent; the request stays pending for the next window.
- **A sandboxed session** (its CLI runs in a container): requests reach the daemon and decisions
  reach the CLI exactly as for an unsandboxed one.
- **Terminal sessions**: unchanged; their CLI asks in its own terminal screen as before.
- **Platforms**: requests, decisions and the views behave the same on macOS, Linux and Windows; the
  tool server is a loopback listener on every platform (feature 034), so nothing here is
  platform-specific.
- **Several sessions waiting at once**: each session's requests are its own; answering one session's
  request never resolves another's, and each session is marked as waiting on its own.

## Requirements *(mandatory)*

### Functional Requirements

**Receiving requests**

- **FR-001**: Every structured Claude session's permission requests MUST reach the daemon instead
  of being refused by the CLI (replacing feature 938 FR-006a's refusal). Only the session that
  raised a request MAY raise it: no other session, and nothing outside the user's own sessions, can
  create a request for, or answer one of, another session. Terminal sessions MUST NOT be changed
  (FR-018). (The transport, the prompt tool the issue names, is in Assumptions.)
- **FR-002**: The daemon MUST turn each prompt-tool call into a pending request of that session,
  carrying the tool name, its input, the tool call's identity, and the request kind: **plan
  approval** for the CLI's plan-exit tool, **question** for its ask-the-user tool when its input has
  the question shape, otherwise **tool request**.
- **FR-003**: A pending request MUST hold the turn: the daemon MUST NOT answer the call until a
  decision is made or the request is withdrawn (FR-011), and MUST NOT time it out on its own.
- **FR-004**: A malformed call (no tool name, or an input that is not structured data) MUST be
  answered with a deny decision and a reason at once, logged without its input, and MUST NOT become
  a pending request.

**Showing requests**

- **FR-005**: The protocol MUST carry each pending request to every client attached to the session,
  and its resolution (allowed, denied, withdrawn) when it ends. A client attaching while a request is
  pending MUST receive it, with no request missed and none shown twice.
- **FR-006**: A tool request MUST be shown at the tool call it belongs to in the session's
  transcript view (#939), naming the tool and showing its input in the tool's familiar form where
  the transcript view has one (the command of a shell call, the file and change of an edit), and
  otherwise as structured text; with **Allow**, **Deny** (with an optional message) and **Always
  allow**.
- **FR-007**: A plan approval MUST show the plan's full text with **Approve and accept edits**,
  **Approve** and **Keep planning** (with an optional note).
- **FR-008**: A question MUST show every question's header and text and every option's label and
  description, as single or multiple choice as the question says, with a free-answer choice; it
  MUST be submittable only when every question is answered; and it MUST offer declining to answer.
- **FR-009**: While a request is pending, the session's input area (#941) MUST show that the
  session waits for the user, and the session MUST get the same activity a terminal session gets
  when its CLI's `Notification` hook reaches the hook receiver: awaiting input, shown as "Blocked:
  needs permission" wherever sessions are listed, and the needs-permission notification (feature
  613) under its existing setting. When no request is pending the activity MUST follow the turn as
  before.

**Returning decisions**

- **FR-010**: The daemon MUST accept a decision for a pending request from any attached client, and
  the first decision MUST win: the daemon MUST return it to the CLI as the prompt tool's answer,
  so that Claude acts on it, and tell every client the request is resolved:
  - **Allow** returns allow with the original input unchanged.
  - **Deny** returns deny with the user's message, or a standard message when none was given.
  - **Always allow** returns allow and a permission rule for the request, taken from the
    suggestions the CLI sent with it when it sent any, scoped
    [NEEDS CLARIFICATION: to this session only, or written to the location's Claude settings so
    that later sessions, terminal ones included, inherit it?].
  - A plan approval's **Approve and accept edits** and **Approve** return allow and set the
    session's mode to Accept edits and Default respectively, whatever mode the session had before
    Plan; **Keep planning** returns deny with the
    note and leaves the mode at Plan.
  - A question's answers return allow with the input carrying each question's chosen labels, or the
    free answer, so that Claude reads them as the user's answers; declining returns deny.
- **FR-011**: A pending request MUST be withdrawn from every client, and the CLI, while it still
  waits, answered deny, when its turn is interrupted, its session stops or is removed, or its
  process ends. Nothing is restored as pending after a daemon restart.
- **FR-012**: A decision for a request that is not pending (answered, withdrawn, unknown, or of
  another session) MUST be refused with a reason and change nothing.

**Permission mode**

- **FR-013**: A structured session MUST have a permission mode, one of Default, Accept edits, Plan
  and Bypass; Default MUST be the mode of every structured session that chose none, including every
  one created before this feature, and in Default the daemon MUST add no permission-mode override
  to the launch (feature 938 FR-006a).
- **FR-014**: A user MUST be able to choose the mode when creating a structured session; choosing
  Bypass MUST require a confirmation that names its effect.
- **FR-015**: The mode MUST be persisted with the session, carried in its summary to clients, shown
  in the session's view, and used on every launch of it (restart, resume, daemon restart).
- **FR-016**: The mode MUST follow what Claude reports: when a plan approval is approved (FR-010)
  or the CLI reports a mode change in its events, the shown and persisted mode MUST change to it.
- **FR-017**: Changing the mode of a running session: governed by the answer to user story 2's
  clarification; if it is allowed, the change MUST take effect for the next tool request without
  restarting the CLI and without losing the conversation.

**Scope, tests and docs**

- **FR-018**: Terminal sessions MUST be unchanged: no prompt tool, no permission-mode override, and
  every existing terminal-session test passes unchanged.
- **FR-019**: Request handling, decisions of every kind, first-decision-wins across two clients,
  withdrawal on stop, process end and interrupt, attach with a request pending, and the mode at
  launch MUST be covered by integration tests that run the daemon against the stand-in CLI
  (feature 938 FR-025) extended to make prompt-tool calls, so no real CLI, network or account is
  needed in CI.
- **FR-020**: The request shapes the parser and the decision shapes the daemon returns MUST be
  checked against recorded fixtures captured from a real CLI: a tool request, an always-allow with
  suggestions, a plan approval, and a question with a multi-select.
- **FR-021**: The user guide MUST describe permission prompts, Always allow and its scope, plan
  approval, questions, the permission modes and Bypass's risk in structured sessions, and remove
  "permission prompts" from the structured kind's not-yet list, in the change that makes them
  user-visible.

### Key Entities

- **Permission request**: session, request id, tool call identity, tool name, input, kind (tool
  request, plan approval, question), the CLI's rule suggestions, state (pending, allowed, denied,
  withdrawn).
- **Decision**: request id, allow (with input, optional rules, optional new mode) or deny (with
  message).
- **Permission mode**: Default, Accept edits, Plan, Bypass; part of a structured session's
  persisted record.
- **Question**: header, text, options (label, description), multi-select flag; the answer is the
  chosen labels or a free text per question.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 100% of test runs, a stand-in CLI that makes a prompt-tool call receives no answer
  until a client decides, and then receives exactly that decision; its turn then ends with its
  result.
- **SC-002**: With two clients deciding the same request, the CLI receives exactly one decision and
  both clients see the same resolution, in 100% of runs.
- **SC-003**: Stopping the session, ending the process or interrupting the turn with a request
  pending withdraws it from every client within one second, and a later decision for it is refused,
  in 100% of runs.
- **SC-004**: 100% of the recorded request fixtures (tool request, always-allow, plan approval,
  multi-question) are classified as their kind, and each decision the daemon returns for them
  matches the decision recorded from the real CLI in the same fixture.
- **SC-005**: A structured session launched in each of the four modes runs in that mode, and keeps
  it after a restart and a daemon restart, in 100% of test runs.
- **SC-006**: A user can answer a tool request, a plan approval and a two-question request in the
  window without leaving the session's view, each in at most three clicks or key presses beyond
  choosing the answers (counted in the quickstart walk-through).
- **SC-007**: Every existing terminal-session and structured-session test passes unchanged.

## Assumptions

- The CLI's permission-prompt option, its call and answer shapes, its rule suggestions, and its
  plan-exit and ask-the-user tools behave in print mode as observed in the recorded fixtures; the
  issue fixes the transport: a prompt tool served by the daemon on the session service's existing
  tool server (feature 034) behind the session's existing tool-server credential, named to the CLI
  by its permission-prompt option (`--permission-prompt-tool`) at every structured launch. Whether the CLI's own in-stream
  control channel would serve better is a research question for the plan, not a change of the
  acceptance.
- The session service's tool server (feature 034) and its per-session binding already reach every
  structured session, sandboxed ones included, so the prompt tool needs no new listener.
- The hook receiver is reused for what it can do and not for the decision: a hook's reply is not
  how the issue's transport returns a decision, so the blocking request and its answer go through
  the prompt tool. What is reused is the activity path the `Notification` hook drives today
  (awaiting input, "Blocked: needs permission", the needs-permission notification): a pending
  request drives the same path, whether or not the CLI also fires its `Notification` hook in print
  mode, and the two never show the session as waiting twice.
- Interrupting a turn, the composer and the idle/busy state are #941's; the transcript view and the
  tool calls' rendering are #939's (#940's for diffs). This feature adds to them and does not
  re-implement them.
- Answers are not time-limited, unlike the 60-second confirmation the tool server's destructive
  operations use (feature 034): a turn waits as long as Claude's own terminal would.
- The CLI's own permission rules (allow and deny lists in its settings) still apply first; only a
  request those rules leave open reaches the user.
- Pending requests are not saved to disk: they live only while the process that asked lives.
- **Out of scope**: permission prompts for terminal sessions (their CLI asks in its own screen);
  editing or listing the saved permission rules; a per-project default permission mode; slash
  commands and model switching (#943); usage figures (#944); rendering diffs inside a request
  beyond what #940 gives the transcript view.
