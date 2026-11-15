# Autopilot ledger — #1945 mcp-parent-sibling-view

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1945 (issue #1945: MCP: let a session see its parent session, parent worktree and siblings, and message its parent; scope items 1-5 per the issue body); labels: enhancement, flow:feature, in-progress
- **Kind**: feature
- **Effort**: default
- **Supervised**: no
- **Issue**: #1945
- **Worktree branch**: claude/project-thread-x5s2f0
- **Started**: 2026-11-15
- **Phase**: implement M1
- **Next step**: M1 review fixes committed locally, not pushed; round 2 by orchestrator; orchestrator runs reviews A and B, then push (merge origin/main first, no rebase)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1947 | design | MERGED | 00f66d534eedc83b1c08c7614e50f5a32194eaa8 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T005, T016, T017 | full | Unparented sessions see default and siblings | - | implemented, local commit, gate partial (fmt, clippy, core lib, daemon mcp green; workspace nextest not run: disk) |
| M2 | T006–T014 | full | neighbourhood and send_parent_input tools | - | pending |
| M3 | T015–T020 | full | Parent recorded before start; agents told | - | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Unparented worktree visibility | Treated as child of `default` for visibility only, no link recorded | user (jaro, 2026-11-15) | issue #1945 scope item 1 |

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Spec | 1 | spec.md | 214f266cddafb8d31574e169e83779f56cf41864 | 45981247472c178d3a281f97d0df0d2ea12c268b:214f266cddafb8d31574e169e83779f56cf41864 | CHANGES: 3 MAJOR, 2 MINOR (all fixed in 78ada4fd) |
| Plan | 1 | plan.md, research.md, data-model.md, contracts | b0980ab565139b251fc87a9497dd36449c28edc0 | 7c4851301a2b528fc4f864afac018e9f8c408791:b0980ab565139b251fc87a9497dd36449c28edc0 | CHANGES: 4 MAJOR, 1 MINOR (fixed; round 2 by orchestrator) |
| Plan | 2 | plan.md fix diff (sonnet, scoped) | 9422046fe | - | CLEAN, 2 MINOR (stub snapshots the link into its marker; quickstart step 2 adds mcp_cross_session; applied by tasks unit) |
| Tasks | 1 | tasks.md, milestones | ee81b0c00822cfb84eb4da4e069425cb28d221cb | 1661122c6c63da6343b641227e009ff7efcf2f0b:ee81b0c00822cfb84eb4da4e069425cb28d221cb | CHANGES: 1 MAJOR, 3 MINOR (fixed; scoped re-review by orchestrator) |
| Spec | 2 | spec.md (fix diff, sonnet) | 78ada4fd75dd1ca8e7c8ff1aebf16e466acde566 | - | CLEAN, 1 MINOR (FR-006 sentence; fixed in clarify unit) |
| M1 review A | 1 | code-review high | e8d0eb7b6fe8746ec74aa98cd05f9e977e000c6d | 428ab0399774fef30f66d8ad6cc66d69dcfa8650:e8d0eb7b6fe8746ec74aa98cd05f9e977e000c6d | CHANGES (A-F1..F4; fixed in the next M1 commit, T016/T017 pulled in) |
| M1 review B | 1 | conformance | e8d0eb7b6fe8746ec74aa98cd05f9e977e000c6d | 428ab0399774fef30f66d8ad6cc66d69dcfa8650:e8d0eb7b6fe8746ec74aa98cd05f9e977e000c6d | CHANGES (B-F1..F3; cycle-log, drift-M1, implicit loop fixed) |
| tasks | 2 | design | a3392c6f602d1ccf19b73576fce51cddfc115fb5 | 5304203d510eef87bf8351eb34b6d456f161cfe9:a3392c6f602d1ccf19b73576fce51cddfc115fb5 | CLEAN (sonnet, scoped) |
| A | 2 | M1 | e265b36a5efb795a3e5b5f4ddea2878008c05b85 | b04e7ce9281b10af5b6f11587a6984cf9c8ff8d7:e265b36a5efb795a3e5b5f4ddea2878008c05b85 | CLEAN (sonnet, scoped; MINOR: no-parent create no longer retires stale ids, carried to M3) |
| B | 2 | M1 | e265b36a5efb795a3e5b5f4ddea2878008c05b85 | b04e7ce9281b10af5b6f11587a6984cf9c8ff8d7:e265b36a5efb795a3e5b5f4ddea2878008c05b85 | CLEAN (sonnet, scoped; MINOR M3 range fixed) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
