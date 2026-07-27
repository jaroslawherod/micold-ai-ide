---

description: "Task list for Material Design Icons"
---

# Tasks: Material Design Icons

**Input**: Design documents from `/specs/004-material-icons/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/icon-api.md, quickstart.md

**Tests**: MANDATORY per Constitution Principle I (Test-First, NON-NEGOTIABLE). Every unit of
production code below is preceded by a failing, reviewed test (Red-Green-Refactor).

**Documentation**: MANDATORY per Constitution Principle VII. Each user-facing story ships its
user-guide docs in the same change.

**Cross-platform**: Per Constitution Principle VI, the embedded font + tests run identically on
Linux, macOS, Windows.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: US1 / US2 / US3 (Setup, Foundational, Polish have no story label)
- Exact file paths are included in every task.

## Path Conventions

Single-project layout (per plan.md): render-free core in `src/`, GUI in `src/ui/` + `src/main.rs`,
tests in `tests/`, font asset in `assets/fonts/`, docs in `docs/user-guide/`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Vendor the icon font asset and record the values the code will pin against.

- [X] T001 Obtain the Material Symbols Outlined **static** instance (weight 400, fill 0, grade 0, optical size 24 — research R2) and vendor it at `assets/fonts/MaterialSymbolsOutlined.ttf`.
- [X] T002 [P] Add `assets/fonts/LICENSE` (upstream Apache-2.0) and `assets/fonts/PROVENANCE.md` recording source, version/commit, axis instance, and the codepoints reference (FR-010, research R6).
- [X] T003 Inspect the shipped `.ttf` (e.g. `fc-query`/font tool) and record in `assets/fonts/PROVENANCE.md` the exact font **family name** and the **codepoint** for each curated glyph in the research R5 table (Help→`help`, About→`info`, OpenProject→`folder_open`, Rename→`edit`, Git→`commit`, ActiveMarker→`check_circle`, Unavailable→`error`, NavigateUp→`arrow_upward`).

**Checkpoint**: Font asset vendored; family name + per-icon codepoints known and documented.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The shared icon vocabulary (core) + font registration and render helper (GUI). No
surface can render an icon until this phase is complete.

**⚠️ CRITICAL**: Blocks all user stories.

### Tests (write first, MUST fail before implementation — Principle I)

- [X] T004 [P] Core mapping test in `tests/icons.rs` (runs under `--no-default-features`): asserts `Icon::glyph()` returns the pinned codepoint (from T003) for every variant, and `Icon::ALL` covers every variant with no duplicates (FR-003, SC-006).
- [X] T005 [P] Font-integrity test in `tests/icons_font.rs` (runs under `--features gui`): loads the embedded font and asserts every `Icon::glyph()` codepoint resolves to a real glyph (no tofu) and the pinned `MATERIAL_SYMBOLS` family name matches the file (SC-005). **Scope note (BUG-004, 2026-07-27)**: correct as written, but it iterates `Icon::ALL` only, so it cannot see a surface that skips the enum and passes a raw literal to `text(..)` — the blind spot that let feature 010's activity badge ship as tofu. Widening the guard is tracked as T103 in `specs/010-daemon-session-persistence/tasks.md`; this task stays closed.

### Implementation (make T004–T005 pass)

- [X] T006 Create the closed `Icon` enum with `const fn glyph(self) -> char` and `const ALL: &[Icon]` in `src/icons.rs` (render-free core; data-model.md, contracts/icon-api.md).
- [X] T007 Register `pub mod icons;` in `src/lib.rs`.
- [X] T008 Register the embedded icon font via the application builder `.font(include_bytes!("../assets/fonts/MaterialSymbolsOutlined.ttf"))` and define the `MATERIAL_SYMBOLS` `iced::Font` constant (pinned family name from T003) in `src/main.rs` (research R3).
- [X] T009 Implement the `icon(icon: Icon, size: u16, color: tokens::Rgb) -> Element` render helper in `src/ui/mod.rs`, reusing the existing `Rgb → iced::Color` conversion in `src/ui/style.rs` (FR-004, contracts/icon-api.md).

**Checkpoint**: Vocabulary + rendering ready; T004–T005 green. User stories can begin.

---

## Phase 3: User Story 1 - Consistent iconography across every surface (Priority: P1) 🎯 MVP

**Goal**: Every existing surface renders its mapped Material icon, with all prior actions
unchanged and the same concept using the same glyph everywhere (FR-005, FR-006, SC-001/002/003).

**Independent Test**: Launch the app, walk every screen/dialog, confirm each mapped
action/state shows its icon, same-concept glyphs match, and open/rename/reopen/about/select
all still work (quickstart.md §3).

### Tests (write first, MUST fail before implementation — Principle I)

- [X] T010 [P] [US1] Extend `tests/toolbar.rs` to assert the Help and About controls still emit `Message::HelpMenuToggled` / `Message::AboutOpened` after iconization (behavior preserved, FR-006).
- [X] T011 [P] [US1] Extend `tests/selector.rs` to assert the git badge, "Up" navigation, and open action are still present/functional with icons (FR-006).
- [X] T012 [P] [US1] Add `tests/shell_icons.rs` asserting the known-projects list preserves Open/Rename actions, the active marker, the git badge, and the blocked reopen for unavailable projects after iconization (FR-005, FR-006).

### Implementation (make tests pass)

- [X] T013 [US1] Apply `Icon::Help` and `Icon::About` to the app-bar Help action and Help→About action in `src/ui/toolbar.rs` (keep messages unchanged).
- [X] T014 [US1] Apply `Icon::OpenProject` (empty-state + "open another" buttons), `Icon::Rename`, `Icon::Git` badge, `Icon::ActiveMarker` (replacing the `●` glyph), and `Icon::Unavailable` in `src/ui/shell.rs`.
- [X] T015 [US1] Apply `Icon::NavigateUp`, `Icon::Git` badge, and `Icon::OpenProject` in `src/ui/project_selector.rs`.
- [X] T016 [US1] Ensure icon-only controls retain their prior wording as accessible/tooltip meaning across `src/ui/toolbar.rs`, `src/ui/shell.rs`, `src/ui/project_selector.rs` (FR-011).
- [X] T017 [US1] Update `docs/user-guide/appearance-theming.md` (or a new `docs/user-guide/icons.md` linked from `docs/README.md`) to describe the shared icon vocabulary and where each icon appears (FR-013, Principle VII).

**Checkpoint**: MVP — every surface shows icons, all behavior preserved, docs updated.

---

## Phase 4: User Story 2 - Icons correct in light and dark themes (Priority: P2)

**Goal**: Every icon is legible and tinted to the correct foreground role for its surface in
both themes, and colors switch live on theme change with none mismatched/invisible (FR-007,
SC-004).

**Independent Test**: View every screen in light and dark, confirm icon colors match each
surface's foreground role; toggle the OS theme while running and confirm live update
(quickstart.md §4).

### Tests (write first, MUST fail before implementation — Principle I)

- [X] T018 [P] [US2] Add core mapping test in `tests/icon_roles.rs` (runs under `--no-default-features`): for every icon call site, assert `icons::icon_role(site, roles)` returns a foreground role (one of `on_surface` / `on_primary` / `on_surface_variant` / `on_error`) and never a background/surface/primary-fill role, so the existing AA-contrast token guarantees (`tests/tokens.rs`) transitively cover icon legibility in both schemes (FR-007, SC-004).

### Implementation (make test pass)

- [X] T019 [US2] Define an `IconSurface` enum and a pure `fn icon_role(surface: IconSurface, roles: tokens::Roles) -> tokens::Rgb` in `src/icons.rs` (render-free core) returning the correct foreground role per surface (app-bar action → `on_surface`; primary button → `on_primary`; badge/caption → `on_surface_variant`; unavailable marker → `error`). Single source of truth for every call site (FR-004, FR-007).
- [X] T020 [US2] Route every `icon(..)` call site in `src/ui/toolbar.rs`, `src/ui/shell.rs`, `src/ui/project_selector.rs` through `icons::icon_role(..)` against the active scheme's `tokens::Roles`, so light/dark and disabled states propagate through the existing style path (FR-007).

  > Live theme-switch is NOT separately unit-tested: icons reuse the same per-frame `Roles` lookup as all other text, so the existing OS-poll theme mechanism (`src/main.rs`, research R4) drives icon recolor automatically. Verified manually via quickstart.md §4 step 3 (SC-004 live-switch clause).

- [X] T021 [US2] Confirm disabled controls render their icon in the control's disabled visual state consistently (edge case: disabled controls) in `src/ui/shell.rs` (Open on unavailable) and `src/ui/toolbar.rs`.

**Checkpoint**: Icons correct and legible in both themes, live-updating; US1 + US2 both pass.

---

## Phase 5: User Story 3 - A reusable icon vocabulary for future surfaces (Priority: P3)

**Goal**: A contributor can render any icon by its stable name at any size/foreground color from
a new call site, and an unknown icon name fails at build time (FR-002, FR-003, SC-005).

**Independent Test**: From a fresh call site, render an icon by name at varied size/color;
confirm the correct glyph and that referencing an undefined variant is a compile error
(quickstart.md §1–§2).

### Tests (write first, MUST fail before implementation — Principle I)

- [X] T022 [P] [US3] Add a reusability test in `tests/icons_font.rs` (`--features gui`) that calls the `icon(..)` helper for every `Icon::ALL` variant at two sizes (`type_scale::LABEL`, `type_scale::DISPLAY`) and two foreground roles, asserting each renders without panic (reusability guarantee).

### Implementation (make test pass)

- [X] T023 [US3] Ensure `Icon`, `Icon::glyph`, `Icon::ALL`, `icon_role`, `MATERIAL_SYMBOLS`, and `icon(..)` are publicly exported with rustdoc from `src/icons.rs` and `src/ui/mod.rs` (documented reusable API).
- [X] T024 [US3] Add an "Adding a new icon" contributor note (curated set → add a variant, pin its codepoint in `PROVENANCE.md`, extend the T004 mapping test) to `docs/user-guide/icons.md` (or the icons section from T017), documenting the build-time-safety guarantee (FR-003).

**Checkpoint**: All three stories independently functional.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Final verification across stories and platforms.

- [X] T025 [P] Cross-cutting docs review: ensure `docs/README.md` links the icons documentation and terminology is consistent across the user guide.
- [X] T026 Verify the render-free core stays green without the GUI: `cargo test --no-default-features --all-targets` (SC-006).
- [X] T027 Verify build + full test suite pass on Linux, macOS, and Windows in CI (Principle VI, SC-007).
- [X] T028 Run `quickstart.md` validation end-to-end (§1–§6), explicitly confirming §4 step 3 (toggle OS theme while running → all icons recolor, none left on the previous theme) and that no "tofu" appears (SC-004, SC-005).

## Phase 7: Convergence

- [X] T029 Correct spec.md's Assumptions section, which still describes "a curated subset of
  icons... not the entire Material Symbols catalog": feature 009 (research R6) replaced the
  shipped `assets/fonts/MaterialSymbolsOutlined.ttf` with **full glyph coverage** so adding an
  `Icon` variant never requires regenerating the font (see `assets/fonts/PROVENANCE.md`). Added
  an alignment note; no code change needed — the shipped font already reflects the current
  reality per FR-002/FR-003 (contradicts)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately. T003 depends on T001.
- **Foundational (Phase 2)**: Depends on Setup (needs the vendored font + pinned codepoints/family). BLOCKS all user stories.
- **User Stories (Phase 3–5)**: All depend on Foundational completion.
  - US1 (P1) is the MVP and has no dependency on US2/US3.
  - US2 (P2) audits/args the US1 call sites; run after US1 (or alongside, but its test asserts against surfaces US1 introduces).
  - US3 (P3) depends only on Foundational (the enum + helper); independent of US1/US2.
- **Polish (Phase 6)**: Depends on all desired stories being complete.

### Within Each Story

- Tests are written and observed FAILING before implementation (Principle I).
- Core (`src/icons.rs`) before GUI (`src/ui/`, `src/main.rs`).
- User-guide docs accompany the story in the same change (Principle VII).

### Parallel Opportunities

- T002 ∥ (after T001); T004 ∥ T005 (different files).
- US1 tests T010 ∥ T011 ∥ T012 (different files).
- US3 (Foundational-only dependency) can proceed in parallel with US1/US2 by a second contributor.
- Setup + Foundational must complete before any story.

---

## Parallel Example: User Story 1

```bash
# Write all US1 tests first (they must fail), in parallel — different files:
Task: "Extend tests/toolbar.rs for Help/About behavior preservation"      # T010
Task: "Extend tests/selector.rs for git badge / Up / open"                # T011
Task: "Add tests/shell_icons.rs for list actions/markers/badge"           # T012
```

---

## Implementation Strategy

### MVP First (User Story 1 only)

1. Phase 1 Setup → 2. Phase 2 Foundational (CRITICAL) → 3. Phase 3 US1 → **STOP & VALIDATE**
   (walk every surface, confirm icons + unchanged behavior) → demo.

### Incremental Delivery

1. Setup + Foundational → backbone ready.
2. US1 → visible icons everywhere (MVP).
3. US2 → verified theme correctness.
4. US3 → documented reusable vocabulary + build-time safety.

---

## Notes

- [P] = different files, no incomplete-task dependency.
- Verify each test fails before implementing it.
- Commit after each task or logical group.
- The closed `Icon` enum is the guardrail: an unknown icon is a compile error, never runtime tofu.
