# Autopilot ledger — #1602 mcp-self-delete [BUG-<issue>]

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1602 (issue #1602: MCP: let a session stop itself and delete its own worktree, after confirmation); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1602
- **Worktree branch**: feat/1602_mcp-let-a-session-stop-itself-and-delete-its-own
- **Started**: 2026-10-29
- **Phase**: spec
- **Next step**: Apply the user's answers to the 2 markers (clarify, continued). Was: run the clarify unit: resolve the 2 [NEEDS CLARIFICATION] markers in spec.md (story 4 scope; deferred-failure reporting).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #… | Design | open / merged | … |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T018 | full | … | #… | pending / in-progress / in-review / ci / merged |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | … | … | agent-resolved / user | <path#section, or "AskUserQuestion YYYY-MM-DD"> |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | d0f9fbb3c51a39e109610a93625505f8ad347b7d:2d2d95f3fae526ce5fcff1f4f34c63d838d9004c | CLEAN (3 MINOR, fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None. <or, when a unit handed over: what is done, the next step, open findings with their review
snapshots. The next unit sets it back to None.>

## Open escalation

Clarify round 1 (2026-10-29), repo does not settle either question (034 has no notice mechanism; issue says "Optionally"):
1. Story 4 (`stop_session`/`delete_session` on self) in scope? Recommended: yes.
2. Failed deferred removal reported how? Recommended: app notice naming W and reason, plus daemon log; session stays stopped.

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>
