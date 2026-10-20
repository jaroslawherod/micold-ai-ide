---

description: "Tasks: Overview cards, full text without truncation"
---

# Tasks: Overview cards, full text without truncation

**Input**: Design documents from `/specs/1121-overview-card-full-text/`

**Prerequisites**: plan.md, spec.md, research.md (R1–R10), data-model.md, contracts/expansion.md
(K1–K6), contracts/card-grid.md (U1–U4), quickstart.md

**Tests**: Constitution Principle I: every test task is written first and run, and must fail for the
stated reason (missing item, or the old rule's output) before the implementation task it covers.
Every bullet of a contract is one assertion. Core tests: `mise run test-core` (without `mise`:
`scripts/build-lock.sh cargo test -p micold-core --all-targets`). Client tests:
`scripts/build-lock.sh cargo test -p micold-client --lib -- <filter>` and
`scripts/build-lock.sh cargo test -p micold-client --test <name>`.

**Documentation**: Principle VII: each milestone's user-guide task ships in that milestone
(`docs/user-guide/overview.md`), so CI's user-guide gate passes on every `feat` PR.

**Cross-platform**: Principle VI: no platform code. `worktree_dir` takes the home folder as a
parameter; the client reads it from `directories::UserDirs`. Card text draws in the bundled font in
logical pixels, so Linux-measured geometry tests hold on macOS and Windows CI.

**Decisions pending confirmation** (ledger D1, D2): D1 (first Escape only collapses) is T039 + T044 +
T045 alone; D2 (`~` path) is the tasks tagged `(D2)`: T009, T011, T012, T017 and T019 (the `home` parameter
of `ExpandedText::of` and its lookup) and T034 (the user guide's `~` wording), plus M2's deliverable
and quickstart §B.8. Changing either answer changes only those tasks. D3
(Escape with a card menu open only closes the menu) is agent-resolved in spec.md *Clarifications*.

**Organization**: by user story, in implementation order. US2 is split in two phases: its expanded
form and placement (reachable only in the component showcase) and its hover wiring in the app. US4's
"unchanged behaviour" checks sit beside the code they guard.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1–US4)

---

## Phase 1: Setup (Shared Infrastructure)

None: no new crate, dependency or fixture. `directories` is already a client dependency
(`shell/workspace.rs::start_dir`).

---

## Phase 2: Foundational (Blocking Prerequisites)

None beyond each story's own tasks: US1 needs only the promoted `LineClamped` (T001–T002); the core
types US2–US3 need are created in their phases.

---

## Phase 3: User Story 1 - Read more of a card without doing anything (Priority: P1) 🎯 MVP

**Goal**: on the collapsed card the title wraps to at most 3 lines and the activity to at most 2,
the last allowed line ending in an ellipsis (FR-001, FR-007). The title, branch and activity
tooltips stay until expansion replaces them (T031), so full text is never unreachable on `main`.

**Independent Test**: `scripts/build-lock.sh cargo test -p micold-client --lib -- overview_card`
and `--lib -- line_clamp`.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T001 [P] [US1] In `crates/micold-client/src/ui/material/line_clamp.rs` `mod tests`, add failing tests: `LineClamped::new(text, TypeRole::CardTitle, 3).tint(c)` draws in colour `c`; its line height is the role's spaced line height (the value `Ellipsized::at_role_spaced` uses for the same role); a 5-line text at 280 px lays out exactly 3 lines with an ellipsis ending the third; an unbroken 120-character run at 280 px lays out within the width and ends in an ellipsis (FR-007)
- [ ] T002 [P] [US1] In `crates/micold-client/src/ui/material/overview_card.rs` `mod tests`, add failing tests at card widths 280, 380 and 480 px: a 1-word title takes 1 line; titles needing 2 and 3 lines take 2 and 3 lines with no ellipsis (US1 sc.1); a title needing 5 lines takes 3, the third ending in an ellipsis (sc.2); an activity needing 2 lines takes 2 in full, one needing 4 takes 2 ending in an ellipsis (sc.3); a 1-line title or activity takes 1 line (sc.4); the branch stays 1 line, ellipsized (FR-001); no text node's bounds leave the card's bounds (FR-007, SC-002)
- [ ] T003 [P] [US1] In `crates/micold-client/src/ui/material/card_grid.rs` `mod tests`, add a failing test: a row where one card's title wraps to 3 lines gives every card in that row that card's height and no two cards' bounds overlap (US1 sc.5, feature 1120 FR-014)

### Implementation for User Story 1

- [ ] T004 [US1] In `crates/micold-client/src/ui/material/line_clamp.rs`: make `LineClamped` and `new` `pub(crate)`, add `pub(crate) fn tint(self, Rgb) -> Self`, and use the role's spaced line height as `Ellipsized::at_role_spaced` does (R2); keep its existing callers (`ui/material/mod.rs` Tooltip label, `review_panel_row.rs`) drawing as before; T001 passes
- [ ] T005 [US1] In `crates/micold-client/src/ui/material/overview_card.rs`: draw the title with `LineClamped::new(title, TypeRole::CardTitle, 3)` and the activity with `LineClamped::new(activity, TypeRole::Body, 2)`, tinted with the colours they use today, still wrapped in their existing tooltips; the branch stays `Ellipsized` (R2, FR-001); T002 and T003 pass
- [ ] T006 [US1] Regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt` with `UPDATE_LAYOUT_SNAPSHOT=1 scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot` if card heights changed, and check the diff shows only Overview card heights and their title/activity anchors moving
- [ ] T007 [P] [US1] In `crates/micold-client/src/showcase/sections/surfaces.rs`: the Overview card examples include titles needing 1, 2, 3 and 5 lines and a short and a long activity, at the widths the section already shows (quickstart §B.1)
- [ ] T008 [US1] In `docs/user-guide/overview.md` *One card per location* / *Title* and *What the agent is doing now*: the title wraps onto up to three lines and the activity onto up to two before an ellipsis (FR-017)

**Checkpoint**: collapsed cards show more text; nothing else changes.

---

## Phase 4: User Story 2 (part 1) - The expanded card's form and placement (Priority: P1)

**Goal**: the expanded text (K5–K6), the card's expanded form (R3, U1 `.expanded`) and the grid
drawing one card expanded over its neighbours without reflow, shifted and capped, scrolling inside
(R4, R6, U2, U3 draw and events). Reachable only from the component showcase until Phase 5 wires
it into the app.

**Independent Test**: `mise run test-core` (`overview_text`, `overview_expansion::place`) and
`scripts/build-lock.sh cargo test -p micold-client --lib -- overview_card card_grid`.

### Tests for User Story 2 part 1 (MANDATORY) ⚠️

- [ ] T009 [P] [US2] (D2) In `crates/micold-core/src/overview_text.rs` `mod tests`, add failing tests for K5 `ExpandedText::of(&CardModel, home)`: `title` is the full title; `display_name` is `Some` only when it differs from the title; `branch` is the full branch, `Some` only when there is one and the title is not the branch; `dir` is `worktree_dir(&model.dir, home)`; `activity` is the whole sanitised activity text, not clipped to a line, `None` with no activity; an orphan/invalid location has `branch: None` and its recorded `dir` (Edge Case)
- [ ] T010 [P] [US2] Create `crates/micold-core/src/overview_expansion.rs` (module doc: pure, no I/O, no logging, FR-016, FR-018) with `pub mod overview_expansion;` in `crates/micold-core/src/lib.rs`, and failing tests for K3 `place(slot: (top, height), natural, view: (top, height)) -> Placement { top, height, scrolls }`: `height = min(natural, view.height)`; `scrolls = natural > view.height`; `top = slot.top` when `slot.top + height <= view.top + view.height`, else `max(view.top, view.bottom - height)` and never greater than `slot.top`; whenever `natural >= slot.height`, `top <= slot.top` and `top + height >= slot.top + slot.height`
- [ ] T011 [US2] (D2) In `crates/micold-core/src/overview_text.rs` `mod tests`, add failing tests for K6 `worktree_dir(path, home: Option<&Path>) -> String`: `home/code/app/.worktrees/x` → `~` + the platform separator + `code/app/.worktrees/x`; `home` itself → `~`; a path outside `home`, or `home` `None` → the path in full; `/home/al` vs home `/home/alice` (string prefix only) → not shortened (FR-014)
- [ ] T012 [US2] (D2) Implement `pub fn worktree_dir` in `crates/micold-core/src/overview_text.rs` with `Path::strip_prefix` (R9); T011 passes
- [ ] T013 [P] [US2] In `crates/micold-client/src/ui/material/overview_card.rs` `mod tests`, add failing tests for `.expanded(true)` (U1, R3): it shows the full title, the display name when it differs from the title, the full branch when the title is not the branch, the worktree directory, the sessions line, the full activity, progress and pull requests, every line wrapped with no ellipsis, at 280, 380 and 480 px with an unbroken 120-character branch and path wrapping inside the card (FR-004, FR-007); header row and title lie outside the inner scrollable (FR-008); the node draws the `elevation::MENU` shadow (FR-003); focus ring, ripple and menu target mark bounds equal the expanded card's drawn bounds, and Enter/Space and a press activate it as on the collapsed card (FR-011, US4 sc.1–2); in every card state and both schemes each text's colour on the expanded face equals the same text's colour on the collapsed face, so feature 1120's contrast checks carry over (FR-015); scrolling the inner body to its top and to its end publishes `.on_body_scroll` with `(up, down)` = `(false, true)` and `(true, false)`, and `(false, false)` with no overflow (feeds `body_edges`, K4)
- [ ] T014 [P] [US2] In `crates/micold-client/src/ui/material/card_grid.rs` `mod tests`, add failing tests for U2–U3 with `.expanded(index, twin)`: row heights and every card's slot equal those of the same grid with nothing expanded, also when the expanded card's text is longer than its twin's (FR-009, US2 sc.2, US4 sc.5); the expanded node has its slot's x and width, height `min(natural, view height)`; it is drawn after every other card; a card in the last row is shifted up by `place` and stays inside the view, and moves with its slot when the grid's viewport scrolls (FR-008, US2 sc.5); a 1-column grid's expanded card has the column's width (Edge Case); a press, release and secondary click over the part lying on a neighbour publish only the expanded card's messages and `mouse_interaction` is the expanded card's (FR-010, US4 sc.1); the twin takes no `operate` focus traversal and publishes nothing; the expanded node is drawn at its full placed size on the first frame after `.expanded` is set, with no animation or intermediate frame (Edge Case *Reduced motion*); the Overview's scroll offset is the same before an expand, while expanded and after the collapse (FR-009); the expanded card given longer activity text stays expanded at the same slot with the new text (Edge Case *content changes*)
- [ ] T015 [US2] In `crates/micold-client/src/ui/material/card_grid.rs` `mod tests`, add a failing test: an expanded card whose content is taller than the view is as tall as the view, and a wheel over it scrolls its inner content first, then (at its end, or with no overflow) the Overview (US2 sc.6, FR-008)

### Implementation for User Story 2 part 1

- [ ] T016 [US2] First add a failing test in `crates/micold-client/tests/overview.rs` that the cards built by `crates/micold-client/src/ui/overview.rs` carry `dir` = the worktree's path for a worktree, the project root for *Default*, and the recorded path for an orphan (Edge Cases); then in `crates/micold-core/src/overview.rs`: add `pub dir: PathBuf` to `CardModel` (R8) and fill it in every `CardModel { … }` literal (`crates/micold-core/src/{overview,overview_text}.rs`, `crates/micold-client/src/{features/changes.rs,showcase/sections/surfaces.rs,ui/material/card_grid.rs,ui/material/overview_card.rs,ui/overview.rs}`, `crates/micold-client/tests/{features_changes,overview}.rs`); in `crates/micold-client/src/ui/overview.rs::card` set it from `Worktree::path` (orphan/invalid: the recorded path) and from the project root for *Default*
- [ ] T017 [US2] (D2) Implement `ExpandedText { title, display_name: Option<String>, branch: Option<String>, dir: String, activity: Option<String> }` and `ExpandedText::of` in `crates/micold-core/src/overview_text.rs` (K5, data-model); T009 passes
- [ ] T018 [US2] Implement `Placement` and `place` in `crates/micold-core/src/overview_expansion.rs` (K3); T010 passes
- [ ] T019 [US2] (D2) In `crates/micold-client/src/ui/material/overview_card.rs`: `.expanded(bool)` builds the same wrapper chain (ContextArea → Ripple → TakesTheKeyboard → button) with the expanded face of R3 from `ExpandedText::of(model, home)` (home from `directories::UserDirs::new()`), header and wrapped title outside an inner `Scrollable`; add `.scroll_id(id)` and `.on_body_scroll(f)`; shadow `style::elevation_shadow(roles, elevation::MENU)`; colours from `Roles` (FR-015); T013 passes
- [ ] T020 [US2] In `crates/micold-client/src/ui/material/card_grid.rs`: `.expanded(index, twin)`; `layout` measures the twin (own tree slot after the cards) for that card's `naturals` entry and lays the expanded card at its slot's x, width and top with height `min(natural, view height)`; `operate`, `overlay`, `update`, `mouse_interaction` and `draw` skip the twin (R4, U3)
- [ ] T021 [US2] In `crates/micold-client/src/ui/material/card_grid.rs`: `draw` skips the expanded card in the piece loop and draws it last in `renderer.with_layer(view)` translated by `place` from the last viewport (stored in `update`/`draw`) with `Layout::with_offset`; `update` and `mouse_interaction` go to the expanded card first, and every other card gets `cursor.levitate()` while the cursor is in the expanded card's drawn rectangle (R4, R6, FR-010); T014 and T015 pass
- [ ] T022 [US2] In `crates/micold-client/src/showcase/sections/surfaces.rs`: an `unscrolled` grid with `.expanded(0, twin)` beside the collapsed cards, with a long title, branch, directory and activity (FR-015, quickstart §B.2); `showcase_completeness` passes
- [ ] T023 [US2] Visual pass through an `autopilot-worker` running the `visual-pass` skill on the showcase, quickstart §B.1–§B.2, both themes, scale 1.0 and 2.0; evidence in `specs/1121-overview-card-full-text/visual-pass/`

**Checkpoint**: the showcase shows a card expanded over its neighbours; the app does not expand cards yet.

---

## Phase 5: User Story 2 (part 2) - Hover expands the card in the app (Priority: P1)

**Goal**: the expansion rules (K1–K2) and the pointer-rest timer (R5, U3), wired into the app
(U4): a resting pointer expands a card, leaving collapses it, a card menu freezes the form, the
window blur, closing the Overview and removed cards collapse it. The three full-text tooltips go.

**Independent Test**: `mise run test-core` (`overview_expansion`),
`scripts/build-lock.sh cargo test -p micold-client --lib -- card_grid overview`,
`--test overview`, `--test layout_snapshot`.

### Tests for User Story 2 part 2 (MANDATORY) ⚠️

- [ ] T024 [P] [US2] In `crates/micold-core/src/overview_expansion.rs` `mod tests`, add failing tests for every K1 bullet (data-model *Events and transitions*): rest A → A (Pointer), leave A → none; rest A then rest B → B only (FR-006); focus A → A (Focus), focus B → B; focus A, rest B, leave B → A; focus A, Escape, rest B, leave B → none; focus A, rest B, window blurred → none and nothing re-expands; Escape with A focus-expanded → none, `focused` still A, focus B and back to A → A; rest A, Escape → none, further events with the pointer still on A → none, leave A and rest A → A; focus A, rest A, Escape, rest B, leave B → none; menu open: rest B / focus B → no change, menu closed → they act again (FR-012); `Listed` without A while A expanded → none, then `FocusGained(first)` → first; `Expansion::default()` → none (FR-013); each collapse clears `body_edges`
- [ ] T025 [US2] In the same tests, K2: `EXPAND_REST <= Duration::from_millis(500)` (SC-006)
- [ ] T026 [P] [US2] In `crates/micold-client/src/ui/material/card_grid.rs` `mod tests`, add failing tests for the rest timer (U3, R5): the pointer resting on a card publishes its `on_rest` after `EXPAND_REST`, not before; leaving publishes its `on_leave`; rest A, then move straight from the expanded A's drawn area onto B → A's `on_leave`, then B's `on_rest` after `EXPAND_REST`; a scroll that brings another card under a resting pointer publishes the old card's `on_leave` and the new card's `on_rest` (Edge Case); nothing is published while a zoom runs
- [ ] T027 [P] [US2] In `crates/micold-client/tests/overview.rs`, add failing app tests: `Msg::Expansion(PointerRested(l))` expands `l` in the view; `CardMenuOpened` then a rest on another card leaves the form unchanged until `CardMenuDismissed` (FR-012, US4 sc.3); closing the Overview, `Toggled` and `CardActivated` reset the expansion, so reopening shows none expanded (FR-013); a listing without the expanded card leaves none expanded with no menu open; the collapsed card has no tooltip on title, branch or activity while the state-reason and location-type tooltips stay (FR-004). In `crates/micold-client/src/main_tests.rs` (beside the existing `update_inner_applies_window_focus_changed` test), a failing test `expansion_window_blur_collapses`: with a card expanded, `update_inner(&mut app, Message::WindowFocusChanged(false))` leaves none expanded, and `WindowFocusChanged(true)` expands none (FR-005, Edge Case *Window regains focus*)

### Implementation for User Story 2 part 2

- [ ] T028 [US2] Implement `Expansion`, `Event { PointerRested, PointerLeft, FocusGained, FocusLost, Escape, WindowBlurred, MenuOpened, MenuClosed, Listed(Vec<SessionLocation>), BodyScrolled { up, down } }`, `Cause { Pointer, Focus }`, `apply`, `expanded()`, `cause()` and `EXPAND_REST` (400 ms) in `crates/micold-core/src/overview_expansion.rs`, field by field as data-model *Expansion*; T024 and T025 pass
- [ ] T029 [US2] In `crates/micold-client/src/ui/material/overview_card.rs` add `.on_rest(m)` and `.on_leave(m)`; in `card_grid.rs` keep one `micold_core::tooltip::ShowTimer` for the topmost card under the cursor (expanded card first), observed with `EXPAND_REST`, `reset()` when the topmost card changes, a redraw requested at `wake_at`, no rest while `ZoomState::ends` is set (R5, U3); T026 passes
- [ ] T030 [US2] In `crates/micold-client/src/features/overview.rs`: `State.expansion: Expansion`, `Msg::Expansion(Event)` applied in the reducer; `CardMenuOpened` → `MenuOpened`, `CardMenuDismissed` / `CardMenuEntryChosen` → `MenuClosed` (U4); in `crates/micold-client/src/ui/overview.rs::settle_card_menu`, `!open` → `Expansion::default()`, else `Listed` fed before the no-menu early `return`; each card built with `.expanded(expansion.expanded() == Some(location))`, `.on_rest(Msg::Expansion(PointerRested(location)))`, `.on_leave(...PointerLeft(location))` and `.scroll_id(card_scroll_id(location))` (add `pub fn card_scroll_id(&SessionLocation) -> scrollable::Id` in `crates/micold-client/src/ui/overview.rs`) / `.on_body_scroll(...BodyScrolled)`, and the grid given `.expanded(index, twin)`
- [ ] T031 [US2] Drop `title_tooltip`, `Line::tooltip` (branch, activity) and `pub fn title_tooltip` from `crates/micold-core/src/overview_text.rs` and its tests, and the three tooltips from `crates/micold-client/src/ui/material/overview_card.rs` and its tests (FR-004); header tooltips stay
- [ ] T032 [US2] In `crates/micold-client/src/main.rs` `Message::WindowFocusChanged(false)` also sends `Msg::Expansion(Event::WindowBlurred)` (U4); T027's `main_tests.rs` test passes
- [ ] T033 [US2] Add a covered state `overview-expanded` (Overview open, first card expanded via `state.overview.expansion.apply(Event::FocusGained(first))`) in `crates/micold-client/tests/support/covered_states.rs` beside `overview-open`; regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt` (`UPDATE_LAYOUT_SNAPSHOT=1`); its text-overflow and containment gates pass in both schemes (R10, FR-017, SC-002)
- [ ] T034 [US2] (D2) In `docs/user-guide/overview.md`: new *Reading a card in full* section (rest the pointer on a card to expand it in place over the cards below; leaving collapses it; the directory line with `~`; a long card scrolls inside with the wheel; a card menu keeps the form); remove feature 1120's title, branch and activity tooltip sentences (FR-017)
- [ ] T035 [US2] Visual pass through an `autopilot-worker` (`visual-pass` skill) on the client, quickstart §B.3, §B.4, §B.6, §B.7, §B.8; evidence in `specs/1121-overview-card-full-text/visual-pass/`

**Checkpoint**: hovering a card in the Overview expands it; keyboard focus does not yet.

---

## Phase 6: User Story 3 - See a card's full text from the keyboard (Priority: P1)

**Goal**: keyboard focus expands the card (only shown focus, U1), Escape collapses it first (D1),
Page Up / Page Down scroll a long expanded card (K4), focus order unchanged (US4 sc.4).

**Independent Test**: `scripts/build-lock.sh cargo test -p micold-client --lib -- keyboard_focus overview_card`,
`--test overview`, `mise run test-core` (`page_target`).

### Tests for User Story 3 (MANDATORY) ⚠️

- [ ] T036 [P] [US3] In `crates/micold-client/src/ui/material/keyboard_focus.rs` `mod tests`, add failing tests for `TakesTheKeyboard::on_shown_change`: Tab focus publishes `true`; leaving it publishes `false`; a pointer press that takes the keyboard publishes nothing; a press on a keyboard-focused control publishes `false` (U1)
- [ ] T037 [P] [US3] In `crates/micold-core/src/overview_expansion.rs` `mod tests`, add failing tests for K4 `page_target(&Expansion, down) -> Option<PageTarget>`: `Some(Card)` only with a card expanded by Focus and `body_edges` allowing that direction; `None` with no card expanded, a Pointer cause, or the body at its end that way
- [ ] T038 [P] [US3] In `crates/micold-client/tests/overview.rs`, add failing app tests: a card's `on_keyboard_focus(true)` expands it and `(false)` collapses it (Story 3 sc.1, sc.3); press on a card then move away → nothing expanded (FR-002); a removed focused card's focus hand-back expands the card that receives focus (Edge Case); Tab, Shift+Tab and arrow moves visit cards in the same order with a card expanded as without (US4 sc.4); `Msg::CardPaged { down }` produces a scroll of the expanded card's `card_scroll_id` by + / − one visible height (test `expansion_card_paged_scrolls` in `crates/micold-client/src/main_tests.rs`, since `main.rs` answers it)
- [ ] T039 [US3] (D1) In `crates/micold-client/tests/overview.rs`, add failing tests for `on_escape`: with a card expanded, `Message::EscapePressed` collapses it, keeps it focused and the Overview (and full-screen) open; a second Escape closes the Overview (Story 3 sc.2, FR-005); with a card menu open, Escape only closes the menu and the card keeps its form (D3)

### Implementation for User Story 3

- [ ] T040 [US3] Add `TakesTheKeyboard::on_shown_change(f)` in `crates/micold-client/src/ui/material/keyboard_focus.rs`, published when `focused && visible` changes; T036 passes
- [ ] T041 [US3] Implement `PageTarget` and `page_target` in `crates/micold-core/src/overview_expansion.rs` (K4); T037 passes
- [ ] T042 [US3] In `crates/micold-client/src/ui/material/overview_card.rs`, first failing tests: with `.on_page(f)` set and the card focused, Page Down / Page Up publish `f(true)` / `f(false)` and are captured; without `.on_page`, or unfocused, they are not captured (FR-008); `.on_keyboard_focus(f)` publishes on Tab focus. Then add `.on_keyboard_focus(f)` passed to `on_shown_change` and `.on_page(f)` (U1)
- [ ] T043 [US3] Wire `.on_keyboard_focus(|f| Msg::Expansion(FocusGained/FocusLost(location)))` and `.on_page` (set only when `page_target` is `Some`) in `crates/micold-client/src/ui/overview.rs`; add `Msg::CardPaged { down }` in `crates/micold-client/src/features/overview.rs` and answer it in `crates/micold-client/src/main.rs::overview_message` with `scrollable::scroll_by` of ± one visible height on `card_scroll_id` (R6, U4); T038 passes
- [ ] T044 [US3] (D1) In `crates/micold-client/src/app.rs::on_escape`, after the registry and Settings arms: `Msg::Expansion(Event::Escape)` when `state.overview.expansion.expanded()` is some, else `Msg::Closed` as today (R7); T039 passes
- [ ] T045 [US3] (D1) In `docs/user-guide/overview.md` *Reading a card in full* and *Keyboard*: focus expands a card; Escape collapses it first and closes the Overview when no card is expanded; Page Up / Page Down scroll a long expanded card (FR-017)
- [ ] T046 [US3] Visual pass through an `autopilot-worker` (`visual-pass` skill), quickstart §B.5 and §B.6 by keyboard; evidence in `specs/1121-overview-card-full-text/visual-pass/`

**Checkpoint**: every story works; quickstart §B passes.

---

## Phase 7: Polish & Cross-Cutting Concerns

Code-free; done by the close unit in the close PR.

- [ ] T047 Run quickstart §A: `mise run test-core`, `scripts/gate.sh`, and the FR-018 grep over `crates/micold-core/src/overview_expansion.rs` and `crates/micold-core/src/overview_text.rs` prints nothing; record the result in `specs/1121-overview-card-full-text/quickstart.md`
- [ ] T048 Record the whole quickstart §B result in `specs/1121-overview-card-full-text/visual-pass.md` from the evidence of T023, T035 and T046

---

## Dependencies & Execution Order

### Phase Dependencies

- **US1 (Phase 3)**: none.
- **US2 part 1 (Phase 4)**: after US1 (T005's collapsed face is the twin and the base of `.expanded`).
- **US2 part 2 (Phase 5)**: after Phase 4 (renders `.expanded` and the grid's twin).
- **US3 (Phase 6)**: after Phase 5 (`Expansion`, `State.expansion`, `Msg::Expansion`, `card_scroll_id`).
- **Polish (Phase 7)**: after all.

### User Story Dependencies

US4 has no phase of its own: its scenarios are asserted in T013, T014 (sc.1, sc.2, sc.5), T027
(sc.3), T038 (sc.4); sc.6 (full-screen) by T026's zoom clause and the visual passes.

### Within Each User Story

Tests first and failing; core before client; card before grid; grid before app wiring; user guide
and visual pass last.

### Parallel Opportunities

- Phase 3: T001, T002, T003 together; T007 beside T004–T005.
- Phase 4: T009, T010, T013, T014 together (T011 after T009, T015 after T014: same files); then
  T012 and T018 together; T016 before T017;
  T019 after T016 and T017 (it calls `ExpandedText::of`); T020–T021 after T019.
- Phase 5: T024, T026, T027 together; T025 after T024 (same file).
- Phase 6: T036–T038 together; T039 after T038 (same file); T040 and T041 together.

## Parallel Example: User Story 2 part 1

```text
T009 ExpandedText tests (core)      T013 expanded card tests (client)
T010 place tests (core)             T014 grid twin/draw/event tests (client)
then T011 worktree_dir tests (core, after T009)   T015 inner scroll test (client, after T014)
```

## Implementation Strategy

### MVP First

US1 alone (M1): more of every card is readable at a glance, no interaction.

### Incremental Delivery

M2 builds the expanded form where it can be seen in the showcase; M3 makes hover expand cards in the
app; M4 adds the keyboard and Escape. Each merges green; no milestone leaves a half-wired path in
the app.

## Notes

- Between M3 and M4 Escape closes the Overview even with a card expanded (today's behaviour); M4's
  T044 changes that (D1).
- Commit after each test/implementation pair; never log a path, prompt or title (FR-018).

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Titles on three lines, activity on two 🎯 MVP

- **Tasks**: T001–T008
- **Deliverable**: on `main`, Overview cards wrap the title onto up to three lines and the activity onto up to two before an ellipsis, with nothing clipped at 280/380/480 px.
- **Satisfies**: US1 acceptance scenarios 1–5; FR-001, FR-007 (collapsed), FR-017 (wrapping); SC-004
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --lib -- overview_card line_clamp card_grid`; showcase *Surfaces* (quickstart §B.1)
- **Depends on**: —
- **Tier**: full

### M2 — The expanded card, in the showcase

- **Tasks**: T009–T023
- **Deliverable**: the component showcase shows a card expanded in place over its neighbours, elevated, with full title, branch, `~` directory and activity, shifted, capped and scrolling inside, while no other card moves; the app does not expand cards yet.
- **Satisfies**: US2 acceptance scenarios 1, 2, 5, 6 (wheel), 7 in the showcase; US4 sc.1, sc.2, sc.5 at widget level; FR-003, FR-004 (text), FR-007 (expanded), FR-008 (shift, cap, wheel), FR-009, FR-010, FR-011 (widget), FR-014, FR-015, FR-016 (`place`, `ExpandedText`); SC-002, SC-003
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-client --lib -- overview_card card_grid`; quickstart §B.1–§B.2
- **Depends on**: M1
- **Tier**: full
- **User guide**: none: the app does not change for a user (the showcase is a developer tool); the PR carries `docs-not-needed`. M3 and M4 document the behaviour when it becomes reachable.

### M3 — Hover expands a card in the Overview

- **Tasks**: T024–T035
- **Deliverable**: in the app, resting the pointer on a card for 0.4 s expands it over its neighbours; leaving, window blur, closing the Overview or the card's removal collapses it; an open card menu freezes it; the title, branch and activity tooltips are gone; the layout snapshot has `overview-expanded`.
- **Satisfies**: US2 acceptance scenarios 1–7; US4 sc.3, sc.6; FR-002 (pointer), FR-004 (tooltips), FR-005 (pointer, blur, close, another card), FR-006, FR-012, FR-013, FR-016, FR-017 (hover, snapshot), FR-018; SC-005, SC-006
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-client --lib -- card_grid overview`; `scripts/build-lock.sh cargo test -p micold-client --test overview --test layout_snapshot`; `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide -- expansion_` (`main_tests.rs` is compiled into the binary); the FR-018 grep of quickstart §A prints nothing; quickstart §B.3, §B.4, §B.6–§B.8
- **Depends on**: M2
- **Tier**: full

### M4 — Keyboard focus expands, Escape collapses first

- **Tasks**: T036–T046
- **Deliverable**: with no pointer, Tab and the arrows expand each focused card, Escape collapses it before it closes the Overview, and Page Down scrolls a long expanded card.
- **Satisfies**: US3 acceptance scenarios 1–6; US4 sc.4; FR-002 (focus), FR-005 (focus, Escape), FR-008 (Page keys), FR-011, FR-017 (keyboard); SC-001, SC-005
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-client --lib -- keyboard_focus overview_card`; `scripts/build-lock.sh cargo test -p micold-client --test overview`; `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide -- expansion_`; quickstart §B.5
- **Depends on**: M3
- **Tier**: full

Polish (T047–T048) changes no code: the close unit does it in the close PR.
