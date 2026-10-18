# Feature Specification: Overview Card Ripple

**Feature Branch**: `claude/project-thread-kqz6b6`

**Created**: 2026-10-18

**Status**: Draft

**Input**: User description: "Implement GitHub issue #971: Overview: ripple feedback when clicking
a card. The problem you want solved: Clicking an overview card gives no pressed feedback beyond the
navigation itself. What you would like to happen: Card clicks show a ripple originating at the
pointer position, consistent with the other ripple-capable controls and the state layers;
activation via keyboard (Enter/Space) shows a centered ripple. Alternatives you considered: A plain
pressed-state tint only." Labels: enhancement, flow:feature.

## Terms

- **Card**: an entry of the project Overview, one per location (feature 813). A card that can be
  activated is an **activatable card**; a card shown without an activation (for example an
  informational card in the component showcase) is a **plain card**.
- **Activation**: a pointer click on an activatable card, or Enter or Space while the card holds the
  keyboard focus. Activation performs the card's action (feature 813, FR-013: close the Overview and
  show the location's main session).
- **Ripple**: Material's press indication as the app already draws it on its other pressable
  controls (feature 018, FR-024a–FR-024h): a circle that starts at an origin, grows until it covers
  the element, then fades, drawn in the element's content colour at the pressed opacity and clipped
  to the element's own shape.
- **State layer**: the translucent tint a control shows for hover, focus or press (feature 018, §5).
  Layers never sum: the heaviest single layer is what shows.

## Clarifications

### Session 2026-10-18

- Q: When does each ripple start relative to the activation: on pointer press or release, on key
  press or release? → A: The pointer ripple starts when the primary button goes down (the app's
  ripple starts on `ButtonPressed`), while the activation stays on release inside the card as an
  iced `button` does today; the keyboard ripple starts with the activation itself, on the key press
  that publishes the card's message (cards answer `KeyPressed`, not the release).
  _(agent-resolved: crates/micold-client/src/ui/material/ripple.rs#update,
  crates/micold-client/src/ui/material/keyboard_focus.rs#update)_
- Q: How does the ripple stay visible when activation closes the Overview (feature 813 FR-013)? →
  A: The Overview stays up for the ripple's expand time (300 ms, the ripple's `duration::MEDIUM_2`)
  after the activation, so the ripple plays, then closes and shows the location's main session as
  feature 813 specifies. _(decided by user)_
- Q: What does input to the Overview do during that delay? → A: Nothing: the activation's outcome
  is fixed when it happens, and until the Overview closes it takes no further card activation or
  key, so a second click, Enter or Escape neither changes the target nor restarts the delay. This
  keeps the outcome identical to today's, where the Overview is already gone at that point.
  _(agent-resolved: specs/971-overview-card-ripple/spec.md#FR-008, specs/813-worktree-overview/spec.md#FR-013)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Pointer press ripples from where the card was pressed (Priority: P1)

A user scanning the Overview presses a card with the pointer. A ripple starts exactly where the
pointer went down and spreads across the card, so the user sees that the press landed on that card
before the view changes.

**Why this priority**: This is the issue's core request; pointer is the primary way cards are
activated, and it brings the card in line with every other pressable control in the app.

**Independent Test**: In the component showcase (where activating a card does not close anything),
press a card near one corner and hold: a circle appears centred at the press point, grows to cover
the card, stays inside the card's rounded corners, and fades after release.

**Acceptance Scenarios**:

1. **Given** an activatable card, **When** the user presses the pointer on it, **Then** a ripple
   starts at the pointer's position within the card, in the card's content colour at the pressed
   opacity.
2. **Given** a ripple running on a card, **When** it grows, **Then** it never draws outside the
   card's rounded shape.
3. **Given** a card pressed in its top-left area and then in its bottom-right area, **When** each
   ripple starts, **Then** each originates at its own press point, not at the card's centre or its
   corner.
4. **Given** a card of any state (Needs action, Needs attention, Working, Done, Idle), **When** it
   is pressed, **Then** the ripple is visible against that state's fill and is drawn in that
   state's content colour.
5. **Given** a ripple has faded, **When** nothing else happens, **Then** nothing on the card keeps
   animating.
6. **Given** a card pressed with the pointer, **When** its ripple fades, **Then** the card shows no
   state layer (a pointer press draws no focus layer).

---

### User Story 2 - Keyboard activation ripples from the centre (Priority: P2)

A user moving through the Overview with the keyboard presses Enter or Space on the focused card. A
ripple starts from the centre of the card, so keyboard users get the same confirmation as pointer
users.

**Why this priority**: Keyboard activation is required for cards (feature 813, FR-014) and the
issue names it explicitly; it depends on the same ripple as Story 1 and adds only the centred
origin.

**Independent Test**: In the component showcase, move the focus to a card and press Enter, then
Space: each starts a ripple centred on the card.

**Acceptance Scenarios**:

1. **Given** an activatable card holding the keyboard focus, **When** the user presses Enter,
   **Then** a ripple starts at the card's centre.
2. **Given** an activatable card holding the keyboard focus, **When** the user presses Space,
   **Then** a ripple starts at the card's centre.
3. **Given** a card that does not hold the focus, **When** the user presses Enter or Space,
   **Then** that card shows no ripple.

---

### User Story 3 - The ripple is visible before the Overview closes (Priority: P1)

Activating a card closes the Overview (feature 813, FR-013). The user still sees the ripple begin on
the card they activated, so the feedback is not lost to the navigation it confirms.

**Why this priority**: Without it, Stories 1 and 2 are invisible in the real Overview: the view that
draws the ripple disappears on the same activation.

**Independent Test**: Open the Overview in the client, click a card with a quick click, and press
Enter on a focused card: in each case the ripple is seen on the card before the main session is
shown.

**Acceptance Scenarios**:

1. **Given** the Overview is open, **When** the user activates a card with the pointer or the
   keyboard, **Then** the Overview stays up for the ripple's expand time (300 ms) so the ripple on that
   card plays, and then closes and the location's main session is shown as feature 813 specifies.
3. **Given** a card has been activated and the Overview has not closed yet, **When** the user
   clicks another card or presses Enter, Space or Escape, **Then** nothing changes: the first
   activation's target stands and the close is neither hastened nor postponed.
2. **Given** the user activates a card, **When** the Overview closes, **Then** the outcome of the
   activation (which location is selected, which session is shown, the offer to start a session)
   is the same as without this feature.

---

### Edge Cases

- **Plain card**: a card without an activation shows no ripple on pointer press and takes no
  keyboard focus, as today.
- **Repeated presses**: pressing the same card again while its ripple runs restarts the ripple from
  the new origin; it does not stack a second circle. This applies where the card stays up (the
  component showcase); in the Overview the first activation ends input until it closes (US3).
- **Key auto-repeat**: a repeated Enter or Space key event while the card's ripple runs does not
  restart it; at most one ripple runs on a card at a time.
- **Concurrent cards (Principle II)**: ripples are per card; pressing a second card while the first
  card's ripple fades animates both independently. A card whose location changes state while its
  ripple runs finishes the ripple in the colour it started with.
- **Card removed mid-ripple**: a card whose location disappears from the Overview while its ripple
  runs leaves nothing animating behind.
- **Press in a rounded corner**: a press inside the card's bounding box but outside its rounded
  shape activates the card exactly as it does today; its ripple starts at the press point and is
  still clipped to the rounded shape. A press reported at or past the card's edge starts the ripple
  at the nearest point inside the card.
- **Press, drag off, release**: a pointer pressed on a card and released outside it does not
  activate the card (as today); the ripple that started on press fades normally.
- **State layers**: while the ripple runs it is the card's only state layer (feature 018, FR-024h):
  the focus layer of a focused card is not summed with the ripple, and the card settles back to the
  layer it resumes (the focus layer while the card holds keyboard-arrived focus, per feature 018
  FR-022a; none otherwise, including after a pointer press) without a visible step.
- **Reduced motion and platforms (Principle VI)**: the ripple behaves the same on Linux, macOS and
  Windows; the app has no reduced-motion setting today, so none applies here.
- **Pointer button**: only the primary pointer button starts a ripple, matching the other
  ripple-capable controls.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Pressing an activatable card with the primary pointer button MUST start a ripple whose
  origin is the pointer's position within the card.
- **FR-002**: Activating a focused card with Enter or Space MUST start a ripple whose origin is the
  card's centre.
- **FR-003**: The card's ripple MUST look and move like the ripple of the app's other pressable
  controls: drawn in the card's content colour for its state at the pressed opacity, growing over
  the same expand time and fading over the same fade time, and clipped to the card's rounded shape.
- **FR-004**: While a card's ripple runs it MUST be the card's only state layer, and as it fades the
  card MUST settle to the layer it resumes (the focus layer while the card holds keyboard-arrived
  focus, none otherwise, including after a pointer press) with no visible step, per feature 018 FR-024h.
- **FR-005**: Each card MUST hold its own ripple: ripples on different cards MUST NOT interfere, a
  new press on the same card MUST restart its ripple from the new origin, a repeated key event
  (auto-repeat) while the ripple runs MUST NOT restart it, and once a ripple has
  faded nothing on the card MUST keep animating.
- **FR-006**: A plain card MUST NOT ripple.
- **FR-007**: Activating a card in the Overview MUST keep the Overview up for the ripple's expand
  time (300 ms) before it closes and shows the location's main session, so the ripple on that card
  is seen for pointer and keyboard activation alike. During that time the Overview MUST take no
  further card activation or key; the first activation's outcome stands.
- **FR-008**: The outcome of activating a card (feature 813, FR-013 and FR-014) MUST be unchanged:
  the same location is selected, the same session shown, the same offer made.
- **FR-009**: The component showcase's Overview card sample MUST demonstrate the pointer and
  keyboard ripple, so the behaviour can be checked without a project.

### Key Entities

- **Card ripple**: per card, the ripple's origin, how far it has grown and how much strength is
  left; created on press or keyboard activation and gone once faded.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For a pointer press anywhere inside a card, the ripple's origin lies within 1 px of the
  press point (clamped to the card), in 100% of automated checks across the card's four corners and
  centre.
- **SC-002**: For Enter and for Space on a focused card, the ripple's origin is the card's centre in
  100% of automated checks.
- **SC-003**: No pixel of a card's ripple is drawn outside the card's rounded shape in any automated
  check.
- **SC-004**: A ripple on a card completes (grows and fades) in the same time as a ripple on the
  app's buttons, and the card draws no further frames for it afterwards.
- **SC-005**: In the client, a user who activates a card sees the ripple on it before the main
  session is shown, by pointer and by keyboard, on every card state, and the main session is shown 300 ms (±1 frame) after the activation.
- **SC-006**: Every existing Overview activation check (feature 813) passes unchanged.

## Assumptions

- The ripple is the one the app already has (feature 018); this feature applies it to cards and
  adds a centred keyboard origin, it does not redesign the ripple.
- The card's content colour for its state (feature 813, research R11) is its state-layer colour.
- A centred ripple on keyboard activation is added for cards only. Whether the app's buttons, rows
  and chips should also ripple on Enter or Space is a separate change; this feature neither adds nor
  removes it there.

## Out of Scope

- A hover state layer on cards (cards show no hover tint today); a separate issue if wanted.
- A plain pressed-state tint instead of a ripple (the issue's rejected alternative).
- Changing what activating a card does, beyond the 300 ms delay before the Overview closes
  (User Story 3); keeping the Overview open after activation would change feature 813 FR-013.
- Keyboard-origin ripples on controls other than Overview cards.
