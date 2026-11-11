# Feature Specification: Sidebar user groups: create, name, assign and drag worktrees

**Feature Branch**: `feat/1786_feat-sidebar-let-the-user-create-groups`

**Created**: 2026-11-11

**Status**: Draft

**Input**: GitHub issue #1786 "feat: sidebar: let the user create groups". Labels: enhancement,
flow:feature. Related: #1718 (`specs/1718-sidebar-stable-order`), which removed grouping by state.

## Terms

- **Location row**, **Overview row**, **chip bar**, **run group**, **stable order**: as defined by
  feature 1718 (`specs/1718-sidebar-stable-order/spec.md#Terms`).
- **User group**: a named, user-created section of the sidebar that holds zero or more worktrees.
  Not an automatic grouping: only the user creates, fills, reorders and deletes it.
- **Group header**: the row that names a user group and shows its collapse control.
- **Ungrouped list**: the worktrees that belong to no user group, listed after the groups.
- **Drag**: picking up a worktree row with the pointer and dropping it on a target.
- **Insertion marker**: the line drawn while dragging that shows where the worktree will land.

## Clarifications

### Open questions for the clarify unit

- Run groups (feature 483): whether a run group row can be dragged into a user group is marked in
  FR-016.

## User Scenarios & Testing

### User Story 1 - Create, rename and delete a group (Priority: P1)

A user with many worktrees wants to organise them. They create a group in the sidebar, give it a
name, later rename it, and delete it when it is no longer useful.

**Why this priority**: without groups nothing else in this feature exists.

**Independent Test**: create a group, type a name, rename it, delete it; the sidebar shows the group
header after each of the first two steps and shows no header after the last, with the worktree list unchanged.

**Acceptance Scenarios**:

1. **Given** a project open in the sidebar, **When** the user chooses "New group" and enters a name,
   **Then** a group header with that name appears and holds no worktree.
2. **Given** a group, **When** the user renames it, **Then** the header shows the new name and its
   worktrees stay in it.
3. **Given** a group holding worktrees with running sessions, **When** the user deletes it,
   **Then** the header disappears, every worktree returns to the ungrouped list and no worktree or
   session is deleted or stopped.
4. **Given** the naming field, **When** the user enters an empty or blank name, to create or to rename,
   **Then** it is refused and the previous state stays.
5. **Given** a group named "Work", **When** the user creates or renames another group to "Work",
   **Then** it is refused with a visible message and the previous state stays.

---

### User Story 2 - Assign a worktree without dragging (Priority: P1)

A keyboard user, or anyone who prefers menus, opens a worktree's context menu and picks "Move to
group", then a group, or "Ungrouped".

**Why this priority**: the feature must be reachable by keyboard; it is also the simplest way to assign.

**Independent Test**: open the context menu on a worktree by keyboard, choose "Move to group" and a
group; the row now appears under that group.

**Acceptance Scenarios**:

1. **Given** a worktree row and at least one group, **When** the user opens its context menu and
   picks "Move to group" and a group, **Then** the row moves under that group's header.
2. **Given** a worktree in a group, **When** the user picks "Move to group" and "Ungrouped", **Then**
   the row moves to the ungrouped list.
3. **Given** no group exists, **When** the user opens "Move to group", **Then** it offers only "New
   group…", which creates the group and moves the worktree into it.
4. **Given** the keyboard only, **When** the user navigates to a row, opens the menu and completes the
   move, **Then** no pointer is needed.

---

### User Story 3 - Drag a worktree between lists (Priority: P1)

The user drags a worktree row onto a group (assign), onto another group (move) or onto the ungrouped
list (remove from its group). While dragging, the drop target is highlighted and an insertion marker
shows where the row will land. Its sessions follow it.

**Why this priority**: the core interaction the issue asks for.

**Independent Test**: drag a worktree from the ungrouped list onto a group, from there onto a second
group, then onto the ungrouped list; after each drop the row is in the target list at the marked place.

**Acceptance Scenarios**:

1. **Given** an ungrouped worktree, **When** the user drags it onto a group header or into a group's
   rows and drops, **Then** the worktree belongs to that group at the marked position.
2. **Given** a worktree in group A, **When** it is dropped on group B, **Then** it leaves A and belongs to B only.
3. **Given** a worktree in a group, **When** it is dropped on the ungrouped list, **Then** it belongs to no group.
4. **Given** a drag in progress over a valid target, **When** the pointer moves, **Then** the target is
   highlighted and an insertion marker shows the landing place; over an invalid place neither appears.
5. **Given** a dragged worktree with sessions, **When** it is dropped, **Then** its sessions are listed
   with it in the new place and keep running.
6. **Given** a worktree dropped onto a collapsed group, **When** the drop completes, **Then** the
   worktree belongs to that group and the group expands to show it.

---

### User Story 4 - Reorder within a list, and reorder groups (Priority: P2)

The user drags a worktree up or down inside its group or inside the ungrouped list, or drags a group
header among the other groups, to put things where they want them. The order stays until the user
changes it. Each of these also has a "Move up" / "Move down" menu entry for keyboard use.

**Why this priority**: the issue lists it, but groups work without it.

**Independent Test**: drag the last worktree of a list above the first; the list order is now last,
first, second, and stays so after the sidebar redraws.

**Acceptance Scenarios**:

1. **Given** a list of worktrees, **When** the user drops one at a marked position in the same list,
   **Then** it takes that position and the others keep their relative order.
2. **Given** several groups, **When** the user drops a group header at a marked position among the
   groups (or picks "Move up" / "Move down"), **Then** the group takes that position and its worktrees go with it.
3. **Given** a worktree or group, **When** the user picks "Move up" or "Move down" from its menu,
   **Then** it moves one place within its list.
4. **Given** a session whose card state changes, **When** the list redraws, **Then** no row moves
   (feature 1718 FR-003 still holds).

---

### User Story 5 - Cancel a drag (Priority: P1)

The user starts a drag and changes their mind: drops it outside any valid target, or presses Escape.
Nothing changes.

**Why this priority**: a drag that cannot be undone makes users afraid to try it.

**Independent Test**: start a drag, press Escape; start another, drop on empty space outside the
sidebar; the sidebar is identical to before in both cases.

**Acceptance Scenarios**:

1. **Given** a drag in progress, **When** the user presses Escape, **Then** the drag ends, the
   highlight and marker vanish and the worktree is where it was.
2. **Given** a drag in progress, **When** the user drops outside a valid target, **Then** the same.
3. **Given** a drag in progress, **When** the worktree is removed elsewhere (another window) before the drop, **Then** the drag ends with no change and no error.

---

### User Story 6 - Collapse groups and keep everything across restarts (Priority: P2)

The user collapses a group they are not working in and expands it later. Groups, their names,
order, worktree assignments, the order of worktrees and each group's collapsed state are there after
the app restarts.

**Why this priority**: groups the user must rebuild at every start are not usable.

**Independent Test**: create two groups, assign worktrees, reorder, collapse one, restart the app;
the sidebar looks as it did.

**Acceptance Scenarios**:

1. **Given** a group, **When** the user collapses it, **Then** its worktree rows are hidden and its
   header stays; expanding shows them again.
2. **Given** groups, assignments, order and collapsed state, **When** the app restarts, **Then** all
   are restored.
3. **Given** a persisted assignment to a worktree that no longer exists, **When** the sidebar loads,
   **Then** the stale assignment is ignored and everything else loads.

---

### User Story 7 - Groups do not break what 1718 fixed (Priority: P2)

User groups are the only grouping. Nothing groups, sorts or moves rows by session or card state; a
row's state still shows on the row.

**Why this priority**: the issue asks to settle how user groups and any automatic grouping interact.

**Independent Test**: with worktrees in two groups, cycle one through every card state; no row, in
any group or in the ungrouped list, changes position or group.

**Acceptance Scenarios**:

1. **Given** worktrees in user groups, **When** a card state changes, **Then** the worktree stays in
   its group and position, and its row indicator updates.
2. **Given** the sidebar, **When** it is drawn, **Then** every group header is a user group; no
   header names a state.

---

### Edge Cases

- No project open: no groups, no "New group" action.
- Two groups with the same name: refused (FR-004); never silently merged.
- A very long group name: the header shows it truncated and the full name is reachable (tooltip or rename field).
- An empty group: the header stays, with a visible empty state; it is a valid drop target.
- A drag started but the pointer never moves (a click): it selects the row as before; it is not a drag.
- A drop onto the row's own position: no change.
- The Default entry (the project's main checkout): it stays first and is not draggable or assignable.
  <!-- Assumption A2 -->
- Tag filter hides worktrees: hidden rows are not drop targets; group assignment and order of the hidden rows is kept.
- A worktree added by another window or on disk: it appears in the ungrouped list; a removed one leaves its group.
- Two windows change groups at once (Principle II): the last saved layout wins as a whole, and on
  load a worktree named by two groups is kept in one only; no worktree is lost or in two groups.
- Group state file unreadable or corrupt: the sidebar falls back to the ungrouped stable order and keeps the file's content for inspection; no crash.
- Platform differences: drag works with a mouse or trackpad on Linux, macOS and Windows; the "Move to group" menu is the path where drag is unavailable (Principle VI).
- Escape with no drag in progress keeps its existing meaning.

## Requirements

### Functional Requirements

- **FR-001**: Users MUST be able to create a user group from the sidebar and give it a name.
- **FR-002**: Users MUST be able to rename and to delete a user group from the sidebar.
- **FR-003**: Deleting a group MUST NOT delete or stop any worktree or session; its worktrees MUST return to the ungrouped list.
- **FR-004**: A group name MUST NOT be empty or blank; duplicate names MUST be refused with a visible message.
- **FR-005**: A worktree MUST belong to at most one group; a worktree with none MUST be in the ungrouped list.
- **FR-006**: Users MUST be able to drag a worktree row onto a group (assign), from one group to another (move), onto the ungrouped list (remove from its group), and within a list (reorder).
- **FR-007**: A worktree's sessions MUST be listed with it wherever it is dropped, and MUST keep running through the drag.
- **FR-008**: While a drag is in progress the sidebar MUST highlight the valid drop target under the pointer and draw an insertion marker where the worktree will land; over an invalid place it MUST show neither.
- **FR-009**: A drop outside a valid target, and the Escape key, MUST cancel the drag and leave every worktree where it was.
- **FR-010**: A worktree's context menu MUST offer "Move to group", listing every group, "Ungrouped" and "New group…", so assignment works without dragging and is reachable by keyboard (feature 803). A worktree's and a group's menu MUST also offer "Move up" and "Move down".
- **FR-020**: Users MUST be able to reorder groups by dragging a group header (with the same highlight, marker and cancel rules as FR-008 and FR-009) and by the menu of FR-010.
- **FR-011**: Groups MUST be collapsible and expandable, by pointer and by keyboard.
- **FR-012**: Groups, their names, their order, each worktree's assignment, the order of worktrees within each list and each group's collapsed state MUST persist across restarts.
- **FR-013**: User groups MUST be the only grouping in the sidebar: no group, order or row position depends on a session's or card's state; a row's state indicator stays (feature 1718 FR-001 to FR-004).
- **FR-014**: Groups MUST be kept per project, and creating, moving or deleting them MUST NOT alter any worktree, branch or session.
- **FR-015**: The Default entry MUST stay first and MUST NOT be assignable to a group.
- **FR-016**: Run groups (feature 483) MUST [NEEDS CLARIFICATION: may a run group row be dragged into a user group (its runs follow), or does it always stay in the ungrouped list?].
- **FR-017**: Hover, selection, ripple, context menu, tag filtering, keyboard navigation and reveal-current-session MUST keep working over the grouped list; reveal MUST expand a collapsed group that holds the target row.
- **FR-018**: A corrupt, missing or older stored layout MUST load without error as "no groups".

### Key Entities

- **User group**: id, name, position among groups, collapsed flag, ordered list of the worktrees it holds.
- **Assignment**: a worktree's membership in at most one group, and its position in that list.
- **Sidebar layout**: the stored whole (groups, assignments, ungrouped order) for one project.

## Success Criteria

### Measurable Outcomes

- **SC-001**: A user can create a named group and move a worktree into it in under 15 seconds, by drag and by menu.
- **SC-002**: In a test that applies every drag outcome (assign, move, remove, reorder, cancel), 100% leave each worktree in exactly one list, and every cancel leaves the sidebar looking and ordered exactly as before.
- **SC-003**: After a restart, 100% of groups, assignments, order and collapsed states are restored.
- **SC-004**: Deleting a group with N worktrees leaves all N worktrees and all their sessions present.
- **SC-005**: In a test that cycles one grouped worktree through all five card states, group and position are identical at every step.
- **SC-006**: Every action (create, rename, delete, move, reorder, collapse) is completed without a pointer.
- **SC-007**: The visual pass (quickstart Part B) records the layout, the drop highlight and the insertion marker in light and dark themes.

## Assumptions

- Groups are per project, as worktrees are (Principle III), and are stored with the app's own local
  state (Principle IV); no new remote or shared state.
- Groups are listed first, in the user's order (new groups append), and the ungrouped list follows. The Default entry
  stays on top. (A2)
- A new worktree lands in the ungrouped list, at its existing stable-order place.
- Ungrouped order is the stable order of feature 1718 until the user reorders; after that the saved
  order wins and new worktrees append.
- Interaction with #1718: no automatic grouping returns. User groups are explicit, so they do not
  reintroduce rows jumping; state still shows on each row.
- The docs, layout snapshot and showcase describe user groups (Principle VII).
- Nested groups, group colours or icons, and several groups per worktree are not part of this feature.

## Out of Scope

- Nested groups; one worktree in several groups; group colours, icons or per-group settings.
- Dragging sessions between worktrees (a session always follows its worktree).
- Reordering the Overview row, chip bar or Default entry.
- Sharing groups between machines or projects.
- Any automatic or state-based grouping (see #1718).
