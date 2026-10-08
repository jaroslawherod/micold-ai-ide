# Autopilot ledger — #483 parallel-agent-runs

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #483 "Run one prompt across several agents in parallel worktrees and pick the best result" (scope verbatim from the issue); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #483
- **Worktree branch**: claude/project-thread-wysm57
- **Started**: 2026-10-08
- **Phase**: tasks
- **Next step**: tasks unit (continue from Handover: tasks review round 1)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T027 | full | Run in parallel starts N prompted runs, shown under a group row with failures and reasons | — | planned |
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

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Tasks unit, handed over at 161k context. Done: branch-start (rebased onto origin/main ec102904),
speckit-tasks (tasks.md T001–T070, `## Milestones` M1–M5, all full tier), speckit-analyze run once
and every HIGH (I1–I4) and the MEDIUM/LOW it named fixed in spec, plan, data-model, contracts,
research and quickstart (see Decisions), committed e9cc8d6f. Next: tasks.md step 2's fresh reviewer,
round 1 (rubrics/tasks.md, snapshot 6a04f8064c1ab9b25b64c6bc524a600ebe368371:e9cc8d6f13feaa1c4ce1536dc51d9470185fbab8;
the spec-artifact edits from analyze are behaviour-bearing, so the reviewer should check them too);
then step 3 close `checklists/requirements.md`, step 4 Specs-only gate, step 5 design PR (cloud
override: PR body to the scratchpad file, orchestrator opens it). No open findings. No PR yet.

## Open escalation

None.

## Follow-ups not done

None yet.
