# TDD test list — 483 parallel agent runs

Derived after the fact (T072) from the acceptance scenarios of `spec.md`; the test names are those of
the Traceability table in `verification.md`. `traces` is the scenario. Level: D = daemon over the wire,
R = client reducer, C = core with real git. Rows 5 and 9 of "Class" in the verification are the
behaviours that were written after the code; their mutant red is in `cycle-log.md` ("Verification follow-up", T072).

| # | traces | Behaviour | Level | Tests |
|---|---|---|---|---|
| 1 | US1 s1 | N runs get N worktrees (own branch, starting at the base commit) and N sessions | D | `run_group_create::us1_s1_s2_...`, `every_run_starts_at_the_recorded_base_commit_...` |
| 2 | US1 s2 | The prompt is typed once into each run's session | D | `run_group_create::us1_s1_s2_...` |
| 3 | US1 s3 | One run that fails to create does not stop the others | D | `us1_s3_...`, `fr005_...` |
| 4 | US1 s4 | A session that never started fails at its session; the prompt is not typed | D | `us1_s4_...`, `a_run_whose_session_never_starts_...` |
| 5 | US1 s5 | The dialog refuses an empty prompt, no type, empty or bad name, run count, unoffered CLI, no base, in that order | R, D | `features_runs::validate_reports_the_first_failing_rule_...`, `w1_each_refusal_...` |
| 6 | US2 s1 | A group is one row with its runs as children and counts | R | `features_sidebar` group tests |
| 7 | US2 s2 | A group collapses and expands | R | `features_runs::a_group_can_be_collapsed_...`, `features_sidebar::a_collapsed_group_hides_its_runs_...` |
| 8 | US2 s3 | Groups survive a daemon restart | D | `run_group_persist::after_a_restart_...` |
| 9 | US2 s4 | Deleting a run's worktree removes the run; an emptied group goes | D | `w5_deleting_a_runs_worktree_...`, `deleting_the_winners_worktree_...` |
| 10 | US2 s5 | A create interrupted by a restart is settled (failed and cleaned up, or kept) | D | `an_interrupted_*` (3) |
| 11 | US2 s6 | Dismiss group asks first, then removes only the group | D, R | `w4_dismiss_removes_only_the_group_...`, `features_runs` dismiss tests |
| 12 | US3 s1, s2 | Compare shows files, added and removed per run against the base | C, R | `runs_summary` (3), `summary::tests`, `features_runs` Compare tests |
| 13 | US3 s3 | A change in a run's worktree re-reads only that run | R | `a_change_re_reads_only_that_run_...`, `changes_while_a_read_runs_coalesce_...` |
| 14 | US3 s4 | Open diff asks for that run's changes view | R | `open_diff_asks_for_that_runs_changes_view` |
| 15 | US3 s5 | A failed run shows its reason and no diff | R | `a_failed_run_shows_its_reason_...` |
| 16 | US4 s1, s2 | Pick fast-forwards or merges the run into the base and records the winner | D, C | `run_group_pick` (8), `runs_integrate` (9), `features_runs` pick tests |
| 17 | US4 s3-s5 | The cleanup offer lists the losers, never removes uncommitted ones unconfirmed, removes the selected | R | `features_runs` cleanup tests |
| 18 | US4 s6-s9 | Pick is refused while a run is busy or failed, with uncommitted files, or in conflict; nothing changes | D, C | `w3_conflicts_refuse_...`, `a_conflicting_pick_names_the_files_and_changes_nothing` |
| - | SC-001 | Four runs under a minute | - | none (timing outcome; not automated) |
| - | SC-006 | A conflict leaves everything as it was | D | `w3_conflicts_refuse_...` |
