---

description: "Tasks: Overview card focus and expansion motion"
---

# Tasks: Overview Card Focus and Expansion Motion

**Input**: Design documents from `/specs/1299-overview-card-focus-motion/`

**Prerequisites**: plan.md (D1–D7, Test strategy), spec.md, research.md (R1–R8), data-model.md,
contracts/ui-contract.md (C1–C5), quickstart.md

**Tests**: Constitution Principle I: every test task is written first and run, and must fail for the
stated reason (missing item, or today's one-step output) before the implementation task it covers.
Client tests: `scripts/build-lock.sh cargo test -p micold-client --lib -- <filter>` and
`scripts/build-lock.sh cargo test -p micold-client --test <name>`. Timed tests send
`RedrawRequested(origin + 16 ms × i)` ticks, never `Instant::now()`, so they do not depend on the
machine's speed.

**Documentation**: Principle VII: each milestone's user-guide task ships in that milestone
(`docs/user-guide/settings.md` *Reduce motion*, `docs/user-guide/overview.md`), so CI's user-guide
gate passes on every `feat` PR.

**Cross-platform**: Principle VI: no platform code; durations are wall-clock, the OS reduced-motion
preference is not read.

**Reduce motion travels with each motion** (US3): nothing that eases reaches `main` without
*Reduce motion* making it instant, so US3's tasks are split between the focus phase (focus half)
and the lift phase (shadow half, and the full FR-012 test).

**Organization**: by user story, in implementation order. US4 ("unchanged") has no phase of its
own: its checks run in T011 and T023 and its scenarios are asserted where noted.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1–US4)

---

## Phase 1: Setup (Shared Infrastructure)

None: no new crate, dependency or fixture.

---

## Phase 2: Foundational (Blocking Prerequisites)

- [x] T001 In `crates/micold-client/src/ui/material/test_support.rs`, add `Mounted::tick(&mut self, at: Instant, cursor: mouse::Cursor) -> (Vec<String>, RedrawRequest)` (one `RedrawRequested(at)` through `update`, returning what was published and `shell.redraw_request()`) and `Mounted::rebuild(&mut self, element)` (replace the element and `diff` the existing tree, keeping widget state); replace the two hand-written copies of the tick in `crates/micold-client/src/ui/material/card_grid.rs` `mod tests` (the `asked` closures in `a_rest_on_a_link_asks_for_no_frame_once_it_is_over` and `a_rest_on_a_number_asks_for_no_frame_once_it_is_over`) with `tick`, their assertions unchanged

**Checkpoint**: timed widget tests can step frames at chosen instants and read whether a next frame
was asked.

---

## Phase 3: User Story 1 - Keyboard focus eases onto and off a card (Priority: P1) 🎯 MVP

**Goal**: the focus outline and focus layer of an Overview card fade in and out over 150 ms on
`STANDARD` (FR-001, FR-002, FR-004, FR-005, FR-009, FR-010), and *Reduce motion* makes that instant
(FR-007, focus half). The expanded card's shadow is unchanged (still one step) until Phase 4.

**Independent Test**: `scripts/build-lock.sh cargo test -p micold-client --lib -- keyboard_focus overview_card card_grid`.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

- [x] T002 [P] [US1] In `crates/micold-client/src/ui/material/keyboard_focus.rs` `mod tests`, add failing tests for a control built `.fade(Some(Duration::from_millis(150)))`, stepping `Mounted::tick` 16 ms apart from an origin: after `focus_next`, the drawn strength (the `Focus` state's `fade.value()`) is `0 < s < 1` at the tick nearest 75 ms and `1.0` by 150 ms + 16 ms; after focus moves away it falls to `0.0` the same way (FR-001, FR-002, SC-001); a reversal at ~75 ms changes the strength by at most one step on the next tick and reaches its end within 150 ms + 16 ms of the reversal (FR-004); two faded controls, Tab from one to the other: one falling and one rising on the same ticks (FR-005); the tick after the end returns `RedrawRequest::Wait` (FR-009, SC-003)
- [x] T003 [US1] In `crates/micold-client/src/ui/material/keyboard_focus.rs` `mod tests`, add failing tests: a pointer press on a faded control whose focus is shown gives strength `0.0` on the next tick (D1, FR-011); a faded control at strength 1 draws the same pixels (`drawn`) as the same control without `fade` (FR-010); a faded wrapper with `on_focus_change` built already focused and shown starts at `1.0` and asks no frame (R8); the existing ring and state-layer tests stay unmodified (a control without `fade` is unchanged, C1)
- [x] T004 [P] [US1] In `crates/micold-client/src/ui/material/overview_card.rs` `mod tests`, add failing tests: `CARD_MOTION == Duration::from_millis(duration::SHORT_3)`; an activatable card's focus, after `focus_next`, is part-way at the tick nearest 75 ms (it is built with `.fade(Some(CARD_MOTION))`); an `.instant(true)` card is fully indicated on the first tick after `focus_next` and that tick returns `RedrawRequest::Wait` (FR-007); the issue and pull-request links' focus rings are not faded (US4-3, FR-011)
- [x] T005 [P] [US3] In `crates/micold-client/src/ui/material/card_grid.rs` `mod tests`, add failing tests: a `CardGrid` built `.instant(true)` shows a card's focus at full on the first tick after Tab onto it and at none on the first tick after Tab away, with `RedrawRequest::Wait` each time (US3-1, FR-007); a default grid's card is part-way at ~75 ms (the flag reaches the cards); a grid mid-fade `Mounted::rebuild` with `.instant(true)` is at its end state on the next tick (FR-008)

### Implementation for User Story 1

- [x] T006 [US1] In `crates/micold-client/src/ui/material/keyboard_focus.rs` (D1, C1): `Focus` gains `fade: Progress` on `STANDARD` with a written-out `Default` at `0.0`; `state()` starts it at `1.0` for a wrapper with `on_focus_change` built focused and shown; `TakesTheKeyboard::fade(self, over: Option<Duration>) -> Self`, default `None`; at the end of `update`, target = `shown` as `0.0`/`1.0`: `fade.on_frame(event, target, over, shell)` with `Some(over)`, `fade.restart_at(target)` with `None`; the press branch that sets `visible = false` calls `fade.restart_at(0.0)`; `draw` takes strength = target without `fade`, else `fade.value()`, multiplies today's state-layer top-up (`Focus::indicator`'s body, computed whether or not the focus is shown) and the ring colour's alpha by it, and draws nothing at `0.0`; T002 and T003 pass
- [x] T007 [US1] In `crates/micold-client/src/ui/material/overview_card.rs` (D2, C2): `pub(crate) const CARD_MOTION: Duration = Duration::from_millis(duration::SHORT_3)`; `pub fn instant(self, instant: bool) -> Self`, default `false`; the activatable card's `TakesTheKeyboard` is built `.fade((!instant).then_some(CARD_MOTION))`; the links' focusables are untouched; nothing else in the card's face changes (#1298 is in parallel); T004 passes
- [x] T008 [US3] In `crates/micold-client/src/ui/material/card_grid.rs` (D3, C3): `Grid` gains `instant`; `pub fn CardGrid::instant(self, instant: bool) -> Self`, default `false`; `into_grid` builds each card `.instant(instant)`; T005 passes
- [x] T009 [US3] In `crates/micold-client/src/ui/overview.rs` `view` (D4, C5): `.instant(state.settings.reduce_motion)` on the `CardGrid` (the saved setting, the same flag `drawer_flags` reads)
- [x] T010 [US1] In `docs/user-guide/settings.md` *Reduce motion*: it also makes the Overview cards' focus outline appear and disappear at once instead of fading over 150 ms; in `docs/user-guide/overview.md` beside the existing *Reduce motion* line: the focus outline fades in and out (FR-013)
- [x] T011 [US4] Run `scripts/build-lock.sh cargo test -p micold-client --lib -- ripple overview_card card_grid keyboard_focus menu` and `--test idle_requests_no_frames --test motion_tokens --test overview --test overview_card_menu`; `git diff origin/main -- crates/micold-client` shows no edited line inside an existing test body except T001's `tick` replacement (SC-004, US4-1, US4-2, US4-4)

**Checkpoint**: in the app, Tab and the arrows fade a card's focus in and out; with Reduce motion
on it is instant. The expanded shadow still switches in one step.

---

## Phase 4: User Story 2 - An expanded card's elevation eases in and out (Priority: P1)

**Goal**: the grid draws each card's shadow, eased per location over `CARD_MOTION` on `STANDARD`,
with a settling card drawn above its neighbours until its shadow is gone (FR-003, FR-004, FR-005,
FR-009, FR-010, SC-005); *Reduce motion* makes it instant (FR-007, FR-008, FR-012, shadow half);
the showcase shows both motions (FR-013). The growth is not animated (FR-006, D7).

**Independent Test**: `scripts/build-lock.sh cargo test -p micold-client --lib -- style card_lift card_grid overview_card`.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T012 [P] [US2] In `crates/micold-client/src/ui/material/style.rs` `mod tests`, add failing tests (C4): `eased_shadow(r, elevation::MENU, 1.0) == elevation_shadow(r, elevation::MENU)`; `eased_shadow(.., 0.0) == Shadow::default()`; at `0.5` the colour alpha, `offset.y` and `blur_radius` are half the level's; strength outside `0..=1` is clamped
- [ ] T013 [P] [US2] Create `crates/micold-client/src/ui/material/card_lift.rs` with a failing `mod tests` (D3): `Lifts::step` with location A expanded, ticks 16 ms apart: `strength(A)` is `0 < s < 1` at the tick nearest 75 ms and `1.0` by 150 ms + 16 ms; A collapsed: back to `0.0` the same way and its entry dropped at rest (SC-001, both directions, FR-003); a reversal at ~75 ms continues from its value and ends within 150 ms + 16 ms (FR-004); A collapsing while B expands: A falls and B rises on the same ticks, `settling()` yields A only (FR-005, US2-3); `Lifts::seeded(Some(&A))` gives `strength(A) == 1.0` with no frame asked (R8); a location missing from `present` is dropped (Edge Cases); `instant` sets every strength to its target on the first step with `RedrawRequest::Wait` (FR-007); the tick after every entry rests returns `RedrawRequest::Wait` (FR-009); expanded but not yet stepped gives `0.0`
- [ ] T014 [P] [US2] In `crates/micold-client/src/ui/material/card_grid.rs` `mod tests`, add failing tests: an expanded card at rest paints the same pixels (`drawn`) below and beside it as feature 1121's card-drawn menu shadow did (FR-010); collapsing a card in the middle of a 3×3 grid, mid-transition its shadow shows over the card below and to its right (not cut by them) and the card is drawn after its neighbours (FR-003 draw order); the slot rectangles of every card are equal on every tick of an expand and a collapse (SC-005, US2-5); a grid whose state is created with a card expanded draws it at full shadow on its first frame (R8); while a zoom runs the lifts end at once (C3); `focus_next` onto a card in a default grid, which expands it: at the tick nearest 75 ms both the ring strength and the lift strength are `0 < s < 1`, and both reach `1.0` by 150 ms + 16 ms (US2-4)
- [ ] T015 [US3] In `crates/micold-client/src/ui/material/card_grid.rs` `mod tests`, add **the Reduce motion test** (FR-012, SC-002): with `.instant(true)`, on the first tick after gaining and after losing the keyboard focus the card's ring is at full / none, and after expanding and collapsing (hover, focus, Escape) its shadow is full / none, each tick returning `RedrawRequest::Wait`; and a grid mid-lift `Mounted::rebuild` with `.instant(true)` draws the end shadow on the next tick (FR-008, US3-3)
- [ ] T016 [P] [US2] In `crates/micold-client/src/ui/material/overview_card.rs` `mod tests`, add a failing test: an expanded card built `.shadow_by_owner()` paints nothing outside its bounds; `an_expanded_card_casts_the_menu_shadow` stays unmodified (C2)

### Implementation for User Story 2

- [ ] T017 [P] [US2] In `crates/micold-client/src/ui/material/style.rs`: `pub(crate) fn eased_shadow(r: Roles, level: u8, strength: f32) -> Shadow`, scaling the level's colour alpha, `offset.y` and `blur_radius` by `strength.clamp(0.0, 1.0)` (R4); T012 passes
- [ ] T018 [US2] In `crates/micold-client/src/ui/material/card_lift.rs` and `mod card_lift;` in `crates/micold-client/src/ui/material/mod.rs`: `pub(super) struct Lifts(Vec<(SessionLocation, Progress)>)` on `STANDARD` over `overview_card::CARD_MOTION`, with `seeded`, `step(&mut self, event, expanded, present, instant, shell)`, `strength(at, expanded, instant) -> f32` and `settling()` as in data-model.md; an entry exists only while off rest; T013 passes
- [ ] T019 [US2] In `crates/micold-client/src/ui/material/overview_card.rs`: `RADIUS` becomes `pub(crate)`; `pub(crate) fn shadow_by_owner(self) -> Self` makes the expanded fill container cast no shadow (the `if card.expanded` shadow branch only); T016 passes
- [ ] T020 [US2] In `crates/micold-client/src/ui/material/card_grid.rs` (D3): `ZoomState` gains `lifts: Lifts`, seeded in `Grid::state()` from the expanded card; `Grid` gains `roles`; `into_grid` builds each card `.shadow_by_owner()`; `update` calls `lifts.step(...)` on every event beside `observe_rest`, with the expanded location and the cards' locations, `instant` forced while `state.ends.is_some()`; `draw` in three passes: every piece as today except the expanded and settling cards, then each settling card whole at its slot after its shadow quad (in a layer over the viewport), then the expanded card after its shadow quad; the quad is `fill_quad` at the card's drawn bounds, radius `overview_card::RADIUS`, transparent fill, `style::eased_shadow(roles, elevation::MENU, strength)`; layout, events, focus and hit-testing unchanged; T014 and T015 pass
- [ ] T021 [US2] In `crates/micold-client/src/showcase/sections/surfaces.rs` `overview_card` section (D5): a pose of a few activatable cards (`on_press(NoOp)`) whose first card is expanded while `showcase.shown(index)`, toggled by an "Expand / Collapse" button sending `Message::Reversed(index)` as the zoom pose does; showcase grids keep the default (eased); the static expanded poses stay as they are (seeded lifted)
- [ ] T022 [US2] In `docs/user-guide/settings.md` *Reduce motion*: it also makes the expanded card's shadow appear and disappear at once instead of easing over 150 ms ("other animations are unchanged" stays true of the rest); in `docs/user-guide/overview.md`: the expanded card's shadow eases in and out, its height changes at once (FR-013, FR-006)
- [ ] T023 [US4] Run `scripts/build-lock.sh cargo test -p micold-client --lib -- ripple overview_card card_grid keyboard_focus menu` and `--test idle_requests_no_frames --test motion_tokens --test overview --test overview_card_menu --test layout_snapshot`; `git diff origin/main -- crates/micold-client` shows no edited line inside an existing test body (SC-004, US4-1–US4-4)

**Checkpoint**: an expanded card's shadow eases in and out, never cut by its neighbours; the
showcase shows both motions; Reduce motion makes all of it instant.

---

## Phase 5: Polish & Cross-Cutting Concerns

Code-free; done by the close unit in the close PR.

- [ ] T024 Post research R1's comment text on issue #1299 (FR-006: the growth is not animated, and why), with `gh api repos/{owner}/{repo}/issues/1299/comments` (REST)
- [ ] T025 Run quickstart §A (`scripts/gate.sh`) and record the result in `specs/1299-overview-card-focus-motion/quickstart.md`
- [ ] T026 Run quickstart §B with the `visual-pass` skill and record it in `specs/1299-overview-card-focus-motion/visual-pass.md` (FR-013, SC-001 by eye, FR-010)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Foundational (Phase 2)**: none; blocks every timed test.
- **US1 (Phase 3)**: after T001.
- **US2 (Phase 4)**: after Phase 3 (T008's `Grid::instant` reaches the lifts; T015 checks the
  focus half T005 built).
- **Polish (Phase 5)**: after all.

### User Story Dependencies

US3 has no phase of its own: its focus half is T005, T008, T009 and its shadow half T015 (with
T018's and T020's `instant` paths). US4 is asserted in T003 (press), T004 (links), T016 (a card on
its own) and run whole in T011 and T023.

### Within Each User Story

Tests first and failing; wrapper before card; card before grid; grid before app wiring; showcase
and user guide last.

### Parallel Opportunities

- Phase 3: T002, T004, T005 together; T003 after T002 (same file, `keyboard_focus.rs`); T007 after T006.
- Phase 4: T012, T013, T014, T016 together; T015 after T014 (same file, `card_grid.rs`); T017 beside T018; T019 before T020; T021 and T022
  after T020.

## Parallel Example: User Story 2

```text
T012 eased_shadow tests (style.rs)     T013 Lifts tests (card_lift.rs)
T014 → T015 grid tests (card_grid.rs)  T016 shadow_by_owner test (overview_card.rs)
then T017 ‖ T018, T019 → T020
```

## Implementation Strategy

### MVP First

US1 with its Reduce motion half (M1): keyboard focus eases on every Overview card.

### Incremental Delivery

M2 adds the eased shadow and its draw order, the showcase pose, and the full Reduce motion test.
Each merges green; nothing eased reaches `main` without Reduce motion governing it.

## Notes

- Between M1 and M2 the expanded card's shadow still switches in one step (today's behaviour).
- `card_grid.rs` is over 3,000 lines: the lift bookkeeping stays in `card_lift.rs`.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Keyboard focus fades on Overview cards 🎯 MVP

- **Tasks**: T001–T011
- **Deliverable**: on `main`, Tab and the arrow keys fade an Overview card's focus outline and tint in and out over 150 ms, each card on its own; with Settings → Appearance → Reduce motion on it is instant; every other control's focus is unchanged.
- **Satisfies**: US1 acceptance scenarios 1, 2, 4, 5, 6; US3 sc.1 (focus); US4 sc.1–sc.4; FR-001, FR-002, FR-004, FR-005, FR-007 (focus), FR-008 (focus), FR-009, FR-010 (focus), FR-011, FR-013 (user guide, focus); SC-001 (focus), SC-002 (focus), SC-003, SC-004
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --lib -- keyboard_focus overview_card card_grid`; `scripts/build-lock.sh cargo test -p micold-client --test idle_requests_no_frames --test motion_tokens`; in the client, Overview open: Tab onto a card and away from it (the outline and tint fade in, then out, over ~150 ms), quickstart §B.4 (arrow keys half), §B.6, §B.7, §B.8 (focus half) (the showcase's live pose arrives in M2)
- **Depends on**: —
- **Tier**: full

### M2 — The expanded card's shadow eases in and out

- **Tasks**: T012–T023
- **Deliverable**: on `main`, an expanded Overview card's shadow eases in over 150 ms and out again, drawn over its neighbours and never cut while it settles, with no other card moving; the showcase has a live expand/collapse pose; Reduce motion makes it instant.
- **Satisfies**: US1 sc.3; US2 acceptance scenarios 1–6 (sc.6: height in one step, D7); US3 sc.1–sc.3; US4 sc.1–sc.4; FR-003, FR-004 (shadow), FR-005 (shadow), FR-006 (implementation side), FR-007, FR-008, FR-009, FR-010, FR-011, FR-012, FR-013; SC-001, SC-002, SC-004, SC-005
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --lib -- style card_lift card_grid overview_card`; `scripts/build-lock.sh cargo test -p micold-client --test idle_requests_no_frames --test layout_snapshot`; quickstart §B.1–§B.5, §B.8–§B.9
- **Depends on**: M1
- **Tier**: full
- **Size**: 12 tasks, kept whole: the lift, its shadow quad and the draw order are one observable change (a shadow eased without the draw order is cut by the next card, US2-2), so no split along an acceptance scenario leaves a deliverable.

Polish (T024–T026) changes no code: the close unit does it in the close PR.
