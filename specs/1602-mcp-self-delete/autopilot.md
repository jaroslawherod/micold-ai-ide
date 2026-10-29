# Autopilot ledger — #1602 mcp-self-delete [BUG-<issue>]

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1602 (issue #1602: MCP: let a session stop itself and delete its own worktree, after confirmation); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1602
- **Worktree branch**: feat/1602_mcp-let-a-session-stop-itself-and-delete-its-own
- **Started**: 2026-10-29
- **Phase**: tasks
- **Next step**: Design PR merges; then milestone M1.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1611 | Design | open | … |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| C1 | clarify | Story 4 (stop/delete self) in scope? | Yes | user | spec.md Clarifications |
| C2 | clarify | Failed deferred removal reported how? | App notice naming W and reason, plus daemon log | user | spec.md Clarifications |
| M1 | T001–T016 | full | Own-worktree delete after confirmation (incl. wire bump 74, open-handle test) | #… | pending |
| M2 | T017–T025 | full | Delete prompt lists uncommitted files and unpushed commits | #… | pending |
| M3 | T026–T030 | full | Session stops or deletes only itself | #… | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | d0f9fbb3c51a39e109610a93625505f8ad347b7d:2d2d95f3fae526ce5fcff1f4f34c63d838d9004c | CLEAN (3 MINOR, fixed, prose only) |
| Plan review | 2 (scoped, sonnet) | d39a7dfcbc667917467e82b2b58c34ddfb9b6c65:8e4b6b4b0c4feab124dfc49f3c7d767bd6fcb81e | CLEAN (1 MINOR fixed, prose only: policy test path) |
| Plan review | 1 | 0b78d32b342988dd1f35378088cb36a84b3aa567:9dff441a2bbac0be6f7d913c2c0e5dc53484cc7b | CHANGES (3 MAJOR, 3 MINOR, all fixed, prose only: test rows, until_answered plumbing note, R6 wording) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None. <or, when a unit handed over: what is done, the next step, open findings with their review
snapshots. The next unit sets it back to None.>

## Open escalation

None.

## Follow-ups not done

- M1 has 16 tasks (over the 10-task guide): it is Foundational (wire bump) plus two P1 stories sharing `decide` and `Deferred`; no acceptance-scenario split gives a observable deliverable. Accepted.
- R6: T007 open-handle test; T015 extends the `ops.rs` retry bound only if the Windows CI job of M1's PR fails. Orchestrator: check the Windows job on M1.
- Spec FR ids renamed: local FR-014a to FR-017, FR-015a to FR-016 (034's FR-015a keeps its name).
