# Autopilot ledger — #551 session-spawn-tree

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Show sessions spawned by an agent as a tree under their origin session (issue #551); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #551
- **Worktree branch**: feat/551_show-sessions-spawned-by-an-agent-as-a-tree-under
- **Started**: 2026-10-12
- **Phase**: plan
- **Next step**: plan unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | Fold state persisted across restarts? | Persisted; never-folded origin starts expanded (FR-017) | agent | specs/017-material-component-architecture/spec.md#FR-016 |
| 2 | clarify | Children of a stopped or deleted origin? | Stopped: stay nested. Deleted: top-level "origin removed", never deleted (FR-018) | agent | spec.md FR-006, FR-008, US2.3 |
| 3 | clarify | MCP create_session child of caller? | Yes, same worktree only (FR-019) | agent | specs/034-daemon-mcp-server/contracts/binding.md#2, crates/micold-daemon/src/mcp/tools.rs |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec 551 | 1 | b598212212983763962acb3bdf6d453c5d13095c:13165b3a9e4938a42528aae5df288475aa63ddae | CHANGES: 1 MAJOR (F1 origin pending vs removed, fixed: FR-016), 3 MINOR fixed |
| Spec 551 | 2 | 72380bb32aa8cae88205f35b4fcae1ce1459a190:13165b3a9e4938a42528aae5df288475aa63ddae | CLEAN (2 MINOR fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

Companion issue #552 (replace name-based hiding with native hooks) is out of scope.
