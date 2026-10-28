---

description: "Task list for #1465 Back/forward navigation"
---

# Tasks: Back/Forward Navigation (Mouse Side Buttons and Alt+←/→)

**Input**: Design documents from `specs/1465-back-forward-navigation/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/keys.md](./contracts/keys.md),
[contracts/ui.md](./contracts/ui.md), [quickstart.md](./quickstart.md)

**Tests**: Constitution Principle I — every story's tests are written first and fail before its
implementation tasks (Red-Green-Refactor).

**Documentation**: Constitution Principle VII — each story's user-guide change is in the story
(`docs/user-guide/terminal-panes.md` §Keyboard shortcuts, plan D7), so every `feat` milestone
passes CI's user-guide gate.

**Cross-platform**: Principle VI — the one platform split is `cfg(target_os = "macos")` inside
`keymap.rs` (H1); everything else is platform-agnostic.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1…US5)

Paths are relative to the repository root; the client crate is `crates/micold-client/`.

---

## Phase 1: Setup (Shared Infrastructure)

No setup: existing Cargo workspace, no new crate or dependency (plan *Technical Context*).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the history list, the feature's state and the recording of every move (U1). Nothing a
user can press yet.

- [x] T001 [P] Write failing unit tests in `crates/micold-core/src/history.rs` (`#[cfg(test)] mod tests`), contracts/ui.md U7 core list: `record` of a place equal to `current()` adds nothing (FR-003); recording after a Back drops every forward entry (US1 AS3, FR-004); 60 records keep 50 (`CAP = 50`), the oldest dropped, and Back stops at the oldest kept (FR-005, SC-004); `step` Back/Forward at either end returns `None` with `position()` unchanged (US1 AS6); A→B→C with B gone, Back from C returns A and B is removed (US4 AS1, FR-007); every entry behind gone → `None` and they are removed (US4 AS3); A→B→A→C with B gone merges the neighbouring As and Back from C lands on A once; an entry equal to `shown` is passed over and kept, never returned (US4 AS4); `step` never records; SC-002 sweep: for every subset of gone entries in a five-entry list and every position, each `step` returns an existing entry different from `shown`, or `None`
- [x] T002 Implement `Dir { Back, Forward }`, `View { Session { location: SessionLocation, id: SessionId }, Review(SessionLocation), Overview, Compare(GroupId) }`, `Place { project: PathBuf, view: View }` (derive `Clone, PartialEq, Eq, Debug`) and `History<P>` in `crates/micold-core/src/history.rs`, with `pub mod history;` in `crates/micold-core/src/lib.rs`, per data-model.md: private `entries: Vec<P>` and `pos: usize`; invariants "`len() <= CAP` (50, FR-005, SC-004); no two neighbouring entries equal" and "`pos < entries.len()` when non-empty; `0` when empty"; `History::default()` empty; `record(p)`, `step(dir, shown: Option<&P>, exists: impl FnMut(&P) -> bool) -> Option<P>` (rejected entries removed with `pos` adjusted, neighbouring equal entries merged keeping `pos` on the merged entry), `current() -> Option<&P>`, `entries() -> &[P]`, `position() -> usize`. T001 green with `mise run test-core`
- [x] T003 [P] Extract `pub fn blocked(state: &crate::app::State) -> bool` from the first line of `navigation::update` in `crates/micold-client/src/features/navigation.rs` (line 83) and make `update` call it; no behaviour change, `crates/micold-client/tests/features_navigation.rs` and `tests/navigation.rs` stay green
- [x] T004 [P] Extract `State::live_focused_field(&self) -> Option<FieldId>` in `crates/micold-client/src/app.rs` from the field rule inside `changes::keys::review_holder` (`crates/micold-client/src/features/changes/keys.rs` line 38): `focused_field`, except a `ReviewComposer` whose comment box is closed and a `Composer(_)` / `QueuedMessage(_)` while the Review tab is shown (research R8); refactor `review_holder` onto it; `crates/micold-client/tests/review_keys.rs` and the existing review-holder tests stay green
- [x] T005 Add the render-free feature `crates/micold-client/src/features/history.rs` with `State { list: micold_core::history::History<Place>, moving: bool }`, `Source { Button, Key }`, `Msg::Pressed { dir: Dir, source: Source }` and `Step { None, ReviewBack, Show(Place) }` (types only); register it in `crates/micold-client/src/features/mod.rs`; root `State.history: history::State` and `Message::History(history::Msg)` in `crates/micold-client/src/app.rs`, declined by `State::update` beside `Message::Navigation(_)` (app.rs line 1349, shape B); add the isolation test `crates/micold-client/tests/features_history.rs`, and update `tests/feature_registration_cost.rs` (feature list; `ALLOWED_CROSS_FEATURE_NAMES` for `history` → `navigation` (`blocked`) only; the `changes` entry comes with its first use in T025), `tests/feature_write_isolation.rs` and `tests/features_are_render_free.rs` as they require
- [x] T006 Write failing tests in `crates/micold-client/tests/history.rs` (new; root `State`, no window), contracts/ui.md U1 (FR-002): one recorded entry per mover — sidebar `Selected`, quick switcher choice, navigation chord step, Overview `Toggled`, `CompareOpened`, `ReviewChangesRequested`, and the shown session's removal moving the shown place; a background `Started` or a catalog update of another session records nothing; re-selecting the shown session records nothing (FR-003); `observe_place` while `history.moving` records nothing; two root `State`s keep independent histories (FR-001)
- [x] T007 Implement `State::shown_place() -> Option<Place>` (research R3's order) and `State::observe_place()` in `crates/micold-client/src/app.rs`, and `history::observe(state, shown)` in `crates/micold-client/src/features/history.rs` (`moving` → nothing; `Some(p)` → `list.record(p)`; `None` → nothing); call `app.core.observe_place()` after `update_inner` beside `follow_header` in `crates/micold-client/src/main.rs` (line 485). T006 green

**Checkpoint**: every move is recorded; nothing walks the history yet.

---

## Phase 3: User Story 1 - Mouse back/forward between places (Priority: P1) 🎯 MVP

**Goal**: the mouse's Back/Forward side buttons walk the history from anywhere in the window, in
the active project.

**Independent Test**: select session A, then B; mouse back shows A; mouse forward shows B
(quickstart B1).

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

- [x] T008 [P] [US1] Write failing unit tests for `side_button(event, status, window) -> Option<Message>` in `crates/micold-client/src/shell/subscriptions.rs` (`#[cfg(test)]`), contracts/keys.md H4: `Mouse(ButtonPressed(Back))` → `History(Pressed { dir: Back, source: Button })`, `Forward` likewise, each with `Status::Captured` and `Status::Ignored`; releases, other buttons and other events → `None`
- [x] T009 [P] [US1] Write tests in the in-crate headless `dispatch` harness of `crates/micold-client/src/ui/material/terminal_pane.rs` (beside `no_press_outside_the_pane_reaches_the_process`): Back and Forward presses inside a focused pane, with and without mouse-reporting mode, publish nothing that `reaches_the_process` (FR-008)
- [x] T010 [US1] Write failing tests in `crates/micold-client/tests/history.rs`: contracts/ui.md U3 gate rows with `Source::Button` — Settings open, a dialog open, the quick switcher open, the Review sheet open → `Step::None` and nothing moves (FR-010); a live focused field → the press acts (FR-008); U5 in the active project — an archived session, a worktree removed from `worktree.worktrees` → skipped and removed (US4 AS1, FR-007); a place of a non-active project → skipped and removed (until T031 replaces this test); U4 `apply_history` for each target view in the active project: `Session { id, .. }` → `[Session(Selected(id))]`; `Review(location)` → `Overview(Closed)` if open, `Runs(CompareClosed)` if open, `Sidebar(ReviewChangesRequested(location))`; `Overview` → `Runs(CompareClosed)` if open, `Overview(Opened { settings_open: false })`; `Compare(group)` → `Overview(Closed)` if open, `Runs(CompareOpened { group, project })`; `finish_history_move` records only a place different from the target
- [x] T011 [US1] Write failing end-to-end tests in `crates/micold-client/src/main_tests.rs` through the real root `update` with `Message::History(Pressed { source: Button })`: A → B → C, Back, Back, select D: forward does nothing and the entries are A, D (US1 AS1–AS3); Overview open → Back → the previous session with the Overview closed; Back from Compare to a Review tab

### Implementation for User Story 1

- [x] T012 [US1] Implement in `crates/micold-client/src/features/history.rs`: `resolve(app: &mut crate::app::State, msg) -> Step` — `crate::features::navigation::blocked(app)` → `None` (spelled in full so the T005 allowlist entry counts as live); else, with the borrows split (`let shown = app.shown_place(); let mut list = std::mem::take(&mut app.history.list); let found = list.step(dir, shown.as_ref(), |p| exists(app, p)); app.history.list = list;`), `Show(p)` with `moving = true`, or `None`; `exists(app: &crate::app::State, place) -> bool` for the active project per research R5 (Default always; `Worktree(dir)` listed in `worktree.worktrees`; a session is a non-archived record in `workspace.sessions[project]` at that location; Review: its location exists; Overview: its project; Compare: its group in `runs.groups`; a place of another project → `false` until T031); `finish(state, shown)` — `moving = false`, then `observe`. In `crates/micold-client/src/app.rs`: `State::press_history(msg) -> Step`, `State::finish_history_move()`, `State::apply_history(step) -> Vec<Message>` with the U4 rows for the active project. T010 green
- [x] T013 [US1] Add `crates/micold-client/src/shell/history.rs` (shape B: `pub fn update(app: &mut App, msg: history::Msg) -> Task<Message>`): `app.core.press_history(msg)`, each of `app.core.apply_history(step)`'s messages through `crate::update`, then `app.core.finish_history_move()` for `Step::Show` (contracts/ui.md U2 steps 1–3); `pub mod history;` in `crates/micold-client/src/shell/mod.rs`; route `Message::History(msg) => shell::history::update(app, msg)` beside `Message::Navigation` in `crates/micold-client/src/main.rs` (line 1322). T011 green
- [x] T014 [US1] Implement `side_button` in `crates/micold-client/src/shell/subscriptions.rs` (H4; `status` ignored so a captured press still navigates) and push an always-on `iced::event::listen_with(side_button)` into `subscription`'s batch. T008 green
- [x] T015 [US1] Make T009 green in `crates/micold-client/src/ui/material/terminal_pane.rs`: if a Back/Forward press reaches the process, report only Left, Middle and Right to a mouse-reporting program (research R7); otherwise no code change
- [x] T016 [US1] In `docs/user-guide/terminal-panes.md` §Keyboard shortcuts, add the rows "Back" and "Forward" to the app-chord table with "Mouse back button" / "Mouse forward button" in both key cells (true as shipped until T023 adds the keys), and add to the intro that the mouse's side buttons go back and forward everywhere in the window, over a terminal too, and the terminal program never receives them (FR-008, FR-013)

**Checkpoint**: US1 works with the mouse in the active project (quickstart B1–B3, B9, B11).

---

## Phase 4: User Story 2 - Alt+← / Alt+→ from the keyboard (Priority: P1)

**Goal**: Alt+←/→ (Cmd+[ / Cmd+] on macOS) walk the same history, except while a text field or a
terminal holds the keyboard.

**Independent Test**: select A, then B with nothing focused; Alt+← shows A; Alt+→ shows B
(quickstart B4, B5, B7).

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [x] T017 [P] [US2] Write failing tests in `crates/micold-client/tests/keymap.rs`, contracts/keys.md H5: `history_key` decodes Back and Forward on the running platform's modifier set (Alt+←/→; macOS Cmd+[ / Cmd+] by physical `BracketLeft`/`BracketRight` and by the logical `[` / `]` with `PhysicalKey::Other`); every H1 near miss is `None` (Alt+Shift+←, Ctrl+Alt+←, Alt+↑, plain ←; macOS Cmd+Shift+[, Option+←/→, Cmd+Option+[); the four H2 `encode` rows keep their output (SC-003); no existing chord (`nav_chord`, `is_switcher_chord`, `is_overview_chord`, pane, copy/paste, release, new terminal) decodes under `history_key` and no `history_key` press decodes under them; docs test: both strings of each `history_key_labels(dir)` appear in `docs/user-guide/terminal-panes.md`'s app-chord table on the row whose action is "Back" / "Forward", next to "mouse back button" / "mouse forward button" (case-insensitive) (FR-013)
- [x] T018 [P] [US2] Write failing tests in `crates/micold-client/src/ui/app_chord_tests.rs`: `app_chord` gives `History(Pressed { dir, source: Key })` for each H1 press and nothing for the near misses; and in `crates/micold-client/src/ui/overview_arrow_tests.rs`: `overview_arrow` gives nothing for an H1 press and a card step for a plain arrow (contracts/keys.md H3)
- [x] T019 [US2] Write failing tests in `crates/micold-client/tests/history.rs`, contracts/ui.md U3 key rows: `Source::Key` with a live focused field → `None`; with a stale `ReviewComposer` (comment box closed) → acts; with a focused terminal → `None`; with Settings open → `None`; three successive `Key` presses step back three entries (FR-012, US2 AS4)

### Implementation for User Story 2

- [x] T020 [US2] Implement in `crates/micold-client/src/keymap.rs`, beside `is_primary_shift`: `history_key(&KeyInput) -> Option<Dir>` (H1; `#[cfg(target_os = "macos")]` split: Cmd + physical `BracketLeft`/`BracketRight`, logical fallback as `nav_chord`'s; else Alt + `NamedKey::ArrowLeft`/`ArrowRight`, modifiers exactly), `history_key_labels(Dir) -> (String, String)` = `("Alt+←", "Cmd+[")` / `("Alt+→", "Cmd+]")`, `history_key_label(Dir) -> String` (the running platform's form); `encode` unchanged
- [x] T021 [US2] In `crates/micold-client/src/ui/mod.rs`: `app_chord` (line 1603) checks `history_key` after the existing chords and publishes `Message::History(Pressed { dir, source: Key })`; `overview_arrow` (line 1552) returns `None` for a press `history_key` decodes. T018 green
- [x] T022 [US2] Add the key gate row to `resolve` in `crates/micold-client/src/features/history.rs`, after `blocked`: `source == Source::Key` and (`app.terminal_focused()` or `app.live_focused_field().is_some()`) → `Step::None` (FR-009, contracts/ui.md U3). T017's decoding tests and T019 green (T017's docs test turns green in T023)
- [x] T023 [US2] In `docs/user-guide/terminal-panes.md` §Keyboard shortcuts, fill the Back and Forward rows' key cells with `Alt+← or the mouse back button` / `Cmd+[ or the mouse back button` and `Alt+→ or the mouse forward button` / `Cmd+] or the mouse forward button`, and add to the intro's "The app takes every chord below" the exception: a focused text field or terminal keeps Back and Forward's keys (Option+←/→ on macOS stays word movement) (FR-013). T017's docs test green

**Checkpoint**: US1 and US2 work in the active project (quickstart B1–B5, B7, B9, B11, B13).

---

## Phase 5: User Story 3 - A view's own back comes first (Priority: P2)

**Goal**: in a Review tab with a browse frame, Back performs the tab's own **Back**; only then the
global history.

**Independent Test**: open a browsed file from the Review tab's diff; Back shows the diff in the same
tab; Back again shows the previous place (quickstart B6).

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [x] T024 [US3] Write failing tests in `crates/micold-client/tests/history.rs` (FR-011): a shown Review tab with a browse frame, Back (Button and Key) → `Step::ReviewBack` with `history.list.position()` unchanged and `apply_history(ReviewBack)` → `[Changes(BrowseBack)]`; without a browse frame → the global Back; Forward from a Review tab → the global Forward; and in `crates/micold-client/src/main_tests.rs`: from a browsed file, Back returns to the tab's previous reading in one press and the next Back leaves the tab (SC-005)

### Implementation for User Story 3

- [x] T025 [US3] In `crates/micold-client/src/features/history.rs` `resolve`, before the `step` row: `Dir::Back`, the shown place a Review tab and `crate::features::changes::can_go_back` on its view → `Step::ReviewBack` (list untouched, `moving` unset); in `crates/micold-client/src/app.rs` `apply_history`: `ReviewBack` → `[Message::Changes(BrowseBack)]`; add the `ALLOWED_CROSS_FEATURE_NAMES` entry `history` → `changes` (`can_go_back`) in `crates/micold-client/tests/feature_registration_cost.rs`, calling it as `crate::features::changes::can_go_back` so the entry counts as live. T024 green
- [x] T026 [US3] In `docs/user-guide/terminal-panes.md` §Keyboard shortcuts, add one sentence: in the Review tab, Back first does the tab's own **Back** (from a browsed file to what was being read) and leaves the tab only when that has nothing left; Forward always uses the app's history

**Checkpoint**: quickstart B6 passes.

---

## Phase 6: User Story 4 - Gone places are skipped, across projects (Priority: P2)

**Goal**: Back/Forward reach places in other projects (switching project as the switcher does) and
pass over places that are gone there too, deciding before anything moves (research R5).

**Independent Test**: show Q1 in project Q, then a session of P; Back switches to Q and shows Q1
with no intermediate entry; delete a worktree of the other project and its entries are skipped
(quickstart B8, B10).

### Tests for User Story 4 (MANDATORY — Constitution Principle I) ⚠️

- [x] T027 [P] [US4] Write failing tests in `crates/micold-client/tests/features_runs.rs`: `runs.known` holds each project's last `GroupsChanged` list (`known[active] = groups`); `CompareOpened { group, project }` with `project` active, `group` not in `groups` and in `known[project]` sets `groups = known[project].clone()` first and opens Compare
- [x] T028 [US4] Write failing tests in `crates/micold-client/tests/history.rs`, contracts/ui.md U5/U4 with an injected `lookup` map: a worktree of another project missing from `lookup(project)` → skipped and removed; `lookup` → `None` → that project's worktree locations are gone; a project turned unavailable → its places skipped and removed (US4 AS2, AS3); Default of a known, available project always exists; `apply_history` for a target in another project starts with `Project(Reopened(project))`; cross-project Compare: in P record Compare(G), switch to Q, Back → P active and Compare of G open with the forward entries kept (FR-006, FR-007); a Compare entry whose group is gone from `known[P]` is skipped and removed. Replace T010's "non-active project → skipped" test
- [x] T029 [US4] Write failing end-to-end tests in `crates/micold-client/src/main_tests.rs`: cross-project Back switches project and shows the target session, and the entries are as before the press except the position (research R2: no intermediate place recorded); Compare of G in P, show Q1, Back → P with Compare of G open, Forward → Q1

### Implementation for User Story 4

- [x] T030 [P] [US4] In `crates/micold-client/src/features/runs.rs`: `State.known: BTreeMap<PathBuf, Vec<RunGroup>>` ("each project's last group list this launch"); in `routed` (line 1367), `Msg::GroupsChanged(groups)` also sets `known[workspace.active] = groups`, and `Msg::CompareOpened { group, project }` with `project` active, `group` not in `groups` and in `known[project]` sets `groups = known[project].clone()` first, then as today; `known[P]` is not pruned when P is removed (data-model.md). T027 green
- [x] T031 [US4] Extend `crates/micold-client/src/features/history.rs`: `resolve(app, msg, lookup: &dyn Fn(&Path) -> Option<Vec<String>>)` and `exists(app, place, lookup)` per research R5 for every project — the project known and available (`switcher_entries`' `available`, or the active one); `Worktree(dir)` in `worktree.worktrees` for the active project or in `lookup(project)` for another (`None` = unreadable → gone); a session in `workspace.sessions[project]`; Compare in `runs.groups` for the active project or `runs.known[project]` for another; `State::press_history(msg, lookup)` and `apply_history` prefixing `Project(Reopened(project))` when `project` is not the active one, in `crates/micold-client/src/app.rs`. T028 green
- [x] T032 [US4] In `crates/micold-client/src/shell/history.rs`: first `app.core.workspace.refresh_availability(app.caps.scanner())` (contracts/ui.md U2 step 0), then pass `lookup = |project| Some(discover_worktrees(git, project)` mapped to their `dir_name`s`)` (the worktree identity a `SessionLocation::Worktree` holds) (`crates/micold-client/src/shell/workspace.rs` line 91) to `press_history`. T029 green
- [x] T033 [US4] In `docs/user-guide/terminal-panes.md` §Keyboard shortcuts, add one sentence: Back and Forward pass over places that no longer exist (a closed session, a removed worktree, a project closed or unavailable) without an error, and a place in another project switches to that project as the project switcher does

**Checkpoint**: quickstart B8 and B10 pass.

---

## Phase 7: User Story 5 - Discoverable shortcuts (Priority: P3)

**Goal**: the Review tab's shortcut sheet lists Back and Forward (the guide's table already does,
from US1/US2).

**Independent Test**: open the Review sheet with single-key shortcuts on, then off; both list Back
and Forward (quickstart B12).

### Tests for User Story 5 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T034 [P] [US5] Write failing tests in `crates/micold-client/tests/review_keys.rs`: `sheet_model(true)` and `sheet_model(false)` end with `(history_key_label(Back), "Back (also the mouse back button)")` and `(history_key_label(Forward), "Forward (also the mouse forward button)")` (FR-014, contracts/ui.md U6)

### Implementation for User Story 5

- [ ] T035 [US5] In `crates/micold-client/src/features/changes/keys.rs` `sheet_model` (line 500), append the two U6 rows after the `REVIEW_KEYS` rows, for `single_keys` on and off. T034 green
- [ ] T036 [US5] In `docs/user-guide/terminal-panes.md` §Keyboard shortcuts, add that the Review tab's shortcut sheet (`?`) lists Back and Forward too

**Checkpoint**: every story works (quickstart §B).

---

## Phase 8: Polish & Cross-Cutting Concerns

No code. Done by the close unit in the close PR.

- [ ] T037 Run quickstart §B (B1–B13) with the `visual-pass` skill and save the evidence under `specs/1465-back-forward-navigation/`
- [ ] T038 Record quickstart §C (C1–C4, macOS and Windows) results or the CI evidence in `specs/1465-back-forward-navigation/quickstart.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Foundational (Phase 2)**: none; blocks every story.
- **US1 (Phase 3)**: after Phase 2.
- **US2 (Phase 4)**: after US1 (it adds a row to US1's `resolve` and fills US1's guide rows).
- **US3 (Phase 5)**: after US1 (a row in `resolve` and `apply_history`).
- **US4 (Phase 6)**: after US1 (extends `exists`, `resolve`, `apply_history`, `shell/history.rs`).
- **US5 (Phase 7)**: after US2 (`history_key_label`).
- **Polish (Phase 8)**: after all stories.

### Within Each User Story

Tests first, failing; then implementation in task order; then the guide sentence.

### Parallel Opportunities

- T001, T003, T004 touch different files.
- US1: T008 and T009.
- US2: T017 and T018.
- US4: T027 and T030 (runs.rs) beside T028.
- US3, US4 and US5 can be worked in parallel after their dependencies, but merge one milestone at a
  time.

## Parallel Example: User Story 1

```text
T008 side_button tests in shell/subscriptions.rs
T009 terminal pane side-button tests in ui/material/terminal_pane.rs
```

## Implementation Strategy

### MVP First

Phase 2 + US1 (M1): the mouse walks the history of the active project. Stop and validate with
quickstart B1–B3.

### Incremental Delivery

M1 mouse → M2 keys → M3 Review tab's own back → M4 other projects → M5 Review sheet. Each merges
green and adds a behaviour a user can observe.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Mouse back/forward in the active project 🎯 MVP

- **Tasks**: T001–T016
- **Deliverable**: the mouse's Back/Forward side buttons walk a per-window history of sessions, Review tabs, the Overview and Compare in the active project, from anywhere in the window (terminal panes and fields included), skipping gone places; places in other projects are passed over as gone until M4
- **Satisfies**: US1 acceptance scenarios 1–6; US4 AS1, AS3, AS4 (active project); FR-001–FR-008, FR-010, FR-012 (buttons), FR-015 (core and routing tests); SC-002 and SC-004 (core sweep and cap)
- **Verify**: `mise run test-core` (`history::tests`); `cargo test -p micold-client --test history`; `main_tests` A→B→C/Back/Back/D; `shell::subscriptions` `side_button` tests; quickstart B1–B3, B9, B11
- **Depends on**: —
- **Tier**: full

### M2 — Alt+←/→ (Cmd+[ / Cmd+] on macOS)

- **Tasks**: T017–T023
- **Deliverable**: the platform's Back/Forward keys walk the history unless a text field or terminal holds the keyboard, and the user guide's app-chord table lists them
- **Satisfies**: US2 acceptance scenarios 1–4; FR-009, FR-012 (keys), FR-013; SC-001, SC-003
- **Verify**: `cargo test -p micold-client --test keymap`; `app_chord_tests`, `overview_arrow_tests`; `tests/history.rs` key rows; quickstart B4, B5, B7, B13
- **Depends on**: M1
- **Tier**: full

### M3 — The Review tab's own Back first

- **Tasks**: T024–T026
- **Deliverable**: in a Review tab with a browsed file, Back returns to what was being read and only the next Back leaves the tab
- **Satisfies**: US3 acceptance scenarios 1–3; FR-011; SC-005
- **Verify**: `tests/history.rs` FR-011 tests; `main_tests` Review back; quickstart B6
- **Depends on**: M1
- **Tier**: light

### M4 — Back/Forward across projects

- **Tasks**: T027–T033
- **Deliverable**: Back/Forward reach places in other projects, switching project as the switcher does without recording an intermediate place, and pass over that project's gone worktrees, sessions and Compare groups
- **Satisfies**: US4 acceptance scenarios 1–4 in every project; edge cases *Several projects*, *Gone targets*; FR-006, FR-007 across projects; SC-002
- **Verify**: `tests/history.rs` cross-project and injected-`lookup` tests; `tests/features_runs.rs` `known` tests; `main_tests` cross-project Back and Compare; quickstart B8, B10
- **Depends on**: M1
- **Tier**: full

### M5 — Back/Forward in the Review sheet

- **Tasks**: T034–T036
- **Deliverable**: the Review tab's shortcut sheet lists Back and Forward in the platform's form, with single-key shortcuts on and off
- **Satisfies**: US5 acceptance scenarios 1–2; FR-014
- **Verify**: `cargo test -p micold-client --test review_keys`; quickstart B12
- **Depends on**: M2
- **Tier**: light

Polish (T037–T038) changes no code: the close unit does it in the close PR.

## Notes

- [P] tasks = different files, no dependencies
- Verify tests fail before implementing
- `mise run gate` before every push
