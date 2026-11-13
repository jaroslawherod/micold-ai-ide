# Implementation Plan: Worktree parent registration (MCP + sidebar) and supervisor support

**Branch**: `docs/1857_design-worktree-parent-registration-mcp-sidebar` | **Date**: 2026-11-13 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/1857-worktree-parent/spec.md`

## Summary

Record, for each worktree an agent creates, the worktree whose session created it, keyed on a stable
worktree id kept in the per-project state file. Surface it as `parent` / `parent_state` on the MCP
worktree and session rows, a `parent` filter on `list_worktrees`, and an optional `parent` argument
on `create_worktree`; show it in the sidebar as a label and a filter; and give the supervisor
(#1813, #1814, #1816) a rule to scope itself by it. Three milestones: M1 MCP and persistence, M2
sidebar UI, M3 supervisor integration. Design only here; no code is written by this issue.

## Technical Context

**Language/Version**: Rust (workspace edition as in `Cargo.toml`); shell for `scripts/autopilot`

**Primary Dependencies**: existing `micold-core` (store, mcp tool schemas), `micold-daemon` (catalog, mcp tools, ops), `micold-client` (iced sidebar); no new crates

**Storage**: per-project state file in `micold-core/src/store.rs` (sits beside `worktree_display_names`); additive, defaulted, skipped when empty, no schema bump (same pattern as spec 010 and the per-project split)

**Testing**: `cargo test` (core unit and daemon `tests/mcp_*.rs`), client sidebar tests, `scripts/tests/*.test.sh` for the supervisor rule fixtures; sandbox suite not needed

**Target Platform**: Linux, macOS, Windows (no path semantics added: ids and dir names only)

**Project Type**: desktop app with background daemon and MCP server

**Performance Goals**: listing adds one map lookup per worktree; reconcile is O(worktrees) on the existing catalog refresh

**Constraints**: local-first, additive on-disk change readable by older builds (unknown fields ignored); parent is advisory, never a permission (FR-012)

**Scale/Scope**: tens of worktrees per project

## Constitution Check

- [x] **I. Test-First**: tasks.md orders a failing test before each change; TDD plan derives the list.
- [x] **II. Multi-Session Support**: parent is per worktree, not per session; session rows only read it from their worktree; no cross-session state.
- [x] **III. Worktree Integration**: the feature is worktree-aware; the app records the link at create, no manual git step.
- [x] **IV. Local-First Storage**: only the local per-project state file changes; nothing leaves the device.
- [x] **V. Rust + iced**: `ParentRef` is an enum (`Default` | `Worktree(WorktreeId)`) with a `removed` flag; an unparented worktree is the absence of an entry, so "parent that is neither" is unrepresentable.
- [x] **VI. Cross-Platform Parity**: no OS-specific code.
- [x] **VII. Documentation**: M1 updates `docs/user-guide/agent-tools.md`, M2 `docs/user-guide/worktrees-and-sessions.md`, in the same change as the code.
- [x] **VIII. Reusable UI**: the label reuses the existing worktree tag chip and `TagFilter`; a new `Parent` variant, no feature-local widget.

Re-checked after Phase 1: still PASS. No Complexity Tracking entries.

## Project Structure

### Documentation (this feature)

```text
specs/1857-worktree-parent/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── mcp-parent.md
│   └── storage-parent.md
└── tasks.md
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── worktree.rs        # WorktreeId, ParentRef, ParentState (new types)
├── store.rs           # worktree_ids, worktree_parents in the per-project state
└── mcp/tools.rs       # schema: parent input on create_worktree / list_worktrees; row fields
crates/micold-daemon/src/
├── catalog.rs         # reconcile: mint ids, tombstone links, resolve parent for snapshot
├── ops.rs             # create/delete hooks: record link, mint id, tombstone children
└── mcp/tools.rs       # create_worktree_row, list_worktrees, session_row, get_session
crates/micold-client/src/features/
├── sidebar.rs         # TagFilter::Parent, available_tag_filters, parent label
└── worktree.rs        # worktree_tags gains the parent tag
docs/user-guide/       # agent-tools.md, worktrees-and-sessions.md
.claude/skills/speckit-supervisor/ and scripts/autopilot/   # M3: ownership rule, ledger field
```

**Structure Decision**: extend the existing core/daemon/client split; no new crate. Milestones:
**M1** (core, daemon, MCP, docs; US1, US2, US3, US5), **M2** (client sidebar; US4, blocked by M1),
**M3** (supervisor rule and ledger; blocked by M1; consumed by #1813, #1814, #1816).

## Complexity Tracking

None.
