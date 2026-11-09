# TDD cycle log, feature 1718, M1

Honest record: the first M1 unit edited production code before the tests (a removal refactor: the
group API is deleted, not extended), then rewrote the tests against the new API. There is no
per-behaviour red run from that unit.

Red evidence, by construction: the new tests (`sidebar_state.rs` `flat_list`,
`navigation_apply.rs` `over_the_flat_list`, `features_sidebar.rs`, `composite_call_sites.rs`,
`sidebar_spawn_tree.rs`) call `sidebar_list() -> Vec<SidebarEntry>` and `row_index(&[SidebarEntry])`.
On `origin/main` those return `Vec<ListRow>` with `ListRow::Header`, so the test crate does not
compile there: the compile error is the red. The green run is the milestone gate on this branch.

| Behaviour | Test | Red | Green |
|---|---|---|---|
| Card state change never moves a row (SC-001, FR-001) | `sidebar_state::flat_list` | compile failure on origin/main | gate |
| No header row or toggle (FR-002) | `sidebar_state::flat_list` | compile failure on origin/main | gate |
| Keyboard and reveal walk the flat list (FR-006) | `navigation_apply::over_the_flat_list` | compile failure on origin/main | gate |
| One row height per location (G4) | `features_sidebar` | compile failure on origin/main | gate |
| Old state file loads (FR-008) | `sidebar_state::flat_list` | compile failure on origin/main | gate |

## M2 (T015-T030), tests first

Order kept this time: the tests (T015-T017, T023-T026, T020/T028 completeness) were written and
committed (`test(1718): five-state row indicator and row tint tests, red`) on top of the unchanged
production code of M1, then production (T018, T019, T021, T027-T029) followed.

Red run on that test-only tree, `scripts/build-lock.sh cargo check -p micold-client --lib --tests`:
`could not compile micold-client (lib test) due to 35 previous errors` and `(test "main") due to
5 previous errors`: `cannot find function row_tint / tint_ink / resting_fill`, `unresolved import
features::sidebar::state_line`, `ui::row_tint`, `no method row_tint on TreeItem`, `StateIcon::new`
given a `CardState` (E0308 x23). The failures are the missing M2 API, the right reason.

| Behaviour | Test | Red | Green |
|---|---|---|---|
| Five states, five glyphs, one slot width, Idle drawn (SC-002, G5) | `state_icon::tests` | compile failure (E0308, no `glyph().glyph()`) | `--lib state_icon` |
| Glyph colour is the tint ink (R1, R7) | `state_icon::tests::each_glyph_is_drawn_in_its_role` | compile failure | green |
| Tint by card state (G7, FR-010, FR-011) | `state_icon::tests::row_tint_by_card_state` | `row_tint` missing | green |
| Glyph 3:1 and text 4.5:1 on tint, rest and hover, both themes (G9, SC-005) | `composition_contrast` | `tint_ink`, `row_tint` missing | green |
| Tooltip state line, Done reads "Done (merged)" (G10, FR-011) | `sidebar_state_icon` | `state_line` missing | `--test main` |
| Order unchanged while the tint changes (SC-001, FR-012) | `sidebar_state::flat_list` | `ui::row_tint` missing | green |
| Selection distinct from tint, hover over tint (G8, FR-012) | `tree_view::tests` | `resting_fill`, `TreeItem::row_tint` missing | green |
| Showcase poses the five states and tints (T020, T028) | `showcase_completeness` | old poses (Running, Blocked, Unknown) | green |
