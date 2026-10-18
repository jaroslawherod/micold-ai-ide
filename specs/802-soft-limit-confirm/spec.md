# Feature Specification: Soft Session Limit - Confirm Before a Parallel Session

**Feature Branch**: `feat/802_make-one-session-per-worktree-a-soft-limit-confirm`

**Created**: 2026-10-18

**Status**: Draft

**Input**: User description: "Implement GitHub issue #802: Make one session per worktree a soft limit: confirm before creating a parallel session. Feature 776 hard-refuses creating a second session in a worktree that already has one, which blocks the legitimate case of the user wanting another session in the same worktree working in parallel. Make it a soft limit: when the worktree already has a session and the user creates another, tell them there is already a session and ask them to confirm; on confirmation the new session is created and works in parallel. Declining leaves the existing session untouched and creates nothing. Decide and document whether an agent-initiated create (no human present) needs the confirmation."

## Terms

Terms of feature 776 apply: **worktree**, **session**, **own session**, **additional session**, **main
session**, **helper session**. Added here:

- **Occupied location**: a worktree (or the Default project root) that has at least one
  un-archived session, in any state.
- **Parallel-session confirmation**: the question "this worktree already has session *X*; start
  another beside it?" with a confirm and a decline choice. It is the confirmation 776 FR-016
  already shows for the worktree row entry "Start another session...".
- **Person-initiated create**: a request that comes from a window, made by the user.
- **Agent-initiated create**: a request that comes from a tool call (`create_session`,
  `start_session`) made by an agent, with no person at that moment answering for it.

## Background

Feature 776 made one session per worktree a default, with an explicit request (the worktree row
entry plus a confirmation, or `allow_additional_session: true` on the tools) as the way past it.
Person-initiated routes to a create or start, and what they do today:

| Route | Today |
|---|---|
| Worktree row entry "Start another session..." | Already confirms (776 FR-016); regression test only. |
| Starting a stopped session blocked by a live sibling | Already confirms (776 FR-013, FR-016); regression test only. |
| The start action (default AI CLI, or the AI CLI chosen from its override list) on a location the window still believes free, because the session appeared after the row was drawn (for example created by an agent or another window) | Sends an unflagged create; the daemon refuses and the window shows an error. **This is the gap.** |

This feature turns that last route into the parallel-session confirmation, and records the
decision for agent-initiated creates.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Creating a session where one exists asks, it does not refuse (Priority: P1)

A user asks, from a window, for a new session in a worktree that already has one. Instead of an
error, the window tells them the worktree already has a session (naming it) and asks them to
confirm starting another beside it.

**Why this priority**: It is the whole request: the legitimate parallel-session case must not be
blocked by a refusal.

**Independent Test**: With a worktree that has a session, request a new session in it from a window
whose start action is stale; the confirmation opens, no error shows, and the catalog is unchanged
until the user answers. The test drives the start request after another source's session has
appeared in the window's catalog but before the row is redrawn (or sends the start request for an
occupied location directly).

**Acceptance Scenarios**:

1. **Given** a worktree with a session in any state, **When** the user requests a new session in
   it from a window (including one that still shows the start action from before the session
   existed), **Then** the parallel-session confirmation opens, naming the worktree and its
   existing session, and nothing is created yet.
2. **Given** the confirmation is open, **When** the user confirms, **Then** a new session is
   created in that worktree as a helper (776), is selected, and runs in parallel with the existing
   session, which keeps running untouched.
3. **Given** the confirmation is open, **When** the user declines (cancel, Escape, or dismiss),
   **Then** no session is created, the existing session is untouched (same id, state, process),
   and no error shows.
4. **Given** the user starts a stopped session that is blocked by a live sibling (776 FR-013),
   **When** they do so, **Then** the same confirmation opens (as in 776 FR-016) rather than an
   error.

---

### User Story 2 - Both sessions run in parallel after confirming (Priority: P1)

After the user confirms, the worktree hosts both sessions at once, each with its own process, and
both stay usable.

**Why this priority**: Confirming is only worth something if the second session truly runs
alongside the first.

**Independent Test**: Confirm a parallel session in a worktree with a running session; both list as
running in that worktree and each accepts input.

**Acceptance Scenarios**:

1. **Given** a worktree with a running session, **When** the user confirms a parallel session,
   **Then** the worktree lists two un-archived sessions, both running, the original still main.
2. **Given** two confirmed parallel sessions, **When** the user stops or deletes one, **Then** the
   other keeps running and stays usable (if the deleted one was main, 776 FR-022 promotes the
   other to main).

---

### User Story 3 - An agent's create has a defined, documented rule (Priority: P2)

An agent calling `create_session` or `start_session` has no person present to confirm. The rule
for it is decided once and written in the user guide and the tool catalogue.

**Decision**: an agent-initiated create on an occupied location does **not** open a confirmation
and is **not** auto-allowed: it keeps the 776 refusal (naming the existing session and
`allow_additional_session: true`), because nobody is present to answer and an unattended agent
silently doubling the sessions in one worktree is the harm 776 guards against. Setting
`allow_additional_session: true` is the agent's explicit, deliberate request and is honoured as in
776, so a legitimate agent-driven parallel session stays possible.

**Why this priority**: The issue requires the decision to exist and be documented; no behaviour
changes for agents.

**Independent Test**: Call `create_session` on an occupied worktree without the flag: refused with
the 776 error, no confirmation opens in any window. With the flag: created.

**Acceptance Scenarios**:

1. **Given** an occupied worktree, **When** an agent calls `create_session` without the flag,
   **Then** the call fails naming the existing session and the flag, nothing is created, and no
   window opens a confirmation.
2. **Given** an occupied worktree, **When** an agent calls it with `allow_additional_session:
   true`, **Then** the session is created and no confirmation is asked.
3. **Given** the user guide and the tool descriptions, **When** read, **Then** they state this
   rule and why.

### Edge Cases

- **Race: the session is deleted while the confirmation is open.** Confirming then creates the
  session on what is now a free location; the result is one session, main, not a helper.
- **Race: another session appears while the confirmation is open.** The confirmation names the
  session that existed when it opened; confirming adds beside whatever exists now. A new request
  shows the current state.
- **Two windows ask at once.** Each asks its own user; each confirmation that is accepted creates
  its own session (explicit requests each create their own, 776 FR-005).
- **Start of a location's own stopped session with nothing live** (776 FR-009), and restart or
  resume of it (776 FR-008a), ask nothing; this feature touches only creates.
- **Failure after confirming** (CLI missing, policy denies): fails as any create does; nothing is
  created and the existing session is untouched.
- **Daemon disconnected while the confirmation is open.** Confirming reports the usual connection
  error and creates nothing.
- **Default (project root) location**: behaves as a named worktree.
- **Platforms**: the confirmation is a window dialog, identical on every desktop platform.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: A person-initiated request for a session on an occupied location MUST open the
  parallel-session confirmation instead of failing with the 776 refusal. The confirmation MUST
  name the worktree and its existing session. Nothing is created before the user confirms.
- **FR-002**: Confirming MUST create the session as an additional (helper) session beside the
  existing one, select it, and leave the existing session untouched; both run in parallel.
- **FR-003**: Declining the confirmation by any means MUST create nothing and leave the existing
  session unchanged, without showing an error.
- **FR-004**: One confirmation MUST serve every person-initiated route in the table of
  Background: the worktree row entry and the blocked start (already so, 776) and the start request
  for a location that is occupied by the time it is made. No route shows the refusal text to a
  person in place of it. When several sessions exist (776 FR-013) the confirmation names the main
  one.
- **FR-005**: An agent-initiated create on an occupied location MUST keep the 776 refusal
  (FR-002, FR-003 of 776) unless it carries `allow_additional_session: true`, and MUST NOT open a
  confirmation in any window. With the flag it behaves as in 776 FR-015.
- **FR-006**: The user guide and the descriptions of `create_session` and `start_session` MUST
  state that a person is asked to confirm, that an agent is refused unless it sets
  `allow_additional_session: true`, and why.
- **FR-007**: A window never creates a second session in an occupied location before the user has
  confirmed; the unflagged create the daemon receives is still refused (776 FR-002).

### Key Entities

- **Parallel-session confirmation**: the question shown to a person, with the worktree name and the
  existing session's name; produces either a flagged create or nothing.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In 100% of person-initiated create requests on an occupied location, the user sees
  the confirmation and never the refusal text.
- **SC-002**: After confirming, the worktree lists both sessions running; after declining, the
  catalog is byte-for-byte as before.
- **SC-003**: A user starts a parallel session in at most one request and one confirmation.
- **SC-004**: 100% of unflagged agent creates on an occupied location are still refused with an
  error naming the existing session and the flag.

## Assumptions

- The wire protocol and the daemon's default-refuse rule do not change.
- Feature 776's mechanics (flag, helper marking, main session, row entry and its confirmation) are
  in place and are reused; this feature adds routes to that confirmation.
- Out of scope: showing agent-spawned sessions as a tree under their origin session (#551);
  asking a person on behalf of an agent (a tool-initiated confirmation in the app, as
  `stop_session` and `delete_session` do with `needs_confirmation`): not reused here because a
  person may not be present and `allow_additional_session` is already the agent's explicit
  opt-in; changing
  resume's replace-or-keep-both confirmation (776 FR-008b, FR-008c).
