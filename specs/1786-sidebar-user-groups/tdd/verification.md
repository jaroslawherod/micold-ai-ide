---
feature: 1786-sidebar-user-groups
verdict: FAIL # at 77ee45453; F1 and F2 cleared by the close unit, see the last section
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md # rubric graded against (no override, no preset)
verified_at: 77ee45453 # HEAD of the working tree; M1-M6 all merged (b479914eb .. 77ee45453)
behaviors: 59 # A1-A28, U1-U31
proven: 49
likely: 7
test_after: 1
no_test: 0
not_applicable: 2 # A19, A26: green-on-arrival regression guards
high_smells: 1
criteria_total: 27 # acceptance scenarios US1-1 .. US7-2
criteria_covered: 26 # US6-2 (restart) is not exercised through the real load/save path, see F1
mutation_score: unmeasured # no mutation tool; cargo-mutants install was refused by the sandbox, not worked around
mutants_survived: 1 # deliberate mutants: 16 applied, 15 caught, 1 survived (F1)
suite: 8261 passed, 0 failed, 1 ignored, about 3 min of test time (partial suite, see "What was not audited")
independent: yes # the auditing session did not write these tests
---

# TDD Verification: Sidebar user groups (feature 1786)

**Verdict: FAIL.** The whole persistence path between the in-memory layout and the store, `shell::sidebar_layout::after_update`
(`crates/micold-client/src/shell/sidebar_layout.rs:13`), has no test: neutering it leaves all 805 tests of its crate green, yet
behaviors A24 and U7 and criterion US6-2 (SC-003, FR-012: groups survive a restart) are marked `DONE`.

Everything else holds up well. 15 of 16 deliberate mutants were caught, the core layout is thoroughly pinned, no existing
test was weakened, and every test named in the test list exists and passes. Stage that failed, in the rubric's order: question 4
(is every requirement covered through the real entry point), plus one tautological assertion under question 2.

Test-first (question 1) is `PROVEN` or `LIKELY` for 56 of 59 behaviors, but the evidence has a systematic weakness (F6).

## Method and independence

- The auditing session did not write the tests; everything below was read cold from the files at `77ee45453`.
- Local git history is six squash merges (`b479914eb`, `6006f3e02`, `cae50ef59`, `d1712abc8`, `719e9f3b9`, `77ee45453`), so local
  history alone cannot show ordering. The PR commit lists were read through the GitHub REST API (`pulls/{n}/commits`, and
  `commits/{sha}` for the file mix of each `feat`/`fix` commit). The base `553474089` named in the request is not in this
  shallow clone; the feature range audited is the six commits above.
- `PROVEN` below means: the cycle log records the red command and output for those tests, and the PR commit that introduced
  them also carries the source (no source-only commit landed first). The order *inside* one commit cannot be verified from
  history, so it rests on the log. `LIKELY` means the red is only indirect (shared with another cycle, or a compile error for a
  behavior whose logic already existed).
- `.specify/feature.json` points at `specs/1816-supervisor-verify-report`; the feature directory was taken from the request,
  not from the resolver, and `check-prerequisites.sh` was not run for that reason.

## Test-first evidence

Discrepancies between log and history: none contradicting a claimed red. The log is candid where it has no red (cycles 5, 9, 12, 13).
The log names one test that no longer exists (see F10).

| Behaviors | Class | Evidence |
|---|---|---|
| U1-U5, U13, U14 | PROVEN | cycle 1: `unimplemented!` stubs, `0 passed; 10 failed` (`panicked at sidebar_layout.rs:142`); commit `103a2827` |
| U7, U8 | PROVEN | cycle 2: `E0599 no method save_layout` (compile red); `103a2827` |
| U9-U12, A28 | PROVEN | cycle 3: assertion red `left: ["Default","a","b","c","d"]`, `1 passed; 6 failed`; `68e5f157` |
| A1, A4, A5 | PROVEN | cycle 4: 17 compile errors; `c8ed1f15` |
| A2, A3, U15, U16, U30 (menu half) | PROVEN | cycle 6: core `E0599 rename/delete`, client 20 errors; `22dd5c22` |
| A6-A9, U27 | PROVEN | cycle 8: 5 errors, plus one runtime red recorded; `23a411d0` |
| U6, U17-U19 | PROVEN | cycle 7: `E0599 move_item` (10 errors) |
| A10-A12, A14, A15, A16 (drag half), A27, U20, U21, U23 | PROVEN | cycle 10: 31 compile errors; `8c55c9c7` |
| A16, A17, A18, U24, U25, U26 | PROVEN | cycles 16 and 18: `E0599 move_group/step_item/...`; `ac4a529d`, `3406a71a` |
| A23, U28, U29, U30 (keyboard half) | PROVEN | cycles 17 and 18: `E0599 set_collapsed`, `no field on_menu_key` |
| A13 | LIKELY | state half red in cycle 10; the drawn highlight and marker (`ui::sidebar::tests`, cycle 12) was written beside the code, red "argued" by mutation |
| A20, A21, A22 | LIKELY | cycle 11 names no red of its own: "same 31 compile errors" as cycle 10, plus a note that Escape "needed" `on_escape` |
| A24, A25 | LIKELY | the end-to-end tests came in cycle 18, after the behavior existed (U3, U7, U8 from M1); compile red only |
| U22 | LIKELY | `default_never_arms` shares cycle 10's red; the view half (cycle 12) is test-after glue |
| U31 | TEST_AFTER | the empty-area menu tests (cycle 5, `sidebar_group_actions.rs:174-199`, `popover_displacement`, `overlay_registration`) "were written beside the code, not before it"; the chord and more-menu tests (cycle 4) are PROVEN. Worst part wins: see F3 |
| A19, A26 | NOT_APPLICABLE | declared regression guards, green on arrival (`a_grouped_worktree_cycling...`) |

`tasks.md` against the test list: every behavioral task is ticked and every behavior is `DONE`; no ticked task points at a
non-`DONE` behavior and no `DONE` behavior has an unticked task. T045 is ticked while its own note says the dark theme was
not captured (F14).

### Existing tests changed by the six commits

Reviewed every removed line under `crates/*/tests` in the six commits. **No weakened, loosened, skipped or renamed-away test.**

- `features_sidebar.rs` (about lines 2275-2355) and `sidebar_state.rs` (about lines 592-617): the more menu gained a fourth
  item, so `[true, true, true]` became `[true, true, true, true]` and the order and labels test gained `New group…`; the test
  was renamed to include `new_group` (kept within the same module, still selected by the same filters). Strengthened, not weakened.
- `sidebar_state.rs::a_location_cycling_through_every_card_state_never_moves`: its inline step list moved to
  `support/sidebar_fixture.rs::card_state_steps`. Same four steps, same `pr_read`/`session_in` calls, parameterised on the
  directory; equivalent.
- `overlay_registration.rs` (10 to 11), `overlay_registry.rs` (`EXEMPT` 11 to 12 to 13), `popover_displacement.rs`
  (`DIALOGS` 30 to 31), `root_vocabulary_is_cross_cutting.rs` (32 to 34), `root_state_is_shared.rs` (new `COMPONENT_LOCAL` pins
  for `sidebar_drag.over` and `.held`): guard allowlists grew, each with a stated reason and with the guard count moving in
  the same diff. Not a loosening, listed per the rule (F13).
- `sidebar_drag.rs`: `a_group_header_arms_a_group_drag_that_drops_nowhere_yet` was replaced by
  `..._that_lands_between_headers_only` (own feature, M4 placeholder superseded by M5; declared in cycle 18).
- `sidebar_group_actions.rs`: `assert_eq!(GROUP_MENU_ITEMS, ["Rename", "Delete"])` removed when the menu grew; replaced by
  `menu_labels` assertions at lines 353-383 (stronger).
- `layout_snapshot.txt`: regenerated with `UPDATE_LAYOUT_SNAPSHOT=1` in M1, M3 and M5 (F12).
- No config threshold, coverage or mutation scope, `#[ignore]`, or exclusion was added: the six commits touch no file outside
  `crates/`, `specs/1786`, `docs/user-guide` and `docs/development/layout-snapshot.md`.

## Findings

Ordered by severity. IDs are used by the remediation tasks in `tasks.md` (Phase 8).

| # | Sev | Finding | Evidence |
|---|---|---|---|
| F1 | HIGH | **Persistence glue is untested.** `after_update` loads the active project's layout on open and saves every `layouts_unsaved` project after each message, and reports a failed save. No test calls it or observes its effect: `grep` finds no test reference, and the deliberate mutant that makes it a no-op (`app.caps.projects().filter(|_| false)`) survives all 805 tests of the `micold-ai-ide` bin (the module is bin-only, so no integration test can reach it). The tests that claim the restart behavior (`order_and_collapsed_state_survive_a_save_and_a_reload`, `sidebar_group_actions.rs:985`) call `JsonFileStore::save_layout` by hand and then `sidebar_layout_loaded`, bypassing the wiring; the others only assert that `layouts_unsaved` contains the project. Deleting the call in `main.rs:516` would break SC-003, FR-012, US6-2 and the R9 failed-save notice (`Couldn't save the sidebar groups`) with every test green. The test list's glue exception covers `ui/sidebar.rs` and `main.rs` routing, not persistence. Behaviors A24 and U7 are `DONE` | `crates/micold-client/src/shell/sidebar_layout.rs:13-35`; `main.rs:516`; mutant G1 below |
| F2 | HIGH | **Tautological assertion** (catalogue: Tautological assertion). After `assert_eq!(names, ["Work", "Later"])`, which are the names the test itself created, the loop asserts that none contains `Idle`, `Working`, `Done`, ... It cannot fail while the first assertion holds. The load-bearing check for A26 is the literal `baseline` list at `:1036-1039`, which would fail if a state header appeared; the loop adds nothing. Should assert instead that the drawn list contains exactly the user-group headers and no other non-row entry after every card-state step (already true of `baseline`), or be deleted | `crates/micold-client/tests/sidebar_group_actions.rs:1050-1072` |
| F3 | MED | **U31 is test-after for the empty-area menu.** Cycle 5 states the menu tests were written beside the code and shows their "red" only by mutating guard lists. History cannot contradict this. Mitigation: the tests assert behavior (menu opens at the press point, its item opens the naming dialog and closes the menu, no project offers nothing) and the guard tests `overlay_registration`/`popover_displacement` pin it | `sidebar_group_actions.rs:174-199`; cycle log cycle 5 |
| F4 | MED | **Untracked behavior D7 (header count under a filter).** Three tests and a unit test pin `n`, `n of m` and `empty` words and the gone-versus-hidden rule, but no behavior id in the test list names them and cycle 9 records no red ("shown by mutation"). The list is incomplete for what the code now does | `sidebar_group_actions.rs:689-750`; `ui/material/group_header.rs:184-193`; cycle log cycle 9 |
| F5 | MED | **Review-fix tests with no red run.** Cycle 13 says its test changes "went in with their fixes, so their red was not run". PR #1968 shows source-only commits (`c8a3831b`, three files, no test file) and, for M3, `d360ecd8` (one file, no test). Tests concerned: `escape_cancels_the_drag_before_a_surface_open_under_it` (`sidebar_drag.rs:681`), `cdk::context_area::a_held_release_reaches_the_content_with_no_cursor` (`context_area.rs:410`), the inert checks in `ui::sidebar::tests::rows_and_headers_arm_*`. Cycle 15 then did record real rereds for the same area | cycle log cycle 13; PR #1968, #1946 commit lists |
| F6 | MED | **Most reds are compile errors.** Cycles 2, 4, 6, 7, 8, 10, 11, 16, 17, 18 and 20 record a missing symbol, not a failing assertion, so for those tests nobody saw the assertion fail for the right reason. This is one reason 7 behaviors are `LIKELY` rather than `PROVEN`. Compensated here by deliberate mutation (15 of 16 caught), but the discipline gap is real and repeated | cycle log, 11 of 20 cycles |
| F7 | MED | **A9 (US2-4, SC-006) is proven through the Overview card menu, not the sidebar row.** The spec scenario is "navigates to a row, opens the menu"; the autopilot ledger records that no sidebar worktree-row or run-group menu has a keyboard model (`autopilot.md`, "Follow-ups not done"). The test is honest about its path (`:630-670`) and passes, but the behavior is marked `DONE` without that qualification, and SC-006 ("every action ... without a pointer") is not met for Move on a sidebar row | `sidebar_group_actions.rs:630-670`; `autopilot.md` follow-ups |
| F8 | MED | **Eager test.** `rows_and_headers_arm_a_drag_default_does_not_and_the_target_list_is_highlighted` asserts arming, the tint of the target list, the marker edge (both sides) and the inert state of the dragged row in one body, so four behaviors share one failure message. Related LOW: `the_dragged_row_of_every_kind_holds_its_release_until_the_button_is_up` branches on `escape`/`label` inside a three-case loop (every case still asserts) | `crates/micold-client/src/ui/sidebar.rs:2360-2440`, `:2491-2548` |
| F9 | MED | **Self-comparison.** `assert_eq!(redrawn, state.sidebar_list(), "the order stays on every redraw")` compares two calls of one function on one state; it can only fail on nondeterminism. The neighbouring `entry_at(e) < entry_at(c)` carries the real check | `crates/micold-client/tests/sidebar_drag.rs:513-518` |
| F10 | LOW | Test list traces point at task ids instead of tests for A2, A3, A4, A5, U15 and U16 (`test` column reads `tasks T018, T019` and the like); the tests exist (`renaming_a_group_changes_the_header...`, `deleting_a_group_returns...`, `a_blank_name_is_refused...`, `a_duplicate_name_is_refused...`, core `rename_*`, `delete_*`) but a reader of the list cannot find them. Cycle 12 names `shell::subscriptions::tests::sidebar_drag_message_follows_the_pointer_and_ends_on_release_or_blur`, which no longer exists; it became `sidebar_drag_message_ends_on_release_or_blur_and_ignores_the_pointer` (`subscriptions.rs:523`) | `tdd/test-list.md`; `tdd/cycle-log.md` cycle 12 |
| F11 | LOW | Duplicated setup: four spellings of the same one-line constructor (`wt` in `sidebar_drag.rs:20`, `wt_item` in `sidebar_group_actions.rs:423`, `item` in `sidebar_tree.rs:1218`, `worktree` in core tests), two near-identical `grouped()` fixtures (`sidebar_drag.rs:30`, `sidebar_group_actions.rs:221`) and a third layout builder in `sidebar_tree.rs:1222`; the module doc of `sidebar_group_actions.rs:1-2` still says "creating, renaming and deleting" though the file now holds move, reorder, collapse and persistence | files cited |
| F12 | LOW | Snapshot regenerated by the authoring session in M1 (`+1048/-524` lines), M3 and M5. The M1 delta is consistent with one more more-menu item (+48 dp per scene, 524 added lines in groups of four rows) and the M5 delta with two taller row-menu entries, but no independent review of the fixture is recorded | `tests/fixtures/layout_snapshot.txt`; cycles 5, 19 |
| F13 | LOW | Guard allowlists grew (see list above). Reasoned and numbered in the same diff; recorded because the rule asks for every one | `overlay_registry.rs:~700`, `root_state_is_shared.rs` |
| F14 | LOW | T045 is ticked `[x]` and SC-007 asks for light and dark; only light was captured (`visual-pass/` holds four light screenshots; the task's own note says dark was not captured). Dark is covered by the header-fits gate's two-scheme loop only for geometry | `tasks.md` T045; `visual-pass/` |
| F15 | LOW | No suite baseline: `test-list.md` `suite_baseline: unknown`, cycle log "suite not run as a whole". The only full gate run recorded is cycle 19 (13151 tests, 4 failures fixed, one environmental). M1 and M6 PR bodies say the full gate was left to CI | cycle log baseline, cycle 19 |

Smell pass, everything checked and not listed above: no assertion-free tests, no doubled subjects, no over-mocking (the layout is
pure and the client tests drive the real `State::update`), no re-implemented expectations (expected lists are literals), no
`#[ignore]`, sleeps, real clocks or randomness in the new files, no secrets. Style matches the exemplars (inline
`#[cfg(test)] mod tests` for core logic, one concern per file under `tests/`, assertion messages state the rule, shared
`support::sidebar_fixture`, `JsonFileStore` over a `tempfile` directory in the store test as the runs-store tests do).
Properties: deterministic, isolated, fast (291 targeted client tests in 0.5 s, 30 core tests in 0.1 s), and a failing test names its
rule through its message and name.

## Mutation results

**No mutation tool was used.** `cargo-mutants` is not installed and `cargo install cargo-mutants` was refused by the sandbox's
permission classifier; it was not worked around. `mise` is not installed either, so `mutants-diff` could not run. Mutation
score is therefore **unmeasured**.

Fallback per the rubric: 16 deliberate mutants on the highest-risk behaviors, one at a time, each restored with
`git checkout -- <file>` and the tree verified clean (only the pre-existing, unrelated modification to
`specs/1786-sidebar-user-groups/autopilot.md` remains). Sample, not exhaustive; scope: `sidebar_layout.rs` (core), `sidebar_drag.rs`,
`sidebar_group_actions.rs`, `features/sidebar.rs`, `shell/sidebar_layout.rs`.

| # | Mutant | Behavior | Result |
|---|---|---|---|
| M1 | `move_item` in-list index no longer decremented | U17, U23 | Caught: `move_item_within_its_list_counts_the_index_before_the_removal` |
| M2 | `reconcile` stops dropping a doubly-named item | U4 | Caught (3 tests) |
| M3 | duplicate check case-sensitive | U14 | Caught: `create_refuses_a_duplicate_name_case_insensitively...` |
| M4 | `delete` drops the group's items | U16, A3 | Caught (2 tests) |
| M5 | `create` loses the `next_id` floor | U13 | Caught: `created_ids_grow_and_are_never_reused` |
| M6 | `stepped` bound `<` to `<=` | U24 | Caught (2 tests) |
| M7 | `rename` treats own name as duplicate | U15 | Caught |
| M8 | `move_group` index not adjusted for the removal | U25, A17 | Caught |
| C1 | drag activation threshold `>=` to `>` (4 px) | U20 | Caught (2 tests) |
| C2 | drop on a collapsed group no longer expands it | A15 | Caught |
| C3 | collapsed group still emits its rows | A23 | Caught (4 tests) |
| C4 | a drop on the row's own place commits and saves | U23 | Caught |
| C5 | any row can arm a drag (Default, vanished) | U22, FR-015 | Caught (2 tests) |
| C6 | `sidebar_layout_changed` stops marking the layout unsaved | A1, A6, A10, ... | Caught (8 tests); note this proves the flag is asserted, not that the flag is acted on (F1) |
| C7 | `MoveUp` steps down | A18 | Caught (4 tests) |
| G1 | `shell::sidebar_layout::after_update` made a no-op (load and save) | A24, U7, SC-003 | **SURVIVED** all 805 bin tests. Not equivalent: restart persistence would be silently lost. F1 |

Survivors inside a `DONE` behavior: 1 (G1, F1). Equivalent mutants: none judged.

Coverage: `mise run coverage-diff` unavailable (no `mise`, no `cargo-llvm-cov`); not used as corroboration.

## Traceability

Entry point declared by the test list: the client's render-free state (`State::update` with `Message::SidebarGroups` and
`Message::SidebarDrag`, `sidebar_list`, `sidebar_layout`), with `ui/sidebar.rs` pointer glue left to quickstart §B. "End to end"
below means through that entry point.

| Criterion | Tests | End to end |
|---|---|---|
| US1-1, FR-001 | A1 `a_valid_name_adds_a_header...`, U13 | Yes |
| US1-2 | A2 `renaming_a_group_changes_the_header...` | Yes |
| US1-3, SC-004, FR-003 | A3 `deleting_a_group_returns_its_worktrees...` | Yes |
| US1-4 | A4 `a_blank_name_is_refused...`, `a_blank_or_duplicate_rename...` | Yes |
| US1-5 | A5 `a_duplicate_name_is_refused...` | Yes |
| US2-1..3 | A6, A7, A8 | Yes |
| US2-4, SC-006 | A9 | Partly: through the Overview card menu only (F7) |
| US3-1..3, 5, 6 | A10, A11, A12, A14, A15 | Yes |
| US3-4, FR-008 | A13 | State half yes; drawn half by view-level unit test only |
| US4-1..3 | A16, A17, A18 | Yes |
| US4-4, US7-1, SC-005 | A19 | Yes (regression guard) |
| US5-1..3 | A20, A21, A22 | Yes |
| US6-1 | A23 | Yes |
| US6-2, FR-012, SC-003 | A24 `a_saved_layout_loads_back...`, `order_and_collapsed_state_survive...` | **No**: the load-on-open and save-after-change wiring is never exercised (F1) |
| US6-3 | A25 | Yes |
| US7-2 | A26 | Yes (via the literal baseline; F2 for the tautological tail) |

Functional requirements with no behavior of their own: FR-009 to FR-011, FR-014, FR-017 are covered through the rows above;
FR-014 ("creating, moving or deleting groups must not alter any worktree, branch or session") is asserted for delete and drop
(`state.worktree.worktrees` and `state.workspace.sessions` unchanged) but not for create or move. Success criteria SC-001 (15 s)
and SC-007 (visual pass, both themes) are human measures, not testable here.

Untested criteria: none outright; one not exercised end to end (US6-2). Tests tracing to nothing: the D7 header-count tests (F4) and the
review-fix tests (F5) have no list id. Every test named in `test-list.md` exists (checked by extracting each name and matching
`fn <name>` in the workspace).

## What was not audited

- **Whole-workspace suite.** Not run (no `mise`, disk 30 GB free, one `--workspace` run is documented in the profile as tens of
  GB). Run instead, all green at `77ee45453`: `micold-client --test main` 3512 passed (95 s), `--test layout_snapshot` 181 (26 s),
  `--lib` 1111, `--bin micold-ai-ide` 805, `micold-core --lib` 2193 (1 ignored), `micold-material --lib` 459. The daemon
  crate, the other ~590 integration binaries, `scripts/tests/*.test.sh`, fmt and clippy were not run.
- **Mutation testing by tool**: not available (see above). The 16 mutants are a sample of core operations, drag reducer and
  state flags; not covered: `group_hit_test`, `entry_lists`, `move_menu_items` ordering, the naming dialog glue, the keyboard
  menu highlight skipping, and the whole of `ui/sidebar.rs`.
- **Coverage tooling**: unavailable.
- **View and pointer glue** (`ui/sidebar.rs` drawing, `shell/subscriptions.rs` release/blur, `main.rs` routing, `ContextArea`) is
  covered by view-level unit tests where present and otherwise by quickstart §B, which was not re-run (needs Xvfb and a
  display). SC-007 dark theme and the restart check of quickstart §B step 5 have no recorded evidence in the repository.
- **Per-behavior ordering inside a commit** cannot be verified from git; `PROVEN` relies on the cycle log for it.
- **Platform differences** (macOS, Windows): the repository supports Linux only (#1897); nothing assessed.
- **Performance and load**: no criterion, not assessed (`apply_layout` clones the layout per draw, a declined review item, not measured).
- The sandbox acceptance suite (`sandbox_real_*`) is outside this feature.

## Close unit resolution (2026-11-15)

F1 and F2 (the blocking findings) are cleared: `shell::sidebar_layout::tests` (3 tests) fail against the no-op `after_update` mutant and pass restored; the tautological state-name loop is deleted (the `names == ["Work", "Later"]` assertion before it is the real check). F7 is recorded as PARTIAL in `test-list.md` (A9), F4 as U32, F9 fixed, F10 fixed. F3, F5, F8, F11, F14 stay open as follow-ups.
