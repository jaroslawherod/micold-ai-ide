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

## M2 plan (not yet run)

Tests first, each red before T027: `row_tint` mapping (T023), order unchanged while the tint changes (T024), tint contrast in both themes (T025), state layers over the tint (T026), `state_line` Done "(merged)" (T017). Evidence is recorded here by the M2 unit.
