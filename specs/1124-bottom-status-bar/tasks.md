---

description: "Task list for the bottom status bar (#1124)"
---

# Tasks: Bottom Status Bar

**Input**: Design documents from `specs/1124-bottom-status-bar/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/status-bar-ui.md,
quickstart.md

**Tests**: Constitution Principle I — every behaviour has a failing test before its implementation.
Each test task is written first and must fail (or not compile) before the task after it starts.

**Documentation**: Constitution Principle VII — each story updates the user guide in its own
milestone.

**Cross-platform**: Constitution Principle VI — chord labels come from `keymap` (Cmd/Ctrl by
`cfg!(target_os)`); no platform code is added.

Paths are relative to the repository root. `client/` below means `crates/micold-client/`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story this task belongs to (US1–US5)

---

## Phase 1: Setup

**Purpose**: the one design figure the bar is sized by.

- [ ] T001 Add the expected value `status_bar::HEIGHT == 28.0` and the `anatomy::ALL` count 48 to
  `crates/micold-core/tests/tokens_anatomy.rs` (fails: no such module)
- [ ] T002 Add `pub mod status_bar { pub const HEIGHT: f32 = 28.0; }` with a doc comment (fixed by
  the design, not density; R11) to `crates/micold-core/src/tokens/anatomy.rs` and register it in
  `anatomy::ALL` (47 → 48 entries)

---

## Phase 2: Foundational — the shared `StatusBar` component

**Purpose**: the material component every story draws its items with (contract B1–B6, R3, R4).

**⚠️ CRITICAL**: no story can mount the bar before this phase is complete.

- [ ] T003 Write failing unit tests in `client/src/ui/material/status_bar.rs` (`#[cfg(test)]`, using
  `material::test_support`): B1 the bar's layout node is exactly `anatomy::status_bar::HEIGHT` high
  and `Fill` wide; B2 the leading group starts and the trailing group ends `spacing::MD` from the
  edges, items separated by `spacing::MD`; B4 at a too-narrow width `Drop(0)` items vanish before
  `Drop(1)`, all `Drop` before any `Shorten`, `Shorten(0)` before `Shorten(1)` before `Shorten(2)`,
  and a shortened item keeps a floor of dot + one character + "…"; B5 a dropped item has a zero-area
  node; B3 a press on any item publishes no message; `ItemTone` maps `Neutral`/`Positive`/`Warning`/
  `Error` to `on_surface_variant`/`success`/`warning`/`error`; a non-busy bar requests no redraw
- [ ] T004 Implement the types and builder in `client/src/ui/material/status_bar.rs`:
  `ItemTone { Neutral, Positive, Warning, Error }`, `GiveWay { Keep, Drop(u8), Shorten(u8) }`,
  `StatusItem::new(text).dot(ItemTone).tone(ItemTone).busy(bool).tooltip(String)
  .give_way(GiveWay)` (default `Keep`, tone `Neutral`) and `StatusBar::new(roles).item(..)
  .item_element(Element, GiveWay).hint(..).into()`, composed from `Ellipsized`, `BusyIndicator`
  and `Tooltip`; surface `surface_container`, 1 dp `outline_variant` top divider, text
  `TypeRole::Label` small, height `anatomy::status_bar::HEIGHT`; re-export from
  `client/src/ui/material/mod.rs` (makes T003's B1, B2, B3, tone and idle cases pass)
- [ ] T004b Implement the give-way pass in `StatusBar`'s own `Widget::layout` in
  `client/src/ui/material/status_bar.rs` (R4): measure natural widths, drop `Drop` items by rank,
  then ellipsize `Shorten` items by rank to the floor; a dropped item gets a zero-area node and no
  tooltip hit area (makes T003's B4, B5 cases pass)
- [ ] T005 [P] Add the `StatusBar` showcase entry (steady, busy, warning and a too-narrow instance;
  B6) to `client/src/showcase/sections/surfaces.rs` so `client/tests/showcase_completeness.rs` and
  `client/tests/material_builder_api.rs` cover it
- [ ] T006 [P] Add contrast checks for `warning` text and `success`/`warning`/`error` dots on
  `surface_container` in both schemes to `client/src/ui/material/composition_contrast.rs` (a
  guard on existing roles: no new role is added, so it may pass on first run; it fails if a role
  pair misses contrast)

**Checkpoint**: `StatusBar` exists, tested and in the showcase; nothing mounts it yet.

---

## Phase 3: User Story 1 — See the window's health at a glance without chrome rows (P1) 🎯 MVP

**Goal**: a 28 dp bar along the bottom of every body branch reads the connection and the sandbox
state.

**Independent Test**: connected, sandbox running, session active: nothing between app bar and body;
the bar reads a positive dot, "Daemon connected · local" and "Sandbox ready" in the session view,
Overview (full screen too), Changes, Settings and the empty screen.

### Tests for User Story 1 ⚠️

- [ ] T007 [US1] Write failing render-free tests for `ui::status_bar::model(&State,
  &ConnectionStatus, &Sandbox, now, offset)` in `client/src/ui/status_bar.rs`: every row of the
  data-model *Connection item* table (text, dot, tooltip) incl. `{location}` = "local" for a Host
  daemon and for no project open, the daemon's name for another runtime, "no daemon" for
  `Binding::NoDaemon` and for a missing entry, and Default (`""`) when
  `current_session_location()` is `None`; every row of the *Sandbox item* table in its stated order
  (fallback wins over the sandbox state; `Acquiring` with and without `percent`; `busy` true only
  for Probing/Acquiring/Starting; tooltip `attempt_line()` = label then detail on a new line for
  the bring-up rows, `persistent_notice()` for fallback/Stale/Failed, none for Running/Disabled)
- [ ] T008 [P] [US1] Add `StateUnderTest::sandbox(Sandbox)` (states that do not call it keep
  `Sandbox::default()`) to `client/tests/support/layout.rs`, and append covered states
  `status-bar-steady` (connected, sandbox running, session active), `status-bar-overview-full-screen`,
  `status-bar-settings`, `status-bar-changes`, `status-bar-compare`, `status-bar-split-panes` and
  `status-bar-under-menu` (a menu open over the bar; it stands in for every floating surface, as all
  are pushed after `base` through the same overlay stack, FR-013) at the end of `covered_states()`
  in `client/tests/support/covered_states.rs`, with anchors on the bar and on the body's top edge;
  the empty screen and the project surface are covered by the existing `empty-no-project-open` and
  `empty-project-without-worktrees`, which gain the bar when re-recorded (T014)
- [ ] T009 [P] [US1] Write the failing geometry gate `client/tests/gates/status_bar_fits.rs` (mounted
  with `#[path]` in `client/tests/layout_snapshot.rs`): at 640 dp wide with a long remote daemon name
  and a long download detail, no text in the bar is painted past its box and the bar is 28 dp high,
  flush with the window's bottom edge
- [ ] T010 [P] [US1] Add a check to `client/tests/terminal_focus.rs` that the window with the bar
  mounted adds no focusable widget (FR-013), and a case to `client/tests/idle_requests_no_frames.rs`
  that the bar at rest (sandbox running) requests no frames while a bar with the sandbox
  `Acquiring` does (busy ring); add a case to `client/tests/features_window.rs` that the
  install-location screen's view draws no status bar (spec edge case)

### Implementation for User Story 1

- [ ] T011 [US1] Factor the location lookup out of `State::daemon_label_of` into one shared helper
  in `client/src/features/sidebar.rs` (Host → "local", other runtime → name, `NoDaemon`/missing →
  `NO_DAEMON_LABEL`), used by both `daemon_label_of` and the bar (R6; no copy); a refactor with no behaviour change,
  guarded by the existing `client/tests/worktree_daemon_label.rs` and by T007's location cases
- [ ] T012 [US1] Implement `StatusModel { connection, sandbox, cli: Option<ItemModel>, usage:
  Option<UsageView>, hints: [String; 2] }`, `ItemModel { text, dot, tone, busy, tooltip }`,
  `model(..)` (connection and sandbox items; `cli`/`usage` `None`, hints empty until US4/US5) and
  `view(model, roles)` feeding `material::StatusBar` with GiveWay CLI `Shorten(0)`, sandbox
  `Shorten(1)`, connection `Shorten(2)` in NEW `client/src/ui/status_bar.rs`; re-export `model` from
  `client/src/ui/mod.rs` for the crate's tests
- [ ] T013 [US1] Mount `status_bar::view(..)` as the last child of the window column in
  `ui::view_with` (`client/src/ui/mod.rs`) after `body`, outside every body branch; the
  install-location early return stays before it (R1, contract P1)
- [ ] T014 [US1] Regenerate `client/tests/fixtures/layout_snapshot.txt`
  (`UPDATE_LAYOUT_SNAPSHOT=1 cargo test -p micold-client --test layout_snapshot`) and review the diff:
  each state gains the 28 dp bar and the body loses 28 dp, nothing else moves; confirm
  `client/tests/layout_text_overflow.rs` passes over the new states
- [ ] T015 [P] [US1] Add a "Status bar" section to `docs/user-guide/appearance-theming.md` (where the
  bar sits, the connection and sandbox items and their colours, what gives way at narrow widths,
  that banners needing an action still appear under the app bar) and list the new covered states in
  `docs/development/layout-snapshot.md`

**Checkpoint**: the bar is on `main` in every body branch with connection and sandbox items.

---

## Phase 4: User Story 2 — Follow the sandbox coming up from the bar (P1)

**Goal**: the bring-up row is gone; the bar's sandbox item carries the stage, busy mark, percent and
tooltip, and the body's top edge never moves.

**Independent Test**: sandbox `Acquiring` at 42 %: no row above the body; the bar reads
"Getting image… 42%" with a busy ring; hover shows the runtime line.

### Tests for User Story 2 ⚠️

- [ ] T016 [US2] Replace `the_indicator_occupies_nothing_unless_the_sandbox_is_coming_up` in
  `client/tests/sandbox_progress.rs` with a failing test that the window column has no row between
  the toolbar and the body during Probing, Acquiring and Starting, and that the body's top edge is
  the same as with the sandbox running (SC-003); keep the `stage_line`/`attempt_line` wording tests
- [ ] T017 [P] [US2] Append covered state `status-bar-bring-up` (Acquiring, 42 %, with a runtime
  detail line) to `client/tests/support/covered_states.rs`

### Implementation for User Story 2

- [ ] T018 [US2] Remove the bring-up row (`sandbox_status::indicator(attempt_line(..))`) from the
  window column in `client/src/ui/mod.rs`; delete `indicator()`/`view()` from
  `client/src/ui/sandbox_status.rs` and the `view as sandbox_indicator` re-export from
  `client/src/ui/mod.rs`, keeping `stage_line`, `attempt_line` and `StageLine`
- [ ] T019 [US2] Regenerate `client/tests/fixtures/layout_snapshot.txt` and review the diff (only the
  new state and the removed row change)
- [ ] T020 [P] [US2] Update `docs/user-guide/sandboxed-daemon.md`: bring-up progress is shown in the
  status bar's sandbox item (stage, percent, hover for the runtime line), no longer as a row under
  the app bar

**Checkpoint**: a full bring-up never moves the body.

---

## Phase 5: User Story 3 — Failures that need me still interrupt me (P1)

**Goal**: the seven action conditions keep their banners; the lost connection moves to the bar only.

**Independent Test**: each of the seven action states draws its banner and action; `Disconnected`
draws no banner and the bar reads "Daemon disconnected · reconnecting…" with an error dot.

### Tests for User Story 3 ⚠️

- [ ] T021 [US3] Extend `client/tests/banner_is_not_a_snackbar.rs` with failing cases: each of the
  seven FR-009 conditions yields a banner with its action label — Displaced → "Take over",
  ProjectBusy → "Take over", VersionMismatch → "Restart service", BuildMismatch → "Restart
  service", Failed sandbox → "Run without it for now", Stale sandbox → "Restart the sandbox",
  fallback taken → "Try the sandbox again" — and `Disconnected` yields none; when the condition
  ends (back to `Connected` / `Running`) the banner is gone (US3 s4)
- [ ] T022 [P] [US3] Append covered states `status-bar-warnings` (ProjectBusy + Failed sandbox: both
  banners and both warning items, session view), `status-bar-warnings-overview` (the same over the
  Overview: the banners sit in the same slot under the toolbar, FR-010) and `status-bar-disconnected` (no banner row, error dot) to
  `client/tests/support/covered_states.rs`

### Implementation for User Story 3

- [ ] T023 [US3] Make `connection_banner` in `client/src/ui/mod.rs` return its zero-size `Space` for
  `ConnectionStatus::Disconnected` as for `Connected`; the four action arms and `sandbox_banner`
  stay unchanged (R2, D3)
- [ ] T024 [US3] Regenerate `client/tests/fixtures/layout_snapshot.txt` and review the diff (the new
  states, and any existing `Disconnected` state loses its banner row)
- [ ] T025 [P] [US3] Update the connection-drop sentence in `docs/user-guide/overview.md` and the
  "Status bar" section of `docs/user-guide/appearance-theming.md`: a lost connection is shown in the
  bar (red dot, reconnecting) with no banner; conditions needing an action keep their banner

**Checkpoint**: SC-004 — 7 of 7 action banners; no banner for a lost connection.

---

## Phase 6: User Story 4 — Know which CLI the session runs and the key chords (P2)

**Goal**: the bar names the focused session's CLI in the session view and ends with the Overview and
Jump chords.

**Independent Test**: two panes with different CLIs: focusing each changes "CLI: …"; Overview,
Changes, Settings hide the item; hints read "Ctrl+Shift+O Overview" / "Ctrl+Shift+P Jump" (Cmd on
macOS).

### Tests for User Story 4 ⚠️

- [ ] T026 [US4] Add failing `model()` tests in `client/src/ui/status_bar.rs`: `cli` is
  `Some("CLI: {display_name}")` from `AiCli::provider().display_name()` of the `session.active`
  session only when the session view is visible, `None` with a Settings draft, no active project,
  `overview.open`, `changes.open`, Compare or no active session (US4 s3, s5); a change to a
  background session (another worktree, another project) leaves the model unchanged; `hints` are
  `"{overview_chord_label()} Overview"` and `"{switcher chord} Jump"` from `client/src/keymap.rs`
  (both forms: "Ctrl+Shift+O" off macOS, "Cmd+Shift+O" on macOS, R9)
- [ ] T027 [P] [US4] Add a test to `client/src/main_tests.rs` that moving focus between two panes
  with different CLIs through `update` changes the bar model's `cli` text (US4 s2)
- [ ] T028 [P] [US4] Extend `client/tests/gates/status_bar_fits.rs` with the CLI item and both hints
  present at 640 dp: hints are dropped first, no text overflows

### Implementation for User Story 4

- [ ] T029 [US4] Extract `fn session_view_visible(state) -> bool` from `view_with`'s branch order in
  `client/src/ui/mod.rs` and use it in `view_with` (R8); `view_with`'s behaviour is unchanged,
  guarded by the layout snapshot and T026
- [ ] T030 [US4] Fill `cli` (tone Neutral, no dot, `Shorten(0)`) and `hints` (each `Drop(0)`, via
  `StatusBar::hint`) in `client/src/ui/status_bar.rs`
- [ ] T031 [US4] Regenerate `client/tests/fixtures/layout_snapshot.txt` and review the diff (the CLI
  item in session-view states, hints in every state)
- [ ] T032 [P] [US4] Add the CLI item and key hints to the "Status bar" section of
  `docs/user-guide/appearance-theming.md`

**Checkpoint**: the bar matches the mockup's items except plan usage.

---

## Phase 7: User Story 5 — Plan usage in the status bar (P3)

**Goal**: the plan-usage indicator moves from the app bar into the bar, after CLI, unchanged.

**Independent Test**: with a reading held and the switch on, the indicator is in the bar (same
label, warning look, details) and not in the app bar; absent with no reading or the switch off.

### Tests for User Story 5 ⚠️

- [ ] T033 [US5] Move `usage_indicator`'s test from `client/src/ui/toolbar.rs` to
  `client/src/ui/status_bar.rs` and add failing cases: `model().usage` is `Some` only with a current
  reading and the setting on; `client/tests/toolbar.rs` asserts the app bar draws no
  `UsageIndicator`
- [ ] T034 [P] [US5] Extend `client/tests/gates/status_bar_fits.rs` with plan usage present at 640 dp:
  usage is dropped after the hints and before any text is shortened; append covered state
  `status-bar-usage` to `client/tests/support/covered_states.rs`

### Implementation for User Story 5

- [ ] T035 [US5] Move `usage_indicator` (logic unchanged) from `client/src/ui/toolbar.rs` to
  `client/src/ui/status_bar.rs`, add it via `StatusBar::item_element(.., GiveWay::Drop(1))` after
  the CLI item, and remove the usage block and its import from `toolbar::view`
- [ ] T036 [P] [US5] Change the `UsageIndicator` showcase caption to say "status bar" in
  `client/src/showcase/sections/atoms.rs`
- [ ] T037 [US5] Regenerate `client/tests/fixtures/layout_snapshot.txt` and review the diff (usage
  leaves the app bar, joins the bar)
- [ ] T038 [P] [US5] Update `docs/user-guide/settings.md` (§Claude plan usage: "status bar", not "app
  bar"; the intro's "connection strip") and `docs/user-guide/worktrees-and-sessions.md` (§Claude
  plan usage), and add plan usage to the "Status bar" section of
  `docs/user-guide/appearance-theming.md`

**Checkpoint**: every item of the issue is in the bar.

---

## Phase 8: Polish

- [ ] T039 Run quickstart Part B through the `visual-pass` skill and save the screenshots under
  `specs/1124-bottom-status-bar/visual/` (light and dark, 640 dp, banners, bring-up)

---

## Dependencies & Execution Order

- Setup (T001–T002) → Foundational (T003–T006, with T004b) → US1 (T007–T015).
- US2, US3, US4 each depend on US1 only (they edit the mounted bar), but their milestones merge
  one at a time: each appends to `covered_states.rs` and regenerates the fixture, so the next
  rebases and regenerates (R12); US5 depends on US4 (usage sits
  after the CLI item, and its gate case extends US4's).
- Within a story: tests first, then implementation, then the fixture regeneration, then docs.
- Polish (T039) after every story.

## Parallel Opportunities

- T005 and T006 after T004b.
- US1: T008, T009, T010 together after T007; T015 beside T011–T014.
- US2: T017 beside T016. US3: T022 beside T021. US4: T027, T028 beside T026. US5: T034 beside T033,
  T036 beside T035.

## Implementation Strategy

MVP is US1 (with Setup and Foundational): the bar on every body branch with connection and sandbox
state. Each later story is one increment that removes a row the bar now carries (US2, US3) or adds
an item (US4, US5). Every story regenerates the layout fixture itself; a parallel redesign PR that
merges first means rebase and regenerate, never hand-merge (R12).

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — The status bar on every body branch 🎯 MVP

- **Tasks**: T001–T015 (with T004b)
- **Deliverable**: every window body (session, project surface, Overview incl. full screen, Changes,
  Compare, Settings, empty screen) has a 28 dp bar at the bottom reading the connection (dot +
  "Daemon connected · local" or the daemon's name) and the sandbox state; the `StatusBar` component
  is in the showcase
- **Satisfies**: US1 acceptance scenarios 1–7 (s7 for the items present); FR-001, FR-002, FR-004,
  FR-005, FR-006, FR-012, FR-013, FR-014, FR-015 (US1 part); SC-001 (excluding bring-up, M2), SC-002,
  SC-005, SC-006
- **Verify**: `mise run test-core`; `cargo test -p micold-client --lib -- status_bar
  composition_contrast`; `cargo test -p micold-client --test layout_snapshot --test
  layout_text_overflow --test showcase_completeness --test material_builder_api --test
  terminal_focus --test idle_requests_no_frames --test anatomy_call_sites --test
  features_window`; quickstart §B 1, 2, 7, 8
- **Depends on**: —
- **Tier**: full

### M2 — Sandbox bring-up in the bar, no row above the body

- **Tasks**: T016–T020
- **Deliverable**: while the sandbox is checked, downloaded or started, the bar's sandbox item shows
  the stage, busy ring and percent, and no row appears between the app bar and the body
- **Satisfies**: US2 acceptance scenarios 1–4; FR-003 (bring-up row), FR-005, FR-006, FR-015 (US2
  docs, bring-up covered state); SC-001 (bring-up), SC-003
- **Verify**: `cargo test -p micold-client --test sandbox_progress --test layout_snapshot --test
  layout_text_overflow`; quickstart §B 4
- **Depends on**: M1
- **Tier**: light

### M3 — Lost connection in the bar only; action banners kept

- **Tasks**: T021–T025
- **Deliverable**: a lost connection shows only in the bar (red dot, "reconnecting…") with no banner;
  the seven action-needing conditions still raise their banners with their buttons
- **Satisfies**: US3 acceptance scenarios 1–5; FR-003 (Disconnected), FR-009, FR-010, FR-015 (US3
  docs, warning covered states); SC-004
- **Verify**: `cargo test -p micold-client --test banner_is_not_a_snackbar --test layout_snapshot
  --test layout_text_overflow`; quickstart §B 5, 6
- **Depends on**: M1
- **Tier**: light

### M4 — CLI item and key hints

- **Tasks**: T026–T032
- **Deliverable**: in the session view the bar names the focused pane's CLI ("CLI: Claude Code") and
  follows focus; other bodies omit it; the trailing hints read the platform's Overview and Jump
  chords and drop first at narrow widths
- **Satisfies**: US4 acceptance scenarios 1–5; FR-007, FR-008, FR-012 (hints), FR-015 (US4 docs)
- **Verify**: `cargo test -p micold-client --lib -- status_bar`; `cargo test -p micold-client --bin
  micold-ai-ide -- main_tests`; `cargo test -p micold-client --test layout_snapshot --test
  layout_text_overflow`; quickstart §B 1, 3, 7
- **Depends on**: M1
- **Tier**: light

### M5 — Plan usage moves into the status bar

- **Tasks**: T033–T038
- **Deliverable**: with a plan-usage reading and the setting on, the indicator is drawn in the status
  bar after the CLI item, with its label, warning look and details, and no longer in the app bar
- **Satisfies**: US5 acceptance scenarios 1–2; FR-011, FR-012 (usage), FR-015 (docs)
- **Verify**: `cargo test -p micold-client --lib -- status_bar toolbar`; `cargo test -p micold-client
  --test toolbar --test showcase_captions --test layout_snapshot --test layout_text_overflow`;
  quickstart §B 1, 7
- **Depends on**: M4
- **Tier**: light

T039 (visual pass, no code) is left to the close unit (milestones rule 4); it records SC-006, the
macOS flush-corner edge case and light/dark contrast.
