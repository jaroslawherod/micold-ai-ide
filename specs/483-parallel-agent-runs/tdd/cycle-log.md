# TDD cycle log — 483 parallel agent runs

No `tdd/test-list.md` was derived at design time; as in 482, the test-first task pairs of tasks.md
are the test list. Reds run against the stubs of commit `9f82d331` (they compile and answer
"nothing": `can_become` false, `RunGroup::new` Err, `validate` Ok, `derive_group` empty,
`RunsFile::to_json` empty / `from_json` Err, `JsonFileStore` runs methods no-ops).

## M1a — core, protocol and daemon (T001–T011, T014–T016)

| Behaviour | Tasks | Red (stub) | Green |
|---|---|---|---|
| Limits, `RunGroup::new`/`validate`, `can_become`, `GroupId` serde | T002 → T003 | `runs::tests`: 3 of 6 FAILED (`a_new_group_takes_two_to_eight_runs_numbered_from_one`, `a_loaded_group_refuses_no_runs_…`, `only_the_data_model_transitions_are_allowed`); the limits, the refusal of bad numbering (stub always Err), the accepted loaded shapes (stub always Ok) and `GroupId` serde pass on the stub by construction || `cargo test -p micold-core --lib runs`: 19 passed (T003) |
| `derive_group` | T004 → T005 | `runs::naming::tests`: 4 of 5 FAILED (stub `Ok(vec![])`); the case-only test passes on the stub by construction (empty equals empty) || `runs::naming::tests` 5 of 5 pass (T005) |
| `RunsFile` JSON, `runs_path`, load/save/remove | T006 → T007 | `runs::store::tests`: 7 of 7 FAILED || `runs::store::tests` 7 of 7 pass (T007; FakeProjectStore keeps a runs map) |
| Wire delta v36 | T008 → T009 | `protocol_roundtrip` did not compile before the wire edit (no `micold_core::runs`, no `RunGroupCreate`/`RunGroupPick`/`RunGroupDismiss`/`RunGroupsChanged`/`RunGroupCreated`/`RunPicked`; warm build log) || `protocol_roundtrip`, `schema_hash` pass (T009) |
| `CreateMode::NewBranchAt` (Decision: a run's branch starts at the base branch) | fix → test | written with the fix, not red first: `worktree_create` 3 new tests. Red shown afterwards by mutation (review B M1a): `NewBranchAt` creating at HEAD instead of `start` FAILED `run_group_create` `us1_s1_s2_…` ("run 1's branch starts at the base branch's tip") | `worktree_create` 17 passed |
| Daemon create, per-run tasks, routing | T010 → T014–T016 | `run_group_create` did not compile before `micold_daemon::runs`; with the routing in place `w1_a_project_that_is_not_a_repository_is_refused` FAILED (NotFound, cached repo flag) | `run_group_create` 8 of 8 pass |
| Runs file before push, whole writes, failing write, two windows, attach push, forget | T011 | passes on arrival: written after T010 and T014–T016 were green (a unit handover), so no red was observed; the failing write uses a regular file at `<store>/runs` instead of a read-only directory, because the container runs as root and root ignores directory modes | `run_group_persist` 5 of 5 pass |
| Red for T011, shown by mutation (review B M1a) | T011 | one build with: the create's `save_runs` skipped, `forget_project`'s `remove_runs` skipped, the attach's `RunGroupsChanged` not sent → `run_group_persist` 4 of 5 FAILED: `w5_the_runs_file_holds_…` ("the runs file holds the group by its first push: []"), `w5_a_failing_write_…` ("refused with IoFailed: Ok(RunGroupCreated …)"), `w5_forgetting_…` ("its runs file is gone"), `w2_an_attach_…` ("RunGroupsChanged follows Attached: Some(CatalogChanged …)"). `w2_two_windows_…` and the whole-file reader were not mutated: the push is the shared `broadcast_locked`, the write the existing `write_then_rename` | mutations reverted; 5 of 5 pass |
| Runs start at the recorded base commit, not the bare branch name (review A M1a) | fix | `run_group_create` `every_run_starts_at_the_recorded_base_commit_even_under_a_same_named_tag` FAILED with the old `start: base_branch`: "fatal: ambiguous object name: 'base'" | passes with `start: base_commit` |
| A session that never started fails at its session, not `PromptNotDelivered` (review A M1a r2) | fix | `run_group_create` `a_run_whose_session_never_starts_fails_at_its_session` FAILED before the fix: "run 1 failed at its session: PromptNotDelivered { reason: \"the session did not start, so the prompt was not typed\" }" | passes. Separately, a run whose group is no longer held now stops before its session (`set_run_status` returns whether it applied). No test covers that guard (review A r3) |

## M1b — client dialog and group rows (T012, T013, T017–T027)

| Behaviour | Task | Evidence | Result |
|---|---|---|---|
| Dialog reducer: open, edit, validation order, add/remove bounds, derived names, branch listing, confirm sends `RunGroupCreate`, dismiss | T012 → T017 | `features_runs.rs` written with the reducer in one step (not red first: the reducer and its test grew together in unit 1). Red by mutation: see below | `features_runs` passed |
| Header action beside New worktree, group tree, run rows, counts, failed run with no worktree, collapse | T013, T022 → T018, T023, T024 | `features_sidebar.rs` tests written with `arrange_groups`; not red first | `features_sidebar` passed |
| Dialog view, registry, focus, showcase poses | T019, T020, T025 | no behaviour test of its own: held by the guards `overlay_registry`, `popover_displacement`, `feature_registration_cost`, `root_is_routing_only`, `surface_registration_cost`, `showcase_*`, `layout_snapshot`. Each failed on the first full run after T019 (missing registration, counts, shell half, root arm) and passed once fixed | full `cargo test -p micold-client` green |

Red by mutation was not recorded for T012/T013/T022 in this milestone's units; the tests assert the
specified rules directly (`validate()` order, bounds, counts), and review B M1b flagged the gap.

## M2 — restart, delete, Dismiss group (T028–T039)

| Behaviour | Tasks | Evidence | Result |
|---|---|---|---|
| Interrupted runs on load (`interrupted_on_load`), daemon restart cleanup, delete follow, `Runs::dismiss` | T028, T029 → T031–T035 | written beside the code, not red first (unit 1): `run_group_persist` (12) and `run_group_create` (10) cover them | pass |
| Group menu = Dismiss group only; ask first naming what stays; confirm sends one `RunGroupDismiss`; cancel sends nothing; menu and confirmation go with a vanished group | T030 → T036 | `features_runs.rs` written first: the new tests did not compile (`GROUP_MENU_ITEMS`, `Msg::MenuToggled`, `DismissAsked`, `DISMISS_CONFIRMATION` missing) | `features_runs` 18 passed |
| A shrunk group shows `#1, #3`; a dropped group is not shown | T030 | passes on arrival: `arrange_groups` already projected `group.runs` as is (M1b) | `features_sidebar` 72 passed |
| Menu render, dialog render, registry, shell route | T037 | no behaviour test of its own; held by the registry and routing guards | `cargo test --workspace` (see gate) |

Red by mutation for the daemon half (review B M2), one build each, then reverted:

- `settle_interrupted` made a no-op → `run_group_persist` 3 FAILED: `an_interrupted_creating_run_is_failed_and_its_folder_and_branch_are_removed`, `an_interrupted_starting_run_fails_at_its_session_and_keeps_its_worktree`, `an_interrupted_run_whose_worktree_hosts_a_session_is_left_and_says_so`; `Runs::dismiss` no longer updating memory → `w4_dismiss_removes_only_the_group_and_an_unknown_group_is_not_found` FAILED.
- `Runs::forget_worktree` returning `None` → 2 FAILED: `w5_deleting_a_runs_worktree_removes_it_from_its_group_and_an_emptied_group_goes`, `deleting_the_winners_worktree_keeps_the_winner_set`.
- The `#1, #3` display and `a_dropped_group_is_not_shown` pass by construction (`arrange_groups` projects `group.runs` as stored); no mutation recorded.

## M3 — Compare (T040–T050)

| Behaviour | Tasks | Evidence | Result |
|---|---|---|---|
| `totals`, `has_uncommitted`, counts against a non-default base, equality with the Changes totals for the default base | T040, T041 → T043 | core unit tests and `runs_summary` were written first and failed to compile/pass before `summary.rs` existed | `cargo test -p micold-core --test runs_summary` green |
| Compare reducer: one read per run with a worktree, stale `seq` dropped, per-run refresh, status text, failed run reason and no Open diff, uncommitted tag, Open diff outcome, close ends reads, menu items | T042 → T044 | tests in `features_runs.rs` were written after the reducer in this milestone (not red first); they assert the specified rules directly | `cargo test -p micold-client --test features_runs` 29 passed |
| Shell reads, per-run watch, Compare view, menu item, showcase pose | T045–T048 | glue; no behaviour test of its own, held by the registry, catalogue and routing guards and the visual pass (`visual-pass/m3-compare-rows-*.png`) | `cargo test --workspace` |

Red by mutation for the reducer half (review B M3), one build each, then reverted:

- the stale-`seq` check always true → `an_answer_shows_and_a_stale_answer_is_dropped` FAILED
- `Msg::RunChanged` a no-op → `a_change_re_reads_only_that_run_and_keeps_the_old_counts_meanwhile` and `changes_while_a_read_runs_coalesce_into_one_more_read` FAILED
- `open_diff` without the worktree filter → `a_failed_run_shows_its_reason_and_has_no_diff_without_a_worktree` FAILED

## M4 — Pick this one (T051–T061)

| Behaviour | Tasks | Evidence | Result |
|---|---|---|---|
| Pick plan (fast-forward / merge commit / merge in the checkout), conflicted paths, merge message, git-too-old classification | T051 → T052 | unit tests and `runs_integrate` written with the module; the pure module and the client reducer were written before their tests in this milestone (not red first), so red is shown by mutation below | `cargo test -p micold-core --test runs_integrate` 9 passed |
| Daemon pick: refusals (unknown, busy, failed), uncommitted files named, conflicts named, fast-forward records the winner, busy checkout, two concurrent picks integrate one | T053, T057, T058 | `run_group_pick` (8 tests) | `cargo test -p micold-daemon --test run_group_pick` green |
| Client: pick offered/hidden/disabled, confirm when working, one pick sent, refusal keeps the view | T054, T059 | `features_runs` pick tests | `cargo test -p micold-client --test features_runs` green |
| A merge the user left in progress in the base checkout is refused and kept (review A M4 F1) | fix | test written first for the fix; mutation below | `runs_integrate` |

Red by mutation (one build each, then reverted):

- `plan` fast-forward branch inverted (`None if !is_ancestor`) → `the_plan_fast_forwards_when_the_base_tip_is_an_ancestor` and `the_plan_writes_a_merge_commit_when_the_base_moved_on` FAILED
- merge message reworded → `the_merge_message_names_the_run_the_group_and_the_base` FAILED
- in-progress-merge check removed from `merge_in_checkout` → `a_merge_the_user_left_in_progress_is_refused_and_kept` and `a_conflicting_merge_is_undone_by_the_call_that_started_it` FAILED
- Creating/Starting runs no longer disable Pick → `pick_is_disabled_on_every_row_with_the_reason_while_a_run_is_creating_or_starting` and `a_pick_that_is_not_offered_sends_nothing` FAILED
- uncommitted-files refusal skipped in `ops::pick_run` → `w3_uncommitted_changes_refuse_naming_the_files_until_committed` FAILED

## M5 — Clean up the losers (T062–T067)

| Behaviour | Tasks | Evidence | Result |
|---|---|---|---|
| Offer opens once with a heading, a row per loser (sessions, branch on) and a fresh read per loser; dismissing emits nothing and it does not reopen | T062 → T063 | `features_runs` cleanup tests; first run compile-red against the missing `CleanupOffer` | `cargo test -p micold-client --test features_runs` green |
| Pending, failed or uncommitted loser starts unselected and is never removable unconfirmed; failed read = uncommitted | T062 → T063 | `a_loser_whose_read_is_pending_failed_or_uncommitted…`, `a_failed_read_counts_as_uncommitted` | green |
| Confirm re-reads every selected loser first, sends nothing until they answer, then one `WorktreeDelete { stop_sessions: true, delete_branch }` per removable loser | T062 → T063 | `confirming_reads_the_selected_again…`, `keeping_the_branch_is_passed_through…` | green |
| Turned-uncommitted or selected-uncommitted loser: second confirmation naming it; declining removes the rest; dismissing removes nothing | T062 → T063 | two second-confirmation tests | green |
| Choice frozen during the re-read; a late first read keeps the user's untick (review A M5 F1, F2) | fix | test written first for the fix | green |
| Offer and second confirmation as `Modal`s, registered; showcase poses | T064, T065 | `overlay_registry`, `popover_displacement`, `showcase_completeness` | green |

The tests and the reducer were written in one session, as in M2–M4: the first run was compile-red. Red by mutation is below.

Red by mutation (one build each, then reverted):

- `removable()` dropping the clean-or-confirmed condition → `a_failed_read_counts_as_uncommitted`, `a_loser_whose_read_is_pending_failed_or_uncommitted…` and `a_loser_that_turned_uncommitted_needs_the_second_confirmation…` FAILED
- the toggle freeze (`is_editable`) removed → `the_choice_is_frozen_while_confirm_re_reads…` FAILED


## Verification follow-up (T072, T076, T078, T079)

Deliberate mutants, applied by hand one at a time against one test target each, restored byte for
byte from a copy taken before (the script compares the file after the run; `git status` shows no
source file changed). No mutation tool is configured.

### T078 — behaviours recorded as "no mutation recorded"

| Mutant | Test target | Failing test |
|---|---|---|
| `arrange_groups` renumbers runs from 1 (`#1, #3` becomes `#1, #2`) | `features_sidebar` | `a_group_whose_list_shrank_shows_the_remaining_numbers` |
| `Msg::GroupsChanged` appends instead of replacing (a dropped group stays) | `features_runs` | `groups_changed_replaces_the_list`, `a_group_that_goes_away_takes_its_menu_and_confirmation_with_it` and 3 more |
| Dialog `RunAdded` allows `<= MAX_RUNS` | `features_runs` | `runs_can_be_added_and_removed_between_the_minimum_and_the_maximum` |
| Group row `expanded` ignores the collapse state | `features_sidebar` | `a_collapsed_group_hides_its_runs_and_keeps_both_counts` |
| Group row `failed_count` always 0 | `features_sidebar` | `a_collapsed_group_hides_its_runs_and_keeps_both_counts`, `a_run_without_a_worktree_is_still_a_child_row_with_its_reason` |

All five killed; no assertion added. (`a_dropped_group_is_not_shown` itself guards only the
`arrange_groups` projection, which has no state to leak; the stored-list behaviour is held by
`groups_changed_replaces_the_list`.)

### T072 — the nine TEST_AFTER behaviours (verification rows 5, 7, 10, 11, 12, 14, 16, 17, 18)

| Row | Mutant | Test target | Failing test |
|---|---|---|---|
| 5 | `create_worktree` for `NewBranchAt` calls `worktree_add_new_branch` (start at HEAD) | `micold-core --test worktree_create` | none: survives (the `FakeGit` ignores `start`, see below) |
| 5 | same | `micold-daemon --test run_group_create` | `us1_s1_s2_three_runs_each_get_a_worktree_a_session_and_the_prompt_once`, `every_run_starts_at_the_recorded_base_commit_even_under_a_same_named_tag` |
| 7 | `forget_project` skips `remove_runs` | `run_group_persist` | `w5_forgetting_the_project_removes_its_runs_file` |
| 10 | dialog `RunAdded` bound `<=` (T078 row above) | `features_runs` | `runs_can_be_added_and_removed_between_the_minimum_and_the_maximum` |
| 11 | group row `expanded` ignored, `failed_count` 0, runs renumbered (T078 rows above) | `features_sidebar` | see T078 |
| 12 | `settle_interrupted` never fails a `Worktree`-step run | `run_group_persist` | `an_interrupted_creating_run_is_failed_and_its_folder_and_branch_are_removed`, `an_interrupted_run_whose_worktree_hosts_a_session_is_left_and_says_so` |
| 14 | renumbering from 1 / `GroupsChanged` appends (T078 rows above) | `features_sidebar`, `features_runs` | see T078 |
| 16 | Compare `summary_read` accepts any `seq` | `features_runs` | `an_answer_shows_and_a_stale_answer_is_dropped` |
| 17 | `integrate::plan` fast-forward test inverted (`None if !is_ancestor`) | `micold-core --lib runs::integrate` | `the_plan_fast_forwards_when_the_base_tip_is_an_ancestor`, `the_plan_writes_a_merge_commit_when_the_base_moved_on` |
| 18 | `ops::pick_run` uncommitted-files refusal off (`if false`) | `run_group_pick` | `w3_uncommitted_changes_refuse_naming_the_files_until_committed` |

Row 5 note: the core-level `NewBranchAt` tests assert only the progress line, because `FakeGit::
worktree_add_new_branch_at` (in `crates/micold-core/src/git.rs`, not a test file) drops `start`. The
behaviour is held by the two real-git daemon tests above, which is where it was first red. Teaching
the fake to record `start` is a source change and was left out of this test-only pass.

### Verification F2–F4 mutants (T071, T073, T074; 2026-10-09)

| Mutant | New test that failed | Restored |
|---|---|---|
| C5 `ops.rs:839` forced to `if false` (contained run still merged) | `run_group_pick::a_run_already_contained_in_the_base_integrates_nothing` (the only failure) | yes, `git diff` empty for `ops.rs` |
| C6 `ops.rs:859` base-moved guard set to `if false` | `run_group_pick::a_base_that_moves_before_the_checkout_merge_is_refused_and_not_merged` | yes |
| C7 `runs.rs:90-92` winner guard removed | `runs::tests::a_second_pick_on_a_group_with_a_winner_changes_nothing` | yes |

T070 (2026-10-09): `docs/user-guide/parallel-runs.md` re-read end to end against the shipped code and FR-001–FR-020, FR-022: entry point (sidebar header beside New worktree, `sidebar.rs`), `GROUP_MENU_ITEMS` (Compare, Dismiss group), refusals, cleanup offer and the second confirmation all match; no wording change was needed.

### T079 — `layout_snapshot.txt`

`git diff e6605e57 6b6b6281 -- crates/micold-client/tests/fixtures/layout_snapshot.txt` was reviewed:
280 lines removed, 344 added. Every node id on a removed line is still present afterwards; the
only new ids are the sixth child of the sidebar header row (`.../0/0/0/1/0/0/0/0/6` and its `/0`, in
the two themes' blocks), i.e. the new "Run in parallel" header action. Everything else is the x
offsets and widths of the sibling action rows moving left (`122.0` to `96.0`, `156.0` to `130.0`, and
so on). Position and size only, plus the added action node: legitimate.

### T076

Every bare assertion in `features_runs.rs` and `runs/integrate.rs` tests now carries a message naming
the rule (the first-line test name plus the asserted expression, or a hand-written sentence in
`integrate.rs`); `cargo test -p micold-client --test features_runs` (49) and `cargo test -p micold-core
--lib runs::integrate` (9) pass.
