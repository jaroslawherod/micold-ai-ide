# Feature Specification: Worktree parent registration (MCP + sidebar) and supervisor support

**Feature Branch**: `docs/1857_design-worktree-parent-registration-mcp-sidebar`

**Created**: 2026-11-13

**Status**: Draft

**Input**: User description: "Worktree parent registration (MCP + sidebar) and supervisor support (issue #1857)"

## Background

The autopilot supervisor (epic #1817) creates worktrees and sessions through the micold tools, but
nothing records who created a worktree. A supervisor cannot tell its own runs from runs the user
started, and the user cannot see in the sidebar which worktrees an agent spawned. Today a session
may report a spawning *session* (`origin`, spec 551), but a session reference changes across
`/clear`, resume and restart, so it is unfit to own a run. A worktree outlives its sessions.

Terms: the **parent** of a worktree is the worktree whose session created it. The project root is
the worktree named `default`. A worktree with no recorded parent is **unparented** (created by the
user in the app, or before this feature).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A supervisor finds its own runs (Priority: P1)

A supervisor session, running in some worktree (or in `default`), creates worktrees for issues
through the micold tools. Later, and after a restart of itself, it asks which worktrees are its
runs and gets exactly those, never the user's own worktrees nor another supervisor's.

**Why this priority**: This is the reason for the feature; supervisor milestones M2, M3 and M5
(#1813, #1814, #1816) cannot scope their actions without it.

**Independent Test**: From a session in `default`, create two worktrees through the tool and one
by hand in the app; list worktrees and confirm the two carry parent `default` and the third carries
none.

**Acceptance Scenarios**:

1. **Given** a session running in the project root, **When** it creates a worktree without naming a
   parent, **Then** the new worktree's parent is `default`.
2. **Given** a session running in worktree `W`, **When** it creates a worktree without naming a
   parent, **Then** the new worktree's parent is `W`.
3. **Given** a session in any worktree, **When** it creates a worktree and names parent `P`
   explicitly, **Then** the parent is `P`, not the caller's worktree.
4. **Given** a worktree created by the user in the app, **When** worktrees are listed, **Then** it
   shows no parent.
5. **Given** worktrees with a parent, **When** a caller lists worktrees filtered by a parent,
   **Then** only that parent's children are returned.

---

### User Story 2 - The link survives renames, restarts and moves (Priority: P1)

A parent link keeps pointing at the same worktree when the parent or child is renamed, when the
app and its background service restart, and when the project is relocated on disk.

**Why this priority**: A link that breaks silently would make the supervisor treat its own runs as
foreign (or foreign runs as its own), which is worse than having no link.

**Independent Test**: Record a parent, rename the parent, restart the service; the child still
lists the renamed parent and is not marked removed.

**Acceptance Scenarios**:

1. **Given** child `C` with parent `P`, **When** `P` is renamed, **Then** `C` still has parent `P`
   (shown under `P`'s new name) with no removed marker.
2. **Given** a recorded parent, **When** the app and background service restart, **Then** the
   parent is unchanged.
3. **Given** a recorded parent, **When** the project folder is relocated and reopened, **Then** the
   parent is unchanged.

---

### User Story 3 - A removed parent is visible, not silent (Priority: P2)

When a parent worktree is deleted, its children keep the recorded parent and are marked as having a
removed parent, in the same way a session whose origin was archived is marked.

**Why this priority**: Supervisors need to recognise orphaned runs; users need to know why a
worktree's parent label no longer resolves.

**Independent Test**: Delete a parent that has a child; list worktrees and see the child with its
parent name and a removed marker, and no error.

**Acceptance Scenarios**:

1. **Given** child `C` with parent `P`, **When** `P` is deleted, **Then** `C` lists parent `P` with
   state `removed`.
2. **Given** a deleted parent, **When** a new worktree is later created with the same name as the
   deleted one, **Then** it is a different worktree and does not silently re-adopt the old children
   (see Edge Cases).
3. **Given** a child whose parent is `default`, **When** any other worktree is deleted, **Then**
   nothing changes for that child.

---

### User Story 4 - The user sees the hierarchy in the sidebar (Priority: P2)

In the sidebar, a worktree created by another worktree shows who its parent is and can be filtered
by parent, next to the existing issue tag and filter.

**Why this priority**: The data is only useful to the user if it can be seen; the supervisor works
without it.

**Independent Test**: With three worktrees created by a supervisor and two by hand, show the parent
label on the first three only, and filter the sidebar to a single parent.

**Acceptance Scenarios**:

1. **Given** a worktree with a parent, **When** the sidebar is shown, **Then** the row carries a
   label naming the parent; an unparented worktree carries none.
2. **Given** a worktree whose parent was removed, **When** the sidebar is shown, **Then** the label
   shows the parent name marked as removed.
3. **Given** the sidebar filter, **When** the user filters by a parent, **Then** only that parent's
   children remain visible, combined with any issue filter already set.
4. **Given** a parent that is renamed in the app, **When** the sidebar refreshes, **Then** the label
   shows the new name.

---

### User Story 5 - Sessions report the parent of their worktree (Priority: P3)

A caller listing sessions sees, for each session, the parent of the worktree it runs in, so a
supervisor can act on "sessions of my runs" without a second lookup.

**Why this priority**: Convenience; derivable from the worktree list.

**Independent Test**: List sessions; a session in a child worktree reports the parent and parent
state, a session in `default` reports none.

**Acceptance Scenarios**:

1. **Given** a session running in a child worktree, **When** sessions are listed, **Then** its row
   carries the worktree's parent and parent state.
2. **Given** a session in an unparented worktree or in `default`, **When** sessions are listed,
   **Then** the row carries no parent.

---

### Edge Cases

- A caller names an explicit parent that does not exist or is not a worktree of this project: the
  create call is refused with an error and no worktree is created.
- A caller names itself or a descendant as parent of an existing worktree: no cycle can form
  because the parent is fixed at creation and must already exist.
- A worktree deleted and recreated under the same name: the new one has no recorded parent and does
  not inherit children; children of the old one remain marked removed.
- A parent deleted outside the app (the directory removed by hand, or pruned by git): children are
  marked removed once the worktree no longer exists.
- A worktree created before this feature, or by the user: no parent; it is never assigned one
  implicitly.
- A session whose caller worktree cannot be determined: the worktree is created unparented, not
  failed.
- A child's own worktree is deleted: its record is dropped with it; nothing is left behind.
- Depth: parents of parents are allowed; no nesting limit applies to the link itself.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The system MUST record, for each worktree created through the tool by a session, the
  worktree the session runs in as its parent; a session in the project root records `default`.
- **FR-002**: The create call MUST accept an optional explicit parent that overrides the caller's
  worktree; it MUST accept only `default` or an existing worktree of the same project, and refuse
  anything else without creating a worktree.
- **FR-003**: A worktree created by the user in the app, or before this feature, MUST have no
  parent, and no later operation MUST assign one implicitly.
- **FR-004**: The parent MUST be fixed when the worktree is created. Changing it after creation is
  out of scope for this feature (see Assumptions).
- **FR-005**: The recorded parent MUST remain correct across a rename of the parent or the child,
  an app and service restart, and relocation of the project on disk.
- **FR-006**: When the parent worktree is deleted or no longer exists, the child MUST keep the
  recorded parent and report a removed state; it MUST NOT lose the link, be re-parented, or fail.
- **FR-007**: A new worktree with the same name as a deleted parent MUST NOT be treated as that
  parent (no re-adoption).
- **FR-008**: The worktree listing MUST return each worktree's parent and parent state (`known` or
  `removed`), omitting both for an unparented worktree, and MUST allow filtering by parent.
- **FR-009**: The session listing and single-session lookup MUST return the parent and parent
  state of the worktree a session runs in, omitting both when there is none.
- **FR-010**: The sidebar MUST label every worktree that has a parent with the parent's current
  name, mark a removed parent as such, and offer a filter by parent that composes with the existing
  issue tag and filter.
- **FR-011**: The parent link MUST be independent of the session `origin` link: both are kept,
  neither is derived from the other, and each keeps its own removed handling.
- **FR-012**: An agent MUST NOT be able to read or write a parent for a worktree of another
  project, and the feature MUST NOT widen what a caller may do to a worktree (the parent is
  information, not permission).
- **FR-013**: The user guide for the agent tools MUST describe the parent, its default, the
  override, the removed state and the filter, and the supervisor skills MUST be told to use the
  parent to find their runs and to leave runs with another parent alone.

### Key Entities

- **Worktree parent**: a reference from one worktree to the worktree (or `default`) whose session
  created it; fixed at creation; carries a state, `known` or `removed`.
- **Parent state**: whether the referenced worktree still exists (`known`) or not (`removed`).
- **Supervisor run**: a worktree whose parent is the supervisor's own worktree.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A supervisor lists its runs in one call and gets exactly the worktrees it created,
  with zero user-created worktrees and zero other supervisors' worktrees in the result, in every
  scenario of User Story 1.
- **SC-002**: In 100% of rename, restart and relocation scenarios of User Story 2, a recorded
  parent is unchanged and no child is marked removed.
- **SC-003**: Deleting a parent never produces an error or a lost link for any of its children;
  all are listed with the removed state.
- **SC-004**: A user can tell which worktrees an agent spawned, and from which parent, by looking
  at the sidebar, and can narrow the sidebar to one parent's children in at most two interactions.
- **SC-005**: The supervisor milestones #1813, #1814 and #1816 can scope every action to their own
  runs using only the listing result, with no transcript reading.

## Assumptions

- The identity of a worktree is its directory name under the project's agent worktree folder; a
  rename changes only the display name, so a link keyed on that identity needs no rewrite. A
  separate stable id is not needed.
- The parent is stored with the project's other per-worktree app state (next to display names), so
  it follows the project through restart and relocation. A git-config location is not used.
- Re-parenting is out of scope: the user's recourse is to delete and recreate the worktree. Revisit
  if a supervisor needs to adopt user-started runs.
- The parent is advisory. The supervisor honouring "never touch a run with another parent" is a
  rule of its skill, not an enforced permission.
- The sidebar presents the parent as a label and a filter, not as nesting of worktree rows under
  parent rows, because the sidebar already groups by state and keeps a stable order; nesting would
  conflict with both. Rows of a supervisor's runs stay in their state group.
- `origin` (the session that spawned a session, spec 551) is kept unchanged. `parent` answers
  "whose run is this worktree", `origin` answers "which session spawned this session".
- Out of scope: any supervisor behaviour (M2 to M5), a queue UI, and changing worktree deletion
  rules.
