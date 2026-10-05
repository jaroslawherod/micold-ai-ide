# Quickstart: Attach a Provider's Existing Worktrees and Sessions

Contracts: [attach-worktree-tool](contracts/attach-worktree-tool.md),
[list-resumable-sessions-tool](contracts/list-resumable-sessions-tool.md),
[attach-protocol](contracts/attach-protocol.md). Types: [data-model.md](data-model.md).

## Part A: automated

```bash
mise run test-core    # attach unit tests, tool catalog, policy, protocol round trip
mise run gate         # fmt, clippy, workspace tests, scripts/tests
```

- `crates/micold-core/tests/attach_discovery.rs`: FR-001, FR-006, FR-007, FR-009, FR-010, FR-014, SC-002, SC-005.
- `crates/micold-core/tests/mcp_tools_catalog.rs`, `mcp_policy.rs`: FR-005, FR-011, FR-015.
- `crates/micold-daemon/tests/attach_apply.rs`: FR-002, FR-003, FR-004, FR-008, FR-013, FR-016, SC-001, SC-003.
- `crates/micold-daemon/tests/attach_mcp.rs`: Story 3, SC-004.
- `crates/micold-client/tests/attach_offer.rs`: FR-012, Story 4, dismissal (R5).

## Part B: visual pass (run with the `visual-pass` skill)

Needs a private Xvfb display, a scratch `XDG_DATA_HOME` (empty catalog), a scratch git repo with 3 worktrees under `.claude/worktrees/`, and a scratch `~/.claude/projects/` seeded with 5 sessions for the repo and 2 for another repo.

| Step | Do | Pass when |
|---|---|---|
| B1 | Open the repo | A banner offers attaching the 3 worktrees (and any not-yet-adopted sessions); "Show agent worktrees" is off |
| B2 | Dismiss, then open "Attach existing…" | The list still shows all items; a note appears if a store is unreadable |
| B3 | Attach all | 3 worktrees show in the sidebar, no agent chip; sessions show idle |
| B4 | Repeat B1 in the light and dark theme | Banner and dialog readable in both |
