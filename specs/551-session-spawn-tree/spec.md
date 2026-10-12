# Feature Specification: Session Tree for Agent-Spawned Sessions

**Feature Branch**: `feat/551_show-sessions-spawned-by-an-agent-as-a-tree-under`

**Created**: 2026-10-12

**Status**: Draft

**Input**: User description: "Implement GitHub issue #551: Show sessions spawned by an agent as a tree under their origin session. When an AI CLI session (Claude, and the other providers) spawns a background agent or sub-agent to perform a task, that agent later shows up in the sidebar as a separate session started by the user under the same worktree. The user did not start it, and nothing shows which session it came from. Record the parent/child relationship when a CLI starts a session on behalf of another one; show child sessions nested under their origin session in the sidebar, as a tree, instead of as siblings; do this for every provider that can spawn background agents, each reporting the relationship through whatever mechanism it offers natively. Acceptance: a session spawned by another session appears as a child in the sidebar; a child is never shown as a top-level session started by the user; the relationship survives a daemon restart; a provider with no way to report the relationship keeps today's behaviour, and the gap is documented."

## Terms

- **Provider**: an AI CLI that Micold runs in a session: Claude Code, Copilot CLI, Pi, Codex or OpenCode.
- **Origin session**: a session in which a provider started another session on the user's behalf.
- **Child session**: a session that was started by a provider from an origin session, not by the user.
- **Session tree**: the sidebar listing of a worktree's sessions, where each child session is nested
  under its origin session.
- **Native signal**: information the provider itself reports about a session it started (a hook
  payload, an event, or session metadata). A guess from a name, a path or timing is not a native signal.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See an agent's spawned sessions under it (Priority: P1)

A user runs a session in a worktree. The session starts a background agent or sub-agent. In the
sidebar the new session appears nested under the session that started it, not beside it as if the
user had started it.

**Why this priority**: This is the whole problem in the issue: today the user cannot tell which
sessions they started and which an agent started, and cannot see where a session came from.

**Independent Test**: With a provider whose native signal names the origin, start a session that
spawns an agent and check the sidebar shows the new session as a child of the origin and not as a
top-level entry.

**Acceptance Scenarios**:

1. **Given** a running session A in a worktree, **When** A's provider starts a background agent
   that becomes session B, **Then** the sidebar shows B nested under A and not as a top-level
   session of the worktree.
2. **Given** session A with a child B, **When** B in turn starts session C, **Then** C is nested
   under B, and B stays nested under A.
3. **Given** a session the user started from the app, **When** it appears in the sidebar, **Then**
   it is a top-level session of its worktree, as today.
4. **Given** a child session, **When** the user selects it, **Then** it opens and behaves as any
   other session (output, input, status, stop), and the sidebar marks it as a child.

---

### User Story 2 - The tree survives restarts (Priority: P1)

A user closes the app or the daemon restarts. After it comes back, the same sessions are still
nested under the same origin sessions.

**Why this priority**: The relationship is recorded information. If it is lost on restart, children
reappear as the user's own top-level sessions, which is the defect the issue reports.

**Independent Test**: Build a tree of one origin and two children, restart the daemon, and compare
the sidebar tree before and after.

**Acceptance Scenarios**:

1. **Given** a tree with an origin and two children, **When** the daemon restarts, **Then** the
   sidebar shows the same tree.
2. **Given** a child session that was running when the daemon stopped and is now resumable,
   **When** the user lists the worktree's sessions, **Then** it is still nested under its origin.
3. **Given** a child whose origin session no longer exists in the catalog, **When** the tree is
   built, **Then** the child is shown as a top-level session marked "origin removed" (provisional, pending the stopped-or-deleted-origin clarification; an origin known to have been deleted, as
   against "origin pending" in FR-016), never dropped and never duplicated.

---

### User Story 3 - A provider that cannot report keeps today's behaviour (Priority: P2)

A user runs a provider that offers no native way to say which session started another. Its
sessions are listed exactly as today, and the user guide says which providers report the
relationship and which do not.

**Why this priority**: It bounds the promise. Guessing the relationship would mis-nest sessions, so
a gap must be an honest, documented gap.

**Independent Test**: Run such a provider's session, check it is top-level, and check the user
guide names the provider and the gap.

**Acceptance Scenarios**:

1. **Given** a provider with no native signal, **When** it starts a session, **Then** that session
   is listed top-level as today and nothing is nested by guesswork.
2. **Given** the user guide, **When** the user looks up session trees, **Then** it lists each
   provider as reporting or not reporting the relationship, and the nature of any gap.

---

### User Story 4 - Find and fold a busy tree (Priority: P3)

A user whose session spawned many agents folds the children away and expands them again, and sees
at a glance when a hidden child needs attention.

**Why this priority**: Useful once trees get large, but the nesting itself already delivers the
value.

**Independent Test**: Spawn several children, collapse the origin, check the children are hidden
and the origin shows whether any child needs attention, then expand.

**Acceptance Scenarios**:

1. **Given** an origin with children, **When** the user collapses it, **Then** its children are
   hidden and the origin shows how many there are.
2. **Given** a collapsed origin and a child that needs the user's attention, **When** the sidebar
   renders, **Then** the origin shows that a child needs attention.
3. **Given** a collapsed origin, **When** the app restarts, **Then** the origin is expanded
   (provisional default) [NEEDS CLARIFICATION: is the fold state remembered across restarts, or do trees start expanded each time?]

---

### Edge Cases

- A child appears before its origin is known to the catalog (events arrive out of order): the child
  is shown top-level with an "origin pending" marker until the origin is known, then moves under
  it; it is never lost and never duplicated (FR-016).
- The origin session is stopped or deleted while children remain:
  [NEEDS CLARIFICATION: do children stay in the tree under a stopped origin, get deleted with it,
  or move to the top level when the origin is deleted?]
- A session reports itself as its own origin, or two sessions as each other's: the relationship is
  rejected, the sessions are shown top-level and the daemon does not loop.
- A provider reports an origin id that is not a session of the same worktree and project: the
  relationship is ignored and the session is top-level.
- Many children (dozens) under one origin: the sidebar stays usable and does not reorder the
  user's own sessions.
- The same session reported twice, or with a different origin the second time: the first recorded
  origin stands; no duplicate entry appears.
- Several Micold windows or daemons observe the same worktree (Principle II): every one shows the
  same tree, and an update in one reaches the others.
- A session started by one session through Micold's own tool server (`create_session`):
  [NEEDS CLARIFICATION: does a session an assistant creates through the tool server count as a child of the calling session, or stay a user-level top-level session?]
- Linux, macOS and Windows: the tree and its persistence behave the same on all three (Principle VI).

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST record, for a session, the origin session that a provider reports
  having started it from, when and only when the provider reports it through a native signal.
- **FR-002**: The system MUST NOT infer an origin from a name, a path, a prefix or timing.
- **FR-003**: The sidebar MUST show a child session nested under its origin session, at any depth.
- **FR-004**: The sidebar MUST NOT show a session with a known, existing origin as a top-level
  session of its worktree.
- **FR-005**: A session without a recorded origin MUST appear exactly as today.
- **FR-006**: The recorded origin MUST persist across a daemon restart and an app restart, for
  running and resumable sessions alike.
- **FR-007**: A child session MUST support every action an ordinary session supports (open, input,
  stop, resume, delete), and MUST be visibly marked as a child.
- **FR-008**: A session whose origin is missing from the catalog MUST remain visible and MUST NOT
  be duplicated.
- **FR-009**: The system MUST reject a cyclic or self-referencing origin and a cross-worktree
  origin, leaving the session top-level.
- **FR-010**: An origin MUST be recorded once per session; a later report with a different origin
  MUST NOT replace it.
- **FR-011**: The user MUST be able to collapse and expand an origin's children, and a collapsed
  origin MUST show how many children it has and whether any needs attention.
- **FR-012**: Every window observing the same worktree MUST show the same tree.
- **FR-013**: The relationship MUST be available to clients of the daemon, so an assistant listing
  sessions can see each session's origin.
- **FR-014**: The user guide MUST list every provider with whether it reports the relationship,
  how it does so, and what a provider that cannot report it does instead.
- **FR-015**: Existing catalogs written before this feature MUST load unchanged, with every
  session top-level.
- **FR-016**: A child whose origin is not yet in the catalog MUST be shown top-level with an
  "origin pending" marker, and MUST move under its origin once the origin is known; an origin that never becomes known
  leaves the child top-level with the marker.

### Key Entities

- **Session**: an existing entity; gains an optional origin, which is the identity of another
  session of the same worktree.
- **Session tree**: a view of a worktree's sessions derived from the origins; it is not stored
  separately.
- **Provider reporting capability**: for each provider, whether and by what native signal it names
  the origin of a session it starts.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a worktree with N agent-spawned sessions from a reporting provider, 0 of them are
  shown as top-level sessions and 100% appear under their origin.
- **SC-002**: After a daemon restart, 100% of recorded origin relationships are shown unchanged.
- **SC-003**: For a provider that cannot report an origin, the sidebar listing is identical to the
  listing before this feature.
- **SC-004**: In the rendered tree, every child row carries the child marker and no top-level row
  that the user started carries it (checked by inspecting the rendered tree); orphaned children (origin pending or removed) carry
  the child marker too.
- **SC-005**: The user guide documents the reporting status of 100% of supported providers.

## Assumptions

- Claude Code is the first provider expected to report an origin (its lifecycle hooks already reach
  the daemon); whether each other provider has a native signal is established during planning, and
  the result goes in the user guide.
- The tree applies within one worktree: an origin and its children share a worktree.
- Removing name-based hiding of agent worktrees is the companion issue #552 and is out of scope
  here; this feature only supplies the session tree that work feeds.
- Out of scope: attaching a provider's existing sessions (feature 582), changing how sessions
  are started, and merging or reparenting sessions by hand.
- A session's origin is an immutable fact once recorded.
