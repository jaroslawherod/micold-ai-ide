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
- `crates/micold-client/src/ui/*.rs` — feature modules. They may be **edited** where a call site must name a type role, pass a density, or migrate a placeholder onto a label (T017–T021, T047, T053, T055, T061). They may not **style** anything; 017's boundary test fails the build if they do.
- `assets/fonts/` — vendored font binaries, license and provenance.

Test command throughout: `mise run test` (`cargo test --workspace`).

---

## Phase 0: Token values (prerequisite — plan.md Phase A)

**Goal**: The six scale tables exist as tested data in the render-free core. Every later phase reads
from here; nothing below can start until this is green.

**Why this exists**: plan.md's phase table calls Phase A "the only hard prerequisite within this
feature — every story reads token values from it", and it had no tasks at all. Added by
`/speckit-analyze` on 2026-07-28.

### The one task that cannot be deferred ⏱️

- [X] T000y Frame-time instrumentation, so T000z has something to measure with. `micold-core`'s
  `frame_probe` module (warm-up discard, nearest-rank p95, the reported line) driven by
  `MICOLD_FRAME_PROBE=<frames>[:<warm_up>]` in the client. Nothing in the workspace could produce a
  frame-time figure before this, so FR-039b/FR-039c/SC-018 rested on a capability that did not
  exist. Tested in `micold-core/tests/frame_probe.rs`; `micold-client/tests/frame_probe_glue.rs`
  keeps the forced-frame subscription unreachable on an ordinary launch, since it would otherwise
  hold the render loop awake and falsify SC-017 with every other gate still green (FR-039b)
- [X] T000x The 20-worktree fixture the reference scene is built on: `scripts/reference-scene.sh`
  (`mise run fixture`). Repeatability is the requirement, not convenience — SC-018 compares three
  figures across a change that alters what the sidebar draws, so a scene hand-built twice is two
  scenes and the difference lands in the figure without appearing in it. Also the fixture
  `quickstart.md`'s Prerequisites asks for: all ten conventional types, with and without issue keys,
  one untyped row, one long enough to ellipsize, one orphaned directory with a health tag. The
  script verifies its own output and refuses to report success on a wrong row count (FR-039b)
- [X] T000z Before any token value changes, capture the frame-time figure for the **pre-change**
  build on the **baseline** reference scene of FR-039b — 20 worktrees in the sidebar, the sidebar
  expanded, one running terminal session, a context menu open over a dialog. **No ripple**: it does
  not exist yet, which is precisely why the scene is split in two. Build the repository with
  `mise run fixture`, compose steps 2–4 by hand, then take it with
  `MICOLD_FRAME_PROBE=300 MICOLD_FRAME_PROBE_SCENE=baseline mise run run`, which composes and
  verifies the scene itself, and record the printed line in `quickstart.md` §B8 — which also states
  what the figure does and does not cover. Reported for trend, not gating (FR-039c). **This cannot
  be done after T000f**: once the palette lands the pre-change build is gone, and SC-018 needs all
  three figures from the same machine (FR-039b, SC-018).
  **Captured 2026-07-29** — `300 frames — mean 0.84 ms, p95 1.07 ms, max 1.40 ms`, three runs
  recorded in §B8 with the noise floor (p95 moves < 0.02 ms run to run), so T076a's delta can be
  told from variance

### Tests for Phase 0 (MANDATORY — write first, confirm they FAIL) ⚠️

- [X] T000a [P] Failing test in `crates/micold-core/tests/tokens_contrast.rs` asserting every
  foreground/background role pair specified to carry text or icons meets WCAG AA (≥ 4.5:1) in both
  schemes, covering every pair this feature introduces and not only those carried over from feature
  003. The test MUST fail the build on any violation (FR-004, FR-005, SC-001)
- [X] T000b Failing test in `crates/micold-core/tests/tokens_contrast.rs` — same file as T000a, so
  not parallel with it — asserting each tonal
  ramp is monotonic in luminance, so a transcription error in a baked ramp surfaces as a failure
  rather than as a subtly wrong colour (plan.md risk 1, research R7)
- [X] T000c [P] Failing test in `crates/micold-core/tests/tokens_scales.rs` asserting the shape of
  every scale: 15 type roles plus 3 sidebar-scoped roles, each carrying size, weight, line height
  and a recorded tracking value; 6 elevation levels, each carrying a tonal role and a shadow;
  7 shape sizes; 7 state-layer opacities; motion tokens partitioned into the standard and
  emphasized sets (FR-007, FR-014, FR-018, FR-020, FR-033, FR-042)
- [X] T000d [P] Failing test in `crates/micold-core/tests/tokens_density.rs` asserting the density
  scale has exactly four steps, that each step below 0 subtracts 4dp, and that no component
  resolves to a fractional height (FR-026b)

### Implementation for Phase 0

- [X] T000e Expand `crates/micold-core/src/tokens.rs` (216 lines) into the `tokens/` module
  directory per plan.md's Structure Decision, preserving the existing `spacing` scale unchanged and
  carrying `Rgb`, `Roles`, `type_scale` and `sidebar` across **without re-valuing them**, so the
  move is separable from the value change in review
- [X] T000f Author `crates/micold-core/src/tokens/palette.rs` — the Material 3 baseline tonal ramps
  (tones 0–100 per key palette) generated from seed `#6750A4`, and re-author every semantic role in
  `LIGHT` and `DARK` as a palette-and-tone pair rather than a hand-picked value, so contrast follows
  from the tone delta (FR-001, FR-005a, FR-005b, D1, D3)
- [X] T000g Author the ten conventional-commit tag hues and the issue tag in
  `crates/micold-core/src/tokens/palette.rs` — one fixed hue per type, read at the accent tone
  recipe (fill 40 / text 100 light, fill 80 / text 20 dark). No tag may be a hand-tuned value
  outside the tonal system (FR-006, FR-006a)
- [X] T000h [P] Author `crates/micold-core/src/tokens/typography.rs` — the 15 Material 3 type roles
  plus the 3 sidebar-scoped reduced-density roles, each carrying size, weight, line height, and the
  Material tracking value recorded but not applied at render time (FR-007, FR-011, FR-042)
- [X] T000i [P] Author `crates/micold-core/src/tokens/elevation.rs`, `shape.rs`, `state.rs` and
  `motion.rs` — 6 elevation levels (tonal shift plus shadow), the 7-size shape scale superseding
  today's 4 radii, 7 state-layer opacities, and the named duration and easing tokens split into the
  standard and emphasized sets (FR-014, FR-018, FR-019, FR-020, FR-033)

**Checkpoint**: `mise run test` green with the contrast gate live.

⚠️ **This phase is where the app changes colour.** `roles(scheme)` returns `LIGHT`/`DARK` to every
call site already, so re-authoring them at T000f turns the accent from today's blue to the baseline
purple *immediately* — before a single component is restyled. T000g shifts the ten tag colours the
same way. Both are intended (FR-005b), and `quickstart.md` §B0 exists so this is confirmed rather
than discovered. Run §B0 at the end of this phase, not after Phase 1.

---

## Phase 1: User Story 1 - The interface reads as Material at a glance (Priority: P1) 🎯 MVP

**Goal**: Real depth. Graded surface tones, drop shadows on everything Material elevates, Material's corner sizes, decorative borders gone.

**Independent Test**: Open the app in light, then dark. Open a dialog, a context menu and the project switcher popover. Each floats above what is behind it; levels are distinguishable without borders; no container carries an outline that is not a divider, an outlined control, or a focus ring.

### Tests for User Story 1 (MANDATORY — write first, confirm they FAIL) ⚠️

- [X] T001 [P] [US1] Failing test in `crates/micold-client/src/ui/material/style_elevation.rs` (inside the crate, not `tests/` — the style layer is `pub(crate)` by 017 FR-002 and `material_boundary.rs` treats that as the enforcement, so an integration test cannot reach it; same resolution `style_snapshot` already uses) asserting each elevated style function returns a style whose shadow blur is non-zero and whose background matches the level's tonal role, and that an elevation-0 surface returns no shadow, in both schemes (FR-015, FR-016, SC-002)
- [X] T002 [P] [US1] Failing test in `crates/micold-client/src/ui/material/style_outline_discipline.rs` (inside the crate — see T001) asserting no style function carrying an elevation also sets a non-transparent border (FR-002, FR-003)
- [X] T003 [P] [US1] Failing test in `crates/micold-client/src/ui/material/style_shape.rs` (inside the crate — see T001) asserting buttons and chips resolve `full`, cards resolve `medium`, dialogs resolve `extra_large` (FR-019)

### Implementation for User Story 1

- [X] T004 [US1] Add the elevation→shadow conversion in `crates/micold-client/src/ui/material/style.rs`, folding Material's key and ambient shadows into the single shadow the renderer exposes per widget (research R1)
- [X] T005 [US1] Wire the elevation scale into `crates/micold-client/src/ui/material/surface.rs` via `.elevation()` and `.shape()` (FR-015)
- [X] T006 [US1] Rewrite the surface, dialog, menu, sidebar and toolbar style functions in `crates/micold-client/src/ui/material/style.rs` to draw from the elevation scale and graded surface-container roles, removing the 1px outline each uses to fake depth (FR-002, FR-015)
- [X] T007 [P] [US1] Apply the `full` pill radius to every button variant in `crates/micold-client/src/ui/material/button.rs` and `icon_button.rs` (FR-019)
- [X] T008 [P] [US1] Apply the `extra_large` (28) corner and the dialog surface role in `crates/micold-client/src/ui/material/modal.rs` (FR-019, FR-028)
- [X] T009 [US1] Remove decorative borders in `crates/micold-client/src/ui/material/tree_view.rs`, `toolbar.rs`, `terminal_pane.rs`, `progress.rs` and `toggle_chip.rs`, retaining only genuine dividers in `outline_variant` (FR-002, FR-003, SC-002)
- [X] T010 [US1] Draw the modal scrim at 32% `scrim` in `crates/micold-client/src/ui/material/modal.rs` (FR-028, contract §4)
- [X] T011 [US1] Verify overlapping elevated surfaces render in elevation order with independent shadows in `crates/micold-client/src/ui/material/modal.rs` and `menu.rs` — a context menu over a dialog must not flatten into it (FR-017)
- [X] T012 [US1] Update `docs/user-guide/` to describe the new surface hierarchy and the accent-color change from blue to baseline purple (FR-041, FR-005b, Principle VII)

**Checkpoint**: The app reads as Material at a glance. Demonstrable via quickstart §B1.

---

## Phase 2: User Story 2 - Text has a typographic voice (Priority: P2)

**Goal**: Roboto ships with the app; every text site resolves a named type role carrying size, weight and line height.

**Independent Test**: Change the OS UI font and relaunch — the app is unchanged. A dialog's title, body and caption are each distinguishable without relying on position. Terminal output is still monospaced.

**Note**: Feature 017 already routed every text site through `material::Text`, so this story assigns *roles* rather than hunting call sites.

### Tests for User Story 2 (MANDATORY — write first, confirm they FAIL) ⚠️

- [X] T013 [P] [US2] Failing test in `crates/micold-client/tests/roboto_font.rs` asserting both shipped faces parse via `ttf-parser` and report weight 400 and 500 (FR-008a, SC-012)
- [X] T014 [P] [US2] Failing test in `crates/micold-client/tests/type_role_call_sites.rs` asserting no source file passes a raw numeric literal as a text size, weight or line height — every site resolves a named role (FR-010, SC-003)

### Implementation for User Story 2

- [X] T014a [US2] Vendor Roboto Regular (400) and Roboto Medium (500) as static instances into `assets/fonts/`, alongside the Material Symbols font already there, and document them to the same standard: the Apache-2.0 licence text in-repo, and a provenance record naming the upstream source, the exact artifact shipped, and how it was produced. Decide explicitly whether the existing `assets/fonts/LICENSE` and `assets/fonts/PROVENANCE.md` are extended to cover both typefaces or whether Roboto gets its own pair — those files today describe only the icon font, and a licence file that silently grows to cover a second work is the failure mode this requirement exists to prevent (FR-008a, FR-009, SC-012)
- [X] T015 [US2] Register both Roboto faces via `.font(...)` and set `.default_font(...)` to Roboto in `crates/micold-client/src/main.rs`, keeping the Material Symbols registration intact (FR-008, research R3)
- [X] T016 [US2] Resolve type roles into size, font weight and absolute line height inside `crates/micold-client/src/ui/material/text.rs`, so the role is the only thing a call site names (FR-007, FR-010)
- [X] T017 [P] [US2] (call sites already name roles via 017's `TypeRole`; this is the refinement pass — pick a *more specific* role where the current one is coarse) Assign the correct type roles across `crates/micold-client/src/ui/shell.rs`, `project_selector.rs` and `terminal.rs`
- [X] T018 [P] [US2] Assign the correct type roles across `crates/micold-client/src/ui/worktree_form.rs`, `worktree_rename.rs`, `rename.rs` and `settings_form.rs`
- [X] T019 [P] [US2] Assign the correct type roles across `crates/micold-client/src/ui/about.rs`, `confirm_delete.rs`, `confirm_forget.rs`, `confirm_session_remove.rs` and `mod.rs`
- [X] T020 [P] [US2] Assign the correct type roles inside `crates/micold-client/src/ui/material/` — `tree_view.rs`, `menu.rs`, `toolbar.rs`, `select.rs`, `progress.rs`, `project_switcher.rs`, `icon_button.rs`, `tag.rs`
- [X] T021 [US2] Apply the sidebar-scoped roles in `crates/micold-client/src/ui/sidebar.rs` so the 80% density decision is one auditable mapping (FR-011)
- [X] T022 [US2] Confirm glyph fallback for characters outside Roboto's coverage at the font registration in `crates/micold-client/src/main.rs` (FR-013)
- [X] T023 [US2] Update `docs/user-guide/` to note the shipped typeface and resulting cross-platform consistency (FR-041, Principle VII)

> **What T017–T021 turned out to be.** The tasks read as a taste pass — swap a coarse role for a
> finer one. Two things made them larger, and both are recorded here because the task text no longer
> describes the work done:
>
> 1. **Eleven shared components were never on the scale at all.** `menu`, `toolbar`, `select`,
>    `progress`, `project_switcher`, `tag`, `tree_view`, `connection_banner`, `toggle_chip`,
>    `activity_badge` and the tooltip still named feature 003's `type_scale::BODY`/`sidebar::TAG`
>    constants. Those are *named* values, so `type_role_call_sites.rs` — which scanned for numeric
>    literals — passed them. They got the right size and, because a bare number carries neither, the
>    renderer's default weight and line spacing: no menu item, chip or toolbar title had ever
>    rendered in Roboto Medium. T020 is therefore a migration, not a reassignment, and the gate
>    gained a second rule that is structural rather than a blocklist (the scale is named in exactly
>    one file). `tokens::type_scale` and `tokens::sidebar` are deleted, as `tokens/mod.rs` said they
>    would be "when the last call site names a role instead (T017–T021)".
>
> 2. **`TypeRole` had nowhere finer to go.** Its eight variants narrowed Material's fifteen, so
>    "pick a more specific role" was not expressible. Three were added — `Section` (`title_medium`),
>    `Caption` (`body_small`) and `Action` (`label_large`) — and the refinement is mostly one
>    distinction: **prose was being set in label roles.** Every explanatory paragraph in
>    `worktree_form.rs`, every error line and diagnostic, the connection banner's detail and the
>    tooltip were at weight 500, which is the voice Material reserves for things you scan, not
>    things you read. `Caption` and `Label` are deliberately the same size and differ only in
>    weight; `src/ui/material/type_role_mapping.rs` pins that, because a mapping that collapsed the
>    pair would leave every size in the application correct.
>
> One latent bug surfaced: the overflow menu's longest label fit its 220dp panel by about a pixel,
> and only at the body weight. `layout_snapshot.rs` caught it wrapping. `PANEL_WIDTH` is now 240.

**Checkpoint**: Typography is role-driven and platform-independent. Demonstrable via quickstart §B2.

---

## Phase 3: User Story 3 - The interface responds under the pointer and the keyboard (Priority: P3)

**Goal**: State layers on every interactive surface, Material's **ripple** on every press, and a focus indicator wherever focus is reachable.

**Independent Test**: Hover every interactive element and confirm a visible change; click each and confirm a ripple from the click point; tab into a text field and confirm a focus indicator.

**Note**: The ripple's state lives **inside the component instance**, not in the core and not in the application (FR-024e). Principle I is satisfied by testing the component directly — feature 017 established that a client-level test can drive a component and assert its state, so this is an ordinary automated test rather than a case for the GUI-wiring exception.

### Tests for User Story 3 (MANDATORY — write first, confirm they FAIL) ⚠️

- [X] T024 [P] [US3] Failing tests for ripple state in `crates/micold-client/tests/ripple_state.rs`, driving the component directly the way `idle_requests_no_frames.rs` drives the motion primitive: pressing element B mid-ripple leaves A's progress and origin untouched; a completed ripple releases its state so nothing is retained at rest; an origin outside the element's bounds is clamped; with no known pointer position the origin is the element's center; the end radius reaches the element's furthest corner. State is read from the component instance — no central registry and no animation key (FR-024b, FR-024d, FR-024e)
- [X] T025 [P] [US3] Failing test in `crates/micold-client/src/ui/material/style_states.rs` (inside the crate with the other style gates — the style layer is `pub(crate)` by 017 FR-002, so `tests/` cannot reach it; T025/T026/T027 share the file) asserting each interactive style function returns visibly different output for active, hovered and pressed, with the pressed delta at least the hover delta (FR-021, SC-005)
- [X] T026 [P] [US3] Failing test in `crates/micold-client/src/ui/material/style_states.rs` (see T025) asserting the focused text-input status yields the 3dp `secondary` focus indicator, distinguishable from hovered (FR-022)
- [X] T027 [P] [US3] Failing test in `crates/micold-client/src/ui/material/style_states.rs` (see T025) asserting disabled content resolves the 0.38 opacity, including the self-coloring icon-glyph path (FR-023)

### Implementation for User Story 3

- [X] T028 [US3] Confirm the coordinate space the pointer area reports, against a real widget, before finalising the ripple renderer — an origin in the wrong frame places every ripple incorrectly, and the terminal canvas works in absolute window coordinates, so element-relative conversion may be required (FR-024g, plan risk: ripple origin coordinate space).
  **Answered**: `iced::mouse::Cursor` exposes three accessors and they are in *different frames* —
  `position()` is absolute window coordinates, `position_over(bounds)` is absolute but `None`
  outside, and `position_in(bounds)` is **element-relative** and `None` outside. The ripple takes
  `position_in`: the canvas draws in the element's own local frame, so element-relative is what the
  geometry needs, and its `None` maps exactly onto FR-024b's "no known position → centre". The risk
  was real — `terminal_pane.rs` uses `position()` and converts by hand against `bounds`, so the
  absolute convention is already present in this codebase and would have been the easy wrong
  choice. `Ripple::press` documents which frame it expects
- [X] T029 [US3] Build the ripple renderer in `crates/micold-client/src/ui/cdk/ripple.rs` — press capture, geometry, phase progression and per-instance lifetime, all held **inside the component instance** and carrying no colour or opacity of its own. Frames are requested through the motion primitive, never directly, so 017's single-frame-request gate stays at one entry (FR-024b, FR-024d, FR-024e, FR-024f, FR-039e)
- [ ] ~~T030~~ — merged into T029. A separate `crates/micold-core/src/ripple.rs` is **not** created: FR-024e places ripple origin, progress and lifetime in the component instance and forbids registering an animation key, so central state would contradict the requirement it claimed to serve. Kept struck rather than deleted so the numbering stays stable.
- [ ] ~~T031~~ — merged into T028, which said the same thing.
- [X] T032 [US3] Create the `Ripple` component in `crates/micold-client/src/ui/material/ripple.rs` per `contracts/component-api.md` §2.1a — expanding circle drawn with the canvas facility, clipped to the element's shape, beneath content and above container (FR-024a, FR-024b)
- [X] T033 [US3] Compose `Ripple` inside `crates/micold-client/src/ui/material/button.rs`, `tree_view.rs`, `menu.rs`, `tag.rs`, `toggle_chip.rs` and `icon_button.rs` so every interactive surface ripples without any call site opting in (FR-024c).
  **`tag.rs` deliberately excluded**: `Tag` is a static display chip with no `on_press`, so wrapping
  it would ripple a surface the user cannot press. `ToggleChip` — the *interactive* chip — does
  ripple. Buttons with no `on_press` are likewise left unwrapped, since a ripple on a disabled
  control reports a press that will never happen and contradicts its own disabled styling
- [X] T034 [US3] Add the state-layer compositing helper to `crates/micold-client/src/ui/material/style.rs` as the single place any state layer is applied (FR-020)
- [X] T035 [US3] Apply the full state-layer set to the shared text-button style in `crates/micold-client/src/ui/material/style.rs`, which brings list rows, tree items and menu items to life (FR-021, research R9)
- [X] T036 [P] [US3] Apply the state-layer set to the filled, outlined and icon button styles in `crates/micold-client/src/ui/material/style.rs` (FR-021)
- [X] T037 [P] [US3] Apply the state-layer set to chips and tags in `crates/micold-client/src/ui/material/tag.rs` and `toggle_chip.rs`, preserving AA under every state (FR-021, FR-024)
- [X] T038 [P] [US3] Apply the state-layer set plus the **focus** indicator to `crates/micold-client/src/ui/material/text_field.rs` (FR-021, FR-022)
- [X] T038a [P] [US3] Apply the state-layer set to `crates/micold-client/src/ui/material/select.rs`, driving the active indicator from the **open** state rather than focus — the rendering stack's select reports only active, hovered and open, and has no focus concept to observe (FR-021, FR-043a)
- [X] T039 [US3] Add the persistent `selected` treatment — `secondary_container` fill with `on_secondary_container` text — in `crates/micold-client/src/ui/material/tree_view.rs` and `filter_panel.rs` (FR-020, contract §7.2)
- [X] T040 [US3] Update `docs/user-guide/` to describe hover, ripple, selection and focus feedback, recording that keyboard focus indicators exist only on text fields (FR-041, FR-043, Principle VII)

**Checkpoint**: The UI responds under the pointer everywhere. Demonstrable via quickstart §B3.

---

## Phase 4: User Story 4 - Components match the components they claim to be (Priority: P4)

**Goal**: Correct anatomy — app bar, row densities, button targets, dialogs, menus, chips, the filled text field, the progress indicator, and the notification surface as a real snackbar.

**Independent Test**: Compare each component against its `contracts/design-tokens.md` §7 entry. Trigger several notifications and confirm one-at-a-time queueing with timed dismissal.

### Tests for User Story 4 (MANDATORY — write first, confirm they FAIL) ⚠️

- [X] T041 [P] [US4] Failing tests for the notification queue in `crates/micold-core/tests/notify_queue.rs`: never more than one visible; a duplicate of the visible notification is not enqueued; the cap drops oldest pending and never the visible one; an error's duration is strictly longer than an info's; manual dismissal promotes the next pending immediately (FR-032a, FR-032b)
- [X] T042 [P] [US4] Failing test in `crates/micold-core/tests/tokens_anatomy.rs` asserting every component anatomy constant — app bar height, both row densities, minimum touch target, dialog padding, menu item height, chip height, text field height, progress thickness, snackbar min height — matches contract §7 (FR-025 – FR-032, SC-008)
- [ ] T043 [P] [US4] Failing test in `crates/micold-client/tests/app_bar_scroll.rs` asserting the app bar's elevated flag derives from the sidebar's scroll offset (FR-025a)
- [X] T044 [P] [US4] Failing test in `crates/micold-client/tests/text_field_anatomy.rs` asserting the filled container role, rounded-top/square-bottom corners, and a bottom active indicator that thickens to 2dp accent on focus (FR-031)
- [X] T044a [P] [US4] Failing test in `crates/micold-client/tests/form_field_anatomy.rs` asserting the wrapper composes the shared parts around **whichever control it is given**: the label renders inside the container above the value in the label role and on-container colour; supporting text renders beneath in the supporting role; in the error state both the active indicator and the supporting text switch to the error role; leading and trailing adornment slots render when supplied and take no space when not; and the same assertions hold with a text input **and** with the select wrapped (FR-031a, FR-031b, FR-031c)

### Implementation for User Story 4

- [X] T044b [US4] Create the `FormField` wrapper in `crates/micold-client/src/ui/material/form_field.rs`, on the model of Angular Material's form field — the precedent this library already mimics. It owns the filled container, the active indicator, the in-container label, the supporting-text slot, the error presentation and the optional leading/trailing adornment slots, and it wraps whichever control it is handed rather than replacing it. Chainable builder terminating in `.into()` per `contracts/component-api.md` §2.1 (FR-031a, FR-031b, FR-031c, Principle VIII)
- [X] T045 [US4] Apply the filled text-field anatomy in `crates/micold-client/src/ui/material/text_field.rs` — 56dp height, per-corner radius, 16dp padding. The container and active indicator come from `FormField` (T044b); this task styles the input itself, not the shared chrome (FR-031)
- [X] T046 [US4] Compose `TextField` inside `FormField` so the label renders persistently in its floating position and the supporting-text slot is available, without `text_field.rs` reassembling either part itself (FR-031a, FR-031b, FR-031c, FR-044)
- [X] T047 [US4] Migrate the seven input call sites off placeholder-as-label onto label + supporting text per the contract §7.7 migration table, across `crates/micold-client/src/ui/worktree_form.rs`, `rename.rs`, `worktree_rename.rs` and `settings_form.rs` (FR-031a, FR-031b)
- [X] T048 [US4] Compose `crates/micold-client/src/ui/material/select.rs` inside `FormField` so it gets the same container, label, supporting text and active indicator every other field has — the indicator thickening and taking the accent colour on the **open** state rather than on focus, since the select cannot report focus (FR-043a) — and style its dropdown as a menu via the per-instance menu style, which does expose a shadow (FR-031, FR-031c, FR-031d, FR-043a)
- [ ] T048a [US4] Compose `crates/micold-client/src/ui/material/typeahead.rs`'s search field inside `FormField` and give it `.label()`/`.supporting()`/`.error()`, then move the free-standing `Branch` label in `crates/micold-client/src/ui/worktree_form.rs` inside that container. Feature 021 landed the type-ahead after §7.7 was written, so the contract's migration table names only the select — but the branch picker is now a type-ahead, and it is the one field left showing its name above the container while every other field shows it inside (FR-031a, FR-031c)
- [ ] T048b [US4] Drive the type-ahead's active indicator from its **open** state. This is the one control where FR-043a's "open rather than focus" is actually reachable: `pick_list` reports `Opened` to its own style closure and to nobody else, which is why `Select::active` has to be supplied and is left unset — but `Typeahead` already takes `.open(bool)` from a caller that tracks it, so the indicator can follow it for real rather than resting permanently (FR-043a, accepted gap #3)
- [ ] T049 [US4] Apply the linear progress anatomy in `crates/micold-client/src/ui/material/progress.rs` — `secondary_container` track, `primary` indicator, 4dp thickness, fully rounded (FR-031e)
- [ ] T050 [US4] Replace the static 0.4 fill in `crates/micold-client/src/ui/material/progress.rs` with Material's indeterminate presentation, so the bar stops asserting a completion fraction the application cannot know (FR-031f)
- [X] T051 [US4] Implement the notification queue in `crates/micold-core/src/notify.rs` — one visible, ordered pending queue, severity-derived duration, dedup and cap preserved (FR-032a, FR-032b)
- [ ] T052 [US4] Create the `Snackbar` component in `crates/micold-client/src/ui/material/snackbar.rs` per `contracts/component-api.md` §2.2 (FR-032, Principle VIII)
- [ ] T053 [US4] Replace the inline notification strip in `crates/micold-client/src/ui/mod.rs` with the floating snackbar overlay, above the dialog scrim and not obstructing a dialog's action row (FR-032)
- [ ] T053a [P] [US4] Assert the connection-status banner stayed a separate component: a test confirming `ConnectionBanner` still renders as a full-width, non-dismissible, non-queued strip and does not route through the snackbar queue. Material treats banners and snackbars as different components, and folding one into the other is the specific mistake this requirement forbids (FR-032c)
- [ ] T054 [US4] Rework `crates/micold-client/src/ui/material/toolbar.rs` to the small app bar anatomy — 64dp height, 16dp padding, `title_large` title, 48dp icon targets — and add `.elevated(bool)` (FR-025)
- [ ] T055 [US4] Wire elevate-on-scroll: add the scroll handler to the sidebar's scrollable in `crates/micold-client/src/ui/sidebar.rs`, a message variant and view-state flag in `crates/micold-client/src/app.rs`, and pass it to the toolbar builder (FR-025a, research R10)
- [ ] T056 [P] [US4] Add the dense (36dp) and standard (48dp) row densities to `crates/micold-client/src/ui/material/tree_view.rs`, defaulting the sidebar to dense (FR-026, FR-026a)
- [ ] T057 [P] [US4] Enforce the 48dp minimum interactive target in `crates/micold-client/src/ui/material/icon_button.rs` (FR-027)
- [ ] T058 [P] [US4] Apply dialog anatomy in `crates/micold-client/src/ui/material/modal.rs` — 24dp padding, `headline_small` title, `body_medium` body, trailing-aligned action row with 8dp gap (FR-028)
- [ ] T059 [P] [US4] Apply menu anatomy in `crates/micold-client/src/ui/material/menu.rs` — `surface_container`, elevation 2, `extra_small` corner, 48dp items (FR-029)
- [ ] T060 [P] [US4] Apply chip anatomy in `crates/micold-client/src/ui/material/tag.rs` and `toggle_chip.rs` — 32dp height, `full` corner, `label_large` (FR-030)
- [ ] T061 [US4] Confirm the known-projects list in `crates/micold-client/src/ui/project_selector.rs` uses the standard row density while the sidebar stays dense (FR-026)
- [ ] T062 [US4] Update `docs/user-guide/` to document the snackbar's one-at-a-time queueing and timed dismissal — the single sanctioned behavior change (FR-036a, FR-041, Principle VII)

> **T044 and T044a moved in-crate.** Both name a path under `crates/micold-client/tests/`, and
> neither is reachable: `material` is `pub(crate)`, so a `FormField` or a field style cannot be
> constructed from an integration test at all. They live beside the code as
> `src/ui/material/form_field_anatomy.rs` and `text_field_anatomy.rs`, following the precedent
> `style_snapshot.rs` set for the same reason.
>
> **What the two tests each cover.** The chrome's colour and thickness is a pure function of
> `(roles, active, error)`, so it is asserted directly — an indicator that thickened without
> recolouring, or an error state that recoloured the supporting text and left the label muted, is
> caught by arithmetic rather than by eye. The *composition* is a layout question, so it is laid out
> and measured: the container at exactly 56dp, the indicator at exactly 1dp, supporting text adding
> height beneath rather than inside, and an absent adornment contributing no layout node at all.
> The same assertions run over a wrapped **select** as over a text input, which is the half of
> FR-031c a single-control test would silently not cover.

**Checkpoint**: Components match their Material counterparts. Demonstrable via quickstart §B4.

---

## Phase 5: User Story 5 - Movement feels like Material (Priority: P5)

**Goal**: Existing animations adopt Material's durations and easing; the four new animations do the same.

**Independent Test**: Trigger each existing animation and confirm it still starts, completes and ends in the same visual state, at the new timing and with acceleration rather than constant-rate motion.

### Tests for User Story 5 (MANDATORY — write first, confirm they FAIL) ⚠️

- [ ] T063 [P] [US5] Failing test in `crates/micold-client/tests/motion_tokens.rs` asserting every animated track's duration and easing resolve to a named core motion token, that the sidebar slide uses the emphasized set while small fades use the standard set, and that no animation uses a hardcoded per-tick step (FR-034, SC-010)

### Implementation for User Story 5

- [ ] T064 [US5] Rework the motion primitive in `crates/micold-client/src/ui/cdk/motion.rs` (017 moved it there; `src/motion.rs` no longer exists) so track speeds derive from the core duration tokens and apply the named easing curves, keeping the `animating()` guard and its single frame-request site intact so no work runs at rest (FR-033, FR-034, FR-039a, FR-039e)
- [ ] T065 [US5] Apply the assigned duration and easing per contract §6.3 in `crates/micold-client/src/ui/material/animation.rs`, `menu.rs`, `tree_view.rs` and `crates/micold-client/src/ui/sidebar.rs`, preserving each animation's existing trigger, start state and end state (FR-034, FR-035)
- [ ] T066 [US5] Drive the four new animations from the same tokens — app bar elevation in `crates/micold-client/src/ui/material/toolbar.rs`, snackbar enter/exit in `snackbar.rs`, indeterminate progress in `progress.rs`, ripple expand/fade in `ripple.rs` — and confirm no fifth animation is introduced (FR-035a, SC-010)
- [ ] T067 [US5] Confirm 017's `crates/micold-client/tests/idle_requests_no_frames.rs` still passes unchanged with all four new animations in play — its structural half already proves no module outside the motion primitive asks for a frame, so this is a check that the gate was honoured rather than a fresh hand-verification. Add **one new test case** to that file for the indeterminate indicator's external settle condition — running while an operation is in flight, stopped within one frame of the operation ending. Adding a case is not editing the gate: the `SANCTIONED` constant stays at exactly one entry, which is what FR-039e forbids changing (FR-024d, FR-039a, FR-039d, FR-039e)
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
- [ ] T076a On the same machine that produced T000z's figure, capture the two post-change measurements: the **baseline** scene, and the **full** scene with a ripple mid-animation. Record both in `quickstart.md` §B8 alongside the pre-change figure. Compare: baseline-before vs baseline-after is like-for-like, and any gap there is a regression in rendering this feature did not add; baseline-after vs full is this feature's own cost. Neither gates the build; a regression is a review finding (FR-039b, FR-039c, SC-018)

---

## Dependencies & Execution Order

### Prerequisite: feature 017

**Every phase here depends on [`017-material-component-architecture`](../017-material-component-architecture/tasks.md) being complete.** That feature wraps the rendering stack, splits the library into behavior and appearance layers, consolidates the overlays, moves presentation state into components, and relocates the tokens to the render-free core — all with zero visual change.

Because it landed first, every task below changes appearance in **one place**. If a task here tempts you to edit a feature module to change how something looks, 017's boundary was not closed properly — fix that, don't work around it.

### Phase Dependencies

- **Phase 0 (token values)**: depends only on 017. **Every other phase depends on it** — each story
  reads token values from here, so nothing below starts until it is green. T000z runs before T000f
  and cannot be recovered afterwards
- **US1 (Phase 1)**: depends on Phase 0. Independent of the other stories
- **US2 (Phase 2)**: depends on Phase 0. 017 already routed every text site through the text component, so this assigns *roles* rather than hunting call sites
- **US3 (Phase 3)**: depends on Phase 0. Within it, ripple appearance builds on 017's behavior layer
- **US4 (Phase 4)**: depends on Phase 0. Reads type roles from US2 and state layers from US3; lands correctly without them, just unstyled in those respects
- **US5 (Phase 5)**: depends on Phase 0. Its final task covers animations introduced in US3 and US4, so run it after those
- **Polish (Phase 6)**: depends on all desired stories

### Within Each Story

- Tests written and confirmed failing before implementation (Principle I)
- Token values before their application
- Pure decision logic before its rendering — the notification queue before the snackbar
- User-guide documentation ships in the same change (Principle VII)

### Parallel Opportunities

- T000a, T000c, T000d — Phase 0 test files (T000b shares T000a's file and follows it)
- T000h, T000i — Phase 0 scale authoring, different files (T000e–T000g are sequential: the module
  split, then the palette, then the tags that read it)
- T001, T002, T003 — US1 test files
- T007, T008 — US1 shape work in different files
- T013, T014 — US2 test files
- T017–T020 — four type-role assignment tasks, different modules
- T024–T027 — US3 test files
- T036–T038a — US3 style application in different concerns
- T041–T044a — US4 test files
- T053a, T056–T060 — component anatomy and guard tasks, all different files
- T069–T071 — polish cleanups
- **Whole stories**: US1–US5 can be staffed concurrently once **Phase 0** is green (017 alone is
  not enough — every story reads token values)

---

## Implementation Strategy

### MVP (User Story 1)

1. Confirm 017 is complete and its parity gate passed
2. **T000z** — capture the pre-change frame-time figure. Unrecoverable once T000f lands in step 3
3. Phase 0 — token values in the core, with the contrast gate live
4. **STOP and VALIDATE** — `quickstart.md` §B0, both schemes. The app is already purple here
5. Phase 1 (US1) — surfaces, elevation, shape
6. **STOP and VALIDATE** — `quickstart.md` §B1, both schemes

US1 alone changes the application's identity: real depth, Material's corners, and the baseline purple accent.

### Incremental Delivery

1. 017 complete → foundation closed, **nothing looked different yet**
2. + Phase 0 → token values authored and gated. **The first visible change**: the accent turns
   purple and the tag colours shift, because `roles(scheme)` already feeds every call site
3. + US1 → depth and shape (**MVP** — the first *structural* visual change)
3. + US2 → typographic voice and cross-platform parity
4. + US3 → ripple and live state feedback
5. + US4 → correct component anatomy, real text fields, the snackbar
6. + US5 → Material motion
7. Polish → regression pass, three-platform verification

### Risk Notes

- **T000f and T000g are the visible break point** — the app turns purple and tag colors shift, in Phase 0, before any component is restyled. Both are intended; `quickstart.md` §B0 exists so this is confirmed rather than discovered
- **The regression pass is a merge gate, not a formality.** This feature permits exactly one behavioral difference: the snackbar
- **Do not re-litigate 017's decisions here.** If a wrapper cannot express something this feature needs, extend the wrapper

---

## Notes

- `[P]` = different files, no dependencies on incomplete tasks
- Confirm every test fails before implementing against it (Principle I)
- Every appearance change belongs in `ui/cdk/` or `ui/material/`, never in a feature module — 017's boundary test enforces this and will fail the build otherwise
