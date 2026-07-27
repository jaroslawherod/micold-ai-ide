---

description: "Task list for feature 018 — Material 3 Visual System"
---

# Tasks: Material 3 Visual System

**Input**: Design documents from `specs/018-material3-visual-system/`

**Depends on**: [`017-material-component-architecture`](../017-material-component-architecture/tasks.md) — must be complete first.

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/](./contracts/)

**Tests**: MANDATORY per Constitution Principle I. Every story writes failing tests before implementation (Red-Green-Refactor).

**Documentation**: MANDATORY per Constitution Principle VII. Each user-facing story ships its user-guide update in the same change.

**Cross-platform**: Per Principle VI, all tests run on Linux, macOS and Windows.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: US1–US5, mapping to the spec's prioritized user stories

## Path Conventions

Three-crate Cargo workspace (see plan.md → Structure Decision):

- `crates/micold-core/` — render-free. Token **values** and pure decision logic live here.
- `crates/micold-client/src/ui/cdk/` — behavior layer established by 017. Appearance is never set here.
- `crates/micold-client/src/ui/material/` — the appearance layer. **Every task in this feature edits here.**
- `crates/micold-client/src/ui/*.rs` — feature modules. Not edited by this feature; 017's boundary test fails the build if they are styled.
- `assets/fonts/` — vendored font binaries, license and provenance.

Test command throughout: `mise run test` (`cargo test --workspace`).

---

## Phase 1: User Story 1 - The interface reads as Material at a glance (Priority: P1) 🎯 MVP

**Goal**: Real depth. Graded surface tones, drop shadows on everything Material elevates, Material's corner sizes, decorative borders gone.

**Independent Test**: Open the app in light, then dark. Open a dialog, a context menu and the project switcher popover. Each floats above what is behind it; levels are distinguishable without borders; no container carries an outline that is not a divider, an outlined control, or a focus ring.

### Tests for User Story 1 (MANDATORY — write first, confirm they FAIL) ⚠️

- [ ] T001 [P] [US1] Failing test in `crates/micold-client/tests/style_elevation.rs` asserting each elevated style function returns a style whose shadow blur is non-zero and whose background matches the level's tonal role, and that an elevation-0 surface returns no shadow, in both schemes (FR-015, FR-016, SC-002)
- [ ] T002 [P] [US1] Failing test in `crates/micold-client/tests/style_outline_discipline.rs` asserting no style function carrying an elevation also sets a non-transparent border (FR-002, FR-003)
- [ ] T003 [P] [US1] Failing test in `crates/micold-client/tests/style_shape.rs` asserting buttons and chips resolve `full`, cards resolve `medium`, dialogs resolve `extra_large` (FR-019)

### Implementation for User Story 1

- [ ] T004 [US1] Add the elevation→shadow conversion in `crates/micold-client/src/ui/material/style.rs`, folding Material's key and ambient shadows into the single shadow the renderer exposes per widget (research R1)
- [ ] T005 [US1] Wire the elevation scale into `crates/micold-client/src/ui/material/surface.rs` via `.elevation()` and `.shape()` (FR-015)
- [ ] T006 [US1] Rewrite the surface, dialog, menu, sidebar and toolbar style functions in `crates/micold-client/src/ui/material/style.rs` to draw from the elevation scale and graded surface-container roles, removing the 1px outline each uses to fake depth (FR-002, FR-015)
- [ ] T007 [P] [US1] Apply the `full` pill radius to every button variant in `crates/micold-client/src/ui/material/button.rs` and `icon_button.rs` (FR-019)
- [ ] T008 [P] [US1] Apply the `extra_large` (28) corner and the dialog surface role in `crates/micold-client/src/ui/material/modal.rs` (FR-019, FR-028)
- [ ] T009 [US1] Remove decorative borders in `crates/micold-client/src/ui/material/tree_view.rs`, `toolbar.rs`, `terminal_pane.rs`, `progress.rs` and `toggle_chip.rs`, retaining only genuine dividers in `outline_variant` (FR-002, FR-003, SC-002)
- [ ] T010 [US1] Draw the modal scrim at 32% `scrim` in `crates/micold-client/src/ui/material/modal.rs` (FR-028, contract §4)
- [ ] T011 [US1] Verify overlapping elevated surfaces render in elevation order with independent shadows in `crates/micold-client/src/ui/material/modal.rs` and `menu.rs` — a context menu over a dialog must not flatten into it (FR-017)
- [ ] T012 [US1] Update `docs/user-guide/` to describe the new surface hierarchy and the accent-color change from blue to baseline purple (FR-041, FR-005b, Principle VII)

**Checkpoint**: The app reads as Material at a glance. Demonstrable via quickstart §B1.

---

## Phase 2: User Story 2 - Text has a typographic voice (Priority: P2)

**Goal**: Roboto ships with the app; every text site resolves a named type role carrying size, weight and line height.

**Independent Test**: Change the OS UI font and relaunch — the app is unchanged. A dialog's title, body and caption are each distinguishable without relying on position. Terminal output is still monospaced.

**Note**: Phase 3 already routed every text site through `material::Text`, so this story assigns *roles* rather than hunting call sites.

### Tests for User Story 2 (MANDATORY — write first, confirm they FAIL) ⚠️

- [ ] T013 [P] [US2] Failing test in `crates/micold-client/tests/roboto_font.rs` asserting both shipped faces parse via `ttf-parser` and report weight 400 and 500 (FR-008a, SC-012)
- [ ] T014 [P] [US2] Failing test in `crates/micold-client/tests/type_role_call_sites.rs` asserting no source file passes a raw numeric literal as a text size, weight or line height — every site resolves a named role (FR-010, SC-003)

### Implementation for User Story 2

- [ ] T015 [US2] Register both Roboto faces via `.font(...)` and set `.default_font(...)` to Roboto in `crates/micold-client/src/main.rs`, keeping the Material Symbols registration intact (FR-008, research R3)
- [ ] T016 [US2] Resolve type roles into size, font weight and absolute line height inside `crates/micold-client/src/ui/material/text.rs`, so the role is the only thing a call site names (FR-007, FR-010)
- [ ] T017 [P] [US2] Assign the correct type roles across `crates/micold-client/src/ui/shell.rs`, `project_selector.rs` and `terminal.rs`
- [ ] T018 [P] [US2] Assign the correct type roles across `crates/micold-client/src/ui/worktree_form.rs`, `worktree_rename.rs`, `rename.rs` and `settings_form.rs`
- [ ] T019 [P] [US2] Assign the correct type roles across `crates/micold-client/src/ui/about.rs`, `confirm_delete.rs`, `confirm_forget.rs`, `confirm_session_remove.rs` and `mod.rs`
- [ ] T020 [P] [US2] Assign the correct type roles inside `crates/micold-client/src/ui/material/` — `tree_view.rs`, `menu.rs`, `toolbar.rs`, `select.rs`, `progress.rs`, `project_switcher.rs`, `icon_button.rs`, `tag.rs`
- [ ] T021 [US2] Apply the sidebar-scoped roles in `crates/micold-client/src/ui/sidebar.rs` so the 80% density decision is one auditable mapping (FR-011)
- [ ] T022 [US2] Confirm glyph fallback for characters outside Roboto's coverage at the font registration in `crates/micold-client/src/main.rs` (FR-013)
- [ ] T023 [US2] Update `docs/user-guide/` to note the shipped typeface and resulting cross-platform consistency (FR-041, Principle VII)

**Checkpoint**: Typography is role-driven and platform-independent. Demonstrable via quickstart §B2.

---

## Phase 3: User Story 3 - The interface responds under the pointer and the keyboard (Priority: P3)

**Goal**: State layers on every interactive surface, Material's **ripple** on every press, and a focus indicator wherever focus is reachable.

**Independent Test**: Hover every interactive element and confirm a visible change; click each and confirm a ripple from the click point; tab into a text field and confirm a focus indicator.

**Note**: The ripple's state is decision logic, so it lands in tested core before any drawing (Principle I, FR-024e).

### Tests for User Story 3 (MANDATORY — write first, confirm they FAIL) ⚠️

- [ ] T024 [P] [US3] Failing tests for ripple state in `crates/micold-core/tests/ripple_state.rs`: pressing element B mid-ripple leaves A's progress and origin untouched; a completed ripple removes its entry so nothing is retained at rest; an origin outside the element's bounds is clamped; with no known pointer position the origin is the element's center; the end radius reaches the element's furthest corner (FR-024b, FR-024d, FR-024e)
- [ ] T025 [P] [US3] Failing test in `crates/micold-client/tests/style_state_layers.rs` asserting each interactive style function returns visibly different output for active, hovered and pressed, with the pressed delta at least the hover delta (FR-021, SC-005)
- [ ] T026 [P] [US3] Failing test in `crates/micold-client/tests/style_focus.rs` asserting the focused text-input status yields the 3dp `secondary` focus indicator, distinguishable from hovered (FR-022)
- [ ] T027 [P] [US3] Failing test in `crates/micold-client/tests/style_disabled.rs` asserting disabled content resolves the 0.38 opacity, including the self-coloring icon-glyph path (FR-023)

### Implementation for User Story 3

- [ ] T028 [US3] Confirm the coordinate space the pointer area reports, against a real widget, before finalising the ripple renderer — an origin in the wrong frame places every ripple incorrectly (FR-024g)
- [ ] T029 [US3] Build the ripple renderer in `crates/micold-client/src/ui/cdk/ripple.rs` — press capture, geometry and per-instance state, carrying no colour or opacity of its own (FR-024f)
- [ ] T030 [US3] Implement ripple origin, progress, phase and lifetime in `crates/micold-core/src/ripple.rs`, keyed per element using the existing per-instance animation-key pattern (FR-024b, FR-024d, FR-024e)
- [ ] T031 [US3] Confirm the coordinate space the pointer-area reports before finalising the wrapper — the terminal canvas works in absolute window coordinates, so element-relative conversion may be required (plan risk: ripple origin coordinate space)
- [ ] T032 [US3] Create the `Ripple` component in `crates/micold-client/src/ui/material/ripple.rs` per `contracts/component-api.md` §2.1a — expanding circle drawn with the canvas facility, clipped to the element's shape, beneath content and above container (FR-024a, FR-024b)
- [ ] T033 [US3] Compose `Ripple` inside `crates/micold-client/src/ui/material/button.rs`, `tree_view.rs`, `menu.rs`, `tag.rs`, `toggle_chip.rs` and `icon_button.rs` so every interactive surface ripples without any call site opting in (FR-024c)
- [ ] T034 [US3] Add the state-layer compositing helper to `crates/micold-client/src/ui/material/style.rs` as the single place any state layer is applied (FR-020)
- [ ] T035 [US3] Apply the full state-layer set to the shared text-button style in `crates/micold-client/src/ui/material/style.rs`, which brings list rows, tree items and menu items to life (FR-021, research R9)
- [ ] T036 [P] [US3] Apply the state-layer set to the filled, outlined and icon button styles in `crates/micold-client/src/ui/material/style.rs` (FR-021)
- [ ] T037 [P] [US3] Apply the state-layer set to chips and tags in `crates/micold-client/src/ui/material/tag.rs` and `toggle_chip.rs`, preserving AA under every state (FR-021, FR-024)
- [ ] T038 [P] [US3] Apply the state-layer set plus the focus indicator to `crates/micold-client/src/ui/material/text_field.rs` and `select.rs` (FR-021, FR-022)
- [ ] T039 [US3] Add the persistent `selected` treatment — `secondary_container` fill with `on_secondary_container` text — in `crates/micold-client/src/ui/material/tree_view.rs` and `filter_panel.rs` (FR-020, contract §7.2)
- [ ] T040 [US3] Update `docs/user-guide/` to describe hover, ripple, selection and focus feedback, recording that keyboard focus indicators exist only on text fields (FR-041, FR-043, Principle VII)

**Checkpoint**: The UI responds under the pointer everywhere. Demonstrable via quickstart §B3.

---

## Phase 4: User Story 4 - Components match the components they claim to be (Priority: P4)

**Goal**: Correct anatomy — app bar, row densities, button targets, dialogs, menus, chips, the filled text field, the progress indicator, and the notification surface as a real snackbar.

**Independent Test**: Compare each component against its `contracts/design-tokens.md` §7 entry. Trigger several notifications and confirm one-at-a-time queueing with timed dismissal.

### Tests for User Story 4 (MANDATORY — write first, confirm they FAIL) ⚠️

- [ ] T041 [P] [US4] Failing tests for the notification queue in `crates/micold-core/tests/notify_queue.rs`: never more than one visible; a duplicate of the visible notification is not enqueued; the cap drops oldest pending and never the visible one; an error's duration is strictly longer than an info's; manual dismissal promotes the next pending immediately (FR-032a, FR-032b)
- [ ] T042 [P] [US4] Failing test in `crates/micold-core/tests/tokens_anatomy.rs` asserting every component anatomy constant — app bar height, both row densities, minimum touch target, dialog padding, menu item height, chip height, text field height, progress thickness, snackbar min height — matches contract §7 (FR-025 – FR-032, SC-008)
- [ ] T043 [P] [US4] Failing test in `crates/micold-client/tests/app_bar_scroll.rs` asserting the app bar's elevated flag derives from the sidebar's scroll offset (FR-025a)
- [ ] T044 [P] [US4] Failing test in `crates/micold-client/tests/text_field_anatomy.rs` asserting the filled container role, rounded-top/square-bottom corners, and a bottom active indicator that thickens to 2dp accent on focus (FR-031)

### Implementation for User Story 4

- [ ] T045 [US4] Apply the filled text-field anatomy in `crates/micold-client/src/ui/material/text_field.rs` — 56dp height, filled container role, per-corner radius, bottom active indicator, 16dp padding (FR-031)
- [ ] T046 [US4] Add the in-container label and the supporting-text slot to `crates/micold-client/src/ui/material/text_field.rs`, rendering the label persistently in its floating position (FR-031a, FR-031b, FR-044)
- [ ] T047 [US4] Migrate the seven input call sites off placeholder-as-label onto label + supporting text per the contract §7.7 migration table, across `crates/micold-client/src/ui/worktree_form.rs`, `rename.rs`, `worktree_rename.rs` and `settings_form.rs` (FR-031a, FR-031b)
- [ ] T048 [US4] Rebuild `crates/micold-client/src/ui/material/select.rs` on the `TextField` anatomy and style its dropdown as a menu (FR-031, FR-031d)
- [ ] T049 [US4] Apply the linear progress anatomy in `crates/micold-client/src/ui/material/progress.rs` — `secondary_container` track, `primary` indicator, 4dp thickness, fully rounded (FR-031e)
- [ ] T050 [US4] Replace the static 0.4 fill in `crates/micold-client/src/ui/material/progress.rs` with Material's indeterminate presentation, so the bar stops asserting a completion fraction the application cannot know (FR-031f)
- [ ] T051 [US4] Implement the notification queue in `crates/micold-core/src/notify.rs` — one visible, ordered pending queue, severity-derived duration, dedup and cap preserved (FR-032a, FR-032b)
- [ ] T052 [US4] Create the `Snackbar` component in `crates/micold-client/src/ui/material/snackbar.rs` per `contracts/component-api.md` §2.2 (FR-032, Principle VIII)
- [ ] T053 [US4] Replace the inline notification strip in `crates/micold-client/src/ui/mod.rs` with the floating snackbar overlay, above the dialog scrim and not obstructing a dialog's action row (FR-032)
- [ ] T054 [US4] Rework `crates/micold-client/src/ui/material/toolbar.rs` to the small app bar anatomy — 64dp height, 16dp padding, `title_large` title, 48dp icon targets — and add `.elevated(bool)` (FR-025)
- [ ] T055 [US4] Wire elevate-on-scroll: add the scroll handler to the sidebar's scrollable in `crates/micold-client/src/ui/sidebar.rs`, a message variant and view-state flag in `crates/micold-client/src/app.rs`, and pass it to the toolbar builder (FR-025a, research R10)
- [ ] T056 [P] [US4] Add the dense (36dp) and standard (48dp) row densities to `crates/micold-client/src/ui/material/tree_view.rs`, defaulting the sidebar to dense (FR-026, FR-026a)
- [ ] T057 [P] [US4] Enforce the 48dp minimum interactive target in `crates/micold-client/src/ui/material/icon_button.rs` (FR-027)
- [ ] T058 [P] [US4] Apply dialog anatomy in `crates/micold-client/src/ui/material/modal.rs` — 24dp padding, `headline_small` title, `body_medium` body, trailing-aligned action row with 8dp gap (FR-028)
- [ ] T059 [P] [US4] Apply menu anatomy in `crates/micold-client/src/ui/material/menu.rs` — `surface_container`, elevation 2, `extra_small` corner, 48dp items (FR-029)
- [ ] T060 [P] [US4] Apply chip anatomy in `crates/micold-client/src/ui/material/tag.rs` and `toggle_chip.rs` — 32dp height, `full` corner, `label_large` (FR-030)
- [ ] T061 [US4] Confirm the known-projects list in `crates/micold-client/src/ui/project_selector.rs` uses the standard row density while the sidebar stays dense (FR-026)
- [ ] T062 [US4] Update `docs/user-guide/` to document the snackbar's one-at-a-time queueing and timed dismissal — the single sanctioned behavior change (FR-036a, FR-041, Principle VII)

**Checkpoint**: Components match their Material counterparts. Demonstrable via quickstart §B4.

---

## Phase 5: User Story 5 - Movement feels like Material (Priority: P5)

**Goal**: Existing animations adopt Material's durations and easing; the four new animations do the same.

**Independent Test**: Trigger each existing animation and confirm it still starts, completes and ends in the same visual state, at the new timing and with acceleration rather than constant-rate motion.

### Tests for User Story 5 (MANDATORY — write first, confirm they FAIL) ⚠️

- [ ] T063 [P] [US5] Failing test in `crates/micold-client/tests/motion_tokens.rs` asserting every animated track's duration and easing resolve to a named core motion token, that the sidebar slide uses the emphasized set while small fades use the standard set, and that no animation uses a hardcoded per-tick step (FR-034, SC-010)

### Implementation for User Story 5

- [ ] T064 [US5] Rework `crates/micold-client/src/motion.rs` so track speeds derive from the core duration tokens and apply the named easing curves, keeping the existing idle gate so no work runs at rest (FR-033, FR-034)
- [ ] T065 [US5] Apply the assigned duration and easing per contract §6.3 in `crates/micold-client/src/ui/material/animation.rs`, `menu.rs`, `tree_view.rs` and `crates/micold-client/src/ui/sidebar.rs`, preserving each animation's existing trigger, start state and end state (FR-034, FR-035)
- [ ] T066 [US5] Drive the four new animations from the same tokens — app bar elevation in `crates/micold-client/src/ui/material/toolbar.rs`, snackbar enter/exit in `snackbar.rs`, indeterminate progress in `progress.rs`, ripple expand/fade in `ripple.rs` — and confirm no fifth animation is introduced (FR-035a, SC-010)
- [ ] T067 [US5] Confirm the animation clock still idles at rest with ripples in play — a completed ripple must remove its state so nothing animates (FR-024d)
- [ ] T068 [US5] Update `docs/user-guide/` if motion is user-visible enough to warrant a note; otherwise record in the PR that no doc change was needed (Principle VII)

**Checkpoint**: All five stories complete and independently demonstrable.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [ ] T069 [P] Delete the superseded notification style function from `crates/micold-client/src/ui/material/style.rs` and any feature-003 token constants left unreferenced after Phase 2
- [X] T070 [P] ~~Fix the stale test command in `CLAUDE.md`~~ — already done as part of [017](../017-material-component-architecture/tasks.md) T003; kept here only so the numbering is stable
- [ ] T071 [P] Cross-cutting documentation review and `docs/` index/navigation updates (Principle VII)
- [ ] T072 Run the full `quickstart.md` Part B walkthrough in the **light** scheme and record the result
- [ ] T073 Run the full `quickstart.md` Part B walkthrough in the **dark** scheme and record the result
- [ ] T074 Complete the no-behavior-change regression pass in `specs/018-material3-visual-system/quickstart.md` §B6. Any unchecked box there blocks merge; exactly one behavioral difference (the snackbar) is permitted (FR-036, FR-036a, SC-007)
- [ ] T075 Verify build and full test suite pass on Linux, macOS and Windows via the CI workflow in `.github/workflows/` (Principle VI, FR-039)
- [ ] T076 Confirm the visible-worktree count rendered by `crates/micold-client/src/ui/sidebar.rs` has not dropped materially against the pre-change baseline, per `quickstart.md` §B4 (FR-026a)

---

## Dependencies & Execution Order

### Prerequisite: feature 017

**Every phase here depends on [`017-material-component-architecture`](../017-material-component-architecture/tasks.md) being complete.** That feature wraps the rendering stack, splits the library into behavior and appearance layers, consolidates the overlays, moves presentation state into components, and relocates the tokens to the render-free core — all with zero visual change.

Because it landed first, every task below changes appearance in **one place**. If a task here tempts you to edit a feature module to change how something looks, 017's boundary was not closed properly — fix that, don't work around it.

### Phase Dependencies

- **US1 (Phase 1)**: depends only on 017. Independent of the other stories
- **US2 (Phase 2)**: depends only on 017. 017 already routed every text site through the text component, so this assigns *roles* rather than hunting call sites
- **US3 (Phase 3)**: depends only on 017. Within it, ripple appearance builds on 017's ripple renderer
- **US4 (Phase 4)**: depends only on 017. Reads type roles from US2 and state layers from US3; lands correctly without them, just unstyled in those respects
- **US5 (Phase 5)**: depends only on 017. Its final task covers animations introduced in US3 and US4, so run it after those
- **Polish (Phase 6)**: depends on all desired stories

### Within Each Story

- Tests written and confirmed failing before implementation (Principle I)
- Token values before their application
- Pure decision logic before its rendering — the notification queue before the snackbar
- User-guide documentation ships in the same change (Principle VII)

### Parallel Opportunities

- T001, T002, T003 — US1 test files
- T007, T008 — US1 shape work in different files
- T012, T013 — US2 test files
- T017–T020 — four type-role assignment tasks, different modules
- T024–T027 — US3 test files
- T034–T036 — US3 style application in different concerns
- T039–T042 — US4 test files
- T053–T058 — component anatomy tasks, all different files
- T069–T071 — polish cleanups
- **Whole stories**: US1–US5 can be staffed concurrently once 017 is green

---

## Implementation Strategy

### MVP (User Story 1)

1. Confirm 017 is complete and its parity gate passed
2. Phase 1 (US1) — surfaces, elevation, shape
3. **STOP and VALIDATE** — `quickstart.md` §B0 and §B1, both schemes

US1 alone changes the application's identity: real depth, Material's corners, and the baseline purple accent.

### Incremental Delivery

1. 017 complete → foundation closed, **nothing looked different yet**
2. + US1 → depth and shape (**MVP** — the first visible change)
3. + US2 → typographic voice and cross-platform parity
4. + US3 → ripple and live state feedback
5. + US4 → correct component anatomy, real text fields, the snackbar
6. + US5 → Material motion
7. Polish → regression pass, three-platform verification

### Risk Notes

- **The first two tasks that touch the palette are the visible break point** — the app turns purple and tag colors shift. Both are intended; `quickstart.md` §B0 exists so this is confirmed rather than discovered
- **The regression pass is a merge gate, not a formality.** This feature permits exactly one behavioral difference: the snackbar
- **Do not re-litigate 017's decisions here.** If a wrapper cannot express something this feature needs, extend the wrapper

---

## Notes

- `[P]` = different files, no dependencies on incomplete tasks
- Confirm every test fails before implementing against it (Principle I)
- Every appearance change belongs in `ui/cdk/` or `ui/material/`, never in a feature module — 017's boundary test enforces this and will fail the build otherwise
