# Autopilot ledger — #483 parallel-agent-runs

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #483 "Run one prompt across several agents in parallel worktrees and pick the best result" (scope verbatim from the issue); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #483
- **Worktree branch**: claude/project-thread-wysm57
- **Started**: 2026-10-08
- **Phase**: milestone M4
- **Next step**: M4 handover: gate, reviews, mutation evidence, visual pass, PR text (see Handover)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #643 | Design PR (spec, clarifications, plan, research, contracts, tasks) | merged | adfee3b1c820e9393c4c7824855cc1090efeeac0 |
| #644 | M1a (core, protocol v36, daemon) | merged (rebase) | 8d574eb591164a3e7ea3d01866461409023c46a8 |
| #647 | M1b (Run in parallel dialog, group row) | merged (rebase) | a90e7642e50390aad3665af07f95d32d34152e49 |
| #650 | M2 (restart survival, deletes, Dismiss group) | merged (rebase) | 0d356a56a83775c03741c14413fd93fd09cee0df |
| #651 | M3 (Compare the runs) | merged (rebase) | 2d07a5ac7d387657f0dfd6fddde527d10fea0d77 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1a | T001–T011, T014–T016 | full | Service starts N prompted runs as one persisted, pushed group; a failing run fails alone | #644 | merged |
| M1b | T012, T013, T017–T027 | full | Run in parallel dialog and the group row with failures and reasons | #647 | merged |
| M2 | T028–T039 | full | Groups survive restarts (interrupted runs cleaned), follow deletes, Dismiss group | #650 | merged |
| M3 | T040–T050 | full | Compare lists runs with status and counts, live refresh, Open diff | #651 | merged |
| M4 | T051–T061 | full | Pick this one: fast-forward or merge commit, refusals change nothing | (orchestrator opens) | implemented, not yet gated/reviewed |
| M5 | T062–T067 | full | Cleanup offer removes selected losers; uncommitted needs second confirmation | — | planned |

## Decisions

- Clarify US4 scenario 9: a picked run with uncommitted changes is refused, naming the files;
  nothing changes. _(decided by user)_
- Clarify FR-012: a pick always merges (fast-forward when possible, else a merge commit); the run's
  branch is never rewritten. _(decided by user)_
- Clarify round 2: no critical ambiguities left; clarify done.
- Tasks: M1 = Setup + Foundational + US1 + US2 Part A (group row, s1–s2), 27 tasks, over the ~10
  guideline: US2 Part A rides with US1 because the group row is the only place a run that failed
  creating its worktree (and its reason) is visible (analyze I2); US1 has no smaller split with an
  observable deliverable (the daemon create alone has no surface). US4 split along its scenarios
  into M4 (pick) and M5 (cleanup). Polish T068–T070 change no code: left to the close unit.
- Tasks: whole wire delta in T009 (protocol 35 → 36 once); RunGroupPick/RunGroupDismiss answered by
  a Refused placeholder until M4/M2, as feature 482 did.
- speckit-analyze fixes that changed spec/plan artifacts: FR-010, SC-004 and US3 s2 narrowed to a
  default-branch base (R7's recorded tension, analyze I1); US3 s3 status follows the session;
  Edge "Base branch missing" aligned with W1 (whole request refused); FR-019, US2 s5 and the Edge
  say a fully created worktree of an interrupted `Starting` run is kept (I5); data-model
  invariants allow gaps and one run after deletes, winner stays when its run is removed (I3);
  `RunStep::Prompt` dropped (never produced); CleanupOffer treats an unknown loser state as
  uncommitted and re-reads before deleting (I4, parallel-surfaces K3/K4); new G1a for a run with no
  worktree; W3 order/wording; plan test-table rows moved; quickstart module list.
- Tasks review round 1: CLEAN; MINORs fixed prose only (checklist note, SC-001 on M1, T068–T070 left to close unit). Checklist requirements.md: all 16 items ticked. Specs-only gate (check-criteria-observables) passed.
- speckit-analyze A1 (the two clarify defaults) left to the user's pending confirmation; C1 (shell
  glue untested) recorded as glue per plan Constitution row I in tasks.md's header.
| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

- M1 split (orchestrator, part 3): M1a = core + protocol + daemon (T001–T011, T014–T016), M1b =
  client (T012, T013, T017–T027); tasks.md § Milestones carries both blocks.
- Daemon: a run's worktree uses a new daemon-only `CreateMode::NewBranchAt { start }`
  (`#[serde(skip)]`, compatible with `BranchSituation::Free` only, rollback owns the branch) and
  `Git::worktree_add_new_branch_at`, because `NewBranch` starts at HEAD, not the base branch. An
  existing run branch → `SituationChanged` → `Failed { Worktree, "the branch <b> already exists" }`.
- Daemon: a run transition whose runs-file write fails still changes memory (the worktree or session
  already exists); logged, and the next write catches the file up. A new group's write failing
  refuses the create (W5).
- Daemon: `RunGroupsChanged` is sent to an attaching client right after `Attached` (before review
  pushes); `RunGroupCreate` answers `OperationOk` before the first push, then spawns the runs.

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | db71250366df0d22ca0ef7ddd4040b1d4304ac11:88c3a363b2acf0a05c3a9ad40b2cb2d052a27229 | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |
| Spec review | 2 | 8c96629cfa543d92ed3e94d0ecb58df2b625fe8a:4a403ba28452cbfef1e57944050a01b8beb435f0 | CLEAN |
| Plan review | 1 | 397f8015f2341c2338e28f4f9a517db315d1fee5:6b5fe648c01176305a208e5fdbacadc4af60fe29 | CLEAN: 3 MINOR (all fixed, prose only) |
| Tasks review | 1 | c61c9b57176ad6134bb9d4cd0fe376dc69bb33bc:d689b0660d22190b4db34f3feb4c401a7a4b8b4d | CLEAN: 3 MINOR (all fixed, prose only) |
| M1a review A (code-review high) | 1 | d2b15e8fe0ecb4b8f9e021a5fe1bda5ace611fae:634a59c6d386d5f1564d6223b4d83269702136a3 | CHANGES: 1 fixed (runs start at base_commit, regression test), 9 declined |
| M1a review B (conformance, sonnet) | 1 | d2b15e8fe0ecb4b8f9e021a5fe1bda5ace611fae:634a59c6d386d5f1564d6223b4d83269702136a3 | CHANGES: 1 MAJOR (red evidence, fixed by mutation runs in cycle-log), 1 MINOR fixed |
| M1a review A (code-review high) | 2 (full, snapshot stale) | e8ce7b374a0e487eb6655918f8450991775f4321:f9cc7ec22c32a6ff5143cde6b720be26e5fdfdfa | CHANGES: 2 fixed (a session that never started is `Failed{Session}`, not pickable; a run stops when its group is gone), 8 declined |
| M1a review B (conformance, sonnet) | 2 (full, snapshot stale) | e8ce7b374a0e487eb6655918f8450991775f4321:f9cc7ec22c32a6ff5143cde6b720be26e5fdfdfa | CLEAN |
| M1a review A (code-review, sonnet) | 3 (fix diff) | a919a4e20e57e980b74b417fcc4831d7e9ea4006:b4963b20c7ee533270a1b9a49a98aef4e981498c | both fixes hold; 5 MINOR: spec prose and cycle-log fixed (prose only), 3 to follow-ups |
| M1b review A (code-review high, fresh) | 1 | 80164d66f8f40785e3b73921cf40b5c06e7b46b4:e710b37db81f20266b2754d01e834d9eb39a6d6f | CLEAN: 3 MINOR (Remove disabled instead of hidden fixed; single-line prompt and silent BranchList failure to follow-ups) |
| M1b review B (conformance, sonnet) | 1 | 80164d66f8f40785e3b73921cf40b5c06e7b46b4:e710b37db81f20266b2754d01e834d9eb39a6d6f | CHANGES: 2 MAJOR (cycle-log M1b section added; T027 re-scoped to B23, B1-B6 live pass deferred), 1 MINOR (placement prose and test name fixed) |
| M2 review A (code-review high) | 1 | (see commit 6bf8796c) | CHANGES: 4 fixed (forget_worktree owner, displaces lists, attach log, popover/registry tables), 4 declined |
| M2 review B (conformance, sonnet) | 1 | (see commit eefde392) | CHANGES: process findings only (Verify not runnable in reviewer sandbox: ran by the unit, all green; red evidence in cycle-log mutation section; stale handover); no re-run |
| M3 review A (code-review high, sonnet reviewer) | 1 | ab8cf27d772af2bb17fb4bef7534e340ba477502:680acdffa53531475dabe73ad702b0efb639c54e | CLEAN: 1 MINOR (declined) |
| M3 review B (conformance, sonnet) | 1 | ab8cf27d772af2bb17fb4bef7534e340ba477502:680acdffa53531475dabe73ad702b0efb639c54e | CHANGES: 2 MAJOR (red by mutation recorded in cycle-log; T050 re-scoped to the showcase pose, B8-B11 live deferred as for T027), 2 MINOR fixed (Compare view built lazily; task text names the real files) |

## Declined review findings

- M2 A: `clean_half_created` needs provenance, which `ops::create_worktree` records only after git succeeds, so a kill mid-`git worktree add` leaves the folder: research R10 says "anything else is left alone and reported in the run's reason"; follow-up: record provenance before the add.
- M2 A: Dismiss while runs are `Creating`/`Starting` lets the in-flight tasks stop silently: W4 refuses only an unknown group; follow-up: let a task of a dismissed group finish its session or refuse the dismiss.
- M2 A: cleanup does git work under the `DaemonState` lock: the first read of a project's file is the only time; same pattern as other daemon ops; follow-up if it shows.
- M2 A: project read from `workspace.active` at view time: the sidebar shows groups of the active project only, and a project switch closes popovers; `GROUP_MENU_ITEMS[0]` coupling is a one-entry list until US3.
- M1a A: restart leaves `Creating`/`Starting` runs: M2 scope (T028–T039, interrupted runs cleaned).
- M1a A: `load_runs` treats an unreadable file as no groups, and a later save replaces it; `.corrupt` names are not unique: mirrors 482's review-file load (data-model "never fails"); follow-up recorded.
- M1a A: worktree path rebuilt in `run_one` from `.claude/worktrees/<dir>`: same layout `ops::create_worktree` uses; taste (MINOR).
- M1a A: `RunGroupsChanged` broadcast to every client: same as every other catalog-level push (`broadcast_locked`); W2 pushes on every change, not per attachment.
- M1a A: runs-file write under the state lock: as the catalog and 482's review file; W5 requires the write before the group is held.
- M1a A: `NewBranchAt` refuses `RemoteOnly`: plan Decision, a run's branch must be new; the reason names the branch.
- M1a A: no up-front name preflight: US1 s3 / FR-006 require the taken run to fail alone, the others to run.
- M1a A: `Debug` derives hold the prompt: no log line formats them; `w2_the_prompt_is_never_logged` guards W2.
- M1a A r2: `Typing` failures said as "not ready": copied on purpose from the MCP create (`mcp/tools.rs`), one wording for one cause.
- M1a A r2: no prompt size cap: the spec sets none; frames are bounded by the codec.
- M1a A r2: session id recorded only when the prompt step ends: W2 records it with the run's next status; taste.
- M1a A r2: a panicking run task leaves its run `Creating`/`Starting`: stale runs are M2's restart cleanup (T028–T039).
- M1a A r2: one invalid group sets the whole file aside; whole list cloned per transition: as round 1's declined store and lock findings.
- M1a A r2: `validate()` does not check the winner is `Picked`: winners are set by pick, M4 (T051–T061).
- M1a A r2: unused public types and empty `integrate.rs`/`summary.rs`: T001 creates them for M3/M4.
- M3 A F1 (MINOR): a `Failed` read is not retried on a group push, only on the watch's next change; retrying every push would re-read a persistently failing run on each push.

## Handover

M4 branch restarted from origin/main (2d07a5ac). Done and committed (2 commits, NOT pushed): core `runs::integrate` (pure) + `Git` trait methods + `summary::read_uncommitted_files` (T051/52/55/56, core tests green), daemon `ops::pick_run`, `Runs::group/pick`, server arm (T053/57/58, run_group_pick + run_group_create green; the placeholder test was removed), client `features/runs.rs` pick availability/confirm/refusal + `ui/confirm_pick_run.rs` (T054/59, features_runs 39 green, clippy client clean), user guide section (T060). T051–T060 ticked; T061 open.

Deviations to record in the PR: `base_branch_is_checked_out` (T057) is not in state.rs: the checkout is read live with `git worktree list` per contract I1 (cache can be stale). `integrate::plan` takes (run_tip, is_ancestor, checkout) without base_tip. A run already contained in the base is a no-op `FastForward{base_tip}`. BaseMoved is covered by the core CAS test only (no daemon race test).

Known gaps for next unit: (1) tests were written AFTER code for the pure integrate module and the client (no red phase): add red-by-mutation evidence to specs/483-parallel-agent-runs/tdd/cycle-log.md as M2/M3 did (review B flagged this in M3). (2) Refusal/success snackbars live in `shell/daemon_sync.rs`, untested. (3) Full gate was started: `cargo` raw commands, log `scratchpad/gate-m4.log` (use `--no-fail-fast`; expect only the 6 root permission failures); rerun if the tree changed. (4) Not yet run: review A (code-review high on origin/main...HEAD), review B (conformance rubric, sonnet, `claude -p --agent autopilot-reviewer`; run review-snapshot.sh first and record it), visual pass B12–B16, B20 (T061, record under quickstart.md § Results § B), write PR text to scratchpad/pr-483-m4.md, gate stamp + push.

## Open escalation

None.

## Follow-ups not done

- `run_one` (review A M1a r3): the forgotten-group guard covers only `Starting`. A project forgotten during the session step still gets a session. A worktree created just before is left unowned. The guard's comment names only one of `set_run_status`'s two `false` causes, and no test covers the guard.
- Runs file: an unreadable (not missing) file loads as no groups and the next save replaces it; `.corrupt` backups overwrite each other (review A M1a, declined for M1a).
- M1b: the Run in parallel prompt is a single-line `TextField`; contract D1 wants a multi-line `TextArea` (needs a binary-owned `text_editor::Content`).
- M1b: a `BranchList` failure leaves the base-branch select empty with only the generic "Choose a base branch" error.
- M1b: quickstart B1-B6 on the live client not driven (no fixture daemon in the cloud container); only B23 poses.
