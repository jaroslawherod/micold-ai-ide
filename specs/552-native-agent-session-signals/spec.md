# Feature Specification: Identify Agent Work From the AI CLI's Native Signals

**Feature Branch**: `feat/552_replace-name-based-hiding-of-agent-work-with-the`

**Created**: 2026-10-12

**Status**: Draft

**Input**: User description: "Implement GitHub issue #552: Replace name-based hiding of agent work with the AI CLI's native hooks. The app keeps agent-created work out of the user's view with heuristics: classifying worktrees by the `agent-` directory prefix and `worktree-agent-` branch prefix, plus the `show_agent_worktrees` sidebar toggle and the worktree provenance records from feature 029. These guess from names and miss cases, and do not address sessions that a background agent creates in an existing worktree. Remove the name-based hiding code and the toggle. Learn about sub-agent and background-agent sessions from the mechanism the CLI itself provides (for Claude, lifecycle hooks such as SubagentStart / SubagentStop with the session id and transcript path in each payload; for other providers the equivalent native hook, event or session metadata). Feed that into the session tree from the companion issue #551. Acceptance: no code classifies a worktree or session as agent-made from its name; sub-agent and background-agent sessions are identified only from provider-native signals; the related specs and docs (014-hide-agent-worktrees, 029-worktree-provenance, user guide) are updated or marked superseded."

## Terms

- **Provider**: an AI CLI that Micold runs in a session: Claude Code, Copilot CLI or Pi.
- **Native signal**: information the provider itself emits about a session it started on behalf of
  another one: a lifecycle hook payload, an event, or session metadata in its own store. A
  directory name, branch name or any other string pattern is not a native signal.
- **Agent session**: a session started by a provider on behalf of another session (a sub-agent or
  a background agent) rather than by the user.
- **Origin session**: the session on whose behalf an agent session was started.
- **Name-based hiding**: classifying a worktree as agent-made from its directory or branch name,
  the `agent-` / `worktree-agent-` convention of feature 014, and hiding it unless the "Show agent
  worktrees" toggle is on.
- **Worktree provenance**: feature 029's per-project record of which worktrees this app created,
  used to decide which worktrees are the user's and which are hidden.
- **Session tree**: the sidebar's nesting of child sessions under their origin session, delivered
  by issue #551.

## Clarifications

### Session 2026-10-12

- Q: With provenance gone, what becomes of feature 582's attach flow (remove it, drive it by "not in
  the catalog", or keep provenance as that record)? -> A: Attaching a worktree is removed. Its only
  durable effect was the provenance record that un-hid the worktree (`attach.rs`: "the one durable
  effect of attaching a worktree is the per-project provenance record"); with every worktree listed
  it has no effect, so the `attach_worktree` tool, the "Attach existing" worktree list and the
  `needs_worktree_attach` status go. What stays is 582's discovery and resume of stored provider
  sessions (FR-006 to FR-011 there), which never depended on provenance; its start-up offer is
  triggered by stored sessions the catalog lacks, not by missing provenance
  _(agent-resolved: crates/micold-core/src/attach.rs, specs/582-attach-provider-worktrees-sessions/spec.md#Requirements)_
- Q: Until #551's tree exists, are identified agent sessions shown flat or must this feature nest
  them? -> A: Shown flat among the worktree's sessions with the relationship recorded; nesting is
  #551's, and this spec already puts the tree out of scope and assumes #551 is "the display half"
  _(agent-resolved: specs/552-native-agent-session-signals/spec.md#Out of Scope; issue #551)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Every worktree is listed, with no name-based hiding (Priority: P1)

A user opens a project whose worktree directory holds worktrees named like an assistant's own
(`agent-<hex>` directories on `worktree-agent-<hex>` branches) beside worktrees they made. All of
them are listed like any other worktree. There is no "Show agent worktrees" control, and nothing in
the app decides what to hide by reading a name.

**Why this priority**: it is the removal the issue asks for, and everything else builds on the
sidebar no longer guessing.

**Independent Test**: open a project fixture holding worktrees with the reserved naming and ordinary
names; assert every one appears, that no toggle exists, and that renaming a worktree or branch to or
from the old convention changes nothing about its visibility.

**Acceptance Scenarios**:

1. **Given** a project with a worktree directory `agent-<17 hex>` on branch
   `worktree-agent-<17 hex>`, **When** the project is open, **Then** that worktree is listed
   exactly as an ordinary one is.
2. **Given** the sidebar, **When** the user looks for a control that reveals hidden agent
   worktrees, **Then** there is none, in the sidebar, in settings and in persisted preferences.
3. **Given** a worktree list, **When** a worktree is renamed (directory or branch) to match or
   stop matching the old convention, **Then** its visibility and ordering do not change.

---

### User Story 2 - Agent sessions are recognised from the provider's own signal (Priority: P1)

A Claude session starts a sub-agent or a background agent. The daemon learns of it from the
lifecycle hooks Claude itself posts once the daemon registers for them, with the agent's session id and the origin session's id, not
from anything it infers by name or timing. The relationship is recorded against the session so the
session tree (#551) can nest the child under its origin, and it survives a daemon restart.

**Why this priority**: it is the replacement for the heuristic; without it, removing the heuristic
leaves agent work indistinguishable from the user's.

**Independent Test**: post the provider's sub-agent start and stop hook payloads to the daemon's
hook receiver for a known session and assert that the session the payload names is recorded as an
agent session of that origin, with no name consulted; restart the daemon and assert the record is
still there.

**Acceptance Scenarios**:

1. **Given** a running Claude session, **When** the provider reports a sub-agent start naming an
   agent session and the origin, **Then** the daemon records that session as an agent session of the
   origin.
2. **Given** an agent session created in an existing worktree, **When** the report arrives,
   **Then** it is recorded as an agent session although its worktree and branch have ordinary names.
3. **Given** a recorded agent session, **When** the daemon restarts, **Then** the record is
   unchanged and still available to the session tree.
4. **Given** a sub-agent stop report, **When** it arrives, **Then** the daemon does not treat it as
   the end of the origin session's turn (the existing activity behaviour is unchanged).
5. **Given** a session reported by no native signal, **When** it appears, **Then** it is treated as
   a user session, never as an agent session.

---

### User Story 3 - Providers without a native signal keep working (Priority: P2)

A user runs Copilot or Pi. Where the provider offers a native hook, event or session metadata that
names an origin session, the daemon uses it. Where it offers none, sessions are shown as the user's
own and the gap is stated in the user guide, as the issue's companion (#551) requires.

**Why this priority**: the behaviour must be honest per provider; it extends US2 without blocking it.

**Independent Test**: for each provider, a table in the docs names the native signal used or states
the gap; a fixture session of a provider without a signal is listed as a user session and nothing
guesses otherwise.

**Acceptance Scenarios**:

1. **Given** a provider with a documented native signal, **When** it reports an agent session,
   **Then** the daemon records it the same way as for Claude.
2. **Given** a provider with no such signal, **When** it creates sessions, **Then** every one is a
   user session and the user guide lists the provider as not covered.

---

### User Story 4 - Superseded specs and docs say so (Priority: P3)

A maintainer reading features 014, 029 and 582 or the user guide is told which of their rules no longer
hold and which feature replaced them.

**Why this priority**: required by the acceptance criteria but carries no runtime behaviour.

**Independent Test**: grep the specs and user guide for the hide-toggle and provenance wording and
assert each hit is either gone or sits under a superseded marker naming this feature.

**Acceptance Scenarios**:

1. **Given** `specs/014-hide-agent-worktrees`, `specs/029-worktree-provenance` and `specs/582-attach-provider-worktrees-sessions`, **When** read,
   **Then** each starts with a superseded note naming this feature and what it replaced.
2. **Given** the user guide pages that describe hidden agent worktrees or the toggle, **When**
   read, **Then** they describe the shipped behaviour (all worktrees listed; agent sessions
   identified from provider signals) and name no toggle.

### Edge Cases

- **A report arrives for a session the daemon does not know**: it cannot occur, because the hook URL
  and bearer token belong to a known session; any other request is refused before its body is read,
  and nothing is recorded or created (plan research R1).
- **A payload with a missing or malformed agent id**: accepted without a transition, logged, and no
  relationship recorded (the receiver is loopback, best effort).
- **Concurrent agent sessions** (several sub-agents in one origin, in different worktrees or the
  same one): each is recorded independently; no report overwrites another's.
- **An agent run whose origin was deleted**: the runs are removed with their origin; a run is not a
  catalog session, so nothing is promoted to a user session.
- **Two daemons** (Principle II multi-session): a report about a session of one daemon never
  changes records of the other (FR-014).
- **Persisted state from before this feature** (hide toggle preference, provenance records and
  migration markers): loads without error, and the obsolete data is ignored or dropped; nothing
  becomes hidden because of it.
- **Cross-platform**: transcript and session paths in payloads are handled as opaque platform
  paths (Principle VI); no path-separator parsing decides anything.
- **An old session exists in a directory with the reserved name**: it is shown like any other.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The application MUST NOT classify any worktree, branch or session as agent-made, or
  show, hide or order it differently, from its directory name, branch name or any other string
  pattern.
- **FR-002**: The "Show agent worktrees" sidebar control, its persisted preference and the
  assistant-owned versus user-owned distinction for worktrees MUST be removed; every discovered
  worktree is listed.
- **FR-003**: The daemon MUST identify an agent session only from a provider-native signal: for
  Claude, the sub-agent and background-agent lifecycle hook events, which the daemon does not
  receive today and MUST begin to request from the provider; for other providers, their equivalent native hook, event or session
  metadata.
- **FR-004**: For each identified agent session the daemon MUST record the agent session's id and
  its origin session's id, and MUST make that relationship available to the session tree (#551).
- **FR-005**: The recorded relationship MUST survive a daemon restart.
- **FR-006**: A session for which no native signal reports an origin MUST be treated as a user
  session.
- **FR-007**: Handling of the sub-agent lifecycle hooks MUST NOT change the origin session's
  activity state: a sub-agent finishing is not the end of the origin's turn.
- **FR-008**: The daemon MUST ignore, without error, hook payloads that are malformed, and the
  receiver MUST refuse any request for a session it does not know, and MUST NOT let any such payload create a catalog entry or a
  user session. A valid payload for a known origin stores the relationship keyed by the agent
  session's id and, by itself, creates no catalog entry.
- **FR-009**: The worktree provenance records and their one-time migration MUST no longer decide
  what is visible, offered or hidden. Feature 582's worktree attaching MUST be removed with them:
  the `attach_worktree` MCP tool, the worktree rows of the attach dialog and banner, and the
  `needs_worktree_attach` resumable status (a session whose worktree exists is simply resumable).
  Discovery and resume of stored provider sessions MUST be kept; the start-up offer MUST be
  triggered by stored sessions the catalog lacks, not by absent provenance, and stays dismissable.
- **FR-010**: Persisted state written by earlier versions (hide preference, provenance records,
  migration markers) MUST load without error and MUST NOT hide anything.
- **FR-011**: While issue #551's session tree is not available, an identified agent session
  MUST remain visible to the user, shown flat among its worktree's sessions with the recorded
  relationship kept for #551 to use. This feature does not ship the nesting.
- **FR-012**: The user guide MUST state, per provider, which native signal identifies agent
  sessions or that none exists, and MUST describe the shipped behaviour with no mention of the
  toggle or name-based hiding.
- **FR-013**: Features 014 (hide-agent-worktrees), 029 (worktree-provenance) and 582
  (attach-provider-worktrees-sessions, whose agent-owned and toggle wording this removes), and any
  other spec whose rules this feature removes, MUST be marked superseded by this feature, naming what
  replaced it.
- **FR-014**: A report about one origin session MUST NOT alter the recorded relationships of any
  other session, including sessions of another daemon.

### Key Entities

- **Agent session record** (the plan's `AgentRun`): the provider's agent id and the origin Micold
  session that received the signal, reported by a provider-native signal, durable across restart.
- **Native signal**: a provider hook payload, event or session metadata item that names a session
  and its origin.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a project whose worktrees follow the old `agent-` / `worktree-agent-` naming,
  100% of its worktrees are listed and the app offers no control to reveal hidden ones.
- **SC-002**: Renaming any worktree directory or branch, to or from the former reserved
  convention, changes its listing in 0 of the tested cases.
- **SC-003**: For every sub-agent or background-agent session a covered provider reports, the
  session is recorded as an agent session of its origin; for a session no provider reports, zero
  are recorded as agent sessions.
- **SC-004**: After a daemon restart, 100% of previously recorded agent-session relationships are
  still present.
- **SC-005**: Every provider is listed in the user guide as covered by a named signal or as a
  documented gap, and each superseded spec carries its marker.

## Assumptions

- Issue #551 (nest child sessions under their origin) is the display half; this feature supplies
  the recorded relationship and does not design the tree.
- Claude's `SubagentStart` / `SubagentStop` hooks carry the origin's `session_id` and
  `transcript_path` plus the sub-agent's `agent_id` and `agent_type` (`SubagentStop` adds
  `agent_transcript_path`); a sub-agent shares its origin's session id and is not a separate
  Micold session, so "agent session id" below means `agent_id` and the origin is the Micold session
  that received the hook (plan research R1). Today the daemon registers neither (`SubagentStop` is
  deliberately omitted and classified Ignored); the plan adds the registration. Background-agent sessions that are not
  sub-agents are covered only if Claude offers a native event for them, to be checked at plan time.
- Copilot and Pi native signals are to be investigated at plan time; a provider with none is a
  documented gap, not a reason to guess.
- Worktrees outside the managed directory and inside it are treated alike; the "outside the managed
  root is always the user's" rule disappears with the distinction.

## Out of Scope

- The sidebar tree itself and its interactions (issue #551).
- Any new name- or timing-based heuristic as a fallback for providers without a signal.
- Changing how worktrees are created, named or deleted.
