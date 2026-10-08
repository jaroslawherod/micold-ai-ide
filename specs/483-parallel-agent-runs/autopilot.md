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
- **Next step**: M1a: write T011 (`run_group_persist.rs`), then reviews A+B, full gate, push, PR body (see Handover)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #643 | Design PR (spec, clarifications, plan, research, contracts, tasks) | merged | adfee3b1c820e9393c4c7824855cc1090efeeac0 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1a | T001–T011, T014–T016 | full | Service starts N prompted runs as one persisted, pushed group; a failing run fails alone | — | in progress |
| M1b | T012, T013, T017–T027 | full | Run in parallel dialog and the group row with failures and reasons | — | planned |
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

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M1 unit 3 handed over at ~155k context (milestone unit; no PR open). Stay on branch
`claude/project-thread-wysm57` (skip `branch-start.sh`; `gh` unauthenticated; no `mise`). Run cargo
detached via `$SCRATCHPAD/bg.sh <name> scripts/build-lock.sh cargo …` (sets `CARGO_INCREMENTAL=0`,
log ends `JOB_EXIT=`), wait with `scripts/autopilot/hold.sh <log>`.
Done (committed "feat(483): M1a core, NewBranchAt, daemon run groups"): T001–T010, T014–T016
green; `cargo test -p micold-core` passes except the 4 root-only permission tests;
`run_group_create` 8/8; `worktree_create` 17/17. tasks.md split into M1a/M1b (see Decisions).
Next step (M1a only):
1. Write T011 `crates/micold-daemon/tests/run_group_persist.rs` on `tests/support/runs.rs`
   (`#[path = "support/runs.rs"] mod runs_support;`, `#![cfg(unix)]`, `ENV` guard): file holds the
   group at the first push and only `<id>.json` remains in `<store>/runs` after settling (temp
   files are `<name>.<pid>.<n>.tmp`); W5 failing write = regular file at `<store>/runs` →
   `OperationError` (IoFailed), no worktree/branch/push, a fresh attach gets empty groups; two
   windows get the same push; frame right after `Attached` is the one `RunGroupsChanged`;
   `ClientMsg::ProjectRemove` removes the runs file. It should pass on the current code (record
   red as "passes on arrival" honestly in tdd/cycle-log.md). Tick T011.
2. Review A (`code-review` high) on `origin/main...HEAD` + scoped gate; review B (conformance,
   sonnet) via `claude -p --agent autopilot-reviewer`. 3. Full gate (raw commands of mise.toml
   `gate`, CARGO_INCREMENTAL=0; 6 root-permission failures only → `echo <sha> >>
   .git/autopilot-gate-ok`, own Bash call), push `--force-with-lease -u origin
   claude/project-thread-wysm57`, PR body to `$SCRATCHPAD/pr-483-m1a.md` (title "#483: Run one
   prompt across several agents in parallel worktrees and pick the best result (M1a)", ends
   `Refs #483`); return `PR: pending (body at …)`.

## Open escalation

None.

## Follow-ups not done

None yet.
