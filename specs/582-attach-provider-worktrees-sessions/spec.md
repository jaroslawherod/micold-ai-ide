# Feature Specification: Attach a Provider's Existing Worktrees and Sessions

**Feature Branch**: `feat/582-attach-provider-worktrees-sessions`

**Created**: 2026-10-05

**Status**: Draft

**Input**: User description: "Implement GitHub issue #582: AI CLI providers should be able to attach existing worktrees and sessions. When Micold starts with an empty catalog (a new machine, or a lost data directory), a provider's existing worktrees and sessions are not picked up. The worktrees under `.claude/worktrees/` are known but marked assistant-owned, so the sidebar hides them until "Show agent worktrees" is on. Claude Code's own sessions for the project are not found; only the running session appears. `create_worktree` cannot re-attach those worktrees because their branches are already checked out in the agent worktree. Expected: an AI CLI provider (Claude Code, Copilot, Pi) can attach worktrees it created and sessions it can resume, and they appear in the app instead of being hidden or lost; resumable sessions from the provider's own store (for example `~/.claude/projects/<project>`) are discovered and listed as resumable; attaching a provider-owned worktree is possible from the app and from the MCP tools (an `attach_worktree` operation) without deleting and recreating it."

## Terms

- **Provider**: an AI CLI that Micold runs in a session: Claude Code, Copilot CLI or Pi.
- **Catalog**: Micold's own record of a project's worktrees and sessions, kept in its data directory.
- **Provider worktree**: a worktree a provider created itself (for Claude Code, under
  `.claude/worktrees/`), which Micold marks as agent-owned and hides by default.
- **Provider store**: the place where a provider keeps its own resumable sessions (for Claude Code,
  `~/.claude/projects/<project>`).
- **Attach**: make an existing provider worktree or provider session a regular entry of the catalog,
  visible in the app, without deleting or recreating it.
- **Resumable session**: a session in a provider store that the provider can resume.

## Clarifications

### Session 2026-10-05

- Q: Is listing resumable sessions a new MCP tool or an option of `list_sessions`? → A: A new
  read-only tool; `list_sessions` keeps listing only catalog sessions, and `list_branches` already
  sets the precedent of a separate discovery tool _(agent-resolved: docs/user-guide/agent-tools.md#What the assistant can do)_
- Q: May a Default session call `attach_worktree` (refuse, allow after confirmation, or amend Principle III)? → A: Refuse outright; Principle III bars a Default session from modifying a worktree by any means, tool server included, and the amendment route is out of this feature _(agent-resolved: .specify/memory/constitution.md#III. Native Worktree Integration)_

- Q: Does attaching a resumable session start it? → A: No; attaching adds an idle catalog entry waiting to be resumed, and only the user's resume (Start) runs the provider, so "attach all" never launches many provider processes _(agent-resolved: docs/user-guide/agent-tools.md#What the assistant can do, `start_session` "waiting to be resumed")_
- Q: What happens when the user resumes a session whose worktree is attachable but not yet attached? → A: Resuming first attaches that worktree (an explicit user action, so FR-012 holds), then resumes; every catalog session maps to a catalog worktree _(agent-resolved: .specify/memory/constitution.md#III. Native Worktree Integration)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Attach a provider worktree from the app (Priority: P1)

A user whose catalog is empty (new machine, lost data directory) opens a project that already has
provider worktrees. They find the worktrees offered for attaching, attach one, and it appears in
the sidebar as a normal worktree with its branch and files untouched.

**Why this priority**: It is the core loss in the issue: the worktrees exist but the user cannot
reach them without turning on a hidden-worktree filter, and cannot re-create them either.

**Independent Test**: Create worktrees under `.claude/worktrees/`, start with an empty catalog,
attach one from the app, and check it shows in the sidebar with the filter off and with its
uncommitted changes intact.

**Acceptance Scenarios**:

1. **Given** an empty catalog and 3 provider worktrees in the project, **When** the user opens the
   project, **Then** the 3 worktrees are listed as available to attach.
2. **Given** an attachable worktree, **When** the user attaches it, **Then** it appears in the
   sidebar with "Show agent worktrees" off, and its branch, files and uncommitted changes are
   unchanged.
3. **Given** an attached worktree, **When** the user attaches it again or attaches one already
   attached, **Then** nothing is duplicated and the user sees that it is already attached.

---

### User Story 2 - Discover and resume provider sessions (Priority: P1)

A user opens a project whose catalog is empty. Micold looks in the provider's own store, finds the
sessions the provider can resume for that project, and lists them as resumable. The user resumes
one and the provider continues that conversation in the right worktree.

**Why this priority**: Today only the running session appears, so past work is lost.

**Independent Test**: Seed a provider store with sessions for a project and another project, start
with an empty catalog, and check only this project's sessions are listed as resumable, and one
resumes.

**Acceptance Scenarios**:

1. **Given** a provider store with 5 sessions for the project and 2 for another project, **When**
   the project opens, **Then** the 5 are listed as resumable and the 2 are not.
2. **Given** a listed resumable session, **When** the user resumes it, **Then** the provider
   resumes that conversation and the session shows in the sidebar like any other session.
3. **Given** a session already in the catalog, **When** discovery runs, **Then** it is not listed
   twice.
4. **Given** a provider store that is missing or unreadable, **When** the project opens, **Then**
   the project opens normally with no resumable sessions and the user can see why none were found.

---

### User Story 3 - Attach through the MCP tools (Priority: P2)

An agent or script using Micold's MCP tools attaches a provider worktree with an `attach_worktree`
operation and lists resumable sessions, instead of failing with "already checked out in the agent
worktree".

**Why this priority**: The issue names the MCP route explicitly; the app route (Story 1) delivers
the core value first.

**Independent Test**: Call `attach_worktree` for an existing provider worktree and check it is
listed by `list_worktrees` without the hidden-worktree option.

**Acceptance Scenarios**:

1. **Given** an existing provider worktree, **When** `attach_worktree` is called with its path or
   branch, **Then** it is attached and `list_worktrees` returns it as not hidden.
2. **Given** a path that is not a worktree of the project, **When** `attach_worktree` is called,
   **Then** it fails with a clear reason and changes nothing.
3. **Given** an already attached worktree, **When** `attach_worktree` is called, **Then** it
   reports it was already attached and creates no duplicate.
4. **Given** a Default session, **When** it calls `attach_worktree`, **Then** it is refused with a
   clear reason and nothing is attached (FR-015).

---

### User Story 4 - Everything is found at start without manual steps (Priority: P3)

When Micold starts with an empty catalog it finds attachable worktrees and resumable sessions on its
own and tells the user, so a lost data directory is recovered in one click.

**Why this priority**: A convenience on top of Stories 1 and 2.

**Independent Test**: Remove the data directory, start the app, and check it offers attaching all
found worktrees and sessions at once.

**Acceptance Scenarios**:

1. **Given** an empty catalog with attachable worktrees and sessions, **When** Micold starts,
   **Then** the user is offered attaching all of them in one action.
2. **Given** a non-empty catalog, **When** Micold starts, **Then** nothing is attached without the
   user asking.

---

### Edge Cases

- A provider worktree directory was deleted but git still lists it (prunable): it is shown as
  unavailable, not attachable.
- A worktree's branch is checked out in another worktree of another project: attaching reports the
  conflict and changes nothing.
- A provider store holds thousands of sessions: listing stays responsive and shows the most recent
  first.
- A session's worktree no longer exists: the session is flagged unresumable with the reason; it is
  never resumed in a different worktree.
- A session whose working directory is the project root is listed as a Default session, not as a
  worktree session.
- The provider's store format changes or a session file is corrupt: that entry is skipped and
  reported, others still list.
- Two windows or an agent and the app attach the same worktree at once: exactly one catalog entry
  results and both callers get a success or "already attached".
- A provider session is resumed in two places at once: the second request is refused with a clear
  reason rather than starting a second copy.
- Paths and provider stores differ on macOS and Windows (home directory, path separators, case):
  discovery and attaching behave the same on every supported platform.
- The sandboxed runtime cannot read the provider store: the user is told discovery is unavailable
  there.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST list a project's existing provider worktrees that are not in the
  catalog as available to attach.
- **FR-002**: Users MUST be able to attach a provider worktree from the app, after which it appears
  in the sidebar without turning on "Show agent worktrees".
- **FR-003**: Attaching MUST NOT delete, recreate, check out, or change the worktree's branch,
  files or uncommitted changes.
- **FR-004**: Attaching an already attached worktree MUST be a no-op that tells the user, never a
  duplicate.
- **FR-005**: The MCP tools MUST offer an `attach_worktree` operation with the same effect as
  attaching in the app, and it MUST fail with a clear reason, changing nothing, for a path that is
  not an attachable worktree of the project.
- **FR-006**: The system MUST discover resumable sessions in Claude Code's own store that belong to
  the project, and list them as resumable. For Copilot and Pi it MUST do the same where the
  provider keeps a store of resumable sessions it can read; Copilot and Pi are otherwise covered by
  FR-014.
- **FR-007**: Discovery MUST NOT list another project's sessions and MUST NOT list a session
  already in the catalog.
- **FR-008**: Users MUST be able to resume a listed session, and the provider MUST continue that
  conversation in the session's worktree. If that worktree is attachable but not attached, resuming
  attaches it first. Attaching a session only adds an idle catalog entry; it never starts the
  provider.
- **FR-009**: A missing, unreadable, or corrupt provider store or entry MUST NOT block opening the
  project; the bad entry is skipped and reported, the rest are listed.
- **FR-010**: Discovery and attaching MUST be read-only toward the provider store: Micold never
  modifies or deletes the provider's files.
- **FR-011**: The MCP tools MUST list resumable sessions through a new read-only tool, separate from
  `list_sessions` (which lists only the catalog's sessions), so an agent can find them without the
  app. Like `list_branches`, it is a discovery tool; its name and fields are for the plan.
- **FR-012**: When Micold starts with an empty catalog and finds attachable worktrees or resumable
  sessions, it MUST tell the user and offer attaching them in one action, and MUST NOT attach
  anything without the user's action (offer only, no automatic attaching).
- **FR-013**: The system MUST NOT break the existing behaviour that agent-created worktrees are
  hidden by default when they are not attached.
- **FR-014**: Discovery MUST work for each provider through the same user-visible behaviour; a
  provider with no store support MUST show no sessions and no error.
- **FR-015**: `attach_worktree` MUST be refused outright for a Default session, with a clear reason
  and nothing attached, because attaching changes a worktree's ownership and constitution
  Principle III allows a Default session only `create_worktree` and forbids it to modify a worktree
  by any means, the tool server included. Principle III is not amended.
- **FR-016**: Concurrent attaches of one worktree (two windows, or an agent and the app) MUST result
  in exactly one catalog entry, and a session MUST be resumed in at most one place at a time; a
  second resume request is refused with a clear reason.

### Key Entities

- **Attachable worktree**: a provider worktree present in git but not in the catalog; attributes:
  path, branch, provider, availability.
- **Resumable session**: a provider-store session for the project; attributes: provider, title or
  first prompt, last activity time, worktree, whether it is resumable.
- **Attach action**: the user or agent request that turns the above into catalog entries.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: After losing the data directory, a user with 16 provider worktrees sees all 16 in
  the sidebar within one attach action and under 10 seconds.
- **SC-002**: 100% of the project's resumable sessions in every readable provider store are listed, and 0 of other
  projects'.
- **SC-003**: Attaching a worktree leaves its branch, files and uncommitted changes byte-for-byte
  unchanged.
- **SC-004**: An agent re-attaches a provider worktree through the MCP tools in one call, with no
  delete-and-recreate.
- **SC-005**: A corrupt provider store entry never prevents a project from opening or any other
  session from being listed.

## Out of Scope

- Importing a provider's conversation history into Micold's own terminal history.
- Creating, deleting, or editing worktrees or provider session files.
- Attaching worktrees that are not under a provider's own worktree location.
- Syncing the catalog between machines.

## Assumptions

- Claude Code's store is `~/.claude/projects/<project>`; Copilot and Pi stores are located by the
  plan; if a provider keeps none that can be read, it shows none (FR-014).
- "Project" is identified by the repository's path, matching how the provider names its store.
- Attached worktrees become regular (not agent-owned) catalog entries; the hidden filter keeps
  applying to unattached ones.
- A resumed session uses the provider's own resume mechanism; Micold does not copy its history.
