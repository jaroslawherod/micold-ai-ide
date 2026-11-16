---
feature: 1945-mcp-parent-sibling-view
verdict: PASS_WITH_GAPS
standard: .specify/extensions/tdd/templates/tdd-test-quality-rubric.md
mode: quick (no mutation run), not independent (the closing unit shares the run with the authors)
verified_at: 7ef5fd5b1
behaviours: 18 cycles in tdd/cycle-log.md (no test-list.md: the feature was not planned through the TDD extension)
---

# TDD verification: 1945

**Verdict: PASS_WITH_GAPS.** Every FR and SC has a test through the real MCP entry point and no existing assertion was weakened; the gaps are red phases proven by mutation or not observed, and test strength unmeasured by a mutation tool.

## Test-first evidence

| Cycles | Class | Basis |
|---|---|---|
| 3, 8, 9, 11, 12, 13, 14, 15, 17, 18 | PROVEN | red output recorded (stubbed arm, origin/main sources, compile error or failing assertion) |
| 10, 16 | PROVEN by mutation | the test was written with the code; the recorded red is a mutation of the shipped code (16 was redone after review A found the first version had no red) |
| 1, 5, 6 | LIKELY / accepted gap | tests written with the code (1) or green on the old code too (5, 6); the log says so (review B-F1) |
| 2, 4, 7 | PROVEN by mutation or old sources | recorded |

## Existing tests

Removed assertions in the feature's PRs: M1 amends the three spec 1857 "no parent has no siblings" expectations (T003, intended by FR-001); M2 moves a `git worktree remove` and a prompt-wait loop into helpers (`remove_worktree`, `ready`) with the same checks. No `#[ignore]`, no loosened tolerance, no threshold change.

## Findings

- F1 MINOR: cycles 1, 5 and 6 have no red (logged as accepted). Cycle 5 and 6 guard against failing open and the old post-call record; the ordering case is covered by cycle 16.
- F2 MINOR: no mutation tool run (below); test strength rests on the deliberate-mutation reds in cycles 2, 7, 10, 16.

## Traceability

| Criterion | Tests (cycle) |
|---|---|
| FR-001, FR-002, SC-001, SC-005 | `worktree_parent` unit tests (1, 2), `mcp_worktree_parent_visibility` (3, 4, 5) |
| FR-003, FR-004, FR-005, SC-002 | `mcp_worktree_parent_neighbourhood` (9, 12, 13) |
| FR-006, FR-007, FR-008, SC-003 | `mcp_cross_session` parent tests (11, 12, 13, 17), `parent_target` (10), `mcp_audit_log` (12) |
| FR-009, SC-004 | `mcp_create_session` (6, 7, 15, 16, 18) |
| FR-010 | grep check in `scripts/tests` (T019), user guide wording (T018) |
| D2 reserved name | `branch_conflict`, `mcp_create_session` (14) |

Every criterion has a test through the real entry point; no test traces to nothing.

## What was not audited

Mutation testing with a tool (`quick`; the `mutants on the diff` job fails at baseline on unrelated netns tests, commented on #1965). Coverage beyond CI's `diff coverage` gate. Test speed and flakiness beyond CI runs. The smell pass was a read of the cycle log and the test diffs' removed lines, not a line-by-line catalogue walk of every new test.
