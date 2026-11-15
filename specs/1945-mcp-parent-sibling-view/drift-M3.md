# Plan drift, M3

| Path | Reason |
|---|---|
| `crates/micold-core/src/worktree.rs`, `crates/micold-core/tests/branch_conflict.rs` | Decision D2: `CreateError::ReservedName`, `is_reserved_worktree_name` and `explain_reserved_name`; `create_worktree` (the one funnel for the app, MCP and runs) refuses the name `default`, case-insensitively, before touching git or disk. |
| `crates/micold-daemon/src/server.rs`, `crates/micold-daemon/src/runs.rs` | D2: the new `CreateError` arm (`invalid_input` for the app's create, the run's failure reason). |
| `crates/micold-daemon/src/mcp/tools.rs` | D2: `create_failure` arm (`invalid_input`). Review A round 3 (a): "only shell sessions" only when the parent has no AI CLI session at all. F4: the `send_parent_input` scope check is a `debug_assert!` (the parent is always in scope). |
| `crates/micold-daemon/src/ops.rs` | Review A round 2 MINOR: `create_worktree_with_session` records the parent even when none is given, which retires a stale id and link under a reused directory name and mints the worktree's own id. |
| `crates/micold-daemon/tests/mcp_create_session.rs` | T015 lives here, not in `mcp_cross_session.rs` (the create sandbox with stand-in CLIs is here): the link is recorded before the CLI's launch line appears. Also D2 over MCP and the no-parent retire test. |
| `crates/micold-daemon/tests/mcp_cross_session.rs` | Review A round 3 (a): a stopped AI CLI session beside a running shell is not "only shell sessions". |
| `docs/user-guide/worktrees-and-sessions.md` | D2: the reserved name in the app's new-worktree text. |
| `specs/1857-worktree-parent/spec.md` | T020: notes that 1945 amends FR-003 and FR-015. |
| `scripts/tests/agent-tools-parent-docs.test.sh` | T019: the grep check for the three items (neighbourhood tool, parent route, implicit parent) in guide and skill, plus the reserved name. |
| `specs/1945-mcp-parent-sibling-view/drift-M2.md` | Table header added before the appended rows. |
| `crates/micold-daemon/src/ops.rs`, `server.rs`, `runs.rs` | Review A round 1 F2: a no-parent create with a session only logs a failed link write; `ops::create_unparented_worktree` (new) records no parent after `create_worktree`, used by the app's no-session create and the runs, so every create path retires a stale id. |
| `.claude/skills/autopilot-supervisor/SKILL.md`, `specs/1945-mcp-parent-sibling-view/tasks.md` | Review A round 1 F3 (worker prompt line names `send_parent_input` and `neighbourhood`); review B F2 (T015's path). |
