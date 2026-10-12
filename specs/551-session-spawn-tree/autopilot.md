# Autopilot ledger — #551 session-spawn-tree

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Show sessions spawned by an agent as a tree under their origin session (issue #551); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #551
- **Worktree branch**: feat/551_show-sessions-spawned-by-an-agent-as-a-tree-under
- **Started**: 2026-10-12
- **Phase**: tasks
- **Next step**: review tasks, gate, open design PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T016, T025, T038 | full | Origin recorded via create_session, persisted, exposed on wire and MCP; spike T025 | | pending |
| M2 | T017–T024, T039 | full | Sidebar nests children with child/pending/removed markers | | pending |
| M3 | T026–T031, T040 | full | Provider readers for spike-confirmed providers, discovery, provider table | | pending |
| M4 | T032–T037, T041 | full | Fold, count, attention rollup, persisted fold | | pending |

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
| Plan 551 | 1 | d39d189620904732240bb92b8325ded71e3e99d0:66c580d0da56f9eb86256315e6aec379967b5a15 | CHANGES: 2 MAJOR (F1 child removal emit, F2 fold prune), 4 MINOR; all fixed |
| Plan 551 | 2 | d3b3e472cd18ecf87fe544cdb57b74a377bf70c4:66c580d0da56f9eb86256315e6aec379967b5a15 | CHANGES: 1 MAJOR (single-delete path missing, fixed; prose only, paths verified by grep), 2 MINOR fixed |
| Tasks 551 | 1 | e04f39460adaad4619f835ff528eb2909e11e215:f602c37c08334f421b8583446a4439c4bde3f094 | (running) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

Companion issue #552 (replace name-based hiding with native hooks) is out of scope.

Tasks unit: speckit-analyze fixed before review: spike moved to M1 (T025), `origin_rejected` tombstone added (data-model, T005, T030), FR-011 count pinned to direct children, spec Assumptions and Clarification 1 corrected to research R2/R6. Polish (quickstart Part B, ticks) left to the close unit.
