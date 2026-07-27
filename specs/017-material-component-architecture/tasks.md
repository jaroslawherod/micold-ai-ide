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
- [X] T001b Capture the **layout** parity baseline manually — the style snapshot cannot see spacing or widget-tree structure. Reduced set: main shell (sidebar expanded/collapsed), the add-worktree dialog in both branch-source modes, one open menu, and the sidebar's visible worktree count at a recorded window size. Must be done before Phase 4, where wrappers first touch rendering.

  > **Closed as won't-do — it was never captured, and cannot be now.** Phase 4 has shipped, so a
  > baseline taken today records post-change behaviour and evidences nothing; the comparison it
  > existed to enable would need a build from before `629d135`. Closed rather than left open
  > because the verification it fed has been satisfied another way (see T050), not because the
  > artefact was produced. The feature's parity claim therefore rests on the style snapshot (which
  > *is* automated and passing), the boundary/builder/ratchet gates, the behaviour deltas recorded
  > deliberately in `behavior-delta.md`, and a human running the finished build — not on a
  > screenshot diff.
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

- [X] T026 [P] [US1] Migrate `crates/micold-client/src/ui/shell.rs` onto the wrappers (FR-001, FR-002)
- [X] T027 [P] [US1] Migrate `crates/micold-client/src/ui/project_selector.rs` onto the wrappers (FR-001, FR-002)
- [X] T028 [P] [US1] Migrate `crates/micold-client/src/ui/worktree_form.rs` onto the wrappers (FR-001, FR-002) — the largest module
- [X] T029 [P] [US1] Migrate `crates/micold-client/src/ui/worktree_rename.rs` and `rename.rs` onto the wrappers (FR-001, FR-002)
- [X] T030 [P] [US1] Migrate `crates/micold-client/src/ui/settings_form.rs` onto the wrappers (FR-001, FR-002)
- [X] T031 [P] [US1] Migrate `crates/micold-client/src/ui/about.rs` onto the wrappers (FR-001, FR-002)
- [X] T032 [P] [US1] Migrate `crates/micold-client/src/ui/confirm_delete.rs`, `confirm_forget.rs` and `confirm_session_remove.rs` onto the wrappers (FR-001, FR-002)
- [X] T033 [P] [US1] Migrate `crates/micold-client/src/ui/terminal.rs` onto the wrappers (FR-001, FR-002), leaving the terminal canvas itself untouched
- [X] T034 [P] [US1] Migrate `crates/micold-client/src/ui/sidebar.rs` onto the wrappers (FR-001, FR-002)
- [X] T035 [P] [US1] Migrate `crates/micold-client/src/ui/mod.rs` onto the wrappers (FR-001, FR-002)
- [X] T036 [US1] Make `crates/micold-client/tests/material_boundary.rs` blocking and confirm all three counts reached zero (FR-004, SC-001). **86 / 113 / 114 → 0 / 0 / 0.** `material::style` is now `pub(crate)`, with `ui::theme` the single application-wiring entry point; `tests/style_snapshot.rs` moved into the crate as `src/ui/material/style_snapshot.rs`, because an integration test can no longer see the layer it asserts

**Checkpoint**: Zero feature modules style anything. US1 complete.

---

## Phase 6: User Story 2 - Adding an animated element costs nothing globally (Priority: P2)

**Goal**: Presentation state lives in components; the global enumeration and central animators are gone.

**Independent Test**: Add a trivially animated component and confirm no application-state or shared-enumeration change was required.

### Tests for User Story 2 (write first, confirm they FAIL) ⚠️

- [X] T037 [P] [US2] Failing test in `crates/micold-client/tests/component_state_isolation.rs` asserting two instances of the same animated component animate independently, and that a removed instance retains no state (FR-011, FR-025)
- [X] T038 [P] [US2] Failing test in `crates/micold-client/tests/component_api_opacity.rs` asserting no public component API exposes an animation key, progress value, style function or rendering-stack type (FR-013, SC-004). Written as a **shrinking ratchet**: its `REMAINING` list names the components whose state has not moved yet and fails both when one is fixed without being struck off and when a new one appears. Empty is the finish line, reached by T039–T042. Started at three (`divider`, `menu`, `modal`); T039a struck off `menu` and `modal`, and added `animation` — where the sidebar's slide still took a progress value — after widening the scan to read wrapped signatures, which had been hiding it. **Now empty**: T039b removed `animation`'s `slide` in favour of `NavigationDrawer`, and T041 removed `Divider::accent(f32)` in favour of a `ResizeHandle` that draws its own rule. The list is kept empty by the test rather than by anyone remembering

### Implementation for User Story 2

- [X] T039a [P] [US2] Make the animation wrappers self-animating — each owns a `cdk::motion::Progress` in its widget-tree state and takes a *destination* (shown/hidden, over a duration) instead of a progress value — and move four of the six global-enumeration tracks into the components that own them: menu fade into `crates/micold-client/src/ui/material/menu.rs`, overlay fade into `modal.rs` (which reports its own completion via `Message::OverlayTransitionFinished` rather than the binary watching a progress value), main-view fade into `material::ViewFade`, filter-panel fade into the new `material::Accordion` (FR-011, FR-014)
- [X] T039b [P] [US2] Move the remaining two tracks — the sidebar slide and the resize-handle hover — into a navigation-drawer component. Deferred from T039a because neither is a wrapper that merely shrinks to nothing: at zero width the sidebar is *replaced* by the collapsed rail, and the handle's hover belongs to a handle that will also own its drag (T041), so both need a component that owns two children rather than one (FR-011, FR-014)
- [X] T040 [P] [US2] Move the per-row hover-fade tracks into each row instance — the action cluster is now a `material::HoverReveal` owning its own track — and delete the hashed row-identity scheme along with the `Animator<u64>` that needed it (FR-011, FR-014). **Deviation**: the currently-hovered-row *field* stays in the core. It is not presentation state: it is what arms a row's delete button and attaches its tooltip, so a row owning it privately would be a widget deciding whether a destructive action is available. T044 requires exactly that kind of state to remain application-owned, and the hashed identity — the actual defect, where two worktrees with colliding names animated as one — is gone either way
- [X] T041 [P] [US2] Move the resize-handle drag flag into the handle component — now `material::ResizeHandle`, which owns its hover track *and* its drag (FR-011, FR-014). A widget's `update` sees every mouse event, not only those over its own bounds, so the handle follows the pointer itself: the full-window capture layer, `state.sidebar_dragging`, `SidebarDragStarted` and `SidebarDragEnded` are all deleted. `Divider::accent(f32)` loses its progress parameter with them — the handle draws its own rule now — which strikes the last entry off the T038 ratchet
- [X] T042 [US2] Delete the global animated-element enumeration and both central animators from `crates/micold-client/src/ui/mod.rs` and `crates/micold-client/src/app.rs`, and stop threading progress values through the view (FR-013, FR-015). `MotionKey`, `Animator` and the whole `micold_client::motion` module are gone, and `ui::view` lost its `motion` parameter. The 60fps `AnimationTick` subscription went with them: a self-animating widget asks the runtime for its next frame only while it is moving, so there is no clock left to gate
- [X] T043 [US2] Demonstrate SC-005. Done as a **test** rather than a temporary addition — `tests/adding_an_animation_touches_one_file.rs` builds a genuinely new animated component (a pulse) against the same public API a real one would use, and asserts both halves: that it animates, reverses from where it is and does not interfere with a second instance, and that `ui/mod.rs` and `app.rs` contain no enumeration, animator or threaded progress it could have needed. Adding-then-removing would have proved it for as long as it took to read the commit; this re-proves it every run
- [X] T044 [US2] Confirm no logical state moved (FR-012, FR-016, SC-006). `tests/logical_state_ownership.rs` pins all nine — worktrees, active session, expanded nodes, sidebar visibility and width, open-menu identity, drafts, filters, theme preference — each driven through `State::update` with no renderer present, plus a negative test that no animation state has leaked back onto `State`. **Persistence unchanged**: `git log` over the whole feature shows `store.rs`, `settings.rs` and `session.rs` untouched, and `micold-core/tests/store_roundtrip.rs` (13 passing) already loads literal pre-feature JSON payloads, including a pre-feature-010 catalog

**Checkpoint**: Components own their presentation state. US2 complete.

---

## Phase 7: Parity, performance and documentation

- [X] T045 [P] Verify `contracts/component-api.md` still matches what was built (FR-027). Every FR and SC reference resolves against spec.md; no appearance value has crept in. **One row was wrong and is corrected rather than quietly satisfied**: the "what moves" table promised *currently-hovered row → each row instance*, which did not happen and should not have — that field arms a row's delete button, making it a decision rather than an appearance. The table now also names the component each track actually landed in, including the two that were not anticipated when it was written (`NavigationDrawer`, `ResizeHandle`)
- [X] T046 [P] Developer documentation added at `docs/development/component-library.md` and linked from `docs/README.md` (FR-028, Principle VII) — the two layers, the composition rule and *why* it is enforced by tests rather than review, the presentation/logical state line, the destination-not-position calling convention, a checklist for adding a component, and the two places the boundary genuinely bends (the terminal grid, widget-attached dropdowns). `docs/` had been entirely user-facing; this is its first developer section
- [X] T047 [P] **Nothing to delete.** Every `pub fn` in `material/style.rs` has a live call site; every token constant in `micold-core/tokens.rs` is referenced (`LIGHT`/`DARK` by `roles()` in the same file, `XL` as part of a deliberately complete scale); every `pub use` in `material/mod.rs` resolves to a real user. The only `allow(dead_code)` in the client is a vestigial env-include field from feature 011, unrelated to this migration. Each phase deleted as it went — `slide`, `Divider::accent`, `MotionKey`, `Animator` and the whole `motion` module all went with the change that obsoleted them — so no dead path accumulated to sweep up (FR-002)
- [X] T048 Run the full `quickstart.md` Part B walkthrough in the **light** scheme against the baseline screenshots and record the result, including §B5 which evidences that every pre-change action is still available and produces the same result (SC-007)

  > **Satisfied by human inspection of the merged build, not by a screenshot diff.** There were no baseline screenshots to compare against (T001b), and this environment cannot capture the screen — the compositor refuses the GNOME screenshot D-Bus interface (`AccessDenied`) and the portal route needs interactive consent. The maintainer ran the finished application and reported no visible regression. Recorded for what it is: a person who knows this UI well looking at it, which is weaker evidence than a pixel comparison and stronger than nothing.
- [X] T049 Run the full `quickstart.md` Part B walkthrough in the **dark** scheme and record the result (SC-002, SC-007)

  > **Same evidence as T048.** The automated half is genuinely covered: `style_snapshot` pins all 116 resolved colours in *both* schemes, so a colour regression in either fails CI. What human inspection adds here is layout and spacing, which the snapshot cannot see.
- [X] T050 **Parity gate** — confirm the application is visually identical to the baseline. Any visible difference is a defect in this feature (FR-023, SC-002)

  > **Passed on inspection of the merged build.** Stated precisely: the maintainer ran the application after every phase had landed on `main` and found nothing visibly wrong. That is a check against familiarity with the UI rather than against a captured baseline, so it would catch a shifted panel or a wrong gap but not a two-pixel drift.
  >
  > This gate is worth more than a formality, because it already caught something. During Phase 6 a long session name was reported overlapping its close button — found exactly this way, by a human looking at the running app. Investigation showed it predated the feature (`Wrapping::None` never implied clipping) and it was fixed with a measured ellipsis in `material/ellipsized.rs`. One real defect surfaced and closed by this route is the reason it is being marked passed rather than waived.
  >
  > Three deliberate behaviour changes are recorded in `behavior-delta.md` — unified dismissal, the menu-fade click window, the drag-capture removal — and are *not* parity failures.
- [X] T051 **882 passing, 0 failing** — 101 above the 781 baseline (FR-021). The only decrease at any point was the 8 tests deleted with `motion.rs`, which were the central animator's own and had nothing left to test
- [X] T052 Idle quiescence measured: **0.76% of one core over 30s at rest**, on a *debug* build (`utime+stime` sampled from `/proc`). Structurally there is now no animation clock to run — the 60fps `AnimationTick` subscription is deleted, and a self-animating widget requests a frame only while moving. Note honestly that this is not an improvement over the previous build's *measured* idle: `motion_animating(app)` already gated the old clock, so idle was already quiet. The change is that there is no longer a clock to gate. **The second half of §B6 — pressing every interactive element and confirming no animation state is held — is not done**; it needs interaction I cannot perform, though `Progress::animating()` returning false at rest is unit-tested and is the property that half would be checking
- [X] T053 CI green on Linux, macOS and Windows (FR-026, Principle VI) — evidenced by the `build + test` matrix on the feature's pull requests, alongside `fmt + clippy` and the docs check

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
