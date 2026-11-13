# Autopilot ledger — #1858 worktree-parent M1

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Milestone M1 of specs/1857-worktree-parent/tasks.md (T001-T014, T020-T022); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Supervised**: no
- **Issue**: #1858
- **Worktree branch**: feat/1858_worktree-parent-m1-persist-and-expose-the-parent
- **Started**: 2026-11-13
- **Phase**: milestones
- **Next step**: gate, reviews A and B, then PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1861 | Design (spec, plan, tasks; merged content, treated as reviewed) | not merged, content on this branch | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T014, T020–T022 | full | Stable worktree ids and parent links in per-project state; create_worktree records the parent; list_worktrees/list_sessions/get_session report it; removed tombstone; user guide | #… | in-progress |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | milestones | Where do the new daemon tests live? | In `crates/micold-daemon/tests/mcp/` as module `mcp_worktree_parent` (one binary, #1175), not new top-level files; catalog tests are in the same module | agent-resolved | tests/mcp/main.rs |
| D2 | milestones | Pure id/link rules | In `micold-core/src/worktree_parent.rs` (unit-tested), applied by `Catalog` | agent-resolved | plan.md |
| D3 | milestones | Relocation test (T011) | No relocation fixture exists for per-project state (state is keyed by project path); covered by the store round trip and daemon restart tests | agent-resolved | store.rs `project_state_path` |

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
|---|---|---|---|---|---|

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Done (uncommitted-to-green; NOTHING COMPILED YET, the build lock was held by other worktrees for 20+ min): all M1 code and docs are written but never built or run.
- core: `WorktreeId/ParentRef/ParentState/ParentLink` in `crates/micold-core/src/worktree.rs`; pure rules + unit tests in `crates/micold-core/src/worktree_parent.rs` (registered in lib.rs); `worktree_ids`/`worktree_parents` in `workspace.rs` and the per-project state in `store.rs` (+ `worktree_parent_tests` module); MCP schema/parse in `crates/micold-core/src/mcp/tools.rs` (`Operation::ListWorktrees { parent }`, `parent` on both `CreateWorktreeRequest` variants, `parent()` accessor); core tests updated (`mcp_tools_catalog.rs`, `mcp_policy.rs`).
- daemon: `Catalog` methods (`reconcile_worktree_parents`, `record_worktree_parent`, `forget_worktree_parent`, `worktree_parent_views`, `worktree_ids`) in `catalog.rs`; `DaemonState` wrappers + reconcile call inside `refresh_worktrees` in `state.rs`; `forget_worktree_parent` in `ops::delete_worktree`; MCP `parent_choice`, `insert_parent`, `Context.parents`, `list_worktrees` filter, session rows, record after create in `crates/micold-daemon/src/mcp/tools.rs`.
- daemon tests: `crates/micold-daemon/tests/mcp/mcp_worktree_parent.rs` (registered in `tests/mcp/main.rs`); includes the catalog tests (T004), US1-US3, US5.
- docs: `docs/user-guide/agent-tools.md` (T010, T014, T022).
Next step: (1) `cargo fmt`, then ONE detached `scripts/build-lock.sh cargo check --workspace --all-targets` (log in scratchpad; use hold.sh; a stale queued one of mine may still be waiting, ignore or let it run) and fix compile errors (likely spots: `refresh_worktrees` borrow of `gone` closure / `inner.catalog` in state.rs; `Context` literals; `WorktreeRef` import in daemon tools.rs; test imports in mcp_worktree_parent.rs). (2) run targeted tests: `scripts/build-lock.sh cargo test -p micold-core --lib worktree_parent`, `-p micold-core --test mcp_tools_catalog --test mcp_policy`, `-p micold-daemon --test mcp worktree_parent`; fix. (3) tick T001-T014, T020-T022 in tasks.md (none ticked yet), TDD cycle log not written (tests were written after code: say so). (4) scoped gate + review A in its shadow, review B, full gate, PR per verify.md/pr.md; PR title `feat(1857): record and expose the worktree parent (#1858)`, body ends `Refs #1858`. Open questions: none. Reviews run: none. PR: none opened. branch-start.sh already ran (rebased 4 commits onto origin/main).

## Open escalation

None.

## Follow-ups not done

None.
