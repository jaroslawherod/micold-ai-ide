---

description: "Task list for Material Design Layout & Theming"
---

# Tasks: Material Design Layout & Theming

**Input**: Design documents from `/specs/003-material-design-layout/`

**Prerequisites**: plan.md ✅, spec.md ✅, research.md ✅, data-model.md ✅, contracts/ ✅ (design-tokens.md, theme-behavior.md, settings-schema.md)

**Tests**: MANDATORY per Constitution Principle I (Test-First, NON-NEGOTIABLE) **for production logic**. All *decision* logic here is pure and lives in the render-free core, so its failing tests are written and reviewed first and run under `cargo test --no-default-features` (no iced). US1 introduces **no new core logic** (pure styling on iced primitives), so it has no unit tests to write first — it is verified by the CI GUI build + the `quickstart.md` §3 walkthrough. This is called out explicitly in the US1 phase.

**Documentation**: MANDATORY per Constitution Principle VII. Each user-facing story ships its section of `docs/user-guide/appearance-theming.md` in the same change.

**Cross-platform**: Per Constitution Principle VI, build + tests MUST pass on Linux, macOS, and Windows, **including OS theme detection**. OS detection is confined to the `dark-light` boundary in the binary; no `cfg(target_os)` in core.

**Organization**: Tasks grouped by user story for independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: US1 / US2 / US3 (maps to spec.md user stories)
- Exact file paths included in every task

## Path Conventions

Single-project desktop app (per plan.md). Rust **lib + bin** layout: `src/lib.rs` exposes the
render-free core (new modules `tokens`, `theme`, `settings`) so tests in `tests/` drive them
without iced; the iced rendering layer (`src/ui/`) is bin-only behind the `gui` feature. Paths
are repo-relative. The one new dependency (`dark-light`) is **optional**, enabled by the `gui`
feature, so `cargo test --no-default-features` still compiles the core without it.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Add the feature's dependency and docs/CI scaffolding.

- [X] T001 Update `Cargo.toml`: add `dark-light` as an **optional** dependency and include it in the `gui` feature (`gui = ["dep:iced", "dep:dark-light"]`); pin the version per research.md R3.
- [X] T002 [P] Update `.github/workflows/ci.yml` docs job to also assert `docs/user-guide/appearance-theming.md` exists (Principle VII / VI).
- [X] T003 [P] Create `docs/user-guide/appearance-theming.md` (stub with the per-story section headings) and add its link to `docs/README.md` (Principle VII).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The centralized design system (pure tokens) and the base `ColorScheme`, the iced
`Theme` builder + shared style helpers, and the themed-render wiring that every restyled surface
depends on.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T004 [P] Write the contrast invariant test in `tests/tokens.rs` (TDD — must FAIL first): for both `LIGHT` and `DARK`, assert every `on_*` role meets WCAG AA (≥ 4.5:1) against its paired surface using the relative-luminance formula (SC-005; contracts/design-tokens.md).
- [X] T005 [P] Implement the pure design tokens in `src/tokens.rs` (makes T004 pass): `struct Rgb`, `struct Roles`, `const LIGHT`/`const DARK` (values from contracts/design-tokens.md), the `type_scale`/`spacing`/`shape` constants, and `fn roles(scheme: ColorScheme) -> Roles`. No iced.
- [X] T006 [P] Define `enum ColorScheme { Light, Dark }` in `src/theme.rs` (`Debug, Clone, Copy, PartialEq, Eq`), per data-model.md. `ThemePreference`/`SystemScheme`/`resolve` are added in their stories.
- [X] T007 Register the new core modules in `src/lib.rs` (`pub mod tokens; pub mod theme;`) so `tests/` can drive them (depends on T005, T006).
- [X] T008 Implement `src/ui/style.rs` (gui): `fn theme(scheme: ColorScheme) -> iced::Theme` building `Theme::custom(name, Palette)` from `tokens::roles(scheme)` (Rgb→`iced::Color`, palette mapping per contracts/design-tokens.md), plus shared style helpers — `surface`/`app_bar` container styles, `filled`/`outlined`/`text` button styles with `button::Status` states (Active/Hovered/Pressed/Disabled), and a `list_item` style (depends on T005, T006).
- [X] T009 Wire themed rendering in `src/main.rs` and `src/ui/mod.rs` (depends on T008): add `.theme(|state| ui::style::theme(ColorScheme::Light))` (temporary fixed scheme, replaced in US2) and `.default_font(...)` on the `iced::application(...)` builder; wrap the base view in a themed background container in `ui/mod.rs`.

**Checkpoint**: The design system exists, is contrast-tested, and the app renders through a custom Material theme in a single (light) scheme. Surfaces can now be restyled.

---

## Phase 3: User Story 1 - Coherent Material layout across the shell (Priority: P1) 🎯 MVP

**Goal**: Every existing surface (app bar, shell header/empty state, known-projects list, About,
project selector, rename) is restyled to the Material design system with consistent typography,
spacing, elevation, and button variants — with no behavior change.

**Independent Test**: `cargo run --features gui` and walk through every screen/dialog per
`quickstart.md` §3: confirm Material layout, correct button variants + hover/press/focus/disabled
states, preserved active marker / "git" badge / unavailable state, and usable reflow at small
window sizes. All prior actions still work.

### Tests for User Story 1

> **NOTE (Principle I)**: US1 adds **no new pure core logic** — it only maps existing state onto
> the foundational style helpers. There are therefore no failing-first unit tests to add here; the
> style helpers and token values are already covered by the foundational contrast test (T004) and
> the CI GUI build. Verification is the `quickstart.md` §3 manual walkthrough (T015). This matches
> the established 001/002 convention (the iced rendering layer is verified by the CI GUI build +
> quickstart; TDD applies to the render-free core), and plan.md's Constitution Check records
> Principle I as PASS on that basis.

### Implementation for User Story 1

- [X] T010 [P] [US1] Restyle `src/ui/toolbar.rs` into a Material top app bar (surface container + `title` typography + primary actions), using the `app_bar`/button helpers from `src/ui/style.rs`. Preserve the existing Help/About entries and behavior (FR-010).
- [X] T011 [P] [US1] Restyle `src/ui/shell.rs`: active-project header and empty state as Material `surface` cards (`display`/`headline`/`body`/`label` typography, filled primary button), and the known-projects list as `list_item`-styled rows preserving the active marker, "git" badge, unavailable state, and Open/Rename buttons with correct enabled/disabled styling (FR-011, FR-012, FR-014, FR-015).
- [X] T012 [P] [US1] Restyle `src/ui/about.rs` to the design system (surface, typography, buttons) — behavior unchanged (FR-013).
- [X] T013 [P] [US1] Restyle `src/ui/project_selector.rs` to the design system (surface, list items, git icons, action buttons) — behavior unchanged (FR-013).
- [X] T014 [P] [US1] Restyle `src/ui/rename.rs` to the design system (dialog surface, text input, buttons, validation-error styling) — behavior unchanged (FR-013).
- [ ] T015 [US1] Verify FR-014/FR-015/FR-016 via `quickstart.md` §3: button variants + interactive states are visibly distinct, and the layout reflows usably when the window is resized small (depends on T010–T014).
- [X] T016 [US1] Document the new look in `docs/user-guide/appearance-theming.md` (the "Appearance & layout" section) (Principle VII).

**Checkpoint**: The whole shell is coherently Material in the light scheme; all 001/002 behavior intact. MVP demoable.

---

## Phase 4: User Story 2 - Light and dark themes following the system (Priority: P2)

**Goal**: The app renders in light or dark, following the OS preference by default and updating
live when the OS preference changes, with both schemes fully designed.

**Independent Test**: Set OS to dark → launch → app is dark (no light flash). Switch OS to light
while running → app switches within ~1s, no restart. Reverse. Every screen legible in both
(`quickstart.md` §4).

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

> **Write these FIRST and ensure they FAIL before implementation.**

- [X] T017 [P] [US2] Write `tests/theme.rs` exercising `theme::resolve(pref, system)` against every row of the truth table in contracts/theme-behavior.md (Light/Dark overrides ignore the OS; FollowSystem tracks Light/Dark; `Unspecified` → Light per FR-018).

### Implementation for User Story 2

- [X] T018 [US2] In `src/theme.rs`, add `enum ThemePreference { FollowSystem, Light, Dark }` (serde-derive, `Default = FollowSystem`), `enum SystemScheme { Light, Dark, Unspecified }`, and `fn resolve(pref, system) -> ColorScheme` (makes T017 pass) (data-model.md; FR-005, FR-007, FR-018).
- [X] T019 [US2] Extend the core `State` in `src/app.rs`: add `theme_pref: ThemePreference` and `system_scheme: SystemScheme` fields, a `fn color_scheme(&self) -> ColorScheme` helper (calls `theme::resolve`), the `SystemThemeChanged(SystemScheme)` `Message` variant, and its pure reducer arm (sets `system_scheme`; not persisted) (depends on T018; FR-006).
- [X] T020 [US2] In `src/main.rs`, map `dark_light::Mode → SystemScheme` at the boundary, call `dark_light::detect()` in `boot` to seed `state.system_scheme`, and change the `.theme(...)` closure to `ui::style::theme(state.color_scheme())` (depends on T019; FR-005, SC-002).
- [X] T021 [US2] In `src/main.rs`, add an OS-scheme polling `Subscription` (`iced::time::every(Duration::from_millis(500))` — sub-second so SC-003's "within 1 second" holds worst-case) that maps `dark_light::detect()` to `SystemScheme` and emits `SystemThemeChanged` **only when the value changes** (no flicker); fold it into the existing `subscription(...)` (depends on T020; FR-006, SC-003).
- [X] T022 [US2] Document system-following theming in `docs/user-guide/appearance-theming.md` (the "Automatic light/dark" section), including the Linux portal fallback note (Principle VII).

**Checkpoint**: App follows the OS live in both fully-designed schemes; US1 layout intact under both.

---

## Phase 5: User Story 3 - User-configurable theme override (Priority: P3)

**Goal**: The user can override the system default with a fixed Light/Dark choice that persists
across restarts, and can return to "Follow system".

**Independent Test**: With OS light, choose **Dark** in the app → turns dark, ignores OS → restart
→ still dark. Choose **Follow system** → resumes tracking the OS live (`quickstart.md` §5).

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

> **Write these FIRST and ensure they FAIL before implementation.**

- [X] T023 [P] [US3] Write `tests/settings_roundtrip.rs`: a `Settings` save→load roundtrip preserves the theme; a missing file and a corrupt file both yield `Settings::default()` (`FollowSystem`) with the correct `LoadStatus`; writes are atomic (contracts/settings-schema.md; FR-009, FR-019).

### Implementation for User Story 3

- [X] T024 [US3] Implement `src/settings.rs` (makes T023 pass): `struct Settings { theme: ThemePreference }` (serde, snake_case, `Default`), the `SettingsStore` trait + `LoadOutcome`/`LoadStatus`, and `JsonFileSettingsStore` (a `settings.json` in the same dir as the projects store, atomic temp-file + rename write, missing/corrupt → default with `.bak` preservation), reusing the pattern in `src/store.rs`; register `pub mod settings;` in `src/lib.rs` (contracts/settings-schema.md; Principle IV).
- [X] T025 [US3] In `src/app.rs`, add the `ThemePreferenceChanged(ThemePreference)` `Message` variant and its pure reducer arm (sets `state.theme_pref`) (depends on T019).
- [X] T026 [P] [US3] Implement `src/ui/theme_menu.rs` (gui): a Material control offering **Follow system / Light / Dark** that emits `ThemePreferenceChanged`, styled via `src/ui/style.rs` and reflecting the current `theme_pref` (depends on T025; FR-007, FR-008).
- [X] T027 [US3] Host the theme menu in the app bar in `src/ui/toolbar.rs` (depends on T026).
- [X] T028 [US3] In `src/main.rs`, load `Settings` in `boot` to seed `state.theme_pref`, and persist the updated `Settings` on `ThemePreferenceChanged` at the I/O boundary (non-fatal on failure) (depends on T024, T025; FR-009, SC-004).
- [X] T029 [US3] Document the theme override + persistence in `docs/user-guide/appearance-theming.md` (the "Choosing your theme" section) (Principle VII).

**Checkpoint**: All three stories functional — coherent Material layout, OS-following theming, and a persisted user override.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Cross-cutting verification and cleanup after all stories.

> Per-story docs already shipped in their phases (Principle VII). This phase only reviews the index.

- [X] T030 [P] Cross-cutting docs review: confirm `docs/user-guide/appearance-theming.md` is complete and linked/navigable from `docs/README.md`.
- [X] T031 Run `cargo fmt --all -- --check` and both clippy passes (`--no-default-features` and `--features gui`) with `-D warnings`; fix any findings.
- [X] T031a [P] SC-007 audit: grep the restyled `src/ui/*.rs` for literal colors (`Color::`, `rgb`/`#` hex), pixel sizes, and radii; confirm every value comes from `src/tokens.rs` via `src/ui/style.rs` — zero per-widget magic numbers.
- [ ] T032 Verify `cargo test --no-default-features --all-targets` and `cargo build --features gui` pass on Linux, macOS, and Windows (Principle VI) — including a spot check of OS theme detection on each.
- [ ] T033 Run the full `quickstart.md` validation (§1–§6) end-to-end.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately.
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories.
- **User Stories (Phase 3–5)**: All depend on Foundational.
  - **US1 (P1)**: Independent after Foundational — pure restyle in the light scheme.
  - **US2 (P2)**: Independent after Foundational — adds theme resolution + OS following. Does not require US1, but is normally done after it.
  - **US3 (P3)**: Depends on **US2** for the `theme_pref` field and `color_scheme()` plumbing (T019); adds the override message, persistence, and the menu on top.
- **Polish (Phase 6)**: Depends on all desired stories being complete.

### Within Each User Story

- Tests (where present) written and FAILING before implementation (Principle I).
- Core enums/logic before the binary wiring that consumes them.
- User-guide documentation ships in the same story (Principle VII).
- Story complete only when its tests pass, docs exist, and it builds on all three platforms.

### Parallel Opportunities

- Setup: T002, T003 in parallel.
- Foundational: T004, T005, T006 in parallel (T004 is the failing test for T005); T007 after T005/T006; T008 after; T009 after T008.
- US1: T010–T014 all in parallel (separate `ui/*` files); T015/T016 after.
- US2: T017 (test) in parallel with nothing blocking; then T018→T019→T020→T021 sequential (T018 is one file, T019–T021 chain through `app.rs`/`main.rs`); T022 in parallel.
- US3: T023 (test) parallelizable; T024 makes it pass; T026 parallel with T024 once T025 lands; T027/T028 after; T029 in parallel.

---

## Parallel Example: User Story 1

```bash
# Restyle all surfaces together (separate files, no shared state):
Task: "Restyle src/ui/toolbar.rs into a Material top app bar"
Task: "Restyle src/ui/shell.rs header/empty-state/known-projects list"
Task: "Restyle src/ui/about.rs to the design system"
Task: "Restyle src/ui/project_selector.rs to the design system"
Task: "Restyle src/ui/rename.rs to the design system"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational (CRITICAL — the design system + themed rendering).
3. Complete Phase 3: User Story 1 (restyle every surface, light scheme).
4. **STOP and VALIDATE**: walk through `quickstart.md` §3.
5. Demo — the app already looks Material end-to-end.

### Incremental Delivery

1. Setup + Foundational → design system ready.
2. US1 → coherent Material layout → demo (MVP).
3. US2 → light/dark following the OS live → demo.
4. US3 → persisted user override + theme menu → demo.
5. Each story adds value without breaking the previous.

---

## Notes

- [P] tasks = different files, no dependencies on incomplete tasks.
- [Story] label maps each task to its spec.md user story for traceability.
- US1 has no failing-first unit tests by design (pure styling, no new core logic) — verified by the GUI build + quickstart; all *logic* stories (US2, US3) keep TDD.
- Verify tests fail before implementing (US2, US3).
- Commit after each task or logical group.
- Keep `cfg(target_os)` out of the core; OS detection stays behind the `dark-light` boundary in the binary.
