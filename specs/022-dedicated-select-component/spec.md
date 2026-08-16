# Feature Specification: Dedicated Select Component on a Shared Picker Base

**Feature Branch**: `feat/make-select-match-angular-design-3`

**Created**: 2026-08-07

**Status**: Draft

**Input**: User description: "Make select an dedicated compoenent that will fully support the material design 3 look and feel similar to type a head. Both should share same common base." Follow-up: "the drop down option list should be animated same as angular material"

**Bugfix**: 2026-08-09 — [BUG-002](./bugs/BUG-002.md) added the interaction-state requirements
(FR-034 – FR-036, SC-011, SC-012). The feature specified where a state layer's colour comes from but
never what area it covers, so the select's trigger drew its layer over 40% of the field it responds
on; and no input in the app answers focus at all, because focus was never named as a state.

## Context

The application has two controls that ask a person to choose one item from a list that appears
beneath a field:

- the **search picker** (used for choosing a branch), which the design system owns end to end — its
  list is a real menu surface, its rows carry the full state treatment, it marks the current
  selection, it answers the keyboard, and it shows an active indicator while the list is open; and
- the **select** (used for choosing a worktree type, and offered by the component gallery), which is
  a thin skin over a list control supplied by the rendering stack. Only its colours are the design
  system's. Its list surface, row metrics, row states, selection marker and keyboard behaviour are
  the stock control's, it cannot report that it is open, and its list appears and disappears
  instantly.

The result is two controls that do the same thing and look and behave like two different products,
and a documented fidelity gap (the select's active indicator is inert because nothing can tell the
control it is open). Neither list animates when it opens or closes.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - The select is a first-class Material control (Priority: P1)

A person opening the "add worktree" dialog picks a type from the select and a branch from the search
picker. Both controls present the same kind of field, open the same kind of list over the same kind
of surface, mark the current choice the same way, respond to hover and press the same way, and give
the same visible signal that their list is open. Nothing about the pair says one of them was
borrowed from somewhere else.

**Why this priority**: This is the whole point of the request, and it is the only story that is
visible to a person using the app on its own. Everything else either supports it or polishes it.

**Independent Test**: Open the dialog (and the component gallery's select and search-picker
entries), open each control's list, and compare surface, corner, elevation, row height, row padding,
label treatment, hover/pressed/selected treatment, selection marker and open-state indicator. Ship
this alone and the select already stops looking foreign.

**Acceptance Scenarios**:

1. **Given** a select with nothing chosen, **When** it is at rest, **Then** it shows its name resting
   on the value's line inside its container, with no second placeholder line beneath it, exactly as
   an empty text field does.
2. **Given** a select with a choice made, **When** it is at rest, **Then** its name sits small at the
   top of the container and the chosen option's text sits on the line below it.
3. **Given** a closed select, **When** it is opened, **Then** its list floats above the rest of the
   form anchored to the control, and nothing else on the form moves.
4. **Given** an open select, **When** its list is compared with the search picker's list, **Then**
   the two lists' surface, corners, elevation, padding, row height, row spacing and row states are
   indistinguishable.
5. **Given** an open select with a current choice, **When** the list is shown, **Then** the row
   holding that choice carries the selected treatment and the same leading marker the search picker
   uses, and the rows that are not selected leave the same space for it so every label starts at the
   same position.
6. **Given** an open select, **When** the pointer moves over a row and presses it, **Then** the row
   shows the same hover and press response — including the press ripple — as a search-picker row.
7. **Given** an open select, **When** the list is open, **Then** the control's active indicator is
   thickened and accented for as long as the list is open, without any surrounding screen having to
   supply that fact.
8. **Given** an open select, **When** a row is chosen, **Then** the list closes and the choice is
   reported in the same step.
9. **Given** an open select, **When** a press lands outside both the list and the control, **Then**
   the list closes and the choice is unchanged.
10. **Given** an open select, **When** the keyboard is used, **Then** the down and up keys move the
    highlight, Enter takes the highlighted row, Escape closes without changing the choice, and Tab
    closes the list and moves on — the same assignments the search picker uses.
11. **Given** a select whose list has no room beneath it, **When** it is opened, **Then** the list
    appears above the control instead of being clipped or pushed off screen.
12. **Given** a select with more options than fit, **When** it is opened, **Then** the list stops
    growing at the same row count the search picker stops at and scrolls beyond it.
13. **Given** a closed select, **When** the pointer rests anywhere over its field — including the
    padding at its edges, where a press already opens it — **Then** the whole field is shaded, not
    an inner part of it, and a press from that position ripples. *(Added by BUG-002.)*
14. **Given** any field on the form, **When** it takes the keyboard, **Then** its container shows
    the focused treatment for as long as it holds focus, and drops it when focus leaves.
    *(Added by BUG-002.)*

---

### User Story 2 - The list animates open and closed (Priority: P2)

A person opening either picker sees its list arrive rather than appear: it grows into place and
fades in, and on closing it fades away. The movement matches what a person expects from a Material
Design picker on the web — brief, decelerating on the way in, accelerating on the way out — so
opening a list reads as one continuous action instead of a flicker.

**Why this priority**: It is the difference between "styled like Material" and "feels like
Material", but the control is fully usable without it, so it follows the anatomy work rather than
blocking it.

**Independent Test**: Open and close each control's list repeatedly and observe the transition;
confirm it plays every time, in both directions, in both colour schemes, and that nothing behind the
list moves while it plays.

**Acceptance Scenarios**:

1. **Given** a closed picker, **When** it is opened, **Then** its list grows from a slightly
   compressed state to its full height while fading in, decelerating as it settles.
2. **Given** an open picker, **When** it is closed by any means — a choice, a press outside, or the
   dismiss key — **Then** its list fades out as it goes rather than vanishing between frames.
3. **Given** a list in the middle of opening, **When** it is dismissed, **Then** the transition
   reverses from where it is rather than jumping to either end.
4. **Given** a list in the middle of closing, **When** a press lands where a row used to be, **Then**
   nothing is chosen — a list on its way out accepts no input.
5. **Given** either picker, **When** its list animates, **Then** no other element on the screen
   changes position or size at any point in the transition.
6. **Given** the two pickers side by side, **When** each is opened, **Then** their transitions are
   the same length and the same shape.
7. **Given** the motion timings already published by the design system, **When** this transition is
   specified, **Then** it uses the existing menu open and menu close timings and introduces no new
   ones.

---

### User Story 3 - One foundation behind both pickers (Priority: P3)

Somebody fixing how a picker's list is positioned, when it closes, or which keys it answers changes
it in one place, and both controls change together. Somebody adding a third picker later gets all of
that behaviour by asking for it.

**Why this priority**: It is the durability half of the request. A person using the app cannot see it
directly, but without it the two controls drift apart again the first time one of them is touched.

**Independent Test**: Take the behaviours the two controls share — where the list sits, how it flips,
what dismisses it, which keys it claims, how a row is drawn and how it responds — and exercise each
through both controls. Every one must be defined once and pass for both.

**Acceptance Scenarios**:

1. **Given** the shared behaviours, **When** each is exercised through the select and through the
   search picker, **Then** both give the same answer.
2. **Given** a change to a shared behaviour, **When** it is made, **Then** it is made in one place and
   both controls are affected.
3. **Given** the component library, **When** the select is inspected, **Then** it is offered in the
   same shape as every other shared component, and the screens using it state no raw sizes, colours
   or timings.
4. **Given** the appearance and behaviour split the library already keeps, **When** the shared
   foundation is inspected, **Then** the behaviour half still names no colour, type, shape or timing
   value, and the appearance half still decides no positioning or dismissal.

---

### Edge Cases

- **No options at all**: an opened select with nothing to offer says so on the list surface rather
  than showing a bare empty panel, matching how the search picker reports a search with no matches.
- **A single option**: the list still opens, animates and marks the option if it is the current
  choice.
- **An option whose label is longer than the control**: the row truncates rather than widening the
  list past the control's width.
- **Very many options**: the list stops at the shared row cap and scrolls; the animation length does
  not change with the row count.
- **Opened near the bottom or the right edge of the window**: the list flips above and stays within
  the window.
- **Opened inside a content-sized dialog**: the list still floats and still measures its width from
  the control, and the dialog does not grow to contain it.
- **Rapid repeat toggling**: opening and closing faster than the transition can finish leaves the
  list in a state consistent with the last action, with no residue left on screen.
- **A picker removed from the screen while its list is open** (the dialog is dismissed): the list
  goes with it and nothing continues animating.
- **Keys pressed while the list is open**: the keys the list claims never also reach the form behind
  it — in particular, taking a row must not also submit the dialog.
- **Both schemes**: every state above is legible in light and in dark.

## Requirements *(mandatory)*

### Functional Requirements

**The select as a first-class component**

- **FR-001**: The select MUST be a component of the shared component library in its own right,
  owning its field, its list surface, its rows and its states — not a re-coloured stock control.
- **FR-002**: The select MUST present the same field anatomy as the shared text field: container,
  name inside the container, value, active indicator on the bottom edge only, optional supporting
  text, and an error state that recolours the indicator, the name and the text beneath.
- **FR-003**: A select with no choice made MUST rest its name on the value's line and suppress a
  second placeholder; a select with a choice made MUST float its name to the top of the container.
- **FR-004**: The select MUST show a trailing chevron in the design system's trailing-icon size and
  muted colour.
- **FR-005**: The select's list MUST float above the surrounding layout, anchored to the control's
  own on-screen position, and MUST NOT displace anything around it.
- **FR-006**: The select's list MUST match the control's width, MUST open beneath the control when
  there is room and above it when there is not, and MUST stay within the window.
- **FR-007**: The select's list surface MUST use the design system's menu surface treatment —
  container tone, elevation, corner radius and vertical padding — identical to the search picker's
  list.
- **FR-008**: The select's rows MUST use the same row anatomy as the search picker's rows: the same
  height, the same horizontal padding, the same leading slot, the same label treatment, and the same
  hover, pressed and selected state layers.
- **FR-009**: The select MUST mark the current choice in the open list with the same leading marker
  the search picker uses, and MUST reserve that slot on unmarked rows so every label starts at the
  same position.
- **FR-010**: Pressing a row MUST produce the same press ripple every other pressable surface in the
  app produces.
- **FR-011**: Choosing a row MUST close the list and report the choice in the same step.
- **FR-012**: A press outside both the list and the control, and the dismiss key, MUST each close the
  list leaving the choice unchanged.
- **FR-013**: While the list is open the select MUST show its active indicator in the open state, and
  MUST do so from its own knowledge of being open — no surrounding screen may be required to supply
  it. This closes the accepted fidelity gap recorded for the select's active indicator.
- **FR-014**: The select MUST answer the same keys as the search picker with the same meanings: move
  the highlight down, move it up, take the highlighted row, dismiss, and dismiss-and-move-on. Keys
  the list takes MUST NOT also reach whatever is behind it; the dismiss-and-move-on key MUST still
  reach it.
- **FR-015**: The select's list MUST stop growing at the same row count the search picker stops at
  and MUST scroll beyond it.
- **FR-016**: An opened select with no options MUST say so on the list surface in the same muted
  treatment the search picker uses for a search with no matches.
- **FR-017**: A select option MAY be presented as unavailable: shown, muted and unpressable, matching
  how the search picker presents an unavailable row. **Unpressable means the press stops there** — the
  row consumes it and does nothing. It MUST NOT pass the press through to whatever is behind the list,
  which for a list floating over a dialog is the dialog's own dismissal. *(Clarified by 016 BUG-002,
  2026-08-14: a row implemented as "a button with no press message" is not pressable and not opaque
  either, and pressing an unavailable branch closed the whole form.)*

**Motion**

- **FR-018**: Opening a picker's list MUST animate: the list grows from a slightly compressed state
  to its full size while fading in, decelerating as it settles.
- **FR-019**: Closing a picker's list MUST animate: the list fades out as it goes, accelerating.
- **FR-020**: The open and close transitions MUST use the design system's already-published menu
  open and menu close timings and easings. No new duration or easing token may be introduced, and
  the count of new animations the visual system permits MUST NOT increase — this is an existing,
  already-assigned animation reaching a surface that was not drawing it.
- **FR-021**: Both pickers MUST animate identically. A transition interrupted mid-flight MUST
  continue from where it is rather than restarting or jumping.
- **FR-022**: A list that is fading out MUST accept no pointer or keyboard input, and MUST leave
  nothing on screen once it has gone.
- **FR-023**: No element outside the list may move, resize or reflow at any point during either
  transition.

**The shared foundation**

- **FR-024**: The behaviours the two pickers share — where the list is anchored, how it flips, what
  dismisses it, which keys it claims and passes on, and how the transition is driven — MUST be
  defined once and consumed by both.
- **FR-025**: The presentation the two pickers share — the list surface, the row anatomy, the row
  states, the selection marker and the empty-list message — MUST likewise be defined once and
  consumed by both.
- **FR-026**: The shared foundation MUST keep the library's existing separation: the behaviour half
  names no colour, type, shape or motion value; the appearance half decides no positioning, capture
  or dismissal.
- **FR-027**: The select MUST be offered to the rest of the app in the same shape every other shared
  component is offered in — required choices first, everything optional added one call at a time —
  so that a screen using it states no raw size, colour or timing of its own.
- **FR-028**: A third picker built on the foundation later MUST obtain positioning, dismissal,
  keyboard handling, row treatment and motion without restating any of them.

**Interaction states** *(added by [BUG-002](./bugs/BUG-002.md), 2026-08-09)*

- **FR-034**: A control's state layer MUST cover the same area the control responds on. Wherever a
  press is accepted and a hover is registered, that is the area the hover, focus and pressed layers
  MUST shade, and the area a ripple MUST originate within — not an inner element that happens to be
  the pressable one. This is the rule the feature assumed and never stated: FR-002's field anatomy,
  FR-008's row anatomy and FR-010's ripple all say what a layer looks like, and none says how far it
  reaches.

  > **The rule runs both ways** *([BUG-003](./bugs/BUG-003.md), 2026-08-09)*. BUG-002 read it as
  > "the layer must grow to the responsive area", which is the direction the select was wrong in.
  > The text field was wrong in the other: its layer covered the whole 56dp container while only
  > the 24dp value line accepted a press, so a click in the padding landed on a box that shaded,
  > hovered and looked entirely pressable, and did nothing. A press anywhere in a filled field's
  > container now reaches its control — excluding the adornment slots, where a trailing icon button
  > is an action of its own.
- **FR-035**: Every input MUST answer focus with the design system's focused state layer, held for
  as long as the input has the keyboard and dropped when it leaves — the select, the text field, the
  search picker and the checkbox alike. Focus is distinct from the active indicator, which already
  answers separately and MUST keep doing so.

  > ~~**Partly unmet, and it is the rendering stack that stops it** *(found while implementing
  > T045, 2026-08-09)*. Every control wearing the shared field container answers focus: the text
  > field and the search picker report it, and the select reports its open state at the pressed
  > opacity. The **checkbox cannot**. It is the stack's own checkbox, whose style is a function of
  > `Status`, and that enum has three variants — active, hovered, disabled. There is no focused
  > variant to answer, so the layer has nowhere to attach. Meeting this for the checkbox means
  > owning the widget the way `FilledField` owns the field, which is a larger change than this bug
  > justifies. The checkbox answers **hover** (FR-036) and nothing else, and this is recorded rather
  > than quietly dropped so the next person does not read the gate as covering it.~~
  >
  > **Met for the checkbox too, and the missing variant was the smaller half of it**
  > *([BUG-003](./bugs/BUG-003.md), 2026-08-09)*. The absent `Status::Focused` was the symptom. The
  > cause is that the rendering stack's checkbox **cannot be focused at all**: its widget state is
  > the label's shaped paragraph, it joins no focus traversal and it answers no key. There was no
  > focus to report because the control was reachable by pointer only — an accessibility gap as
  > much as a visual one, and one no amount of styling would have closed.
  >
  > It is not reimplemented. A wrapper holds the focus, takes it on a press, offers it to the focus
  > traversal, toggles on Space — and leaves Enter to the dialog, which carries it to
  > `TextField::on_submit` today and may grow a default action tomorrow — and
  > reports changes; the stack's checkbox keeps drawing
  > itself and keeps owning the pointer. The layer is still composited into the fill, because
  > `checkbox::Style` still has one opaque background and nowhere to put a quad — but *which* layer
  > is now `Layer`'s to decide, shared with the field, so a focused **and** hovered box shows one.
  >
  > ~~**And for the text field it renders but never fires** *(visual pass, 2026-08-09)*. Focus is a
  > *supplied* flag — `TextField::active` — and **no call site in the application or the gallery
  > passes it**. The container draws the layer when told; nothing tells it. The select is
  > unaffected, since its open state is its own. Closing this needs a screen that tracks which
  > field holds the keyboard, which is a change to every form rather than to this component, and
  > the same gap has kept the *active indicator* dark since feature 018. **FR-035 is therefore met
  > for the select and unmet in practice for every text field.**~~
  >
  > **Met for the text field too** *([BUG-003](./bugs/BUG-003.md), 2026-08-09)*. `active` stays a
  > supplied flag for the reasons above, and the field now *reports* what to supply: the container
  > asks its control, through the rendering stack's own focus traversal, so the answer comes from
  > the input's state rather than from a guess about where the pointer was. The application holds
  > which field has the keyboard (`State::focused_field`) and hands it back on the next view, which
  > is what floats the label as well as shading the container — the label's position is settled
  > when the field is built and no amount of observing later could move it. **FR-035 is now met for
  > every input except the checkbox**, whose limit is unchanged and recorded above.
  >
  > **And the application can say so back** *([BUG-004](./bugs/BUG-004.md), 2026-08-10)*. Observing
  > in the control and holding in the application is two copies of one fact, and until now only the
  > control could correct the application. Feature 023's `focus_terminal()` clears `focused_field`
  > with no press landing anywhere near the control, which left one drawn at rest while it went on
  > answering keys. The supplied flag is now authoritative on the frame it changes — but only where
  > the caller asked to be told, because `active` is focus for a text field and *open* for a picker,
  > and one rule for both would take the keyboard out of the search field whenever its list closed.
- **FR-036**: Every input MUST answer hover with the design system's hover state layer, on the same
  area FR-034 defines. Today the text field and the checkbox draw no hover layer at all; the select
  draws one over part of itself.
- **FR-036a**: FR-034 – FR-036 MUST be satisfied using the design system's already-published state
  opacities — hover, focus and pressed — introducing no new token, exactly as FR-020 requires of the
  motion values. The focused opacity already exists and is unused by any input.

**Parity, consumers and documentation**

- **FR-029**: Every state above MUST be correct in both the light and the dark scheme and MUST behave
  equivalently on Linux, macOS and Windows.
- **FR-030**: The existing consumer of the select — the add-worktree type field — MUST keep working
  with no change to what it accepts, validates or submits. This is a presentation and interaction
  change only.
- **FR-031**: The component gallery MUST demonstrate the select's open list and its transition
  alongside the search picker's, so the two can be compared in one place, in both schemes.
- **FR-032**: The design system's published component anatomy MUST be updated to describe the select
  as a first-class control, and its list of accepted fidelity gaps MUST be updated to remove the
  select's active-indicator gap.
- **FR-033**: The user-facing documentation for the component library MUST be updated in the same
  change to describe the select and the shared foundation.

### Key Entities

- **Picker**: a field that reveals a list of choices anchored to itself. The select and the search
  picker are the two that exist; both are assembled from the shared foundation.
- **Option row**: one choice in the list — its label, whether it is the current choice, whether the
  keyboard is on it, and whether it can be taken.
- **Open state**: whether a picker's list is showing. Owned by the picker itself for the select;
  already owned by the surrounding screen for the search picker, whose query the list depends on.
- **Highlight**: which row the keyboard is on, distinct from which row is the current choice; both
  may be the same row and are shown differently.
- **List surface**: the floating panel the rows sit on — its tone, elevation, corner, padding, row
  cap and scrolling.
- **Transition**: how far the list has come between hidden and shown, and in which direction.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With the select's list and the search picker's list open side by side, a reviewer
  finds **zero** differences across the eight compared properties: surface tone, elevation, corner
  radius, list padding, row height, row padding, row state treatment, and selection marker.
- **SC-002**: Opening or closing either list plays a visible transition **100% of the time**, in both
  directions and both colour schemes, with the open transition settling within the design system's
  published menu-open duration and the close within its menu-close duration.
- **SC-003**: **Nothing outside the list changes position or size** in any frame of either
  transition, in the dialog and in the gallery.
- **SC-004**: A person using only the keyboard can open, traverse, choose from and dismiss **both**
  pickers, and **all five** claimed keys carry the same meaning in each.
- **SC-005**: Opening the select produces visible feedback with **no** state supplied by the screen
  around it, and the design system's list of accepted fidelity gaps drops from **four** entries to
  **three**.
- **SC-006**: The select's list renders fully — never clipped, never off screen, never widening its
  container — in all four placements exercised: inside a content-sized dialog, at the bottom edge of
  the window, at the right edge, and on a full-height page.
- **SC-007**: The change introduces **zero** new motion durations or easings, and the number of
  animations beyond the visual system's sanctioned set stays at **zero**.
- **SC-008**: Every behaviour the two pickers share is verified by tests that exercise it through
  **both** controls, and a deliberate change to any one of those behaviours requires editing exactly
  **one** place.
- **SC-009**: Choosing a worktree type produces the **same** result before and after the change —
  no difference in what the form accepts or submits.
- **SC-010**: A reviewer can see the select and the search picker, open, in both schemes, from a
  **single** page of the component gallery.
- **SC-011**: For every control that draws a state layer, the rectangle it shades and the rectangle
  it accepts a press on are the **same** rectangle — measured, not reviewed. The select's trigger
  currently shades 40% of the field it responds on (440×24 within 472×56). *(Added by BUG-002.)*
- **SC-012**: **Every** input — select, text field, search picker, checkbox — shows a distinct
  treatment at rest, on hover and on focus, in both schemes. The count of inputs that respond to
  focus rises from **zero** to ~~all of them~~ ~~**every input that can report it**: the three
  wearing the shared field container. The checkbox responds to hover only — see FR-035's note, and
  it is a limit of the stack's checkbox rather than a decision.~~ **all of them, as first written.**
  *(Added by BUG-002; amended 2026-08-09 on implementing it, and again on closing BUG-003.)*

  > **Counted in the running application, not in a pose** *(BUG-003, 2026-08-09)*. The count above
  > was three the day it was written and one in the application, because two of the three could
  > only be posed by a test. It is now **four in both** — select, text field, search picker,
  > checkbox: `field_focus_call_sites.rs` holds every input in the application to reporting its
  > focus, which is the check whose absence let the count differ. The exclusion the criterion had
  > acquired is withdrawn rather than met halfway; it asked for every input and every input now
  > answers.

## Assumptions

- **The shared foundation is an extraction, not a new invention.** The search picker's existing
  behaviour half already does anchored positioning, flipping, outside-press dismissal and the
  keyboard rule; it is generalised so the select can use it, rather than a second mechanism being
  built. Likewise the search picker's list surface and row presentation become the shared ones.
- **Where the two lists differ today, the search picker's treatment wins**, because the request is
  for the select to look like the search picker. In particular the row label keeps the search
  picker's current text role rather than moving to the generic menu-item role, so the two lists
  agree; the generic menu component elsewhere in the app is unaffected.
- **"Same as Angular Material" means grow-and-fade in, fade out**, on the design system's existing
  menu open/close timings and easings, rather than importing that library's exact numbers. The
  published timings are already the closest match the design system has.
- **The select stays single-choice.** Multi-select, option groups, and options with secondary text or
  icons are not part of this change.
- **The select does not gain type-to-filter or type-to-jump.** A person who needs to search a long
  list uses the search picker, which exists for that; adding a second searching control would blur
  what each is for.
- **The select's open state is the control's own.** Unlike the search picker — whose surrounding
  screen holds the query the list is derived from and therefore holds openness too — the select has
  nothing for a screen to hold, so it tracks its own and needs no new messages in the screens that
  use it.
- **Existing option types are unchanged.** The types offered by today's select already provide
  everything the new component needs from them.
- **The list's row cap and scroll behaviour** are the search picker's existing ones, expressed in
  rows rather than a fixed height so a density change does not silently change how many fit.
- **The gallery and the design-system documents are deliverables of this change**, per the project's
  documentation rule, not follow-ups.
- **FR-034 – FR-036 reach past the two pickers, deliberately.** *(BUG-002, 2026-08-09.)* The rest of
  this specification is about the select and the search picker, and a state-layer rule scoped to
  them would be the same mistake in a smaller frame: the select's layer is confined to a sub-slot
  because it inherited the text field's arrangement, so fixing one control and not the arrangement
  leaves the next field to repeat it. Hover and focus for the text field and the checkbox are
  therefore **new behaviour adopted into this feature**, not regressions it caused — they have never
  existed. Where a layer already exists and is merely the wrong size, that is a defect of this
  feature.

## Out of Scope

- Multi-select, option groups, and option rows with icons or secondary text.
- Searching or filtering within the select.
- Any change to the generic menu, context menu or popover components beyond what the shared
  foundation naturally covers.
- Changing what any form accepts, validates or submits.
- New motion tokens, new colour roles, or any other addition to the design system's token set.
- Reduced-motion or accessibility-preference handling, which the app does not yet observe anywhere.
