# Autopilot ledger — #483 parallel-agent-runs

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #483 "Run one prompt across several agents in parallel worktrees and pick the best result" (scope verbatim from the issue); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #483
- **Worktree branch**: claude/project-thread-wysm57
- **Started**: 2026-10-08
- **Phase**: milestone M1
- **Next step**: implement M1 (T001–T027): continue from Handover (core green T003/T005/T007)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #643 | Design PR (spec, clarifications, plan, research, contracts, tasks) | merged | adfee3b1c820e9393c4c7824855cc1090efeeac0 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T027 | full | Run in parallel starts N prompted runs, shown under a group row with failures and reasons | — | in progress |
| M2 | T028–T039 | full | Groups survive restarts (interrupted runs cleaned), follow deletes, Dismiss group | — | planned |
| M3 | T040–T050 | full | Compare lists runs with status and counts, live refresh, Open diff | — | planned |
| M4 | T051–T061 | full | Pick this one: fast-forward or merge commit, refusals change nothing | — | planned |
| M5 | T062–T067 | full | Cleanup offer removes selected losers; uncommitted needs second confirmation | — | planned |

## Decisions

- Clarify US4 scenario 9: a picked run with uncommitted changes is refused, naming the files;
  nothing changes. _(default, pending user confirmation)_
- Clarify FR-012: a pick always merges (fast-forward when possible, else a merge commit); the run's
  branch is never rewritten. _(default, pending user confirmation)_
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

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | db71250366df0d22ca0ef7ddd4040b1d4304ac11:88c3a363b2acf0a05c3a9ad40b2cb2d052a27229 | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |
| Spec review | 2 | 8c96629cfa543d92ed3e94d0ecb58df2b625fe8a:4a403ba28452cbfef1e57944050a01b8beb435f0 | CLEAN |
| Plan review | 1 | 397f8015f2341c2338e28f4f9a517db315d1fee5:6b5fe648c01176305a208e5fdbacadc4af60fe29 | CLEAN: 3 MINOR (all fixed, prose only) |
| Tasks review | 1 | c61c9b57176ad6134bb9d4cd0fe376dc69bb33bc:d689b0660d22190b4db34f3feb4c401a7a4b8b4d | CLEAN: 3 MINOR (all fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 unit 2 handed over at ~150k context (milestone unit; no PR open). Stay on branch
`claude/project-thread-wysm57` as it is (skip `branch-start.sh`; `gh` unauthenticated). Cloud: no
`mise`; run cargo through `scripts/build-lock.sh` with `CARGO_INCREMENTAL=0`, detached (helper
`$SCRATCHPAD/bg.sh <name> <cmd…>` writes `<name>.log` ending `JOB_EXIT=`; hold with `hold.sh`).
Done since unit 1 (committed as "wip(483): M1 reds recorded, daemon run-group tests"):
- Core reds run and recorded in `specs/483-parallel-agent-runs/tdd/cycle-log.md` (T002, T004,
  T006, T008 rows; Green column empty).
- T010 test written: `crates/micold-daemon/tests/run_group_create.rs` + shared harness
  `crates/micold-daemon/tests/support/runs.rs` (JsonFileStore catalog, stand-in `claude`, no
  `copilot`, repo on `main` with branch `base` one commit ahead; creates use base `base`). Not yet
  compiled. T011 (`run_group_persist.rs`) NOT written: plan it on the same harness; for W5's
  failing write put a regular file at `<store>/runs` (root ignores read-only dirs; six permission
  tests already fail as root).
Next step: implement T003 (`can_become`, `RunGroup::new`, `validate` in core runs/mod.rs), T005
(`derive_group`: `naming::derive` first for the error, then append `-<n>` to `dir_name` and
`branch`), T007 (RunsFile to/from JSON via StoredRuns, version check, validate each group; store.rs
runs_path = runs_dir/<project_id>.json, load/save/remove as the reviews trio; FakeProjectStore
needs a runs map); run `cargo test -p micold-core --lib runs` and the protocol/schema tests green;
fill the Green column. Then T011, T014–T027.
Findings for the daemon (T014–T016):
- `CreateMode::NewBranch` runs `git worktree add -b <branch> <path> HEAD`: it starts at the repo's
  HEAD, not the base branch. Decision proposed: add a daemon-only `CreateMode::NewBranchAt { start }`
  marked `#[serde(skip)]` (compatible with `BranchSituation::Free` only; rollback owns the branch),
  a `Git::worktree_add_new_branch_at` with a default impl calling `worktree_add_new_branch` (FakeGit)
  and a real one in `GitCli` passing `start`; `create_command_line` arm. Record it in Decisions.
- An existing run branch with NewBranch(At) gives `CreateError::SituationChanged` (no rollback, the
  branch is untouched) → `Failed { step: Worktree, reason: err.to_string() }`.
- Model `micold-daemon/src/runs.rs` on `review.rs` (`Reviews` map + `Refusal`), state handle as
  `inner.reviews` (state.rs:242, 643, 2150–2160), attach push beside
  `review_pushes_on_attach` (server.rs:733, before the attach's CatalogChanged — the harness's
  `attach` reads up to it), catalog `load_runs`/`save_runs` beside catalog.rs:417–433 and
  `remove_runs` beside catalog.rs:1028. Run task: `ops::create_worktree` → `ops::cli_unavailable`
  (Failed Session) → `ops::create_session_with_prompt(… FirstPrompt { require_bracketed: true })`
  → map `Typing(_)` like mcp/tools.rs:1340. Answer the create before broadcasting.

## Open escalation

None.

## Follow-ups not done

None yet.
