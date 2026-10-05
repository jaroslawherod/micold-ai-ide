# Implementation Plan: Attach a Provider's Existing Worktrees and Sessions

**Branch**: `feat/582-attach-provider-worktrees-sessions` | **Date**: 2026-10-05 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/582-attach-provider-worktrees-sessions/spec.md`

## Summary

Three additions on top of machinery that already exists, no new storage:

1. **Attach a worktree = the existing claim.** Feature 029's `Catalog::claim_worktree` writes the
   same provenance record a created worktree gets, which is what flips `classify_owner` from
   `Agent` to `User` and un-hides the row. Attaching reuses it unchanged (no git, no filesystem
   write: FR-003), adds a batch form ("attach all") and a validating front for the MCP tool.
2. **Discover resumable sessions** with a new read-only pass in `micold-core` (`attach.rs`) that
   reuses `AiCliProvider::recorded_session_ids / read_title / read_label / is_archived`. It reads
   the provider stores of the project root, every git-listed worktree (including missing and
   prunable ones, which are reported unresumable) and, for Claude Code, store directories that name
   a deleted worktree of this project. It lists; it adopts nothing.
3. **Surface it**: an `attach_worktree` and a `list_resumable_sessions` MCP tool, a sidebar
   "Attach existing…" dialog (reusing the dialog and button primitives), and a one-time offer banner
   when the project has no provenance records.

Spec alignment note (recorded in research R2): feature 026's `discover_external_sessions` already
adopts, at every project open, sessions found at the project root and at every startable worktree.
That keeps working (FR-013; its tests pin it). The new pass lists what that adoption does not
reach or the user cannot see yet: sessions at worktrees that are not startable, store entries for
deleted worktrees, and adopted sessions whose worktree is still hidden (attaching the worktree is
what shows them).

## Technical Context

**Language/Version**: Rust, stable toolchain via `mise`

**Primary Dependencies**: existing only: `iced` (client), `serde`/`serde_json`, `uuid`, `tokio` (daemon), `directories`. No new crate.

**Storage**: none new. Attach writes the existing per-project provenance records (`store.rs`,
via `Catalog::claim_worktree`); discovery reads provider stores read-only (FR-010).

**Testing**: `cargo test` through `mise run test-core`, `mise run gate`; daemon integration tests
(`crates/micold-daemon/tests/`), client reducer tests (`crates/micold-client/tests/`), the
`visual-pass` skill for quickstart Part B.

**Target Platform**: Linux, macOS, Windows (Principle VI). Paths are `PathBuf`; the Claude store
directory name is derived by the provider's existing `transcript_dir` (alphanumeric-or-`-`), which
has no platform branch.

**Project Type**: desktop-app: three crates, `micold-core` (render-free logic, protocol, MCP
catalog), `micold-daemon` (single writer of `projects.json`, MCP server), `micold-client` (iced).

**Performance Goals**: SC-001: 16 worktrees attached in one action in under 10 s; listing a store
of thousands of sessions stays responsive (one directory listing per location, newest first, a
bounded page for the tool).

**Constraints**: offline; read-only toward provider files; one catalog write per attach action.

**Scale/Scope**: tens of worktrees, thousands of stored sessions per project.

## Constitution Check

*GATE: passed before Phase 0; re-checked after Phase 1 (no change).*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Pure logic lands in `micold-core` first with failing tests; the test list in research R10 maps each FR to a layer. Only `src/ui/` view glue uses the quickstart exception. |
| II. Multi-Session | PASS | Attaching a session adds an idle entry in its own location; nothing is shared. Resume goes through the existing per-session start with the existing `Starting/Running` guard (FR-016). |
| III. Worktree Integration | PASS | Every attached session maps to a catalog worktree or Default (resume attaches the worktree first). `attach_worktree` is refused for a Default caller (FR-015) in `mcp/policy.rs`; Principle III is not amended. |
| IV. Local-First | PASS | Local filesystem and git only. |
| V. Rust + iced | PASS | `AttachTarget`, `ResumableStatus` and `SkipReason` are enums, so "resumable without a worktree" is not representable. |
| VI. Cross-Platform | PASS | No OS branch in core; discovery goes through the existing provider seam; CI runs the matrix. |
| VII. Documentation | PASS | `docs/user-guide/agent-tools.md` and `worktrees-and-sessions.md` updated in the same change (task in tasks.md). |
| VIII. Reusable UI | PASS | The dialog and banner are built from existing shared primitives (dialog, button, list row) with builder calls; if no banner primitive exists, one is added to the shared library (R9), not a feature-local widget. |

## Requirement map

| Requirement | Where |
|---|---|
| FR-001 list attachable | `attach::attachable_worktrees` (core); `ClientMsg::AttachDiscover` (R6); `list_worktrees` unchanged |
| FR-002, FR-003, FR-004 | `Catalog::attach_worktrees` over `claim_worktree`; client `Message::Attach` |
| FR-005, FR-015 | `attach_worktree` in `mcp/tools.rs` (core), `mcp/policy.rs`, daemon `mcp/tools.rs`; [contracts/attach-worktree-tool.md](contracts/attach-worktree-tool.md) |
| FR-006, FR-007, FR-014 | `attach::discover_resumable` over `AiCliProvider`; per-provider stores in research R3 |
| FR-008 | resume = `start_session(LaunchMode::Resume)` after `attach_worktrees` for its worktree; [contracts/attach-protocol.md](contracts/attach-protocol.md) |
| FR-009, FR-010 | `DiscoveryReport.notes`; the pass only calls read methods |
| FR-011 | `list_resumable_sessions`; [contracts/list-resumable-sessions-tool.md](contracts/list-resumable-sessions-tool.md) |
| FR-012 | offer banner state in `features/attach.rs`; dismissal R5 |
| FR-013 | `classify_owner` untouched; test that an unattached provider worktree stays hidden |
| FR-016 | daemon catalog lock (single writer) plus `begin_start` guard, R7 |

## Project Structure

### Documentation (this feature)

```text
specs/582-attach-provider-worktrees-sessions/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── attach-worktree-tool.md
│   ├── list-resumable-sessions-tool.md
│   └── attach-protocol.md
└── tasks.md             # /speckit-tasks, not this command
```

### Source Code

```text
crates/micold-core/src/
├── attach.rs            # NEW: attachable_worktrees, discover_resumable, types (render-free, pure over a provider seam)
├── provider.rs          # EDIT: AiCliProvider gains `store_dirs(config_dir, root) -> Vec<StoreDir>`; default = known locations only, `ClaudeProvider` overrides (R3)
├── mcp/tools.rs         # EDIT: Operation::{AttachWorktree, ListResumableSessions}, catalog entries
├── mcp/policy.rs        # EDIT: Default caller refused for AttachWorktree
├── protocol/messages.rs # EDIT: ClientMsg::{AttachDiscover, AttachApply}, DaemonMsg::AttachReport
crates/micold-daemon/src/
├── catalog.rs           # EDIT: attach_worktrees (batch claim, one persist)
├── state.rs             # EDIT: attach_discover, attach_apply (blocking, off the lock)
├── server.rs            # EDIT: the two message arms
├── mcp/tools.rs         # EDIT: attach_worktree, list_resumable_sessions handlers
crates/micold-client/src/
├── features/attach.rs   # NEW: render-free reducer: offer, dialog selection, dismissal
├── shell/daemon_sync.rs # EDIT: PendingOp::Attach, request/response plumbing
├── ui/                  # EDIT: sidebar banner + "Attach existing…" dialog (view glue only)
docs/user-guide/agent-tools.md, worktrees-and-sessions.md   # EDIT
```

**Structure Decision**: the logic lives in `micold-core::attach` (testable without iced or a
daemon); the daemon only does I/O and the single write; the client reducer is render-free so the
offer and dialog rules are tested in `tests/`.

## Complexity Tracking

No constitution violations. Three deviations from the spec's original wording (now amended in spec.md to match), all for the same reason (existing behaviour must not break, FR-013), recorded for the spec owner:

| Deviation | Why | Rejected |
|---|---|---|
| Root and startable-worktree sessions stay auto-adopted at open (R2); spec FR-012 now scopes "offer only" to this feature and keeps 026's adoption | feature 026 FR-014, pinned by `session_discovery.rs` | removing it regresses 026 |
| The start-up offer triggers on "no provenance records", not "empty catalog" (data-model OfferState, R5) | 026 adoption fills the catalog before the snapshot, so a lost data directory is never catalog-empty | testing catalog emptiness (offer never shows) |
| The branch-conflict edge case maps to `NotAWorktreeOfProject` (research R1, attach-worktree-tool), because attach checks out nothing | attach is metadata-only (FR-003) | a cross-project branch scan with no failing case to guard |
