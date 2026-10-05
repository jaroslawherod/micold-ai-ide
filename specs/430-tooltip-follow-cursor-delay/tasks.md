# Tasks: Tooltip follows the cursor and waits before showing

**Input**: [spec.md](./spec.md), [plan.md](./plan.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/tooltip-api.md](./contracts/tooltip-api.md)

**Tests**: mandatory and first (Constitution I). Each test task comes before the code it covers and
must be seen failing for the right reason.

**Documentation**: no user-visible change ships (spec Assumptions, plan Constitution VII): no
user-guide task; the PRs carry the `docs-not-needed` label.

Paths: `core` = `crates/micold-core`, `client` = `crates/micold-client`, `cdk` = `client/src/ui/cdk/tooltip.rs`.

## Phase 1: Foundational (the wait mode)

- [x] T001 Guard (characterization, green before and after T002; exempt from red-first): the existing tooltip tests are the proof the wait refactor changes nothing (run the existing `client/tests/tooltip_rest_glue.rs` and `client/tests/idle_requests_no_frames.rs` unchanged as the guard; SC-004)
- [x] T002 Replace `rest: Option<Duration>` with `pub(crate) enum Wait { Hover, Rest(Duration) }` in `cdk` (`after_rest` sets `Rest`); `material::Tooltip` in `client/src/ui/material/mod.rs` holds a `Wait` too. `Delay` joins the enum in T016. All existing tooltip tests pass unchanged.

**Foundational, core**: the pure delay rule, used by the follow placement (T008) and later by `show_delay` (T016).

- [x] T011 Test (red): `core/tests/tooltip_show.rs`: `ShowTimer` table of data-model.md (no open before D, open at D, movement never restarts, leave cancels, zero delay opens at once, press then leave, reset, `wake_at` only while waiting)
- [x] T012 Implement `ShowTimer` in `core/src/tooltip.rs` (T011 green)

## Phase 2: User Story 1 + 3 - pointer-following placement (P1) 🎯 MVP

- [x] T003 [US1] Test (red): `placement_tests` in `cdk` for `place_at_pointer`: beside and offset by `gap` (assert the exact offset, `gap` from the pointer to the visible edge); right edge flips left; bottom edge flips above; corner flips both; fits neither takes the side with more room; window too small stays inside; visible panel never contains the pointer while a side has room (US1.1, US1.2, US3.2, SC-001)
- [x] T004 [US3] Guard test (characterization, green on arrival): `placement_tests` in `cdk` that every fixed placement with a trigger at each of the four window edges does not cover the trigger while a side has room (US3.1, SC-003, SC-006)
- [x] T005 [US1] Implement `Position::FollowCursor` and `place_at_pointer` in `cdk` (T003 green)
- [x] T006 [US1] Test (red): `client/tests/tooltip_show_glue.rs` + new builder helper in `client/tests/support/tooltip.rs` (existing helper unchanged): follow panel opens at the pointer; a pointer move while open re-lays it out; a still pointer requests nothing; leaving closes; a press closes until leave; subject change closes; a trigger scrolled from under a still cursor closes on redraw (US1.1, US1.3, edge cases)
- [x] T007 [US1] Test (red): `cdk` unit test that no pointer, a zero-size trigger or an unknown window size opens nothing (contracts/tooltip-api.md, Behaviour 6)
- [x] T009 [US1] Test (red): `client/tests/idle_requests_no_frames.rs`: a follow tooltip that is open with a still pointer, or closed, requests no frame (FR-007, SC-005)
- [x] T008 [US1] Implement in `cdk`: `State.pointer`, recorded in `update`; `overlay()` reads it, adds `translation`, passes it to `Panel`; `FollowCursor` with `Wait::Hover` runs the delay rule with zero delay (uses `ShowTimer` from T012); guard of T007
- [x] T010 [P] [US1] Expose `FollowCursor` through `material::TooltipPosition` (re-export already covers it) and add the "follows the pointer" pose (large trigger) to `client/src/showcase/sections/floating.rs`
- [ ] T019 [US1] Run quickstart §B steps 1–3 and 6 through the `visual-pass` skill; record evidence in `specs/430-tooltip-follow-cursor-delay/visual-pass.md`

## Phase 3: User Story 2 - show delay (P2)

- [ ] T013 [US2] Test (red): `client/tests/tooltip_show_glue.rs`: `show_delay` driven by events: nothing before D, panel at D, leave cancels, movement during the wait does not restart, no delay opens at once, delay with follow opens at the pointer's current position then tracks it; follow with a delay at each window edge never covers the pointer; a `subject()` change mid-delay restarts it (US2.1 to US2.4, SC-002, SC-003, edge cases)
- [ ] T014 [US2] Test (red): `cdk` and `material` unit tests that `after_rest` then `show_delay` yields `Delay`, and `show_delay` then `after_rest` yields `Rest` (last call wins; research R1)
- [ ] T015 [US2] Test (red): `idle_requests_no_frames.rs` and `material_builder_api.rs`: a waiting delay requests exactly one timed wake and no frame; none without a waiting or open tooltip; `show_delay` is a builder step (US2.5, FR-007)
- [ ] T016 [US2] Implement `Wait::Delay`, `Tooltip::show_delay` in `cdk`, `State.show`, reset in `describe`; the delay's wake through `motion::wake_at`; `material::Tooltip::show_delay` (T013 to T015 green)
- [ ] T017 [P] [US2] Add the "show delay" and "show delay and follow" poses to the showcase `floating.rs`
- [ ] T018 [US2] Run quickstart §B steps 4, 5 and 7 through the `visual-pass` skill; append the evidence to `specs/430-tooltip-follow-cursor-delay/visual-pass.md`

## Dependencies

T001 → T002; T011 → T012; (T003, T004) → T005; T006, T007, T009 → T008; T008 needs T002, T005 and T012; T010 needs T008; T019 needs T008 and T010. M2: T013–T015 → T016 → T017 → T018.

## Implementation Strategy

MVP is pointer-following (P1 US1 and US3). `ShowTimer` is a pure core rule with its own test table, so it lands in Foundational (T011, T012) because the follow placement's press rule uses it; the delay's widget exposure (`show_delay`) is M2.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Pointer-following placement 🎯 MVP

- **Tasks**: T001–T012, T019
- **Deliverable**: `Position::FollowCursor` is a working placement of the shared tooltip, shown in the component showcase: the panel tracks the pointer, flips at window edges and never covers the pointer; every fixed placement is proven clear of its trigger at all four edges; `ShowTimer` is in core.
- **Satisfies**: US1 acceptance scenarios 1–3; US3 acceptance scenarios 1–2; FR-001, FR-002, FR-005, FR-006, FR-007 (follow part); SC-001, SC-003, SC-004, SC-005, SC-006
- **Verify**: `cargo test -p micold-client tooltip` and `cargo test -p micold-core --test tooltip_show`; quickstart §B steps 1 to 3 and 6
- **Depends on**: —
- **Tier**: full

### M2 — Show delay

- **Tasks**: T013–T018
- **Deliverable**: `show_delay` on the shared tooltip and on `material::Tooltip`, working alone and with `FollowCursor`, with `after_rest` as an alternative (last call wins); showcase poses; quickstart §B recorded.
- **Satisfies**: US2 acceptance scenarios 1–5; FR-003, FR-004, FR-007; SC-002, SC-005
- **Verify**: `cargo test -p micold-client --test tooltip_show_glue --test idle_requests_no_frames --test material_builder_api`; quickstart §B steps 4, 5 and 7
- **Depends on**: M1
- **Tier**: full
