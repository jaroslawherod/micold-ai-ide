# Research: Settings accordions form a group

Feature 1091. No `NEEDS CLARIFICATION` remained after clarify (FR-006 resolved: error-held sections
stay open). Paths are workspace-relative.

## R1. The group is a state helper the form owns, not state inside the component

- **Decision**: new `ui/material/accordion_group.rs` with `AccordionGroup<K>`; `AccordionSection`
  stays stateless and gains `in_group(&AccordionGroup<K>, key, default)`, which sets `.open(...)`.
  The caller still supplies `on_toggle` and forwards the key to `group.toggle`.
- **Rationale**: `AccordionSection` states "Whether it is open is the caller's: this keeps no
  state" so a form can force a section open on an error. Sections are separate widgets in a
  `column!`; a widget cannot see its siblings, so exclusivity needs one value shared by all of
  them, and iced's Elm architecture puts that in the caller's state. A render-free helper is
  unit-testable (Principle I).
- **Alternatives rejected**: a group id on each section with a global registry (hidden global state,
  breaks multi-window and multi-session isolation, Principle II); a stateful group widget wrapping
  its sections (cannot express "this one is held open by an error", and daemons build sections in
  different functions per kind); keeping `flipped_sections` and special-casing in `daemons.rs`
  (the issue requires a component capability, FR-001).

## R2. Three states: not chosen, none open, one open

- **Decision**: `enum Choice<K> { Default, Closed, Open(K) }` inside the group. `Default` means the
  caller's default key is open (the kind's first section, FR-007); `reset()` returns to it.
- **Rationale**: pressing the open default section must leave none open (FR-002, Scenario 2), which
  `Option<K>` alone cannot tell from "never touched" (where the default reappears).
- **Alternatives rejected**: seed `Some(first)` when a form opens (several creation sites and the
  runtime change would each need the first section; the three existing `flipped_sections.clear()`
  sites already are the reset points); `BTreeSet` as today (cannot be exclusive).

## R3. Error-held sections are outside the group's rule, in the form

- **Decision**: `State::section_open(s) = group.is_open(s, first) || held_open(s)`;
  `SectionToggled(s)` returns early when `held_open(s)`, otherwise `group.toggle(s, first)`. The
  group knows nothing of errors.
- **Rationale**: spec FR-006 (clarified): errors keep sections open alongside the user's choice;
  890 FR-004. When the error clears, the held section falls back to the group's own choice (A
  closes, B stays, US3 scenario 2) with no extra bookkeeping. `held_open` exists already.
- **Alternatives rejected**: the group takes a `held` predicate (mixes form rules into the shared
  component); the user's open replaces the held section (contradicts 890 FR-004).

## R4. Showcase: a group of three and an independent set

- **Decision**: the `AccordionSection` entry keeps its four poses and live toggle and adds poses
  "a group: opening one closes the others" (live, state in `showcase/state.rs`) and "independent
  sections" (reuses the existing live `Reversed` pattern, one per section).
- **Rationale**: FR-008 and the catalogue completeness gate; the existing pattern is kept.
- **Alternatives rejected**: a separate catalogue entry for the group (it has no widget of its
  own; the gate lists components by module).

## R5. Documentation

- **Decision**: one sentence in each daemon user-guide page that describes sections, saying one
  section is open at a time (a section with an error stays open too).
- **Rationale**: Principle VII; the behaviour is user-visible. Pages found by `grep -n -i section`
  in `docs/user-guide/`.
