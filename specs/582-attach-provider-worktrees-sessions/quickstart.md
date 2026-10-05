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

## Result (T037, 2026-10-05)

- **Part A**: `mise run gate` passed on the final tree (fmt, clippy, `cargo test --workspace`, scripts/tests; run with `MICOLD_SKIP_GH_LAUNCH_TEST=1` on this host, CI unaffected). Every test file listed above is in the workspace run.
- **Part B**: passed across the milestone passes in [visual/results.md](visual/results.md): B1 and the dismissal half of B2 and B4 (M4, banner and dismiss, light and dark), B2 and B3 (M1 dialog and Attach all; M2 stored sessions and Resume, light and dark). Defects and observations are listed there, none blocking.
- **Deviations from the plan, as built**: "Attach all" in the banner attaches worktrees only; with only sessions found the button reads "Review" and opens the dialog (sessions are resumed one at a time). `provider` in the MCP tools is `claude_code`. See `tdd/cycle-log.md`.
