# Plan drift, M1

| Path | Reason |
|---|---|
| `crates/micold-daemon/src/ops.rs`, `crates/micold-daemon/src/state.rs`, `crates/micold-daemon/src/server.rs`, `crates/micold-daemon/src/mcp/tools.rs` | T016, T017 pulled forward from M3 (review A-F1): with M1 an unlinked worktree is an implicit child of `default`, so the link must exist before the child's session starts. `StartDefinition.parent` records it between worktree and session create; a failed write is `WorktreeStartFailure::Link` (a `service_error` naming the worktree, no session started). `state.try_record_worktree_parent` returns the error. |
| `crates/micold-daemon/tests/mcp_create_session.rs` | T016 tests live here, not in `tests/mcp/mcp_worktree_parent.rs`: this file holds the stub-CLI sandbox a missing-CLI case needs. T015 (stub `claude` marker snapshot) stays in M3. |
| `crates/micold-core/src/worktree_parent.rs` | A live directory named `default` has no parent and no siblings and is nobody's child (A-F2); `DEFAULT_NAME` is `WorktreeRef::DEFAULT`; `implicit` is set only for children of `default` (B-F3). |

`data-model.md` is reference only: the code follows it, and it needed no change.

`MISSING crates/micold-daemon/tests/mcp/mcp_worktree_parent.rs`: it held no "no parent has no siblings" expectation, so it needed no change (T003); the amended expectations are in `mcp_worktree_parent_visibility.rs`.
