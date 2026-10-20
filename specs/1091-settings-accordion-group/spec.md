# Feature Specification: Settings accordions form a group: opening one closes the others

**Feature Branch**: `feat/1091_settings-accordions-should-form-a-group-opening`

**Created**: 2026-10-19

**Status**: Closed 2026-10-20 — shipped in PRs #1135, #1140, #1142

**Input**: Issue #1091, "Settings accordions should form a group: opening one closes the others":
the accordion sections in Settings should behave as one group, so at most one is open at a time.
Sections in the same settings page (for example the Daemons sections from #890) share a group.
Opening a section collapses the previously open one; clicking the open section collapses it,
leaving none open. The accordion component supports this as an opt-in group (exclusive) mode, so
other uses stay independent. Related: #890 (grouped daemon settings in an accordion).

## Context: what exists

Feature 890 put each daemon's settings in titled collapsible sections (the shared
`AccordionSection`). Each section's open state is independent: the kind's first section starts open,
the user flips any section, and a section holding a shown validation error is forced open (890
FR-004). With the sections independent, a long daemon form can end up with several sections open
and need a lot of scrolling.

## Clarifications

### Session 2026-10-19

- Q: When an error-held section is open and the user opens another section, do both stay open, or does the user's section replace it? → A: Both stay open; the group is exclusive only among non-error-held sections. 890 FR-004 says a section holding a shown error MUST be open, and 890 edge cases already allow several error sections open at once. _(agent-resolved: specs/890-daemon-settings-accordion/spec.md#FR-004)_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Opening a daemon section closes the one that was open (Priority: P1)

A user editing a daemon in Settings › Daemons opens another section and sees the previously open
section close, so the form stays short and the section in use is the only one expanded.

**Why this priority**: This is the issue's whole point and the behaviour the user sees.

**Independent Test**: Open a daemon form (first section open), press the header of a closed section,
and check that it is open and the first section is closed.

**Acceptance Scenarios**:

1. **Given** a daemon form with section A open and B closed, **When** the user opens B, **Then** B
   is open and A is closed.
2. **Given** section B open and no error holding it open, **When** the user presses B's header,
   **Then** B closes and no section in the form is open.
3. **Given** no section open, **When** the user opens any section, **Then** that section is the
   only one open.
4. **Given** any of the above, **When** the user opens or closes sections, **Then** no field value,
   draft or saved setting changes (890 FR-005 still holds).
5. **Given** the keyboard, **When** the user toggles a header with Space or Enter, **Then** the group
   behaves exactly as with the pointer.

---

### User Story 2 - The component offers group mode as an opt-in (Priority: P2)

A developer can place several `AccordionSection`s in one group so that opening one closes the
others, or leave them independent as today.

**Why this priority**: The issue requires the behaviour to be a component capability, not daemon
code, so other uses can stay independent.

**Independent Test**: In the component showcase, a group of sections behaves exclusively and an
ungrouped set of sections still opens and closes each on its own.

**Acceptance Scenarios**:

1. **Given** sections not placed in a group, **When** the user opens one, **Then** the others keep
   their state, exactly as before this feature.
2. **Given** two separate groups on one screen, **When** a section opens in one, **Then** the other
   group is unaffected.
3. **Given** the component showcase, **When** it is opened, **Then** it shows both the grouped and
   the independent behaviour.

---

### User Story 3 - A shown error is never hidden by the group (Priority: P2)

A user whose save was refused, or who left a required field empty, still sees the section holding
the error, even though only one section is normally open.

**Why this priority**: 890 FR-004 promises an error is never hidden; the group must not break it.

**Independent Test**: Leave a required field empty in a closed section, then open another section,
and check the section with the error is still visible.

**Acceptance Scenarios**:

1. **Given** a section holding a shown error, **When** the user opens a different section, **Then**
   the error section stays open (see FR-006).
2. **Given** the error section A was kept open while the user opened B, **When** the error is
   fixed, **Then** A closes and only B, the section the user last opened, stays open.

### Edge Cases

- A form opens, or *Runs in* changes: the kind's first section is open and the others closed
  (890 FR-004 unchanged).
- Several sections hold shown errors at once: all stay open; opening another section does not close
  them (FR-006).
- The only section is pressed: it closes, leaving none open.
- An error-held section's header is pressed: nothing changes.
- No daemon is selected or the draft is gone: no section is open and nothing breaks.
- Two daemon forms edited in turn (multi-session, Principle II): one form's open section does not
  leak into the other's.
- Fast repeated presses on headers: the final state is the same as pressing them one by one; at most
  one section (plus error-held ones) is open once the reveal animations finish.
- The reveal animation of the closing and the opening section runs together without the form
  jumping or overlapping content; behaviour is the same on every platform and in both themes.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The titled collapsible section MUST offer an opt-in group mode: sections placed in the
  same group are exclusive, so opening one closes any other open section of that group.
- **FR-002**: Pressing the header of the open section of a group MUST close it, leaving no section
  of the group open, unless a shown error holds that section open (FR-006), when the press changes
  nothing.
- **FR-003**: Sections not placed in a group MUST behave exactly as before this feature, and
  separate groups MUST NOT affect each other.
- **FR-004**: All sections of one daemon form (every kind, 890 FR-002, including the container
  sections) MUST be in one group.
- **FR-005**: Opening or closing sections MUST NOT change any field value, the draft or anything
  saved, and MUST work the same with the pointer and with Space or Enter on a focused header.
- **FR-006**: A section holding the field of a shown validation error MUST stay open whatever the
  group does (890 FR-004). The group is exclusive only among sections not held open by a
  shown error: opening another section leaves error-held sections open.
- **FR-007**: A form opening or *Runs in* changing MUST leave exactly the kind's first section open
  (890 FR-004), unless error-held sections are also open.
- **FR-008**: The component showcase MUST show a grouped set and an independent set of sections.
- **FR-009**: Collapsed sections MUST keep offering no focus stops, and a closed section MUST keep
  showing its summary and required marker (890 FR-011, FR-012).

### Key Entities

- **Accordion group**: a set of sections on one screen of which at most one is open (plus
  error-held ones, FR-006). A section belongs to no group or one group.
- **Section open state**: whether a section is shown open; today decided per section by the first
  section default, the user's toggles and shown errors.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In every daemon form, after any sequence of header presses with no shown error, at
  most one section is open.
- **SC-002**: 100% of header presses on the open, non-error-held section of a group close it, and
  100% of presses
  on a closed section open it and close the previously open section unless a shown error holds it open.
- **SC-003**: Sets of sections outside a group (including the showcase's independent set) behave
  identically before and after this feature, in 100% of the existing section tests.
- **SC-004**: A shown validation error's section is open in 100% of the states in which it was open
  before this feature.

## Assumptions

- "Settings" here means the Daemons settings, the only Settings page that uses accordion sections
  today (the component showcase also does); any later settings page opts in the same way.
- The daemon form's first-section-open default and error-held rule (890) stay.
- Closing a section does not remember which was open before; reopening is a fresh press.
- Out of scope: the sidebar tag-filter `Accordion` panel and other uses of the downward-revealing
  panel; persisting which section is open across restarts; new section content or look.
