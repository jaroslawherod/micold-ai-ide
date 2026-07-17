# Tasks: Sidebar Filter Toolbar Button

**Input**: Design documents from `/specs/009-sidebar-filter-toolbar/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md (all present)

**Tests**: Per Constitution Principle I (Test-First, NON-NEGOTIABLE), test tasks for all new
*pure/decision* logic are mandatory and precede their implementation. Purely visual/rendering
aspects (panel styling, icon tint, glyph shape) are validated via `quickstart.md`, matching how
feature 008 treated its analogous visual work — this is documented per task below, not silently
skipped.

**Documentation**: Per Constitution Principle VII, User Story 1 includes a user-guide update in
the same change.

**Cross-platform**: Per Constitution Principle VI, no platform-specific code is introduced;
verified in Polish.

**Organization**: Tasks are grouped by user story (spec.md priorities P1/P2/P3).

> **Amendment (research R7, applied during implementation)**: after T012–T018 were built
> against the original floating-`FilterOverlay` design, the user gave direct correction: use
> the `filter_list` icon (not `filter_alt`), move the trigger to the left of the header, and
> present the panel as an inline accordion (not a floating overlay). All three were applied.
> `FilterOverlay` was removed; only `FilterTrigger` remains, and a new `material::expand`
> primitive (vertical sibling to `slide`) now drives the accordion, composed directly in
> `sidebar.rs`. This removed outside-click dismissal (doesn't apply to inline content) — Escape
> and re-toggle remain. Tasks below are left as originally written (the historical record of
> what was built first); see `research.md` R7 and `contracts/filter-panel-ui.md` for the
> as-shipped design.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: US1/US2/US3 per spec.md

## Path Conventions

Single-project Rust + iced desktop app: `src/`, `tests/`, `assets/` at repository root (per
plan.md's Project Structure).

---

## Phase 1: Setup

**Purpose**: Prepare the one-time tooling needed for the font regeneration (research R6);
nothing here touches the repo's tracked source yet.

- [X] T001 Prepare a `uv`-managed Python environment with `fonttools` (`pyftsubset`,
      `fonttools.varLib.instancer`) available, per `contracts/icon-font-coverage.md`'s
      pipeline. Verify `curl` can reach the upstream
      `google/material-design-icons` raw files (variable font + `.codepoints` manifest).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The icon, font, and popover-state groundwork every user story's UI sits on top of.

**⚠️ CRITICAL**: No user story task can begin until this phase is complete.

- [X] T002 Regenerate `assets/fonts/MaterialSymbolsOutlined.ttf` as a full static instance
      (every upstream codepoint, weight 400 / FILL 0 / GRAD 0 / opsz 24) using the pipeline in
      `contracts/icon-font-coverage.md`; update `assets/fonts/PROVENANCE.md`'s "How this file
      was produced" section to describe full coverage instead of a curated subset.
- [X] T003 [P] Write a failing test in `tests/icons.rs` asserting `Icon::Filter` maps to
      `'\u{ef4f}'` and that `Icon::ALL.len()` is `19` (currently `18`) — must fail to compile /
      fail the assertion before T004.
- [X] T004 Add the `Icon::Filter` variant to `src/icons.rs` (enum entry, `Icon::ALL` entry,
      `glyph()` match arm mapping to `'\u{ef4f}'`) so T003 passes. Run
      `cargo test --no-default-features icons` and `cargo test --no-default-features
      icons_font` to confirm the regenerated font (T002) resolves the new glyph.
- [X] T005 [P] Write a failing test in `tests/sidebar_state.rs` asserting: `State::default()`
      (or the existing test fixture) has `sidebar_filter_open == false`; applying
      `Message::SidebarFilterMenuToggled` flips it to `true` and sets both `help_menu_open` and
      `project_switcher_open` to `false`; applying `Message::HelpMenuToggled` or
      `Message::ProjectSwitcherToggled` while `sidebar_filter_open` is `true` sets it back to
      `false` (mutual exclusion, symmetric with the existing `help_menu_open` /
      `project_switcher_open` pair at `src/app.rs:479-487`).
- [X] T006 Add `pub sidebar_filter_open: bool` to `State` (default `false`) and
      `Message::SidebarFilterMenuToggled` to `Message` in `src/app.rs`; implement the reducer
      arm (toggle + mutual exclusion both directions, extending the existing
      `HelpMenuToggled`/`ProjectSwitcherToggled` arms to also clear `sidebar_filter_open`) so
      T005 passes.
- [X] T007 [P] Write a failing test in `tests/sidebar_state.rs` asserting
      `on_escape(&state)` returns `Some(Message::SidebarFilterMenuToggled)` when
      `state.sidebar_filter_open` is `true` (regardless of `state.overlay`), and falls through
      to the existing `state.overlay` match otherwise.
- [X] T008 Extend `on_escape()` in `src/app.rs` with a leading
      `if state.sidebar_filter_open { return Some(Message::SidebarFilterMenuToggled); }` check
      before the existing `match state.overlay` so T007 passes.

**Checkpoint**: Icon, font, and popover-open state all exist and are tested. User story
implementation can now begin.

---

## Phase 3: User Story 1 - Filters tucked away by default (Priority: P1) 🎯 MVP

**Goal**: The sidebar shows no filter chips by default; a new filter-icon button in the
sidebar header toggles a floating panel containing the existing filter chips.

**Independent Test**: Open the app with tagged worktrees present, confirm no filter chips are
visible by default, click the sidebar header's filter button, confirm the chip panel appears
floating over the list; click again, confirm it disappears and the sidebar returns to its
default look.

### Tests for User Story 1

- [X] T009 [P] [US1] Write a failing test in `tests/sidebar_state.rs` asserting
      `State::filtered_worktree_tree()` produces identical output for the same
      `sidebar_filters` set regardless of `sidebar_filter_open`'s value (proves toggling panel
      visibility never mutates or otherwise affects filtering — FR-007/FR-008). This should
      already pass once T006 lands (no coupling exists), so it functions as a regression lock
      going forward — confirm it's genuinely exercising both `true` and `false` states.

### Implementation for User Story 1

- [X] T010 [US1] Add a `SidebarFilter` variant to `MotionKey` in `src/ui/mod.rs` (mirrors the
      existing `Menu` variant's doc-comment style).
- [X] T011 [US1] Add a 5th tuple to `motion_targets()` in `src/main.rs`:
      `(MotionKey::SidebarFilter, if app.core.sidebar_filter_open { 1.0 } else { 0.0 },
      MENU_FADE)`; bump the function's return-type array length from `4` to `5`.
- [X] T012 [US1] Create `src/ui/material/filter_panel.rs` implementing `FilterTrigger<M>` and
      `FilterOverlay<'a, M>` per `contracts/filter-panel-ui.md` (builder API terminating in
      `.into()`, Principle VIII): trigger renders `IconButton::new(Icon::Filter, roles)`
      wrapped in `Tooltip`; overlay follows the `MenuOverlay` `stack![base, backdrop, panel]` +
      `super::fade` idiom, anchored top-left near the sidebar header instead of top-right of
      the window.
- [X] T013 [US1] Register the `filter_panel` module and re-export `FilterTrigger`/
      `FilterOverlay` from `src/ui/material/mod.rs` (matching how `menu`/`project_switcher`
      are exposed today).
- [X] T014 [US1] In `src/ui/sidebar.rs`: add the `FilterTrigger` (wrapped in `Tooltip`, label
      "Filter worktrees") to the header row alongside `add_worktree`/`hide`, wired to
      `Message::SidebarFilterMenuToggled`; remove the unconditional `filter_bar(state, r)` call
      from the top of the non-empty-list branch (`sidebar.rs:120`) — its content is now built
      for the overlay instead (`filter_bar()`/`filter_chip()` themselves are unchanged and
      reused as the panel's content builder).
- [X] T015 [US1] In `src/ui/mod.rs::view()`, compose
      `FilterOverlay::new(base, filter_bar(state, r), Message::SidebarFilterMenuToggled,
      roles).progress(motion.get(MotionKey::SidebarFilter))` into the overlay-stacking chain
      alongside the existing `MenuOverlay`/`ProjectSwitcherOverlay` calls.
- [X] T016 [US1] Add a short section to the user guide (docs) noting that tag filtering now
      lives behind the sidebar's filter button instead of always being visible (Constitution
      Principle VII — user-facing change ships docs in the same change).

**Checkpoint**: User Story 1 is fully functional and independently testable — this is the MVP.
Run `cargo test` and the `quickstart.md` steps 1-3 manual pass before moving on.

---

## Phase 4: User Story 2 - Knowing filters are active without opening the panel (Priority: P2)

**Goal**: The filter trigger button visibly indicates whether any tag filter is currently
active, even while the panel is closed.

**Independent Test**: Activate a filter from within the panel, close the panel, confirm the
trigger button shows an active tint without reopening the panel; clear all filters, confirm
the tint reverts to inactive.

### Implementation for User Story 2

> No new pure/decision logic is introduced here — the active/inactive state is a direct,
> already-tested value (`!state.sidebar_filters.is_empty()`, covered since feature 008); only
> its rendering (icon tint) is new, which is a visual concern validated via `quickstart.md`
> (matching how feature 008 treated its own purely-visual chip-fill work), not a new headless
> unit.

- [X] T017 [US2] Add an `.active(mut self, active: bool) -> Self` builder method to
      `FilterTrigger` in `src/ui/material/filter_panel.rs`: tints the icon `primary` when
      `true`, `on_surface_variant` when `false` (research R4 — matches the existing
      `add_worktree`/`hide` tint convention, so no new `IconSurface` role is needed).
- [X] T018 [US2] At the `FilterTrigger` call site in `src/ui/sidebar.rs` (added in T014), pass
      `.active(!state.sidebar_filters.is_empty())`.
- [X] T019 [US2] Manual validation per `quickstart.md` step 4: confirm the tint change is
      legible in both light and dark themes (reuses the WCAG-AA contrast already asserted for
      `primary`/`on_surface_variant` against `surface` in `tests/icon_roles.rs` — no new
      assertion needed since no new role pair was introduced).

**Checkpoint**: User Stories 1 and 2 both work independently.

---

## Phase 5: User Story 3 - Dismissing the filter panel naturally (Priority: P3)

**Goal**: The filter panel can be dismissed by clicking outside it, pressing Escape, or
clicking the trigger again — all without disturbing the active filter selection.

**Independent Test**: Open the filter panel, then separately verify that an outside click, the
Escape key, and re-clicking the trigger each close it, and that any active filter remains
applied afterward in every case.

### Implementation for User Story 3

> Outside-click dismissal and re-toggle dismissal already work as of User Story 1 — they're
> built into `FilterOverlay`'s backdrop (T012) and `FilterTrigger`'s toggle message (T014)
> respectively, and FR-007/FR-008 (filters survive dismissal) are already regression-locked by
> T009. The only remaining gap is Escape, covered by Foundational T007/T008 at the pure
> `on_escape` level — this phase wires that pure result into the GUI's keyboard subscription.

- [X] T020 [US3] In `src/ui/mod.rs::subscription()`, add a branch: when
      `state.overlay == Overlay::None && state.sidebar_filter_open`, return an
      `iced::keyboard::on_key_press` subscription mapping `Escape` to
      `Message::SidebarFilterMenuToggled` (a new non-capturing closure, following the existing
      per-`Overlay`-variant pattern — `on_key_press` requires a non-capturing `fn`, which is
      why this can't just call the pure `on_escape` directly).
- [X] T021 [US3] Manual validation per `quickstart.md` step 5: confirm both dismissal paths
      (Escape, re-toggle) close the panel and that the worktree list stays filtered exactly as
      before dismissal in every case. (Outside-click dismissal was dropped per the R7
      amendment above — the accordion has no "outside" to click.)

**Checkpoint**: All three user stories are independently functional.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T022 [P] Handle the empty-available-filters edge case (FR-009): in the content passed to
      `FilterOverlay` (`src/ui/sidebar.rs` / `src/ui/mod.rs`), when
      `state.available_tag_filters()` is empty, show a short "No tags to filter yet" message
      (`style::muted`) instead of an empty panel; confirm the trigger button itself stays
      present and clickable regardless (not conditionally hidden).
- [X] T023 [P] Confirm the "live update while panel is open" requirement (FR-010) needs no new
      code or test: `available_tag_filters()`'s existing recompute-on-every-call behavior
      (already regression-locked by `tests/sidebar_tree.rs::filter_recomputes_after_delete` /
      `::filter_recomputes_after_rename`, feature 008) applies unchanged since the overlay
      renders the same call on every frame (iced is immediate-mode) — verify this by re-reading
      those two tests and confirming no additional coverage gap exists.
- [X] T024 Finalize `assets/fonts/PROVENANCE.md`: simplify the "Adding a new icon" section to
      drop the now-unnecessary font-regeneration step (per `contracts/icon-font-coverage.md`),
      leaving only "look up codepoint → add `Icon` variant → extend `tests/icons.rs`".
- [X] T025 Run the full `cargo test` suite (default + `--no-default-features`) and confirm no
      platform-specific code was introduced anywhere in this feature (Constitution Principle
      VI) — everything added is either pure Rust or goes through the existing cross-platform
      `iced`/font-loading path.
- [X] T026 Run the full `quickstart.md` manual validation end-to-end (all 8 GUI steps + the
      visual/asset check) via `cargo run`.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately.
- **Foundational (Phase 2)**: Depends on Setup (T001, for the font pipeline). Blocks all user
  stories.
- **User Story 1 (Phase 3)**: Depends on Foundational. No dependency on US2/US3.
- **User Story 2 (Phase 4)**: Depends on Foundational **and** on US1's `FilterTrigger`
  existing (T012/T014) — it adds a builder method and call-site argument to a type US1 creates.
  Not independently buildable before US1, but independently *testable/demoable* once both are
  in place (the spec's "independent test" is about behavior, not build order).
- **User Story 3 (Phase 5)**: Depends on Foundational (T007/T008) **and** on US1's
  `FilterOverlay`/`FilterTrigger` existing (backdrop + toggle message). Same relationship as
  US2 — behaviorally independent, but built on US1's primitive.
- **Polish (Phase 6)**: Depends on US1 (and touches US1's files); T022 also depends on the
  `FilterOverlay` content call site existing.

### Within Each Phase

- Tests precede their implementation task (Constitution Principle I) — see each phase's task
  ordering above.
- T003→T004, T005→T006, T007→T008, T009 (regression lock, no paired implementation) all follow
  Red-Green.

### Parallel Opportunities

- T003 and T005 (different test files/concerns) can be written in parallel.
- Within Foundational, T003/T004 (icon) and T005/T006 (state) touch disjoint code paths in the
  same file (`src/app.rs` vs `src/icons.rs`) and can proceed in parallel; T007/T008 depends on
  T006 landing first (both edit `on_escape`/reducer context in `src/app.rs`), so keep them
  sequential relative to T006.
- T022 and T023 (Polish) are independent and parallelizable.

---

## Parallel Example: Foundational Phase

```bash
# Can run together (disjoint files):
Task: "Write failing Icon::Filter test in tests/icons.rs"        # T003
Task: "Write failing sidebar_filter_open test in tests/sidebar_state.rs"  # T005
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1 (Setup) and Phase 2 (Foundational).
2. Complete Phase 3 (User Story 1) — this alone delivers the feature's core ask ("filtering
   hidden and slides out on a toolbar button press").
3. **STOP and VALIDATE**: run `cargo test` + `quickstart.md` steps 1-3.
4. This is a demoable MVP even without US2's active-indicator polish or US3's Escape support
   (outside-click and re-toggle dismissal already work from US1 alone).

### Incremental Delivery

1. Setup + Foundational → icon/font/state groundwork ready.
2. Add User Story 1 → test independently → MVP.
3. Add User Story 2 → test independently → active-state indicator ships.
4. Add User Story 3 → test independently → Escape dismissal ships.
5. Polish → edge cases, doc cleanup, full cross-platform test pass, final quickstart run.

---

## Notes

- [P] tasks touch different files with no unresolved dependency between them.
- US2 and US3 build ON TOP OF US1's new primitive rather than being buildable before it — this
  is called out explicitly in Dependencies above since it's a deviation from the "stories are
  fully order-independent" ideal (unavoidable: there is exactly one new UI primitive here, and
  US1 is the story that introduces it).
- No task manufactures a test for behavior that has no decision logic behind it (e.g. icon
  tint, glyph shape) — those are called out as quickstart-validated instead, consistent with
  how feature 008 treated its own visual-only work.
