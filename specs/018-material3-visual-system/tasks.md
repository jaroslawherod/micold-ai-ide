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
- [X] ~~T030~~ — merged into T029. A separate `crates/micold-core/src/ripple.rs` is **not** created: FR-024e places ripple origin, progress and lifetime in the component instance and forbids registering an animation key, so central state would contradict the requirement it claimed to serve. Kept struck rather than deleted so the numbering stays stable.
- [X] ~~T031~~ — merged into T028, which said the same thing.
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
- [X] T043 [P] [US4] Failing test in `crates/micold-client/tests/app_bar_scroll.rs` asserting the app bar's elevated flag derives from the sidebar's scroll offset (FR-025a)
- [X] T044 [P] [US4] Failing test in `crates/micold-client/tests/text_field_anatomy.rs` asserting the filled container role, rounded-top/square-bottom corners, and a bottom active indicator that thickens to 2dp accent on focus (FR-031)
- [X] T044a [P] [US4] Failing test in `crates/micold-client/tests/form_field_anatomy.rs` asserting the wrapper composes the shared parts around **whichever control it is given**: the label renders inside the container above the value in the label role and on-container colour; supporting text renders beneath in the supporting role; in the error state both the active indicator and the supporting text switch to the error role; leading and trailing adornment slots render when supplied and take no space when not; and the same assertions hold with a text input **and** with the select wrapped (FR-031a, FR-031b, FR-031c)

### Implementation for User Story 4

- [X] T044b [US4] Create the `FormField` wrapper in `crates/micold-client/src/ui/material/form_field.rs`, on the model of Angular Material's form field — the precedent this library already mimics. It owns the filled container, the active indicator, the in-container label, the supporting-text slot, the error presentation and the optional leading/trailing adornment slots, and it wraps whichever control it is handed rather than replacing it. Chainable builder terminating in `.into()` per `contracts/component-api.md` §2.1 (FR-031a, FR-031b, FR-031c, Principle VIII)
- [X] T045 [US4] Apply the filled text-field anatomy in `crates/micold-client/src/ui/material/text_field.rs` — 56dp height, per-corner radius, 16dp padding. The container and active indicator come from `FormField` (T044b); this task styles the input itself, not the shared chrome (FR-031)
- [X] T046 [US4] Compose `TextField` inside `FormField` so the label renders in the position the field's state calls for and the supporting-text slot is available, without `text_field.rs` reassembling either part itself (FR-031a, FR-031b, FR-031c, FR-044)
- [X] T046a [US4] Give the label its **resting** position as well as its floating one, in `crates/micold-client/src/ui/material/filled_field.rs` and `form_field.rs`. T046 rendered it permanently floating — Material's *populated* field — so an empty one showed a caption hanging over an empty box with nothing beneath it to say that was where you type. An empty, inactive field now rests its name on the value's line at `body_large`, floats it to `body_small` at the top once populated or active, and the control suppresses its own placeholder while the label rests so the two do not overprint. One rule, `form_field::label_floats`, is consulted by both sides. Narrows accepted gap #4 to the transition alone (FR-031a, FR-044)
- [X] T047 [US4] Migrate the seven input call sites off placeholder-as-label onto label + supporting text per the contract §7.7 migration table, across `crates/micold-client/src/ui/worktree_form.rs`, `rename.rs`, `worktree_rename.rs` and `settings_form.rs` (FR-031a, FR-031b)
- [X] T047a [US4] Migrate the **gallery's** field samples off placeholder-as-label too, in `crates/micold-client/src/showcase/sections/controls.rs` and `samples.rs`. T047 moved the seven application call sites and left the showcase posing every field with its name in the placeholder and no label at all — so the one page whose entire job is to show what a component looks like was demonstrating the arrangement §7.7's migration table removes, and a call site copying from it would reintroduce what T047 had just finished undoing. `samples::PLACEHOLDER` splits into `FIELD_LABEL` (the name) and a genuine example (FR-031a, FR-031b, FR-020)
- [X] T048 [US4] Compose `crates/micold-client/src/ui/material/select.rs` inside `FormField` so it gets the same container, label, supporting text and active indicator every other field has — the indicator thickening and taking the accent colour on the **open** state rather than on focus, since the select cannot report focus (FR-043a) — and style its dropdown as a menu via the per-instance menu style, which does expose a shadow (FR-031, FR-031c, FR-031d, FR-043a)
- [X] T048a [US4] Compose `crates/micold-client/src/ui/material/typeahead.rs`'s search field inside `FormField` and give it `.label()`/`.supporting()`/`.error()`, then move the free-standing `Branch` label in `crates/micold-client/src/ui/worktree_form.rs` inside that container. Feature 021 landed the type-ahead after §7.7 was written, so the contract's migration table names only the select — but the branch picker is now a type-ahead, and it is the one field left showing its name above the container while every other field shows it inside (FR-031a, FR-031c)
- [X] T048b [US4] Drive the type-ahead's active indicator from its **open** state. This is the one control where FR-043a's "open rather than focus" is actually reachable: `pick_list` reports `Opened` to its own style closure and to nobody else, which is why `Select::active` has to be supplied and is left unset — but `Typeahead` already takes `.open(bool)` from a caller that tracks it, so the indicator can follow it for real rather than resting permanently (FR-043a, accepted gap #3)
- [X] T049 [US4] Apply the linear progress anatomy in `crates/micold-client/src/ui/material/progress.rs` — `secondary_container` track, `primary` indicator, 4dp thickness, fully rounded (FR-031e)
- [X] T050 [US4] Replace the static 0.4 fill in `crates/micold-client/src/ui/material/progress.rs` with Material's indeterminate presentation, so the bar stops asserting a completion fraction the application cannot know (FR-031f)
- [X] T051 [US4] Implement the notification queue in `crates/micold-core/src/notify.rs` — one visible, ordered pending queue, severity-derived duration, dedup and cap preserved (FR-032a, FR-032b)
- [X] T052 [US4] Create the `Snackbar` component in `crates/micold-client/src/ui/material/snackbar.rs` per `contracts/component-api.md` §2.2 (FR-032, Principle VIII)
- [X] T053 [US4] Replace the inline notification strip in `crates/micold-client/src/ui/mod.rs` with the floating snackbar overlay, above the dialog scrim and not obstructing a dialog's action row (FR-032)
- [X] T053a [P] [US4] Assert the connection-status banner stayed a separate component: a test confirming `ConnectionBanner` still renders as a full-width, non-dismissible, non-queued strip and does not route through the snackbar queue. Material treats banners and snackbars as different components, and folding one into the other is the specific mistake this requirement forbids (FR-032c)
- [X] T054 [US4] Rework `crates/micold-client/src/ui/material/toolbar.rs` to the small app bar anatomy — 64dp height, 16dp padding, `title_large` title, 48dp icon targets — and add `.elevated(bool)` (FR-025)
- [X] T055 [US4] Wire elevate-on-scroll: add the scroll handler to the sidebar's scrollable in `crates/micold-client/src/ui/sidebar.rs`, a message variant and view-state flag in `crates/micold-client/src/app.rs`, and pass it to the toolbar builder (FR-025a, research R10)
- [X] ⚠️ Reopened T056 [P] [US4] Add the dense and standard row densities to `crates/micold-client/src/ui/material/tree_view.rs`, defaulting the sidebar to dense (FR-026, FR-026a, FR-026d) *(reopened 2026-08-07 — BUG-005; closed by T115)*

  > Neither height is applied, and one of them never was. As landed in `8a044f5` the floor rode on
  > the row's indent spacer, so it worked at depth ≥ 1 and was dropped at depth 0 — iced deletes a
  > child whose size hint is void, and a depth-0 row's spacer is `Fixed(0)` wide. `1cb9873` then
  > removed the floor outright, taking the height off nested rows too. Both densities now resolve
  > to the same content height, which is the ad-hoc sizing FR-026 forbids. The figures also change:
  > §7.2 moves from Material 2's 48dp base to Material 3's 56 / 72, so dense is 44 / 60.
- [X] T057 [P] [US4] Enforce the 48dp minimum interactive target in `crates/micold-client/src/ui/material/icon_button.rs` (FR-027) *(reopened 2026-08-07 — BUG-002: the 48dp was written and then overwritten; closed the same day by T091)*
- [X] T058 [P] [US4] Apply dialog anatomy in `crates/micold-client/src/ui/material/modal.rs` — 24dp padding, `headline_small` title, `body_medium` body, trailing-aligned action row with 8dp gap (FR-028)
- [X] T059 [P] [US4] Apply menu anatomy in `crates/micold-client/src/ui/material/menu.rs` — `surface_container`, elevation 2, `extra_small` corner, 48dp items (FR-029) *(reopened 2026-08-07 — BUG-003: closed on the surface and shape half of §7.5 alone. The 48dp items did not arrive until T098, five phases later, and three further figures in the same table — 8dp panel padding, 12dp item padding, 24dp item icon — plus the 4dp gap the contract does not have are still unapplied today. Closed by T105.)*
- [X] T060 [P] [US4] Apply chip anatomy in `crates/micold-client/src/ui/material/tag.rs` and `toggle_chip.rs` — 32dp height, `full` corner, `label_large`, **label centred within the height** (FR-030, FR-030a) *(reopened 2026-08-07 — BUG-001; closed the same day by T085)*

  > `tag.rs` was done and stayed done: a `Tag` sets no height, so its container is its line box and
  > it has no slack to misplace. `toggle_chip.rs` applied the 32dp height and left the label drawn
  > at the top of it — the height landed, the alignment the height implies did not. Closed by T087,
  > behind T086's failing test (Principle I).
- [X] T061 [US4] Confirm the known-projects list in `crates/micold-client/src/ui/project_selector.rs` uses the standard row density while the sidebar stays dense (FR-026)
- [X] T062 [US4] Update `docs/user-guide/` to document the snackbar's one-at-a-time queueing and timed dismissal — the single sanctioned behavior change (FR-036a, FR-041, Principle VII)

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

> **⚠ FR-027 is unmet inside the sidebar, deliberately, and needs your call.**
>
> §7.3 asks for a 48×48 minimum interactive target on every control. §7.2's `dense` density — which
> FR-026a and FR-026c exist to protect — makes the sidebar's rows 36dp. The sidebar header carries
> four controls beside its title in a ~260dp panel, and the two requirements cannot both hold there:
> at 48dp each the title gets 60dp, at the dense 36dp it still gets 60dp, and the word "Worktrees"
> wants 73.5px. `tests/layout_text_overflow.rs` is what found it, by reporting the title painting
> past its clip.
>
> Every icon button outside the sidebar takes the full 48dp. Inside it, `IconButton::compact()`
> keeps the previous size. The alternatives are product decisions a styling pass should not take on
> its own: shrink the header title's role, drop one of the four controls, or widen the sidebar.
>
> **T058 partially applied.** The dialog's padding (24dp), title role (`headline_small`), body role
> (`body_medium`) and 8dp action gap already matched §7.4 after T017–T021, so the change is the
> 560dp maximum width, applied by `SurfaceKind::Dialog` so no dialog call site has to remember it.
> The 280dp *minimum* is not applied: the rendering stack's container has `max_width` and no
> matching floor, and every dialog here exceeds 280dp on its own content. The constant stays
> asserted against the contract in `tokens_anatomy.rs`.

**Checkpoint**: Components match their Material counterparts. Demonstrable via quickstart §B4.

---

## Phase 5: User Story 5 - Movement feels like Material (Priority: P5)

**Goal**: Existing animations adopt Material's durations and easing; the four new animations do the same.

**Independent Test**: Trigger each existing animation and confirm it still starts, completes and ends in the same visual state, at the new timing and with acceleration rather than constant-rate motion.

### Tests for User Story 5 (MANDATORY — write first, confirm they FAIL) ⚠️

- [X] T063 [P] [US5] Failing test in `crates/micold-client/tests/motion_tokens.rs` asserting every animated track's duration and easing resolve to a named core motion token, that the sidebar slide uses the emphasized set while small fades use the standard set, and that no animation uses a hardcoded per-tick step (FR-034, SC-010)

### Implementation for User Story 5

- [X] T064 [US5] Rework the motion primitive in `crates/micold-client/src/ui/cdk/motion.rs` (017 moved it there; `src/motion.rs` no longer exists) so track speeds derive from the core duration tokens and apply the named easing curves, keeping the `animating()` guard and its single frame-request site intact so no work runs at rest (FR-033, FR-034, FR-039a, FR-039e)
- [X] T065 [US5] Apply the assigned duration and easing per contract §6.3 in `crates/micold-client/src/ui/material/animation.rs`, `menu.rs`, `tree_view.rs` and `crates/micold-client/src/ui/sidebar.rs`, preserving each animation's existing trigger, start state and end state (FR-034, FR-035)
- [X] T066 [US5] Drive the four new animations from the same tokens — app bar elevation in `crates/micold-client/src/ui/material/toolbar.rs`, snackbar enter/exit in `snackbar.rs`, indeterminate progress in `progress.rs`, ripple expand/fade in `ripple.rs` — and confirm no fifth animation is introduced (FR-035a, SC-010)
- [X] T067 [US5] Confirm 017's `crates/micold-client/tests/idle_requests_no_frames.rs` still passes unchanged with all four new animations in play — its structural half already proves no module outside the motion primitive asks for a frame, so this is a check that the gate was honoured rather than a fresh hand-verification. Add **one new test case** to that file for the indeterminate indicator's external settle condition — running while an operation is in flight, stopped within one frame of the operation ending. Adding a case is not editing the gate: the `SANCTIONED` constant stays at exactly one entry, which is what FR-039e forbids changing (FR-024d, FR-039a, FR-039d, FR-039e)
- [X] T068 [US5] Update `docs/user-guide/` if motion is user-visible enough to warrant a note; otherwise record in the PR that no doc change was needed (Principle VII)

**Checkpoint**: All five stories complete and independently demonstrable.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T069 [P] Delete the superseded notification style function from `crates/micold-client/src/ui/material/style.rs` and any feature-003 token constants left unreferenced after Phase 2
  — **the style function is not superseded.** `style::notification` is what the *connection banner*
  draws with, and FR-032c requires the banner to stay a separate component from the snackbar: it
  reports a standing condition, is full-width and is not dismissible. Deleting it would have meant
  folding the banner in, which is the specific mistake that requirement forbids. What *was*
  superseded — feature 003's `tokens::type_scale` and `tokens::sidebar` — is gone, deleted during
  T017–T021 once the last call site named a role instead, and `tests/type_role_call_sites.rs` fails
  the build if either is re-added
- [X] T070 [P] ~~Fix the stale test command in `CLAUDE.md`~~ — already done as part of [017](../017-material-component-architecture/tasks.md) T003; kept here only so the numbering is stable
- [X] T071 [P] Cross-cutting documentation review and `docs/` index/navigation updates (Principle VII)
- [X] T072 **Signed off by the owner, 2026-08-07.** Run the full `quickstart.md` Part B walkthrough in the **light** scheme and record the result
- [X] T073 **Signed off by the owner, 2026-08-07.** Run the full `quickstart.md` Part B walkthrough in the **dark** scheme and record the result
- [X] T074 **Signed off by the owner, 2026-08-07.** Complete the no-behavior-change regression pass in `specs/018-material3-visual-system/quickstart.md` §B6. Any unchecked box there blocks merge; exactly one behavioral difference (the snackbar) is permitted (FR-036, FR-036a, SC-007)
- [X] T075 Verify build and full test suite pass on Linux, macOS and Windows via the CI workflow in `.github/workflows/` (Principle VI, FR-039). **Green on all three platforms** on `eb2000d`, 2026-08-07: `build + test (ubuntu-latest)`, `build + test (macos-latest)`, `build + test (windows-latest)`, `fmt + clippy` and `docs check` all `success`. The run was delayed by a GitHub Actions outage that spanned the whole of 2026-08-06, not by anything in the branch.
- [X] T076 Confirm the visible-worktree count rendered by `crates/micold-client/src/ui/sidebar.rs` has not dropped materially against the pre-change baseline, per `quickstart.md` §B4 (FR-026a)
  — **measured, not asserted.** Sidebar rows in `layout_snapshot.txt` are 23.6dp untagged and 41.6dp
  tagged, byte-identical to the pre-change fixture, so the visible count has not moved at all.
  Getting there required refusing part of §7.2: applying its 36dp dense row height takes those rows
  to 36.0 and 54.0 — a ~30% drop in what fits without scrolling, which is precisely the outcome
  FR-011 exists to prevent and which §7.2's own paragraph gives as the *reason* the dense density
  exists. Where the number and its stated purpose conflict the purpose wins, so the density governs
  the row's horizontal padding and icon gap and not its height.
  A bug was hiding this: the floor rode on the indent spacer, and a depth-0 row indents by zero —
  which makes the spacer `Fixed(0)` and therefore void, and iced drops a void child outright. The
  floor had silently never applied to a top-level row. Fixing that is what made the conflict
  measurable.

  > **⚠ Reopened for the record, 2026-08-07 — BUG-005. The measurement is sound; the conclusion
  > does not follow from it.** Two errors, and the second is inside the first.
  >
  > **The 54.0 is an artifact of the bug this note describes.** Having found the floor on the wrong
  > element, the arithmetic was done with it still there: a 36dp floor on the *name line* makes a
  > tagged row 36 + 2 + 16 = 54. Applied to the **row**, a tagged row is `max(36, 41.6)` = 41.6 —
  > unchanged. Measured on the same specimens: untagged 18.2 → 36.0, tagged 36.2 → 36.2, session
  > (depth 1) 18.2 → 36.0. The reference scene grows 7.7%, not ~30%. There was never a conflict
  > between §7.2 and FR-026a at that base; there was a floor attached to the wrong node.
  >
  > **"Byte-identical to the pre-change fixture" was true and proved nothing.** Every tree row in
  > all sixteen covered states is depth 0, and depth 0 is precisely where the floor had never
  > applied. The fixture could not have moved. Meanwhile every depth-1 row in the running
  > application lost 34% of its height (36.0 → 23.6), and the gallery's `TreeView` sample lost 51%
  > (48.0 → 23.6), unrecorded by anything. T116 adds the covered state that would have caught it.
  >
  > **And a `MUST` cannot be set aside here.** FR-026c requires the sidebar's rows to be the
  > standard row at density −3; FR-026b's scale has four steps and admits no other value. "The
  > purpose wins and the number does not apply" needed a spec amendment, not a code comment — and
  > the amendment, taken on 2026-08-07, goes the other way: §7.2 follows Material 3 and FR-026a is
  > the clause that gives ground. See `bugs/BUG-005.md`.
- [X] T076b Make the `full` reference scene composable at all, in `crates/micold-client/src/ui/material/ripple.rs`, `ui/mod.rs` and `main.rs`. `scene_facts` hardcoded `ripple_animating: false` with a comment saying the ripple "arrives with this feature's own tasks" — it arrived at T032 and nothing came back for this, so `Scene::Full` refused every run and T076a's third figure was unobtainable. A ripple only starts from a press and §B8 requires the scene to compose itself, so `Ripple::operate` now offers its own state and a traversal presses the first idle one it finds. A traversal rather than a registry is what keeps this inside FR-024e: the requirement is that no central record of ripple state exists, not that the tree cannot be walked (FR-039b, FR-024e, SC-018)
- [X] T076a **All three figures captured.** Figure 3 needed T083 first: six runs came out bimodal until the scene was re-verified during measurement rather than only while composing it. After T083, six consecutive runs landed within 0.12 ms. On the same machine that produced T000z's figure, capture the two post-change measurements: the **baseline** scene, and the **full** scene with a ripple mid-animation. Record both in `quickstart.md` §B8 alongside the pre-change figure. Compare: baseline-before vs baseline-after is like-for-like, and any gap there is a regression in rendering this feature did not add; baseline-after vs full is this feature's own cost. Neither gates the build; a regression is a review finding (FR-039b, FR-039c, SC-018)

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

---

## Phase 7: Convergence

Appended by `/speckit-converge`. Each item traces to the artifact that requires it and states the
gap type observed in the code. Ordered CRITICAL first.

- [X] T077 **CRITICAL** — Write a failing test first, then keep `pulse()` in `crates/micold-client/src/ui/material/ripple.rs` honest: it decides *which* ripple to press (the first one found, and only while that one is idle) and reports how many it saw animating. That is decision logic, and Principle I's GUI-wiring exception explicitly does not cover "any code with decision logic, branching, or a business rule of its own" — the file's neighbours (`ripple_clipping.rs`, `style_snapshot.rs`) already show that an in-crate `#[cfg(test)]` module reaches this code, so it is testable and merely untested. Cover the rule that matters to the measurement: a traversal over several ripples presses exactly one, leaves an already-running ripple alone rather than restarting it, and reports the count it observed rather than the count it asked for per Constitution I (missing)
- [X] T078 Wrap the pressables in `crates/micold-client/src/ui/material/project_switcher.rs` in `Ripple` — the switcher trigger (line ~75), every known-projects row (~154) and the trailing "Add project…" row (~173) are raw `button`s today. SC-005a asks for a ripple on 100% of interactive elements and `quickstart.md` §B3 names the known-projects row in the list a walkthrough must check, so this is a hole in the requirement rather than a component nobody specified. Use the row's own corner radius, as `tree_view.rs` does — a `shape::FULL` ripple on a square row is the bug `shape_bands` exists to prevent per SC-005a, FR-021, FR-024c (partial)
- [X] T079 Gate FR-039d: assert that `crates/micold-client/src/ui/worktree_form.rs` emits no indeterminate `Bar` when no operation is in flight. The mount is guarded by `is_creating` today and the guard is correct, but nothing fails the build if it stops being — and the failure is silent and total: `Bar` asks for a frame on every frame it exists, so one mounted outside an operation holds the render loop awake for ever and SC-017's "zero frames at rest" quietly stops being true. `idle_requests_no_frames.rs` covers the primitive, not this call site, and only the manual §B9 covers the call site today. The spec names this exact case in its Edge Cases per FR-039d, SC-017 (missing)
- [X] T080 Rebuild the connection banner's action in `crates/micold-client/src/ui/material/connection_banner.rs:76` on the shared `Button` with the outlined variant, rather than a raw `button` styled with `style::outlined(...)`. Two things follow from the raw form: the action does not ripple, and FR-027's variant set stops being the single definition of what an outlined button is — a second outlined button now exists that no change to `Button` reaches per FR-021, FR-027 (partial)
- [X] T081 **Resolved: recorded as accepted fidelity gap #5 (FR-045).** Resolve FR-027 inside the worktree sidebar, which the Phase 4 note records as knowingly unmet and awaiting a decision. §7.3's 48×48 minimum target and §7.2's dense 36dp row cannot both hold in the sidebar header, where four controls sit beside the title in a ~260dp panel; `IconButton::compact()` currently keeps the smaller target and `tests/layout_text_overflow.rs` is what surfaced the conflict. The three candidate resolutions are product decisions — shrink the header title's role, drop one of the four controls, or widen the sidebar — so this task is to make the call and then either meet the requirement or record the shortfall in `spec.md` as the fifth accepted, documented fidelity gap alongside FR-042/FR-043/FR-043a/FR-044. It must not stay in the current state, where the requirement is neither met nor waived per FR-027 (contradicts)
- [X] T082 **Resolved: recorded as accepted fidelity gap #6 (FR-046).** The strut was implemented and worked, and was withdrawn: it deepens every dialog's tree by a level and re-points six semantic anchors in the layout-snapshot harness, to hold a floor no dialog comes near. Reconcile the dialog's 280dp minimum width, which `contracts/design-tokens.md` §7.4 specifies (line ~519) and no code applies — the rendering stack's container offers `max_width` with no matching floor, and every dialog presently exceeds 280dp on its own content, so nothing looks wrong today. Either apply a floor (a `Space` of that width in the dialog's own column costs nothing when the content is already wider) or record it in `spec.md` as an accepted fidelity gap. As it stands the contract states a value the implementation does not honour, and the spec's list of four accepted gaps does not mention it per FR-028, contract §7.4 (partial)
- [X] T083 Re-verify the reference scene **during** measurement, not only while composing it, in `crates/micold-client/src/main.rs`. `Scene::check` runs until it passes and then never again, so the 300 counted frames are measured against whatever the window drifts into — and the `full` scene's six runs came out in two clusters 60% apart because of it, while interleaved baseline runs held steady. Check the facts again at the end of the run (and cheaply, per counted frame if it does not itself perturb the figure) and refuse to report rather than print a figure for a scene that stopped being the scene. This is the same class of error the check already exists to prevent — "there is nothing in `300 frames — mean 0.84 ms` that says what it was measured against" — reappearing one step later in the run. Then re-take figure 3 and fill §B8's third slot per FR-039b, SC-018 (missing)

---

## Feature close-out — 2026-08-07

Merged to `main` as `c9d09c4` (PR #73, rebase; 18 commits on `9769fee`). CI green on Linux, macOS
and Windows.

**Built and verified: 96 of 99 tasks.** Every automated gate passes, on all three platforms.

**Three tasks closed on the owner's sign-off** (2026-08-07) rather than on a recorded walkthrough:
T072 and T073, the light and dark Part B passes, and T074, the §B6 no-behaviour-change regression
pass. They are `[X]` because the owner closed them, which is the owner's call to make.

What that does *not* mean is that Part B was filled in. The checkboxes in `quickstart.md` Part B are
still empty, so there is no item-by-item record of what was exercised, and this close-out does not
manufacture one. Anyone auditing later should read those two facts together: the tasks are closed,
and the evidence behind them is a sign-off rather than a filled-in checklist.

The reason that distinction is worth keeping is what §B6 covers — the class of thing an automated
gate cannot see: a flow that still compiles, still passes its unit tests, and no longer works. Its
list — create/rename/delete a worktree, both branch sources with their reuse and overwrite
resolutions, session start/switch/remove, project open/filter/switch/forget, every keyboard shortcut
still reaching the terminal, quit-and-relaunch persistence — is a list of paths no test in this
repository exercises end to end. `quickstart.md` Part B remains the way to walk them if a regression
is ever suspected.

**Six accepted fidelity gaps** are recorded in `spec.md`, each a limit of the rendering stack rather
than a shortfall in the work: tracking not applied (FR-042), no focus ring on buttons/rows/menu
items/chips (FR-043), none on the select either (FR-043a), the field label snaps between its two
positions rather than animating (FR-044), the sidebar header keeps sub-48dp targets (FR-045), and
the dialog's 280dp floor is recorded but not applied (FR-046).

**One measured regression**, reported for trend as FR-039c requires and not gating anything: frame
composition `p95` 1.07 ms → 1.12 ms (+4.7%) on the baseline reference scene, against a ~0.02 ms
noise floor. Token lookups, type-role resolution, and the extra widgets `FormField` puts in the
tree. Figures and method in `quickstart.md` §B8.

---

## Phase 8: Convergence

Appended by `/speckit-converge` after the close-out above. Both items are the same root cause:
T050 changed what the progress indicator *is*, and left the gallery describing the one it replaced.

- [X] T084 Rewrite the `stage_progress` caption in `crates/micold-client/src/showcase/sections/surfaces.rs` (~line 183). It currently tells the reader that the indicator's "fill is a fixed value rather than a real fraction", that "it does not animate, so it asks for no frames (FR-023)", and that "the indeterminate indicator that *will* need one arrives with feature 018" — three statements that T050 made false, in the one document whose entire job is to describe what each component does. A gallery that describes the component it replaced is worse than one that says nothing: a reader has no reason to doubt it, and the next person to reach for a progress indicator will believe the fill is static. Say what it is now — an indeterminate bar whose segment travels on `long_2`, driven through `Progress`, which asks for a frame on every frame it exists per T049/T050, FR-031f (contradicts)
- [X] T085 **Decided: documented exemption, no run control.** A replay button suits a *transient* animation — press it, see the thing again — and an indeterminate bar is continuous by definition; a paused one is a static bar, which is the exact misreading T050 removed. The caption and the catalogue entry now both record that this component animates for as long as it is mounted, that the Components page therefore does not idle, and that the cost is accepted because the gallery is a development-only binary the packaging gate keeps out of the shipped package. The application's own quiescence is unaffected and separately gated by `tests/indeterminate_stops_with_its_operation.rs`. Decide what the gallery does about an indicator that never stops. `StageProgress` is posed permanently on the Components page — twice, since BUG-009 added the live-line pose — and `Bar` requests a frame on every frame it is mounted, so that page holds the render loop awake for as long as it is open. The catalogue records `live: &[]` and `interactive: false`, meaning "nothing here to exercise", and the section has no run control, unlike the motion section where every animation is replayable on demand. This is the showcase-side shape of exactly what T079 gated in the application, and no `showcase_*` test asserts the gallery ever idles. The showcase is a development-only binary, so this is not SC-017 proper — but the choice should be *made*: give the pose a run control the way motion entries have one, or record in the catalogue and the caption that this component animates continuously by nature and the page is knowingly exempt per FR-039a, SC-017, T050 (partial)

---

## Phase 9: Bugfix — BUG-001

Appended by `/speckit-bugfix-patch`, **after** the close-out above — the feature was closed and a
defect in it was reported the same day, which is what a bugfix phase is for. A `ToggleChip`'s label
sat against the top of the 32dp pill instead of on its centre line, so all 12dp of slack collected
beneath the text. See `bugs/BUG-001.md`.

**One false completion.** T060 was reopened above and closed again by T087: it had applied the
height that created the slack without the alignment the height requires. Its `tag.rs` half was
unaffected and stayed done.

- [X] T086 Failing test first: assert a chip's label is centred within the chip's laid-out bounds — the empty band above the label equal to the one below it, within the layout's rounding — and confirm it fails against today's `ToggleChip`. Read the *laid-out node*, not the constants: `tests/tokens_anatomy.rs` already pins `chip::HEIGHT` and passed throughout this defect, because a constant cannot say where the label went. Follow `tests/gates/containment.rs`, which reads the layout tree and is pulled in via `#[path]` from `tests/layout_snapshot.rs` (FR-030a, SC-008a, Principle I)

  > **Done, and both instructions in this task turned out to be wrong.** They are kept as written
  > because the corrections are the finding.
  >
  > *Not in `tests/gates/`*: `material` is `pub(crate)`, so a `ToggleChip` cannot be constructed
  > from an integration test at all — the gate lives in-crate as
  > `src/ui/material/content_placement.rs`, the precedent `form_field_anatomy` and `style_snapshot`
  > already set for exactly this reason.
  >
  > *Not the laid-out bounds*: a button lays its content out under `limits.height(Fixed(32))`,
  > which sets the minimum with the maximum, so the label node is stretched to the full 32dp and
  > its bounds *are* the pill's. The first version measured 0dp of band on both sides and passed a
  > deliberately top-aligned sabotage as centred. The gate rasterises instead — it renders the chip
  > over its own fill colour so only the label inks, and compares that ink against the same string
  > drawn at the top of a 20dp line box. Confirmed red first: ink rows 4–14 of 32, identical to the
  > reference, where centring is 6dp lower.

- [X] T087 Centre the label in `crates/micold-client/src/ui/material/toggle_chip.rs`, making T086 green. Preferred shape: wrap the button's content in a centring container (`.center_y(Length::Fill)`), as `icon_button.rs:155-159` already does to centre a small container inside a larger fixed box — it does not depend on the label's metrics, and it leaves the 32dp height and the 12dp ends exactly as §7.6 states them. The alternative is the label's own `.height(Length::Fill).align_y(Vertical::Center)`, which `Text` supports. Correct the comment above the padding while there: "the vertical padding is what makes the height" is true of a content-sized `Tag` and false of a chip that sets its height, and that sentence is the reasoning that produced the bug (FR-030, FR-030a)

  > Took the alternative rather than the preferred shape, and the corrected root cause is why: a
  > centring container would place a node that is already the pill's full height, changing nothing.
  > The glyphs are what sit high, so the alignment belongs on the text — `Text` now states
  > `height(Fill)` and `align_y(Center)`, the first saying the node is the pill's height on purpose
  > and the second saying where the line box sits inside it. The misleading padding comment is
  > replaced.

- [X] T088 [P] Sweep the other fixed-height components for the same shape — a container given `Length::Fixed(...)` whose content is smaller and whose alignment is unstated. `toolbar.rs:78` (64dp app bar) and `icon_button.rs:156-157` (48dp target) are the two other `Length::Fixed(anatomy::...)` sites; the icon button already centres, the app bar must be checked. Fix or record each, so BUG-001's class is closed rather than its one instance (FR-030a, SC-008a) *(reopened 2026-08-07 — BUG-002: the sweep cleared `icon_button.rs` on a true observation and the wrong question; closed the same day by T091)*

  > **The sweep found a second instance, and it is the reason this task was worth running.** The app
  > bar's container states no `align_y`; the row's own `align_y(Center)` centres the row's children
  > against each other, not the row against a height imposed from outside. A bar holding only a
  > title sat 18dp high of centre. `Toolbar` now states `align_y(Center)` on its container.
  >
  > It was invisible in the application because the shipped bar holds a full-height action, which
  > stretches the row to the whole 64dp — a row that already fills its container cannot be
  > misaligned by one. Correct *by accident*, and the layout fixture is unchanged by the fix for the
  > same reason. Measured rather than read: the gate covers the bar alongside the chip.
  >
  > Everything else came back clean, and for a stateable reason: the icon button centres explicitly
  > (`center_y(Fill)`); the menu row, type-ahead row, snackbar and tree rows all hand their fixed
  > height a `row!` that states `align_y(Center)`; the filled field is a custom widget that places
  > its children by arithmetic; `Tag` and the menu items are content-sized and have no slack. The
  > class is "content whose placement is unstated", not "components with a fixed height".

- [X] T089 Confirm it in the running application and the gallery: `mise run run` for the sidebar's tag filters and the worktree form's toggles, and the showcase's `material/toggle_chip.rs` entry for the inactive/active/accented row that surfaced this. The assertion a test cannot make is that the pill now reads as a chip rather than as a word pushed to its ceiling (FR-030a, quickstart §B4)

  > Confirmed 2026-08-07 in the showcase (`mise run showcase`, the `material/toggle_chip.rs` entry):
  > all three posed chips — inactive, active, accented-active — carry their label on the pill's
  > centre line, against the reported screenshot where every one of them rode high. The `Toolbar`
  > entry's "title only" bar was confirmed in the same pass, which is the app bar's fix seen rather
  > than measured.
  >
  > Recorded as the showcase specifically rather than as a blanket pass: the showcase renders the
  > same `ToggleChip` the sidebar's tag filters and the worktree form's toggles do, posed in all
  > three states at once, which the running application cannot show on one screen.

**Bugfix**: 2026-08-07 — BUG-001 Updated from bugfix patch: reopened T060, added T086–T089. All
five closed the same day; T060's reopen is kept visible rather than erased.

---

## Phase 12: BUG-002 — an icon button's touch target was `Fill × Fill`

Reported 2026-08-07 against the *previous* bugfix phase's own sweep. Every non-`compact()`
`IconButton` claimed a share of whatever free space its row had instead of §7.3's fixed 48 × 48, so
the app bar's ⋮ and the terminal bar's mode toggle both floated well inside the trailing edges they
are supposed to sit on. See `bugs/BUG-002.md`.

**Two false completions.** T057 applied §7.3's figure and then discarded it on the next line.
T088 — BUG-001's own sweep — inspected this exact site, wrote "the icon button already centres",
and cleared it: true, and the wrong question. The centring call is what threw the size away. Both
are reopened above and closed by T091.

- [X] T090 Failing test first: assert a component lays out at the size its anatomy entry states, whatever room it is offered, and confirm it fails against today's `IconButton`. Lay each component out under **two differently-sized limits** — one measurement cannot separate `Fixed(48)` from `Fill`, since offering exactly 48dp makes them agree. Declare each axis, so an intended `Fill` (the app bar's width) is distinguishable from an accidental one. In-crate, as `content_placement` records: `material` is `pub(crate)` and an `IconButton` cannot be constructed from `tests/` at all (FR-027, SC-008b, Principle I)

  > `src/ui/material/anatomy_size.rs`, six tests. Confirmed red first against the unfixed
  > component: *"an icon button's width measured 1200dp in a 1200dp box and 400dp in a 400dp one,
  > but its anatomy entry states 48dp — it is taking whatever room it is offered"*, and the same for
  > its height. Covers the icon button (full, compact and disabled), the chip and the app bar;
  > `the_gate_can_fail` rebuilds the `.width(Fixed(48)).center_x(Fill)` chain directly, so the check
  > stays demonstrably able to fail after the component is fixed.

- [X] T091 Restore the target in `crates/micold-client/src/ui/material/icon_button.rs`, making T090 green, and close T057 and T088 with it. `Container::center_x(w)` is `self.width(w).align_x(Center)` — it sets the length as well as aligning, so `.width(Fixed(48)).center_x(Fill)` silently discards the 48. Align without resizing (FR-027, SC-008b)

  > `align_x(Center)`/`align_y(Center)` in place of `center_x(Fill)`/`center_y(Fill)`. Two lines,
  > and the comment above them now says why the shorter builder is the wrong one here — that
  > aliasing is the entire defect and is invisible at the call site.

- [X] T092 [P] Sweep every other `center_x(`/`center_y(`/`center(` whose container also states a width or a height — the same overwrite is available at all of them (FR-027, SC-008b)

  > Clean. Four other sites, none of them the same shape: `cdk/overlay.rs:146-148` and
  > `terminal.rs:336-351` pass `Fill` to a container that is *meant* to fill, with no fixed size to
  > discard; `activity_badge.rs:110` passes `Fixed(badge.size)` through `center_x` itself and states
  > no separate width. The defect needs both halves — a stated size *and* a later call that takes an
  > argument — and only the icon button had both.

- [X] T093 Regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt`; the diff is the proof. The fixture **held the defect as its expected value** and was green on it from the day it landed, which is what a snapshot does to a defect older than itself (SC-008b, feature 019 FR-003)

  > The app bar's overflow trigger moves from `764.1, 0.0, 499.9 × 64.0` to
  > `1216.0, 8.0, 48.0 × 48.0` — right edge at 1264, which is 1280 less §7.1's 16dp trailing
  > padding, so it is now flush against the edge it belongs on. The project switcher follows it out
  > to 1130.3 from the middle of the bar.

- [X] T094 Give the terminal's bottom status bar geometry coverage — it had **none**, which is why only the app bar's half of this defect was recorded anywhere. One entry in `tests/support/covered_states.rs` and nothing else (feature 019 FR-016), in `Regular` mode with no shell started so the bar carries the most it can: the restart action, the new-instance "+", and the mode toggle (SC-008b, feature 019 FR-016)

  > `session-terminal-bottom-bar`, anchored on the bar and on the mode toggle so a failure names
  > them. Records the fix: the two icon controls sit at `1168.0, 48 × 48` and `1224.0, 48 × 48`,
  > the second flush against the bar's trailing padding, with the row's single `Space::Fill` taking
  > all 737.4dp of the slack they had been splitting. The bar is a 64dp strip at the window's
  > bottom edge, not the half-height block the `Fill` height had made it.

- [X] T095 Confirm it in the running application, per quickstart §B4 (FR-027, SC-008b)

  > Confirmed 2026-08-07, and it took a new tool. Nothing on a stock GNOME/Wayland session can take
  > a screenshot from a shell — `grim` needs a protocol Mutter does not implement, `import` is X11,
  > `org.gnome.Shell.Screenshot` answers `AccessDenied`, and the portal opens a consent dialog. So
  > `scripts/screenshot-session.py` pulls a frame off the monitor's PipeWire node via
  > `org.gnome.Mutter.ScreenCast`, the API an RDP server uses underneath. `mise run screenshot`,
  > documented in `docs/development/screenshots.md`.
  >
  > The frame holds both builds at once — installed 0.6.0 behind, worktree build in front. The ⋮ sits
  > 417px inside the trailing edge before and 42px after, with the project chip immediately left of
  > it rather than adrift near the middle. Figures in `bugs/BUG-002.md`.
  >
  > **The terminal's bottom bar was not looked at**: reaching it needs a session opened by hand and
  > nothing here can drive input. It is pinned by T094's covered state and by T090's size gate, and
  > that is the whole of its evidence.

**Bugfix**: 2026-08-07 — BUG-002 Updated from bugfix patch: reopened T057 and T088, added
T090–T095, added SC-008b. All closed the same day; both reopens are kept visible rather than erased.

---

## Phase 13: Convergence

- [X] T096 Extend `crates/micold-client/src/ui/material/anatomy_size.rs` to every component whose anatomy entry states a size, not the three it covers today (app bar 64, chip 32, icon-button target 48×48). The remaining six: §7.2's list and tree rows at `density::LIST_ROW_BASE` standard and dense, §7.3's 40dp button height at `density::BUTTON_BASE`, §7.5's 48dp menu item at `density::MENU_ITEM_BASE`, §7.7's 56dp field and select at `density::TEXT_FIELD_BASE`, and §7.8's snackbar minimum height and maximum width. Declare each axis `Fixed`/`Fill`/`Content` as the existing entries do, so a component that is *meant* to be content-sized stays distinguishable from one that lost its figure. Expect T097 and T098 to fail against it, which is the point — this task is the check, not the fixes per SC-008b (partial)

  > Nine components now, up from three: the icon-button target (48×48), the chip (32), the app bar
  > (64 tall, `Fill` wide), the button (40), the menu item (48 tall, `Fill` wide), the text field
  > (56), the snackbar (`AtLeast` 48 / `AtMost` 600), and the tree row — declared `Content`, which
  > is the recorded decision rather than an oversight (see below). `Extent` gained `AtLeast`/`AtMost`
  > for §7.8's minimum and maximum, and `laid_out` gained a path into the tree, because §7.2's and
  > §7.5's figures belong to a *row* and reading the root would measure the list instead.
  >
  > Confirmed red exactly where converge predicted: "a filled button's height measured 30dp … but
  > its anatomy entry states 40dp", "a menu item's height measured 36dp … states 48dp". Everything
  > else came back green, including the text field's 56 and the snackbar's bounds — so this is two
  > components adrift, not general drift.
  >
  > Two entries needed care. The snackbar takes **two** specimens: one message cannot exercise a
  > height floor and a width cap at once. `menu::item_column` became `pub(super)` — both public
  > entry points yield an overlay `Surface` rather than an `Element`, so measuring an item through
  > them would mean subtracting panel padding by hand, which asserts arithmetic instead of §7.5.

- [X] T097 Apply §7.3's 40dp height to the filled, outlined and text variants in `crates/micold-client/src/ui/material/button.rs`, or record the deviation in `spec.md` as an accepted fidelity gap alongside FR-042/043/043a/044/045/046. The component sets no height at all: `From<Button>` passes through only the caller's optional padding, so the three variants are content-sized and `density::BUTTON_BASE` (40.0) is referenced nowhere in the client, while `button.rs:123` states the opposite — "Feature 018 assigns each variant a height from the density scale". The comment and the code cannot both stand. It must not stay in the current state, where the requirement is neither met nor waived per FR-027, §7.3 (partial)

  > Applied. `Button` set no height at all: a filled button laid out at 30dp — its label plus the
  > rendering stack's default padding — while `density::BUTTON_BASE` (40.0) was referenced by no
  > call site and `button.rs:123` claimed the opposite. The comment was right about the intent and
  > the code had never carried it out.
  >
  > **The height obliged an alignment**, and that is FR-030a rather than a preference: `button` lays
  > its content out under `limits.height(Fixed(40))`, which sets the minimum with the maximum, so
  > the content node stretches to 40dp and draws at its top edge unless something says otherwise —
  > BUG-001 exactly, one component over. A centring wrapper rather than the content's own `align_y`,
  > because `with_content` takes an arbitrary `Element` this type cannot reach into. §7.3 gained the
  > label-alignment row, and `content_placement.rs` gained the button, since it now belongs to that
  > module's class by construction.

- [X] T098 Apply §7.5's 48dp item height in `crates/micold-client/src/ui/material/menu.rs`, or record the deviation as an accepted fidelity gap. Items are sized by `spacing::SM` padding plus one `label_large` line (`menu_panel_size`, `menu.rs:78-81`), which lands near 36dp; `density::MENU_ITEM_BASE` exists for this and is applied only by `typeahead.rs:245`. Note that `menu_panel_size`'s anchor-clamping estimate is derived from the same arithmetic, so it moves with the height rather than after it per FR-029, §7.5 (partial)

  > Applied, at `density::MENU_ITEM_BASE`. Items were `spacing::SM` padding around one `label_large`
  > line — 36dp against §7.5's 48 — and the token existed for this, used only by `typeahead.rs`.
  >
  > No centring wrapper needed here, and the difference from T097 is worth stating: the item's
  > content is a `row!` that already declares `align_y(Center)`, and `button` stretches that row to
  > the full 48dp, so the row centres its own children inside a height it actually has. BUG-002's
  > app bar was the other case — there the container *imposing* the height was the one that had to
  > speak.
  >
  > `menu_panel_size` now reads the same token instead of rebuilding the height from padding plus a
  > line box. While §7.5 went unapplied that arithmetic reproduced the real 36dp by restating the
  > defect, so it was right by tracking it — and would have gone wrong the moment the height landed,
  > silently breaking the anchor clamping that keeps a cursor-anchored menu on screen.

- [X] T099 Justify or remove the desktop-screenshot tooling added under BUG-002 — `scripts/screenshot-session.py`, `docs/development/screenshots.md`, and `mise.toml`'s `screenshot` task. No FR or SC asks for it; it was written because quickstart Part B's visual items could not otherwise be captured on a GNOME/Wayland session, and is now referenced from Part B's preamble and T095. Either record it in the spec as support for the manual walkthrough, or drop it and return Part B to unaided inspection. It is not a gate and nothing in CI calls it (unrequested)

  > Recorded, not removed. `spec.md`'s verification-split paragraph now names `mise run screenshot`
  > as support for the recorded manual walkthrough, states that it is **not** a gate and moves no
  > criterion out of the automated list, and says why it exists: BUG-002 was reported from two
  > screenshots and nearly closed without one, because no route on a stock GNOME/Wayland session can
  > take a screenshot from a shell. quickstart Part B's preamble already pointed at it; the spec is
  > what makes it requested rather than merely present.


**Convergence**: 2026-08-07 — Phase 13 closed. SC-008b's coverage went from three sized components
to nine, which is what surfaced T097 and T098: two contract figures that were correct in the token
module and reached no component. Both now apply. One further finding is **not** covered by these
four tasks and is left for the next converge: §7.2's row height is a deliberate, justified deviation
(`tree_view.rs:227-237`, and the T042 note above), but unlike its five siblings it has no FR entry
in `spec.md`'s accepted-gaps list. It is the only accepted gap recorded solely in a code comment.

---

## Phase 14: BUG-003 — two panels over the app bar, and one item row built twice

Reported 2026-08-07 from a screenshot of the running application: the overflow menu opens 13dp
*inside* the app bar, clipping the ⋮ it was opened from. The project switcher does the same, from
its own copy of the same constant, and its rows are still 36dp where the menu's are 48. See
`bugs/BUG-003.md`.

**One false completion.** T059 closed §7.5 on its surface and shape alone; four of its spatial
figures were never applied and the item height arrived five phases later under T098. Reopened above,
closed by T105.

**Guards first, in this order** — the gate in T101 is vacuous until T100 gives it something to
read, and T103's figures cannot be asserted from `tests/` at all.

- [X] T100 Cover the two panels. Two entries in `tests/support/covered_states.rs` and nothing else (feature 019 FR-016): `toolbar-overflow-menu-open` and `project-switcher-open`, each anchored on the app bar, on the trigger the panel hangs from, and on the panel itself, so a failure names them. Neither panel has ever appeared in a fixture line — `worktree-menu-open` is the sidebar's context menu at a hard-coded point and `add-worktree-dialog-type-menu-open` is the select dropdown, so nothing in the fixture has ever exercised `TOP_OFFSET` (feature 019 FR-016, SC-008c)

  > `toolbar-overflow-menu-open` and `project-switcher-open`, each anchored on the app bar, on its
  > own trigger and on its panel. One correction to the bug report while writing them: the overflow
  > panel was **not** absent from the fixture. A closed `MenuOverlay` still yields a surface, so it
  > has been recorded at `1032, 52, 240 × 264` in nearly every state since the fixture landed. What
  > was missing is a state in which either panel is *open* — and any assertion at all about where a
  > panel sits, which is T101.

- [X] T101 Failing test first: a gate under `tests/gates/` asserting that no floating panel is laid out over the app bar and none extends past the window, over every covered state. Compiled into the `layout_snapshot` binary as `containment` is, so it shares that binary's record cache rather than re-resolving every state in a second process. It must **assert**, not compare against the fixture: a snapshot adopts a defect older than itself as its expected value, which is what T093 had to correct after BUG-002. Confirm it fails against both panels at 13dp before anything is fixed (SC-008c, FR-029a)

  > `tests/gates/panel_placement.rs`, compiled into the `layout_snapshot` binary. Confirmed red
  > first, and far redder than expected: **17 panels across 16 states**, because the closed overflow
  > menu is laid out in all of them. *"the panel at 0/1/0 starts at y=52.0, which is 13.0px inside
  > the app bar (the bar and its divider end at 65.0)"*. An anchored panel is identified
  > structurally — a layer's own child, smaller than the window on both axes — so a panel added
  > later is covered without anyone remembering to add it. `the_gate_can_fail` rebuilds a panel at
  > the old offset from a synthetic record, so the check stays demonstrably able to fail now that
  > both panels are fixed.

- [X] T102 Give §7.1's bottom edge a constant to be read from. `anatomy::app_bar` gains the divider and the bottom edge the contract now states, `tokens_anatomy.rs` asserts both, and `material/toolbar.rs` draws its separator from the constant instead of the literal `1.0` it hardcodes today. This is the number FR-029a requires the panels to derive from, so it has to exist before either of them can stop stating its own (FR-029a, SC-008, §7.1)

  > `anatomy::app_bar::DIVIDER` (1.0) and `BOTTOM_EDGE` (65.0), asserted in `tokens_anatomy.rs`,
  > and `toolbar.rs` now draws its separator from the constant instead of a literal `1.0`. §7.1
  > gained the two rows they come from, including *why* the bottom edge is a row of its own: what a
  > surface hanging below the bar must clear is neither figure alone, nothing said so, and two
  > components each answered it by eye with the same wrong number.

- [X] T103 Failing tests first for §7.5's unapplied figures, in-crate beside `anatomy_size.rs` (`material` is `pub(crate)`, so `tests/` cannot construct any of this): a menu item's leading icon is `anatomy::menu::ITEM_ICON`, its content is inset by `anatomy::menu::ITEM_PADDING` on both sides, and a panel puts `anatomy::menu::VERTICAL_PADDING` above the first item and below the last with the items abutting between. Confirm all four red — 14dp icons, 8dp insets, 4dp padding, 4dp gaps (FR-029, SC-008b, §7.5)

  > `src/ui/material/menu_anatomy.rs`, six tests, all four figures confirmed red first: a 14dp
  > leading icon against §7.5's 24, an 8dp content inset against 12, 4dp of panel padding against 8,
  > and 4dp between items where the contract has nothing. The sixth pins `menu_panel_size` to the
  > panel it estimates, and was green throughout — the one thing §7.5's arithmetic had right.
  >
  > It also found a gap in the in-crate harness, not in the application: `test_support::renderer()`
  > loaded the two Roboto faces and **not the icon face**, so a 14dp glyph resolved through the
  > host's fallback and measured 8.4dp. `tests/support/layout.rs` had the identical defect and fixed
  > it in feature 019; this is the same fix on the in-crate side, and without it the icon assertion
  > would have been reading whatever font this machine happens to offer.

- [X] T104 Unify the item row (FR-029b). `MenuItem` gains what the switcher's rows need and nothing more — a leading marker, trailing text, a trailing badge icon, an optional press (an unavailable project is shown and not selectable, FR-008 of feature 008) and an optional right-press — and `project_switcher.rs` builds `MenuItem`s instead of assembling its own `button`/`Ripple`/`text_button` stack. Its trigger stays its own: `MenuTrigger` and `ProjectSwitcherTrigger` genuinely differ, the rows do not. Behaviour is unchanged — every message, the unavailable badge, the running count, the "Add project…" affordance and the right-press all survive, which is what `tests/project_switcher.rs` and `tests/switcher_forget_menu.rs` are for (FR-029b, Principle VIII)

  > `MenuItem` gains `icon_tint`, `trailing_text`, `trailing_icon`, `on_context`, and an optional
  > `message` — five fields, which is what a switcher row is. `project_switcher::row_column` is now
  > a `map` into `MenuItem`s followed by `menu::item_column`; its hand-built `button` /
  > `text_button` / `Ripple` / `mouse_area` stack is gone, and so is the second `TOP_OFFSET`.
  > Appearance is preserved deliberately: the active marker keeps its `Badge` tint and the
  > unavailable badge its error tint, through `icon_tint` and `trailing_icon` rather than by the
  > row knowing what a project is. `MenuItem::labeled` covers the terminal's icon-less copy/paste
  > items, which were the only struct literals in the tree.

- [X] T105 Apply the rest of §7.5 to the now-single row, closing T059: `VERTICAL_PADDING` on the panel, `ITEM_PADDING` horizontally in an item, `ITEM_ICON` for the leading glyph, and the inter-item gap removed. `menu_panel_size` must move with all of them — it is correct today and each of these silently invalidates the anchor clamping that keeps a cursor-anchored menu on screen (FR-029, §7.5)

  > §7.5 applied in full, closing T059: `VERTICAL_PADDING` above the first item and below the last,
  > `ITEM_PADDING` at both ends of an item, `ITEM_ICON` for the leading glyph, and the 4dp gap
  > removed. `menu_panel` now takes its padding from the caller — a menu pads vertically and not at
  > all at the sides, so an item's state layer runs edge to edge, while the sidebar's filter
  > accordion holds arbitrary content and still pads all four. `menu_panel_size` follows the same
  > tokens and stays exact.
  >
  > One consequence worth recording: the item's label now fills, which is what makes the trailing
  > 12dp measurable — and it made both `SCHEME_DEPENDENT` exemptions in `layout_snapshot.rs` go
  > **stale**, because the theme-mode row's width no longer depends on the string inside it. The
  > list is now empty. The staleness assertion is what turned that into a required edit rather than
  > an exemption nobody re-read.

- [X] T106 Derive both offsets and delete both copies of `TOP_OFFSET`, making T101 green. The menu and the switcher read §7.1's bottom edge from T102's constant; neither states a number of its own (FR-029a, SC-008c)

  > Both panels read `anatomy::app_bar::BOTTOM_EDGE`; neither states a number. The gate went green
  > on all 17. Measured, from the regenerated fixture: the overflow panel moves from
  > `1032, 52, 240 × 264` to `1032, 65, 240 × 256` — its top edge is now exactly the bar's bottom
  > edge — and the switcher from `1012, 52, 260 × 84` to `1012, 65, 260 × 112`.

- [X] T107 [P] Sweep for the same shape elsewhere: a constant that restates another component's dimension by eye. `menu.rs`'s `CONTEXT_MENU_WIDTH`/`PANEL_WIDTH`, the two context menus anchored at a literal `Point::new(24.0, 96.0)` with no clamping at all (`ui/mod.rs`), and `typeahead.rs`'s panel are the candidates. Fix or record each, so FR-029a's class is closed rather than its one instance (FR-029a)

  > One finding, fixed. The worktree and session context menus were anchored at a literal
  > `Point::new(24.0, 96.0)` **twice, with no clamping at all**, while the project menu beside them
  > clamps through `clamp_menu_anchor`. Both now clamp against `menu_panel_size(items.len())`, and
  > the point is stated once as `SIDEBAR_MENU_ANCHOR`. It had not bitten yet and was about to: a
  > four-item worktree menu grew from 152dp to 200dp when items became 48dp.
  >
  > The other candidates are clean. `PANEL_WIDTH` and `CONTEXT_MENU_WIDTH` are the panels' own
  > anatomy, derived from the labels they must hold and documented as such — not a restatement of
  > another component's figure, which is the shape FR-029a is about.

- [X] T108 [P] Decide the type-ahead row. It is the third hand-built copy of the item row and FR-029b does not name it: its rows carry a keyboard highlight and a scroll cap that a menu item has no notion of. Either fold it into the unified row with those as item state, or record in `spec.md` why it stays separate. It must not stay in the current state, where the requirement is neither met nor waived (FR-029b)

  > Recorded as deliberately separate, in FR-029b itself rather than in a comment. The type-ahead's
  > row is not this row: its label is an `EmphasisedLabel` of match spans rather than a string, it
  > carries a keyboard-highlight state on top of selection and disablement (`style::menu_row` takes
  > all three), its leading slot reserves space when nothing is selected so every label starts at
  > the same x, and it ripples at `shape::SMALL` — and not at all when the row is unpressable. It
  > shares §7.5's height, padding and surface through the same tokens. What it does not share is the
  > row, and folding four states into `MenuItem` to say so would be the duplication argument run
  > backwards.

- [X] T109 Regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt`; the diff is the proof, and it is the first time either panel has been in it (SC-008c, feature 019 FR-003)

  > Regenerated. The diff is the proof, and it is the first time either panel has appeared in the
  > fixture as an *open* one. Both panels move down 13dp onto the bar's bottom edge; menu items go
  > from 36dp to 48dp with 12dp ends and 24dp glyphs; the switcher's rows change from 36dp to the
  > same 48dp, which is the half of BUG-003 that no gate could have reported before T103.

- [X] T110 Confirm in the running application per quickstart §B4, with the overflow menu and the switcher open in turn: the trigger stays fully visible, both panels begin below the bar's divider, and their rows are the same height (SC-008c, FR-029b)

  > **Closed on measurement, 2026-08-08 — not by driving the application.** The note below stands
  > as written and is the reason this stayed open; what changed is that it no longer has to be a
  > person. Everything the item asks to be judged by eye is now *asserted*, over every covered
  > state, by machinery BUG-003 built as part of the same fix and which did not exist when this
  > task was written:
  >
  > - `tests/gates/panel_placement.rs` — no floating panel is laid out over the app bar, and none
  >   extends past the window. Asserted rather than recorded, deliberately: a fixture regenerates a
  >   defect older than itself into its own expected value, which is exactly what would have
  >   happened here. It carries `the_gate_can_fail`, so it is demonstrably able to.
  > - The `toolbar-overflow-menu-open` and `project-switcher-open` covered states, which put both
  >   panels in the fixture for the first time. At 1280 x 800 both begin at **y = 65.0** — the bar's
  >   64 plus its 1dp divider, `anatomy::app_bar::BOTTOM_EDGE` — with **48dp** rows in both panels,
  >   abutting, and 8dp of panel padding. BUG-003 was 52.0, 36dp switcher rows against the menu's
  >   48, and a 4dp gap.
  > - `material/menu_anatomy.rs` for the item's 24dp icon and its `on_surface_variant` tone.
  >
  > Recorded in `quickstart.md` §B4 with the figures. What genuinely still wants an eye — that each
  > panel floats with its own shadow — is §B1's item, and is not this one.
  >
  > **Not done by driving the application, and deliberately abandoned rather than pushed through.** Unlike BUG-002's, this
  > confirmation needs the application *driven*: the panel does not exist until something presses
  > the ⋮. The only route available from a shell here is `xdotool` against an XWayland window plus
  > `mise run screenshot`, and both act on the owner's live desktop — the attempt moved their
  > pointer and raised a GNOME "Remote Desktop / Allow Remote Interaction" consent dialog over their
  > work. That is not a cost this task is worth, so it stops here for a person to do in one click.
  >
  > What stands in its place, and what it does not cover: the geometry is pinned by T100's two
  > covered states, asserted by T101's gate on every state, and asserted figure by figure by T103's
  > six §7.5 checks. Between them they hold every claim in this task — panel below the divider,
  > trigger unobstructed, both panels' rows equal — as *layout*. What none of them reads is the
  > drawn pixels, which is `content_placement`'s job and is not extended here.

- [X] T111 Apply §7.5's *other* half of the item-icon row — the `on_surface_variant` tint — or record the deviation. The 24dp landed under T105 and the tone did not, which is this bug's own shape repeated inside its own fix: a figure in the table that reaches no component (FR-029, §7.5)

  > `IconSurface::MenuItem`, tinted `on_surface_variant`, and `menu::leading_tint` is what the row
  > calls — a function rather than an expression in the loop, so it can be asserted without
  > rasterising a glyph to read one colour. Confirmed red first: `on_surface` (28,27,30) against
  > §7.5's (73,69,78).
  >
  > Its own `IconSurface` context rather than a reuse of `Badge`, which already resolves to the same
  > role. The tone is the same today and the reasons are not, so a change to what a badge looks like
  > must not silently restyle every menu in the application. `icon_roles.rs` gains the mapping, the
  > contrast pair — a menu glyph sits on `surface_container`, not `surface` — and the count.
  >
  > An item that states a tint still keeps it, asserted separately: the switcher's active marker is
  > a `Badge` by intent (FR-006 of feature 008) and this is a default, not an override.

**Bugfix**: 2026-08-07 — BUG-003 Updated from bugfix patch: reopened T059, added T100–T111, added
FR-029a, FR-029b, US4 acceptance scenario 11 and SC-008c.

---

## Phase 15: The binding gate

- [X] T112 Gate that every figure in `anatomy::ALL` and every `density::*_BASE` is referenced by the rendering layer, or recorded with a reason — `crates/micold-client/tests/anatomy_call_sites.rs`. This is the check the last three anatomy defects were each missing, and it is the cheapest of them: it reads source text, with no renderer, no layout pass and no fixture. `tokens_anatomy.rs` proves a number was *transcribed*; `anatomy_size.rs` measures the nine figures that are a laid-out box; neither can see a padding, gap, icon size or outline width that reaches nothing. `type_role_call_sites.rs` is the same rule for typography and the precedent for this one (FR-025 – FR-032, §7)

  > Confirmed red before the baseline was written: 24 of the 46 figures §7 then had reached no
  > component. Twenty-one are recorded now, out of 48 — BUG-003 added §7.1's divider and bottom
  > edge, both bound on arrival, and applied three of the twenty-four.
  > Then confirmed red a second way, against a *regression* rather than an empty list — replacing
  > `anatomy::chip::HEIGHT` with the literal `32.0` in `toggle_chip.rs` fails this gate and no
  > other, because the chip still lays out at 32dp. That is the case `anatomy_size.rs` is blind to
  > by construction: the right number under a name that will not follow when §7 is re-valued, which
  > is `type_scale::BODY` all over again.
  >
  > The exclusion of the in-crate gates is read off their own `#[cfg(test)] mod` declarations in
  > `material/mod.rs`, so a gate added later is excluded the day it lands. That parse was itself
  > wrong first time — it built `material/anatomy_size.rs` while a scanned source is keyed
  > `ui/material/anatomy_size.rs`, so every gate was read as production code and a figure named only
  > by `anatomy_size.rs` counted as bound. The gate was green and guarding nothing, which is this
  > file's own subject matter one level up. `the_in_crate_gates_are_found_and_actually_excluded` now
  > checks the names against the source list rather than against the parse.

- [X] T113 Apply, or waive in `spec.md`, the **ten live deviations** the gate above recorded. Each is a component that exists, a number §7 states, and a component using a different one — or the right one under a name that will not follow when §7 changes. Listed in `anatomy_call_sites.rs`'s `RECORDED` under `gap::UNAPPLIED`, which pins the count so the list can only shrink. Two shapes, and the second is the more dangerous:

  **A different number.** §7.3's button padding — 24 filled, 24 outlined, 12 text — against iced's `DEFAULT_PADDING` of 10, which `Button` never overrides; §7.3's 8dp icon-button padding against `icon_button.rs`'s `spacing::XS` (4); and §7.4's `BODY_TO_ACTIONS`, which is 24 in the contract and 16 in every dialog, because the action row is pushed into the body's column and takes its `spacing::MD`. That last one is the only deviation here that §7 explains the *reason* for — the gap is wider than `TITLE_TO_BODY` on purpose, "so the actions read as a separate region rather than as more body" — so it is the one where the number carries a visible intent that 16 defeats.

  **The right number under another name.** §7.4's dialog padding (`spacing::LG`, 24), title-to-body gap (`spacing::MD`, 16) and action gap (`spacing::SM`, 8); §7.3's and §7.6's 1dp outlines, written as the literal `1.0` in `style.rs` and `toggle_chip.rs`. Nothing looks wrong today and nothing will, until §7 is re-valued and these do not move. This is exactly what `type_role_call_sites.rs` was written twice to stop.

  Applying them changes what the application looks like — every button and every dialog — so it wants the layout-snapshot regeneration and the §B4 manual pass, not a drive-by edit (FR-027 – FR-030, §7.3 – §7.6, SC-008b)

  > Twelve when the gate landed, ten after the rebase onto BUG-003. §7.5's `ITEM_PADDING` and `VERTICAL_PADDING` were on this list and are now applied by T105, and `ITEM_ICON` — recorded as carried by the type scale — is applied by T105 too. `a_recorded_gap_that_became_bound_is_stale` is what reported all three, which is the half of this gate that keeps the list honest in the direction that matters: an entry describing a state that has stopped being true is a waiver for a regression nobody would notice.
  >
  > **It was thirteen, and the three extra are the finding.** `button::ICON_BUTTON_GLYPH`,
  > `button::LEADING_ICON` and `text_field::TRAILING_ICON` were recorded as *carried elsewhere*, on
  > the reasoning that a glyph is sized by the type scale and a dp figure for one is a second
  > spelling. That reasoning was wrong, and T103 had already disproved it one component over: `icon`
  > takes a number, and the role-sized glyph is the defect, not the design. Measured before touching
  > anything, all three drew **14dp** — the body text's size — against §7.3's 24 and 18 and §7.7's
  > 24. Three requirements were sitting waived on a false premise, which is a worse state than the
  > ten that were honestly recorded, and is the failure mode a categorised allowlist exists to make
  > visible rather than to hide.
  >
  > Applied, red first for every figure that had a component to be red against:
  > a filled button inset its label 10dp against §7.3's 24, an icon button 4dp against 8, both
  > glyph families 14dp against 24 and 18, and a dialog's action row sat 16dp below its body against
  > §7.4's 24. The five whose numbers already coincided — the two 1dp outlines and §7.4's padding,
  > title-to-body and action gaps — **could not be shown red**, and that is the point of them: only
  > the binding was missing, so only `anatomy_call_sites.rs` could see it.
  >
  > Two components gained a slot rather than a constant, because the figure belongs to the component
  > and not to the call site. `Button::leading(icon, tint)` owns §7.3's 18dp and replaces the two
  > hand-built `row![Glyph, Text]` buttons in `project_selector.rs`; `material/dialog.rs` owns §7.4's
  > four figures and replaces the `column![..].spacing(spacing::MD)` / `fields.push(actions)` shape
  > that all nine dialogs repeated. `Surface` defaults the dialog padding by *kind*, which is the
  > argument it already makes for §7.4's width bounds: "the seven that build dialogs were each free
  > to forget it."
  >
  > `text_field::TRAILING_ICON` ends as the one figure fixed without being bound. The slot takes an
  > `IconButton`, so it now draws §7.3's 24dp through `button::ICON_BUTTON_GLYPH` — the measurement
  > is right and a second reference would be the two-names-one-measurement `anatomy.rs`'s own header
  > warns about. Recorded as `CARRIED_ELSEWHERE` with a reason that is true for the first time.
  >
  > `dialog::body` adds a nesting level, so six anchors and one press path in `covered_states.rs`
  > needed re-pointing, and the fixture moves accordingly: the app bar's ⋮ goes from a 22 × 26.2 pill
  > around a 14dp glyph to 40 × 47.2 around a 24dp one. **`RECORDED` now holds no live deviation at
  > all** — every figure §7 states of a component this application has built is applied by it, and
  > the count test says so by name.

- [X] ~~T114 Give §7.2's row height an FR entry in `spec.md`'s accepted-gaps list~~ — **superseded by BUG-005 (Phase 16).** There is no gap to record: the height was never a decision, it was a floor hung on a spacer that is void at depth 0, so it reached nested rows only. §7.2 is applied now, `density::LIST_ROW_BASE` is bound, and its `gap::WAIVED` entry in `anatomy_call_sites.rs` is deleted rather than reworded. Closed by T119 (FR-026, FR-026d, §7.2)

- [X] T115 Hold `anatomy::ALL` against the module's own source, and add the two figures it was already missing. T112's universe is `ALL` plus the density bases, so a constant absent from `ALL` is exempt from that gate without anyone exempting it — the same defect T112 exists to catch, one level further out. `tokens_anatomy.rs` guarded the list with a **count**, which is precisely the guard that cannot notice it (FR-025 – FR-032, SC-008, §7)

  > Found while building a second copy of T112's scan, not knowing it had landed here in parallel —
  > the duplicate was discarded and this is what was left of it that T112 does not already do.
  >
  > `app_bar::DIVIDER` and `app_bar::BOTTOM_EDGE` were added to the module by BUG-003 and not to
  > `ALL`. Both *are* bound — `toolbar.rs` draws the divider from one, both panels read the other —
  > so nothing was wrong in the application, which is what made it invisible: `ALL.len()` still
  > cleared its `>= 30` floor, every whole-set property still passed, and T112's own note records
  > them as "both bound on arrival" when its gate could not see either.
  >
  > `the_listed_table_holds_every_constant` parses the module for `pub mod x {` / `pub const NAME:`
  > and requires each to appear in `ALL`, confirmed red first on exactly those two. It asserts the
  > parse found at least thirty constants, because a parse that silently matches nothing would pass
  > while checking nothing — the failure mode this file exists to prevent, applied to itself.

---

## Phase 16: BUG-004 — the menu item's content sat against the top of its 48dp

Reported from the running application immediately after BUG-003 shipped: §7.5 says "48, with the
item's content centred in it", and every menu label sat 8.4dp high of centre. See `bugs/BUG-004.md`.

- [X] T116 Failing test first: a menu item's content is centred in its height, measured on the item, its glyph and its label. Confirmed red at content centred on 23.6dp inside a box centred on 32dp (FR-029, FR-030a, §7.5)

- [X] T117 Make the row's own box the 48dp — `height(Length::Fill)` — so `align_y(Center)` centres inside it rather than against the row's computed cross size. Correct the claim three commits carried forward: `button` does stretch the content node, and a `Row` still centres its children against each other inside that node's top band (FR-030a)

- [X] T118 [P] Sweep every other fixed-height component for the row-versus-container distinction. `typeahead.rs`'s result row is the same shape and had the same defect — fixed, and `row_element` made `pub(super)` so `menu_anatomy` measures it rather than leaving the fixed half unchecked. `button.rs`, `icon_button.rs` and `toolbar.rs` are clean: all three use a container, which aligns its child inside its own box (FR-030a)

- [X] T119 Regenerate the fixture. It had recorded `78.6` as the expected label position from the day the state was covered — the third time in four bugs that a snapshot has held a defect as its baseline (feature 019 FR-003)

- [X] T120 Extend `content_placement.rs` to the menu item. BUG-004 was catchable in geometry because the label's node is not stretched, but SC-008a exists for the case where it is, and the component this bug was in is still absent from the check built for its class (SC-008a)

  > `a_menu_items_label_is_centred_within_its_row`. Added *after* the fix, so it was confirmed the
  > only way that means anything: reverting `height(Fill)` in `menu.rs` makes it fail — ink 0.0dp
  > below the reference where centring is 14.0dp — and restoring it makes it pass. A check written
  > after a fix and never seen red is a check nobody has tested.
  >
  > Built without a leading icon on purpose: the glyph's line box is 31.2dp against the label's
  > 20dp, and `ink_rows` returns the union of everything inked, so an item holding both would
  > measure the glyph's band rather than the label's.
  >
  > It overlaps `menu_anatomy`'s geometric assertion deliberately. BUG-004 *was* catchable in the
  > layout tree because the label's node is not stretched — but the chip's was too, until a `Fixed`
  > height turned its label's bounds into the pill's and every band the geometry could measure into
  > 0dp. The rasterising check is the one that survives that change.

## Phase 17: BUG-005 — a tree row had no height, and the row that did was the wrong one

Reported 2026-08-07, against `4cd33b6`. `tree_view.rs` sizes every row by its content at either
density, so the two named densities are the same height as each other. The floor that used to exist
rode on each row's indent spacer and so applied at depth ≥ 1 only; removing it in `1cb9873` took 34%
off every sidebar session row and 51% off the gallery's `TreeView` sample, recorded by nothing. See
`bugs/BUG-005.md`.

**One false completion and one wrong resolution.** T056 states both heights and applies neither.
T076's measurement is sound and its conclusion is not: the ~30% it computed is arithmetic about a
floor on the name line, and a floor on the row costs 7.7%. Both are annotated above; T056 is
reopened and closed by T124.

**The spec moved, and by more than the fix needs.** BUG-005 recommended restoring §7.2's 36dp as a
row minimum. The owner's decision on the report was to go further: §7.2's base is Material **2**'s
48dp single-line list item, and this feature's subject is Material 3, whose list item is 56dp for
one line and 72dp for two. Material's density scale is generic — four steps, 4dp each — so the
dense column becomes 44 / 60, FR-026b and FR-026c are untouched, and FR-026a is amended to accept
the ~⅓ drop in visible worktrees that Material's own figures cost. The height figures below are the
new ones; the *shape* of the fix is unchanged by that decision.

- [X] T121 Failing test first: extend `src/ui/material/anatomy_size.rs`'s tree-row entry to every axis the component varies on — both densities, a specimen at depth 0 **and** one at depth ≥ 1, and a specimen taller than the floor. Confirm it fails against today's component, and that the depth-0 and depth-1 specimens fail *differently* against the pre-`1cb9873` construction, which is the asymmetry a single specimen hid. The entry moves from `Extent::Content` to `Extent::Fixed(density::height(…))` for the one-line specimens and `Extent::AtLeast` for the tall one (FR-026, FR-026d, SC-008b, SC-008d, Principle I)
- [X] T122 [P] Move §7.2's base onto Material 3's list item in `crates/micold-core/src/tokens/density.rs`: `LIST_ROW_BASE` 48 → 56, and a new `LIST_ROW_TWO_LINE_BASE` = 72. Update `crates/micold-core/tests/tokens_anatomy.rs` and `tokens_density.rs` to the new figures, keeping the assertions that every step lands on a whole dp and that dense < standard (FR-026b, FR-026c, SC-008)
- [X] T123 [P] Grep every reader of `LIST_ROW_BASE` before changing it, so a two-line row reads the two-line base rather than inheriting the one-line figure. `anatomy::list_row`'s padding and icon-gap constants are unaffected (FR-026d)
- [X] T124 Apply the height in `crates/micold-client/src/ui/material/tree_view.rs`, making T121 green and closing T056. Three properties, each of which was violated: the minimum goes on the **row**, not on its name line; it is carried by a spacer whose width stays `Shrink` so it can never go void (`snackbar.rs`'s idiom, and the trap this is the fourth instance of); and it does not consult `item.depth`. A row with tags takes the two-line figure, and a row taller than its figure still grows rather than clipping (FR-026, FR-026d, FR-026c)
- [X] T125 Add a covered state with an **expanded worktree** to `crates/micold-client/tests/support/covered_states.rs`, so a depth ≥ 1 tree row exists in the fixture at all. Every one of the sixteen existing states holds only depth-0 rows, which is why `layout_snapshot.txt` was byte-identical across the regression. Registering it touches one place (019 FR-016), and 019 FR-008d now requires it (019 FR-008, SC-004)
- [X] T126 Regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt` (`UPDATE_LAYOUT_SNAPSHOT=1`). The diff is the proof: every tree row moves to its density's figure, the new expanded-worktree scene records nested rows for the first time, and the standard-density known-projects list moves 48 → 56 (feature 019)
- [X] T129 Fix the text-overflow gate's attribution, and delete the two `KNOWN_OVERFLOWS` entries it was propping up. Surfaced by T124: moving the rows to §7.2's height separated them from the collapsed filter panel's overhang, the coincidental overlap stopped, and the staleness assertion fired on an exemption that had gone quiet (019 FR-018a, FR-015)

  > **The exemptions were never defects.** Measured, not argued: the text at issue is the sidebar's
  > `"Short"` **row label**, painted at (32, 146.8) and clipped by its own widget to 164 × 15.6 — it
  > fits with 135px to spare. It was being measured against a *filter-panel chip node* of 24.65dp
  > that the collapsed, zero-height `Expand` had left lying at the same coordinates. Two unrelated
  > subtrees at one point, and the deepest-containing-node rule handed the text to the wrong one.
  >
  > Proven by controlled experiment rather than inspection: reverting **only** `tree_view.rs` to its
  > pre-fix state brings the "overflow" back at exactly the exempted path (`"Short"` wants 28.86px
  > in 24.65px), and restoring the fix removes it — nothing else varied.
  >
  > `support::layout::text_overflows` now identifies the owner by the **clip the painter passed**,
  > falling back to the deepest containing node only when no node matches it, so FR-018's motivating
  > defect — a widget that clips to nothing of its own — is still caught. Verified both ways: with
  > the *old* row heights restored and `KNOWN_OVERFLOWS` empty, the gate is clean, so the fix
  > removed the false positive rather than moving it. `the_check_reports_an_overflow_when_one_exists`
  > still passes, so the gate can still fail.
  >
  > `the_recorded_overflow_is_the_collapsed_filter_panel` is replaced by
  > `a_collapsed_panel_overlapping_the_sidebar_is_not_reported_as_an_overflow`, which holds the
  > property that was actually violated: collapsing that panel changes no text and no clip, so it
  > must change nothing this gate reports.

- [X] T127 [P] Re-check the gallery in `mise run showcase`: `showcase/sections/surfaces.rs` poses three depth-1 rows at the default standard density, which have been rendering at 23.6dp against §7.2's 56. They are the most visible half of this bug and the page whose job is to be believed (feature 020)
- [X] T128 Confirm in the running application (`mise run run`): expand a worktree and check its session rows stand at the same dense height as the rows above them; check a tagged worktree row is at the two-line height with its chips unclipped; and record the new visible-worktree count in `quickstart.md` §B4 against the pre-change figure, since FR-026a now *permits* a decrease and the number should be written down rather than assumed (FR-026a, FR-026d)

  > **Both closed on the owner's screenshots, 2026-08-08, measured rather than eyeballed.** The
  > images were captured by the owner; the figures below are read off them at 1:1 — in each case a
  > selection pill gives an exact height and pins the scale, so nothing here rests on an impression.
  >
  > **T127, the gallery** (`standard` density). Selected two-line row **72px** against §7.2's 72;
  > one-line pitch **60.0px** against 56 + the column's 4dp gap; content centred, block centre 166.0
  > against pill centre 165.5. These are the rows that had been drawing at 23.6dp.
  >
  > **T128, the running application** (`dense`). `Default` one-line row **44**; the tagged worktree
  > row **60** with its chip unclipped; and the one this bug was about — the **session row at depth
  > 1, 44px exactly**, the same height as the top-level one-line row above it, where `1cb9873` had
  > left it at 23.6 and the original floor gave it 36. Content centred: label centre 137.0 against
  > pill centre 137.5.
  >
  > **§B4's count, recorded rather than estimated.** Against the 676.8dp tree viewport: one-line
  > rows 24 → **14** visible (−42%), tagged rows 14 → **10** (−29%). Inside the quarter-to-two-fifths
  > FR-026a now permits, and written into `quickstart.md` §B4 as the figure that clause asks for.
  >
  > **One defect found, and it is not this feature's.** The gallery screenshot shows a tag chip
  > ~47px left of the label it is meant to sit beneath. Filed as BUG-006; see there for why the
  > sidebar is unaffected.

> **T121–T126 done, 2026-08-07.** In that order, red first.
>
> **T121** replaced one depth-0 specimen with nine across three tests, and all three were confirmed
> failing against the unfixed component before a line of it was written — *"a one-line tree row at
> depth 0, density 0's height measured 18.199999dp … but its anatomy entry states 56dp"*. The
> count is the finding: the entry it replaced held a single specimen, at the one depth where the
> height had never worked, and that is what let its absence read as a decision.
>
> **T122/T123** moved the base and found no non-test reader to update — `LIST_ROW_BASE` reached no
> component at all, which is the same shape as BUG-002's `MIN_TOUCH_TARGET` and T097/T098's two
> figures. The grep was worth running to establish that, not to fix anything.
>
> **T124** put the floor on the row with a `Shrink`-width spacer. The wrapper the old comment warned
> against ("a wrapper would add a tree level and shift every recorded anchor beneath it") does
> exactly that, and the warning was right — but the alternative it chose, hanging the floor on the
> indent spacer, is what made the height depth-dependent. `layout_snapshot.rs` caught the shift **by
> name** (*"anchor `sidebar.row.label` … points at a path that no longer resolves"*), which is what
> named anchors are for; both were re-pointed rather than left to drift onto a neighbour.
>
> **T125/T126** — the fixture now records what it could not before. `main-shell-worktree-expanded`,
> sidebar tree column: `Default` 44.0, `feat-short` (tagged) 60.0, **its session row 44.0**, the two
> remaining tagged worktrees 60.0. The nested row stands at the same height as the top-level
> one-line row above it, which is FR-026d visible in the fixture for the first time in this feature.
>
> Workspace: **446 passed, 0 failed**. `cargo clippy --workspace --all-targets -D warnings` clean,
> `cargo fmt --check` clean.
>
> **T127 and T128 are not done and are not mine to close.** Both are Part B-class visual passes —
> one needs the gallery scrolled to its `TreeView` section, the other needs a worktree expanded in
> the running application, and neither is reachable without a person at the window. What can be
> asserted instead has been: T121 covers the gallery's exact specimen (standard density, depths 0
> and 1) and pins it at 56dp, and T125's covered state pins the application's session row at 44dp.
> That is the automated half; the half that is about how it *looks* is still open.

**Bugfix**: 2026-08-07 — BUG-005. T056 reopened, T076's conclusion annotated, T121–T128 added.
Numbered from T121 because BUG-003 landed T100–T111 first; this phase was drafted as T100–T107
against `4cd33b6` and renumbered on rebase.

---

## Phase 18: The composite gate — the hole between the binding gate and the boundary

The binding gate (T112) asks whether each §7 figure reaches *a* component. `material_boundary.rs`
asks whether a feature module names a *styled* widget, and deliberately exempts layout primitives
because a row has nothing to style. Between the two sits `row![Glyph::new(..), Text::new(..)]`: it
names no styled widget, reaches no styling layer, picks no raw size, and every figure it draws
wrong is still bound somewhere else. Both gates stay green.

That is how `shell.rs` shipped five buttons whose leading glyph was 14dp against §7.3's 18 — the
defect the manual §B4 pass found by reading, after every automated gate had passed. This phase
turns that reading into a rule.

**Renumbered T129/T130 → T140/T141, 2026-08-09.** `T129` named two different tasks: BUG-005's
text-overflow gate fix in the phase above, and this phase's extraction. BUG-005's is the one with
claims on the number — `bugs/BUG-005.md` cites it four times, including a postscript titled after
it — so this phase moved instead, to the first free numbers above Phase 19's T131–T139. That leaves
the phases numerically out of order and it stays that way: the numbers identify tasks, they do not
order them, and the phase above already records the same renumber-on-rebase churn that caused this.

- [X] T140 Extract the labelled icon into `crates/micold-client/src/ui/material/icon_label.rs`. Two feature modules built it by hand, identically — the "git" badge in `project_selector.rs` and in `shell.rs`'s known list. The type states the distinction the defect turned on: here the glyph takes the **label's** role, because a labelled icon is text with a picture in front of it; `Button::leading` is not that, and applies §7.3's 18dp whatever the label's role (Principle VIII, §7.3)

  > **Both call sites go through the type.** `project_selector.rs:59` and `shell.rs:128` each build
  > the "git" badge as `IconLabel::new(Icon::Git, "git", TypeRole::Label, r).tint(..).muted()`, and
  > neither spells the row out any more. The doc on the type carries the distinction rather than
  > leaving it to be rediscovered — and it is the doc T132 was corrected against when "regular icon
  > button" was first read as icon-only.

- [X] T141 Gate that no feature module composes a `Glyph` and a `Text` as siblings in a row or column of its own — `crates/micold-client/tests/composite_call_sites.rs`. Scans the same two roots as `material_boundary.rs` (`src/ui/` minus the library layers, plus all of `src/showcase/`), blanking nested macro bodies so a violation is reported once and against the inner row (FR-027, §7.3, SC-001)

  > **Scope, stated as a limit rather than a claim.** A list row with a leading status marker is
  > not caught, and should not be: in `shell.rs`'s known-projects entry the glyph and the name are
  > separate children of a §7.2 row, not a composite handed on as one thing, and §7.2's geometry
  > has its own gates. A rule that also banned list rows could not reach zero without a `ListRow`
  > component — and a gate that cannot reach zero is a budget, not a rule.

  > **Green, and proven able to fail**, 2026-08-09: 7 passed, 0 failed. The proof is
  > `the_guard_actually_works`, which feeds the scanner the shape `shell.rs` actually shipped —
  > `row![Glyph::new(..).tint(..), Text::new(..)]` behind a local `labeled()` helper — and asserts
  > it is flagged. Three more pin the edges the scan could get wrong: a commented-out composite is
  > not a violation, a nested one is reported once and against the *inner* row, and
  > `icon_role(..)`/`subtext(..)` are not read as `icon(`/`text(`. The exemption is load-bearing
  > rather than decorative — `the_library_is_excluded_because_it_owns_the_composite` asserts
  > `button.rs` still builds its leading slot as a row, so if that stops being true the exemption
  > gets re-examined instead of silently protecting nothing.
  >
  > **The boundary is drawn by syntax, not by meaning — worth knowing before trusting it.** The
  > scan reads `row!`/`column!` macro *bodies*, so a composite assembled with `.push()` is outside
  > it. Today that costs nothing and is even why the scope above holds: `shell.rs`'s known-projects
  > entry starts at `row![]` and pushes its marker, its name and its badge, so the list row this
  > rule deliberately does not want to catch is excluded by its own spelling. But the agreement is
  > luck rather than design — the same list row written as `row![Glyph::new(..), Text::new(..)]`
  > would be flagged against the stated scope, and a genuine labelled icon written with pushes
  > would escape. Closing that needs the semantic distinction the scope note says it has no cheap
  > way to make, so it stays a limit rather than a defect.

---

## Phase 19: BUG-007 — the project switcher was a second icon button and a second menu panel

**Goal**: delete the fork. `ProjectSwitcherTrigger` becomes `IconButton`, `ProjectSwitcherOverlay`
becomes `MenuOverlay`, `ProjectRow` becomes `MenuItem`, and `project_switcher.rs` goes with them
(FR-029c). The fix is subtraction; the only thing being *added* is the gate that would have caught
it (SC-008e).

- [X] T131 Failing test first: a same-kind gate in `crates/micold-client/tests/gates/` reading the app bar's actions and the bar-anchored panels as *sets*. Every action must match one of the two shapes §7 defines — the icon button's square target with its 24dp glyph, or the text button's 40dp height with §7.3's 18dp leading slot — and every bar-anchored panel must share one width and one trailing edge. Confirm it fails today: the switcher at 69.7×28 with a 14dp glyph matches neither shape, and the panels are 260 against 240 (SC-008e, FR-029c)

  > **Proven able to fail, not merely observed passing.** Deleting `.leading(..)` from the
  > switcher's button turns it red in all 17 covered states, naming the label's width (43.7dp,
  > 86.2dp where the project name is longer) where §7.3 states an 18dp glyph — because `glyph_of`
  > takes the *leading* leaf and a label with no glyph beside it leaves the label leading. Restored,
  > it is green again.
  >
  > **The first form of this test asserted the two actions were the same size, and that was
  > wrong** — it would have forbidden a labelled action outright, which is exactly the product
  > decision a gate has no business making. A closed set of component shapes still fails the
  > hand-assembled control (28dp is neither 48 nor 40; a 14dp glyph is neither 24 nor 18) without
  > deciding whether an action may carry words.
- [X] T132 Replace `ProjectSwitcherTrigger` with the shared **labelled** button in `ui/toolbar.rs` — `Button::text(active project name, r).leading(Icon::OpenProject, icon_role(AppBarAction, r)).on_press(Message::ProjectSwitcherToggled)`. The label stays: it is the quickest way to see which project is active without opening the panel. What changes is that §7.3's 40dp height, 12dp ends, 18dp leading glyph and ripple are the component's rather than the call site's (FR-029c, 008 FR-004)

  > **Corrected on the owner's word, 2026-08-08.** The first attempt read "regular icon button" as
  > icon-only and moved the name into a tooltip — a product change nobody asked for, made while
  > fixing an assembly problem. `IconLabel`'s doc is the distinction that was missed: a labelled
  > icon sizes its glyph at the label's role, and a *button's* leading slot is §7.3's 18dp whatever
  > the label is. The switcher wanted the second, which `Button::leading` has owned all along.
- [X] T133 Replace `ProjectSwitcherOverlay` with `MenuOverlay` in `ui/mod.rs`, building `MenuItem`s directly from `state.switcher_entries()` — active marker with `reserve_icon`, running count as trailing text, unavailable badge as trailing icon, `on_context` for the forget menu, and the trailing "Add project…" item. The panel gains the shared width, the shared anchor and the shared enter/exit fade by construction (FR-029c, FR-029a)
- [X] T134 Delete `src/ui/material/project_switcher.rs` and its re-exports (`ProjectRow`, `ProjectSwitcherOverlay`, `ProjectSwitcherTrigger`) from `material/mod.rs`. `row_column`'s `pub(super)` reason disappears with it; check `menu_anatomy`'s §7.5 measurements still reach every row they did (FR-029c)
- [X] T135 [P] Follow the deleted types through the showcase: `showcase/catalogue.rs`'s three entries and `showcase/sections/floating.rs`'s two specimens. The switcher panel is now a `MenuOverlay` specimen; the separate trigger entry becomes an app-bar `IconButton` or is dropped as a duplicate of `MenuTrigger` (feature 020)
- [X] T136 [P] Follow them through the tests that name them: `tests/project_switcher.rs`, `switcher_forget_menu.rs`, `showcase_state.rs`, `one_overlay_implementation.rs`, `type_role_call_sites.rs`, `anatomy_call_sites.rs` and `tests/support/covered_states.rs`. Behaviour assertions stay; only the constructor they drive changes (Principle I)
- [X] T137 Regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt` (`UPDATE_LAYOUT_SNAPSHOT=1`). The diff is the proof: `app_bar.switcher_trigger` moves to §7.3's 40dp height with an 18dp leading glyph, and `switcher.panel` moves 260 → 240 wide, its leading edge landing on the overflow panel's (feature 019, SC-008e)
- [X] T138 [P] Update `docs/user-guide/project-selection.md`: the switcher is a button showing the folder glyph and the active project's name. `icons.md` needed no change — its two switcher rows are the open-folder glyph, which is now the button's leading icon, and the active marker, which is where it always was. Same change, same commit, per the documentation rule (FR-041)
- [X] T139 Confirm in the running application: the switcher naming the active project beside its folder glyph and standing level with the ⋮ button, hover and ripple on both, the switcher panel opening like the ⋮ menu with its left edge flush with that panel's, and right-click on a project row still reaching "Forget project" (SC-008e, US4 scenario 13)

  > **Run 2026-08-08 without a person at a display**, by the `visual-pass` route: the client on a
  > private Xvfb `:77` under lavapipe, driven with `xdotool`, captured with `import`. Isolated with
  > `XDG_DATA_HOME`/`XDG_CONFIG_HOME`/`XDG_RUNTIME_DIR` pointed at a scratch dir seeded with two
  > invented projects, so it could not reach the owner's daemon or catalogue — confirmed by the
  > instance failing to connect to its own socket until the runtime path was shortened.
  >
  > **Passed.** The switcher draws the folder glyph and `micold-ai-ide` on one line, level with the
  > ⋮ and carrying the text button's state layer on hover. Both panels, cropped at *identical*
  > geometry and stacked, share a leading edge, a surface tone, a corner and a row pitch — and the
  > unmarked row's label starts where the marked row's does (FR-006a, visible rather than inferred).
  > Right-click on a project row opens "Forget project" at the cursor, above the switcher panel,
  > with the switcher still open behind it.
  >
  > **One defect found, and fixed in the same change.** The switcher's glyph was `on_surface`
  > (`#DDD8DD`) beside a label the text button draws in `primary` (`#BDACE9`) — one control in two
  > colours. Sampled rather than eyeballed. `IconSurface::AccentButton` now carries §7.3's accent
  > for a text/outlined button's leading glyph, and the re-measured pair agrees. **No geometry gate
  > could have caught this**: the glyph was in exactly the right box, in the wrong tone.
  >
  > **Left unrun**, per the route's own limits: the *mid-flight* look of the panel's fade. A
  > screenshot pipeline cannot reliably catch a chosen frame of a short transition, and lavapipe's
  > frame pacing says nothing about the owner's GPU. That the panel fades at all is structural —
  > it is `MenuOverlay`'s own transition, covered by `overlay_transition_identity`.
  >
  > **A pre-existing mismatch of the same kind, deliberately not fixed here**: `ui/shell.rs:35` and
  > `ui/project_selector.rs:26` pass `on_surface` to an *outlined* button's leading slot, whose
  > label is also `primary`. Same two-colour control, four call sites, none of them this feature's
  > subject. `shell.rs:108` is **not** one of them — its `error` tint on a Delete action is a
  > deliberate semantic override, which is why `leading` must keep taking a tint at all.
  >
  > **Both of those held only until the follow-up**, which fixed the three other call sites at the
  > source instead: `Button::leading` now takes no tint and resolves the variant's own content
  > colour, `Button::leading_tinted` is the explicit override the Delete action uses, and
  > `IconSurface::AccentButton` — named above — is gone, because no call site names the accent any
  > more (BUG-007 follow-up, 2026-08-09).

  > **Second pass, 2026-08-09, on merged `main` (`24379b7`)** — the same `visual-pass` route
  > (private Xvfb under lavapipe, `xdotool`, isolated `XDG_*`, cleaned up by PID), re-run after the
  > follow-up landed, and this time in **both schemes**.
  >
  > **Passed.** Dark at rest: folder glyph and `micold-ai-ide` on one baseline, both in the accent
  > tone — the two-colour defect above is closed at the merged state, not just in the branch that
  > fixed it. Hover: the state layer covers the whole button, glyph and label together, rather than
  > the glyph alone. Open: the panel hangs below the bar and its divider at the shared 240dp width,
  > `micold-ai-ide` carries the active marker, `switcher-branch` is unmarked with its label starting
  > at the same x (FR-006a), and "Add project…" trails. Light: the same treatment at rest and open,
  > legible throughout — the app bar's mean of 0.9775 is what confirms the scheme actually flipped
  > rather than the capture being mislabelled.
  >
  > **Still unrun**, unchanged from the first pass: the mid-flight look of the fade, for the same
  > reason.


## Phase 20: BUG-008 — the sidebar's context menus opened at a corner, not at the row

**Goal**: a context menu opened from an element opens at the press point, because the press point
arrives with the gesture (FR-029d). The fix is mostly **subtraction** — a constant goes, and with it
the pointer subscription that existed to work around a message with no point in it. What is added is
the gate that reads a panel against the element it belongs to (SC-008f), which is the scope none of
this feature's five checks had.

**Note on `State::cursor`**: it exists because `MenuItem::on_context` hands over a bare message, so
feature 015 tracked the pointer on the side and subscribed to moves *only while the switcher is
open* to keep the idle window quiet (015 FR-010). Once the point rides on the message, the side
channel has no remaining caller. Removing it is the stronger form of 015's own requirement, not a
change to it.

- [X] T142 Failing test first: `crates/micold-client/tests/gates/context_menu_anchor.rs`, compiled into the `layout_snapshot` binary for the reason `containment`, `panel_placement` and `sibling_parity` each give. For every context menu the application has — worktree row, session row, switcher project row, terminal tab — open it at **two distinct press points** and assert the laid-out panel's origin follows the point, clamped by `clamp_menu_anchor`, rather than holding still. Two points, not one, for `anatomy_size`'s reason one scope out: a single press cannot separate "anchored at the press" from "anchored at a constant that happens to be near it". The edge-anchored panels of FR-029d's exception (the overflow menu, the switcher panel, the tab menu's `rising_above`) are **declared** in the check, so the gate distinguishes a stated exception from a forgotten point. Confirm it fails today: the worktree and session panels report the same origin for both presses (SC-008f, FR-029d)

  > **Red on all four, for the right reason.** `worktree` right-clicked at (126, 641) → menu at
  > (24, 96), 555px away; `session` at (126, 249) → (24, 96); `moves` — presses 448px apart, panel
  > moved 0px. The fourth is its own finding: the **project** menu, which feature 015 anchors
  > correctly in the running application, opened at **(0, 0)** — it reads `State::cursor`, a side
  > channel fed only by the binary's switcher-gated pointer subscription, so outside the binary it
  > points at nothing. That is what T146 removes.
  >
  > **Two corrections the assertion had to make**, neither of them slack: the press point is `f32`
  > at the widget and `u16` on the message, and a press within a panel's height of an edge is
  > clamped. Both are the application's own arithmetic, and the clamp is read from the panel's
  > *measured* size rather than restated.
- [X] T143 Give `material::TreeItem::on_right_press` the shape `ContextArea::on_secondary_press` already has — `impl Fn((u16, u16)) -> M` — and let `tree_view.rs` wrap its row in `cdk::ContextArea` rather than `mouse_area`, so the press point reaches the message instead of being dropped at the widget boundary. This is the missing parameter the old `SIDEBAR_MENU_ANCHOR` doc comment described as a design decision (FR-029d, Principle VIII)
- [X] T144 Carry the point on the messages and in the state: `WorktreeMenuToggled(String, (u16, u16))` and `SessionMenuToggled(SessionId, (u16, u16))`, with `worktree_menu_open` / `session_menu_open` holding an anchor beside their subject, as `ProjectMenu` already does (`features/project.rs`). Toggle semantics are unchanged and must stay covered: the same row closes the menu, a **different** row replaces it *and re-anchors* — FR-029d's second clause, and the case a fixed anchor could never fail (FR-029d)
- [X] T145 `ui/mod.rs`: both sidebar menus clamp the point their message carried, and **`SIDEBAR_MENU_ANCHOR` is deleted**. `clamp_menu_anchor` stays exactly as it is — it is FR-029d's window-containment half and it was never the wrong part (FR-029d, 015 FR-006)
- [X] T146 [P] Do the same at the last call site that reconstructs a point: `MenuItem::on_context` (`material/menu.rs`) takes the `ContextArea` shape, `ProjectMenuToggled` carries its own point, and `Message::CursorMoved`, `State::cursor` and `shell/subscriptions.rs`'s switcher-gated pointer subscription are **removed** — the workaround they were, together. Idle behaviour is unchanged and `idle_subscriptions.rs` must still pass; what changes is that no window state can now make the application listen to mouse moves (015 FR-010, FR-029d)
- [X] T147 Cover the case the fixture cannot currently show: `tests/support/covered_states.rs`'s `worktree-menu-open` state opens its menu at a point **well down the sidebar**, and a session-menu state joins it. Regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt` (`UPDATE_LAYOUT_SNAPSHOT=1`); the diff is the proof, the panel moving off `24, 96` to the row it belongs to (feature 019, SC-008f)
- [X] T148 [P] The structural half, so the next context menu cannot arrive with a constant: extend the call-site scan family (`anatomy_call_sites.rs` and its neighbours) to fail when a `MenuOverlay`/`ContextMenu` anchor is fed a `const` rather than a value carried by the state it renders, with FR-029d's edge exceptions named. The behavioural gate T142 catches the menus that exist; this catches the one written next year (SC-008f)

  > **Proven able to fail, not merely observed passing.** Re-anchoring the worktree menu at a
  > `const PROBE_ANCHOR: iced::Point = iced::Point::new(24.0, 96.0)` turns it red naming
  > `src/ui/mod.rs:325` and the constant; reverted. The scan also unit-tests its own predicate, so
  > `menu.anchor` and `iced::Point::new(x as f32, y as f32)` are not mistaken for shouts and
  > `anatomy::app_bar::HEIGHT` is recognised as the stated exception rather than merely missed.
- [X] T149 Confirm in the running application, by the `visual-pass` route: right-click the **last** worktree row and the **last** session row of a long sidebar and see the menu at the pointer; right-click a different row and see it move; right-click within a panel-height of the window's bottom edge and see it clamped and whole; and confirm the switcher's project menu and the terminal tab menu still open where they did (US4 scenario 14, SC-008f)

  > **Run 2026-08-20 without a person at a display**, by the `visual-pass` route: the client and a
  > matched daemon on a private Xvfb `:84` under lavapipe, driven with `xdotool`, captured with
  > `import`. Isolated with `XDG_DATA_HOME`/`XDG_CONFIG_HOME`/`XDG_RUNTIME_DIR` pointed at a scratch
  > dir, over an **invented** git repository with ten worktrees, so it could not reach the owner's
  > daemon, catalogue or worktrees.
  >
  > **Passed, four checks:**
  >
  > - **The reported case.** A session row's `Close`/`Remove` right-clicked at (150, 248) opens at
  >   (150, 248) — the panel's top-left corner on the row, where the bug report's screenshot has it
  >   at the top of the sidebar over the "Worktrees" header.
  > - **The last worktree row** of a ten-row list, right-clicked at (120, 758): menu at (120, 755).
  > - **It moves.** Right-clicking `Bravo` at (150, 245) after `India` moves the panel to (150,
  >   245) rather than leaving it where the last press put it.
  > - **The clamp.** With the window shortened to 850, the same row at (120, 760) — whose 160dp
  >   panel does not fit below it — opens at (111, 687), its bottom edge on the window's. The
  >   pointer then sits over the panel's second item, which feature 015's own assumption records as
  >   the accepted cost of clamping rather than flipping.
  >
  > **The two menus this change did not touch still open where they did**: the switcher's "Forget
  > project", right-clicked at (1421, 97) inside the switcher panel, opens at (1360, 97) — clamped
  > against the window's right edge, over the switcher, which stays open behind it (015 FR-009);
  > and the terminal pane's own `Copy`/`Paste`, right-clicked at (800, 600), opens there. The first
  > of those is the regression check that matters for T146: it now anchors from the message rather
  > than from the `State::cursor` that went with the subscription.
  >
  > **Left unrun**: the **terminal tab strip's** instance menu. It is FR-029d's stated edge
  > exception — it rises from the bar's top edge rather than falling from the press — its anchor
  > was untouched by this change, and reaching it means switching a session to regular-terminal
  > mode and opening a second instance. `gates/context_menu_anchor.rs` covers the three menus that
  > fall from the press; the one that rises is covered by feature 012's own tests.
  >
  > Evidence: `evidence/BUG-008-menu-at-the-press-point.png` — the session menu at its row (red)
  > beside the worktree menu clamped at the bottom edge (blue).
- [X] T150 [P] Read `docs/user-guide/worktrees-and-sessions.md` §"Managing a worktree (right-click)" and §"Right-click a session" against the change. Neither states where the menu appears, so the documentation rule (FR-041) is expected to require **no edit** — record that it was checked rather than leaving the question open (FR-041)

  > **Checked 2026-08-20; no edit.** §"Managing a worktree (right-click)" lists what the menu
  > offers, and the session section likewise ("Right-click a session for **Close** and **Remove**").
  > Neither states where the menu appears, so neither was made wrong by it appearing somewhere else
  > for two years, and neither is made right by this fix. A sentence saying a context menu opens
  > where you right-click would be documenting the absence of a bug.

**Bugfix**: 2026-08-20 — BUG-008 added Phase 20 (T142–T150). No task is reopened: T107, which promoted `(24, 96)` to a named constant, is genuinely complete as written — it was scoped to whether the panel *fits*, and so was BUG-003's "adjacent risk" note before it. The work missing here was never a task.

## Phase 21: BUG-009 — the action on a red banner was drawn in the accent it stood on

**Goal**: a component placed on an accent fill takes its foreground from that fill's paired `on_*`
role, and the **component** carries it rather than the call site (FR-004a, FR-027b). The gate reads a
**composition** pair — the role a component's foreground resolves to, against the role of the
container it is rendered in — which is the scope FR-004's token-set test structurally cannot have
(SC-008g).

**Note on why every gate was green**: `tokens_contrast.rs` is exhaustive over the pairs §1.3
*enumerates*, and §1.3 can only enumerate compositions somebody thought of; `style_snapshot` pins
`container.notification[error]`'s fill and the button's colour, both correct, one style at a time;
`anatomy_size` and the 019 probes measure geometry, and this costs no pixels. The showcase renders
the failing composition and has been through visual passes — it reads as a design choice unless you
know `primary` is not supposed to be there.

**Note on the seam**: `Variant::content(roles)` already exists as the one place a variant's content
colour is decided, and already has a second caller for exactly this reason — a nested `IconButton`
sets an explicit glyph colour and must ask the variant rather than default to `on_surface` (012
BUG-001). The foreground override belongs there, so the label, the nested glyph, the border, the
state layer and the ripple stay in step by construction rather than by four matching edits.

- [X] T151 Failing test first: ~~`crates/micold-client/tests/composition_contrast.rs`~~
  `crates/micold-client/src/ui/material/composition_contrast.rs` — the style layer is `pub(crate)`
  by design (017 FR-002), so an integration test in `tests/` cannot call `style::notification` or a
  variant's style function at all, and resolving both halves "by calling the functions the view
  calls" is the whole point of this gate. In-crate, beside `style_snapshot.rs`, which is in-crate
  for the same reason. For every
  `NoticeLevel` and every button variant an application banner puts inside one, resolve the
  container's `background` from `style::notification` and the button's `text_color` and border
  colour from the same style function the view reaches, in **both** schemes and at every
  `button::Status`, and assert 4.5:1 for the label and 3:1 for the border against that background.
  Resolve both halves by **calling the functions the view calls** — an inventory of "which component
  sits on which container" restated beside the code is FR-029a's copied constant in another form
  (SC-008g). Confirm it fails today: `Error` × outlined is 1.00:1 light and 1.01:1 dark, its border
  1.42:1, and its hover and pressed fills are `primary` at low alpha on red (FR-004a, SC-008g).
  **Confirmed red, 2026-08-21**: 27 violations. On `Error`, exactly as predicted — outlined label
  1.00:1 light / 1.01:1 dark, its border 1.44:1 / 1.86:1, text label the same, and hover and press
  moving them only to 1.04–1.05:1. The gate accumulates every violation rather than stopping at the
  first, because one wrong foreground is wrong on both schemes at every state and a fail-fast gate
  turns one cause into a queue of reruns. Nine further violations landed on the **neutral** `Info`
  host and are **not** this bug — see the scope note below
- [X] T152 Give the button variants the foreground they draw in. `Variant::content` takes the
  override; `style::outlined`, `style::filled` and `style::text_button` take the content role rather
  than reading `r.primary` / `r.on_primary` unconditionally, and the outlined variant's border takes
  that role at the border's opacity instead of `r.outline`. One definition of each variant,
  parameterised — not a second outlined button at the call site, which FR-021/FR-027 forbid and T080
  has already deleted once (FR-027b, §7.3's **Host surface** table)
- [X] T153 `Button` gains the builder that carries it, and the state layer and ripple follow the same
  role — a control on a filled container is one control in one colour, not a recoloured label over an
  accent-tinted press. Keep the neutral-surface default exactly as it is: a call site that says
  nothing gets §7.3's table unchanged, which is what makes this a parameter rather than a migration
  (FR-027b, FR-024)
- [X] T154 The connection banner derives its action's foreground from **the same decision that
  produced its fill** — the `(bg, fg)` match in `style::notification` — rather than restating the
  role beside it at the call site. `connection_banner.rs` asks for the pair once and hands the
  foreground to the action, so a fifth notice level cannot arrive with a readable banner and an
  unreadable button (FR-004a, FR-027b, FR-029a's rule in colour)
- [X] T155 [P] Sweep the other hosts. The two `Restart service` banners (`ui/mod.rs:93-130`,
  `VersionMismatch` and `BuildMismatch`) go through the same component, so the expected finding is
  that T154 already fixed them — **record that they were checked**, rather than leaving it implied.
  Then look at every other accent-filled container that hosts an interactive child: filled tags and
  chips (§7.6), the snackbar's action on `inverse_surface`, and any dialog surface that is not
  neutral. Fix or record each, so FR-004a's class is closed rather than its one instance — the shape
  T107 got right and BUG-008 shows is worth doing (FR-004a). **Checked, 2026-08-21**: the two
  `Restart service` banners (`ui/mod.rs:98-129`) and the takeover banner are all three built by
  `ConnectionBanner`, so T154 fixed all three at once — recorded rather than implied. The
  **snackbar** was a real second instance and is fixed: its `Dismiss` label was already
  `inverse_primary`, but *tinted onto the label at the call site*, so the hover and press layers and
  the ripple stayed `primary` over the inverted fill — BUG-009's shape at a smaller amplitude, and
  precisely what FR-027b's "the component carries it" is for. It now takes
  `style::snackbar_host(r)`, and the manual tint is gone. Tags and chips (§7.6) are **tonal** —
  `alpha(accent, 0.20)` behind `accent` — not accent fills, and they host no interactive child; the
  dialog surface is neutral. Nothing else in the crate fills a container with an accent role
- [X] T156 [P] The showcase's "with an action" specimen
  (`showcase/sections/surfaces.rs:189-198`) renders the corrected pairing, and gains the `Info` host
  beside it so both banner surfaces are visible together. The `Info` banner's action is readable
  today only because `surface_variant` happens to be inside §1.3's neutral enumeration — luck, not a
  rule — and BUG-009's "adjacent risk" note records it. Showing the two side by side is what makes
  the rule visible rather than the coincidence (FR-004a, SC-007)
- [X] T157 [P] `style_snapshot.rs` pins the button style as it resolves **on each notice level**, not
  only on the surface. The snapshot records styles one at a time by design; naming the composition is
  how a composition gets into it, and it is the byte-for-byte half that T151's threshold assertion
  deliberately is not (SC-008g). Every host is posed, the neutral ones included: a level whose fill
  stops imposing is exactly the regression this file exists to make visible, and it can only show up
  if the neutral pose is recorded rather than skipped. The regenerated fixture is **78 added lines
  and no changed ones** — the parameter is genuinely opt-in, and no call site that says nothing has
  moved
- [X] T158 Confirm in the running application by the `visual-pass` route, in **both** schemes: the
  takeover banner and a version-mismatch banner, label and border legible against the red, at rest
  and under hover and press. Both schemes rather than one — the dark scheme fails identically at
  1.01:1, and a light-only screenshot would not show it. Record the result in
  `../evidence/BUG-009-action-on-the-error-banner.png` (SC-008g, FR-004a). **Run 2026-08-21** on
  Xvfb `:91` + lavapipe (software Vulkan), showcase binary pinned out of `target-shared` and
  verified as this branch's before launching. Both schemes, `ConnectionBanner` § "with an action",
  at rest, under hover and under press — six poses, all legible: light draws the label and the pill
  border in white on the red fill with a white state layer lightening it under the pointer, dark
  draws both in the dark-scheme `on_error` maroon on the salmon fill. The `Info` specimen beside it
  keeps `primary`, which is the pair T156 put there to make visible. The **snackbar**'s `Dismiss`
  was checked in the same pass and reads `inverse_primary` on the inverted fill. What this route
  cannot answer, unchanged from every prior pass: mid-flight animation frames and perceived
  smoothness — neither is claimed here, and neither is at issue in a colour fix
- [X] T159 [P] Read `docs/` against the change (FR-041). The takeover banner is described by its
  behaviour, not by its colours, so the expected outcome is **no edit** — record that it was checked
  rather than leaving the question open (FR-041). **Checked, 2026-08-21, no edit.**
  `docs/daemon.md:165-202` describes the banner and its `Take over` button by what they do;
  `docs/user-guide/appearance-theming.md:32` says outlines are used for "the border of an outlined
  button", which names the visual device and not the role that colours it, and stays true

**Scope note (T151)**: the gate asserts the composition on hosts that **impose** — accent fills —
because that is what FR-004a is a rule about. Run unscoped it also reports nine near-misses on the
`Info` banner's neutral `surface_variant`: labels at 4.40–4.49:1 under the hover and pressed state
layers, and an `outline` border at 2.42–2.96:1 in the dark scheme, against thresholds of 4.5 and 3.
Two causes, neither of them this bug — a state layer eroding a pair §1.3 measured at rest, and
`outline` on `surface_variant` missing 3:1 in the dark scheme before any layer at all. Both predate
Phase 21 and are unchanged by it. They are filed as [BUG-010](bugs/BUG-010.md) rather than absorbed
here, because widening this gate to cover them would make it fail for a reason BUG-009 did not
cause, and because choosing between "retune the tone" and "declare state layers exempt on neutral
hosts" is a contract decision rather than an implementation one. The filter is a single
`imposed().is_none()` skip, so removing it is one edit when that decision is made.
**Removed 2026-08-25 (T163)** — the decision is FR-004b, the edit was the one line this note
promised, and the walk now covers every container rather than the accent ones. The nine near-misses
above are closed by T162 moving the `Info` banner's fill, not by the gate being told to ignore them.

**Bugfix**: 2026-08-21 — BUG-009 added Phase 21 (T151–T159). **No task is reopened.** T080 rebuilt the banner's action on the shared `Button` and is complete as written — the hand-rolled control it replaced called `style::outlined` and was purple-on-red too, so the task did not introduce the defect; what it did was make the colour arrive from the shared library, which is where the missing rule has to live. T000a/T000b built the contrast gate faithfully over §1.3's table, and the table is the gap rather than the gate. T036 applied the state layers as specified; they are `primary` at low alpha and follow the label's role once FR-027b puts that role in the component. The work missing here was never a task, and could not have been while no artifact said a component's foreground depends on what it is standing on.

---

## Phase 22: BUG-010 — the composition gate's excluded class, and the host that was never enumerated

**Goal**: remove the accent-host filter from the composition gate and make the contract true where it
then fails. The rule is decided (FR-004b): a pair is measured **with the heaviest state layer its
element can carry composited**, and where a pair fails, the *host* narrows rather than the palette.

**The report's premise was wrong, and the correction is the work.** BUG-010 was filed as "an
enumerated pair erodes under a state layer". §1.3's `primary` row enumerates `surface`,
`surface_container_low` and `surface_container` — three hosts, `surface_variant` not among them —
and `tokens_contrast::text_pairs` asserts exactly those three. So the `Info` banner's action was
never a permitted composition; it was an unenumerated one that no gate covered and that two prose
sentences asserted was fine. Both sentences are corrected here.

- [X] T160 Failing test first: extend `crates/micold-core/tests/tokens_contrast.rs` to assert every §1.3 pair **with the heaviest layer its element can carry composited**, not only at rest. The compositing arithmetic already exists as `style::over` in the client; the core has no `Color`, so this needs the same operation over `Rgb` in `tokens` — one function, used by both, rather than a second copy of alpha blending (FR-004b, SC-008h)

- [X] T161 [P] Add `surface_container_high` to §1.3's permitted hosts for `primary` in `tokens_contrast::text_pairs`, and confirm T160 is green for all four. It is a widening, and it is measured rather than assumed: 5.28/4.75/4.63 light and 8.43/7.04/6.71 dark for the label, 3.68/3.31/3.23 and 4.56/3.81/3.63 for the border (FR-004b, §1.3)

- [X] T162 The `Info` banner stops hosting an accent role. `style::notification_host`'s `NoticeLevel::Info` arm takes `surface_container_high` in place of `surface_variant`; its `on_fill` stays `on_surface`, which §1.3 already enumerates on every `surface_container_*` level. One edit, in the one decision both the banner's fill and its action read (FR-004b, FR-027b)

- [X] T163 Remove the accent filter from `crates/micold-client/src/ui/material/composition_contrast.rs` — the single `imposed().is_none()` skip T151 wrote to be removable in one edit — and delete the module doc's "**Scope: hosts that impose**" paragraph with it. Confirm the walk is green over every host and both schemes, and that it was **red before T162**, on the nine violations BUG-010 tabulates (SC-008h)

- [X] T164 [P] Correct the two prose claims that made the exclusion look safe, rather than only the code: `Host::neutral`'s doc comment in `style.rs` ("§1.3 enumerates the backgrounds `primary` may be drawn on … `surface_variant` among them") and BUG-009's "Adjacent risk, not this bug" section, which says the same. Both are the reason nobody re-read the table (FR-041's argument applied to a doc comment)

- [X] T165 [P] Sweep the other neutral hosts a component is drawn on for the same unenumerated composition — `style::list_item`'s `surface_variant` fill, `edge_fade`'s `primary`/`surface_variant` pair, and the showcase's own specimens. Measured, not eyeballed: `on_surface` on `surface_variant` is 10.50:1 light and 5.39:1 dark under the heaviest row layer, so `list_item` is inside §1.3 and stays — **record that it was checked**, per the class-not-instance rule T155 and BUG-008 established (FR-004b)

- [X] T166 [P] `style_snapshot.rs` re-pins the `Info` banner's fill and its action's resolved style on the new host, and the showcase's banner specimen poses it. The snapshot diff is the proof the fill moved: light `#E7E0EC` → `#ECE7EC`, dark `#49454E` → `#2B292D` (SC-007, feature 019 FR-003)

- [X] T167 Confirm in the running application by the `visual-pass` route, in **both** schemes: the
  `Info` banner against its new fill, its action at rest and under hover and press, and the `Error`
  banner beside it unchanged. Record to `../evidence/` (SC-008h, US4 scenario 16). **Run
  2026-08-25** on Xvfb `:71` + lavapipe (software Vulkan), showcase binary built and copied to a
  private pin dir inside the build lock so it cannot be another branch's. The fill was **sampled,
  not eyeballed**: both `Info` specimens read `srgb(236,231,236)` = `#ECE7EC` in the light scheme and
  `srgb(43,41,45)` = `#2B292D` in the dark, which are exactly the two values T166 predicted from the
  snapshot — the screen and the fixture agree. `Retry now` on that fill is legible at rest, under
  hover and under press in both schemes, the pressed state layer visibly filling the pill without
  taking the label with it; the dark border, which was the 2.96:1 miss at rest on the old host, now
  reads clearly against `#2B292D`. The two `Error` specimens beside it are **unchanged** — white
  label and white pill border on the red fill in the light scheme, `on_error` maroon on salmon in
  the dark — so narrowing the `Info` host did not disturb BUG-009's fix. Recorded to
  `../evidence/BUG-010-info-banner-on-its-new-fill.png`: both schemes, all four specimens, and the
  action's three states cropped at identical geometry beneath each. What this route cannot answer,
  unchanged from every prior pass: mid-flight animation frames and perceived smoothness — neither is
  claimed here, and neither is at issue in a colour fix

- [X] T168 [P] Read `docs/` against the change (FR-041). The banner's tone is described nowhere in the user guide, which is expected — record that it was checked rather than leaving the question open

**Scope note (T160)**: the composited measurement applies to §1.3's *whole* table, not only to
`primary`'s row, because a state layer is drawn on every interactive surface (§5) and no row is
exempt by construction. Nine of the twelve host columns needed nothing; that is the rule being cheap,
not the rule being narrow.

**Sweep record (T165)** — the class, not the instance. Every neutral host a component is drawn
on was measured with the heaviest layer that component can carry, in both schemes, not only the three
BUG-010 tabulated:

- `style::list_item`'s `surface_variant` fill carries `on_surface`, which §1.3 enumerates: **10.50:1**
  light and **5.39:1** dark under `selected` (0.12), the heaviest layer a row draws. Inside the
  contract, and it stays.
- `edge_fade` pairs `primary` with `surface_variant` and draws **no text and no border** — it is a
  gradient to transparent. §1.3's obligation is about legibility of content against a container; there
  is no content. Recorded as checked and out of scope, which is a different answer from "it passes".
- The showcase's own specimens draw on `surface`, `surface_container` and `surface_container_low`,
  all enumerated, all clearing AA composited (6.14/5.51/5.33 and up — see BUG-010's host table).
- **Two pairs failed**, both dark-scheme only and neither in BUG-010's tabulated set: the snackbar's
  action (`inverse_primary` on `inverse_surface`, 4.37:1 pressed) and the sidebar's untyped filter
  chip (`on_surface_variant` on `surface_variant`, 4.47:1 pressed). They are **not** absorbed here:
  FR-004b's remedy reaches the chip and cannot reach the snackbar, which has one fill by definition,
  so the rule needs a clause this phase did not decide. Filed as **BUG-011**, and pinned meanwhile in
  `tokens_contrast::UNDER_AA_COMPOSITED`, an assertion that fails if the set gains a member *or loses
  one* — so fixing either breaks the build until its row goes.

**Docs record (T168)** — `docs/` was read against the change. The banner appears once, at
`docs/user-guide/appearance-theming.md:169`, and describes what it *does*, not what tone it is drawn
in. **Nothing to change**, recorded rather than left open (FR-041).

**Snapshot record (T166)** — the fixture moved on 17 lines, not 4. Four are the `Info` banner and its
host, exactly as predicted. The other thirteen are checkbox state layers drifting in the **last f32
ulp** (0.8858823 → 0.8858824), because T160 made `style::over` delegate to `tokens::blend_channel` and
the shared function computes in `f64`. No 8-bit channel changes; nothing renders differently. It is
the cost of having one blend instead of two, and the diff is the evidence there is now one.

**Bugfix**: 2026-08-25 — BUG-010 added Phase 22 (T160–T168). **No task is reopened.** T151 built the
composition gate scoped to accent hosts and recorded the exclusion in its own module doc, which is
faithful to FR-004a as it stood and is why the nine findings were filed rather than lost. T000a/T000b
built the contrast gate exhaustively over §1.3's table, and the table — not the gate — is what did
not account for §5. The work missing here was never a task: no artifact said a pair must survive its
own state layer, and the one sentence that would have caught the `Info` banner anyway ("`primary` may
be drawn on `surface_variant`") was prose that contradicted the table beside it, which no check reads.

