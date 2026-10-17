# Tasks: Keyboard Navigation of Projects, Worktrees and Sessions

**Input**: Design documents from `specs/803-keyboard-navigation/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/keys.md](./contracts/keys.md),
[contracts/ui.md](./contracts/ui.md), [quickstart.md](./quickstart.md)

**Tests**: mandatory (Constitution I). Every test task is written and seen failing (red) before the
implementation task that makes it pass.

**Documentation**: each story's user-guide rows ship in that story's milestone (Constitution VII,
`scripts/check-user-guide-updated.sh`). The docs test of K5 reads both guide files and checks every
row of `NAV_CHORDS`, so each milestone that adds chords adds their docs rows.

**Cross-platform**: one modifier rule (`is_primary_shift`, Ctrl+Shift on Linux/Windows, Cmd+Shift on
macOS); macOS-modifier tests live under `cfg(target_os = "macos")`, so a milestone touching them also
runs `scripts/build-lock.sh cargo check --workspace --target aarch64-apple-darwin`.

**Staged chord table**: `NAV_CHORDS` (K1) is the only place a chord is chosen. It grows by story:
M1 adds the two location rows, M3 the two session rows, M4 the two project rows, so no chord is
captured before its level is resolved (no half-wired chord on `main`).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story the task belongs to (US1–US4)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: the two new modules exist and compile, empty of behaviour.

- [x] T001 Create `crates/micold-core/src/navigation.rs` with `pub enum Dir { Next, Previous }` (derive `Debug, Clone, Copy, PartialEq, Eq`) and a module doc naming FR-005, and register it with `pub mod navigation;` in `crates/micold-core/src/lib.rs`
- [x] T002 Create `crates/micold-client/src/features/navigation.rs` with `pub enum Step { None, Session(SessionId), Stop(SessionLocation), Project(PathBuf) }` (data-model.md *Client state*; derive `Debug, Clone, PartialEq, Eq`) and register it with `pub mod navigation;` in `crates/micold-client/src/features/mod.rs`; create `crates/micold-client/tests/navigation.rs` (resolution) and `crates/micold-client/tests/navigation_apply.rs` (apply) with a `State` fixture that reuses the builders of `crates/micold-client/tests/sidebar_state.rs` / `features_sidebar.rs` (grep for them first; extract a shared helper into `tests/common/` rather than copying)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the wrap rule (D1) and the key decoding/routing machinery (D2, K1–K3) that every level
uses.

**⚠️ CRITICAL**: no user story work can begin until this phase is complete.

- [x] T003 [P] Write the failing unit tests for `step` in `crates/micold-core/src/navigation.rs` (`#[cfg(test)]`): next and previous from the middle; Next on the last → first and Previous on the first → last (wrap, SC-001); a one-entry list returns that entry for both directions; an empty list → `None`; `current = None` → first (Next) / last (Previous); for N up to 6, every target is reached from every start within ⌊N/2⌋ presses in the shorter direction (SC-001); `current` not in `order` → first (Next) / last (Previous) (data-model.md: "empty → `None`; `current` in `order` → neighbour with wrap; a one-entry list → that entry; `current` absent or `None` → first (Next) / last (Previous)")
- [x] T004 Implement `pub fn step<T: PartialEq + Clone>(order: &[T], current: Option<&T>, dir: Dir) -> Option<T>` in `crates/micold-core/src/navigation.rs` until T003 passes (`mise run test-core`)
- [x] T005 [P] Write the failing key tests in `crates/micold-client/tests/keymap.rs` for the two location rows of K1 (contracts/keys.md K2, K5): give the `ki` helper a `physical: PhysicalKey` (default `Other`) and update the `KeyInput { .. }` literal in `crates/micold-client/tests/client_input.rs`; Ctrl+Shift with physical `Comma`/`Period` → `KeyOutput::Navigate(NavAction { level: Location, dir: Previous/Next })` (Cmd+Shift under `cfg(target_os = "macos")`); with `physical = Other` the logical fallback `,`/`<` and `.`/`>` decodes the same; near misses keep their outputs (Shift missing, primary missing, Alt added; Ctrl+Shift+E/T/O/D/H/W/Arrows/C/V unchanged); no two `NAV_CHORDS` rows share a `PhysicalKey`; no `NAV_CHORDS` chord decodes to any pre-existing reserved output (R9); `route_key(false, Navigate(a)) == route_key(true, Navigate(a)) == KeyRouting::Navigate(a)`; docs test: both strings of `nav_chord_labels(a)` for every `NAV_CHORDS` row appear in `docs/user-guide/terminal-panes.md` and in `docs/user-guide/worktrees-and-sessions.md`
- [x] T006 Implement in `crates/micold-client/src/keymap.rs`: `pub enum PhysicalKey { Comma, Period, Other }` (more variants arrive in T027, T033), `KeyInput.physical: PhysicalKey`, `NavLevel { Session, Location, Project }`, `NavAction { level, dir: micold_core::navigation::Dir }` (`Copy`), `NAV_CHORDS` with the two location rows of K1, `is_primary_shift` factored out of `is_primary_shift_chord` (which then calls it), `nav_chord`, `nav_chord_labels(action) -> (String, String)` (`Ctrl+Shift+,` / `Cmd+Shift+,` form), `KeyOutput::Navigate(NavAction)` checked first in `encode` (tier 1, before Pane, Copy/Paste, named keys and control bytes); add the `physical` field to the `KeyInput` literal in `crates/micold-client/src/ui/material/terminal_pane.rs` (`PhysicalKey::Other` until T015)
- [x] T007 Add `KeyRouting::Navigate(NavAction)` and its arm in `route_key` (before the `terminal_focused` check, like `ToggleOverview`) in `crates/micold-client/src/app.rs` until the routing test of T005 passes (the docs test stays red until T016)

**Checkpoint**: `step` and decoding are green; no chord is dispatched yet.

---

## Phase 3: User Story 1 — Step between worktrees from the keyboard (Priority: P1) 🎯 MVP

**Goal**: Ctrl/Cmd+Shift+, and . step through the active project's locations as a row click does,
wrapping, skipping filtered and collapsed-group worktrees, scrolling the target into view, sending
0 bytes to the terminal; an empty location is a stop; the keyboard lands in the terminal or on the
sidebar row (FR-010).

**Independent Test**: quickstart §B B1, B2, B5, B7, B8, B9 and B10 (location chords).

### Tests for User Story 1, press and resolution (M1) ⚠️

- [x] T008 [P] [US1] Write the failing resolution tests in `crates/micold-client/tests/navigation.rs` (contracts/ui.md U1, U2 location row): `update` returns `Step::None` while `settings_draft.is_some()`, a dialog, a context menu, the project switcher panel or the filter panel is open (`overlay::registry::topmost`); `location_order` is Default first, then worktrees in drawn order, without tag-filtered worktrees and without the worktrees of a collapsed run group, with those of an expanded group, and with the current session's location even when the tag filter does not match it (the sidebar keeps it shown); Next/Previous from the middle, wrap at both ends, Default-only project → `Step::None`, no active project → `Step::None`, no current session → first/last location; a target with a main session → `Step::Session(main)`; a target without one (empty, missing or invalid worktree) → `Step::Stop(location)`; `nav_stop` set and in the order is the current location, `nav_stop` no longer in the order is ignored
- [x] T009 [P] [US1] Write the failing sidebar state tests in `crates/micold-client/tests/sidebar_state.rs` (plan D4, U5): `row_index(entries, &RowRef)` for a location row, a session sub-row and a run row, and `current_session_row` still agrees with it; `reveal_scroll_offset` targets `keyboard_row` when set, else the current session's row; `nav_stop` is cleared by `set_current_session`, `session::selected` and `project_entered`; `SidebarMsg::KeyboardRowLeft` clears `keyboard_row` and `nav_stop`; a pointer selection clears both; `tests/current_session_writers.rs` stays green (no new writer of `session.active`)
- [x] T010 [P] [US1] Write the failing apply tests in `crates/micold-client/tests/navigation_apply.rs` (U3): `apply_navigation(Step::Session(id), true)` returns `[Message::Session(SessionMsg::Selected(id))]`, arms the reveal scroll and leaves `keyboard_row` `None`; `apply_navigation(Step::Stop(loc), _)` returns no message, sets `nav_stop = Some(loc)`, releases the terminal, sets `keyboard_row = Some(RowRef::Location(loc))`, arms the reveal scroll, and leaves `session.active` and the session count unchanged (SC-005); `Step::None` changes nothing; with the Overview open, the `Selected` follow-up dispatched through `session::selected` closes it (FR-012)
- [x] T011 [P] [US1] Write the failing routing tests in the `#[cfg(test)]` module of `crates/micold-client/src/ui/material/terminal_pane.rs` (beside the `overview_chord` test) and for `app_chord` in `crates/micold-client/src/ui/mod.rs`: a focused terminal given Ctrl+Shift+. (physical `Period`) publishes `Message::Navigate(Location Next)`, captures the event and writes 0 bytes to the PTY (SC-002); `to_keymap_physical(Code::Comma)` → `Comma`, `Code::Period` → `Period`, other codes and `Unidentified` → `Other`; `app_chord` answers both the Overview chord and a location chord with the right message

### Implementation for User Story 1, press and resolution (M1)

- [x] T012 [US1] In `crates/micold-client/src/features/sidebar.rs` add `pub enum RowRef { Location(SessionLocation), Session(SessionId) }`, `State::nav_stop: Option<SessionLocation>`, `State::keyboard_row: Option<RowRef>`, `SidebarMsg::KeyboardRowLeft`, `row_index(entries, &RowRef)` with `current_session_row` built on it, the `keyboard_row` target in `reveal_scroll_offset`, and the clearings of U5 (`set_current_session` and `session::selected` in `crates/micold-client/src/features/session.rs`, `project_entered`, pointer selection, `KeyboardRowLeft`) until T009 passes
- [x] T013 [US1] In `crates/micold-client/src/features/navigation.rs` implement `location_order(&State) -> Vec<SessionLocation>` over `sidebar_entries()` and `update(&State, NavAction) -> Step` with the U1 gate and the location row of U2 (session and project levels return `Step::None` until T028, T034) until T008 passes
- [x] T014 [US1] In `crates/micold-client/src/app.rs` add `Message::Navigate(NavAction)` and `State::apply_navigation(step, terminal_had_focus) -> Vec<Message>` for `Session` (returns the `Selected` follow-up and arms the reveal scroll), `Stop` and `None`, until T010 passes; in M1 a press with the terminal unfocused ends with the terminal focused by `Selected`, as a click does (the not-focused branch is T020)
- [x] T015 [US1] Wire the press (D2, D5, K3): in `crates/micold-client/src/ui/material/terminal_pane.rs` add `to_keymap_physical`, fill `KeyInput.physical` from `KeyPressed { physical_key, .. }`, and on `KeyRouting::Navigate(a)` publish `Message::Navigate(a)` and capture; in `crates/micold-client/src/ui/mod.rs` rename `overview_chord_listener`/`overview_chord` to `app_chord_listener`/`app_chord` answering the Overview and navigation chords; in `crates/micold-client/src/main.rs` route `Message::Navigate(action)`: read `terminal_focused()`, call `navigation::update`, then `apply_navigation`, and dispatch each returned message through `update` so `Selected` runs its full shell path; until T011 passes
- [x] T016 [US1] Document the location chords (contracts/keys.md K4): in `docs/user-guide/terminal-panes.md` §Keyboard shortcuts add the "Previous / next location" row with `Ctrl+Shift+,` / `Ctrl+Shift+.` and `Cmd+Shift+,` / `Cmd+Shift+.`, and make the section intro say the app takes these chords whether or not a terminal is focused; in `docs/user-guide/worktrees-and-sessions.md` add a **Navigation** bullet beside **Overview**: the location chords, the wrap rule, filtered and collapsed-group worktrees skipped, an empty location becomes a stop without starting a session; until the docs test of T005 passes

### Tests for User Story 1, keyboard on the sidebar row (M2) ⚠️

- [ ] T017 [P] [US1] Write the failing component tests in `crates/micold-client/src/ui/material/keyboard_focus.rs` and `crates/micold-client/src/ui/material/tree_view.rs` (D6, U4): `TakesTheKeyboard` with a supplied focus and `shown(true)` draws the focus ring on the first build after the flag turns true (wrapper already in the tree) and on a freshly created wrapper (`state()` seeded from the supplied flag when `on_focus_change` is set); a disabled wrapper is skipped by Tab traversal and draws nothing; `TreeItem::keyboard_focus` leaves the row's layout size unchanged
- [ ] T018 [P] [US1] Write the failing focus-rule tests in `crates/micold-client/tests/navigation_apply.rs` (U3, FR-010): `apply_navigation(Step::Session(id), false)` returns the `Selected` follow-up, and `finish_navigation(Step::Session(id), false)` run after it releases the terminal and sets `keyboard_row = Some(RowRef::Session(id))`; `finish_navigation(_, true)` changes nothing; after a `Stop`, the next location press (terminal no longer focused) that lands on a session sets `keyboard_row`, not terminal focus

### Implementation for User Story 1, keyboard on the sidebar row (M2)

- [ ] T019 [US1] In `crates/micold-client/src/ui/material/keyboard_focus.rs` add `TakesTheKeyboard::shown(bool)` and seed `Focus` in `state()` from the supplied flag when `on_focus_change` is set; in `crates/micold-client/src/ui/material/tree_view.rs` add `TreeItem::keyboard_focus(focused: bool, on_change: impl Fn(bool) -> M)` wrapping the row in `TakesTheKeyboard` (enabled only when `focused`) with the row's indicator; until T017 passes
- [ ] T020 [US1] In `crates/micold-client/src/app.rs` add `State::finish_navigation(step, terminal_had_focus)` (plan D5): when the terminal did not have the keyboard and the step is `Session(id)`, release the terminal and set `keyboard_row = Some(RowRef::Session(id))`; in `crates/micold-client/src/main.rs` call it after dispatching the follow-ups (no branch in `main.rs`); until T018 passes
- [ ] T021 [US1] In `crates/micold-client/src/ui/sidebar.rs` call `keyboard_focus` on **every** row (location rows, session sub-rows, run rows), `focused` true only for the row whose `RowRef` equals `sidebar.keyboard_row`, sending `SidebarMsg::KeyboardRowLeft` on focus loss (U4: the tree keeps its shape across presses)
- [ ] T022 [P] [US1] Add a keyboard-focused worktree row variant to the TreeView entry in `crates/micold-client/src/showcase/catalogue.rs` and `crates/micold-client/src/showcase/sections/surfaces.rs` so `crates/micold-client/tests/showcase_completeness.rs` lists it (Constitution VIII)
- [ ] T023 [US1] Extend the **Navigation** bullet of `docs/user-guide/worktrees-and-sessions.md` with where the keyboard lands: in the new session's terminal when a terminal had focus, else on the sidebar row, which shows the focus ring (also on a stop)

**Checkpoint**: location chords work end to end in both focus states (M1 + M2).

---

## Phase 4: User Story 2 — Step between sessions of one worktree (Priority: P2)

**Goal**: Ctrl/Cmd+Shift+[ and ] cycle the current location's sessions.

**Independent Test**: quickstart §B B3, B4 and B10 (session chords, Ctrl+[ still `^[`).

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T024 [P] [US2] Extend the failing key tests in `crates/micold-client/tests/keymap.rs` with the two session rows: physical `BracketLeft`/`BracketRight` with the primary modifier + Shift → `Navigate(Session Previous/Next)`; logical fallback `[`/`{` and `]`/`}` with `physical = Other`; Ctrl+[ without Shift still gives `0x1b` and Ctrl+] gives `0x1d`; the K5 uniqueness, reserved-output and docs tests now run over four rows
- [ ] T025 [P] [US2] Write the failing session-level tests in `crates/micold-client/tests/navigation.rs` (U2 session row): order is the current location node's `listed_sessions()` (main first, helpers in spawn-tree order); Next from main → first helper and two more presses → main (wrap); Previous wraps backwards; one session → `Step::None`; no current session → `Step::None`; `nav_stop` set → `Step::None`; after applying a helper's step on a collapsed location row that row is open by `effective_open` (FR-009)
- [ ] T026 [P] [US2] Extend the `to_keymap_physical` test in `crates/micold-client/src/ui/material/terminal_pane.rs`: `Code::BracketLeft` → `BracketLeft`, `Code::BracketRight` → `BracketRight`

### Implementation for User Story 2

- [ ] T027 [US2] Add `PhysicalKey::BracketLeft`/`BracketRight` and the two session rows of K1 to `NAV_CHORDS` and the fallback in `crates/micold-client/src/keymap.rs`, and their mapping in `to_keymap_physical` in `crates/micold-client/src/ui/material/terminal_pane.rs`, until T024 and T026 pass (docs test stays red until T029)
- [ ] T028 [US2] Implement the session level of `update` in `crates/micold-client/src/features/navigation.rs` until T025 passes
- [ ] T029 [US2] Add the "Previous / next session" row (`Ctrl+Shift+[` / `Ctrl+Shift+]`, `Cmd+Shift+[` / `Cmd+Shift+]`) to `docs/user-guide/terminal-panes.md` §Keyboard shortcuts and the session chords to the **Navigation** bullet of `docs/user-guide/worktrees-and-sessions.md`, noting that plain Ctrl+[ and Ctrl+] still reach the terminal; until the docs test passes

**Checkpoint**: location and session chords work; `NAV_CHORDS` has four rows.

---

## Phase 5: User Story 3 — Step between projects (Priority: P3)

**Goal**: Ctrl/Cmd+Shift+PageUp and PageDown step through the available known projects as the
switcher does.

**Independent Test**: quickstart §B B6 and B10 (project chords).

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T030 [P] [US3] Extend the failing key tests in `crates/micold-client/tests/keymap.rs` with the two project rows: physical `PageUp`/`PageDown` with the primary modifier + Shift → `Navigate(Project Previous/Next)`; the named-key fallback with `physical = Other`; PageUp/PageDown without Shift or without the primary modifier keep their existing outputs
- [ ] T031 [P] [US3] Write the failing project-level tests in `crates/micold-client/tests/navigation.rs` (U2 project row, U3): order is `switcher_entries()` with `available`, unavailable projects skipped; wrap both ends; one available project → `Step::None`; welcome screen (no active project) → first (Next) / last (Previous); no known project → `Step::None`; `apply_navigation(Step::Project(path), true)` returns `[Message::Project(ProjectMsg::Reopened(path))]`; after `Reopened` ran, `finish_navigation(Step::Project(path), false)` releases the terminal and sets `keyboard_row = Some(RowRef::Session(restored current))` when a session was restored and `None` otherwise; with the Overview open, a project step leaves it to the project switch
- [ ] T032 [P] [US3] Extend the `to_keymap_physical` test in `crates/micold-client/src/ui/material/terminal_pane.rs`: `Code::PageUp` → `PageUp`, `Code::PageDown` → `PageDown`

### Implementation for User Story 3

- [ ] T033 [US3] Add `PhysicalKey::PageUp`/`PageDown` and the two project rows of K1 (marked ledger decision D4, provisional) to `NAV_CHORDS` and the fallback in `crates/micold-client/src/keymap.rs`, and their mapping in `to_keymap_physical` in `crates/micold-client/src/ui/material/terminal_pane.rs`, until T030 and T032 pass (docs test stays red until T036)
- [ ] T034 [US3] Implement the project level of `update` in `crates/micold-client/src/features/navigation.rs`
- [ ] T035 [US3] Implement `Step::Project` in `State::apply_navigation` (the `Reopened` follow-up) and in `State::finish_navigation` (the restored current session as `keyboard_row`) in `crates/micold-client/src/app.rs`, until T031 passes
- [ ] T036 [US3] Add the "Previous / next project" row (`Ctrl+Shift+PageUp` / `Ctrl+Shift+PageDown`, `Cmd+Shift+PageUp` / `Cmd+Shift+PageDown`) to `docs/user-guide/terminal-panes.md` §Keyboard shortcuts and the project chords to the **Navigation** bullet of `docs/user-guide/worktrees-and-sessions.md`, with unavailable projects skipped and the macOS laptop form Cmd+Shift+Fn+Up / Cmd+Shift+Fn+Down; until the docs test passes

**Checkpoint**: all six chords work.

---

## Phase 6: User Story 4 — Find the shortcuts (Priority: P3)

**Goal**: every user-guide shortcut list carries the six chords with both platform forms and the
wrap rule (FR-013, SC-006).

**Independent Test**: the docs test of `crates/micold-client/tests/keymap.rs` with all six rows; read
both files.

- [ ] T037 [US4] Check that `grep -rn 'Ctrl+Shift+O' docs/user-guide/` finds no shortcut list beyond `terminal-panes.md` and `worktrees-and-sessions.md` (else add the six chords there and that file to the docs test's file list in `crates/micold-client/tests/keymap.rs`), and that the **Navigation** bullet in `docs/user-guide/worktrees-and-sessions.md` states the wrap rule for all three levels

---

## Phase 7: Polish & Cross-Cutting Concerns

**Purpose**: evidence only; no code (done by the close unit).

- [ ] T038 Run quickstart §B B1–B12 through the `visual-pass` skill and record screenshots for B1, B3 and B7 in `specs/803-keyboard-navigation/visual-pass/`
- [ ] T039 Record the quickstart §C platform-parity status (macOS, Windows: B1, B3, B6, B10, no system action) in `specs/803-keyboard-navigation/quickstart.md`, or note it as not run with the reason

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: none.
- **Foundational (Phase 2)**: after Setup; blocks every story.
- **US1 (Phase 3)**: after Foundational. T008–T011 before T012–T016; T017–T018 before T019–T023.
- **US2 (Phase 4)**: after US1 (uses `update`, `apply_navigation`, the press wiring, `nav_stop`).
- **US3 (Phase 5)**: after US2 (M4 depends on M3): both edit `NAV_CHORDS` and the docs rows, and
  M4's six-row docs test needs the session rows.
- **US4 (Phase 6)**: after US3 (the docs test then covers all six rows).
- **Polish (Phase 7)**: after all stories.

### Within each story

- Tests first and red, then implementation, then docs.
- `sidebar.rs` (T012) before `navigation.rs` (T013) before `app.rs` (T014) before the wiring (T015).
- Component (T019) before the sidebar rows (T021).

### Parallel Opportunities

- T003 ∥ T005 (core vs client keys).
- T008–T011: four different files; T017 ∥ T018.
- T024 ∥ T025 ∥ T026; T030 ∥ T031 ∥ T032.
- T022 (showcase) ∥ T021 once T019 is done.

## Parallel Example: User Story 1

```text
Task: "T008 resolution tests in crates/micold-client/tests/navigation.rs"
Task: "T009 sidebar state tests in crates/micold-client/tests/sidebar_state.rs"
Task: "T011 routing tests in ui/material/terminal_pane.rs and ui/mod.rs"
```

## Implementation Strategy

### MVP First (User Story 1)

Setup, Foundational and US1 make the location chords work: the most frequent switch without the
mouse. Stop and validate with quickstart §B B1, B2, B5, B7–B10.

### Incremental Delivery

US1 location chords (M1, then the sidebar focus ring in M2) → US2 session chords (M3) → US3 project
chords and US4 docs check (M4) → quickstart evidence in the close PR. Each milestone adds its chords
to `NAV_CHORDS` and its docs rows together, so `main` never captures a chord it does not act on.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Location chords 🎯 MVP

- **Tasks**: T001–T016
- **Deliverable**: Ctrl/Cmd+Shift+. and , step through the active project's locations exactly as a row click does (wrap, filtered and collapsed-group worktrees skipped, target scrolled into view, Overview closed, nothing while a dialog, menu, panel or Settings is open), from a focused terminal (0 bytes to it) or not, the new session's terminal takes the keyboard, and an empty location is a stop the next press steps on from; both chords are in the user guide
- **Satisfies**: US1 acceptance scenarios 1, 2, 3, 5 (no session started), 6, 7; FR-001 (location rows), FR-002, FR-005, FR-006, FR-007, FR-008, FR-009, FR-011, FR-012, FR-013 (part: location rows), FR-015, FR-016; SC-001, SC-002, SC-003, SC-004 (part: presses that show a session), SC-005
- **Verify**: `mise run test-core` and `mise run gate` (`tests/keymap.rs`, `tests/navigation.rs`, `tests/navigation_apply.rs`, `tests/sidebar_state.rs`, `terminal_pane.rs` and `ui/mod.rs` routing tests); quickstart §B B2, B5, B8, B9, B10 with the location chords
- **Depends on**: —
- **Tier**: full
- **Note**: FR-010's "otherwise" branch (keyboard on the sidebar row, focus ring) lands in M2. Until then a press with the terminal unfocused focuses the new session's terminal as a click does, and a stop releases the terminal and scrolls to the stop's row without a ring.

### M2 — The keyboard lands on the sidebar row

- **Tasks**: T017–T023
- **Deliverable**: a location chord pressed while the terminal is not focused, or one that lands on an empty location, moves the sidebar's keyboard focus to the new row and draws its focus ring; the showcase lists a keyboard-focused tree row
- **Satisfies**: US1 acceptance scenarios 4, 5 (ring); FR-010; SC-004
- **Verify**: `mise run gate` (`keyboard_focus.rs`, `tree_view.rs`, `tests/navigation_apply.rs` focus-rule tests, `tests/showcase_completeness.rs`); quickstart §B B1 and B7
- **Depends on**: M1
- **Tier**: full

### M3 — Session chords

- **Tasks**: T024–T029
- **Deliverable**: Ctrl/Cmd+Shift+[ and ] cycle the current location's sessions (main, then helpers, wrapping; opening a collapsed row), plain Ctrl+[ and Ctrl+] still reach the terminal, and the user guide lists them
- **Satisfies**: US2 acceptance scenarios 1–4; FR-001 (session rows), FR-003, FR-008, FR-013 (session rows)
- **Verify**: `mise run gate` (`tests/keymap.rs` session rows, `tests/navigation.rs` session level); quickstart §B B3, B4, B10
- **Depends on**: M1, M2
- **Tier**: light

### M4 — Project chords and the complete shortcut list

- **Tasks**: T030–T037
- **Deliverable**: Ctrl/Cmd+Shift+PageUp and PageDown step through the available known projects as the switcher does (wrap, unavailable skipped, welcome screen → first/last), and the user guide lists all six chords with both platform forms and the wrap rule
- **Satisfies**: US3 acceptance scenarios 1–4; US4 acceptance scenario 1; FR-001 (project rows), FR-004, FR-013; SC-006
- **Verify**: `mise run gate` (`tests/keymap.rs` project rows and the six-row docs test, `tests/navigation.rs` project level); quickstart §B B6, B10
- **Depends on**: M1, M2, M3
- **Tier**: full

Polish (T038, T039) changes no code: the close unit does it in the close PR.
