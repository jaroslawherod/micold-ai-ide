# Feature Specification: The settings rail slides when it collapses and expands

**Feature Branch**: `fix/settings-side-bar-should-be-animated`

**Created**: 2026-09-14

**Status**: Draft

**Input**: User description: "bug: the settings side bar should be animated. it should has the hide and show animation similar like the worktrees tree view"

Reported as a bug and recorded as [BUG-004](../027-sandboxed-daemon-runtime/bugs/BUG-004.md) against
feature 027, which built the collapsible rail. It is specified here because 027 never asked for the
change between the rail's two states to move. The record reproduces the snap on `origin/main` and
explains why it is new behaviour rather than a 027 defect.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Watch the rail slide in and out (Priority: P1)

Settings shows its sections in a rail down the left side. The rail's **Collapse** control narrows
it to icons so the section beside it gets the width, and the same control widens it again. Today
the rail jumps between the two widths in a single frame, and everything in the section jumps
sideways with it. The user loses their place in the page they were reading, and nothing shows what
changed.

The user presses Collapse and watches the rail slide in, with the section following its edge, the
same way the worktree sidebar's panel slides when it is hidden. Pressing the same control again,
now drawn as an expand icon, slides the rail back out.

**Why this priority**: This is the reported problem, and the whole of the value. A rail that moves
shows the user where the width went and where it came back from.

**Independent Test**: Open Settings, press Collapse and record the rail's width on each frame until
it settles. Then press the control again and record again. Both recordings pass through widths
strictly between the two states, and the section's left edge matches the rail's right edge on every
frame.

**Acceptance Scenarios**:

1. **Given** Settings is open with the rail expanded, **When** the user presses Collapse, **Then**
   the rail narrows through intermediate widths to its icon width, and the section beside it widens
   to match on every frame.
2. **Given** Settings is open with the rail collapsed, **When** the user presses the collapse
   control, drawn as an expand icon, **Then** the rail widens through intermediate widths to its
   labelled width, and the section narrows to match on every frame.
3. **Given** the rail is sliding, **When** it settles, **Then** it is exactly as wide as it is today
   in that state, shows the same content, and the selected section is unchanged.
4. **Given** the rail is collapsing, **When** the user presses the control again before it settles,
   **Then** the rail turns round from the width it has reached rather than jumping to either end.
5. **Given** the rail was collapsed when Settings was last closed, **When** the user opens Settings,
   **Then** the rail appears already collapsed and does not slide.
6. **Given** the worktree sidebar's panel and the settings rail, **When** each is hidden and shown,
   **Then** both take the same time to move and follow the same curve: each starts and finishes
   gently, and neither moves at a constant speed.
7. **Given** the settings rail is sliding, **When** the user looks at its rows, **Then** no label wraps onto a
   second line and no row changes height. What does not fit is cut off at the rail's edge, and no
   icon jumps sideways at any point in the slide.

---

### User Story 2 - Keep the keyboard and pointer working while it moves (Priority: P2)

A user working from the keyboard focuses Collapse and presses it. While the rail slides, the
labelled rows give way to icon-only ones. Focus has to survive that swap: it stays on the control
the user pressed, so pressing it again undoes the collapse. Tab never lands on a control that is no
longer drawn. With the pointer, only what is drawn can be hovered or pressed, and nothing the rail
draws spills over the section beside it.

**Why this priority**: A slide that strands focus or leaves a hidden copy of the rail reachable
would make keyboard use worse than the snap it replaces. It is second only because Story 1 has to
exist first.

**Independent Test**: Focus the collapse control with the keyboard, activate it, and on every frame
until the rail settles check two things: focus is on that control, and the number of controls Tab
can reach equals the count at rest. Then move the pointer across the rail during a slide and confirm
hover and press land only on controls that are drawn.

**Acceptance Scenarios**:

1. **Given** the collapse control has keyboard focus, **When** the user activates it, **Then** focus
   is on the same control on every frame of the slide and after it settles, in both directions.
2. **Given** the rail is sliding, **When** the user presses Tab, **Then** focus never lands on a rail
   control that is not drawn, such as a hidden copy of the rail, and each rail control is reached
   once.
3. **Given** the rail is sliding, **When** the pointer moves over the rail or the section beside it,
   **Then** hover and press affect only controls drawn under the pointer, and no rail content is
   drawn over the section.
4. **Given** the rail is sliding, **When** the user selects a section, **Then** that section is
   shown and the rail finishes its slide.

---

### Edge Cases

- **Repeated presses**: pressing the control several times in quick succession never jumps. The rail
  always heads for the latest state from wherever it is.
- **Leaving Settings mid-slide**: closing Settings while the rail moves is harmless. On reopening,
  the rail is at the state it was heading for, not partway.
- **Window resized mid-slide**: the rail's width depends only on its own progress, not on the window.
  The section absorbs the change.
- **Theme or section changed mid-slide**: the slide carries on. The new theme and the newly selected
  section are drawn at the rail's current width.
- **Badges**: a section with something to report still marks its row throughout the slide, in
  whichever form the row is drawn (FR-015).
- **A destination with no icon**: the rail component keeps such a destination's name even when
  collapsed, so that it stays pressable. Settings has none, but another screen built on the same
  rail might. Mid-slide, that row follows the rail's current width instead of being cut off like
  the others: its name may wrap, its badge stays in view (FR-015), and it is never drawn empty
  (FR-013's exemption). Such a row's two rest heights already differ, since its name wraps in the
  collapsed rail, so no slide can keep its height; the rows below it move up or down with it, and the
  rail's column is never taller than at the taller of its two rest states.
- **Empty or minimal rails**: a rail with no destinations, one destination, no badges or no icons
  slides the same way and settles at the same two widths. Nothing about the slide depends on how many
  rows there are.
- **No failure path**: the slide loads nothing, saves nothing and asks no service for anything. If
  frames arrive late, each frame advances the slide by no more than a bounded step, so the slide
  finishes late rather than jumping. A hidden window picks the slide up where it stopped.
- **Refresh rate and display scale**: the slide is measured in elapsed time and progress, not in
  frames, so it looks the same at 60 Hz, 120 Hz or uncapped, and at any display scale.
- **Focus while clipped**: a focused control that is partly cut off at the rail's edge keeps its
  focus indicator wherever the control itself is drawn (FR-007, FR-009).
- **The toggle state is not held back**: the collapsed/expanded state flips the moment the control is
  pressed. Only the drawing moves. Nothing that reads the state waits for the slide.
- **Several windows or sessions** (Principle II): the rail's state belongs to the application
  instance. Sessions running in the daemon, and other clients attached to it, see no change and pay
  no cost.
- **Platforms** (Principle VI): the slide looks and behaves the same on Linux, macOS and Windows,
  with no per-platform branch.
- **At rest**: nothing is redrawn for the rail's sake once it has settled.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Collapsing the rail MUST move it through widths strictly between its labelled width and
  its icon width before it settles at the icon width. Expanding it MUST do the same in reverse.
- **FR-002**: On every frame of a slide, the section's region MUST begin at the rail's right edge,
  with no gap and no overlap, and the section's content MUST keep the offset from that edge it has at
  rest.
- **FR-003**: The rail's slide and the worktree sidebar panel's hide-and-show slide MUST both use the
  duration and curve the design system assigns to the sidebar's slide. Today the sidebar has that
  duration but moves at a constant speed rather than on its assigned emphasized curve, so this
  feature brings the sidebar onto its curve as well. Otherwise the two would not move alike.
- **FR-004**: Pressing the control again before a slide settles MUST reverse it from the width
  already reached. It MUST NOT restart from either end.
- **FR-005**: A rail that appears when Settings opens MUST already be at the state it is in. Only a
  press of the collapse control starts a slide.
- **FR-006**: At rest in either state, the rail MUST have the widths, rows, icons, labels, badges and
  selection marker it has today. This feature changes the time between the states, not the states.
- **FR-007**: A control that has keyboard focus MUST keep it for the whole of a slide in both
  directions, including when the rows change between their labelled and icon-only forms, unless the
  user moves focus or changes section during the slide.
- **FR-008**: On every frame of a slide, a rail control that is not drawn MUST NOT be reachable by
  keyboard, and each rail control that is drawn MUST be reachable exactly once. The section beside
  the rail keeps its keyboard order unchanged, including controls scrolled out of view.
- **FR-009**: On every frame of a slide, pointer hover and press MUST reach only controls drawn under
  the pointer, and the rail MUST draw nothing outside its own bounds.
- **FR-010**: The collapsed or expanded state MUST change the moment the control is pressed. A slide
  in progress MUST NOT delay section selection, saving, cancelling or closing Settings.
- **FR-011**: A settled rail MUST request no further redraws on its own account.
- **FR-012**: The user guide's Settings page MUST describe the rail's collapse control, labelled
  **Collapse** when the rail is expanded and drawn as an expand icon when it is collapsed, and its
  slide. The control itself was never documented when feature 027 added it, so this covers both.
- **FR-013**: On every frame of a slide before it settles, the rail's rows MUST NOT reflow. No label
  wraps, no row changes height, and content wider than the rail's current width is cut off at its
  edge. A row with no icon, which keeps its name when collapsed, is exempt from this requirement:
  it follows the rail's current width, so its name may wrap and its height may change, and its badge
  is not cut off. The rows below such a row move vertically with it; that movement is part of the
  same exemption.
- **FR-014**: An icon in the rail MUST NOT jump because of the slide. Across a slide, each icon's
  horizontal position MUST change continuously from where it sits at rest in the starting state to
  where it sits at rest in the final state. That includes the moment the rows change between their
  labelled and icon-only forms. Today those rest positions differ by up to 13dp. Selecting a section
  mid-slide is the one exception, and only for the rows whose selection changed: their icons move at
  once to their new position for the rail's current width. At rest that move is 12dp expanded and
  0dp collapsed; mid-slide it is in proportion to how far the rail is through its width change.
  Animating that move is a separate feature.
- **FR-015**: On every frame of a slide, a row with a badge MUST show its mark inside the rail's
  bounds: at least part of its badge chip, or its tinted icon.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In both directions, at least one rendered frame between the press and the settled
  state shows the rail at a width strictly between its two states.
- **SC-002**: While frames arrive at least every 64 ms, a slide settles within the sidebar's slide
  duration, counted from the most recent press, plus twice the longest gap between frames during
  the slide. The settings rail and the worktree
  sidebar's panel take the same duration and follow the same curve: sampled at the same elapsed
  times, both have completed the same fraction of their width change. The sidebar's swap to its
  narrow strip is not part of that travel, nor is any stretch where the panel's width is held at the
  strip's.
- **SC-003**: On every frame of a slide in which the user neither moves focus nor changes section,
  focus is on the control the user activated (100% of frames), and the number of keyboard-reachable
  controls equals the count at rest.
- **SC-004**: Across a reversal, the rail's width has no discontinuity. Over any interval, its change
  is no greater than the largest change an uninterrupted slide makes over an interval of the same
  length.
- **SC-005**: With the rail settled in either state, every visible control and region of the
  settings surface has the rectangle it has today.
- **SC-006**: Opening Settings shows the rail at its final width on the first frame.
- **SC-007**: On every frame of a slide, every row of the rail that has an icon, and the collapse
  control, has its rest height. Between
  consecutive frames, each icon in a row whose selection did not change over those frames moves
  sideways by no more than the difference between its two rest positions times the fraction of the
  rail's full width change made over those frames, plus 0.5dp, and never away from the rest position
  the slide is heading for.

## Assumptions

- **"Similar like the worktrees tree view"** means the sidebar's hide-and-show slide: the same
  duration and curve, and the content beside the panel following its edge. It does not mean the
  sidebar's other behaviours, such as its resize handle or hiding the panel entirely. The rail still
  collapses to icons, not to nothing, so every section stays one press away (feature 027, FR-026c).
- **The design system's assignment is authoritative, and the sidebar has drifted from it.** Feature
  018's motion contract (§6.3, *sidebar slide*) gives the sidebar's slide `medium_4` and the
  `emphasized` curve. The sidebar component sets that duration but never its curve, so it runs
  linear, which already falls short of 018 SC-010 (every animation "takes its duration and easing
  from a named motion token"). Matching the rail to that drift would put a second panel off the
  contract. Leaving the
  sidebar alone would make the two panels move differently, the opposite of the request. FR-003
  therefore corrects the sidebar's curve. The curve's slow tail needs two more: the panel's laid-out
  width is floored at its narrow strip's, so the content beside it does not dip past the strip's
  edge, and a closing panel swaps to the strip once it is no wider than it, so no still sliver of
  panel lingers before the strip appears (D20, D21). Those are the only changes to the sidebar.
- **The rail is an instance of the sidebar slide, not a new kind of animation.** Feature 018 capped
  the new animations *it* introduced at four (018 FR-035a, "introduced by this feature"). Its
  contract repeats the cap more strongly: "Four is the count FR-035a and SC-010 both carry, and no
  fifth animation is permitted" (§6.3). Read in place, that sentence counts the four rows 018's own
  new surfaces added, so it bounds 018's scope and does not ban later features. It would not bite
  here in any case: a fifth animation would need a fifth *new* row, a new pairing of trigger, duration and
  easing, and the rail adds none. This feature adds no new motion
  assignment: the rail reuses the *sidebar slide* row. Its plan records that use, and 018's contract
  is not edited.
- **The opening slide 027 described never happened.** Feature 027's view wrapped the rail in the
  sidebar's drawer with a comment saying the rail "slides in with the view". The drawer was pinned
  open and never moved ([BUG-004](../027-sandboxed-daemon-runtime/bugs/BUG-004.md)). FR-005 makes
  the actual behaviour, a rail that does not slide in, the specified one.
- **No reduced-motion preference.** The application has no such setting and does not read one from
  the operating system. Adding one is a separate feature.
- **The rail's state stays view state.** It is not saved to disk, Cancel does not revert it, and it
  lasts as long as the process (feature 027, FR-026d). This feature changes none of that.
- **Scope**: the shared navigation rail component (feature 027, FR-026a) and every screen built on
  it: Settings, and the component showcase's rail example. The worktree sidebar is in scope only for
  its panel's curve (FR-003) and what that curve needs: the width floor and the earlier swap to its
  narrow strip (D20, D21). Other layout changes that snap today, such as the text field's floating label (feature 018, FR-044),
  are out of scope.
- **Dependencies**: feature 027's collapsible rail (FR-026a–e), feature 018's motion tokens (§6.3),
  and feature 017's motion primitive, through which every animation requests its frames (018
  FR-039e).
