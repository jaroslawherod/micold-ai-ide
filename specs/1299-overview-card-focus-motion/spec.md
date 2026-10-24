# Feature Specification: Overview Card Focus and Expansion Motion

**Feature Branch**: `claude/project-thread-vugbvf`

**Created**: 2026-10-24

**Status**: Draft

**Input**: User description: "Implement GitHub issue #1299: Overview cards: animate keyboard focus
and expansion. Keyboard focus and hover on an Overview card change it at once: the focus outline
appears and the card's menu-elevation shadow switches on with no transition. The Overview Card
Redesign design (https://claude.ai/artifact/3xzeTa9hKufvxuiqjGFWPs) eases both over 150 ms
(`transition: box-shadow .15s`). Animate the card's focus: the focus outline and layer fade in and
out, and the elevation shadow of an expanded card eases in and out. Duration
`motion::duration::SHORT_3` (150 ms), `STANDARD` easing (contract §6), driven the way the ripple of
#971 is. Where the in-place growth of an expanded card can be animated without reflowing the grid,
animate it; otherwise the outline and shadow are enough, and the issue says which it did.
Acceptance: Tab or an arrow key onto a card animates the focus in, moving away or Escape animates it
out; hover expansion eases the same way; with Settings → Appearance → Reduce motion on, all of it is
instant; ripple, context menu, link focus and keyboard navigation unchanged; a test covers the
reduce-motion path. Separate from the design-match fix #1298." Labels: enhancement, flow:feature.

## Clarifications

### Session 2026-10-24

- Q: When the window loses or regains focus, does a focused card's focus indication change (and so
  fade)? → A: No. Window blur collapses the expanded card (feature 1121, FR-005) but leaves the
  keyboard focus on the card, so its focus indication stays as it is on blur and on refocus and no
  focus fade runs; only the collapsing card's shadow eases out. _(agent-resolved:
  crates/micold-client/src/ui/material/card_grid.rs#on_window_blur, which publishes only the
  expansion blur; specs/1121-overview-card-full-text/spec.md#Edge Cases, "Window regains focus")_
- Q: Does the hover state layer of a card (the tint drawn under the pointer) fade too? → A: No. The
  issue names only the focus outline and layer and the expanded card's shadow, and the design
  transitions only `box-shadow`; the hover layer keeps changing in one step. _(agent-resolved:
  specs/1299-overview-card-focus-motion/spec.md#Input)_
- Q: When a focused card's context menu opens from the keyboard, and closes again, does the card's
  focus indication fade? → A: Yes. Opening moves the keyboard focus to the menu's first entry
  (feature 972, US3-1), so the card's focus indication fades out (FR-002) while the target mark
  appears in one step as today; closing the menu without a choice returns the focus to the card
  (feature 972, FR-011), and its focus indication fades in (FR-001). _(agent-resolved:
  specs/972-overview-card-context-menu/spec.md#FR-011)_

## Terms

- **Card**: an entry of the project Overview, one per location (feature 813).
- **Focus indication**: what a card shows while it holds the keyboard focus: the focus outline (a
  ring inside the card's edge) and the focus state layer (a translucent tint over the card), as
  features 018 (§5) and 1121 draw them today.
- **Expanded card**: the card drawn in place over its neighbours with its full text (feature 1121),
  when the pointer rests on it or it holds the keyboard focus. It casts the menu elevation's shadow;
  a collapsed card casts none.
- **Growth**: the change of an expanded card's height from its collapsed height to its expanded
  height (and back on collapse).
- **Transition**: the change of the focus indication or the expanded card's shadow (and, where it
  is animated, its growth) from its start state to its end state. Today it happens in one frame.
- **Reduce motion**: the app setting under Settings → Appearance (feature 970, FR-008), off by
  default, saved like the other settings.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Keyboard focus eases onto and off a card (Priority: P1)

A user moving through the Overview with Tab or the arrow keys sees the focus outline and the focus
tint fade in on the card they reach and fade out on the card they leave, instead of snapping on and
off, as the Overview Card Redesign design shows.

**Why this priority**: Keyboard focus is the change the issue names first, and it is the one a
keyboard user sees on every step through the grid.

**Independent Test**: In the component showcase or a real Overview with Reduce motion off, Tab onto
a card and watch the outline and tint: about 75 ms after the key they are partly drawn, and after
150 ms they are fully drawn; Tab away and they fade out over the same time.

**Acceptance Scenarios**:

1. **Given** the Overview with no card focused and Reduce motion off, **When** the user presses Tab
   onto a card, **Then** the card's focus outline and focus layer fade in from none to full over
   150 ms along the standard easing curve.
2. **Given** a focused card, **When** the user presses an arrow key to move to a neighbour, **Then**
   the focus indication fades out on the card left and fades in on the card reached, both over
   150 ms, at the same time.
3. **Given** a focused, expanded card, **When** the user presses Escape, **Then** the card collapses
   (feature 1121), keeps the keyboard focus, and its shadow eases out over 150 ms.
4. **Given** a focused card, **When** the focus leaves the grid (Tab past the last card, Shift+Tab
   before the first, or the focus moves to a card's link), **Then** the card's focus indication
   fades out over 150 ms; when the focus moves to the card's own link, the card stays expanded and
   its shadow does not change (feature 1122, US1-1).
5. **Given** a fade in progress, **When** the focus returns to the card before the fade ends,
   **Then** the indication turns back from the opacity it had reached, with no jump to none or to
   full.
6. **Given** a transition has ended, **When** nothing else happens, **Then** nothing on the card
   keeps animating.

---

### User Story 2 - An expanded card's elevation eases in and out (Priority: P1)

A user pointing at a card, or moving the keyboard focus onto it, sees the expanded card's shadow
ease in as it lifts over its neighbours, and ease out as it settles back, instead of switching on
and off.

**Why this priority**: The shadow is the other half of the design's `transition: box-shadow .15s`;
hover is the most frequent way cards expand.

**Independent Test**: With Reduce motion off, rest the pointer on a card in the middle of a full
grid: its shadow grows from none to the menu elevation's shadow over 150 ms; move the pointer to the
gap between cards and the shadow fades back to none over 150 ms.

**Acceptance Scenarios**:

1. **Given** a collapsed card and Reduce motion off, **When** the pointer rests on it and it
   expands, **Then** its shadow eases from none to the menu elevation's shadow over 150 ms along the
   standard easing curve.
2. **Given** a hover-expanded card, **When** the pointer leaves it, **Then** its shadow eases out
   over 150 ms.
3. **Given** a hover-expanded card, **When** the pointer moves straight onto another card, **Then**
   the first card's shadow eases out while the second card's eases in, and at most one card is
   expanded at a time (feature 1121).
4. **Given** keyboard focus moved onto a card, **When** the card expands, **Then** its shadow and
   its focus indication ease in together over the same 150 ms.
5. **Given** an expanded card, **When** it is drawn at any moment of the transition, **Then** no
   other card in the grid moves or changes size (feature 1121, FR-009).
6. **Given** the growth of an expanded card can be animated without any other card moving, **When**
   the card expands or collapses, **Then** its height eases between its collapsed and expanded
   height over the same 150 ms; otherwise its height changes in one step as today and only the
   shadow and the focus indication ease. Which of the two the feature does is stated on issue #1299.

---

### User Story 3 - Reduce motion makes card focus and expansion instant (Priority: P1)

A user who has turned on Settings → Appearance → Reduce motion sees the focus indication, the shadow
and the growth change in one step, exactly as before this feature.

**Why this priority**: The issue's acceptance requires it and names a test for it; motion the user
has asked the app to reduce must not be added behind their back.

**Independent Test**: Turn Reduce motion on and save; Tab onto a card, arrow to the next, hover a
card, leave it: each change is complete on the first frame drawn after the input, with no
intermediate frames.

**Acceptance Scenarios**:

1. **Given** Reduce motion on, **When** a card gains or loses the keyboard focus, **Then** its focus
   indication is at full or none on the first frame drawn after the input.
2. **Given** Reduce motion on, **When** a card expands or collapses by hover, focus or Escape,
   **Then** its shadow (and its height) reaches its end state on the first frame drawn after the
   input.
3. **Given** Reduce motion off and a card mid-transition, **When** the user turns Reduce motion on
   and saves, **Then** every later card transition is instant, and no card is left part-way: any
   transition still running reaches its end state no later than its 150 ms would have.

---

### User Story 4 - Everything else on the card is unchanged (Priority: P2)

A user who presses, right-clicks or keyboards through cards finds the ripple, the context menu, the
card's links and keyboard navigation behaving exactly as before.

**Why this priority**: The issue names these as unchanged; they are the guard against regressions,
not new value.

**Independent Test**: Run the existing ripple (feature 971), context-menu (feature 972), link focus
(features 1122, 1123) and keyboard-navigation (features 813, 1121) checks: each passes unchanged.

**Acceptance Scenarios**:

1. **Given** an activatable card, **When** it is pressed with the pointer or activated with Enter
   or Space, **Then** its ripple starts, grows and fades as feature 971 specifies, and a pointer
   press still draws no focus indication.
2. **Given** a card, **When** its context menu is opened with the secondary button or the Menu key,
   **Then** the menu opens, is targeted and closes as feature 972 specifies.
3. **Given** a card with an issue or pull-request link, **When** the keyboard focus moves onto a
   link, **Then** the link's own focus indicator appears as it does today, unanimated.
4. **Given** the Overview, **When** the user moves with Tab, Shift+Tab, the arrow keys, Page Up /
   Page Down and Escape, **Then** which card or link receives the focus, which card expands and
   what Escape does are exactly as before.

### Edge Cases

- **Rapid moves**: the user holds an arrow key so the focus crosses several cards within 150 ms.
  Each card's transition runs on its own from where it is; a card the focus passes through fades in
  part-way and back out, and only the card that keeps the focus ends fully indicated.
- **Hover and focus on different cards**: the pointer leaves a hover-expanded card while another
  card holds the focus; the focused card re-expands (feature 1121, FR-005) and its shadow eases in
  while the left card's eases out.
- **Pointer press on an unfocused card**: the press takes the focus without drawing a focus
  indication (feature 971, US1-6), so no focus fade starts; the ripple plays as before.
- **Activation closes the Overview** (feature 971, Story 3): the Overview closes mid-transition;
  nothing keeps animating once the card is gone, and reopening the Overview starts from the cards'
  current state, not a stale part-way one.
- **Card state changes mid-transition** (Principle II, multi-session): a session's state changes
  (for example Working to Done) while its card's focus fades in, or a location is added or removed
  from the grid. The fade continues to its end in the card's new colours; a removed card drops its
  transition with it; a card added mid-move starts unfocused and collapsed with no transition.
- **Several windows or Overviews**: each Overview animates its own cards; a transition in one never
  shows in another.
- **Empty Overview**: with no cards, there is nothing to animate and nothing runs.
- **Window loses and regains focus**: on window blur an expanded card collapses (feature 1121,
  FR-005) and its shadow eases out over 150 ms (FR-003); the keyboard focus stays on the card, so
  its focus indication does not change on blur or on refocus and no focus fade runs.
- **Context menu on a focused card** (feature 972): opening the menu from the keyboard moves the
  focus to the menu, so the card's focus indication fades out (FR-002) while the target mark appears
  in one step; closing the menu without a choice hands the focus back and the indication fades in
  (FR-001).
- **Frames missed**: when the app draws fewer frames than usual (a busy machine), the transition
  still ends at 150 ms, on time, rather than stretching.
- **Cross-platform** (Principle VI): durations, easing and the Reduce motion behaviour are the same
  on macOS, Linux and Windows; the operating system's own reduced-motion preference is not read
  (feature 970, FR-008).
- **Targeted card** (feature 972): the *selected* outline and layer of a card whose context menu is
  open keep their current behaviour; this feature does not animate them.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: When a card gains the keyboard focus (Tab, Shift+Tab, an arrow move, or the focus
  handed back to it), its focus outline and focus state layer MUST fade from none to full opacity
  over 150 ms along the standard easing curve.
- **FR-002**: When a card loses the keyboard focus, by any route, its focus outline and focus state
  layer MUST fade from their current opacity to none over 150 ms along the same curve.
- **FR-003**: When a card expands (hover or keyboard focus, feature 1121), its shadow MUST ease
  from none to the menu elevation's shadow over 150 ms along the standard easing curve; when it
  collapses (pointer leaves, focus leaves, Escape), the shadow MUST ease from its current strength
  to none over the same time and curve.
- **FR-004**: A transition reversed before it ends MUST continue from the value it had reached, with
  no jump, and MUST take no longer to reach its new end than a full transition would.
- **FR-005**: Each card's transitions MUST run independently of every other card's, so a card
  fading out and a card fading in can be drawn together.
- **FR-006**: Where the growth of an expanded card can be animated without any other card in the
  grid moving or changing size, it MUST ease between the collapsed and expanded height over 150 ms
  along the standard easing curve; where it cannot, the height MUST change in one step as today.
  The feature MUST record on issue #1299 which of the two it does and why.
- **FR-007**: With Reduce motion on, every transition in FR-001 to FR-006 MUST reach its end state
  on the first frame drawn after the change that caused it, with no intermediate frame.
- **FR-008**: A Reduce motion change takes effect when saved (Settings' one-form rule, feature 970);
  a transition running at that moment MUST reach its end state no later than it would have without
  the change.
- **FR-009**: Once every transition on a card has ended, the card MUST stop requesting frames.
- **FR-010**: The end states MUST be exactly today's: a fully focused card, a fully expanded card
  and a collapsed card look the same as before this feature in every card state.
- **FR-011**: The card's ripple (feature 971), its context menu and targeted mark (feature 972), the
  focus indicators of its issue and pull-request links (features 1122, 1123) and the Overview's
  keyboard navigation (features 813, 1121, 970) MUST behave exactly as before.
- **FR-012**: An automated test MUST cover the Reduce motion path: with Reduce motion on, a card's
  focus indication and shadow reach their end states on the first frame after gaining and after
  losing the keyboard focus and after expanding and collapsing.
- **FR-013**: The user guide's description of Reduce motion MUST say that it also makes Overview
  card focus and expansion instant (Principle VII), and the component showcase's Overview card MUST
  show the eased focus and expansion.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With Reduce motion off, a card's focus indication and expanded shadow are part-way
  (neither none nor full) at the frame nearest 75 ms after the triggering input, and full or none
  at 150 ms ± one frame (16 ms).
- **SC-002**: With Reduce motion on, 0 intermediate frames are drawn for any card focus or expansion
  change: the first frame after the input shows the end state.
- **SC-003**: Within one frame after the last card transition ends, the Overview requests no
  further frames when nothing else changes.
- **SC-004**: Every existing ripple, context-menu, link-focus and keyboard-navigation test passes
  without modification.
- **SC-005**: Across a transition, 0 cards other than the transitioning one change position or size.

## Assumptions

- The standard easing curve is the cubic bezier (0.2, 0, 0, 1) of contract §6.2.
- The design's `transition: box-shadow .15s` sets the time; the issue fixes the curve as Material's
  standard easing, so the browser's default `ease` in the design is not copied.
- The focus fade covers both the outline and the state layer together, at one opacity, since the
  design animates the card's focus as one change.
- The ripple keeps its current timings under Reduce motion: the issue lists the ripple as unchanged
  and limits Reduce motion's new reach to card focus and expansion.
- Feature 970's statement that Reduce motion governs the full-screen zoom only is widened by this
  feature to card focus and expansion; no other animation in the app changes.
- This feature adds a new row to the motion assignment table (contract §6.3), card focus and
  elevation at `short_3` with `standard`, and the plan records it there.
- The design-match work of issue #1298 (colours, spacing and layout of the cards) is separate and
  is not part of this feature; this feature changes only how fast the focus and expansion changes
  happen.

### Out of scope

- Animating the targeted (*selected*) outline and layer of feature 972.
- Animating the focus indicators of a card's links.
- Animating the hover state layer of a card; it changes in one step as today.
- Animating the ripple differently, or making it obey Reduce motion.
- Reading the operating system's reduced-motion preference.
- Any change to the cards' appearance in their end states (issue #1298).
