# Tasks: Settings accordions form a group: opening one closes the others

**Input**: `specs/1091-settings-accordion-group/` (plan.md, spec.md, research.md, data-model.md, contracts/ui.md, quickstart.md)

**Tests**: Mandatory (Constitution I): each test task comes before the implementation it covers.

**Documentation**: Constitution VII: the user-guide task sits in the milestone that ships the daemon-form behaviour.

## Format: `[ID] [P?] [Story] Description`

Paths are under `crates/micold-client/` unless they start with `docs/` or `specs/`. No Setup phase: no new dependency, crate or config.

## Phase 1: Foundational (blocks all stories)

**Purpose**: the render-free group and the section's opt-in hook.

- [x] T001 [P] Write failing unit tests in `src/ui/material/accordion_group.rs` (new file, `#[cfg(test)]` only at first): default key open when nothing pressed, open replaces, pressing the open key closes to none, reopen after close, `reset()` returns to the default, two groups independent (FR-001, FR-002, FR-003)
- [x] T002 [P] Write failing unit test in `src/ui/material/accordion_section.rs`: `in_group(&group, key, default)` sets `open` from the group; a section never passed to `in_group` is unchanged (FR-001, FR-003)
- [x] T003 Implement `AccordionGroup<K: Copy + Eq>` with the `Default | Closed | Open(K)` choice (`new`, `reset`, `open_key(default: Option<K>)`, `is_open(key, default: Option<K>)`, `toggle(key, default: Option<K>)`) in `src/ui/material/accordion_group.rs`; add `mod` and `pub use` in `src/ui/material/mod.rs` (data-model.md)
- [x] T004 Implement `AccordionSection::in_group` in `src/ui/material/accordion_section.rs`

**Checkpoint**: `cargo test -p micold-client accordion` passes.

## Phase 2: User Story 1 + User Story 3 - Daemon sections form a group, errors stay visible (Priority: P1 / P2) 🎯 MVP

**Goal**: in a daemon form at most one non-error-held section is open; a section holding a shown error is never hidden.

**Independent Test**: open a daemon form, press a closed section's header: it opens and the first closes; leave a required field empty, open another section: both stay open; fix the field: the error section closes.

US3's rule (FR-006) is the same `State` change as US1's, so the two ship together (milestones.md rule 7).

### Tests (write first, must fail)

- [x] T005 [P] [US1] Reducer tests in `tests/daemons_feature.rs`: for every kind (`Section::of`), opening B closes A, pressing the open section leaves none, opening from none opens only it, a form opening and a `RuntimeChosen` kind change leave exactly the first section open, two forms in turn do not leak, a rapid sequence of presses ends as the same presses one by one, no draft leaves no section open, the draft is unchanged after toggles (extend `sections_open_by_toggle_and_by_error_without_touching_the_draft`) (FR-002, FR-004, FR-005, FR-007)
- [x] T006 [US3] Reducer tests in `tests/daemons_feature.rs`: a held (error) section stays open when another opens, a press on it changes nothing, several held sections stay open together, and when the error is fixed the held section closes and only the user's section stays open; with the group still at its default, an error in a non-first section opens beside the first and closing it leaves the first (FR-006, SC-004)

### Implementation

- [x] T007 [US1] In `src/features/daemons.rs` replace `flipped_sections: BTreeSet<Section>` with `sections: AccordionGroup<Section>`; `section_open(s)` = `sections.is_open(s, first) || held_open(s)`; `Msg::SectionToggled` ignores a held section, else `sections.toggle(s, first)`; convert all four existing `flipped_sections.clear()` sites (three form openings, the `RuntimeChosen` arm) to `sections.reset()` (FR-004, FR-006, FR-007)
- [x] T008 [US1] Update `tests/daemons_section_geometry.rs`: its `open_form` helper, which opens every section after the first, becomes one measurement per section so each is still laid out open; confirm `src/features/container_fields.rs` and `src/ui/settings/daemons.rs` need no change beyond `state.section_open(section)` (FR-009)
- [x] T009 [P] [US1] Add a sentence that one section is open at a time to `docs/user-guide/sandboxed-daemon.md`, `docs/user-guide/kubernetes-daemons.md` and `docs/user-guide/remote-daemons.md`

**Checkpoint**: `mise run gate` green; quickstart Part B steps 2, 3 and 5 hold.

## Phase 3: User Story 2 - The component offers group mode as an opt-in (Priority: P2)

**Goal**: the showcase shows a grouped and an independent set of `AccordionSection`s.

**Independent Test**: in the showcase, opening a section in the group closes the other; sections of the independent set open and close on their own.

- [x] T010 [US2] Failing reducer test in `tests/showcase_state.rs`: `Message::GroupSectionToggled(i)` is exclusive on the group and leaves the independent set's toggles untouched (FR-001, FR-003, FR-008)
- [x] T011 [US2] Add `group: AccordionGroup<usize>` and `Message::GroupSectionToggled(usize)` (calls `group.toggle(i, Some(0))`, the first section open at rest) and `Message::IndependentSectionToggled(usize)` for the independent set in `src/showcase/state.rs`
- [x] T012 [US2] Render the grouped set and the independent set in `src/showcase/sections/surfaces.rs` (`accordion_section`), and add the poses "a group: opening one closes the others" and "independent sections" in `src/showcase/catalogue.rs`; `tests/showcase_completeness.rs` and `tests/showcase_captions.rs` stay green and fail until the poses exist (FR-008)

**Checkpoint**: `mise run gate` green; quickstart Part B step 1 holds.

## Phase 4: Polish

Quickstart Part B steps 1 to 5 (visual pass, step 4 included) and ticking results change no code: left to the close unit, not a milestone.

## Dependencies and order

Phase 1 before Phases 2 and 3. Phase 3 needs only Phase 1; it ships after Phase 2. Within a phase, tests (T001/T002, T005/T006, T010) before the implementation they cover. `[P]` tasks touch disjoint files; T006 follows T005 because both edit `tests/daemons_feature.rs`.

## Implementation Strategy

MVP first: Phase 1 + Phase 2 ship the issue's behaviour in the daemon form (M1). The showcase (M2) follows.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Daemon sections form a group 🎯 MVP

- **Tasks**: T001–T009
- **Deliverable**: in a Settings > Daemons form, opening a section closes the one that was open, pressing the open section closes it, and a section holding a shown error stays open.
- **Satisfies**: US1 acceptance scenarios 1–5; US3 acceptance scenarios 1–2; FR-001–FR-007, FR-009; SC-001–SC-004 (SC-003 by the existing section and geometry tests staying green)
- **Verify**: `cargo test -p micold-client --test daemons_feature --test daemons_section_geometry` (T005/T006 cover the error-held rule) and `cargo test -p micold-client accordion` pass; quickstart §B steps 2, 3, 5 (step 4, both themes, runs at close)
- **Depends on**: —
- **Tier**: full

### M2 — Showcase shows grouped and independent sections

- **Tasks**: T010–T012
- **Deliverable**: the component showcase's `AccordionSection` entry shows a group where opening one closes the others, beside an independent set that keeps each section's own state.
- **Satisfies**: US2 acceptance scenarios 1–3; FR-008; SC-003
- **Verify**: `cargo test -p micold-client --test showcase_state --test showcase_completeness`; quickstart §B step 1
- **Depends on**: M1
- **Tier**: light

## Phase 5: TDD remediation

The feature is not done until the HIGH finding (T013) and the MED survivors (T014-T017) are cleared. Findings refer to `tdd/verification.md`. Paths are under `crates/micold-client/`.

- [x] T013 [US1] Finding 1: write `specs/1091-settings-accordion-group/tdd/test-list.md` and `cycle-log.md`, recording per behavior the red (apply the verification's matching mutant, quote the failing test, restore); proof: `tdd/verification.md` re-run shows no `TEST_AFTER` (files written; earlier behaviors remain TEST_AFTER, red is retroactive and mutant-based only, see tdd/cycle-log.md; the "no TEST_AFTER" proof is not met)
- [x] T014 [US1] Findings 2 and 3 in `tests/daemons_feature.rs`: a stale `SectionToggled` for a section outside the open kind changes nothing (`src/features/daemons.rs:1904`), and `begin_edit` on a second daemon with a flipped section leaves only the first section open (`:862`); proof: `cargo test -p micold-client --test daemons_feature` fails with mutants A+B and F applied, passes restored
- [x] T015 [US2] Finding 4 in `tests/showcase_state.rs:131-156`: independent sections close and reopen on their own (toggle index 1 and index 0 twice); proof: `cargo test -p micold-client --test showcase_state` fails with `*open = true` mutant
- [x] T016 [US2] Finding 5: a render test that paints the grouped pose and asserts one open body by default and the group's state after a toggle (`src/showcase/sections/surfaces.rs:278`); proof: `cargo test -p micold-client --test showcase_completeness` (or a new `showcase_*` test) fails with `.open(true)` mutant
- [x] T017 [US1] Findings 6 to 9 in `tests/daemons_feature.rs:1801-1944`: remove the self-comparing rapid-press block, split `the_sections_of_every_kind_form_one_group`, assert literal expected sections, assert `held.len() >= 2`; proof: `cargo test -p micold-client --test daemons_feature` green
