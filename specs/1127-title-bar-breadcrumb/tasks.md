---

description: "Task list for the 44 px title bar with a breadcrumb and a jump-to field (#1127)"
---

# Tasks: One 44 px title bar with a breadcrumb and a jump-to field

**Input**: Design documents from `specs/1127-title-bar-breadcrumb/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/title-bar-ui.md,
quickstart.md

**Tests**: Constitution Principle I — every behaviour has a failing test before its implementation.
Each test task is written first and must fail (or not compile) before the task after it starts.

**Documentation**: Constitution Principle VII — each milestone updates the user guide for the bar it
ships (FR-018).

**Cross-platform**: Constitution Principle VI — the field's hint comes from
`keymap::switcher_chord_label()` (Cmd on macOS, Ctrl elsewhere); no platform code is added.

Paths are relative to the repository root. `client/` means `crates/micold-client/`, `lib/` means
`crates/micold-material/src/` (the generic component library, re-exported whole as
`client/src/ui/material`, #1262).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story this task belongs to (US1–US3)

---

## Phase 1: Setup — the bar's figures

**Purpose**: the anatomy figures every panel under the bar already reads (research R1, contract T1).

- [ ] T001 Change the expected `app_bar` figures in `crates/micold-core/tests/tokens_anatomy.rs`:
  `HEIGHT` 43, `DIVIDER` 1, `BOTTOM_EDGE` 44, `PADDING` 12, `ICON_TARGET` 40, NEW `GAP` 12, `MARK`
  24, `FIELD_HEIGHT` 32, `FIELD_MAX_WIDTH` 360, and the `anatomy::ALL` length 48 → 52 (fails: old
  figures, no new constants)
- [ ] T002 Set those figures in `crates/micold-core/src/tokens/anatomy.rs` `mod app_bar`, add `GAP`,
  `MARK`, `FIELD_HEIGHT`, `FIELD_MAX_WIDTH` to `anatomy::ALL`, and say in the module's doc comment
  that the figures are feature 1127's (contract T1), superseding 018 §7.1's height, padding, target
  and bottom edge; `LEADING_ICON_PADDING` stays for its other readers

---

## Phase 2: Foundational — the shared parts of the bar

**Purpose**: the library pieces US1 composes (research R2, R3, R9, R10, R11). **⚠️ CRITICAL**: the
bar cannot be rebuilt until this phase is complete.

- [ ] T003 [P] Move `plan()`, `Widths` and their unit tests out of `lib/status_bar.rs` into NEW
  `lib/give_way.rs` (`pub(crate)`; `GiveWay` stays public and re-exported for `StatusItem::give_way`),
  register the module in `lib/lib.rs`, and make `StatusBar` call it; the moved tests and
  `client/tests/gates/status_bar_fits.rs` stay green unchanged (R3, no behaviour change)
- [ ] T004 [P] Write failing unit tests in `lib/icon_button.rs` (`#[cfg(test)]`, `test_support`):
  `.selected(true)` draws the selected state layer also with no press (not the disabled look);
  `.keyboard(true)` makes a pressable button a `TakesTheKeyboard` stop whose Enter and Space publish
  its press, and a button without it or without a press is no Tab stop; and in `lib/menu.rs` that a
  `MenuTrigger` is a Tab stop with the tooltip it is given
- [ ] T005 Implement `IconButton::selected(bool)` and `IconButton::keyboard(bool)` in
  `lib/icon_button.rs` (the wrapper of `lib/keyboard_focus.rs`, as `Button` uses it), and turn
  keyboard reach on with a tooltip (`More`) in `MenuTrigger` in `lib/menu.rs` (R10, R11, FR-013)
- [ ] T006 [P] Write failing unit tests in NEW `lib/app_mark.rs`: `AppMark::new(handle, name, roles)`
  lays out an `app_bar::MARK` square, shows `name` in a tooltip on hover, publishes nothing when
  pressed and is not a Tab stop (FR-003, contract C1)
- [ ] T007 Implement `AppMark` in `lib/app_mark.rs` (an `iced::widget::image` of `MARK` side inside
  `Tooltip`, no press, no `TakesTheKeyboard`; it holds no Micold asset, #1262) and `pub mod` /
  `pub use AppMark` in `lib/lib.rs` (R9)
- [ ] T008 Write failing unit tests in `lib/toolbar.rs`: B1 `Toolbar::new(roles)` is `HEIGHT` + `DIVIDER`
  tall at rest and `HEIGHT` elevated, full width, `PADDING` at each end, `GAP` between children,
  children centred vertically; B2 order mark, leading, leading-fixed, actions; B4 at a narrow width
  the leading child shortens (planner `Shorten`) while the mark, leading-fixed items and actions
  keep their natural width, and no child leaves the bar or overlaps another; B6 the bar's children
  keep their bounds whatever is drawn over them
- [ ] T009 Reshape `Toolbar` in `lib/toolbar.rs`: `new(roles)` (no title),
  `.mark(e)`, `.leading(e)`, `.leading_fixed(e)`, `.action(e)`/`.actions(v)`, `.elevated(b)`; `From<Toolbar>`
  builds a measuring widget that lays fixed children at natural width and gives the leading child
  the rest through `give_way::plan` (R2); divider and elevation unchanged (FR-019)

---

## Phase 3: User Story 1 — One compact bar that says where I am (Priority: P1) 🎯 MVP

**Goal**: one 44 px bar: app mark, `project / place` breadcrumb, (Overview button until US3),
Settings and the overflow trigger; the switcher panel opens under the project segment.

**Independent Test**: quickstart Part B 1–3, 5, 6 (field steps excepted); spec US1 *Independent
Test*.

### Part A — the bar's shape (US1 s1 without the place segment and field, s7, s8)

#### Tests for Part A ⚠️

- [ ] T010 [US1] Write failing render-free tests in `client/src/ui/toolbar.rs` for `settings_press(state)`:
  Settings closed → `Some(Message::Settings(SettingsMsg::Opened))`; Settings open → `None`
  (FR-011, R10)
- [ ] T011 [P] [US1] Change `client/tests/switcher_anchor.rs`: a reported corner `(300, 42)` puts the
  panel at `x` 300 and `y` `anatomy::app_bar::BOTTOM_EDGE`, whatever `y` was reported (R8, FR-005;
  fails: the view reads the reported `y`)
- [ ] T012 [P] [US1] Write NEW `client/tests/title_bar_keyboard.rs`: the bar's focusable controls in
  tree order are the project switcher, the Overview button, Settings, the overflow trigger; Enter on
  Settings publishes `Settings(Opened)`; the app mark is not focusable and publishes nothing when
  pressed (FR-003, FR-013, contract C1, C4, C6, C7)
- [ ] T013 [P] [US1] Write NEW geometry gate `client/tests/gates/title_bar_fits.rs` and its `#[path]`
  `mod` in `client/tests/layout_snapshot.rs`: at 640 and 1280 dp wide, light and dark, the bar's
  bottom edge is 44, every child lies inside its 43 dp height, no two children overlap, and the
  mark, project switcher, Overview button with its badge, Settings and overflow trigger are all
  drawn — with a 60-character project name and an unread mark (FR-001, FR-010, SC-001, SC-003,
  contract B5)
- [ ] T014 [US1] Add covered states `title-bar-session`, `title-bar-overview`,
  `title-bar-overview-full-screen`, `title-bar-changes`, `title-bar-settings`, `title-bar-no-project`,
  `title-bar-menu-open` at the end of `client/tests/support/covered_states.rs`, and move the
  existing `app_bar.*` component paths (`app_bar.overflow_trigger`, `app_bar.switcher_trigger`) to
  the new child indices (R13)
- [ ] T015 [US1] Rewrite `app_bar_actions` in `client/tests/gates/sibling_parity.rs` for the new
  children (no title, no `Fill` spacer: the trailing actions after the mark and leading children),
  keeping its icon-only shape check against `anatomy::app_bar::ICON_TARGET` (R13)

#### Implementation for Part A

- [ ] T016 [US1] Rename `ICON_RGBA` in `client/src/shell/startup.rs` to `pub(crate) APP_ICON_RGBA`
  (startup keeps reading it) (R9)
- [ ] T017 [US1] Rebuild `view()` in `client/src/ui/toolbar.rs`: `Toolbar::new(r)`
  `.mark(AppMark::new(APP_MARK.clone(), APP_NAME, r))` from one
  `static APP_MARK: LazyLock<image::Handle>` built from `APP_ICON_RGBA` (64 × 64);
  `.leading(today's switcher Button in its ContextArea)`; `.leading_fixed(overview_button(state, r))`
  (unchanged, FR-015); `.action(` a `Tooltip` "Settings" over `IconButton::new(Icon::Settings, r)`
  `.selected(settings_open).keyboard(true).on_press_maybe(settings_press(state)))`; `.action(menu)`;
  `.elevated(state.app_bar_elevated())`; add `settings_press()`; no title text, no usage
  (FR-002, FR-011, FR-012)
- [ ] T018 [US1] Hang the switcher panel from `(anchor.x, anatomy::app_bar::BOTTOM_EDGE)` in
  `client/src/ui/mod.rs`'s `hung_from`, ignoring the reported `y`, before
  `features::project::clamp_menu_anchor` (R8)
- [ ] T019 [US1] Build `Toolbar::new(LABEL…)` callers in the new shape: the content-placement test in
  `client/src/ui/material/content_placement.rs` and the `Toolbar` entry in
  `client/src/showcase/sections/surfaces.rs` (mark, leading, actions), plus a NEW `AppMark` showcase
  entry; `client/tests/showcase_completeness.rs` and `client/tests/material_builder_api.rs` green
- [ ] T020 [US1] Regenerate `client/tests/fixtures/layout_snapshot.txt`
  (`UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client --test layout_snapshot`), check every body
  moved up 21 dp and the bar's children are in B2 order, and list the new `title-bar-*` states in
  `docs/development/layout-snapshot.md`
- [ ] T021 [US1] Update the user guide for Part A: `docs/user-guide/appearance-theming.md` (the
  44 px title bar: app mark, project switcher, Overview button, Settings, overflow menu; no title
  text), `docs/user-guide/settings.md` (the Settings button in the bar, drawn selected while open),
  `docs/user-guide/icons.md` (Settings gear in the title bar; the app mark) (FR-018)

### Part B — the breadcrumb (US1 s1–s6)

#### Tests for Part B ⚠️

- [ ] T023 [P] [US1] Write failing unit tests in `lib/button.rs` for `Button::trailing(Icon)`: the
  glyph is drawn after the label at the leading slot's size, in both orders with `.trailing_mark`
  (R4)
- [ ] T024 [P] [US1] Write failing unit tests in NEW `lib/breadcrumb.rs`: `Breadcrumb::new(label, roles)`
  draws the project segment as a pressable text button with a trailing drop-down glyph; `.place(text,
  tooltip)` adds a `/` separator and a non-pressable place label, and without it neither is drawn;
  `.trailing_mark(count, tooltip)` draws the unread mark; `.open(true)` draws the segment selected;
  the project segment always carries its tooltip (`Switch project`, contract C2);
  narrowed, the place shortens with `…` before the project segment (to a floor of one character and
  `…`), the glyph and mark never shorten, and a shortened segment's tooltip holds its full text
  (FR-004, FR-006, FR-007, contract C2, C3)
- [ ] T025 [US1] Write failing render-free tests in `client/src/ui/toolbar.rs` for
  `model(state) -> TitleBarModel` over data-model *Place*: the project segment's tooltip is
  `Switch project`, with the full name on a second line when the name is shortened (FR-013, C2); no project → project `Select project`,
  place `None`; project, no location → place `None`; a session on `fix/1117-x` → place short branch
  `fix/1117` with the full branch as tooltip; the Default location without a branch → its display
  name; Overview (normal and full screen) → `Overview`; Settings → `Settings`; Changes → its entry's
  location; Compare → the current session's location; a location removed from the state → `None`;
  `unread` = `other_projects_unread()`, `switcher_open` follows the switcher (FR-004, FR-006, SC-005)
- [ ] T026 [P] [US1] Add to `client/tests/switcher_anchor.rs`: opened with no reported corner
  (keyboard), the panel hangs at `x` = `PADDING + MARK + GAP`, `y` = `BOTTOM_EDGE`; and add a covered
  state `title-bar-switcher-open` to `client/tests/support/covered_states.rs` so
  `client/tests/gates/panel_placement.rs` checks it (FR-005, SC-004, contract P1)
- [ ] T027 [P] [US1] Add breadcrumb pairs to `client/src/ui/material/composition_contrast.rs`: project
  segment in `on_surface`, separator and place in `on_surface_variant`, each ≥ 4.5:1 on the bar's
  surface at rest and elevated, light and dark (FR-017, contract T3)

#### Implementation for Part B

- [ ] T022 [P] [US1] Make `context_header::branch_chip` `pub` in `crates/micold-core/src/context_header.rs`
  with a doc comment naming its second reader (the title bar's place segment); its existing tests
  stay green (R7)
- [ ] T028 [US1] Implement `Button::trailing(Icon)` in `lib/button.rs` (R4)
- [ ] T029 [US1] Implement `Breadcrumb` in `lib/breadcrumb.rs` over `Button` (`.trailing`,
  `.trailing_mark`, `.selected`) and `Ellipsized`, reporting its two segments to the toolbar's
  planner as `Shorten(0)` (place) and `Shorten(1)` (project); `pub mod` / `pub use` in `lib/lib.rs`
  (R4)
- [ ] T030 [US1] Implement `TitleBarModel`, `Segment` and `model()` in `client/src/ui/toolbar.rs` per
  data-model (place text from `ui::overview::location_card` and `context_header::branch_chip`) (R7)
- [ ] T031 [US1] Replace the switcher `Button` in `view()` (`client/src/ui/toolbar.rs`) with the
  `Breadcrumb` built from `model()`, still in its `ContextArea::on_primary_bounds`, and give the
  switcher's `hung_from` in `client/src/ui/mod.rs` the keyboard fallback `x` (R8)
- [ ] T032 [US1] Extend `client/tests/gates/title_bar_fits.rs` with a 60-character branch (place
  shortens before project, both keep `…`), add the `title-bar-*` states to
  `client/tests/layout_text_overflow.rs` if it lists states, add a `Breadcrumb` showcase entry in
  `client/src/showcase/sections/surfaces.rs`, update the `app_bar.*` component paths in
  `client/tests/support/covered_states.rs` for the breadcrumb's children, and regenerate `client/tests/fixtures/layout_snapshot.txt`
- [ ] T033 [US1] Update the user guide for Part B: `docs/user-guide/project-selection.md`
  (§Switching projects from the top bar: the breadcrumb's project segment, where the panel opens,
  the place segment), `docs/user-guide/worktrees-and-sessions.md` (the "Open another project (app
  bar)" sentence), `docs/user-guide/overview.md` (the Overview button after the breadcrumb),
  `docs/user-guide/appearance-theming.md` (the breadcrumb) (FR-018)

**Checkpoint**: the 44 px bar with its breadcrumb works in every body branch.

---

## Phase 4: User Story 2 — Jump anywhere from a field in the bar (Priority: P2)

**Goal**: a field centred in the bar's free space opens the quick switcher, and gives way first.

**Independent Test**: quickstart Part B 4, 6, 7; spec US2 *Independent Test*.

### Tests for User Story 2 ⚠️

- [ ] T034 [P] [US2] Write failing unit tests in NEW `lib/field_button.rs`:
  `FieldButton::new(Icon::Search, label, roles).hint(text).max_width(w).on_press_maybe(m).id(id)` is
  a `FIELD_HEIGHT` pill; its three widths (`Full`: glyph, label, hint; `GlyphAndHint`; `Glyph`: an
  `ICON_TARGET` square) are reported; at `Full` it never grows past `max_width`; with `None` it draws
  content at the shared 38% disabled treatment and is no Tab stop; with a press, Enter and Space
  publish it (FR-008, FR-009, FR-013, data-model *Fit*)
- [ ] T035 [P] [US2] Write failing unit tests in `lib/toolbar.rs` for `.centre(e)`: B3 the centre
  child has equal free space either side between the leading and trailing groups (not the window's
  centre line, also with a long breadcrumb); B4 the five-step order of data-model *Give-way order* —
  field spare width, field label `Drop(0)`, field hint `Drop(1)`, place `Shorten(0)`, project
  `Shorten(1)` — each only as far as needed
- [ ] T036 [P] [US2] Write failing render-free tests in `client/src/ui/toolbar.rs` for
  `jump_press(state)`: Settings open → `None`; session view, Overview, full screen, no project →
  `Some(QuickSwitch(Toggled { terminal_had_focus: state.terminal_focused() }))` (FR-009, US2 s4, R6)
- [ ] T037 [P] [US2] Write a failing test in `client/src/main_tests.rs` through `update`: the field's
  message opens the quick switcher and the returned task focuses `QUICK_SWITCHER_FIELD_ID`, the same
  as the chord's (FR-009, US2 s1)
- [ ] T038 [P] [US2] Add a case to `client/tests/overlay_dismissal_rules.rs`: with the quick switcher
  open, a press at the field's bounds yields `QuickSwitch(Dismissed)` and opens no second switcher
  (US2 s3, contract P3)
- [ ] T039 [P] [US2] Extend `client/tests/title_bar_keyboard.rs`: the field is a Tab stop between the
  Overview button and Settings and Enter on it publishes `jump_press`'s message; with Settings open
  it is skipped (FR-013, contract C5)
- [ ] T040 [P] [US2] Add field pairs to `client/src/ui/material/composition_contrast.rs`: label and
  hint text in `on_surface_variant` ≥ 4.5:1 and the pill outline and key-cap outline in the outline
  role ≥ 3:1 on the field's and the bar's surface, light and dark (FR-017, contract T2)

### Implementation for User Story 2

- [ ] T041 [US2] Implement `FieldButton` in `lib/field_button.rs` (`surface_container_highest` pill,
  `shape::FULL`, outline role per T2, outlined key-cap hint, `TakesTheKeyboard` when pressable,
  `Fit` decided at layout); `pub mod` / `pub use` in `lib/lib.rs` (R5)
- [ ] T042 [US2] Implement `Toolbar::centre(e)` in `lib/toolbar.rs`: the centre child gets the free
  space up to its maximum, centred in it, and yields through `give_way::plan` per T035 (R2, R3)
- [ ] T043 [US2] Add `jump_press()` and `TitleBarModel::jump` in `client/src/ui/toolbar.rs`, and
  `.centre(FieldButton::new(Icon::Search, "Jump to location, session or project", r)
  .hint(keymap::switcher_chord_label()).max_width(app_bar::FIELD_MAX_WIDTH).on_press_maybe(model.jump))`
  in a `Tooltip` "Jump to location, session or project (<chord>)" in `view()` (FR-008, FR-009,
  contract C5)
- [ ] T044 [US2] Extend `client/tests/gates/title_bar_fits.rs` (at 640 dp the field shows at least its
  glyph; at 1280 dp it is centred in the free space), add covered state `title-bar-settings-field`
  if `title-bar-settings` does not already show the disabled field, add a `FieldButton` showcase
  entry in `client/src/showcase/sections/surfaces.rs`, update the `app_bar.*` component paths in
  `client/tests/support/covered_states.rs` behind the new centre child, and regenerate
  `client/tests/fixtures/layout_snapshot.txt`
- [ ] T045 [US2] Update the user guide: `docs/user-guide/worktrees-and-sessions.md` (the quick
  switcher opens from the title bar's field too), `docs/user-guide/appearance-theming.md` (the field
  and what gives way first in a narrow window), `docs/user-guide/icons.md` (Search on the field)
  (FR-018)

**Checkpoint**: the field opens the quick switcher by pointer and keyboard, and yields first.

---

## Phase 5: User Story 3 — The Overview leaves the bar for the sidebar (Priority: P3)

**Goal**: no Overview button in the bar once #1125's Overview row is on `main` (R12).

**Independent Test**: spec US3 *Independent Test*; quickstart Part B 8.

**Starts only after #1125 M3 ("Overview as the sidebar's first row") is merged.**

### Tests for User Story 3 ⚠️

- [ ] T046 [US3] Replace the Overview-button tests in `client/src/ui/toolbar.rs` with a failing test
  that no element of the bar publishes an `Overview` message in any body branch (Overview normal and
  full screen included), and drop the Overview button from the expected Tab order in
  `client/tests/title_bar_keyboard.rs` (FR-016, contract O1)
- [ ] T047 [P] [US3] Make sure `client/src/ui/app_chord_tests.rs` covers the Overview chord opening and
  closing the Overview from the session view (terminal focused), from Changes and in full screen;
  and that #1125's sidebar Overview-row tests press the row from the session view and from Changes;
  add the missing cases (FR-016, contract O2, SC-006)

### Implementation for User Story 3

- [ ] T048 [US3] Remove `overview_button`, `badge_look`, `overview_press` (and `overview_tooltip` if
  #1125 left it) from `client/src/ui/toolbar.rs`; remove `Toolbar::leading_fixed` from
  `lib/toolbar.rs` with its tests unless the showcase or another caller uses it (contract O3)
- [ ] T049 [US3] Drop the Overview button from `client/tests/gates/title_bar_fits.rs`, update the
  `app_bar.*` component paths in `client/tests/support/covered_states.rs`, and regenerate
  `client/tests/fixtures/layout_snapshot.txt`
- [ ] T050 [US3] Update the user guide: `docs/user-guide/overview.md` (opened from the sidebar's
  Overview row and Ctrl+Shift+O / Cmd+Shift+O only), `docs/user-guide/icons.md` (the *Dashboard
  (grid)* row loses "Top bar"), `docs/user-guide/appearance-theming.md` (no Overview button)
  (FR-018)

**Checkpoint**: the Overview opens only from the sidebar row and its chord.

---

## Phase 6: Polish

- [ ] T051 Run quickstart Part B through the `visual-pass` skill and save the screenshots under
  `specs/1127-title-bar-breadcrumb/visual/` (light and dark, 640 and 1280 dp, every body branch)

---

## Dependencies & Execution Order

- Setup (T001–T002) → Foundational (T003–T009) → US1 Part A (T010–T021) → US1 Part B (T022–T033).
- US2 (T034–T045) depends on US1 (it fills the bar's centre slot and extends its gate and keyboard
  test). US3 (T046–T050) runs last: after US2 (it edits the keyboard test and gate US2 extends) and after
  #1125 M3 is merged.
- Within a part: tests first, then implementation, then the fixture regeneration, then docs.
- Every milestone appends covered states and regenerates the fixture; a parallel PR (#1125) that
  merges first means rebase and regenerate, never hand-merge (plan *Overlap*).
- Polish (T051) after every story.

## Parallel Opportunities

- T003, T004, T006 together; T008 after T005 and T007 (it builds the bar from them).
- Part A: T011, T012, T013 together after T010. Part B: T022, T023, T024, T026, T027 together.
- US2: T034–T040 together.

## Implementation Strategy

MVP is US1 Part A: one 44 px bar with the app mark and Settings, every body 21 dp taller. Part B
turns the switcher button into the breadcrumb. US2 adds the field. US3 waits on #1125 and only
removes code. Each increment regenerates the layout fixture itself.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — The 44 px title bar 🎯 MVP

- **Tasks**: T001–T021
- **Deliverable**: in every body branch the top bar is 44 px (43 + 1 divider) and shows the app mark
  (tooltip `Micold`), the project switcher, the Overview button, a Settings button (selected while
  Settings is open) and the overflow trigger; the switcher panel hangs at the bar's bottom edge;
  every body starts 21 dp higher
- **Satisfies**: US1 acceptance scenarios 1 (without the place segment and the field), 7, 8;
  FR-001, FR-002 (without field), FR-003, FR-005 (`y`), FR-011, FR-012, FR-013 (C1, C4, C6, C7),
  FR-014, FR-015, FR-018 (Part A pages), FR-019; SC-001, SC-002, SC-003 (without the field)
- **Verify**: `cargo test -p micold-core --test tokens_anatomy`; `cargo test -p micold-material`;
  `cargo test -p micold-client --lib -- toolbar`; `cargo test -p micold-client --test layout_snapshot
  --test switcher_anchor --test title_bar_keyboard --test toolbar --test app_bar_scroll --test
  showcase_completeness --test material_builder_api --test anatomy_call_sites --test
  idle_requests_no_frames --test keymap`; quickstart §B 1, 5, 7 for their M1 parts only (mark,
  project switcher, Overview button, Settings, overflow trigger; 44 px height; dark scheme), since the
  place segment and the field arrive in M2 and M3
- **Depends on**: —
- **Tier**: full

US1 has 24 tasks with Setup and Foundational, so it is split along its acceptance scenarios: M1 ships
the bar's shape (s1 partly, s7, s8), M2 the breadcrumb (s1–s6). Setup and Foundational cannot be cut
further: the 43 dp figure forces the reshaped `Toolbar` and the 40 dp targets in the same change.
M1's 21 tasks fit one session (Task-size gate): nine are test-first edits to existing test files or
covered-state lists, two are fixture regeneration and docs, and the code tasks touch one figure
module, three small `micold-material` components and `ui/toolbar.rs`. Splitting off Setup and
Foundational would leave a milestone with nothing to observe (milestones rule 1).

### M2 — The `project / place` breadcrumb

- **Tasks**: T022–T033
- **Deliverable**: the bar reads `micold-ai-ide ▾ / fix/1117` (or `Overview`, `Settings`, the
  project alone, `Select project`), long names shorten place-first with `…` and a full-text tooltip,
  and pressing the project segment by pointer or keyboard opens the switcher under its leading edge
- **Satisfies**: US1 acceptance scenarios 1–6; FR-004, FR-005, FR-006, FR-007, FR-013 (C2), FR-017 (breadcrumb),
  FR-018 (Part B pages); SC-004, SC-005
- **Verify**: `cargo test -p micold-core`; `cargo test -p micold-material -- button breadcrumb`;
  `cargo test -p micold-client --lib -- toolbar composition_contrast`; `cargo test -p micold-client
  --test switcher_anchor --test layout_snapshot --test layout_text_overflow --test
  showcase_completeness`; quickstart §B 1–3, 6
- **Depends on**: M1
- **Tier**: full

### M3 — The jump-to field

- **Tasks**: T034–T045
- **Deliverable**: a field centred in the bar's free space reads "Jump to location, session or
  project" with `Ctrl+Shift+P` (`Cmd+Shift+P` on macOS); pressing it or Enter on it opens the quick
  switcher with its input focused; it is disabled in Settings, closes an open switcher, and drops its
  text then its hint as the window narrows
- **Satisfies**: US2 acceptance scenarios 1–5; FR-008, FR-009, FR-010, FR-013 (C5), FR-017 (field),
  FR-018 (US2 pages); SC-003
- **Verify**: `cargo test -p micold-material -- field_button toolbar`; `cargo test -p micold-client
  --lib -- toolbar composition_contrast`; `cargo test -p micold-client --bin micold-ai-ide --
  main_tests`; `cargo test -p micold-client --test overlay_dismissal_rules --test title_bar_keyboard
  --test layout_snapshot --test layout_text_overflow`; quickstart §B 4, 6, 7
- **Depends on**: M2
- **Tier**: full

### M4 — The Overview button leaves the bar

- **Tasks**: T046–T050
- **Deliverable**: the bar has no Overview button or badge in any body branch; the Overview opens
  and closes from the sidebar's Overview row and from Ctrl+Shift+O, also in full screen
- **Satisfies**: US3 acceptance scenarios 1–4; FR-016, FR-018 (no Overview button described);
  SC-006
- **Verify**: `cargo test -p micold-client --lib -- toolbar app_chord`; `cargo test -p micold-client
  --test title_bar_keyboard --test keymap --test layout_snapshot`; quickstart §B 8
- **Depends on**: M3 (it edits the keyboard test and gate M2 and M3 extend) and #1125 M3 merged to
  `main` (R12)
- **Tier**: light

T051 (visual pass, no code) is left to the close unit (milestones rule 4); each milestone's own
visual pass runs at its gate.
