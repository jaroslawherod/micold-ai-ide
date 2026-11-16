# Autopilot ledger — #1726 app-via-orchestrator

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1726 "feat: the desktop app talks only to the orchestrator (aggregated lists, routed ops and streams, starts it when absent)" (part of #1722; covers FR-005, FR-006, FR-007, FR-010; blocked by #1725, closed). Cloud overrides: no mise, gh unauthenticated, gate scripts/gate.sh, GitHub MCP for PRs; desktop change minimal (iced app sunsetting), prefer orchestrator/protocol-level tests; aim 3-4 milestones. labels: effort:high, flow:feature
- **Kind**: feature
- **Effort**: high
- **Supervised**: no
- **Issue**: #1726
- **Worktree branch**: claude/project-thread-gq91k7
- **Started**: 2026-11-16
- **Phase**: milestones
- **Next step**: gate, push, PR for M1
- **PR**: none

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1996 | design | merged | 9f6e6784d |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T013 | full | contract, core, requests and streams routed through the orchestrator | | implemented, round 1 fixed, awaiting round 2 |
| M2 | T014–T016 | full | aggregated lists, 2 s deadline, SC-003 bench | | pending |
| M3 | T017–T022 | full | desktop app uses one orchestrator connection | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | FR-002 per-daemon list deadline | 2 s, daemons asked in parallel | agent | SC-004 budget |
| 2 | clarify | Who writes `[orchestrator_link]` into provisioned daemons | this feature, the app on provision (FR-013) | agent | specs/1725-daemons-dial-orchestrator/spec.md#Assumptions |
| 3 | tasks | M1 has no observable user story (milestones rule 1) | superseded by #4: contract+core merged into the US2 milestone | agent | tasks review r1 F1 |
| 4 | tasks | Milestones bundling two stories (rule 2) | keep combined, reasons in each milestone's Note (US4 halves and US5 have no code of their own) | agent | tasks review r1 F4 |

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| spec | 1 | design | 7c25a28d9e4bec1417923f0dd050b8627f79bebd | f3b201b3e2dc11ea9d3cb4d365efcd62e7a71cae:b3f47d077d50c0a5339d29f608b5f351591a7057 | CLEAN (3 MINOR applied in clarify) |
| plan | 1 | design | e458a16fca33bf98126ebd10ded77ccc18266133 | 8487a8d58e89bb17100164e9cf76b5dfba6a0c1d:e458a16fca33bf98126ebd10ded77ccc18266133 | pending |
| plan | 2 | design | 6e241d6b1c3744d78f712e7998d7f26265227f3a | a6f9de08200acab60e831c1660b93e5f63aeb46a:6e241d6b1c3744d78f712e7998d7f26265227f3a | pending |
| plan | 2 | design | 6e241d6b1c3744d78f712e7998d7f26265227f3a | a6f9de08200acab60e831c1660b93e5f63aeb46a:6e241d6b1c3744d78f712e7998d7f26265227f3a | CLEAN (1 MINOR applied: R6 slugify trim/fallback) |
| tasks | 1 | design | aac33e2f643fabba7645ab9a83e0acec7db3e0c8 | a3a350cbee7cbea5eb623c27828a8217ba16f17a:aac33e2f643fabba7645ab9a83e0acec7db3e0c8 | CHANGES (2 MAJOR, 2 MINOR; all fixed) |
| tasks | 2 | design | 64197e73ad0729840a83965f8c5364b7950443fd | c73c0d8512eb2c789d089baa13c46fcb6cb0613a:64197e73ad0729840a83965f8c5364b7950443fd | CLEAN |
| code A+B (M1) | 1 | M1 | 52b08b293b4bf90cb8227ce28d2724bdbcd4cebe | 181479f93ce2eaed78d7913c940dba7f37a4ea0b:52b08b293b4bf90cb8227ce28d2724bdbcd4cebe | pending (orchestrator dispatches) |
| code A+B (M1) | 1 | M1 | c2d10f85181940a6640d88ff0061ac7680fefb1f | 181479f93ce2eaed78d7913c940dba7f37a4ea0b:52b08b293b4bf90cb8227ce28d2724bdbcd4cebe | CHANGES fixed: A F1-F5 (hub keeps queued answers, answered only once queued, per-route queue toward link, seq gap; F4 documented in contract), B F1 (drift-M1.md) |
| code A+B (M1) | 2 | M1 | 2aa9f0a4e5ba1b186cb09e0b8a2c7bf7ce2c1ee6 | 5aa7439a20be847cc0dd0815fc78ff252713794f:2aa9f0a4e5ba1b186cb09e0b8a2c7bf7ce2c1ee6 | CLEAN (MINOR 1 fixed in contract prose; 2 and 3 declined as bounded/unchanged) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done
