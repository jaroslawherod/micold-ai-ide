# Feature Specification: Overview Card Context Menu

**Feature Branch**: `claude/project-thread-a1yb7j`

**Created**: 2026-10-18

**Status**: Draft

**Input**: User description: "Overview: right-click context menu on cards (GitHub issue #972).
Overview cards only respond to a primary click, so actions on a worktree/session need opening it
first. Right-clicking a card opens a context menu at the pointer with the card's actions (e.g. open,
rename, delete). It is also reachable from the keyboard (Menu key / Shift+F10) on the focused card,
dismisses on Esc or outside click, and the right-clicked card shows as the target. Alternative
considered: per-card overflow button only — discoverable but clutters every card."

## Terms

- **Overview**, **card**, **location**, **main session**: as defined by feature 813 (Worktree
  Overview). A card stands for one location: a worktree, an included worktree, an orphan / invalid
  worktree folder, or the project root (**Default** in the sidebar).
- **Row menu**: the right-click menu the sidebar already offers on that location's row (the worktree
  row's menu, or the Default row's menu for the project root), with the entries it offers today for
  that location's current state (for example *Review changes*, *Copy name*, *Rename*, *Stop
  showing*, *Open pull request*, the session entries, *Delete*).
- **Card menu**: the menu this feature adds to a card.
- **Target card**: the card a card menu was opened on.
- **Navigating entry**: a card menu entry whose result is shown in the window's main area in place
  of the Overview (*Open*, *Review changes*, an entry that starts or selects a session). Every other
  entry (copying, renaming, deleting, stop showing, opening a page in the browser, binding to a
  daemon) is an **in-place entry**: it acts, or opens a dialog, without leaving the Overview.

## Clarifications

### Session 2026-10-18

- Q: What happens to an open card menu when the Overview's grid scrolls beneath it (wheel, trackpad,
  scrollbar)? → A: The menu closes without running an entry, like every other non-modal floating
  surface. _(agent-resolved: specs/017-material-component-architecture/spec.md#FR-009;
  crates/micold-core/src/overlay.rs#dismisses)_
- Q: Where does keyboard focus go after an entry runs from a keyboard-opened card menu and the
  Overview stays open (an in-place entry, or its dialog closing)? → A: Back to the target card when
  it is still listed; otherwise to the Overview's first card. _(agent-resolved:
  specs/972-overview-card-context-menu/spec.md#FR-011; specs/813-worktree-overview/spec.md#FR-014)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Act on a location from its card with the pointer (Priority: P1)

The user has the Overview open, sees a card they want to rename, delete or review, and right-clicks
it. A menu opens where they clicked, listing what they can do to that location; they pick an entry
and it happens without first opening the location.

**Why this priority**: This is the issue's core: today every action needs the user to leave the
Overview, find the location in the sidebar and act there.

**Independent Test**: Open the Overview, right-click a worktree card, choose *Rename*: the rename
dialog for that worktree opens over the Overview, and after renaming the card shows the new name.

**Acceptance Scenarios**:

1. **Given** the Overview open, **When** the user right-clicks a worktree card, **Then** a menu
   opens at the pointer listing *Open* first, followed by exactly the entries that worktree's row
   menu offers at that moment, in the row menu's order.
2. **Given** the Overview open, **When** the user right-clicks the project root's card, **Then** the
   menu lists *Open* first, followed by exactly the Default row menu's entries.
3. **Given** a card menu open, **When** the user chooses *Open*, **Then** the menu closes and the
   result is the same as a primary click on the card (feature 813, FR-013): the Overview closes and
   the location's main session is shown, or a session start is offered, or the location is
   revealed.
4. **Given** a card menu open, **When** the user chooses any other entry, **Then** the result is
   the same as choosing that entry from the location's row menu, for the target card's location.
5. **Given** a card menu open, **When** the user chooses an in-place entry, **Then** the Overview
   stays open (with any dialog the entry opens shown over it), and the card reflects the result
   within the Overview's normal refresh (for example a renamed card shows its new name, a deleted
   worktree's card disappears).
6. **Given** a card menu open, **When** the user chooses a navigating entry other than *Open*,
   **Then** the Overview closes and the entry's result is shown, as it is when chosen from the row
   menu.
7. **Given** the Overview open, **When** the user primary-clicks a card, **Then** it activates as
   before; this feature does not change the primary click.

---

### User Story 2 - See which card the menu belongs to, and dismiss it (Priority: P1)

While the menu is open the right-clicked card is visibly marked as its target, so in a dense grid
the user knows which location the entries will act on. Escape or a click anywhere outside the menu
closes it without doing anything.

**Why this priority**: Acting on the wrong worktree (rename, delete) is costly; and a menu that
cannot be dismissed safely blocks the Overview.

**Independent Test**: Right-click a card, move the pointer over the menu and off the card: the card
keeps its target mark; press Escape: the menu closes, the mark goes, the Overview is still open.

**Acceptance Scenarios**:

1. **Given** a card menu open, **When** the user looks at the grid, **Then** the target card shows a
   target mark that no other card shows, distinct from hover, and it stays while the pointer is over
   the menu or anywhere else.
2. **Given** a card menu open, **When** the user presses Escape, **Then** the menu closes, no entry
   runs, the target mark goes, and the Overview stays open.
3. **Given** a card menu open, **When** the user clicks outside the menu (on another card, on empty
   Overview space, or elsewhere in the window), **Then** the menu closes, no entry runs, and the
   click does not also activate what it landed on.
4. **Given** a card menu open on card A, **When** the user right-clicks card B, **Then** the menu
   for card A closes and a menu for card B opens at the new pointer position, with B as the target.

---

### User Story 3 - Open the card menu from the keyboard (Priority: P2)

A keyboard user focuses a card (feature 813, FR-014) and presses the Menu key or Shift+F10. The
same menu opens at the card, its first entry focused; they move with the arrow keys and choose with
Enter.

**Why this priority**: The Overview is already keyboard-operable; without a keyboard way in, its
actions would be pointer-only. It is P2 because the pointer path delivers the issue's value first.

**Independent Test**: Open the Overview, Tab to a card, press Shift+F10, press Down then Enter: the
second entry runs for that card.

**Acceptance Scenarios**:

1. **Given** a focused card, **When** the user presses the Menu key or Shift+F10, **Then** the card
   menu opens at the card (inside the card's area, not at the pointer), with that card as the target
   and the menu's first entry focused.
2. **Given** a card menu opened from the keyboard, **When** the user presses Up or Down, **Then**
   focus moves between entries, wrapping from the last to the first and back; **When** they press
   Enter or Space, **Then** the focused entry runs as in User Story 1.
3. **Given** a card menu open, **When** the user presses Escape, **Then** the menu closes and
   keyboard focus returns to the target card, not to the start of the Overview.
4. **Given** a menu opened by right-click, **When** the user presses Up or Down, **Then** the same
   keyboard navigation works on it.
5. **Given** no card focused, **When** the user presses the Menu key or Shift+F10 in the Overview,
   **Then** no card menu opens.

---

### Edge Cases

- **Menu near the window edge**: a card menu opened near the right or bottom edge stays fully inside
  the window, as the row menus do.
- **Small window / one-column grid**: the menu still fits; when it cannot fit below the anchor it
  opens where it fits, never off-screen.
- **Target card disappears** while its menu is open (the worktree is deleted elsewhere, or by
  another window — Principle II, several sessions and windows): the menu closes and no entry runs on
  a location that no longer exists.
- **Target card changes** while its menu is open (its state, sessions or pull request change): the
  menu stays open on the same location; the entries chosen still act on that location; the entries
  offered follow the row menu's rules for the location's current state the next time the menu opens.
- **Cards reorder** while the menu is open (feature 813 orders cards by state): the target mark
  follows the target location, not a grid position.
- **Grid scrolls** while a card menu is open (wheel, trackpad, scrollbar): the menu closes and no
  entry runs (feature 017, FR-009).
- **Overview closes** while a card menu is open (shortcut, top bar button, project switch): the menu
  closes with it, and no entry runs.
- **Orphan / invalid card**: the menu offers *Open* (which reveals the location, as activation does)
  and whatever the row menu offers for that location; entries the row menu does not offer for it
  are absent, not shown disabled.
- **Several sessions in one location**: the session entries are the ones the row menu offers for
  that location; *Open* opens the main session, as activation does.
- **A stale entry**: an entry chosen after the location's state changed under the open menu (for
  example *Open pull request* once the pull request is gone) behaves as the same entry does when
  chosen from a row menu that went stale.
- **A dialog opened from the menu is cancelled** (rename, delete confirmation): the Overview is
  still open and unchanged.
- **An entry fails** (rename or delete refused): the same error the row menu's entry shows is
  shown, and the Overview stays open.
- **No card to target**: the Overview always has at least the project root's card (feature 813);
  a secondary click on empty Overview space opens no menu.
- **Shift+F10 and the Menu key elsewhere**: outside the Overview these keys keep doing whatever they
  do today; in the Overview with a card focused they open its menu and are not passed to a terminal.
- **Cross-platform (Principle VI)**: the secondary click is the platform's (right-click, or
  Ctrl+click / two-finger click on macOS as the system reports it); Shift+F10 works on Linux,
  Windows and macOS; the Menu key works where the keyboard has one; the menu looks the same on all
  three and in light and dark themes.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: A secondary click on a card in the Overview MUST open a card menu anchored at the
  pointer position, kept fully inside the window.
- **FR-002**: The card menu MUST list *Open* first, followed by exactly the entries the target
  location's row menu offers at the moment it opens, in the row menu's order and with the row menu's
  labels and icons; an entry the row menu does not offer MUST NOT appear.
- **FR-003**: Choosing *Open* MUST have the same effect as activating the card (feature 813,
  FR-013).
- **FR-004**: Choosing any other entry MUST have the same effect as choosing that entry from the
  target location's row menu, including any confirmation it asks for.
- **FR-005**: Choosing an in-place entry MUST leave the Overview open, with any dialog it opens shown
  above the Overview; choosing a navigating entry MUST close the Overview and show its result.
- **FR-006**: While a card menu is open, the target card MUST show a target mark that no other card
  shows, distinguishable from hover and focus without relying on colour alone, in light and dark
  themes; the mark MUST go when the menu closes.
- **FR-007**: Escape MUST close an open card menu without running an entry and without closing the
  Overview.
- **FR-008**: A primary or secondary click outside an open card menu MUST close it without running an
  entry; a primary click that closes the menu MUST NOT also activate what it landed on, and a
  secondary click on another card MUST open that card's menu (FR-001).
- **FR-009**: With a card focused, the Menu key and Shift+F10 MUST open that card's menu, anchored
  within the card's area, with its first entry focused.
- **FR-010**: In an open card menu, Up and Down MUST move focus between entries (wrapping), and
  Enter or Space MUST run the focused entry.
- **FR-011**: Closing a card menu without choosing an entry MUST return keyboard focus to the target
  card when the menu was opened from the keyboard. After an entry chosen from a keyboard-opened menu
  runs and the Overview stays open (an in-place entry, once any dialog it opened closes), keyboard
  focus MUST return to the target card when it is still listed, and otherwise to the Overview's
  first card.
- **FR-012**: A card menu MUST close, without running an entry, when its target location stops being
  listed, the Overview's grid scrolls beneath it, or the Overview closes.
- **FR-013**: The primary click, Enter and Space on a card MUST keep their feature 813 behaviour
  while no card menu is open (while one is open, Enter and Space belong to the menu, FR-010).
- **FR-014**: At most one card menu (and no other context menu) MUST be open at a time.

### Key Entities

- **Card menu**: which location it targets, where it is anchored, whether it was opened from the
  keyboard (for focus return), and its entries (*Open* plus the target's row menu entries).
- **Target mark**: the visual state a card has while it is the target of the open card menu.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Renaming, deleting or reviewing a location from the Overview takes 2 interactions
  (open the menu, choose the entry) plus any dialog the entry already asks for, instead of leaving
  the Overview and finding the location in the sidebar.
- **SC-002**: For every location type, the card menu's entries after *Open* equal that location's
  row menu entries in 100% of checked cases (same labels, same order).
- **SC-003**: The card menu opens within 100 ms of the secondary click or key press, as the row
  menus do.
- **SC-004**: Every card menu action is reachable without a pointer: from a focused card, any entry
  runs with at most the menu key plus arrow keys plus Enter.
- **SC-005**: In 100% of dismissals (Escape, outside click), no entry runs and nothing under the
  outside click is activated.

## Out of Scope

- A per-card overflow (⋮) button; the issue considered and rejected it as clutter.
- New actions that no row menu offers today (for example archiving all of a location's sessions or
  merging its pull request); the card menu offers what the row menu offers plus *Open*.
- Context menus anywhere else (sidebar rows already have theirs); adding Shift+F10 or the Menu key
  to the sidebar rows or other surfaces.
- Acting on several cards at once (multi-select).
- Changing the row menus' entries or behaviour. This feature lifts feature 813's out-of-scope note
  "acting on a location from its card" only through this menu.

## Assumptions

- The Overview (feature 813), its focusable, activatable cards and its card activation already
  exist; this feature adds to them.
- The sidebar's row menus are the single source of a location's actions; the card menu reuses them
  so the two cannot drift, and a change to a row menu reaches the card menu too.
- The app's existing menu surface, its edge clamping and its outside-click and Escape dismissal are
  reused; where that surface lacks keyboard entry navigation, adding it is part of this feature.
- Dialogs the row menu entries open (rename, delete confirmation, bind to daemon) can show above the
  Overview; where one cannot today, making it able to is part of this feature.
- "Open" is not on the row menus because a primary click on a row already opens it; on a card it is
  added because the issue names it and it gives keyboard users a visible way to activate from the
  menu.
