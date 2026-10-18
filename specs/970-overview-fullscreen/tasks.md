---

description: "Tasks: Overview full-screen mode"
---

# Tasks: Overview full-screen mode

**Input**: Design documents from `/specs/970-overview-fullscreen/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/ui.md, quickstart.md

**Tests**: Constitution Principle I: every test task below is written first and seen failing (red)
for the stated reason before the implementation task it covers. Each story's red run is its own
task. Tests that pin behaviour a story gets structurally from an earlier story's code (US4) are
written with that story's tests and turn green with its implementation.

**Documentation**: Principle VII: each milestone's user-guide task ships in the same `feat` PR as
its behaviour (FR-013).

**Cross-platform**: Principle VI: widths are iced logical pixels; the chord label follows the
existing `cfg!(target_os = "macos")` pattern of `keymap::overview_chord_label`; CI runs every test
below on Linux, macOS and Windows.

**Organization**: grouped by user story, in priority order with US4 (P1) after US1 because it has
no code of its own: it is proved over US1's code. Paths are relative to the repository root;
`client/` stands for `crates/micold-client/`. Run client tests with
`cargo test -p micold-client <filter>` (the gate: `mise run gate`, in the cloud container
`scripts/gate.sh`). Find each symbol with the `LSP` tool (`findReferences` before changing a
signature).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3, US4)

---

## Phase 1: Setup (Shared Infrastructure)

None: no new crate, module, dependency or fixture.

---

## Phase 2: Foundational (Blocking Prerequisites)

None: every change belongs to the first story that needs it.

---

## Phase 3: User Story 1 - Give the Overview the whole window (Priority: P1) 🎯 MVP

**Goal**: While the Overview is open, a header button and Ctrl/Cmd+Shift+F switch it to full-screen
mode: the sidebar (expanded or collapsed strip, with its resize handle) slides off at the left edge
on the sidebar's own 400 ms `EMPHASIZED` curve and the Overview, the drawer's `Fill` sibling, grows
to the window's full width; every exit from the Overview leaves the mode.

**Independent Test**: `cargo test -p micold-client navigation_drawer overview keymap icons`; then
quickstart §B 1–2 and 5 (the cards follow feature 1024's rest rule at each width; the smooth card
reflow is US2).

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

> Write these first and run them (T007): they must FAIL, first to compile, then (after stubs) on
> behaviour.

- [ ] T001 [US1] Mode state tests in `crates/micold-client/tests/features_overview.rs`, reusing its `toggled(settings_open)` helper: `full_screen_toggles_only_while_open` (open, `Msg::FullScreenToggled { settings_open: false }` → `state.full_screen()` true; again → false; while closed → stays false; with `settings_open: true` → unchanged; while an activation is pending (`state.request(..)` accepted) → unchanged); `opening_the_overview_clears_full_screen` (open, toggle on, `Msg::Closed`, `Msg::Toggled` again → `full_screen()` false: the field is cleared on open, not only hidden); `every_exit_leaves_full_screen` (from full screen: `Msg::Closed`, `Msg::Toggled` that closes, `Msg::CardActivated` for both `Activation::OfferStart` and `Activation::RevealOnly`, and setting `state.open = false` directly as `app.rs` does for `MainAreaTaken` → `full_screen()` false each time); `a_project_switch_keeps_full_screen` (`Msg::ProjectSwitched` → still true). Data-model rule: "`full_screen()` … `open && full_screen`; the only reader" (FR-001, FR-010, FR-011, SC-005)
- [ ] T002 [P] [US1] App-level tests in `crates/micold-client/tests/overview.rs` driving `state.update(Message::Overview(..))` on the app `State` (as the test at its line ~678 does): `settings_open_and_closed_keeps_full_screen` (enter full screen, open Settings, close it → `state.overview.full_screen()` still true); `the_mode_writes_no_sidebar_state` (enter, leave by toggle → `state.sidebar.hidden` and `state.sidebar.width` equal their values before) (FR-009, FR-011)
- [ ] T003 [P] [US1] Chord tests in the `mod tests` of `crates/micold-client/src/keymap.rs`, beside the `is_overview_chord` tests: `is_overview_full_screen_chord` is true for Ctrl+Shift+F off macOS and Cmd+Shift+F on macOS (the platform's primary modifier, as `is_primary_shift_chord` decides), false for F, Shift+F, primary+F, primary+Shift+G and primary+Alt+Shift+F; `overview_full_screen_chord_label()` is "Ctrl+Shift+F" off macOS and "Cmd+Shift+F" on macOS. In `crates/micold-client/src/ui/app_chord_tests.rs`: Ctrl+Shift+F (Cmd on macOS) maps to `Message::Overview(Msg::FullScreenToggled { settings_open: false })` (FR-001, FR-014)
- [ ] T004 [P] [US1] Icon locks in `crates/micold-client/tests/icons.rs`: add `Icon::FullScreen` → U+E5D0 and `Icon::FullScreenExit` → U+E5D1 to `expected`, and extend `the_overview_icons_are_pinned_and_distinct_from_every_other_icon` (or a sibling test) so both are pinned and distinct from every other variant; `all_covers_every_variant_without_duplicates` covers them (FR-012)
- [ ] T005 [US1] Drawer geometry gates in the `mod tests` of `crates/micold-client/src/ui/material/navigation_drawer.rs`, reusing its `drawer()` helper and the layout helpers its existing tests use: `a_retracted_drawer_settles_at_zero_width_with_every_child_parked` (built shown, then `.retracted(true)` and frames past 400 ms, from `.open(true)` with a handle and from `.open(false)`: node width 0, panel, rail and handle all `parked`); `a_drawer_built_retracted_starts_at_zero_width` (first layout already 0 wide, no frames needed: FR-011's "without a zoom", like `a_drawer_starts_at_its_destination_rather_than_animating_to_it`); `a_retracting_drawer_narrows_on_the_emphasized_curve` (sampled every 16 ms: width = full × (1 − `EMPHASIZED`(t / 400 ms)) within 0.5 px, monotone non-increasing, never negative, the shown child's x = width − full (translated left), settled by 500 ms; the reverse when un-retracted); `a_retraction_reversed_midway_continues_from_its_width` (flip at 200 ms: the next frame's width is within one frame's motion of the last, no jump to either end; every exit from the Overview only flips `retracted` through `full_screen()`, so this is also the drawer side of spec Edge *Leaving the Overview while a zoom in is still running*); `leaving_retraction_shows_the_drawer_as_given` (retracted from open at a panel width then shown again settles at panel + handle width; from closed at the rail's width) (FR-002, FR-005, FR-007, FR-009, SC-002; contracts/ui.md *Geometry gates — Drawer*, all but `.instant`)
- [ ] T006 [P] [US1] Header text tests in a `mod tests` of `crates/micold-client/src/ui/overview.rs`, the pattern of `overview_tooltip`'s tests in `crates/micold-client/src/ui/toolbar.rs`: `full_screen_label(false)` = "Full screen", `full_screen_label(true)` = "Exit full screen"; `full_screen_tooltip(on)` = `"{label} ({overview_full_screen_chord_label()})"` for both values; `full_screen_icon(false)` = `Icon::FullScreen`, `full_screen_icon(true)` = `Icon::FullScreenExit` (FR-012, contracts/ui.md *Overview header*)
- [ ] T007 [US1] Run `cargo test -p micold-client --no-run` and record the first red reason (no `full_screen`, `FullScreenToggled`, `is_overview_full_screen_chord`, `Icon::FullScreen`, `retracted`, `full_screen_label`); after T008–T011 add signatures with stub bodies (toggle ignored, chord false, drawer ignores `retracted`, labels empty), run `cargo test -p micold-client navigation_drawer overview keymap icons app_chord` and record the behavioural red of T001–T006 and T016–T017 (US4's tests, written in this red phase)

### Implementation for User Story 1

- [ ] T008 [US1] Mode state in `crates/micold-client/src/features/overview.rs` (plan D1, data-model.md): `State.full_screen: bool` with a doc comment ("read only through `full_screen()`"), `pub fn full_screen(&self) -> bool { self.open && self.full_screen }`, `Msg::FullScreenToggled { settings_open: bool }` that flips the field only while `open`, `!settings_open` and no activation pending; `Msg::Toggled` that opens sets `full_screen = false`. Add the arm to the pending-activation early return so a toggle during the delay is ignored ("the first one stands", feature 971). No `Settings` field: the mode is not persisted (FR-001)
- [ ] T009 [P] [US1] Chord in `crates/micold-client/src/keymap.rs`: `pub fn is_overview_full_screen_chord(key: &Key, mods: Mods) -> bool` on `is_primary_shift_chord(.., 'f')` and `pub fn overview_full_screen_chord_label() -> &'static str` beside `overview_chord_label`; not added to the terminal's reserved tier (research R7). In `crates/micold-client/src/ui/mod.rs` `app_chord`, after the `is_overview_chord` arm, emit `Message::Overview(overview::Msg::FullScreenToggled { settings_open: false })` (the update ignores it while closed; Settings open is filled where the root routes it, as for `Toggled`)
- [ ] T010 [P] [US1] Icons in `crates/micold-client/src/icons.rs`: variants `FullScreen` (Material Symbols `fullscreen`, U+E5D0) and `FullScreenExit` (`fullscreen_exit`, U+E5D1) in the enum, `glyph`, `all` and any name table, both in the shipped full-coverage font (`assets/fonts/PROVENANCE.md`)
- [ ] T011 [US1] Drawer retraction in `crates/micold-client/src/ui/material/navigation_drawer.rs` (plan D2, research R2–R3): builder `pub fn retracted(mut self, retracted: bool) -> Self` (doc: hides the whole drawer without touching its open state); `Track` gains `presence: Progress` built with `Progress::new(target)` (1 shown, 0 retracted) so a drawer built retracted starts at 0; `layout` multiplies the width the open/close logic yields (panel + handle, or rail) by the `EMPHASIZED`-eased presence and translates the children left by the rest; at presence ≤ `CLOSED` and retracted, all children `parked` and the node 0 wide; `update` retargets presence and advances it with `on_layout_frame(.., SLIDE, ..)` (reversal from the current value is `Progress::retarget`'s); `update`, `mouse_interaction`, `draw`, `operate` and `overlay` reach no child while parked. Request redraws only while `Progress::animating` (idle quiescence)
- [ ] T012 [US1] Glue in `crates/micold-client/src/ui/mod.rs`: build the drawer with `.retracted(state.overview.full_screen())` (glue exception, quickstart §B 1); confirm with LSP `findReferences` on `overview::Msg::Toggled` where the root fills `settings_open`, and fill it the same way for `FullScreenToggled` in `crates/micold-client/src/app.rs`
- [ ] T013 [US1] Overview header in `crates/micold-client/src/ui/overview.rs` (plan D5, contracts/ui.md *Overview header*): pure `full_screen_label(on)`, `full_screen_tooltip(on)`, `full_screen_icon(on)`; a header row above the grid with, at its end, `Button::outlined(full_screen_label(on))` with leading `full_screen_icon(on)`, wrapped in the shared `Tooltip` with `full_screen_tooltip(on)`, emitting `Msg::FullScreenToggled { settings_open }` with `settings_open` filled as the toolbar fills it for `Toggled` (`state.settings.settings_draft.is_some()`); `on` is `state.overview.full_screen()`. Keyboard focus comes from `Button`'s `TakesTheKeyboard` (FR-012); the connection-lost state keeps the header (spec Edge *Connection lost*)
- [ ] T014 [US1] Showcase poses in `crates/micold-client/src/showcase/sections/surfaces.rs`: `NavigationDrawer` entries "retracted" (built `.retracted(true)`) and "retracting" (a toggle in the showcase state that flips `.retracted`); list them where `crates/micold-client/tests/showcase_completeness.rs` checks poses (Principle VIII, contracts/ui.md *Component API*)
- [ ] T015 [P] [US1] User guide (FR-013, plan D6): in `docs/user-guide/overview.md` add a `## Full screen` section after *Opening and closing it*: the *Full screen* button in the Overview's header and Ctrl+Shift+F (Cmd+Shift+F on macOS) enter and leave it; the sidebar slides away and the cards fill the window's width; the top bar stays; leaving the Overview by a card, Escape, the top-bar button or Ctrl+Shift+O also leaves full screen, and the sidebar returns exactly as you left it (collapsed or expanded, same width); Settings and a project switch keep it; the Overview always opens in the normal layout. Add the chord to `## Keyboard` there, as a row of the shortcut table in `docs/user-guide/terminal-panes.md` (beside the Overview row, line ~38) and to the list in `docs/user-guide/worktrees-and-sessions.md` (beside **Overview**, line ~1684)

**Checkpoint**: The Overview can take the whole window and gives it back; the guide says how.

---

## Phase 4: User Story 4 - The sidebar comes back as it was (Priority: P1)

**Goal**: Leaving full-screen mode by any route shows the sidebar in the state and width it had,
because the mode never writes the sidebar's state (research R2).

**Independent Test**: `cargo test -p micold-client overview`; quickstart §B 5.

### Tests for User Story 4 (MANDATORY — Constitution Principle I) ⚠️

> Written with US1's tests (same red run, T007); green with T008–T012. No implementation of its own.

- [ ] T016 [US4] In `crates/micold-client/tests/overview.rs`: `twenty_cycles_return_the_sidebar_as_it_was` — for the sidebar collapsed (`state.sidebar.hidden = true`) and expanded at the default width, 240 px and 420 px (set `state.sidebar.width` directly), 20 enter / leave cycles in total, leaving in turn by `FullScreenToggled`, `Msg::Closed`, `Msg::Toggled` and `Msg::CardActivated`, each followed by reopening: after every cycle `state.sidebar.hidden` and `state.sidebar.width` equal their values before the cycle and `state.overview.full_screen()` is false (SC-004, US4 scenarios 1–3, FR-009, FR-010)
- [ ] T017 [US4] In `crates/micold-client/tests/overview.rs`: `settings_and_a_project_switch_keep_the_mode_and_the_sidebar` — from full screen with the sidebar collapsed, open and close Settings, then `Msg::ProjectSwitched`: `full_screen()` still true and the sidebar fields unchanged; then leave: still unchanged (US4 scenario 4, FR-011)

### Implementation for User Story 4

None: T008 (derived accessor, no write) and T011 (`leaving_retraction_shows_the_drawer_as_given`
in T005) give it.

**Checkpoint**: US1 and US4 hold; M1 is complete.

---

## Phase 5: User Story 2 - Enter and leave with a zoom (Priority: P1), scenarios 1–4

**Goal**: During the drawer's slide the cards widen and travel continuously on a ribbon between the
two rest layouts, never overlapping, never outside the start–end width range, reversing with the
drawer (research R5). The sidebar half of scenarios 1, 2 and 4 is T005/T011.

**Independent Test**: `cargo test -p micold-client card_grid`; quickstart §B 1, 3, 8.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T018 [US2] Pure-helper tests in the `mod tests` of `crates/micold-client/src/ui/material/card_grid.rs`: `zoom_fraction(start, end, w)` is `clamp((w − start)/(end − start), 0, 1)` for entering and leaving ranges and 0 or 1 outside them; `the_ribbon_at_either_end_equals_the_rest_grid` — for 1150 → 1450, 1300 → 1650, 1464 → 1770 px and back, at `f = 0` and `f = 1` `ribbon(count, w, gap, W, card_height)` gives exactly the rectangles of feature 1024's `columns` rule at that width (one piece per card, within 0.01) (FR-004)
- [ ] T019 [US2] Same file: `ribbon_pieces_never_overlap_and_stay_in_range` — for the three pairs both ways and 1, 7 and 30 cards, sampled at `f` steps of 0.05 with `W = lerp(start, end, f)` and `w = lerp(w_rest(start), w_rest(end), f)`: no two pieces of different cards intersect; every card is one piece or two pieces (end of one row, start of the next) whose widths sum to `w` within 0.01; every piece lies inside `0..W`; `min(w_start, w_end) ≤ w ≤ max(w_start, w_end)`; the grid is never empty (FR-006, SC-002, data-model.md *Invariants*)
- [ ] T020 [US2] Same file, laid-out tests through `test_support::laid_out` with a `CardGrid::new(..).zoom(Zoom { full_screen, freed })`: `a_grid_built_full_screen_lays_out_at_rest` (no zoom on a fresh tree: FR-011); `a_flip_starts_a_zoom_between_the_two_end_widths` (laid out at 1150 px with `full_screen: false`, then `true` with `freed = 300` at 1300 px: card boxes match `ribbon` at `f = 0.5`; at 1450 px they equal the rest grid); `a_zoom_ends_at_an_end_outside_the_range_or_after_the_slide` (W within 0.5 px of an end, W outside `ends` (a resize mid-zoom), or `SLIDE` + 100 ms after the flip: the next layout is the rest grid); `a_reversal_midway_keeps_the_ends` (flip back at `f = 0.5`: `ends` kept, the next layout's boxes within one frame's motion of the last); `a_card_count_change_mid_zoom_keeps_the_zoom` (at `f = 0.5` rebuild with 7 → 9 cards and one card's text changed: `ends` and `started` kept, the zoom neither restarts nor ends, no two pieces intersect: spec Edge *Cards change*, US2 scenario 5 "the change shows"); `full_screen_rows_span_the_freed_width` (SC-001: a 1400 px window with the sidebar expanded at its default width, grid widths `1400 − 48 − freed` and `1400 − 48`: at rest in full screen a full row spans `1400 − 48` within 0.01 and is wider than the normal row by at least `freed`) (FR-006, FR-007, data-model.md `ZoomState`, contracts/ui.md *Geometry gates — Grid*)
- [ ] T021 [US2] Run `cargo test -p micold-client card_grid` and record the red reason (does not compile: no `Zoom`, `zoom_fraction`, `ribbon`, `.zoom`); after stubs (`ribbon` returns the rest grid, `.zoom` ignored) record the behavioural red of T019–T020

### Implementation for User Story 2

- [ ] T022 [US2] Pure helpers in `crates/micold-client/src/ui/material/card_grid.rs` (plan D3): `pub struct Zoom { pub full_screen: bool, pub freed: f32 }`, `struct Piece` (card index, rectangle, clip), `fn zoom_fraction(..) -> f32`, `fn ribbon(count, w, gap, width, card_height) -> Vec<Piece>` placing card `i` at offset `i·(w + gap)` along rows of length `W + gap`, splitting a card that crosses a row end into two clipped pieces; doc comments state the invariants
- [ ] T023 [US2] Zoom widget in `crates/micold-client/src/ui/material/card_grid.rs`: builder `.zoom(Zoom)`; the grid inside the `Scrollable` becomes a custom `Widget` with tree state `ZoomState { full_screen_seen, ends: Option<(f32, f32)>, started: Instant }` (data-model.md): a flip of `full_screen` sets `ends = (W, W ± freed)` (kept on a flip during a zoom); layout by the rest rule at rest and by `ribbon` during a zoom; draw a row-crossing card twice under clip; route events to the card's larger piece; leave zoom mode by the three end rules; request redraws only while zooming. Keep the existing `columns`/`rows` path for rest layout and the existing tests green
- [ ] T024 [US2] Glue: in `crates/micold-client/src/ui/sidebar.rs` make `STRIP_WIDTH` `pub(crate)`; in `crates/micold-client/src/ui/overview.rs` pass `.zoom(Zoom { full_screen: state.overview.full_screen(), freed })` with `freed` = `sidebar_width_px() + ResizeHandle::WIDTH` when `state.sidebar` is expanded, `sidebar::STRIP_WIDTH` when collapsed (research R5)
- [ ] T025 [US2] Showcase: a `CardGrid` "zoom" pose in `crates/micold-client/src/showcase/sections/surfaces.rs` with a toggle in the showcase state flipping `Zoom::full_screen` (and the pose's width); list it for `crates/micold-client/tests/showcase_completeness.rs` (Principle VIII)
- [ ] T026 [P] [US2] User guide: in the `## Full screen` section of `docs/user-guide/overview.md` say that entering and leaving zoom: the cards widen and slide to their new places, a card that changes row slides along the row ends, and pressing the button or the shortcut again mid-way reverses the motion (FR-013)

**Checkpoint**: The zoom is one continuous motion of sidebar and cards.

---

## Phase 6: User Story 2 - Enter and leave with a zoom (Priority: P1), scenario 5

**Goal**: The first card visible before a zoom is still visible after it, even when the zoom moves
it by whole rows; a scroll during the zoom moves the anchor (research R6).

**Independent Test**: `cargo test -p micold-client card_grid overview`; quickstart §B 7.

### Tests for User Story 2, scenario 5 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T027 [US2] In the `mod tests` of `crates/micold-client/src/ui/material/card_grid.rs`: `the_first_visible_card_stays_in_view_across_a_zoom` — 30 cards, a 400 px tall viewport scrolled so card 20 is the first visible, a zoom from 4 to 5 columns (1300 → 1650 px): each layout step `on_reflow` publishes an `AbsoluteOffset` whose y moves by card 20's `dy`; applying it keeps card 20's viewport-relative top within 1 px; a grid at rest publishes nothing; a scroll mid-zoom changes the anchor to the new first visible card (US2 scenario 5, contracts/ui.md *anchor* gate)
- [ ] T028 [US2] In `crates/micold-client/tests/features_overview.rs`: `reflowed_changes_no_state` (`Msg::Reflowed(offset)` leaves `State` unchanged and returns no outcome; the scroll is the root's task, glue exception). Run `cargo test -p micold-client card_grid features_overview` and record the red (no `on_reflow`, `scroll_id`, `Msg::Reflowed`), then the behavioural red after stubs

### Implementation for User Story 2, scenario 5

- [ ] T029 [US2] In `crates/micold-client/src/ui/material/card_grid.rs`: builders `.on_reflow(impl Fn(AbsoluteOffset) -> M)` and `.scroll_id(id)`; during a zoom, in `update`, find the first card meeting the viewport iced hands `update`, compare its top with the new placement and `shell.publish` the scroll offset moved by `dy`; nothing at rest
- [ ] T030 [US2] Glue: `Msg::Reflowed(AbsoluteOffset)` in `crates/micold-client/src/features/overview.rs` (no state change); `GRID_SCROLL_ID` on the grid's `Scrollable` and `.on_reflow(|o| Message::Overview(Msg::Reflowed(o)))` in `crates/micold-client/src/ui/overview.rs`; in `crates/micold-client/src/app.rs` answer it with `iced::widget::operation::scroll_to(GRID_SCROLL_ID, offset)`, as `crates/micold-client/src/shell/changes.rs` does for the diff
- [ ] T031 [P] [US2] User guide: one sentence in the `## Full screen` section of `docs/user-guide/overview.md`: the zoom keeps the card at the top of your view in view (FR-013)

**Checkpoint**: US2 complete.

---

## Phase 7: User Story 3 - Reduced motion switches instantly (Priority: P2)

**Goal**: A *Reduce motion* setting on the Appearance page, off by default, saved like the other
settings; when on, the drawer's presence snaps, and the cards snap with it (research R4).

**Independent Test**: `cargo test -p micold-core settings` and
`cargo test -p micold-client settings navigation_drawer card_grid persist`; quickstart §B 6.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T032 [P] [US3] Core tests in `crates/micold-core/src/settings.rs`: `reduce_motion_defaults_off` (`Settings::default().reduce_motion` false; a settings file without the key loads with it false and every other field as before: "`#[serde(default)]` → `false`; old files load unchanged", data-model.md); `reduce_motion_round_trips` (true saves and loads back true) (FR-008)
- [ ] T033 [P] [US3] Client settings tests in the `mod tests` of `crates/micold-client/src/features/settings.rs`: the draft is seeded from `SettingsState.reduce_motion` by `SettingsDraft::from_settings`; `Msg::ReduceMotionToggled(true)` edits only the draft; Save copies it through `ValidSettings` to `SettingsState.reduce_motion`; Cancel and Esc discard it (clarification 4). In `crates/micold-client/src/shell/persist.rs` tests: `saving_reduce_motion_keeps_every_other_setting`, the pattern of `saving_a_theme_keeps_every_other_setting`. `crates/micold-client/tests/settings_sections.rs` passes with `("reduce_motion", "ReduceMotionToggled")` in `appearance::SETTINGS` (FR-008)
- [ ] T034 [US3] Drawer gate in `crates/micold-client/src/ui/material/navigation_drawer.rs`: `an_instant_drawer_snaps_its_presence` — with `.instant(true)`, retracting from shown and showing from retracted reach the end width on the first frame after the flip, also when the flip comes 200 ms into a non-instant retraction; `.instant(true)` leaves the open/close slide animated (FR-008 "governs this feature's zoom only", Story 3 #1, #3, SC-003)
- [ ] T035 [US3] Grid gate in `crates/micold-client/src/ui/material/card_grid.rs`: `a_snapped_width_ends_the_zoom_on_the_next_layout` — a flip of `full_screen` with W already at the end width lays out the rest grid for that width at once, equal to the animated route's end; and a zoom at `f = 0.4` whose next layout comes at the end width (the drawer snapped mid-way) lays out the rest grid on that layout (Story 3 #1–3)
- [ ] T036 [US3] Run `cargo test -p micold-core settings` and `cargo test -p micold-client settings navigation_drawer card_grid persist settings_sections`; record the red (no `reduce_motion`, `ReduceMotionToggled`, `instant`), then the behavioural red after stubs

### Implementation for User Story 3

- [ ] T037 [US3] Core: `pub reduce_motion: bool` with `#[serde(default)]` and a doc comment in `micold_core::settings::Settings` (`crates/micold-core/src/settings.rs`)
- [ ] T038 [US3] Client settings in `crates/micold-client/src/features/settings.rs` (data-model.md): `AppearanceDraft.reduce_motion`, `ValidSettings.reduce_motion`, `SettingsState.reduce_motion` (applied by `saved` next to `theme_pref`), `Msg::ReduceMotionToggled(bool)`; persist it in `crates/micold-client/src/shell/persist.rs` on the theme's path
- [ ] T039 [US3] Appearance page in `crates/micold-client/src/ui/settings/appearance.rs`: a `LabelledToggle` *Reduce motion* with supporting text "Switch the Overview's full-screen zoom in one step", emitting `ReduceMotionToggled`; `SETTINGS` gains `("reduce_motion", "ReduceMotionToggled")` (plan D4)
- [ ] T040 [US3] Drawer: builder `.instant(bool)` in `crates/micold-client/src/ui/material/navigation_drawer.rs` that snaps the presence track with `Progress::restart_at` instead of easing (also mid-way); pass `.instant(state.settings.reduce_motion)` in `crates/micold-client/src/ui/mod.rs`
- [ ] T041 [P] [US3] User guide: `### Reduce motion` under `## Appearance` in `docs/user-guide/settings.md` (off by default; when on, the Overview's full-screen zoom switches in one step; other animations unchanged; Save applies, Cancel discards; the operating system's setting is not read); one paragraph in the `## Full screen` section of `docs/user-guide/overview.md` linking it (FR-013)

**Checkpoint**: All four stories hold.

---

## Phase 8: Polish & Cross-Cutting Concerns

Changes no code: left to the close unit (milestones rule 4).

- [ ] T042 Visual pass per quickstart §B (steps 1–8) with the `visual-pass` skill on the client and the showcase; record it in `specs/970-overview-fullscreen/visual-pass.md` (FR-003, FR-005, FR-010, FR-012, SC-002, SC-006). The pass is the only proof of: the top bar working in full screen (FR-003), the sidebar and Overview edges touching on every frame (SC-002; structural from the `row!` with a `Fill` sibling), the connection-lost state and a very narrow window in full screen (spec Edge cases)
- [ ] T043 Full gate (`mise run gate`, in the cloud container `scripts/gate.sh`) green on `main` after the last milestone, quickstart §A

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup, Foundational**: empty.
- **US1 (Phase 3)** with **US4 (Phase 4)**: T001–T006, T016–T017 → T007 (red) → T008–T011 → T012–T014; T015 any time.
- **US2 scenarios 1–4 (Phase 5)**: after US1 (needs the drawer's retraction to drive `W` and `full_screen()`); T018–T020 → T021 → T022 → T023 → T024–T025; T026 any time.
- **US2 scenario 5 (Phase 6)**: after Phase 5 (needs the zoom); T027–T028 → T029 → T030; T031 any time.
- **US3 (Phase 7)**: after US1 (drawer presence track) and Phase 5 (T035 needs the zoom); T032–T035 → T036 → T037 → T038 → T039–T040; T041 any time.
- **Polish**: after Phase 7.

### Within Each User Story

- Tests MUST be written and FAIL before implementation (Constitution Principle I).
- The user-guide task ships in the same milestone as its behaviour (Principle VII).

### Parallel Opportunities

- T002, T003, T004, T006 (different files from T001 and T005); T009 and T010 beside T008.
- Every user-guide task (T015, T026, T031, T041) beside the code tasks; they all edit
  `docs/user-guide/overview.md`, so among themselves they run in task order.
- T032 and T033 (core vs client).

## Parallel Example: User Story 1

```bash
Task: "T003 chord tests in crates/micold-client/src/keymap.rs"
Task: "T004 icon locks in crates/micold-client/tests/icons.rs"
Task: "T006 header text tests in crates/micold-client/src/ui/overview.rs"
Task: "T015 Full screen section in docs/user-guide/overview.md"
```

## Implementation Strategy

### MVP First

US1 with US4 (T001–T017): the Overview takes the whole window and gives it back; the sidebar
already slides. Until M2 the cards follow feature 1024's rest rule at every width of the slide.

### Incremental Delivery

M1 (US1 + US4) → M2 (the ribbon zoom) → M3 (the anchor) → M4 (Reduce motion). Polish (visual pass,
final gate) is the close unit's.

## Notes

- `sidebar.hidden` and `sidebar.width` are never written by this feature (FR-009, research R2):
  a review that finds a write is a BLOCKER.
- Never point `CARGO_TARGET_DIR` elsewhere; run tests through the shared target.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Full screen hides the sidebar and gives it back 🎯 MVP

- **Tasks**: T001–T017
- **Deliverable**: with the Overview open, the header's *Full screen* button and Ctrl+Shift+F
  (Cmd+Shift+F) slide the sidebar off and the Overview fills the window's width; the button, the
  shortcut and every way out of the Overview bring the sidebar back exactly as it was; the user
  guide describes it
- **Satisfies**: US1 scenarios 1–4; US4 scenarios 1–4; US2 scenarios 1, 2, 4 for the sidebar's
  edge; FR-001–FR-003, FR-005 (sidebar), FR-007 (sidebar), FR-009–FR-012, FR-013 (Overview page,
  shortcut lists), FR-014; SC-004, SC-005, SC-006 (FR-004 and SC-001 hold at rest through the
  unchanged grid; their gate is T020 in M2)
- **Verify**: `cargo test -p micold-client navigation_drawer overview keymap icons app_chord
  showcase_completeness`; quickstart §B 1–2, 4–5
- **Depends on**: —
- **Tier**: full

### M2 — The cards zoom with the sidebar

- **Tasks**: T018–T026
- **Deliverable**: entering and leaving full screen, the cards widen and slide to their new places
  in one motion with the sidebar, never overlapping, and reverse with it; the showcase poses the
  zoom
- **Satisfies**: US2 scenarios 1–4; FR-004, FR-006, FR-007 (cards); SC-002
- **Verify**: `cargo test -p micold-client card_grid showcase_completeness`; quickstart §B 1, 3, 8
- **Depends on**: M1
- **Tier**: full

### M3 — The zoom keeps your place

- **Tasks**: T027–T031
- **Deliverable**: a card at the top of the view before a zoom is still in view after it
- **Satisfies**: US2 scenario 5
- **Verify**: `cargo test -p micold-client card_grid features_overview`; quickstart §B 7
- **Depends on**: M2
- **Tier**: full

### M4 — Reduce motion

- **Tasks**: T032–T041
- **Deliverable**: Settings → Appearance → *Reduce motion*, off by default and saved like the other
  settings; when on, full screen switches in one step; the user guide's Settings and Overview pages
  describe it
- **Satisfies**: US3 scenarios 1–3; FR-008, FR-013 (Settings page); SC-003
- **Verify**: `cargo test -p micold-core settings`; `cargo test -p micold-client settings
  navigation_drawer card_grid persist settings_sections`; quickstart §B 6
- **Depends on**: M1, M2
- **Tier**: full
