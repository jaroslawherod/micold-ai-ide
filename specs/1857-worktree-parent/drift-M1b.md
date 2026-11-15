# Drift, milestone M1b

- D7 named `mcp_read_tools.rs` and `mcp_lifecycle_tools.rs`. The one visibility filter also changes
  what every caller outside `default` sees, so these tests, which drive tools as a worktree's
  session, needed a caller that sees the whole project or a fixture adjusted to the new scope:
  - `tests/attach_mcp.rs`, `tests/codex_opencode_start.rs`, `tests/mcp_audit_log.rs`,
    `tests/mcp_create_session.rs`, `tests/mcp_cross_session.rs`, `tests/mcp_default_prompt.rs`,
    `tests/mcp_read_latency.rs`, `tests/mcp/mcp_create_worktree.rs`,
    `tests/mcp/mcp_worktree_parent.rs`: they act from a session in a worktree on targets outside
    its parent/sibling/child set (the tests predate FR-015); they now use `full_view_credential`
    or place the target in scope. Behaviour under test is unchanged.
  - `tests/support/mcp.rs`: adds `full_view_credential`, a credential for a session that sees the
    whole project (the caller's worktree becomes a child of `default` and the parent of all others), shared by the files above.
- Review B F4: `Scope` carries the paths of out-of-scope worktrees and `Scope::explain` words a
  branch's holder generically for them (FR-015 covers error text, not only lists).
