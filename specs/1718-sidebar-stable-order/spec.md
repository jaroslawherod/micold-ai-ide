# Feature Specification: Sidebar keeps a stable order; state shows on the row, not by position

**Feature Branch**: `feat/1718_sidebar-stop-grouping-worktrees-sessions-by-state`

**Created**: 2026-11-06

**Status**: Closed 2026-11-10 — shipped in PRs #1750, #1782, #1758, #1791 and the close PR

**Input**: GitHub issue #1718 "Sidebar: stop grouping worktrees/sessions by state — rows jump between
groups and are hard to track". Labels: enhancement. Reverses the grouping of feature 1125
(`specs/1125-sidebar-state-groups`): `micold_core::sidebar_groups::StateGroup`, the sidebar's
`group_header` / `list_rows`.

## Terms

- **Location row**, **state group**, **group header**, **Overview row**, **chip bar**, **run group**:
  as defined by feature 1125 (`specs/1125-sidebar-state-groups/spec.md#Terms`).
- **Card state**: as defined by feature 813; the state a location's Overview card shows.
- **Stable order**: the order of the sidebar's location rows before feature 1125 put them in groups,
  that is the order `State::sidebar_entries` returns (Default entry first, then worktrees in their
  existing listing order). It does not depend on any session's or card's state.

## Clarifications

### Session 2026-11-06

No critical ambiguities. The plan verifies feature 905's icons differ per state by glyph or text (FR-004, SC-002).

## User Scenarios & Testing

### User Story 1 - Rows stay where they are when a session's state changes (Priority: P1)

A user runs several sessions. A session goes working, then waiting, then idle. Its row never moves,
so the user can find it and click it without it jumping away.

**Why this priority**: it is the whole problem the issue reports.

**Independent Test**: open a project with three worktrees, change the card state of the middle one
through every state; the list of rows (identity and index) is identical after each change.

**Acceptance Scenarios**:

1. **Given** a sidebar with several worktrees in different states, **When** one worktree's card state
   changes (to any other state), **Then** every row keeps its position.
2. **Given** the sidebar, **When** it is drawn, **Then** it shows no group header and no section
   between the Overview row / chip bar and the first location row.
3. **Given** a row under the pointer or selected, **When** states change, **Then** the hover, selection and
   keyboard focus stay on that same row, because no row moves.

---

### User Story 2 - State is still visible on every row (Priority: P1)

Each location row shows its card state at a glance, so nothing is lost by dropping the groups.

**Why this priority**: acceptance criterion of the issue; without it, removing groups hides state.

**Independent Test**: render rows for locations in each of the five card states; each row carries a
state indicator distinct per state, and the row's second line still reads `type/number · session state`.

**Acceptance Scenarios**:

1. **Given** locations in Needs action, Needs attention, Working, Done and Idle, **When** the sidebar
   is drawn, **Then** each row shows the indicator of its card state (the existing per-row state icon
   of feature 905 and the session-state text of feature 1125's row), distinguishable without colour
   alone.
2. **Given** a location whose state changes, **When** the sidebar redraws, **Then** that row's
   indicator updates in place.

---

### User Story 3 - Group state is removed with no residue (Priority: P2)

Rows that used to sit in a collapsed "Done" group are listed like any other, and keyboard navigation
and reveal work over the flat list. Group open/closed choices were held in memory only, so nothing
on disk needs migrating.

**Why this priority**: acceptance criterion: group headers and their open/closed state are removed.

**Independent Test**: with a worktree whose card state is Done, the sidebar list contains its row in
the stable order, and keyboard Up/Down steps over every row with no header stop.

**Acceptance Scenarios**:

1. **Given** a location whose card state is Done, **When** the sidebar is drawn, **Then** its row is
   listed in the stable order like any other.
2. **Given** the keyboard navigation of feature 803 and reveal of feature 024, **When** used,
   **Then** they step through and reveal location rows in the stable order, with no group header
   stop and no group to expand.

### User Story 4 - Rows that need me, or can go, stand out (Priority: P2)

A user scanning a stable list wants the rows that need their input to catch the eye, and the rows
whose work is merged (cleanup candidates) to be recognisable, without any row moving.

**Why this priority**: the position no longer says what needs attention, so colour must.

**Independent Test**: render rows in each of the five card states; Needs action and Needs attention
rows carry a louder container tint, Done rows a quieter tonal tint, Working and Idle rows none; the
row order is identical.

**Acceptance Scenarios**:

1. **Given** a location in Needs action or Needs attention, **When** its row is drawn, **Then** it
   has a loud Material container tint (`error_container` for Needs action, `tertiary_container`
   for Needs attention) under its text and glyph.
2. **Given** a location in Done (PR merged, branch verified removable, no running session), **When**
   its row is drawn, **Then** it has a quieter tonal tint (`surface_container_high`), keeps its Done glyph, and
   its tooltip state line reads "Done (merged)".
3. **Given** a row that is hovered or selected, **When** it is also tinted, **Then** the hover and
   selection state layers draw over the tint, and the selection is still plainly visible.
4. **Given** a row whose state changes, **When** the sidebar redraws, **Then** its tint changes in
   place and no row moves.

### Edge Cases

- A worktree whose PR was closed without merging stays Idle: no tint, not offered for cleanup
  (unmerged work may be the only copy; feature 813 never marks it removable).

- No project open: the list is empty, as before.
- Tag filter hides a worktree: it is not listed; the others keep their relative order.
- A run group (feature 483) stays one row with its runs nested; its position is stable, its state
  shows on its row.
- A location with no session: shows the idle indicator, order unchanged.
- Several windows or sessions changing state at once (Principle II): no row moves in any window.
- A worktree is added or removed while states change: the other rows keep their relative order.
- Keyboard navigation or reveal when the target row is in a previously collapsed group: the row is
  simply visible.
- Platform differences: none; the order and indicator logic is pure core logic shared by all platforms.

## Requirements

### Functional Requirements

- **FR-001**: The sidebar MUST list location rows in the stable order, independent of card state.
- **FR-002**: The sidebar MUST NOT draw state group headers or any per-state section.
- **FR-003**: A change of a location's card state MUST NOT change the position of any row.
- **FR-004**: Every location row MUST show its card state as a per-row indicator that is
  distinguishable by glyph, shape or text, not by colour alone, and keep its `type/number · session state` line.
- **FR-005**: Hover, selection, ripple, context menu, tag filtering, run groups and the Overview row's
  attention badge MUST behave as before feature 1125's grouping, apart from the groups.
- **FR-006**: Keyboard navigation (feature 803) and reveal-current-session (feature 024) MUST work over
  the flat list, with no group-header stop and no expanding of a collapsed group.
- **FR-007**: No group header, group toggle control or group open/closed state MUST be reachable in
  the app (pointer, keyboard or showcase).
- **FR-008**: The settings and state files MUST load as before; no new or changed stored field is
  required.
- **FR-009**: The docs, layout snapshot and showcase that describe state groups MUST describe the flat
  list instead.
- **FR-010**: A row in Needs action or Needs attention MUST be tinted with a Material container
  role louder than a Done row's; Working and Idle rows MUST carry no tint.
- **FR-011**: A Done row (the cleanup candidate: PR merged, branch removable, no running session,
  per feature 813's `CardState::Done`) MUST carry a quieter tonal tint, keep its Done glyph and
  show "Done (merged)" as its tooltip state line. No sixth card state is added, and a closed-unmerged PR stays Idle.
- **FR-012**: The tint MUST be colour only: it never changes a row's position (FR-003), never
  replaces the glyph (FR-004: state is never colour-only), sits under the hover and selected state
  layers, and keeps row text and glyph legible (4.5:1 text, 3:1 glyph).

### Key Entities

- **Location row**: one sidebar row (Default entry, worktree or run group) with its stable position
  and its card-state indicator.
- **Stable order**: the state-independent sequence of location rows.

## Success Criteria

- **SC-001**: In a test that cycles one location through all five card states, the row order is
  identical in 100% of the steps.
- **SC-002**: Every one of the five card states yields a row indicator distinct from the other four.
- **SC-003**: With a Done location present, the sidebar lists every location, in stable order.
- **SC-004**: No group header appears in the sidebar, the showcase or the docs.
- **SC-005**: Of the five card states, exactly Needs action, Needs attention and Done yield a row
  tint, the first two louder than Done; text and glyph on every tint meet 4.5:1 / 3:1 in light and
  dark themes.

## Assumptions

- The issue's recommended alternative 1 (per-row indicator, stable order) is what ships. Alternatives
  2 to 5 (attention count with jump, state filter, opt-in grouping, debounced reordering) are out
  of scope and may become follow-up issues.
- The stable order is the order `sidebar_entries` already returns; no new manual or alphabetical
  ordering is added.
- The existing per-row state icon (feature 905) and session-state text suffice as the indicator; the
  plan verifies that the five states differ by glyph or text, and adds one if not.
  (Plan finding: 905's icon has three glyphs, so a `CardState`-keyed icon supersedes it; research R1.)
- Group open/closed choices live in memory only (`group_choices`), so no migration is needed.
- The cleanup candidate is `CardState::Done`; no new data source or sixth state (decision of
  2026-11-09, research R7). Closing an unmerged PR does not make a row a candidate.
- Grouping is removed outright, with no opt-in setting.

## Out of Scope

- A manual (drag) order, alphabetical sorting, an attention count or jump-to-next, a state filter, an
  opt-in grouping setting, and debounced reordering.
- Changes to the Overview's own ordering or grouping.
- Follow-up issue (not in this feature): treat a closed-unmerged PR as a cleanup candidate; it needs
  its own data-safety design because the branch may hold the only copy of the work.
