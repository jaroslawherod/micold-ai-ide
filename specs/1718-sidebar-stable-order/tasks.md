# Tasks: Sidebar keeps a stable order; state shows on the row (#1718)

**Input**: `specs/1718-sidebar-stable-order/` (spec.md, plan.md, research.md, data-model.md,
contracts/sidebar-flat-list.md, quickstart.md)

Format: `[ID] [P?] [Story] Description`. `[P]` = different files, no dependency. Tests come first in
every phase and must fail (red) before the code that makes them pass (Constitution I). Use
`mise run check` while editing; run the single targeted test, not the workspace suite, until the gate.

## Phase 1: Setup

- [x] T001 Inventory every use of the removed symbols (`StateGroup`, `Group`, `group_by_state`,
  `header_text`, `group_open`, `SidebarGroup`, `GroupHeaderRow`, `ListRow`, `group_choices`,
  `GroupToggled`, `open_group_of_*`, `TypeRole::SidebarGroup`) with `LSP findReferences`, and `grep -rn` "state group", "group header",
  "Done group" across `docs/`, `README.md` and code comments (SC-004); note the files in the
  scratchpad. No repo change.

## Phase 2: User Story 1 + 3 — flat stable list, no group residue (P1 / P2) 🎯 MVP

The two stories are one change (removing the grouping removes the header, the toggle and the group
state together), so they ship as one milestone.

**Tests first (red)**

- [x] T002 [US1] `crates/micold-client/tests/sidebar_state.rs`: replace the feature 1125 group tests
  with flat-list tests: cycling one location (Default, a worktree, a run group) through all five
  card states leaves `sidebar_list()` identity and index sequence equal to `sidebar_entries()`
  (SC-001, SC-003, FR-001, FR-003, G1); a Done location is listed in stable order; no header row and
  no toggle message exists (FR-002, G2); tag filter removes without reordering.
- [x] T003 [US3] `crates/micold-client/tests/navigation_apply.rs`: `location_order`, keyboard Up/Down
  and reveal-current-session walk `sidebar_entries()`; reveal of a Done row selects it with nothing to
  expand and no header stop (FR-006, G3).
- [x] T004 [P] [US1] `crates/micold-client/tests/features_sidebar.rs`: `row_heights` has one entry per
  location row and no header row (G4).
- [x] T005 [P] [US3] `crates/micold-client/tests/composite_call_sites.rs` and
  `tests/sidebar_spawn_tree.rs`: `ListRow` becomes `SidebarEntry`; `row_index` takes
  `&[SidebarEntry]`.
- [x] T006 [P] [US3] FR-008 test in `crates/micold-client/tests/sidebar_state.rs`: a state file written before this change loads unchanged; no `group_choices` key exists.

**Implementation**

- [x] T007 [US3] `crates/micold-core/src/sidebar_groups.rs`, `lib.rs`: delete `StateGroup`, `Group`,
  `group_by_state`, `header_text`, `group_open` and their tests; keep `RunCard` and `run_group_state`
  with tests; the module keeps its name `sidebar_groups` (research R3), only its docs change.
- [x] T008 [US1] `crates/micold-client/src/features/sidebar.rs`: drop `SidebarGroup`, `GroupHeaderRow`,
  `ListRow::Header`, `ListRow`, `group_choices`, `Msg::GroupToggled`, `open_group_of_*`;
  `sidebar_list()` returns `Vec<SidebarEntry>`; `row_heights` and `row_index` take `&[SidebarEntry]`.
- [x] T009 [US3] `crates/micold-client/src/features/navigation.rs`: `location_nodes` over
  `sidebar_entries`; remove the `open_group_of_*` calls in `reveal_ready`.
- [x] T010 [US1] `crates/micold-client/src/ui/sidebar.rs`: drop `group_header`, the Header row key and
  the `SidebarGroup` / `ListRow` uses; render one row per entry.
- [x] T011 [P] [US3] Delete the group-only UI parts: the group-heading builder and its tests in
  `crates/micold-client/src/ui/material/tree_view.rs`; `TypeRole::SidebarGroup` and test
  `a_sidebar_group_header_is_twelve_px_bold` in `crates/micold-material/src/text.rs` and
  `crates/micold-material/src/type_role_mapping.rs`; `a_sidebar_group_header_is_legible_on_the_sidebar` in
  `crates/micold-client/src/ui/material/composition_contrast.rs`; the `SidebarGroup` entry in `crates/micold-client/src/showcase/catalogue.rs`; the
  group header poses in `crates/micold-client/src/showcase/sections/surfaces.rs` and `atoms.rs`; the matching rows in
  `tests/showcase_completeness.rs` (FR-007).
- [x] T012 [US3] Re-record the layout snapshot (`crates/micold-client/tests/layout_snapshot.rs` and its fixtures) for
  the flat list.
- [x] T013 [P] [US1] Docs: `docs/user-guide/worktrees-and-sessions.md` (rows stay in place; no state
  groups) and `docs/development/layout-snapshot.md` (flat list), and every other doc or comment T001 found that
  mentions state groups (FR-009, SC-004, Constitution VII).
- [x] T014 Make T002–T006 green; `mise run gate`.

**Checkpoint**: the sidebar is a flat stable list; nothing group-related is reachable.

## Phase 3: User Story 2 — every row shows its card state (P1)

**Tests first (red)**

- [ ] T015 [US2] `crates/micold-client/src/ui/material/state_icon.rs` unit tests: the five `CardState`s
  give five different `Icon` code points (e037, e645, e8f4, e86c, e034) and the same slot width
  (SC-002, G5); a no-session location is Idle, never blank.
- [ ] T016 [P] [US2] `crates/micold-client/src/ui/material/composition_contrast.rs`: re-key the
  state-icon contrast test on `CardState` and the five roles (4.5:1 / 3:1).
- [ ] T017 [P] [US2] `crates/micold-client/tests/features_sidebar.rs` and
  `tests/sidebar_state_icon.rs`: `with_state_line` takes a `CardState` and the tooltip state line is
  `state_line(CardState)` (G10: `CardState::label()`, Done reads "Done (merged)", FR-011); `row_second_line` still reads `type/number · session state` (G6, FR-004).

- [ ] T023 [P] [US4] `crates/micold-client/src/ui/material/state_icon.rs` unit tests for `row_tint`
  (G7, FR-010, FR-011): Needs action, Needs attention and Done give `error_container`,
  `tertiary_container` and `surface_container_high`; Working and Idle give `None`.
- [ ] T024 [P] [US4] `crates/micold-client/tests/sidebar_state.rs`: cycling one location through the
  five card states leaves the row order identical while its tint changes (SC-001, FR-012).
- [ ] T025 [US4] `crates/micold-client/src/ui/material/composition_contrast.rs`: text on each
  tint (`on_error_container`, `on_tertiary_container`, `on_surface`) is 4.5:1 and the state glyph
  3:1, light and dark (G9, SC-005).
- [ ] T026 [P] [US4] `crates/micold-client/src/ui/material/tree_view.rs` render-free test: a tinted
  row's hover and selected layers are drawn over the tint and the selected row stays distinct from
  a merely tinted one (G8, FR-012).

**Implementation**

- [ ] T018 [US2] `crates/micold-client/src/ui/material/state_icon.rs`: re-key `StateIcon` on
  `CardState`, take glyphs from `overview_card::state_icon`, roles per research R1; keep
  `Icon::StateBlocked` in the font; rewrite the `StateIcon` tests in `ui/material/tree_view.rs`
  (~1290-1303).
- [ ] T019 [US2] `crates/micold-client/src/ui/sidebar.rs`, `features/sidebar.rs`: pass the row's card
  state (`entry_card_state`, run group via `run_group_state`) to `StateIcon` and `with_state_line`.
- [ ] T020 [P] [US2] `crates/micold-client/src/showcase/sections/atoms.rs` and
  `tests/showcase_completeness.rs`: the Running/Blocked/Idle/Unknown poses become the five card states.
- [ ] T021 [P] [US2] `docs/user-guide/worktrees-and-sessions.md`: describe the five row state icons.
- [ ] T027 [US4] `row_tint` in `ui/material/state_icon.rs`; `tree_view.rs` and `ui/sidebar.rs` draw it
  as the row's resting fill, beneath hover and selected layers, with `on_*_container` text and glyph;
  add `state_line(CardState)` (G10) so Done's tooltip state line reads "Done (merged)" (R7, FR-010 to FR-012).
- [ ] T028 [US4] `crates/micold-client/src/showcase/sections/atoms.rs` and
  `tests/showcase_completeness.rs`: five-state poses show the tints, plus a tinted row hovered and
  selected pose, in both themes.
- [ ] T029 [US4] `docs/user-guide/worktrees-and-sessions.md`: describe the row tints (what Needs
  action, Needs attention and Done rows look like, that Done means merged and removable, that a
  closed unmerged PR is not marked) and that colour never replaces the glyph.
- [ ] T030 Make T015–T017 and T023–T026 green; `mise run gate`; quickstart §B step 3 via `visual-pass`.

**Checkpoint**: each of the five states has its own glyph on its row; Needs action, Needs attention and Done rows are tinted.

## Dependencies and order

- T001 first. Within a phase: tests (red), then implementation, then the gate task.
- Phase 3 depends on Phase 2 (`ui/sidebar.rs`, `features/sidebar.rs`, showcase and doc files are
  shared).
- Parallel: T004–T006, T011, T013; T016–T017 (not T025), T020–T021, T023–T024, T026.

## Implementation Strategy

MVP: Phase 2 alone makes rows stable (the issue's complaint) with the existing 905 icon. Phase 3
completes the state indicator for all five states and adds the row tints. No Polish phase changes code; quickstart §B
results and task ticks belong to the close unit.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Flat stable sidebar list 🎯 MVP

- **Tasks**: T001–T014
- **Deliverable**: the sidebar lists every location row in the stable order with no group header, toggle or group state; keyboard navigation and reveal walk the flat list.
- **Satisfies**: US1 acceptance scenarios 1–3; US3 acceptance scenarios 1–2; FR-001, FR-002, FR-003, FR-005, FR-006, FR-007, FR-008, FR-009; SC-001, SC-003, SC-004
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --test sidebar_state` and `--test navigation_apply`; quickstart §B steps 2 and 3
- **Depends on**: —
- **Tier**: full

### M2 — Five-state row indicator

- **Tasks**: T015–T030
- **Deliverable**: every location row shows a distinct glyph for each of the five card states, and its tooltip names the same state; Needs action, Needs attention and Done rows carry a colour-only tint (Done quieter) under the hover and selected layers.
- **Satisfies**: US2 acceptance scenarios 1–2; US4 acceptance scenarios 1–4; FR-004, FR-010, FR-011, FR-012; SC-002, SC-005
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --lib state_icon`; quickstart §B steps 1 and 3
- **Depends on**: M1
- **Tier**: full
