---

description: "Task list for the quick-switcher (feature 968)"
---

# Tasks: Quick-Switcher to Jump to a Project, Location or Session

**Input**: Design documents from `specs/968-quick-switcher/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/keys.md](./contracts/keys.md),
[contracts/ui.md](./contracts/ui.md), [quickstart.md](./quickstart.md)

**Tests**: Constitution Principle I (Test-First, NON-NEGOTIABLE): every behaviour task is preceded
by a failing-test task in the same milestone. Write the test, see it fail for the right reason, then
implement.

**Documentation**: Principle VII: the user-guide rows ship in the milestone of the behaviour they
describe (`scripts/check-user-guide-updated.sh` fires on `feat`).

**Cross-platform**: Principle VI: the chord uses the platform primary modifier (`is_primary_shift`)
and a physical key; no `cfg` arm beyond the macOS test variant.

**Scope note (ledger D4, provisional)**: only the active project's locations and sessions are
entries. Every "which entries exist" rule is in `features/quick_switch.rs::candidates` and its
tests; if the user picks option B, only those tasks (T009, T013, T022, T025, T027, T029), the
choose path for a non-active entry, the T031 doc bullet and FR-003/FR-004 change.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story the task belongs to (US1–US4)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: the new modules exist and are registered, so tests can name them.

- [X] T001 Create `crates/micold-core/src/quick_switch.rs` with the types of data-model.md *Core*:
  `pub enum EntryKey { Location(SessionLocation), Session(SessionId), Project(PathBuf) }`,
  `pub struct Candidate { pub key: EntryKey, pub name: String, pub parent: Option<usize>, pub
  selectable: bool }` ("`parent` always points earlier in the list"; "`selectable` `false` only
  for an unavailable project"), `pub enum Rank { Prefix, Run, Scattered }` (derive `Ord`: "derived
  Ord is the rank order"), `pub struct Ranked { pub index: usize, pub rank: Rank, pub spans:
  Vec<Range<usize>> }` (all derive `Debug, Clone, PartialEq, Eq`), a module doc naming FR-005,
  FR-007, FR-013, and register it with `pub mod quick_switch;` in `crates/micold-core/src/lib.rs`
- [X] T002 [P] Create `crates/micold-client/src/features/quick_switch.rs` with `pub enum Kind {
  Location, Session, Project }`, `pub struct Entry { pub candidate: Candidate, pub kind: Kind, pub
  context: String, pub current: bool }`, `pub struct State { pub open: Option<Open> }` (derive
  `Default`), `pub struct Open { pub query: String, pub highlighted: Option<EntryKey>, pub
  terminal_had_focus: bool, pub keyboard_row_before: Option<RowRef> }` and `pub enum Msg { Toggled
  { terminal_had_focus: bool }, QueryChanged(String), Moved(Dir), Chosen(EntryKey), Dismissed }`
  (data-model.md *Client*; `Dir` is `micold_core::navigation::Dir`; `RowRef` is the type of
  `sidebar.keyboard_row`, find it with `LSP hover`), registered in
  `crates/micold-client/src/features/mod.rs`; add `quick_switch: quick_switch::State` to the root
  `State` and `Message::QuickSwitch(crate::features::quick_switch::Msg)` beside
  `Message::Navigation` in `crates/micold-client/src/app.rs` (the core `update` ignores it in the
  same arm as `Message::Navigation`: the binary routes it); create
  `crates/micold-client/tests/quick_switch.rs` with a `State` fixture built from
  `tests/support/sidebar_fixture.rs` (reuse its builders, extend them there rather than copying)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the matching rules in core, the chord, and the two shared readers the switcher reuses
from #803. Nothing here is observable alone; it ships inside M1.

- [X] T003 [P] Write the failing unit tests for `filter`, `reseat` and `step` in
  `crates/micold-core/src/quick_switch.rs` (`#[cfg(test)]`, plan *Test strategy* core row): an
  empty query keeps every candidate in order with rank `Prefix` and no spans; `logpg` keeps
  `Login page` and drops names without l, o, g, p, g in order; `Prefix` before `Run` before
  `Scattered`, input order kept within a rank (stable); case ignored (`LOG` matches `login`); `e`
  does not match `é` (no accent folding); spans are byte ranges of the original name and index
  correctly past a non-ASCII character; a typo (`lgoin`) does not match (no single-edit tier);
  `reseat` returns the key's position when listed and selectable, else the first selectable, `None`
  when none is selectable or the list is empty; `step` moves to the next selectable below/above,
  skips an unselectable candidate, and stays at the first/last selectable; 500 candidates, 10
  successive queries, each `filter` under 100 ms (SC-003)
- [X] T004 Make `Folded`, `literal` and `subsequence` `pub(crate)` in
  `crates/micold-core/src/typeahead.rs` (taking the folded name, research R2) without changing
  their behaviour, so the existing `typeahead.rs` tests stay green; then implement `pub fn
  filter(candidates: &[Candidate], query: &Query) -> Vec<Ranked>`, `pub fn reseat(candidates:
  &[Candidate], ranked: &[Ranked], key: Option<&EntryKey>) -> Option<usize>` and `pub fn
  step(candidates: &[Candidate], ranked: &[Ranked], at: usize, dir: Dir) -> usize` in
  `crates/micold-core/src/quick_switch.rs` with one fold of the query per call and those two tier
  functions plus a prefix check (no copy of their bodies), until T003 passes (`mise run test-core`).
  `filter` ranks each candidate on its own name only; the sessions-kept-by-their-location rule is
  T024
- [X] T005 [P] Write the failing key tests in `crates/micold-client/tests/keymap.rs` (contracts/keys.md
  K5): Ctrl+Shift with physical `KeyP` → `KeyOutput::ToggleSwitcher` (Cmd+Shift under
  `cfg(target_os = "macos")`); with `physical = Other` the logical fallback `p` and `P` decodes the
  same; physical `KeyP` with logical `l` (Dvorak) decodes to `ToggleSwitcher` (FR-016); the K2
  near misses keep their outputs (Ctrl+P → `0x10`, Shift+P, Alt+Shift+P, primary+Alt+Shift+P);
  Ctrl+Shift+E/T/O/D/H/W/C/V/Arrows/`[ ] , .`/PageUp/PageDown never decode to `ToggleSwitcher` and
  the switcher chord decodes to nothing else (FR-002's app-chord half; the system and input-method
  lists are not data in the repo, so that half rests on spec.md *Assumptions* and quickstart §C);
  `route_key(true, ToggleSwitcher) ==
  route_key(false, ToggleSwitcher) == KeyRouting::ToggleSwitcher`; docs test: both strings of
  `switcher_chord_labels()` appear in `docs/user-guide/terminal-panes.md` and
  `docs/user-guide/worktrees-and-sessions.md` (FR-017, SC-006; stays red until T020)
- [X] T006 Add `PhysicalKey::KeyP`, the `p`/`P` arm of `logical_physical`, `pub fn
  is_switcher_chord(input: &KeyInput) -> bool` (`is_primary_shift(mods)` and physical `KeyP`, the
  one place the chord is chosen), `pub fn switcher_chord_labels() -> (String, String)` and
  `KeyOutput::ToggleSwitcher` checked in `encode`'s tier 1 beside `Navigate` and `ToggleOverview`
  in `crates/micold-client/src/keymap.rs`; `KeyRouting::ToggleSwitcher` and its arm in `route_key`
  before the `terminal_focused` check (like `ToggleOverview`) in `crates/micold-client/src/app.rs`;
  `Code::KeyP → PhysicalKey::KeyP` in `to_keymap_physical` in
  `crates/micold-client/src/ui/material/terminal_pane.rs`; until T005 passes except its docs test
- [X] T007 [P] Add `pub(crate) fn location_node(state: &State, location: &SessionLocation)` in
  `crates/micold-client/src/features/navigation.rs` returning the sidebar's node for the location
  (name, tags, `unfolded_sessions()`, `main_session`), and refactor `main_session_at` and
  `session_step` onto it, removing their copies of the `SidebarEntry` match (research R4); the
  existing `tests/navigation.rs` and `tests/navigation_apply.rs` stay green
- [X] T008 [P] Factor the apply sequence out of `crates/micold-client/src/shell/navigation.rs` into
  one `pub(crate) fn apply_step(app: &mut App, step: Step, terminal_had_focus: bool) ->
  Task<Message>` (`State::apply_navigation`, dispatch its follow-ups through `update`,
  `finish_navigation`) that `shell::navigation::update` calls; behaviour unchanged, the existing
  `tests/navigation_apply.rs` stays green

**Checkpoint**: core rules green under `mise run test-core`; the chord decodes and routes; the
#803 tests still pass.

---

## Phase 3: User Story 1 — Jump to a location of the active project (Priority: P1) 🎯 MVP

**Goal**: Ctrl/Cmd+Shift+P opens a dialog listing the active project's locations; typing filters
them, Enter or a click makes one current exactly as a row click does.

**Independent Test**: spec.md US1 *Independent Test*: Default and ten worktrees each with a
session; from a focused terminal press the chord, type three letters of the seventh worktree's
name, Enter; the terminal shows its session and its sidebar row is selected.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

- [X] T009 [P] [US1] Write the failing `candidates` tests (location entries) in
  `crates/micold-client/tests/quick_switch.rs`: in `navigation::location_order` (Default first,
  then the worktrees as drawn); a worktree the tag filter hides is absent but the current location
  under the filter is listed; a collapsed run group's worktrees follow `location_order`; a missing
  and an invalid worktree are listed with `missing` / `invalid` in `context`; names are `"Default"`
  or the node's display name; `context` holds the tag labels joined by ` · ` (data-model.md
  *Entry*: "location: its tag labels (`naming::Tag`), e.g. `feat · PROJ-12 · missing`"); `current`
  is set on the current location only (`current_session_location()` or `sidebar.nav_stop`); every
  location entry is `selectable`; with no active project the list has no location entries
- [X] T010 [US1] Write the failing reducer tests in `crates/micold-client/tests/quick_switch.rs`
  (contracts/ui.md U1, U3 location rows, U4): `Toggled` opens with `query == ""`, `highlighted` =
  the first entry, `terminal_had_focus` from the message and `keyboard_row_before` =
  `sidebar.keyboard_row`; it does nothing while Settings is open (`settings_draft.is_some()`) or
  while any registered surface is open (a dialog, a context menu, the project switcher panel, the
  filter panel: `registry::topmost(state).is_some()`); `Toggled` while open and `Dismissed` close
  it and restore `sidebar.keyboard_row`, current location and session unchanged (FR-009);
  `QueryChanged` sets the query and re-seats the highlight on the first selectable result;
  `Moved(Next/Previous)` steps and stops at the ends; `Chosen(Location(l))` with a main session
  closes and returns `Step::Session(main)`, without one closes and returns `Step::Stop(l)`; a
  `Chosen` key not in a fresh `candidates` returns `None` and stays open with the query kept
  (FR-013); a list change (a worktree added while open) keeps the query and keeps the highlight on
  the same key, and moves it to the first entry when that key leaves; two `State`s are independent
  (FR-015); while open, `navigation::update` returns `Step::None` for every #803 chord (FR-012);
  Tab (the type-ahead's `Dismiss` intent) closes as `Dismissed` does; `list` reports
  `Empty::NoMatches` for a non-empty query that matches nothing and `Empty::NothingToSwitchTo` with
  no entries, and Enter (`Chosen` of no highlight) then changes nothing (FR-014); SC-001 for
  locations: in a project of 50 locations, for each location some query of at most 4 characters of
  its name leaves it within the first 3 selectable entries (at most 2 `Moved(Next)`)
- [X] T011 [US1] Write the failing apply tests in `crates/micold-client/tests/quick_switch.rs`
  driving the switcher through `apply_navigation` / `finish_navigation` as
  `tests/navigation_apply.rs` does (reuse its helpers; move them to `tests/support/` when both need
  them): choosing a location with a session makes it `session.active`, selects its row, arms the
  reveal scroll and, with `terminal_had_focus` true, gives the terminal the keyboard; with it false,
  `sidebar.keyboard_row` is the chosen row (FR-010, SC-004); choosing a location without a session
  starts no session, makes it the stop and puts `keyboard_row` on it (US1-7); with the Overview open
  the resolved choice says to close it before the `Stop` is applied (`Choice::close_overview_first`,
  decided in `features/quick_switch.rs`, research R5), and applied that way the Overview is closed
  and the stop's row is revealed; the running-session count is unchanged across open,
  filter, choose and close (FR-011, SC-005)
- [ ] T012 [P] [US1] Write the failing registry tests in
  `crates/micold-client/tests/overlay_registry.rs`: `QuickSwitcherDialog` is open exactly when
  `state.quick_switch.open.is_some()`, sits on `Layer::Dialog`, its cancellation is
  `Message::QuickSwitch(Msg::Dismissed)`, and Escape and an outside click while it is topmost
  resolve to that cancellation (FR-009); and in `crates/micold-client/tests/showcase_completeness.rs`
  the quick-switcher dialog variant is listed (Principle VIII); and the routing tests beside the
  #803 ones in `crates/micold-client/src/ui/material/terminal_pane.rs` and
  `crates/micold-client/src/ui/mod.rs` (contracts/keys.md K3): with a terminal focused the chord
  publishes `Toggled { terminal_had_focus: true }` and is captured (0 PTY bytes); unfocused,
  `app_chord` gives `Toggled { terminal_had_focus: false }`; pressed while the switcher's field has
  the keyboard, it still reaches `Toggled` (research R8)

### Implementation for User Story 1

- [X] T013 [US1] Implement `pub fn candidates(state: &State) -> Vec<Entry>` in
  `crates/micold-client/src/features/quick_switch.rs` for location entries (data-model.md
  `candidates` step 1 without its sessions): one entry per `navigation::location_order(state)` via
  `navigation::location_node`, `parent: None`, `selectable: true`, `context` from the tag labels,
  `current` as T009 states; until T009 passes
- [X] T014 [US1] Implement `pub fn update(state: &mut State, msg: Msg) -> Option<Choice>` in
  `crates/micold-client/src/features/quick_switch.rs` returning `Option<Choice>` (`pub struct
  Choice { pub step: Step, pub terminal_had_focus: bool, pub close_overview_first: bool }`,
  `close_overview_first` true only for a `Stop` with the Overview open) with the U1 gate, U2
  derivation (`candidates`, `quick_switch::filter`, `reseat`, never stored), U3 resolution for keys
  not listed and location keys, and U4 restore; and `pub fn list(state: &State) -> List` (`entries:
  Vec<Entry>`, `ranked: Vec<Ranked>`, `highlighted: Option<usize>`, `empty: Option<Empty>` with
  `pub enum Empty { NoMatches, NothingToSwitchTo }`) the view and `update` both read; until T010
  passes
- [ ] T015 [US1] Add `QuickSwitcherDialog` (`floating_surface!` with `id:
  SurfaceId::new("quick_switcher")`, `layer: Dialog`, `cancelled_by:
  Message::QuickSwitch(Msg::Dismissed)`, `Registered::open_in` reading
  `state.quick_switch.open.is_some()`) in `crates/micold-client/src/features/quick_switch.rs`, and
  its one `REGISTERED` line with `crate::ui::quick_switcher::dialog` in
  `crates/micold-client/src/overlay/registry.rs` (contracts/ui.md U6); until T012's registry tests
  pass
- [ ] T016 [US1] Create `crates/micold-client/src/ui/quick_switcher.rs` (registered in
  `crates/micold-client/src/ui/mod.rs`) with `pub const QUICK_SWITCHER_FIELD_ID` and `pub fn
  dialog(state: &State) -> Element<Message>` per contracts/ui.md U6: `Surface::new(
  material::dialog::fields(column![typeahead]), SurfaceKind::Dialog, roles)` (no `actions` row)
  holding `material::Typeahead::new(query, rows, QueryChanged, roles).open(true).highlighted(i)
  .selected(current).empty_message(..).on_move(Moved).on_pick(Chosen(key)).on_dismiss(Dismissed)`;
  each row `picker::Row::new(name, spans).details("<Kind> · <context>")`, `.disabled()` when not
  selectable; the empty message renders `List::empty` ("No matches" / "Nothing to switch to",
  FR-014). Glue only: every decision comes from `quick_switch::list`
- [ ] T017 [US1] Create `crates/micold-client/src/shell/quick_switch.rs` (registered in
  `crates/micold-client/src/shell/mod.rs`) with `pub fn update(app: &mut App, msg: Msg) ->
  Task<Message>`: run `features::quick_switch::update`; on open return
  `operation::focus(QUICK_SWITCHER_FIELD_ID)`; for a returned `Choice`, dispatch the Overview's
  close when `close_overview_first`, then `shell::navigation::apply_step(app, choice.step,
  choice.terminal_had_focus)`; no decision of its own; route `Message::QuickSwitch` to it in
  `crates/micold-client/src/main.rs` beside `Message::Navigation`; until T011 passes
- [ ] T018 [US1] Wire the chord's two routes (contracts/keys.md K3): `KeyRouting::ToggleSwitcher`
  in `crates/micold-client/src/ui/material/terminal_pane.rs` publishes
  `Message::QuickSwitch(Msg::Toggled { terminal_had_focus: true })` and captures the event (0 PTY
  bytes); `app_chord` in `crates/micold-client/src/ui/mod.rs` maps `is_switcher_chord` to `Toggled
  { terminal_had_focus: false }`; if the text field captures the chord, the dialog's content maps
  the press itself (research R8); until T012's routing tests pass
- [ ] T019 [US1] Add the quick-switcher to the showcase's dialogs section in
  `crates/micold-client/src/showcase/sections/` and `crates/micold-client/src/showcase/catalogue.rs`
  (sample rows: a query with an emphasised match, a location row with tags, a current row; the
  session row is T025's, the unavailable project T030's) until T012's showcase test passes
- [ ] T020 [US1] Document the quick-switcher (contracts/keys.md K4, research R10): in
  `docs/user-guide/terminal-panes.md` §Keyboard shortcuts add the row "Quick-switcher (jump to a
  location, session or project)" with `Ctrl+Shift+P` and `Cmd+Shift+P`; in
  `docs/user-guide/worktrees-and-sessions.md` §Interacting with the terminal add a
  **Quick-switcher** bullet beside **Navigation**: it lists the active project's locations (tag
  filter applies), in-order letter matching ignoring case with prefix matches first,
  Up/Down/Enter/click to choose, Escape / the chord / a click outside to close, choosing is a row
  click and never starts a session; until T005's docs test passes

**Checkpoint**: US1 works end to end; `mise run gate` green.

---

## Phase 4: User Story 2 — Jump to a session (Priority: P2)

**Goal**: each location entry is followed by its open sessions; a session can be found by its own
name, and a location match keeps its sessions listed after it.

**Independent Test**: spec.md US2 *Independent Test*: three locations with a main session and two
helpers each; type part of a helper of the third location, Enter; that helper shows and its
sub-row is selected under an opened row.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T021 [P] [US2] Write the failing core tests in `crates/micold-core/src/quick_switch.rs`
  (research R3, clarification D5): a location that matches pulls in its sessions that did not
  match, right after it, in input (session) order, with the location's rank and empty spans; a
  session that matched on its own name keeps its own rank and place and is not listed twice; a
  session whose location did not match stays only if it matched itself; with an empty query every
  session follows its location in input order
- [ ] T022 [P] [US2] Write the failing `candidates` tests (session entries) in
  `crates/micold-client/tests/quick_switch.rs`: below each location its open sessions in session
  order (`unfolded_sessions()`, main first), named `session.label.display()`, `context` = the
  location's name, `parent` = the location's index, `current` only on `session.active`; a session
  closed in the sidebar is absent (US2-4); the session of a collapsed (unopened) location row is
  still listed
- [ ] T023 [US2] Write the failing choose tests in `crates/micold-client/tests/quick_switch.rs`:
  `Chosen(Session(id))` closes and returns `Step::Session(id)`; applied, it makes `id`
  `session.active`, opens its location's row and selects its sub-row with the reveal armed
  (US2-2, SC-004); typing a location's name then `Moved(Next)` twice highlights its second session
  (US2-3); a session created while open appears with the query and highlight kept (FR-013)

### Implementation for User Story 2

- [ ] T024 [US2] Extend `filter` in `crates/micold-core/src/quick_switch.rs` with the
  kept-by-location rule (data-model.md: "insert each matched location's unmatched sessions right
  after it with its rank") until T021 passes (`mise run test-core`)
- [ ] T025 [US2] Add session entries to `candidates` in
  `crates/micold-client/src/features/quick_switch.rs` (data-model.md `candidates` step 1), and the
  `Session` arm of U3 in `update`, until T022 and T023 pass; add a session row (`Session ·
  <location>`) to the showcase sample of T019 in `crates/micold-client/src/showcase/sections/`
- [ ] T026 [US2] Update the **Quick-switcher** bullet in `docs/user-guide/worktrees-and-sessions.md`:
  each location is followed by its open sessions, a session is found by its name, a location's
  name keeps its sessions listed after it, closed sessions are not listed

**Checkpoint**: US1 and US2 work; `mise run gate` green.

---

## Phase 5: User Story 3 — Jump to a project (Priority: P3)

**Goal**: after the active project's entries, the other known projects, unavailable ones badged
and unselectable; choosing one switches project as the project switcher does.

**Independent Test**: spec.md US3 *Independent Test*: five known projects, one unavailable; type
part of the fourth's name, Enter; it becomes active; the unavailable one is listed and cannot be
chosen.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T027 [P] [US3] Write the failing `candidates` tests (project entries) in
  `crates/micold-client/tests/quick_switch.rs`: after the active project's locations and sessions,
  every `state.switcher_entries()` entry with `!is_active` in that order (unavailable ones
  included), named by its `label`, `context` = `"<n> running · <m> unread"` plus `unavailable` for
  an unavailable one, `selectable == available`; the active project is not a project entry
  (US3-4); no location or session of another project is listed (US3-5, D4); on the welcome screen
  the list is every known project; with no known project and no active one the list is empty
- [ ] T028 [US3] Write the failing reducer and apply tests in
  `crates/micold-client/tests/quick_switch.rs`: `Chosen(Project(p))` for an available project
  closes and returns `Step::Project(p)`; applied, `p` is active, its current session is restored
  and the previous project's sessions keep running (count unchanged, US3-2, SC-005); for an
  unavailable project it returns `None` and stays open; `Moved` never highlights it and opening with
  an unavailable project first highlights the first selectable entry (D5); `Moved(Next)` at the
  last available project stays there; choosing a project with the Overview open leaves it as
  `Step::Project` already does (`close_overview_first` false)

### Implementation for User Story 3

- [ ] T029 [US3] Add project entries to `candidates` in
  `crates/micold-client/src/features/quick_switch.rs` (data-model.md `candidates` step 2) and the
  two `Project` arms of U3 in `update`, until T027 and T028 pass
- [ ] T030 [US3] Add an unavailable project row (disabled, `Project · unavailable`) to the
  showcase sample of T019 in `crates/micold-client/src/showcase/sections/`
- [ ] T031 [US3] Complete the **Quick-switcher** bullet in
  `docs/user-guide/worktrees-and-sessions.md`: the other known projects follow with their running
  and unread counts, unavailable ones are shown but cannot be chosen, only the active project's
  locations and sessions are listed; check that the bullet and the `terminal-panes.md` row now
  describe what the switcher lists and how to filter and choose (US4 acceptance scenario 1)

**Checkpoint**: all stories work; `mise run gate` green.

---

## Phase 6: User Story 4 — Find the shortcut (Priority: P3)

US4 needs no task of its own: the docs test (T005), the shortcut row and bullet (T020) and their
completion (T026, T031) ship with the behaviour they describe (Principle VII).

---

## Phase 7: Polish & Cross-Cutting Concerns

No code. These are the close unit's (milestones.md rule 4).

- [ ] T032 Run quickstart §B B1–B12 with the `visual-pass` skill and record the evidence in
  `specs/968-quick-switcher/visual-pass/`
- [ ] T033 Record quickstart §C (macOS, Windows, a non-US layout) in
  `specs/968-quick-switcher/quickstart.md`, or note it as not run with the CI evidence of the three
  platforms
- [ ] T034 `speckit-converge` against spec.md, and tick the tasks it confirms

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: none.
- **Foundational (Phase 2)**: after Setup; blocks every story. T003 before T004; T005 before T006.
- **US1 (Phase 3)**: after Foundational. T009–T012 before T013–T019; T013 before T014; T014
  before T015–T017; T016 before T015 (its registry line names `ui::quick_switcher::dialog`);
  T018 after T017; T020 last (closes T005's docs test).
- **US2 (Phase 4)**: after US1 (extends `filter`, `candidates`, `update` and the doc bullet).
- **US3 (Phase 5)**: after US2 (the project entries follow the sessions; T030 extends the showcase
  sample, T031 the bullet).
- **US4 (Phase 6)**: covered by T005, T020, T026, T031.
- **Polish (Phase 7)**: after all stories; close unit.

### Within each story

- The failing test task comes first and must fail for the right reason.
- Core before client; `candidates` before `update`; `update` before the surface, view and shell.
- The user-guide task ships in the story's milestone.

### Parallel Opportunities

- T001 ∥ T002 (core vs client files).
- T003 ∥ T005 ∥ T007 ∥ T008 (core, keymap, `features/navigation.rs`, `shell/navigation.rs`).
- T009 ∥ T012 (`tests/quick_switch.rs` vs `tests/overlay_registry.rs` and the routing files);
  T010 and T011 share `tests/quick_switch.rs` with T009, so they follow it in sequence.
- T022 and T023 share `tests/quick_switch.rs`; so do T027 and T028: each pair runs in sequence.
- T021 ∥ T022 (core vs client tests).

---

## Parallel Example: User Story 1

```bash
Task: "T009 candidates tests (locations) in crates/micold-client/tests/quick_switch.rs"
Task: "T012 registry and showcase tests in crates/micold-client/tests/overlay_registry.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1)

1. Setup and Foundational (T001–T008).
2. US1 (T009–T020): the switcher over locations, end to end.
3. **Validate**: US1 *Independent Test*; `mise run gate`.

### Incremental Delivery

1. M1: locations → merge.
2. M2: sessions → merge.
3. M3: projects → merge.
4. Close unit: quickstart §B/§C and convergence (T032–T034).

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Quick-switcher over the active project's locations 🎯 MVP

- **Tasks**: T001–T020
- **Deliverable**: Ctrl/Cmd+Shift+P (from a focused terminal with 0 bytes to it, or not) opens a
  dialog listing the active project's locations in sidebar order with their tags and the current
  one marked; typing filters them by in-order letters (prefix, then run, then scattered); Up/Down,
  Enter or a click makes one current exactly as a row click does (a session-less one becomes the
  stop, no session started); Escape, the chord or an outside click closes it and returns the
  keyboard; nothing opens while Settings or another surface is open; both shortcut lists name it
- **Satisfies**: US1 acceptance scenarios 1–8; FR-001, FR-002, FR-003 (locations), FR-004
  (locations), FR-005 (own-name ranking), FR-006 (locations), FR-007, FR-008 (locations), FR-009,
  FR-010, FR-011, FR-012, FR-013, FR-014, FR-015, FR-016, FR-017 (row and bullet); SC-001
  (locations), SC-002, SC-003, SC-004 (locations), SC-005, SC-006
- **Verify**: `mise run test-core` (`quick_switch` tests incl. the 500-entry timing) and `mise run
  gate` (`tests/keymap.rs`, `tests/quick_switch.rs`, `tests/overlay_registry.rs`,
  `tests/showcase_completeness.rs`, `tests/navigation.rs`, `tests/navigation_apply.rs`, the
  `terminal_pane.rs` / `ui/mod.rs` routing tests); quickstart §B B1, B2 (locations), B4, B7–B10
- **Depends on**: —
- **Tier**: full
- **Note**: not split further. The chord, the dialog, the list and the choose path are one
  observable loop: the chord without the dialog, or the dialog without choosing, is half-wired UI
  (rule 6), and the core rules alone show nothing (rule 1). Sessions and projects, the separable
  parts, are M2 and M3.

### M2 — Sessions in the quick-switcher

- **Tasks**: T021–T026
- **Deliverable**: the quick-switcher lists each location's open sessions below it; typing a
  helper's name and Enter shows that helper with its sub-row selected under the opened row; typing
  a worktree's name keeps its sessions right after it; the user guide says so
- **Satisfies**: US2 acceptance scenarios 1–4; FR-003 (sessions), FR-004 (sessions), FR-005
  (kept-by-location), FR-006 (sessions), FR-008 (sessions), FR-013 (sessions); SC-001 (sessions),
  SC-004 (sessions)
- **Verify**: `mise run test-core` (kept-by-location tests) and `mise run gate`
  (`tests/quick_switch.rs` session tests, `tests/showcase_completeness.rs`); quickstart §B B2, B3, B11
- **Depends on**: M1
- **Tier**: full (T024 is a new ranking rule in core, not an existing pattern)

### M3 — Projects in the quick-switcher

- **Tasks**: T027–T031
- **Deliverable**: after the active project's entries the quick-switcher lists the other known
  projects with their running and unread counts; Enter switches project as the project switcher
  does; an unavailable project is shown with its badge and cannot be highlighted or chosen; on the
  welcome screen it lists every known project; the user guide describes the complete switcher
- **Satisfies**: US3 acceptance scenarios 1–5; US4 acceptance scenario 1; FR-003 (projects),
  FR-004 (projects), FR-006 (projects), FR-008 (projects); SC-004 (projects), SC-005 (project
  switch)
- **Verify**: `mise run gate` (`tests/quick_switch.rs` project tests,
  `tests/showcase_completeness.rs`); quickstart §B B5, B6, B12
- **Depends on**: M1, M2
- **Tier**: light (follows `candidates`/`update` from M1–M2 and `State::switcher_entries`, which
  plan.md D3 and data-model.md name; `Step::Project` is #803's)

Polish T032–T034 changes no code: the close unit does it in the close PR.
