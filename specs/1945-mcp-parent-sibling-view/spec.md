# Feature Specification: A session sees its parent, siblings and children, and messages its parent

**Feature Branch**: `claude/project-thread-x5s2f0`

**Created**: 2026-11-15

**Status**: Draft

**Input**: User description: "MCP: let a session see its parent session, parent worktree and siblings, and message its parent (issue #1945)"

## Background

Spec 1857 scopes what a session sees through the MCP tools by worktree parent: its own worktree,
its parent, its siblings and its direct children (FR-015). A worktree gets a parent only when a
session creates it through the `create_worktree` tool (FR-001); a worktree made in the app, or
before 1857, has none (FR-003) and its sessions see only themselves and their children. A session
reported: "I can't message a parent session from here. `list_sessions` shows only this session, and
mine has no origin, so no parent session is registered for me."

Terms: the **parent worktree** is the worktree named by the recorded parent link (spec 1857). The
**parent sessions** are the sessions running in the parent worktree. **Unparented**: a worktree
with no recorded parent. `origin` (spec 551) is the conversation a session was forked from and is
unrelated to the hierarchy.

Scope: the daemon and its MCP tools only; the iced app is being sunset and changes nothing here.

## Clarifications

### Session 2026-11-15

- Q: What do an unparented worktree's sessions see? → A: (jaro, 2026-11-15) An unparented worktree is treated as a child of `default` for visibility only. No link is recorded. Its sessions see `default`'s sessions and, as siblings, every other worktree whose parent is `default`, recorded or implicit. This amends spec 1857 FR-003 and FR-015 (last sentence) for visibility.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - An unparented session sees where it stands (Priority: P1)

A session running in a worktree the user made by hand asks which sessions it can see. It sees the
sessions in `default` (its implicit parent) and in the other hand-made worktrees (its siblings),
not only itself.

**Why this priority**: It is the reported failure; every other story depends on a non-empty view.

**Independent Test**: With two unparented worktrees `A` and `B` and sessions in `default`, `A` and
`B`, list sessions and worktrees as the session in `A`.

**Acceptance Scenarios**:

1. **Given** unparented worktrees `A` and `B`, **When** the session in `A` lists sessions, **Then**
   it sees its own, `default`'s and `B`'s.
2. **Given** the same state and a worktree `C` whose recorded parent is `A`, **When** the session in
   `A` lists worktrees, **Then** `C` is listed as its child, and `A` shows no recorded parent.
3. **Given** a worktree `D` whose recorded parent is `default`, **When** the session in `A` lists
   sessions, **Then** `D`'s sessions are visible: `D` and `A` share the parent `default`, recorded
   or implicit, so they are siblings.
4. **Given** a worktree `D` whose recorded parent is `default`, **When** the session in `D` lists
   sessions, **Then** it sees the unparented worktrees `A` and `B` and their sessions as siblings,
   but not their children. Siblings are mutual.
5. **Given** the view of scenario 1, **When** the session in `A` addresses a session in `B` through
   any tool, **Then** the tool accepts it as it does any visible session.

---

### User Story 2 - One call shows the whole neighbourhood (Priority: P1)

A session calls one new tool and gets its worktree, its parent worktree with the parent's state,
the sessions in the parent worktree, its sibling worktrees with their sessions, and its direct
children. When it has no recorded parent, the answer says so and says how the view is derived.

**Why this priority**: Today this takes several calls and the result does not say why a part is
empty.

**Independent Test**: Call the tool from a session in a parented worktree, from one in an
unparented worktree, and from one whose parent was deleted.

**Acceptance Scenarios**:

1. **Given** a session in worktree `W` with recorded parent `P`, siblings `S1`, `S2` and child `K`,
   **When** it calls the tool, **Then** the answer holds `W`, `P` with state `known`, `P`'s
   sessions, `S1` and `S2` with their sessions, and `K` with its sessions.
2. **Given** a session in an unparented worktree, **When** it calls the tool, **Then** the answer
   names `default` as the parent, marks it as implicit (no recorded parent), and lists as siblings
   every other worktree whose parent is `default`, recorded or implicit.
3. **Given** a session whose parent worktree was deleted, **When** it calls the tool, **Then** the
   parent is reported with state `removed`, no parent sessions, and no siblings.
4. **Given** a session in `default`, **When** it calls the tool, **Then** it gets its own
   worktree, no parent, and its direct children, which include the unparented worktrees, each marked
   implicit; the answer says `default` has no parent.
5. **Given** any caller, **When** it calls the tool, **Then** nothing outside its FR-015 scope
   appears in the answer.

---

### User Story 3 - Message the parent without knowing its session id (Priority: P1)

A child session sends text to its parent without looking up a session id. If the parent worktree
has several sessions the choice is deterministic; if it has none, the call is refused with a clear
reason.

**Why this priority**: It is the communication the issue asks for.

**Independent Test**: Send from a child whose parent has one session, several sessions and no
session; repeat the several-session case and compare the chosen session.

**Acceptance Scenarios**:

1. **Given** a parent worktree with one session, **When** the child sends to its parent, **Then**
   the text reaches that session, and the answer names the session reached.
2. **Given** a parent worktree with several sessions, **When** the child sends to its parent twice,
   **Then** both reach the same session, chosen by a documented rule (Assumptions).
3. **Given** a parent worktree with no session, **When** the child sends to its parent, **Then**
   nothing is sent and the refusal says the parent worktree has no session and names it.
4. **Given** no recorded parent, **When** an unparented session sends to its parent, **Then** the
   implicit parent `default` is the target (it is the same relation as in User Story 1).
5. **Given** a session in `default`, **When** it sends to its parent, **Then** the call is refused:
   `default` has no parent.
6. **Given** a parent whose state is `removed`, **When** the child sends to its parent, **Then**
   the call is refused and says the parent was removed.

---

### User Story 4 - A fast child finds the link already recorded (Priority: P2)

A supervisor creates a worktree with a session. The child session, whatever its first prompt does,
sees its parent from its first tool call.

**Why this priority**: Without it the P1 stories fail at random for freshly created runs.

**Independent Test**: Create a worktree with a session whose start is delayed after the worktree
exists; observe the parent link at the moment the session would start.

**Acceptance Scenarios**:

1. **Given** a create with a session, **When** the session's first prompt is typed, **Then** the
   parent link is already recorded.
2. **Given** a create with a session that fails before the worktree exists, **When** the call
   returns, **Then** no parent link is recorded (as in spec 1857).

---

### User Story 5 - Agents are told (Priority: P3)

The agent tools guide and the autopilot-supervisor skill describe the new tool, the messaging route
to the parent, and the implicit `default` parent.

**Why this priority**: Without the text, agents do not use the tool.

**Independent Test**: Read the guide and the skill for the three items.

**Acceptance Scenarios**:

1. **Given** the guide, **When** read, **Then** it describes the neighbourhood tool, the parent
   message route, and the implicit parent.
2. **Given** the supervisor skill, **When** read, **Then** it tells a worker to ask its parent
   through the parent route rather than looking for a session id.

### Edge Cases

- A recorded parent worktree that exists but whose id no longer matches (recreated): reported as
  `removed` (spec 1857 FR-007), like a deleted one.
- A worktree whose recorded parent is another worktree `X` is not a sibling of unparented
  worktrees; it is a sibling only of `X`'s other children and a child of `X`.
- An unparented worktree whose own children exist: it sees them (spec 1857) in addition to the
  implicit parent and siblings.
- Concurrency: two sessions send to the same parent at once; each delivery is one atomic input, the
  same as a send by session id. A parent session that ends between choice and delivery gives the
  existing "session gone" refusal, not a hang.
- The caller's own worktree has several sessions: the tool lists them all and marks the caller's.
- The project has no worktree besides `default`: the unparented-sibling set is empty.
- A session whose worktree is unknown to the daemon (not in the project): the view is unrestricted
  as today (spec 1857) and the tool reports no neighbourhood rather than failing.
- Platform: no platform-specific behaviour; the tools are the daemon's, identical on every OS.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: For visibility only, a worktree with no recorded parent MUST be treated as a child of
  `default`, in every tool that applies the spec 1857 FR-015 limit. No parent link MUST be
  recorded by this, and listings MUST keep omitting `parent` for it (spec 1857 FR-008). This amends spec 1857 FR-003 (visibility only; still no recorded link) and
  replaces the last sentence of its FR-015. Siblings are mutual: a caller whose recorded parent is
  `default` sees the unparented worktrees and their sessions as siblings, not their children. For a
  caller in `default`, the unparented worktrees are among its direct children.
- **FR-002**: A caller in an unparented worktree MUST see `default`, the other worktrees whose
  parent is `default` (recorded or implicit) and their sessions, its own worktree, and its direct
  children. It MUST NOT see a worktree whose recorded parent is another worktree.
- **FR-003**: A new tool MUST return, in one call: the caller's worktree and its sessions; the
  parent worktree, its state (`known`, `removed`) and whether the parent is recorded or implicit;
  the sessions in the parent worktree; each sibling worktree with its sessions; and each direct
  child worktree with its sessions.
- **FR-004**: The tool's answer MUST say when the caller has no recorded parent and that `default` is
  taken as its parent for visibility, and MUST say when the caller is in `default` and has no parent.
- **FR-005**: The tool MUST list nothing outside the caller's FR-015 scope as amended by FR-001.
- **FR-006**: A tool MUST send text to the caller's parent without a session id. The target is the
  earliest-created live session in the parent worktree, ties broken by the lowest session id;
  the answer MUST name the session reached.
- **FR-007**: The parent message tool MUST refuse, sending nothing, when the parent worktree has no
  session, when the parent was removed, and when the caller is in `default`; each refusal MUST state
  its reason and, where it applies, name the parent worktree.
- **FR-008**: The parent message tool MUST NOT reach a session the caller could not address by id,
  and MUST NOT widen what a caller may do (spec 1857 FR-012, FR-015).
- **FR-009**: The worktree parent MUST be recorded before the child's session starts, so the
  session sees the link from its first tool call. A failure that leaves no worktree MUST leave no
  link; a failure after the worktree exists MUST keep it (spec 1857).
- **FR-010**: The agent tools guide MUST describe the neighbourhood tool, the parent message route
  and the implicit `default` parent; the autopilot-supervisor skill MUST tell a worker to reach its
  parent through the parent route.

### Key Entities

- **Neighbourhood**: the caller's view: own worktree, parent (with state and recorded/implicit),
  parent sessions, siblings with sessions, children with sessions.
- **Implicit parent**: `default` as the parent of an unparented worktree, for visibility only.
- **Parent target**: the one session in the parent worktree a parent message goes to.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A session in an unparented worktree sees at least the `default` sessions and every
  other unparented worktree's sessions in one list call (was: only itself).
- **SC-002**: A session learns its parent, parent sessions, siblings and children in exactly one
  tool call.
- **SC-003**: A child reaches its parent in one call with no session id, in 100% of cases where the
  parent has a session; in 100% of the other cases the refusal names the cause.
- **SC-004**: For a create with a session, the parent link exists before the session's first prompt
  in 100% of runs, including with the session start delayed.
- **SC-005**: No call by a non-`default` caller reveals a worktree or session outside the amended
  scope (zero leaks across the visibility tests).

## Assumptions

- Several parent sessions: the earliest-created live session wins, ties by lowest session id (FR-006); the guide states it.
- The tool names are chosen in the plan; only the behaviour is specified here.
- Messaging reuses the existing input path of the send-input tool, so its policy checks, "session
  gone" refusal and atomic delivery apply unchanged.
- A session in `default` keeps seeing everything (spec 1857 FR-015); the amendment widens only the
  view of unparented worktrees.
- Out of scope: recording a parent for hand-made worktrees, re-parenting, any iced app change,
  relaying beyond the parent (spec 1857 FR-017 convention), a sibling message route.
