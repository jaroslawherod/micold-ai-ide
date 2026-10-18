# Tasks: Overview Card Ripple

**Input**: `specs/971-overview-card-ripple/` (spec.md, plan.md, research.md, data-model.md, contracts/ui-contract.md, quickstart.md)

**Tests**: mandatory (Constitution I): each test task precedes the code it covers and must fail first.

**Documentation**: `docs/user-guide/overview.md` is updated in the same milestone as the behaviour (Constitution VII).

All code is in `crates/micold-client/src/` (paths below are relative to it, except the `crates/micold-client/tests/` ones).

## Format: `[ID] [P?] [Story] Description`

## Phase 1: Setup

- [ ] T001 Read `ui/material/ripple.rs`, `ui/material/overview_card.rs`, `features/overview.rs` and `ui/material/ripple_clipping.rs` test harnesses (`test_support`, Mounted) and note the helpers to reuse; no code change.

## Phase 2: Foundational

- [ ] T002 [P] Add `OVERVIEW_ACTIVATION_DELAY = Duration::from_millis(duration::MEDIUM_2)` constant beside the Overview feature state in `features/overview.rs`, with a unit test asserting it equals the ripple's EXPAND token.

## Phase 3: User Story 1 — Pointer press ripples from the press point (P1) 🎯 MVP, with User Story 3 — ripple visible before the Overview closes (P1)

**Goal**: a pressed card ripples from the press point, clipped to its rounded shape, and an activation holds the Overview open 300 ms before it closes.

**Independent Test**: showcase card press shows the ripple; in the client a click shows the ripple, then the main session ~300 ms later.

### Tests (write first, see them fail)

- [ ] T003 [P] [US1] Test in `ui/material/overview_card.rs`: pressing an activatable card at its four corners and centre starts a ripple whose origin is within 1 px of the press (clamped), in each of the five states (FR-001, SC-001, US1-1/3/4); the ripple tint equals the state's content colour at the pressed opacity (US1-4).
- [ ] T004 [US1] Test in `ui/material/overview_card.rs`: a plain card press leaves no ripple (FR-006); a secondary-button press starts none; a pointer press draws no focus layer once the ripple fades (US1-6, FR-004).
- [ ] T005 [P] [US1] Test in `ui/material/ripple_clipping.rs`: card-shaped, RADIUS 12 rasterised case draws no ripple pixel outside the rounded shape (FR-003, SC-003).
- [ ] T006 [US1] Tests in `ui/material/overview_card.rs` and `crates/micold-client/tests/ripple_state.rs`: two cards ripple independently; a second press restarts from the new origin; no frames once faded; the card's EXPAND and FADE equal the button's tokens; a press then drag off then release does not activate while the ripple fades; a press in a rounded corner outside the shape ripples from the press point; a state change or removal mid-ripple keeps the starting colour / leaves nothing animating (FR-003, FR-005, SC-004, edge cases).
- [ ] T007 [P] [US3] Tests in `features/overview.rs`: `request` accepts when open and nothing is pending and stores the activation; a second `ActivationRequested`, `Closed` and `Toggled` while pending are ignored; `ProjectSwitched` clears it; `due` takes the activation once and then returns `None`; a `Closed` inside the delay is dropped (FR-007).
- [ ] T008 [P] [US3] Test in `ui/overview.rs`: a card's press emits `ActivationRequested` carrying its `activation`; the existing feature 813 `CardActivated` tests in `crates/micold-client/tests/overview.rs` are left unchanged and still pass (FR-008, SC-006).

### Implementation

- [ ] T009 [US1] In `ui/material/overview_card.rs`, wrap the activatable branch in `Ripple::new(focusable, p.text, RADIUS)`; leave the plain branch unwrapped (D1; T003–T006).
- [ ] T010 [US3] In `features/overview.rs`, add `State.activating`, the pure `request()` and `due()`, `Msg::ActivationRequested`/`Msg::ActivationDue`, the update gating and the `ProjectSwitched` clear (D3; T007).
- [ ] T011 [US3] In `ui/overview.rs`, the card `on_press` emits `ActivationRequested(activation)` (D3; T008).
- [ ] T012 [US3] In `main.rs::overview_message`, start the `OVERVIEW_ACTIVATION_DELAY` timer when `request` returned true, and on `ActivationDue` call `due()` and, on `Some`, handle `Msg::CardActivated` through the existing path unchanged (D3; FR-008).
- [ ] T013 [US1] In `showcase/sections/surfaces.rs`, give the sample Overview cards an inert `on_press` so the pointer ripple shows in the showcase (D4, FR-009, pointer half).
- [ ] T014 [US1] Update `docs/user-guide/overview.md`: clicking a card shows a ripple from the press point and the Overview closes about 0.3 s later.
- [ ] T015 Run the full gate (`scripts/gate.sh`; `mise run gate` is unavailable in the cloud); then quickstart §B steps 1, 3 and 4 (pointer part) through the `visual-pass` skill.

**Checkpoint**: M1 deliverable works on `main`.

## Phase 4: User Story 2 — Keyboard activation ripples from the centre (P2)

**Goal**: Enter or Space on a focused card ripples from the centre.

**Independent Test**: showcase, Tab to a card, Enter then Space: centred ripple each; unfocused card none.

### Tests (write first, see them fail)

- [ ] T016 [US2] Tests in `ui/material/overview_card.rs`: Enter and Space on a focused card start a ripple at the centre; an unfocused card shows none (FR-002, SC-002, US2-1/2/3).
- [ ] T017 [P] [US2] Tests in `ui/material/ripple.rs`: with `centred_on_keys`, a captured Enter/Space starts a centred press; auto-repeat while running does not restart it; without the builder nothing changes for other controls; a focused-card case keeps the ripple the only state layer (FR-004, FR-005).

### Implementation

- [ ] T018 [US2] In `ui/material/ripple.rs`, add the `Ripple::centred_on_keys` builder and the post-child key handling (D2; T016, T017).
- [ ] T019 [US2] In `ui/material/overview_card.rs`, enable `.centred_on_keys()` on the card's ripple (D2; completes FR-009 keyboard half in the showcase).
- [ ] T020 [US2] Update `docs/user-guide/overview.md`: Enter or Space on a focused card ripples from the centre.
- [ ] T021 Run the full gate (`scripts/gate.sh`); then quickstart §B step 2 and step 4 (keyboard part) through the `visual-pass` skill, saving evidence in the spec directory.

## Dependencies & Execution Order

T001 → T002 → Phase 3 tests (parallel) → Phase 3 implementation → T015 → Phase 4. T009 needs T003–T006; T010 needs T002 and T007; T011 needs T008 and T010; T012 needs T010 and T011. T018 needs T016, T017 and T009.

## Implementation Strategy

M1 ships the pointer ripple together with the delay (both P1), because without the delay the ripple is invisible in the real Overview. M2 adds the keyboard origin.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Pointer ripple and delayed close 🎯 MVP

- **Tasks**: T001–T015
- **Deliverable**: clicking an Overview card ripples from the press point, clipped to the card, and the Overview closes 300 ms later with the outcome unchanged; the showcase card sample shows the pointer ripple.
- **Satisfies**: US1 acceptance scenarios 1–6; US3 acceptance scenarios 1–3; FR-001, FR-003, FR-004, FR-005, FR-006–FR-008, FR-009 (pointer); SC-001, SC-003–SC-006; US3-1 and SC-005 for pointer activation
- **Verify**: `scripts/gate.sh` green with the new `overview_card`, `ripple_clipping`, `ripple_state` and `features::overview` tests; quickstart §B steps 1, 3, 4 (pointer).
- **Depends on**: —
- **Tier**: full

### M2 — Keyboard ripple from the centre

- **Tasks**: T016–T021
- **Deliverable**: Enter or Space on a focused Overview card ripples from the card's centre, in the showcase and before the Overview closes.
- **Satisfies**: US2 acceptance scenarios 1–3; FR-002, FR-004, FR-005, FR-009 (keyboard); SC-002; US3-1 and SC-005 for keyboard activation
- **Verify**: `scripts/gate.sh` green with the new `ripple` and `overview_card` keyboard tests; quickstart §B step 2 and step 4 (keyboard).
- **Depends on**: M1
- **Tier**: full
