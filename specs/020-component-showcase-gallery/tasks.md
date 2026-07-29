---

description: "Task list for feature 020 — Component Showcase Gallery"
---

# Tasks: Component Showcase Gallery

**Input**: Design documents from `/specs/020-component-showcase-gallery/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: Per Constitution Principle I (Test-First Development, NON-NEGOTIABLE), test tasks are
MANDATORY. Every user story writes its failing tests before its implementation.

**A note on what "Red" means for a gate.** Most of this feature's tests are *gates* — they read
source text or `const` data and assert a property of the repository. A gate cannot be made to fail by
"not having written the feature yet" when the property it asserts is already true (the packaging
manifest does not name the showcase today, and never should). So each gate's Red step is stated
explicitly per task, and is one of two shapes:

- **vacuity Red** — the gate asserts it found something to scan, and fails until the directory or
  catalogue exists (this is FR-016's shape, applied to the gate itself);
- **synthetic Red** — the gate's rule is a function over its inputs, and a test drives it against a
  deliberately-broken synthetic input, so the failure behaviour is proved on every run rather than
  once by hand ([contracts/completeness-check.md §5](./contracts/completeness-check.md)).

A gate written without one of these is a gate nobody has seen fail, which is the thing this feature
is about.

**Constitution 1.5.0.** Principle I's GUI-wiring exception previously named only `src/main.rs` and
`src/ui/`, so the showcase's own render glue fell outside it. It was amended to cover a
development-only binary's render glue — a **MINOR** bump, because widening what a NON-NEGOTIABLE
principle exempts is a material expansion rather than a wording fix. T054 is the other half of that
amendment: it asserts the glue holds no decision logic, so the exception's precondition is checked
on every build rather than trusted.

**Documentation**: Per Constitution Principle VII, documentation ships in the same change. FR-024
directs it to **developer** documentation and explicitly forbids extending the user guide, and one
document covers all four stories rather than four fragments — so it lands in the final phase as a
**mandatory** deliverable (T047), not as optional polish. The feature is not done without it.

**Cross-platform**: Per Constitution Principle VI, scoped for the showcase to **compilation** on
Linux, macOS and Windows (spec, Assumptions). The feature's gates additionally *run* on all three —
the authoritative list is T049's CI step (T049, T052).

**Organization**: Tasks are grouped by user story so each can be implemented and tested independently.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1–US4)
- Paths are relative to the repository root

## Path Conventions

Rust workspace. Everything this feature touches lives in `crates/micold-client/` (source in `src/`,
integration tests in `tests/`), plus `docs/`, `mise.toml` and `.github/workflows/ci.yml`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: make a second binary exist, without breaking the first.

- [X] T001 Add `[[bin]] name = "micold-showcase", path = "src/showcase/main.rs"` **and**
  `default-run = "micold-ai-ide"` to `crates/micold-client/Cargo.toml`. Both lines, together:
  without `default-run` a second binary makes `cargo run -p micold-client` — i.e. `mise run run` —
  ambiguous and it fails ([research R1a](./research.md#r1a--default-run-is-not-optional)). Add no
  dependency.
- [X] T002 Add `pub mod showcase;` to `crates/micold-client/src/lib.rs` and create
  `crates/micold-client/src/showcase/mod.rs` with the module's own docs and `mod`/`pub mod`
  declarations for `catalogue`, `state`, `samples`, `gallery` and `sections`.
- [X] T003 [P] Add a `[tasks.showcase]` entry to `mise.toml` running
  `cargo run -p micold-client --bin micold-showcase`, described as the component showcase
  (development only, never installed).
- [X] T004 Confirm the application is unbroken after `crates/micold-client/Cargo.toml`'s change:
  `cargo run -p micold-client` still starts the IDE and `cargo build --workspace` builds both
  binaries. This is T001's regression guard and the first half of FR-019.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: a launchable, empty gallery, a tested reducer, and the two gates that guard the
requirements with the worst failure modes.

**⚠️ CRITICAL**: no user story work can begin until T015 opens a window.

**Note on T006**: the packaging gate has no dependency on the gallery's content and lands here
deliberately rather than in polish — shipping a development tool to end users is the worst outcome
this feature can produce (spec, FR-018a clarification), so it is the first thing held rather than the
last.

### Tests (write first)

- [X] T005 Write failing reducer tests in `crates/micold-client/tests/showcase_state.rs`: a fresh
  `Showcase` starts in `Light` with every replay counter at zero, every run flag false and nothing
  open; `Replayed(i)` bumps only `i`; `Reversed(i)` flips only `i`; `Opened` replaces whatever was
  open; `Dismissed` clears it. **Red**: the module does not compile — `Showcase` does not exist yet.
- [X] T006 [P] Write `crates/micold-client/tests/packaging_excludes_showcase.rs` (FR-018a, SC-008)
  as a rule function over `(manifest_text, desktop_text, showcase_bin_name)` plus wrappers that read
  the real files. Asserts: neither file names the showcase binary or its path; both files exist; the
  `[package.metadata.deb] assets` list is present and non-empty. **Red (synthetic)**: two tests drive
  the rule against a manifest whose `assets` names `target/release/micold-showcase`, and against a
  desktop entry whose `Exec` names it, and require each to fail naming the offending file.
- [X] T007 [P] Write `crates/micold-client/tests/showcase_determinism.rs` (FR-022, SC-010): scans
  `crates/micold-client/src/showcase/` with the comment-stripping helper the existing gates use, and
  fails on `Instant::now`, `SystemTime`, `rand`, `uuid::new_v4`, `std::env::var`, `current_dir`,
  `home_dir`, `read_to_string`. **Red (synthetic)**: a companion test drives the rule against a source
  string containing `Instant::now()` and requires the failure, naming the line — a vacuity Red would
  not hold here, because T002 has already created `mod.rs` for the scan to find. Keep the vacuity
  guard as a second assertion.

### Implementation

- [X] T008 Implement `Showcase`, `Message`, `Floating` and `update` in
  `crates/micold-client/src/showcase/state.rs` per [data-model.md](./data-model.md) — render-free, no
  iced widget in scope. `open: Option<Floating>` so two floating surfaces cannot both be open
  (Principle V; the spec's deadlock Edge Case becomes unrepresentable). The three per-entry vectors are
  sized at boot from `catalogue::COMPONENTS.len()` — not fixed-length arrays, which would ask the
  compiler to resolve `COMPONENTS` and `Showcase` through each other. Greens T005.
- [X] T009 [P] Implement the catalogue's types in `crates/micold-client/src/showcase/catalogue.rs`:
  `Entry` (including the `interactive` flag), `MotionEntry`, `Exemption`, `Section`, `Layout`, with
  `render: for<'a> fn(&'a Showcase,
  Roles) -> Element<'a, Message>`, and the three `const` slices initially empty
  ([contracts/gallery-catalogue.md §1–§2](./contracts/gallery-catalogue.md)).
- [X] T010 [P] Implement `crates/micold-client/src/showcase/samples.rs`: fixed invented labels,
  `TreeItem` list, `ProjectRow` list, menu items, and a `GridCache` built by applying one hand-written
  `GridFrame`. All `const`/`static` or a pure no-argument function — no clock, randomness, environment
  or filesystem (FR-006, FR-022).
- [X] T011 Implement the page in `crates/micold-client/src/showcase/gallery.rs`: one
  `material::Scrollable` over a column of sections, each section a heading (`material::Text` with a
  `TypeRole`) plus its instances chunked into rows at a fixed count, with `Layout::FullWidth` entries
  on their own row, ending in one `cdk::overlay::Overlay` host
  ([research R7](./research.md#r7--floating-surfaces-reuse-the-applications-overlay-host-fr-007-edge-cases),
  [R9](./research.md#r9--laying-out-a-page-of-unequal-components-edge-cases)). The page never scrolls
  horizontally.
- [X] T012 Implement `crates/micold-client/src/showcase/main.rs`: `iced::application(boot, update,
  view)` with the Material Symbols font registered, `micold_client::ui::theme(scheme)` as the theme,
  and the Escape subscription. Thin glue only — no decision logic (Principle I's GUI-wiring
  exception). It must name none of `micold_core::{store,settings,endpoint,spawn,git}`,
  `micold_client::daemon` or `dark_light` (FR-017, FR-020;
  [contracts/showcase-launch.md §2](./contracts/showcase-launch.md)).
- [X] T013 Greens T007: run `cargo test -p micold-client --test showcase_determinism` and confirm the
  vacuity guard now passes and the scan is clean.
- [X] T014 Widen `crates/micold-client/tests/idle_requests_no_frames.rs` to scan
  `src/showcase/` alongside `src/ui/` (FR-023,
  [research R12](./research.md#r12--the-showcase-is-bound-by-the-frame-request-rule-fr-023)). **Red**:
  add the assertion that the scanned set includes a `showcase/` path *before* extending the walk, and
  observe it fail. Keep `SANCTIONED = "ui/cdk/motion.rs"` and the exactly-one-frame-request assertion.
- [X] T015 Widen `crates/micold-client/tests/material_boundary.rs` so `src/showcase/*.rs` is scanned
  as feature-module source at the existing zero budgets (FR-021,
  [research R13](./research.md#r13--the-showcase-is-bound-by-the-boundary-rule-fr-021-principle-viii)).
  **Red**: add the assertion that the scanned module set includes the showcase's files before
  extending the walk. Note the existing "unexpected directory `ui/<name>/`" assertion stays as-is —
  the showcase is deliberately *not* under `ui/`.
- [X] T054 Write `crates/micold-client/tests/showcase_glue.rs` (Principle I): assert that
  `src/showcase/gallery.rs` and `src/showcase/main.rs` hold **no decision logic** — no `match` on
  showcase state, no `if` on a `Showcase` field, and no arithmetic on one — so the GUI-wiring
  exception's precondition is checked rather than asserted. Iteration over the catalogue,
  `Option`/`if let` unwrapping, and `match` on a catalogue `Section`/`Layout` are permitted and listed
  as such. **Red (synthetic)**: drive the rule against a source string containing
  `match self.scheme {` and require the failure. Plus a vacuity guard that both files were read.
  *Numbered out of sequence deliberately* — it belongs to Phase 2 and runs alongside T014/T015;
  renumbering thirty-eight tasks to preserve a cosmetic ordering would churn every cross-reference in
  this file for no gain.
- [X] T016 Confirm `mise run showcase` (the `mise.toml` task from T003) opens a window rendering an
  empty gallery from `crates/micold-client/src/showcase/gallery.rs`, and that `mise run test` is green.

**Checkpoint**: the binary exists, the reducer is tested, the showcase is inside the boundary,
frame-request and no-decision-logic rules, and the packaging exclusion is gated.

---

## Phase 3: User Story 1 — Every component is visible without running the IDE (Priority: P1) 🎯 MVP

**Goal**: the whole library on one scrolling page, live and interactive, with no daemon, no
repository and no saved state.

**Independent Test**: on a machine with no configuration for this application and no git repository,
launch the showcase; it opens and renders the full catalogue, no session daemon was started, and no
state was written. Scroll the page and find every component under a heading naming it.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

- [X] T017 [P] [US1] Write `crates/micold-client/tests/showcase_isolation.rs` (FR-017, FR-020): scan
  `src/showcase/` for `micold_core::store`, `micold_core::settings`, `micold_core::endpoint`,
  `micold_core::spawn`, `micold_core::git`, `micold_client::daemon` and `dark_light`, comment-stripped,
  and fail naming any hit. **Red (synthetic)**: a companion test drives the rule against a source
  string that names one of them and requires the failure. Plus a vacuity guard that the scan found
  sources. This is the structural half of what quickstart §B1 checks by inspecting processes.

### Implementation for User Story 1

> These six section tasks each add entries to the **one** `catalogue.rs` list and a render function to
> their own file under `src/showcase/sections/`. They therefore serialise on `catalogue.rs` — that is
> the deliberate cost of having one place a developer can read the whole gallery
> ([contracts/gallery-catalogue.md §6](./contracts/gallery-catalogue.md)).

- [X] T018 [US1] Atoms section — `Text`, `Ellipsized`, `Glyph`, `Divider`, `Tag`, `ActivityBadge`:
  render functions in `crates/micold-client/src/showcase/sections/atoms.rs`, entries in
  `catalogue.rs`.
- [X] T019 [US1] Controls section — `Button`, `IconButton`, `Checkbox`, `ToggleChip`, `TextField`,
  `Select`, `FilterTrigger`, `ResizeHandle`: `src/showcase/sections/controls.rs` + entries.
- [X] T020 [US1] Surfaces and containers section — `material::Surface`, `Scrollable`, `Accordion`,
  `Toolbar`, `ConnectionBanner`, `StageProgress`, `TreeView`, `NavigationDrawer`:
  `src/showcase/sections/surfaces.rs` + entries.
- [X] T021 [US1] Floating section — `Modal`, `MenuTrigger`, `MenuOverlay`, `ContextMenu`,
  `ProjectSwitcherTrigger`, `ProjectSwitcherOverlay`, `Tooltip`: each openable from its own section and
  dismissible without leaving the page (FR-007), pushed onto the existing overlay host.
  `src/showcase/sections/floating.rs` + entries. **Also** add the `EXEMPTIONS` entries for the
  behaviour-layer host types that have no appearance of their own (`cdk/overlay.rs::Overlay`,
  `cdk/overlay.rs::Surface`), each with its reason (FR-015).
- [X] T022 [US1] Terminal section — `TerminalPane`, rendered from T010's fabricated `GridCache` so a
  component needing live session output is present rather than omitted (FR-006).
  `src/showcase/sections/terminal.rs` + entry, `Layout::FullWidth`.
- [X] T023 [US1] Motion section — `MOTION` entries for the four animation helpers (`fade`, `expand`,
  `scale`, `scrim`), each named on screen (FR-007c), plus `Section::Motion` entries for the six
  components whose appearance *is* an animation (`Fade`, `Expand`, `Scale`, `Scrim`, `ViewFade`,
  `HoverReveal`). `src/showcase/sections/motion.rs` + entries.
- [X] T024 [US1] Replay and reverse controls in `src/showcase/sections/motion.rs` and
  `gallery.rs`: **Replay** bumps the entry's generation counter, which reaches the wrapper as
  `.restart_on(key)`; **Reverse** flips `shown` so the exit is watchable (FR-007b). No timer, no
  subscription, no animation clock anywhere
  ([research R6](./research.md#r6--replay-and-run-controls-without-a-clock-fr-007b-fr-023a)).
- [X] T025 [US1] Run-control mechanism for a component whose appearance runs continuously (FR-023a):
  the trigger, the caption, and the `running` flag wired through. **Zero entries use it at delivery** —
  nothing in the library runs continuously yet (`StageProgress`'s fill is a fixed non-animated value).
  Record that in the code comment, so 018's indeterminate indicator plugs in without the catalogue
  changing shape.
- [X] T026 [US1] Confirm the narrow-window and oversized-component Edge Cases against
  `crates/micold-client/src/showcase/gallery.rs`'s chunked layout (quickstart §B5): resize very narrow,
  and check no instance is clipped out of view and the page never scrolls horizontally.
- [X] T027 [US1] Confirm US1's independent test per [quickstart.md](./quickstart.md) §B1's structural
  rows, on a clean environment: no configuration, no project,
  no repository — the page renders, `pgrep -f micold-daemon` finds nothing this launch started, no
  terminal session exists, and no state file or directory was created.

**Checkpoint**: every component in the library is on one page, live, with no setup. This is the MVP —
018's visual walkthrough is already cheaper here than it was before.

---

## Phase 4: User Story 2 — Every state a component can be put into is shown side by side (Priority: P2)

**Goal**: a component's posed states sit next to each other, and the page says which states are live
rather than posed.

**Independent Test**: pick any component with more than one variant; all its variants render on one
screen simultaneously and each is labelled. Move the pointer across the row — each responds; press and
hold — each responds more strongly.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [X] T028 [P] [US2] Write `crates/micold-client/tests/showcase_captions.rs` — the catalogue's own
  test, per [contracts/gallery-catalogue.md §4](./contracts/gallery-catalogue.md)'s two rules (FR-005).
  **Agreement**: an entry has a non-empty `live` if and only if it is `interactive`, so a state absent
  from the page is read as live rather than missing and a caption never promises a response that never
  comes. **Non-vacuity**: at least one entry is `interactive`, and the catalogue is non-empty.
  **Red**: fails against the catalogue as T018–T023 left it, with `interactive` set and `live` still
  empty.

### Implementation for User Story 2

- [X] T029 [US2] Populate `variants` for every entry whose module declares a `pub enum`, in
  `crates/micold-client/src/showcase/catalogue.rs`, and render one instance per variant:
  `button::Variant`, `text::TypeRole`, `activity_badge::BadgeEmphasis`, `surface::Kind`,
  `overlay::Anchor` (FR-003). Names must match the library's variants exactly.
- [X] T030 [US2] Populate `posed` and render the corresponding instances side by side wherever the
  component admits the state: enabled/disabled, selected/unselected, and any empty state (FR-003).
  Leave `density` empty on every entry — FR-003a is dormant until 018 introduces the axis.
- [X] T031 [US2] Render each section's caption from `live` in `gallery.rs`, naming hover, pressed and
  focus as exercised rather than posed (FR-005). Greens T028. Do **not** fake any of the three with a
  static approximation (FR-004).
- [X] T032 [US2] Run quickstart §B2 and record it: hover, press and tab through every interactive
  component in one scrolling pass. Anything with no hover, no pressed or no visible focus is a defect
  in the **component** — record it; do not paper over it in the gallery.

**Checkpoint**: confirming a hover and a pressed state across the whole library is one pass, which is
what feature 018's SC-002/SC-004 need.

---

## Phase 5: User Story 3 — Light and dark are comparable without a restart (Priority: P3)

**Goal**: switch scheme from inside the showcase and watch every component re-render.

**Independent Test**: note several components' appearance, activate the scheme control, confirm every
component re-rendered in the other scheme with no restart — including a section that was off screen
when you switched.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [X] T033 [US3] Extend `crates/micold-client/tests/showcase_state.rs` with a failing test for
  `SchemeToggled`: it flips `scheme` and changes nothing else. **Red**: the message does not exist yet.
  (Same file as T005, so not parallel with it.)

### Implementation for User Story 3

- [X] T034 [US3] Handle `SchemeToggled` in `crates/micold-client/src/showcase/state.rs`. Greens T033.
- [X] T035 [US3] Add the scheme control to `crates/micold-client/src/showcase/gallery.rs` (a
  `material::ToggleChip` or `material::Button` — a component, never a hand-styled control), and resolve
  `tokens::roles(scheme)` **per render** so every section re-renders including those off screen
  (FR-008, FR-009).
- [X] T036 [US3] Hand the window `micold_client::ui::theme(scheme)` from
  `crates/micold-client/src/showcase/main.rs` so the renderer's own theme follows the control. Confirm
  the showcase reads neither the OS preference nor the settings file (FR-009, FR-020) — the isolation
  gate (T017) already forbids `dark_light`.
- [X] T037 [US3] Run quickstart §B3 and record it, including the sharp row: any component whose
  colours differ from the application's in the same scheme is a defect in the **showcase**, never a
  licence to style the gallery's copy differently.

**Checkpoint**: a colour decision can be checked in both schemes seconds apart.

---

## Phase 6: User Story 4 — The gallery cannot fall out of date unnoticed (Priority: P4)

**Goal**: the build fails, naming the component, when the library and the gallery disagree in either
direction.

**Independent Test**: add a component to the library without adding it to the gallery — the build
fails naming it. Remove a component the gallery lists — the build fails naming the stale entry.

**Why last**: there must be a gallery before there is anything to hold complete. Nothing here changes
the page; it holds it.

### Tests for User Story 4 (MANDATORY — Constitution Principle I) ⚠️

- [X] T038 [US4] Extract the component scanner out of
  `crates/micold-client/tests/material_builder_api.rs` into a shared
  `crates/micold-client/tests/inventory/mod.rs`, and have the builder-API gate `mod inventory;` it
  instead of holding its own copy (FR-014). Behaviour-preserving: the builder gate's four tests must
  stay green, unchanged. A directory under `tests/` is not compiled as its own test binary, which is
  why this works — the same arrangement `tests/support/mod.rs` already uses.
- [X] T039 [US4] Key the shared inventory by **(module, component)** and collapse duplicates within a
  module ([research R2](./research.md#r2--one-definition-of-a-component-shared-by-both-gates-fr-014)):
  `material/surface.rs::Surface` and `cdk/overlay.rs::Surface` are different components, and
  `material/animation.rs` yields a `Fade` twice — the wrapper and a private widget-tree tag. Add unit
  tests in `tests/inventory/mod.rs`'s own `#[cfg(test)]` module asserting both cases, so the keying
  cannot regress into name-only matching.
- [X] T040 [P] [US4] Write `crates/micold-client/tests/showcase_completeness.rs` with rules **C1 and
  C2** written as functions over `(inventory, catalogue)`, plus the §5 demonstrations that drive them
  against synthetic sets: a component absent from a stub catalogue fails C1 naming it; an entry naming
  a component absent from a stub inventory fails C2 naming it (FR-011, FR-012, SC-002, SC-004).
  **Red**: the real-library wrappers fail until T046 closes the catalogue's gaps.

### Implementation for User Story 4

- [X] T041 [US4] Add enum-variant scanning to `tests/inventory/mod.rs` — every `pub enum`'s variant
  **names**, payloads stripped (`Notification(NoticeLevel)` → `Notification`) — and rules **C3/C4** to
  `showcase_completeness.rs`: every library variant name is named by some entry's `variants` from any
  module, and every name an entry lists still exists somewhere in the library (FR-013, SC-003).
  Attribution is library-wide, not per-module: `cdk/overlay.rs`'s `Anchor` belongs to a module whose
  every component is exempted, and `Anchor`'s variants are posed in the floating section. Include the
  synthetic-Red demonstration for each direction, and a unit test that a payload-carrying variant is
  matched by name.
- [X] T042 [US4] Add the motion category to `tests/inventory/mod.rs` — the `pub fn`s declared in
  `src/ui/material/animation.rs` — and rules **C5/C6**: every animation helper has exactly one
  `MotionEntry`, and every `MotionEntry` names one that still exists (FR-013a, SC-003a). Enumerated
  deliberately, not inherited from the component definition
  ([research R5](./research.md#r5--the-motion-category-and-the-one-thing-neither-category-reaches)).
- [X] T043 [US4] Add rules **C7/C8/C9** to `showcase_completeness.rs`: every exemption names a
  component that still exists and carries a non-blank reason (FR-015); no component is both listed and
  exempted, and keys are unique (FR-011/FR-015); a `Section::Motion` entry is a component the library
  implements as an animation and vice versa (FR-007a).
- [X] T044 [US4] Add the four vacuity guards **V1–V4** (FR-016,
  [contracts/completeness-check.md §3](./contracts/completeness-check.md)): at least 30 components
  found (38 today); both `material/surface.rs::Surface` and `cdk/overlay.rs::Overlay` present;
  `animation.rs` exists and yields at least one `pub fn`; `COMPONENTS` and `MOTION` non-empty. V1 is a
  floor that must never be tightened into a count someone has to edit.
- [X] T045 [US4] Document in `showcase_completeness.rs`'s module doc what the check deliberately does
  **not** reach — the three element-producing free functions (`menu_panel`, `glyph::icon`,
  `glyph::icon_colored`) that are neither a component nor an animation helper, and density, which is
  dormant until 018. A recorded limit, not a silent one.
- [X] T046 [US4] Green the check: add the entries and exemptions it names until it passes, and confirm
  every failure message names the thing that is missing rather than only the count.

**Checkpoint**: the gallery is now trustworthy enough to verify against — the property that makes
this feature worth having.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [X] T047 **(MANDATORY, not polish — FR-024, Principle VII)** Write
  `docs/development/component-showcase.md`: what the showcase is for, how to launch it, how to add a
  component to it (the five steps from
  [contracts/gallery-catalogue.md §7](./contracts/gallery-catalogue.md)), what each completeness
  failure means, and the fact that it is never installed. Do **not** extend `docs/user-guide/` —
  FR-024 forbids it, because the showcase is not a user-facing capability.
  Also record what the gallery deliberately does **not** show: overlay layer ordering. `Showcase::open`
  is an `Option`, so two floating surfaces can never be open at once — which is what makes the spec's
  deadlock edge case unrepresentable, and also means 017's "a dialog is above a menu because it is a
  dialog" is not visible here. It is covered by `tests/overlay_stacking.rs`; say so, so nobody
  concludes from the page that it is untested.
- [X] T048 [P] Link the new document from `docs/README.md`'s **Development** section, and add a
  pointer from the "Adding a component" steps in `docs/development/component-library.md` — which is
  where a developer adding a component actually looks, and therefore the only pointer that stops the
  gallery being the thing everyone forgets.
- [X] T049 [P] Update `.github/workflows/ci.yml`: add `test -f docs/development/component-showcase.md`
  to the `docs` job, and add a step to the `test` job (all three platforms) running
  `showcase_completeness`, `showcase_determinism`, `showcase_isolation`, `showcase_captions`,
  `showcase_state`, `showcase_glue`, `packaging_excludes_showcase`, `material_boundary`,
  `material_builder_api`, `idle_requests_no_frames`. **This step is the authoritative list** — no other
  artifact restates it or counts it. They open no window, so they run wherever the crate compiles
  ([research R14](./research.md#r14--what-cross-platform-means-here-and-where-the-gates-run)).
- [X] T050 Confirm SC-007 — the application is unaffected: `mise run test` green, and
  `crates/micold-client/tests/fixtures/style_snapshot.txt` **unchanged**. If the style snapshot fails,
  this feature changed an appearance, which FR-019 forbids; the fix is the change, never
  `UPDATE_STYLE_SNAPSHOT=1`.
- [X] T051 [P] Confirm no dependency was added: `git diff` on `Cargo.toml`, `crates/*/Cargo.toml` and
  `Cargo.lock` shows only the `[[bin]]`, `default-run` and version-independent changes
  ([research R16](./research.md#r16--no-new-dependency)).
- [X] T052 Confirm cross-platform compilation: `cargo build --workspace` builds both binaries on
  Linux, macOS and Windows in CI, and every gate in T049's step passes on all three. No per-platform
  appearance claim is made or required (spec, Assumptions).

  > Closed 2026-07-29 by CI on PR #50 and #51: `build + test` green on ubuntu-latest, macos-latest
  > and windows-latest, and all gates in the step executed on each. Verified by reading the Windows
  > job log rather than by trusting the green tick — the first attempt at this step *looked* like a
  > partial pass while the gates were in fact never running there: the step used `\` line
  > continuations, which Windows' default `pwsh` does not honour, so it died at parse time before
  > cargo started (fixed in `f630cea`). A platform check that never executed on the platform is the
  > exact failure this task exists to catch.
- [ ] T053 Run [quickstart.md](./quickstart.md) §B end to end and fill in the recorded tables: B1
  (SC-001 timing on a clean machine), B2 (SC-005 hover/press pass), B3 (SC-006 scheme comparison), B4
  (motion replay, then idle with nothing moving — SC-009), B5 (floating surfaces and the narrow
  window).

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies.
- **Foundational (Phase 2)**: depends on Phase 1. Blocks every user story — nothing can be rendered
  or scanned before `src/showcase/` exists and opens a window (T016).
- **US1 (Phase 3)**: depends on Phase 2. No dependency on US2–US4.
- **US2 (Phase 4)**: depends on US1's entries existing (it populates their states). This is the one
  real inter-story dependency, and it follows from the spec's own ordering — a component must be on
  screen before comparing its states means anything.
- **US3 (Phase 5)**: depends on Phase 2 only. Could be done before US2 if wanted; sequenced after it
  per the spec's priorities.
- **US4 (Phase 6)**: depends on US1 (and reads whatever US2 added). Deliberately last: it holds a
  gallery complete, so there must be one.
- **Polish (Phase 7)**: depends on all four stories. T047 is mandatory, not optional.

### Within Each User Story

- Tests are written and observed failing before implementation (Principle I), with each gate's Red
  shape stated in its task.
- The reducer before the view; the catalogue's types before its entries; entries before the check.
- The story is done when its tests pass and its walkthrough row is recorded.

### Parallel Opportunities

- T003 with T001/T002; T006 with T007; T009 with T010; T054 with T014/T015 (three different test
  files, all after T012 has written the glue).
- T017 (a new test file) with T018–T023.
- T040 with T041–T043 only if split by rule into separate commits — they share
  `showcase_completeness.rs`, so in practice they serialise.
- T048, T049 and T051 are three different files and run in parallel.

**Where parallelism is deliberately absent**: T018–T023 all add entries to the single
`catalogue.rs` list, so they serialise on it. That is the cost of having one place a developer can
read the whole gallery, and it is worth paying — the alternative is six lists and a question about
which one is authoritative.

---

## Parallel Example: Phase 2

```bash
# Two independent gate files, neither touching the other:
Task: "T006 Write tests/packaging_excludes_showcase.rs with its synthetic-Red demonstrations"
Task: "T007 Write tests/showcase_determinism.rs with its vacuity guard"

# Two independent source files, once T008 has defined Showcase:
Task: "T009 Implement the catalogue's types in src/showcase/catalogue.rs"
Task: "T010 Implement src/showcase/samples.rs"
```

---

## Implementation Strategy

### MVP first (User Story 1 only)

1. Phase 1 — the second binary exists and the first still runs (T004).
2. Phase 2 — an empty gallery opens, the reducer is tested, and the boundary, frame-request, glue,
   determinism and packaging gates hold.
3. Phase 3 — every component on one page.
4. **Stop and validate**: run US1's independent test (T027) on a clean environment.

At that point 018's visual walkthrough is already dramatically cheaper, which is the whole reason for
this feature's timing. Everything after refines it.

### Incremental delivery

1. Setup + Foundational → a launchable, gated, empty gallery.
2. US1 → the page (MVP).
3. US2 → posed states and captions.
4. US3 → the scheme control.
5. US4 → the completeness check, which is what makes the page trustworthy rather than merely useful.
6. Polish → the developer document (mandatory), CI's three-platform step, and the recorded walkthrough.

### Suggested commit boundaries

One commit per task, except T014/T015 and each gate's Red/Green pair, which are worth splitting so the
failing assertion is visible in history — for a gate, the commit that shows it failing is the only
evidence it can fail.

---

## Notes

- `[P]` = different files, no dependency on an incomplete task.
- `[USn]` maps a task to its user story for traceability.
- **Every gate has a stated Red shape.** A gate nobody has seen fail is the failure mode this whole
  feature exists to remove; do not write one without its synthetic or vacuity Red.
- FR-003a (density) stays dormant: `Entry::density` is empty on every entry, and 018 adds both the
  rows and the rule when it introduces the axis.
- FR-023a's run control ships with zero users; that is expected, and T025 records why in the code.
- Do not regenerate `tests/fixtures/style_snapshot.txt`. If it fails, this feature changed an
  appearance — which FR-019 forbids.
