# Feature Specification: Sidebar Filter Toolbar Button

**Feature Branch**: `feat/filtering-moved-to-dedicated-toolbar-button`

**Created**: 2026-07-17

**Status**: Draft

**Input**: User description: "the filtering by tags should be hidden and slide out when button with filter icon was pressed at sidebar toolbar"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Filters tucked away by default (Priority: P1)

A user working with several worktrees, most of them untagged or lightly tagged, opens the
app and sees the sidebar's worktree list without any filter chips taking up space above it.
The sidebar header shows a dedicated filter control on its left edge. When the user clicks it,
the tag filtering options expand open as an accordion panel below the header, pushing the
worktree list down; clicking the control again (or dismissing the panel) collapses it back to
the sidebar's clean, filter-free look.

**Why this priority**: This is the core of the request — the filter row is currently always
visible, permanently consuming sidebar space regardless of whether the user wants to filter.
Hiding it by default and only revealing it on demand is the primary value of this feature.

**Independent Test**: Can be fully tested by opening the app with tagged worktrees present,
confirming no filter chips are shown by default, then clicking the sidebar toolbar's filter
control and confirming the filter options appear in a panel.

**Acceptance Scenarios**:

1. **Given** the sidebar has worktrees with tags available for filtering, **When** the sidebar
   is shown, **Then** no filter options are visible and the worktree list occupies the space
   they previously used.
2. **Given** the filter panel is closed, **When** the user clicks the filter control in the
   sidebar header, **Then** the filter panel expands open below the header and displays the
   same filter options available today (by type, "has a Jira issue", "untyped").
3. **Given** the filter panel is open, **When** the user clicks the filter control again,
   **Then** the panel closes and the sidebar returns to its default (filter-free) appearance.

---

### User Story 2 - Knowing filters are active without opening the panel (Priority: P2)

A user has already narrowed the worktree list down using one or more tag filters, then closes
the filter panel to get it out of the way while they work. Later, they notice the list looks
shorter than expected and want to know why without having to reopen the panel to check.

**Why this priority**: Hiding the filter UI must not hide the fact that filtering is active,
or users will be confused about why worktrees are missing from the list. This is essential to
avoid a regression in clarity versus the always-visible chip row.

**Independent Test**: Can be fully tested by activating a filter, closing the filter panel, and
confirming the toolbar filter control visibly indicates that a filter is active, without
needing to reopen the panel.

**Acceptance Scenarios**:

1. **Given** one or more tag filters are active, **When** the filter panel is closed, **Then**
   the filter control in the sidebar toolbar visibly indicates that filtering is currently
   active.
2. **Given** no tag filters are active, **When** the filter panel is closed, **Then** the
   filter control shows its normal, inactive appearance.
3. **Given** the filter control indicates an active filter, **When** the user clears all
   filters from within the panel, **Then** the control's indication reverts to inactive.

---

### User Story 3 - Dismissing the filter panel naturally (Priority: P3)

A user opens the filter panel, decides they don't need to change anything, and wants to get
back to browsing worktrees without hunting for a specific close button.

**Why this priority**: Consistent, low-friction dismissal is expected of any transient panel in
the app and directly affects how usable the feature feels, but the app is still functional
without every dismissal path (e.g. Escape) — polish on top of the P1 behavior.

**Independent Test**: Can be fully tested by opening the filter panel and separately verifying
that pressing Escape and clicking the toolbar control again each close it.

**Acceptance Scenarios**:

1. **Given** the filter panel is open, **When** the user presses the Escape key, **Then** the
   panel closes.
2. **Given** the filter panel is open, **When** the user clicks the toolbar filter control
   again, **Then** the panel closes.

---

### Edge Cases

- What happens when no worktrees have tags yet (no filters would be available)? The filter
  control MUST still be present but MUST communicate there is nothing to filter, rather than
  opening an empty or broken-looking panel.
- What happens if the set of available filters changes (a worktree is added, renamed, or
  deleted) while the filter panel is already open? The panel MUST reflect the updated set of
  available filters without requiring the user to close and reopen it.
- What happens if the user closes the filter panel while filters are active? The active
  filters MUST remain applied and the worktree list MUST stay filtered; closing the panel is
  purely a display change, not a reset.
- What happens if the user clears all filters from inside the open panel? The worktree list
  MUST immediately show all worktrees again, and the toolbar control's active-filter
  indication MUST clear, while the panel itself MAY remain open.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The sidebar toolbar MUST provide a dedicated filter control, distinct from the
  other sidebar toolbar actions, whose purpose is to reveal or hide the tag filtering options.
- **FR-002**: The tag filtering options MUST be hidden by default and MUST NOT occupy sidebar
  layout space until the user reveals them via the filter control.
- **FR-003**: Activating the filter control MUST reveal the filtering options as an accordion
  panel that expands below the sidebar header, pushing the worktree list down, rather than the
  filtering options permanently occupying that space.
- **FR-004**: Activating the filter control a second time (or otherwise dismissing the panel)
  MUST hide the filtering options again and restore the sidebar's default appearance.
- **FR-005**: The filter control MUST visibly indicate whenever one or more tag filters are
  currently active, regardless of whether the filter panel is open or closed.
- **FR-006**: Users MUST be able to close the filter panel by pressing the Escape key or by
  activating the filter control again.
- **FR-007**: While the filter panel is open, users MUST retain the existing filtering
  capabilities: activating one or more tag filters (by type, by "has a Jira issue", or by the
  "untyped" bucket), and clearing all active filters in a single action.
- **FR-008**: Closing the filter panel MUST NOT change, clear, or otherwise alter any currently
  active tag filters; the worktree list MUST continue reflecting the last-applied filter
  selection.
- **FR-009**: When no tag filters are available (no worktrees carry tags yet), the filter
  control MUST remain present and MUST communicate that there is nothing to filter, rather
  than opening an empty or non-functional panel.
- **FR-010**: The filter panel MUST reflect the live, current set of available tag filters, so
  that changes to worktrees (added, renamed, or deleted) while the panel is open are reflected
  without requiring the user to close and reopen it.
- **FR-011**: All filtering behavior defined for the sidebar's tag filters (activating one or
  more filters with OR-logic matching, clearing all filters, and the empty-match state with a
  one-action way to clear the filter) MUST continue to function unchanged; only how these
  controls are shown to the user changes.

### Key Entities

- **Filter control**: The toolbar element a user interacts with to reveal or hide the tag
  filtering options; carries an active/inactive visual state reflecting whether any filter is
  currently applied.
- **Filter panel**: The transient, dismissible surface that displays the tag filtering options
  when revealed; hidden by default.
- **Filter selection (existing)**: The set of tag filters currently active, unchanged by this
  feature — only its visibility mechanism changes.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With filters inactive, the sidebar shows zero filter-related controls by
  default, giving the worktree list the full vertical space previously shared with the filter
  row.
- **SC-002**: A user can determine whether any tag filter is currently active by glancing at
  the sidebar toolbar, without opening the filter panel.
- **SC-003**: A user can reveal the filter panel, select or clear filters, and dismiss the
  panel using either of two dismissal methods (Escape, toggle button), each completing in a
  single action.
- **SC-004**: Existing filtering behavior (activating multiple filters, OR-matching, clearing
  all filters, empty-match messaging) produces identical worktree-list results before and
  after this change.

## Assumptions

- The existing tag-filtering logic (which filters exist, how they combine, how they're
  cleared) is unchanged by this feature; only how and when the filtering controls are
  displayed to the user changes.
- The filter control belongs in the sidebar's own toolbar/header (not the application's main
  top toolbar), on its left edge, since tag filtering is scoped to the sidebar's worktree list.
- The filter panel is presented as an inline accordion (expanding/collapsing in the sidebar's
  own layout, pushing the worktree list) rather than a floating panel over other content — a
  deliberate deviation from this app's other transient popovers (which float and dismiss on
  outside click), chosen because an accordion reads more naturally as part of the sidebar's own
  content than an overlay would.
- This feature does not introduce new filter types or change filter-matching logic; it is
  strictly a relocation of existing controls behind a toggle.
- When no tags exist anywhere yet, showing the filter control in a visibly inactive/disabled
  state (rather than hiding it entirely) is the reasonable default, so its position in the
  toolbar stays stable.
