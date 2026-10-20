# Feature Specification: Overview cards, full text without truncation

**Feature Branch**: `claude/project-thread-7041o6`

**Created**: 2026-10-20

**Status**: Draft

**Input**: User description: "Implement GitHub issue #1121: Overview cards: full text without
truncation. Blocked by: #1120. Title wraps to 3 lines and activity to 2. Hover or keyboard focus
expands the card in place, over its neighbours, with the full title, full branch, worktree dir and
full activity. Acceptance: expands on focus without a pointer; Escape or blur collapses; no clipped
text at 280 px; ripple, context menu and keyboard navigation unchanged." Labels: enhancement,
flow:feature. Builds on feature 1120 (`specs/1120-overview-card-readability`, shipped) and features
813 (`specs/813-worktree-overview`), 970, 971, 972 and 1024.

## Terms

- **Overview**, **card**, **location**, **card state**, **display name**: as defined by feature 813
  (`specs/813-worktree-overview/spec.md#Terms`).
- **Header row**, **body**, **title**, **short branch**, **sessions line**, **activity line**,
  **card width**: as defined by feature 1120 (`specs/1120-overview-card-readability/spec.md#Terms`).
- **Collapsed card**: a card as the grid lays it out, the only form a card has today.
- **Expanded card**: the same card drawn larger, in place and above its neighbours, showing every
  piece of its text in full. The grid's layout does not change while a card is expanded.
- **Worktree directory**: the folder on disk the location's sessions run in (a worktree's folder,
  or the project root for *Default*).
- **Clipped text**: text cut off by an edge so that part of a glyph or a line is not drawn, with no
  ellipsis to say text is missing. An ellipsis is not clipping.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Read more of a card without doing anything (Priority: P1)

A developer scans the Overview. Since feature 1120, a long issue title or a long activity line
ends in an ellipsis after one line, so the words that tell two similar worktrees apart are often
the ones hidden. With this change the title wraps onto up to three lines and the activity line onto
up to two before an ellipsis, so most titles and activities read in full on the collapsed card.

**Why this priority**: It helps every user on every glance, with no hover or key press.

**Independent Test**: Show cards with a one-word, a two-line, a three-line and a five-line title,
and a short and a long activity, at card widths of 280, 380 and 480 px, and check how many lines
each takes and where the ellipsis appears.

**Acceptance Scenarios**:

1. **Given** a card whose title needs two or three lines at its card width, **When** it is shown,
   **Then** the title wraps onto that many lines in full, with no ellipsis.
2. **Given** a card whose title needs more than three lines, **When** it is shown, **Then** it
   shows three lines, the third ending in an ellipsis.
3. **Given** a card whose activity needs two lines, **When** it is shown, **Then** the activity
   wraps onto two lines in full; needing more, it shows two lines, the second ending in an
   ellipsis.
4. **Given** a card whose title or activity fits on one line, **When** it is shown, **Then** it
   takes one line, as today.
5. **Given** a grid row in which one card's title wraps to three lines, **When** it is shown,
   **Then** every card in that row is as tall as that card (feature 1120 FR-014), and nothing
   overlaps.

---

### User Story 2 - See a card's full text by hovering it (Priority: P1)

The developer points at a card to learn which worktree it is. With this change the card expands in
place, drawn over its neighbours, and shows its full title, full branch name, worktree directory
and full activity. Moving the pointer off the card collapses it again.

**Why this priority**: It is the issue's core: the full text of a card, where the card is, without
leaving the Overview or reading tooltips one by one.

**Independent Test**: Hover a card with a long title, a long branch and a long activity in the
middle of a full grid, and check that it expands over its neighbours with every text in full, that
no other card moves, and that leaving it collapses it.

**Acceptance Scenarios**:

1. **Given** the Overview, **When** the pointer rests on a card, **Then** that card expands in
   place: its top-left corner and width stay where they are, it grows downward over the cards
   below it, and it shows the full title, the location's display name (when it differs from the
   title), the full branch name (when the title is not the branch), the worktree directory and the
   full activity, wrapped, with no ellipsis.
2. **Given** an expanded card, **When** it is shown, **Then** no other card moves or changes size,
   and the expanded card is drawn above every card it covers, with an elevation that sets it apart
   from them.
3. **Given** an expanded card, **When** the pointer leaves it (onto another card, the gaps, or out
   of the Overview), **Then** it collapses back to its grid form.
4. **Given** the pointer moving across the grid from card to card, **When** it rests on a new card,
   **Then** the previous card collapses and the new one expands: at most one card is expanded at a
   time.
5. **Given** a card near the bottom of the visible area, **When** it expands and would extend below
   the visible area of the Overview, **Then** it is shifted up just enough to be seen whole, still
   covering its own grid slot.
6. **Given** a card whose full content is taller than the visible area of the Overview, **When** it
   expands, **Then** it is as tall as the visible area, its header row and title stay in view, and
   the rest of its content scrolls inside the card: with the mouse wheel while the pointer is on it,
   and with Page Up / Page Down while it has keyboard focus (Story 3 scenario 6).
7. **Given** a card whose rows (sessions, progress, pull requests) are unchanged by expansion,
   **When** it expands, **Then** those rows read as on the collapsed card; only text that was
   wrapped short or ellipsized grows, and the lines of FR-004 that the collapsed card lacks are added.

---

### User Story 3 - See a card's full text from the keyboard (Priority: P1)

A keyboard user moves through the cards with Tab or the arrow keys (feature 813 FR-014). With this
change the focused card expands just as a hovered one does, so the full text is available without a
pointer. Escape collapses it; moving focus away collapses it.

**Why this priority**: The issue's first acceptance item: expansion must not depend on a pointer.

**Independent Test**: Open the Overview, press Tab and the arrow keys through the grid without
touching the pointer, and check that each focused card expands, that the previous one collapses,
and what Escape does.

**Acceptance Scenarios**:

1. **Given** the Overview open, **When** the user moves keyboard focus onto a card with Tab,
   Shift+Tab or an arrow key, **Then** that card expands as in Story 2, and the card that lost
   focus collapses.
2. **Given** a card expanded because it has keyboard focus, **When** the user presses Escape,
   **Then** the card collapses and keeps keyboard focus. [NEEDS CLARIFICATION: Today Escape closes
   the Overview (feature 813 FR-014). Should the first Escape only collapse a focus-expanded card,
   with a second Escape closing the Overview (recommended), or should one Escape both collapse the
   card and close the Overview, as today?]
3. **Given** a card expanded because it has keyboard focus, **When** focus leaves it (to another
   card, out of the grid, or because the window loses focus), **Then** it collapses.
4. **Given** a focused, expanded card, **When** the user presses Enter or Space, **Then** the card
   activates as today (feature 813 FR-013) with its ripple (feature 971).
5. **Given** a focused card that the user collapsed with Escape, **When** focus moves to another
   card and back, **Then** the card expands again.
6. **Given** a focused, expanded card whose content scrolls inside it (Story 2 scenario 6), **When**
   the user presses Page Down or Page Up, **Then** its content scrolls by about one visible height
   in that direction.

---

### User Story 4 - Everything else about the card behaves as before (Priority: P2)

Expansion adds a way to read a card; it must not change what a card does. Activation, the ripple,
the context menu, focus order, the target mark and the grid's layout stay as they are.

**Why this priority**: The issue's last acceptance item; a regression here would cost more than the
feature gives.

**Independent Test**: With a card expanded by hover and with one expanded by focus, click it,
press it with Enter, open its context menu with a secondary click and with the Menu key, and move
focus through the grid; compare each with the same action on a collapsed card before this change.

**Acceptance Scenarios**:

1. **Given** an expanded card, **When** the user clicks anywhere on it, including the part that lies
   over a neighbour, **Then** the expanded card activates (not the neighbour beneath), with its
   ripple over the card as it is drawn.
2. **Given** an expanded card, **When** the user opens its context menu by a secondary click or the
   Menu key / Shift+F10, **Then** the menu opens as feature 972 defines, the card shows the target
   mark over the whole of its drawn shape, and the card stays expanded while the menu is open.
3. **Given** a card menu open, **When** the pointer moves over other cards, **Then** no other card
   expands until the menu closes.
4. **Given** the keyboard focus order and arrow-key moves of feature 813 FR-014, **When** cards
   expand and collapse, **Then** the order and the moves are unchanged.
5. **Given** a grid row whose card expands, **When** it is shown, **Then** no row changes height and
   no card changes position (the expansion draws over the grid; it does not reflow it).
6. **Given** full-screen mode (feature 970), **When** a card expands, **Then** it behaves as in the
   normal Overview, and the zoom and its scroll anchor are unaffected.

---

### Edge Cases

- **Nothing more to show**: a card whose text all fits collapsed still expands (the worktree
  directory is only on the expanded card), so hover and focus behave the same on every card.
- **Card in the last column or last row**: it expands within its own column's width and is shifted
  up as Story 2 scenario 5 says; it never extends past the Overview's left or right edge.
- **Grid of one column** (pane narrower than two minimum-width cards, feature 1024): the expanded
  card has the column's width; nothing is cut at the right edge.
- **Card width 280 px**: in both forms no text is clipped (FR-007); a branch name or directory with
  no spaces wraps at any character rather than running past the card's edge.
- **Content changes while expanded** (the activity updates, an issue title arrives, the card state
  changes): the expanded card shows the new text and stays expanded; it collapses only on the
  triggers of FR-005.
- **Card goes away while expanded** (its worktree is removed): no card is expanded afterwards; a
  focused card that goes away moves focus as feature 972 FR-011 already defines, and the card that
  receives focus expands.
- **Overview opened**: opening the Overview expands no card by itself; a card expands only once the
  user hovers it or moves keyboard focus onto it.
- **Pointer on one card, focus on another**: one card is expanded at a time: the one the user
  interacted with last (pointer resting on a card, or a keyboard focus move).
- **Hover ends while another card has focus**: card A is expanded by focus, the pointer then
  rests on card B (B expands, A collapses) and leaves B without resting on another card: B
  collapses and A, which still has keyboard focus, expands again, unless the user collapsed A with
  Escape since it last received focus.
- **Window regains focus**: no card expands on that alone; the focused card expands again on the
  next focus move onto it, and a card under the pointer on the next pointer rest on it.
- **Escape on a card under the pointer**: Escape collapses the expanded card whichever way it was
  expanded; a card collapsed by Escape stays collapsed while the pointer stays on it, and expands
  again only when the pointer leaves it and rests on it again, or keyboard focus moves onto it
  again.
- **Scrolling the Overview while a card is expanded**: the expanded card moves with its grid slot,
  and the upward shift of FR-008 is worked out again for the new position; the card stays
  expanded while the pointer is still on it.
- **Scrolling under the pointer**: when scrolling the Overview brings another card under a resting
  pointer, the previous card collapses and the new one expands as for a pointer move.
- **Orphan / invalid location**: the expanded card shows the worktree directory as recorded and no
  branch line, as the collapsed card.
- **Project root (*Default*)**: the worktree directory is the project root's folder.
- **Tooltips**: the title, branch and activity tooltips of feature 1120 are no longer needed for
  full text, because expansion shows it; the state-reason and location-type tooltips stay.
- **Both themes**: the expanded card meets feature 1120 FR-003's contrast in every state, light and
  dark, and its elevation is visible in both.
- **Platforms and display scale** (Principle VI): expansion, wrapping and no-clipping hold on
  Linux, macOS and Windows in the bundled UI font, at display scale 1.0 and 2.0.
- **Reduced motion**: the card expands and collapses without an animation the user must wait for.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: On a collapsed card the title MUST wrap onto at most three lines and the activity line
  onto at most two; text that needs more MUST end in an ellipsis on the last allowed line. The short
  branch stays one line, ellipsized (feature 1120 FR-007).
- **FR-002**: A card MUST expand when the pointer rests on it, and when it receives keyboard focus
  (Tab, Shift+Tab, an arrow key, or a focus hand-back of feature 972 FR-011), with no pointer
  needed for the latter.
- **FR-003**: An expanded card MUST be drawn in place, above every other card, with an elevation that
  sets it apart: its top-left corner and width are those of its grid slot (shifted up only per
  FR-008), and its height is what its full content needs, up to the cap of FR-008.
- **FR-004**: An expanded card MUST show, in full and wrapped with no ellipsis: the title, the
  location's display name (when it differs from the title), the full branch name (when the location
  has one and the title is not the branch), the worktree directory, and the activity (when there is
  one); plus every other row the collapsed card shows. The full-text tooltips of the title, branch
  and activity on the collapsed card are removed, since expansion shows that text: this replaces
  the tooltip clauses of feature 1120 FR-006, FR-007 and FR-015. The state-reason and location-type
  tooltips of the header row stay.
- **FR-005**: An expanded card MUST collapse when the pointer leaves it (if it was expanded by
  hover), when keyboard focus leaves it (if it was expanded by focus), when the window loses focus,
  when the Overview closes, when another card expands, and when the user presses Escape while it has
  keyboard focus or the pointer on it. [See Story 3 scenario 2 for Escape's effect on the
  Overview.] When the card that collapses was expanded by hover and another card still has keyboard
  focus (not collapsed by Escape since it received focus), that card MUST expand again; no other
  collapse re-expands a card (Edge Cases).
- **FR-006**: At most one card MUST be expanded at any time.
- **FR-007**: No text on a card MUST be clipped, collapsed or expanded, at card widths of 280, 380
  and 480 px nor at any other width the grid produces; an unbroken run of characters wider than the
  card (a long branch or path) MUST wrap within the card on the expanded card and end in an ellipsis
  on the collapsed one.
- **FR-008**: An expanded card that would extend below the visible area of the Overview MUST be
  shifted up so it is seen whole, while still covering its own grid slot, and the shift MUST follow
  the card as the Overview scrolls. An expanded card is at most as tall as the visible area: content
  beyond that MUST scroll inside the card, with its header row and title kept in view, by the mouse
  wheel over the card and by Page Up / Page Down while it has keyboard focus. When the card's
  content does not overflow, is already at its end in the scroll's direction, or the card is
  collapsed, the wheel and Page Up / Page Down scroll the Overview as today.
- **FR-009**: Expanding or collapsing a card MUST NOT move, resize or reflow any card or grid row,
  and MUST NOT change the Overview's scroll position.
- **FR-010**: A pointer press, release or secondary click anywhere on an expanded card, including
  where it lies over a neighbour, MUST go to the expanded card, not to the card beneath.
- **FR-011**: Activation (feature 813 FR-013), the ripple (feature 971), the context menu and its
  target mark (feature 972), keyboard focus order and arrow-key moves (feature 813 FR-014), the
  focus ring, and full-screen mode (feature 970) MUST behave as today on a collapsed and on an
  expanded card; the ripple, the focus ring and the target mark MUST cover the card as it is drawn.
- **FR-012**: While a card menu is open, the target card MUST keep its expanded or collapsed form,
  and no other card MUST expand.
- **FR-013**: Opening the Overview MUST NOT expand a card by itself.
- **FR-014**: The worktree directory MUST be shown as [NEEDS CLARIFICATION: which form of the
  worktree directory — its full path with the home folder shortened to `~` (recommended), the path
  relative to the project root, or the folder name only?].
- **FR-015**: The expanded card MUST meet the contrast of feature 1120 FR-003 and the type scale of
  feature 1120 FR-004 in every card state and both themes, and MUST be built from the shared UI
  components with every colour and elevation from the theme's roles (Principle VIII); the
  component showcase MUST show a card collapsed and expanded.
- **FR-016**: The rules that decide which card is expanded (pointer, focus, Escape, menu open,
  Overview open or closed, card removed) and which text an expanded card shows MUST live in
  render-free code so they are unit-tested without the GUI (Principle I).
- **FR-017**: The user guide's Overview section MUST describe the wrapped title and activity, how a
  card expands on hover and on keyboard focus, and how it collapses, and the recorded layout
  snapshot MUST gain an Overview state with an expanded card (Principle VII).
- **FR-018**: Worktree paths, prompts and titles shown on the expanded card MUST stay on this machine
  (feature 813 FR-020) and MUST NOT be logged.

### Key Entities

- **Expanded card**: which card, if any, is expanded, and why (pointer resting on it, or keyboard
  focus); derived from pointer, focus and menu state, not stored across Overview openings.
- **Card text**: feature 1120's card text, plus the display name (when it differs from the title),
  the full branch name (when the title is not the branch) and the worktree directory for the
  expanded form.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With no pointer use at all, a keyboard user can bring every card's full text on screen:
  one focus move expands the card, and Page Down reaches any content beyond the visible area.
- **SC-002**: At card widths of 280, 380 and 480 px, in both forms and with every optional row, no
  text on any card is clipped and no text overlaps other text.
- **SC-003**: Expanding and collapsing a card moves no other card by even one pixel and changes no
  row's height.
- **SC-004**: A title of up to three lines and an activity of up to two lines at the card's width
  read in full on the collapsed card, with no ellipsis.
- **SC-005**: Activation, ripple, context menu, focus order and full-screen behaviour pass the same
  checks as before this change, on a collapsed and on an expanded card.
- **SC-006**: A card expands or collapses within 0.5 s of the trigger that causes it (a resting
  pointer, a focus move, Escape, the pointer or focus leaving).

## Out of scope

- Expanding more than one card at a time, pinning a card open, or a user setting for expansion.
- Changes to what the collapsed card shows beyond the title and activity line counts and the
  removal of its title, branch and activity tooltips (FR-004); feature 1120 stays as shipped
  otherwise.
- Changes to the grid's columns, card widths or row-height rule (features 1024, 1120).
- Expansion anywhere but the Overview.

## Assumptions

- "Hover" means the pointer rests on the card for a short moment (about the delay a tooltip uses,
  at most 0.5 s), so a pointer crossing the grid does not flash every card it passes; keyboard
  focus expands at once.
- The expanded card grows downward and keeps its grid slot's width, so it covers the cards below
  it, not those beside it; "in place" rules out widening it over the next column.
- The issue's "worktree dir" is the folder the location's sessions run in, so the project root is
  *Default*'s directory.
- The branch line of the collapsed card stays one line (feature 1120 FR-007): the issue names only
  the title and the activity for wrapping.
- Feature 1120's full-text tooltips on the title, branch and activity are dropped, because every
  way to reach them (hovering the card) now expands the card instead; the state-reason and type
  tooltips stay, as they say what no line shows.
- "Blur" in the issue means keyboard focus leaving the card, including the window losing focus.
