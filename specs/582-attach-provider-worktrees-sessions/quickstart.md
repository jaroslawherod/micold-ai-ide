# Quickstart: Attach a Provider's Existing Worktrees and Sessions

Contracts: [attach-worktree-tool](contracts/attach-worktree-tool.md),
[list-resumable-sessions-tool](contracts/list-resumable-sessions-tool.md),
[attach-protocol](contracts/attach-protocol.md). Types: [data-model.md](data-model.md).

## Part A: automated

```bash
mise run test-core    # attach unit tests, tool catalog, policy, protocol round trip
mise run gate         # fmt, clippy, workspace tests, scripts/tests
```

- `crates/micold-core/tests/provider_store_dirs.rs`: FR-014 (provider store locations).
- `crates/micold-client/tests/attach_dialog.rs`: FR-001, FR-004, FR-008 (dialog state; the reducer tests have no recorded red, see tdd/cycle-log.md M1).
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

- **Part A**: `mise run gate` passed on the final tree (fmt, clippy, `cargo test --workspace`, scripts/tests). On this host it needs `MICOLD_SKIP_GH_LAUNCH_TEST=1`, because `a_desktop_launch_finds_a_working_gh` fails here for host reasons; CI is unaffected.
- **Part B, what was exercised** (evidence in [visual/results.md](visual/results.md), from three passes on earlier commits, worktrees-only fixtures): the "Attach existing..." button and dialog (B2, light and dark); "Attach all" from the dialog, nothing left to attach on a second press, uncommitted changes untouched (B3); stored-session rows, Resume, and the footer note (M2 pass, light and dark); the start-up banner and its Dismiss (B1 and the dismiss half of B2, light and dark).
- **Part B, what was NOT exercised**: the banner with stored sessions seeded (its sessions text and the "Review" button), the banner's own "Attach all" and its confirmation message, and "sessions show idle" in B3. Those are covered by `crates/micold-client/tests/attach_offer.rs` and `attach_dialog.rs` only. Not a full B1-B4 pass on the final tree.
- **Open visual observations**, listed in the ledger's follow-ups and in results.md: light-theme sidebar staying dimmed after the dialog closes and a misaligned new session row (M2 defects 3 and 4; possibly lavapipe or XTEST artefacts, not retested), the dialog lingering a few seconds after Resume, and an empty gap in the dialog. They are not recorded as blockers because no behaviour is wrong, but they are unresolved.
- **Deviations from the plan, as built**: "Attach all" in the banner attaches worktrees only; with only sessions found the button reads "Review" and opens the dialog (sessions are resumed one at a time). `provider` in the MCP tools is `claude_code`; `total` counts at most the 200 newest sessions. See `tdd/cycle-log.md`.
