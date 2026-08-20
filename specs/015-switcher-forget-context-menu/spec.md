# Feature Specification: Forget a Project from the Switcher's Right-Click Menu

**Feature Branch**: `feat/make-posibile-to-remove-a-project-also-from-project-switcher`

**Created**: 2026-07-23

**Status**: Draft

**Input**: User description: "allow to do right click on project switcher and delete/forgot the project"

## Context

Feature [014-forget-project](../014-forget-project/spec.md) already lets a user forget a project
— dropping Micold's remembered entry (name, worktree-name overrides, session records) while
leaving everything on disk untouched — but only from the **Known projects** list in the main
window.

This feature adds the **missing entry point**: the same forget action, reached by right-clicking
a row in the top-bar project switcher. It deliberately adds **no second forget path**; the menu
item hands off to feature 014's existing confirmation and removal flow, so the two entry points
can never diverge in behavior.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Forget a project without leaving the switcher (Priority: P1)

A user opens the project switcher to change projects and notices an entry that is finished or was
added by mistake. Rather than dismissing the switcher and hunting for the project in the main
window's Known projects list, they right-click the row directly and choose **Forget project**.
The familiar confirmation appears, and confirming removes the project exactly as it would from
the known-projects list.

**Why this priority**: This is the whole feature. The switcher is where users already go to manage
which project they are working on, so it is the natural place to curate the list.

**Independent Test**: With two or more projects, open the switcher, right-click a row, choose
Forget project, confirm, and verify the project is gone from both the switcher and the
known-projects list — and that its folder still exists on disk.

**Acceptance Scenarios**:

1. **Given** the switcher is open, **When** the user right-clicks a project row, **Then** a context
   menu appears offering **Forget project**.
2. **Given** that menu is open, **When** the user chooses Forget project, **Then** the existing
   forget confirmation opens naming that project, and the context menu closes.
3. **Given** the confirmation is open, **When** the user confirms, **Then** the project is
   forgotten exactly as it is from the known-projects list (same records dropped, nothing on disk
   touched).
4. **Given** the context menu is open, **When** the user clicks elsewhere or dismisses it,
   **Then** nothing is forgotten and no confirmation appears.

---

### User Story 2 - The menu behaves like a normal desktop context menu (Priority: P2)

The menu opens at the pointer, not in a fixed corner, and never opens off-screen — even when the
user right-clicks a row near the window's right or bottom edge.

**Why this priority**: A context menu that appears far from the cursor, or that runs off the
window, reads as broken. This is what makes the P1 interaction feel native rather than bolted on.

**Independent Test**: Right-click rows at various positions, including near the bottom-right
corner of a small window, and verify the menu is always at the pointer and always fully visible.

**Acceptance Scenarios**:

1. **Given** the switcher is open, **When** the user right-clicks a row, **Then** the menu's
   top-left corner is at the click point (opening below-right of the pointer).
2. **Given** a menu is open, **When** the user right-clicks a different row, **Then** the menu
   moves to the new click point.
3. **Given** the user right-clicks near the window's right or bottom edge, **Then** the menu is
   shifted back inside so it remains fully visible.
4. **Given** a menu is open near an edge, **When** the window is resized, **Then** the menu
   remains fully visible.

---

### Edge Cases

- **"Add project…" row**: it is an action, not a project, and MUST NOT offer a context menu.
- **Unavailable projects**: a project whose folder is missing MUST still be right-clickable and
  forgettable — it is the most likely candidate for removal.
- **One menu at a time**: opening a project's menu closes any other open popover, and opening any
  other popover closes it.
- **Switcher stays visible**: the switcher panel remains open behind the context menu, so the user
  can still see the row they right-clicked.
- **Window size unknown**: before the window reports its size, the menu opens unclamped at the
  cursor rather than being forced into a corner.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Users MUST be able to open a context menu by right-clicking a project row in the
  top-bar switcher.
- **FR-002**: That menu MUST offer a **Forget project** action for the right-clicked project.
- **FR-003**: Choosing it MUST hand off to feature 014's existing forget confirmation and removal
  flow — this feature MUST NOT introduce a second, parallel way to forget a project.
- **FR-004**: The menu MUST close when the user chooses an action, clicks outside it, or opens any
  other menu/panel; dismissing it MUST forget nothing.
- **FR-005**: The menu MUST be positioned at the pointer, with its top-left corner at the click
  point, and MUST re-position when a different row is right-clicked.

  > **Lifted to the kind, 2026-08-20 ([018](../018-material3-visual-system/spec.md)'s BUG-008).**
  > This requirement, and US2 above it, are scoped to the switcher's project rows — and the
  > sidebar's worktree and session menus, written before them, opened at a fixed corner for years
  > because nothing said the rule was about *context menus* rather than about this one. It now is:
  > **018's FR-029d** states it for every context menu opened from an element, with the press point
  > carried by the gesture, and **SC-008f** gates it over the set. Nothing here changes; what
  > changes is that this is no longer the only place it is written.
- **FR-006**: The menu MUST remain fully within the window, including when opened near an edge and
  when the window is resized while it is open.
- **FR-007**: Non-project rows (the "Add project…" affordance) MUST NOT offer a context menu.
- **FR-008**: The menu MUST be available for unavailable (missing-folder) projects.
- **FR-009**: The switcher panel MUST remain open behind the context menu.
- **FR-010**: Pointer tracking needed for positioning MUST NOT degrade idle performance when the
  switcher is closed.
- **FR-011**: User-facing documentation MUST describe the right-click route alongside the existing
  known-projects-list route.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can forget a project from the switcher in three interactions: right-click →
  Forget project → confirm.
- **SC-002**: The menu opens fully on-screen in 100% of right-clicks, at any pointer position and
  any window size.
- **SC-003**: Forgetting from the switcher and from the known-projects list produce identical
  results (same records removed, same on-disk no-op) in 100% of cases.
- **SC-004**: With the switcher closed, the feature adds zero additional per-frame or
  per-mouse-move work versus before the change.

## Assumptions

- **Feature 014 is the source of truth for forgetting.** Its confirmation copy, its removal
  semantics (including clearing the active working space when the active project is forgotten),
  and its persistence are reused as-is and are out of scope here.
- **Clamping, not flipping.** Near an edge the menu slides back inside rather than mirroring
  up-left. It stays adjacent to the click point; the trade-off is that it may sit under the
  pointer rather than beside it.
- **Right-click is the only new trigger.** The known-projects list keeps its explicit Forget
  button; no context menu is added there.

**Amended**: 2026-08-20 — 018's BUG-008 records that FR-005/US2 were lifted from this feature's switcher to the whole context-menu kind (018 FR-029d, SC-008f). No behaviour specified here changes.
