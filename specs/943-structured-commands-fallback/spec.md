# Feature Specification: Structured Session Commands — slash commands, model switching and the terminal fallback

**Feature Branch**: `claude/project-thread-k392yq`

**Created**: 2026-10-22

**Status**: Draft

**Input**: GitHub issue #943 "Structured session: slash commands, model switching and PTY
fallback". "Part of the structured-session (stream-json) UI epic. Keeps the PTY/TUI session mode as
the fallback throughout. Handle slash commands that work in stream-json mode (skills, /compact,
/clear, custom commands) with a command palette, add model switching, and offer 'open in terminal'
that restarts the same session id in PTY mode for TUI-only commands (/config, /mcp, /agents,
/login). Acceptance: Autopilot/speckit skills run unchanged in structured mode. Documented list of
commands that need the PTY fallback. Switching structured <-> PTY keeps the conversation (resume)."
labels: enhancement, flow:feature, in-progress

## In short (for non-technical readers)

A structured Claude Code session (#938) shows Claude's work as a readable conversation (#939, #940)
and takes messages from a message box (#941). Claude Code also understands **slash commands**: a
message starting with `/` that runs a skill (`/speckit-autopilot #943`), a command the user or
project wrote, or one of Claude Code's own commands such as `/compact` (shorten the conversation to
free room) or `/clear` (start over). This feature makes them comfortable to use in the message box:
typing `/` opens a list of the commands this session knows, filtered as the user types, and picking
one fills it in. The user can also pick which Claude model answers, from the session's view, without
restarting. A few of Claude Code's commands only work in its own text screen (`/config`, `/mcp`,
`/agents`, `/login` and others); the app knows which, says so in the list, and offers **Open in
terminal**: the same session restarts in the terminal way, continuing the same conversation, and
the command runs there. **Open in structured view** brings the session back, still in the same
conversation. The user guide lists every command that needs the terminal.

## Context: where this sits in epic #945

| Issue | Delivers | State |
|---|---|---|
| #938 | The structured session kind, event model, daemon side, wire, resume | merged (`specs/938-structured-session-mode`) |
| #939 | Read-only transcript view | merged (`specs/939-structured-transcript-view`) |
| #940 | Markdown, highlighted code, Edit/Write diffs in that view | `specs/940-transcript-rich-rendering` |
| #941 | The composer: sending, multi-line input, history, attachments, interrupt, the queue, idle/busy | design merged, implementation in progress |
| #942 | Permission prompts, permission mode, plan approval, questions | in progress |
| **#943 (this)** | Slash commands and their palette, model switching, switching a session between the structured and the terminal kind | |
| #944 | Usage, context meter, sub-agents, background tasks | `specs/944-session-usage-subagents` |

**Boundaries.** #941 owns the composer and how a message is sent, queued and recalled; this feature
adds the command palette inside it, through the seam #941 leaves for it (#941, Constraints), and
decides what a message that starts with `/` does. #942 owns permission requests, questions and the
permission mode; a skill that asks for permission or asks a question relies on it. #944 owns the
context meter and sub-agent display; it already follows a model change (#944, Edge Cases). #938's
rule that a session's kind never changes (#938 FR-002) is the one this feature replaces.

**Dependencies.** The daemon side (switching a session's kind, changing the model of a running
session, the command list a session reports) builds only on #938. The palette needs #941's composer
on main. Acceptance item 1 ("autopilot/speckit skills run unchanged") also needs #942 on main,
since those skills ask questions and request permissions; the plan orders milestones so that the
daemon side lands first.

## Terms

- **Slash command** (command): a user message whose text starts with `/` followed by a command
  name, optionally followed by arguments (`/speckit-plan`, `/compact keep the API notes`). The CLI
  expands or runs it.
- **Command kinds**: a **skill** (a packaged instruction set, from a plugin, the user or the
  project); a **custom command** (a prompt file the user or the project wrote); a **built-in
  command** (one of Claude Code's own, such as `/compact`, `/clear`, `/context`).
- **Session command list**: the commands a running structured session's CLI reports it knows, as
  it reports them when it starts (the session's init event), with the CLI's own marking of which
  ones work only in its terminal screen.
- **Terminal-only command**: a command that does nothing useful, or is refused, in structured mode
  because it needs Claude Code's own text screen (an interactive settings screen, a sign-in flow).
  The **terminal-only list** is the application's documented list of them, joined with the ones
  the CLI marks terminal-only itself.
- **App-handled command**: a command the application acts on itself instead of sending it to the
  CLI: `/model` (this feature's model picker) and every terminal-only command (this feature's
  fallback).
- **Command palette** (palette): the list the composer shows while its draft is a command being
  typed: matching commands, their kind and description, and the terminal-only marking.
- **Session model**: the model that answers a structured session's turns. **Model choice**: the
  model the user picked for the session, or **Default** (the CLI's own choice, as configured for
  that location).
- **Switching kind**: ending a session's CLI process and starting it again in the other kind,
  terminal or structured, resuming the same conversation. **Open in terminal** switches a
  structured session to terminal; **Open in structured view** switches a Claude Code terminal
  session to structured.
- **Structured session**, **terminal session**, **event**, **event history**, **turn**, **Save
  terminal history**: as feature 938 defines them. **Composer**, **draft**, **queue**, **busy**,
  **idle**: as feature 941 defines them. **Permission mode**: as feature 942 defines it.

## Clarifications

### Session 2026-10-22

- Q: When switched back to structured, are the turns the user had in the terminal period shown in
  the transcript (rebuilt from Claude Code's stored conversation) or only marked? → A: Only marked.
  The transcript is built from the session's event history alone; the terminal-period marker says
  the conversation continued in the terminal and that those turns are not shown here (they stay in
  the terminal's scrollback, saved under **Save terminal history**). No turn count, which would need
  reading Claude Code's store. _(agent-resolved: specs/938-structured-session-mode/spec.md#User
  Story 7 — a history not saved "starts at the resume", never rebuilt from the CLI's store;
  specs/582-attach-provider-worktrees-sessions/spec.md#Out of Scope — "Importing a provider's
  conversation history into Micold's own terminal history")_

- Q: After `/clear` in a structured session, which conversation does a later restart, resume or
  switch of kind continue? → A: The cleared one. A structured session follows the CLI: it keeps
  the conversation handle reported by its latest init and resumes that on every later launch in
  either kind; the session's identity does not change. Terminal sessions keep feature 582 FR-017's
  trade-off (a `/clear` typed in the terminal is not seen). Added FR-005a. _(decided by user —
  provisional A, pending jaro; revert to B (keep 582 FR-017 everywhere) or C (app-handled `/clear`)
  if he picks otherwise)_
- Q: When switched to the terminal after a structured `/clear`, which handle does the terminal
  launch resume, and what after a `/clear` typed in the terminal? → A: The recorded handle
  (FR-005a, FR-020); a terminal `/clear` is not seen, so switching back resumes the last handle the
  structured session recorded, and discovery treats the terminal's successor transcript as a
  continuation, not a new session. _(agent-resolved:
  specs/582-attach-provider-worktrees-sessions/spec.md#FR-017, bugs/BUG-904.md#Decisions)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Run skills and commands from the composer (Priority: P1)

A user types `/speckit-autopilot #943` into a structured session's composer and sends it. The CLI
runs the skill exactly as it would in the terminal: the transcript shows the user's command, then
Claude's work; the skill's questions and permission requests appear as #942 shows them. Custom
commands and built-in commands that work in structured mode (`/compact`, `/clear`, `/context` and
the like) work the same way, and the transcript shows what each did.

**Why this priority**: It is the issue's first acceptance item. The app's own workflows start
sessions with a skill as their first prompt; a structured session that cannot run them is not a
replacement for the terminal one.

**Independent Test**: With a stand-in CLI replaying recorded fixtures of a skill invocation with
arguments, a custom command, a `/compact` and a `/clear`, send each command from the composer:
the CLI receives the text unchanged, and the transcript shows the command as the user typed it
(not the skill's expanded instructions), followed by its outcome. A manual check in a real
structured session runs `/speckit-specify` to its first question.

**Acceptance Scenarios**:

1. **Given** an idle structured session, **When** the user sends `/<skill> <arguments>`, **Then**
   the CLI receives one user message whose text is exactly what the user typed, and the skill runs.
2. **Given** a structured session created with a first prompt that is a command (as the app's own
   workflows do), **When** the session starts, **Then** the command is delivered unchanged as the
   first user message (feature 938 FR-007) and runs.
3. **Given** the CLI expanded a skill or custom command into its instructions, **When** the turn
   is shown, **Then** the transcript shows the user's turn as the command name and its arguments,
   with the expanded instructions collapsed behind it, not as the raw instructions.
4. **Given** the user sends `/compact` (with or without instructions), **When** the CLI compacts
   the conversation, **Then** the session is busy until it is done, and the transcript shows a
   marker that the conversation was compacted at that point, with the summary available behind it;
   earlier turns stay visible above it.
5. **Given** the user sends `/clear`, **When** the CLI starts over, **Then** the transcript shows a
   marker that the conversation was cleared, earlier turns stay visible above it marked as before
   the clear, and later turns continue the cleared conversation; every later restart, restore
   after a daemon restart or switch of kind resumes the cleared conversation (the one the CLI
   reported at the clear), not the one before it (FR-005a).
6. **Given** a built-in command that answers with text and no model turn (such as `/context`),
   **When** it is sent, **Then** its output is shown in the transcript as the command's output,
   readable, without the CLI's internal markup.
7. **Given** a command the CLI does not know, **When** it is sent, **Then** it is delivered as
   typed and the CLI's answer (an unknown-command message, or an ordinary reply) is shown.
8. **Given** a busy session, **When** the user sends a command that is not app-handled, **Then**
   it is queued like any message (feature 941) and runs when its turn comes.

---

### User Story 2 - Find a command with the palette (Priority: P1)

The user types `/` at the start of an empty draft. A palette opens above the composer listing the
session's commands, grouped by kind, each with its description where the CLI gives one. Typing
narrows the list; arrow keys move the selection; Tab or Enter fills the command in. Terminal-only
commands are listed too, marked "opens in terminal".

**Why this priority**: The issue names a command palette; without it the user must remember every
skill's exact name, and cannot tell which commands will not work here.

**Independent Test**: In the client's reducer tests, a session whose recorded init reports skills,
custom and built-in commands and terminal-only commands: typing `/` shows them all grouped; typing
`sp` keeps those matching; selection moves with the arrows; Tab fills `/<name> ` into the draft;
Escape closes the palette without changing the draft; the terminal-only ones carry their marking.

**Acceptance Scenarios**:

1. **Given** a structured session's composer with an empty draft, **When** the user types `/`,
   **Then** the palette opens listing the session command list, grouped as skills, custom commands
   and built-in commands, and the app-handled `/model`.
2. **Given** the palette is open, **When** the user types more characters, **Then** only commands
   whose name matches the typed text (FR-008) are listed, in FR-008's order, the first one
   selected, within 100 ms of the key.
3. **Given** a selected command, **When** the user presses Tab, or Enter, or clicks it, **Then**
   the draft becomes `/<name> ` with the caret after it, the palette closes, and nothing is sent;
   a second Enter sends the draft (feature 941).
4. **Given** the palette is open, **When** the user presses Escape, deletes the `/`, or moves the
   caret out of the command name, **Then** the palette closes and the draft is unchanged; Escape
   here does not interrupt the turn (feature 941 FR-012).
5. **Given** the typed text matches no command, **When** the palette is open, **Then** it says no
   command matches and the draft can still be sent as typed (User Story 1, scenario 7).
6. **Given** a terminal-only command, **When** it is listed, **Then** it is marked as opening in the
   terminal, with that stated in words, not only by an icon or a colour.
7. **Given** a `/` that is not the first character of the draft, or a draft that starts with `/`
   followed by a space, **When** the user types, **Then** no palette opens.
8. **Given** a session that has not reported its command list yet (it has not started since the
   app opened and has no earlier init in its event history), **When** the palette opens, **Then**
   it lists the built-in commands the app knows and says the session's own commands appear once
   the session starts.

---

### User Story 3 - Switch the model of a running session (Priority: P2)

The user wants a faster model for a simple follow-up. The session's view shows the model answering
it; the user opens the model picker (from the view, or by sending `/model`) and picks another. The
next turn is answered by that model, the conversation continues, and the session keeps the choice
across restarts.

**Why this priority**: Named in the issue; today the only way to change model in a structured
session is to stop it and start a new one, losing the conversation.

**Independent Test**: With the stand-in CLI, pick a model for an idle session: the CLI is asked to
use that model for later turns, without the process ending; the next replayed turn reports it; the
view and a second test client show it; restart the session and check the CLI is launched with the
chosen model and resumes the same conversation.

**Acceptance Scenarios**:

1. **Given** a structured session, **When** its view is shown, **Then** it shows the session model
   as the CLI last reported it, and whether it is the user's choice or Default.
2. **Given** the model picker, **When** it opens, **Then** it lists Default and the models the CLI
   reports as available for the session (or, when it reports none, the model names Claude Code
   accepts as shortcuts), marks the current one, and accepts a typed model name.
3. **Given** an idle running session, **When** the user picks a model, **Then** the CLI uses it
   from the next turn on, the session's process keeps running, the conversation is kept, and every
   window viewing the session shows the new model.
4. **Given** a busy session, **When** the user picks a model, **Then** the choice is accepted, the
   turn in progress is not interrupted, and the new model answers from the next turn on.
5. **Given** the user sends `/model` with no arguments, **When** it is sent, **Then** nothing
   reaches the CLI and the model picker opens; **When** they send `/model <name>`, **Then** it is
   treated as picking `<name>`.
6. **Given** a model change, **When** the transcript is shown, **Then** a marker at that point says
   which model answers from there on.
7. **Given** a model the CLI refuses (unknown, or not available to the account), **When** it is
   picked, **Then** the view shows the CLI's reason, the session model is unchanged, and the
   session keeps running.
8. **Given** a session with a model choice, **When** it is restarted, restored after a daemon
   restart, or switched to the terminal and back, **Then** it runs with the same choice.

---

### User Story 4 - Fall back to the terminal for a terminal-only command (Priority: P2)

The user needs `/mcp` to sign in to a tool server. They pick it in the palette (or type and send
it); the app explains it needs the terminal and offers **Open in terminal**. On confirming, the
session restarts as a terminal session in the same conversation, and `/mcp` runs there. When done,
the user activates **Open in structured view** and is back in the structured view of the same
conversation.

**Why this priority**: It is the issue's third acceptance item and what keeps the terminal mode a
true fallback; without it, a user hitting a terminal-only command must abandon the session.

**Independent Test**: With the stand-in CLI, switch a structured session to terminal and back:
each launch is in the other kind, resumes the same conversation, keeps the session's identity, and
every test client sees the kind change; when switched by a terminal-only command, that command is
the terminal's first input.

**Acceptance Scenarios**:

1. **Given** an idle structured session, **When** the user sends or picks a terminal-only command,
   **Then** nothing is sent to the CLI and the view offers **Open in terminal** for that command,
   saying why; dismissing the offer leaves the session and the draft as they were.
2. **Given** the user confirms **Open in terminal**, **When** the switch completes, **Then** the
   session's CLI process ended, the session is a terminal session resuming the same conversation,
   its row and every window show it as terminal, and the command was entered into the terminal
   once the CLI was ready for input, as a terminal session's first prompt is today.
3. **Given** a structured session, **When** the user chooses **Open in terminal** from the session's
   view or its menu without a command, **Then** it switches the same way and the terminal gets no
   input.
4. **Given** a Claude Code terminal session (switched from structured or created as terminal),
   **When** the user chooses **Open in structured view**, **Then** the terminal CLI process ends,
   the session restarts as a structured session resuming the same conversation, and every window
   shows the structured view.
5. **Given** a switch while a turn is in progress, a permission request is pending, or (terminal)
   the CLI reports it is working, **When** the user asks to switch, **Then** the app asks for
   confirmation, saying the work in progress will be ended; nothing changes unless they confirm.
6. **Given** a session that is not running, **When** the user switches its kind, **Then** the kind
   changes and the session starts in the new kind, resuming its conversation if it has one.
7. **Given** a session of another AI CLI (Copilot, Pi, Codex, OpenCode), **When** its menu or view
   is shown, **Then** no switching control is offered.
8. **Given** a structured turn whose result the CLI reports as an authentication failure (not
   signed in, or the sign-in expired), **When** it is shown, **Then** the transcript shows the CLI's
   error and the view offers **Open in terminal** with `/login`; confirming switches as in scenario
   2.

---

### User Story 5 - Switching keeps the conversation and the session's state (Priority: P2)

Whatever the user did in one kind is still there in the other: the conversation (Claude remembers
it), the transcript of the structured periods, the queue, the draft, the model choice and the
permission mode.

**Why this priority**: The issue's acceptance requires that switching keeps the conversation; a
switch that loses state would make users avoid the fallback.

**Independent Test**: With the stand-in CLI and saving on: send turns in structured mode, queue a
message on hold, pick a model, switch to terminal, switch back, then restart the daemon; check the
CLI resumed the same conversation at every launch, the transcript shows the earlier turns then a
marker for the terminal period then the new turns with no gap or duplicate, and the queue, the
model choice and the permission mode are as before.

**Acceptance Scenarios**:

1. **Given** a structured session with turns, **When** it is switched to terminal, **Then**
   Claude's terminal screen continues the same conversation (the CLI resumed it, not a new one).
2. **Given** a session switched back to structured, **When** its view is shown, **Then** the
   transcript holds the event history from before the switch, then a marker saying the
   conversation continued in the terminal, then the resumed run's events, with no gap or duplicate
   at either seam; the turns of the terminal period are not shown in the transcript (it is built
   only from the event history), and the marker says so and that the terminal's own scrollback
   holds them.
3. **Given** a session with a queue (feature 941), **When** it is switched to terminal, **Then** the
   queue is kept on hold and nothing in it is delivered to the terminal; the terminal view says
   how many queued messages wait for the structured view; **When** it is switched back, **Then**
   the queue is there, on hold, until **Send queued**.
4. **Given** a draft in a window, **When** the session is switched away and back, **Then** the
   draft is still in that window's composer.
5. **Given** a session with a model choice and a permission mode (feature 942), **When** it runs as
   a terminal session, **Then** it runs with the model choice and with no permission-mode override
   (feature 942 FR-018); **When** it is switched back, **Then** it runs with both again.
6. **Given** a switched session, **When** the application or the daemon restarts, **Then** the
   session is restored in the kind it last had, resuming the same conversation.
7. **Given** the conversation can no longer be resumed (Claude Code's stored conversation is gone),
   **When** a switch starts the session, **Then** the failure is reported as a failed resume is
   today (feature 938, User Story 3 scenario 4), and the session keeps the new kind so the user
   can switch back.

---

### Edge Cases

- **Empty input**: `/` alone keeps the palette open listing every command; Enter with no command
  selected (no command listed) sends the draft as text. `/` followed by a space is not a command:
  the palette closes and the draft is sent as text by feature 941's rules.
- **Command typed while the session is not running**: the palette still opens with the last known
  command list; sending follows feature 941 (not sent). A terminal-only command still offers
  **Open in terminal**, which starts the session in the terminal kind.
- **Command list changes**: a session that reports a different command list on a later start (a
  plugin installed, a skill added) shows the new list from that start on.
- **Name clashes**: a skill and a custom command with the same name are both listed, each with its
  kind; the CLI decides which runs, as in the terminal.
- **Commands with arguments**: the palette never adds or quotes arguments; text after the name is
  sent as typed, including quotes, line breaks and non-ASCII characters.
- **A terminal-only command inside a longer message** (not at the start): not a command; sent as
  text.
- **A command in the queue**: a command queued while busy runs when its turn comes; a terminal-only
  command or `/model` is never queued, it is acted on at once (offer, or picker).
- **The CLI marks a command terminal-only that is not in the documented list**, or the reverse: the
  command is treated as terminal-only if either says so; the documented list is updated in the next
  change that notices.
- **Model picked while a permission request is pending**: the request stays pending; the new model
  answers from the next turn.
- **Two windows pick different models at once** (Principle II): the last choice to reach the daemon
  wins and every window shows it; no window shows a model the session does not use.
- **Two windows switch kind at once**, or one switches while another sends a message: the session
  switches once; a message that reached the daemon before the switch was delivered or queued, one
  that arrives during the switch is kept in the queue on hold; nothing is lost or delivered twice.
- **A switch while another window is viewing**: every window follows the session to its new kind;
  a window with a draft keeps it.
- **The CLI exits or crashes during a switch**: the session's lifecycle and crash-loop protection
  behave as for any restart (feature 938 FR-009); it is not switched back on its own.
- **Several sessions** (Principle II): switching, model choice and command lists are per session;
  switching or re-modelling one never affects another, in the same worktree or not.
- **Sandboxed sessions** (CLI in a container): switching kind and model work the same; the switch
  keeps the session in its sandbox.
- **Model changed inside the terminal** (the user ran `/model` in the terminal screen): the app
  cannot see it; on switching back, the session's model choice is used, and the guide says so.
- **Login lost**: a structured turn whose result the CLI reports as an authentication failure shows
  the CLI's error and offers **Open in terminal** with `/login` (User Story 4, scenario 8).
- **Platforms** (Principle VI): the palette keys, the picker and switching behave the same on Linux,
  macOS and Windows; ending the CLI process for a switch ends it on every platform, and a terminal
  launched by a switch uses the platform's terminal as any terminal session does.
- **Light and dark theme**: the palette, the terminal-only marking, the model shown and the markers
  are readable in both themes, never conveyed by colour alone.

## Requirements *(mandatory)*

### Functional Requirements

**Slash commands**

- **FR-001**: A draft that starts with `/` followed by a command name MUST be sent to the CLI
  unchanged as one user message, by feature 941's rules (sent when idle, queued when busy), unless
  the command is app-handled (FR-010, FR-018).
- **FR-002**: Skills and custom commands, with or without arguments, MUST run in a structured
  session as they run in a terminal session of the same location; a session's first prompt that is
  a command MUST be delivered and run the same way.
- **FR-003**: The transcript MUST show a user turn that invoked a skill or custom command as the
  command name and its arguments, with the expanded instructions available collapsed behind it.
- **FR-004**: A compaction (asked for with `/compact`, or done by the CLI on its own) MUST be shown
  in the transcript as a marker at the point it happened, with the summary available behind it;
  turns before it MUST stay visible.
- **FR-005**: A `/clear` MUST be shown in the transcript as a marker at the point it happened, with
  the turns before it still visible and marked as belonging to the conversation before the clear.
- **FR-005a**: A structured session MUST keep, with the session, the conversation handle its CLI
  last reported (the session identity of its latest init, which changes on `/clear`) and MUST
  resume that conversation on every later launch in either kind (restart, restore after a daemon
  restart, switch of kind); the session's own identity MUST NOT change. A `/clear` run inside the
  terminal screen is not seen (as for terminal sessions today, feature 582 FR-017): a later launch
  resumes the last conversation handle the structured session recorded.
- **FR-006**: The output of a built-in command that answers without a model turn MUST be shown in
  the transcript as that command's output, without the CLI's internal markup.

**Command palette**

- **FR-007**: Typing `/` as the first character of an empty draft MUST open the palette; it MUST
  list the session command list grouped by kind (skills, custom commands, built-in commands), plus
  `/model`, each with the description the CLI reports for it when it reports one.
- **FR-008**: The palette MUST narrow, within 100 ms of each key, to the commands whose name
  contains the typed text, ignoring case; it MUST order them: the name equal to the typed text,
  then names starting with it, then names in which a word (a part after `-`, `_` or `:`) starts
  with it, then the rest, alphabetically within each group, with the first one selected.
- **FR-008a**: In the palette, arrow keys MUST move the selection; Tab, Enter and a click MUST fill
  `/<name> ` into the draft without sending; Escape MUST close it without changing the draft or
  interrupting the turn.
- **FR-009**: The session command list MUST be the one the session's CLI reported at its latest
  start whose events the session still holds, so that it is offered before the session starts
  again; with none known (the session never started, or the events of its latest start are no
  longer held), the palette MUST list the built-in commands the application knows and say so.
- **FR-010**: Terminal-only commands MUST be listed in the palette marked, in words, as opening in
  the terminal; picking or sending one MUST NOT send it to the CLI and MUST offer **Open in
  terminal** for it (User Story 4).

**Terminal-only list**

- **FR-011**: The application MUST keep a documented terminal-only list, at least `/config`,
  `/mcp`, `/agents`, `/login` and `/logout`, plus every command the CLI reports as terminal-only;
  a command is terminal-only if either source says so.
- **FR-012**: The user guide MUST list every command of the documented terminal-only list with why
  it needs the terminal, and the Claude Code version the list was checked against; the list in the
  guide and the application's list MUST be the same.

**Model switching**

- **FR-013**: A structured session's view MUST show the session model as the CLI last reported it,
  and whether it is the user's choice or Default.
- **FR-014**: A user MUST be able to change a running structured session's model at any time from
  its view; the picker MUST offer Default, the models the CLI reports as available (or, when it
  reports none, the model shortcut names Claude Code accepts) and a typed model name.
- **FR-015**: A model change MUST take effect from the next turn without ending the CLI process,
  MUST keep the conversation, MUST NOT interrupt a turn in progress, and MUST be shown in every
  window viewing the session within 1 second of the CLI accepting it, with a marker in the
  transcript at that point.
- **FR-016**: A model the CLI refuses MUST leave the session model unchanged and show the CLI's
  reason; the session MUST keep running.
- **FR-017**: The model choice MUST be persisted with the session and used on every later launch
  of it, in either kind (restart, restore after a daemon restart, switch of kind).
- **FR-018**: Sending `/model` MUST open the picker and send nothing to the CLI; `/model <name>`
  MUST be treated as picking `<name>`.

**Switching kind**

- **FR-019**: A Claude Code session's kind MUST be switchable, structured to terminal and terminal
  to structured, from its view and its menu, replacing feature 938 FR-002; sessions of other AI
  CLIs MUST NOT offer it, and a request to switch one MUST be refused with a reason.
- **FR-020**: A switch MUST end the session's CLI process and start it in the other kind resuming
  the same conversation (its recorded conversation handle, FR-005a; or, for a session whose
  conversation has not started, starting it as a new one), keeping the session's identity, worktree, sandbox and terminals; the new kind MUST be
  persisted and shown in every window and wherever sessions are listed.
- **FR-021**: A switch asked for while a turn is in progress, a permission request is pending, or
  a terminal session's CLI reports it is working MUST be confirmed by the user first, saying that
  the work in progress will end.
- **FR-022**: A switch to terminal for a terminal-only command MUST enter that command into the
  terminal once its CLI is ready for input, as a terminal session's first prompt is delivered.
- **FR-022a**: A turn whose result the CLI reports as an authentication failure MUST offer **Open
  in terminal** with `/login`, beside the CLI's error.
- **FR-023**: Switching back to structured MUST show the event history from before the switch,
  then a marker for the terminal period, then the resumed run's events, with no gap or duplicate;
  the terminal period's turns MUST NOT be rebuilt into the transcript from Claude Code's stored
  conversation, and the marker MUST say they are not shown;
  the event history MUST be kept, and saved under **Save terminal history** (feature 938 FR-021a),
  while the session is a terminal session.
- **FR-024**: A switch MUST keep the session's queue (feature 941), on hold and undelivered while
  the session is a terminal session, each window's draft, the model choice and the permission mode
  (feature 942); a terminal session MUST run with no permission-mode override.
- **FR-025**: Concurrent switch requests for one session MUST switch it once; a message that
  arrives during a switch MUST be queued on hold, never lost or delivered twice.

**Testing and documentation**

- **FR-026**: Command delivery, the command list, model switching and switching kind MUST be
  covered by tests that run the daemon against the stand-in CLI replaying fixtures recorded from a
  real CLI, covering at least: a skill invocation with arguments, a custom command, a `/compact`, a
  `/clear`, a built-in command's text output, a model change and a refused model, and a resumed
  init after a switch.
- **FR-027**: The user guide's structured-sessions section MUST describe the palette, the model
  picker, **Open in terminal** and **Open in structured view**, what a switch keeps, and the
  terminal-only list (FR-012), in the same change that ships each.

### Constraints

- **Terminal mode stays the fallback**: a terminal session created as such behaves exactly as
  today unless the user switches it; switching is the only new thing it offers.
- **Builds on #938, #941, #942**: the session kind, event history, resume and first-prompt delivery
  (#938), the composer, queue and idle/busy state (#941), and permission requests and modes (#942)
  are reused, not re-specified.
- **Shared UI components (Principle VIII)**: the palette, the model picker and the switch offer are
  built from, or added to, the shared component library.

### Out of Scope

- Screens inside the app that replace terminal-only commands (settings, tool-server management,
  agent editing, sign-in); the terminal fallback is how those are reached.
- Choosing a model when creating a session; a new session starts on Default and can be changed at
  once.
- Changing reasoning effort, output style or other per-session settings beyond the model; such
  commands are sent to the CLI like any command, or need the terminal.
- Detecting a model change made inside the terminal screen.
- Writing, editing or installing skills and custom commands.
- Switching a session to or from any AI CLI other than Claude Code.

### Key Entities

- **Session command list**: per structured session: each command's name, kind, description when
  reported, and whether the CLI marks it terminal-only; as of the session's latest start.
- **Terminal-only list**: the application's documented commands that need the terminal, with why,
  and the Claude Code version they were checked against.
- **Conversation handle**: per Claude Code session: the conversation a launch resumes; the session's
  identity until a structured session's CLI reports another (after `/clear`); persisted.
- **Model choice**: per session: Default or a model name; persisted with the session.
- **Session model**: the model the CLI last reported for the session.
- **Session kind** (from feature 938): now changeable by a switch; persisted.
- **Terminal period marker**: a point in a structured session's event history where the session
  ran in the terminal kind, until it came back.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 100% of the command fixtures (skill with arguments, custom command, `/compact`,
  `/clear`, built-in output, unknown command), the command runs once with exactly the text the user
  sent, and the transcript shows the command and its outcome.
- **SC-002**: For every command in the recorded command lists, typing a prefix of its name that no
  other command's name starts with lists it first and selected, so that Tab then Enter runs it; for
  a skill such as `/speckit-plan` that is at most its name's length plus two key presses after the
  `/`. Typing a command's full name lists it first and selected even when other names start with
  it.
- **SC-003**: The palette updates within 100 ms of each key with a session command list of 200
  commands.
- **SC-004**: A model change takes effect on the next turn, without the session restarting, in 100%
  of test runs; every window viewing the session shows the new model within 1 second of the CLI
  accepting it.
- **SC-005**: Across switch test runs (structured → terminal → structured, with restarts and a
  daemon restart in between), every launch resumes the same conversation in 100% of runs, and 0
  events, queued messages or model choices are lost or duplicated.
- **SC-006**: Every command of the documented terminal-only list, sent from the composer, leads to
  the **Open in terminal** offer and never reaches the CLI in structured mode, in 100% of test runs.
- **SC-007**: In a real structured session, `/speckit-specify` and `/speckit-autopilot` run from the
  composer to their first question or permission request with no step that needs the terminal
  (manual check in the quickstart).
- **SC-008**: Every existing terminal-session and structured-session test passes unchanged, apart
  from tests of feature 938 FR-002, which this feature replaces.

## Assumptions

- The CLI reports, when a structured session starts, the commands and skills it knows, the model it
  uses and the commands it marks as terminal-only (seen in the recorded fixtures of feature 938);
  descriptions and the available models are reported where the CLI's structured mode offers them,
  and the plan settles from recordings what it does.
- The CLI's structured input accepts a request to change the model of a running session for later
  turns, as it does a permission-mode change (feature 942); a model can also be named at launch.
- Slash commands typed as a user message's text are expanded or run by the CLI in structured mode,
  as the first-prompt delivery of the app's own workflows already relies on.
- The conversation identity the app assigns to a Claude Code session (feature 938 FR-006) is the
  same in both kinds, so a switch resumes it in either.
- The documented terminal-only list is checked against a real Claude Code release when the feature
  ships; later releases may add or drop commands, and the CLI's own marking (FR-011) covers the gap
  until the list is updated.
- Feature 942 is on main before the acceptance check of User Story 1 is made with a real CLI.
