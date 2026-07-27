---

description: "Task list for feature 017 — Material Component Architecture"
---

# Tasks: Material Component Architecture

**Input**: Design documents from `specs/017-material-component-architecture/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/component-api.md](./contracts/component-api.md)

**Behaviour delta**: [behavior-delta.md](./behavior-delta.md) — the complete list of what this feature changes that a user can notice, produced by T015.

**Tests**: MANDATORY per Constitution Principle I. Failing tests before implementation.

**Documentation**: MANDATORY per Constitution Principle VII — developer documentation of the layer split ships in the same change.

**Cross-platform**: Per Principle VI, tests run on Linux, macOS and Windows.

**Defining constraint**: this feature ends with **zero visual change**. Baseline to preserve: 781 passing tests, and the application's current appearance.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: US1–US3

## Path Conventions

- `crates/micold-core/` — render-free; no rendering dependency. Tokens and pure decision logic.
- `crates/micold-client/src/ui/cdk/` — behavior layer, no appearance.
- `crates/micold-client/src/ui/material/` — appearance layer. With `cdk`, the only modules naming a rendering widget.
- `crates/micold-client/src/ui/*.rs` — feature modules. Compose components and layout primitives only.

Test command: `mise run test` (`cargo test --workspace`).

---

## Phase 1: Setup

- [X] T001a Capture the **style** parity baseline as a committed fixture in `crates/micold-client/tests/fixtures/style_snapshot.txt` — every style function x every widget status x both schemes (116 resolved styles), asserted byte-for-byte by `crates/micold-client/tests/style_snapshot.rs`. Replaces the automatable half of the original screenshot baseline: exhaustive, re-runnable in CI, and it names the component and status that drifted rather than saying "something looks off"
- [ ] T001b Capture the **layout** parity baseline manually — the style snapshot cannot see spacing or widget-tree structure. Reduced set: main shell (sidebar expanded/collapsed), the add-worktree dialog in both branch-source modes, one open menu, and the sidebar's visible worktree count at a recorded window size. Must be done before Phase 4, where wrappers first touch rendering
- [X] T002 [P] Record the current boundary counts as the migration target: feature modules importing rendering widgets, style applications outside the library, and raw text-size references (research R2)
- [X] T003 [P] Fix the stale test command in `CLAUDE.md`, which documents `cargo test --no-default-features --all-targets` against a repository whose `mise.toml` runs `cargo test --workspace` (research R6)

---

## Phase 2: Foundational A — Tokens and pure logic in the render-free core

**⚠️ CRITICAL**: Blocks everything downstream.

### Tests (write first, confirm they FAIL) ⚠️

- [X] T004 [P] Failing test in `crates/micold-core/tests/tokens_move.rs` asserting every token value is byte-identical to its pre-move value, so the relocation cannot silently re-value anything (FR-021)
- [X] T005 [P] Failing tests in `crates/micold-core/tests/overlay_dismissal.rs` for the unified dismissal rules: non-modal dismisses on outside click, Escape and scroll; modal dismisses on Escape and scrim click only; a non-dismissible surface ignores every trigger; the rule is total, with no undefined input combination (FR-009, FR-017)

### Implementation

- [X] T006 Move the token module from `crates/micold-client/src/tokens.rs` to `crates/micold-core/src/tokens/`, unchanged in value; its imports already resolve within the core, so no dependency edge is added (FR-020, research R4)
- [X] T007 Re-point every token import in `crates/micold-client/src/` at the core and delete the client-side module (FR-020)
- [X] T008 Retarget or fold `crates/micold-client/tests/tokens.rs` so no existing assertion is lost in the move (FR-021, FR-022)
- [X] T009 [P] Implement the unified dismissal rules as pure logic in `crates/micold-core/src/overlay.rs` (FR-009, FR-017)
- [X] T010 Verify the boundary is structural: `crates/micold-core/Cargo.toml` declares no rendering dependency, and `cargo test -p micold-core` exercises every token value with no renderer present (FR-020, FR-022, SC-009)

**Checkpoint**: Tokens and decision logic in the core, green. Zero visual change.

---

## Phase 3: Foundational B — The `cdk` behavior layer (delivers US3; **not required for the MVP**)

**Purpose**: One overlay primitive, carrying no appearance. Five implementations become one.

### Tests (write first, confirm they FAIL) ⚠️

- [X] T011 [P] [US3] Failing test in `crates/micold-client/tests/overlay_stacking.rs` asserting two open floating surfaces stack in a deterministic order independent of composition order (FR-010, SC-003)
- [X] T012 [P] [US3] Failing test in `crates/micold-client/tests/cdk_no_appearance.rs` asserting no module under `ui/cdk/` references a color role, elevation level, shape size or type role — the behavior layer carries no appearance (FR-007)

### Implementation

- [X] T013 [US3] Create `crates/micold-client/src/ui/cdk/overlay.rs` — **one** overlay primitive owning positioning, backdrop, dismissal (delegating to the core rules) and stacking order (FR-006, FR-008)
- [X] T014 [US3] Migrate all five floating surfaces onto the single overlay — `crates/micold-client/src/ui/material/modal.rs`, `menu.rs` (overflow and context menus), `project_switcher.rs` and `select.rs` — deleting their independent positioning, backdrop and dismissal code (FR-008, SC-003). **Four of the five moved**; `select.rs` delegates to the rendering stack's own widget-attached overlay system and must keep doing so — see [`behavior-delta.md`](./behavior-delta.md) *Deviations*
- [X] T015 [US3] Verify each surface's dismissal now follows the unified rule, and record every surface whose behavior changed — that list is the complete sanctioned behavior delta (FR-009, FR-024). Recorded in [`behavior-delta.md`](./behavior-delta.md), asserted by `crates/micold-client/tests/overlay_dismissal_delta.rs`

**Checkpoint**: One overlay, consistent dismissal. Appearance still unchanged.

---

## Phase 4: User Story 1 - A developer changes an appearance in one place (Priority: P1) 🎯 MVP

**Goal**: The library wraps the rendering stack; feature modules can no longer style anything.

**Independent Test**: Change a shared component's appearance and confirm every instance changes with no feature module edited; confirm the application still looks identical.

### Tests for User Story 1 (write first, confirm they FAIL) ⚠️

- [X] T016 [P] [US1] Failing test in `crates/micold-client/tests/material_boundary.rs` asserting no module outside `ui/cdk/` and `ui/material/` imports a styled rendering widget or references the styling layer; layout primitives are explicitly allowed (FR-001, FR-002, FR-003, FR-004, SC-001)
- [X] T017 [P] [US1] Failing test in `crates/micold-client/tests/material_builder_api.rs` asserting every public component is constructed with required inputs only and terminates via the conversion into an element, per `contracts/component-api.md` §4 (Principle VIII)

### Implementation for User Story 1

- [X] T018 [US1] Move the styling module into `crates/micold-client/src/ui/material/style.rs` and reduce its visibility to the library so feature modules cannot reach it (FR-002). **Moved**; a `pub use material::style` shim in `ui/mod.rs` keeps unmigrated modules compiling. Deleting the shim — and moving `tests/style_snapshot.rs` inside the crate, since an integration test cannot see a crate-internal module — is part of T036
- [X] T019 [P] [US1] Create the `Text` wrapper in `crates/micold-client/src/ui/material/text.rs` at today's appearance (FR-001, FR-005)
- [X] T020 [P] [US1] Create the `Button` wrapper in `crates/micold-client/src/ui/material/button.rs` with the variants currently in use, at today's appearance (FR-001, FR-005)
- [X] T021 [P] [US1] Create the `TextField` wrapper in `crates/micold-client/src/ui/material/text_field.rs` at today's appearance (FR-001, FR-005)
- [X] T022 [P] [US1] Create the `Checkbox` wrapper in `crates/micold-client/src/ui/material/checkbox.rs` at today's appearance (FR-001, FR-005)
- [X] T023 [P] [US1] Create the `Scrollable` wrapper in `crates/micold-client/src/ui/material/scrollable.rs` at today's appearance (FR-001, FR-005)
- [X] T024 [P] [US1] Create the `Surface` wrapper in `crates/micold-client/src/ui/material/surface.rs` for container-as-surface usage, at today's appearance (FR-001, FR-005)
- [X] T025 [US1] Export every wrapper from `crates/micold-client/src/ui/material/mod.rs` (FR-001)

**Checkpoint**: The library owns the rendering stack. Appearance unchanged.

---

## Phase 5: User Story 1 (continued) - Migrate the feature modules

- [ ] T026 [P] [US1] Migrate `crates/micold-client/src/ui/shell.rs` onto the wrappers (FR-001, FR-002)
- [ ] T027 [P] [US1] Migrate `crates/micold-client/src/ui/project_selector.rs` onto the wrappers (FR-001, FR-002)
- [ ] T028 [P] [US1] Migrate `crates/micold-client/src/ui/worktree_form.rs` onto the wrappers (FR-001, FR-002) — the largest module
- [X] T029 [P] [US1] Migrate `crates/micold-client/src/ui/worktree_rename.rs` and `rename.rs` onto the wrappers (FR-001, FR-002)
- [ ] T030 [P] [US1] Migrate `crates/micold-client/src/ui/settings_form.rs` onto the wrappers (FR-001, FR-002)
- [X] T031 [P] [US1] Migrate `crates/micold-client/src/ui/about.rs` onto the wrappers (FR-001, FR-002)
- [X] T032 [P] [US1] Migrate `crates/micold-client/src/ui/confirm_delete.rs`, `confirm_forget.rs` and `confirm_session_remove.rs` onto the wrappers (FR-001, FR-002)
- [ ] T033 [P] [US1] Migrate `crates/micold-client/src/ui/terminal.rs` onto the wrappers (FR-001, FR-002), leaving the terminal canvas itself untouched
- [ ] T034 [P] [US1] Migrate `crates/micold-client/src/ui/sidebar.rs` onto the wrappers (FR-001, FR-002)
- [ ] T035 [P] [US1] Migrate `crates/micold-client/src/ui/mod.rs` onto the wrappers (FR-001, FR-002)
- [ ] T036 [US1] Make `crates/micold-client/tests/material_boundary.rs` blocking and confirm all three counts reached zero (FR-004, SC-001)

**Checkpoint**: Zero feature modules style anything. US1 complete.

---

## Phase 6: User Story 2 - Adding an animated element costs nothing globally (Priority: P2)

**Goal**: Presentation state lives in components; the global enumeration and central animators are gone.

**Independent Test**: Add a trivially animated component and confirm no application-state or shared-enumeration change was required.

### Tests for User Story 2 (write first, confirm they FAIL) ⚠️

- [ ] T037 [P] [US2] Failing test in `crates/micold-client/tests/component_state_isolation.rs` asserting two instances of the same animated component animate independently, and that a removed instance retains no state (FR-011, FR-025)
- [ ] T038 [P] [US2] Failing test in `crates/micold-client/tests/component_api_opacity.rs` asserting no public component API exposes an animation key, progress value, style function or rendering-stack type (FR-013, SC-004)

### Implementation for User Story 2

- [ ] T039 [P] [US2] Move the six global-enumeration animation tracks into the components that own them — menu fade into `crates/micold-client/src/ui/material/menu.rs`, sidebar slide into `sidebar.rs`, main-view and overlay fades into `crates/micold-client/src/ui/material/animation.rs` and the cdk overlay, resize-handle hover into the handle, filter-panel fade into `filter_panel.rs` (FR-011, FR-014)
- [ ] T040 [P] [US2] Move the per-row hover-fade tracks and the currently-hovered-row field into each row instance in `crates/micold-client/src/ui/material/tree_view.rs`, removing the hashed row-identity scheme (FR-011, FR-014)
- [ ] T041 [P] [US2] Move the resize-handle drag flag into the handle component in `crates/micold-client/src/ui/sidebar.rs` (FR-011, FR-014)
- [ ] T042 [US2] Delete the global animated-element enumeration and both central animators from `crates/micold-client/src/ui/mod.rs` and `crates/micold-client/src/app.rs`, and stop threading progress values through the view (FR-013, FR-015)
- [ ] T043 [US2] Demonstrate SC-005: add a trivially animated component to a screen and confirm no enumeration variant and no application-state field was needed; then remove it
- [ ] T044 [US2] Confirm no logical state moved — worktrees, active session, expanded nodes, sidebar visibility and width, open-menu identity, drafts, filters and theme preference all remain application-owned, and state written by the pre-change build still loads identically (FR-012, FR-016, SC-006)

**Checkpoint**: Components own their presentation state. US2 complete.

---

## Phase 7: Parity, performance and documentation

- [ ] T045 [P] Verify `contracts/component-api.md` still matches what was built — every requirement reference resolves, and no appearance value has crept in; it is the durable reference this feature is checked against (FR-027)
- [ ] T046 [P] Update developer documentation in `docs/` describing the two layers and the rule that feature modules compose components rather than styling widgets (FR-028, Principle VII)
- [ ] T047 [P] Delete any styling helpers and token constants left unreferenced after the migration, so no dead path survives alongside the new structure (FR-002)
- [ ] T048 Run the full `quickstart.md` Part B walkthrough in the **light** scheme against the baseline screenshots and record the result, including §B5 which evidences that every pre-change action is still available and produces the same result (SC-007)
- [ ] T049 Run the full `quickstart.md` Part B walkthrough in the **dark** scheme and record the result (SC-002, SC-007)
- [ ] T050 **Parity gate** — confirm the application is visually identical to the baseline. Any visible difference is a defect in this feature (FR-023, SC-002)
- [ ] T051 Confirm the full suite passes at or above the 781-test baseline, so no assertion was lost in the moves (FR-021)
- [ ] T052 Verify idle quiescence per `quickstart.md` §B6 — no frames requested and no measurable CPU at rest, and no animation state held after pressing every interactive element (FR-025, SC-008)
- [ ] T053 Verify build and full test suite pass on Linux, macOS and Windows via the CI workflow in `.github/workflows/` (FR-026, Principle VI)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 Setup**: no dependencies. T001 must happen **before any code change** — the baseline cannot be captured afterwards
- **Phase 2 (core)**: blocks everything
- **Phase 3 (cdk / overlay)**: depends on Phase 2. Delivers US3. **Not on the MVP path** — Phase 4 does not depend on it
- **Phase 4 (wrappers)**: depends on Phase 2 only
- **Phase 5 (module migration)**: depends on Phase 4
- **Phase 6 (state)**: depends on Phase 4; can overlap Phase 5 per component
- **Phase 7 (parity, performance, docs)**: depends on everything

Phase order is dependency order, not priority order: Phase 3 delivers the P3 story but is sequenced early because the overlay consolidation is self-contained and touches files the later phases also touch. The MVP path skips it.

### Parallel Opportunities

- T002, T003 in Setup
- T004, T005 — two core test files
- T011, T012 — cdk test files
- T017, T018 — US1 test files
- T020–T025 — six wrapper components, all different files
- T027–T036 — ten module migrations, all different files
- T039, T040 — US2 test files
- T041, T042, T043 — three presentation-state extractions
- T048, T049, T050 — contract check, documentation and cleanup

---

## Implementation Strategy

### MVP (User Story 1)

1. Phase 1 Setup — **T001 first, always**
2. Phase 2 core
3. Phase 4 wrappers
4. Phase 5 migration
5. **STOP and VALIDATE** — parity walkthrough, both schemes

US1 alone delivers the feature's central value: appearance is decided in one place.

### Incremental Delivery

1. Setup + core → green, nothing visible changed
2. + wrappers and migration → the boundary closes (**MVP**)
3. + cdk and overlay consolidation → five implementations become one (**the one sanctioned behavior change**)
4. + state extraction → components own their transients
5. Parity gate, performance, three-platform verification

### Risk Notes

- **T001 is unrecoverable if skipped.** Without baseline screenshots, "looks identical" is an opinion
- **T036 flips the boundary test to blocking.** Keep it advisory until the final module migrates so intermediate states stay buildable
- **T050 (the parity gate) is the merge gate.** This feature's entire value is that it changed nothing visible

---

## Notes

- `[P]` = different files, no dependencies on incomplete tasks
- Confirm every test fails before implementing against it (Principle I)
- No task in this feature may change a token *value*, a colour, a height or any other appearance. Re-valuing is [`018`](../018-material3-visual-system/tasks.md)'s work
- The press ripple and the density scale were deliberately deferred to 018 — both serve an appearance that does not exist yet, and building them here would add untested code with no consumer
- If a call site tempts you to style something, the wrapper is missing a capability — add it to the wrapper (FR-002)
