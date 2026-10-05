# Feature Specification: Tooltip follows the cursor and waits before showing

**Feature Branch**: `fix/issue-430`

**Created**: 2026-10-05

**Status**: Draft

**Input**: User description: "#430 Tooltip: restore FollowCursor and show delay in the cdk overlay. Follow-up from BUG-001 (spec 029, PR #425). The shared tooltip no longer offers a follow-the-cursor placement or a show delay, because no current caller uses them. Add them when a caller needs them, and keep the flip behaviour (FR-013) for every position."

## Background

Spec 029 (BUG-001) replaced the rendering stack's tooltip with the app's own overlay so that a tooltip
never covers the row it describes (029 FR-013). The replacement offers four fixed placements (above,
below, left, right) and an optional wait for the pointer to *rest* on the trigger (`after_rest`, from
spec 038: it cancels on leave, restarts on movement beyond a small tolerance). It dropped the stack's
"follow the cursor" placement. No caller uses a pointer-following placement today, so that part is
groundwork: it lands the capability so the next screen that wants it does not rebuild the overlay.
The issue's "show delay" is a separate wait, counted from entering the trigger (see Clarifications).

## Out of Scope

- Changing the existing rest wait, the four fixed placements, or the tooltip's content and styling.
- Any caller adopting the new capabilities (no user-visible change ships beyond the component showcase).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A tooltip that tracks the pointer (Priority: P1)

A screen author asks for a tooltip that is placed next to the pointer rather than next to a side of
its trigger, for a large trigger (a canvas, a long list) where "below the trigger" is far from where
the user is looking. While the pointer moves over the trigger, the tooltip moves with it.

**Why this priority**: It is the placement the issue names as missing; without it the shared tooltip
cannot serve large triggers.

**Independent Test**: Show a tooltip with the pointer-following placement over a large trigger, move
the pointer, and check the panel's position after each move.

**Acceptance Scenarios**:

1. **Given** a pointer-following tooltip is open, **When** the pointer moves within the trigger,
   **Then** the panel stays beside the pointer, offset so it does not sit under it.
2. **Given** the pointer is near the window's right or bottom edge, **When** the panel would run past
   the edge, **Then** it opens on the pointer's other side so that it stays inside the window and
   does not cover the pointer.
3. **Given** the pointer leaves the trigger, **When** it has left, **Then** the tooltip closes as any
   other tooltip does.

---

### User Story 2 - A tooltip that waits before showing (Priority: P2)

A screen author asks for a tooltip that does not appear the instant the pointer touches the trigger,
so that sweeping across a dense list does not flash a tooltip per row. This must work together with
the pointer-following placement, and it is distinct from the existing rest wait (FR-003).

**Why this priority**: The second capability named in the issue; lower than P1 because the existing
rest wait already covers sweeping, so the gap is narrower.

**Independent Test**: Hover a trigger with a show delay, and check that nothing shows before the delay
and the panel shows after it.

**Acceptance Scenarios**:

1. **Given** a tooltip with a show delay, **When** the pointer enters the trigger, **Then** no panel
   shows until the delay has elapsed with the pointer still on the trigger.
2. **Given** the delay is running, **When** the pointer leaves the trigger, **Then** the delay is
   cancelled and no panel shows.
3. **Given** a tooltip with no show delay, **When** the pointer enters the trigger, **Then** it shows
   at once, as it does today.
4. **Given** a tooltip with a show delay and the pointer-following placement, **When** the delay has
   elapsed, **Then** the panel shows beside the pointer's current position and then tracks it.
5. **Given** no tooltip is open or waiting, **When** the window is idle, **Then** nothing is repainted
   continuously for the tooltip.

---

### User Story 3 - Every placement keeps clear of the trigger (Priority: P1)

Every placement, including the two new ones in combination, keeps the guarantee from 029 FR-013: the
panel does not cover what it describes while either side has room.

**Why this priority**: The issue states it as a condition on both additions; breaking it would
reintroduce the bug that moved the tooltip off the stack's widget.

**Independent Test**: For each placement, with and without a show delay, put the trigger at each
window edge and check the panel does not overlap the trigger (or the pointer, when following it).

**Acceptance Scenarios**:

1. **Given** any placement and a trigger at the window's bottom edge, **When** the tooltip opens,
   **Then** it does not cover the trigger.
2. **Given** the pointer-following placement and a pointer in the window's corner, **When** the
   tooltip opens, **Then** it does not cover the pointer and stays inside the window.

### Edge Cases

- The pointer moves while the show delay runs: the show delay keeps counting from entering the trigger; movement does not restart it (the rest wait does).
- A press on the trigger while a pointer-following panel is open: it closes until the pointer leaves, as the rest-wait tooltip does.
- The panel is larger than the room on both sides of the pointer, or the window is too small to fit it anywhere: it takes the side with more room and stays as far inside the window as it fits (029 FR-013).
- The trigger scrolls away or the window resizes while open: the panel stays inside the window and closes when the pointer is no longer over the trigger.
- The trigger describes a different subject mid-hover: the delay starts afresh, as the rest wait does today.
- The pointer is still: the panel's position does not change without pointer movement.
- Several tooltips or windows (Principle II): a tooltip is local to its window and trigger; one window's tooltip neither opens nor closes another's. No per-session state.
- Display scale and monitors (Principle VI): the pointer offset and edge padding are in logical units so they look the same at any scale; the window's own edge bounds the panel on every platform, including at a monitor boundary.
- A zero-size trigger or unknown window size: the tooltip does not open.
- No pointer (touch-only or keyboard use): the tooltip does not open, as today.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The shared tooltip MUST offer a placement that positions the panel next to the pointer
  and moves it as the pointer moves over the trigger.
- **FR-002**: The pointer-following placement MUST keep the panel inside the window and MUST NOT
  place it under the pointer, opening on the pointer's other side when its first side lacks room.
- **FR-003**: The shared tooltip MUST work with a show delay: no panel before the delay has elapsed
  with the pointer on the trigger; leaving the trigger cancels it. The show delay is distinct from
  the existing rest wait: it counts from the pointer entering the trigger and does not restart on
  movement. The rest wait is unchanged and stays a separate opt-in.
- **FR-004**: A tooltip with no show delay MUST open at once on hover, as today.
- **FR-005**: Every placement, with and without a show delay, MUST keep 029 FR-013: the panel does
  not cover its trigger while either side has room, and takes the side with more room when neither does.
- **FR-006**: Existing tooltips (the four fixed placements, the rest wait) MUST behave and look as
  they do today.
- **FR-007**: A show delay MUST NOT keep the application redrawing while no tooltip is open or
  waiting (an idle window repaints nothing).

### Key Entities

- **Placement**: where the panel sits relative to the trigger or the pointer: above, below, left,
  right, or beside the pointer.
- **Show delay**: the wait between the pointer entering the trigger and the panel appearing; not restarted by movement, unlike the rest wait.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With the pointer-following placement, after every pointer move the panel is inside the
  window and does not overlap the pointer.
- **SC-006**: With the four fixed placements the panel does not overlap its trigger while a side has room (029 SC-006).
- **SC-002**: A tooltip with a show delay of D shows no earlier than D after the pointer enters, and
  never when the pointer leaves before D.
- **SC-003**: For every placement, across triggers at all four window edges, zero cases cover the
  trigger while a side with room exists.
- **SC-004**: Every existing tooltip test passes unchanged.
- **SC-005**: An idle window with no tooltip waiting repaints nothing.

## Clarifications

### Session 2026-10-05

- Q: Is "show delay" the existing `after_rest` wait working with every placement, or a separate fixed delay counted from entering the trigger? → A: A separate delay counted from entering, not restarted by movement; `after_rest` stays as is. _(agent-resolved: specs/038-issue-list-reporter-tooltip/autopilot.md#D5 — the removed stack's show delay "counts from pointer entry and cannot restart on movement", and 038 built `after_rest` as its own mode so both coexist)_
- Q: What happens when a caller sets both a show delay and `after_rest`? → A: Not specified here; each must work alone (FR-003, FR-006). The combined rule is a plan decision. _(agent-resolved: no caller uses either; out of scope per Out of Scope)_

## Assumptions

- No screen uses either capability yet; no user-visible change ships with this feature, and a visual
  demonstration lives in the component showcase.
- "Show delay" is a delay a caller opts into; the default stays "no delay".
- The pointer-following offset reuses the gap and edge padding the other placements use.
- Tooltip content and styling are unchanged; this feature is about placement and timing only.
- Depends on the shared tooltip overlay from spec 029 (BUG-001) and its flip rule (FR-013).
