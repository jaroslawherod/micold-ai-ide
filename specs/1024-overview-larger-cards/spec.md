# Feature Specification: Overview with larger cards, five per row

**Feature Branch**: `claude/project-thread-iq9j8q`

**Created**: 2026-10-18

**Status**: Closed 2026-10-18 — shipped in PRs #1033, #1041, #<close>

**Input**: User description: "Implement GitHub issue #1024: Overview should show larger cards, 5 per
row. The Overview view should lay out its cards larger, with 5 cards in each row. Expected: larger
cards, 5 per row. Actual: smaller cards, fewer/more per row than desired." Labels: enhancement,
flow:feature. Builds on feature 813 (`specs/813-worktree-overview`).

## Terms

- **Overview**, **card**, **location**: as defined by feature 813
  (`specs/813-worktree-overview/spec.md#Terms`). The Overview shows one card per location in a grid
  that takes the main pane.
- **Row**: one horizontal line of cards in the Overview's grid.
- **Columns**: how many cards a full row holds.
- **Main pane width**: the width the Overview is given, which is the window width less the sidebar
  (and changes when the window is resized or the sidebar is resized or collapsed).
- **Minimum card width**: the narrowest a card is ever drawn (280 px today, feature 813).
- **Pixel widths in this spec** (thresholds, edge cases, success criteria) are measured on the
  grid itself: the main pane width less the Overview's own padding around the grid. The rules are
  stated in main pane width; a number like "1464 px" or "2400 px" is the width the grid gets.

## Clarifications

### Session 2026-10-18

- Q: Does "larger" mean only wider (each card takes a fifth of the row), or also taller with more
  room for its content (a second summary line, larger text)? → A: Wider only; card height, text
  size and the one-line task summary stay as in feature 813, and the extra width shows more of each
  line before the ellipsis. _(agent-resolved: specs/813-worktree-overview/spec.md#Terms (task
  summary is one line); this spec#Out of scope and FR-010 (card content unchanged))_
- Q: On a pane too narrow for five cards, reflow to fewer columns or keep five per row and scroll
  sideways? → A: Reflow to as many columns as fit at the minimum card width, down to one; never
  scroll sideways. _(agent-resolved: specs/813-worktree-overview/spec.md#Edge Cases (Narrow
  window))_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Five cards per row on a wide pane (Priority: P1)

A developer with many worktrees opens the Overview on a large monitor. Today the grid fits as many
cards across as the pane allows, so a wide pane shows six, seven or more small cards per row. With
this change a full row holds five cards, and each card grows to share the pane's width, so the
cards are larger and easier to read.

**Why this priority**: It is the issue: five cards per row, each larger.

**Independent Test**: With twelve locations, open the Overview in a main pane 2400 px wide: the
first two rows hold five cards each, the third holds two, and every card is the same width, wider
than today's cards in the same pane.

**Acceptance Scenarios**:

1. **Given** a main pane wide enough for five cards at the minimum card width, **When** the user
   opens the Overview with at least five locations, **Then** every full row holds exactly five
   cards.
2. **Given** a main pane wide enough for six or more cards at the minimum card width (for example
   2400 px), **When** the user opens the Overview, **Then** a row still holds five cards, not more,
   and each card is wider than it would be with six or more per row.
3. **Given** twelve locations, **When** the user opens the Overview on a wide pane, **Then** the
   last row holds the two remaining cards, each the same width as the cards in the full rows above
   it, aligned to the left.
4. **Given** fewer than five locations (for example three), **When** the user opens the Overview on
   a wide pane, **Then** the cards keep the width they have in a five-card row; they do not stretch
   to fill the row.
5. **Given** the Overview open on a wide pane, **When** the user widens the window, **Then** the
   cards grow wider and each full row still holds five.

---

### User Story 2 - Larger cards that stay readable (Priority: P1)

The cards are larger than today's, so the name, branch, counts, state and task summary are easier
to read at a glance and less of the summary is cut short.

**Why this priority**: "Larger cards" is half of the issue's expected result.

**Independent Test**: Open the Overview on a 2400 px pane and compare a card with the same card
before the change: it is larger, and a long task summary that was cut short before
shows more of its text.

**Acceptance Scenarios**:

1. **Given** a main pane wide enough for six or more cards at the minimum card width (for example
   2400 px), **When** the user opens the Overview, **Then** each card is larger than today's card in
   the same pane; between the five-card and the six-card threshold today's grid already shows five
   per row, so there the cards are as wide as today's. "Larger" means wider: the card keeps feature
   813's height, text size and one-line task summary, and the extra width shows more of each line
   before the ellipsis.
2. **Given** cards of every card state, **When** the user looks at a row, **Then** every card in the
   grid has the same height, as today, so no row is ragged.
3. **Given** a card with a long name, branch or task summary, **When** it is shown, **Then** text
   that does not fit still ends in an ellipsis with the full text in a tooltip, as today; no text
   overlaps another part of the card or another card.

---

### User Story 3 - Narrow panes still work (Priority: P2)

On a laptop, or with a wide sidebar, the main pane is too narrow for five cards. The Overview stays
usable there.

**Why this priority**: The Overview must not break on smaller screens, but the issue is about the
wide case.

**Independent Test**: Shrink the main pane step by step from wide to 300 px and check that at every
width no card overlaps another, no card is narrower than the minimum card width, and every card
stays reachable by scrolling.

**Acceptance Scenarios**:

1. **Given** a main pane too narrow for five cards at the minimum card width, **When** the user
   opens the Overview, **Then** the grid reflows to as many cards per row as fit at the minimum card
   width, down to one, as feature 813's grid does; it never keeps five per row and scrolls
   sideways.
2. **Given** any pane width at or above the minimum card width, **When** the user opens the
   Overview, **Then** no card is drawn narrower than the minimum card width and no card overlaps
   another.
3. **Given** more locations than fit on screen, **When** the user scrolls the Overview, **Then**
   every card can be reached, and the order of cards (feature 813 FR-007) is unchanged.

---

### Edge Cases

- **Exactly at the threshold**: a pane exactly as wide as five minimum-width cards plus the gaps
  between them shows five per row; one pixel narrower shows four.
- **Very wide pane** (an ultrawide monitor, 3800 px or more): still five per row; the cards grow
  with the pane.
- **Sidebar collapsed or resized while the Overview is open**: the grid re-lays out at once to the
  new main pane width, with the same rules.
- **One location**: one card, as wide as a card in a five-card row, at the start of the first row.
- **Keyboard navigation**: focus moves between cards in card order, as it does today; the new layout changes where the cards sit, not the order focus visits them.
- **Component showcase**: the grid shown in the showcase follows the same layout rules.
- **Pane narrower than the minimum card width** (under 280 px): one card per row, drawn as wide as
  the pane, as today; this is the only case where a card is narrower than the minimum card width.
- **No locations**: the Overview shows feature 813's empty state, unchanged.
- **Cards changing while open** (sessions start and stop in several worktrees at once, Principle
  II): when a card's state changes and it moves position (feature 813 FR-007), the grid keeps five
  per row and the same card width; only the order changes.
- **Several windows** on different projects or monitors: each window's Overview lays out to its own
  main pane width.
- **Display scaling and platforms** (Principle VI): widths are measured in the same logical units
  on Linux, macOS and Windows, so a pane of a given logical width shows the same number of columns
  at any display scale factor.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The Overview grid MUST never show more than five cards in a row, whatever the main
  pane width.
- **FR-002**: When the main pane is wide enough for five cards at the minimum card width (including
  the gaps between them), every full row MUST hold exactly five cards.
- **FR-003**: When the main pane is too narrow for five cards at the minimum card width, the grid
  MUST reflow to as many cards per row as fit at the minimum card width, never fewer than one; the
  grid MUST NOT scroll sideways.
- **FR-004**: Cards in the grid MUST all have the same width, which is the main pane width shared
  equally by the columns of a full row less the gaps; the cards of a last, partial row and of a
  grid with fewer cards than columns MUST keep that width and sit at the start of the row.
- **FR-005**: Cards MUST be larger than feature 813's cards in the same pane wherever the pane is
  wide enough for more than five of them at the minimum card width. Larger means wider only: card
  height, text size and the one-line task summary stay as in feature 813.
- **FR-006**: Every card in the grid MUST keep one common height, the same for every card state and
  every content.
- **FR-007**: A card's text MUST NOT overlap other card parts or other cards at any card width the
  grid can produce; text that does not fit MUST end in an ellipsis, with the full text in a tooltip,
  as feature 813 requires.
- **FR-008**: The grid MUST re-lay out when the main pane width changes (window resize, sidebar
  resize or collapse) without the user closing and reopening the Overview.
- **FR-009**: Keyboard focus movement between cards (feature 813 FR-014) MUST keep visiting the cards
  in card order, as it does today, so it reads left to right along a row, then on to the next row.
- **FR-010**: Card order, card content, card states, activation and the attention badge (feature 813
  FR-001 to FR-020) MUST be unchanged.
- **FR-011**: The layout MUST be built in the shared card grid component (Principle VIII), so the
  Overview and the component showcase show the same layout.
- **FR-012**: The user guide's Overview section MUST say that the grid shows up to five cards per
  row and how it behaves on narrower windows (Principle VII).

### Key Entities

- **Card grid layout**: for a given main pane width, the number of columns (one to five) and the
  card width; derived, not stored.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: At every main pane width from the five-card threshold to 3840 px, a full row of the
  Overview holds exactly five cards.
- **SC-002**: At a main pane width of 2400 px, each card is at least 50% wider than a card of
  feature 813's layout at the same width (which fits eight cards per row there).
- **SC-003**: At every main pane width from 280 px to 3840 px, no card overlaps another card and no
  card text overlaps another part of its card.
- **SC-004**: With 30 locations, every card can be reached by scrolling and by keyboard at every
  width in SC-003.

## Out of scope

- A user setting for the number of cards per row.
- Changing what a card shows, its states or its order (feature 813).
- Two-dimensional arrow-key navigation (Up / Down to the card above / below).
- Any view other than the Overview.

## Assumptions

- The minimum card width stays at feature 813's 280 px; the gap between cards is unchanged.
- "5 per row" is a maximum: below the five-card threshold the grid keeps feature 813's reflow to
  fewer columns.
- No user setting for the number of columns is added; five is fixed.
- Only the Overview's grid changes; other views and the sidebar are untouched.
- The cards of a partial last row align to the start (left), as feature 813's grid does today.
