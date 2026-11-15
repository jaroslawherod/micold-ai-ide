# Test list: 1857 worktree parent

Derived after the fact from tasks.md and the TDD verification (`verification.md`, audited at c53be1dc9). Behaviors B1-B13, their tests and the scenarios they trace are in the *Test-first evidence* and *Traceability* tables there; this list points to them rather than copying.

## Test-first evidence

There is no `tdd/test-list.md`. Behaviors are derived from `tasks.md` (docs-only, tracking and visual-pass
tasks T010, T014, T018-T022 docs parts, T026-T029, T032 are `NOT_APPLICABLE`). Git history cannot show
ordering: each of #1873, #1912, #1921, #1917 is one squash commit, and the WIP commits the log names
(`5cfb009ca`, `b733a25c6`, `307e04e11`) are not in this clone.

| Id | Behavior (tasks) | Class | Evidence |
|---|---|---|---|
| B1 | Store round-trip of ids and links (T002/T003) | TEST_AFTER | log "Honest account": not test-first; no red; core unit mutants only |
| B2 | Id minting and reconcile (T004/T005) | TEST_AFTER | same; red substitute is mutants M-a..M-c in the log, after the fact |
| B3 | Tool schemas accept `parent` (T006/T008) | TEST_AFTER | same; test and code in one commit |
| B4 | create_worktree records parent, list filter (T007/T009) | TEST_AFTER | same; first-ever run was against finished code |
| B5 | Visibility by parent, all tools (T030/T031) | TEST_AFTER | log: "tests and the filter landed in one commit"; red recorded after the fact by swapping in origin/main `tools.rs` |
| B6 | Reason text names no hidden worktree (review B F4) | LIKELY | log records two tests red before the fix; squash cannot corroborate order |
| B7 | Rename and restart keep the link (T011/T013) | TEST_AFTER | as B1 |
| B8 | Removed parent, no re-adoption (T012/T013) | TEST_AFTER | as B1; one test failed first for a test bug, not a behavior red |
| B9 | Session rows carry the parent (T020/T021) | TEST_AFTER | as B1; one daemon-wiring mutant red after the fact |
| B10 | Sidebar parent tag, chip, filter (T015-T017) | TEST_AFTER | log: code and tests in `b733a25c6`; red by stubbing, after the fact |
| B11a | `runs-of-parent.sh`, original 25 cases (T023/T024) | TEST_AFTER | log: test and helper in `307e04e11`; red against origin/main after the fact |
| B11b | Review fixes A F1-F6 / B F2 (ledger, `--check`, input errors) | LIKELY | log: rebuilt 39-case test red (20 FAILED) on the old code before the fix; uncorroborated |
| B12 | Supervisor skill ownership and relay text (T025/T033) | TEST_AFTER | as B11a |

### Existing tests changed by the feature

Diffed every test file touched in the four PRs. No assertion was removed in favor of a weaker one, no test
was renamed, skipped, `#[ignore]`d or excluded, and no threshold changed. Reviewed edits:

- `mcp_worktree_parent.rs:223-226` (M1b, #1912): the positive case `create(sid(3), "c2", {"parent": "default"})`
  expecting `("default","known")` became `{"parent": "c1"}` expecting `("c1","known")`. Legitimate under
  FR-016 (a worktree caller may not name `default`), and the refusal is pinned at
  `mcp_worktree_parent_visibility.rs:208`. But it removed the only positive test of an explicit

## Traceability

Acceptance scenarios and requirements, mapped by reading the tests (there is no `traces` column).
"End to end" means the real MCP tool entry point (daemon `mcp` binary over a real repo) or the real
client `State::update`/sidebar entry point.

| Criterion | Tests | End to end |
|---|---|---|
| US1 AC1-AC2 (root and worktree callers record parent), FR-001 | `a_root_session_records_default...`, `a_session_in_a_worktree_records_it...`, `mcp_create_worktree.rs:412` | Yes |
| US1 AC3 / FR-002 (explicit override, refuse unknown) | `...explicit_parent_overrides` (named worktree), `an_explicit_parent_that_is_not_live_is_refused...`; explicit `default` positive case missing | Partial (finding 3) |
| US1 AC4 / FR-003 (app-made has none) | `a_root_session_records_default...` (`hand` made by git) | Partial (finding 5) |
| US1 AC5 / FR-008 (filter, direct children, `default` valid) | `the_parent_filter_returns_direct_children_only`, `create_worktree_and_list_worktrees_accept_an_optional_parent` | Yes |
| US2 AC1 / FR-005 rename of parent | `renaming_the_parent_keeps_the_link` | Yes |
| US2 AC2 / FR-005 restart | `a_restart_keeps_the_link` (daemon state reload, reads views directly, not via MCP) | Partial |
| US2 AC3 / FR-005, SC-002 relocation | none; store round-trip only (`store.rs` tests) | No (finding 5) |
| FR-005 rename of the child | none | No (finding 5) |
| US3 AC1 / FR-006 delete parent | `deleting_the_parent_leaves_children_removed...` via `ops`; MCP tool path not tombstone-tested | Partial (finding 4) |
| US3 AC2 / FR-007 same-name re-creation | same test | Yes |
| US3 AC3 `default` link unaffected | `default_links_are_never_removed_and_an_unrelated_delete...` | Yes |
| FR-014 stable id | `the_first_refresh_mints...`, core unit tests | Yes |
| US4 AC1-AC4 / FR-010 | `sidebar_parent.rs` (9), `the_snapshot_labels_a_child...` | Yes (renaming case weak, finding 8) |
| US5 AC1-AC2 / FR-009, FR-011 | `session_rows_carry_the_worktrees_parent_independent_of_origin` | Yes |
| FR-015 | `mcp_worktree_parent_visibility.rs` (8 tests) | Yes |
| FR-016 | `an_explicit_parent_outside_the_callers_set_is_invalid_input` | Yes |
| FR-017 | scope refusal tested; parent reachable only as "not `not_found`" | Partial (finding 2) |
| FR-012 other-project isolation | none | No (finding 5) |
| FR-004 parent fixed at creation | none (absence of a tool) | No (finding 5) |
| FR-013 / SC-005 supervisor and docs | `supervisor-ownership.test.sh` (39 cases); docs not tested | Yes (finding 9) |
| SC-001, SC-003, SC-004 | covered by the rows above | Yes |

Untested criteria: FR-004, FR-005 (child rename, relocation), FR-012; partly FR-002 (explicit `default`),
FR-003 (app op), FR-006 (tool path), FR-017 (success path). Tests tracing to nothing: none found.

