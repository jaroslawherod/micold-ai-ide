# Autopilot ledger — #1815 supervisor-question-routing

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: autopilot supervisor M4: route session questions to the user in one batch (issue #1815); labels: enhancement, in-progress, flow:feature
- **Kind**: feature
- **Effort**: default
- **Supervised**: no
- **Issue**: #1815
- **Worktree branch**: claude/project-thread-y16qre
- **Runs as**: the orchestrator dispatches units with the `autopilot-unit` type in a cloud thread (no `mise`, no `gh`; no PR for the spec unit, pushed to the harness branch only)
- **Started**: 2026-11-14
- **Phase**: milestones
- **Next step**: M2 review A and B round 1, then PR (orchestrator opens it); local gate: cloud, scripts/gate.sh too slow, CI is the gate

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1899 | design (spec, plan, tasks) | merged (squash) | e75744062ef543f93a3085b22f4b846e56d1bf4f |
| #1901 | M1 batch routing core | merged (squash) | afe9a3fe0d243743505cea0a95c47ed551c51e83 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T010 | full | two asking runs produce one batch, one notification, per-session answers | #1901 | merged |
| M2 | T011–T016 | full | routine questions resolved from repo evidence; answer.sh --by | | implemented, in review |
| M3 | T017–T019 | light | context-cap survival: reask, overdue | | pending |
| M4 | T020–T022 | light | upward relay to a reported parent | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | #1858 (parent field, sibling visibility) lives only in open PR #1873, not on main: depend on it? | No. Spec batching, repo-evidence resolution, answer routing and context cap in full against main. Spec the upward relay as conditional: applies when the daemon reports the supervisor's parent; no parent reported means top tier, ask the user. | orchestrator | spec.md Background, US4, FR-012, FR-013 |

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Review spec | 1 | spec | 861681bf64f50b5ce0c1aa81af07c06480367738 | 861681bf64f50b5ce0c1aa81af07c06480367738:fa0e41a737c21651652a6954264cefd2d07b1065 | CHANGES: 4 MAJOR, 2 MINOR, all fixed (round 2 pending) |
| Review spec | 2 | spec | b4f7aad4441e4a553d5eb93fdca3e56b77f1bf20 | 798b505d3b1d77b4744a9e98db61a4b244bfbe7f:1a3b3ce1940b26d815cc6cdc18e61163e7f2b1af | CHANGES: 1 MAJOR, 1 MINOR; fixed, prose only (out-of-scope wording), no round 3 |
| Review plan | 1 | plan | 2fb8d78017e17ab256129910edceaa365b11491e | 1ddc4daf07dab41e2627303773d39f3db679b3b8:2fb8d78017e17ab256129910edceaa365b11491e | CHANGES: 3 MAJOR, 3 MINOR, all fixed (round 2 pending) |
| Review plan | 2 | plan | f776b812d17e464ba995cf3257722a28600bc394 | 6611638df9e715ff6514de81ce5b274a5a25e502:f776b812d17e464ba995cf3257722a28600bc394 | CLEAN |
| Review tasks | 1 | tasks | 31a0ad5679de60b9819e52ace88fc5ae2d865e0a | 6d90bd6be35040bcf4ee9990b7bf877249f9b210:31a0ad5679de60b9819e52ace88fc5ae2d865e0a | CHANGES: 1 MAJOR (F1), 3 MINOR; fixed (F2 split declined), round 2 pending |
| Review tasks | 2 | tasks | b511c1eb1a58470dce0d0a0897113cb619495f25 | 902c2d4b6c4d1d7b47ee7ddaf0ffdedb63fbfa41:b511c1eb1a58470dce0d0a0897113cb619495f25 | CLEAN |
| Review A | 1 | M1 | 376190a4be97904b43a9e7f016686821aac73982 | 2bd106940fdee380edb76ad2873a20ad878779dd:376190a4be97904b43a9e7f016686821aac73982 | CHANGES: 1 MAJOR, 2 MINOR; all fixed (round 2 pending) |
| Review B | 1 | M1 | 376190a4be97904b43a9e7f016686821aac73982 | 2bd106940fdee380edb76ad2873a20ad878779dd:376190a4be97904b43a9e7f016686821aac73982 | CHANGES: 2 MAJOR, 1 MINOR; fixed or declined with reasons (round 2 pending) |
| Review A | 2 | M1 | d95705445c1ef4707649eac990722985e6fe2e40 | 0be4456b2a3083247fe354b62a7f7f4706c8fee9:d95705445c1ef4707649eac990722985e6fe2e40 | CLEAN (1 MINOR: --by supervisor prefix not yet parsed by answer.sh; M2 owns it) |
| Review B | 2 | M1 | d95705445c1ef4707649eac990722985e6fe2e40 | 0be4456b2a3083247fe354b62a7f7f4706c8fee9:d95705445c1ef4707649eac990722985e6fe2e40 | CLEAN (1 MINOR: drift-M1.md not written; reason in Declined table) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | Review A 1 | F1 (carry a batch flag so a batch of one gets `1.`) | run-status JSON cannot tell a batch of one from a single; bare for one question is the answer.sh single shape, and a batch escalation always has 2+ questions. Fixed by dropping the Options test, not by a flag. |
| M1 | Review B 1 | F1 (drift-M1.md) | contracts/routing-cli.md and data-model.md are references cited by T003/T004, not files the milestone creates: false positive. |
| M1 | Review B 1 | F2 (`facts: a 50-file diff` failure) | Flaky and environmental (git push negotiation in the container); untouched code; autopilot.test.sh passed 348/0 on re-runs with and without this diff. CI decides. |
| M1 | Review B 1 | F3 (no red-before-green evidence) | Tests and code were written in one commit; later milestones commit tests first. Noted, not retroactively splittable. |
| all | Review tasks 1 | F2 (split T019 and T022 into test and edit tasks) | Test files are now named; splitting would renumber T020–T022 and every milestone range for no gain, as the failing test comes first within the task. |

## Handover

2026-11-14: M1 PR #1901 squash-merged (afe9a3fe) after Review A and B round 2 CLEAN and green CI; origin/main merged into claude/project-thread-fy8cr5. Next thread starts M2 (T011–T016, tier full) and must land `--by` parsing in answer.sh (see Follow-ups). The new thread pushes to its own harness branch. Cloud: no mise, gh unauthenticated (GitHub MCP for PR/merge, REST `gh api` for check-runs), units have no Agent tool (orchestrator dispatches reviewers), CI is the gate, never rebase or force-push, squash-merge.

## Open escalation

None.

## Follow-ups not done

None.
