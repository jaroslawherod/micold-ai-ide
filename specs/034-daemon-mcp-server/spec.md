# Feature Specification: The session service exposes an MCP server to the AI sessions it runs

**Feature Branch**: `feat/daemon-should-expose-mcp-server-for-agent`

**Created**: 2026-09-29

**Status**: Draft

**Input**: User description: "the daemon server should expose mcp server that will be automaticly
binded into AI sessions. Should allow to manage the sessions and worktries."

## Background: what exists today

Read on `main` at the commit this spec was branched from:

- The **session service** (the `micold-daemon` process, "the service" below) owns the catalog of
  projects, their worktrees and their sessions, and spawns every AI CLI session (Claude Code,
  Copilot, Pi — feature 026/029) in a PTY. Clients (application windows) drive it over a private,
  versioned protocol: create/rename/delete a worktree (`WorktreeCreate`, `WorktreeRename`,
  `WorktreeDelete` with `stop_sessions`/`delete_branch`), list branches, create/start/stop/kill/
  interrupt/delete a session, and receive a catalog snapshot that carries every session's lifecycle
  (`Idle`, `Starting`, `Running`, `Restarting`, `Failed`, `InterruptedResumable`) and activity
  signal (`Unknown`, `Working`, `AwaitingInput`, `Ended`).
- An AI agent running **inside** one of those sessions has no access to any of this. It can run
  `git worktree add` in its own shell, but the application does not know about the result until a
  manual refresh, the worktree is not recorded as app-created, and no session is started in it.
  It cannot see its sibling sessions at all.
- The service already runs one loopback-only HTTP endpoint with a per-session token and path: the
  **hook receiver** (feature 010, `contracts/hooks.md`) that Claude Code reports lifecycle hooks to.
  The service writes a per-session settings file it passes to the CLI on its command line, so **the
  user's own configuration is never modified**. That receiver deliberately exposes no capability
  beyond reporting one session's activity.
- The service can run on the host or inside a sandbox container (feature 027), where sessions run
  in the container alongside it.

This feature gives every AI session a **tool server** in the Model Context Protocol (MCP) sense,
hosted by the service and wired into the session automatically, through which the agent can see
and manage the project's sessions and worktrees the way the user does from the sidebar.

## Terms

- **Calling session**: the AI session whose agent is invoking a tool. Every request the tool server
  receives is attributable to exactly one calling session.
- **Calling project**: the project the calling session belongs to.
- **Binding**: the per-session wiring that makes the tool server available to the AI CLI the
  session runs, without the user configuring anything.
- **Operation**: one tool the server offers (see *Operations* below).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The agent sees the project's sessions and worktrees without any setup (Priority: P1)

A developer opens a project and starts a Claude Code session. Without editing any configuration
file, the agent in that session can list the project's worktrees (name, branch, status, whether
the app created it) and sessions (label, AI CLI, lifecycle, activity, which worktree hosts it),
and can tell which of those sessions is itself.

**Why this priority**: Everything else builds on the binding and on the agent being able to read
the project's state. On its own it already lets an agent answer "what else is going on in this
repository?" and avoid clobbering a sibling session's worktree.

**Independent Test**: Start a session of a supported AI CLI in a fresh project with two worktrees
and one other session; ask the agent to list the tools it has from the service, then to list
worktrees and sessions. The answers match the sidebar exactly, and the calling session is marked
as itself. No file under the user's home or the project was edited to make this happen.

**Acceptance Scenarios**:

1. **Given** a project with worktrees `a` and `b` and sessions S1 (in `a`) and S2 (Default),
   **When** a new session S3 starts in `b` with a supported AI CLI, **Then** the agent in S3 has
   the service's tools available on its first turn, with no user action.
2. **Given** S3 above, **When** its agent invokes `list_worktrees`, **Then** it receives the
   project root ("Default") plus `a` and `b`, each with branch, status and app-created flag, the
   same set the sidebar shows.
3. **Given** S3 above, **When** its agent invokes `list_sessions`, **Then** it receives S1, S2 and
   S3 with label, AI CLI, lifecycle, activity and hosting worktree, and S3 is marked as the calling
   session.
4. **Given** the user's own AI CLI configuration files, **When** any session is started with the
   binding, **Then** those files are byte-identical before and after.
5. **Given** a session whose AI CLI cannot accept a tool server, **When** it starts, **Then** it
   starts exactly as it does today, and the absence of a binding and its reason are recorded in the
   service log and documented per CLI in the user guide (FR-005).
6. **Given** the FR-004 toggle turned off, **When** a new session starts, **Then** it has no
   binding, the service log records that and why, and sessions already running keep theirs.

---

### User Story 2 - The agent creates a worktree and starts a session in it (Priority: P1)

An agent that has planned parallel work asks the service to create a new worktree on a new branch
and start a session there, optionally with a first prompt. The worktree appears in the sidebar as
if the user had created it, and the new session appears under it and starts working.

**Why this priority**: This is the "manage worktrees and sessions" half of the request, and the
flow the user cannot get today without switching to the IDE and clicking through the create-
worktree dialog.

**Independent Test**: In a session that runs in a worktree, ask the agent to create worktree `feat-x` on a new branch
`feat-x` and start a Claude Code session in it with the prompt "print the branch name". Observe the
sidebar: a worktree row `feat-x` and a session under it, which receives the prompt.

**Acceptance Scenarios**:

1. **Given** no branch `feat-x` exists, **When** the agent invokes `create_worktree` with branch
   `feat-x`, **Then** the worktree is created exactly as the create-worktree dialog would create
   it (same location, same naming rules, recorded as app-created), every connected window shows it
   without a manual refresh, and the operation returns its name, branch and path.
2. **Given** branch `feat-x` already exists or is checked out elsewhere, **When** the agent
   invokes `create_worktree` for it, **Then** the operation fails with a reason that names the
   conflict (the same classification the dialog's pre-flight uses), and nothing on disk changes.
3. **Given** worktree `feat-x`, **When** the agent invokes `create_session` for it with an AI CLI
   and an initial prompt, **Then** a session is created and started there, appears in every
   window, and receives the prompt as its first input once its CLI is ready.
4. **Given** the agent omits the AI CLI, **When** it invokes `create_session`, **Then** the
   session uses the user's default AI CLI from Settings.
5. **Given** a requested AI CLI that is not installed where sessions run, **When** the agent
   invokes `create_session`, **Then** the operation fails naming the missing CLI and no session
   record is left behind.

---

### User Story 3 - The agent controls session lifecycle (Priority: P2)

The agent can start, stop and interrupt sessions of its project, and rename and delete worktrees
and sessions, with the same effects and safeguards the sidebar applies.

**Why this priority**: Completes the management surface. Less urgent than creating work, and
carries the destructive operations whose safeguards need care.

**Independent Test**: From a session that runs in a worktree, with two idle sessions and another
worktree whose branch is merged, ask the agent
to start one session, stop it again, rename the worktree, then delete it. Each step shows in the
sidebar; the stop and the delete follow FR-014.

**Acceptance Scenarios**:

1. **Given** an `Idle`, `Failed` or `InterruptedResumable` session, **When** the agent invokes
   `start_session` on it, **Then** it moves to `Starting` then `Running`, as if started from the
   sidebar.
2. **Given** a `Running` session, **When** the agent invokes `stop_session`, **Then** it stops
   gracefully and becomes `Idle`; `interrupt_session` instead delivers an interrupt keystroke and
   leaves it running.
3. **Given** a worktree with live sessions, **When** the agent invokes `delete_worktree` without
   asking to stop them, **Then** the operation is refused naming the live sessions, and nothing
   changes.
4. **Given** any operation FR-014 classifies as destructive, **When** the agent invokes it,
   **Then** the policy of FR-014 applies before anything is changed.
5. **Given** the calling session, **When** its agent invokes `stop_session`, `delete_session`, or
   `delete_worktree` on its own session or hosting worktree, **Then** the operation is refused
   (FR-015).
6. **Given** a Default session (running in the project root), **When** its agent invokes
   `create_worktree`, `rename_worktree` or `delete_worktree`, **Then** the operation is refused by
   policy (FR-015a, Principle III).

---

### User Story 4 - The agent coordinates with a sibling session (Priority: P3)

An agent that delegated work to another session can check on it — read what that session's
terminal recently showed, see whether it is working or waiting for input — and send it a follow-up
prompt.

**Why this priority**: Turns separate sessions into an orchestratable team, but it crosses the
session isolation boundary (Principle II) and so is gated on a product decision and its own
Settings toggle (FR-016).

**Independent Test**: With the FR-016 toggle on, start session S2 with a prompt via
`create_session`; from S1, poll `get_session` until S2 is `AwaitingInput`, read its recent output
with `read_session_output`, and send it a follow-up with `send_session_input`. The follow-up appears in S2's terminal.

**Acceptance Scenarios**:

1. **Given** the FR-016 toggle on and sibling S2 in the same project, **When** S1's agent invokes
   `read_session_output` on it, **Then** it receives the last N lines of S2's primary terminal as plain text (N bounded by
   FR-012).
2. **Given** the FR-016 toggle on and sibling S2 awaiting input, **When** S1's agent invokes `send_session_input` with a
   prompt, **Then** the text is delivered to S2's primary process exactly as if typed, and
   submitted.
3. **Given** a session in another project, **When** the agent targets it with any operation,
   **Then** the operation fails as if the session did not exist (FR-010).
4. **Given** the FR-016 toggle off, **When** S1's agent invokes `read_session_output` or
   `send_session_input` on S2, **Then** the operation is refused by policy and S2 is untouched.

---

### Edge Cases

- **Empty project**: a project with no worktrees and no sessions except the caller lists just the
  Default location and the calling session; an empty list is a successful answer, not an error.
- **Unknown or malformed target**: a session id or worktree name that does not exist, or belongs to
  another project, fails with "not found" and never reveals whether it exists elsewhere.
- **Invalid names**: a branch or worktree name git or the app's naming rules reject fails with the
  same validation message the create-worktree dialog shows, before anything is created.
- **Concurrency (Principle II)**: two sessions (or a session and a user in the sidebar) create the
  same branch at once — exactly one succeeds and the other gets the "already exists" conflict;
  neither leaves a half-created worktree. Two agents deleting the same worktree — one succeeds, the
  other gets "not found". Operations from many sessions at once are each attributed to their own
  calling session.
- **Several windows attached**: every window connected to the service sees an agent-made change as
  it sees a change made from another window — through the ordinary catalog update, with no refresh.
- **No window attached**: the service keeps running sessions with no window open (feature 010).
  Agent operations still work; a window that connects later shows their results. A destructive
  operation that needs the user's confirmation (FR-014) is refused with "needs confirmation, no
  window available" rather than waiting indefinitely.
- **Service restart**: a session found `InterruptedResumable` after a service restart gets a fresh
  binding when it is started again; credentials from before the restart do not work.
- **Stale credential**: a credential of a deleted session, or one issued before a service restart,
  is refused and reveals nothing (FR-006). A local process of the same user that obtains a live
  session's credential acts as that session; this is accepted (see Assumptions).
- **Target changes while a confirmation is pending (FR-014)**: if the target worktree or session is
  deleted, or the calling session is stopped or deleted, before the user answers, the pending
  request fails with "not found" or "cancelled", the confirmation prompt is withdrawn from every
  window, and nothing changes.
- **Hidden assistant-owned worktrees (feature 014)**: `list_worktrees` returns the same set the
  sidebar shows with its reveal control off, so worktrees feature 014 hides are left out unless the
  agent asks to include them, in which case each is flagged as assistant-owned.
- **Session respawn**: a session restarted after a crash (`Restarting`) keeps working with its
  binding without the agent noticing.
- **Sandboxed placement (feature 027)**: when sessions run in the sandbox container, the tool
  server is reachable from inside the container where the CLI runs, and paths it returns are the
  paths the agent sees, not host paths.
- **Unsupported CLI**: a CLI with no tool-server support starts without a binding, and nothing about
  its launch changes.
- **User already configured a server with the same name**: the binding must not collide with, or
  silently replace, a tool server the user configured themselves.
- **Long outputs**: `read_session_output` on a session with a very large scrollback returns at most
  the bounded amount, never the whole buffer.
- **Cross-platform (Principle VI)**: the binding works for every supported CLI on Linux, macOS and
  Windows, including Windows path forms in returned worktree paths, and on every platform no other
  local user account can reach the tool server or read a session's credential.
- **Disabled**: when the user turns the feature off (FR-004), sessions started afterwards get no
  binding; sessions already running keep theirs until restarted.

## Requirements *(mandatory)*

### Functional Requirements

**Binding**

- **FR-001**: The service MUST host one tool server, speaking the Model Context Protocol, that is
  available whenever the service is running, in both host and sandboxed placements.
- **FR-002**: Every session the service starts with an AI CLI that can accept an MCP tool server
  MUST be given a binding to it automatically, at spawn, with no user action. At minimum Claude
  Code MUST be bound; Copilot and Pi MUST be bound if their CLI accepts a tool server supplied at
  launch.
- **FR-003**: The binding MUST NOT create, modify or delete any user or project configuration file
  (for example the CLI's user settings or a project `.mcp.json`); like the hook settings today, it
  lives only in what the service itself owns for that session. The binding MUST NOT displace a tool
  server the user configured under any name.
- **FR-004**: A Settings toggle MUST let the user turn the binding off (default: on). Changing it
  affects sessions started afterwards only.
- **FR-005**: A session started without a binding (unsupported CLI, setting off, or the binding
  could not be prepared) MUST still start; the absence of a binding and its reason MUST be recorded
  in the service log, and which CLIs are bound MUST be stated in the user guide.
- **FR-006**: Each binding MUST carry a credential unique to its session. The tool server MUST
  refuse any request without a valid credential, and MUST attribute every accepted request to the
  one calling session the credential belongs to. A credential MUST stop working when its session is
  deleted or the service restarts.
- **FR-007**: The tool server MUST be reachable only from the machine (or container) where sessions
  run; it MUST NOT accept connections from other hosts. On every platform, no other local user
  account MUST be able to read a session's credential.

**Operations**

- **FR-008**: The tool server MUST offer the operations in the *Operations* table below, with the
  inputs, outputs and classification given there.
- **FR-009**: Every mutating operation MUST have the same effect as the equivalent user action in
  the application: the same validation, the same naming and placement rules, the same catalog
  records (a worktree created by an agent is app-created), and the same safeguards (a worktree with
  live sessions is not deleted unless stopping them was requested).
- **FR-010**: Operations MUST be scoped to the calling project. [NEEDS CLARIFICATION: Should an
  agent be able to see and manage sessions and worktrees of *other* projects in the catalog, or only
  its own project? Default assumed here: own project only.] A target outside the scope MUST be
  reported as "not found".
- **FR-011**: Every change an operation makes MUST reach every connected window through the same
  catalog update a user-made change produces, within the time bound of SC-003.
- **FR-012**: `read_session_output` MUST return plain text (no escape sequences) of at most a
  bounded number of the most recent lines (default 200, maximum 2,000), drawn from the session's
  retained scrollback. A requested count above the maximum is clamped to it; a count below 1 is
  invalid input.
- **FR-012a**: `send_session_input` with empty text MUST be refused as invalid input.
  `start_session` on a session that is already `Starting`, `Running` or `Restarting` MUST succeed
  without changing it and report its current lifecycle; `stop_session` on an `Idle` session
  likewise.
- **FR-013**: Every failed operation MUST return a reason category (not found, invalid input,
  conflict, refused by policy, needs confirmation, service error) and a human-readable message; it
  MUST leave nothing half-done.
- **FR-014**: The destructive operations are exactly: `delete_worktree`, `delete_session`, and
  `stop_session` or `interrupt_session` on a session other than the caller. They MUST follow one
  policy.
  [NEEDS CLARIFICATION: Should destructive operations (a) run without asking, (b) require the user
  to confirm each one in the application window, or (c) be left out of the tool server entirely?
  Default assumed here: (b) — the operation waits for the user's confirmation in a window and fails
  with "needs confirmation" if none is given within 60 seconds or no window is attached.]
- **FR-015**: An agent MUST NOT stop, delete, or delete the hosting worktree of its own calling
  session through the tool server; such a request MUST be refused by policy. `send_session_input`
  and `read_session_output` targeting the calling session itself MUST be refused as invalid input.
- **FR-015a**: When the calling session is a Default session (it runs in the project root),
  `create_worktree`, `rename_worktree` and `delete_worktree` MUST be refused by policy, because
  Principle III forbids a Default session to create, modify or remove any git worktree. Every
  other operation stays available to it.
- **FR-016**: `read_session_output` and `send_session_input` on a session other than the caller
  MUST be governed by a Settings toggle separate from FR-004, "Let agents read and type into other
  sessions". When it is off, both MUST be refused by policy; when it is on, both MUST work within
  the scope of FR-010 without the FR-014 confirmation. [NEEDS CLARIFICATION: Should cross-session
  output reading and input sending be (a) allowed by default (toggle on), (b) allowed but off by
  default, (c) allowed only with a per-send confirmation like FR-014, or (d) left out entirely?
  Default assumed here: (b) — the toggle exists and starts off, since input into another agent can
  make it run any command in that session's worktree.]
- **FR-017**: `create_session` MUST accept an optional initial prompt and deliver it to the new
  session's primary process as its first submitted input when the session's activity first
  reports awaiting input. If that has not happened within 60 seconds of the session starting, or
  the session fails to start, the operation MUST report that the prompt was not delivered, and the
  prompt MUST NOT be delivered later. With a prompt, `create_session` returns only once the prompt
  is delivered or that bound has passed; without one, it returns once the session is created.
- **FR-018**: The service MUST log every mutating operation at its default log level with the
  calling session, the operation and its target, and MUST NOT log prompt or input text (FR-047 of
  feature 010).

**Visibility**

- **FR-019**: The user guide MUST document the tool server: what it offers, which CLIs are bound,
  the Settings toggle, the confirmation policy, and the scope.

### Operations

All operations are scoped by FR-010. "Session ref" is a session identifier as returned by
`list_sessions`; "worktree ref" is a worktree name as returned by `list_worktrees`, or `default`
for the project root.

| Operation | Purpose | Key inputs | Key outputs | Class |
|---|---|---|---|---|
| `whoami` | Identify the calling session and its place | — | calling session ref, project name and path, hosting worktree ref, AI CLI | read-only |
| `list_worktrees` | List the project's worktrees as the sidebar shows them | optional include-hidden flag (feature 014 assistant-owned worktrees) | per worktree: ref, display name, branch, path, status (clean/missing/locked/prunable), app-created flag, assistant-owned flag, session count | read-only |
| `list_branches` | List local and remote-tracking branches, with why each is unavailable for a new worktree | — | per branch: name, local/remote, checked-out-in worktree ref if any | read-only |
| `list_sessions` | List the project's sessions, Regular-terminal sessions included | optional worktree ref filter | per session: ref, label, AI CLI (or "regular terminal"), lifecycle, activity, hosting worktree ref, is-caller flag | read-only |
| `get_session` | One session's current state | session ref | as one `list_sessions` row, plus failure reason if `Failed` | read-only |
| `read_session_output` | Recent terminal text of another session (FR-012, FR-016) | session ref, line count | plain-text lines, whether output was truncated | read-only; gated by FR-016 |
| `create_worktree` | Create a worktree as the create-worktree dialog does | branch name, optional worktree name, mode (new branch / existing local branch / track remote branch) | worktree ref, branch, path | mutating; refused from a Default session (FR-015a) |
| `rename_worktree` | Change a worktree's display name | worktree ref, new display name | updated worktree row | mutating; refused from a Default session (FR-015a) |
| `delete_worktree` | Remove a worktree | worktree ref, stop live sessions (default no), delete branch (default yes) | confirmation of what was removed | destructive (FR-014); refused for the caller's own worktree (FR-015) and from a Default session (FR-015a) |
| `create_session` | Create and start a session in a worktree or Default | worktree ref, optional AI CLI (default: Settings default), optional initial prompt | session ref, lifecycle, whether the prompt was delivered (FR-017) | mutating |
| `start_session` | Start or resume an `Idle`, `Failed` or `InterruptedResumable` session | session ref | lifecycle after the request | mutating |
| `stop_session` | Gracefully stop a session | session ref | lifecycle after the request | destructive for other sessions (FR-014); refused for self (FR-015) |
| `interrupt_session` | Deliver an interrupt keystroke to a session | session ref | acknowledgement | mutating; destructive for other sessions (FR-014) |
| `send_session_input` | Type and submit text into another session's primary process | session ref, text | acknowledgement | mutating; gated by FR-016 |
| `delete_session` | Delete a session record (stopping it first) | session ref | acknowledgement | destructive (FR-014); refused for self (FR-015) |

Operations deliberately **not** offered: adding, removing or renaming projects; changing Settings;
force-killing a session; opening, closing or restarting Regular-terminal shell instances; claiming,
including or excluding worktrees the app did not create. These stay user-only in this feature.

### Key Entities

- **Tool server**: the service-hosted MCP endpoint. One per running service; lives and dies with
  it.
- **Binding**: per session — the credential, the endpoint address as seen from where the session
  runs, and the launch wiring for that session's AI CLI. Created at spawn, revoked on delete or
  service restart. Kept only in service-owned storage that no other local user account can read,
  never in user or project configuration.
- **Operation request**: calling session, operation, inputs, outcome. Logged per FR-018 for
  mutating operations.
- **Pending confirmation** (if FR-014 resolves to (b)): the destructive request awaiting the user,
  with its calling session, target and expiry.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 100% of sessions started with a supported AI CLI and the setting on, the agent can
  invoke `whoami` on its first turn, with zero configuration steps by the user.
- **SC-002**: After starting 20 bound sessions, the user's own CLI configuration files and every
  project's configuration files are byte-identical to before.
- **SC-003**: A worktree or session created, renamed or deleted by an agent appears in, or
  disappears from, every connected window within 2 seconds, with no manual refresh.
- **SC-004**: Read-only operations answer within 1 second for a project with 50 worktrees and 50
  sessions.
- **SC-005**: Of 10 concurrent attempts from 10 sessions to create the same branch, exactly 1
  succeeds and 9 fail with a conflict, leaving exactly one worktree on disk.
- **SC-006**: 100% of requests without a valid credential, or with a credential of a deleted
  session, are refused without revealing any project, session or worktree data.
- **SC-007**: An agent in a worktree session can go from "I want a parallel worktree with a session working on X" to that
  session receiving its first prompt in one request sequence of at most two operations
  (`create_worktree`, `create_session`).
- **SC-008**: The binding behaves identically on Linux, macOS and Windows, in every placement feature
  027 supports on that platform, verified by the same acceptance scenarios on each.
- **SC-009**: A second local user account on the same machine cannot connect to the tool server
  or read any session's credential, on each platform (FR-007).
- **SC-010**: Every failed operation in the acceptance scenarios returns one of the FR-013 reason
  categories, and after every mutating operation the service log holds one entry naming the calling
  session, the operation and its target, with no prompt or input text in it (FR-018).

## Assumptions

- "The daemon server" is the session service (`micold-daemon`); "worktries" means git worktrees.
- "Manage the sessions and worktrees" means the operations the sidebar already offers, not new
  capabilities; anything the user cannot do from the sidebar today is out of scope.
- Claude Code accepts a tool server supplied through launch arguments without touching user
  configuration, as it already does for hook settings. Whether Copilot and Pi can is established in
  planning; if one cannot, FR-005 applies to it.
- The calling agent acts with the user's authority inside its project: it is the user's own agent,
  on the user's machine. The confirmation policy (FR-014) is the guard against mistakes, not
  against a hostile user.
- A local process running as the same user that reads a live session's credential can act as that
  session. The tool server does not try to tell processes apart, exactly as the hook receiver does
  not today; the guard is FR-007's per-user protection, not per-process identity.
- Principle III's rule that a Default session must not create, modify or remove a worktree is read
  as covering agent-requested operations too (FR-015a). Letting a Default session's agent manage
  worktrees would need a constitution amendment, which is outside this feature.
- A Regular-terminal session (feature 010-regular-terminal-mode) runs a shell, not an AI CLI, and
  gets no binding.
- The initial prompt of `create_session` is delivered as typed input, the same way the user would
  type it; no CLI-specific "prompt argument" is assumed.

## Out of Scope

- Exposing the tool server to agents or tools the service did not start (an external MCP client, a
  CLI the user runs in their own terminal).
- Project-level management (add/remove/rename projects), Settings changes, and shell-instance
  management through the tool server.
- Remote access to the tool server from another machine.
- Any change to the client/service protocol other than what carrying agent-made changes and
  confirmation prompts (FR-014) to windows needs.
- An MCP resource or prompt surface; only tools are offered.
