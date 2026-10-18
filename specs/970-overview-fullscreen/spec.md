# Feature Specification: Overview Full-Screen Mode

**Feature Branch**: `claude/project-thread-ag6ana`

**Created**: 2026-10-18

**Status**: Draft

**Input**: User description: "Implement GitHub issue #970: Overview: full-screen mode hides the
sidebar, animated. The problem you want solved: The overview can't use the whole window; the
sidebar stays visible and takes space from the cards. What you would like to happen: A full-screen
overview mode that hides the sidebar. Entering the mode animates a zoom in: the overview scales up
to fill the window as the sidebar goes away. Leaving it animates a zoom out back to the normal
layout with the sidebar restored. Respect reduced-motion (switch instantly), and the cards reflow
smoothly into the freed width. Alternatives you considered: Manually collapsing the sidebar first —
an extra step, and it does not restore itself on leaving the overview." Labels: enhancement,
flow:feature. Builds on feature 813 (`specs/813-worktree-overview`) and feature 1024
(`specs/1024-overview-larger-cards`).

## Terms

- **Overview**, **card**, **location**: as defined by feature 813
  (`specs/813-worktree-overview/spec.md#Terms`). The Overview replaces the main area; today the
  sidebar and the top bar stay beside it.
- **Normal layout**: the window as it is with the Overview open today: top bar, sidebar (expanded
  or collapsed to its thin strip, as the user left it), and the Overview in the main area.
- **Full-screen mode**: a state of the open Overview in which the sidebar, including its collapsed
  strip, is off screen and the Overview fills the whole width of the window below the top bar.
- **Zoom in**: the transition from the normal layout to full-screen mode: the Overview visibly
  grows from the area it has in the normal layout to the whole width of the window while the
  sidebar leaves the screen at its left edge.
- **Zoom out**: the reverse transition, back to the normal layout with the sidebar returned to the
  state it was in before full-screen mode.
- **Sidebar state**: whether the sidebar is expanded (at its width) or collapsed to its thin strip,
  as the user set it with the sidebar's hide / show buttons.
- **Reduced motion**: a preference that asks the app to avoid animated movement; when it is on,
  transitions happen in one step.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Give the Overview the whole window (Priority: P1)

A developer with many worktrees opens the Overview and wants every pixel for the cards. They switch
the Overview to full-screen mode: the sidebar goes away and the cards fill the window's full width,
so more of each card's text fits and more cards are visible without scrolling.

**Why this priority**: It is the problem the issue states: the sidebar takes space from the cards.
Without it there is nothing to animate.

**Independent Test**: With the sidebar expanded and the Overview open in a window 1400 pixels wide,
enter full-screen mode with motion off: the sidebar and its strip are gone, the Overview's left
edge is at the window's left edge, and the cards' row is as wide as the window less the Overview's
own margins.

**Acceptance Scenarios**:

1. **Given** the Overview open in the normal layout, **When** the user enters full-screen mode,
   **Then** the sidebar is not on screen (neither expanded nor as its collapsed strip), the top bar
   stays, and the Overview spans the window's full width.
2. **Given** full-screen mode, **When** the user looks at the cards, **Then** they follow feature
   1024's grid rules for the new width (up to five per row, equal shares of the row, fewer per row
   only when the width calls for it), so each card is at least as wide as in the normal layout.
3. **Given** full-screen mode, **When** the user looks at the top bar, **Then** the Overview button,
   its attention badge and the project selector are where they are in the normal layout and work
   as they do there.
4. **Given** full-screen mode, **When** the user activates a card, presses Escape once, or closes
   the Overview with its button or its shortcut, **Then** that one action both leaves full-screen
   mode and completes as it does today: the sidebar returns in the state it had before (with the
   zoom out, FR-005) while the main area changes to the action's destination (the card's session,
   or the view shown before the Overview); no second action is needed.

---

### User Story 2 - Enter and leave with a zoom (Priority: P1)

Switching into full-screen mode reads as zooming into the Overview: the Overview grows to fill the
window as the sidebar slides away, and the cards widen and move to their new places in one
continuous motion. Leaving zooms back out: the Overview shrinks back to its place and the sidebar
returns.

**Why this priority**: The issue asks for the animation explicitly; an abrupt jump of the whole
layout loses the user's place among the cards.

**Independent Test**: Enter full-screen mode and record the screen: across the transition the
sidebar's right edge moves steadily to the window's left edge, the Overview's left edge follows it,
the cards widen continuously, and no frame shows the cards overlapping, blank, or at a width
outside the range between their start and end widths. Leave full-screen mode and see the reverse.

**Acceptance Scenarios**:

1. **Given** the normal layout with reduced motion off, **When** the user enters full-screen mode,
   **Then** the sidebar leaves at the window's left edge while the Overview grows to fill the freed
   width, as one motion that ends within 500 ms.
2. **Given** full-screen mode with reduced motion off, **When** the user leaves it, **Then** the
   Overview shrinks back to its area in the normal layout while the sidebar returns in its earlier
   state, as one motion that ends within 500 ms.
3. **Given** either transition, **When** the cards' width changes, **Then** each card's width and
   position change continuously; when the number of cards per row changes, a card that moves to
   another row or column travels to its new place rather than disappearing and reappearing.
4. **Given** a transition in progress, **When** the user toggles full-screen mode again, **Then**
   the transition reverses from where it is, without jumping to either end first.
5. **Given** a transition in progress, **When** the user scrolls the cards or a card's content
   changes, **Then** the transition continues and the change shows; the first card visible before
   the transition is still visible after it.

---

### User Story 3 - Reduced motion switches instantly (Priority: P2)

A user who has asked for reduced motion enters or leaves full-screen mode and sees the end layout at
once, with no zoom and no sliding.

**Why this priority**: The issue requires it; it matters to users for whom large movement is
uncomfortable, but the mode is usable without it.

**Independent Test**: Turn reduced motion on, enter full-screen mode: the next frame already shows
the full-screen layout; leave it: the next frame shows the normal layout.

**Acceptance Scenarios**:

1. **Given** reduced motion on, **When** the user enters or leaves full-screen mode, **Then** the
   layout changes in one step, with no intermediate frames of the sidebar or cards in motion.
2. **Given** reduced motion on, **When** the layout changes, **Then** the end state is the same as
   with reduced motion off (same sidebar state restored, same card grid).
3. **Given** a transition in progress, **When** reduced motion is turned on, **Then** the
   transition completes in one step.

---

### User Story 4 - The sidebar comes back as it was (Priority: P1)

Leaving full-screen mode restores the sidebar exactly as the user had it, so the mode never leaves
the window rearranged.

**Why this priority**: The issue names this as what the manual workaround lacks.

**Independent Test**: Collapse the sidebar to its strip, open the Overview, enter full-screen mode,
leave it: the strip is back and the sidebar is still collapsed. Expand the sidebar, resize it to a
non-default width, repeat: the sidebar is back expanded at that width.

**Acceptance Scenarios**:

1. **Given** the sidebar expanded at some width, **When** the user enters and then leaves
   full-screen mode, **Then** the sidebar is expanded at the same width.
2. **Given** the sidebar collapsed to its strip, **When** the user enters and then leaves
   full-screen mode, **Then** the sidebar is collapsed to its strip.
3. **Given** full-screen mode, **When** the user leaves the Overview by any route (card, Escape,
   button, shortcut), **Then** the sidebar shows again in the state and width it had when the mode
   was entered.
4. **Given** full-screen mode, **When** the user opens Settings and closes it again, or switches
   project, **Then** the Overview is still in full-screen mode, the sidebar is still off screen, and
   leaving the mode later still restores the sidebar's earlier state.

---

### Edge Cases

- **Settings while full-screen**: Settings takes the whole content area, sidebar included, as it
  does today, so neither the sidebar nor the Overview shows while it is open. When Settings closes,
  the Overview shows again in full-screen mode (as feature 813 restores an open Overview after
  Settings), without a zoom.
- **Project switched while full-screen**: the Overview stays open, as today, and stays in
  full-screen mode with the new project's cards.
- **Window resized while full-screen**: the Overview keeps filling the window's width; the cards
  follow feature 1024's grid rules for each new width.
- **Very narrow window**: where the normal layout already leaves room for one card per row, full
  screen still applies; the cards get the extra width.
- **Connection lost while full-screen**: the Overview shows its connection-lost state (feature 813)
  in the full-screen layout; the mode is not left by itself.
- **Leaving the Overview while a zoom in is still running**: the zoom out starts from where the
  zoom in is, and the main area changes to the destination as in User Story 1, scenario 4.
- **Cards change during full-screen mode or a zoom** (Principle II: sessions in many worktrees
  change state at any time): a session starting or ending, a card's state, counts or order
  changing, or a worktree added or removed update the cards as feature 813 does today, in the
  full-screen layout and during a zoom alike; FR-006's no-overlap rule still holds, and the
  transition neither restarts nor stops.
- **Empty Overview**: a project with only the project root shows its one card; the mode and the
  zoom work the same with one card as with many.
- **Several windows**: if the app ever has more than one window open, each window's Overview has its own
  mode; entering it in one window does not change another window's layout.
- **Platforms** (Principle VI): the shortcut uses Ctrl on Linux and Windows and Cmd on macOS, as
  the Overview shortcut does; the reduced-motion source per platform follows FR-008; the top bar
  is the same as in the normal layout on every platform.
- **The sidebar's own toggle**: while full-screen, the sidebar's hide / show buttons are not on
  screen; a keyboard route to the sidebar's hide / show, if any, does not bring the sidebar into
  the full-screen layout.
- **Sidebar resize handle**: not on screen and not draggable while full-screen.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: While the Overview is open, the user MUST be able to enter and leave full-screen mode
  [NEEDS CLARIFICATION: how is full-screen mode entered — (a) a toggle control on the Overview
  (button in the Overview's header, plus a keyboard shortcut), off by default, the Overview
  opening in the normal layout each time; (b) the same toggle, but the choice is remembered so the
  Overview reopens in the mode last used; or (c) no toggle: the Overview always opens full-screen
  and closing it is leaving the mode?].
- **FR-002**: In full-screen mode the sidebar, including its collapsed strip and its resize handle,
  MUST NOT be on screen, and the Overview MUST span the window's full width below the top bar.
- **FR-003**: The top bar MUST stay visible and working in full-screen mode, including the Overview
  button, its attention badge and the project selector.
- **FR-004**: In full-screen mode the cards MUST follow feature 1024's grid rules for the
  Overview's new width.
- **FR-005**: Entering full-screen mode MUST animate a zoom in and leaving it MUST animate a zoom
  out, each as one continuous motion that ends within 500 ms, with the sidebar and the Overview's
  edge moving together so no gap or overlap opens between them.
- **FR-006**: During a zoom, card widths and positions MUST change continuously; a card whose row or
  column changes MUST travel to its new place; no frame may show cards overlapping one another or
  the grid empty.
- **FR-007**: Toggling full-screen mode during a transition MUST reverse the transition from its
  current point.
- **FR-008**: With reduced motion on, entering and leaving full-screen mode MUST change the layout
  in one step, reaching the same end state as the animated route. The reduced-motion preference
  comes from [NEEDS CLARIFICATION: the app has no reduced-motion setting today (features 007, 030,
  971 note this) — (a) a new app setting *Reduce motion* in Settings, off by default; (b) the
  operating system's reduced-motion preference where the platform exposes one; or (c) both, the
  app setting overriding the system?].
- **FR-009**: Leaving full-screen mode, by any route, MUST return the sidebar to the sidebar state
  and width it had when the mode was entered; the mode MUST NOT change the user's sidebar state.
- **FR-010**: Activating a card, pressing Escape once, or closing the Overview with its button or
  shortcut while in full-screen mode MUST, as one action, leave full-screen mode (the sidebar
  returning with the zoom out) and do what the action does today, the main area changing to the
  action's destination.
- **FR-011**: Opening Settings and closing it again while in full-screen mode MUST show the
  Overview again in full-screen mode, without a zoom; switching project MUST keep the mode.
- **FR-012**: The control that enters or leaves full-screen mode (if FR-001 provides one) MUST be
  reachable by keyboard, MUST show whether the mode is on, and MUST carry a tooltip or accessible
  label naming its action and any shortcut for the user's platform.
- **FR-013**: The user guide's Overview page MUST describe full-screen mode, how to enter and leave
  it, that the sidebar returns as it was, and how reduced motion affects it; any new shortcut MUST
  be listed where the guide lists shortcuts.
- **FR-014**: The mode MUST behave the same on Linux, macOS and Windows, with the platform's
  modifier in any shortcut (Ctrl on Linux and Windows, Cmd on macOS), as listed under Edge Cases,
  *Platforms*.

### Key Entities

- **Full-screen mode**: on or off, belonging to the open Overview. Carries the sidebar state and
  width to restore on leaving (FR-009). Whether it outlives the Overview depends on FR-001.
- **Transition**: the zoom in progress, if any: its direction and how far along it is, so it can
  reverse from its current point (FR-007) or finish in one step (FR-008).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a 1400-pixel-wide window with the sidebar expanded at its default width, a row of
  cards in full-screen mode is at least as wide as the window less the Overview's own margins, and
  wider than in the normal layout by at least the sidebar's width.
- **SC-002**: Entering or leaving full-screen mode with reduced motion off completes within 500 ms,
  and every frame in between shows the sidebar edge and the Overview edge touching and the cards'
  widths between their start and end values.
- **SC-003**: With reduced motion on, the frame after the user's action already shows the end
  layout.
- **SC-004**: Across 20 enter / leave cycles, with the sidebar collapsed and expanded at several
  widths, the sidebar always returns in the state and width it had before (20 of 20).
- **SC-005**: Entering full-screen mode takes one action from the open Overview; leaving the
  Overview by card, Escape, button or shortcut takes no extra action to restore the sidebar.
- **SC-006**: The control for full-screen mode (if FR-001 provides one) can be reached and operated
  with the keyboard alone, its tooltip names its action and shortcut for the platform, and the user
  guide's Overview page describes the mode (FR-012, FR-013).

## Out of Scope

- The operating system's window full-screen state; the window's size and state do not change.
- Hiding the top bar.
- A full-screen mode for any view other than the Overview.
- Changes to the sidebar itself (its controls, width, collapse behaviour).
- Changes to the Overview's grid rules or card content (features 813, 1024).
- Any settings beyond what FR-001 and FR-008 settle.

## Assumptions

- "Full-screen" means the Overview filling the app window's content area, not the window entering
  the operating system's full-screen state; the window's own size and state do not change.
- The top bar stays in full-screen mode: it holds the Overview button and its badge, the way out of
  the Overview, and the project selector (feature 813).
- Full-screen mode exists only while the Overview is open; it does not hide the sidebar for any
  other view.
- The animation uses the app's existing motion durations and easing; 500 ms is the upper bound,
  not the target.
- Feature 1024's grid rules (up to five cards per row, equal shares, scroll down only) are
  unchanged; this feature only gives the grid more width.
- The sidebar's width and collapsed state remain owned by the sidebar's own controls; full-screen
  mode reads them and puts them back, never writes them.
