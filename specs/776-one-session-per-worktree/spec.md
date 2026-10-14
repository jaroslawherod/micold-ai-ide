# Feature Specification: One Session per Worktree, Chosen at Start

**Feature Branch**: `feat/776_one-session-per-worktree-chosen-at-start`

**Created**: 2026-10-12

**Status**: Draft

**Amended**: 2026-10-13 (soft limit, default session; see Clarifications, Session 2026-10-13). The
issue's "no way to run more than one session" is superseded by the user: the limit is a default
that an explicit request overrides.

**Amended**: 2026-10-14 (main session; see Clarifications, Session 2026-10-14). The derived
"default session" (last viewed) is replaced by a designated main session per worktree; the other
sessions are helpers.

**Input**: User description: "Implement GitHub issue #776: One session per worktree, chosen at start. A worktree should host exactly one session. The session is defined when the worktree is started, and there must be no way to run more than one session in the same worktree. Expected: starting a session in a worktree that already has one is refused (UI and the `create_session` / `start_session` MCP tools), with a clear error naming the existing session; the session's definition (agent/model/prompt) is set at worktree start, not added afterwards; resuming or restarting replaces the worktree's session rather than adding a second one. Why: concurrent sessions in one worktree edit the same files and branch, and race on the build lock and git index."

## Terms

- **Worktree**: a git worktree the application manages, shown in the sidebar under its own name.
- **Session**: one AI CLI conversation (Claude Code, Copilot CLI or Pi) running in a worktree.
- **Session definition**: what makes the session what it is: the AI CLI (agent), its model where
  the CLI takes one, and the first prompt.
- **Worktree start**: the moment a worktree is created (create-worktree dialog or `create_worktree`
  MCP tool) together with the session that will run in it.
- **Own session**: the (main) session a worktree hosts by default, whatever its state (starting, running,
  stopped, failed, exited). A deleted (archived) session does not occupy the worktree. Unless an
  explicit request says otherwise a worktree has at most one; it has none only in the cases of
  FR-010 (created without a definition, attached with nothing to resume, or its session deleted),
  and then takes one, defined once at that moment.
- **Additional session**: a second or later session in a worktree that already has one, created or
  started only on an explicit request (FR-015, FR-016). Never created by a default path. It is a
  helper.
- **Main session**: the one session of a worktree that is responsible for its work. It is
  designated, not derived: the session the worktree is started with (or the first one it takes, or
  the one that replaced it by a confirmed resume) is main, and stays main until the user changes it
  (FR-020) or it is deleted (FR-022). A worktree has at most one; selecting the worktree opens it.
- **Helper session**: every other un-archived session of a worktree: an additional session, or an
  extra one of a legacy catalog. Meant for short side activities; marked as a helper wherever
  sessions of the worktree are listed. Nothing ends it automatically.

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

### Session 2026-10-13

- Q: Is the one-session limit absolute? → A: No, it is soft: a refusal naming the existing session
  stays the default, and starting more than one session in a worktree is allowed when really
  requested _(decided by user: "the limit should be soft, allow user to start more than one session
  if really requested")_
- Q: What is an explicit request? → A: In the app, a confirmation step naming the existing session;
  on the tool server, the boolean input `allow_additional_session` (default false) on
  `create_session` and `start_session`; on the wire, the same-named field on `SessionCreate`,
  `SessionStart`, `SessionRestart` and `AttachItem::Session`. Default behaviour is unchanged
  _(the app's confirm and the tools' opt-in are decided by user; the parameter's name is
  agent-resolved: snake_case boolean, as `ai_cli` and `prompt` on the same tools)_
- Q: What does the sidebar show? → A: By default only worktrees: selecting a worktree selects its
  default session _(decided by user: "by default it should show only work tree and when selecting a
  worktree it should select default session")_
- Q: Which session is the default? → A: _(superseded 2026-10-14 by the main session, Session
  2026-10-14)_ The one last viewed in that worktree in this window if it
  is still un-archived, else the first un-archived session in list order _(agent-resolved: the list
  order is stable and persisted, and the refusal already names that first session, FR-003;
  `Session` has no persisted activity timestamp, only the transient `attention_seq`)_
- Q: Where are additional sessions reachable? → A: As sub-rows of their worktree, collapsed by
  default and present only when the worktree has more than one session; a worktree with one
  session shows a plain row. Adding one is the worktree row menu's "Start another session..."
  with the confirmation _(agent-resolved: the sidebar already draws worktrees with expandable
  session sub-rows, `crates/micold-client/src/ui/sidebar.rs`; this keeps that tree and hides it
  until there is something to expand)_

### Session 2026-10-14

- Q: Which session does a worktree open, and who is responsible for the work? → A: Each worktree
  has one designated main session, responsible for the work; the other sessions are helpers,
  meant for short activities. Selecting the worktree row always opens the main session, never a
  helper _(decided by user: "for each worktree there should one main session that is responsible
  for work the others should be considered a only helpers or some short time activities")_.
  Supersedes the 2026-10-13 answer that the default is the last viewed.
- Q: When is the main session set, and when does it change? → A: It is set at start (the session
  the worktree is created or started with) and stays main until the user changes it _(decided by
  user: the same message; "amend after M2")_. Creating a helper, viewing one or resuming beside the
  main never changes it.
- Q: What happens when the main session is deleted (archived)? → A: The first remaining
  un-archived session in list order is promoted to main, without asking, and the delete
  confirmation names it; with none left the worktree is free (FR-010) _(agent-resolved: the user
  asked for "a rule"; list order is stable, persisted and already what a refusal names; asking
  would add a dialog to a delete)_.
- Q: How does the user change the main session? → A: The sub-row menu of a helper has "Make main
  session"; the previous main becomes a helper. The tool server has no equivalent: agents start
  helpers with `allow_additional_session` but do not re-designate _(agent-resolved: the app owns
  who is responsible, as it owns confirmations; a tool can be added without a wire change later)_.
- Q: Is a helper marked, and does it expire? → A: It is marked "helper" in the worktree's
  expanded session list and is not auto-closed; the user deletes it _(agent-resolved: "short time"
  describes intent; auto-deletion would lose output and is not asked for)_.
- Q: What does a confirmed resume replace and what role does the result have? → A: It replaces the
  main session and the resumed session is main; "keep both" adds it as a helper _(agent-resolved:
  the refusal names the main session, so that is what the user confirmed against)_.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A second session in a worktree is refused unless really requested (Priority: P1)

A user, or an agent through the tool server, asks for a new session in a worktree that already has
one. The request is refused by default with an error that names the existing session and says how
to ask for another on purpose, so neither a person nor an agent gets two sessions editing the same
files, branch, build lock and git index by accident. A request that really asks for it (US6) is
honoured.

**Why this priority**: This is the default invariant. Everything else supports it.

**Independent Test**: Create a worktree with a session, then ask for another session in it from the
sidebar and through `create_session`; both are refused, and the worktree still has exactly one
session. Then call `create_session` with `allow_additional_session: true`: the worktree has two.

**Acceptance Scenarios**:

1. **Given** a worktree with a session in any state, **When** a window that still shows the
   start action from before the session existed asks for a session in it without an explicit
   request, **Then** no session is created and the user is told which session the worktree already
   has.
2. **Given** a worktree with a session, **When** an agent calls `create_session` for that worktree,
   **Then** the call fails with an error naming the existing session (its id and name) and no
   record is added.
3. **Given** a worktree with a session, **When** two `create_session` calls for it arrive at the
   same moment, **Then** exactly one succeeds when the worktree had none, and none succeeds when it
   had one.
4. **Given** the refusal, **When** the agent reads the error, **Then** it holds the existing
   session's id and name (FR-003) and names `allow_additional_session` as the way to ask for
   another (FR-015).
5. **Given** a worktree with a session, **When** an agent calls `create_session` with
   `allow_additional_session: true`, **Then** a second session is created in that worktree and the
   first is untouched (FR-015).
6. **Given** a worktree with a session, **When** `create_session` is called with
   `allow_additional_session: true` and the AI CLI is missing, **Then** it fails as without the
   flag and nothing is created.

---

### User Story 2 - The session is defined when the worktree starts (Priority: P1)

A user creating a worktree may pick the AI CLI and the first prompt in the same step; when they
do, the worktree and its session are created together. There is no start action on the row of a worktree that
already started; the definition is not changed by adding another session later.

**Why this priority**: It is what makes "exactly one" a property of the model, not only a check.

**Independent Test**: Create a worktree through the dialog and through `create_worktree` with an AI
CLI and a prompt; the worktree has one session with that definition and the row offers no start action.

**Acceptance Scenarios**:

1. **Given** the create-worktree dialog, **When** the user picks an AI CLI and a first prompt and
   confirms, **Then** the worktree is created and its one session starts with that definition.
2. **Given** a `create_worktree` call that names an AI CLI and a prompt, **When** it succeeds,
   **Then** the session runs in the new worktree with that definition, and the reply names it.
3. **Given** a worktree with a session, **When** the user looks at the sidebar, **Then** its row
   offers no start action (adding another session is the explicit request of US6); a worktree
   with no session offers exactly one start action, with the definition given there.
4. **Given** the AI CLI chosen at start is not available, **When** the worktree is created,
   **Then** the failure is reported as it is for a session start today and nothing is created: no
   worktree and no session, as `create_session` leaves no record for a missing CLI.
5. **Given** a `create_worktree` call without an AI CLI or prompt, **When** it succeeds, **Then**
   the worktree has no session, and `create_session` on it then creates its one session (the
   `create_worktree` then `create_session` flow keeps working).

---

### User Story 3 - Resume and restart replace, never add (Priority: P2)

Resuming or restarting a worktree's session brings that one session back or replaces its process.
It never leaves two sessions by itself, and a session picked from a provider's resumable list takes
the worktree's one place unless the user chose to keep both.

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
   the user confirms, rather than being added beside it; the confirmation also offers to keep
   both (FR-008c).
4. **Given** the replaced session, **When** the replacement finishes, **Then** no process of the
   old session keeps running in the worktree.

---

### User Story 4 - Worktrees that hold several sessions (Priority: P3)

A catalog written before this feature, or a worktree where the user asked for an additional
session, holds more than one session. The application must not lose them, must not run two of them
at once by accident, and must let the user run both on purpose.

**Why this priority**: Only affects data that already exists or was explicitly requested.

**Independent Test**: Open a catalog whose worktree holds two sessions; both remain visible, no
data is lost, no further session is added without an explicit request, and starting the second
while the first runs needs one.

**Acceptance Scenarios**:

1. **Given** a catalog worktree with two sessions, **When** the application opens it, **Then**
   neither session is deleted automatically.
2. **Given** that worktree, **When** another session is asked for without an explicit request,
   **Then** it is refused as in story 1.
3. **Given** that worktree with one of its sessions running, **When** another of its sessions is
   started without an explicit request, **Then** it is refused and the refusal names the running
   one; with the request (`allow_additional_session: true`, or the app's confirmation) both run.

---

### User Story 5 - The sidebar shows worktrees; selecting one opens its main session (Priority: P2)

A user sees one row per worktree, with no session rows under it. Selecting a worktree opens its
main session. A worktree that holds more than one session can be expanded to show them, the main
one first and the others marked as helpers.

**Why this priority**: It matches the model (a worktree is the unit, its session is its content)
and removes rows that duplicate the worktree.

**Independent Test**: With one session in each of two worktrees the sidebar lists two rows and no
session rows; selecting a worktree views its session. Give one worktree a second session: its row
gains an expander, collapsed, listing both when opened, the second marked as a helper.

**Acceptance Scenarios**:

1. **Given** worktrees each with one session, **When** the sidebar renders, **Then** it shows one
   row per worktree, no session sub-rows and no expander.
2. **Given** a worktree with a session, **When** the user selects its row, **Then** its main
   session is selected and shown (FR-018).
3. **Given** a worktree with several sessions, **When** the sidebar renders, **Then** its row is
   collapsed with an expander; opening it lists every session, the main one first and the others
   marked as helpers, and selecting a listed session views that one.
4. **Given** a worktree whose helper session was last viewed, **When** the user selects another
   worktree and then this one, **Then** the main session is shown (a helper is only opened by
   choosing it in the list).
5. **Given** a worktree without a session, **When** the user selects it, **Then** nothing is
   selected and its row offers the start action of FR-006.

---

### User Story 6 - Starting a helper session on purpose (Priority: P2)

A user who really wants a second session in a worktree asks for it and confirms; an agent passes
`allow_additional_session: true`. The worktree then holds both, each independently addressable;
the new one is a helper and the main session stays main.

**Why this priority**: The user made the limit soft; without this story the soft limit is
unreachable from the app.

**Independent Test**: In the app, open the worktree row menu on an occupied worktree, choose
"Start another session...", confirm: the worktree holds two sessions and the second is shown.

**Acceptance Scenarios**:

1. **Given** a worktree with a session, **When** the user opens its row menu, **Then** it offers
   "Start another session..."; a worktree without a session does not (its row has the start
   action).
2. **Given** that choice, **When** the confirmation opens, **Then** it names the existing session
   and the worktree, and offers confirm and cancel; cancel changes nothing.
3. **Given** the user confirms, **When** the session is created, **Then** the worktree holds both,
   the new one is a helper, selected and shown, and the first is untouched and still main.
4. **Given** the confirm step is pending, **When** a second window or an agent adds a session
   first, **Then** the confirmed request still creates its session (the user asked for one more).

---

### User Story 7 - One main session, changeable by the user (Priority: P2)

A user sees which session of a worktree is the main one, can make another session the main one,
and keeps a sensible main when it is deleted.

**Why this priority**: The user wants one session responsible for the work and the rest as
helpers; without a way to change it, the choice at start would be permanent.

**Independent Test**: With a worktree holding a main and a helper, make the helper main; selecting
the worktree row now opens it, and the former main is listed as a helper. Delete the main: the
remaining session is main.

**Acceptance Scenarios**:

1. **Given** a worktree created with a session, **When** it is listed, **Then** that session is
   main and any session added later is a helper.
2. **Given** a worktree with a main and a helper, **When** the user chooses "Make main session" in
   the helper's menu, **Then** the helper is main, the previous main is a helper, the change
   survives a restart, and selecting the row opens the new main.
3. **Given** a worktree with a main and one or two helpers, **When** the main session is deleted,
   **Then** the first remaining un-archived session in list order is main, and the delete
   confirmation named it.
4. **Given** a worktree with only a main session, **When** it is deleted, **Then** the worktree is
   free and the next session created takes the main role.
5. **Given** a catalog written before this feature with several sessions in a worktree, **When** it
   opens, **Then** the first un-archived session in list order is its main and the others are
   helpers, and no record is rewritten until the user changes the main.

---

### Edge Cases

- **Failed or exited session**: counts as the worktree's session. A failed start does not free the
  worktree for a different definition; the user restarts it or deletes it first.
- **Delete, then create**: after the session is deleted the worktree has none, and one new session
  may be created in it, with its definition given then (FR-010).
- **Empty definition**: an empty prompt at start is a worktree with a session and no first prompt,
  as `create_session` without a prompt is today; an AI CLI left unnamed takes the default one.
- **Zero-session worktrees**: one created without a definition, one attached with nothing to
  resume, one whose session was deleted, or one from an older catalog: each takes one
  session through the sidebar or `create_session`, and is refused a second unless it is requested
  explicitly.
- **Default (project root)**: the same rule (FR-011).
- **Concurrency**: two windows, or an agent and a window, asking at once without an explicit
  request: one outcome, never two sessions (Principle II). With the request each asks for its own
  session and each gets one.
- **Explicit request to a free worktree**: `allow_additional_session: true` on a worktree with no
  session simply creates its first session; the flag changes nothing there.
- **Main session deleted**: the first remaining un-archived session in list order becomes main
  (FR-022); with none, the worktree is free (FR-010).
- **Helper deleted**: the main is unchanged.
- **Make main on the main or on an archived session**: a no-op or refused; never two mains.
- **Main session stopped, failed or exited**: still main; the user restarts it or makes another
  main.
- **Several sessions running**: only on an explicit request; the build lock and git index are the
  user's to manage, the warning is the confirmation's wording.
- **Daemon restart or lost connection** between the check and the create: no duplicate after
  reconnecting.
- **Worktree deleted** while a create is in flight: the create fails as it does today and leaves no
  session.
- **Attached provider worktree** (feature 582): at most one session catalogued (FR-012).
- **Platforms**: the rule is the same on Linux, macOS and Windows; nothing in it depends on a
  path or process behaviour (Principle VI).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: A worktree MUST by default host at most one session at any time, whatever the state
  of that session. This is a soft limit: only an explicit request (FR-015, FR-016) puts a second
  session beside it.
- **FR-002**: Asking for a new session in a worktree that has one, without an explicit request,
  MUST be refused, from a window and from the `create_session` tool, and MUST leave the catalog
  unchanged.
- **FR-003**: The refusal MUST name the existing session (its id and its displayed name), say that
  `start_session` on that id brings it back, and say how to ask for another on purpose
  (`allow_additional_session: true` on the tool; the confirmation in the app).
- **FR-004**: The sidebar row of a worktree that has a session MUST NOT offer a start action;
  adding a session to it is only FR-016's menu entry and confirmation.
- **FR-005**: Concurrent requests for a session in one worktree without an explicit request, from
  any mix of windows and agents, MUST yield at most one session. Requests with the explicit
  request each create their own.
- **FR-006**: The create-worktree dialog and the `create_worktree` tool MUST accept the session's
  definition (AI CLI, model where the CLI takes one, first prompt) and, when given, create the
  worktree together with its session in that one step. Without a definition a worktree is created
  with no session (FR-010). This adds optional inputs to `create_worktree`; existing calls keep
  their result.
- **FR-007**: After a provider session is resumed into a worktree and the user chose to replace
  (FR-008b) the worktree MUST list exactly one session, the resumed one; the session it replaced
  is no longer listed.
- **FR-008a**: Resuming or restarting a worktree's own session MUST keep that session (same id,
  name and definition) and replace only its process; a running process MUST be stopped first, so
  no two processes of it run, and no confirmation is asked beyond what restart asks today.
- **FR-008b**: Resuming a provider session into a worktree that has a session MUST replace that
  session's record with the resumed one, stop the old process first, and require the user's
  confirmation. Only a person can ask for it (no tool resumes a provider session). It never adds
  a second session unless the user chose to keep both (FR-008c).
- **FR-008c**: The confirmation of FR-008b MUST also offer to keep both: the resumed session is
  then added beside the existing one, nothing is stopped, and the item carries
  `allow_additional_session: true`.
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
  or changing any of them, and no new session may be created there without an explicit request.
  Starting one of them while another session of that worktree is starting or running MUST be
  refused unless the request is explicit (FR-015, FR-016), the refusal naming the running one (as
  FR-003); the user may delete the extras. FR-009's start of a stopped session is subject to
  this.
- **FR-014**: The user guide and the tool catalogue (`create_session`, `start_session`,
  `create_worktree`) MUST describe the rule, the definition at start, the refusal's wording and
  the explicit request. The guide states that a second session is refused by default, that the
  refusal names the existing one and how to ask for another.
- **FR-015**: `create_session` and `start_session` MUST accept an optional boolean
  `allow_additional_session` (default false). When true, the check of FR-002 and FR-013 is skipped
  and the call behaves as for a free worktree; policy, CLI availability and every other check still
  apply. The same field, defaulting false, is carried by the wire messages `SessionCreate`,
  `SessionStart`, `SessionRestart` and `AttachItem::Session`.
- **FR-016**: The app MUST ask for an additional session only through the worktree row menu entry
  "Start another session..." (offered only when the worktree has a session) and a confirmation that
  names the worktree and its existing session; starting a stopped session that the refusal of
  FR-013 blocked uses the same confirmation. Cancel changes nothing; confirm sends the request with
  `allow_additional_session: true` and selects the session.
- **FR-017**: The sidebar MUST show one row per worktree (and the Default root) and no session
  rows by default. A row of a worktree with more than one session MUST have an expander, collapsed
  by default, that lists its sessions, the main one first and the others marked as helpers. A
  worktree with one session has no expander.
- **FR-018**: Selecting a worktree row MUST select and show its main session (Terms), never a
  helper, whatever was last viewed. With no session nothing is selected and the row keeps its start
  action (FR-010).
- **FR-020**: Every worktree with an un-archived session MUST have exactly one main session. It is
  set when the first session of the worktree is created (by worktree start, `create_session` on a
  free worktree, or adoption), is persisted, and changes only by the user's explicit choice
  (FR-021), by a confirmed resume that replaces it (FR-007, the resumed session becomes main) or by
  its deletion (FR-022). A session created beside it (FR-015, FR-008c) is a helper. A catalog
  without the marker derives the main as the first un-archived session in list order and rewrites
  nothing.
- **FR-021**: The app MUST let the user make a helper the main session (menu entry "Make main
  session" on its sub-row); the previous main becomes a helper in the same change. Making the main
  main again is a no-op.
- **FR-022**: When the main session is deleted (archived) and the worktree still has un-archived
  sessions, the first of them in list order MUST become main in the same change; the delete
  confirmation MUST name it. Deleting a helper never changes the main.
- **FR-019**: Session actions the removed sub-row carried (rename, close) MUST stay reachable: for
  the one session of a worktree from the worktree row menu, for each session of a multi-session
  worktree from its sub-row menu as today.

### Key Entities

- **Worktree**: has at most one session by default; the invariant is on it, not on the session.
  Exactly one of its un-archived sessions is main.
- **Session**: belongs to one worktree; carries its definition (AI CLI, model, first prompt) and
  whether it is the worktree's main session.
- **Refusal**: the answer to a second-session request: names the existing session's id and name.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In any sequence of create, start, stop, restart, resume and delete operations from
  any window and any agent that carry no explicit request, no worktree is ever observed gaining a
  second session (legacy catalogs that already held several, FR-013, only keep what they had).
- **SC-002**: 100% of second-session requests without an explicit request, through the windows and
  the tools, are refused with an answer that names the existing session and the way to ask for
  another.
- **SC-003**: Of 100 concurrent create requests for one empty worktree without an explicit
  request, exactly one succeeds.
- **SC-006**: A user with one session per worktree sees no session rows in the sidebar, and a
  single selection opens the worktree's session.
- **SC-007**: A user adds an additional session in at most the menu entry and one confirmation.
- **SC-008**: In a worktree with several sessions, selecting its row opens the same session (the
  main) every time until the user changes it, and the main survives a daemon restart.
- **SC-004**: A user can create a worktree and have its session running with the chosen definition
  in a single dialog confirmation, with no further step.
- **SC-005**: No session record is lost when an existing catalog is opened.

## Assumptions

- A session in any lifecycle state counts as the worktree's session; "exactly one" is about
  records, not only running processes.
- The model is part of the definition only for AI CLIs that take one; others have agent and prompt. No AI CLI takes a model today (research R4), so the definition carries agent and prompt. Replacing (FR-007, FR-008b) archives the old record rather than deleting it.
- Restart and resume reuse the existing start paths; no new lifecycle state is introduced.
- The limit is soft by the user's decision: the explicit request is a boolean per call and a
  confirmation in the app, not a setting; nothing stores a per-worktree "allow many".
- The main session is designated and persisted (one marker per session record, serde-defaulted so
  an older catalog loads unchanged); a catalog without it derives the main as the first
  un-archived session in list order (FR-020). Nothing else new is persisted.
- Out of scope: changing which worktree a session belongs to, running several agents inside one
  session, auto-closing or expiring helpers, a tool that changes the main, a setting that lifts the limit for a worktree or a project, ordering or merging the
  sessions of one worktree, and any change to sessions across different worktrees (Principle II is
  unchanged there).
- Depends on features 491 (multi-session daemons), 550 (create_worktree form inputs) and 582
  (attach provider worktrees and sessions).
