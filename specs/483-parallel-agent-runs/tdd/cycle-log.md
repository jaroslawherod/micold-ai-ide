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
