# Feature Specification: One Session per Worktree, Chosen at Start

**Feature Branch**: `feat/776_one-session-per-worktree-chosen-at-start`

**Created**: 2026-10-12

**Status**: Draft

**Input**: User description: "Implement GitHub issue #776: One session per worktree, chosen at start. A worktree should host exactly one session. The session is defined when the worktree is started, and there must be no way to run more than one session in the same worktree. Expected: starting a session in a worktree that already has one is refused (UI and the `create_session` / `start_session` MCP tools), with a clear error naming the existing session; the session's definition (agent/model/prompt) is set at worktree start, not added afterwards; resuming or restarting replaces the worktree's session rather than adding a second one. Why: concurrent sessions in one worktree edit the same files and branch, and race on the build lock and git index."

## Terms

- **Worktree**: a git worktree the application manages, shown in the sidebar under its own name.
- **Session**: one AI CLI conversation (Claude Code, Copilot CLI or Pi) running in a worktree.
- **Session definition**: what makes the session what it is: the AI CLI (agent), its model where
  the CLI takes one, and the first prompt.
- **Worktree start**: the moment a worktree is created (create-worktree dialog or `create_worktree`
  MCP tool) together with the session that will run in it.
- **Own session**: the one session a worktree hosts, whatever its state (starting, running,
  stopped, failed, exited). A worktree has at most one; it has none only in the cases of FR-010
  (created without a definition, attached with nothing to resume, or its session deleted), and
  then takes exactly one, defined once at that moment.

## Clarifications

### Session 2026-10-12

- Q: Does one-session-per-location hold for Default (project root)? → A: Yes, the same rule _(decided by user)_
- Q: Which provider session is catalogued when a worktree has several resumable? → A: A single resume is the user's pick; attach-all catalogues the most recent per worktree and leaves the rest listed as resumable _(decided by user)_
- Q: What happens to legacy worktrees with several sessions? → A: Every record is kept listed and untouched, none is created, starting an extra while another is starting/running is refused naming it, the user may delete extras; SC-001 carves out legacy data _(decided by user)_

- Q: Can an agent resume a provider session into a worktree (FR-008b), and does that need a
  confirmation? → A: No agent path exists: resuming a listed provider session is done in the app
  only (the tool server lists them with `list_resumable_sessions` and attaches worktrees, it never
  resumes), so FR-008b is always a person's action and always asks for confirmation. Resuming into
  an empty worktree needs none _(agent-resolved: docs/user-guide/agent-tools.md#What the assistant can do, `list_resumable_sessions`; specs/582-attach-provider-worktrees-sessions/spec.md#FR-008)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A second session in a worktree is refused (Priority: P1)

A user, or an agent through the tool server, asks for a new session in a worktree that already has
one. The request is refused with an error that names the existing session, so neither a person nor
an agent can have two sessions editing the same files, branch, build lock and git index.

**Why this priority**: This is the whole invariant. Everything else supports it.

**Independent Test**: Create a worktree with a session, then ask for another session in it from the
sidebar and through `create_session`; both are refused, and the worktree still has exactly one
session.

**Acceptance Scenarios**:

1. **Given** a worktree with a session in any state, **When** the user asks for a session in it
   (sidebar new-session action, or a window that still shows the action from before the session
   existed), **Then** no session is created and the user is told which session the worktree already
   has.
2. **Given** a worktree with a session, **When** an agent calls `create_session` for that worktree,
   **Then** the call fails with an error naming the existing session (its id and name) and no
   record is added.
3. **Given** a worktree with a session, **When** two `create_session` calls for it arrive at the
   same moment, **Then** exactly one succeeds when the worktree had none, and none succeeds when it
   had one.
4. **Given** the refusal, **When** the agent reads the error, **Then** it holds the existing
   session's id and name (FR-003).

---

### User Story 2 - The session is defined when the worktree starts (Priority: P1)

A user creating a worktree may pick the AI CLI and the first prompt in the same step; when they
do, the worktree and its session are created together. There is no "add session" affordance on a worktree that
already started; the definition is not changed by adding another session later.

**Why this priority**: It is what makes "exactly one" a property of the model, not only a check.

**Independent Test**: Create a worktree through the dialog and through `create_worktree` with an AI
CLI and a prompt; the worktree has one session with that definition and nothing in either surface
offers to add another.

**Acceptance Scenarios**:

1. **Given** the create-worktree dialog, **When** the user picks an AI CLI and a first prompt and
   confirms, **Then** the worktree is created and its one session starts with that definition.
2. **Given** a `create_worktree` call that names an AI CLI and a prompt, **When** it succeeds,
   **Then** the session runs in the new worktree with that definition, and the reply names it.
3. **Given** a worktree with a session, **When** the user looks at the sidebar, **Then** it offers
   no way to add a second session to that worktree; a worktree with no session offers exactly one
   way to create its session, with the definition given there.
4. **Given** the AI CLI chosen at start is not available, **When** the worktree is created,
   **Then** the failure is reported as it is for a session start today and nothing is created: no
   worktree and no session, as `create_session` leaves no record for a missing CLI.
5. **Given** a `create_worktree` call without an AI CLI or prompt, **When** it succeeds, **Then**
   the worktree has no session, and `create_session` on it then creates its one session (the
   `create_worktree` then `create_session` flow keeps working).

---

### User Story 3 - Resume and restart replace, never add (Priority: P2)

Resuming or restarting a worktree's session brings that one session back or replaces its process.
It never leaves two sessions, and a session picked from a provider's resumable list takes the
worktree's one place.

**Why this priority**: The refusal in story 1 would otherwise make recovery a dead end.

**Independent Test**: Stop the session, start it again, restart it, and resume a provider session
into the worktree; after each the worktree has exactly one session.

**Acceptance Scenarios**:

1. **Given** a stopped session, **When** `start_session` or the sidebar's start runs, **Then** the
   same session runs again and no second session appears.
2. **Given** a running session, **When** it is restarted, **Then** the running process is replaced
   and the worktree still has one session.
3. **Given** a worktree with a session and a resumable provider session, **When** the user resumes
   the provider session into that worktree, **Then** it replaces the worktree's session, after
   the user confirms, rather than being added beside it.
4. **Given** the replaced session, **When** the replacement finishes, **Then** no process of the
   old session keeps running in the worktree.

---

### User Story 4 - Existing worktrees with several sessions (Priority: P3)

A catalog written before this feature may hold worktrees with more than one session. The
application must not lose them, and must reach the one-session state without a surprise.

**Why this priority**: Only affects data that already exists; new data cannot get there.

**Independent Test**: Open a catalog whose worktree holds two sessions; both remain visible, no
data is lost, and no further session can be added.

**Acceptance Scenarios**:

1. **Given** a catalog worktree with two sessions, **When** the application opens it, **Then**
   neither session is deleted automatically.
2. **Given** that worktree, **When** the user asks for another session, **Then** it is refused as
   in story 1.
3. **Given** that worktree with one of its sessions running, **When** another of its sessions is
   started, **Then** it is refused and the refusal names the running one.

---

### Edge Cases

- **Failed or exited session**: counts as the worktree's session. A failed start does not free the
  worktree for a different definition; the user restarts it or deletes it first.
- **Delete, then create**: after the session is deleted the worktree has none, and one new session
  may be created in it, with its definition given then (FR-010).
- **Empty definition**: an empty prompt at start is a worktree with a session and no first prompt,
  as `create_session` without a prompt is today; an AI CLI left unnamed takes the default one.
- **Zero-session worktrees**: one created without a definition, one attached with nothing to
  resume, one whose session was deleted, or one from an older catalog: each takes exactly one
  session through the sidebar or `create_session`, and is refused a second.
- **Default (project root)**: the same rule (FR-011).
- **Concurrency**: two windows, or an agent and a window, asking at once: one outcome, never two
  sessions (Principle II).
- **Daemon restart or lost connection** between the check and the create: no duplicate after
  reconnecting.
- **Worktree deleted** while a create is in flight: the create fails as it does today and leaves no
  session.
- **Attached provider worktree** (feature 582): at most one session catalogued (FR-012).
- **Platforms**: the rule is the same on Linux, macOS and Windows; nothing in it depends on a
  path or process behaviour (Principle VI).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: A worktree MUST host at most one session at any time, whatever the state of that
  session.
- **FR-002**: Asking for a new session in a worktree that has one MUST be refused, from the
  sidebar and from the `create_session` tool, and MUST leave the catalog unchanged.
- **FR-003**: The refusal MUST name the existing session (its id and its displayed name) and say
  that `start_session` on that id brings it back.
- **FR-004**: The sidebar MUST NOT offer a way to add a session to a worktree that has one.
- **FR-005**: Concurrent requests for a session in one worktree, from any mix of windows and
  agents, MUST yield at most one session.
- **FR-006**: The create-worktree dialog and the `create_worktree` tool MUST accept the session's
  definition (AI CLI, model where the CLI takes one, first prompt) and, when given, create the
  worktree together with its session in that one step. Without a definition a worktree is created
  with no session (FR-010). This adds optional inputs to `create_worktree`; existing calls keep
  their result.
- **FR-007**: After a provider session is resumed into a worktree (FR-008b) the worktree MUST list
  exactly one session, the resumed one; the session it replaced is no longer listed.
- **FR-008a**: Resuming or restarting a worktree's own session MUST keep that session (same id,
  name and definition) and replace only its process; a running process MUST be stopped first, so
  no two processes of it run, and no confirmation is asked beyond what restart asks today.
- **FR-008b**: Resuming a provider session into a worktree that has a session MUST replace that
  session's record with the resumed one, stop the old process first, and require the user's
  confirmation. Only a person can ask for it (no tool resumes a provider session). It never adds
  a second session.
- **FR-009**: `start_session` on a session that is starting, running or restarting MUST stay a
  no-op that reports success; on a stopped one it MUST start that same session.
- **FR-010**: A worktree with no session (created without a definition, attached with nothing to
  resume, or its session deleted) MUST be able to take exactly one new session, through the sidebar
  or `create_session`, with its definition given then; after that FR-002 applies.
- **FR-011**: The rule applies to the Default (project root) location exactly as to named
  worktrees: at most one session, refused second, defined once.
- **FR-012**: A worktree attached from a provider (feature 582) with several resumable sessions
  MUST follow the rule: at most one of its sessions is catalogued. A single resume is the user's
  pick of the listed session; "attach all" catalogues only the most recent session per worktree and
  leaves the others listed as resumable.
- **FR-013**: A catalog that already holds several sessions in a worktree MUST open without losing
  or changing any of them, and no new session may be created there. Starting one of them while
  another session of that worktree is starting or running MUST be refused, the refusal naming the
  running one (as FR-003); the user may delete the extras. FR-009's start of a stopped session is
  subject to this.
- **FR-014**: The user guide and the tool catalogue (`create_session`, `start_session`,
  `create_worktree`) MUST describe the rule, the definition at start and the refusal's wording.
  The guide states that a second session is refused and that the refusal names the existing one.

### Key Entities

- **Worktree**: has at most one session; the invariant is on it, not on the session.
- **Session**: belongs to one worktree; carries its definition (AI CLI, model, first prompt).
- **Refusal**: the answer to a second-session request: names the existing session's id and name.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Outside legacy catalogs that already held several sessions in a worktree (FR-013), in any sequence of create, start, stop, restart, resume and delete operations from
  any window and any agent, no worktree is ever observed with more than one session.
- **SC-002**: 100% of second-session requests through the sidebar and the tools are refused with
  an answer that names the existing session.
- **SC-003**: Of 100 concurrent create requests for one empty worktree, exactly one succeeds.
- **SC-004**: A user can create a worktree and have its session running with the chosen definition
  in a single dialog confirmation, with no further step.
- **SC-005**: No session record is lost when an existing catalog is opened.

## Assumptions

- A session in any lifecycle state counts as the worktree's session; "exactly one" is about
  records, not only running processes.
- The model is part of the definition only for AI CLIs that take one; others have agent and prompt.
- Restart and resume reuse the existing start paths; no new lifecycle state is introduced.
- Out of scope: changing which worktree a session belongs to, running several agents inside one
  session, and any change to sessions across different worktrees (Principle II is unchanged there).
- Depends on features 491 (multi-session daemons), 550 (create_worktree form inputs) and 582
  (attach provider worktrees and sessions).
